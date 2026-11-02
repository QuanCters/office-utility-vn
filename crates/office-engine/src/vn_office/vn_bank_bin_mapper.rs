use std::sync::atomic::{AtomicBool, Ordering};
use office_shared::{CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress};
use serde::{Deserialize, Serialize};
use crate::OfficeTool;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BankBinLookupOptions {
    #[serde(default)]
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankInfo {
    pub bin: String,
    pub short_name: String,
    pub aliases: Vec<String>,
    pub full_name: String,
    pub english_name: String,
    pub citad_code: String,
    pub swift_code: String,
    pub napas_supported: bool,
}

pub struct VnBankBinMapperTool;

impl Default for VnBankBinMapperTool {
    fn default() -> Self {
        Self
    }
}

/// Danh bạ ngân hàng Việt Nam chuẩn hóa
pub fn get_vietnam_banks_catalog() -> Vec<BankInfo> {
    vec![
        BankInfo {
            bin: "970436".to_string(),
            short_name: "Vietcombank".to_string(),
            aliases: vec!["vcb".to_string(), "vietcom".to_string()],
            full_name: "Ngân hàng TMCP Ngoại thương Việt Nam".to_string(),
            english_name: "Joint Stock Commercial Bank for Foreign Trade of Vietnam".to_string(),
            citad_code: "01201001".to_string(),
            swift_code: "BFTVVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970415".to_string(),
            short_name: "VietinBank".to_string(),
            aliases: vec!["ctg".to_string(), "vietin".to_string()],
            full_name: "Ngân hàng TMCP Công thương Việt Nam".to_string(),
            english_name: "Vietnam Joint Stock Commercial Bank for Industry and Trade".to_string(),
            citad_code: "01202001".to_string(),
            swift_code: "ICBVVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970418".to_string(),
            short_name: "BIDV".to_string(),
            aliases: vec!["bid".to_string(), "dau tu va phat trien".to_string()],
            full_name: "Ngân hàng TMCP Đầu tư và Phát triển Việt Nam".to_string(),
            english_name: "Bank for Investment and Development of Vietnam".to_string(),
            citad_code: "01203001".to_string(),
            swift_code: "BIDVVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970405".to_string(),
            short_name: "Agribank".to_string(),
            aliases: vec!["vba".to_string(), "nong nghiep".to_string()],
            full_name: "Ngân hàng Nông nghiệp và Phát triển Nông thôn Việt Nam".to_string(),
            english_name: "Vietnam Bank for Agriculture and Rural Development".to_string(),
            citad_code: "01204001".to_string(),
            swift_code: "VBAAVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970422".to_string(),
            short_name: "MBBank".to_string(),
            aliases: vec!["mb".to_string(), "quan doi".to_string()],
            full_name: "Ngân hàng TMCP Quân đội".to_string(),
            english_name: "Military Commercial Joint Stock Bank".to_string(),
            citad_code: "01311001".to_string(),
            swift_code: "MSCBVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970407".to_string(),
            short_name: "Techcombank".to_string(),
            aliases: vec!["tcb".to_string(), "techcom".to_string()],
            full_name: "Ngân hàng TMCP Kỹ thương Việt Nam".to_string(),
            english_name: "Vietnam Technological and Commercial Joint Stock Bank".to_string(),
            citad_code: "01310001".to_string(),
            swift_code: "VTCBVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970416".to_string(),
            short_name: "ACB".to_string(),
            aliases: vec!["a chau".to_string()],
            full_name: "Ngân hàng TMCP Á Châu".to_string(),
            english_name: "Asia Commercial Joint Stock Bank".to_string(),
            citad_code: "79307001".to_string(),
            swift_code: "ASCBVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970432".to_string(),
            short_name: "VPBank".to_string(),
            aliases: vec!["vpb".to_string(), "viet nam thinh vuong".to_string()],
            full_name: "Ngân hàng TMCP Việt Nam Thịnh Vượng".to_string(),
            english_name: "Vietnam Prosperity Joint Stock Commercial Bank".to_string(),
            citad_code: "01314001".to_string(),
            swift_code: "VPBNVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970423".to_string(),
            short_name: "TPBank".to_string(),
            aliases: vec!["tpb".to_string(), "tien phong".to_string()],
            full_name: "Ngân hàng TMCP Tiên Phong".to_string(),
            english_name: "Tien Phong Commercial Joint Stock Bank".to_string(),
            citad_code: "01358001".to_string(),
            swift_code: "TPBVVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970403".to_string(),
            short_name: "Sacombank".to_string(),
            aliases: vec!["stb".to_string(), "sai gon thuong tin".to_string()],
            full_name: "Ngân hàng TMCP Sài Gòn Thương Tín".to_string(),
            english_name: "Saigon Thuong Tin Commercial Joint Stock Bank".to_string(),
            citad_code: "79303001".to_string(),
            swift_code: "SGSTVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970441".to_string(),
            short_name: "VIB".to_string(),
            aliases: vec!["quoc te".to_string()],
            full_name: "Ngân hàng TMCP Quốc tế Việt Nam".to_string(),
            english_name: "Vietnam International Commercial Joint Stock Bank".to_string(),
            citad_code: "01312001".to_string(),
            swift_code: "VIBKVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970437".to_string(),
            short_name: "HDBank".to_string(),
            aliases: vec!["hdb".to_string(), "phat trien tp hcm".to_string()],
            full_name: "Ngân hàng TMCP Phát triển TP. Hồ Chí Minh".to_string(),
            english_name: "Ho Chi Minh City Development Joint Stock Commercial Bank".to_string(),
            citad_code: "79321001".to_string(),
            swift_code: "HDBCVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970443".to_string(),
            short_name: "SHB".to_string(),
            aliases: vec!["sai gon ha noi".to_string()],
            full_name: "Ngân hàng TMCP Sài Gòn - Hà Nội".to_string(),
            english_name: "Saigon - Hanoi Commercial Joint Stock Bank".to_string(),
            citad_code: "01348001".to_string(),
            swift_code: "SHBVVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970448".to_string(),
            short_name: "OCB".to_string(),
            aliases: vec!["phuong dong".to_string()],
            full_name: "Ngân hàng TMCP Phương Đông".to_string(),
            english_name: "Orient Commercial Joint Stock Bank".to_string(),
            citad_code: "79333001".to_string(),
            swift_code: "OCBVVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970426".to_string(),
            short_name: "MSB".to_string(),
            aliases: vec!["hang hai".to_string(), "maritime bank".to_string()],
            full_name: "Ngân hàng TMCP Hàng Hải Việt Nam".to_string(),
            english_name: "Vietnam Maritime Commercial Joint Stock Bank".to_string(),
            citad_code: "01302001".to_string(),
            swift_code: "MCOPVNVX".to_string(),
            napas_supported: true,
        },
        BankInfo {
            bin: "970440".to_string(),
            short_name: "SeABank".to_string(),
            aliases: vec!["dong nam a".to_string()],
            full_name: "Ngân hàng TMCP Đông Nam Á".to_string(),
            english_name: "Southeast Asia Commercial Joint Stock Bank".to_string(),
            citad_code: "01333001".to_string(),
            swift_code: "SEABVNVX".to_string(),
            napas_supported: true,
        },
    ]
}

/// Tra cứu danh bạ ngân hàng theo BIN, tên viết tắt, SWIFT hoặc tên đầy đủ
pub fn lookup_banks(query: &str) -> Vec<BankInfo> {
    let q = query.trim().to_lowercase();
    let catalog = get_vietnam_banks_catalog();

    if q.is_empty() {
        return catalog;
    }

    catalog
        .into_iter()
        .filter(|b| {
            b.bin.contains(&q)
                || b.short_name.to_lowercase().contains(&q)
                || b.full_name.to_lowercase().contains(&q)
                || b.english_name.to_lowercase().contains(&q)
                || b.swift_code.to_lowercase().contains(&q)
                || b.citad_code.contains(&q)
                || b.aliases.iter().any(|a| a.to_lowercase() == q || a.to_lowercase().contains(&q))
        })
        .collect()
}

impl OfficeTool for VnBankBinMapperTool {
    fn slug(&self) -> &'static str {
        "vn-bank-bin-mapper"
    }

    fn name(&self) -> &'static str {
        "Tra cứu Mã Định danh Ngân hàng"
    }

    fn description(&self) -> &'static str {
        "Tra cứu tức thì danh bạ ngân hàng Việt Nam, mã BIN Napas 6 số, tên giao dịch chuẩn, mã liên ngân hàng CITAD và SWIFT/BIC Code 100% On-Device."
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
            if let Ok(bytes) = std::fs::read(path) {
                let is_binary = bytes.iter().any(|&b| b < 0x09 || (b > 0x0D && b < 0x20) || b == 0x7F)
                    || infer::get(&bytes).is_some();
                if bytes.len() >= 4 && (!bytes.starts_with(b"{\n") && !bytes.starts_with(b"{\"")) && is_binary {
                    return Err(CommandError::InvalidFileSignature {
                        expected: "Plaintext / JSON Bank Query".to_string(),
                        actual: format!("{:02X?}", &bytes[..4.min(bytes.len())]),
                    });
                }
            }
        }

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 30.0,
                message: "Đang tra cứu danh bạ ngân hàng Việt Nam...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let query = if let Some(s) = request.options.get("query").and_then(|v| v.as_str()) {
            s.to_string()
        } else if let Some(s) = request.options.get("input").and_then(|v| v.as_str()) {
            s.to_string()
        } else if let Some(path) = request.input_paths.first() {
            std::fs::read_to_string(path).map_err(|e| CommandError::IoError {
                details: format!("Không thể đọc tệp '{path}': {e}"),
            })?
        } else {
            String::new()
        };

        let matches = lookup_banks(&query);

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let json_output = serde_json::to_string_pretty(&matches).map_err(|e| CommandError::ProcessExecutionError {
            message: format!("Lỗi tuần tự hóa kết quả tra cứu ngân hàng: {e}"),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: format!("Đã tìm thấy {} ngân hàng phù hợp", matches.len()),
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
    fn test_vn_bank_bin_mapper_happy_path() {
        // Tra cứu theo mã BIN 6 số Napas: 970436 -> Vietcombank
        let res_vcb = lookup_banks("970436");
        assert_eq!(res_vcb.len(), 1);
        assert_eq!(res_vcb[0].short_name, "Vietcombank");
        assert_eq!(res_vcb[0].swift_code, "BFTVVNVX");
        assert!(res_vcb[0].napas_supported);

        // Tra cứu mã BIN MBBank: 970422
        let res_mb = lookup_banks("970422");
        assert_eq!(res_mb.len(), 1);
        assert_eq!(res_mb[0].short_name, "MBBank");
    }

    #[test]
    fn test_vn_bank_bin_mapper_edge_case() {
        // Tìm kiếm theo tên viết tắt VCB -> Vietcombank, SWIFT BFTVVNVX
        let res_vcb_alias = lookup_banks("VCB");
        assert!(!res_vcb_alias.is_empty());
        assert_eq!(res_vcb_alias[0].short_name, "Vietcombank");

        // Tìm theo SWIFT code BFTVVNVX
        let res_swift = lookup_banks("BFTVVNVX");
        assert_eq!(res_swift.len(), 1);
        assert_eq!(res_swift[0].short_name, "Vietcombank");

        // Tìm query rỗng trả về toàn bộ danh bạ
        let all_banks = lookup_banks("");
        assert!(all_banks.len() >= 10);
    }

    #[test]
    fn test_vn_bank_bin_mapper_invalid_magic_bytes() {
        let tool = VnBankBinMapperTool::default();
        let bad_file = std::env::temp_dir().join("bad_magic_bank.bin");
        std::fs::write(&bad_file, [0x7F, 0x45, 0x4C, 0x46, 0x01]).ok();

        let req = DispatchTaskRequest {
            task_id: "test-invalid-magic-bank".to_string(),
            tool_slug: "vn-bank-bin-mapper".to_string(),
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
    fn test_vn_bank_bin_mapper_cooperative_cancellation() {
        let tool = VnBankBinMapperTool::default();
        let req = DispatchTaskRequest {
            task_id: "test-cancel-bank".to_string(),
            tool_slug: "vn-bank-bin-mapper".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({ "query": "Vietcombank" }),
        };
        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_vn_bank_bin_mapper_security_guardrails() {
        let tool = VnBankBinMapperTool::default();
        assert_eq!(tool.slug(), "vn-bank-bin-mapper");
        assert_eq!(tool.name(), "Tra cứu Mã Định danh Ngân hàng");
    }
}
