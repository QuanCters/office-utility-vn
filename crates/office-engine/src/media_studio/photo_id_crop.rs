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

/// Tùy chọn cấu hình cắt ảnh thẻ chuẩn 300 DPI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoIdCropOptions {
    #[serde(default = "default_id_size")]
    pub id_size: String, // "3x4" hoặc "4x6"
    #[serde(default = "default_sheet_layout")]
    pub sheet_layout: String, // "single" hoặc "sheet_10x15"
}

fn default_id_size() -> String {
    "3x4".to_string()
}

fn default_sheet_layout() -> String {
    "single".to_string()
}

pub struct PhotoIdCropTool;

impl Default for PhotoIdCropTool {
    fn default() -> Self {
        Self
    }
}

/// Cắt vùng trung tâm theo tỷ lệ và resize về kích thước chuẩn (W x H)
fn crop_and_resize(img: &RawImage, target_w: u32, target_h: u32) -> RawImage {
    if img.width == 0 || img.height == 0 || target_w == 0 || target_h == 0 {
        return RawImage {
            width: target_w.max(1),
            height: target_h.max(1),
            pixels: vec![255; (target_w.max(1) * target_h.max(1) * 3) as usize],
        };
    }

    let target_aspect = target_w as f32 / target_h as f32;
    let src_aspect = img.width as f32 / img.height as f32;

    let (crop_w, crop_h, start_x, start_y) = if src_aspect > target_aspect {
        // Ảnh gốc bè ngang hơn mục tiêu -> cắt bớt 2 bên
        let ch = img.height;
        let cw = ((ch as f32 * target_aspect).round() as u32).min(img.width);
        let sx = (img.width - cw) / 2;
        (cw, ch, sx, 0)
    } else {
        // Ảnh gốc cao hơn mục tiêu -> cắt bớt trên/dưới
        let cw = img.width;
        let ch = ((cw as f32 / target_aspect).round() as u32).min(img.height);
        let sy = (img.height - ch) / 2;
        (cw, ch, 0, sy)
    };

    let mut out_pixels = Vec::with_capacity((target_w * target_h * 3) as usize);

    for y in 0..target_h {
        let sy = start_y + ((y as f32 / target_h as f32) * crop_h as f32).floor() as u32;
        let clamped_sy = sy.min(img.height - 1);
        for x in 0..target_w {
            let sx = start_x + ((x as f32 / target_w as f32) * crop_w as f32).floor() as u32;
            let clamped_sx = sx.min(img.width - 1);
            let idx = ((clamped_sy * img.width + clamped_sx) * 3) as usize;
            if idx + 2 < img.pixels.len() {
                out_pixels.push(img.pixels[idx]);
                out_pixels.push(img.pixels[idx + 1]);
                out_pixels.push(img.pixels[idx + 2]);
            } else {
                out_pixels.extend_from_slice(&[255, 255, 255]);
            }
        }
    }

    RawImage {
        width: target_w,
        height: target_h,
        pixels: out_pixels,
    }
}

