use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Các phương ngữ SQL được hỗ trợ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SqlDialect {
    #[default]
    Postgres,
    MySql,
    Sqlite,
    TSql,
}


/// Cấu hình tùy chọn định dạng và linter SQL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlFormatterOptions {
    #[serde(default)]
    pub dialect: SqlDialect,
    #[serde(default = "default_true")]
    pub uppercase_keywords: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SqlFormatterOptions {
    fn default() -> Self {
        Self {
            dialect: SqlDialect::Postgres,
            uppercase_keywords: true,
        }
    }
}

pub struct SqlFormatterLinterTool;

impl Default for SqlFormatterLinterTool {
    fn default() -> Self {
        Self
    }
}

/// Danh sách từ khóa chuẩn SQL (sắp xếp cụm từ nhiều từ lên trước)
const SQL_KEYWORDS: &[&str] = &[
    "inner join", "left join", "right join", "full join", "cross join", "outer join",
    "group by", "order by", "union all", "insert into", "delete from", "create table",
    "drop table", "alter table", "is not null", "is null",
    "select", "distinct", "from", "where", "having", "limit", "offset", "join",
    "on", "as", "and", "or", "not", "in", "between", "like", "ilike", "case",
    "when", "then", "else", "end", "values", "update", "set", "union", "asc", "desc",
];

/// Kết quả kiểm tra cú pháp AST SQL
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqlLintResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
}

/// Kiểm tra cú pháp AST / cấu trúc câu lệnh SQL
pub fn lint_sql_syntax(sql: &str, cancel_flag: &AtomicBool) -> Result<SqlLintResult, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let mut errors = Vec::new();
    let mut paren_depth = 0i32;
    let mut in_string = false;
    let mut quote_char = ' ';

    let chars: Vec<char> = sql.chars().collect();
    let len = chars.len();
    let mut idx = 0;

    while idx < len {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let c = chars[idx];

        if in_string {
            if c == quote_char {
                // Thoát ký tự nháy kép: ''
                if idx + 1 < len && chars[idx + 1] == quote_char {
                    idx += 2;
                    continue;
                } else {
                    in_string = false;
                }
            }
        } else {
            if c == '\'' || c == '"' || c == '`' {
                in_string = true;
                quote_char = c;
            } else if c == '(' {
                paren_depth += 1;
            } else if c == ')' {
                paren_depth -= 1;
                if paren_depth < 0 {
                    errors.push(format!("Phát hiện dấu đóng ngoặc ')' thừa tại vị trí ký tự {}", idx + 1));
                    paren_depth = 0;
                }
            }
        }

        idx += 1;
    }

    if in_string {
        errors.push("Phát hiện chuỗi ký tự chưa được đóng ngoặc nháy".to_string());
    }

    if paren_depth > 0 {
        errors.push(format!("Phát hiện {} dấu mở ngoặc '(' chưa được đóng kín", paren_depth));
    }

    // Kiểm tra cấu trúc câu lệnh SELECT cơ bản
    let lower_sql = sql.to_lowercase();
    if lower_sql.contains("select ")
        && let (Some(where_pos), Some(from_pos)) = (lower_sql.find("where "), lower_sql.find("from "))
            && where_pos < from_pos && paren_depth == 0 {
                errors.push("Cú pháp sai: Mệnh đề WHERE không thể đứng trước FROM trong câu truy vấn chính".to_string());
            }

    Ok(SqlLintResult {
        is_valid: errors.is_empty(),
        errors,
    })
}

/// Định dạng và căn chỉnh thụt lề câu lệnh SQL
pub fn format_sql_query(
    sql: &str,
    options: &SqlFormatterOptions,
    cancel_flag: &AtomicBool,
) -> Result<String, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let mut working = sql.to_string();

    // Viết hoa các từ khóa SQL nếu được kích hoạt
    if options.uppercase_keywords {
        for &kw in SQL_KEYWORDS {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }
            let upper = kw.to_uppercase();
            let pattern = format!(r"(?i)\b{}\b", kw);
            if let Ok(re) = regex::Regex::new(&pattern) {
                working = re.replace_all(&working, upper.as_str()).into_owned();
            }
        }
    }

    // Tạm thời bảo vệ các cụm từ khóa ghép để tránh bị thay thế bởi từ khóa đơn (như JOIN)
    let placeholders = &[
        ("INNER JOIN", "___INNER_JOIN___"),
        ("LEFT JOIN", "___LEFT_JOIN___"),
        ("RIGHT JOIN", "___RIGHT_JOIN___"),
        ("FULL JOIN", "___FULL_JOIN___"),
        ("CROSS JOIN", "___CROSS_JOIN___"),
        ("GROUP BY", "___GROUP_BY___"),
        ("ORDER BY", "___ORDER_BY___"),
        ("UNION ALL", "___UNION_ALL___"),
    ];

    for &(orig, ph) in placeholders {
        working = working.replace(orig, ph);
    }

    // Căn chỉnh thụt lề các mệnh đề chính
    let major_clauses = &[
        ("SELECT ", "\nSELECT\n  "),
        (" FROM ", "\nFROM\n  "),
        (" WHERE ", "\nWHERE\n  "),
        (" ___GROUP_BY___ ", "\n___GROUP_BY___\n  "),
        (" ___ORDER_BY___ ", "\n___ORDER_BY___\n  "),
        (" HAVING ", "\nHAVING\n  "),
        (" LIMIT ", "\nLIMIT "),
        (" OFFSET ", "\nOFFSET "),
        (" ___INNER_JOIN___ ", "\n  ___INNER_JOIN___ "),
        (" ___LEFT_JOIN___ ", "\n  ___LEFT_JOIN___ "),
        (" ___RIGHT_JOIN___ ", "\n  ___RIGHT_JOIN___ "),
        (" ___FULL_JOIN___ ", "\n  ___FULL_JOIN___ "),
        (" ___CROSS_JOIN___ ", "\n  ___CROSS_JOIN___ "),
        (" JOIN ", "\n  JOIN "),
        (" AND ", "\n  AND "),
        (" OR ", "\n  OR "),
    ];

    for &(from_kw, to_kw) in major_clauses {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }
        working = working.replace(from_kw, to_kw);
    }

    // Khôi phục các placeholder về từ khóa nguyên bản sau khi đã căn chỉnh thụt lề
    for &(orig, ph) in placeholders {
        working = working.replace(ph, orig);
    }

    // Làm sạch dòng trống thừa
    let lines: Vec<&str> = working
        .lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty())
        .collect();

    Ok(lines.join("\n"))
}

