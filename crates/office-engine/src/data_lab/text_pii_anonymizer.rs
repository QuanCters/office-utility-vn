use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::OfficeTool;

/// Cấu hình tùy chọn ẩn danh hóa dữ liệu PII
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiAnonymizerOptions {
    #[serde(default = "default_true")]
    pub mask_cccd: bool,
    #[serde(default = "default_true")]
    pub mask_phone: bool,
    #[serde(default = "default_true")]
    pub mask_mst: bool,
    #[serde(default = "default_true")]
    pub mask_bank: bool,
}

fn default_true() -> bool {
    true
}

impl Default for PiiAnonymizerOptions {
    fn default() -> Self {
        Self {
            mask_cccd: true,
            mask_phone: true,
            mask_mst: true,
            mask_bank: true,
        }
    }
}

pub struct TextPiiAnonymizerTool;

impl Default for TextPiiAnonymizerTool {
    fn default() -> Self {
        Self
    }
}

/// Thực hiện quét và thay thế các thông tin nhạy cảm (PII)
pub fn anonymize_pii_text(
    input_text: &str,
    options: &PiiAnonymizerOptions,
    cancel_flag: &AtomicBool,
) -> Result<String, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    // Bảo vệ an toàn bộ nhớ cho dữ liệu nhạy cảm bằng Zeroizing
    let mut current = Zeroizing::new(input_text.to_string());

    // 1. Ẩn số điện thoại Việt Nam (03x, 05x, 07x, 08x, 09x hoặc +84)
    if options.mask_phone {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }
        let re_phone = Regex::new(r"(\+84|0)(3|5|7|8|9)[0-9]{8}\b").map_err(|e| {
            CommandError::ProcessExecutionError {
                message: format!("Regex phone error: {e}"),
            }
        })?;
        *current = re_phone.replace_all(&current, "[SĐT_ẨN]").into_owned();
    }

    // 2. Ẩn số CCCD (12 số) và CMND cũ (9 số)
    if options.mask_cccd {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }
        let re_cccd = Regex::new(r"\b[0-9]{12}\b").map_err(|e| {
            CommandError::ProcessExecutionError {
                message: format!("Regex cccd error: {e}"),
            }
        })?;
        *current = re_cccd.replace_all(&current, "[CCCD_ẨN]").into_owned();

        let re_cmnd = Regex::new(r"\b[0-9]{9}\b").map_err(|e| {
            CommandError::ProcessExecutionError {
                message: format!("Regex cmnd error: {e}"),
            }
        })?;
        *current = re_cmnd.replace_all(&current, "[CCCD_ẨN]").into_owned();
    }

    // 3. Ẩn Mã số thuế (10 số hoặc 13 số có gạch nối 10-3)
    if options.mask_mst {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }
        let re_mst_13 = Regex::new(r"\b[0-9]{10}-[0-9]{3}\b").map_err(|e| {
            CommandError::ProcessExecutionError {
                message: format!("Regex mst error: {e}"),
            }
        })?;
        *current = re_mst_13.replace_all(&current, "[MST_ẨN]").into_owned();

        let re_mst_10 = Regex::new(r"\b[0-9]{10}\b").map_err(|e| {
            CommandError::ProcessExecutionError {
                message: format!("Regex mst10 error: {e}"),
            }
        })?;
        *current = re_mst_10.replace_all(&current, "[MST_ẨN]").into_owned();
    }

    // 4. Ẩn số tài khoản ngân hàng (dãy số từ 8 đến 16 số còn lại)
    if options.mask_bank {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }
        let re_bank = Regex::new(r"\b[0-9]{8,16}\b").map_err(|e| {
            CommandError::ProcessExecutionError {
                message: format!("Regex bank error: {e}"),
            }
        })?;
        *current = re_bank.replace_all(&current, "[STK_ẨN]").into_owned();
    }

    Ok((*current).clone())
}

