use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use chrono::{NaiveDate, NaiveDateTime};
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Danh bạ 63 mã tỉnh/thành phố trực thuộc Trung ương của Việt Nam theo chuẩn Tổng cục Thống kê / Bộ Công an
pub const VIETNAM_PROVINCE_CODES: &[(&str, &str)] = &[
    ("001", "Hà Nội"),
    ("002", "Hà Giang"),
    ("004", "Cao Bằng"),
    ("006", "Bắc Kạn"),
    ("008", "Tuyên Quang"),
    ("010", "Lào Cai"),
    ("011", "Điện Biên"),
    ("012", "Lai Châu"),
    ("014", "Sơn La"),
    ("015", "Yên Bái"),
    ("017", "Hòa Bình"),
    ("019", "Thái Nguyên"),
    ("020", "Lạng Sơn"),
    ("022", "Quảng Ninh"),
    ("024", "Bắc Giang"),
    ("025", "Phú Thọ"),
    ("026", "Vĩnh Phúc"),
    ("027", "Bắc Ninh"),
    ("030", "Hải Dương"),
    ("031", "Hải Phòng"),
    ("033", "Hưng Yên"),
    ("034", "Thái Bình"),
    ("035", "Hà Nam"),
    ("036", "Nam Định"),
    ("037", "Ninh Bình"),
    ("038", "Thanh Hóa"),
    ("040", "Nghệ An"),
    ("042", "Hà Tĩnh"),
    ("044", "Quảng Bình"),
    ("045", "Quảng Trị"),
    ("046", "Thừa Thiên Huế"),
    ("048", "Đà Nẵng"),
    ("049", "Quảng Nam"),
    ("051", "Quảng Ngãi"),
    ("052", "Bình Định"),
    ("054", "Phú Yên"),
    ("056", "Khánh Hòa"),
    ("058", "Ninh Thuận"),
    ("060", "Bình Thuận"),
    ("062", "Kon Tum"),
    ("064", "Gia Lai"),
    ("066", "Đắk Lắk"),
    ("067", "Đắk Nông"),
    ("068", "Lâm Đồng"),
    ("070", "Bình Phước"),
    ("072", "Tây Ninh"),
    ("074", "Bình Dương"),
    ("075", "Đồng Nai"),
    ("077", "Bà Rịa - Vũng Tàu"),
    ("079", "TP. Hồ Chí Minh"),
    ("080", "Long An"),
    ("082", "Tiền Giang"),
    ("083", "Bến Tre"),
    ("084", "Trà Vinh"),
    ("086", "Vĩnh Long"),
    ("087", "Đồng Tháp"),
    ("089", "An Giang"),
    ("091", "Kiên Giang"),
    ("092", "Cần Thơ"),
    ("093", "Hậu Giang"),
    ("094", "Sóc Trăng"),
    ("095", "Bạc Liêu"),
    ("096", "Cà Mau"),
];

/// Thông tin giải mã từ số CCCD 12 số
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CccdInfo {
    pub raw: String,
    pub province_code: String,
    pub province_name: String,
    pub gender: String,
    pub century: u32,
    pub birth_year: u32,
    pub random_sequence: String,
}

/// Thông tin giải mã từ Mã số thuế
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MstInfo {
    pub raw: String,
    pub base_mst: String,
    pub check_digit: u32,
    pub branch_code: Option<String>,
    pub is_13_digits: bool,
}

/// Thông tin giải mã thẻ ngân hàng
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankCardInfo {
    pub raw: String,
    pub length: usize,
    pub is_valid_luhn: bool,
}

/// Phân loại bất thường dữ liệu
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueType {
    InvalidCccd,
    InvalidMst,
    InvalidBankCard,
    InvalidDate,
    FinancialOutlier,
    DuplicatePayment,
}

/// Chi tiết một vấn đề kiểm định chất lượng phát hiện được
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityIssue {
    pub row_index: usize,
    pub column_name: String,
    pub value: String,
    pub issue_type: IssueType,
    pub message: String,
}

/// Báo cáo tổng hợp chất lượng dữ liệu
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaQualityGateReport {
    pub total_rows: usize,
    pub total_columns: usize,
    pub valid_cccd_count: usize,
    pub invalid_cccd_count: usize,
    pub valid_mst_count: usize,
    pub invalid_mst_count: usize,
    pub valid_card_count: usize,
    pub invalid_card_count: usize,
    pub normalized_date_count: usize,
    pub invalid_date_count: usize,
    pub financial_outlier_count: usize,
    pub duplicate_payment_count: usize,
    pub issues: Vec<QualityIssue>,
}

fn default_date_target() -> String {
    "YYYY-MM-DD".to_string()
}

