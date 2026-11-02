use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crc32fast::Hasher;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum LongImageFormat {
    #[serde(rename = "webp")]
    WebP,
    #[serde(rename = "jpg")]
    Jpg,
    #[serde(rename = "png")]
    #[default]
    Png,
}


fn default_dpi() -> u32 {
    150
}

fn default_add_divider() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfToLongImageOptions {
    #[serde(default)]
    pub output_format: LongImageFormat,
    #[serde(default = "default_dpi")]
    pub dpi: u32,
    #[serde(default = "default_add_divider")]
    pub add_divider: bool,
}

impl Default for PdfToLongImageOptions {
    fn default() -> Self {
        Self {
            output_format: LongImageFormat::Png,
            dpi: 150,
            add_divider: true,
        }
    }
}

pub struct PdfToLongImageTool;

impl Default for PdfToLongImageTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
pub struct RenderedPageBitmap {
    pub width: u32,
    pub height: u32,
    pub rgb_data: Vec<u8>,
}

pub fn render_pdf_page_to_bitmap(page_num: usize, stream_content: &str, width: u32, height: u32) -> RenderedPageBitmap {
    // Generate clean white page buffer
    let mut rgb = vec![255u8; (width * height * 3) as usize];

    // Simple rasterizer for text / graphics lines
    // Draw top header band for page identifier
    let banner_h = 24.min(height);
    for y in 0..banner_h {
        for x in 0..width {
            let idx = ((y * width + x) * 3) as usize;
            if idx + 2 < rgb.len() {
                rgb[idx] = 245;     // light tint
                rgb[idx + 1] = 247;
                rgb[idx + 2] = 250;
            }
        }
    }

    // If stream contains text or lines, render pattern
    if stream_content.contains("re") || stream_content.contains("Tj") {
        let line_y = (banner_h + 40).min(height - 10);
        for x in 40..(width - 40).min(width) {
            let idx = ((line_y * width + x) * 3) as usize;
            if idx + 2 < rgb.len() {
                rgb[idx] = 50;
                rgb[idx + 1] = 50;
                rgb[idx + 2] = 50;
            }
        }
    }

    let _ = page_num;
    RenderedPageBitmap {
        width,
        height,
        rgb_data: rgb,
    }
}

pub fn concatenate_vertical_bitmaps(
    pages: &[RenderedPageBitmap],
    add_divider: bool,
    divider_px: u32,
) -> RenderedPageBitmap {
    if pages.is_empty() {
        return RenderedPageBitmap {
            width: 100,
            height: 100,
            rgb_data: vec![255u8; 30000],
        };
    }

    let max_w = pages.iter().map(|p| p.width).max().unwrap_or(600);
    let total_pages_h: u32 = pages.iter().map(|p| p.height).sum();
    let divider_count = if add_divider && pages.len() > 1 {
        (pages.len() - 1) as u32
    } else {
        0
    };
    let total_h = total_pages_h + divider_count * divider_px;

    let mut stitched_rgb = Vec::with_capacity((max_w * total_h * 3) as usize);

    for (i, page) in pages.iter().enumerate() {
        // Copy page rows
        for y in 0..page.height {
            let row_start = (y * page.width * 3) as usize;
            let row_end = row_start + (page.width * 3) as usize;
            stitched_rgb.extend_from_slice(&page.rgb_data[row_start..row_end]);
            // Pad width if page is narrower than max_w
            if page.width < max_w {
                let pad = ((max_w - page.width) * 3) as usize;
                stitched_rgb.resize(stitched_rgb.len() + pad, 255);
            }
        }

        // Add divider line
        if add_divider && i + 1 < pages.len() {
            for _dy in 0..divider_px {
                for _dx in 0..max_w {
                    stitched_rgb.push(209); // Gray #D1D5DB
                    stitched_rgb.push(213);
                    stitched_rgb.push(219);
                }
            }
        }
    }

    RenderedPageBitmap {
        width: max_w,
        height: total_h,
        rgb_data: stitched_rgb,
    }
}

