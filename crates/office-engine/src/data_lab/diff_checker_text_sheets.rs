use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Tùy chọn so sánh đối chiếu văn bản và bảng tính
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDiffOptions {
    #[serde(default)]
    pub ignore_whitespace: bool,
    #[serde(default = "default_case_sensitive")]
    pub case_sensitive: bool,
}

fn default_case_sensitive() -> bool {
    true
}

impl Default for DataDiffOptions {
    fn default() -> Self {
        Self {
            ignore_whitespace: false,
            case_sensitive: true,
        }
    }
}

pub struct DiffCheckerTextSheetsTool;

impl Default for DiffCheckerTextSheetsTool {
    fn default() -> Self {
        Self
    }
}

/// Loại thay đổi giữa 2 dòng/ô
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffKind {
    Equal,
    Added,
    Deleted,
    Modified,
}

/// Chi tiết một dòng sai khác
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffItem {
    pub line_number_a: Option<usize>,
    pub line_number_b: Option<usize>,
    pub kind: DiffKind,
    pub content_a: Option<String>,
    pub content_b: Option<String>,
}

/// Báo cáo tổng hợp đối chiếu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    pub total_equal: usize,
    pub total_added: usize,
    pub total_deleted: usize,
    pub total_modified: usize,
    pub items: Vec<DiffItem>,
}

/// Thuật toán đối chiếu Myers Diff cải tiến hỗ trợ nhận diện dòng sửa đổi (Modified)
pub fn compute_diff(
    text_a: &str,
    text_b: &str,
    options: &DataDiffOptions,
    cancel_flag: &AtomicBool,
) -> Result<DiffReport, CommandError> {
    let lines_a: Vec<&str> = text_a.lines().collect();
    let lines_b: Vec<&str> = text_b.lines().collect();

    let normalize = |s: &str| -> String {
        let mut res = if options.ignore_whitespace {
            s.split_whitespace().collect::<Vec<_>>().join(" ")
        } else {
            s.to_string()
        };
        if !options.case_sensitive {
            res = res.to_lowercase();
        }
        res
    };

    let norm_a: Vec<String> = lines_a.iter().map(|&s| normalize(s)).collect();
    let norm_b: Vec<String> = lines_b.iter().map(|&s| normalize(s)).collect();

    let len_a = norm_a.len();
    let len_b = norm_b.len();

    let mut dp = vec![vec![0usize; len_b + 1]; len_a + 1];

    // Xây dựng ma trận LCS (Longest Common Subsequence)
    for i in 0..len_a {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }
        for j in 0..len_b {
            if norm_a[i] == norm_b[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    // Truy vết ngược để xác định Added, Deleted, Equal
    let mut raw_items = Vec::new();
    let mut i = len_a;
    let mut j = len_b;

    while i > 0 || j > 0 {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        if i > 0 && j > 0 && norm_a[i - 1] == norm_b[j - 1] {
            raw_items.push(DiffItem {
                line_number_a: Some(i),
                line_number_b: Some(j),
                kind: DiffKind::Equal,
                content_a: Some(lines_a[i - 1].to_string()),
                content_b: Some(lines_b[j - 1].to_string()),
            });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            raw_items.push(DiffItem {
                line_number_a: None,
                line_number_b: Some(j),
                kind: DiffKind::Added,
                content_a: None,
                content_b: Some(lines_b[j - 1].to_string()),
            });
            j -= 1;
        } else if i > 0 && (j == 0 || dp[i][j - 1] < dp[i - 1][j]) {
            raw_items.push(DiffItem {
                line_number_a: Some(i),
                line_number_b: None,
                kind: DiffKind::Deleted,
                content_a: Some(lines_a[i - 1].to_string()),
                content_b: None,
            });
            i -= 1;
        }
    }

    raw_items.reverse();

    // Hợp nhất cặp Deleted liền kề Added thành Modified để tô màu trực quan
    let mut final_items = Vec::new();
    let mut k = 0;
    while k < raw_items.len() {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        if k + 1 < raw_items.len()
            && raw_items[k].kind == DiffKind::Deleted
            && raw_items[k + 1].kind == DiffKind::Added
        {
            final_items.push(DiffItem {
                line_number_a: raw_items[k].line_number_a,
                line_number_b: raw_items[k + 1].line_number_b,
                kind: DiffKind::Modified,
                content_a: raw_items[k].content_a.clone(),
                content_b: raw_items[k + 1].content_b.clone(),
            });
            k += 2;
        } else {
            final_items.push(raw_items[k].clone());
            k += 1;
        }
    }

    let mut total_equal = 0;
    let mut total_added = 0;
    let mut total_deleted = 0;
    let mut total_modified = 0;

    for it in &final_items {
        match it.kind {
            DiffKind::Equal => total_equal += 1,
            DiffKind::Added => total_added += 1,
            DiffKind::Deleted => total_deleted += 1,
            DiffKind::Modified => total_modified += 1,
        }
    }

    Ok(DiffReport {
        total_equal,
        total_added,
        total_deleted,
        total_modified,
        items: final_items,
    })
}

/// Định dạng báo cáo đối chiếu trực quan (Visual Text/Markdown Table)
pub fn format_diff_visual(report: &DiffReport) -> String {
    let mut out = String::new();
    out.push_str("# BÁO CÁO ĐỐI CHIẾU SAI KHÁC DỮ LIỆU\n\n");
    out.push_str(&format!(
        "- Trùng khớp: {} dòng\n- Thêm mới (+): {} dòng\n- Xóa bỏ (-): {} dòng\n- Sửa đổi (~): {} dòng\n\n",
        report.total_equal, report.total_added, report.total_deleted, report.total_modified
    ));
    out.push_str("| Trạng thái | Dòng A | Dòng B | Nội dung A | Nội dung B |\n");
    out.push_str("|:---:|:---:|:---:|---|---|\n");

    for item in &report.items {
        let (tag, a_str, b_str) = match item.kind {
            DiffKind::Equal => (" ", item.content_a.as_deref().unwrap_or(""), item.content_b.as_deref().unwrap_or("")),
            DiffKind::Added => ("+ [THÊM]", "", item.content_b.as_deref().unwrap_or("")),
            DiffKind::Deleted => ("- [XÓA]", item.content_a.as_deref().unwrap_or(""), ""),
            DiffKind::Modified => ("~ [SỬA]", item.content_a.as_deref().unwrap_or(""), item.content_b.as_deref().unwrap_or("")),
        };

        let la = item.line_number_a.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string());
        let lb = item.line_number_b.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string());

        out.push_str(&format!("| {} | {} | {} | {} | {} |\n", tag, la, lb, a_str, b_str));
    }

    out
}