/// Tùy chọn cấu hình cho Cổng kiểm định chất lượng dữ liệu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaQualityGateOptions {
    #[serde(default)]
    pub cccd_columns: Vec<String>,
    #[serde(default)]
    pub mst_columns: Vec<String>,
    #[serde(default)]
    pub card_columns: Vec<String>,
    #[serde(default)]
    pub date_columns: Vec<String>,
    #[serde(default = "default_date_target")]
    pub target_date_format: String,
    #[serde(default)]
    pub amount_columns: Vec<String>,
    #[serde(default)]
    pub recipient_column: Option<String>,
    #[serde(default)]
    pub date_column_for_dup: Option<String>,
    #[serde(default = "default_true")]
    pub auto_detect: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SchemaQualityGateOptions {
    fn default() -> Self {
        Self {
            cccd_columns: Vec::new(),
            mst_columns: Vec::new(),
            card_columns: Vec::new(),
            date_columns: Vec::new(),
            target_date_format: "YYYY-MM-DD".to_string(),
            amount_columns: Vec::new(),
            recipient_column: None,
            date_column_for_dup: None,
            auto_detect: true,
        }
    }
}

pub struct SchemaQualityGateTool;

impl Default for SchemaQualityGateTool {
    fn default() -> Self {
        Self
    }
}

/// Tra cứu tên tỉnh/thành phố theo mã CCCD 3 số
pub fn lookup_province(code: &str) -> Option<&'static str> {
    VIETNAM_PROVINCE_CODES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, name)| *name)
}

/// 1. Kiểm định CCCD 12 số Việt Nam
pub fn validate_vietnamese_cccd(cccd_str: &str) -> Result<CccdInfo, String> {
    let trimmed = cccd_str.trim();
    if trimmed.len() != 12 {
        return Err(format!("Độ dài CCCD phải đúng 12 chữ số, thực tế: {}", trimmed.len()));
    }
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Err("Số CCCD chỉ được chứa các chữ số từ 0-9".to_string());
    }

    let prov_code = &trimmed[0..3];
    let prov_name = lookup_province(prov_code)
        .ok_or_else(|| format!("Mã tỉnh/thành phố '{prov_code}' không hợp lệ trong danh mục Bộ Công an"))?;

    let gender_century_char = trimmed.chars().nth(3).unwrap_or(' ');
    let gender_century_digit = gender_century_char
        .to_digit(10)
        .ok_or_else(|| "Ký tự thế kỷ/giới tính không phải số".to_string())?;

    let (gender, century_base) = match gender_century_digit {
        0 => ("Nam", 1900),
        1 => ("Nữ", 1900),
        2 => ("Nam", 2000),
        3 => ("Nữ", 2000),
        4 => ("Nam", 2100),
        5 => ("Nữ", 2100),
        6 => ("Nam", 2200),
        7 => ("Nữ", 2200),
        8 => ("Nam", 2300),
        9 => ("Nữ", 2300),
        _ => return Err(format!("Chữ số thế kỷ/giới tính '{gender_century_digit}' không hợp lệ")),
    };

    let year_suffix: u32 = trimmed[4..6]
        .parse()
        .map_err(|_| "2 chữ số năm sinh không hợp lệ".to_string())?;
    let birth_year = century_base + year_suffix;

    let random_sequence = trimmed[6..12].to_string();

    Ok(CccdInfo {
        raw: trimmed.to_string(),
        province_code: prov_code.to_string(),
        province_name: prov_name.to_string(),
        gender: gender.to_string(),
        century: (century_base / 100) + 1,
        birth_year,
        random_sequence,
    })
}

/// 2. Kiểm định Mã số thuế (MST 10 số & 13 số) theo thuật toán Modulo 11 chuẩn Tổng cục Thuế
pub fn validate_vietnamese_mst(mst_str: &str) -> Result<MstInfo, String> {
    let trimmed = mst_str.trim();
    let cleaned = trimmed.replace(' ', "");

    let (base_10, branch_code, is_13) = if cleaned.contains('-') {
        let parts: Vec<&str> = cleaned.split('-').collect();
        if parts.len() != 2 {
            return Err("Định dạng MST có dấu gạch ngang không hợp lệ".to_string());
        }
        if parts[0].len() != 10 || parts[1].len() != 3 {
            return Err("MST 13 số chuẩn phải có dạng 10 số - 3 số".to_string());
        }
        (parts[0], Some(parts[1].to_string()), true)
    } else if cleaned.len() == 13 {
        (&cleaned[0..10], Some(cleaned[10..13].to_string()), true)
    } else if cleaned.len() == 10 {
        (cleaned.as_str(), None, false)
    } else {
        return Err(format!("Độ dài MST phải là 10 hoặc 13 số, thực tế: {}", cleaned.len()));
    };

    if !base_10.chars().all(|c| c.is_ascii_digit()) {
        return Err("10 số đầu của MST phải hoàn toàn là chữ số".to_string());
    }
    if let Some(ref br) = branch_code {
        if !br.chars().all(|c| c.is_ascii_digit()) {
            return Err("3 số chi nhánh của MST 13 số phải là chữ số".to_string());
        }
    }

    let digits: Vec<u32> = base_10
        .chars()
        .filter_map(|c| c.to_digit(10))
        .collect();

    if digits.len() != 10 {
        return Err("Không đủ 10 chữ số để kiểm tra MST".to_string());
    }

    // Trọng số chuẩn của Tổng cục Thuế Việt Nam
    let weights = [31, 29, 23, 19, 17, 13, 7, 5, 3];
    let mut sum: u32 = 0;
    for i in 0..9 {
        sum += digits[i] * weights[i];
    }

    let remainder = sum % 11;
    let expected_check_digit = match 11 - remainder {
        10 => 0,
        11 => 0,
        other => other,
    };

    let actual_check_digit = digits[9];
    if actual_check_digit != expected_check_digit {
        return Err(format!(
            "Sai chữ số kiểm tra Modulo 11: thực tế là {actual_check_digit}, tính toán phải là {expected_check_digit}"
        ));
    }

    Ok(MstInfo {
        raw: trimmed.to_string(),
        base_mst: base_10.to_string(),
        check_digit: actual_check_digit,
        branch_code,
        is_13_digits: is_13,
    })
}

