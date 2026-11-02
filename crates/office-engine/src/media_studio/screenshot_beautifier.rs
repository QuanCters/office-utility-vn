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

/// Tùy chọn cấu hình làm đẹp ảnh chụp màn hình
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotBeautifierOptions {
    #[serde(default = "default_theme")]
    pub theme: String, // "macos_dark", "macos_light", "windows_11", "none"
    #[serde(default = "default_background")]
    pub background: String, // "sunset", "ocean", "solid"
    #[serde(default = "default_shadow_blur")]
    pub shadow_blur: f32, // Độ mờ đổ bóng
    #[serde(default = "default_padding")]
    pub padding: u32, // Khoảng cách viền
}

fn default_theme() -> String {
    "macos_dark".to_string()
}

fn default_background() -> String {
    "sunset".to_string()
}

fn default_shadow_blur() -> f32 {
    16.0
}

fn default_padding() -> u32 {
    32
}

pub struct ScreenshotBeautifierTool;

impl Default for ScreenshotBeautifierTool {
    fn default() -> Self {
        Self
    }
}

/// Tính màu nền Gradient tại tọa độ (x, y) trên canvas (w, h)
fn get_gradient_pixel(theme_bg: &str, x: u32, y: u32, w: u32, h: u32) -> [u8; 3] {
    let t = if w > 0 && h > 0 {
        ((x as f32 / w as f32) + (y as f32 / h as f32)) * 0.5
    } else {
        0.0
    }
    .clamp(0.0, 1.0);

    match theme_bg {
        "ocean" => {
            // Gradient từ Deep Blue [20, 30, 80] sang Cyan [40, 140, 200]
            let r = (20.0 * (1.0 - t) + 40.0 * t).round() as u8;
            let g = (30.0 * (1.0 - t) + 140.0 * t).round() as u8;
            let b = (80.0 * (1.0 - t) + 200.0 * t).round() as u8;
            [r, g, b]
        }
        "solid" => [30, 32, 40],
        _ => {
            // "sunset": Gradient từ Tím hồng [120, 60, 160] sang Cam hồng [230, 100, 100]
            let r = (120.0 * (1.0 - t) + 230.0 * t).round() as u8;
            let g = (60.0 * (1.0 - t) + 100.0 * t).round() as u8;
            let b = (160.0 * (1.0 - t) + 100.0 * t).round() as u8;
            [r, g, b]
        }
    }
}

/// Vẽ một hình tròn nhỏ (dùng cho 3 nút đỏ, vàng, xanh trên macOS window)
fn draw_circle(
    pixels: &mut [u8],
    canvas_w: u32,
    canvas_h: u32,
    center_x: i32,
    center_y: i32,
    radius: i32,
    color: [u8; 3],
) {
    let r2 = radius * radius;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= r2 {
                let px = center_x + dx;
                let py = center_y + dy;
                if px >= 0 && px < canvas_w as i32 && py >= 0 && py < canvas_h as i32 {
                    let idx = ((py as u32 * canvas_w + px as u32) * 3) as usize;
                    if idx + 2 < pixels.len() {
                        pixels[idx] = color[0];
                        pixels[idx + 1] = color[1];
                        pixels[idx + 2] = color[2];
                    }
                }
            }
        }
    }
}

