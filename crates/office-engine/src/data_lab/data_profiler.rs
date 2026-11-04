use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Kiểu dữ liệu nhận diện của cột
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnType {
    Text,
    Number,
    Date,
    Boolean,
    Empty,
}

/// Thống kê tần suất xuất hiện của giá trị trong cột
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueFrequency {
    pub value: String,
    pub count: usize,
    pub percentage: f32,
}

/// Hồ sơ thống kê chi tiết của một cột dữ liệu
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnProfile {
    pub name: String,
    pub col_type: ColumnType,
    pub total_rows: usize,
    pub non_empty_count: usize,
    pub empty_count: usize,
    pub null_percentage: f32,
    pub distinct_count: usize,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub top_frequencies: Vec<ValueFrequency>,
}

/// Báo cáo tổng thể hồ sơ dữ liệu dạng bảng
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataProfileReport {
    pub total_rows: usize,
    pub total_columns: usize,
    pub columns: Vec<ColumnProfile>,
}

fn default_top_k() -> usize {
    10
}

/// Tùy chọn cấu hình bộ phân tích hồ sơ dữ liệu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataProfilerOptions {
    #[serde(default)]
    pub sample_size: Option<usize>,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

impl Default for DataProfilerOptions {
    fn default() -> Self {
        Self {
            sample_size: None,
            top_k: 10,
        }
    }
}

pub struct DataProfilerTool;

impl Default for DataProfilerTool {
    fn default() -> Self {
        Self
    }
}

/// Kiểm tra một chuỗi có phải là giá trị Boolean hợp lệ hay không
pub fn is_bool_str(s: &str) -> bool {
    let lower = s.trim().to_lowercase();
    matches!(lower.as_str(), "true" | "false" | "yes" | "no" | "đúng" | "sai")
}

/// Kiểm tra một chuỗi có phải là số (Number) hợp lệ hay không
pub fn is_number_str(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Tránh NaN và Inf
    if trimmed.eq_ignore_ascii_case("nan") || trimmed.to_lowercase().contains("inf") {
        return false;
    }
    if let Ok(val) = trimmed.parse::<f64>() {
        return val.is_finite();
    }
    // Hỗ trợ dấu phẩy phân cách hàng nghìn
    if trimmed.contains(',') {
        let without_commas = trimmed.replace(',', "");
        if let Ok(val) = without_commas.parse::<f64>() {
            return val.is_finite();
        }
    }
    false
}

/// Phân tích linh hoạt chuỗi ngày tháng theo các định dạng phổ biến
pub fn parse_date_flexible(s: &str) -> Option<chrono::NaiveDateTime> {
    let trimmed = s.trim();
    if trimmed.len() < 8 {
        return None;
    }
    let date_formats = [
        "%Y-%m-%d",
        "%d/%m/%Y",
        "%m/%d/%Y",
        "%Y/%m/%d",
        "%d-%m-%Y",
        "%Y.%m.%d",
        "%d.%m.%Y",
    ];
    for fmt in date_formats {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(trimmed, fmt) {
            return d.and_hms_opt(0, 0, 0);
        }
    }

    let datetime_formats = [
        "%Y-%m-%d %H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%d/%m/%Y %H:%M",
    ];
    for fmt in datetime_formats {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, fmt) {
            return Some(dt);
        }
    }

    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.naive_utc());
    }

    None
}

/// Kiểm tra một chuỗi có phải là Ngày tháng (Date/DateTime) hợp lệ hay không
pub fn is_date_str(s: &str) -> bool {
    parse_date_flexible(s).is_some()
}

/// Suy diễn kiểu dữ liệu của cột từ danh sách giá trị
pub fn infer_column_type(values: &[String]) -> ColumnType {
    let non_empty: Vec<&str> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();

    if non_empty.is_empty() {
        return ColumnType::Empty;
    }

    if non_empty.iter().all(|&v| is_bool_str(v)) {
        return ColumnType::Boolean;
    }

    if non_empty.iter().all(|&v| is_number_str(v)) {
        return ColumnType::Number;
    }

    if non_empty.iter().all(|&v| is_date_str(v)) {
        return ColumnType::Date;
    }

    ColumnType::Text
}

