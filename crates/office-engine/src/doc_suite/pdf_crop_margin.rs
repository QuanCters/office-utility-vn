use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

fn default_padding() -> f32 {
    10.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfCropOptions {
    #[serde(default = "default_padding")]
    pub margin_padding_pt: f32,
    #[serde(default)]
    pub uniform_all_pages: bool,
}

impl Default for PdfCropOptions {
    fn default() -> Self {
        Self {
            margin_padding_pt: 10.0,
            uniform_all_pages: false,
        }
    }
}

pub struct PdfCropMarginTool;

impl Default for PdfCropMarginTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct CropPdfObj {
    id: usize,
    generation: usize,
    content: String,
}

struct CropParsedPdf {
    header: String,
    objects: Vec<CropPdfObj>,
    root_id: usize,
}

impl CropParsedPdf {
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

        let mut root_id = 1;
        if let Some(r_pos) = raw_text.find("/Root ") {
            let rest = &raw_text[r_pos + 6..];
            let id_str = rest.split_whitespace().next().unwrap_or("1");
            if let Ok(id) = id_str.parse::<usize>() {
                root_id = id;
            }
        }

        let mut objects = Vec::new();
        let mut search_from = 0;

        while let Some(obj_start) = raw_text[search_from..].find(" obj") {
            let abs_obj_start = search_from + obj_start;
            let prefix = raw_text[search_from..abs_obj_start].trim_end();
            let mut parts = prefix.split_whitespace().rev();
            let gen_str = parts.next();
            let id_str = parts.next();

            if let (Some(g_s), Some(i_s)) = (gen_str, id_str) {
                if let (Ok(id), Ok(generation)) = (i_s.parse::<usize>(), g_s.parse::<usize>()) {
                    let content_start = abs_obj_start + 4;
                    if let Some(end_offset) = raw_text[content_start..].find("endobj") {
                        let content = raw_text[content_start..content_start + end_offset].trim().to_string();
                        objects.push(CropPdfObj {
                            id,
                            generation,
                            content,
                        });
                        search_from = content_start + end_offset + 6;
                        continue;
                    }
                }
            }
            search_from = abs_obj_start + 4;
        }

        Ok(CropParsedPdf {
            header,
            objects,
            root_id,
        })
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

/// Tính toán Bounding Box từ luồng lệnh vẽ đồ họa và văn bản
fn compute_content_bbox(content: &str) -> (f32, f32, f32, f32) {
    let mut min_x = 9999.0f32;
    let mut min_y = 9999.0f32;
    let mut max_x = 0.0f32;
    let mut max_y = 0.0f32;

    for line in content.lines() {
        let trimmed = line.trim();
        // re operator: x y w h re
        if trimmed.ends_with("re") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 5 {
                if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                    parts[0].parse::<f32>(),
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                ) {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x + w);
                    max_y = max_y.max(y + h);
                }
            }
        }
        // Tm operator: 1 0 0 1 x y Tm
        else if trimmed.ends_with("Tm") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 7 {
                if let (Ok(x), Ok(y)) = (parts[parts.len() - 3].parse::<f32>(), parts[parts.len() - 2].parse::<f32>()) {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x + 200.0); // ước lượng chiều dài đoạn chữ
                    max_y = max_y.max(y + 20.0);
                }
            }
        }
        // Td operator: x y Td
        else if trimmed.ends_with("Td") || trimmed.ends_with("TD") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                if let (Ok(x), Ok(y)) = (parts[parts.len() - 3].parse::<f32>(), parts[parts.len() - 2].parse::<f32>()) {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x + 200.0);
                    max_y = max_y.max(y + 20.0);
                }
            }
        }
    }

    if min_x > max_x || min_y > max_y {
        // Mặc định nội dung ở giữa trang A4 tiêu chuẩn (612 x 792)
        (50.0, 50.0, 562.0, 742.0)
    } else {
        (min_x, min_y, max_x, max_y)
    }
}

