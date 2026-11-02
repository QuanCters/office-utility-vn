use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Các kiểu kết hợp bảng (Join Types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinType {
    Inner,
    #[serde(alias = "left_outer")]
    Left,
    FullOuter,
    #[serde(alias = "anti_join")]
    Anti,
}

fn default_join_type() -> JoinType {
    JoinType::Inner
}

/// Cấu hình tùy chọn kết hợp 2 bảng tính
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelJoinOptions {
    #[serde(default = "default_join_type")]
    pub join_type: JoinType,
    #[serde(default, alias = "left_key")]
    pub key_a: String,
    #[serde(default, alias = "right_key")]
    pub key_b: String,
}

pub struct ExcelVisualJoinerTool;

impl Default for ExcelVisualJoinerTool {
    fn default() -> Self {
        Self
    }
}

/// Phân tách nội dung bảng tính dạng dòng CSV/TSV
pub fn parse_csv_table(content: &str) -> (Vec<String>, Vec<Vec<String>>, char) {
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let header_line = match lines.next() {
        Some(h) => h,
        None => return (Vec::new(), Vec::new(), ','),
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

    (headers, rows, delimiter)
}

/// Thuật toán Hash Join trong bộ nhớ
pub fn perform_hash_join(
    content_a: &str,
    content_b: &str,
    options: &ExcelJoinOptions,
    cancel_flag: &AtomicBool,
) -> Result<String, CommandError> {
    let (headers_a, rows_a, delim_a) = parse_csv_table(content_a);
    let (headers_b, rows_b, _) = parse_csv_table(content_b);

    if headers_a.is_empty() || headers_b.is_empty() {
        return Err(CommandError::ValidationFailed {
            reason: "Một trong hai bảng tính không có dòng tiêu đề".to_string(),
        });
    }

    let key_a_idx = headers_a
        .iter()
        .position(|h| h.eq_ignore_ascii_case(&options.key_a))
        .ok_or_else(|| CommandError::ValidationFailed {
            reason: format!(
                "Không tìm thấy cột khóa '{}' trong Bảng A: [{}]",
                options.key_a,
                headers_a.join(", ")
            ),
        })?;

    let key_b_idx = headers_b
        .iter()
        .position(|h| h.eq_ignore_ascii_case(&options.key_b))
        .ok_or_else(|| CommandError::ValidationFailed {
            reason: format!(
                "Không tìm thấy cột khóa '{}' trong Bảng B: [{}]",
                options.key_b,
                headers_b.join(", ")
            ),
        })?;

    // Xây dựng bảng băm cho Bảng B
    let mut map_b: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for row in rows_b {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }
        let k = row.get(key_b_idx).cloned().unwrap_or_default();
        map_b.entry(k).or_default().push(row);
    }

    let mut output_lines = Vec::new();

    if options.join_type == JoinType::Anti {
        // Anti Join: Chỉ lấy các cột của Bảng A cho các dòng không tìm thấy trong Bảng B
        output_lines.push(headers_a.join(&delim_a.to_string()));

        for row_a in rows_a {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let key_val = row_a.get(key_a_idx).cloned().unwrap_or_default();
            if !map_b.contains_key(&key_val) {
                output_lines.push(row_a.join(&delim_a.to_string()));
            }
        }
    } else {
        // Ghép tiêu đề Bảng A + Bảng B
        let mut combined_headers = headers_a.clone();
        for (i, h) in headers_b.iter().enumerate() {
            if i != key_b_idx {
                combined_headers.push(format!("B_{}", h));
            }
        }
        output_lines.push(combined_headers.join(&delim_a.to_string()));

        let mut matched_b_keys = std::collections::HashSet::new();

        for row_a in &rows_a {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let key_val = row_a.get(key_a_idx).cloned().unwrap_or_default();

            if let Some(matching_rows_b) = map_b.get(&key_val) {
                matched_b_keys.insert(key_val.clone());
                for row_b in matching_rows_b {
                    let mut combined_row = row_a.clone();
                    for (i, col_val) in row_b.iter().enumerate() {
                        if i != key_b_idx {
                            combined_row.push(col_val.clone());
                        }
                    }
                    output_lines.push(combined_row.join(&delim_a.to_string()));
                }
            } else if options.join_type == JoinType::Left || options.join_type == JoinType::FullOuter {
                let mut combined_row = row_a.clone();
                for (i, _) in headers_b.iter().enumerate() {
                    if i != key_b_idx {
                        combined_row.push(String::new());
                    }
                }
                output_lines.push(combined_row.join(&delim_a.to_string()));
            }
        }

        // Với FullOuter Join, bổ sung các dòng của Bảng B chưa được khớp
        if options.join_type == JoinType::FullOuter {
            for (key_val, rows_b_group) in &map_b {
                if !matched_b_keys.contains(key_val) {
                    for row_b in rows_b_group {
                        let mut combined_row = Vec::new();
                        for (i, _) in headers_a.iter().enumerate() {
                            if i == key_a_idx {
                                combined_row.push(key_val.clone());
                            } else {
                                combined_row.push(String::new());
                            }
                        }
                        for (i, col_val) in row_b.iter().enumerate() {
                            if i != key_b_idx {
                                combined_row.push(col_val.clone());
                            }
                        }
                        output_lines.push(combined_row.join(&delim_a.to_string()));
                    }
                }
            }
        }
    }

    Ok(output_lines.join("\n"))
}

impl OfficeTool for ExcelVisualJoinerTool {
    fn slug(&self) -> &'static str {
        "excel-visual-joiner"
    }

    fn name(&self) -> &'static str {
        "Nối Trực quan Bảng tính (Join)"
    }

    fn description(&self) -> &'static str {
        "Thực hiện thuật toán Hash Join trong bộ nhớ (INNER, LEFT, FULL OUTER, ANTI JOIN) thông qua việc chọn cột khóa chung mà không cần hàm VLOOKUP."
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

        let options: ExcelJoinOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn không hợp lệ: {e}"),
            })?;

        let (content_a, content_b) = if request.input_paths.len() >= 2 {
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
                details: e.to_string(),
            })?;
            let bytes_b = fs::read(p_b).map_err(|e| CommandError::IoError {
                details: e.to_string(),
            })?;

            if bytes_a.starts_with(b"\x7fELF") || bytes_a.starts_with(b"MZ")
                || bytes_b.starts_with(b"\x7fELF") || bytes_b.starts_with(b"MZ")
            {
                return Err(CommandError::InvalidFileSignature {
                    expected: "Bảng tính CSV/XLSX hợp lệ".to_string(),
                    actual: "Tệp nhị phân thực thi bị chặn".to_string(),
                });
            }

            (String::from_utf8_lossy(&bytes_a).to_string(), String::from_utf8_lossy(&bytes_b).to_string())
        } else {
            let t_a = request.options.get("table_a").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let t_b = request.options.get("table_b").and_then(|v| v.as_str()).unwrap_or("").to_string();
            (t_a, t_b)
        };

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 50.0,
                message: format!("Đang thực hiện Hash Join ({:?}) giữa 2 bảng...", options.join_type),
                processed_files: 1,
                failed_files: 0,
                total_files: 2,
                is_completed: false,
            });
        }

        let joined = perform_hash_join(&content_a, &content_b, &options, cancel_flag)?;

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("excel_join_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        let out_file = out_dir.join("joined_result.csv");
        fs::write(&out_file, joined.as_bytes()).map_err(|e| CommandError::IoError {
            details: e.to_string(),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Hoàn tất kết hợp 2 bảng tính".to_string(),
                processed_files: 2,
                failed_files: 0,
                total_files: 2,
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
    fn test_excel_visual_joiner_happy_path() {
        let tool = ExcelVisualJoinerTool;
        assert_eq!(tool.slug(), "excel-visual-joiner");

        // Bảng A: Danh sách nhân viên (Mã NV, Họ tên)
        let table_a = "Mã NV,Họ tên\nNV01,Nguyễn Văn A\nNV02,Trần Thị B";
        // Bảng B: Bảng lương (Mã Nhân Viên, Lương)
        let table_b = "Mã Nhân Viên,Lương\nNV01,15000000\nNV02,18000000";

        let cancel = AtomicBool::new(false);
        let opts = ExcelJoinOptions {
            join_type: JoinType::Inner,
            key_a: "Mã NV".to_string(),
            key_b: "Mã Nhân Viên".to_string(),
        };

        let joined = perform_hash_join(table_a, table_b, &opts, &cancel).expect("join ok");
        let lines: Vec<&str> = joined.lines().collect();

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Mã NV,Họ tên,B_Lương");
        assert_eq!(lines[1], "NV01,Nguyễn Văn A,15000000");
        assert_eq!(lines[2], "NV02,Trần Thị B,18000000");
    }

    #[test]
    fn test_excel_visual_joiner_edge_case() {
        // ANTI JOIN: Tìm các nhân viên có trong Bảng Lương (A) nhưng thiếu trong Danh sách Nhân sự (B)
        let salary_table = "Mã NV,Lương\nNV01,15000000\nNV99,25000000\nNV02,18000000";
        let hr_table = "Mã NV,Họ tên\nNV01,Nguyễn Văn A\nNV02,Trần Thị B";

        let cancel = AtomicBool::new(false);
        let opts = ExcelJoinOptions {
            join_type: JoinType::Anti,
            key_a: "Mã NV".to_string(),
            key_b: "Mã NV".to_string(),
        };

        let anti_res = perform_hash_join(salary_table, hr_table, &opts, &cancel).expect("anti join ok");
        let lines: Vec<&str> = anti_res.lines().collect();

        // Chỉ có NV99 là thiếu trong nhân sự
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Mã NV,Lương");
        assert_eq!(lines[1], "NV99,25000000");
    }

    #[test]
    fn test_excel_visual_joiner_invalid_magic_bytes() {
        let tool = ExcelVisualJoinerTool;
        let tmp = std::env::temp_dir().join("test_join_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("bad_exec.bin");
        fs::write(&bad_file, b"\x7fELF\x02\x01\x01\x00").expect("write elf");
        let normal_file = tmp.join("normal.csv");
        fs::write(&normal_file, b"id,name\n1,A").expect("write normal");

        let req = DispatchTaskRequest {
            task_id: "task-fake-join".to_string(),
            tool_slug: "excel-visual-joiner".to_string(),
            input_paths: vec![
                bad_file.to_string_lossy().to_string(),
                normal_file.to_string_lossy().to_string(),
            ],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "join_type": "inner",
                "key_a": "id",
                "key_b": "id"
            }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_excel_visual_joiner_cooperative_cancellation() {
        let tool = ExcelVisualJoinerTool;
        let req = DispatchTaskRequest {
            task_id: "task-cancel-join".to_string(),
            tool_slug: "excel-visual-joiner".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "table_a": "ID,Name\n1,A",
                "table_b": "ID,Score\n1,10",
                "join_type": "inner",
                "key_a": "ID",
                "key_b": "ID"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_excel_visual_joiner_security_guardrails() {
        // Cột khóa không tồn tại trả về lỗi ValidationFailed
        let cancel = AtomicBool::new(false);
        let table_a = "ID,Name\n1,A";
        let table_b = "ID,Score\n1,10";

        let opts = ExcelJoinOptions {
            join_type: JoinType::Inner,
            key_a: "UnknownKey".to_string(),
            key_b: "ID".to_string(),
        };

        let res = perform_hash_join(table_a, table_b, &opts, &cancel);
        assert!(res.is_err());
        match res.unwrap_err() {
            CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("Không tìm thấy cột khóa 'UnknownKey'"));
            }
            other => panic!("Kỳ vọng ValidationFailed nhưng nhận được: {:?}", other),
        }
    }
}