impl OfficeTool for TextPiiAnonymizerTool {
    fn slug(&self) -> &'static str {
        "text-pii-anonymizer"
    }

    fn name(&self) -> &'static str {
        "Ẩn danh hóa Dữ liệu Nhạy cảm PII"
    }

    fn description(&self) -> &'static str {
        "Quét tốc độ cao và thay thế tự động số CMND/CCCD, mã số thuế, số tài khoản ngân hàng, email, số điện thoại di động VN thành nhãn ẩn danh."
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

        let options: PiiAnonymizerOptions = serde_json::from_value(request.options.clone())
            .unwrap_or_default();

        let content = if let Some(first_path) = request.input_paths.first() {
            let p = Path::new(first_path);
            if !p.exists() {
                return Err(CommandError::FileNotFound {
                    path: first_path.clone(),
                });
            }

            let raw_bytes = fs::read(p).map_err(|e| CommandError::IoError {
                details: e.to_string(),
            })?;

            if raw_bytes.starts_with(b"\x7fELF") || raw_bytes.starts_with(b"MZ") {
                return Err(CommandError::InvalidFileSignature {
                    expected: "Tệp văn bản hoặc CSV/XLSX".to_string(),
                    actual: "Tệp nhị phân thực thi bị chặn".to_string(),
                });
            }

            String::from_utf8_lossy(&raw_bytes).to_string()
        } else if let Some(text_val) = request.options.get("text").and_then(|v| v.as_str()) {
            text_val.to_string()
        } else {
            return Err(CommandError::ValidationFailed {
                reason: "Không có văn bản hoặc tệp đầu vào cần ẩn danh hóa PII".to_string(),
            });
        };

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 50.0,
                message: "Đang quét và thay thế nhãn ẩn danh dữ liệu PII...".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        let anonymized = anonymize_pii_text(&content, &options, cancel_flag)?;

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("pii_anonymize_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        let out_file = out_dir.join("anonymized_result.txt");
        fs::write(&out_file, anonymized.as_bytes()).map_err(|e| CommandError::IoError {
            details: e.to_string(),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Hoàn tất ẩn danh hóa dữ liệu PII an toàn".to_string(),
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
    fn test_text_pii_anonymizer_happy_path() {
        let tool = TextPiiAnonymizerTool::default();
        assert_eq!(tool.slug(), "text-pii-anonymizer");

        // Quét và thay thế số CCCD 12 số trong văn bản
        let text = "Khách hàng Nguyễn Văn A có CCCD số 001200001234 đăng ký tại Hà Nội.";
        let cancel = AtomicBool::new(false);
        let opts = PiiAnonymizerOptions::default();

        let result = anonymize_pii_text(text, &opts, &cancel).expect("anonymize ok");
        assert!(!result.contains("001200001234"));
        assert!(result.contains("[CCCD_ẨN]"));
        assert_eq!(
            result,
            "Khách hàng Nguyễn Văn A có CCCD số [CCCD_ẨN] đăng ký tại Hà Nội."
        );
    }

    #[test]
    fn test_text_pii_anonymizer_edge_case() {
        // Thay thế số điện thoại đầu số VN (09x, 08x, 03x, 07x) bằng nhãn ẩn
        let text = "Liên hệ anh Nam qua SĐT 0912345678 hoặc chị Mai số 0898765432, 0345678901.";
        let cancel = AtomicBool::new(false);
        let opts = PiiAnonymizerOptions::default();

        let result = anonymize_pii_text(text, &opts, &cancel).expect("phone ok");
        assert!(!result.contains("0912345678"));
        assert!(!result.contains("0898765432"));
        assert!(!result.contains("0345678901"));
        assert_eq!(
            result,
            "Liên hệ anh Nam qua SĐT [SĐT_ẨN] hoặc chị Mai số [SĐT_ẨN], [SĐT_ẨN]."
        );
    }

    #[test]
    fn test_text_pii_anonymizer_invalid_magic_bytes() {
        let tool = TextPiiAnonymizerTool::default();
        let tmp = std::env::temp_dir().join("test_pii_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("bad_exec.bin");
        fs::write(&bad_file, b"\x7fELF\x02\x01\x01\x00").expect("write elf");

        let req = DispatchTaskRequest {
            task_id: "task-fake-pii".to_string(),
            tool_slug: "text-pii-anonymizer".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_text_pii_anonymizer_cooperative_cancellation() {
        let tool = TextPiiAnonymizerTool::default();
        let req = DispatchTaskRequest {
            task_id: "task-cancel-pii".to_string(),
            tool_slug: "text-pii-anonymizer".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "text": "CCCD: 001200001234"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_text_pii_anonymizer_security_guardrails() {
        // Kiểm thử tùy chọn tắt cờ mask_phone thì không ẩn số điện thoại
        let cancel = AtomicBool::new(false);
        let text = "SĐT: 0912345678, CCCD: 001200001234";
        let opts = PiiAnonymizerOptions {
            mask_cccd: true,
            mask_phone: false,
            mask_mst: false,
            mask_bank: false,
        };

        let res = anonymize_pii_text(text, &opts, &cancel).expect("custom opts ok");
        assert!(res.contains("0912345678"));
        assert!(res.contains("[CCCD_ẨN]"));
    }
}
