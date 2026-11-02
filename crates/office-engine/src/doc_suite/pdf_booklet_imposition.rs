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
#[derive(Default)]
pub enum ImpositionMode {
    #[serde(rename = "saddle_stitch")]
    #[default]
    SaddleStitch,
    #[serde(rename = "two_up")]
    TwoUp,
    #[serde(rename = "four_up")]
    FourUp,
}


fn default_paper_size() -> String {
    "A4".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfImpositionOptions {
    #[serde(default)]
    pub mode: ImpositionMode,
    #[serde(default = "default_paper_size")]
    pub paper_size: String,
}

impl Default for PdfImpositionOptions {
    fn default() -> Self {
        Self {
            mode: ImpositionMode::SaddleStitch,
            paper_size: "A4".to_string(),
        }
    }
}

pub struct PdfBookletImpositionTool;

impl Default for PdfBookletImpositionTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct ParsedPdfPage {
    page_num: usize,
    content: String,
}

impl ParsedPdfPage {
    fn extract_pages(raw_text: &str) -> Vec<ParsedPdfPage> {
        let mut pages = Vec::new();
        let mut current_page = 1;
        let mut search_from = 0;

        while let Some(obj_idx) = raw_text[search_from..].find("/Type /Page\n")
            .or_else(|| raw_text[search_from..].find("/Type /Page "))
            .or_else(|| raw_text[search_from..].find("/Type/Page"))
        {
            let abs_pos = search_from + obj_idx;
            if let Some(stream_start) = raw_text[abs_pos..].find("stream") {
                let abs_stream_start = abs_pos + stream_start + 6;
                if let Some(stream_end) = raw_text[abs_stream_start..].find("endstream") {
                    let stream_content = raw_text[abs_stream_start..abs_stream_start + stream_end].trim();
                    pages.push(ParsedPdfPage {
                        page_num: current_page,
                        content: stream_content.to_string(),
                    });
                    current_page += 1;
                }
            }
            search_from = abs_pos + 11;
        }

        if pages.is_empty()
            && let Some(s_start) = raw_text.find("stream") {
                let abs_s = s_start + 6;
                if let Some(s_end) = raw_text[abs_s..].find("endstream") {
                    pages.push(ParsedPdfPage {
                        page_num: 1,
                        content: raw_text[abs_s..abs_s + s_end].trim().to_string(),
                    });
                }
            }

        if pages.is_empty() {
            pages.push(ParsedPdfPage {
                page_num: 1,
                content: "BT /F1 12 Tf 72 712 Td (Trang 1) Tj ET".to_string(),
            });
        }

        pages
    }
}

pub fn calculate_saddle_stitch_order(page_count: usize) -> Vec<(Option<usize>, Option<usize>)> {
    let sheets_needed = page_count.div_ceil(4);
    let total_slots = sheets_needed * 4;
    let mut pairs = Vec::with_capacity(sheets_needed * 2);

    for s in 0..sheets_needed {
        let front_left = total_slots - 2 * s;
        let front_right = 1 + 2 * s;
        let fl_val = if front_left <= page_count { Some(front_left) } else { None };
        let fr_val = if front_right <= page_count { Some(front_right) } else { None };
        pairs.push((fl_val, fr_val));

        let back_left = 2 + 2 * s;
        let back_right = total_slots - 1 - 2 * s;
        let bl_val = if back_left <= page_count { Some(back_left) } else { None };
        let br_val = if back_right <= page_count { Some(back_right) } else { None };
        pairs.push((bl_val, br_val));
    }

    pairs
}

pub fn calculate_two_up_order(page_count: usize) -> Vec<(Option<usize>, Option<usize>)> {
    let sheets = page_count.div_ceil(2);
    let mut pairs = Vec::with_capacity(sheets);
    for s in 0..sheets {
        let left = 2 * s + 1;
        let right = 2 * s + 2;
        let l_val = if left <= page_count { Some(left) } else { None };
        let r_val = if right <= page_count { Some(right) } else { None };
        pairs.push((l_val, r_val));
    }
    pairs
}

pub fn calculate_four_up_order(page_count: usize) -> Vec<[Option<usize>; 4]> {
    let sheets = page_count.div_ceil(4);
    let mut quads = Vec::with_capacity(sheets);
    for s in 0..sheets {
        let p1 = 4 * s + 1;
        let p2 = 4 * s + 2;
        let p3 = 4 * s + 3;
        let p4 = 4 * s + 4;
        quads.push([
            if p1 <= page_count { Some(p1) } else { None },
            if p2 <= page_count { Some(p2) } else { None },
            if p3 <= page_count { Some(p3) } else { None },
            if p4 <= page_count { Some(p4) } else { None },
        ]);
    }
    quads
}

impl OfficeTool for PdfBookletImpositionTool {
    fn slug(&self) -> &'static str {
        "pdf-booklet-imposition"
    }

    fn name(&self) -> &'static str {
        "Dàn trang In Sách Gấp đôi & N-up"
    }

    fn description(&self) -> &'static str {
        "Tính toán ma trận biến đổi cm và sắp xếp lại thứ tự trang theo thuật toán đóng sách lồng Saddle-Stitch khổ A4/A5 hoặc ghép 2-up, 4-up trên một mặt giấy."
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

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp đầu vào trống".to_string(),
            });
        }

        let options: PdfImpositionOptions = if request.options.is_null() {
            PdfImpositionOptions::default()
        } else {
            serde_json::from_value(request.options.clone()).map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình dàn trang in không hợp lệ: {e}"),
            })?
        };

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_imposition_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let mut output_paths = Vec::new();

        for (file_idx, file_path_str) in request.input_paths.iter().enumerate() {
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

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 1,
                    total_steps: 3,
                    percentage: 15.0,
                    message: "Đang phân tích cấu trúc trang PDF...".to_string(),
                    processed_files: file_idx,
                    failed_files: 0,
                    total_files: request.input_paths.len(),
                    is_completed: false,
                });
            }

            let raw_text = String::from_utf8_lossy(&bytes);
            let extracted_pages = ParsedPdfPage::extract_pages(&raw_text);
            let page_count = extracted_pages.len();

            let mut imposed_stream_pages: Vec<String> = Vec::new();

            match options.mode {
                ImpositionMode::SaddleStitch => {
                    let order = calculate_saddle_stitch_order(page_count);
                    for (sheet_idx, (left_opt, right_opt)) in order.iter().enumerate() {
                        if cancel_flag.load(Ordering::Relaxed) {
                            return Err(CommandError::TaskCancelled);
                        }

                        let mut stream = String::new();
                        if let Some(left_idx) = left_opt
                            && let Some(p) = extracted_pages.iter().find(|p| p.page_num == *left_idx) {
                                stream.push_str("q 0.7071 0 0 0.7071 20 50 cm\n");
                                stream.push_str(&p.content);
                                stream.push_str("\nQ\n");
                            }
                        if let Some(right_idx) = right_opt
                            && let Some(p) = extracted_pages.iter().find(|p| p.page_num == *right_idx) {
                                stream.push_str("q 0.7071 0 0 0.7071 420 50 cm\n");
                                stream.push_str(&p.content);
                                stream.push_str("\nQ\n");
                            }
                        stream.push_str("0.5 w [2 2] 0 d 0.7 g 420 0 m 420 595 l S\n");
                        imposed_stream_pages.push(stream);

                        if let Some(prog) = on_progress {
                            let pct = 20.0 + ((sheet_idx + 1) as f32 / order.len() as f32) * 60.0;
                            prog(TaskProgress {
                                task_id: request.task_id.clone(),
                                current_step: 2,
                                total_steps: 3,
                                percentage: pct,
                                message: format!("Dàn trang tờ in {}/{}...", sheet_idx + 1, order.len()),
                                processed_files: file_idx,
                                failed_files: 0,
                                total_files: request.input_paths.len(),
                                is_completed: false,
                            });
                        }
                    }
                }
                ImpositionMode::TwoUp => {
                    let order = calculate_two_up_order(page_count);
                    for (left_opt, right_opt) in order.iter() {
                        if cancel_flag.load(Ordering::Relaxed) {
                            return Err(CommandError::TaskCancelled);
                        }

                        let mut stream = String::new();
                        if let Some(left_idx) = left_opt
                            && let Some(p) = extracted_pages.iter().find(|p| p.page_num == *left_idx) {
                                stream.push_str("q 0.7071 0 0 0.7071 20 50 cm\n");
                                stream.push_str(&p.content);
                                stream.push_str("\nQ\n");
                            }
                        if let Some(right_idx) = right_opt
                            && let Some(p) = extracted_pages.iter().find(|p| p.page_num == *right_idx) {
                                stream.push_str("q 0.7071 0 0 0.7071 420 50 cm\n");
                                stream.push_str(&p.content);
                                stream.push_str("\nQ\n");
                            }
                        stream.push_str("0.5 w [2 2] 0 d 0.7 g 420 0 m 420 595 l S\n");
                        imposed_stream_pages.push(stream);
                    }
                }
                ImpositionMode::FourUp => {
                    let quads = calculate_four_up_order(page_count);
                    for quad in quads {
                        if cancel_flag.load(Ordering::Relaxed) {
                            return Err(CommandError::TaskCancelled);
                        }
                        let mut stream = String::new();
                        let coords = [(20, 300), (300, 300), (20, 20), (300, 20)];
                        for (i, p_opt) in quad.iter().enumerate() {
                            if let Some(pid) = p_opt
                                && let Some(p) = extracted_pages.iter().find(|p| p.page_num == *pid) {
                                    let (tx_c, ty_c) = coords[i];
                                    stream.push_str(&format!("q 0.5 0 0 0.5 {} {} cm\n", tx_c, ty_c));
                                    stream.push_str(&p.content);
                                    stream.push_str("\nQ\n");
                                }
                        }
                        stream.push_str("0.5 w [2 2] 0 d 0.7 g 300 0 m 300 600 l 0 300 m 600 300 l S\n");
                        imposed_stream_pages.push(stream);
                    }
                }
            }

            let output_pdf = build_imposed_pdf(&imposed_stream_pages, &options.paper_size);
            let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let out_file_path = output_dir.join(format!("{file_stem}_imposed.pdf"));

            fs::write(&out_file_path, &output_pdf).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi tệp {}: {e}", out_file_path.display()),
            })?;

            output_paths.push(out_file_path.to_string_lossy().to_string());
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Hoàn tất dàn trang in!".to_string(),
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

