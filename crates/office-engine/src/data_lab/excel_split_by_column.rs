use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Cấu hình tùy chọn tách file bảng tính theo cột
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelSplitOptions {
    pub split_column_name: String,
}

pub struct ExcelSplitByColumnTool;

impl Default for ExcelSplitByColumnTool {
    fn default() -> Self {
        Self
    }
}

/// Làm sạch tên tệp để an toàn trên mọi hệ điều hành
pub fn sanitize_filename(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect();
    let trimmed = clean.trim();
    if trimmed.is_empty() {
        "khong_ten".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Thuật toán nhóm và tách bảng tính theo cột chỉ định
pub fn split_table_by_column(
    content: &str,
    column_name: &str,
    cancel_flag: &AtomicBool,
) -> Result<(String, BTreeMap<String, Vec<String>>), CommandError> {
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let header_line = match lines.next() {
        Some(h) => h,
        None => {
            return Err(CommandError::ValidationFailed {
                reason: "Dữ liệu bảng tính trống không có tiêu đề".to_string(),
            })
        }
    };

    let delimiter = if header_line.contains('\t') {
        '\t'
    } else if header_line.contains(';') {
        ';'
    } else {
        ','
    };

    let headers: Vec<&str> = header_line
        .split(delimiter)
        .map(|s| s.trim().trim_matches('"'))
        .collect();

    let target_col_lower = column_name.trim().to_lowercase();
    let col_idx = headers
        .iter()
        .position(|&h| h.to_lowercase() == target_col_lower)
        .ok_or_else(|| CommandError::ValidationFailed {
            reason: format!(
                "Không tìm thấy cột '{}' trong danh sách tiêu đề: [{}]",
                column_name,
                headers.join(", ")
            ),
        })?;

    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for line in lines {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let cols: Vec<&str> = line.split(delimiter).collect();
        let group_key = if col_idx < cols.len() {
            let k = cols[col_idx].trim().trim_matches('"');
            if k.is_empty() {
                "Khac".to_string()
            } else {
                k.to_string()
            }
        } else {
            "Khac".to_string()
        };

        groups.entry(group_key).or_default().push(line.to_string());
    }

    Ok((header_line.to_string(), groups))
}

impl OfficeTool for ExcelSplitByColumnTool {
    fn slug(&self) -> &'static str {
        "excel-split-by-column"
    }

    fn name(&self) -> &'static str {
        "Tách File Excel theo Cột"
    }

    fn description(&self) -> &'static str {
        "Phân nhóm dữ liệu theo giá trị duy nhất của một cột khóa chỉ định và xuất song song ra hàng chục tệp Excel/CSV riêng biệt bảo tồn nguyên vẹn header."
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

        let options: ExcelSplitOptions = serde_json::from_value(request.options.clone())
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
                    expected: "Bảng tính CSV hoặc XLSX hợp lệ".to_string(),
                    actual: "Tệp nhị phân thực thi bị chặn".to_string(),
                });
            }

            String::from_utf8_lossy(&raw_bytes).to_string()
        } else if let Some(text_val) = request.options.get("text").and_then(|v| v.as_str()) {
            text_val.to_string()
        } else {
            return Err(CommandError::ValidationFailed {
                reason: "Không có tệp đầu vào hoặc dữ liệu văn bản".to_string(),
            });
        };

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 33.0,
                message: format!("Đang phân nhóm dữ liệu theo cột '{}'...", options.split_column_name),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        let (header, groups) = split_table_by_column(&content, &options.split_column_name, cancel_flag)?;

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("excel_split_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        let total_groups = groups.len();
        let mut output_paths = Vec::new();

        for (idx, (group_name, rows)) in groups.into_iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let safe_name = sanitize_filename(&group_name);
            let out_file = out_dir.join(format!("{}.csv", safe_name));

            let mut file_content = String::new();
            file_content.push_str(&header);
            file_content.push('\n');
            file_content.push_str(&rows.join("\n"));
            file_content.push('\n');

            fs::write(&out_file, file_content.as_bytes()).map_err(|e| CommandError::IoError {
                details: e.to_string(),
            })?;

            output_paths.push(out_file.to_string_lossy().to_string());

            if let Some(cb) = on_progress {
                cb(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: (idx + 1) as u32,
                    total_steps: total_groups as u32,
                    percentage: ((idx + 1) as f32 / total_groups as f32) * 100.0,
                    message: format!("Đã xuất nhóm: {} ({} dòng)", group_name, rows.len()),
                    processed_files: idx + 1,
                    failed_files: 0,
                    total_files: total_groups,
                    is_completed: false,
                });
            }
        }

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: total_groups as u32,
                total_steps: total_groups as u32,
                percentage: 100.0,
                message: format!("Hoàn tất tách thành {} file", output_paths.len()),
                processed_files: total_groups,
                failed_files: 0,
                total_files: total_groups,
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths,
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
    fn test_excel_split_by_column_happy_path() {
        let tool = ExcelSplitByColumnTool::default();
        assert_eq!(tool.slug(), "excel-split-by-column");

        // Tạo bảng lương 5.000 dòng theo 20 phòng ban (PB01 .. PB20)
        let mut csv = String::from("Mã NV,Họ và tên,Mã Phòng Ban,Lương\n");
        for i in 1..=5_000 {
            let dept_num = (i % 20) + 1;
            csv.push_str(&format!("NV{:04},Nhân viên {},PB{:02},15000000\n", i, i, dept_num));
        }

        let cancel = AtomicBool::new(false);
        let (header, groups) = split_table_by_column(&csv, "Mã Phòng Ban", &cancel).expect("split ok");

        assert_eq!(header, "Mã NV,Họ và tên,Mã Phòng Ban,Lương");
        assert_eq!(groups.len(), 20);

        // Mỗi phòng ban có đúng 250 nhân viên
        for (dept, rows) in &groups {
            assert!(dept.starts_with("PB"));
            assert_eq!(rows.len(), 250);
        }
    }

    #[test]
    fn test_excel_split_by_column_edge_case() {
        let tool = ExcelSplitByColumnTool::default();
        let csv = "ID,Name,Department\n1,Alice,Finance/Accounting\n2,Bob,IT:Dev\n3,Charlie,Finance/Accounting";

        let tmp = std::env::temp_dir().join("test_split_edge");
        let _ = fs::create_dir_all(&tmp);

        let req = DispatchTaskRequest {
            task_id: "task-split-edge".to_string(),
            tool_slug: "excel-split-by-column".to_string(),
            input_paths: vec![],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "text": csv,
                "split_column_name": "Department"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("split success");

        // Có 2 nhóm: Finance/Accounting -> Finance_Accounting và IT:Dev -> IT_Dev
        assert_eq!(res.output_paths.len(), 2);

        // Kiểm tra bảo tồn nguyên vẹn header
        for file in &res.output_paths {
            let text = fs::read_to_string(file).expect("read file");
            let mut lines = text.lines();
            assert_eq!(lines.next(), Some("ID,Name,Department"));
        }
    }

    #[test]
    fn test_excel_split_by_column_invalid_magic_bytes() {
        let tool = ExcelSplitByColumnTool::default();
        let tmp = std::env::temp_dir().join("test_split_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("bad_exec.bin");
        fs::write(&bad_file, b"\x7fELF\x02\x01\x01\x00").expect("write elf");

        let req = DispatchTaskRequest {
            task_id: "task-fake-split".to_string(),
            tool_slug: "excel-split-by-column".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "split_column_name": "Dept" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_excel_split_by_column_cooperative_cancellation() {
        let tool = ExcelSplitByColumnTool::default();
        let req = DispatchTaskRequest {
            task_id: "task-cancel-split".to_string(),
            tool_slug: "excel-split-by-column".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "text": "ID,Dept\n1,IT\n2,HR",
                "split_column_name": "Dept"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_excel_split_by_column_security_guardrails() {
        // Kiểm thử cột không tồn tại trả về ValidationFailed an toàn không hoảng loạn
        let cancel = AtomicBool::new(false);
        let csv = "ID,Name\n1,A";
        let res = split_table_by_column(csv, "NonExistentColumn", &cancel);
        assert!(res.is_err());
        match res.unwrap_err() {
            CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("Không tìm thấy cột 'NonExistentColumn'"));
            }
            other => panic!("Kỳ vọng ValidationFailed nhưng nhận được: {:?}", other),
        }
    }
}