/// Phân tích thống kê toàn diện dữ liệu bảng theo hàng và cột
pub fn profile_tabular_rows(
    headers: &[String],
    rows: &[Vec<String>],
    options: &DataProfilerOptions,
    cancel_flag: &AtomicBool,
) -> Result<DataProfileReport, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let effective_rows = match options.sample_size {
        Some(sz) if sz > 0 && sz < rows.len() => &rows[..sz],
        _ => rows,
    };

    let total_rows = effective_rows.len();
    let max_row_cols = effective_rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let total_columns = headers.len().max(max_row_cols);

    if total_columns == 0 {
        return Ok(DataProfileReport {
            total_rows,
            total_columns: 0,
            columns: Vec::new(),
        });
    }

    let mut columns = Vec::with_capacity(total_columns);

    for col_idx in 0..total_columns {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let col_name = headers
            .get(col_idx)
            .cloned()
            .unwrap_or_else(|| format!("Cột {}", col_idx + 1));

        let mut empty_count = 0;
        let mut non_empty_values = Vec::new();
        let mut freq_map: HashMap<String, usize> = HashMap::new();

        for row in effective_rows {
            let val = row.get(col_idx).map(|s| s.trim()).unwrap_or("");
            if val.is_empty() {
                empty_count += 1;
            } else {
                non_empty_values.push(val.to_string());
                *freq_map.entry(val.to_string()).or_insert(0) += 1;
            }
        }

        let non_empty_count = total_rows.saturating_sub(empty_count);
        let null_percentage = if total_rows > 0 {
            let p = (empty_count as f32 / total_rows as f32) * 100.0;
            (p * 100.0).round() / 100.0
        } else {
            0.0
        };

        let col_type = infer_column_type(&non_empty_values);
        let distinct_count = freq_map.len();

        let (min_value, max_value) = match col_type {
            ColumnType::Number => {
                let mut min_num: Option<(f64, String)> = None;
                let mut max_num: Option<(f64, String)> = None;
                for v in &non_empty_values {
                    let num_val = if v.contains(',') {
                        v.replace(',', "").parse::<f64>()
                    } else {
                        v.parse::<f64>()
                    };
                    if let Ok(n) = num_val {
                        if min_num.as_ref().map_or(true, |(m, _)| n < *m) {
                            min_num = Some((n, v.clone()));
                        }
                        if max_num.as_ref().map_or(true, |(m, _)| n > *m) {
                            max_num = Some((n, v.clone()));
                        }
                    }
                }
                (min_num.map(|(_, s)| s), max_num.map(|(_, s)| s))
            }
            ColumnType::Date => {
                let mut min_d: Option<(chrono::NaiveDateTime, String)> = None;
                let mut max_d: Option<(chrono::NaiveDateTime, String)> = None;
                for v in &non_empty_values {
                    if let Some(dt) = parse_date_flexible(v) {
                        if min_d.as_ref().map_or(true, |(m, _)| dt < *m) {
                            min_d = Some((dt, v.clone()));
                        }
                        if max_d.as_ref().map_or(true, |(m, _)| dt > *m) {
                            max_d = Some((dt, v.clone()));
                        }
                    }
                }
                (min_d.map(|(_, s)| s), max_d.map(|(_, s)| s))
            }
            ColumnType::Text | ColumnType::Boolean => {
                let mut min_s: Option<String> = None;
                let mut max_s: Option<String> = None;
                for v in &non_empty_values {
                    if min_s.as_ref().map_or(true, |m| v < m) {
                        min_s = Some(v.clone());
                    }
                    if max_s.as_ref().map_or(true, |m| v > m) {
                        max_s = Some(v.clone());
                    }
                }
                (min_s, max_s)
            }
            ColumnType::Empty => (None, None),
        };

        let mut freq_vec: Vec<ValueFrequency> = freq_map
            .into_iter()
            .map(|(val, count)| {
                let percentage = if total_rows > 0 {
                    (count as f32 / total_rows as f32) * 100.0
                } else {
                    0.0
                };
                ValueFrequency {
                    value: val,
                    count,
                    percentage: (percentage * 100.0).round() / 100.0,
                }
            })
            .collect();

        freq_vec.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.cmp(&b.value)));

        let k = if options.top_k > 0 { options.top_k } else { 10 };
        let top_frequencies = freq_vec.into_iter().take(k).collect();

        columns.push(ColumnProfile {
            name: col_name,
            col_type,
            total_rows,
            non_empty_count,
            empty_count,
            null_percentage,
            distinct_count,
            min_value,
            max_value,
            top_frequencies,
        });
    }

    Ok(DataProfileReport {
        total_rows,
        total_columns,
        columns,
    })
}

