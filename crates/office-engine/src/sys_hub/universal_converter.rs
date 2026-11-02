use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::data_lab::VietnameseFontConverterTool;
use crate::doc_suite::PdfMergeSplitTool;
use crate::media_studio::ImageBatchTool;
use crate::OfficeTool;

/// Cấu hình tùy chọn trạm chuyển đổi tệp vạn năng
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalConverterOptions {
    pub target_extension: String,
}

pub struct UniversalConverterTool {
    pdf_tool: PdfMergeSplitTool,
    image_tool: ImageBatchTool,
    font_tool: VietnameseFontConverterTool,
}

impl Default for UniversalConverterTool {
    fn default() -> Self {
        Self {
            pdf_tool: PdfMergeSplitTool,
            image_tool: ImageBatchTool,
            font_tool: VietnameseFontConverterTool,
        }
    }
}

/// Loại định dạng đã phân tích qua magic bytes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileCategory {
    DocumentPdf,
    ImageMedia(String),
    TextData,
    Unsupported(String),
}

/// Nhận diện loại tệp qua chữ ký magic bytes bằng crate `infer`
pub fn inspect_file_category(path: &Path) -> Result<FileCategory, CommandError> {
    if !path.exists() {
        return Err(CommandError::FileNotFound {
            path: path.to_string_lossy().to_string(),
        });
    }

    let bytes = fs::read(path).map_err(|e| CommandError::IoError {
        details: format!("Không thể đọc tệp: {e}"),
    })?;

    if bytes.is_empty() {
        return Err(CommandError::InvalidFileSignature {
            expected: "valid header".to_string(),
            actual: "Tệp rỗng (0 bytes)".to_string(),
        });
    }

    // Kiểm tra PDF
    if bytes.starts_with(b"%PDF-") {
        return Ok(FileCategory::DocumentPdf);
    }

    // Kiểm tra ảnh bằng infer
    if let Some(kind) = infer::get(&bytes) {
        let mime = kind.mime_type();
        if mime.starts_with("image/") {
            return Ok(FileCategory::ImageMedia(kind.extension().to_string()));
        }
    }

    // Nhận diện thủ công BMP / PPM
    if bytes.starts_with(b"BM") {
        return Ok(FileCategory::ImageMedia("bmp".to_string()));
    }
    if bytes.starts_with(b"P6") {
        return Ok(FileCategory::ImageMedia("ppm".to_string()));
    }

    // Kiểm tra nếu là tệp văn bản hợp lệ UTF-8 không chứa byte null
    if !bytes.contains(&0) && std::str::from_utf8(&bytes).is_ok() {
        return Ok(FileCategory::TextData);
    }

    let sample_hex = bytes
        .iter()
        .take(8)
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(FileCategory::Unsupported(sample_hex))
}

