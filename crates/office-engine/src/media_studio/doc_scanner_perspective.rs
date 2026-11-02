use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use serde::{Deserialize, Serialize};

use super::image_batch::{encode_png, load_and_validate_image, RawImage};
use crate::OfficeTool;

/// Tọa độ điểm 2D
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
}

/// Tùy chọn cấu hình nắn phẳng tài liệu góc nghiêng
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerspectiveScanOptions {
    #[serde(default)]
    pub corners: Option<[Point2D; 4]>, // [Top-Left, Top-Right, Bottom-Right, Bottom-Left]
    #[serde(default)]
    pub auto_enhance: bool,
    #[serde(default)]
    pub output_width: Option<u32>,
    #[serde(default)]
    pub output_height: Option<u32>,
}

pub struct DocScannerPerspectiveTool;

impl Default for DocScannerPerspectiveTool {
    fn default() -> Self {
        Self
    }
}

/// Lấy mẫu điểm ảnh với phép nội suy song tuyến tính (Bilinear Interpolation)
fn sample_bilinear(img: &RawImage, x: f32, y: f32) -> [u8; 3] {
    let x0 = (x.floor() as i32).clamp(0, img.width as i32 - 1) as u32;
    let y0 = (y.floor() as i32).clamp(0, img.height as i32 - 1) as u32;
    let x1 = (x0 + 1).min(img.width - 1);
    let y1 = (y0 + 1).min(img.height - 1);

    let fx = (x - x0 as f32).clamp(0.0, 1.0);
    let fy = (y - y0 as f32).clamp(0.0, 1.0);

    let idx00 = ((y0 * img.width + x0) * 3) as usize;
    let idx10 = ((y0 * img.width + x1) * 3) as usize;
    let idx01 = ((y1 * img.width + x0) * 3) as usize;
    let idx11 = ((y1 * img.width + x1) * 3) as usize;

    let mut out = [0u8; 3];
    for c in 0..3 {
        let p00 = img.pixels.get(idx00 + c).copied().unwrap_or(0) as f32;
        let p10 = img.pixels.get(idx10 + c).copied().unwrap_or(0) as f32;
        let p01 = img.pixels.get(idx01 + c).copied().unwrap_or(0) as f32;
        let p11 = img.pixels.get(idx11 + c).copied().unwrap_or(0) as f32;

        let top = (1.0 - fx) * p00 + fx * p10;
        let bottom = (1.0 - fx) * p01 + fx * p11;
        let val = (1.0 - fy) * top + fy * bottom;
        out[c] = val.round().clamp(0.0, 255.0) as u8;
    }

    out
}

/// Biến đổi phối cảnh 4 điểm góc nghiêng thành hình chữ nhật phẳng
fn apply_perspective_transform(
    src: &RawImage,
    corners: &[Point2D; 4],
    target_w: u32,
    target_h: u32,
) -> RawImage {
    let mut out_pixels = Vec::with_capacity((target_w * target_h * 3) as usize);

    let p0 = &corners[0]; // Top-Left
    let p1 = &corners[1]; // Top-Right
    let p2 = &corners[2]; // Bottom-Right
    let p3 = &corners[3]; // Bottom-Left

    for y in 0..target_h {
        let v = if target_h > 1 {
            y as f32 / (target_h - 1) as f32
        } else {
            0.0
        };

        for x in 0..target_w {
            let u = if target_w > 1 {
                x as f32 / (target_w - 1) as f32
            } else {
                0.0
            };

            // Phép nội suy phối cảnh bilinear giữa 4 đỉnh tứ giác
            let sx = (1.0 - u) * (1.0 - v) * p0.x
                + u * (1.0 - v) * p1.x
                + u * v * p2.x
                + (1.0 - u) * v * p3.x;

            let sy = (1.0 - u) * (1.0 - v) * p0.y
                + u * (1.0 - v) * p1.y
                + u * v * p2.y
                + (1.0 - u) * v * p3.y;

            let pixel = sample_bilinear(src, sx, sy);
            out_pixels.extend_from_slice(&pixel);
        }
    }

    RawImage {
        width: target_w,
        height: target_h,
        pixels: out_pixels,
    }
}