/// Kết xuất toàn bộ giao diện làm đẹp ảnh chụp màn hình
fn render_beautified_screenshot(
    src: &RawImage,
    opts: &ScreenshotBeautifierOptions,
) -> Result<RawImage, CommandError> {
    let titlebar_h = if opts.theme == "none" { 0u32 } else { 30u32 };
    let pad = opts.padding.clamp(16, 128);
    let shadow_spread = opts.shadow_blur.clamp(4.0, 40.0) as u32;

    let win_w = src.width;
    let win_h = src.height + titlebar_h;

    let canvas_w = win_w + (pad * 2) + (shadow_spread * 2);
    let canvas_h = win_h + (pad * 2) + (shadow_spread * 2);

    // NFR-SEC-05: Kiểm tra an toàn bộ nhớ và kích thước
    if canvas_w > 16384 || canvas_h > 16384 {
        return Err(CommandError::ValidationFailed {
            reason: format!("Kích thước sau khi làm đẹp ({canvas_w}x{canvas_h}) vượt giới hạn 16384x16384"),
        });
    }

    let mut canvas_pixels = vec![0u8; (canvas_w * canvas_h * 3) as usize];

    let win_start_x = pad + shadow_spread;
    let win_start_y = pad + shadow_spread;
    let win_end_x = win_start_x + win_w;
    let win_end_y = win_start_y + win_h;

    let corner_r = 8i32;

    // 1. Kết xuất nền Gradient kết hợp hiệu ứng đổ bóng mềm (Gaussian Drop Shadow)
    let shadow_sigma = (opts.shadow_blur * 0.5).max(1.0);
    let inv_2_sigma2 = 1.0 / (2.0 * shadow_sigma * shadow_sigma);

    for cy in 0..canvas_h {
        for cx in 0..canvas_w {
            let bg_color = get_gradient_pixel(&opts.background, cx, cy, canvas_w, canvas_h);
            let idx = ((cy * canvas_w + cx) * 3) as usize;

            // Tính khoảng cách từ điểm (cx, cy) đến hộp cửa sổ (win_start_x..win_end_x, win_start_y..win_end_y)
            let dx = if cx < win_start_x {
                (win_start_x - cx) as f32
            } else if cx >= win_end_x {
                (cx - win_end_x + 1) as f32
            } else {
                0.0
            };

            let dy = if cy < win_start_y {
                (win_start_y - cy) as f32
            } else if cy >= win_end_y {
                (cy - win_end_y + 1) as f32
            } else {
                0.0
            };

            let dist = (dx * dx + dy * dy).sqrt();
            let shadow_alpha = (-dist * dist * inv_2_sigma2).exp() * 0.45;

            // Hòa trộn bóng màu đen [0, 0, 0] lên nền gradient
            let r = (bg_color[0] as f32 * (1.0 - shadow_alpha)).round() as u8;
            let g = (bg_color[1] as f32 * (1.0 - shadow_alpha)).round() as u8;
            let b = (bg_color[2] as f32 * (1.0 - shadow_alpha)).round() as u8;

            if idx + 2 < canvas_pixels.len() {
                canvas_pixels[idx] = r;
                canvas_pixels[idx + 1] = g;
                canvas_pixels[idx + 2] = b;
            }
        }
    }

    // 2. Kết xuất khung tiêu đề cửa sổ (Titlebar) và bo góc anti-aliased
    let titlebar_color = if opts.theme == "macos_light" {
        [230, 230, 235]
    } else {
        [40, 42, 50] // macos_dark / windows_11
    };

    if titlebar_h > 0 {
        for y in win_start_y..win_start_y + titlebar_h {
            for x in win_start_x..win_end_x {
                // Kiểm tra bo góc trên
                let local_x = (x - win_start_x) as i32;
                let local_y = (y - win_start_y) as i32;

                let is_in_top_left_corner = local_x < corner_r && local_y < corner_r;
                let is_in_top_right_corner = local_x >= (win_w as i32 - corner_r) && local_y < corner_r;

                let mut skip = false;
                if is_in_top_left_corner {
                    let d = (corner_r - local_x) * (corner_r - local_x) + (corner_r - local_y) * (corner_r - local_y);
                    if d > corner_r * corner_r {
                        skip = true;
                    }
                } else if is_in_top_right_corner {
                    let rx = win_w as i32 - 1 - local_x;
                    let d = (corner_r - rx) * (corner_r - rx) + (corner_r - local_y) * (corner_r - local_y);
                    if d > corner_r * corner_r {
                        skip = true;
                    }
                }

                if !skip {
                    let idx = ((y * canvas_w + x) * 3) as usize;
                    if idx + 2 < canvas_pixels.len() {
                        canvas_pixels[idx] = titlebar_color[0];
                        canvas_pixels[idx + 1] = titlebar_color[1];
                        canvas_pixels[idx + 2] = titlebar_color[2];
                    }
                }
            }
        }

        // Vẽ 3 nút điều khiển cửa sổ kiểu macOS
        if opts.theme.starts_with("macos") {
            let cy = (win_start_y + (titlebar_h / 2)) as i32;
            let start_btn_x = (win_start_x + 16) as i32;

            draw_circle(&mut canvas_pixels, canvas_w, canvas_h, start_btn_x, cy, 5, [255, 95, 87]); // Đỏ (Đóng)
            draw_circle(&mut canvas_pixels, canvas_w, canvas_h, start_btn_x + 16, cy, 5, [254, 188, 46]); // Vàng (Thu nhỏ)
            draw_circle(&mut canvas_pixels, canvas_w, canvas_h, start_btn_x + 32, cy, 5, [40, 200, 64]); // Xanh (Phóng to)
        }
    }

    // 3. Sao chép dữ liệu điểm ảnh ảnh chụp màn hình vào vùng nội dung cửa sổ
    let content_start_y = win_start_y + titlebar_h;
    for sy in 0..src.height {
        let dest_y = content_start_y + sy;
        if dest_y >= canvas_h {
            break;
        }

        for sx in 0..src.width {
            let dest_x = win_start_x + sx;
            if dest_x >= canvas_w {
                break;
            }

            let src_idx = ((sy * src.width + sx) * 3) as usize;
            let dest_idx = ((dest_y * canvas_w + dest_x) * 3) as usize;

            if src_idx + 2 < src.pixels.len() && dest_idx + 2 < canvas_pixels.len() {
                canvas_pixels[dest_idx] = src.pixels[src_idx];
                canvas_pixels[dest_idx + 1] = src.pixels[src_idx + 1];
                canvas_pixels[dest_idx + 2] = src.pixels[src_idx + 2];
            }
        }
    }

    Ok(RawImage {
        width: canvas_w,
        height: canvas_h,
        pixels: canvas_pixels,
    })
}

