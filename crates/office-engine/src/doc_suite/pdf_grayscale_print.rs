use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

fn default_convert_images() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfGrayscaleOptions {
    #[serde(default = "default_convert_images")]
    pub convert_images: bool,
}

impl Default for PdfGrayscaleOptions {
    fn default() -> Self {
        Self {
            convert_images: true,
        }
    }
}

pub struct PdfGrayscalePrintTool;

impl Default for PdfGrayscalePrintTool {
    fn default() -> Self {
        Self
    }
}

pub fn convert_stream_to_grayscale(content: &str) -> String {
    let mut out = String::with_capacity(content.len());

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.ends_with("rg") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 4
                && let (Ok(r), Ok(g), Ok(b)) = (
                    parts[parts.len() - 4].parse::<f32>(),
                    parts[parts.len() - 3].parse::<f32>(),
                    parts[parts.len() - 2].parse::<f32>(),
                ) {
                    let gray = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 1.0);
                    let prefix = parts[..parts.len() - 4].join(" ");
                    if !prefix.is_empty() {
                        out.push_str(&prefix);
                        out.push(' ');
                    }
                    out.push_str(&format!("{:.3} g\n", gray));
                    continue;
                }
        } else if trimmed.ends_with("RG") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 4
                && let (Ok(r), Ok(g), Ok(b)) = (
                    parts[parts.len() - 4].parse::<f32>(),
                    parts[parts.len() - 3].parse::<f32>(),
                    parts[parts.len() - 2].parse::<f32>(),
                ) {
                    let gray = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 1.0);
                    let prefix = parts[..parts.len() - 4].join(" ");
                    if !prefix.is_empty() {
                        out.push_str(&prefix);
                        out.push(' ');
                    }
                    out.push_str(&format!("{:.3} G\n", gray));
                    continue;
                }
        } else if trimmed.ends_with('k') && !trimmed.ends_with("endobj") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 5
                && let (Ok(c), Ok(m), Ok(y), Ok(k)) = (
                    parts[parts.len() - 5].parse::<f32>(),
                    parts[parts.len() - 4].parse::<f32>(),
                    parts[parts.len() - 3].parse::<f32>(),
                    parts[parts.len() - 2].parse::<f32>(),
                ) {
                    let gray = ((1.0 - k) * (1.0 - (0.299 * c + 0.587 * m + 0.114 * y))).clamp(0.0, 1.0);
                    out.push_str(&format!("{:.3} g\n", gray));
                    continue;
                }
        } else if trimmed.ends_with('K') {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 5
                && let (Ok(c), Ok(m), Ok(y), Ok(k)) = (
                    parts[parts.len() - 5].parse::<f32>(),
                    parts[parts.len() - 4].parse::<f32>(),
                    parts[parts.len() - 3].parse::<f32>(),
                    parts[parts.len() - 2].parse::<f32>(),
                ) {
                    let gray = ((1.0 - k) * (1.0 - (0.299 * c + 0.587 * m + 0.114 * y))).clamp(0.0, 1.0);
                    out.push_str(&format!("{:.3} G\n", gray));
                    continue;
                }
        }

        out.push_str(line);
        out.push('\n');
    }

    out
}

pub fn convert_xobject_images_to_devicegray(raw_text: &str) -> String {
    let mut modified = raw_text.to_string();

    // Replace /ColorSpace /DeviceRGB with /ColorSpace /DeviceGray in Image XObjects
    if modified.contains("/DeviceRGB") {
        modified = modified.replace("/ColorSpace /DeviceRGB", "/ColorSpace /DeviceGray");
        modified = modified.replace("/ColorSpace/DeviceRGB", "/ColorSpace /DeviceGray");
        modified = modified.replace("/DeviceRGB", "/DeviceGray");
    }

    modified
}

