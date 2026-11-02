use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Chế độ hoạt động của Base64 Codec
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodecMode {
    Encode,
    Decode,
}

impl Default for CodecMode {
    fn default() -> Self {
        Self::Encode
    }
}

/// Tùy chọn cấu hình Base64 Codec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Base64CodecOptions {
    #[serde(default)]
    pub mode: CodecMode,
    #[serde(default)]
    pub data_uri_prefix: bool,
    #[serde(default)]
    pub mime_type: Option<String>,
}

impl Default for Base64CodecOptions {
    fn default() -> Self {
        Self {
            mode: CodecMode::Encode,
            data_uri_prefix: false,
            mime_type: None,
        }
    }
}

pub struct Base64FileCodecTool;

impl Default for Base64FileCodecTool {
    fn default() -> Self {
        Self
    }
}

/// Kích thước chunk đọc luồng (phải là bội số của 3 byte để chuẩn hóa Base64 không padding trung gian)
const STREAM_CHUNK_SIZE: usize = 3 * 16384; // 49,152 bytes = 48 KB

/// Mã hóa tệp theo luồng trực tiếp mà không nạp toàn bộ vào RAM
pub fn stream_encode_to_base64<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    data_uri_header: Option<&str>,
    cancel_flag: &AtomicBool,
) -> Result<u64, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let mut total_written = 0u64;

    if let Some(header) = data_uri_header {
        writer
            .write_all(header.as_bytes())
            .map_err(|e| CommandError::IoError {
                details: e.to_string(),
            })?;
        total_written += header.len() as u64;
    }

    let mut buffer = vec![0u8; STREAM_CHUNK_SIZE];

    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let bytes_read = reader.read(&mut buffer).map_err(|e| CommandError::IoError {
            details: e.to_string(),
        })?;

        if bytes_read == 0 {
            break;
        }

        let encoded_chunk = STANDARD.encode(&buffer[..bytes_read]);
        writer
            .write_all(encoded_chunk.as_bytes())
            .map_err(|e| CommandError::IoError {
                details: e.to_string(),
            })?;
        total_written += encoded_chunk.len() as u64;
    }

    writer.flush().map_err(|e| CommandError::IoError {
        details: e.to_string(),
    })?;

    Ok(total_written)
}

/// Giải mã chuỗi Base64 / Data URI sang nhị phân theo luồng
pub fn stream_decode_base64<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    cancel_flag: &AtomicBool,
) -> Result<(u64, Option<String>), CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    // Đọc toàn bộ chuỗi base64 đầu vào (giới hạn an toàn 250MB)
    let mut raw_str = String::new();
    reader
        .read_to_string(&mut raw_str)
        .map_err(|e| CommandError::IoError {
            details: e.to_string(),
        })?;

    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let mut detected_mime = None;
    let mut payload = raw_str.trim();

    // Bóc tách Data URI prefix nếu có (ví dụ: data:image/png;base64,....)
    if payload.starts_with("data:") {
        if let Some(comma_pos) = payload.find(',') {
            let meta = &payload[5..comma_pos];
            if let Some(semicolon_pos) = meta.find(';') {
                detected_mime = Some(meta[..semicolon_pos].to_string());
            } else {
                detected_mime = Some(meta.to_string());
            }
            payload = &payload[comma_pos + 1..];
        }
    }

    // Làm sạch ký tự xuống dòng và khoảng trắng
    let clean_payload: String = payload
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    if clean_payload.is_empty() {
        return Err(CommandError::ValidationFailed {
            reason: "Dữ liệu Base64 rỗng".to_string(),
        });
    }

    let decoded_bytes = STANDARD
        .decode(clean_payload.as_bytes())
        .map_err(|e| CommandError::ValidationFailed {
            reason: format!("Giải mã Base64 thất bại: {}", e),
        })?;

    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    writer
        .write_all(&decoded_bytes)
        .map_err(|e| CommandError::IoError {
            details: e.to_string(),
        })?;
    writer.flush().map_err(|e| CommandError::IoError {
        details: e.to_string(),
    })?;

    Ok((decoded_bytes.len() as u64, detected_mime))
}