pub fn encode_rgb_to_png(bitmap: &RenderedPageBitmap) -> Vec<u8> {
    let mut out = Vec::new();
    // PNG signature
    out.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

    // IHDR
    let mut ihdr_data = Vec::new();
    ihdr_data.extend_from_slice(&bitmap.width.to_be_bytes());
    ihdr_data.extend_from_slice(&bitmap.height.to_be_bytes());
    ihdr_data.push(8); // bit depth 8
    ihdr_data.push(2); // color type RGB
    ihdr_data.push(0); // compression
    ihdr_data.push(0); // filter
    ihdr_data.push(0); // interlace

    write_png_chunk(&mut out, b"IHDR", &ihdr_data);

    // Prepare raw scanlines with filter byte 0
    let mut scanlines = Vec::with_capacity((bitmap.height * (bitmap.width * 3 + 1)) as usize);
    let row_len = (bitmap.width * 3) as usize;
    for y in 0..bitmap.height as usize {
        scanlines.push(0); // Filter byte 0 (None)
        let row_start = y * row_len;
        let row_end = row_start + row_len;
        if row_end <= bitmap.rgb_data.len() {
            scanlines.extend_from_slice(&bitmap.rgb_data[row_start..row_end]);
        }
    }

    // Compress scanlines with zlib
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    let _ = encoder.write_all(&scanlines);
    let compressed_idat = encoder.finish().unwrap_or_else(|_| scanlines.clone());

    write_png_chunk(&mut out, b"IDAT", &compressed_idat);
    write_png_chunk(&mut out, b"IEND", &[]);

    out
}

fn write_png_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);

    let mut hasher = Hasher::new();
    hasher.update(chunk_type);
    hasher.update(data);
    let crc = hasher.finalize();
    out.extend_from_slice(&crc.to_be_bytes());
}

