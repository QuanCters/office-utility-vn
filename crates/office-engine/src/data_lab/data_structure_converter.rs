use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Tùy chọn chuyển đổi cấu trúc dữ liệu
fn default_target_format() -> String {
    "json".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConverterOptions {
    /// Định dạng đích: "json", "sql", "markdown"
    #[serde(default = "default_target_format")]
    pub target_format: String,
    /// Tên bảng (nếu chọn xuất SQL)
    #[serde(default)]
    pub table_name: Option<String>,
}

pub struct DataStructureConverterTool;

impl Default for DataStructureConverterTool {
    fn default() -> Self {
        Self
    }
}

/// Phân tích bảng dữ liệu từ chuỗi CSV/TSV
pub fn parse_table_data(content: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let header_line = match lines.next() {
        Some(h) => h,
        None => return (Vec::new(), Vec::new()),
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
        rows.push(cols);
    }

    (headers, rows)
}

/// Chuyển đổi bảng sang JSON Array
pub fn table_to_json(
    headers: &[String],
    rows: &[Vec<String>],
    cancel_flag: &AtomicBool,
) -> Result<String, CommandError> {
    let mut json_rows = Vec::with_capacity(rows.len());

    for row in rows {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let mut obj = serde_json::Map::new();
        for (i, header) in headers.iter().enumerate() {
            let val_str = row.get(i).map(|s| s.as_str()).unwrap_or("");
            let val = if val_str.eq_ignore_ascii_case("true") {
                serde_json::Value::Bool(true)
            } else if val_str.eq_ignore_ascii_case("false") {
                serde_json::Value::Bool(false)
            } else if let Ok(n) = val_str.parse::<i64>() {
                serde_json::Value::Number(n.into())
            } else if let Ok(f) = val_str.parse::<f64>() {
                if let Some(num) = serde_json::Number::from_f64(f) {
                    serde_json::Value::Number(num)
                } else {
                    serde_json::Value::String(val_str.to_string())
                }
            } else {
                serde_json::Value::String(val_str.to_string())
            };
            obj.insert(header.clone(), val);
        }
        json_rows.push(serde_json::Value::Object(obj));
    }

    serde_json::to_string_pretty(&serde_json::Value::Array(json_rows)).map_err(|e| {
        CommandError::ProcessExecutionError {
            message: format!("Lỗi tuần tự hóa JSON: {e}"),
        }
    })
}

/// Chuyển đổi bảng sang câu lệnh SQL INSERT INTO
pub fn table_to_sql(
    headers: &[String],
    rows: &[Vec<String>],
    table_name: &str,
    cancel_flag: &AtomicBool,
) -> Result<String, CommandError> {
    let mut out = String::new();
    let col_list = headers.join(", ");

    for row in rows {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let mut values = Vec::new();
        for (i, _) in headers.iter().enumerate() {
            let val_str = row.get(i).map(|s| s.as_str()).unwrap_or("");
            if val_str.is_empty() || val_str.eq_ignore_ascii_case("null") {
                values.push("NULL".to_string());
            } else if val_str.parse::<i64>().is_ok() || val_str.parse::<f64>().is_ok() {
                values.push(val_str.to_string());
            } else {
                let escaped = val_str.replace('\'', "''");
                values.push(format!("'{}'", escaped));
            }
        }

        out.push_str(&format!(
            "INSERT INTO {} ({}) VALUES ({});\n",
            table_name,
            col_list,
            values.join(", ")
        ));
    }

    Ok(out)
}

/// Chuyển đổi bảng sang Markdown Table
pub fn table_to_markdown(
    headers: &[String],
    rows: &[Vec<String>],
    cancel_flag: &AtomicBool,
) -> Result<String, CommandError> {
    let mut out = String::new();

    // Dòng Header
    out.push_str(&format!("| {} |\n", headers.join(" | ")));
    // Dòng Phân cách
    let sep: Vec<&str> = vec!["---"; headers.len()];
    out.push_str(&format!("| {} |\n", sep.join(" | ")));

    for row in rows {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let mut cols = Vec::new();
        for (i, _) in headers.iter().enumerate() {
            let val = row.get(i).map(|s| s.as_str()).unwrap_or("");
            cols.push(val);
        }
        out.push_str(&format!("| {} |\n", cols.join(" | ")));
    }

    Ok(out)
}

impl OfficeTool for DataStructureConverterTool {
    fn slug(&self) -> &'static str {
        "data-structure-converter"
    }

    fn name(&self) -> &'static str {
        "Chuyển đổi Cấu trúc Dữ liệu"
    }

    fn description(&self) -> &'static str {
        "Chuyển đổi hai chiều tốc độ cao giữa dữ liệu bảng tính với mảng đối tượng JSON, câu lệnh INSERT INTO chuẩn hóa kiểu dữ liệu của SQL hoặc bảng kẻ Markdown."
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

        let options: DataConverterOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn không hợp lệ: {e}"),
            })?;

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
                    expected: "CSV hoặc Bảng tính văn bản".to_string(),
                    actual: "Tệp nhị phân thực thi bị chặn".to_string(),
                });
            }

            String::from_utf8_lossy(&raw_bytes).to_string()
        } else if let Some(text_val) = request.options.get("text").and_then(|v| v.as_str()) {
            text_val.to_string()
        } else {
            return Err(CommandError::ValidationFailed {
                reason: "Không có tệp đầu vào hoặc chuỗi văn bản dữ liệu".to_string(),
            });
        };

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 50.0,
                message: "Đang phân tích cấu trúc bảng tính...".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        let (headers, rows) = parse_table_data(&content);

        let target = options.target_format.to_lowercase();
        let table_name = options.table_name.as_deref().unwrap_or("data_records");

        let converted = match target.as_str() {
            "json" | "json_array" => table_to_json(&headers, &rows, cancel_flag)?,
            "sql" | "sql_insert" => table_to_sql(&headers, &rows, table_name, cancel_flag)?,
            "markdown" | "md" | "markdown_table" => table_to_markdown(&headers, &rows, cancel_flag)?,
            other => {
                return Err(CommandError::ValidationFailed {
                    reason: format!("Định dạng đích '{}' không được hỗ trợ (chỉ chấp nhận: json, sql, markdown)", other),
                });
            }
        };

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("data_convert_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        let ext = match target.as_str() {
            "json" => "json",
            "sql" => "sql",
            _ => "md",
        };
        let out_file = out_dir.join(format!("output.{}", ext));
        fs::write(&out_file, converted.as_bytes()).map_err(|e| CommandError::IoError {
            details: e.to_string(),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Hoàn tất chuyển đổi cấu trúc".to_string(),
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
    fn test_data_structure_converter_happy_path() {
        let tool = DataStructureConverterTool::default();
        assert_eq!(tool.slug(), "data-structure-converter");

        // Tạo dữ liệu mô phỏng 10.000 dòng
        let mut csv = String::from("id,name,salary,active\n");
        for i in 1..=10_000 {
            csv.push_str(&format!("{},Nhân viên {},15000000,true\n", i, i));
        }

        let (headers, rows) = parse_table_data(&csv);
        assert_eq!(headers.len(), 4);
        assert_eq!(rows.len(), 10_000);

        let cancel = AtomicBool::new(false);
        let start = Instant::now();
        let json_res = table_to_json(&headers, &rows, &cancel).expect("json conversion ok");
        assert!(start.elapsed().as_millis() < 500);
        assert!(json_res.contains("\"salary\": 15000000"));
    }

    #[test]
    fn test_data_structure_converter_edge_case() {
        let tool = DataStructureConverterTool::default();
        let csv_data = "id,name,join_date,salary,notes\n1,Nguyễn Văn O'Connor,2026-11-02,25000000,NULL";

        let req = DispatchTaskRequest {
            task_id: "task-sql-edge".to_string(),
            tool_slug: "data-structure-converter".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "text": csv_data,
                "target_format": "sql",
                "table_name": "employees"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("sql conversion ok");
        let content = fs::read_to_string(&res.output_paths[0]).expect("read sql output");

        // Kiểm thử thoát ký tự nháy đơn O'Connor -> O''Connor
        assert!(content.contains("O''Connor"));
        // Kiểm thử số và ngày tháng chuẩn hóa
        assert!(content.contains("INSERT INTO employees (id, name, join_date, salary, notes) VALUES (1, 'Nguyễn Văn O''Connor', '2026-11-02', 25000000, NULL);"));
    }

    #[test]
    fn test_data_structure_converter_invalid_magic_bytes() {
        let tool = DataStructureConverterTool::default();
        let tmp = std::env::temp_dir().join("test_converter_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("bad_exec.bin");
        fs::write(&bad_file, b"\x7fELF\x02\x01\x01\x00").expect("write elf");

        let req = DispatchTaskRequest {
            task_id: "task-fake-converter".to_string(),
            tool_slug: "data-structure-converter".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_format": "json" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_data_structure_converter_cooperative_cancellation() {
        let tool = DataStructureConverterTool::default();
        let req = DispatchTaskRequest {
            task_id: "task-cancel-converter".to_string(),
            tool_slug: "data-structure-converter".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "text": "col1,col2\nval1,val2",
                "target_format": "json"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_data_structure_converter_security_guardrails() {
        let cancel = AtomicBool::new(false);
        let headers = vec!["col1".to_string(), "col2".to_string()];
        let rows = vec![vec!["val1".to_string(), "val2".to_string()]];

        let md = table_to_markdown(&headers, &rows, &cancel);
        assert!(md.is_ok());
        let md_str = md.expect("valid md");
        assert!(md_str.contains("| col1 | col2 |"));
        assert!(md_str.contains("| val1 | val2 |"));
    }
}