/// Dàn nhiều ảnh thẻ lên khổ giấy in 10x15 cm (1181 x 1771 pixels tại 300 DPI)
fn create_print_sheet(photo: &RawImage, id_size: &str) -> RawImage {
    // Khổ giấy in chuẩn 10x15 cm ở 300 DPI
    let sheet_w = 1181u32;
    let sheet_h = 1771u32;
    let mut sheet_pixels = vec![255u8; (sheet_w * sheet_h * 3) as usize];

    // Xác định số hàng và cột
    let (cols, rows) = if id_size.contains("4x6") {
        (2u32, 2u32) // 4 ảnh 4x6
    } else {
        (2u32, 3u32) // 6 hoặc 8 ảnh 3x4
    };

    let total_photos_w = cols * photo.width;
    let total_photos_h = rows * photo.height;

    let margin_x = if sheet_w > total_photos_w {
        (sheet_w - total_photos_w) / (cols + 1)
    } else {
        10
    };
    let margin_y = if sheet_h > total_photos_h {
        (sheet_h - total_photos_h) / (rows + 1)
    } else {
        10
    };

    // Vẽ từng ảnh lên canvas
    for r in 0..rows {
        for c in 0..cols {
            let dest_x = margin_x + c * (photo.width + margin_x);
            let dest_y = margin_y + r * (photo.height + margin_y);

            // Copy điểm ảnh
            for py in 0..photo.height {
                let sy = dest_y + py;
                if sy >= sheet_h {
                    break;
                }
                for px in 0..photo.width {
                    let sx = dest_x + px;
                    if sx >= sheet_w {
                        break;
                    }

                    let photo_idx = ((py * photo.width + px) * 3) as usize;
                    let sheet_idx = ((sy * sheet_w + sx) * 3) as usize;

                    // Vẽ viền cắt ảnh (đường kẻ mỏng màu xám nhạt ở biên ngoài)
                    let is_border = py == 0
                        || py == photo.height - 1
                        || px == 0
                        || px == photo.width - 1;

                    if is_border {
                        sheet_pixels[sheet_idx] = 200;
                        sheet_pixels[sheet_idx + 1] = 200;
                        sheet_pixels[sheet_idx + 2] = 200;
                    } else if photo_idx + 2 < photo.pixels.len() && sheet_idx + 2 < sheet_pixels.len() {
                        sheet_pixels[sheet_idx] = photo.pixels[photo_idx];
                        sheet_pixels[sheet_idx + 1] = photo.pixels[photo_idx + 1];
                        sheet_pixels[sheet_idx + 2] = photo.pixels[photo_idx + 2];
                    }
                }
            }
        }
    }

    RawImage {
        width: sheet_w,
        height: sheet_h,
        pixels: sheet_pixels,
    }
}

