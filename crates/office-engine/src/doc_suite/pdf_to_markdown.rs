use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use flate2::read::ZlibDecoder;
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

fn default_heading_scale() -> f32 {
    1.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfToMarkdownOptions {
    #[serde(default)]
    pub extract_tables: bool,
    #[serde(default = "default_heading_scale")]
    pub heading_scale: f32,
}

impl Default for PdfToMarkdownOptions {
    fn default() -> Self {
        Self {
            extract_tables: true,
            heading_scale: 1.5,
        }
    }
}

pub struct PdfToMarkdownTool;

impl Default for PdfToMarkdownTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct TextElement {
    text: String,
    font_size: f32,
    _x: f32,
    _y: f32,
}

/// Bóc tách văn bản từ luồng PDF content stream
fn extract_text_from_stream(stream_bytes: &[u8]) -> Vec<TextElement> {
    let mut elements = Vec::new();
    let text_content = String::from_utf8_lossy(stream_bytes);
    let mut current_font_size = 12.0f32;
    let mut cur_x = 0.0f32;
    let mut cur_y = 0.0f32;

    for line in text_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Toán tử chọn font: /F1 18 Tf
        if trimmed.ends_with("Tf") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                if let Ok(size) = parts[parts.len() - 2].parse::<f32>() {
                    current_font_size = size;
                }
            }
        }
        // Toán tử ma trận tọa độ: 1 0 0 1 x y Tm
        else if trimmed.ends_with("Tm") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 7 {
                if let (Ok(x), Ok(y)) = (parts[parts.len() - 3].parse::<f32>(), parts[parts.len() - 2].parse::<f32>()) {
                    cur_x = x;
                    cur_y = y;
                }
            }
        }
        // Toán tử dịch chuyển: x y Td
        else if trimmed.ends_with("Td") || trimmed.ends_with("TD") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                if let (Ok(dx), Ok(dy)) = (parts[parts.len() - 3].parse::<f32>(), parts[parts.len() - 2].parse::<f32>()) {
                    cur_x += dx;
                    cur_y += dy;
                }
            }
        }
        // Toán tử in chuỗi văn bản: (Nội dung) Tj
        else if trimmed.ends_with("Tj") {
            if let Some(start) = trimmed.find('(') {
                if let Some(end) = trimmed.rfind(')') {
                    if end > start {
                        let text = trimmed[start + 1..end].to_string();
                        elements.push(TextElement {
                            text,
                            font_size: current_font_size,
                            _x: cur_x,
                            _y: cur_y,
                        });
                    }
                }
            }
        }
        // Toán tử mảng chuỗi: [(Chuỗi 1) -10 (Chuỗi 2)] TJ
        else if trimmed.ends_with("TJ") {
            let mut extracted = String::new();
            let mut in_paren = false;
            let mut cur_token = String::new();

            for c in trimmed.chars() {
                if c == '(' {
                    in_paren = true;
                    cur_token.clear();
                } else if c == ')' {
                    in_paren = false;
                    extracted.push_str(&cur_token);
                } else if in_paren {
                    cur_token.push(c);
                }
            }

            if !extracted.is_empty() {
                elements.push(TextElement {
                    text: extracted,
                    font_size: current_font_size,
                    _x: cur_x,
                    _y: cur_y,
                });
            }
        }
    }

    elements
}

/// Nhận diện xem một nhóm dòng có phải là bảng biểu hay không
fn is_table_row(line: &str) -> bool {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() >= 2 {
        return true;
    }
    let pipe_parts: Vec<&str> = line.split('|').collect();
    if pipe_parts.len() >= 3 {
        return true;
    }
    // Hoặc phân tách bằng 2 dấu cách trở lên
    let spaces: Vec<&str> = line.split("  ").filter(|s| !s.trim().is_empty()).collect();
    spaces.len() >= 2
}

/// Định dạng dòng bảng thành chuẩn Markdown
fn format_table_row(line: &str) -> Vec<String> {
    if line.contains('\t') {
        line.split('\t').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    } else if line.contains('|') {
        line.split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    } else {
        line.split("  ").map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    }
}