/// 3. Kiểm định Thẻ Ngân hàng bằng thuật toán Luhn chuẩn ISO/IEC 7812
pub fn validate_luhn_bank_card(card_str: &str) -> Result<BankCardInfo, String> {
    let cleaned: String = card_str
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();

    if cleaned.len() < 8 || cleaned.len() > 19 {
        return Err(format!(
            "Độ dài số thẻ ngân hàng phải từ 8 đến 19 chữ số, thực tế: {}",
            cleaned.len()
        ));
    }

    if !cleaned.chars().all(|c| c.is_ascii_digit()) {
        return Err("Số thẻ ngân hàng chỉ được chứa các chữ số 0-9".to_string());
    }

    let digits: Vec<u32> = cleaned
        .chars()
        .filter_map(|c| c.to_digit(10))
        .collect();

    let mut sum: u32 = 0;
    let len = digits.len();

    // Duyệt từ phải qua trái: chữ số ở vị trí chẵn từ phải sang (index lẻ từ cuối) nhân 2
    for (i, &digit) in digits.iter().rev().enumerate() {
        if i % 2 == 1 {
            let doubled = digit * 2;
            sum += if doubled > 9 { doubled - 9 } else { doubled };
        } else {
            sum += digit;
        }
    }

    if sum % 10 != 0 {
        return Err(format!("Số thẻ không vượt qua kiểm định Luhn (tổng kiểm tra: {sum})"));
    }

    Ok(BankCardInfo {
        raw: cleaned,
        length: len,
        is_valid_luhn: true,
    })
}

/// 4. Chuẩn hóa Ngày tháng (Hỗ trợ 19 định dạng thời gian chuẩn văn phòng / kế toán)
pub fn normalize_date_flexible(s: &str, target_format: &str) -> Result<String, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("Chuỗi ngày tháng rỗng".to_string());
    }

    // Danh sách 19 formats chuẩn (bao gồm ngày đơn và ngày kèm giờ)
    let date_formats = [
        "%Y-%m-%d",
        "%d/%m/%Y",
        "%m/%d/%Y",
        "%d-%m-%Y",
        "%Y/%m/%d",
        "%d.%m.%Y",
        "%Y.%m.%d",
        "%Y%m%d",
        "%d%m%Y",
    ];

    for fmt in date_formats {
        if let Ok(d) = NaiveDate::parse_from_str(trimmed, fmt) {
            return format_date_output(d, target_format);
        }
    }

    let datetime_formats = [
        "%Y-%m-%d %H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
        "%m/%d/%Y %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%d-%m-%Y %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%d/%m/%Y %H:%M",
    ];

    for fmt in datetime_formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            return format_date_output(dt.date(), target_format);
        }
    }

    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        return format_date_output(dt.date_naive(), target_format);
    }

    // Định dạng linh hoạt với 1 chữ số ngày/tháng (ví dụ D/M/YYYY hoặc D-M-YYYY)
    let re_dmy = Regex::new(r"^(\d{1,2})[/\.-](\d{1,2})[/\.-](\d{4})$").map_err(|e| e.to_string())?;
    if let Some(caps) = re_dmy.captures(trimmed) {
        let d: u32 = caps.get(1).map_or(0, |m| m.as_str().parse().unwrap_or(0));
        let m: u32 = caps.get(2).map_or(0, |m| m.as_str().parse().unwrap_or(0));
        let y: i32 = caps.get(3).map_or(0, |m| m.as_str().parse().unwrap_or(0));
        if let Some(nd) = NaiveDate::from_ymd_opt(y, m, d) {
            return format_date_output(nd, target_format);
        }
    }

    let re_ymd = Regex::new(r"^(\d{4})[/\.-](\d{1,2})[/\.-](\d{1,2})$").map_err(|e| e.to_string())?;
    if let Some(caps) = re_ymd.captures(trimmed) {
        let y: i32 = caps.get(1).map_or(0, |m| m.as_str().parse().unwrap_or(0));
        let m: u32 = caps.get(2).map_or(0, |m| m.as_str().parse().unwrap_or(0));
        let d: u32 = caps.get(3).map_or(0, |m| m.as_str().parse().unwrap_or(0));
        if let Some(nd) = NaiveDate::from_ymd_opt(y, m, d) {
            return format_date_output(nd, target_format);
        }
    }

    Err(format!("Không thể nhận diện định dạng ngày tháng của '{trimmed}'"))
}