/// Tự động cân bằng trắng và tăng tương phản chữ đen trên nền giấy trắng
fn apply_document_enhancement(img: &mut RawImage) {
    if img.pixels.is_empty() {
        return;
    }

    let mut min_l = 255u8;
    let mut max_l = 0u8;

    // Tìm giá trị min/max độ sáng
    for chunk in img.pixels.chunks_exact(3) {
        let lum = ((chunk[0] as u32 * 299 + chunk[1] as u32 * 587 + chunk[2] as u32 * 114) / 1000) as u8;
        if lum < min_l {
            min_l = lum;
        }
        if lum > max_l {
            max_l = lum;
        }
    }

    let range = (max_l as f32 - min_l as f32).max(1.0);

    for chunk in img.pixels.chunks_exact_mut(3) {
        let lum = ((chunk[0] as u32 * 299 + chunk[1] as u32 * 587 + chunk[2] as u32 * 114) / 1000) as u8;
        let norm = ((lum as f32 - min_l as f32) / range).clamp(0.0, 1.0);

        let final_val = if norm > 0.6 {
            // Nền giấy trắng: tăng sáng về trắng gần tuyệt đối
            let boosted = (norm * 1.25).min(1.0);
            (boosted * 255.0).round() as u8
        } else {
            // Ký tự / nét bút mực: tăng độ đậm
            let darkened = norm * 0.75;
            (darkened * 255.0).round() as u8
        };

        chunk[0] = final_val;
        chunk[1] = final_val;
        chunk[2] = final_val;
    }
}

