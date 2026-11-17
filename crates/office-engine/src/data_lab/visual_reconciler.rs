use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use chrono::NaiveDate;
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Trạng thái phân loại kết quả đối soát theo 4 nhóm chuẩn
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationStatus {
    /// Nhóm 1: Khớp mã khóa, số tiền và ngày giao dịch nằm trong dung sai cho phép
    KhopHoanToan,
    /// Nhóm 2: Khớp mã khóa nhưng số tiền hoặc ngày giao dịch vượt ngưỡng dung sai
    KhopMaLechTien,
    /// Nhóm 3: Có trên sao kê ngân hàng nhưng không tìm thấy trong sổ nội bộ
    ChiCoOSaoKe,
    /// Nhóm 4: Có trong sổ nội bộ nhưng chưa ghi nhận giao dịch trên sao kê
    ChiCoOSoNoiBo,
}

impl ReconciliationStatus {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::KhopHoanToan => "Khớp hoàn toàn",
            Self::KhopMaLechTien => "Khớp mã - Lệch tiền/ngày",
            Self::ChiCoOSaoKe => "Chỉ có ở sao kê",
            Self::ChiCoOSoNoiBo => "Chỉ có ở sổ nội bộ",
        }
    }
}

/// Chi tiết một dòng kết quả đối soát
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciledItem {
    pub match_key: String,
    pub status: ReconciliationStatus,
    pub amount_a: Option<f64>,
    pub amount_b: Option<f64>,
    pub amount_diff: Option<f64>,
    pub date_a: Option<String>,
    pub date_b: Option<String>,
    pub date_diff_days: Option<i64>,
    pub row_a: Option<HashMap<String, String>>,
    pub row_b: Option<HashMap<String, String>>,
    pub note: String,
}

/// Báo cáo tổng thể đối soát dữ liệu
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualReconcilerReport {
    pub total_records_a: usize,
    pub total_records_b: usize,
    pub total_matched_exact: usize,
    pub total_matched_diff: usize,
    pub total_only_in_statement: usize,
    pub total_only_in_ledger: usize,
    pub total_amount_a: f64,
    pub total_amount_b: f64,
    pub items: Vec<ReconciledItem>,
}

/// Cấu hình tùy chọn đối soát đa khóa và dung sai
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualReconcilerOptions {
    /// Cột chứa mã khóa hoặc nội dung giao dịch của Bảng A (Sao kê ngân hàng)
    #[serde(default, alias = "statement_key_column")]
    pub key_column_a: String,

    /// Cột chứa mã khóa đối soát của Bảng B (Sổ nội bộ / Đơn hàng)
    #[serde(default, alias = "ledger_key_column")]
    pub key_column_b: String,

    /// Cột số tiền Bảng A
    #[serde(default, alias = "statement_amount_column")]
    pub amount_column_a: String,

    /// Cột số tiền Bảng B
    #[serde(default, alias = "ledger_amount_column")]
    pub amount_column_b: String,

    /// Cột ngày giao dịch Bảng A (tùy chọn)
    #[serde(default)]
    pub date_column_a: Option<String>,

    /// Cột ngày giao dịch Bảng B (tùy chọn)
    #[serde(default)]
    pub date_column_b: Option<String>,

    /// Biểu thức Regex trích xuất mã đơn từ nội dung mờ (ví dụ r"DH\d+" hoặc r"(?:DH|ORD)[0-9]+")
    #[serde(default)]
    pub key_pattern: Option<String>,

    /// Dung sai số tiền cho phép (mặc định 0.0)
    #[serde(default)]
    pub amount_tolerance: f64,

    /// Dung sai ngày làm việc/giao dịch cho phép (mặc định 0 ngày)
    #[serde(default)]
    pub date_tolerance_days: u32,

    /// Dữ liệu dạng dòng (truyền trực tiếp qua options JSON)
    #[serde(default)]
    pub source_a_rows: Option<Vec<HashMap<String, String>>>,

    #[serde(default)]
    pub source_b_rows: Option<Vec<HashMap<String, String>>>,

    /// Dữ liệu dạng bảng CSV/TSV (chuỗi văn bản)
    #[serde(default)]
    pub table_a: Option<String>,

    #[serde(default)]
    pub table_b: Option<String>,
}