fn format_date_output(d: NaiveDate, target_format: &str) -> Result<String, String> {
    match target_format.to_uppercase().as_str() {
        "DD/MM/YYYY" => Ok(d.format("%d/%m/%Y").to_string()),
        "DD-MM-YYYY" => Ok(d.format("%d-%m-%Y").to_string()),
        "YYYY/MM/DD" => Ok(d.format("%Y/%m/%d").to_string()),
        _ => Ok(d.format("%Y-%m-%d").to_string()),
    }
}

/// 5. Phát hiện dị biệt tài chính (IQR Outlier)
pub fn detect_iqr_outliers(values: &[(usize, f64)]) -> Vec<(usize, f64, String)> {
    if values.len() < 4 {
        return Vec::new();
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();
    let q1_idx = n / 4;
    let q3_idx = (3 * n) / 4;

    let q1 = sorted[q1_idx].1;
    let q3 = sorted[q3_idx].1;
    let iqr = q3 - q1;

    let lower_bound = q1 - 1.5 * iqr;
    let upper_bound = q3 + 1.5 * iqr;

    let mut outliers = Vec::new();
    for &(row_idx, val) in values {
        if val < lower_bound || val > upper_bound {
            let msg = format!(
                "Giá trị {val:.2} bất thường ngoài ngưỡng an toàn [{lower_bound:.2}, {upper_bound:.2}] (Q1={q1:.2}, Q3={q3:.2}, IQR={iqr:.2})"
            );
            outliers.push((row_idx, val, msg));
        }
    }

    outliers
}

/// Giao dịch phục vụ kiểm tra thanh toán đúp
#[derive(Debug, Clone)]
pub struct PaymentRecord {
    pub row_index: usize,
    pub recipient: String,
    pub amount: f64,
    pub date: NaiveDate,
}

/// 6. Phát hiện giao dịch thanh toán trùng lặp (Duplicate Payment Detector)
pub fn detect_duplicate_payments(records: &[PaymentRecord]) -> Vec<(usize, usize, String)> {
    let mut warnings = Vec::new();

    for i in 0..records.len() {
        for j in (i + 1)..records.len() {
            let a = &records[i];
            let b = &records[j];

            if a.recipient.eq_ignore_ascii_case(&b.recipient)
                && (a.amount - b.amount).abs() < 0.001
            {
                let day_diff = (a.date - b.date).num_days().abs();
                if day_diff <= 3 {
                    let msg = format!(
                        "Cảnh báo thanh toán đúp: Dòng {} và Dòng {} cùng người nhận '{}', cùng số tiền {:.2}, cách nhau {} ngày",
                        a.row_index + 1,
                        b.row_index + 1,
                        a.recipient,
                        a.amount,
                        day_diff
                    );
                    warnings.push((a.row_index, b.row_index, msg));
                }
            }
        }
    }

    warnings
}

impl OfficeTool for SchemaQualityGateTool {
    fn slug(&self) -> &'static str {
        "schema-quality-gate"
    }

    fn name(&self) -> &'static str {
        "Cổng Kiểm định Chất lượng Dữ liệu & Định danh VN"
    }

    fn description(&self) -> &'static str {
        "Kiểm định tính hợp lệ của số CCCD 12 số, Mã số thuế cá nhân/doanh nghiệp, thẻ ngân hàng Luhn, chuẩn hóa 19 định dạng ngày tháng và phát hiện bất thường tài chính."
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

        let options: SchemaQualityGateOptions = serde_json::from_value(request.options.clone())
            .unwrap_or_default();

        // 1. Chế độ xử lý trực tiếp từ văn bản nếu không truyền tệp
        if request.input_paths.is_empty() {
            if let Some(text_val) = request.options.get("text").and_then(|v| v.as_str()) {
                let raw_rows = super::excel_reader::read_tabular_rows(text_val.as_bytes(), cancel_flag)?;
                let (headers, rows) = if raw_rows.is_empty() {
                    (Vec::new(), Vec::new())
                } else {
                    (raw_rows[0].clone(), raw_rows[1..].to_vec())
                };

                let (report, cleaned_rows) = process_tabular_data(&headers, &rows, &options, cancel_flag)?;
                let report_json = serde_json::to_string_pretty(&report).map_err(|e| {
                    CommandError::ValidationFailed {
                        reason: format!("Lỗi tạo JSON báo cáo: {e}"),
                    }
                })?;

                let mut full_cleaned = vec![headers];
                full_cleaned.extend(cleaned_rows);
                let cleaned_csv = super::excel_reader::format_table_to_csv(&full_cleaned);

                return Ok(DispatchTaskResponse {
                    task_id: request.task_id.clone(),
                    status: "SUCCESS".to_string(),
                    output_paths: vec![report_json, cleaned_csv],
                    failed_items: vec![],
                    elapsed_ms: start_time.elapsed().as_millis() as u64,
                });
            } else {
                return Err(CommandError::ValidationFailed {
                    reason: "Không có tệp đầu vào hoặc nội dung bảng tính cần kiểm định chất lượng".to_string(),
                });
            }
        }

        let total_files = request.input_paths.len();
        let mut output_paths = Vec::new();
        let mut failed_items = Vec::new();

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("quality_gate_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        for (idx, in_path_str) in request.input_paths.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let in_path = Path::new(in_path_str);
            if !in_path.exists() {
                failed_items.push(FileFailureDetail {
                    input_path: in_path_str.clone(),
                    reason: "Tệp nguồn không tồn tại".to_string(),
                });
                continue;
            }

            let raw_bytes = match fs::read(in_path) {
                Ok(b) => b,
                Err(e) => {
                    failed_items.push(FileFailureDetail {
                        input_path: in_path_str.clone(),
                        reason: e.to_string(),
                    });
                    continue;
                }
            };

            // Phòng vệ Magic Bytes tệp thực thi độc hại
            if raw_bytes.starts_with(b"\x7fELF") || raw_bytes.starts_with(b"MZ") {
                return Err(CommandError::InvalidFileSignature {
                    expected: "Bảng tính CSV/XLSX".to_string(),
                    actual: "Tệp thực thi nhị phân bị chặn".to_string(),
                });
            }

            let raw_rows = match super::excel_reader::read_tabular_rows(&raw_bytes, cancel_flag) {
                Ok(r) => r,
                Err(e) => {
                    failed_items.push(FileFailureDetail {
                        input_path: in_path_str.clone(),
                        reason: format!("Lỗi đọc bảng tính: {e:?}"),
                    });
                    continue;
                }
            };

            let (headers, rows) = if raw_rows.is_empty() {
                (Vec::new(), Vec::new())
            } else {
                (raw_rows[0].clone(), raw_rows[1..].to_vec())
            };

            let (report, cleaned_rows) = match process_tabular_data(&headers, &rows, &options, cancel_flag) {
                Ok(res) => res,
                Err(e) => {
                    failed_items.push(FileFailureDetail {
                        input_path: in_path_str.clone(),
                        reason: format!("Lỗi phân tích kiểm định: {e:?}"),
                    });
                    continue;
                }
            };

            let stem = in_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "dataset".to_string());

            let report_file = out_dir.join(format!("{stem}_quality_report.json"));
            let cleaned_file = out_dir.join(format!("{stem}_cleaned.csv"));

            if let Ok(json_str) = serde_json::to_string_pretty(&report) {
                let _ = fs::write(&report_file, json_str);
                output_paths.push(report_file.to_string_lossy().to_string());
            }

            let mut full_cleaned = vec![headers];
            full_cleaned.extend(cleaned_rows);
            let cleaned_csv = super::excel_reader::format_table_to_csv(&full_cleaned);
            let _ = fs::write(&cleaned_file, cleaned_csv);
            output_paths.push(cleaned_file.to_string_lossy().to_string());

            if let Some(ref cb) = on_progress {
                let pct = ((idx + 1) as f32 / total_files as f32) * 100.0;
                cb(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: (idx + 1) as u32,
                    total_steps: total_files as u32,
                    percentage: pct,
                    message: format!("Đã kiểm định xong tệp {}", in_path.file_name().unwrap_or_default().to_string_lossy()),
                    processed_files: idx + 1,
                    failed_files: failed_items.len(),
                    total_files,
                    is_completed: idx + 1 == total_files,
                });
            }
        }

        let status = if failed_items.is_empty() {
            "SUCCESS".to_string()
        } else if output_paths.is_empty() {
            "FAILED".to_string()
        } else {
            "PARTIAL_SUCCESS".to_string()
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

/// Thực hiện phân tích toàn bộ dữ liệu bảng, chuẩn hóa và tổng hợp lỗi
pub fn process_tabular_data(
    headers: &[String],
    rows: &[Vec<String>],
    options: &SchemaQualityGateOptions,
    cancel_flag: &AtomicBool,
) -> Result<(SchemaQualityGateReport, Vec<Vec<String>>), CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let mut report = SchemaQualityGateReport {
        total_rows: rows.len(),
        total_columns: headers.len(),
        valid_cccd_count: 0,
        invalid_cccd_count: 0,
        valid_mst_count: 0,
        invalid_mst_count: 0,
        valid_card_count: 0,
        invalid_card_count: 0,
        normalized_date_count: 0,
        invalid_date_count: 0,
        financial_outlier_count: 0,
        duplicate_payment_count: 0,
        issues: Vec::new(),
    };

    let mut cccd_cols = Vec::new();
    let mut mst_cols = Vec::new();
    let mut card_cols = Vec::new();
    let mut date_cols = Vec::new();
    let mut amount_cols = Vec::new();
    let mut recipient_col_idx = None;
    let mut date_for_dup_col_idx = None;

    for (idx, header) in headers.iter().enumerate() {
        let h_lower = header.to_lowercase();

        let is_cccd = options.cccd_columns.iter().any(|c| c.eq_ignore_ascii_case(header) || c == &idx.to_string())
            || (options.auto_detect && (h_lower.contains("cccd") || h_lower.contains("cmnd") || h_lower.contains("dinh_danh")));
        if is_cccd {
            cccd_cols.push(idx);
        }

        let is_mst = options.mst_columns.iter().any(|c| c.eq_ignore_ascii_case(header) || c == &idx.to_string())
            || (options.auto_detect && (h_lower.contains("mst") || h_lower.contains("ma_so_thue") || h_lower.contains("tax")));
        if is_mst {
            mst_cols.push(idx);
        }

        let is_card = options.card_columns.iter().any(|c| c.eq_ignore_ascii_case(header) || c == &idx.to_string())
            || (options.auto_detect && (h_lower.contains("the") || h_lower.contains("card") || h_lower.contains("atm")));
        if is_card {
            card_cols.push(idx);
        }

        let is_date = options.date_columns.iter().any(|c| c.eq_ignore_ascii_case(header) || c == &idx.to_string())
            || (options.auto_detect && (h_lower.contains("ngay") || h_lower.contains("date") || h_lower.contains("created")));
        if is_date {
            date_cols.push(idx);
        }

        let is_amount = options.amount_columns.iter().any(|c| c.eq_ignore_ascii_case(header) || c == &idx.to_string())
            || (options.auto_detect && (h_lower.contains("tien") || h_lower.contains("amount") || h_lower.contains("gia") || h_lower.contains("total")));
        if is_amount {
            amount_cols.push(idx);
        }

        if let Some(ref r_name) = options.recipient_column {
            if r_name.eq_ignore_ascii_case(header) || r_name == &idx.to_string() {
                recipient_col_idx = Some(idx);
            }
        } else if options.auto_detect && recipient_col_idx.is_none() && (h_lower.contains("nguoi_nhan") || h_lower.contains("recipient") || h_lower.contains("stk") || h_lower.contains("tai_khoan")) {
            recipient_col_idx = Some(idx);
        }

        if let Some(ref d_name) = options.date_column_for_dup {
            if d_name.eq_ignore_ascii_case(header) || d_name == &idx.to_string() {
                date_for_dup_col_idx = Some(idx);
            }
        }
    }

    if date_for_dup_col_idx.is_none() && !date_cols.is_empty() {
        date_for_dup_col_idx = Some(date_cols[0]);
    }

    let mut cleaned_rows = Vec::with_capacity(rows.len());
    let mut amount_values: HashMap<usize, Vec<(usize, f64)>> = HashMap::new();
    let mut payment_records: Vec<PaymentRecord> = Vec::new();

    for (row_idx, row) in rows.iter().enumerate() {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let mut cleaned_row = row.clone();

        // 1. Kiểm tra CCCD
        for &c_idx in &cccd_cols {
            if let Some(val) = row.get(c_idx) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    match validate_vietnamese_cccd(trimmed) {
                        Ok(_) => report.valid_cccd_count += 1,
                        Err(e) => {
                            report.invalid_cccd_count += 1;
                            report.issues.push(QualityIssue {
                                row_index: row_idx + 1,
                                column_name: headers.get(c_idx).cloned().unwrap_or_default(),
                                value: trimmed.to_string(),
                                issue_type: IssueType::InvalidCccd,
                                message: e,
                            });
                        }
                    }
                }
            }
        }

        // 2. Kiểm tra MST
        for &m_idx in &mst_cols {
            if let Some(val) = row.get(m_idx) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    match validate_vietnamese_mst(trimmed) {
                        Ok(_) => report.valid_mst_count += 1,
                        Err(e) => {
                            report.invalid_mst_count += 1;
                            report.issues.push(QualityIssue {
                                row_index: row_idx + 1,
                                column_name: headers.get(m_idx).cloned().unwrap_or_default(),
                                value: trimmed.to_string(),
                                issue_type: IssueType::InvalidMst,
                                message: e,
                            });
                        }
                    }
                }
            }
        }

        // 3. Kiểm tra Thẻ Ngân hàng
        for &k_idx in &card_cols {
            if let Some(val) = row.get(k_idx) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    match validate_luhn_bank_card(trimmed) {
                        Ok(_) => report.valid_card_count += 1,
                        Err(e) => {
                            report.invalid_card_count += 1;
                            report.issues.push(QualityIssue {
                                row_index: row_idx + 1,
                                column_name: headers.get(k_idx).cloned().unwrap_or_default(),
                                value: trimmed.to_string(),
                                issue_type: IssueType::InvalidBankCard,
                                message: e,
                            });
                        }
                    }
                }
            }
        }

        // 4. Chuẩn hóa ngày tháng
        let mut parsed_row_date: Option<NaiveDate> = None;
        for &d_idx in &date_cols {
            if let Some(val) = row.get(d_idx) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    match normalize_date_flexible(trimmed, &options.target_date_format) {
                        Ok(norm) => {
                            report.normalized_date_count += 1;
                            if let Some(cell) = cleaned_row.get_mut(d_idx) {
                                *cell = norm.clone();
                            }
                            if date_for_dup_col_idx == Some(d_idx) {
                                if let Ok(nd) = NaiveDate::parse_from_str(&norm, "%Y-%m-%d") {
                                    parsed_row_date = Some(nd);
                                }
                            }
                        }
                        Err(e) => {
                            report.invalid_date_count += 1;
                            report.issues.push(QualityIssue {
                                row_index: row_idx + 1,
                                column_name: headers.get(d_idx).cloned().unwrap_or_default(),
                                value: trimmed.to_string(),
                                issue_type: IssueType::InvalidDate,
                                message: e,
                            });
                        }
                    }
                }
            }
        }

        // 5. Thu thập số tiền cho Outlier
        let mut parsed_row_amount: Option<f64> = None;
        for &a_idx in &amount_cols {
            if let Some(val) = row.get(a_idx) {
                let cleaned_num = val.trim().replace(',', "");
                if let Ok(num) = cleaned_num.parse::<f64>() {
                    if num.is_finite() {
                        amount_values.entry(a_idx).or_default().push((row_idx, num));
                        if parsed_row_amount.is_none() {
                            parsed_row_amount = Some(num);
                        }
                    }
                }
            }
        }

        // 6. Ghi nhận cho kiểm tra Duplicate Payment
        if let (Some(r_idx), Some(amt), Some(dt)) = (recipient_col_idx, parsed_row_amount, parsed_row_date) {
            if let Some(recip) = row.get(r_idx) {
                let trimmed_recip = recip.trim();
                if !trimmed_recip.is_empty() {
                    payment_records.push(PaymentRecord {
                        row_index: row_idx,
                        recipient: trimmed_recip.to_string(),
                        amount: amt,
                        date: dt,
                    });
                }
            }
        }

        cleaned_rows.push(cleaned_row);
    }

    // 7. Quét Outlier tài chính bằng IQR
    for (a_idx, vals) in amount_values {
        let outliers = detect_iqr_outliers(&vals);
        for (r_idx, val, msg) in outliers {
            report.financial_outlier_count += 1;
            report.issues.push(QualityIssue {
                row_index: r_idx + 1,
                column_name: headers.get(a_idx).cloned().unwrap_or_default(),
                value: format!("{val}"),
                issue_type: IssueType::FinancialOutlier,
                message: msg,
            });
        }
    }

    // 8. Quét Duplicate Payments
    if !payment_records.is_empty() {
        let dups = detect_duplicate_payments(&payment_records);
        for (r_a, r_b, msg) in dups {
            report.duplicate_payment_count += 1;
            report.issues.push(QualityIssue {
                row_index: r_a + 1,
                column_name: recipient_col_idx.and_then(|i| headers.get(i)).cloned().unwrap_or_else(|| "GiaoDich".to_string()),
                value: format!("Dòng {} & {}", r_a + 1, r_b + 1),
                issue_type: IssueType::DuplicatePayment,
                message: msg,
            });
        }
    }

    Ok((report, cleaned_rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_quality_gate_cccd_validation() {
        // Hợp lệ: 001 (Hà Nội), 0 (Nam TK20), 98 (1998), 123456
        let res = validate_vietnamese_cccd("001098123456");
        assert!(res.is_ok());
        let info = res.expect("valid cccd");
        assert_eq!(info.province_name, "Hà Nội");
        assert_eq!(info.gender, "Nam");
        assert_eq!(info.birth_year, 1998);

        // Hợp lệ: 079 (TP.HCM), 3 (Nữ TK21), 05 (2005), 654321
        let res2 = validate_vietnamese_cccd("079305654321");
        assert!(res2.is_ok());
        let info2 = res2.expect("valid cccd 2");
        assert_eq!(info2.province_name, "TP. Hồ Chí Minh");
        assert_eq!(info2.gender, "Nữ");
        assert_eq!(info2.birth_year, 2005);

        // Mã tỉnh sai
        let err_prov = validate_vietnamese_cccd("999098123456");
        assert!(err_prov.is_err());
        assert!(err_prov.unwrap_err().contains("Mã tỉnh/thành phố"));

        // Chiều dài sai
        let err_len = validate_vietnamese_cccd("001098123");
        assert!(err_len.is_err());

        // Chứa ký tự không phải số
        let err_char = validate_vietnamese_cccd("00109812345A");
        assert!(err_char.is_err());
    }

    #[test]
    fn test_schema_quality_gate_mst_modulo11() {
        // Test MST hợp lệ theo công thức: 0300000019 (sum = 3*29 + 1*3 = 90, 90%11 = 2, 11-2 = 9)
        let res = validate_vietnamese_mst("0300000019");
        assert!(res.is_ok());
        let info = res.expect("valid mst");
        assert_eq!(info.check_digit, 9);
        assert!(!info.is_13_digits);

        // Test MST 13 số hợp lệ có dấu gạch ngang
        let res13 = validate_vietnamese_mst("0300000019-001");
        assert!(res13.is_ok());
        let info13 = res13.expect("valid mst 13");
        assert_eq!(info13.branch_code.as_deref(), Some("001"));
        assert!(info13.is_13_digits);

        // Test MST sai số kiểm tra
        let err_check = validate_vietnamese_mst("0300000015");
        assert!(err_check.is_err());
        assert!(err_check.unwrap_err().contains("Sai chữ số kiểm tra Modulo 11"));

        // Sai độ dài
        let err_len = validate_vietnamese_mst("030001");
        assert!(err_len.is_err());
    }

    #[test]
    fn test_schema_quality_gate_luhn_card() {
        // Số thẻ Luhn chuẩn ISO/IEC 7812: 79927398713
        let res = validate_luhn_bank_card("79927398713");
        assert!(res.is_ok());

        // Thẻ test chuẩn
        let res_visa = validate_luhn_bank_card("4000001234562");
        assert!(res_visa.is_ok());

        // Thẻ sai checksum
        let err_luhn = validate_luhn_bank_card("79927398714");
        assert!(err_luhn.is_err());
        assert!(err_luhn.unwrap_err().contains("không vượt qua kiểm định Luhn"));

        // Thẻ quá ngắn
        let err_len = validate_luhn_bank_card("12345");
        assert!(err_len.is_err());
    }

    #[test]
    fn test_schema_quality_gate_date_normalization() {
        // 1. DD/MM/YYYY
        let d1 = normalize_date_flexible("18/05/2024", "YYYY-MM-DD").expect("d1");
        assert_eq!(d1, "2024-05-18");

        // 2. YYYY-MM-DD
        let d2 = normalize_date_flexible("2024-12-31", "DD/MM/YYYY").expect("d2");
        assert_eq!(d2, "31/12/2024");

        // 3. DD-MM-YYYY
        let d3 = normalize_date_flexible("01-01-2023", "YYYY-MM-DD").expect("d3");
        assert_eq!(d3, "2023-01-01");

        // 4. YYYY/MM/DD
        let d4 = normalize_date_flexible("2023/10/25", "YYYY-MM-DD").expect("d4");
        assert_eq!(d4, "2023-10-25");

        // 5. YYYYMMDD
        let d5 = normalize_date_flexible("20240229", "YYYY-MM-DD").expect("d5");
        assert_eq!(d5, "2024-02-29");

        // 6. DateTime có giờ phút giây: 2024-05-18 14:30:00
        let d6 = normalize_date_flexible("2024-05-18 14:30:00", "YYYY-MM-DD").expect("d6");
        assert_eq!(d6, "2024-05-18");

        // 7. Định dạng linh hoạt 1 chữ số ngày/tháng: 5/2/2024
        let d7 = normalize_date_flexible("5/2/2024", "YYYY-MM-DD").expect("d7");
        assert_eq!(d7, "2024-02-05");

        // Ngày không hợp lệ
        let err_date = normalize_date_flexible("invalid_date_str", "YYYY-MM-DD");
        assert!(err_date.is_err());
    }

    #[test]
    fn test_schema_quality_gate_outlier_and_duplicate() {
        // Test Outlier IQR
        let values = vec![
            (0, 100.0),
            (1, 102.0),
            (2, 105.0),
            (3, 98.0),
            (4, 101.0),
            (5, 103.0),
            (6, 10000.0), // Cực đại dị biệt
        ];
        let outliers = detect_iqr_outliers(&values);
        assert_eq!(outliers.len(), 1);
        assert_eq!(outliers[0].0, 6);
        assert_eq!(outliers[0].1, 10000.0);

        // Test Duplicate Payments
        let recs = vec![
            PaymentRecord {
                row_index: 0,
                recipient: "Nguyen Van A".to_string(),
                amount: 5000000.0,
                date: NaiveDate::from_ymd_opt(2024, 5, 1).unwrap(),
            },
            PaymentRecord {
                row_index: 1,
                recipient: "Nguyen Van A".to_string(),
                amount: 5000000.0,
                date: NaiveDate::from_ymd_opt(2024, 5, 2).unwrap(), // Cách 1 ngày -> Thanh toán đúp
            },
            PaymentRecord {
                row_index: 2,
                recipient: "Tran Thi B".to_string(),
                amount: 3000000.0,
                date: NaiveDate::from_ymd_opt(2024, 5, 1).unwrap(),
            },
        ];

        let dups = detect_duplicate_payments(&recs);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].0, 0);
        assert_eq!(dups[0].1, 1);
    }

    #[test]
    fn test_schema_quality_gate_cooperative_cancellation() {
        let cancel = AtomicBool::new(true);
        let tool = SchemaQualityGateTool;
        let req = DispatchTaskRequest {
            task_id: "cancel_task".to_string(),
            tool_slug: "schema-quality-gate".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: serde_json::json!({
                "text": "CCCD,Ten\n001098123456,Nguyen Van A"
            }),
        };

        let res = tool.execute(&req, &cancel, None);
        assert_eq!(res.unwrap_err(), CommandError::TaskCancelled);
    }

    #[test]
    fn test_schema_quality_gate_invalid_magic_bytes() {
        let cancel = AtomicBool::new(false);
        let tool = SchemaQualityGateTool;

        // Tạo tệp giả mạo nhị phân ELF
        let elf_path = std::env::temp_dir().join("malicious_test.elf");
        let _ = fs::write(&elf_path, b"\x7fELFfakeexecutabledata");

        let req = DispatchTaskRequest {
            task_id: "security_test".to_string(),
            tool_slug: "schema-quality-gate".to_string(),
            input_paths: vec![elf_path.to_string_lossy().to_string()],
            output_directory: None,
            options: serde_json::json!({}),
        };

        let res = tool.execute(&req, &cancel, None);
        assert!(matches!(res, Err(CommandError::InvalidFileSignature { .. })));

        let _ = fs::remove_file(&elf_path);
    }
}