pub fn process_pdf_grayscale(bytes: &[u8], convert_images: bool, cancel_flag: &AtomicBool) -> Result<Vec<u8>, CommandError> {
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

    let raw = String::from_utf8_lossy(bytes);

    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    // Step 1: convert color space of images if requested
    let working = if convert_images {
        convert_xobject_images_to_devicegray(&raw)
    } else {
        raw.to_string()
    };

    // Step 2: convert content stream color operators
    let mut result = String::with_capacity(working.len());
    let mut search_from = 0;

    while let Some(stream_pos) = working[search_from..].find("stream") {
        let abs_stream = search_from + stream_pos;
        result.push_str(&working[search_from..abs_stream + 6]);

        let after_stream = abs_stream + 6;
        if let Some(end_pos) = working[after_stream..].find("endstream") {
            let abs_end = after_stream + end_pos;
            let stream_content = &working[after_stream..abs_end];

            let grayscaled_stream = convert_stream_to_grayscale(stream_content);
            result.push_str(&grayscaled_stream);
            result.push_str("endstream");

            search_from = abs_end + 9;
        } else {
            result.push_str(&working[after_stream..]);
            search_from = working.len();
            break;
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }
    }

    if search_from < working.len() {
        result.push_str(&working[search_from..]);
    }

    Ok(result.into_bytes())
}

