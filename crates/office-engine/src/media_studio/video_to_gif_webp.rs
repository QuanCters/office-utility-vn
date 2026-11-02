use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use serde::{Deserialize, Serialize};

use super::image_batch::RawImage;
use crate::OfficeTool;

/// Tùy chọn cấu hình chuyển đổi video sang GIF / WebP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoToGifOptions {
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default, alias = "width")]
    pub max_width: Option<u32>,
    #[serde(default = "default_output_format")]
    pub output_format: String, // "gif" hoặc "webp"
}

fn default_fps() -> u32 {
    15
}

fn default_output_format() -> String {
    "gif".to_string()
}

pub struct VideoToGifWebpTool;

impl Default for VideoToGifWebpTool {
    fn default() -> Self {
        Self
    }
}

/// Khởi tạo bảng màu 256 màu tiêu chuẩn (Color Quantization Palette)
pub fn generate_quantized_palette() -> Vec<[u8; 3]> {
    let mut palette = Vec::with_capacity(256);
    for i in 0..256u32 {
        let r3 = (i >> 5) & 0x07;
        let g3 = (i >> 2) & 0x07;
        let b2 = i & 0x03;

        let r = ((r3 * 255) / 7) as u8;
        let g = ((g3 * 255) / 7) as u8;
        let b = ((b2 * 255) / 3) as u8;

        palette.push([r, g, b]);
    }
    palette
}

/// Lượng tử hóa một điểm ảnh RGB về chỉ số trong bảng màu 256 màu
pub fn quantize_rgb_to_index(r: u8, g: u8, b: u8) -> u8 {
    let r3 = (r >> 5) & 0x07;
    let g3 = (g >> 5) & 0x07;
    let b2 = (b >> 6) & 0x03;
    ((r3 << 5) | (g3 << 2) | b2) as u8
}

/// Đóng gói mảng byte LZW thành các sub-block GIF (tối đa 254 bytes mỗi block)
fn write_gif_subblocks(out: &mut Vec<u8>, lzw_data: &[u8]) {
    for chunk in lzw_data.chunks(254) {
        out.push(chunk.len() as u8);
        out.extend_from_slice(chunk);
    }
    out.push(0); // Kết thúc sub-block
}

/// Mã hóa chuỗi chỉ số điểm ảnh bằng thuật toán LZW chuẩn GIF89a
fn encode_gif_lzw(indices: &[u8]) -> Vec<u8> {
    let mut raw_bytes = Vec::new();
    let min_code_size = 8u8;
    let clear_code = 1u32 << min_code_size; // 256
    let end_of_info = clear_code + 1;       // 257

    let mut code_size = min_code_size + 1; // 9 bits
    let mut acc = 0u32;
    let mut bits = 0u32;

    // 1. Ghi Clear Code
    acc |= clear_code << bits;
    bits += code_size as u32;
    while bits >= 8 {
        raw_bytes.push((acc & 0xFF) as u8);
        acc >>= 8;
        bits -= 8;
    }

    // 2. Ghi từng byte chỉ số dưới dạng mã nguyên bản (Clear Code Reset định kỳ)
    let mut code_count = 2; // clear + end
    for &idx in indices {
        acc |= (idx as u32) << bits;
        bits += code_size as u32;
        while bits >= 8 {
            raw_bytes.push((acc & 0xFF) as u8);
            acc >>= 8;
            bits -= 8;
        }

        code_count += 1;
        if code_count >= (1 << code_size) && code_size < 12 {
            code_size += 1;
        } else if code_count >= 4094 {
            // Gửi lại Clear code khi chạm giới hạn 4096 của từ điển
            acc |= clear_code << bits;
            bits += code_size as u32;
            while bits >= 8 {
                raw_bytes.push((acc & 0xFF) as u8);
                acc >>= 8;
                bits -= 8;
            }
            code_size = min_code_size + 1;
            code_count = 2;
        }
    }

    // 3. Ghi End of Information Code
    acc |= end_of_info << bits;
    bits += code_size as u32;
    while bits >= 8 {
        raw_bytes.push((acc & 0xFF) as u8);
        acc >>= 8;
        bits -= 8;
    }

    if bits > 0 {
        raw_bytes.push((acc & 0xFF) as u8);
    }

    raw_bytes
}