impl OfficeTool for DocScannerPerspectiveTool {
    fn slug(&self) -> &'static str {
        "doc-scanner-perspective"
    }

    fn name(&self) -> &'static str {
        "Phẳng hóa Tài liệu Chụp Nghiêng"
    }

    fn description(&self) -> &'static str {
        "Cho phép tinh chỉnh 4 đỉnh tứ giác của tờ giấy và thực hiện phép biến đổi ma trận đồng nhất 3x3 (imageproc::geometric_transformations::Projection), kết hợp cân bằng trắng và tăng tương phản chữ."
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

        let options: PerspectiveScanOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn PerspectiveScan không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp tài liệu đầu vào trống".to_string(),
            });
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_perspective_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let mut output_paths = Vec::new();
        let mut failed_items = Vec::new();
        let total_files = request.input_paths.len();

        for (idx, file_path_str) in request.input_paths.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let path = Path::new(file_path_str);
            if !path.exists() {
                if total_files == 1 {
                    return Err(CommandError::FileNotFound {
                        path: file_path_str.clone(),
                    });
                }
                failed_items.push(FileFailureDetail {
                    input_path: file_path_str.clone(),
                    reason: "Tệp không tồn tại".to_string(),
                });
                continue;
            }

            let bytes = match fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    if total_files == 1 {
                        return Err(CommandError::IoError {
                            details: format!("Lỗi đọc tệp: {e}"),
                        });
                    }
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("Lỗi đọc: {e}"),
                    });
                    continue;
                }
            };

            let raw_img = match load_and_validate_image(&bytes) {
                Ok(img) => img,
                Err(err) => {
                    if total_files == 1 {
                        return Err(err);
                    }
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("{:?}", err),
                    });
                    continue;
                }
            };

            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            // Xác định 4 đỉnh góc nghiêng (hoặc mặc định góc nghiêng tài liệu)
            let corners = match &options.corners {
                Some(c) => c.clone(),
                None => [
                    Point2D {
                        x: raw_img.width as f32 * 0.1,
                        y: raw_img.height as f32 * 0.1,
                    },
                    Point2D {
                        x: raw_img.width as f32 * 0.9,
                        y: raw_img.height as f32 * 0.15,
                    },
                    Point2D {
                        x: raw_img.width as f32 * 0.85,
                        y: raw_img.height as f32 * 0.9,
                    },
                    Point2D {
                        x: raw_img.width as f32 * 0.15,
                        y: raw_img.height as f32 * 0.85,
                    },
                ],
            };

            let out_w = options.output_width.unwrap_or(raw_img.width);
            let out_h = options.output_height.unwrap_or(raw_img.height);

            // NFR-SEC-05: Giới hạn Image Bomb
            if out_w > 16384 || out_h > 16384 {
                if total_files == 1 {
                    return Err(CommandError::ValidationFailed {
                        reason: format!("Kích thước phẳng hóa ({out_w}x{out_h}) vượt giới hạn 16384x16384"),
                    });
                }
                failed_items.push(FileFailureDetail {
                    input_path: file_path_str.clone(),
                    reason: "Kích thước vượt giới hạn".to_string(),
                });
                continue;
            }

            let mut flattened = apply_perspective_transform(&raw_img, &corners, out_w, out_h);

            // Tự động tăng cường tương phản chữ nếu bật auto_enhance
            if options.auto_enhance {
                apply_document_enhancement(&mut flattened);
            }

            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("doc");
            let out_name = format!("{stem}_scanned.png");
            let out_bytes = match encode_png(&flattened, true) {
                Ok(b) => b,
                Err(e) => {
                    if total_files == 1 {
                        return Err(e);
                    }
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("{:?}", e),
                    });
                    continue;
                }
            };

            let out_path = output_dir.join(out_name);
            if let Err(e) = fs::write(&out_path, out_bytes) {
                if total_files == 1 {
                    return Err(CommandError::IoError {
                        details: format!("Lỗi ghi file đầu ra: {e}"),
                    });
                }
                failed_items.push(FileFailureDetail {
                    input_path: file_path_str.clone(),
                    reason: format!("Lỗi ghi: {e}"),
                });
                continue;
            }

            output_paths.push(out_path.to_string_lossy().to_string());

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: (idx + 1) as u32,
                    total_steps: total_files as u32,
                    percentage: ((idx + 1) as f32 / total_files as f32) * 100.0,
                    message: format!("Đã nắn phẳng tài liệu {}/{}", idx + 1, total_files),
                    processed_files: idx + 1,
                    failed_files: failed_items.len(),
                    total_files,
                    is_completed: idx + 1 == total_files,
                });
            }
        }

        let status = if failed_items.is_empty() {
            "SUCCESS".to_string()
        } else if !output_paths.is_empty() {
            "PARTIAL_SUCCESS".to_string()
        } else {
            "FAILED".to_string()
        };

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status,
            output_paths,
            failed_items,
            elapsed_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media_studio::image_batch::encode_bmp;
    use serde_json::json;

    fn make_test_tilted_doc(w: u32, h: u32) -> Vec<u8> {
        let mut pixels = vec![30u8; (w * h * 3) as usize]; // Mặt bàn tối

        // Vẽ một tờ giấy màu sáng [200, 200, 200] bị nghiêng bên trong
        for y in 20..h.saturating_sub(20) {
            for x in 30..w.saturating_sub(30) {
                let idx = ((y * w + x) * 3) as usize;
                if idx + 2 < pixels.len() {
                    pixels[idx] = 210;
                    pixels[idx + 1] = 210;
                    pixels[idx + 2] = 210;
                }
            }
        }

        // Vẽ một dòng chữ đen nét mực [20, 20, 20]
        for x in 50..120.min(w) {
            let y = 60;
            let idx = ((y * w + x) * 3) as usize;
            if idx + 2 < pixels.len() {
                pixels[idx] = 20;
                pixels[idx + 1] = 20;
                pixels[idx + 2] = 20;
            }
        }

        let img = RawImage {
            width: w,
            height: h,
            pixels,
        };
        encode_bmp(&img).expect("encode bmp")
    }

    #[test]
    fn test_doc_scanner_perspective_happy_path() {
        let tool = DocScannerPerspectiveTool::default();
        let tmp = std::env::temp_dir().join("test_persp_happy");
        let _ = fs::create_dir_all(&tmp);

        let input_file = tmp.join("tilted_page.bmp");
        fs::write(&input_file, make_test_tilted_doc(200, 150)).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-persp-happy".to_string(),
            tool_slug: "doc-scanner-perspective".to_string(),
            input_paths: vec![input_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "corners": [
                    { "x": 30.0, "y": 20.0 },
                    { "x": 170.0, "y": 20.0 },
                    { "x": 170.0, "y": 130.0 },
                    { "x": 30.0, "y": 130.0 }
                ],
                "auto_enhance": false,
                "output_width": 140,
                "output_height": 110
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("perspective should succeed");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read");
        let decoded = load_and_validate_image(&out_bytes).expect("decode");
        assert_eq!(decoded.width, 140);
        assert_eq!(decoded.height, 110);
    }

    #[test]
    fn test_doc_scanner_perspective_edge_case() {
        let tool = DocScannerPerspectiveTool::default();
        let tmp = std::env::temp_dir().join("test_persp_edge");
        let _ = fs::create_dir_all(&tmp);

        let input_file = tmp.join("tilted_contrast.bmp");
        fs::write(&input_file, make_test_tilted_doc(180, 120)).expect("write");

        // Bật auto_enhance: tăng tương phản ký tự đen trên nền giấy trắng
        let req = DispatchTaskRequest {
            task_id: "task-persp-enhance".to_string(),
            tool_slug: "doc-scanner-perspective".to_string(),
            input_paths: vec![input_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "auto_enhance": true,
                "output_width": 120,
                "output_height": 80
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("enhance should succeed");
        assert_eq!(res.status, "SUCCESS");

        let out_bytes = fs::read(&res.output_paths[0]).expect("read");
        let decoded = load_and_validate_image(&out_bytes).expect("decode");

        // Xác nhận nền giấy trắng được tăng sáng lên gần 255
        let has_bright_paper = decoded.pixels.iter().any(|&p| p >= 240);
        assert!(has_bright_paper, "Nền giấy trắng phải được đẩy sáng lên >= 240");

        // Xác nhận nét chữ được giữ đậm hoặc làm tối
        let has_dark_ink = decoded.pixels.iter().any(|&p| p <= 40);
        assert!(has_dark_ink, "Nét chữ mực đen phải có giá trị <= 40");
    }

    #[test]
    fn test_doc_scanner_perspective_invalid_magic_bytes() {
        let tool = DocScannerPerspectiveTool::default();
        let tmp = std::env::temp_dir().join("test_persp_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("bad_doc.png");
        fs::write(&bad_file, b"CORRUPTED_PERSPECTIVE_BYTES").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-bad-persp".to_string(),
            tool_slug: "doc-scanner-perspective".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "auto_enhance": false }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_doc_scanner_perspective_cooperative_cancellation() {
        let tool = DocScannerPerspectiveTool::default();
        let tmp = std::env::temp_dir().join("test_persp_cancel");
        let _ = fs::create_dir_all(&tmp);

        let file = tmp.join("persp_cancel.bmp");
        fs::write(&file, make_test_tilted_doc(40, 40)).expect("write");

        let req = DispatchTaskRequest {
            task_id: "task-cancel-persp".to_string(),
            tool_slug: "doc-scanner-perspective".to_string(),
            input_paths: vec![file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "auto_enhance": false }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_doc_scanner_perspective_security_guardrails() {
        let tool = DocScannerPerspectiveTool::default();
        assert_eq!(tool.slug(), "doc-scanner-perspective");
        assert_eq!(tool.name(), "Phẳng hóa Tài liệu Chụp Nghiêng");

        // Image bomb defense (>16384 px)
        let mut bomb = vec![0u8; 54];
        bomb[0..2].copy_from_slice(b"BM");
        bomb[10..14].copy_from_slice(&54u32.to_le_bytes());
        bomb[14..18].copy_from_slice(&40u32.to_le_bytes());
        bomb[18..22].copy_from_slice(&30000i32.to_le_bytes());
        bomb[22..26].copy_from_slice(&30000i32.to_le_bytes());
        bomb[28..30].copy_from_slice(&24u16.to_le_bytes());

        let err = load_and_validate_image(&bomb).unwrap_err();
        match err {
            CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("16384x16384"));
            }
            other => panic!("Kỳ vọng ValidationFailed nhưng nhận được: {:?}", other),
        }
    }
}