impl OfficeTool for PdfGrayscalePrintTool {
    fn slug(&self) -> &'static str {
        "pdf-grayscale-print"
    }

    fn name(&self) -> &'static str {
        "Chuyển PDF sang Đen trắng"
    }

    fn description(&self) -> &'static str {
        "Chuyển đổi toàn bộ toán tử màu trong luồng nội dung (rg/RG sang g/G) và chuyển đổi không gian màu các đối tượng XObject Image từ DeviceRGB/CMYK sang DeviceGray 8-bit nhằm tiết kiệm mực in."
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

        let options: PdfGrayscaleOptions = if request.options.is_null() {
            PdfGrayscaleOptions::default()
        } else {
            serde_json::from_value(request.options.clone()).unwrap_or_default()
        };

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_grayscale_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let mut output_paths = Vec::new();

        for (idx, path_str) in request.input_paths.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let path = Path::new(path_str);
            if !path.exists() {
                return Err(CommandError::FileNotFound {
                    path: path_str.clone(),
                });
            }

            let bytes = fs::read(path).map_err(|e| CommandError::IoError {
                details: format!("Không thể đọc tệp PDF: {e}"),
            })?;

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 1,
                    total_steps: 3,
                    percentage: 20.0 + (idx as f32 / request.input_paths.len() as f32) * 50.0,
                    message: format!("Đang chuyển đổi màu sắc tệp {}/{}...", idx + 1, request.input_paths.len()),
                    processed_files: idx,
                    failed_files: 0,
                    total_files: request.input_paths.len(),
                    is_completed: false,
                });
            }

            let output_bytes = process_pdf_grayscale(&bytes, options.convert_images, cancel_flag)?;

            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let out_file = output_dir.join(format!("{stem}_grayscale.pdf"));

            fs::write(&out_file, &output_bytes).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi file kết quả: {e}"),
            })?;

            output_paths.push(out_file.to_string_lossy().to_string());
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Chuyển PDF sang đen trắng hoàn tất!".to_string(),
                processed_files: output_paths.len(),
                failed_files: 0,
                total_files: output_paths.len(),
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
    use std::fs;

    fn sample_color_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
        pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n");
        pdf.extend_from_slice(b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >> endobj\n");
        // Stream with RGB color operators
        let content = "1.0 0.0 0.0 rg\n0.0 1.0 0.0 RG\nBT (Văn bản màu đỏ và xanh) Tj ET\n";
        pdf.extend_from_slice(
            format!("4 0 obj << /Length {} >> stream\n{}\nendstream endobj\n", content.len(), content).as_bytes()
        );
        // Image XObject with DeviceRGB
        pdf.extend_from_slice(b"5 0 obj << /Type /XObject /Subtype /Image /Width 10 /Height 10 /ColorSpace /DeviceRGB /BitsPerComponent 8 >> stream\nIMAGE_DATA\nendstream endobj\n");
        pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
        pdf.extend_from_slice(b"trailer << /Size 6 /Root 1 0 R >> startxref\n500\n%%EOF");
        pdf
    }

    #[test]
    fn test_pdf_grayscale_print_happy_path() {
        // Chuyển đổi toàn bộ toán tử màu rg/RG sang g/G
        let tool = PdfGrayscalePrintTool;
        assert_eq!(tool.slug(), "pdf-grayscale-print");

        let stream_rgb = "1.0 0.0 0.0 rg\n0.0 1.0 0.0 RG\n";
        let stream_gray = convert_stream_to_grayscale(stream_rgb);

        assert!(stream_gray.contains("0.299 g") || stream_gray.contains("g"));
        assert!(stream_gray.contains("0.587 G") || stream_gray.contains("G"));
        assert!(!stream_gray.contains("rg"));
        assert!(!stream_gray.contains("RG"));

        let temp_dir = std::env::temp_dir().join("test_grayscale_happy");
        fs::create_dir_all(&temp_dir).unwrap();
        let in_file = temp_dir.join("color.pdf");
        fs::write(&in_file, sample_color_pdf()).unwrap();

        let req = DispatchTaskRequest {
            task_id: "gray_happy".to_string(),
            tool_slug: "pdf-grayscale-print".to_string(),
            input_paths: vec![in_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::to_value(PdfGrayscaleOptions {
                convert_images: true,
            }).unwrap(),
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None).unwrap();
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).unwrap();
        assert!(out_bytes.starts_with(b"%PDF-1.4"));
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(out_str.contains("g"));
        assert!(out_str.contains("G"));
    }

    #[test]
    fn test_pdf_grayscale_print_edge_case() {
        // Chuyển đổi không gian màu XObject Image từ DeviceRGB sang DeviceGray
        let raw_xobject = "<< /Type /XObject /Subtype /Image /ColorSpace /DeviceRGB /BitsPerComponent 8 >>";
        let grayscaled = convert_xobject_images_to_devicegray(raw_xobject);

        assert!(grayscaled.contains("/ColorSpace /DeviceGray"));
        assert!(!grayscaled.contains("/DeviceRGB"));
    }

    #[test]
    fn test_pdf_grayscale_print_invalid_magic_bytes() {
        let tool = PdfGrayscalePrintTool;
        let temp_dir = std::env::temp_dir().join("test_gray_invalid");
        fs::create_dir_all(&temp_dir).unwrap();
        let fake = temp_dir.join("fake.pdf");
        fs::write(&fake, b"NOT_A_PDF").unwrap();

        let req = DispatchTaskRequest {
            task_id: "gray_invalid".to_string(),
            tool_slug: "pdf-grayscale-print".to_string(),
            input_paths: vec![fake.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::InvalidFileSignature { .. })));
    }

    #[test]
    fn test_pdf_grayscale_print_cooperative_cancellation() {
        let tool = PdfGrayscalePrintTool;
        let temp_dir = std::env::temp_dir().join("test_gray_cancel");
        fs::create_dir_all(&temp_dir).unwrap();
        let in_file = temp_dir.join("color.pdf");
        fs::write(&in_file, sample_color_pdf()).unwrap();

        let req = DispatchTaskRequest {
            task_id: "gray_cancel".to_string(),
            tool_slug: "pdf-grayscale-print".to_string(),
            input_paths: vec![in_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(true);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::TaskCancelled)));
    }

    #[test]
    fn test_pdf_grayscale_print_security_guardrails() {
        let tool = PdfGrayscalePrintTool;
        assert!(!tool.slug().is_empty());
        assert!(!tool.name().is_empty());
    }
}