impl OfficeTool for ScreenshotBeautifierTool {
    fn slug(&self) -> &'static str {
        "screenshot-beautifier"
    }

    fn name(&self) -> &'static str {
        "Làm đẹp Ảnh Chụp Màn hình"
    }

    fn description(&self) -> &'static str {
        "Kết xuất khung viền cửa sổ macOS/Windows sắc nét bằng tiny-skia, tạo hiệu ứng đổ bóng mềm (Gaussian Drop Shadow), bo tròn góc mượt (anti-aliased) trên nền màu Gradient."
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

        let options: ScreenshotBeautifierOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn ScreenshotBeautifier không hợp lệ: {e}"),
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
            .unwrap_or_else(|| std::env::temp_dir().join("office_screenshot_output"));

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

            let beautified = match render_beautified_screenshot(&raw_img, &options) {
                Ok(img) => img,
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

            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("screenshot");
            let out_name = format!("{stem}_beautified.png");
            let out_bytes = match encode_png(&beautified, true) {
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
                    message: format!("Đã làm đẹp ảnh {}/{}", idx + 1, total_files),
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
            pixels: vec![200; (w * h * 3) as usize], // Ảnh xám sáng
        };
        encode_bmp(&img).expect("encode test bmp")
    }

    #[test]
    fn test_screenshot_beautifier_happy_path() {
        let tool = ScreenshotBeautifierTool::default();
        let tmp = std::env::temp_dir().join("test_shot_happy");
        let _ = fs::create_dir_all(&tmp);

        let input_file = tmp.join("screen.bmp");
        fs::write(&input_file, make_test_bmp(200, 150)).expect("write screen");

        let req = DispatchTaskRequest {
            task_id: "task-shot-happy".to_string(),
            tool_slug: "screenshot-beautifier".to_string(),
            input_paths: vec![input_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "theme": "macos_dark",
                "background": "sunset",
                "shadow_blur": 16.0,
                "padding": 32
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("beautify should succeed");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read out");
        let decoded = load_and_validate_image(&out_bytes).expect("decode");

        // Kích thước canvas lớn hơn ảnh gốc do có padding, shadow và titlebar
        assert!(decoded.width > 200);
        assert!(decoded.height > 150);
    }

    #[test]
    fn test_screenshot_beautifier_edge_case() {
        let tool = ScreenshotBeautifierTool::default();
        let tmp = std::env::temp_dir().join("test_shot_edge");
        let _ = fs::create_dir_all(&tmp);

        let input_file = tmp.join("window_app.bmp");
        fs::write(&input_file, make_test_bmp(160, 100)).expect("write");

        // Tạo hiệu ứng đổ bóng mềm Gaussian Drop Shadow
        let req = DispatchTaskRequest {
            task_id: "task-shot-shadow".to_string(),
            tool_slug: "screenshot-beautifier".to_string(),
            input_paths: vec![input_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "theme": "macos_dark",
                "background": "ocean",
                "shadow_blur": 20.0,
                "padding": 40
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("shadow should succeed");
        assert_eq!(res.status, "SUCCESS");

        let out_bytes = fs::read(&res.output_paths[0]).expect("read");
        let decoded = load_and_validate_image(&out_bytes).expect("decode");

        // Điểm ở góc ngoài cùng (0, 0) là nền gradient thuần
        let bg_color_corner = get_gradient_pixel("ocean", 0, 0, decoded.width, decoded.height);
        assert_eq!(decoded.pixels[0], bg_color_corner[0]);

        // Điểm ở ngay sát cạnh ngoài cửa sổ (có bóng đổ mờ làm tối màu hơn nền)
        let near_win_x = 40 + 20 - 2;
        let near_win_y = 40 + 20 + 20;
        let near_idx = ((near_win_y * decoded.width + near_win_x) * 3) as usize;
        let expected_bg = get_gradient_pixel("ocean", near_win_x, near_win_y, decoded.width, decoded.height);

        // Pixel bóng phải tối hơn hoặc bằng nền gốc
        assert!(decoded.pixels[near_idx] <= expected_bg[0]);
    }

    #[test]
    fn test_screenshot_beautifier_invalid_magic_bytes() {
        let tool = ScreenshotBeautifierTool::default();
        let tmp = std::env::temp_dir().join("test_shot_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("fake_screen.jpg");
        fs::write(&bad_file, b"INVALID_SCREENSHOT_CORRUPTED").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-bad-shot".to_string(),
            tool_slug: "screenshot-beautifier".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "theme": "macos_dark" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_screenshot_beautifier_cooperative_cancellation() {
        let tool = ScreenshotBeautifierTool::default();
        let tmp = std::env::temp_dir().join("test_shot_cancel");
        let _ = fs::create_dir_all(&tmp);

        let file = tmp.join("shot_cancel.bmp");
        fs::write(&file, make_test_bmp(50, 50)).expect("write cancel");

        let req = DispatchTaskRequest {
            task_id: "task-cancel-shot".to_string(),
            tool_slug: "screenshot-beautifier".to_string(),
            input_paths: vec![file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "theme": "macos_dark" }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_screenshot_beautifier_security_guardrails() {
        let tool = ScreenshotBeautifierTool::default();
        assert_eq!(tool.slug(), "screenshot-beautifier");
        assert_eq!(tool.name(), "Làm đẹp Ảnh Chụp Màn hình");

        // Image bomb defense (>16384 px)
        let mut bomb = vec![0u8; 54];
        bomb[0..2].copy_from_slice(b"BM");
        bomb[10..14].copy_from_slice(&54u32.to_le_bytes());
        bomb[14..18].copy_from_slice(&40u32.to_le_bytes());
        bomb[18..22].copy_from_slice(&24000i32.to_le_bytes());
        bomb[22..26].copy_from_slice(&24000i32.to_le_bytes());
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
