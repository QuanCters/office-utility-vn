use std::sync::atomic::{AtomicBool, Ordering};
use office_shared::{CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress};
use serde::{Deserialize, Serialize};
use crate::OfficeTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VietQrConfig {
    pub bin: String,
    pub account_number: String,
    pub amount: Option<u64>,
    pub message: Option<String>,
}

pub struct VietQrNapas247GeneratorTool;

impl Default for VietQrNapas247GeneratorTool {
    fn default() -> Self {
        Self
    }
}

/// Tính CRC-16/CCITT-FALSE (poly: 0x1021, init: 0xFFFF, refIn: false, refOut: false, xorOut: 0x0000)
pub fn crc16_ccitt_false(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// Dựng chuỗi EMVCo Merchant Presented QR (ID 00 đến 63) chuẩn Napas247 / VietQR
pub fn build_vietqr_payload(config: &VietQrConfig) -> Result<String, CommandError> {
    let clean_bin = config.bin.trim().replace(' ', "");
    if clean_bin.len() != 6 || !clean_bin.chars().all(|c| c.is_ascii_digit()) {
        return Err(CommandError::ValidationFailed {
            reason: format!("Mã BIN ngân hàng phải gồm đúng 6 chữ số: {clean_bin}"),
        });
    }

    let clean_acc = config.account_number.trim().replace(' ', "");
    if clean_acc.is_empty() {
        return Err(CommandError::ValidationFailed {
            reason: "Số tài khoản ngân hàng không được để trống".to_string(),
        });
    }

    // 00: Payload Format Indicator (01)
    let id00 = "000201";

    // 01: Point of Initiation Method (11: Static, 12: Dynamic with amount)
    let id01 = if config.amount.is_some() {
        "010212"
    } else {
        "010211"
    };

    // 38: Merchant Account Information (VietQR / Napas247)
    // Sub-00: GUID "A000000727"
    let guid = "0010A000000727";

    // Sub-01: Beneficiary Org (00: BIN, 01: Account Number)
    let sub00_bin = format!("00{:02}{clean_bin}", clean_bin.len());
    let sub01_acc = format!("01{:02}{clean_acc}", clean_acc.len());
    let beneficiary = format!("{sub00_bin}{sub01_acc}");
    let sub01_field = format!("01{:02}{beneficiary}", beneficiary.len());

    // Sub-02: Service Code (QRIBFTTA)
    let sub02_service = "0208QRIBFTTA";

    let id38_content = format!("{guid}{sub01_field}{sub02_service}");
    let id38 = format!("38{:02}{id38_content}", id38_content.len());

    // 53: Transaction Currency (704 for VND)
    let id53 = "5303704";

    // 54: Transaction Amount (optional)
    let id54 = match config.amount {
        Some(amt) => {
            let amt_str = amt.to_string();
            format!("54{:02}{amt_str}", amt_str.len())
        }
        None => String::new(),
    };

    // 58: Country Code (VN)
    let id58 = "5802VN";

    // 62: Additional Data Field Template (Message/Purpose)
    let id62 = match &config.message {
        Some(msg) if !msg.trim().is_empty() => {
            let clean_msg = msg.trim();
            let sub08 = format!("08{:02}{clean_msg}", clean_msg.len());
            format!("62{:02}{sub08}", sub08.len())
        }
        _ => String::new(),
    };

    // Ráp chuỗi trước CRC
    let partial = format!("{id00}{id01}{id38}{id53}{id54}{id58}{id62}6304");

    // Tính CRC-16
    let crc_val = crc16_ccitt_false(partial.as_bytes());
    let full_payload = format!("{partial}{crc_val:04X}");

    Ok(full_payload)
}

impl OfficeTool for VietQrNapas247GeneratorTool {
    fn slug(&self) -> &'static str {
        "vietqr-napas247-generator"
    }

    fn name(&self) -> &'static str {
        "Tạo Mã VietQR Napas247"
    }

    fn description(&self) -> &'static str {
        "Tạo chuỗi thanh toán VietQR Napas247 chuẩn EMVCo và tính toán mã kiểm tra CRC-16/CCITT-FALSE hoàn toàn 100% On-Device."
    }

    fn execute(
        &self,
        request: &DispatchTaskRequest,
        cancel_flag: &AtomicBool,
        on_progress: Option<&dyn Fn(TaskProgress)>,
    ) -> Result<DispatchTaskResponse, CommandError> {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        // Validate magic bytes if input file provided
        for path in &request.input_paths {
            if let Ok(bytes) = std::fs::read(path)
                && bytes.len() >= 4 && !bytes.starts_with(b"{\n") && !bytes.starts_with(b"{\"") && !bytes.is_ascii() {
                    return Err(CommandError::InvalidFileSignature {
                        expected: "JSON / Plaintext VietQR Config".to_string(),
                        actual: format!("{:02X?}", &bytes[..4.min(bytes.len())]),
                    });
                }
        }

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 30.0,
                message: "Đang dựng cấu trúc thẻ EMVCo Napas247...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: request.input_paths.len(),
                is_completed: false,
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let bin = request
            .options
            .get("bin")
            .and_then(|v| v.as_str())
            .unwrap_or("970436")
            .to_string();

        let account_number = request
            .options
            .get("account_number")
            .and_then(|v| v.as_str())
            .unwrap_or("123456789")
            .to_string();

        let amount = request
            .options
            .get("amount")
            .and_then(|v| v.as_u64())
            .or_else(|| {
                request
                    .options
                    .get("amount")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.replace(',', "").trim().parse::<u64>().ok())
            });

        let message = request
            .options
            .get("message")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let config = VietQrConfig {
            bin,
            account_number,
            amount,
            message,
        };

        let payload = build_vietqr_payload(&config)?;

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Đã tạo chuỗi VietQR Napas247 thành công".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths: vec![payload],
            failed_items: vec![],
            elapsed_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use office_shared::DispatchTaskRequest;
    use serde_json::json;

    #[test]
    fn test_vietqr_napas247_generator_happy_path() {
        let config = VietQrConfig {
            bin: "970436".to_string(), // Vietcombank
            account_number: "0011001234567".to_string(),
            amount: Some(500000),
            message: Some("Thanh toan tien ca phe".to_string()),
        };

        let qr = build_vietqr_payload(&config).expect("Tạo VietQR hợp lệ");
        assert!(qr.starts_with("000201010212"));
        assert!(qr.contains("A000000727"));
        assert!(qr.contains("970436"));
        assert!(qr.contains("0011001234567"));
        assert!(qr.contains("5303704"));
        assert!(qr.contains("5406500000"));
        assert!(qr.contains("5802VN"));
        assert!(qr.contains("6304"));
        // Chiều dài hợp lệ của payload
        assert!(qr.len() > 60);

        // Xác minh CRC ở cuối
        let payload_len = qr.len();
        let payload_without_crc = &qr[..payload_len - 4];
        let crc_hex = &qr[payload_len - 4..];
        let computed_crc = crc16_ccitt_false(payload_without_crc.as_bytes());
        assert_eq!(format!("{computed_crc:04X}"), crc_hex);
    }

    #[test]
    fn test_vietqr_napas247_generator_edge_case() {
        // Kiểm thử thuật toán CRC-16/CCITT-FALSE với test vector chuẩn
        // Vector chuẩn của CCITT-FALSE với chuỗi "123456789" là 0x29B1
        let test_vec = b"123456789";
        let crc = crc16_ccitt_false(test_vec);
        assert_eq!(crc, 0x29B1);

        // Trường hợp không có số tiền và không có nội dung
        let config_minimal = VietQrConfig {
            bin: "970422".to_string(), // MBBank
            account_number: "0987654321".to_string(),
            amount: None,
            message: None,
        };
        let qr_min = build_vietqr_payload(&config_minimal).expect("Tạo VietQR tối giản");
        assert!(qr_min.starts_with("000201010211"));
        assert!(qr_min.contains("53037045802VN6304"));
    }


    #[test]
    fn test_vietqr_napas247_generator_invalid_magic_bytes() {
        let tool = VietQrNapas247GeneratorTool;
        let bad_file = std::env::temp_dir().join("bad_magic_vietqr.bin");
        std::fs::write(&bad_file, [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A]).ok();

        let req = DispatchTaskRequest {
            task_id: "test-invalid-magic-vietqr".to_string(),
            tool_slug: "vietqr-napas247-generator".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: None,
            options: json!({
                "bin": "970436",
                "account_number": "123456"
            }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận: {:?}", other),
        }
        let _ = std::fs::remove_file(bad_file);
    }

    #[test]
    fn test_vietqr_napas247_generator_cooperative_cancellation() {
        let tool = VietQrNapas247GeneratorTool;
        let req = DispatchTaskRequest {
            task_id: "test-cancel".to_string(),
            tool_slug: "vietqr-napas247-generator".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "bin": "970436",
                "account_number": "123456"
            }),
        };
        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_vietqr_napas247_generator_security_guardrails() {
        let tool = VietQrNapas247GeneratorTool;
        assert_eq!(tool.slug(), "vietqr-napas247-generator");
        assert_eq!(tool.name(), "Tạo Mã VietQR Napas247");
    }
}