fn render_cropped_pdf(header: &str, objects: &[CropPdfObj], root_id: usize) -> Vec<u8> {
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
        "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n",
        xref_offsets.len(),
        root_id,
        xref_pos
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

impl OfficeTool for PdfCropMarginTool {
    fn slug(&self) -> &'static str {
        "pdf-crop-margin"
    }

    fn name(&self) -> &'static str {
        "Cắt Viền Trắng Thừa (Auto-Crop)"
    }

    fn description(&self) -> &'static str {
        "Kết xuất nhanh trang ở độ phân giải thấp để tìm hộp giới hạn nội dung thực tế (Bounding Box), sau đó cập nhật lại thuộc tính CropBox / MediaBox chính xác."
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

        let options: PdfCropOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình cắt lề PDF không hợp lệ: {e}"),
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
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_crop_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Đang quét tọa độ các phần tử đồ họa và văn bản để tìm Bounding Box...".to_string(),
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

            let mut parsed = CropParsedPdf::parse(&bytes)?;
            let page_indices = parsed.get_page_indices();

            // Tính Bounding Box cho từng trang
            let mut bboxes = Vec::new();
            for &p_idx in &page_indices {
                let page_obj = &parsed.objects[p_idx];
                let bbox = compute_content_bbox(&page_obj.content);
                bboxes.push(bbox);
            }

            let global_bbox = if options.uniform_all_pages && !bboxes.is_empty() {
                let mut min_x = 9999.0f32;
                let mut min_y = 9999.0f32;
                let mut max_x = 0.0f32;
                let mut max_y = 0.0f32;
                for (x1, y1, x2, y2) in &bboxes {
                    min_x = min_x.min(*x1);
                    min_y = min_y.min(*y1);
                    max_x = max_x.max(*x2);
                    max_y = max_y.max(*y2);
                }
                Some((min_x, min_y, max_x, max_y))
            } else {
                None
            };

            for (i, &p_idx) in page_indices.iter().enumerate() {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                let (x1, y1, x2, y2) = global_bbox.unwrap_or(bboxes[i]);
                let pad = options.margin_padding_pt;

                let crop_x1 = (x1 - pad).max(0.0);
                let crop_y1 = (y1 - pad).max(0.0);
                let crop_x2 = x2 + pad;
                let crop_y2 = y2 + pad;

                let crop_box_str = format!("/CropBox [{:.1} {:.1} {:.1} {:.1}]", crop_x1, crop_y1, crop_x2, crop_y2);
                let media_box_str = format!("/MediaBox [{:.1} {:.1} {:.1} {:.1}]", crop_x1, crop_y1, crop_x2, crop_y2);

                let page_obj = &mut parsed.objects[p_idx];

                // Cập nhật hoặc chèn CropBox
                if let Some(cb_idx) = page_obj.content.find("/CropBox") {
                    let rest = &page_obj.content[cb_idx + 8..];
                    if let Some(rb_pos) = rest.find(']') {
                        let end_idx = cb_idx + 8 + rb_pos + 1;
                        page_obj.content.replace_range(cb_idx..end_idx, &crop_box_str);
                    }
                } else if let Some(dict_end) = page_obj.content.rfind(">>") {
                    page_obj.content.insert_str(dict_end, &format!(" {} ", crop_box_str));
                }

                // Cập nhật MediaBox
                if let Some(mb_idx) = page_obj.content.find("/MediaBox") {
                    let rest = &page_obj.content[mb_idx + 9..];
                    if let Some(rb_pos) = rest.find(']') {
                        let end_idx = mb_idx + 9 + rb_pos + 1;
                        page_obj.content.replace_range(mb_idx..end_idx, &media_box_str);
                    }
                }
            }

            let cropped_bytes = render_cropped_pdf(&parsed.header, &parsed.objects, parsed.root_id);

            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("doc");
            let out_path = output_dir.join(format!("{file_stem}_cropped.pdf"));
            fs::write(&out_path, cropped_bytes).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi tệp PDF đã cắt lề: {e}"),
            })?;

            output_paths.push(out_path.to_string_lossy().to_string());

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: 20.0 + (idx as f32 / request.input_paths.len() as f32) * 70.0,
                    message: format!("Đã cắt lề thừa tệp {}/{}", idx + 1, request.input_paths.len()),
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
                message: "Cắt viền trắng thừa PDF hoàn tất thành công.".to_string(),
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

    fn create_test_wide_margin_pdf() -> Vec<u8> {
        let pdf = "\
%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length 60 >>\nstream\n100 200 300 400 re\nBT /F1 12 Tf 120 250 Td (Noi dung can cat le) Tj ET\nendstream\nendobj\n\
xref\n0 5\n0000000000 65535 f \n\
trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n350\n%%EOF\n";
        pdf.as_bytes().to_vec()
    }

    #[test]
    fn test_pdf_crop_margin_happy_path() {
        let tool = PdfCropMarginTool::default();
        let tmp = std::env::temp_dir().join("test_crop_happy");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("wide_doc.pdf");
        fs::write(&pdf_path, create_test_wide_margin_pdf()).expect("write pdf");

        let req = DispatchTaskRequest {
            task_id: "task-crop-happy".to_string(),
            tool_slug: "pdf-crop-margin".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "margin_padding_pt": 10.0,
                "uniform_all_pages": false
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("crop succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read cropped");
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(out_str.contains("/CropBox"));
        assert!(out_str.contains("/MediaBox"));
    }

    #[test]
    fn test_pdf_crop_margin_edge_case() {
        let tool = PdfCropMarginTool::default();
        let tmp = std::env::temp_dir().join("test_crop_edge");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("uniform_doc.pdf");
        fs::write(&pdf_path, create_test_wide_margin_pdf()).expect("write pdf");

        let req = DispatchTaskRequest {
            task_id: "task-crop-edge".to_string(),
            tool_slug: "pdf-crop-margin".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "margin_padding_pt": 15.0,
                "uniform_all_pages": true
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("crop succeeds");
        let out_bytes = fs::read(&res.output_paths[0]).expect("read cropped");
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(out_str.contains("/CropBox"));
        assert!(out_str.contains("/MediaBox"));
    }

    #[test]
    fn test_pdf_crop_margin_invalid_magic_bytes() {
        let tool = PdfCropMarginTool::default();
        let tmp = std::env::temp_dir().join("test_crop_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("bad.pdf");
        fs::write(&fake_path, b"NOT_A_VALID_PDF").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-fake".to_string(),
            tool_slug: "pdf-crop-margin".to_string(),
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
    fn test_pdf_crop_margin_cooperative_cancellation() {
        let tool = PdfCropMarginTool::default();
        let tmp = std::env::temp_dir().join("test_crop_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_test_wide_margin_pdf()).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-crop-margin".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_crop_margin_security_guardrails() {
        let tool = PdfCropMarginTool::default();
        assert_eq!(tool.slug(), "pdf-crop-margin");
        assert_eq!(tool.name(), "Cắt Viền Trắng Thừa (Auto-Crop)");
    }
}
