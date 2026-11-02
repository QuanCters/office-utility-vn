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

/// Vùng hình chữ nhật cần che mờ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RectArea {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Tùy chọn cấu hình làm mờ vùng nhạy cảm trên ảnh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRedactOptions {
    #[serde(default = "default_redact_type")]
    pub redact_type: String, // "pixelate", "blur", "blackout"
    #[serde(default)]
    pub regions: Vec<RectArea>,
    #[serde(default)]
    pub auto_detect: bool,
}

fn default_redact_type() -> String {
    "pixelate".to_string()
}

pub struct ImageBgRedactTool;

impl Default for ImageBgRedactTool {
    fn default() -> Self {
        Self
    }
}

/// Tô đen hoàn toàn vùng chỉ định
fn apply_blackout(img: &mut RawImage, area: &RectArea) {
    let end_x = (area.x + area.width).min(img.width);
    let end_y = (area.y + area.height).min(img.height);

    for y in area.y..end_y {
        for x in area.x..end_x {
            let idx = ((y * img.width + x) * 3) as usize;
            if idx + 2 < img.pixels.len() {
                img.pixels[idx] = 0;
                img.pixels[idx + 1] = 0;
                img.pixels[idx + 2] = 0;
            }
        }
    }
}

/// Khảm điểm ảnh ô vuông (Pixelate) vĩnh viễn không thể khôi phục
fn apply_pixelate(img: &mut RawImage, area: &RectArea, block_size: u32) {
    let bs = block_size.max(4);
    let end_x = (area.x + area.width).min(img.width);
    let end_y = (area.y + area.height).min(img.height);

    let mut by = area.y;
    while by < end_y {
        let block_h = bs.min(end_y - by);
        let mut bx = area.x;
        while bx < end_x {
            let block_w = bs.min(end_x - bx);

            // Tính trung bình màu của khối
            let mut sum_r = 0u64;
            let mut sum_g = 0u64;
            let mut sum_b = 0u64;
            let mut count = 0u64;

            for dy in 0..block_h {
                for dx in 0..block_w {
                    let py = by + dy;
                    let px = bx + dx;
                    let idx = ((py * img.width + px) * 3) as usize;
                    if idx + 2 < img.pixels.len() {
                        sum_r += img.pixels[idx] as u64;
                        sum_g += img.pixels[idx + 1] as u64;
                        sum_b += img.pixels[idx + 2] as u64;
                        count += 1;
                    }
                }
            }

            if count > 0 {
                let avg_r = (sum_r / count) as u8;
                let avg_g = (sum_g / count) as u8;
                let avg_b = (sum_b / count) as u8;

                // Gán màu trung bình cho toàn bộ khối
                for dy in 0..block_h {
                    for dx in 0..block_w {
                        let py = by + dy;
                        let px = bx + dx;
                        let idx = ((py * img.width + px) * 3) as usize;
                        if idx + 2 < img.pixels.len() {
                            img.pixels[idx] = avg_r;
                            img.pixels[idx + 1] = avg_g;
                            img.pixels[idx + 2] = avg_b;
                        }
                    }
                }
            }

            bx += bs;
        }
        by += bs;
    }
}

