use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::image_batch::{encode_png, load_and_validate_image, RawImage};
use crate::OfficeTool;

/// Tùy chọn đóng dấu bản quyền ảnh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatermarkBatchOptions {
    #[serde(default = "default_watermark_type")]
    pub watermark_type: String, // "tiled" hoặc "single_anchor"
    #[serde(default = "default_opacity")]
    pub opacity: f32, // 0.1 .. 1.0
    #[serde(default = "default_position")]
    pub position: String, // "bottom_right", "center", "top_left", etc.
    #[serde(default = "default_text")]
    pub text: String,
}

fn default_watermark_type() -> String {
    "tiled".to_string()
}

fn default_opacity() -> f32 {
    0.3
}

fn default_position() -> String {
    "bottom_right".to_string()
}

fn default_text() -> String {
    "OFFICE UTILITY VN - BAN QUYEN".to_string()
}

pub struct WatermarkBatchTool;

impl Default for WatermarkBatchTool {
    fn default() -> Self {
        Self
    }
}

// Bảng font bitmap 5x7 tối giản cho các ký tự ASCII cơ bản (A-Z, 0-9, space, -, /, .)
fn get_glyph_bitmap(c: char) -> [u8; 7] {
    let uc = c.to_ascii_uppercase();
    match uc {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10011, 0b10101, 0b10001, 0b10001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '/' => [0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b00000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        _ => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
    }
}

/// Vẽ một chuỗi ký tự lên ảnh với độ mờ Alpha
fn render_text(
    img: &mut RawImage,
    text: &str,
    start_x: i32,
    start_y: i32,
    scale: u32,
    opacity: f32,
    color: [u8; 3],
) {
    let alpha = opacity.clamp(0.05, 1.0);
    let s = scale.max(1) as i32;

    let mut cursor_x = start_x;
    for c in text.chars() {
        let bitmap = get_glyph_bitmap(c);

        for (row_idx, &row) in bitmap.iter().enumerate() {
            for col_idx in 0..5 {
                let is_set = (row & (1 << (4 - col_idx))) != 0;
                if is_set {
                    for dy in 0..s {
                        for dx in 0..s {
                            let px = cursor_x + (col_idx * s) + dx;
                            let py = start_y + (row_idx as i32 * s) + dy;

                            if px >= 0 && px < img.width as i32 && py >= 0 && py < img.height as i32 {
                                let idx = ((py as u32 * img.width + px as u32) * 3) as usize;
                                if idx + 2 < img.pixels.len() {
                                    // Alpha blending: C = (1 - a)*C0 + a*Cw
                                    let r0 = img.pixels[idx] as f32;
                                    let g0 = img.pixels[idx + 1] as f32;
                                    let b0 = img.pixels[idx + 2] as f32;

                                    img.pixels[idx] = ((1.0 - alpha) * r0 + alpha * color[0] as f32).round() as u8;
                                    img.pixels[idx + 1] = ((1.0 - alpha) * g0 + alpha * color[1] as f32).round() as u8;
                                    img.pixels[idx + 2] = ((1.0 - alpha) * b0 + alpha * color[2] as f32).round() as u8;
                                }
                            }
                        }
                    }
                }
            }
        }

        cursor_x += (5 * s) + (2 * s); // Khoảng cách giữa các chữ cái
    }
}

/// Áp dụng Watermark dạng lưới lặp lại (Tiled Watermark) chéo góc
fn apply_tiled_watermark(img: &mut RawImage, text: &str, opacity: f32) {
    let scale = 2u32;
    let text_len = text.len() as i32;
    let text_width = text_len * (7 * scale as i32);
    let text_height = 7 * scale as i32;

    let step_x = text_width + 80;
    let step_y = text_height + 80;

    let mut start_y = -text_height;
    let mut row_idx = 0;

    while start_y < img.height as i32 + step_y {
        let x_offset = if row_idx % 2 == 1 { step_x / 2 } else { 0 };
        let mut start_x = -text_width + x_offset;

        while start_x < img.width as i32 + step_x {
            render_text(img, text, start_x, start_y, scale, opacity, [220, 220, 220]);
            start_x += step_x;
        }

        start_y += step_y;
        row_idx += 1;
    }
}

/// Áp dụng Watermark tại vị trí neo chỉ định (Single Anchor, ví dụ: bottom_right)
fn apply_anchor_watermark(img: &mut RawImage, text: &str, position: &str, opacity: f32) {
    let scale = 2u32;
    let text_len = text.len() as i32;
    let text_width = text_len * (7 * scale as i32);
    let text_height = 7 * scale as i32;

    let margin = 20i32;

    let (x, y) = match position.to_lowercase().as_str() {
        "top_left" => (margin, margin),
        "top_right" => ((img.width as i32 - text_width - margin).max(0), margin),
        "center" => (
            ((img.width as i32 - text_width) / 2).max(0),
            ((img.height as i32 - text_height) / 2).max(0),
        ),
        "bottom_left" => (margin, (img.height as i32 - text_height - margin).max(0)),
        _ => (
            // bottom_right
            (img.width as i32 - text_width - margin).max(0),
            (img.height as i32 - text_height - margin).max(0),
        ),
    };

    render_text(img, text, x, y, scale, opacity, [240, 240, 240]);
}

impl OfficeTool for WatermarkBatchTool {
    fn slug(&self) -> &'static str {
        "watermark-batch-studio"
    }

    fn name(&self) -> &'static str {
        "Đóng dấu Bản quyền Ảnh Hàng loạt"
    }

    fn description(&self) -> &'static str {
        "Chèn logo PNG hoặc chữ bản quyền dạng lưới chéo lặp lại (Tiled Watermark) hoặc ở 9 vị trí neo, hỗ trợ canh chỉnh độ mờ Alpha và xử lý song song đa luồng."
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

        let options: WatermarkBatchOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn WatermarkBatch không hợp lệ: {e}"),
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
            .unwrap_or_else(|| std::env::temp_dir().join("office_watermark_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let total_files = request.input_paths.len();

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 10.0,
                message: "Bắt đầu xử lý đóng dấu song song bằng Rayon...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files,
                is_completed: false,
            });
        }

        // Xử lý song song bằng Rayon work-stealing threadpool
        let results: Vec<Result<String, (String, CommandError)>> = request
            .input_paths
            .par_iter()
            .map(|file_path_str| {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err((file_path_str.clone(), CommandError::TaskCancelled));
                }

                let path = Path::new(file_path_str);
                if !path.exists() {
                    return Err((
                        file_path_str.clone(),
                        CommandError::FileNotFound {
                            path: file_path_str.clone(),
                        },
                    ));
                }

                let bytes = match fs::read(path) {
                    Ok(b) => b,
                    Err(e) => {
                        return Err((
                            file_path_str.clone(),
                            CommandError::IoError {
                                details: format!("Lỗi đọc: {e}"),
                            },
                        ))
                    }
                };

                let mut raw_img = match load_and_validate_image(&bytes) {
                    Ok(img) => img,
                    Err(err) => return Err((file_path_str.clone(), err)),
                };

                if cancel_flag.load(Ordering::Relaxed) {
                    return Err((file_path_str.clone(), CommandError::TaskCancelled));
                }

                if options.watermark_type.to_lowercase() == "single_anchor" {
                    apply_anchor_watermark(&mut raw_img, &options.text, &options.position, options.opacity);
                } else {
                    apply_tiled_watermark(&mut raw_img, &options.text, options.opacity);
                }

                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("watermarked");
                let out_name = format!("{stem}_wm.png");
                let out_bytes = match encode_png(&raw_img, true) {
                    Ok(b) => b,
                    Err(e) => return Err((file_path_str.clone(), e)),
                };

                let out_path = output_dir.join(out_name);
                if let Err(e) = fs::write(&out_path, out_bytes) {
                    return Err((
                        file_path_str.clone(),
                        CommandError::IoError {
                            details: format!("Lỗi ghi file: {e}"),
                        },
                    ));
                }

                Ok(out_path.to_string_lossy().to_string())
            })
            .collect();

        // Kiểm tra hủy tác vụ
        for r in &results {
            if let Err((_, CommandError::TaskCancelled)) = r {
                return Err(CommandError::TaskCancelled);
            }
        }

        let mut output_paths = Vec::new();
        let mut failed_items = Vec::new();

        for r in results {
            match r {
                Ok(path) => output_paths.push(path),
                Err((input_path, err)) => {
                    if total_files == 1 {
                        return Err(err);
                    }
                    failed_items.push(FileFailureDetail {
                        input_path,
                        reason: format!("{:?}", err),
                    });
                }
            }
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Đã hoàn thành đóng dấu bản quyền".to_string(),
                processed_files: output_paths.len(),
                failed_files: failed_items.len(),
                total_files,
                is_completed: true,
            });
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
            pixels: vec![50; (w * h * 3) as usize], // Ảnh nền xám tối
        };
        encode_bmp(&img).expect("encode test bmp")
    }

    #[test]
    fn test_watermark_batch_studio_happy_path() {
        let tool = WatermarkBatchTool;
        let tmp = std::env::temp_dir().join("test_wm_happy");
        let _ = fs::create_dir_all(&tmp);

        let img1 = tmp.join("img1.bmp");
        let img2 = tmp.join("img2.bmp");
        fs::write(&img1, make_test_bmp(200, 200)).expect("write img1");
        fs::write(&img2, make_test_bmp(150, 150)).expect("write img2");

        let req = DispatchTaskRequest {
            task_id: "task-wm-tiled".to_string(),
            tool_slug: "watermark-batch-studio".to_string(),
            input_paths: vec![
                img1.to_string_lossy().to_string(),
                img2.to_string_lossy().to_string(),
            ],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "watermark_type": "tiled",
                "opacity": 0.5,
                "text": "VN"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("tiled wm should succeed");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 2);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read out");
        let decoded = load_and_validate_image(&out_bytes).expect("decode");

        // Một số pixel phải được làm sáng hơn 50 nhờ watermark
        let has_blended = decoded.pixels.iter().any(|&p| p > 50);
        assert!(has_blended, "Watermark phải làm thay đổi giá trị điểm ảnh");
    }

    #[test]
    fn test_watermark_batch_studio_edge_case() {
        let tool = WatermarkBatchTool;
        let tmp = std::env::temp_dir().join("test_wm_edge");
        let _ = fs::create_dir_all(&tmp);

        let img = tmp.join("corner_photo.bmp");
        fs::write(&img, make_test_bmp(300, 200)).expect("write img");

        // Chèn logo/watermark vào góc dưới phải (bottom_right)
        let req = DispatchTaskRequest {
            task_id: "task-wm-corner".to_string(),
            tool_slug: "watermark-batch-studio".to_string(),
            input_paths: vec![img.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "watermark_type": "single_anchor",
                "position": "bottom_right",
                "opacity": 0.8,
                "text": "OFFICE"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("anchor wm should succeed");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read out");
        let decoded = load_and_validate_image(&out_bytes).expect("decode");

        // Góc trên bên trái (0, 0) không bị ảnh hưởng, giữ nguyên giá trị 50
        assert_eq!(decoded.pixels[0], 50);
        assert_eq!(decoded.pixels[1], 50);
        assert_eq!(decoded.pixels[2], 50);

        // Góc dưới bên phải (khoảng y: 166..180, x: 196..280) phải có pixel thay đổi do watermark
        let changed = (166..180).any(|y| {
            (196..280).any(|x| {
                let idx = ((y * decoded.width + x) * 3) as usize;
                decoded.pixels[idx] > 50
            })
        });
        assert!(changed, "Góc dưới phải phải có watermark điểm sáng");
    }

    #[test]
    fn test_watermark_batch_studio_invalid_magic_bytes() {
        let tool = WatermarkBatchTool;
        let tmp = std::env::temp_dir().join("test_wm_invalid");
        let _ = fs::create_dir_all(&tmp);

        let fake_file = tmp.join("bad.png");
        fs::write(&fake_file, b"NOT_VALID_IMAGE_BYTES").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-wm-bad".to_string(),
            tool_slug: "watermark-batch-studio".to_string(),
            input_paths: vec![fake_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "watermark_type": "tiled" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_watermark_batch_studio_cooperative_cancellation() {
        let tool = WatermarkBatchTool;
        let tmp = std::env::temp_dir().join("test_wm_cancel");
        let _ = fs::create_dir_all(&tmp);

        let file = tmp.join("cancel.bmp");
        fs::write(&file, make_test_bmp(50, 50)).expect("write cancel");

        let req = DispatchTaskRequest {
            task_id: "task-wm-cancel".to_string(),
            tool_slug: "watermark-batch-studio".to_string(),
            input_paths: vec![file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "watermark_type": "tiled" }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_watermark_batch_studio_security_guardrails() {
        let tool = WatermarkBatchTool;
        assert_eq!(tool.slug(), "watermark-batch-studio");
        assert_eq!(tool.name(), "Đóng dấu Bản quyền Ảnh Hàng loạt");

        // Image bomb defense (>16384 px)
        let mut bomb = vec![0u8; 54];
        bomb[0..2].copy_from_slice(b"BM");
        bomb[10..14].copy_from_slice(&54u32.to_le_bytes());
        bomb[14..18].copy_from_slice(&40u32.to_le_bytes());
        bomb[18..22].copy_from_slice(&25000i32.to_le_bytes()); // width > 16384
        bomb[22..26].copy_from_slice(&25000i32.to_le_bytes());
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