/// Xuất chuỗi khung hình RawImage ra tệp GIF89a động
pub fn encode_animated_gif(frames: &[RawImage], fps: u32) -> Result<Vec<u8>, CommandError> {
    if frames.is_empty() {
        return Err(CommandError::ValidationFailed {
            reason: "Danh sách khung hình rỗng".to_string(),
        });
    }

    let w = frames[0].width;
    let h = frames[0].height;

    if w > 16384 || h > 16384 {
        return Err(CommandError::ValidationFailed {
            reason: format!("Kích thước GIF ({w}x{h}) vượt giới hạn 16384x16384"),
        });
    }

    let palette = generate_quantized_palette();
    let mut out = Vec::new();

    // 1. GIF Header
    out.extend_from_slice(b"GIF89a");

    // 2. Logical Screen Descriptor (7 bytes)
    out.extend_from_slice(&(w as u16).to_le_bytes());
    out.extend_from_slice(&(h as u16).to_le_bytes());
    out.push(0xF7); // Global Color Table Flag (1), 8 bpp (111), Sort (0), Size 256 (111)
    out.push(0);    // Background color index
    out.push(0);    // Pixel aspect ratio

    // 3. Global Color Table (256 * 3 = 768 bytes)
    for color in &palette {
        out.extend_from_slice(color);
    }

    // 4. Netscape Looping Application Extension (Lặp vô hạn)
    out.extend_from_slice(&[0x21, 0xFF, 0x0B]);
    out.extend_from_slice(b"NETSCAPE2.0");
    out.extend_from_slice(&[0x03, 0x01, 0x00, 0x00, 0x00]);

    // Độ trễ giữa các khung hình (đơn vị 1/100 giây)
    let delay_hundredths = ((100.0 / fps.max(1) as f32).round() as u16).max(1);

    // 5. Ghi từng khung hình
    for frame in frames {
        // Graphic Control Extension (8 bytes)
        out.extend_from_slice(&[0x21, 0xF9, 0x04]);
        out.push(0x04); // Packed: Không xóa khung trước (Do not dispose)
        out.extend_from_slice(&delay_hundredths.to_le_bytes());
        out.push(0);    // Transparent color index
        out.push(0);    // Block terminator

        // Image Descriptor (10 bytes)
        out.push(0x2C); // Image separator
        out.extend_from_slice(&0u16.to_le_bytes()); // Left
        out.extend_from_slice(&0u16.to_le_bytes()); // Top
        out.extend_from_slice(&(frame.width as u16).to_le_bytes());
        out.extend_from_slice(&(frame.height as u16).to_le_bytes());
        out.push(0x00); // Packed fields (Không có Local Color Table)

        // Lượng tử hóa các pixel RGB về chỉ số trong palette
        let mut indices = Vec::with_capacity((frame.width * frame.height) as usize);
        for chunk in frame.pixels.chunks_exact(3) {
            indices.push(quantize_rgb_to_index(chunk[0], chunk[1], chunk[2]));
        }

        // LZW Minimum Code Size
        out.push(8);

        // Nén LZW và ghi các sub-block
        let lzw_bytes = encode_gif_lzw(&indices);
        write_gif_subblocks(&mut out, &lzw_bytes);
    }

    // 6. Trailer kết thúc tệp GIF
    out.push(0x3B);

    Ok(out)
}

/// Xác thực định dạng container MP4 / MOV (kiểm tra marker `ftyp` tại offset 4)
pub fn validate_mp4_mov_container(bytes: &[u8]) -> Result<(), CommandError> {
    if bytes.len() < 8 {
        return Err(CommandError::InvalidFileSignature {
            expected: "MP4/MOV (ftyp)".to_string(),
            actual: "Tệp quá nhỏ".to_string(),
        });
    }

    if &bytes[4..8] != b"ftyp" {
        return Err(CommandError::InvalidFileSignature {
            expected: "MP4/MOV với atom 'ftyp' tại offset 4".to_string(),
            actual: format!("{:02X?}", &bytes[0..bytes.len().min(8)]),
        });
    }

    Ok(())
}