/// Làm mờ vùng ảnh (Blur) qua thuật toán Box Blur
fn apply_blur(img: &mut RawImage, area: &RectArea, radius: u32) {
    let r = radius.clamp(2, 16) as i32;
    let end_x = (area.x + area.width).min(img.width);
    let end_y = (area.y + area.height).min(img.height);

    let mut temp = img.pixels.clone();

    // Pass 1: Mờ theo chiều ngang
    for y in area.y..end_y {
        for x in area.x..end_x {
            let mut sum_r = 0u32;
            let mut sum_g = 0u32;
            let mut sum_b = 0u32;
            let mut count = 0u32;

            for dx in -r..=r {
                let sample_x = (x as i32 + dx).clamp(area.x as i32, end_x as i32 - 1) as u32;
                let idx = ((y * img.width + sample_x) * 3) as usize;
                if idx + 2 < img.pixels.len() {
                    sum_r += img.pixels[idx] as u32;
                    sum_g += img.pixels[idx + 1] as u32;
                    sum_b += img.pixels[idx + 2] as u32;
                    count += 1;
                }
            }

            let idx = ((y * img.width + x) * 3) as usize;
            if count > 0 && idx + 2 < temp.len() {
                temp[idx] = (sum_r / count) as u8;
                temp[idx + 1] = (sum_g / count) as u8;
                temp[idx + 2] = (sum_b / count) as u8;
            }
        }
    }

    // Pass 2: Mờ theo chiều dọc
    for y in area.y..end_y {
        for x in area.x..end_x {
            let mut sum_r = 0u32;
            let mut sum_g = 0u32;
            let mut sum_b = 0u32;
            let mut count = 0u32;

            for dy in -r..=r {
                let sample_y = (y as i32 + dy).clamp(area.y as i32, end_y as i32 - 1) as u32;
                let idx = ((sample_y * img.width + x) * 3) as usize;
                if idx + 2 < temp.len() {
                    sum_r += temp[idx] as u32;
                    sum_g += temp[idx + 1] as u32;
                    sum_b += temp[idx + 2] as u32;
                    count += 1;
                }
            }

            let idx = ((y * img.width + x) * 3) as usize;
            if count > 0 && idx + 2 < img.pixels.len() {
                img.pixels[idx] = (sum_r / count) as u8;
                img.pixels[idx + 1] = (sum_g / count) as u8;
                img.pixels[idx + 2] = (sum_b / count) as u8;
            }
        }
    }
}