impl Default for VisualReconcilerOptions {
    fn default() -> Self {
        Self {
            key_column_a: String::new(),
            key_column_b: String::new(),
            amount_column_a: String::new(),
            amount_column_b: String::new(),
            date_column_a: None,
            date_column_b: None,
            key_pattern: None,
            amount_tolerance: 0.0,
            date_tolerance_days: 0,
            source_a_rows: None,
            source_b_rows: None,
            table_a: None,
            table_b: None,
        }
    }
}

pub struct VisualReconcilerTool;

impl Default for VisualReconcilerTool {
    fn default() -> Self {
        Self
    }
}

/// Trích xuất mã khóa từ chuỗi văn bản (hỗ trợ Regex Substring / Pattern matching)
pub fn extract_key_from_text(text: &str, regex: Option<&Regex>) -> String {
    let trimmed = text.trim();
    if let Some(re) = regex {
        if let Some(caps) = re.captures(trimmed) {
            // Ưu tiên Capture Group 1 nếu có, ngược lại lấy toàn bộ Match Group 0
            if let Some(g1) = caps.get(1) {
                let m = g1.as_str().trim();
                if !m.is_empty() {
                    return m.to_string();
                }
            }
            if let Some(g0) = caps.get(0) {
                let m = g0.as_str().trim();
                if !m.is_empty() {
                    return m.to_string();
                }
            }
        }
    }
    trimmed.to_string()
}

/// Giải mã số tiền linh hoạt hỗ trợ định dạng kế toán VN & Quốc tế
pub fn parse_amount(s: &str) -> Option<f64> {
    let mut cleaned = s.trim().to_string();
    if cleaned.is_empty() {
        return None;
    }

    // Loại bỏ ký hiệu tiền tệ
    for token in &[
        "₫", "đ", "Đ", "VND", "vnd", "Vnd", "USD", "usd", "$", "EUR", "eur", "€",
    ] {
        cleaned = cleaned.replace(token, "");
    }
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return None;
    }

    let is_negative = cleaned.starts_with('-') || (cleaned.starts_with('(') && cleaned.ends_with(')'));
    let num_part = cleaned
        .trim_start_matches('-')
        .trim_start_matches('+')
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim();

    let has_dot = num_part.contains('.');
    let has_comma = num_part.contains(',');

    let normalized = if has_dot && has_comma {
        let last_dot = num_part.rfind('.').unwrap_or(0);
        let last_comma = num_part.rfind(',').unwrap_or(0);
        if last_comma > last_dot {
            // Định dạng Việt Nam/Châu Âu: 1.500.000,50
            num_part.replace('.', "").replace(',', ".")
        } else {
            // Định dạng Quốc tế: 1,500,000.50
            num_part.replace(',', "")
        }
    } else if has_comma {
        let comma_count = num_part.matches(',').count();
        if comma_count > 1 {
            // Phân cách hàng nghìn nhiều dấu phẩy: 1,500,000
            num_part.replace(',', "")
        } else {
            let parts: Vec<&str> = num_part.split(',').collect();
            if parts.len() == 2 && parts[1].len() == 3 && parts[0].len() <= 3 {
                num_part.replace(',', "")
            } else {
                num_part.replace(',', ".")
            }
        }
    } else if has_dot {
        let dot_count = num_part.matches('.').count();
        if dot_count > 1 {
            // Phân cách hàng nghìn nhiều dấu chấm: 1.500.000
            num_part.replace('.', "")
        } else {
            let parts: Vec<&str> = num_part.split('.').collect();
            if parts.len() == 2 && parts[1].len() == 3 && parts[0].len() <= 3 {
                num_part.replace('.', "")
            } else {
                num_part.to_string()
            }
        }
    } else {
        num_part.replace(' ', "")
    };

    let mut val: f64 = normalized.trim().parse().ok()?;
    if is_negative {
        val = -val;
    }
    Some(val)
}