impl OfficeTool for UniversalConverterTool {
    fn slug(&self) -> &'static str {
        "universal-converter"
    }

    fn name(&self) -> &'static str {
        "Trạm Chuyển đổi Tệp Vạn năng"
    }

    fn description(&self) -> &'static str {
        "Tự động nhận diện định dạng tệp thông qua magic bytes (infer) và điều phối tác vụ chuyển đổi đến phân hệ chuyên trách tương ứng."
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

        let options: UniversalConverterOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình UniversalConverter không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp đầu vào trống".to_string(),
            });
        }

        let target_ext = options.target_extension.trim().trim_start_matches('.').to_lowercase();
        if target_ext.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Chưa cung cấp định dạng đầu ra (target_extension)".to_string(),
            });
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Bắt đầu nhận diện magic bytes các tệp đầu vào...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: request.input_paths.len(),
                is_completed: false,
            });
        }

        let mut output_paths = Vec::new();
        let mut failed_items = Vec::new();
        let total_files = request.input_paths.len();

        for (idx, file_path_str) in request.input_paths.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let path = Path::new(file_path_str);
            let category = match inspect_file_category(path) {
                Ok(cat) => cat,
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

            let single_req = DispatchTaskRequest {
                task_id: format!("{}-{}", request.task_id, idx),
                tool_slug: request.tool_slug.clone(),
                input_paths: vec![file_path_str.clone()],
                output_directory: request.output_directory.clone(),
                options: serde_json::json!({}),
            };

            let sub_res: Result<DispatchTaskResponse, CommandError> = match category {
                FileCategory::ImageMedia(_) => {
                    let mut img_req = single_req.clone();
                    img_req.tool_slug = "image-batch".to_string();
                    img_req.options = serde_json::json!({
                        "target_format": target_ext,
                        "quality": 90,
                        "strip_exif": true
                    });
                    self.image_tool.execute(&img_req, cancel_flag, None)
                }

                FileCategory::DocumentPdf => {
                    let mut pdf_req = single_req.clone();
                    pdf_req.tool_slug = "pdf-merge-split".to_string();
                    pdf_req.options = serde_json::json!({
                        "mode": "split",
                        "page_ranges": "1"
                    });
                    self.pdf_tool.execute(&pdf_req, cancel_flag, None)
                }

                FileCategory::TextData => {
                    let mut font_req = single_req.clone();
                    font_req.tool_slug = "vietnamese-font-converter".to_string();
                    font_req.options = serde_json::json!({
                        "source_encoding": "auto",
                        "target_encoding": "unicode"
                    });
                    self.font_tool.execute(&font_req, cancel_flag, None)
                }

                FileCategory::Unsupported(hex_sig) => {
                    let err = CommandError::ValidationFailed {
                        reason: format!(
                            "Định dạng tệp không được hỗ trợ (Magic bytes: {hex_sig}). Chỉ hỗ trợ PDF, Hình ảnh (PNG, JPEG, WebP, BMP) và Văn bản."
                        ),
                    };
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

            match sub_res {
                Ok(resp) => {
                    output_paths.extend(resp.output_paths);
                }
                Err(err) => {
                    if total_files == 1 {
                        return Err(err);
                    }
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("{:?}", err),
                    });
                }
            }

            if let Some(prog) = on_progress {
                let pct = 10.0 + ((idx + 1) as f32 / total_files as f32) * 85.0;
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: pct,
                    message: format!("Đã điều phối và chuyển đổi {}/{} tệp...", idx + 1, total_files),
                    processed_files: idx + 1,
                    failed_files: failed_items.len(),
                    total_files,
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
                message: "Hoàn tất điều phối chuyển đổi vạn năng.".to_string(),
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
    use std::sync::atomic::AtomicBool;
    use serde_json::json;

    fn create_dummy_bmp() -> Vec<u8> {
        let raw = crate::media_studio::image_batch::RawImage {
            width: 16,
            height: 16,
            pixels: vec![120; 16 * 16 * 3],
        };
        crate::media_studio::image_batch::encode_bmp(&raw).expect("encode bmp")
    }

    #[test]
    fn test_universal_converter_happy_path() {
        let tool = UniversalConverterTool::default();
        let tmp = std::env::temp_dir().join("test_univ_happy");
        let _ = fs::create_dir_all(&tmp);

        let img_path = tmp.join("sample.bmp");
        fs::write(&img_path, create_dummy_bmp()).expect("write bmp");

        let req = DispatchTaskRequest {
            task_id: "task-univ-happy".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![img_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "target_extension": "png"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("dispatch ok");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);
        let out_bytes = fs::read(&res.output_paths[0]).expect("read converted");
        assert!(out_bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn test_universal_converter_edge_case() {
        let tool = UniversalConverterTool::default();
        let tmp = std::env::temp_dir().join("test_univ_edge");
        let _ = fs::create_dir_all(&tmp);

        // Tệp định dạng không được hỗ trợ (chứa byte lạ)
        let unk_path = tmp.join("unknown.dat");
        fs::write(&unk_path, b"\x00\x01\x02\x03\x04\x05\x06\x07").expect("write unk");

        let req = DispatchTaskRequest {
            task_id: "task-univ-edge".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![unk_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_extension": "pdf" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("không được hỗ trợ"));
            }
            other => panic!("Kỳ vọng ValidationFailed nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_universal_converter_invalid_magic_bytes() {
        let tool = UniversalConverterTool::default();
        let tmp = std::env::temp_dir().join("test_univ_invalid");
        let _ = fs::create_dir_all(&tmp);

        // Tệp rỗng 0 byte
        let empty_path = tmp.join("empty.bin");
        fs::write(&empty_path, b"").expect("write empty");

        let req = DispatchTaskRequest {
            task_id: "task-univ-empty".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![empty_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_extension": "png" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature với tệp rỗng, nhận: {:?}", other),
        }
    }

    #[test]
    fn test_universal_converter_cooperative_cancellation() {
        let tool = UniversalConverterTool::default();
        let req = DispatchTaskRequest {
            task_id: "task-cancel-univ".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec!["fake.jpg".to_string()],
            output_directory: None,
            options: json!({ "target_extension": "png" }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_universal_converter_security_guardrails() {
        let tool = UniversalConverterTool::default();
        assert_eq!(tool.slug(), "universal-converter");
        assert_eq!(tool.name(), "Trạm Chuyển đổi Tệp Vạn năng");
    }
}
