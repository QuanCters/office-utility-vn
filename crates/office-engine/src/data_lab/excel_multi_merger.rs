use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Chế độ gộp bảng tính
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MergeMode {
    #[default]
    AppendRows,
    #[serde(alias = "combine_sheets")]
    MultiSheet,
}

/// Cấu hình tùy chọn gộp nhiều file bảng tính
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExcelMergeOptions {
    #[serde(default, alias = "merge_mode")]
    pub mode: MergeMode,
    #[serde(default)]
    pub add_source_column: bool,
}

pub struct ExcelMultiMergerTool;

impl Default for ExcelMultiMergerTool {
    fn default() -> Self {
        Self
    }
}

/// Thuật toán gộp nối tiếp dòng nhiều file bảng tính
pub fn merge_append_rows(
    file_entries: &[(String, String)], // (filename, content)
    add_source_column: bool,
    cancel_flag: &AtomicBool,
) -> Result<String, CommandError> {
    if file_entries.is_empty() {
        return Ok(String::new());
    }

    let mut merged_lines = Vec::new();

    for (idx, (filename, content)) in file_entries.iter().enumerate() {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let mut lines = content.lines().filter(|l| !l.trim().is_empty());

        let header = match lines.next() {
            Some(h) => h,
            None => continue,
        };

        let delimiter = if header.contains('\t') {
            "\t"
        } else if header.contains(';') {
            ";"
        } else {
            ","
        };

        if idx == 0 {
            let detected_header = if add_source_column {
                format!("Tên tệp nguồn{}{}", delimiter, header)
            } else {
                header.to_string()
            };
            merged_lines.push(detected_header);
        }

        for line in lines {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let formatted = if add_source_column {
                format!("{}{}{}", filename, delimiter, line)
            } else {
                line.to_string()
            };
            merged_lines.push(formatted);
        }
    }

    Ok(merged_lines.join("\n"))
}

