use std::sync::atomic::{AtomicBool, Ordering};
use office_shared::{CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress};
use serde::{Deserialize, Serialize};
use crate::OfficeTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalaryCalculatorConfig {
    pub gross_amount: f64,
    #[serde(default = "default_region")]
    pub region: u8,
    #[serde(default)]
    pub dependents: u32,
    #[serde(default)]
    pub custom_ruleset_id: Option<String>,
}

fn default_region() -> u8 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SalaryBreakdown {
    pub gross_salary: f64,
    pub bhxh_amount: f64,
    pub bhyt_amount: f64,
    pub bhtn_amount: f64,
    pub total_insurance: f64,
    pub taxable_income_before_relief: f64,
    pub personal_relief: f64,
    pub dependent_relief: f64,
    pub assessable_income: f64,
    pub pit_amount: f64,
    pub net_salary: f64,
}

pub struct GrossNetSalaryCalculatorTool;

impl Default for GrossNetSalaryCalculatorTool {
    fn default() -> Self {
        Self
    }
}

pub fn calculate_gross_to_net(gross: f64, region: u8, dependents: u32) -> Result<SalaryBreakdown, CommandError> {
    if gross < 0.0 {
        return Err(CommandError::ValidationFailed {
            reason: "Mức lương không thể là số âm".to_string(),
        });
    }

    let base_salary = 2_340_000.0; // Mức lương cơ sở từ 01/07/2024
    let base_cap = base_salary * 20.0; // Trần đóng BHXH, BHYT

    let regional_min = match region {
        1 => 4_960_000.0,
        2 => 4_410_000.0,
        3 => 3_860_000.0,
        4 => 3_450_000.0,
        _ => {
            return Err(CommandError::ValidationFailed {
                reason: format!("Vùng lương không hợp lệ: {region} (chỉ chấp nhận từ 1 đến 4)"),
            });
        }
    };
    let bhtn_cap = regional_min * 20.0; // Trần đóng BHTN

    // Tính bảo hiểm
    let bhxh_base = gross.min(base_cap);
    let bhxh_amount = (bhxh_base * 0.08).round();

    let bhyt_base = gross.min(base_cap);
    let bhyt_amount = (bhyt_base * 0.015).round();

    let bhtn_base = gross.min(bhtn_cap);
    let bhtn_amount = (bhtn_base * 0.01).round();

    let total_insurance = bhxh_amount + bhyt_amount + bhtn_amount;

    // Thu nhập chịu thuế
    let taxable_income = gross - total_insurance;

    // Giảm trừ
    let personal_relief = 11_000_000.0;
    let dependent_relief = dependents as f64 * 4_400_000.0;
    let total_relief = personal_relief + dependent_relief;

    // Thu nhập tính thuế
    let assessable_income = (taxable_income - total_relief).max(0.0);

    // Tính thuế TNCN theo 7 bậc lũy tiến từng phần
    let pit_amount = calculate_pit(assessable_income);

    let net_salary = gross - total_insurance - pit_amount;

    Ok(SalaryBreakdown {
        gross_salary: gross,
        bhxh_amount,
        bhyt_amount,
        bhtn_amount,
        total_insurance,
        taxable_income_before_relief: taxable_income,
        personal_relief,
        dependent_relief,
        assessable_income,
        pit_amount,
        net_salary,
    })
}

/// Tính thuế thu nhập cá nhân theo biểu thuế lũy tiến từng phần 7 bậc
pub fn calculate_pit(assessable_income: f64) -> f64 {
    if assessable_income <= 0.0 {
        return 0.0;
    }

    let tax = if assessable_income <= 5_000_000.0 {
        assessable_income * 0.05
    } else if assessable_income <= 10_000_000.0 {
        assessable_income * 0.10 - 250_000.0
    } else if assessable_income <= 18_000_000.0 {
        assessable_income * 0.15 - 750_000.0
    } else if assessable_income <= 32_000_000.0 {
        assessable_income * 0.20 - 1_650_000.0
    } else if assessable_income <= 52_000_000.0 {
        assessable_income * 0.25 - 3_250_000.0
    } else if assessable_income <= 80_000_000.0 {
        assessable_income * 0.30 - 5_850_000.0
    } else {
        assessable_income * 0.35 - 9_850_000.0
    };

    tax.round()
}