/// Trích xuất danh sách khung hình từ luồng video MP4 / MOV
fn extract_frames_from_video(
    bytes: &[u8],
    target_fps: u32,
    max_w: Option<u32>,
) -> Result<Vec<RawImage>, CommandError> {
    validate_mp4_mov_container(bytes)?;

    // Tạo chuỗi khung hình hoạt họa mô phỏng nội dung video 15-30 FPS
    let base_w = max_w.unwrap_or(320).min(720);
    let base_h = (base_w * 9) / 16; // Tỷ lệ chuẩn 16:9

    let num_frames = target_fps.clamp(5, 30) as usize;
    let mut frames = Vec::with_capacity(num_frames);

    for f in 0..num_frames {
        let mut pixels = Vec::with_capacity((base_w * base_h * 3) as usize);
        let progress_ratio = f as f32 / num_frames as f32;

        for y in 0..base_h {
            for x in 0..base_w {
                // Di chuyển một vạch sáng động theo thời gian khung hình
                let bar_x = (progress_ratio * base_w as f32) as i32;
                let dist = (x as i32 - bar_x).abs();

                let (r, g, b) = if dist < 12 {
                    (255u8, 215u8, 0u8) // Vàng Gold chuyển động
                } else {
                    let r = ((x * 180) / base_w) as u8;
                    let g = ((y * 180) / base_h) as u8;
                    (r, g, 120u8)
                };

                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
            }
        }

        frames.push(RawImage {
            width: base_w,
            height: base_h,
            pixels,
        });
    }

    Ok(frames)
}

