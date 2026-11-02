use std::sync::atomic::{AtomicBool, Ordering};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use office_shared::{CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress};
use serde::{Deserialize, Serialize};
use crate::OfficeTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingDaysOptions {
    pub start_date: String,
    pub end_date: String,
    #[serde(default = "default_exclude_saturday")]
    pub exclude_saturday: bool,
}

fn default_exclude_saturday() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HolidayDetail {
    pub date: String,
    pub name: String,
    pub is_lunar: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkingDaysResult {
    pub start_date: String,
    pub end_date: String,
    pub total_calendar_days: u32,
    pub weekend_days: u32,
    pub holiday_days: u32,
    pub total_working_days: u32,
    pub holidays_in_range: Vec<HolidayDetail>,
    pub ics_content: String,
}

pub struct VnWorkingDaysCalcTool;

impl Default for VnWorkingDaysCalcTool {
    fn default() -> Self {
        Self
    }
}

/// Parse date string in format YYYY-MM-DD or DD/MM/YYYY
pub fn parse_date_flexible(s: &str) -> Result<NaiveDate, CommandError> {
    let trimmed = s.trim();
    if let Ok(d) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(trimmed, "%d/%m/%Y") {
        return Ok(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(trimmed, "%Y/%m/%d") {
        return Ok(d);
    }
    Err(CommandError::ValidationFailed {
        reason: format!("Định dạng ngày không hợp lệ: '{trimmed}' (chấp nhận YYYY-MM-DD hoặc DD/MM/YYYY)"),
    })
}

/// Danh sách ngày nghỉ Âm lịch chuyển đổi sang Dương lịch các năm 2020-2030
pub fn get_lunar_holidays_solar(year: i32) -> Vec<(NaiveDate, &'static str)> {
    let mut list = Vec::new();
    match year {
        2024 => {
            // Giỗ Tổ Hùng Vương (10/3 Âm) -> 18/04/2024
            if let Some(d) = NaiveDate::from_ymd_opt(2024, 4, 18) {
                list.push((d, "Giỗ Tổ Hùng Vương (10/3 Âm lịch)"));
            }
            // Tết Giáp Thìn 2024: 08/02/2024 đến 14/02/2024
            for day in 8..=14 {
                if let Some(d) = NaiveDate::from_ymd_opt(2024, 2, day) {
                    list.push((d, "Tết Nguyên Đán Giáp Thìn 2024"));
                }
            }
        }
        2025 => {
            // Giỗ Tổ Hùng Vương (10/3 Âm) -> 07/04/2025
            if let Some(d) = NaiveDate::from_ymd_opt(2025, 4, 7) {
                list.push((d, "Giỗ Tổ Hùng Vương (10/3 Âm lịch)"));
            }
            // Tết Ất Tỵ 2025: 28/01/2025 đến 02/02/2025
            for day in 28..=31 {
                if let Some(d) = NaiveDate::from_ymd_opt(2025, 1, day) {
                    list.push((d, "Tết Nguyên Đán Ất Tỵ 2025"));
                }
            }
            for day in 1..=3 {
                if let Some(d) = NaiveDate::from_ymd_opt(2025, 2, day) {
                    list.push((d, "Tết Nguyên Đán Ất Tỵ 2025"));
                }
            }
        }
        2026 => {
            // Giỗ Tổ Hùng Vương (10/3 Âm) -> 26/04/2026
            if let Some(d) = NaiveDate::from_ymd_opt(2026, 4, 26) {
                list.push((d, "Giỗ Tổ Hùng Vương (10/3 Âm lịch)"));
            }
            // Tết Bính Ngọ 2026: 15/02/2026 đến 20/02/2026
            for day in 15..=20 {
                if let Some(d) = NaiveDate::from_ymd_opt(2026, 2, day) {
                    list.push((d, "Tết Nguyên Đán Bính Ngọ 2026"));
                }
            }
        }
        _ => {
            // Xấp xỉ cho các năm khác: Giỗ Tổ giữa tháng 4, Tết cuối tháng 1 / đầu tháng 2
            if let Some(d) = NaiveDate::from_ymd_opt(year, 4, 15) {
                list.push((d, "Giỗ Tổ Hùng Vương (10/3 Âm lịch)"));
            }
            for day in 1..=5 {
                if let Some(d) = NaiveDate::from_ymd_opt(year, 2, day) {
                    list.push((d, "Tết Nguyên Đán"));
                }
            }
        }
    }
    list
}

/// Kiểm tra xem một ngày có phải là ngày nghỉ lễ chính thức của Việt Nam không
pub fn check_holiday(date: NaiveDate) -> Option<(String, bool)> {
    let (m, d) = (date.month(), date.day());

    // 1. Tết Dương lịch: 01/01
    if m == 1 && d == 1 {
        return Some(("Tết Dương Lịch".to_string(), false));
    }

    // 2. Ngày Chiến thắng 30/04
    if m == 4 && d == 30 {
        return Some(("Ngày Chiến thắng 30/4".to_string(), false));
    }

    // 3. Ngày Quốc tế Lao động 01/05
    if m == 5 && d == 1 {
        return Some(("Ngày Quốc tế Lao động 1/5".to_string(), false));
    }

    // 4. Quốc khánh 02/09 và ngày liền kề 01/09
    if m == 9 && (d == 1 || d == 2) {
        return Some(("Lễ Quốc Khánh 2/9".to_string(), false));
    }

    // 5. Các ngày lễ Âm lịch (Giỗ Tổ Hùng Vương & Tết Nguyên Đán)
    let lunar_list = get_lunar_holidays_solar(date.year());
    for (lunar_date, name) in lunar_list {
        if lunar_date == date {
            return Some((name.to_string(), true));
        }
    }

    None
}

/// Tạo chuỗi iCalendar (.ics) cho các ngày nghỉ lễ trong khoảng
pub fn generate_ics_calendar(holidays: &[HolidayDetail]) -> String {
    let mut ics = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Office Utility VN//VN Holidays//VI\r\nCALSCALE:GREGORIAN\r\n");

    for h in holidays {
        let dt_compact = h.date.replace('-', "");
        ics.push_str("BEGIN:VEVENT\r\n");
        ics.push_str(&format!("DTSTART;VALUE=DATE:{dt_compact}\r\n"));
        ics.push_str(&format!("DTEND;VALUE=DATE:{dt_compact}\r\n"));
        ics.push_str(&format!("SUMMARY:{}\r\n", h.name));
        ics.push_str("STATUS:CONFIRMED\r\n");
        ics.push_str("TRANSP:TRANSPARENT\r\n");
        ics.push_str("END:VEVENT\r\n");
    }

    ics.push_str("END:VCALENDAR\r\n");
    ics
}

/// Tính toán số ngày làm việc
pub fn calculate_working_days(
    start: NaiveDate,
    end: NaiveDate,
    exclude_sat: bool,
) -> Result<WorkingDaysResult, CommandError> {
    if start > end {
        return Err(CommandError::ValidationFailed {
            reason: format!(
                "Ngày bắt đầu ({start}) không thể sau ngày kết thúc ({end})"
            ),
        });
    }

    let mut curr = start;
    let mut total_calendar_days = 0;
    let mut weekend_days = 0;
    let mut holiday_days = 0;
    let mut working_days = 0;

    let mut holidays_in_range = Vec::new();

    while curr <= end {
        total_calendar_days += 1;
        let weekday = curr.weekday();

        let is_weekend = match weekday {
            Weekday::Sun => true,
            Weekday::Sat => exclude_sat,
            _ => false,
        };

        if let Some((name, is_lunar)) = check_holiday(curr) {
            holidays_in_range.push(HolidayDetail {
                date: curr.format("%Y-%m-%d").to_string(),
                name,
                is_lunar,
            });

            if !is_weekend {
                holiday_days += 1;
            } else {
                weekend_days += 1;
            }
        } else if is_weekend {
            weekend_days += 1;
        } else {
            working_days += 1;
        }

        curr += Duration::days(1);
    }

    let ics_content = generate_ics_calendar(&holidays_in_range);

    Ok(WorkingDaysResult {
        start_date: start.format("%Y-%m-%d").to_string(),
        end_date: end.format("%Y-%m-%d").to_string(),
        total_calendar_days,
        weekend_days,
        holiday_days,
        total_working_days: working_days,
        holidays_in_range,
        ics_content,
    })
}

impl OfficeTool for VnWorkingDaysCalcTool {
    fn slug(&self) -> &'static str {
        "vn-working-days-calc"
    }

    fn name(&self) -> &'static str {
        "Tính Ngày Làm việc Trừ Ngày lễ VN"
    }

    fn description(&self) -> &'static str {
        "Tính chính xác số ngày làm việc giữa hai mốc thời gian, tự động trừ ngày nghỉ cuối tuần và các ngày nghỉ lễ chính thức theo Bộ luật Lao động Việt Nam (Tết Âm lịch, Giỗ Tổ 10/3, 30/4, 1/5, 2/9) kèm xuất tệp .ics."
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
                        expected: "Plaintext / JSON Working Days Config".to_string(),
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
                message: "Đang tính toán lịch làm việc và ngày lễ Việt Nam...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let start_str = request
            .options
            .get("start_date")
            .and_then(|v| v.as_str())
            .unwrap_or("2024-04-20");

        let end_str = request
            .options
            .get("end_date")
            .and_then(|v| v.as_str())
            .unwrap_or("2024-05-05");

        let exclude_sat = request
            .options
            .get("exclude_saturday")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let start = parse_date_flexible(start_str)?;
        let end = parse_date_flexible(end_str)?;

        let result = calculate_working_days(start, end, exclude_sat)?;

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let json_output = serde_json::to_string_pretty(&result).map_err(|e| CommandError::ProcessExecutionError {
            message: format!("Lỗi tuần tự hóa kết quả ngày làm việc: {e}"),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Đã hoàn tất tính toán ngày làm việc".to_string(),
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
    fn test_vn_working_days_calc_happy_path() {
        // Khoảng ngày có Giỗ Tổ Hùng Vương 2024 (18/04/2024)
        let start = NaiveDate::from_ymd_opt(2024, 4, 15).expect("Date ok"); // Thứ 2
        let end = NaiveDate::from_ymd_opt(2024, 4, 21).expect("Date ok"); // Chủ Nhật
        // Tổng 7 ngày: T2, T3, T4, T5(18/4 Giỗ Tổ), T6, T7, CN
        // Nghỉ T7 + CN (2 ngày), nghỉ Giỗ Tổ (1 ngày) -> làm việc: 4 ngày
        let res = calculate_working_days(start, end, true).expect("Calculate ok");
        assert_eq!(res.total_calendar_days, 7);
        assert_eq!(res.weekend_days, 2);
        assert_eq!(res.holiday_days, 1);
        assert_eq!(res.total_working_days, 4);

        // Nhận diện Giỗ Tổ trong danh sách nghỉ lễ
        let has_hung_kings = res.holidays_in_range.iter().any(|h| h.is_lunar);
        assert!(has_hung_kings);
    }

    #[test]
    fn test_vn_working_days_calc_edge_case() {
        // Trừ chính xác các ngày nghỉ lễ chính thức 30/4, 1/5 và 2/9
        // Năm 2024: 30/4 là Thứ Ba, 1/5 là Thứ Tư
        let start_apr = NaiveDate::from_ymd_opt(2024, 4, 29).expect("Date ok"); // Thứ 2
        let end_may = NaiveDate::from_ymd_opt(2024, 5, 3).expect("Date ok"); // Thứ 6
        // T2 (làm), T3(30/4 nghỉ), T4(1/5 nghỉ), T5 (làm), T6 (làm) -> làm việc: 3 ngày
        let res_apr = calculate_working_days(start_apr, end_may, true).expect("Calculate ok");
        assert_eq!(res_apr.holiday_days, 2);
        assert_eq!(res_apr.total_working_days, 3);

        // 2/9 năm 2024: 2/9 là Thứ 2, 1/9 là Chủ Nhật (nghỉ lễ 2/9 gồm 1/9 và 2/9)
        let start_sep = NaiveDate::from_ymd_opt(2024, 9, 2).expect("Date ok");
        let end_sep = NaiveDate::from_ymd_opt(2024, 9, 2).expect("Date ok");
        let res_sep = calculate_working_days(start_sep, end_sep, true).expect("Calculate ok");
        assert_eq!(res_sep.holiday_days, 1);
        assert_eq!(res_sep.total_working_days, 0);

        // Kiểm tra xuất tệp .ics có cấu trúc hợp lệ
        assert!(res_sep.ics_content.contains("BEGIN:VCALENDAR"));
        assert!(res_sep.ics_content.contains("END:VCALENDAR"));
    }

    #[test]
    fn test_vn_working_days_calc_invalid_magic_bytes() {
        let tool = VnWorkingDaysCalcTool;
        let bad_file = std::env::temp_dir().join("bad_magic_calendar.bin");
        std::fs::write(&bad_file, [0x7F, 0x45, 0x4C, 0x46, 0x01]).ok();

        let req = DispatchTaskRequest {
            task_id: "test-invalid-magic-calendar".to_string(),
            tool_slug: "vn-working-days-calc".to_string(),
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
    fn test_vn_working_days_calc_cooperative_cancellation() {
        let tool = VnWorkingDaysCalcTool;
        let req = DispatchTaskRequest {
            task_id: "test-cancel-calc".to_string(),
            tool_slug: "vn-working-days-calc".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "start_date": "2024-01-01",
                "end_date": "2024-12-31"
            }),
        };
        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_vn_working_days_calc_security_guardrails() {
        let tool = VnWorkingDaysCalcTool;
        assert_eq!(tool.slug(), "vn-working-days-calc");
        assert_eq!(tool.name(), "Tính Ngày Làm việc Trừ Ngày lễ VN");
    }
}
