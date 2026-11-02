use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Cấu hình tùy chọn làm sạch và chuẩn hóa bảng tính
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelCleanOptions {
    #[serde(default)]
    pub normalize_dates: bool,
    #[serde(default)]
    pub split_vietnamese_names: bool,
    #[serde(default)]
    pub trim_spaces: bool,
    #[serde(default)]
    pub deduplicate: bool,
}

pub struct ExcelCleanerNormalizerTool;

impl Default for ExcelCleanerNormalizerTool {
    fn default() -> Self {
        Self
    }
}

/// Danh sách họ kép tiếng Việt thông dụng
const COMPOUND_SURNAMES: &[&str] = &[
    "nguyễn phước",
    "nguyễn phúc",
    "nguyễn đình",
    "nguyễn hữu",
    "nguyễn huy",
    "nguyễn bá",
    "nguyễn văn",
    "phan huy",
    "hoàng phủ",
    "hoàng kim",
    "võ đình",
    "trần bá",
    "đặng vũ",
    "lê văn",
    "phạm đình",
    "vũ đình",
    "bùi hữu",
];

/// Kết quả tách họ và tên tiếng Việt
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VietnameseNameParts {
    pub ho: String,
    pub dem: String,
    pub ten: String,
}

/// Thuật toán tách họ, đệm, tên tiếng Việt có nhận diện họ kép
pub fn split_vietnamese_name(full_name: &str) -> VietnameseNameParts {
    let clean = clean_spaces_and_capitalize(full_name);
    let words: Vec<&str> = clean.split_whitespace().collect();

    match words.len() {
        0 => VietnameseNameParts {
            ho: String::new(),
            dem: String::new(),
            ten: String::new(),
        },
        1 => VietnameseNameParts {
            ho: words[0].to_string(),
            dem: String::new(),
            ten: String::new(),
        },
        2 => VietnameseNameParts {
            ho: words[0].to_string(),
            dem: String::new(),
            ten: words[1].to_string(),
        },
        3 => VietnameseNameParts {
            ho: words[0].to_string(),
            dem: words[1].to_string(),
            ten: words[2].to_string(),
        },
        _ => {
            let potential_compound = format!("{} {}", words[0], words[1]).to_lowercase();
            let is_compound = COMPOUND_SURNAMES.contains(&potential_compound.as_str());

            if is_compound && words.len() >= 4 {
                let ho = format!("{} {}", words[0], words[1]);
                let dem = words[2..words.len() - 1].join(" ");
                let ten = words[words.len() - 1].to_string();
                VietnameseNameParts { ho, dem, ten }
            } else {
                let ho = words[0].to_string();
                let dem = words[1..words.len() - 1].join(" ");
                let ten = words[words.len() - 1].to_string();
                VietnameseNameParts { ho, dem, ten }
            }
        }
    }
}

/// Xóa khoảng trắng thừa và viết hoa chữ cái đầu mỗi từ
pub fn clean_spaces_and_capitalize(text: &str) -> String {
    let mut result_words = Vec::new();
    for word in text.split_whitespace() {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            let mut formatted = first.to_uppercase().to_string();
            formatted.push_str(&chars.as_str().to_lowercase());
            result_words.push(formatted);
        }
    }
    result_words.join(" ")
}

/// Chuẩn hóa ngày tháng về định dạng DD/MM/YYYY
pub fn normalize_date_string(date_str: &str) -> String {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Tách theo -, /, .
    let parts: Vec<&str> = trimmed
        .split(|c| c == '-' || c == '/' || c == '.')
        .map(|s| s.trim())
        .collect();

    if parts.len() != 3 {
        return trimmed.to_string();
    }

    // Trường hợp YYYY-MM-DD hoặc YYYY/MM/DD
    if parts[0].len() == 4 {
        if let (Ok(y), Ok(m), Ok(d)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            if (1..=12).contains(&m) && (1..=31).contains(&d) {
                return format!("{:02}/{:02}/{:04}", d, m, y);
            }
        }
    }

    // Trường hợp DD-MM-YYYY hoặc MM-DD-YYYY
    if parts[2].len() == 4 {
        if let (Ok(p1), Ok(p2), Ok(y)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            // Nếu p1 > 12 và p2 <= 12 => chắc chắn là DD/MM/YYYY
            if p1 > 12 && p1 <= 31 && p2 <= 12 {
                return format!("{:02}/{:02}/{:04}", p1, p2, y);
            }
            // Nếu p2 > 12 và p1 <= 12 => MM/DD/YYYY -> đổi sang DD/MM/YYYY
            if p2 > 12 && p2 <= 31 && p1 <= 12 {
                return format!("{:02}/{:02}/{:04}", p2, p1, y);
            }
            // Mặc định xem p1 là ngày, p2 là tháng
            if (1..=31).contains(&p1) && (1..=12).contains(&p2) {
                return format!("{:02}/{:02}/{:04}", p1, p2, y);
            }
        }
    }

    trimmed.to_string()
}

