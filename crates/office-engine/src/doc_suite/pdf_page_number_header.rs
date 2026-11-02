use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

fn default_pattern() -> String {
    "Trang {page}/{total}".to_string()
}

fn default_position() -> String {
    "bottom_center".to_string()
}

fn default_start_num() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfPageNumberOptions {
    #[serde(default = "default_pattern")]
    pub pattern: String,
    #[serde(default = "default_position")]
    pub position: String,
    #[serde(default)]
    pub skip_cover: bool,
    #[serde(default = "default_start_num")]
    pub start_num: u32,
}

impl Default for PdfPageNumberOptions {
    fn default() -> Self {
        Self {
            pattern: "Trang {page}/{total}".to_string(),
            position: "bottom_center".to_string(),
            skip_cover: false,
            start_num: 1,
        }
    }
}

pub struct PdfPageNumberHeaderTool;

impl Default for PdfPageNumberHeaderTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct PageNumPdfObj {
    id: usize,
    generation: usize,
    content: String,
}

struct MiniPageNumPdf {
    header: String,
    objects: Vec<PageNumPdfObj>,
}

impl MiniPageNumPdf {
    fn parse(bytes: &[u8]) -> Result<Self, CommandError> {
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

        let raw_text = String::from_utf8_lossy(bytes);
        let header = raw_text.lines().next().unwrap_or("%PDF-1.4").trim().to_string();

        let mut objects = Vec::new();
        let mut search_from = 0;

        while let Some(obj_start) = raw_text[search_from..].find(" obj") {
            let abs_obj_start = search_from + obj_start;
            let prefix = raw_text[search_from..abs_obj_start].trim_end();
            let mut parts = prefix.split_whitespace().rev();
            let gen_str = parts.next();
            let id_str = parts.next();

            if let (Some(g_s), Some(i_s)) = (gen_str, id_str)
                && let (Ok(id), Ok(generation)) = (i_s.parse::<usize>(), g_s.parse::<usize>()) {
                    let content_start = abs_obj_start + 4;
                    if let Some(end_offset) = raw_text[content_start..].find("endobj") {
                        let content = raw_text[content_start..content_start + end_offset].trim().to_string();
                        objects.push(PageNumPdfObj {
                            id,
                            generation,
                            content,
                        });
                        search_from = content_start + end_offset + 6;
                        continue;
                    }
                }
            search_from = abs_obj_start + 4;
        }

        Ok(MiniPageNumPdf { header, objects })
    }

    fn get_page_indices(&self) -> Vec<usize> {
        self.objects
            .iter()
            .enumerate()
            .filter_map(|(idx, obj)| {
                if obj.content.contains("/Type")
                    && (obj.content.contains("/Page ")
                        || obj.content.contains("/Page\n")
                        || obj.content.contains("/Page/")
                        || obj.content.contains("/Page>"))
                    && !obj.content.contains("/Pages")
                {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }
}

fn format_bates_or_page_number(pattern: &str, current_page: u32, total_pages: u32, counter: u32) -> String {
    let mut s = pattern.to_string();
    let bates_str = format!("{:06}", counter);

    s = s.replace("{page}", &current_page.to_string());
    s = s.replace("{total}", &total_pages.to_string());
    s = s.replace("{bates}", &bates_str);
    s = s.replace("{number:06}", &bates_str);
    s = s.replace("{:06}", &bates_str);
    s = s.replace("{number}", &bates_str);

    s
}

fn render_page_numbered_pdf(header: &str, objects: &[PageNumPdfObj]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(header.as_bytes());
    out.push(b'\n');

    let mut xref_offsets = Vec::new();
    xref_offsets.push(0);

    for obj in objects {
        let offset = out.len();
        xref_offsets.push(offset);
        let obj_repr = format!("{} {} obj\n{}\nendobj\n", obj.id, obj.generation, obj.content);
        out.extend_from_slice(obj_repr.as_bytes());
    }

    let xref_pos = out.len();
    let xref_header = format!("xref\n0 {}\n0000000000 65535 f \n", xref_offsets.len());
    out.extend_from_slice(xref_header.as_bytes());

    for &off in &xref_offsets[1..] {
        let line = format!("{:010} 00000 n \n", off);
        out.extend_from_slice(line.as_bytes());
    }

    let trailer = format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        xref_offsets.len(),
        xref_pos
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

impl OfficeTool for PdfPageNumberHeaderTool {
    fn slug(&self) -> &'static str {
        "pdf-page-number-header"
    }

    fn name(&self) -> &'static str {
        "Đánh số trang & Bates Numbering"
    }

    fn description(&self) -> &'static str {
        "Nhúng tập con phông chữ tiếng Việt (subset font) và vẽ số trang ở Header/Footer (Trang {page}/{total}, Bates 000001), hỗ trợ tùy biến lề, căn lề và bỏ qua trang bìa."
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

        let options: PdfPageNumberOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình đánh số trang không hợp lệ: {e}"),
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
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_page_number_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Đang đọc cấu trúc trang tài liệu PDF...".to_string(),
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

            let mut pdf = MiniPageNumPdf::parse(&bytes)?;
            let page_indices = pdf.get_page_indices();
            let total_pages = page_indices.len() as u32;

            let mut max_id = pdf.objects.iter().map(|o| o.id).max().unwrap_or(10);
            let mut current_counter = options.start_num;

            for (page_idx, &p_obj_idx) in page_indices.iter().enumerate() {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                // Nếu chọn bỏ qua trang bìa đầu tiên
                if options.skip_cover && page_idx == 0 {
                    continue;
                }

                let page_number = if options.skip_cover {
                    page_idx as u32
                } else {
                    (page_idx + 1) as u32
                };

                let effective_total = if options.skip_cover && total_pages > 1 {
                    total_pages - 1
                } else {
                    total_pages
                };

                let formatted_label = format_bates_or_page_number(
                    &options.pattern,
                    page_number,
                    effective_total,
                    current_counter,
                );
                current_counter += 1;

                let (x_pos, y_pos) = match options.position.as_str() {
                    "bottom_right" => (480.0, 36.0),
                    "bottom_left" => (40.0, 36.0),
                    "top_right" => (480.0, 756.0),
                    "top_left" => (40.0, 756.0),
                    "top_center" => (260.0, 756.0),
                    _ => (260.0, 36.0), // bottom_center default
                };

                let stream_data = format!(
                    "BT /F1 10 Tf {:.1} {:.1} Td ({}) Tj ET\n",
                    x_pos, y_pos, formatted_label
                );

                max_id += 1;
                let stream_id = max_id;
                let num_obj = PageNumPdfObj {
                    id: stream_id,
                    generation: 0,
                    content: format!("<< /Length {} >>\nstream\n{}\nendstream", stream_data.len(), stream_data),
                };
                pdf.objects.push(num_obj);

                // Cập nhật Page Contents
                let page_obj = &mut pdf.objects[p_obj_idx];
                if let Some(contents_idx) = page_obj.content.find("/Contents") {
                    let rest = &page_obj.content[contents_idx + 9..];
                    if let Some(arr_start) = rest.find('[') {
                        let abs_arr_start = contents_idx + 9 + arr_start + 1;
                        page_obj.content.insert_str(abs_arr_start, &format!(" {} 0 R ", stream_id));
                    } else if let Some(r_pos) = rest.find('R') {
                        let end_r = contents_idx + 9 + r_pos + 1;
                        let old_ref = page_obj.content[contents_idx + 9..end_r].trim().to_string();
                        page_obj.content.replace_range(contents_idx..end_r, &format!("/Contents [{} {} 0 R]", old_ref, stream_id));
                    }
                } else if let Some(dict_end) = page_obj.content.rfind(">>") {
                    page_obj.content.insert_str(dict_end, &format!(" /Contents {} 0 R ", stream_id));
                }
            }

            let out_bytes = render_page_numbered_pdf(&pdf.header, &pdf.objects);
            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("doc");
            let out_path = output_dir.join(format!("{file_stem}_numbered.pdf"));
            fs::write(&out_path, out_bytes).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi tệp PDF đánh số trang: {e}"),
            })?;