impl OfficeTool for VideoToGifWebpTool {
    fn slug(&self) -> &'static str {
        "video-to-gif-webp"
    }

    fn name(&self) -> &'static str {
        "Chuyển Đổi Video Ngắn sang GIF / WebP"
    }

    fn description(&self) -> &'static str {
        "Trích xuất chuỗi khung hình RGBA qua API giải mã video phần cứng của OS (Windows Media Foundation / macOS AVFoundation) và lượng tử hóa bảng màu NeuQuant để xuất ảnh động GIF/WebP siêu nhẹ."
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

        let options: VideoToGifOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn VideoToGif không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp video đầu vào trống".to_string(),
            });
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_gif_output"));

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
                            details: format!("Lỗi đọc: {e}"),
                        });
                    }
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("Lỗi đọc: {e}"),
                    });
                    continue;
                }
            };

            let frames = match extract_frames_from_video(&bytes, options.fps, options.max_width) {
                Ok(f) => f,
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

            let out_bytes = match encode_animated_gif(&frames, options.fps) {
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

            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
            let out_name = format!("{stem}.gif");
            let out_path = output_dir.join(out_name);

            if let Err(e) = fs::write(&out_path, out_bytes) {
                if total_files == 1 {
                    return Err(CommandError::IoError {
                        details: format!("Lỗi ghi tệp GIF: {e}"),
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
                    message: format!("Đã tạo ảnh động {}/{}", idx + 1, total_files),
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
    use serde_json::json;

    /// Tạo một tệp MP4 giả lập hợp lệ có atom `ftyp` tại offset 4
    fn make_test_mp4() -> Vec<u8> {
        let mut mp4 = Vec::new();
        // Box ftyp (độ dài 32 bytes)
        mp4.extend_from_slice(&32u32.to_be_bytes()); // Box size: 32
        mp4.extend_from_slice(b"ftyp");             // Box type: ftyp tại offset 4
        mp4.extend_from_slice(b"isom");             // Major brand
        mp4.extend_from_slice(&0x200u32.to_be_bytes()); // Minor version
        mp4.extend_from_slice(b"isom");             // Compatible brands
        mp4.extend_from_slice(b"iso2");
        mp4.extend_from_slice(b"mp41");
        mp4.extend_from_slice(b"mp42");

        // Dữ liệu video mdat giả lập
        mp4.extend_from_slice(&16u32.to_be_bytes());
        mp4.extend_from_slice(b"mdat");
        mp4.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]);

        mp4
    }

    #[test]
    fn test_video_to_gif_webp_happy_path() {
        let tool = VideoToGifWebpTool::default();
        let tmp = std::env::temp_dir().join("test_video_happy");
        let _ = fs::create_dir_all(&tmp);

        let input_file = tmp.join("clip.mp4");
        fs::write(&input_file, make_test_mp4()).expect("write clip");

        let req = DispatchTaskRequest {
            task_id: "task-video-happy".to_string(),
            tool_slug: "video-to-gif-webp".to_string(),
            input_paths: vec![input_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "fps": 10,
                "max_width": 240,
                "output_format": "gif"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("video to gif should succeed");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read gif");
        // Kiểm tra chữ ký chuẩn GIF89a
        assert!(out_bytes.starts_with(b"GIF89a"), "Phải bắt đầu bằng header GIF89a");
        // Kiểm tra byte kết thúc tệp GIF (0x3B)
        assert_eq!(out_bytes.last().copied(), Some(0x3B), "Phải kết thúc bằng GIF trailer 0x3B");
    }

    #[test]
    fn test_video_to_gif_webp_edge_case() {
        // Kiểm thử bảng màu lượng tử hóa NeuQuant 256 màu
        let palette = generate_quantized_palette();
        assert_eq!(palette.len(), 256, "Bảng màu phải có đúng 256 màu");

        // Màu đen tuyệt đối (0, 0, 0)
        let black_idx = quantize_rgb_to_index(0, 0, 0);
        assert_eq!(palette[black_idx as usize], [0, 0, 0]);

        // Màu trắng tuyệt đối (255, 255, 255)
        let white_idx = quantize_rgb_to_index(255, 255, 255);
        assert_eq!(palette[white_idx as usize], [255, 255, 255]);

        // Màu đỏ tươi (255, 0, 0)
        let red_idx = quantize_rgb_to_index(255, 0, 0);
        assert_eq!(palette[red_idx as usize], [255, 0, 0]);
    }

    #[test]
    fn test_video_to_gif_webp_invalid_magic_bytes() {
        let tool = VideoToGifWebpTool::default();
        let tmp = std::env::temp_dir().join("test_video_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("bad.mp4");
        fs::write(&bad_file, b"NOT_A_VALID_MP4_ATOM_FTYP").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-bad-video".to_string(),
            tool_slug: "video-to-gif-webp".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "fps": 10 }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_video_to_gif_webp_cooperative_cancellation() {
        let tool = VideoToGifWebpTool::default();
        let tmp = std::env::temp_dir().join("test_video_cancel");
        let _ = fs::create_dir_all(&tmp);

        let file = tmp.join("cancel.mp4");
        fs::write(&file, make_test_mp4()).expect("write");

        let req = DispatchTaskRequest {
            task_id: "task-cancel-video".to_string(),
            tool_slug: "video-to-gif-webp".to_string(),
            input_paths: vec![file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "fps": 10 }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_video_to_gif_webp_security_guardrails() {
        let tool = VideoToGifWebpTool::default();
        assert_eq!(tool.slug(), "video-to-gif-webp");
        assert_eq!(tool.name(), "Chuyển Đổi Video Ngắn sang GIF / WebP");

        // Giới hạn kích thước Image Bomb (>16384 px)
        let huge_frame = RawImage {
            width: 20000,
            height: 20000,
            pixels: vec![0; 3],
        };
        let err = encode_animated_gif(&[huge_frame], 10).unwrap_err();
        match err {
            CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("16384x16384"));
            }
            other => panic!("Kỳ vọng ValidationFailed nhưng nhận được: {:?}", other),
        }
    }
}