/// Làm sạch một dòng CSV/bảng tính
pub fn clean_csv_content(
    content: &str,
    options: &ExcelCleanOptions,
    cancel_flag: &AtomicBool,
) -> Result<String, CommandError> {
    let mut seen_rows = HashSet::new();
    let mut output_lines = Vec::new();

    for line in content.lines() {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let raw_line = line.trim();
        if raw_line.is_empty() {
            continue;
        }

        // Tách các cột theo dấu phẩy hoặc tab hoặc chấm phẩy
        let delimiter = if raw_line.contains('\t') {
            '\t'
        } else if raw_line.contains(';') {
            ';'
        } else {
            ','
        };

        let mut columns: Vec<String> = raw_line
            .split(delimiter)
            .map(|col| {
                let mut c = col.trim().to_string();
                if options.trim_spaces {
                    c = clean_spaces_and_capitalize(&c);
                }
                if options.normalize_dates {
                    c = normalize_date_string(&c);
                }
                c
            })
            .collect();

        // Nếu kích hoạt tách tên tiếng Việt và có cột tên
        if options.split_vietnamese_names && !columns.is_empty() {
            let first_col = columns[0].clone();
            let parts = split_vietnamese_name(&first_col);
            if !parts.ho.is_empty() && !parts.ten.is_empty() {
                // Chèn thêm Họ, Đệm, Tên vào sau cột gốc
                columns.push(parts.ho);
                columns.push(parts.dem);
                columns.push(parts.ten);
            }
        }

        let formatted_line = columns.join(&delimiter.to_string());

        if options.deduplicate {
            if seen_rows.contains(&formatted_line) {
                continue;
            }
            seen_rows.insert(formatted_line.clone());
        }

        output_lines.push(formatted_line);
    }

    Ok(output_lines.join("\n"))
}