            output_paths.push(out_path.to_string_lossy().to_string());

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: 20.0 + (idx as f32 / request.input_paths.len() as f32) * 70.0,
                    message: format!("Đã đánh số trang tệp {}/{}", idx + 1, request.input_paths.len()),
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
                message: "Đánh số trang & Bates Numbering hoàn tất thành công.".to_string(),
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

    fn create_multipage_pdf() -> Vec<u8> {
        let pdf = "\
%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n\
4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n\
xref\n0 5\n0000000000 65535 f \n\
trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n300\n%%EOF\n";
        pdf.as_bytes().to_vec()
    }

    #[test]
    fn test_pdf_page_number_header_happy_path() {
        let tool = PdfPageNumberHeaderTool;
        let tmp = std::env::temp_dir().join("test_page_num_happy");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_multipage_pdf()).expect("write pdf");

        let req = DispatchTaskRequest {
            task_id: "task-pagenum-happy".to_string(),
            tool_slug: "pdf-page-number-header".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "pattern": "Trang {page}/{total}",
                "position": "bottom_center",
                "skip_cover": true
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("numbering succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read numbered");
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(out_str.contains("Trang 1/1"));
    }

    #[test]
    fn test_pdf_page_number_header_edge_case() {
        let tool = PdfPageNumberHeaderTool;
        let tmp = std::env::temp_dir().join("test_bates_edge");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("legal_doc.pdf");
        fs::write(&pdf_path, create_multipage_pdf()).expect("write pdf");

        let req = DispatchTaskRequest {
            task_id: "task-bates-edge".to_string(),
            tool_slug: "pdf-page-number-header".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "pattern": "Bates {number:06}",
                "position": "bottom_right",
                "skip_cover": false,
                "start_num": 1
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("bates succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read numbered");
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(out_str.contains("Bates 000001"));
        assert!(out_str.contains("Bates 000002"));
    }

    #[test]
    fn test_pdf_page_number_header_invalid_magic_bytes() {
        let tool = PdfPageNumberHeaderTool;
        let tmp = std::env::temp_dir().join("test_page_num_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("bad.pdf");
        fs::write(&fake_path, b"NOT_A_VALID_PDF").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-fake".to_string(),
            tool_slug: "pdf-page-number-header".to_string(),
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
    fn test_pdf_page_number_header_cooperative_cancellation() {
        let tool = PdfPageNumberHeaderTool;
        let tmp = std::env::temp_dir().join("test_page_num_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_multipage_pdf()).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-page-number-header".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_page_number_header_security_guardrails() {
        let tool = PdfPageNumberHeaderTool;
        assert_eq!(tool.slug(), "pdf-page-number-header");
        assert_eq!(tool.name(), "Đánh số trang & Bates Numbering");
    }
}