impl OfficeTool for DataProfilerTool {
    fn slug(&self) -> &'static str {
        "data-profiler"
    }

    fn name(&self) -> &'static str {
        "Phân tích Hồ sơ Dữ liệu (Data Profiler)"
    }

    fn description(&self) -> &'static str {
        "Thống kê tự động tỷ lệ dữ liệu rỗng/hợp lệ, nhận diện kiểu dữ liệu cột, đếm giá trị phân biệt và vẽ biểu đồ phân phối tần suất tương tự OpenRefine."
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

        let options: DataProfilerOptions = serde_json::from_value(request.options.clone())
            .unwrap_or_default();

        // Xử lý trực tiếp từ chuỗi nếu không có input_paths
        if request.input_paths.is_empty() {
            if let Some(text_val) = request.options.get("text").and_then(|v| v.as_str()) {
                let raw_rows = super::excel_reader::read_tabular_rows(text_val.as_bytes(), cancel_flag)?;
                let (headers, rows) = if raw_rows.is_empty() {
                    (Vec::new(), Vec::new())
                } else {
                    (raw_rows[0].clone(), raw_rows[1..].to_vec())
                };

                let report = profile_tabular_rows(&headers, &rows, &options, cancel_flag)?;
                let report_json = serde_json::to_string_pretty(&report).map_err(|e| {
                    CommandError::ValidationFailed {
                        reason: format!("Lỗi tạo JSON báo cáo: {e}"),
                    }
                })?;

                return Ok(DispatchTaskResponse {
                    task_id: request.task_id.clone(),
                    status: "SUCCESS".to_string(),
                    output_paths: vec![report_json],
                    failed_items: vec![],
                    elapsed_ms: start_time.elapsed().as_millis() as u64,
                });
            } else {
                return Err(CommandError::ValidationFailed {
                    reason: "Không có tệp đầu vào hoặc nội dung bảng tính cần phân tích hồ sơ".to_string(),
                });
            }
        }

        let total_files = request.input_paths.len();
        let mut output_paths = Vec::new();
        let mut failed_items = Vec::new();

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("data_profile_{}", request.task_id)),
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

            // Phòng vệ Magic Bytes tệp thực thi nhị phân ELF và MZ
            if raw_bytes.starts_with(b"\x7fELF") || raw_bytes.starts_with(b"MZ") {
                return Err(CommandError::InvalidFileSignature {
                    expected: "XLSX hoặc CSV".to_string(),
                    actual: "Tệp thực thi nhị phân bị chặn".to_string(),
                });
            }

            let raw_rows = super::excel_reader::read_tabular_rows(&raw_bytes, cancel_flag)?;
            let (headers, rows) = if raw_rows.is_empty() {
                (Vec::new(), Vec::new())
            } else {
                (raw_rows[0].clone(), raw_rows[1..].to_vec())
            };

            let report = profile_tabular_rows(&headers, &rows, &options, cancel_flag)?;
            let report_json = match serde_json::to_string_pretty(&report) {
                Ok(j) => j,
                Err(e) => {
                    failed_items.push(FileFailureDetail {
                        input_path: in_path_str.clone(),
                        reason: format!("Lỗi tạo JSON: {e}"),
                    });
                    continue;
                }
            };

            let file_stem = in_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("report_{}", idx));
            let out_file = out_dir.join(format!("profile_{}.json", file_stem));

            if let Err(e) = fs::write(&out_file, report_json.as_bytes()) {
                failed_items.push(FileFailureDetail {
                    input_path: in_path_str.clone(),
                    reason: e.to_string(),
                });
                continue;
            }

            output_paths.push(out_file.to_string_lossy().to_string());

            if let Some(cb) = on_progress {
                cb(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: (idx + 1) as u32,
                    total_steps: total_files as u32,
                    percentage: ((idx + 1) as f32 / total_files as f32) * 100.0,
                    message: format!("Đã phân tích hồ sơ tệp: {}", file_stem),
                    processed_files: idx + 1,
                    failed_files: failed_items.len(),
                    total_files,
                    is_completed: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_infer_column_type() {
        assert_eq!(
            infer_column_type(&["true".to_string(), "false".to_string(), "đúng".to_string()]),
            ColumnType::Boolean
        );
        assert_eq!(
            infer_column_type(&["123".to_string(), "45.67".to_string(), "-89".to_string()]),
            ColumnType::Number
        );
        assert_eq!(
            infer_column_type(&[
                "2026-11-04".to_string(),
                "15/08/2025".to_string(),
                "2024/12/31".to_string()
            ]),
            ColumnType::Date
        );
        assert_eq!(
            infer_column_type(&["Xin chào".to_string(), "Hà Nội".to_string()]),
            ColumnType::Text
        );
        assert_eq!(
            infer_column_type(&["".to_string(), "   ".to_string()]),
            ColumnType::Empty
        );
    }

    #[test]
    fn test_data_profiler_happy_path() {
        let headers = vec![
            "Họ và tên".to_string(),
            "Tuổi".to_string(),
            "Ngày vào làm".to_string(),
            "Chính thức".to_string(),
            "Ghi chú rỗng".to_string(),
        ];

        let rows = vec![
            vec![
                "Nguyễn Văn A".to_string(),
                "25".to_string(),
                "2022-01-10".to_string(),
                "true".to_string(),
                "".to_string(),
            ],
            vec![
                "Trần Thị B".to_string(),
                "30".to_string(),
                "2020-05-15".to_string(),
                "false".to_string(),
                "".to_string(),
            ],
            vec![
                "Lê Văn C".to_string(),
                "25".to_string(),
                "2023-08-20".to_string(),
                "true".to_string(),
                "".to_string(),
            ],
        ];

        let options = DataProfilerOptions::default();
        let cancel = AtomicBool::new(false);
        let report = profile_tabular_rows(&headers, &rows, &options, &cancel).unwrap();

        assert_eq!(report.total_rows, 3);
        assert_eq!(report.total_columns, 5);

        // Cột 0: Họ và tên (Text)
        let col0 = &report.columns[0];
        assert_eq!(col0.name, "Họ và tên");
        assert_eq!(col0.col_type, ColumnType::Text);
        assert_eq!(col0.distinct_count, 3);
        assert_eq!(col0.empty_count, 0);

        // Cột 1: Tuổi (Number)
        let col1 = &report.columns[1];
        assert_eq!(col1.name, "Tuổi");
        assert_eq!(col1.col_type, ColumnType::Number);
        assert_eq!(col1.distinct_count, 2);
        assert_eq!(col1.min_value, Some("25".to_string()));
        assert_eq!(col1.max_value, Some("30".to_string()));
        assert_eq!(col1.top_frequencies[0].value, "25");
        assert_eq!(col1.top_frequencies[0].count, 2);

        // Cột 2: Ngày vào làm (Date)
        let col2 = &report.columns[2];
        assert_eq!(col2.col_type, ColumnType::Date);
        assert_eq!(col2.min_value, Some("2020-05-15".to_string()));
        assert_eq!(col2.max_value, Some("2023-08-20".to_string()));

        // Cột 3: Chính thức (Boolean)
        let col3 = &report.columns[3];
        assert_eq!(col3.col_type, ColumnType::Boolean);

        // Cột 4: Ghi chú rỗng (Empty)
        let col4 = &report.columns[4];
        assert_eq!(col4.col_type, ColumnType::Empty);
        assert_eq!(col4.empty_count, 3);
        assert_eq!(col4.null_percentage, 100.0);
    }

    #[test]
    fn test_data_profiler_cooperative_cancellation() {
        let tool = DataProfilerTool;
        let req = DispatchTaskRequest {
            task_id: "task-cancel-prof".to_string(),
            tool_slug: "data-profiler".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({ "text": "A,B\n1,2" }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_data_profiler_edge_cases() {
        let cancel = AtomicBool::new(false);
        let options = DataProfilerOptions::default();

        // Trường hợp 1: Bảng rỗng hoàn toàn
        let empty_rep = profile_tabular_rows(&[], &[], &options, &cancel).unwrap();
        assert_eq!(empty_rep.total_rows, 0);
        assert_eq!(empty_rep.total_columns, 0);
        assert!(empty_rep.columns.is_empty());

        // Trường hợp 2: Chỉ có header không có dữ liệu
        let headers_only = vec!["Col1".to_string(), "Col2".to_string()];
        let rep_headers = profile_tabular_rows(&headers_only, &[], &options, &cancel).unwrap();
        assert_eq!(rep_headers.total_rows, 0);
        assert_eq!(rep_headers.total_columns, 2);
        assert_eq!(rep_headers.columns[0].col_type, ColumnType::Empty);

        // Trường hợp 3: Hàng có số cột không đều nhau
        let rows_uneven = vec![
            vec!["1".to_string()],
            vec!["2".to_string(), "text".to_string(), "extra".to_string()],
        ];
        let rep_uneven = profile_tabular_rows(&headers_only, &rows_uneven, &options, &cancel).unwrap();
        assert_eq!(rep_uneven.total_columns, 3);
    }

    #[test]
    fn test_data_profiler_security_guardrails() {
        let tool = DataProfilerTool;
        let tmp = std::env::temp_dir().join("test_profiler_invalid");
        let _ = fs::create_dir_all(&tmp);

        let elf_path = tmp.join("fake_data.csv");
        fs::write(&elf_path, b"\x7fELF\x02\x01\x01\x00").expect("write elf");

        let req = DispatchTaskRequest {
            task_id: "task-fake-prof".to_string(),
            tool_slug: "data-profiler".to_string(),
            input_paths: vec![elf_path.to_string_lossy().to_string()],
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
    fn test_data_profiler_sampling_and_top_k() {
        let cancel = AtomicBool::new(false);
        let headers = vec!["NumberCol".to_string()];
        let mut rows = Vec::new();
        for i in 1..=20 {
            rows.push(vec![format!("{}", i % 3)]);
        }

        let options = DataProfilerOptions {
            sample_size: Some(10),
            top_k: 2,
        };

        let rep = profile_tabular_rows(&headers, &rows, &options, &cancel).unwrap();
        assert_eq!(rep.total_rows, 10);
        assert!(rep.columns[0].top_frequencies.len() <= 2);
    }
}