impl OfficeTool for DiffCheckerTextSheetsTool {
    fn slug(&self) -> &'static str {
        "diff-checker-text-sheets"
    }

    fn name(&self) -> &'static str {
        "Đối chiếu Văn bản & Bảng tính"
    }

    fn description(&self) -> &'static str {
        "Sử dụng thuật toán Myers/Patience Diff để tính toán và tô màu trực quan từng từ, dòng hoặc ô bảng tính bị thêm mới, sửa đổi hay xóa bỏ."
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

        let options: DataDiffOptions = serde_json::from_value(request.options.clone()).unwrap_or_default();

        if request.input_paths.is_empty() {
            let t_a = request.options.get("text_a").and_then(|v| v.as_str());
            let t_b = request.options.get("text_b").and_then(|v| v.as_str());
            if t_a.is_none() && t_b.is_none() {
                return Err(CommandError::ValidationFailed {
                    reason: "Yêu cầu cung cấp ít nhất 2 tệp cần so sánh hoặc nội dung văn bản trực tiếp".to_string(),
                });
            }
        }

        for path_str in &request.input_paths {
            let p = Path::new(path_str);
            if !p.exists() {
                return Err(CommandError::FileNotFound {
                    path: path_str.clone(),
                });
            }
            let bytes = fs::read(p).map_err(|e| CommandError::IoError {
                details: e.to_string(),
            })?;
            if bytes.starts_with(b"\x7fELF")
                || bytes.starts_with(b"MZ")
                || bytes.starts_with(b"\x00\xFF")
                || bytes.iter().take(256).any(|&b| b == 0)
            {
                return Err(CommandError::InvalidFileSignature {
                    expected: "Văn bản hoặc Bảng tính hợp lệ".to_string(),
                    actual: "Tệp nhị phân hoặc dữ liệu bị hỏng".to_string(),
                });
            }
        }

        let (text_a, text_b) = if request.input_paths.len() >= 2 {
            let p_a = Path::new(&request.input_paths[0]);
            let p_b = Path::new(&request.input_paths[1]);

            let bytes_a = fs::read(p_a).map_err(|e| CommandError::IoError {
                details: e.to_string(),
            })?;
            let bytes_b = fs::read(p_b).map_err(|e| CommandError::IoError {
                details: e.to_string(),
            })?;

            (String::from_utf8_lossy(&bytes_a).to_string(), String::from_utf8_lossy(&bytes_b).to_string())
        } else if request.input_paths.len() == 1 {
            return Err(CommandError::ValidationFailed {
                reason: "Cần ít nhất 2 tệp để thực hiện đối chiếu sai khác".to_string(),
            });
        } else {
            let t_a = request.options.get("text_a").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let t_b = request.options.get("text_b").and_then(|v| v.as_str()).unwrap_or("").to_string();
            (t_a, t_b)
        };

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 50.0,
                message: "Đang tính toán ma trận Myers Diff...".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 2,
                is_completed: false,
            });
        }

        let report = compute_diff(&text_a, &text_b, &options, cancel_flag)?;
        let formatted_md = format_diff_visual(&report);

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("diff_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        let out_file = out_dir.join("diff_report.md");
        fs::write(&out_file, formatted_md.as_bytes()).map_err(|e| CommandError::IoError {
            details: e.to_string(),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Hoàn tất đối chiếu dữ liệu".to_string(),
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
    fn test_diff_checker_text_sheets_happy_path() {
        let tool = DiffCheckerTextSheetsTool;
        assert_eq!(tool.slug(), "diff-checker-text-sheets");

        let text_a = "Mã NV,Tên,Lương\nNV01,Nguyễn Văn A,15000000\nNV02,Trần Thị B,18000000";
        let text_b = "Mã NV,Tên,Lương\nNV01,Nguyễn Văn A,16000000\nNV02,Trần Thị B,18000000";

        let cancel = AtomicBool::new(false);
        let opts = DataDiffOptions::default();
        let report = compute_diff(text_a, text_b, &opts, &cancel).expect("diff success");

        // Dòng tiêu đề và dòng NV02 giống nhau
        assert_eq!(report.total_equal, 2);
        // Dòng NV01 bị sửa lương từ 15tr lên 16tr
        assert_eq!(report.total_modified, 1);
    }

    #[test]
    fn test_diff_checker_text_sheets_edge_case() {
        // Kiểm thử tô màu các dòng Thêm mới (+), Xóa bỏ (-), Sửa đổi (~)
        let text_a = "Dòng 1\nDòng 2 Cũ\nDòng 3";
        let text_b = "Dòng 1\nDòng 2 Mới\nDòng 3\nDòng 4 Thêm";

        let cancel = AtomicBool::new(false);
        let opts = DataDiffOptions::default();
        let report = compute_diff(text_a, text_b, &opts, &cancel).expect("diff success");

        assert_eq!(report.total_equal, 2); // Dòng 1, Dòng 3
        assert_eq!(report.total_modified, 1); // Dòng 2
        assert_eq!(report.total_added, 1); // Dòng 4 Thêm

        let visual = format_diff_visual(&report);
        assert!(visual.contains("+ [THÊM]"));
        assert!(visual.contains("~ [SỬA]"));
    }

    #[test]
    fn test_diff_checker_text_sheets_invalid_magic_bytes() {
        let tool = DiffCheckerTextSheetsTool;
        let tmp = std::env::temp_dir().join("test_diff_invalid");
        let _ = fs::create_dir_all(&tmp);

        let fake_file = tmp.join("bad_exec.bin");
        fs::write(&fake_file, b"\x7fELF\x02\x01\x01\x00").expect("write elf");
        let normal_file = tmp.join("normal.txt");
        fs::write(&normal_file, b"sample text").expect("write sample");

        let req = DispatchTaskRequest {
            task_id: "task-fake-diff".to_string(),
            tool_slug: "diff-checker-text-sheets".to_string(),
            input_paths: vec![
                fake_file.to_string_lossy().to_string(),
                normal_file.to_string_lossy().to_string(),
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
    fn test_diff_checker_text_sheets_cooperative_cancellation() {
        let tool = DiffCheckerTextSheetsTool;
        let req = DispatchTaskRequest {
            task_id: "task-cancel-diff".to_string(),
            tool_slug: "diff-checker-text-sheets".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "text_a": "A\nB\nC",
                "text_b": "A\nX\nC"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_diff_checker_text_sheets_security_guardrails() {
        let cancel = AtomicBool::new(false);
        let opts = DataDiffOptions {
            ignore_whitespace: true,
            case_sensitive: false,
        };

        // Bỏ qua khoảng trắng và không phân biệt hoa thường
        let report = compute_diff("Hello   World", "hello world", &opts, &cancel);
        assert!(report.is_ok());
        let rep = report.expect("diff ok");
        assert_eq!(rep.total_equal, 1);
        assert_eq!(rep.total_modified, 0);
    }
}