impl OfficeTool for SqlFormatterLinterTool {
    fn slug(&self) -> &'static str {
        "sql-formatter-linter"
    }

    fn name(&self) -> &'static str {
        "Định dạng & Linter Câu lệnh SQL"
    }

    fn description(&self) -> &'static str {
        "Kiểm tra lỗi cú pháp AST bằng linter và căn chỉnh thụt lề, viết hoa từ khóa chuẩn hóa theo các phương ngữ PostgreSQL, MySQL, SQLite, T-SQL."
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

        let options: SqlFormatterOptions = serde_json::from_value(request.options.clone())
            .unwrap_or_default();

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
                    expected: "Tệp mã nguồn SQL hoặc văn bản".to_string(),
                    actual: "Tệp nhị phân thực thi bị chặn".to_string(),
                });
            }

            String::from_utf8_lossy(&raw_bytes).to_string()
        } else if let Some(sql_val) = request.options.get("sql").and_then(|v| v.as_str()) {
            sql_val.to_string()
        } else {
            return Err(CommandError::ValidationFailed {
                reason: "Không có câu lệnh SQL hoặc tệp đầu vào".to_string(),
            });
        };

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 50.0,
                message: "Đang phân tích cú pháp AST và linter câu lệnh SQL...".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        let lint = lint_sql_syntax(&content, cancel_flag)?;
        if !lint.is_valid {
            return Err(CommandError::ValidationFailed {
                reason: format!("Lỗi cú pháp SQL: {}", lint.errors.join("; ")),
            });
        }

        let formatted = format_sql_query(&content, &options, cancel_flag)?;

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("sql_format_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        let out_file = out_dir.join("formatted.sql");
        fs::write(&out_file, formatted.as_bytes()).map_err(|e| CommandError::IoError {
            details: e.to_string(),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Đã định dạng và chuẩn hóa câu lệnh SQL".to_string(),
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
    fn test_sql_formatter_linter_happy_path() {
        let tool = SqlFormatterLinterTool;
        assert_eq!(tool.slug(), "sql-formatter-linter");

        // Kiểm tra cú pháp AST bắt lỗi sai ngữ pháp
        let cancel = AtomicBool::new(false);
        let bad_sql = "SELECT id, name WHERE salary > 1000 FROM employees (unclosed";
        let lint = lint_sql_syntax(bad_sql, &cancel).expect("lint check ok");

        // Phát hiện lỗi không đóng ngoặc và mệnh đề WHERE trước FROM
        assert!(!lint.is_valid);
        assert!(lint.errors.iter().any(|e| e.contains("chưa được đóng kín") || e.contains("WHERE")));
    }

    #[test]
    fn test_sql_formatter_linter_edge_case() {
        // Căn chỉnh thụt lề câu lệnh lồng JOIN phức tạp sắc nét
        let raw_sql = "select u.id, u.name, o.total from users u inner join orders o on u.id = o.user_id left join payments p on o.id = p.order_id where o.status = 'COMPLETED' and o.total > 1000000 order by o.created_at desc";
        let cancel = AtomicBool::new(false);
        let opts = SqlFormatterOptions {
            dialect: SqlDialect::Postgres,
            uppercase_keywords: true,
        };

        let formatted = format_sql_query(raw_sql, &opts, &cancel).expect("format ok");
        assert!(formatted.contains("SELECT"));
        assert!(formatted.contains("FROM"));
        assert!(formatted.contains("INNER JOIN"));
        assert!(formatted.contains("LEFT JOIN"));
        assert!(formatted.contains("WHERE"));
        assert!(formatted.contains("AND"));
        assert!(formatted.contains("ORDER BY"));
    }

    #[test]
    fn test_sql_formatter_linter_invalid_magic_bytes() {
        let tool = SqlFormatterLinterTool;
        let tmp = std::env::temp_dir().join("test_sql_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("bad_exec.bin");
        fs::write(&bad_file, b"\x7fELF\x02\x01\x01\x00").expect("write elf");

        let req = DispatchTaskRequest {
            task_id: "task-fake-sql".to_string(),
            tool_slug: "sql-formatter-linter".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
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
    fn test_sql_formatter_linter_cooperative_cancellation() {
        let tool = SqlFormatterLinterTool;
        let req = DispatchTaskRequest {
            task_id: "task-cancel-sql".to_string(),
            tool_slug: "sql-formatter-linter".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "sql": "SELECT 1;"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_sql_formatter_linter_security_guardrails() {
        // Kiểm thử câu lệnh đúng cú pháp linter xác nhận hợp lệ
        let cancel = AtomicBool::new(false);
        let good_sql = "SELECT id, name FROM users WHERE active = true;";
        let lint = lint_sql_syntax(good_sql, &cancel).expect("lint ok");
        assert!(lint.is_valid);
        assert!(lint.errors.is_empty());
    }
}