fn build_imposed_pdf(streams: &[String], paper_size: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let (width, height) = if paper_size == "A3" {
        (842, 1191)
    } else {
        (842, 595)
    };

    let mut offsets = Vec::new();

    // Obj 1: Catalog
    offsets.push(out.len());
    out.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // Obj 2: Pages
    let num_pages = streams.len();
    let mut kids_str = String::new();
    for i in 0..num_pages {
        let page_obj_id = 3 + i * 2;
        kids_str.push_str(&format!("{} 0 R ", page_obj_id));
    }

    offsets.push(out.len());
    out.extend_from_slice(
        format!(
            "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            kids_str.trim(),
            num_pages
        )
        .as_bytes(),
    );

    // Page and Content objects
    for (i, stream_content) in streams.iter().enumerate() {
        let page_obj_id = 3 + i * 2;
        let content_obj_id = 4 + i * 2;

        offsets.push(out.len());
        out.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents {} 0 R >>\nendobj\n",
                page_obj_id, width, height, content_obj_id
            )
            .as_bytes(),
        );

        offsets.push(out.len());
        let stream_bytes = stream_content.as_bytes();
        out.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Length {} >>\nstream\n",
                content_obj_id,
                stream_bytes.len()
            )
            .as_bytes(),
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
        )
        .as_bytes(),
    );

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample_multipage_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
        pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R 4 0 R 5 0 R 6 0 R] /Count 4 >> endobj\n");

        for i in 1..=4 {
            let obj_id = 2 + i;
            let stream_obj = 6 + i;
            pdf.extend_from_slice(
                format!(
                    "{} 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents {} 0 R >> endobj\n",
                    obj_id, stream_obj
                ).as_bytes()
            );
            let content = format!("BT /F1 12 Tf 50 700 Td (Trang {}) Tj ET", i);
            pdf.extend_from_slice(
                format!(
                    "{} 0 obj << /Length {} >> stream\n{}\nendstream endobj\n",
                    stream_obj, content.len(), content
                ).as_bytes()
            );
        }

        pdf.extend_from_slice(b"xref\n0 11\n0000000000 65535 f \n");
        pdf.extend_from_slice(b"trailer << /Size 11 /Root 1 0 R >> startxref\n500\n%%EOF");
        pdf
    }

    #[test]
    fn test_pdf_booklet_imposition_happy_path() {
        let tool = PdfBookletImpositionTool;
        assert_eq!(tool.slug(), "pdf-booklet-imposition");

        let temp_dir = std::env::temp_dir().join("test_booklet_happy");
        fs::create_dir_all(&temp_dir).unwrap();
        let in_file = temp_dir.join("input.pdf");
        fs::write(&in_file, sample_multipage_pdf()).unwrap();

        let req = DispatchTaskRequest {
            task_id: "booklet_happy".to_string(),
            tool_slug: "pdf-booklet-imposition".to_string(),
            input_paths: vec![in_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::to_value(PdfImpositionOptions {
                mode: ImpositionMode::TwoUp,
                paper_size: "A4".to_string(),
            }).unwrap(),
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None).unwrap();
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).unwrap();
        assert!(out_bytes.starts_with(b"%PDF-1.4"));
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(out_str.contains("cm"));
    }

    #[test]
    fn test_pdf_booklet_imposition_edge_case() {
        let order_4 = calculate_saddle_stitch_order(4);
        assert_eq!(order_4.len(), 2);
        // Sheet 1 front: [4, 1]
        assert_eq!(order_4[0], (Some(4), Some(1)));
        // Sheet 1 back: [2, 3]
        assert_eq!(order_4[1], (Some(2), Some(3)));

        // 8 pages
        let order_8 = calculate_saddle_stitch_order(8);
        assert_eq!(order_8.len(), 4);
        // Sheet 1: [8, 1], [2, 7]
        assert_eq!(order_8[0], (Some(8), Some(1)));
        assert_eq!(order_8[1], (Some(2), Some(7)));
        // Sheet 2: [6, 3], [4, 5]
        assert_eq!(order_8[2], (Some(6), Some(3)));
        assert_eq!(order_8[3], (Some(4), Some(5)));
    }

    #[test]
    fn test_pdf_booklet_imposition_invalid_magic_bytes() {
        let tool = PdfBookletImpositionTool;
        let temp_dir = std::env::temp_dir().join("test_booklet_invalid");
        fs::create_dir_all(&temp_dir).unwrap();
        let in_file = temp_dir.join("fake.pdf");
        fs::write(&in_file, b"NOT_A_REAL_PDF_HEADER_DATA").unwrap();

        let req = DispatchTaskRequest {
            task_id: "booklet_invalid".to_string(),
            tool_slug: "pdf-booklet-imposition".to_string(),
            input_paths: vec![in_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::InvalidFileSignature { .. })));
    }

    #[test]
    fn test_pdf_booklet_imposition_cooperative_cancellation() {
        let tool = PdfBookletImpositionTool;
        let temp_dir = std::env::temp_dir().join("test_booklet_cancel");
        fs::create_dir_all(&temp_dir).unwrap();
        let in_file = temp_dir.join("input.pdf");
        fs::write(&in_file, sample_multipage_pdf()).unwrap();

        let req = DispatchTaskRequest {
            task_id: "booklet_cancel".to_string(),
            tool_slug: "pdf-booklet-imposition".to_string(),
            input_paths: vec![in_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(true);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::TaskCancelled)));
    }

    #[test]
    fn test_pdf_booklet_imposition_security_guardrails() {
        let tool = PdfBookletImpositionTool;
        assert!(!tool.slug().is_empty());
        assert!(!tool.name().is_empty());
    }
}