/// Nhận diện ngày tháng linh hoạt không gây panic
pub fn parse_date_flexible(s: &str) -> Option<NaiveDate> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    let date_token = trimmed.split_whitespace().next().unwrap_or(trimmed);

    let formats = [
        "%Y-%m-%d",
        "%d/%m/%Y",
        "%d-%m-%Y",
        "%Y/%m/%d",
        "%d.%m.%Y",
        "%Y.%m.%d",
        "%Y%m%d",
    ];

    for fmt in formats {
        if let Ok(d) = NaiveDate::parse_from_str(date_token, fmt) {
            return Some(d);
        }
    }

    None
}

/// Phân tách chuỗi CSV/TSV thành danh sách các dòng dạng Dictionary
pub fn parse_csv_to_rows(content: &str) -> Vec<HashMap<String, String>> {
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let header_line = match lines.next() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let delimiter = if header_line.contains('\t') {
        '\t'
    } else if header_line.contains(';') {
        ';'
    } else {
        ','
    };

    let headers: Vec<String> = header_line
        .split(delimiter)
        .map(|s| s.trim().trim_matches('"').to_string())
        .collect();

    let mut rows = Vec::new();
    for line in lines {
        let cols: Vec<String> = line
            .split(delimiter)
            .map(|s| s.trim().trim_matches('"').to_string())
            .collect();

        let mut row_map = HashMap::new();
        for (i, col_val) in cols.into_iter().enumerate() {
            if let Some(header) = headers.get(i) {
                row_map.insert(header.clone(), col_val);
            }
        }
        rows.push(row_map);
    }

    rows
}

