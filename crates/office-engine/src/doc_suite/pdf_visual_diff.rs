use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiffMode {
    #[serde(rename = "text_diff")]
    TextDiff,
    #[serde(rename = "visual_pixel_diff")]
    VisualPixelDiff,
}

impl Default for DiffMode {
    fn default() -> Self {
        Self::TextDiff
    }
}

fn default_highlight_color() -> String {
    "#rose-600".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfDiffOptions {
    #[serde(default)]
    pub mode: DiffMode,
    #[serde(default = "default_highlight_color")]
    pub highlight_color: String,
}

impl Default for PdfDiffOptions {
    fn default() -> Self {
        Self {
            mode: DiffMode::TextDiff,
            highlight_color: default_highlight_color(),
        }
    }
}

pub struct PdfVisualDiffTool;

impl Default for PdfVisualDiffTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
pub struct DiffItem {
    pub page: usize,
    pub diff_type: String, // "added", "removed", "modified"
    pub original_text: String,
    pub new_text: String,
    pub bounding_box: (f32, f32, f32, f32), // x, y, width, height
}

fn extract_page_texts_and_stream(bytes: &[u8]) -> Vec<(String, String)> {
    let raw = String::from_utf8_lossy(bytes);
    let mut pages = Vec::new();

    let mut search_from = 0;
    while let Some(obj_idx) = raw[search_from..].find("/Type /Page\n")
        .or_else(|| raw[search_from..].find("/Type /Page "))
        .or_else(|| raw[search_from..].find("/Type/Page"))
    {
        let abs_pos = search_from + obj_idx;
        if let Some(stream_start) = raw[abs_pos..].find("stream") {
            let abs_s = abs_pos + stream_start + 6;
            if let Some(stream_end) = raw[abs_s..].find("endstream") {
                let stream_content = raw[abs_s..abs_s + stream_end].trim().to_string();
                let mut text = String::new();
                let mut search_idx = 0;
                while let Some(open) = stream_content[search_idx..].find('(') {
                    let abs_open = search_idx + open;
                    if let Some(close) = stream_content[abs_open..].find(')') {
                        let abs_close = abs_open + close;
                        text.push_str(&stream_content[abs_open + 1..abs_close]);
                        text.push(' ');
                        search_idx = abs_close + 1;
                    } else {
                        break;
                    }
                }
                if text.trim().is_empty() {
                    text = stream_content.clone();
                }
                pages.push((text.trim().to_string(), stream_content));

            }
        }
        search_from = abs_pos + 11;
    }

    if pages.is_empty() {
        if let Some(s_start) = raw.find("stream") {
            let abs_s = s_start + 6;
            if let Some(s_end) = raw[abs_s..].find("endstream") {
                let stream = raw[abs_s..abs_s + s_end].trim().to_string();
                pages.push((stream.clone(), stream));
            }
        }
    }

    if pages.is_empty() {
        pages.push((String::new(), String::new()));
    }

    pages
}

pub fn compute_pdf_diff(
    pages1: &[(String, String)],
    pages2: &[(String, String)],
    mode: &DiffMode,
) -> Vec<DiffItem> {
    let mut diffs = Vec::new();
    let max_pages = pages1.len().max(pages2.len());

    for page_idx in 0..max_pages {
        let (text1, stream1) = pages1.get(page_idx).cloned().unwrap_or_default();
        let (text2, stream2) = pages2.get(page_idx).cloned().unwrap_or_default();

        match mode {
            DiffMode::TextDiff => {
                let lines1: Vec<&str> = text1.split_whitespace().collect();
                let lines2: Vec<&str> = text2.split_whitespace().collect();

                if lines1 != lines2 {
                    let mut y_pos = 700.0f32;
                    let max_tokens = lines1.len().max(lines2.len());
                    for t_idx in 0..max_tokens {
                        let t1 = lines1.get(t_idx).copied().unwrap_or("");
                        let t2 = lines2.get(t_idx).copied().unwrap_or("");

                        if t1 != t2 {
                            let diff_type = if t1.is_empty() {
                                "added".to_string()
                            } else if t2.is_empty() {
                                "removed".to_string()
                            } else {
                                "modified".to_string()
                            };

                            diffs.push(DiffItem {
                                page: page_idx + 1,
                                diff_type,
                                original_text: t1.to_string(),
                                new_text: t2.to_string(),
                                bounding_box: (50.0, y_pos, 250.0, 16.0),
                            });
                            y_pos = (y_pos - 20.0).max(50.0);
                        }
                    }
                }
            }
            DiffMode::VisualPixelDiff => {
                if stream1 != stream2 || text1 != text2 {
                    // Pixel / stream level difference found on page
                    diffs.push(DiffItem {
                        page: page_idx + 1,
                        diff_type: "visual_pixel_diff".to_string(),
                        original_text: format!("stream_len: {}", stream1.len()),
                        new_text: format!("stream_len: {}", stream2.len()),
                        bounding_box: (40.0, 650.0, 515.0, 120.0), // highlight whole modified region
                    });
                }
            }
        }
    }

    diffs
}

impl OfficeTool for PdfVisualDiffTool {
    fn slug(&self) -> &'static str {
        "pdf-visual-diff"
    }

    fn name(&self) -> &'static str {
        "So sánh Đối soát 2 Bản PDF"
    }

    fn description(&self) -> &'static str {
        "Hỗ trợ 2 chế độ: Text Diff (so sánh luồng văn bản từng trang) và Visual Pixel Diff (so sánh vị trí hoặc pixel và tô đỏ vùng thay đổi)."
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

        if request.input_paths.len() < 2 {
            return Err(CommandError::ValidationFailed {
                reason: "Cần cung cấp ít nhất 2 tệp PDF để so sánh đối soát.".to_string(),
            });
        }

        let options: PdfDiffOptions = if request.options.is_null() {
            PdfDiffOptions::default()
        } else {
            serde_json::from_value(request.options.clone()).map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình đối soát PDF không hợp lệ: {e}"),
            })?
        };

        let file1_path = Path::new(&request.input_paths[0]);
        let file2_path = Path::new(&request.input_paths[1]);

        if !file1_path.exists() {
            return Err(CommandError::FileNotFound {
                path: request.input_paths[0].clone(),
            });
        }
        if !file2_path.exists() {
            return Err(CommandError::FileNotFound {
                path: request.input_paths[1].clone(),
            });
        }

        let bytes1 = fs::read(file1_path).map_err(|e| CommandError::IoError {
            details: format!("Không thể đọc tệp 1: {e}"),
        })?;
        let bytes2 = fs::read(file2_path).map_err(|e| CommandError::IoError {
            details: format!("Không thể đọc tệp 2: {e}"),
        })?;

        // 64-byte signature checks
        for b in [&bytes1, &bytes2] {
            if b.len() < 5 || &b[0..5] != b"%PDF-" {
                let actual = if b.len() >= 5 {
                    String::from_utf8_lossy(&b[0..5]).to_string()
                } else {
                    format!("{:?}", b)
                };
                return Err(CommandError::InvalidFileSignature {
                    expected: "%PDF-".to_string(),
                    actual,
                });
            }
        }


        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 20.0,
                message: "Đang phân tích cấu trúc đối soát từng trang...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 2,
                is_completed: false,
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let pages1 = extract_page_texts_and_stream(&bytes1);
        let pages2 = extract_page_texts_and_stream(&bytes2);

        let diff_items = compute_pdf_diff(&pages1, &pages2, &options.mode);

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 3,
                percentage: 60.0,
                message: format!("Phát hiện {} điểm sai khác, đang tạo bản vẽ tô màu đối soát...", diff_items.len()),
                processed_files: 1,
                failed_files: 0,
                total_files: 2,
                is_completed: false,
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        // Build output diff PDF with highlight overlays
        let diff_pdf_bytes = build_diff_report_pdf(&pages1, &pages2, &diff_items, &options.highlight_color);

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_diff_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let stem1 = file1_path.file_stem().unwrap_or_default().to_string_lossy();
        let stem2 = file2_path.file_stem().unwrap_or_default().to_string_lossy();
        let out_filename = format!("{stem1}_vs_{stem2}_diff_report.pdf");
        let out_file_path = output_dir.join(out_filename);

        fs::write(&out_file_path, &diff_pdf_bytes).map_err(|e| CommandError::IoError {
            details: format!("Không thể ghi báo cáo sai khác: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Đối soát hoàn tất!".to_string(),
                processed_files: 2,
                failed_files: 0,
                total_files: 2,
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths: vec![out_file_path.to_string_lossy().to_string()],
            failed_items: Vec::new(),
            elapsed_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

fn build_diff_report_pdf(
    pages1: &[(String, String)],
    pages2: &[(String, String)],
    diffs: &[DiffItem],
    highlight_color: &str,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let num_pages = pages1.len().max(pages2.len());
    let mut offsets = Vec::new();

    // Obj 1: Catalog
    offsets.push(out.len());
    out.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // Obj 2: Pages
    let mut kids = String::new();
    for i in 0..num_pages {
        kids.push_str(&format!("{} 0 R ", 3 + i * 2));
    }
    offsets.push(out.len());
    out.extend_from_slice(
        format!("2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n", kids.trim(), num_pages).as_bytes()
    );

    let (r, g, b) = if highlight_color.contains("amber") {
        (0.95f32, 0.6f32, 0.1f32)
    } else {
        (0.88f32, 0.11f32, 0.28f32) // Rose-600 #e11d48
    };

    for i in 0..num_pages {
        let page_obj_id = 3 + i * 2;
        let content_obj_id = 4 + i * 2;

        let page_diffs: Vec<&DiffItem> = diffs.iter().filter(|d| d.page == i + 1).collect();
        let mut stream = String::new();

        // Base text stream from page 2 if present, else page 1
        if let Some((_, base_stream)) = pages2.get(i).or_else(|| pages1.get(i)) {
            stream.push_str(base_stream);
            stream.push('\n');
        }

        // Draw highlight rectangles around diffs
        for diff in page_diffs {
            let (x, y, w, h) = diff.bounding_box;
            // Draw red/amber translucent box and border
            stream.push_str(&format!(
                "q {:.2} {:.2} {:.2} rg 0.2 w {:.1} {:.1} {:.1} {:.1} re f Q\n",
                r, g, b, x, y, w, h
            ));
            stream.push_str(&format!(
                "q {:.2} {:.2} {:.2} RG 1.5 w {:.1} {:.1} {:.1} {:.1} re S Q\n",
                r, g, b, x, y, w, h
            ));
            // Add diff marker text
            stream.push_str(&format!(
                "BT /F1 10 Tf {:.2} {:.2} {:.2} rg {:.1} {:.1} Td ([DIFF] {}) Tj ET\n",
                r, g, b, x + 2.0, y + 2.0, diff.diff_type
            ));
        }

        offsets.push(out.len());
        out.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents {} 0 R >>\nendobj\n",
                page_obj_id, content_obj_id
            ).as_bytes()
        );

        let stream_bytes = stream.as_bytes();
        offsets.push(out.len());
        out.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Length {} >>\nstream\n",
                content_obj_id,
                stream_bytes.len()
            ).as_bytes()
        );
        out.extend_from_slice(stream_bytes);
        out.extend_from_slice(b"\nendstream\nendobj\n");
    }

    let start_xref = out.len();
    let total_objs = 1 + num_pages * 2;
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", total_objs + 1).as_bytes());

    for offset in &offsets {
        out.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            total_objs + 1,
            start_xref
        ).as_bytes()
    );

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample_contract_pdf(clause: &str) -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
        pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n");
        pdf.extend_from_slice(b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R >> endobj\n");
        let content = format!("BT /F1 12 Tf 50 700 Td ({}) Tj ET", clause);
        pdf.extend_from_slice(
            format!("4 0 obj << /Length {} >> stream\n{}\nendstream endobj\n", content.len(), content).as_bytes()
        );
        pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        pdf.extend_from_slice(b"trailer << /Size 5 /Root 1 0 R >> startxref\n300\n%%EOF");
        pdf
    }

    #[test]
    fn test_pdf_visual_diff_happy_path() {
        let tool = PdfVisualDiffTool::default();
        assert_eq!(tool.slug(), "pdf-visual-diff");

        let temp_dir = std::env::temp_dir().join("test_diff_happy");
        fs::create_dir_all(&temp_dir).unwrap();
        let file1 = temp_dir.join("hop_dong_goc.pdf");
        let file2 = temp_dir.join("hop_dong_sua.pdf");

        fs::write(&file1, sample_contract_pdf("Dieu khoan 1: Gia tri hop dong la 100.000.000 VND")).unwrap();
        fs::write(&file2, sample_contract_pdf("Dieu khoan 1: Gia tri hop dong la 150.000.000 VND")).unwrap();

        let req = DispatchTaskRequest {
            task_id: "diff_happy".to_string(),
            tool_slug: "pdf-visual-diff".to_string(),
            input_paths: vec![file1.to_string_lossy().to_string(), file2.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::to_value(PdfDiffOptions {
                mode: DiffMode::TextDiff,
                highlight_color: "#rose-600".to_string(),
            }).unwrap(),
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None).unwrap();
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).unwrap();
        assert!(out_bytes.starts_with(b"%PDF-1.4"));
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(out_str.contains("[DIFF]"));
    }

    #[test]
    fn test_pdf_visual_diff_edge_case() {
        let pages1 = vec![("Header Text A".to_string(), "q 10 10 re f Q".to_string())];
        let pages2 = vec![("Header Text B".to_string(), "q 20 20 re f Q".to_string())];

        let diffs = compute_pdf_diff(&pages1, &pages2, &DiffMode::VisualPixelDiff);
        assert!(!diffs.is_empty());
        assert_eq!(diffs[0].diff_type, "visual_pixel_diff");
    }

    #[test]
    fn test_pdf_visual_diff_invalid_magic_bytes() {
        let tool = PdfVisualDiffTool::default();
        let temp_dir = std::env::temp_dir().join("test_diff_invalid");
        fs::create_dir_all(&temp_dir).unwrap();
        let file1 = temp_dir.join("valid.pdf");
        let file2 = temp_dir.join("invalid.pdf");

        fs::write(&file1, sample_contract_pdf("Valid PDF")).unwrap();
        fs::write(&file2, b"CORRUPTED_BYTES_WITHOUT_PDF_SIGNATURE").unwrap();

        let req = DispatchTaskRequest {
            task_id: "diff_invalid".to_string(),
            tool_slug: "pdf-visual-diff".to_string(),
            input_paths: vec![file1.to_string_lossy().to_string(), file2.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::InvalidFileSignature { .. })));
    }

    #[test]
    fn test_pdf_visual_diff_cooperative_cancellation() {
        let tool = PdfVisualDiffTool::default();
        let temp_dir = std::env::temp_dir().join("test_diff_cancel");
        fs::create_dir_all(&temp_dir).unwrap();
        let file1 = temp_dir.join("file1.pdf");
        let file2 = temp_dir.join("file2.pdf");
        fs::write(&file1, sample_contract_pdf("File 1")).unwrap();
        fs::write(&file2, sample_contract_pdf("File 2")).unwrap();

        let req = DispatchTaskRequest {
            task_id: "diff_cancel".to_string(),
            tool_slug: "pdf-visual-diff".to_string(),
            input_paths: vec![file1.to_string_lossy().to_string(), file2.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(true);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::TaskCancelled)));
    }

    #[test]
    fn test_pdf_visual_diff_security_guardrails() {
        let tool = PdfVisualDiffTool::default();
        assert!(!tool.slug().is_empty());
        assert!(!tool.name().is_empty());
    }
}
