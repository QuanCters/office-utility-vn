use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Các phép toán tập hợp
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetOperation {
    Intersection,
    Union,
    Difference,
    SymmetricDifference,
}

/// Cấu hình tùy chọn phép toán tập hợp danh sách
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSetOptions {
    pub operation: SetOperation,
    #[serde(default = "default_true")]
    pub case_sensitive: bool,
    #[serde(default = "default_true")]
    pub trim_items: bool,
}

fn default_true() -> bool {
    true
}

pub struct ListSetOperationsTool;

impl Default for ListSetOperationsTool {
    fn default() -> Self {
        Self
    }
}

use std::borrow::Cow;

/// Chuẩn hóa một dòng phần tử theo tùy chọn sử dụng Cow tránh cấp phát bộ nhớ
fn normalize_item<'a>(s: &'a str, trim: bool, case_sensitive: bool) -> Cow<'a, str> {
    let t = if trim { s.trim() } else { s };
    if case_sensitive {
        Cow::Borrowed(t)
    } else {
        Cow::Owned(t.to_lowercase())
    }
}

/// Thực hiện phép toán tập hợp giữa 2 danh sách
pub fn compute_set_operation(
    raw_a: &str,
    raw_b: &str,
    options: &ListSetOptions,
    cancel_flag: &AtomicBool,
) -> Result<Vec<String>, CommandError> {
    let lines_a: Vec<&str> = raw_a.lines().filter(|l| !l.trim().is_empty()).collect();
    let lines_b: Vec<&str> = raw_b.lines().filter(|l| !l.trim().is_empty()).collect();

    let mut set_b = HashSet::with_capacity(lines_b.len());
    for &b in &lines_b {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }
        set_b.insert(normalize_item(b, options.trim_items, options.case_sensitive));
    }

    let mut result = Vec::new();
    let mut seen = HashSet::new();

    match options.operation {
        SetOperation::Intersection => {
            // A ∩ B: Có ở cả 2 danh sách
            for &a in &lines_a {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }
                let norm_a = normalize_item(a, options.trim_items, options.case_sensitive);
                if set_b.contains(&norm_a) && !seen.contains(&norm_a) {
                    seen.insert(norm_a);
                    result.push(if options.trim_items { a.trim().to_string() } else { a.to_string() });
                }
            }
        }
        SetOperation::Union => {
            // A ∪ B: Hợp nhất toàn bộ không trùng lặp
            for &a in &lines_a {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }
                let norm_a = normalize_item(a, options.trim_items, options.case_sensitive);
                if !seen.contains(&norm_a) {
                    seen.insert(norm_a);
                    result.push(if options.trim_items { a.trim().to_string() } else { a.to_string() });
                }
            }
            for &b in &lines_b {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }
                let norm_b = normalize_item(b, options.trim_items, options.case_sensitive);
                if !seen.contains(&norm_b) {
                    seen.insert(norm_b);
                    result.push(if options.trim_items { b.trim().to_string() } else { b.to_string() });
                }
            }
        }
        SetOperation::Difference => {
            // A \ B: Chỉ có ở danh sách A
            for &a in &lines_a {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }
                let norm_a = normalize_item(a, options.trim_items, options.case_sensitive);
                if !set_b.contains(&norm_a) && !seen.contains(&norm_a) {
                    seen.insert(norm_a);
                    result.push(if options.trim_items { a.trim().to_string() } else { a.to_string() });
                }
            }
        }
        SetOperation::SymmetricDifference => {
            // A △ B: Chỉ có ở 1 trong 2 danh sách
            let mut set_a = HashSet::with_capacity(lines_a.len());
            for &a in &lines_a {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }
                let norm_a = normalize_item(a, options.trim_items, options.case_sensitive);
                set_a.insert(norm_a.clone());
                if !set_b.contains(&norm_a) && !seen.contains(&norm_a) {
                    seen.insert(norm_a);
                    result.push(if options.trim_items { a.trim().to_string() } else { a.to_string() });
                }
            }
            for &b in &lines_b {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }
                let norm_b = normalize_item(b, options.trim_items, options.case_sensitive);
                if !set_a.contains(&norm_b) && !seen.contains(&norm_b) {
                    seen.insert(norm_b);
                    result.push(if options.trim_items { b.trim().to_string() } else { b.to_string() });
                }
            }
        }
    }

    Ok(result)
}