impl OfficeTool for PhotoIdCropTool {
    fn slug(&self) -> &'static str {
        "photo-id-crop"
    }

    fn name(&self) -> &'static str {
        "Cắt Ảnh Thẻ Chuẩn 300 DPI"
    }

    fn description(&self) -> &'static str {
        "Cung cấp khung căn chỉnh tỷ lệ khuôn mặt chuẩn hộ chiếu/CCCD; tự động dàn trang ghép nhiều ảnh thẻ lên một khổ giấy in 10x15 cm ở mật độ điểm ảnh chuẩn 300 DPI."
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

        let options: PhotoIdCropOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn PhotoIdCrop không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp chân dung đầu vào trống".to_string(),
            });
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_photo_id_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let (target_w, target_h) = if options.id_size.contains("4x6") {
            (472u32, 708u32) // 4x6 cm tại 300 DPI
        } else {
            (354u32, 472u32) // 3x4 cm tại 300 DPI
        };

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

            // Cắt ảnh theo kích thước ảnh thẻ chuẩn
            let cropped = crop_and_resize(&raw_img, target_w, target_h);

            // Xử lý dàn trang in hoặc ảnh đơn
            let final_img = if options.sheet_layout == "sheet_10x15" {
                create_print_sheet(&cropped, &options.id_size)
            } else {
                cropped
            };

            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("photo");
            let out_name = if options.sheet_layout == "sheet_10x15" {
                format!("{stem}_sheet_10x15.png")
            } else {
                format!("{stem}_id_{}x{}.png", target_w, target_h)
            };

            let out_bytes = match encode_png(&final_img, true) {
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
                    message: format!("Đã cắt ảnh thẻ {}/{}", idx + 1, total_files),
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
        let img = RawImage {
            width: w,
            height: h,
            pixels: vec![180; (w * h * 3) as usize],
        };
        encode_bmp(&img).expect("encode test bmp")
    }

    #[test]
    fn test_photo_id_crop_happy_path() {
        let tool = PhotoIdCropTool;
        let tmp = std::env::temp_dir().join("test_photo_id_happy");
        let _ = fs::create_dir_all(&tmp);

        let input_file = tmp.join("portrait.bmp");
        fs::write(&input_file, make_test_bmp(600, 800)).expect("write portrait");

        let req = DispatchTaskRequest {
            task_id: "task-crop-happy".to_string(),
            tool_slug: "photo-id-crop".to_string(),
            input_paths: vec![input_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "id_size": "3x4",
                "sheet_layout": "single"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("crop should succeed");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read cropped");
        let decoded = load_and_validate_image(&out_bytes).expect("decode result");
        assert_eq!(decoded.width, 354);
        assert_eq!(decoded.height, 472);
    }

    #[test]
    fn test_photo_id_crop_edge_case() {
        let tool = PhotoIdCropTool;
        let tmp = std::env::temp_dir().join("test_photo_id_edge");
        let _ = fs::create_dir_all(&tmp);

        let input_file = tmp.join("portrait_edge.bmp");
        fs::write(&input_file, make_test_bmp(500, 700)).expect("write portrait");

        // Dàn trang in 10x15 cm
        let req = DispatchTaskRequest {
            task_id: "task-crop-sheet".to_string(),
            tool_slug: "photo-id-crop".to_string(),
            input_paths: vec![input_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "id_size": "4x6",
                "sheet_layout": "sheet_10x15"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("sheet imposition should succeed");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read sheet");
        let decoded = load_and_validate_image(&out_bytes).expect("decode sheet");
        assert_eq!(decoded.width, 1181);
        assert_eq!(decoded.height, 1771);
    }

    #[test]
    fn test_photo_id_crop_invalid_magic_bytes() {
        let tool = PhotoIdCropTool;
        let tmp = std::env::temp_dir().join("test_photo_id_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("corrupted.png");
        fs::write(&bad_file, b"NOT_A_VALID_IMAGE_SIGNATURE").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-bad-photo".to_string(),
            tool_slug: "photo-id-crop".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "id_size": "3x4" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_photo_id_crop_cooperative_cancellation() {
        let tool = PhotoIdCropTool;
        let tmp = std::env::temp_dir().join("test_photo_id_cancel");
        let _ = fs::create_dir_all(&tmp);

        let file = tmp.join("portrait_cancel.bmp");
        fs::write(&file, make_test_bmp(100, 100)).expect("write cancel");

        let req = DispatchTaskRequest {
            task_id: "task-cancel-photo".to_string(),
            tool_slug: "photo-id-crop".to_string(),
            input_paths: vec![file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "id_size": "3x4" }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_photo_id_crop_security_guardrails() {
        let tool = PhotoIdCropTool;
        assert_eq!(tool.slug(), "photo-id-crop");
        assert_eq!(tool.name(), "Cắt Ảnh Thẻ Chuẩn 300 DPI");

        // Thử nghiệm Image Bomb (>16384 px)
        let mut fake_bomb_bytes = vec![0u8; 54];
        fake_bomb_bytes[0..2].copy_from_slice(b"BM");
        fake_bomb_bytes[10..14].copy_from_slice(&54u32.to_le_bytes());
        fake_bomb_bytes[14..18].copy_from_slice(&40u32.to_le_bytes());
        fake_bomb_bytes[18..22].copy_from_slice(&20000i32.to_le_bytes()); // width 20000 > 16384
        fake_bomb_bytes[22..26].copy_from_slice(&20000i32.to_le_bytes()); // height
        fake_bomb_bytes[28..30].copy_from_slice(&24u16.to_le_bytes());
        let err = load_and_validate_image(&fake_bomb_bytes).unwrap_err();
        match err {
            CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("16384x16384"));
            }
            other => panic!("Kỳ vọng ValidationFailed cho bomb nhưng nhận được: {:?}", other),
        }
    }
}