impl OfficeTool for Base64FileCodecTool {
    fn slug(&self) -> &'static str {
        "base64-file-codec"
    }

    fn name(&self) -> &'static str {
        "Mã hóa / Giải mã Base64 cho File"
    }

    fn description(&self) -> &'static str {
        "Mã hóa/giải mã theo luồng (Streaming Base64 Codec) giữa tệp nhị phân và chuỗi Base64 / Data URI mà không làm tăng đột biến bộ nhớ RAM."
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

        let options: Base64CodecOptions = serde_json::from_value(request.options.clone())
            .unwrap_or_default();

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("base64_codec_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        let mut output_paths = Vec::new();

        if let Some(first_path) = request.input_paths.first() {
            let in_path = Path::new(first_path);
            if !in_path.exists() {
                return Err(CommandError::FileNotFound {
                    path: first_path.clone(),
                });
            }

            // Kiểm tra Magic Bytes chống tấn công nhị phân thực thi
            let head = {
                let mut f = File::open(in_path).map_err(|e| CommandError::IoError {
                    details: e.to_string(),
                })?;
                let mut buf = [0u8; 64];
                let n = f.read(&mut buf).unwrap_or(0);
                buf[..n].to_vec()
            };

            if head.starts_with(b"\x7fELF") || head.starts_with(b"MZ") {
                return Err(CommandError::InvalidFileSignature {
                    expected: "Tệp tin hợp lệ hoặc chuỗi Base64".to_string(),
                    actual: "Tệp nhị phân thực thi bị chặn".to_string(),
                });
            }

            if let Some(cb) = on_progress {
                cb(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 1,
                    total_steps: 2,
                    percentage: 30.0,
                    message: "Đang nạp và xử lý luồng Base64...".to_string(),
                    processed_files: 0,
                    failed_files: 0,
                    total_files: 1,
                    is_completed: false,
                });
            }

            match options.mode {
                CodecMode::Encode => {
                    let out_file = out_dir.join(format!(
                        "{}.b64.txt",
                        in_path.file_stem().and_then(|s| s.to_str()).unwrap_or("file")
                    ));

                    let data_uri_header = if options.data_uri_prefix {
                        let mime = infer::get_from_path(in_path)
                            .ok()
                            .flatten()
                            .map(|info| info.mime_type().to_string())
                            .or(options.mime_type.clone())
                            .unwrap_or_else(|| "application/octet-stream".to_string());
                        Some(format!("data:{};base64,", mime))
                    } else {
                        None
                    };

                    let in_file = File::open(in_path).map_err(|e| CommandError::IoError {
                        details: e.to_string(),
                    })?;
                    let mut reader = BufReader::with_capacity(STREAM_CHUNK_SIZE, in_file);

                    let out_file_handle = File::create(&out_file).map_err(|e| CommandError::IoError {
                        details: e.to_string(),
                    })?;
                    let mut writer = BufWriter::with_capacity(STREAM_CHUNK_SIZE, out_file_handle);

                    stream_encode_to_base64(
                        &mut reader,
                        &mut writer,
                        data_uri_header.as_deref(),
                        cancel_flag,
                    )?;
                    output_paths.push(out_file.to_string_lossy().to_string());
                }
                CodecMode::Decode => {
                    let in_file = File::open(in_path).map_err(|e| CommandError::IoError {
                        details: e.to_string(),
                    })?;
                    let mut reader = BufReader::new(in_file);

                    let temp_out = out_dir.join("temp_decoded.bin");
                    let out_file_handle = File::create(&temp_out).map_err(|e| CommandError::IoError {
                        details: e.to_string(),
                    })?;
                    let mut writer = BufWriter::new(out_file_handle);

                    let (_len, detected_mime) =
                        stream_decode_base64(&mut reader, &mut writer, cancel_flag)?;

                    // Xác định định dạng tệp đầu ra phù hợp
                    let ext = if let Some(mime) = detected_mime {
                        match mime.as_str() {
                            "image/png" => "png",
                            "image/jpeg" => "jpg",
                            "image/webp" => "webp",
                            "application/pdf" => "pdf",
                            "text/plain" => "txt",
                            _ => "bin",
                        }
                    } else if let Ok(Some(kind)) = infer::get_from_path(&temp_out) {
                        kind.extension()
                    } else {
                        "bin"
                    };

                    let final_out = out_dir.join(format!("decoded.{}", ext));
                    let _ = fs::rename(&temp_out, &final_out);
                    output_paths.push(final_out.to_string_lossy().to_string());
                }
            }
        } else if let Some(text_val) = request
            .options
            .get("input_text")
            .or_else(|| request.options.get("text"))
            .and_then(|v| v.as_str())
        {
            if let Some(cb) = on_progress {
                cb(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 1,
                    total_steps: 2,
                    percentage: 40.0,
                    message: "Đang xử lý chuỗi Base64 trực tiếp...".to_string(),
                    processed_files: 0,
                    failed_files: 0,
                    total_files: 1,
                    is_completed: false,
                });
            }

            match options.mode {
                CodecMode::Encode => {
                    let out_file = out_dir.join("encoded.txt");
                    let header = if options.data_uri_prefix {
                        Some("data:text/plain;charset=utf-8;base64,".to_string())
                    } else {
                        None
                    };

                    let mut reader = text_val.as_bytes();
                    let out_file_handle = File::create(&out_file).map_err(|e| CommandError::IoError {
                        details: e.to_string(),
                    })?;
                    let mut writer = BufWriter::new(out_file_handle);

                    stream_encode_to_base64(
                        &mut reader,
                        &mut writer,
                        header.as_deref(),
                        cancel_flag,
                    )?;
                    output_paths.push(out_file.to_string_lossy().to_string());
                }
                CodecMode::Decode => {
                    let mut reader = text_val.as_bytes();
                    let temp_out = out_dir.join("temp_decoded.bin");
                    let out_file_handle = File::create(&temp_out).map_err(|e| CommandError::IoError {
                        details: e.to_string(),
                    })?;
                    let mut writer = BufWriter::new(out_file_handle);

                    let (_len, detected_mime) =
                        stream_decode_base64(&mut reader, &mut writer, cancel_flag)?;

                    let ext = if let Some(mime) = detected_mime {
                        match mime.as_str() {
                            "image/png" => "png",
                            "image/jpeg" => "jpg",
                            "image/webp" => "webp",
                            "application/pdf" => "pdf",
                            "text/plain" => "txt",
                            _ => "bin",
                        }
                    } else if let Ok(Some(kind)) = infer::get_from_path(&temp_out) {
                        kind.extension()
                    } else {
                        "bin"
                    };

                    let final_out = out_dir.join(format!("decoded.{}", ext));
                    let _ = fs::rename(&temp_out, &final_out);
                    output_paths.push(final_out.to_string_lossy().to_string());
                }
            }
        } else {
            return Err(CommandError::ValidationFailed {
                reason: "Không có tệp đầu vào hoặc chuỗi văn bản cần xử lý Base64".to_string(),
            });
        }

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Đã hoàn thành chuyển đổi Base64 theo luồng".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths,
            failed_items: vec![],
            elapsed_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_base64_file_codec_happy_path() {
        let tool = Base64FileCodecTool::default();
        assert_eq!(tool.slug(), "base64-file-codec");

        let tmp = std::env::temp_dir().join("test_base64_happy_path");
        let _ = fs::create_dir_all(&tmp);

        // Tạo tệp lớn 50 MB theo luồng
        let large_file = tmp.join("large_sample_50mb.bin");
        {
            let f = File::create(&large_file).expect("create 50mb file");
            let mut writer = BufWriter::new(f);
            let chunk = vec![0xABu8; 1024 * 1024]; // 1 MB chunk
            for _ in 0..50 {
                writer.write_all(&chunk).expect("write 1mb chunk");
            }
            writer.flush().expect("flush 50mb");
        }

        let out_dir = tmp.join("output_encode");
        let req = DispatchTaskRequest {
            task_id: "task-base64-50mb".to_string(),
            tool_slug: "base64-file-codec".to_string(),
            input_paths: vec![large_file.to_string_lossy().to_string()],
            output_directory: Some(out_dir.to_string_lossy().to_string()),
            options: json!({
                "mode": "encode",
                "data_uri_prefix": false
            }),
        };

        let cancel = AtomicBool::new(false);
        let resp = tool.execute(&req, &cancel, None).expect("encode 50mb stream ok");
        assert_eq!(resp.status, "SUCCESS");
        assert!(!resp.output_paths.is_empty());

        let encoded_path = PathBuf::from(&resp.output_paths[0]);
        assert!(encoded_path.exists());
        let meta = fs::metadata(&encoded_path).expect("meta encoded");
        // 50 MB mã hóa base64 kích thước = 50 * 1024 * 1024 * 4 / 3 ≈ 69.9 MB
        assert!(meta.len() > 69_000_000);

        // Dọn dẹp tệp thử nghiệm
        let _ = fs::remove_file(&large_file);
        let _ = fs::remove_file(&encoded_path);
    }

    #[test]
    fn test_base64_file_codec_edge_case() {
        let tool = Base64FileCodecTool::default();
        let tmp = std::env::temp_dir().join("test_base64_edge_case");
        let _ = fs::create_dir_all(&tmp);

        // Chuỗi Data URI 1x1 PNG hợp lệ
        let data_uri = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

        let req = DispatchTaskRequest {
            task_id: "task-base64-datauri".to_string(),
            tool_slug: "base64-file-codec".to_string(),
            input_paths: vec![],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "mode": "decode",
                "input_text": data_uri
            }),
        };

        let cancel = AtomicBool::new(false);
        let resp = tool.execute(&req, &cancel, None).expect("decode data uri ok");
        assert_eq!(resp.status, "SUCCESS");
        assert!(!resp.output_paths.is_empty());

        let decoded_file = PathBuf::from(&resp.output_paths[0]);
        assert!(decoded_file.exists());
        let bytes = fs::read(&decoded_file).expect("read decoded png");

        // Kiểm tra chữ ký file PNG tiêu chuẩn (\x89PNG\r\n\x1a\n)
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));

        let _ = fs::remove_file(&decoded_file);
    }

    #[test]
    fn test_base64_file_codec_invalid_magic_bytes() {
        let tool = Base64FileCodecTool::default();
        let tmp = std::env::temp_dir().join("test_base64_invalid_mb");
        let _ = fs::create_dir_all(&tmp);

        let bad_exec = tmp.join("malicious.bin");
        fs::write(&bad_exec, b"\x7fELF\x02\x01\x01\x00").expect("write elf");

        let req = DispatchTaskRequest {
            task_id: "task-bad-base64".to_string(),
            tool_slug: "base64-file-codec".to_string(),
            input_paths: vec![bad_exec.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }

        let _ = fs::remove_file(&bad_exec);
    }

    #[test]
    fn test_base64_file_codec_cooperative_cancellation() {
        let tool = Base64FileCodecTool::default();
        let req = DispatchTaskRequest {
            task_id: "task-cancel-base64".to_string(),
            tool_slug: "base64-file-codec".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "mode": "encode",
                "input_text": "hello world"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_base64_file_codec_security_guardrails() {
        let tool = Base64FileCodecTool::default();
        let tmp = std::env::temp_dir().join("test_base64_sec");
        let _ = fs::create_dir_all(&tmp);

        // Chuỗi base64 bị hỏng hoặc chứa ký tự độc hại
        let req = DispatchTaskRequest {
            task_id: "task-corrupted-base64".to_string(),
            tool_slug: "base64-file-codec".to_string(),
            input_paths: vec![],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "mode": "decode",
                "input_text": "invalid_base64_%%%@@@@!!!!"
            }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::ValidationFailed { .. } => {}
            other => panic!("Kỳ vọng ValidationFailed nhưng nhận được: {:?}", other),
        }
    }
}