impl OfficeTool for ImageBgRedactTool {
    fn slug(&self) -> &'static str {
        "image-bg-redact"
    }

    fn name(&self) -> &'static str {
        "Làm mờ Dữ liệu Nhạy cảm Giấy tờ"
    }

    fn description(&self) -> &'static str {
        "Cho phép quét chọn vùng hoặc tự động khoanh vùng số CCCD, mã QR để áp dụng bộ lọc làm mờ vĩnh viễn (Gaussian Blur / Pixelate) không thể khôi phục ngược."
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

        let options: ImageRedactOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn ImageRedact không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp ảnh đầu vào trống".to_string(),
            });
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_image_redact_output"));

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
                        reason: format!("Lỗi đọc tệp: {e}"),
                    });
                    continue;
                }
            };

            let mut raw_img = match load_and_validate_image(&bytes) {
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

            // Xác định danh sách vùng cần che
            let mut regions = options.regions.clone();
            if regions.is_empty() && options.auto_detect {
                // Tự động khoanh vùng khu vực số CCCD (ước lượng ở nửa dưới giữa ảnh)
                let card_w = raw_img.width;
                let card_h = raw_img.height;
                regions.push(RectArea {
                    x: card_w / 4,
                    y: (card_h * 5) / 8,
                    width: card_w / 2,
                    height: card_h / 8,
                });
            }

            // Áp dụng bộ lọc cho từng vùng
            for area in &regions {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                match options.redact_type.to_lowercase().as_str() {
                    "blackout" | "black" | "box" => apply_blackout(&mut raw_img, area),
                    "blur" | "gaussian" => apply_blur(&mut raw_img, area, 10),
                    _ => apply_pixelate(&mut raw_img, area, 12),
                }
            }

            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("redacted");
            let out_name = format!("{stem}_redacted.png");
            let out_bytes = match encode_png(&raw_img, true) {
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
                        details: format!("Lỗi ghi tệp: {e}"),
                    });
                }
                failed_items.push(FileFailureDetail {
                    input_path: file_path_str.clone(),
                    reason: format!("Lỗi ghi file: {e}"),
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
                    message: format!("Đã che mờ ảnh {}/{}", idx + 1, total_files),
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

    fn make_test_bmp(w: u32, h: u32) -> Vec<u8> {
        let mut pixels = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            for x in 0..w {
                // Tạo mẫu gradient để kiểm tra làm mờ
                pixels.push(((x * 255) / w.max(1)) as u8);
                pixels.push(((y * 255) / h.max(1)) as u8);
                pixels.push(128);
            }
        }
        let img = RawImage {
            width: w,
            height: h,
            pixels,
        };
        encode_bmp(&img).expect("encode test bmp")
    }

    #[test]
    fn test_image_bg_redact_happy_path() {
        let tool = ImageBgRedactTool::default();
        let tmp = std::env::temp_dir().join("test_redact_happy");
        let _ = fs::create_dir_all(&tmp);

        let input_file = tmp.join("doc_sample.bmp");
        fs::write(&input_file, make_test_bmp(200, 200)).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-redact-happy".to_string(),
            tool_slug: "image-bg-redact".to_string(),
            input_paths: vec![input_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "redact_type": "pixelate",
                "regions": [
                    { "x": 20, "y": 20, "width": 60, "height": 40 }
                ]
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("redact should succeed");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read output");
        let decoded = load_and_validate_image(&out_bytes).expect("decode result");
        assert_eq!(decoded.width, 200);
        assert_eq!(decoded.height, 200);
    }

    #[test]
    fn test_image_bg_redact_edge_case() {
        let tool = ImageBgRedactTool::default();
        let tmp = std::env::temp_dir().join("test_redact_edge");
        let _ = fs::create_dir_all(&tmp);

        let input_file = tmp.join("cccd_edge.bmp");
        fs::write(&input_file, make_test_bmp(300, 150)).expect("write cccd");

        // Tô đen hoàn toàn (blackout) vùng số CMND/CCCD
        let req = DispatchTaskRequest {
            task_id: "task-redact-blackout".to_string(),
            tool_slug: "image-bg-redact".to_string(),
            input_paths: vec![input_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "redact_type": "blackout",
                "regions": [
                    { "x": 50, "y": 50, "width": 100, "height": 30 }
                ]
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("blackout should succeed");
        assert_eq!(res.status, "SUCCESS");

        let out_bytes = fs::read(&res.output_paths[0]).expect("read blackout");
        let decoded = load_and_validate_image(&out_bytes).expect("decode");

        // Kiểm tra vùng [50..150, 50..80] phải toàn bộ là 0 (đen)
        for y in 50..80 {
            for x in 50..150 {
                let idx = ((y * decoded.width + x) * 3) as usize;
                assert_eq!(decoded.pixels[idx], 0, "R must be 0 at ({x}, {y})");
                assert_eq!(decoded.pixels[idx + 1], 0, "G must be 0 at ({x}, {y})");
                assert_eq!(decoded.pixels[idx + 2], 0, "B must be 0 at ({x}, {y})");
            }
        }
    }

    #[test]
    fn test_image_bg_redact_invalid_magic_bytes() {
        let tool = ImageBgRedactTool::default();
        let tmp = std::env::temp_dir().join("test_redact_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("fake_doc.jpg");
        fs::write(&bad_file, b"CORRUPTED_NOT_AN_IMAGE_FILE").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-bad-doc".to_string(),
            tool_slug: "image-bg-redact".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "redact_type": "blur" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_image_bg_redact_cooperative_cancellation() {
        let tool = ImageBgRedactTool::default();
        let tmp = std::env::temp_dir().join("test_redact_cancel");
        let _ = fs::create_dir_all(&tmp);

        let file = tmp.join("doc_cancel.bmp");
        fs::write(&file, make_test_bmp(50, 50)).expect("write cancel");

        let req = DispatchTaskRequest {
            task_id: "task-cancel-doc".to_string(),
            tool_slug: "image-bg-redact".to_string(),
            input_paths: vec![file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "redact_type": "blackout" }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_image_bg_redact_security_guardrails() {
        let tool = ImageBgRedactTool::default();
        assert_eq!(tool.slug(), "image-bg-redact");
        assert_eq!(tool.name(), "Làm mờ Dữ liệu Nhạy cảm Giấy tờ");

        // Image bomb defense (>16384 px)
        let mut fake_bomb = vec![0u8; 54];
        fake_bomb[0..2].copy_from_slice(b"BM");
        fake_bomb[10..14].copy_from_slice(&54u32.to_le_bytes());
        fake_bomb[14..18].copy_from_slice(&40u32.to_le_bytes());
        fake_bomb[18..22].copy_from_slice(&18000i32.to_le_bytes()); // width > 16384
        fake_bomb[22..26].copy_from_slice(&18000i32.to_le_bytes());
        fake_bomb[28..30].copy_from_slice(&24u16.to_le_bytes());

        let err = load_and_validate_image(&fake_bomb).unwrap_err();
        match err {
            CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("16384x16384"));
            }
            other => panic!("Kỳ vọng ValidationFailed nhưng nhận được: {:?}", other),
        }
    }
}