/// Thuật toán lõi đối soát đa khóa và dung sai trong bộ nhớ
pub fn perform_reconciliation(
    rows_a: &[HashMap<String, String>],
    rows_b: &[HashMap<String, String>],
    options: &VisualReconcilerOptions,
    cancel_flag: &AtomicBool,
) -> Result<VisualReconcilerReport, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let regex_a = match &options.key_pattern {
        Some(pat) if !pat.trim().is_empty() => {
            let re = Regex::new(pat).map_err(|e| CommandError::ValidationFailed {
                reason: format!("Biểu thức Regex không hợp lệ: {e}"),
            })?;
            Some(re)
        }
        _ => None,
    };

    // Gom nhóm các dòng của Bảng B theo mã khóa trích xuất
    let mut map_b: HashMap<String, Vec<(usize, HashMap<String, String>)>> = HashMap::new();
    for (idx, row) in rows_b.iter().enumerate() {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }
        let raw_key = row
            .get(&options.key_column_b)
            .map(|s| s.as_str())
            .unwrap_or("");
        let key_b = extract_key_from_text(raw_key, regex_a.as_ref());
        map_b.entry(key_b).or_default().push((idx, row.clone()));
    }

    let mut matched_b_indices: HashSet<usize> = HashSet::new();
    let mut items: Vec<ReconciledItem> = Vec::new();

    // Đối soát từng giao dịch trong Bảng A (Sao kê ngân hàng)
    for row_a in rows_a {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let raw_key_a = row_a
            .get(&options.key_column_a)
            .map(|s| s.as_str())
            .unwrap_or("");
        let key_a = extract_key_from_text(raw_key_a, regex_a.as_ref());

        let amount_a = row_a
            .get(&options.amount_column_a)
            .and_then(|v| parse_amount(v));
        let date_a = options
            .date_column_a
            .as_ref()
            .and_then(|col| row_a.get(col))
            .and_then(|val| parse_date_flexible(val));

        if let Some(candidates) = map_b.get(&key_a) {
            // Tìm bản ghi phù hợp nhất trong Bảng B chưa được đối soát
            let mut chosen: Option<(usize, &HashMap<String, String>, Option<f64>, Option<NaiveDate>, bool, bool)> = None;

            for (b_idx, row_b) in candidates {
                if matched_b_indices.contains(b_idx) {
                    continue;
                }

                let amount_b = row_b
                    .get(&options.amount_column_b)
                    .and_then(|v| parse_amount(v));
                let date_b = options
                    .date_column_b
                    .as_ref()
                    .and_then(|col| row_b.get(col))
                    .and_then(|val| parse_date_flexible(val));

                let amount_match = match (amount_a, amount_b) {
                    (Some(a), Some(b)) => (a - b).abs() <= options.amount_tolerance,
                    (None, None) => true,
                    _ => false,
                };

                let date_match = match (date_a, date_b) {
                    (Some(da), Some(db)) => {
                        let diff_days = (da - db).num_days().abs() as u32;
                        diff_days <= options.date_tolerance_days
                    }
                    _ => true,
                };

                if amount_match && date_match {
                    chosen = Some((*b_idx, row_b, amount_b, date_b, true, true));
                    break;
                } else if chosen.is_none() {
                    chosen = Some((*b_idx, row_b, amount_b, date_b, amount_match, date_match));
                }
            }

            if let Some((b_idx, row_b, amount_b, date_b, amount_match, date_match)) = chosen {
                matched_b_indices.insert(b_idx);

                let amount_diff = match (amount_a, amount_b) {
                    (Some(a), Some(b)) => Some((a - b).abs()),
                    _ => None,
                };
                let date_diff_days = match (date_a, date_b) {
                    (Some(da), Some(db)) => Some((da - db).num_days().abs()),
                    _ => None,
                };

                let status = if amount_match && date_match {
                    ReconciliationStatus::KhopHoanToan
                } else {
                    ReconciliationStatus::KhopMaLechTien
                };

                let note = match status {
                    ReconciliationStatus::KhopHoanToan => {
                        "Khớp mã khóa và số tiền/ngày hợp lệ".to_string()
                    }
                    ReconciliationStatus::KhopMaLechTien => {
                        let mut reasons = Vec::new();
                        if !amount_match {
                            reasons.push(format!(
                                "Lệch tiền: {:?} (vượt dung sai {:.2})",
                                amount_diff, options.amount_tolerance
                            ));
                        }
                        if !date_match {
                            reasons.push(format!(
                                "Lệch ngày: {:?} ngày (vượt dung sai {})",
                                date_diff_days, options.date_tolerance_days
                            ));
                        }
                        reasons.join("; ")
                    }
                    _ => String::new(),
                };

                items.push(ReconciledItem {
                    match_key: key_a,
                    status,
                    amount_a,
                    amount_b,
                    amount_diff,
                    date_a: date_a.map(|d| d.format("%Y-%m-%d").to_string()),
                    date_b: date_b.map(|d| d.format("%Y-%m-%d").to_string()),
                    date_diff_days,
                    row_a: Some(row_a.clone()),
                    row_b: Some(row_b.clone()),
                    note,
                });
            } else {
                // Toàn bộ các dòng có mã khóa này trong Bảng B đã được ghép với dòng khác
                items.push(ReconciledItem {
                    match_key: key_a,
                    status: ReconciliationStatus::ChiCoOSaoKe,
                    amount_a,
                    amount_b: None,
                    amount_diff: None,
                    date_a: date_a.map(|d| d.format("%Y-%m-%d").to_string()),
                    date_b: None,
                    date_diff_days: None,
                    row_a: Some(row_a.clone()),
                    row_b: None,
                    note: "Không tìm thấy bản ghi còn trống tương ứng trong sổ nội bộ".to_string(),
                });
            }
        } else {
            // Không tìm thấy mã khóa trong Bảng B
            items.push(ReconciledItem {
                match_key: key_a,
                status: ReconciliationStatus::ChiCoOSaoKe,
                amount_a,
                amount_b: None,
                amount_diff: None,
                date_a: date_a.map(|d| d.format("%Y-%m-%d").to_string()),
                date_b: None,
                date_diff_days: None,
                row_a: Some(row_a.clone()),
                row_b: None,
                note: "Giao dịch sao kê chưa có thông tin trong sổ nội bộ".to_string(),
            });
        }
    }

    // Các giao dịch còn lại trong Bảng B (Sổ nội bộ) chưa được đối soát
    for (idx, row_b) in rows_b.iter().enumerate() {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        if !matched_b_indices.contains(&idx) {
            let raw_key = row_b
                .get(&options.key_column_b)
                .map(|s| s.as_str())
                .unwrap_or("");
            let key_b = extract_key_from_text(raw_key, regex_a.as_ref());
            let amount_b = row_b
                .get(&options.amount_column_b)
                .and_then(|v| parse_amount(v));
            let date_b = options
                .date_column_b
                .as_ref()
                .and_then(|col| row_b.get(col))
                .and_then(|val| parse_date_flexible(val));

            items.push(ReconciledItem {
                match_key: key_b,
                status: ReconciliationStatus::ChiCoOSoNoiBo,
                amount_a: None,
                amount_b,
                amount_diff: None,
                date_a: None,
                date_b: date_b.map(|d| d.format("%Y-%m-%d").to_string()),
                date_diff_days: None,
                row_a: None,
                row_b: Some(row_b.clone()),
                note: "Đơn hàng/Sổ nội bộ chưa ghi nhận tiền về trên sao kê".to_string(),
            });
        }
    }

    let total_matched_exact = items
        .iter()
        .filter(|i| i.status == ReconciliationStatus::KhopHoanToan)
        .count();
    let total_matched_diff = items
        .iter()
        .filter(|i| i.status == ReconciliationStatus::KhopMaLechTien)
        .count();
    let total_only_in_statement = items
        .iter()
        .filter(|i| i.status == ReconciliationStatus::ChiCoOSaoKe)
        .count();
    let total_only_in_ledger = items
        .iter()
        .filter(|i| i.status == ReconciliationStatus::ChiCoOSoNoiBo)
        .count();

    let total_amount_a: f64 = rows_a
        .iter()
        .filter_map(|r| r.get(&options.amount_column_a).and_then(|v| parse_amount(v)))
        .sum();
    let total_amount_b: f64 = rows_b
        .iter()
        .filter_map(|r| r.get(&options.amount_column_b).and_then(|v| parse_amount(v)))
        .sum();

    Ok(VisualReconcilerReport {
        total_records_a: rows_a.len(),
        total_records_b: rows_b.len(),
        total_matched_exact,
        total_matched_diff,
        total_only_in_statement,
        total_only_in_ledger,
        total_amount_a,
        total_amount_b,
        items,
    })
}