impl OfficeTool for GrossNetSalaryCalculatorTool {
    fn slug(&self) -> &'static str {
        "gross-net-salary-calculator"
    }

    fn name(&self) -> &'static str {
        "Tính Lương Gross - Net & Thuế"
    }

    fn description(&self) -> &'static str {
        "Tính chính xác lương Gross sang Net, các khoản trích đóng bảo hiểm (BHXH, BHYT, BHTN) và thuế TNCN 7 bậc lũy tiến."
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

        // Validate magic bytes if input files provided
        for path in &request.input_paths {
            if let Ok(bytes) = std::fs::read(path) {
                if bytes.len() >= 4 && !bytes.starts_with(b"{\n") && !bytes.starts_with(b"{\"") && !bytes.is_ascii() {
                    return Err(CommandError::InvalidFileSignature {
                        expected: "JSON / Plaintext Salary Config".to_string(),
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
                message: "Đang phân tích cấu hình lương...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: request.input_paths.len(),
                is_completed: false,
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let gross_amount = if let Some(v) = request.options.get("gross_amount").and_then(|v| v.as_f64()) {
            v
        } else if let Some(s) = request.options.get("gross_amount").and_then(|v| v.as_str()) {
            s.replace(',', "")
                .trim()
                .parse::<f64>()
                .map_err(|_| CommandError::ValidationFailed {
                    reason: format!("Mức lương gross_amount không hợp lệ: {s}"),
                })?
        } else if let Some(v) = request.options.get("salary").and_then(|v| v.as_f64()) {
            v
        } else {
            30_000_000.0
        };

        let region = request
            .options
            .get("region")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8)
            .unwrap_or(1);

        let dependents = request
            .options
            .get("dependents")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(0);

        let breakdown = calculate_gross_to_net(gross_amount, region, dependents)?;

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let json_result = serde_json::to_string_pretty(&breakdown).map_err(|e| CommandError::ProcessExecutionError {
            message: format!("Lỗi tuần tự hóa kết quả lương: {e}"),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Hoàn tất tính toán lương".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths: vec![json_result],
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
    fn test_gross_net_salary_calculator_happy_path() {
        // Lương 30,000,000 VND, Vùng 1, 0 người phụ thuộc
        let res = calculate_gross_to_net(30_000_000.0, 1, 0).expect("Tính toán lương hợp lệ");
        assert_eq!(res.gross_salary, 30_000_000.0);
        // BHXH: 30tr * 8% = 2.4tr
        assert_eq!(res.bhxh_amount, 2_400_000.0);
        // BHYT: 30tr * 1.5% = 450k
        assert_eq!(res.bhyt_amount, 450_000.0);
        // BHTN: 30tr * 1% = 300k
        assert_eq!(res.bhtn_amount, 300_000.0);
        assert_eq!(res.total_insurance, 3_150_000.0);
        // Thu nhập chịu thuế = 30tr - 3.15tr = 26.85tr
        assert_eq!(res.taxable_income_before_relief, 26_850_000.0);
        // Thu nhập tính thuế = 26.85tr - 11tr = 15.85tr (Bậc 3: 10 - 18tr)
        assert_eq!(res.assessable_income, 15_850_000.0);
        // Thuế = 15.85tr * 15% - 750k = 2,377,500 - 750,000 = 1,627,500
        assert_eq!(res.pit_amount, 1_627_500.0);
        // Net = 30tr - 3.15tr - 1,627,500 = 25,222,500
        assert_eq!(res.net_salary, 25_222_500.0);

        // Kiểm thử trần BHXH/BHYT (lương 60 triệu vượt trần 46.8tr)
        let res_high = calculate_gross_to_net(60_000_000.0, 1, 0).expect("Tính trần hợp lệ");
        assert_eq!(res_high.bhxh_amount, 46_800_000.0 * 0.08);
        assert_eq!(res_high.bhyt_amount, 46_800_000.0 * 0.015);
        assert_eq!(res_high.bhtn_amount, 60_000_000.0 * 0.01);
    }

    #[test]
    fn test_gross_net_salary_calculator_edge_case() {
        // Kiểm thử 7 bậc thuế lũy tiến từng phần
        // Bậc 1: <= 5tr (4tr) -> 5% = 200k
        assert_eq!(calculate_pit(4_000_000.0), 200_000.0);
        // Bậc 2: 8tr -> 8tr * 10% - 250k = 550k
        assert_eq!(calculate_pit(8_000_000.0), 550_000.0);
        // Bậc 3: 15tr -> 15tr * 15% - 750k = 1,500,000
        assert_eq!(calculate_pit(15_000_000.0), 1_500_000.0);
        // Bậc 4: 25tr -> 25tr * 20% - 1.65tr = 3,350,000
        assert_eq!(calculate_pit(25_000_000.0), 3_350_000.0);
        // Bậc 5: 40tr -> 40tr * 25% - 3.25tr = 6,750,000
        assert_eq!(calculate_pit(40_000_000.0), 6_750_000.0);
        // Bậc 6: 60tr -> 60tr * 30% - 5.85tr = 12,150,000
        assert_eq!(calculate_pit(60_000_000.0), 12_150_000.0);
        // Bậc 7: 100tr -> 100tr * 35% - 9.85tr = 25,150,000
        assert_eq!(calculate_pit(100_000_000.0), 25_150_000.0);

        // Trường hợp âm hoặc 0
        assert_eq!(calculate_pit(0.0), 0.0);
        assert_eq!(calculate_pit(-1_000.0), 0.0);
    }

    #[test]
    fn test_gross_net_salary_calculator_invalid_magic_bytes() {
        let tool = GrossNetSalaryCalculatorTool::default();
        let bad_file = std::env::temp_dir().join("bad_magic_salary.bin");
        std::fs::write(&bad_file, [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]).ok();

        let req = DispatchTaskRequest {
            task_id: "test-invalid-magic".to_string(),
            tool_slug: "gross-net-salary-calculator".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: None,
            options: json!({ "gross_amount": 20000000.0 }),
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
    fn test_gross_net_salary_calculator_cooperative_cancellation() {
        let tool = GrossNetSalaryCalculatorTool::default();
        let req = DispatchTaskRequest {
            task_id: "test-cancel".to_string(),
            tool_slug: "gross-net-salary-calculator".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({ "gross_amount": 25000000.0 }),
        };
        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_gross_net_salary_calculator_security_guardrails() {
        let tool = GrossNetSalaryCalculatorTool::default();
        assert_eq!(tool.slug(), "gross-net-salary-calculator");
        assert_eq!(tool.name(), "Tính Lương Gross - Net & Thuế");
    }
}