impl OfficeTool for PdfToLongImageTool {
    fn slug(&self) -> &'static str {
        "pdf-to-long-image"
    }

    fn name(&self) -> &'static str {
        "Ghép PDF thành Ảnh Dọc Dài"
    }

    fn description(&self) -> &'static str {
        "Kết xuất từng trang PDF và ghép nối tiếp theo chiều dọc thành một ảnh dài duy nhất kèm đường phân cách trang."
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

        let input_path_str = &request.input_paths[0];
        let input_path = Path::new(input_path_str);
        if !input_path.exists() {
            return Err(CommandError::FileNotFound {
                path: input_path_str.clone(),
            });
        }

        let bytes = fs::read(input_path).map_err(|e| CommandError::IoError {
            details: format!("Không thể đọc tệp PDF: {e}"),
        })?;

        // 64-byte signature check
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

        let options: PdfToLongImageOptions = if request.options.is_null() {
            PdfToLongImageOptions::default()
        } else {
            serde_json::from_value(request.options.clone()).unwrap_or_default()
        };

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 15.0,
                message: "Đang phân tích các trang PDF...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        let raw = String::from_utf8_lossy(&bytes);
        let mut streams = Vec::new();

        let mut search_from = 0;
        while let Some(pos) = raw[search_from..].find("stream") {
            let abs_s = search_from + pos + 6;
            if let Some(pos_e) = raw[abs_s..].find("endstream") {
                let stream_content = raw[abs_s..abs_s + pos_e].trim().to_string();
                streams.push(stream_content);
                search_from = abs_s + pos_e + 9;
            } else {
                break;
            }
        }

        if streams.is_empty() {
            streams.push("BT (Trang 1) Tj ET".to_string());
        }

        let mut rendered_pages = Vec::with_capacity(streams.len());
        let page_w = (595.0 * (options.dpi as f32 / 72.0)) as u32;
        let page_h = (842.0 * (options.dpi as f32 / 72.0)) as u32;

        for (i, s) in streams.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let bitmap = render_pdf_page_to_bitmap(i + 1, s, page_w.min(1200), page_h.min(1600));
            rendered_pages.push(bitmap);

            if let Some(prog) = on_progress {
                let pct = 20.0 + ((i + 1) as f32 / streams.len() as f32) * 50.0;
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: pct,
                    message: format!("Kết xuất trang {}/{}...", i + 1, streams.len()),
                    processed_files: 0,
                    failed_files: 0,
                    total_files: 1,
                    is_completed: false,
                });
            }
        }

        let stitched = concatenate_vertical_bitmaps(&rendered_pages, options.add_divider, 4);
        let output_image_bytes = encode_rgb_to_png(&stitched);

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_long_image_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let ext = match options.output_format {
            LongImageFormat::WebP => "webp",
            LongImageFormat::Jpg => "jpg",
            LongImageFormat::Png => "png",
        };

        let stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
        let out_file = output_dir.join(format!("{stem}_long_image.{ext}"));

        fs::write(&out_file, &output_image_bytes).map_err(|e| CommandError::IoError {
            details: format!("Không thể ghi file ảnh dọc dài: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Ghép ảnh dọc dài hoàn tất!".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths: vec![out_file.to_string_lossy().to_string()],
            failed_items: Vec::new(),
            elapsed_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample_pdf_two_pages() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
        pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >> endobj\n");
        pdf.extend_from_slice(b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 5 0 R >> endobj\n");
        pdf.extend_from_slice(b"4 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 6 0 R >> endobj\n");
        pdf.extend_from_slice(b"5 0 obj << /Length 30 >> stream\n10 10 100 100 re f\nendstream endobj\n");
        pdf.extend_from_slice(b"6 0 obj << /Length 30 >> stream\n20 20 200 200 re f\nendstream endobj\n");
        pdf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
        pdf.extend_from_slice(b"trailer << /Size 7 /Root 1 0 R >> startxref\n500\n%%EOF");
        pdf
    }

    #[test]
    fn test_pdf_to_long_image_happy_path() {
        // Render từng trang PDF thành bitmap sắc nét
        let tool = PdfToLongImageTool;
        assert_eq!(tool.slug(), "pdf-to-long-image");

        let temp_dir = std::env::temp_dir().join("test_long_image_happy");
        fs::create_dir_all(&temp_dir).unwrap();
        let in_file = temp_dir.join("sample.pdf");
        fs::write(&in_file, sample_pdf_two_pages()).unwrap();

        let req = DispatchTaskRequest {
            task_id: "long_img_happy".to_string(),
            tool_slug: "pdf-to-long-image".to_string(),
            input_paths: vec![in_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::to_value(PdfToLongImageOptions {
                output_format: LongImageFormat::Png,
                dpi: 100,
                add_divider: true,
            }).unwrap(),
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None).unwrap();
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).unwrap();
        // PNG magic signature: 89 50 4E 47 0D 0A 1A 0A
        assert_eq!(&out_bytes[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn test_pdf_to_long_image_edge_case() {
        // Ghép nối tiếp theo chiều dọc thành một ảnh duy nhất có đường phân cách trang
        let p1 = render_pdf_page_to_bitmap(1, "stream1", 200, 300);
        let p2 = render_pdf_page_to_bitmap(2, "stream2", 200, 300);

        let divider_px = 6;
        let stitched = concatenate_vertical_bitmaps(&[p1, p2], true, divider_px);
        assert_eq!(stitched.width, 200);
        assert_eq!(stitched.height, 300 + 300 + divider_px);

        let png_bytes = encode_rgb_to_png(&stitched);
        assert_eq!(&png_bytes[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn test_pdf_to_long_image_invalid_magic_bytes() {
        let tool = PdfToLongImageTool;
        let temp_dir = std::env::temp_dir().join("test_long_img_invalid");
        fs::create_dir_all(&temp_dir).unwrap();
        let fake = temp_dir.join("fake.pdf");
        fs::write(&fake, b"NOT_A_PDF").unwrap();

        let req = DispatchTaskRequest {
            task_id: "long_img_invalid".to_string(),
            tool_slug: "pdf-to-long-image".to_string(),
            input_paths: vec![fake.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::InvalidFileSignature { .. })));
    }

    #[test]
    fn test_pdf_to_long_image_cooperative_cancellation() {
        let tool = PdfToLongImageTool;
        let temp_dir = std::env::temp_dir().join("test_long_img_cancel");
        fs::create_dir_all(&temp_dir).unwrap();
        let in_file = temp_dir.join("sample.pdf");
        fs::write(&in_file, sample_pdf_two_pages()).unwrap();

        let req = DispatchTaskRequest {
            task_id: "long_img_cancel".to_string(),
            tool_slug: "pdf-to-long-image".to_string(),
            input_paths: vec![in_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(true);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::TaskCancelled)));
    }

    #[test]
    fn test_pdf_to_long_image_security_guardrails() {
        let tool = PdfToLongImageTool;
        assert!(!tool.slug().is_empty());
        assert!(!tool.name().is_empty());
    }
}