impl OfficeTool for ExcelCleanerNormalizerTool {
    fn slug(&self) -> &'static str {
        "excel-cleaner-normalizer"
    }

    fn name(&self) -> &'static str {
        "Làm sạch & Chuẩn hóa Bảng tính"
    }

    fn description(&self) -> &'static str {
        "Chuẩn hóa định dạng ngày tháng hỗn hợp về DD/MM/YYYY, viết hoa chữ cái đầu tên riêng, tách cột Họ / Tên đệm / Tên theo quy tắc họ kép tiếng Việt, xóa khoảng trắng thừa và loại bỏ dòng trùng lặp."
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

        let options: ExcelCleanOptions = serde_json::from_value(request.options.clone())
            .unwrap_or(ExcelCleanOptions {
                normalize_dates: true,
                split_vietnamese_names: true,
                trim_spaces: true,
                deduplicate: true,
            });

        // Xử lý trực tiếp từ chuỗi nếu không có input_paths
        if request.input_paths.is_empty() {
            if let Some(text_val) = request.options.get("text").and_then(|v| v.as_str()) {
                let cleaned = clean_csv_content(text_val, &options, cancel_flag)?;
                return Ok(DispatchTaskResponse {
                    task_id: request.task_id.clone(),
                    status: "SUCCESS".to_string(),
                    output_paths: vec![cleaned],
                    failed_items: vec![],
                    elapsed_ms: start_time.elapsed().as_millis() as u64,
                });
            } else {
                return Err(CommandError::ValidationFailed {
                    reason: "Không có tệp đầu vào hoặc nội dung bảng tính cần xử lý".to_string(),
                });
            }
        }

        let total_files = request.input_paths.len();
        let mut output_paths = Vec::new();
        let mut failed_items = Vec::new();

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("excel_clean_{}", request.task_id)),
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

            // Đọc bytes kiểm tra chữ ký ma thuật
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

            // Kiểm tra magic bytes phòng vệ nhị phân: cấm ELF, MZ thực thi
            if raw_bytes.starts_with(b"\x7fELF") || raw_bytes.starts_with(b"MZ") {
                return Err(CommandError::InvalidFileSignature {
                    expected: "XLSX hoặc CSV".to_string(),
                    actual: "Tệp thực thi nhị phân bị chặn".to_string(),
                });
            }

            // Xử lý nội dung văn bản CSV hoặc Excel
            let content = String::from_utf8_lossy(&raw_bytes);
            let cleaned = clean_csv_content(&content, &options, cancel_flag)?;

            let filename = in_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("cleaned_{}.csv", idx));
            let out_file = out_dir.join(format!("cleaned_{}", filename));

            if let Err(e) = fs::write(&out_file, cleaned.as_bytes()) {
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
                    message: format!("Đã chuẩn hóa tệp: {}", filename),
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
    fn test_excel_cleaner_normalizer_happy_path() {
        let tool = ExcelCleanerNormalizerTool::default();
        assert_eq!(tool.slug(), "excel-cleaner-normalizer");

        // Kiểm thử tách tên chuẩn 'Nguyễn Thị Thu Trang'
        let parts = split_vietnamese_name("Nguyễn Thị Thu Trang");
        assert_eq!(parts.ho, "Nguyễn");
        assert_eq!(parts.dem, "Thị Thu");
        assert_eq!(parts.ten, "Trang");
    }

    #[test]
    fn test_excel_cleaner_normalizer_edge_case() {
        // Kiểm thử họ kép 'Nguyễn Phước Vĩnh Lộc'
        let parts = split_vietnamese_name("Nguyễn Phước Vĩnh Lộc");
        assert_eq!(parts.ho, "Nguyễn Phước");
        assert_eq!(parts.dem, "Vĩnh");
        assert_eq!(parts.ten, "Lộc");

        // Kiểm thử chuẩn hóa ngày tháng
        assert_eq!(normalize_date_string("2026-11-02"), "02/11/2026");
        assert_eq!(normalize_date_string("15/08/2025"), "15/08/2025");
        assert_eq!(normalize_date_string("12.25.2024"), "25/12/2024");
    }

    #[test]
    fn test_excel_cleaner_normalizer_invalid_magic_bytes() {
        let tool = ExcelCleanerNormalizerTool::default();
        let tmp = std::env::temp_dir().join("test_cleaner_invalid");
        let _ = fs::create_dir_all(&tmp);

        let elf_path = tmp.join("fake_excel.xlsx");
        fs::write(&elf_path, b"\x7fELF\x02\x01\x01\x00").expect("write elf");

        let req = DispatchTaskRequest {
            task_id: "task-fake-excel".to_string(),
            tool_slug: "excel-cleaner-normalizer".to_string(),
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
    fn test_excel_cleaner_normalizer_cooperative_cancellation() {
        let tool = ExcelCleanerNormalizerTool::default();
        let req = DispatchTaskRequest {
            task_id: "task-cancel-clean".to_string(),
            tool_slug: "excel-cleaner-normalizer".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({ "text": "Họ và tên\nNguyễn Văn A" }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_excel_cleaner_normalizer_security_guardrails() {
        let cancel = AtomicBool::new(false);
        let content = "  nguyễn   văn   a  ,  2026-05-10  \n  nguyễn   văn   a  ,  2026-05-10  ";
        let opts = ExcelCleanOptions {
            normalize_dates: true,
            split_vietnamese_names: false,
            trim_spaces: true,
            deduplicate: true,
        };

        let cleaned = clean_csv_content(content, &opts, &cancel);
        assert!(cleaned.is_ok());
        let res = cleaned.expect("valid clean");
        // Dòng trùng lặp phải được loại bỏ còn đúng 1 dòng
        assert_eq!(res.lines().count(), 1);
        assert_eq!(res, "Nguyễn Văn A,10/05/2026");
    }
}