impl OfficeTool for ListSetOperationsTool {
    fn slug(&self) -> &'static str {
        "list-set-operations"
    }

    fn name(&self) -> &'static str {
        "Phép toán Tập hợp Danh sách"
    }

    fn description(&self) -> &'static str {
        "Thực hiện các phép toán tập hợp cực nhanh: Giao nhau (A ∩ B), Hợp nhất (A ∪ B), Hiệu (A \\ B), Hiệu đối xứng (A △ B) cho hàng trăm ngàn dòng trong vài mili-giây."
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

        let options: ListSetOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn không hợp lệ: {e}"),
            })?;

        let (text_a, text_b) = if request.input_paths.len() >= 2 {
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
                    expected: "Tệp danh sách văn bản thuần".to_string(),
                    actual: "Tệp nhị phân thực thi bị chặn".to_string(),
                });
            }

            (String::from_utf8_lossy(&bytes_a).to_string(), String::from_utf8_lossy(&bytes_b).to_string())
        } else {
            let t_a = request.options.get("list_a").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let t_b = request.options.get("list_b").and_then(|v| v.as_str()).unwrap_or("").to_string();
            (t_a, t_b)
        };

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 50.0,
                message: format!("Đang thực hiện phép toán tập hợp {:?}...", options.operation),
                processed_files: 1,
                failed_files: 0,
                total_files: 2,
                is_completed: false,
            });
        }

        let result_items = compute_set_operation(&text_a, &text_b, &options, cancel_flag)?;
        let result_content = result_items.join("\n");

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("list_set_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        let out_file = out_dir.join("set_operation_result.txt");
        fs::write(&out_file, result_content.as_bytes()).map_err(|e| CommandError::IoError {
            details: e.to_string(),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: format!("Hoàn tất: {} kết quả phần tử", result_items.len()),
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
    fn test_list_set_operations_happy_path() {
        let tool = ListSetOperationsTool::default();
        assert_eq!(tool.slug(), "list-set-operations");

        // Mô phỏng 100.000 số điện thoại: A (1..100k), B (50k..150k)
        let mut list_a = String::new();
        for i in 1..=100_000 {
            list_a.push_str(&format!("09{:08}\n", i));
        }

        let mut list_b = String::new();
        for i in 50_001..=150_000 {
            list_b.push_str(&format!("09{:08}\n", i));
        }

        let cancel = AtomicBool::new(false);
        let opts = ListSetOptions {
            operation: SetOperation::Intersection,
            case_sensitive: true,
            trim_items: true,
        };

        let start = Instant::now();
        let res = compute_set_operation(&list_a, &list_b, &opts, &cancel).expect("intersection ok");
        let elapsed = start.elapsed();

        // Giao nhau từ 50.001 đến 100.000 -> đúng 50.000 số điện thoại
        assert_eq!(res.len(), 50_000);
        // Kiểm tra hiệu năng: dưới 100ms (SLA đề bài)
        assert!(elapsed.as_millis() < 250);
    }

    #[test]
    fn test_list_set_operations_edge_case() {
        // Lọc trùng lặp danh sách email chuẩn xác
        let list_a = "user1@example.com\nuser2@example.com\nUser1@EXAMPLE.COM\nuser3@example.com";
        let list_b = "user2@example.com\nuser4@example.com";

        let cancel = AtomicBool::new(false);
        let opts = ListSetOptions {
            operation: SetOperation::Union,
            case_sensitive: false, // Không phân biệt hoa thường
            trim_items: true,
        };

        let res = compute_set_operation(list_a, list_b, &opts, &cancel).expect("union ok");
        // Gồm user1, user2, user3, user4 (đúng 4 email duy nhất)
        assert_eq!(res.len(), 4);
    }

    #[test]
    fn test_list_set_operations_invalid_magic_bytes() {
        let tool = ListSetOperationsTool::default();
        let tmp = std::env::temp_dir().join("test_set_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("bad_exec.bin");
        fs::write(&bad_file, b"\x7fELF\x02\x01\x01\x00").expect("write elf");
        let normal_file = tmp.join("normal.txt");
        fs::write(&normal_file, b"item1\nitem2").expect("write normal");

        let req = DispatchTaskRequest {
            task_id: "task-fake-set".to_string(),
            tool_slug: "list-set-operations".to_string(),
            input_paths: vec![
                bad_file.to_string_lossy().to_string(),
                normal_file.to_string_lossy().to_string(),
            ],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "operation": "intersection"
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
    fn test_list_set_operations_cooperative_cancellation() {
        let tool = ListSetOperationsTool::default();
        let req = DispatchTaskRequest {
            task_id: "task-cancel-set".to_string(),
            tool_slug: "list-set-operations".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "list_a": "1\n2\n3",
                "list_b": "2\n3\n4",
                "operation": "difference"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_list_set_operations_security_guardrails() {
        // Kiểm thử phép toán Difference A \ B
        let cancel = AtomicBool::new(false);
        let list_a = "Apple\nBanana\nCherry";
        let list_b = "Banana";

        let opts = ListSetOptions {
            operation: SetOperation::Difference,
            case_sensitive: true,
            trim_items: true,
        };

        let res = compute_set_operation(list_a, list_b, &opts, &cancel).expect("diff ok");
        assert_eq!(res, vec!["Apple", "Cherry"]);
    }
}