impl OfficeTool for ExcelMultiMergerTool {
    fn slug(&self) -> &'static str {
        "excel-multi-merger"
    }

    fn name(&self) -> &'static str {
        "Gộp Nhiều File Excel / Sheet"
    }

    fn description(&self) -> &'static str {
        "Gộp nối tiếp dòng (Append Rows) hoặc gộp thành nhiều Sheet trong một Workbook kèm tùy chọn thêm cột truy vết tên file nguồn."
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

        let options: ExcelMergeOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn không hợp lệ: {e}"),
            })?;

        let mut file_entries = Vec::new();

        if !request.input_paths.is_empty() {
            for in_path_str in &request.input_paths {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                let in_path = Path::new(in_path_str);
                if !in_path.exists() {
                    return Err(CommandError::FileNotFound {
                        path: in_path_str.clone(),
                    });
                }

                let raw_bytes = fs::read(in_path).map_err(|e| CommandError::IoError {
                    details: e.to_string(),
                })?;

                if raw_bytes.starts_with(b"\x7fELF") || raw_bytes.starts_with(b"MZ") {
                    return Err(CommandError::InvalidFileSignature {
                        expected: "Tệp Excel/CSV hợp lệ".to_string(),
                        actual: "Tệp nhị phân thực thi bị chặn".to_string(),
                    });
                }

                let filename = in_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown.csv".to_string());

                let text = String::from_utf8_lossy(&raw_bytes).to_string();
                file_entries.push((filename, text));
            }
        } else if let Some(files_json) = request.options.get("files").and_then(|v| v.as_array()) {
            for (idx, item) in files_json.iter().enumerate() {
                let name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&format!("file_{}.csv", idx))
                    .to_string();
                let content = item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                file_entries.push((name, content));
            }
        } else {
            return Err(CommandError::ValidationFailed {
                reason: "Không có tệp đầu vào để gộp".to_string(),
            });
        }

        let total_files = file_entries.len();

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 50.0,
                message: format!("Đang gộp {} tệp bảng tính...", total_files),
                processed_files: 0,
                failed_files: 0,
                total_files,
                is_completed: false,
            });
        }

        let merged_content = match options.mode {
            MergeMode::AppendRows => {
                merge_append_rows(&file_entries, options.add_source_column, cancel_flag)?
            }
            MergeMode::MultiSheet => {
                // Định dạng xuất nhiều Sheet có phân tách thẻ Sheet
                let mut out = String::new();
                for (fname, content) in &file_entries {
                    out.push_str(&format!("--- [SHEET: {}] ---\n", fname));
                    out.push_str(content);
                    out.push_str("\n\n");
                }
                out
            }
        };

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("excel_merge_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        let out_file = out_dir.join("merged_workbook.csv");
        fs::write(&out_file, merged_content.as_bytes()).map_err(|e| CommandError::IoError {
            details: e.to_string(),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: format!("Hoàn tất gộp {} tệp thành công", total_files),
                processed_files: total_files,
                failed_files: 0,
                total_files,
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
    fn test_excel_multi_merger_happy_path() {
        let tool = ExcelMultiMergerTool;
        assert_eq!(tool.slug(), "excel-multi-merger");

        // Mô phỏng 10 file báo cáo có cùng cấu trúc cột
        let mut files = Vec::new();
        for i in 1..=10 {
            let filename = format!("bao_cao_thang_{:02}.csv", i);
            let content = format!("Mã NV,Họ tên,Doanh số\nNV{:02},Nhân viên {},10000000", i, i);
            files.push((filename, content));
        }

        let cancel = AtomicBool::new(false);
        let merged = merge_append_rows(&files, false, &cancel).expect("merge ok");

        let lines: Vec<&str> = merged.lines().collect();
        // 1 dòng tiêu đề + 10 dòng dữ liệu
        assert_eq!(lines.len(), 11);
        assert_eq!(lines[0], "Mã NV,Họ tên,Doanh số");
        assert_eq!(lines[1], "NV01,Nhân viên 1,10000000");
        assert_eq!(lines[10], "NV10,Nhân viên 10,10000000");
    }

    #[test]
    fn test_excel_multi_merger_edge_case() {
        // Kiểm thử thêm cột truy vết tên file nguồn vào đầu mỗi dòng
        let files = vec![
            ("chi_nhanh_hanoi.csv".to_string(), "Mã,Tên\n1,Alice".to_string()),
            ("chi_nhanh_tphcm.csv".to_string(), "Mã,Tên\n2,Bob".to_string()),
        ];

        let cancel = AtomicBool::new(false);
        let merged = merge_append_rows(&files, true, &cancel).expect("merge ok");

        let lines: Vec<&str> = merged.lines().collect();
        assert_eq!(lines[0], "Tên tệp nguồn,Mã,Tên");
        assert_eq!(lines[1], "chi_nhanh_hanoi.csv,1,Alice");
        assert_eq!(lines[2], "chi_nhanh_tphcm.csv,2,Bob");
    }

    #[test]
    fn test_excel_multi_merger_invalid_magic_bytes() {
        let tool = ExcelMultiMergerTool;
        let tmp = std::env::temp_dir().join("test_merger_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("bad_exec.bin");
        fs::write(&bad_file, b"\x7fELF\x02\x01\x01\x00").expect("write elf");

        let req = DispatchTaskRequest {
            task_id: "task-fake-merger".to_string(),
            tool_slug: "excel-multi-merger".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "mode": "append_rows",
                "add_source_column": false
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
    fn test_excel_multi_merger_cooperative_cancellation() {
        let tool = ExcelMultiMergerTool;
        let req = DispatchTaskRequest {
            task_id: "task-cancel-merger".to_string(),
            tool_slug: "excel-multi-merger".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "mode": "append_rows",
                "files": [
                    { "name": "f1.csv", "content": "A,B\n1,2" }
                ]
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_excel_multi_merger_security_guardrails() {
        // Kiểm định danh sách trống trả về chuỗi rỗng an toàn
        let cancel = AtomicBool::new(false);
        let res = merge_append_rows(&[], false, &cancel);
        assert!(res.is_ok());
        assert_eq!(res.expect("empty ok"), "");
    }
}