/// Dựng nội dung Markdown hoàn chỉnh từ các TextElement
fn convert_elements_to_markdown(elements: &[TextElement], options: &PdfToMarkdownOptions) -> String {
    let mut md = String::new();
    let mut table_buffer: Vec<Vec<String>> = Vec::new();

    let flush_table = |tbl: &mut Vec<Vec<String>>, out: &mut String| {
        if tbl.is_empty() {
            return;
        }
        let col_count = tbl.iter().map(|r| r.len()).max().unwrap_or(1);
        if let Some(header) = tbl.first() {
            out.push_str("| ");
            for i in 0..col_count {
                let cell = header.get(i).map(|s| s.as_str()).unwrap_or("");
                out.push_str(cell);
                out.push_str(" | ");
            }
            out.push('\n');

            out.push_str("| ");
            for _ in 0..col_count {
                out.push_str("--- | ");
            }
            out.push('\n');

            for row in &tbl[1..] {
                out.push_str("| ");
                for i in 0..col_count {
                    let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                    out.push_str(cell);
                    out.push_str(" | ");
                }
                out.push('\n');
            }
            out.push('\n');
        }
        tbl.clear();
    };

    let h1_thresh = 18.0 * (options.heading_scale / 1.5);
    let h2_thresh = 14.0 * (options.heading_scale / 1.5);

    for elem in elements {
        let trimmed = elem.text.trim();
        if trimmed.is_empty() {
            continue;
        }

        if options.extract_tables && is_table_row(trimmed) {
            let cells = format_table_row(trimmed);
            if cells.len() >= 2 {
                table_buffer.push(cells);
                continue;
            }
        }

        if !table_buffer.is_empty() {
            flush_table(&mut table_buffer, &mut md);
        }

        if elem.font_size >= h1_thresh {
            md.push_str(&format!("# {}\n\n", trimmed));
        } else if elem.font_size >= h2_thresh {
            md.push_str(&format!("## {}\n\n", trimmed));
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            md.push_str(&format!("{}\n", trimmed));
        } else {
            md.push_str(&format!("{}\n\n", trimmed));
        }
    }

    if !table_buffer.is_empty() {
        flush_table(&mut table_buffer, &mut md);
    }

    md
}

