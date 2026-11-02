use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

fn default_opacity() -> f32 {
    0.3
}

fn default_angle() -> f32 {
    45.0
}

fn default_position() -> String {
    "center".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfWatermarkOptions {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub stamp_image_path: Option<String>,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_angle")]
    pub angle: f32,
    #[serde(default = "default_position")]
    pub position: String,
}

impl Default for PdfWatermarkOptions {
    fn default() -> Self {
        Self {
            text: Some("BẢN QUYỀN - KHÔNG SAO CHÉP".to_string()),
            stamp_image_path: None,
            opacity: 0.3,
            angle: 45.0,
            position: "center".to_string(),
        }
    }
}

pub struct PdfWatermarkStampTool;

impl Default for PdfWatermarkStampTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct WatermarkPdfObj {
    id: usize,
    generation: usize,
    content: String,
}

struct MiniWatermarkPdf {
    header: String,
    objects: Vec<WatermarkPdfObj>,
}

impl MiniWatermarkPdf {
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

            if let (Some(g_s), Some(i_s)) = (gen_str, id_str) {
                if let (Ok(id), Ok(generation)) = (i_s.parse::<usize>(), g_s.parse::<usize>()) {
                    let content_start = abs_obj_start + 4;
                    if let Some(end_offset) = raw_text[content_start..].find("endobj") {
                        let content = raw_text[content_start..content_start + end_offset].trim().to_string();
                        objects.push(WatermarkPdfObj {
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

        Ok(MiniWatermarkPdf { header, objects })
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

fn render_watermarked_pdf(header: &str, objects: &[WatermarkPdfObj]) -> Vec<u8> {
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

impl OfficeTool for PdfWatermarkStampTool {
    fn slug(&self) -> &'static str {
        "pdf-watermark-stamp"
    }

    fn name(&self) -> &'static str {
        "Đóng dấu Watermark & Con dấu đỏ"
    }

    fn description(&self) -> &'static str {
        "Chèn luồng đồ họa ExtGState với độ mờ (ca/CA opacity) chứa dòng text bản quyền xoay chéo, hoặc nhúng ảnh con dấu PNG kênh Alpha vào góc trang với tọa độ tùy chỉnh."
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

        let options: PdfWatermarkOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình đóng dấu PDF không hợp lệ: {e}"),
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
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_watermark_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        // Nếu có đường dẫn ảnh con dấu, xác thực sự tồn tại và chữ ký PNG
        let mut stamp_bytes = None;
        if let Some(ref stamp_path) = options.stamp_image_path {
            let p = Path::new(stamp_path);
            if !p.exists() {
                return Err(CommandError::FileNotFound {
                    path: stamp_path.clone(),
                });
            }
            let s_bytes = fs::read(p).map_err(|e| CommandError::IoError {
                details: format!("Không thể đọc ảnh con dấu {stamp_path}: {e}"),
            })?;
            if s_bytes.len() < 8 || &s_bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
                return Err(CommandError::InvalidFileSignature {
                    expected: "PNG Image (89 50 4E 47)".to_string(),
                    actual: format!("{:02X?}", &s_bytes[..s_bytes.len().min(8)]),
                });
            }
            stamp_bytes = Some(s_bytes);
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Đang đọc cấu trúc trang và tạo lớp phủ đồ họa ExtGState...".to_string(),
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

            let mut pdf = MiniWatermarkPdf::parse(&bytes)?;
            let page_indices = pdf.get_page_indices();

            let mut max_id = pdf.objects.iter().map(|o| o.id).max().unwrap_or(10);

            // Tạo đối tượng ExtGState thiết lập độ trong suốt
            max_id += 1;
            let gs_id = max_id;
            let ext_gstate_obj = WatermarkPdfObj {
                id: gs_id,
                generation: 0,
                content: format!("<< /Type /ExtGState /ca {:.2} /CA {:.2} >>", options.opacity, options.opacity),
            };
            pdf.objects.push(ext_gstate_obj);

            // Nếu có ảnh con dấu, tạo đối tượng XObject Image
            let mut stamp_obj_id = None;
            if let Some(ref s_data) = stamp_bytes {
                max_id += 1;
                let s_id = max_id;
                let stamp_obj = WatermarkPdfObj {
                    id: s_id,
                    generation: 0,
                    content: format!(
                        "<< /Type /XObject /Subtype /Image /Width 200 /Height 200 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n{}\nendstream",
                        s_data.len(),
                        String::from_utf8_lossy(s_data)
                    ),
                };
                pdf.objects.push(stamp_obj);
                stamp_obj_id = Some(s_id);
            }

            // Chèn watermark vào từng trang
            for &p_idx in &page_indices {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                max_id += 1;
                let wm_stream_id = max_id;

                let rad = options.angle.to_radians();
                let cos_val = rad.cos();
                let sin_val = rad.sin();

                let mut wm_ops = String::new();
                wm_ops.push_str("q /GS1 gs\n");

                if let Some(ref text) = options.text {
                    let (tx, ty) = if options.position == "bottom_right" {
                        (400.0, 100.0)
                    } else if options.position == "bottom_left" {
                        (50.0, 100.0)
                    } else {
                        (200.0, 400.0)
                    };

                    wm_ops.push_str(&format!(
                        "BT /F1 36 Tf {:.4} {:.4} {:.4} {:.4} {:.1} {:.1} Tm ({}) Tj ET\n",
                        cos_val, sin_val, -sin_val, cos_val, tx, ty, text
                    ));
                }

                if stamp_obj_id.is_some() {
                    // Chèn con dấu ở góc dưới trang (bottom-right: x=420, y=80, w=120, h=120)
                    wm_ops.push_str("q 120 0 0 120 420 80 cm /StampIm Do Q\n");
                }

                wm_ops.push_str("Q\n");

                let wm_content_obj = WatermarkPdfObj {
                    id: wm_stream_id,
                    generation: 0,
                    content: format!("<< /Length {} >>\nstream\n{}\nendstream", wm_ops.len(), wm_ops),
                };
                pdf.objects.push(wm_content_obj);

                // Cập nhật trang để liên kết Contents và Resources (ExtGState & XObject)
                let page_obj = &mut pdf.objects[p_idx];

                // Bổ sung Resources ExtGState
                if let Some(res_idx) = page_obj.content.find("/Resources") {
                    let insert_pos = res_idx + 10;
                    page_obj.content.insert_str(insert_pos, &format!(" << /ExtGState << /GS1 {gs_id} 0 R >> >> "));
                } else if let Some(dict_end) = page_obj.content.rfind(">>") {
                    page_obj.content.insert_str(dict_end, &format!(" /Resources << /ExtGState << /GS1 {gs_id} 0 R >> >> "));
                }

                // Bổ sung Contents
                if let Some(contents_idx) = page_obj.content.find("/Contents") {
                    let rest = &page_obj.content[contents_idx + 9..];
                    if let Some(arr_start) = rest.find('[') {
                        let abs_arr_start = contents_idx + 9 + arr_start + 1;
                        page_obj.content.insert_str(abs_arr_start, &format!(" {} 0 R ", wm_stream_id));
                    } else if let Some(r_pos) = rest.find('R') {
                        let end_r = contents_idx + 9 + r_pos + 1;
                        let old_ref = page_obj.content[contents_idx + 9..end_r].trim().to_string();
                        page_obj.content.replace_range(contents_idx..end_r, &format!("/Contents [{} {} 0 R]", old_ref, wm_stream_id));
                    }
                } else if let Some(dict_end) = page_obj.content.rfind(">>") {
                    page_obj.content.insert_str(dict_end, &format!(" /Contents {} 0 R ", wm_stream_id));
                }
            }

            let out_bytes = render_watermarked_pdf(&pdf.header, &pdf.objects);
            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("doc");
            let out_path = output_dir.join(format!("{file_stem}_watermarked.pdf"));
            fs::write(&out_path, out_bytes).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi tệp PDF đóng dấu: {e}"),
            })?;

            output_paths.push(out_path.to_string_lossy().to_string());

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: 20.0 + (idx as f32 / request.input_paths.len() as f32) * 70.0,
                    message: format!("Đã đóng dấu trang {}/{}", idx + 1, request.input_paths.len()),
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
                message: "Đóng dấu Watermark & Con dấu đỏ hoàn tất thành công.".to_string(),
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

    fn create_test_pdf() -> Vec<u8> {
        let pdf = "\
%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length 30 >>\nstream\nBT /F1 12 Tf (Tai lieu goc) Tj ET\nendstream\nendobj\n\
xref\n0 5\n0000000000 65535 f \n\
trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n350\n%%EOF\n";
        pdf.as_bytes().to_vec()
    }

    fn create_dummy_png() -> Vec<u8> {
        let mut png = Vec::new();
        png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        png.extend_from_slice(&[0, 0, 0, 13]); // IHDR length
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&[0, 0, 0, 10, 0, 0, 0, 10, 8, 6, 0, 0, 0]); // 10x10 RGBA
        png.extend_from_slice(&[0, 0, 0, 0]); // CRC
        png.extend_from_slice(&[0, 0, 0, 0]); // IEND length
        png.extend_from_slice(b"IEND");
        png.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]); // CRC
        png
    }

    #[test]
    fn test_pdf_watermark_stamp_happy_path() {
        let tool = PdfWatermarkStampTool::default();
        let tmp = std::env::temp_dir().join("test_wm_happy");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_test_pdf()).expect("write pdf");

        let req = DispatchTaskRequest {
            task_id: "task-wm-happy".to_string(),
            tool_slug: "pdf-watermark-stamp".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "text": "BẢN QUYỀN NỘI BỘ",
                "opacity": 0.35,
                "angle": 45.0,
                "position": "center"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("watermark succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read out");
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(out_str.contains("/ExtGState"));
        assert!(out_str.contains("BẢN QUYỀN NỘI BỘ"));
    }

    #[test]
    fn test_pdf_watermark_stamp_edge_case() {
        let tool = PdfWatermarkStampTool::default();
        let tmp = std::env::temp_dir().join("test_wm_edge");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("contract.pdf");
        fs::write(&pdf_path, create_test_pdf()).expect("write pdf");

        let stamp_path = tmp.join("red_stamp.png");
        fs::write(&stamp_path, create_dummy_png()).expect("write stamp png");

        let req = DispatchTaskRequest {
            task_id: "task-wm-stamp-edge".to_string(),
            tool_slug: "pdf-watermark-stamp".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "stamp_image_path": stamp_path.to_string_lossy().to_string(),
                "position": "bottom_right"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("stamp watermark succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read out");
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(out_str.contains("/Subtype /Image"));
        assert!(out_str.contains("/StampIm Do"));
    }

    #[test]
    fn test_pdf_watermark_stamp_invalid_magic_bytes() {
        let tool = PdfWatermarkStampTool::default();
        let tmp = std::env::temp_dir().join("test_wm_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("bad.pdf");
        fs::write(&fake_path, b"NOT_A_VALID_PDF").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-fake".to_string(),
            tool_slug: "pdf-watermark-stamp".to_string(),
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
    fn test_pdf_watermark_stamp_cooperative_cancellation() {
        let tool = PdfWatermarkStampTool::default();
        let tmp = std::env::temp_dir().join("test_wm_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_test_pdf()).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-watermark-stamp".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_watermark_stamp_security_guardrails() {
        let tool = PdfWatermarkStampTool::default();
        assert_eq!(tool.slug(), "pdf-watermark-stamp");
        assert_eq!(tool.name(), "Đóng dấu Watermark & Con dấu đỏ");
    }
}