/// Chuyển đổi báo cáo đối soát sang định dạng CSV
pub fn format_report_to_csv(report: &VisualReconcilerReport) -> String {
    let mut lines = Vec::new();
    lines.push(
        "NhomDoiSoat,MaKhoa,SoTienSaoKe,SoTienSoNoiBo,ChenhLechTien,NgaySaoKe,NgaySoNoiBo,ChenhLechNgay,GhiChu"
            .to_string(),
    );

    for item in &report.items {
        let amount_a_str = item
            .amount_a
            .map(|v| format!("{:.0}", v))
            .unwrap_or_default();
        let amount_b_str = item
            .amount_b
            .map(|v| format!("{:.0}", v))
            .unwrap_or_default();
        let diff_str = item
            .amount_diff
            .map(|v| format!("{:.0}", v))
            .unwrap_or_default();
        let date_a_str = item.date_a.clone().unwrap_or_default();
        let date_b_str = item.date_b.clone().unwrap_or_default();
        let diff_days_str = item
            .date_diff_days
            .map(|d| d.to_string())
            .unwrap_or_default();

        let row = format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
            item.status.display_name(),
            item.match_key,
            amount_a_str,
            amount_b_str,
            diff_str,
            date_a_str,
            date_b_str,
            diff_days_str,
            item.note.replace('"', "\"\"")
        );
        lines.push(row);
    }

    lines.join("\n")
}