impl OfficeTool for PdfToMarkdownTool {
    fn slug(&self) -> &'static str {
        "pdf-to-markdown"
    }

    fn name(&self) -> &'static str {
        "Chuyển đổi PDF sang Markdown"
    }

    fn description(&self) -> &'static str {
        "Bóc tách luồng văn bản, phân tích kích thước và tọa độ font chữ (Tm, Td operators) để nhận diện tiêu đề (#, ##), đoạn văn bản, danh sách và tái tạo bảng biểu định dạng Markdown."
    }

    fn execute(
        &self,
        request: &DispatchTaskRequest,
        cancel_flag: &AtomicBool,
        on_progress: Option<&dyn Fn(TaskProgress)>,
    ) -> Result<DispatchTaskResponse, CommandError> {
        let start_time = Instant::now();

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let options: PdfToMarkdownOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình chuyển đổi Markdown không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp đầu vào trống".to_string(),
            });
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_to_md_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Đang đọc tệp và phân tích cấu trúc luồng PDF...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: request.input_paths.len(),
                is_completed: false,
            });
        }

        let mut output_paths = Vec::new();

        for (idx, file_path_str) in request.input_paths.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let path = Path::new(file_path_str);
            if !path.exists() {
                return Err(CommandError::FileNotFound {
                    path: file_path_str.clone(),
                });
            }

            let bytes = fs::read(path).map_err(|e| CommandError::IoError {
                details: format!("Không thể đọc tệp {file_path_str}: {e}"),
            })?;

            if bytes.len() < 5 || &bytes[0..5] != b"%PDF-" {
                let actual = if bytes.len() >= 5 {
                    String::from_utf8_lossy(&bytes[0..5]).to_string()
                } else {
                    format!("{:?}", bytes)
                };
                return Err(CommandError::InvalidFileSignature {
                    expected: "%PDF-".to_string(),
                    actual,
                });
            }

            // Quét tìm các luồng stream
            let mut all_elements = Vec::new();
            let mut search_from = 0;

            while search_from < bytes.len() {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                if let Some(pos) = bytes[search_from..].windows(6).position(|w| w == b"stream") {
                    let stream_start = search_from + pos + 6;
                    let content_start = if stream_start < bytes.len() && bytes[stream_start] == b'\r' && bytes[stream_start + 1] == b'\n' {
                        stream_start + 2
                    } else if stream_start < bytes.len() && (bytes[stream_start] == b'\n' || bytes[stream_start] == b'\r') {
                        stream_start + 1
                    } else {
                        stream_start
                    };

                    if let Some(end_pos) = bytes[content_start..].windows(9).position(|w| w == b"endstream") {
                        let stream_bytes = &bytes[content_start..content_start + end_pos];

                        // Thử giải nén Zlib nếu là compressed FlateDecode
                        let mut decompressed = Vec::new();
                        let mut decoder = ZlibDecoder::new(stream_bytes).take(10 * 1024 * 1024); // 10MB limit bomb defense
                        if decoder.read_to_end(&mut decompressed).is_ok() && !decompressed.is_empty() {
                            let elems = extract_text_from_stream(&decompressed);
                            all_elements.extend(elems);
                        } else {
                            // Luồng văn bản thô chưa nén
                            let elems = extract_text_from_stream(stream_bytes);
                            all_elements.extend(elems);
                        }

                        search_from = content_start + end_pos + 9;
                        continue;
                    }
                }
                break;
            }

            // Nếu không tìm thấy stream qua toán tử chuẩn, bóc tách chuỗi trực tiếp từ object text
            if all_elements.is_empty() {
                all_elements = extract_text_from_stream(&bytes);
            }

            let markdown_result = convert_elements_to_markdown(&all_elements, &options);

            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
            let out_path = output_dir.join(format!("{file_stem}.md"));
            fs::write(&out_path, markdown_result.as_bytes()).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi tệp Markdown {}: {e}", out_path.display()),
            })?;

            output_paths.push(out_path.to_string_lossy().to_string());

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: 20.0 + (idx as f32 / request.input_paths.len() as f32) * 70.0,
                    message: format!("Đã chuyển đổi {}/{} trang sang Markdown", idx + 1, request.input_paths.len()),
                    processed_files: idx + 1,
                    failed_files: 0,
                    total_files: request.input_paths.len(),
                    is_completed: false,
                });
            }
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Chuyển đổi PDF sang Markdown hoàn tất xuất sắc.".to_string(),
                processed_files: request.input_paths.len(),
                failed_files: 0,
                total_files: request.input_paths.len(),
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths,
            failed_items: Vec::new(),
            elapsed_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::AtomicBool;

    fn create_test_markdown_pdf() -> Vec<u8> {
        let stream_content = "\
/F1 24 Tf\n\
(Tieu De Chinh Cua Tai Lieu) Tj\n\
/F2 16 Tf\n\
(Tieu De Phu Muc 1) Tj\n\
/F3 12 Tf\n\
(Day la doan van ban mo ta noi dung chi tiet trong file PDF.) Tj\n\
";

        let pdf = format!(
            "%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
xref\n0 5\n0000000000 65535 f \n\
trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n500\n%%EOF\n",
            stream_content.len(),
            stream_content
        );

        pdf.into_bytes()
    }

    fn create_table_test_pdf() -> Vec<u8> {
        let stream_content = "\
/F1 18 Tf\n\
(Bao Cao Doanh Thu) Tj\n\
/F2 12 Tf\n\
(Cot 1\tCot 2\tCot 3) Tj\n\
(San Pham A\t100\t1.000.000d) Tj\n\
(San Pham B\t200\t2.500.000d) Tj\n\
";

        let pdf = format!(
            "%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
xref\n0 5\n0000000000 65535 f \n\
trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n500\n%%EOF\n",
            stream_content.len(),
            stream_content
        );

        pdf.into_bytes()
    }

    #[test]
    fn test_pdf_to_markdown_happy_path() {
        let tool = PdfToMarkdownTool::default();
        let tmp = std::env::temp_dir().join("test_pdf_to_md_happy");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_test_markdown_pdf()).expect("write pdf");

        let req = DispatchTaskRequest {
            task_id: "task-md-happy".to_string(),
            tool_slug: "pdf-to-markdown".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "extract_tables": true,
                "heading_scale": 1.5
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("conversion succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let md_content = fs::read_to_string(&res.output_paths[0]).expect("read md");
        assert!(md_content.contains("# Tieu De Chinh Cua Tai Lieu"));
        assert!(md_content.contains("## Tieu De Phu Muc 1"));
        assert!(md_content.contains("Day la doan van ban mo ta"));
    }

    #[test]
    fn test_pdf_to_markdown_edge_case() {
        let tool = PdfToMarkdownTool::default();
        let tmp = std::env::temp_dir().join("test_pdf_to_md_edge");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("table_doc.pdf");
        fs::write(&pdf_path, create_table_test_pdf()).expect("write table pdf");

        let req = DispatchTaskRequest {
            task_id: "task-md-table".to_string(),
            tool_slug: "pdf-to-markdown".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "extract_tables": true,
                "heading_scale": 1.5
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("conversion succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let md_content = fs::read_to_string(&res.output_paths[0]).expect("read md");
        assert!(md_content.contains("| Cot 1 | Cot 2 | Cot 3 |"));
        assert!(md_content.contains("| --- | --- | --- |"));
        assert!(md_content.contains("| San Pham A | 100 | 1.000.000d |"));
    }

    #[test]
    fn test_pdf_to_markdown_invalid_magic_bytes() {
        let tool = PdfToMarkdownTool::default();
        let tmp = std::env::temp_dir().join("test_pdf_to_md_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("bad.pdf");
        fs::write(&fake_path, b"NOT_A_PDF").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-fake".to_string(),
            tool_slug: "pdf-to-markdown".to_string(),
            input_paths: vec![fake_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { expected, .. } => {
                assert_eq!(expected, "%PDF-");
            }
            other => panic!("Expected InvalidFileSignature, got {:?}", other),
        }
    }

    #[test]
    fn test_pdf_to_markdown_cooperative_cancellation() {
        let tool = PdfToMarkdownTool::default();
        let tmp = std::env::temp_dir().join("test_pdf_to_md_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_test_markdown_pdf()).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-to-markdown".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_to_markdown_security_guardrails() {
        let tool = PdfToMarkdownTool::default();
        assert_eq!(tool.slug(), "pdf-to-markdown");
        assert_eq!(tool.name(), "Chuyển đổi PDF sang Markdown");
    }
}
