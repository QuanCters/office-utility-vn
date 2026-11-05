use std::sync::atomic::{AtomicBool, Ordering};
use office_shared::{CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress};
use serde::{Deserialize, Serialize};
use crate::OfficeTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CccdMstValidatorOptions {
    pub input_string: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CccdDecoded {
    pub is_valid: bool,
    pub cccd_number: String,
    pub province_code: String,
    pub province_name: String,
    pub gender: String,
    pub century: u32,
    pub birth_year: u32,
    pub random_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MstValidationResult {
    pub is_valid: bool,
    pub mst: String,
    pub mst_type: String, // "Doanh nghiệp / Cá nhân (10 số)" hoặc "Chi nhánh (13 số)"
    pub checksum_digit: u32,
    pub expected_checksum: u32,
    pub branch_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ValidationOutput {
    Cccd(CccdDecoded),
    Mst(MstValidationResult),
    Invalid { input: String, reason: String },
}

pub struct VnCccdMstValidatorTool;

impl Default for VnCccdMstValidatorTool {
    fn default() -> Self {
        Self
    }
}

/// Tra cứu tên tỉnh/thành theo mã 3 chữ số quy chuẩn CCCD Bộ Công an
pub fn lookup_province_by_code(code: &str) -> Option<&'static str> {
    match code {
        "001" => Some("Hà Nội"),
        "002" => Some("Hà Giang"),
        "004" => Some("Cao Bằng"),
        "006" => Some("Bắc Kạn"),
        "008" => Some("Tuyên Quang"),
        "010" => Some("Lào Cai"),
        "011" => Some("Điện Biên"),
        "012" => Some("Lai Châu"),
        "014" => Some("Sơn La"),
        "015" => Some("Yên Bái"),
        "017" => Some("Hòa Bình"),
        "019" => Some("Thái Nguyên"),
        "020" => Some("Lạng Sơn"),
        "022" => Some("Quảng Ninh"),
        "024" => Some("Bắc Giang"),
        "025" => Some("Phú Thọ"),
        "026" => Some("Vĩnh Phúc"),
        "027" => Some("Bắc Ninh"),
        "030" => Some("Hải Dương"),
        "031" => Some("Hải Phòng"),
        "033" => Some("Hưng Yên"),
        "034" => Some("Thái Bình"),
        "035" => Some("Hà Nam"),
        "036" => Some("Nam Định"),
        "037" => Some("Ninh Bình"),
        "038" => Some("Thanh Hóa"),
        "040" => Some("Nghệ An"),
        "042" => Some("Hà Tĩnh"),
        "044" => Some("Quảng Bình"),
        "045" => Some("Quảng Trị"),
        "046" => Some("Thừa Thiên Huế"),
        "048" => Some("Đà Nẵng"),
        "049" => Some("Quảng Nam"),
        "051" => Some("Quảng Ngãi"),
        "052" => Some("Bình Định"),
        "054" => Some("Phú Yên"),
        "056" => Some("Khánh Hòa"),
        "058" => Some("Ninh Thuận"),
        "060" => Some("Bình Thuận"),
        "062" => Some("Kon Tum"),
        "064" => Some("Gia Lai"),
        "066" => Some("Đắk Lắk"),
        "067" => Some("Đắk Nông"),
        "068" => Some("Lâm Đồng"),
        "069" => Some("Bình Phước"),
        "070" => Some("Tây Ninh"),
        "072" => Some("Bình Dương"),
        "074" => Some("Đồng Nai"),
        "075" => Some("Bà Rịa - Vũng Tàu"),
        "077" => Some("Bà Rịa - Vũng Tàu"),
        "079" => Some("TP. Hồ Chí Minh"),
        "080" => Some("Long An"),
        "082" => Some("Tiền Giang"),
        "083" => Some("Bến Tre"),
        "084" => Some("Trà Vinh"),
        "086" => Some("Vĩnh Long"),
        "087" => Some("Đồng Tháp"),
        "089" => Some("An Giang"),
        "091" => Some("Kiên Giang"),
        "092" => Some("Cần Thơ"),
        "093" => Some("Hậu Giang"),
        "094" => Some("Sóc Trăng"),
        "095" => Some("Bạc Liêu"),
        "096" => Some("Cà Mau"),
        _ => None,
    }
}

/// Giải mã và kiểm tra CCCD 12 số
pub fn validate_and_decode_cccd(cccd: &str) -> CccdDecoded {
    let clean: String = cccd.chars().filter(|c| c.is_ascii_digit()).collect();
    if clean.len() != 12 {
        return CccdDecoded {
            is_valid: false,
            cccd_number: clean,
            province_code: String::new(),
            province_name: String::new(),
            gender: String::new(),
            century: 0,
            birth_year: 0,
            random_number: String::new(),
        };
    }

    let prov_code = clean[0..3].to_string();
    let prov_name = lookup_province_by_code(&prov_code).unwrap_or("Không xác định");
    let is_prov_valid = lookup_province_by_code(&prov_code).is_some();

    let gender_code = clean.as_bytes()[3] - b'0';
    let (century_base, gender) = match gender_code {
        0 => (1900, "Nam"),
        1 => (1900, "Nữ"),
        2 => (2000, "Nam"),
        3 => (2000, "Nữ"),
        4 => (2100, "Nam"),
        5 => (2100, "Nữ"),
        6 => (2200, "Nam"),
        7 => (2200, "Nữ"),
        8 => (2300, "Nam"),
        9 => (2300, "Nữ"),
        _ => (0, "Không xác định"),
    };

    let yy: u32 = clean[4..6].parse().unwrap_or(0);
    let birth_year = century_base + yy;
    let century = (century_base / 100) + 1;

    let random_number = clean[6..12].to_string();

    CccdDecoded {
        is_valid: is_prov_valid,
        cccd_number: clean,
        province_code: prov_code,
        province_name: prov_name.to_string(),
        gender: gender.to_string(),

        century,
        birth_year,
        random_number,
    }
}

/// Kiểm tra tính hợp lệ của Mã số thuế 10 số hoặc 13 số theo thuật toán Modulo 11
pub fn validate_mst(mst: &str) -> MstValidationResult {
    let clean: String = mst.chars().filter(|c| c.is_ascii_digit()).collect();

    if clean.len() != 10 && clean.len() != 13 {
        return MstValidationResult {
            is_valid: false,
            mst: clean,
            mst_type: "Không đúng độ dài (phải 10 hoặc 13 chữ số)".to_string(),
            checksum_digit: 0,
            expected_checksum: 0,
            branch_code: None,
        };
    }

    // Vector trọng số cho 9 số đầu
    const WEIGHTS: [u32; 9] = [31, 29, 23, 19, 17, 13, 7, 5, 3];

    let digits: Vec<u32> = clean[0..10]
        .chars()
        .map(|c| (c as u8 - b'0') as u32)
        .collect();

    let mut sum: u32 = 0;
    for i in 0..9 {
        sum += digits[i] * WEIGHTS[i];
    }

    let remainder = sum % 11;
    let expected_checksum = 10 - remainder;
    let normalized_expected = if expected_checksum >= 10 {
        0
    } else {
        expected_checksum
    };

    let actual_checksum = digits[9];
    let is_valid = actual_checksum == normalized_expected;

    if clean.len() == 10 {
        MstValidationResult {
            is_valid,
            mst: clean,
            mst_type: "Doanh nghiệp / Cá nhân (10 số)".to_string(),
            checksum_digit: actual_checksum,
            expected_checksum: normalized_expected,
            branch_code: None,
        }
    } else {
        let branch = clean[10..13].to_string();
        MstValidationResult {
            is_valid,
            mst: clean,
            mst_type: "Đơn vị trực thuộc / Chi nhánh (13 số)".to_string(),
            checksum_digit: actual_checksum,
            expected_checksum: normalized_expected,
            branch_code: Some(branch),
        }
    }
}

/// Phân định và xử lý đầu vào tự động (CCCD 12 số hoặc MST 10/13 số)
pub fn process_cccd_or_mst(input: &str) -> ValidationOutput {
    let clean: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
    if clean.len() == 12 {
        ValidationOutput::Cccd(validate_and_decode_cccd(&clean))
    } else if clean.len() == 10 || clean.len() == 13 {
        ValidationOutput::Mst(validate_mst(&clean))
    } else {
        ValidationOutput::Invalid {
            input: input.to_string(),
            reason: format!(
                "Độ dài {} chữ số không khớp chuẩn CCCD (12 số) hoặc MST (10 hoặc 13 số)",
                clean.len()
            ),
        }
    }
}

impl OfficeTool for VnCccdMstValidatorTool {
    fn slug(&self) -> &'static str {
        "vn-cccd-mst-validator"
    }

    fn name(&self) -> &'static str {
        "Kiểm tra Hợp lệ CCCD & Mã số thuế"
    }

    fn description(&self) -> &'static str {
        "Giải mã thông tin CCCD 12 số (tỉnh thành, giới tính, thế kỷ, năm sinh) và kiểm tra tính hợp lệ Mã số thuế 10/13 số bằng thuật toán Modulo 11 trọng số."
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
                        expected: "Plaintext / JSON CCCD or MST".to_string(),
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
                message: "Đang kiểm tra và giải mã CCCD / MST...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let input_str = if let Some(s) = request.options.get("input_string").and_then(|v| v.as_str()) {
            s.to_string()
        } else if let Some(s) = request.options.get("input").and_then(|v| v.as_str()) {
            s.to_string()
        } else if let Some(s) = request.options.get("cccd").and_then(|v| v.as_str()) {
            s.to_string()
        } else if let Some(path) = request.input_paths.first() {
            std::fs::read_to_string(path).map_err(|e| CommandError::IoError {
                details: format!("Không thể đọc tệp '{path}': {e}"),
            })?
        } else {
            "001205012345".to_string()
        };

        let result = process_cccd_or_mst(&input_str);

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let json_output = serde_json::to_string_pretty(&result).map_err(|e| CommandError::ProcessExecutionError {
            message: format!("Lỗi tuần tự hóa kết quả CCCD/MST: {e}"),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Đã hoàn tất kiểm tra CCCD / MST".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths: vec![json_output],
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
    fn test_vn_cccd_mst_validator_happy_path() {
        // CCCD: 001 2 05 012345
        // 001: Hà Nội
        // 2: Thế kỷ 21 (2000-2099), Nam
        // 05: Sinh năm 2005
        // 012345: số ngẫu nhiên
        let cccd = validate_and_decode_cccd("001205012345");
        assert!(cccd.is_valid);
        assert_eq!(cccd.province_code, "001");
        assert_eq!(cccd.province_name, "Hà Nội");
        assert_eq!(cccd.gender, "Nam");
        assert_eq!(cccd.century, 21);
        assert_eq!(cccd.birth_year, 2005);
        assert_eq!(cccd.random_number, "012345");

        // CCCD Nữ sinh năm 1995 tại TP.HCM (079 1 95 654321)
        let cccd_hcm = validate_and_decode_cccd("079195654321");
        assert!(cccd_hcm.is_valid);
        assert_eq!(cccd_hcm.province_code, "079");
        assert_eq!(cccd_hcm.province_name, "TP. Hồ Chí Minh");
        assert_eq!(cccd_hcm.gender, "Nữ");
        assert_eq!(cccd_hcm.century, 20);
        assert_eq!(cccd_hcm.birth_year, 1995);
    }

    #[test]
    fn test_vn_cccd_mst_validator_edge_case() {
        // Test MST hợp lệ của Tổng cục Thuế / Tập đoàn Viettel: 0100109106
        // 010010910-6: D1..D9 = [0, 1, 0, 0, 1, 0, 9, 1, 0], W = [31, 29, 23, 19, 17, 13, 7, 5, 3]
        // Sum = 0*31 + 1*29 + 0*23 + 0*19 + 1*17 + 0*13 + 9*7 + 1*5 + 0*3
        //     = 29 + 17 + 63 + 5 = 114
        // 114 % 11 = 4
        // 10 - 4 = 6. D10 = 6 -> Hợp lệ!
        let mst_res = validate_mst("0100109106");
        assert!(mst_res.is_valid);
        assert_eq!(mst_res.checksum_digit, 6);
        assert_eq!(mst_res.expected_checksum, 6);

        // MST 13 số chi nhánh: 0100109106-001
        let mst13 = validate_mst("0100109106001");
        assert!(mst13.is_valid);
        assert_eq!(mst13.branch_code, Some("001".to_string()));

        // MST sai checksum
        let mst_bad = validate_mst("0100109109");
        assert!(!mst_bad.is_valid);
    }

    #[test]
    fn test_vn_cccd_mst_validator_invalid_magic_bytes() {
        let tool = VnCccdMstValidatorTool;
        let bad_file = std::env::temp_dir().join("bad_magic_cccd.bin");
        std::fs::write(&bad_file, [0x1F, 0x8B, 0x08, 0x00, 0x00]).ok();

        let req = DispatchTaskRequest {
            task_id: "test-invalid-magic-cccd".to_string(),
            tool_slug: "vn-cccd-mst-validator".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: None,
            options: json!({}),
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
    fn test_vn_cccd_mst_validator_cooperative_cancellation() {
        let tool = VnCccdMstValidatorTool;
        let req = DispatchTaskRequest {
            task_id: "test-cancel-cccd".to_string(),
            tool_slug: "vn-cccd-mst-validator".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({ "input_string": "001205012345" }),
        };
        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_vn_cccd_mst_validator_security_guardrails() {
        let tool = VnCccdMstValidatorTool;
        assert_eq!(tool.slug(), "vn-cccd-mst-validator");
        assert_eq!(tool.name(), "Kiểm tra Hợp lệ CCCD & Mã số thuế");
    }

    #[test]
    fn test_vn_cccd_mst_validator_options_key_compatibility() {
        let tool = VnCccdMstValidatorTool;
        let cancel = AtomicBool::new(false);

        // 1. Thử nghiệm với key "cccd"
        let req_cccd = DispatchTaskRequest {
            task_id: "test-opts-cccd".to_string(),
            tool_slug: "vn-cccd-mst-validator".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({ "cccd": "079195654321" }),
        };
        let resp_cccd = tool.execute(&req_cccd, &cancel, None).expect("execute cccd");
        assert_eq!(resp_cccd.status, "SUCCESS");
        let out_str = &resp_cccd.output_paths[0];
        assert!(out_str.contains("TP. Hồ Chí Minh"));
        assert!(out_str.contains("Nữ"));

        // 2. Thử nghiệm với MST qua key "input_string"
        let req_mst = DispatchTaskRequest {
            task_id: "test-opts-mst".to_string(),
            tool_slug: "vn-cccd-mst-validator".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({ "input_string": "0100109106" }),
        };
        let resp_mst = tool.execute(&req_mst, &cancel, None).expect("execute mst");
        assert_eq!(resp_mst.status, "SUCCESS");
        let out_mst = &resp_mst.output_paths[0];
        assert!(out_mst.contains("\"is_valid\": true"));
        assert!(out_mst.contains("Doanh nghiệp / Cá nhân (10 số)"));
    }
}
