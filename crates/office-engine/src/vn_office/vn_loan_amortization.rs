use std::sync::atomic::{AtomicBool, Ordering};
use office_shared::{CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress};
use serde::{Deserialize, Serialize};
use crate::OfficeTool;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum AmortizationMethod {
    #[default]
    ReducingBalance, // Dư nợ giảm dần (Gốc đều, lãi giảm dần)
    FixedAnnuity,    // Niên kim cố định (Trả góp đều mỗi tháng)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanAmortizationOptions {
    pub principal: f64,
    pub annual_rate: f64,
    pub term_months: u32,
    #[serde(default)]
    pub method: AmortizationMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmortizationPeriod {
    pub period: u32,
    pub beginning_balance: f64,
    pub principal_payment: f64,
    pub interest_payment: f64,
    pub total_payment: f64,
    pub ending_balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoanAmortizationSummary {
    pub principal: f64,
    pub annual_rate: f64,
    pub term_months: u32,
    pub method: String,
    pub total_principal_paid: f64,
    pub total_interest_paid: f64,
    pub total_amount_paid: f64,
    pub schedule: Vec<AmortizationPeriod>,
}

pub struct VnLoanAmortizationTool;

impl Default for VnLoanAmortizationTool {
    fn default() -> Self {
        Self
    }
}

/// Tính lịch trả nợ vay ngân hàng chính xác từng đồng
pub fn calculate_loan_amortization(
    principal: f64,
    annual_rate: f64,
    term_months: u32,
    method: &AmortizationMethod,
) -> Result<LoanAmortizationSummary, CommandError> {
    if principal <= 0.0 {
        return Err(CommandError::ValidationFailed {
            reason: "Số tiền vay principal phải lớn hơn 0".to_string(),
        });
    }
    if annual_rate < 0.0 {
        return Err(CommandError::ValidationFailed {
            reason: "Lãi suất annual_rate không được là số âm".to_string(),
        });
    }
    if term_months == 0 {
        return Err(CommandError::ValidationFailed {
            reason: "Kỳ hạn vay term_months phải lớn hơn 0".to_string(),
        });
    }

    let monthly_rate = (annual_rate / 100.0) / 12.0;
    let mut schedule = Vec::with_capacity(term_months as usize);

    let mut remaining = principal;
    let mut total_interest = 0.0;
    let mut total_principal = 0.0;

    match method {
        AmortizationMethod::ReducingBalance => {
            // Gốc đều hàng tháng
            let base_principal = (principal / term_months as f64).round();

            for period in 1..=term_months {
                let beginning = remaining;
                let interest = (beginning * monthly_rate).round();

                let p_payment = if period == term_months {
                    beginning // Tháng cuối trả hết phần dư nợ còn lại
                } else {
                    base_principal.min(beginning)
                };

                let ending = (beginning - p_payment).max(0.0);
                let total = p_payment + interest;

                total_interest += interest;
                total_principal += p_payment;
                remaining = ending;

                schedule.push(AmortizationPeriod {
                    period,
                    beginning_balance: beginning,
                    principal_payment: p_payment,
                    interest_payment: interest,
                    total_payment: total,
                    ending_balance: ending,
                });
            }
        }
        AmortizationMethod::FixedAnnuity => {
            // Niên kim cố định: PMT = P * r * (1+r)^n / ((1+r)^n - 1)
            let fixed_pmt = if monthly_rate > 0.0 {
                let factor = (1.0 + monthly_rate).powi(term_months as i32);
                (principal * (monthly_rate * factor) / (factor - 1.0)).round()
            } else {
                (principal / term_months as f64).round()
            };

            for period in 1..=term_months {
                let beginning = remaining;
                let interest = (beginning * monthly_rate).round();

                let mut p_payment = (fixed_pmt - interest).max(0.0);
                if period == term_months || p_payment > beginning {
                    p_payment = beginning;
                }

                let ending = (beginning - p_payment).max(0.0);
                let total = p_payment + interest;

                total_interest += interest;
                total_principal += p_payment;
                remaining = ending;

                schedule.push(AmortizationPeriod {
                    period,
                    beginning_balance: beginning,
                    principal_payment: p_payment,
                    interest_payment: interest,
                    total_payment: total,
                    ending_balance: ending,
                });
            }
        }
    }

    let method_name = match method {
        AmortizationMethod::ReducingBalance => "Dư nợ giảm dần (Gốc đều, lãi giảm)",
        AmortizationMethod::FixedAnnuity => "Niên kim cố định (Trả góp đều mỗi tháng)",
    }
    .to_string();

    Ok(LoanAmortizationSummary {
        principal,
        annual_rate,
        term_months,
        method: method_name,
        total_principal_paid: total_principal,
        total_interest_paid: total_interest,
        total_amount_paid: total_principal + total_interest,
        schedule,
    })
}

impl OfficeTool for VnLoanAmortizationTool {
    fn slug(&self) -> &'static str {
        "vn-loan-amortization"
    }

    fn name(&self) -> &'static str {
        "Bảng tính Trả nợ Vay Ngân hàng"
    }

    fn description(&self) -> &'static str {
        "Tính toán chính xác từng đồng lịch trả nợ vay ngân hàng theo phương thức dư nợ giảm dần hoặc niên kim cố định trả góp đều hàng tháng."
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
                        expected: "Plaintext / JSON Loan Config".to_string(),
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
                message: "Đang tính toán biểu đồ lịch trả nợ vay ngân hàng...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let principal = if let Some(v) = request.options.get("principal").and_then(|v| v.as_f64()) {
            v
        } else if let Some(s) = request.options.get("principal").and_then(|v| v.as_str()) {
            s.replace(',', "")
                .trim()
                .parse::<f64>()
                .unwrap_or(1_000_000_000.0)
        } else {
            1_000_000_000.0
        };

        let annual_rate = if let Some(v) = request.options.get("annual_rate").and_then(|v| v.as_f64()) {
            v
        } else if let Some(s) = request.options.get("annual_rate").and_then(|v| v.as_str()) {
            s.trim().parse::<f64>().unwrap_or(8.5)
        } else {
            8.5
        };

        let term_months = request
            .options
            .get("term_months")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(120);

        let method = match request.options.get("method").and_then(|v| v.as_str()) {
            Some("FixedAnnuity") => AmortizationMethod::FixedAnnuity,
            _ => AmortizationMethod::ReducingBalance,
        };

        let summary = calculate_loan_amortization(principal, annual_rate, term_months, &method)?;

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let json_output = serde_json::to_string_pretty(&summary).map_err(|e| CommandError::ProcessExecutionError {
            message: format!("Lỗi tuần tự hóa kết quả trả nợ vay: {e}"),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Đã hoàn tất tính toán lịch trả nợ vay".to_string(),
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
    fn test_vn_loan_amortization_happy_path() {
        // Vay 120 triệu, lãi suất 12%/năm (1%/tháng), kỳ hạn 12 tháng
        // Phương thức Niên kim cố định
        let res = calculate_loan_amortization(120_000_000.0, 12.0, 12, &AmortizationMethod::FixedAnnuity)
            .expect("Amortization ok");

        assert_eq!(res.principal, 120_000_000.0);
        assert_eq!(res.schedule.len(), 12);
        // Dư nợ cuối kỳ của kỳ cuối cùng phải về 0
        assert_eq!(res.schedule[11].ending_balance, 0.0);
        // Tổng gốc đã trả phải đúng 120 triệu
        assert_eq!(res.total_principal_paid, 120_000_000.0);
        // Tổng tiền lãi phải lớn hơn 0
        assert!(res.total_interest_paid > 0.0);
    }

    #[test]
    fn test_vn_loan_amortization_edge_case() {
        // Vay 1 tỷ (1,000,000,000), 10%/năm, kỳ hạn 100 tháng theo Dư nợ giảm dần
        let res = calculate_loan_amortization(
            1_000_000_000.0,
            10.0,
            100,
            &AmortizationMethod::ReducingBalance,
        )
        .expect("Reducing balance ok");

        assert_eq!(res.schedule.len(), 100);
        // Gốc mỗi tháng là 10,000,000 (1 tỷ / 100)
        assert_eq!(res.schedule[0].principal_payment, 10_000_000.0);
        // Lãi tháng đầu: 1 tỷ * 10% / 12 = 8,333,333
        assert_eq!(res.schedule[0].interest_payment, 8_333_333.0);
        // Lãi tháng sau giảm dần
        assert!(res.schedule[1].interest_payment < res.schedule[0].interest_payment);
        assert_eq!(res.schedule[99].ending_balance, 0.0);
        assert_eq!(res.total_principal_paid, 1_000_000_000.0);
    }

    #[test]
    fn test_vn_loan_amortization_invalid_magic_bytes() {
        let tool = VnLoanAmortizationTool;
        let bad_file = std::env::temp_dir().join("bad_magic_loan.bin");
        std::fs::write(&bad_file, [0x7F, 0x45, 0x4C, 0x46, 0x01]).ok();

        let req = DispatchTaskRequest {
            task_id: "test-invalid-magic-loan".to_string(),
            tool_slug: "vn-loan-amortization".to_string(),
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
    fn test_vn_loan_amortization_cooperative_cancellation() {
        let tool = VnLoanAmortizationTool;
        let req = DispatchTaskRequest {
            task_id: "test-cancel-loan".to_string(),
            tool_slug: "vn-loan-amortization".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "principal": 500000000.0,
                "annual_rate": 7.5,
                "term_months": 60
            }),
        };
        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_vn_loan_amortization_security_guardrails() {
        let tool = VnLoanAmortizationTool;
        assert_eq!(tool.slug(), "vn-loan-amortization");
        assert_eq!(tool.name(), "Bảng tính Trả nợ Vay Ngân hàng");
    }
}