impl OfficeTool for VisualReconcilerTool {
    fn slug(&self) -> &'static str {
        "visual-reconciler"
    }

    fn name(&self) -> &'static str {
        "Động cơ Đối soát Công nợ & Sao kê Đa khóa"
    }

    fn description(&self) -> &'static str {
        "Đối soát đa khóa giữa sao kê ngân hàng và sổ kế toán/bảng đơn hàng nội bộ với khả năng trích xuất mã đơn mờ trong chuỗi nội dung chuyển khoản, hỗ trợ dung sai số tiền và ngày giao dịch."
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

        let options: VisualReconcilerOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn đối soát không hợp lệ: {e}"),
            })?;

        // 1. Phân giải dữ liệu đầu vào Bảng A và Bảng B
        let (rows_a, rows_b) = if request.input_paths.len() >= 2 {
            let p_a = Path::new(&request.input_paths[0]);
            let p_b = Path::new(&request.input_paths[1]);

            if !p_a.exists() {
                return Err(CommandError::FileNotFound {
                    path: request.input_paths[0].clone(),
                });
            }
            if !p_b.exists() {
                return Err(CommandError::FileNotFound {
                    path: request.input_paths[1].clone(),
                });
            }

            let bytes_a = fs::read(p_a).map_err(|e| CommandError::IoError {
                details: format!("Không thể đọc tệp Bảng A: {e}"),
            })?;
            let bytes_b = fs::read(p_b).map_err(|e| CommandError::IoError {
                details: format!("Không thể đọc tệp Bảng B: {e}"),
            })?;

            // Kiểm tra Magic Bytes chống tệp thực thi độc hại
            if bytes_a.starts_with(b"\x7fELF")
                || bytes_a.starts_with(b"MZ")
                || bytes_b.starts_with(b"\x7fELF")
                || bytes_b.starts_with(b"MZ")
            {
                return Err(CommandError::InvalidFileSignature {
                    expected: "Bảng tính CSV/XLSX hợp lệ".to_string(),
                    actual: "Tệp nhị phân thực thi bị chặn".to_string(),
                });
            }

            let raw_rows_a = super::excel_reader::read_tabular_rows(&bytes_a, cancel_flag)?;
            let raw_rows_b = super::excel_reader::read_tabular_rows(&bytes_b, cancel_flag)?;

            let convert_table = |raw: Vec<Vec<String>>| -> Vec<HashMap<String, String>> {
                if raw.is_empty() {
                    return Vec::new();
                }
                let headers = &raw[0];
                let mut out = Vec::new();
                for r in &raw[1..] {
                    let mut map = HashMap::new();
                    for (i, val) in r.iter().enumerate() {
                        if let Some(h) = headers.get(i) {
                            map.insert(h.trim().to_string(), val.trim().to_string());
                        }
                    }
                    out.push(map);
                }
                out
            };

            (convert_table(raw_rows_a), convert_table(raw_rows_b))
        } else {
            let ra = match &options.source_a_rows {
                Some(rows) => rows.clone(),
                None => {
                    let txt = options.table_a.as_deref().unwrap_or("");
                    parse_csv_to_rows(txt)
                }
            };
            let rb = match &options.source_b_rows {
                Some(rows) => rows.clone(),
                None => {
                    let txt = options.table_b.as_deref().unwrap_or("");
                    parse_csv_to_rows(txt)
                }
            };
            (ra, rb)
        };

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 50.0,
                message: format!(
                    "Đang thực hiện đối soát {} giao dịch sao kê với {} bản ghi nội bộ...",
                    rows_a.len(),
                    rows_b.len()
                ),
                processed_files: 1,
                failed_files: 0,
                total_files: 2,
                is_completed: false,
            });
        }

        // 2. Thực thi thuật toán đối soát
        let report = perform_reconciliation(&rows_a, &rows_b, &options, cancel_flag)?;

        // 3. Ghi kết quả ra thư mục chỉ định
        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("reconcile_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        let csv_path = out_dir.join("reconciliation_result.csv");
        let json_path = out_dir.join("reconciliation_report.json");

        let csv_content = format_report_to_csv(&report);
        let json_content = serde_json::to_string_pretty(&report).map_err(|e| {
            CommandError::ValidationFailed {
                reason: format!("Lỗi tuần tự hóa báo cáo JSON: {e}"),
            }
        })?;

        fs::write(&csv_path, csv_content.as_bytes()).map_err(|e| CommandError::IoError {
            details: format!("Không thể ghi tệp kết quả CSV: {e}"),
        })?;
        fs::write(&json_path, json_content.as_bytes()).map_err(|e| CommandError::IoError {
            details: format!("Không thể ghi tệp báo cáo JSON: {e}"),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: format!(
                    "Hoàn tất đối soát: {} khớp hoàn toàn, {} lệch tiền/ngày, {} chỉ có sao kê, {} chỉ có sổ nội bộ.",
                    report.total_matched_exact,
                    report.total_matched_diff,
                    report.total_only_in_statement,
                    report.total_only_in_ledger
                ),
                processed_files: 2,
                failed_files: 0,
                total_files: 2,
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths: vec![
                csv_path.to_string_lossy().to_string(),
                json_path.to_string_lossy().to_string(),
            ],
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
    fn test_visual_reconciler_happy_path() {
        let tool = VisualReconcilerTool;
        assert_eq!(tool.slug(), "visual-reconciler");

        let csv_a = "\
NoiDung,SoTien,NgayGD
DH001,1000000,2026-11-01
DH002,2000000,2026-11-02
DH003,500000,2026-11-03
DH005,300000,2026-11-04";

        let csv_b = "\
MaDon,SoTien,NgayDon
DH001,1000000,2026-11-01
DH002,2500000,2026-11-02
DH003,500000,2026-11-03
DH004,700000,2026-11-05";

        let cancel = AtomicBool::new(false);
        let opts = VisualReconcilerOptions {
            key_column_a: "NoiDung".to_string(),
            key_column_b: "MaDon".to_string(),
            amount_column_a: "SoTien".to_string(),
            amount_column_b: "SoTien".to_string(),
            date_column_a: Some("NgayGD".to_string()),
            date_column_b: Some("NgayDon".to_string()),
            key_pattern: None,
            amount_tolerance: 0.0,
            date_tolerance_days: 0,
            source_a_rows: None,
            source_b_rows: None,
            table_a: Some(csv_a.to_string()),
            table_b: Some(csv_b.to_string()),
        };

        let rows_a = parse_csv_to_rows(csv_a);
        let rows_b = parse_csv_to_rows(csv_b);

        let report = perform_reconciliation(&rows_a, &rows_b, &opts, &cancel)
            .expect("Đối soát thành công");

        assert_eq!(report.total_matched_exact, 2); // DH001, DH003 khớp hoàn toàn
        assert_eq!(report.total_matched_diff, 1); // DH002 lệch tiền (2.000.000 vs 2.500.000)
        assert_eq!(report.total_only_in_statement, 1); // DH005 chỉ có trên sao kê
        assert_eq!(report.total_only_in_ledger, 1); // DH004 chỉ có trong sổ nội bộ
    }

    #[test]
    fn test_visual_reconciler_substring_pattern_key() {
        let cancel = AtomicBool::new(false);

        let csv_a = "\
MoTaGiaoDich,SoTien
MBVCB.1234567.DH89234 Nguyen Van A chuyen tien,1500000
NAP TIEN TU TK 9876 DH99881 PHI DICH VU,300000";

        let csv_b = "\
MaDonHang,SoTien
DH89234,1500000
DH99881,300000";

        let opts = VisualReconcilerOptions {
            key_column_a: "MoTaGiaoDich".to_string(),
            key_column_b: "MaDonHang".to_string(),
            amount_column_a: "SoTien".to_string(),
            amount_column_b: "SoTien".to_string(),
            date_column_a: None,
            date_column_b: None,
            key_pattern: Some(r"DH\d+".to_string()),
            amount_tolerance: 0.0,
            date_tolerance_days: 0,
            source_a_rows: None,
            source_b_rows: None,
            table_a: None,
            table_b: None,
        };

        let rows_a = parse_csv_to_rows(csv_a);
        let rows_b = parse_csv_to_rows(csv_b);

        let report = perform_reconciliation(&rows_a, &rows_b, &opts, &cancel)
            .expect("Trích xuất mã regex thành công");

        assert_eq!(report.total_matched_exact, 2);
        assert_eq!(report.total_matched_diff, 0);
        assert_eq!(report.total_only_in_statement, 0);
        assert_eq!(report.total_only_in_ledger, 0);
        assert_eq!(report.items[0].match_key, "DH89234");
        assert_eq!(report.items[1].match_key, "DH99881");
    }

    #[test]
    fn test_visual_reconciler_amount_and_date_tolerance() {
        let cancel = AtomicBool::new(false);

        let csv_a = "\
MaDon,SoTien,NgayGD
DH100,1000500,2026-11-01
DH101,2000000,2026-11-01";

        let csv_b = "\
MaDon,SoTien,NgayDon
DH100,1000000,2026-11-03
DH101,2000000,2026-11-06";

        let opts = VisualReconcilerOptions {
            key_column_a: "MaDon".to_string(),
            key_column_b: "MaDon".to_string(),
            amount_column_a: "SoTien".to_string(),
            amount_column_b: "SoTien".to_string(),
            date_column_a: Some("NgayGD".to_string()),
            date_column_b: Some("NgayDon".to_string()),
            key_pattern: None,
            amount_tolerance: 1000.0, // Cho phép lệch tới 1.000đ
            date_tolerance_days: 2,    // Cho phép lệch 2 ngày
            source_a_rows: None,
            source_b_rows: None,
            table_a: None,
            table_b: None,
        };

        let rows_a = parse_csv_to_rows(csv_a);
        let rows_b = parse_csv_to_rows(csv_b);

        let report = perform_reconciliation(&rows_a, &rows_b, &opts, &cancel)
            .expect("Khớp có dung sai thành công");

        // DH100 lệch 500đ (<= 1000) và lệch 2 ngày (<= 2 ngày) -> Khớp hoàn toàn
        assert_eq!(report.items[0].match_key, "DH100");
        assert_eq!(report.items[0].status, ReconciliationStatus::KhopHoanToan);

        // DH101 tiền bằng nhau nhưng lệch 5 ngày (> 2 ngày) -> Khớp mã nhưng lệch ngày
        assert_eq!(report.items[1].match_key, "DH101");
        assert_eq!(report.items[1].status, ReconciliationStatus::KhopMaLechTien);
    }

    #[test]
    fn test_visual_reconciler_cooperative_cancellation() {
        let tool = VisualReconcilerTool;
        let req = DispatchTaskRequest {
            task_id: "task-cancel-reconciler".to_string(),
            tool_slug: "visual-reconciler".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "key_column_a": "A",
                "key_column_b": "B",
                "amount_column_a": "Tien",
                "amount_column_b": "Tien"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_visual_reconciler_invalid_magic_bytes() {
        let tool = VisualReconcilerTool;
        let tmp = std::env::temp_dir().join("test_reconciler_magic");
        let _ = fs::create_dir_all(&tmp);

        let bad_bin = tmp.join("bad_exec.bin");
        fs::write(&bad_bin, b"\x7fELF\x02\x01\x01\x00").expect("ghi tệp nhị phân giả mạo");
        let normal_csv = tmp.join("valid.csv");
        fs::write(&normal_csv, b"MaDon,SoTien\nDH1,100").expect("ghi tệp csv");

        let req = DispatchTaskRequest {
            task_id: "task-invalid-magic".to_string(),
            tool_slug: "visual-reconciler".to_string(),
            input_paths: vec![
                bad_bin.to_string_lossy().to_string(),
                normal_csv.to_string_lossy().to_string(),
            ],
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
    fn test_parse_amount_various_formats() {
        assert_eq!(parse_amount("1,500,000"), Some(1500000.0));
        assert_eq!(parse_amount("1.500.000"), Some(1500000.0));
        assert_eq!(parse_amount("1.500.000,50"), Some(1500000.5));
        assert_eq!(parse_amount("1,500,000.50"), Some(1500000.5));
        assert_eq!(parse_amount(" 250.000 đ "), Some(250000.0));
        assert_eq!(parse_amount("-50.000 VND"), Some(-50000.0));
        assert_eq!(parse_amount("(100,000)"), Some(-100000.0));
    }
}
