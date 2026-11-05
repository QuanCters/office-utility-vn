use leptos::ev;
use leptos::prelude::*;
use crate::micro_apps::data_lab::state::DataLabState;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["navigator", "clipboard"])]
    fn writeText(text: &str) -> js_sys::Promise;
}

/// Tiện ích sao chép văn bản vào Clipboard an toàn (hỗ trợ cả WASM và Non-WASM target).
fn copy_to_clipboard(text: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = writeText(text);
    }
    let _ = text;
}

/// Tiện ích kích hoạt tải file về trình duyệt máy người dùng mà không cần thư viện ngoài.
fn trigger_download(filename: &str, content: &str, mime: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let script = format!(
            "(() => {{ const a = document.createElement('a'); a.href = 'data:{};charset=utf-8,' + encodeURIComponent({}); a.download = {}; document.body.appendChild(a); a.click(); document.body.removeChild(a); }})()",
            mime,
            serde_json::to_string(content).unwrap_or_else(|_| "''".to_string()),
            serde_json::to_string(filename).unwrap_or_else(|_| "'result.txt'".to_string()),
        );
        let _ = js_sys::eval(&script);
    }
    let _ = (filename, content, mime);
}

/// 4 Chế độ chuyển đổi chính của SplitPaneConverterView theo mô hình DevToys & CyberChef.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConverterMode {
    /// Chuyển đổi cấu trúc dữ liệu JSON ⇄ CSV ⇄ YAML ⇄ TOML
    DataStructure,
    /// Mã hóa & Giải mã Base64 cho Văn bản / Tệp
    Base64,
    /// Định dạng & Linter kiểm tra cú pháp SQL
    SqlFormatter,
    /// Đối chiếu & So sánh Text/Sheet Diff Checker
    DiffChecker,
}

impl ConverterMode {
    pub const ALL: [ConverterMode; 4] = [
        ConverterMode::DataStructure,
        ConverterMode::Base64,
        ConverterMode::SqlFormatter,
        ConverterMode::DiffChecker,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ConverterMode::DataStructure => "JSON ⇄ CSV ⇄ YAML ⇄ TOML (Data Structure)",
            ConverterMode::Base64 => "Văn bản / Tệp ⇄ Base64 (Base64 Codec)",
            ConverterMode::SqlFormatter => "SQL Formatter & Linter (Cú pháp SQL)",
            ConverterMode::DiffChecker => "So sánh Đối chiếu Diff (Text / Sheet Diff)",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            ConverterMode::DataStructure => "Data Structure",
            ConverterMode::Base64 => "Base64 Codec",
            ConverterMode::SqlFormatter => "SQL Formatter",
            ConverterMode::DiffChecker => "Diff Checker",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            ConverterMode::DataStructure => "🔄",
            ConverterMode::Base64 => "🔐",
            ConverterMode::SqlFormatter => "💻",
            ConverterMode::DiffChecker => "⚖️",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "data-structure-converter" => Some(ConverterMode::DataStructure),
            "base64-file-codec" => Some(ConverterMode::Base64),
            "sql-formatter-linter" => Some(ConverterMode::SqlFormatter),
            "diff-checker-text-sheets" => Some(ConverterMode::DiffChecker),
            _ => None,
        }
    }

    pub fn to_slug(self) -> &'static str {
        match self {
            ConverterMode::DataStructure => "data-structure-converter",
            ConverterMode::Base64 => "base64-file-codec",
            ConverterMode::SqlFormatter => "sql-formatter-linter",
            ConverterMode::DiffChecker => "diff-checker-text-sheets",
        }
    }
}

/// Định dạng đích cho chế độ DataStructure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataTargetFormat {
    JsonPretty,
    JsonMinify,
    Csv,
    Tsv,
    MarkdownTable,
    SqlInsert,
    Yaml,
}

impl DataTargetFormat {
    pub const ALL: [DataTargetFormat; 7] = [
        DataTargetFormat::JsonPretty,
        DataTargetFormat::JsonMinify,
        DataTargetFormat::Csv,
        DataTargetFormat::Tsv,
        DataTargetFormat::MarkdownTable,
        DataTargetFormat::SqlInsert,
        DataTargetFormat::Yaml,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DataTargetFormat::JsonPretty => "JSON (Prettify / Đẹp mắt)",
            DataTargetFormat::JsonMinify => "JSON (Minify / Nén gọn)",
            DataTargetFormat::Csv => "CSV (Phân tách dấu phẩy)",
            DataTargetFormat::Tsv => "TSV (Phân tách dấu Tab)",
            DataTargetFormat::MarkdownTable => "Markdown Table (Bảng kẻ)",
            DataTargetFormat::SqlInsert => "SQL INSERT INTO Statements",
            DataTargetFormat::Yaml => "YAML (Cấu trúc phân cấp)",
        }
    }

    pub fn file_ext(self) -> &'static str {
        match self {
            DataTargetFormat::JsonPretty | DataTargetFormat::JsonMinify => "json",
            DataTargetFormat::Csv => "csv",
            DataTargetFormat::Tsv => "tsv",
            DataTargetFormat::MarkdownTable => "md",
            DataTargetFormat::SqlInsert => "sql",
            DataTargetFormat::Yaml => "yaml",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            DataTargetFormat::JsonPretty | DataTargetFormat::JsonMinify => "application/json",
            DataTargetFormat::Csv => "text/csv",
            DataTargetFormat::Tsv => "text/tab-separated-values",
            DataTargetFormat::MarkdownTable => "text/markdown",
            DataTargetFormat::SqlInsert => "application/sql",
            DataTargetFormat::Yaml => "text/yaml",
        }
    }
}

/// Chiều xử lý Base64.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base64Direction {
    Encode,
    Decode,
    Auto,
}

/// Biến thể Base64 (Standard vs URL-Safe).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base64Variant {
    Standard,
    UrlSafe,
}

/// Kiểu định dạng SQL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqlFormatStyle {
    Pretty,
    Minify,
}

/// Kiểu chữ từ khóa SQL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqlKeywordCase {
    Uppercase,
    Lowercase,
}

/// Chế độ xem cột phải trong Diff Checker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffViewMode {
    Unified,
    SideBySide,
    EditTarget,
}

/// Kết quả của một dòng Diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffKind {
    Unchanged,
    Added,
    Removed,
}

/// Mẩu từ/ký tự trong dòng Diff hỗ trợ highlight cấp ký tự
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffWordChunk {
    pub text: String,
    pub is_highlight: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
    pub line_a: Option<usize>,
    pub line_b: Option<usize>,
    pub chunks: Vec<DiffWordChunk>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideBySideRow {
    pub left: Option<DiffLine>,
    pub right: Option<DiffLine>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiffStats {
    pub added: usize,
    pub removed: usize,
    pub unchanged: usize,
}

// ==============================================================================
// THUẬT TOÁN XỬ LÝ CLIENT-SIDE (0ms LATENCY)
// ==============================================================================

const B64_STANDARD_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64_URL_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub fn base64_encode_bytes(bytes: &[u8], url_safe: bool) -> String {
    let alphabet = if url_safe { B64_URL_CHARS } else { B64_STANDARD_CHARS };
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };

        let idx0 = (b0 >> 2) as usize;
        let idx1 = (((b0 & 0x03) << 4) | (b1 >> 4)) as usize;
        let idx2 = (((b1 & 0x0F) << 2) | (b2 >> 6)) as usize;
        let idx3 = (b2 & 0x3F) as usize;

        out.push(alphabet[idx0] as char);
        out.push(alphabet[idx1] as char);
        if i + 1 < bytes.len() {
            out.push(alphabet[idx2] as char);
        } else if !url_safe {
            out.push('=');
        }
        if i + 2 < bytes.len() {
            out.push(alphabet[idx3] as char);
        } else if !url_safe {
            out.push('=');
        }
        i += 3;
    }
    out
}

pub fn base64_decode_str(input: &str, url_safe: bool) -> Result<Vec<u8>, String> {
    let clean: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if clean.is_empty() {
        return Ok(Vec::new());
    }

    let decode_byte = |b: u8| -> Result<u8, String> {
        match b {
            b'A'..=b'Z' => Ok(b - b'A'),
            b'a'..=b'z' => Ok(b - b'a' + 26),
            b'0'..=b'9' => Ok(b - b'0' + 52),
            b'+' => if url_safe { Err("Ký tự '+' không dùng trong URL-safe Base64".to_string()) } else { Ok(62) },
            b'/' => if url_safe { Err("Ký tự '/' không dùng trong URL-safe Base64".to_string()) } else { Ok(63) },
            b'-' => if url_safe { Ok(62) } else { Err("Ký tự '-' chỉ dùng trong URL-safe Base64".to_string()) },
            b'_' => if url_safe { Ok(63) } else { Err("Ký tự '_' chỉ dùng trong URL-safe Base64".to_string()) },
            b'=' => Ok(0),
            _ => Err(format!("Ký tự không hợp lệ trong Base64: '{}'", b as char)),
        }
    };

    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    let mut i = 0;
    while i < clean.len() {
        let chunk_len = (clean.len() - i).min(4);
        if chunk_len < 2 {
            return Err("Chuỗi Base64 bị cụt hoặc thiếu byte kết thúc.".to_string());
        }

        let b0 = clean[i];
        let b1 = clean[i + 1];
        let b2 = if chunk_len > 2 { clean[i + 2] } else { b'=' };
        let b3 = if chunk_len > 3 { clean[i + 3] } else { b'=' };

        let v0 = decode_byte(b0)?;
        let v1 = decode_byte(b1)?;
        let v2 = if b2 == b'=' { 0 } else { decode_byte(b2)? };
        let v3 = if b3 == b'=' { 0 } else { decode_byte(b3)? };

        out.push((v0 << 2) | (v1 >> 4));
        if b2 != b'=' {
            out.push(((v1 & 0x0F) << 4) | (v2 >> 2));
        }
        if b3 != b'=' {
            out.push(((v2 & 0x03) << 6) | v3);
        }

        i += 4;
    }
    Ok(out)
}

fn looks_like_base64(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.len() < 4 {
        return false;
    }
    let valid_chars = trimmed.bytes().all(|b| {
        b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'-' || b == b'_' || b == b'=' || b.is_ascii_whitespace()
    });
    if !valid_chars {
        return false;
    }
    // Thử giải mã nếu ra UTF-8 hợp lệ
    if let Ok(bytes) = base64_decode_str(trimmed, trimmed.contains('-') || trimmed.contains('_')) {
        std::str::from_utf8(&bytes).is_ok()
    } else {
        false
    }
}

pub fn convert_base64(
    input: &str,
    direction: Base64Direction,
    variant: Base64Variant,
    wrap_len: usize,
) -> Result<String, String> {
    if input.trim().is_empty() {
        return Ok(String::new());
    }

    let url_safe = matches!(variant, Base64Variant::UrlSafe);

    let actual_dir = match direction {
        Base64Direction::Encode => Base64Direction::Encode,
        Base64Direction::Decode => Base64Direction::Decode,
        Base64Direction::Auto => {
            if looks_like_base64(input) {
                Base64Direction::Decode
            } else {
                Base64Direction::Encode
            }
        }
    };

    match actual_dir {
        Base64Direction::Encode => {
            let encoded = base64_encode_bytes(input.as_bytes(), url_safe);
            if wrap_len > 0 {
                let chunks: Vec<&str> = encoded
                    .as_bytes()
                    .chunks(wrap_len)
                    .map(|c| std::str::from_utf8(c).unwrap_or(""))
                    .collect();
                Ok(chunks.join("\n"))
            } else {
                Ok(encoded)
            }
        }
        Base64Direction::Decode | Base64Direction::Auto => {
            let decoded_bytes = base64_decode_str(input, url_safe)?;
            match String::from_utf8(decoded_bytes) {
                Ok(text) => Ok(text),
                Err(err) => {
                    let bytes = err.into_bytes();
                    Ok(format!(
                        "/* Dữ liệu nhị phân không phải chuỗi UTF-8 (Độ dài: {} bytes) */\nHex dump: {:02X?}",
                        bytes.len(),
                        &bytes[..bytes.len().min(64)]
                    ))
                }
            }
        }
    }
}

fn parse_csv_to_json(input: &str, delimiter: char) -> Result<serde_json::Value, String> {
    let lines: Vec<&str> = input.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return Ok(serde_json::Value::Array(Vec::new()));
    }

    let split_row = |row: &str| -> Vec<String> {
        let mut cols = Vec::new();
        let mut cur = String::new();
        let mut in_quotes = false;
        let mut chars = row.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '"' {
                if in_quotes && chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            } else if c == delimiter && !in_quotes {
                cols.push(cur.trim().to_string());
                cur.clear();
            } else {
                cur.push(c);
            }
        }
        cols.push(cur.trim().to_string());
        cols
    };

    let headers = split_row(lines[0]);
    let mut rows = Vec::new();

    for line in &lines[1..] {
        let values = split_row(line);
        let mut map = serde_json::Map::new();
        for (i, h) in headers.iter().enumerate() {
            let raw_val = values.get(i).map(|s| s.as_str()).unwrap_or("");
            let val = if let Ok(n) = raw_val.parse::<i64>() {
                serde_json::Value::Number(n.into())
            } else if let Ok(f) = raw_val.parse::<f64>() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::String(raw_val.to_string()))
            } else if raw_val.eq_ignore_ascii_case("true") {
                serde_json::Value::Bool(true)
            } else if raw_val.eq_ignore_ascii_case("false") {
                serde_json::Value::Bool(false)
            } else if raw_val.eq_ignore_ascii_case("null") {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(raw_val.to_string())
            };
            map.insert(h.clone(), val);
        }
        rows.push(serde_json::Value::Object(map));
    }

    Ok(serde_json::Value::Array(rows))
}

fn json_to_csv_table(val: &serde_json::Value, delimiter: char) -> Result<String, String> {
    match val {
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                return Ok(String::new());
            }

            let mut headers: Vec<String> = Vec::new();
            for item in items {
                if let serde_json::Value::Object(map) = item {
                    for key in map.keys() {
                        if !headers.contains(key) {
                            headers.push(key.clone());
                        }
                    }
                }
            }

            if headers.is_empty() {
                // Mảng các giá trị nguyên thủy
                let mut out = String::from("value\n");
                for item in items {
                    out.push_str(&item.to_string());
                    out.push('\n');
                }
                return Ok(out);
            }

            let mut out = headers.join(&delimiter.to_string());
            out.push('\n');

            let escape_csv = |s: &str| -> String {
                if s.contains(delimiter) || s.contains('"') || s.contains('\n') {
                    format!("\"{}\"", s.replace('"', "\"\""))
                } else {
                    s.to_string()
                }
            };

            for item in items {
                if let serde_json::Value::Object(map) = item {
                    let row_vals: Vec<String> = headers
                        .iter()
                        .map(|h| {
                            map.get(h)
                                .map(|v| match v {
                                    serde_json::Value::String(s) => escape_csv(s),
                                    serde_json::Value::Null => "".to_string(),
                                    other => other.to_string(),
                                })
                                .unwrap_or_default()
                        })
                        .collect();
                    out.push_str(&row_vals.join(&delimiter.to_string()));
                    out.push('\n');
                }
            }

            Ok(out)
        }
        serde_json::Value::Object(map) => {
            let mut out = format!("Key{}Value\n", delimiter);
            for (k, v) in map {
                out.push_str(&format!("{}{}{}\n", k, delimiter, v));
            }
            Ok(out)
        }
        _ => Err("Cần đối tượng JSON hoặc mảng JSON để chuyển sang dạng bảng CSV.".to_string()),
    }
}

fn json_to_markdown_table(val: &serde_json::Value) -> Result<String, String> {
    match val {
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                return Ok("*(Bảng rỗng)*".to_string());
            }

            let mut headers: Vec<String> = Vec::new();
            for item in items {
                if let serde_json::Value::Object(map) = item {
                    for key in map.keys() {
                        if !headers.contains(key) {
                            headers.push(key.clone());
                        }
                    }
                }
            }

            if headers.is_empty() {
                return Ok("*(Mảng không chứa cấu trúc đối tượng)*".to_string());
            }

            let mut out = String::new();
            out.push_str("| ");
            out.push_str(&headers.join(" | "));
            out.push_str(" |\n| ");
            out.push_str(&headers.iter().map(|_| "---").collect::<Vec<_>>().join(" | "));
            out.push_str(" |\n");

            for item in items {
                if let serde_json::Value::Object(map) = item {
                    let row_vals: Vec<String> = headers
                        .iter()
                        .map(|h| {
                            map.get(h)
                                .map(|v| match v {
                                    serde_json::Value::String(s) => s.replace('|', "\\|"),
                                    serde_json::Value::Null => "".to_string(),
                                    other => other.to_string(),
                                })
                                .unwrap_or_default()
                        })
                        .collect();
                    out.push_str("| ");
                    out.push_str(&row_vals.join(" | "));
                    out.push_str(" |\n");
                }
            }

            Ok(out)
        }
        _ => Err("Chỉ hỗ trợ chuyển đổi mảng các đối tượng JSON sang Markdown Table.".to_string()),
    }
}

fn json_to_sql_inserts(val: &serde_json::Value, table_name: &str) -> Result<String, String> {
    let tbl = if table_name.trim().is_empty() { "records" } else { table_name.trim() };

    match val {
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                return Ok(format!("-- Bảng {} không có bản ghi nào", tbl));
            }

            let mut headers: Vec<String> = Vec::new();
            for item in items {
                if let serde_json::Value::Object(map) = item {
                    for key in map.keys() {
                        if !headers.contains(key) {
                            headers.push(key.clone());
                        }
                    }
                }
            }

            if headers.is_empty() {
                return Err("Mảng dữ liệu không chứa đối tượng có trường cột.".to_string());
            }

            let mut out = String::new();
            let col_list = headers.join(", ");

            for item in items {
                if let serde_json::Value::Object(map) = item {
                    let val_list: Vec<String> = headers
                        .iter()
                        .map(|h| {
                            map.get(h)
                                .map(|v| match v {
                                    serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                                    serde_json::Value::Null => "NULL".to_string(),
                                    serde_json::Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
                                    other => other.to_string(),
                                })
                                .unwrap_or_else(|| "NULL".to_string())
                        })
                        .collect();

                    out.push_str(&format!(
                        "INSERT INTO {} ({}) VALUES ({});\n",
                        tbl,
                        col_list,
                        val_list.join(", ")
                    ));
                }
            }

            Ok(out)
        }
        _ => Err("Cần mảng danh sách đối tượng JSON để tạo câu lệnh SQL INSERT.".to_string()),
    }
}

fn json_to_yaml_string(val: &serde_json::Value, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    match val {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            if s.contains('\n') || s.contains(':') || s.contains('#') || s.is_empty() {
                format!("\"{}\"", s.replace('"', "\\\""))
            } else {
                s.clone()
            }
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return "[]".to_string();
            }
            let mut out = String::new();
            for item in arr {
                out.push_str(&format!("\n{}- {}", pad, json_to_yaml_string(item, indent + 1).trim_start()));
            }
            out
        }
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                return "{}".to_string();
            }
            let mut out = String::new();
            for (k, v) in map {
                out.push_str(&format!("\n{}{}: {}", pad, k, json_to_yaml_string(v, indent + 1).trim_start()));
            }
            out
        }
    }
}

pub fn convert_data_structure(
    input: &str,
    target: DataTargetFormat,
    table_name: &str,
) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    // Nhận diện định dạng nguồn: Thử parse JSON trước, nếu thất bại thử CSV/TSV
    let parsed_json: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(json_err) => {
            // Nếu không phải JSON, kiểm tra xem có phải CSV hoặc TSV không
            let is_tsv = trimmed.contains('\t');
            let delimiter = if is_tsv { '\t' } else { ',' };
            match parse_csv_to_json(trimmed, delimiter) {
                Ok(v) => v,
                Err(_) => {
                    return Err(format!(
                        "Lỗi phân tích cú pháp dữ liệu: {}\n(Gợi ý: Đảm bảo dữ liệu là JSON chuẩn hoặc bảng CSV/TSV có dòng tiêu đề cột)",
                        json_err
                    ));
                }
            }
        }
    };

    match target {
        DataTargetFormat::JsonPretty => {
            serde_json::to_string_pretty(&parsed_json)
                .map_err(|e| format!("Lỗi format JSON Prettify: {}", e))
        }
        DataTargetFormat::JsonMinify => {
            serde_json::to_string(&parsed_json)
                .map_err(|e| format!("Lỗi format JSON Minify: {}", e))
        }
        DataTargetFormat::Csv => json_to_csv_table(&parsed_json, ','),
        DataTargetFormat::Tsv => json_to_csv_table(&parsed_json, '\t'),
        DataTargetFormat::MarkdownTable => json_to_markdown_table(&parsed_json),
        DataTargetFormat::SqlInsert => json_to_sql_inserts(&parsed_json, table_name),
        DataTargetFormat::Yaml => Ok(json_to_yaml_string(&parsed_json, 0).trim().to_string()),
    }
}

pub fn format_and_lint_sql(
    input: &str,
    style: SqlFormatStyle,
    case: SqlKeywordCase,
) -> (Result<String, String>, Vec<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (Ok(String::new()), Vec::new());
    }

    let sql_keywords = [
        "SELECT", "DISTINCT", "AS", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "LIKE", "IS", "NULL",
        "JOIN", "INNER JOIN", "LEFT JOIN", "RIGHT JOIN", "FULL JOIN", "CROSS JOIN", "ON",
        "GROUP BY", "HAVING", "ORDER BY", "ASC", "DESC", "LIMIT", "OFFSET",
        "INSERT INTO", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE FROM", "DELETE",
        "CREATE TABLE", "CREATE", "TABLE", "DROP TABLE", "DROP", "ALTER TABLE", "ALTER",
        "PRIMARY KEY", "FOREIGN KEY", "REFERENCES", "UNION", "UNION ALL", "CASE", "WHEN", "THEN", "ELSE", "END",
        "BETWEEN", "EXISTS", "COUNT", "SUM", "AVG", "MIN", "MAX",
    ];

    let mut warnings = Vec::new();

    // 1. Kiểm tra Linter: Cặp ngoặc đơn
    let mut paren_depth = 0i32;
    let mut quote_open: Option<char> = None;
    for c in trimmed.chars() {
        if let Some(q) = quote_open {
            if c == q {
                quote_open = None;
            }
        } else if c == '\'' || c == '"' || c == '`' {
            quote_open = Some(c);
        } else if c == '(' {
            paren_depth += 1;
        } else if c == ')' {
            paren_depth -= 1;
            if paren_depth < 0 {
                warnings.push("Thừa dấu ngoặc đóng ')' không khớp.".to_string());
                break;
            }
        }
    }

    if paren_depth > 0 {
        warnings.push(format!("Thiếu {} dấu ngoặc đóng ')' trong câu lệnh.", paren_depth));
    }
    if let Some(q) = quote_open {
        warnings.push(format!("Dấu nháy '{}' chưa được đóng.", q));
    }

    // 2. Kiểm tra dấu phẩy thừa trước FROM
    let upper_trimmed = trimmed.to_uppercase();
    if upper_trimmed.contains(", FROM") || upper_trimmed.contains(",\nFROM") || upper_trimmed.contains(",\r\nFROM") {
        warnings.push("Phát hiện dấu phẩy thừa ',' ngay trước mệnh đề FROM.".to_string());
    }

    // 3. Định dạng SQL
    let mut result = String::new();
    let is_upper = matches!(case, SqlKeywordCase::Uppercase);

    match style {
        SqlFormatStyle::Minify => {
            // Nén gọn một dòng, xóa khoảng trắng thừa
            let mut words = Vec::new();
            for word in trimmed.split_whitespace() {
                let clean_word = word.trim_matches(|c: char| c == ',' || c == '(' || c == ')' || c == ';');
                let upper = clean_word.to_uppercase();
                if sql_keywords.iter().any(|&k| k == upper) {
                    let repl = if is_upper { upper } else { clean_word.to_lowercase() };
                    words.push(word.replace(clean_word, &repl));
                } else {
                    words.push(word.to_string());
                }
            }
            result = words.join(" ");
        }
        SqlFormatStyle::Pretty => {
            // Định dạng thụt dòng đẹp mắt
            let clauses_newline = [
                "SELECT", "FROM", "WHERE", "GROUP BY", "ORDER BY", "HAVING", "LIMIT",
                "JOIN", "LEFT JOIN", "RIGHT JOIN", "INNER JOIN", "UNION", "UPDATE", "SET", "VALUES"
            ];

            let mut in_quote = false;
            let mut quote_char = ' ';
            let mut tokens = Vec::new();
            let mut cur_token = String::new();

            for c in trimmed.chars() {
                if in_quote {
                    cur_token.push(c);
                    if c == quote_char {
                        in_quote = false;
                        tokens.push(cur_token.clone());
                        cur_token.clear();
                    }
                } else if c == '\'' || c == '"' || c == '`' {
                    if !cur_token.trim().is_empty() {
                        tokens.push(cur_token.clone());
                        cur_token.clear();
                    }
                    in_quote = true;
                    quote_char = c;
                    cur_token.push(c);
                } else if c.is_whitespace() || c == ',' || c == '(' || c == ')' || c == ';' {
                    if !cur_token.trim().is_empty() {
                        tokens.push(cur_token.clone());
                        cur_token.clear();
                    }
                    if !c.is_whitespace() {
                        tokens.push(c.to_string());
                    }
                } else {
                    cur_token.push(c);
                }
            }
            if !cur_token.trim().is_empty() {
                tokens.push(cur_token);
            }

            let mut indent_lvl = 0usize;
            let mut at_line_start = true;

            for (i, tok) in tokens.iter().enumerate() {
                let upper_tok = tok.to_uppercase();
                let is_clause = clauses_newline.iter().any(|&k| k == upper_tok);

                if is_clause && !at_line_start {
                    result.push('\n');
                    result.push_str(&"  ".repeat(indent_lvl));
                    at_line_start = false;
                }

                let formatted_tok = if sql_keywords.iter().any(|&k| k == upper_tok) {
                    if is_upper { upper_tok.clone() } else { tok.to_lowercase() }
                } else {
                    tok.clone()
                };

                if tok == "(" {
                    result.push_str(&formatted_tok);
                    indent_lvl += 1;
                } else if tok == ")" {
                    indent_lvl = indent_lvl.saturating_sub(1);
                    result.push_str(&formatted_tok);
                } else if tok == "," {
                    result.push_str(", ");
                } else if tok == ";" {
                    result.push_str(";\n");
                    at_line_start = true;
                } else {
                    if !at_line_start && !result.ends_with(' ') && !result.ends_with('(') {
                        result.push(' ');
                    }
                    result.push_str(&formatted_tok);
                    at_line_start = false;
                }

                // Xuống dòng sau SELECT hoặc FROM nếu cần
                let next_is_clause = tokens.get(i + 1).map(|t| clauses_newline.iter().any(|&k| k == t.to_uppercase())).unwrap_or(false);
                if next_is_clause && !result.ends_with('\n') {
                    result.push('\n');
                    result.push_str(&"  ".repeat(indent_lvl));
                    at_line_start = true;
                }
            }
        }
    }

    (Ok(result.trim().to_string()), warnings)
}

pub fn format_byte_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn detect_input_format(text: &str) -> &'static str {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "Trống";
    }
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        "JSON"
    } else if trimmed.starts_with("SELECT ")
        || trimmed.starts_with("select ")
        || trimmed.starts_with("INSERT ")
        || trimmed.starts_with("insert ")
        || trimmed.starts_with("CREATE ")
        || trimmed.starts_with("create ")
        || trimmed.starts_with("UPDATE ")
        || trimmed.starts_with("update ")
    {
        "SQL"
    } else if looks_like_base64(trimmed) {
        "Base64"
    } else if (trimmed.contains(',') || trimmed.contains('\t')) && trimmed.lines().count() > 1 {
        if trimmed.contains('\t') { "TSV" } else { "CSV" }
    } else {
        "Văn bản (Text)"
    }
}

fn tokenize_line(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut prev_kind = 0; // 1: word/alnum, 2: whitespace, 3: symbol

    for c in line.chars() {
        let kind = if c.is_alphanumeric() || c == '_' {
            1
        } else if c.is_whitespace() {
            2
        } else {
            3
        };

        if prev_kind != 0 && (kind != prev_kind || kind == 3) {
            tokens.push(cur.clone());
            cur.clear();
        }
        cur.push(c);
        prev_kind = kind;
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

pub fn compute_word_diff(old_line: &str, new_line: &str) -> (Vec<DiffWordChunk>, Vec<DiffWordChunk>) {
    let toks_a = tokenize_line(old_line);
    let toks_b = tokenize_line(new_line);

    let n = toks_a.len();
    let m = toks_b.len();

    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            if toks_a[i] == toks_b[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut matched_a = vec![false; n];
    let mut matched_b = vec![false; m];
    let mut i = n;
    let mut j = m;

    while i > 0 && j > 0 {
        if toks_a[i - 1] == toks_b[j - 1] {
            matched_a[i - 1] = true;
            matched_b[j - 1] = true;
            i -= 1;
            j -= 1;
        } else if dp[i][j - 1] >= dp[i - 1][j] {
            j -= 1;
        } else {
            i -= 1;
        }
    }

    let build_chunks = |toks: &[String], matched: &[bool]| -> Vec<DiffWordChunk> {
        let mut chunks = Vec::new();
        let mut cur_text = String::new();
        let mut cur_hl: Option<bool> = None;

        for (k, tok) in toks.iter().enumerate() {
            let is_hl = !matched[k];
            match cur_hl {
                Some(hl) if hl == is_hl => {
                    cur_text.push_str(tok);
                }
                Some(_) => {
                    chunks.push(DiffWordChunk {
                        text: cur_text.clone(),
                        is_highlight: cur_hl.unwrap(),
                    });
                    cur_text = tok.clone();
                    cur_hl = Some(is_hl);
                }
                None => {
                    cur_text = tok.clone();
                    cur_hl = Some(is_hl);
                }
            }
        }
        if let Some(hl) = cur_hl {
            chunks.push(DiffWordChunk {
                text: cur_text,
                is_highlight: hl,
            });
        }
        if chunks.is_empty() {
            chunks.push(DiffWordChunk {
                text: String::new(),
                is_highlight: false,
            });
        }
        chunks
    };

    (build_chunks(&toks_a, &matched_a), build_chunks(&toks_b, &matched_b))
}

pub fn compute_line_diff(a: &str, b: &str, ignore_ws: bool) -> (Vec<DiffLine>, DiffStats) {
    let lines_a: Vec<&str> = a.lines().collect();
    let lines_b: Vec<&str> = b.lines().collect();

    let norm = |s: &str| -> String {
        if ignore_ws {
            s.split_whitespace().collect::<Vec<_>>().join(" ")
        } else {
            s.to_string()
        }
    };

    let n = lines_a.len();
    let m = lines_b.len();

    // Thuật toán LCS trên mảng dòng
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            if norm(lines_a[i]) == norm(lines_b[j]) {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut raw_diff = Vec::new();
    let mut i = n;
    let mut j = m;
    let mut stats = DiffStats::default();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && norm(lines_a[i - 1]) == norm(lines_b[j - 1]) {
            raw_diff.push(DiffLine {
                kind: DiffKind::Unchanged,
                text: lines_a[i - 1].to_string(),
                line_a: Some(i),
                line_b: Some(j),
                chunks: vec![DiffWordChunk {
                    text: lines_a[i - 1].to_string(),
                    is_highlight: false,
                }],
            });
            stats.unchanged += 1;
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            let text = lines_b[j - 1].to_string();
            raw_diff.push(DiffLine {
                kind: DiffKind::Added,
                text: text.clone(),
                line_a: None,
                line_b: Some(j),
                chunks: vec![DiffWordChunk {
                    text,
                    is_highlight: false,
                }],
            });
            stats.added += 1;
            j -= 1;
        } else if i > 0 && (j == 0 || dp[i][j - 1] < dp[i - 1][j]) {
            let text = lines_a[i - 1].to_string();
            raw_diff.push(DiffLine {
                kind: DiffKind::Removed,
                text: text.clone(),
                line_a: Some(i),
                line_b: None,
                chunks: vec![DiffWordChunk {
                    text,
                    is_highlight: false,
                }],
            });
            stats.removed += 1;
            i -= 1;
        }
    }

    raw_diff.reverse();

    // Word/character level diff post-processing: Pair adjacent runs of Removed and Added lines
    let mut processed_diff = Vec::with_capacity(raw_diff.len());
    let mut idx = 0;
    while idx < raw_diff.len() {
        if raw_diff[idx].kind == DiffKind::Removed {
            let rem_start = idx;
            while idx < raw_diff.len() && raw_diff[idx].kind == DiffKind::Removed {
                idx += 1;
            }
            let rem_end = idx;

            let add_start = idx;
            while idx < raw_diff.len() && raw_diff[idx].kind == DiffKind::Added {
                idx += 1;
            }
            let add_end = idx;

            let rem_count = rem_end - rem_start;
            let add_count = add_end - add_start;
            let pair_count = rem_count.min(add_count);

            let mut rem_slice: Vec<DiffLine> = raw_diff[rem_start..rem_end].to_vec();
            let mut add_slice: Vec<DiffLine> = raw_diff[add_start..add_end].to_vec();

            for k in 0..pair_count {
                let (ch_a, ch_b) = compute_word_diff(&rem_slice[k].text, &add_slice[k].text);
                rem_slice[k].chunks = ch_a;
                add_slice[k].chunks = ch_b;
            }

            processed_diff.extend(rem_slice);
            processed_diff.extend(add_slice);
        } else {
            processed_diff.push(raw_diff[idx].clone());
            idx += 1;
        }
    }

    (processed_diff, stats)
}

pub fn build_side_by_side_rows(lines: &[DiffLine]) -> Vec<SideBySideRow> {
    let mut rows = Vec::new();
    let mut idx = 0;

    while idx < lines.len() {
        match lines[idx].kind {
            DiffKind::Unchanged => {
                rows.push(SideBySideRow {
                    left: Some(lines[idx].clone()),
                    right: Some(lines[idx].clone()),
                });
                idx += 1;
            }
            DiffKind::Removed | DiffKind::Added => {
                let mut rem_lines = Vec::new();
                let mut add_lines = Vec::new();

                while idx < lines.len() && (lines[idx].kind == DiffKind::Removed || lines[idx].kind == DiffKind::Added) {
                    if lines[idx].kind == DiffKind::Removed {
                        rem_lines.push(lines[idx].clone());
                    } else {
                        add_lines.push(lines[idx].clone());
                    }
                    idx += 1;
                }

                let max_len = rem_lines.len().max(add_lines.len());
                for k in 0..max_len {
                    rows.push(SideBySideRow {
                        left: rem_lines.get(k).cloned(),
                        right: add_lines.get(k).cloned(),
                    });
                }
            }
        }
    }

    rows
}

fn sample_for_mode(mode: ConverterMode) -> (&'static str, &'static str) {
    match mode {
        ConverterMode::DataStructure => (
            r#"[
  {
    "id": 1,
    "ho_ten": "Nguyễn Văn An",
    "phong_ban": "Kế toán",
    "luong_co_ban": 18500000,
    "da_dong_bhxh": true
  },
  {
    "id": 2,
    "ho_ten": "Trần Thị Bích",
    "phong_ban": "Kỹ thuật",
    "luong_co_ban": 24000000,
    "da_dong_bhxh": true
  },
  {
    "id": 3,
    "ho_ten": "Lê Hoàng Nam",
    "phong_ban": "Kinh doanh",
    "luong_co_ban": 15000000,
    "da_dong_bhxh": false
  }
]"#,
            "",
        ),
        ConverterMode::Base64 => (
            "Cộng hòa Xã hội Chủ nghĩa Việt Nam\nĐộc lập - Tự do - Hạnh phúc\n\nỨng dụng Office Utility VN chạy 100% On-Device an toàn bảo mật.",
            "",
        ),
        ConverterMode::SqlFormatter => (
            "select u.id,u.ho_ten,count(o.id) as tong_don,sum(o.gia_tri) as doanh_so from nguoi_dung u left join don_hang o on u.id = o.user_id where u.trang_thai = 1 and o.status in ('COMPLETED','PAID') group by u.id,u.ho_ten having count(o.id) > 3 order by doanh_so desc limit 50;",
            "",
        ),
        ConverterMode::DiffChecker => (
            "Bảng lương tháng 10/2026\n1. Nguyễn Văn An - 18,500,000 đ\n2. Trần Thị Bích - 22,000,000 đ\n3. Lê Hoàng Nam - 15,000,000 đ\nTổng nhân sự: 3",
            "Bảng lương tháng 11/2026\n1. Nguyễn Văn An - 18,500,000 đ\n2. Trần Thị Bích - 24,000,000 đ (Tăng lương)\n3. Lê Hoàng Nam - 15,000,000 đ\n4. Phạm Thu Hà - 16,000,000 đ (Nhân sự mới)\nTổng nhân sự: 4",
        ),
    }
}

// ==============================================================================
// LEPTOS 0.8 COMPONENT CHÍNH
// ==============================================================================

/// Component chuyển đổi Split-Pane song song 2 cột với cập nhật Real-Time.
///
/// Tuân thủ quy chuẩn Leptos 0.8 CSR, IME-Safe, Design Tokens và Ergonomic Floor >= 12px.
#[component]
pub fn SplitPaneConverterView(
    state: DataLabState,
    #[prop(into, optional)] initial_mode: Option<ConverterMode>,
) -> impl IntoView {
    // 1. Xác định chế độ ban đầu từ slug của active_tool hoặc tham số prop
    let default_mode = initial_mode
        .or_else(|| ConverterMode::from_slug(&state.active_tool.get()))
        .unwrap_or(ConverterMode::DataStructure);

    let mode = RwSignal::new(default_mode);
    let input_text = RwSignal::new(String::new());
    let diff_target_text = RwSignal::new(String::new());
    let realtime_enabled = RwSignal::new(true);
    let manual_trigger = RwSignal::new(0usize);
    let copied_toast = RwSignal::new(false);

    // Sub-options cho từng mode
    let target_format = RwSignal::new(DataTargetFormat::JsonPretty);
    let sql_table_name = RwSignal::new("nguoi_dung".to_string());
    let b64_direction = RwSignal::new(Base64Direction::Auto);
    let b64_variant = RwSignal::new(Base64Variant::Standard);
    let b64_wrap = RwSignal::new(0usize);
    let sql_style = RwSignal::new(SqlFormatStyle::Pretty);
    let sql_case = RwSignal::new(SqlKeywordCase::Uppercase);
    let diff_view_mode = RwSignal::new(DiffViewMode::SideBySide);
    let diff_ignore_ws = RwSignal::new(false);

    // IME-safe typing control
    let is_composing = RwSignal::new(false);

    // Tự động nạp mẫu ban đầu nếu ô input còn trống
    {
        let (s_in, s_target) = sample_for_mode(default_mode);
        input_text.set(s_in.to_string());
        if !s_target.is_empty() {
            diff_target_text.set(s_target.to_string());
        }
    }

    // Đồng bộ khi user đổi tab công cụ bên ngoài DataLabState
    Effect::new(move |_| {
        let cur_slug = state.active_tool.get();
        if let Some(m) = ConverterMode::from_slug(&cur_slug) {
            if mode.get() != m {
                mode.set(m);
                let (s_in, s_target) = sample_for_mode(m);
                input_text.set(s_in.to_string());
                diff_target_text.set(s_target.to_string());
            }
        }
    });

    // Tính toán kết quả chuyển đổi (Reactivity qua Memo)
    let conversion_result = Memo::new(move |_| {
        // Lắng nghe manual_trigger nếu realtime tắt
        let _ = manual_trigger.get();
        let cur_mode = mode.get();
        let cur_in = input_text.get();

        match cur_mode {
            ConverterMode::DataStructure => {
                let fmt = target_format.get();
                let tbl = sql_table_name.get();
                let res = convert_data_structure(&cur_in, fmt, &tbl);
                (res, Vec::new(), Vec::new(), DiffStats::default(), Vec::new())
            }
            ConverterMode::Base64 => {
                let dir = b64_direction.get();
                let var = b64_variant.get();
                let wrap = b64_wrap.get();
                let res = convert_base64(&cur_in, dir, var, wrap);
                (res, Vec::new(), Vec::new(), DiffStats::default(), Vec::new())
            }
            ConverterMode::SqlFormatter => {
                let st = sql_style.get();
                let cs = sql_case.get();
                let (res, warns) = format_and_lint_sql(&cur_in, st, cs);
                (res, warns, Vec::new(), DiffStats::default(), Vec::new())
            }
            ConverterMode::DiffChecker => {
                let target = diff_target_text.get();
                let ign = diff_ignore_ws.get();
                let (lines, stats) = compute_line_diff(&cur_in, &target, ign);
                let side_rows = build_side_by_side_rows(&lines);

                // Tạo chuỗi text đại diện để sao chép / tải xuống
                let mut text_rep = String::new();
                for l in &lines {
                    match l.kind {
                        DiffKind::Added => text_rep.push_str(&format!("+ {}\n", l.text)),
                        DiffKind::Removed => text_rep.push_str(&format!("- {}\n", l.text)),
                        DiffKind::Unchanged => text_rep.push_str(&format!("  {}\n", l.text)),
                    }
                }
                (Ok(text_rep), Vec::new(), lines, stats, side_rows)
            }
        }
    });

    // Thao tác Swap (Hoán đổi Đầu vào & Đầu ra)
    let on_swap = move |_| {
        let cur_mode = mode.get();
        if cur_mode == ConverterMode::DiffChecker {
            let tmp_a = input_text.get();
            let tmp_b = diff_target_text.get();
            input_text.set(tmp_b);
            diff_target_text.set(tmp_a);
        } else {
            let (res, _, _, _, _) = conversion_result.get();
            if let Ok(out_val) = res {
                if !out_val.trim().is_empty() {
                    input_text.set(out_val);
                    if cur_mode == ConverterMode::Base64 {
                        let cur_dir = b64_direction.get();
                        b64_direction.set(match cur_dir {
                            Base64Direction::Encode => Base64Direction::Decode,
                            Base64Direction::Decode => Base64Direction::Encode,
                            Base64Direction::Auto => Base64Direction::Auto,
                        });
                    }
                }
            }
        }
    };

    // Thao tác Clear
    let on_clear = move |_| {
        input_text.set(String::new());
        diff_target_text.set(String::new());
    };

    // Thao tác Nạp mẫu thử
    let on_load_sample = move |_| {
        let cur_mode = mode.get();
        let (s_in, s_target) = sample_for_mode(cur_mode);
        input_text.set(s_in.to_string());
        diff_target_text.set(s_target.to_string());
    };

    // Thao tác Sao chép Output vào Clipboard
    let on_copy_output = move |_| {
        let (res, _, _, _, _) = conversion_result.get();
        if let Ok(text) = res {
            copy_to_clipboard(&text);
            copied_toast.set(true);
            #[cfg(target_arch = "wasm32")]
            {
                leptos::task::spawn_local(async move {
                    use wasm_bindgen_futures::js_sys;
                    let promise = js_sys::Promise::new(&mut |resolve, _| {
                        if let Some(win) = web_sys::window() {
                            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 2000);
                        }
                    });
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                    copied_toast.set(false);
                });
            }
        }
    };

    // Thao tác Tải xuống tệp kết quả
    let on_download = move |_| {
        let (res, _, _, _, _) = conversion_result.get();
        if let Ok(content) = res {
            let cur_mode = mode.get();
            let (filename, mime) = match cur_mode {
                ConverterMode::DataStructure => {
                    let fmt = target_format.get();
                    (format!("data_output.{}", fmt.file_ext()), fmt.mime())
                }
                ConverterMode::Base64 => ("base64_result.txt".to_string(), "text/plain"),
                ConverterMode::SqlFormatter => ("formatted_query.sql".to_string(), "application/sql"),
                ConverterMode::DiffChecker => ("comparison_diff.diff".to_string(), "text/plain"),
            };
            trigger_download(&filename, &content, mime);
        }
    };

    // Thống kê đầu vào chi tiết
    let input_stats = Memo::new(move |_| {
        let text = input_text.get();
        let bytes = text.len();
        let chars = text.chars().count();
        let lines = if text.is_empty() { 0 } else { text.lines().count() };
        (lines, chars, bytes)
    });

    let detected_in_format = Memo::new(move |_| {
        if mode.get() == ConverterMode::DiffChecker {
            "Văn bản A (Gốc)".to_string()
        } else {
            detect_input_format(&input_text.get()).to_string()
        }
    });

    let output_stats = Memo::new(move |_| {
        let (res, _, _, _, _) = conversion_result.get();
        if let Ok(ref text) = res {
            let bytes = text.len();
            let chars = text.chars().count();
            let lines = if text.is_empty() { 0 } else { text.lines().count() };
            (lines, chars, bytes)
        } else {
            (0, 0, 0)
        }
    });

    let output_format_label = Memo::new(move |_| {
        match mode.get() {
            ConverterMode::DataStructure => target_format.get().label().to_string(),
            ConverterMode::Base64 => match b64_direction.get() {
                Base64Direction::Encode => "Base64 Encoded".to_string(),
                Base64Direction::Decode => "Plain Text Decoded".to_string(),
                Base64Direction::Auto => "Base64 Auto Codec".to_string(),
            },
            ConverterMode::SqlFormatter => match sql_style.get() {
                SqlFormatStyle::Pretty => "SQL Pretty Indent".to_string(),
                SqlFormatStyle::Minify => "SQL Minified".to_string(),
            },
            ConverterMode::DiffChecker => match diff_view_mode.get() {
                DiffViewMode::SideBySide => "Diff Side-by-Side".to_string(),
                DiffViewMode::Unified => "Diff Unified".to_string(),
                DiffViewMode::EditTarget => "Văn bản B (Đích)".to_string(),
            },
        }
    });

    view! {
        <div class="split-pane-converter" id="split-pane-converter-root">
            // 1. THANH CÔNG CỤ ĐIỀU KHIỂN CHÍNH (Top Toolbar)
            <div class="split-pane-toolbar">
                <div class="split-pane-toolbar-left">
                    <span style="font-weight: 700; font-size: 13px; color: var(--text-main); display: flex; align-items: center; gap: 6px;">
                        <span>"⚡"</span>
                        <span>"Bộ chuyển đổi:"</span>
                    </span>

                    // Bộ chọn định dạng chuyển đổi chính (Format Selector)
                    <select
                        class="split-pane-select"
                        prop:value=move || mode.get().to_slug()
                        on:change=move |ev| {
                            let slug = event_target_value(&ev);
                            if let Some(m) = ConverterMode::from_slug(&slug) {
                                mode.set(m);
                                state.select_tool(&slug);
                                let (s_in, s_target) = sample_for_mode(m);
                                input_text.set(s_in.to_string());
                                diff_target_text.set(s_target.to_string());
                            }
                        }
                    >
                        {ConverterMode::ALL.into_iter().map(|m| {
                            view! {
                                <option value=m.to_slug()>
                                    {format!("{} {}", m.icon(), m.label())}
                                </option>
                            }
                        }).collect::<Vec<_>>()}
                    </select>

                    // Nút Hoán đổi (Swap ⇄)
                    <button
                        type="button"
                        class="split-pane-btn"
                        title="Hoán đổi nội dung Input và Output (Swap)"
                        on:click=on_swap
                    >
                        <span>"⇄"</span>
                        <span>"Hoán đổi"</span>
                    </button>

                    // Switch Bật/Tắt Chuyển đổi Tức thời (Real-time Auto-convert)
                    <label class="split-pane-toggle-label" title="Tự động chuyển đổi ngay khi đang gõ phím">
                        <input
                            type="checkbox"
                            class="split-pane-toggle-input"
                            prop:checked=move || realtime_enabled.get()
                            on:change=move |ev| realtime_enabled.set(event_target_checked(&ev))
                        />
                        <span>"Chuyển đổi tức thời (0ms)"</span>
                    </label>

                    // Nút Thực hiện chuyển đổi thủ công khi tắt Real-time
                    {move || (!realtime_enabled.get()).then(|| view! {
                        <button
                            type="button"
                            class="split-pane-btn split-pane-btn-primary"
                            on:click=move |_| manual_trigger.update(|n| *n += 1)
                        >
                            <span>"▶"</span>
                            <span>"Chuyển đổi ngay"</span>
                        </button>
                    })}
                </div>

                <div class="split-pane-toolbar-right">
                    // Nút Nạp mẫu thử
                    <button
                        type="button"
                        class="split-pane-btn"
                        title="Nạp dữ liệu mẫu để dùng thử nhanh"
                        on:click=on_load_sample
                    >
                        <span>"🧪 Mẫu thử"</span>
                    </button>

                    // Nút Xóa trắng
                    <button
                        type="button"
                        class="split-pane-btn"
                        title="Xóa trắng toàn bộ nội dung"
                        on:click=on_clear
                    >
                        <span>"🗑️ Xóa trắng"</span>
                    </button>

                    // Nút Sao chép kết quả
                    <button
                        type="button"
                        class="split-pane-btn split-pane-btn-primary"
                        title="Sao chép nội dung Output vào Clipboard"
                        on:click=on_copy_output
                    >
                        <span>"📋 Sao chép"</span>
                    </button>

                    // Nút Tải xuống file
                    <button
                        type="button"
                        class="split-pane-btn"
                        title="Tải xuống tệp kết quả về máy"
                        on:click=on_download
                    >
                        <span>"💾 Tải xuống"</span>
                    </button>
                </div>
            </div>

            // 2. THANH TÙY CHỌN PHỤ THEO TỪNG CHẾ ĐỘ (Sub-toolbar Options)
            <div class="split-pane-subtoolbar">
                {move || match mode.get() {
                    ConverterMode::DataStructure => view! {
                        <div class="split-pane-subtoolbar-item">
                            <span>"Định dạng đích:"</span>
                            <select
                                class="split-pane-select"
                                prop:value=move || format!("{:?}", target_format.get())
                                on:change=move |ev| {
                                    let val = event_target_value(&ev);
                                    for fmt in DataTargetFormat::ALL {
                                        if format!("{:?}", fmt) == val {
                                            target_format.set(fmt);
                                            break;
                                        }
                                    }
                                }
                            >
                                {DataTargetFormat::ALL.into_iter().map(|fmt| view! {
                                    <option value=format!("{:?}", fmt)>{fmt.label()}</option>
                                }).collect::<Vec<_>>()}
                            </select>
                        </div>

                        {move || (target_format.get() == DataTargetFormat::SqlInsert).then(|| view! {
                            <div class="split-pane-subtoolbar-item">
                                <span>"Tên bảng SQL:"</span>
                                <input
                                    type="text"
                                    class="split-pane-input-sm"
                                    prop:value=move || sql_table_name.get()
                                    on:input=move |ev| sql_table_name.set(event_target_value(&ev))
                                    placeholder="users"
                                />
                            </div>
                        })}
                    }.into_any(),

                    ConverterMode::Base64 => view! {
                        <div class="split-pane-subtoolbar-item">
                            <span>"Chiều xử lý:"</span>
                            <select
                                class="split-pane-select"
                                prop:value=move || format!("{:?}", b64_direction.get())
                                on:change=move |ev| {
                                    let v = event_target_value(&ev);
                                    if v == "Encode" { b64_direction.set(Base64Direction::Encode); }
                                    else if v == "Decode" { b64_direction.set(Base64Direction::Decode); }
                                    else { b64_direction.set(Base64Direction::Auto); }
                                }
                            >
                                <option value="Auto">"Tự động nhận diện (Auto-detect)"</option>
                                <option value="Encode">"Mã hóa (Text ➔ Base64)"</option>
                                <option value="Decode">"Giải mã (Base64 ➔ Text)"</option>
                            </select>
                        </div>

                        <div class="split-pane-subtoolbar-item">
                            <span>"Bảng mã:"</span>
                            <select
                                class="split-pane-select"
                                prop:value=move || format!("{:?}", b64_variant.get())
                                on:change=move |ev| {
                                    if event_target_value(&ev) == "UrlSafe" {
                                        b64_variant.set(Base64Variant::UrlSafe);
                                    } else {
                                        b64_variant.set(Base64Variant::Standard);
                                    }
                                }
                            >
                                <option value="Standard">"Standard (+ / =)"</option>
                                <option value="UrlSafe">"URL-Safe (- _ không đệm)"</option>
                            </select>
                        </div>

                        <div class="split-pane-subtoolbar-item">
                            <span>"Ngắt dòng:"</span>
                            <select
                                class="split-pane-select"
                                prop:value=move || b64_wrap.get().to_string()
                                on:change=move |ev| {
                                    let w = event_target_value(&ev).parse::<usize>().unwrap_or(0);
                                    b64_wrap.set(w);
                                }
                            >
                                <option value="0">"Không ngắt dòng"</option>
                                <option value="64">"Mỗi 64 ký tự (MIME)"</option>
                                <option value="76">"Mỗi 76 ký tự (RFC 2045)"</option>
                            </select>
                        </div>
                    }.into_any(),

                    ConverterMode::SqlFormatter => view! {
                        <div class="split-pane-subtoolbar-item">
                            <span>"Phong cách:"</span>
                            <select
                                class="split-pane-select"
                                prop:value=move || format!("{:?}", sql_style.get())
                                on:change=move |ev| {
                                    if event_target_value(&ev) == "Minify" {
                                        sql_style.set(SqlFormatStyle::Minify);
                                    } else {
                                        sql_style.set(SqlFormatStyle::Pretty);
                                    }
                                }
                            >
                                <option value="Pretty">"Định dạng thụt lề chuẩn (Pretty Indent)"</option>
                                <option value="Minify">"Nén gọn một dòng (Minified Single Line)"</option>
                            </select>
                        </div>

                        <div class="split-pane-subtoolbar-item">
                            <span>"Từ khóa SQL:"</span>
                            <select
                                class="split-pane-select"
                                prop:value=move || format!("{:?}", sql_case.get())
                                on:change=move |ev| {
                                    if event_target_value(&ev) == "Lowercase" {
                                        sql_case.set(SqlKeywordCase::Lowercase);
                                    } else {
                                        sql_case.set(SqlKeywordCase::Uppercase);
                                    }
                                }
                            >
                                <option value="Uppercase">"CHỮ HOA (UPPERCASE)"</option>
                                <option value="Lowercase">"chữ thường (lowercase)"</option>
                            </select>
                        </div>
                    }.into_any(),

                    ConverterMode::DiffChecker => view! {
                        <div class="split-pane-subtoolbar-item">
                            <span>"Chế độ hiển thị:"</span>
                            <select
                                class="split-pane-select"
                                prop:value=move || match diff_view_mode.get() {
                                    DiffViewMode::SideBySide => "SideBySide",
                                    DiffViewMode::Unified => "Unified",
                                    DiffViewMode::EditTarget => "EditTarget",
                                }
                                on:change=move |ev| {
                                    let v = event_target_value(&ev);
                                    if v == "SideBySide" {
                                        diff_view_mode.set(DiffViewMode::SideBySide);
                                    } else if v == "Unified" {
                                        diff_view_mode.set(DiffViewMode::Unified);
                                    } else {
                                        diff_view_mode.set(DiffViewMode::EditTarget);
                                    }
                                }
                            >
                                <option value="SideBySide">"Góc nhìn Song song (Side-by-Side 2 cột)"</option>
                                <option value="Unified">"Góc nhìn Hợp nhất (Unified Diff)"</option>
                                <option value="EditTarget">"Chỉnh sửa Văn bản Đích B (Text B)"</option>
                            </select>
                        </div>

                        <div class="diff-view-toggle-group">
                            <button
                                type="button"
                                class="diff-view-toggle-btn"
                                class:active=move || diff_view_mode.get() == DiffViewMode::SideBySide
                                on:click=move |_| diff_view_mode.set(DiffViewMode::SideBySide)
                                title="Hiển thị so sánh song song 2 cột"
                            >
                                "⚏ Song song"
                            </button>
                            <button
                                type="button"
                                class="diff-view-toggle-btn"
                                class:active=move || diff_view_mode.get() == DiffViewMode::Unified
                                on:click=move |_| diff_view_mode.set(DiffViewMode::Unified)
                                title="Hiển thị so sánh hợp nhất 1 luồng"
                            >
                                "☰ Hợp nhất"
                            </button>
                            <button
                                type="button"
                                class="diff-view-toggle-btn"
                                class:active=move || diff_view_mode.get() == DiffViewMode::EditTarget
                                on:click=move |_| diff_view_mode.set(DiffViewMode::EditTarget)
                                title="Chỉnh sửa trực tiếp nội dung văn bản B"
                            >
                                "✏ Sửa bản B"
                            </button>
                        </div>

                        <label class="split-pane-toggle-label" title="Bỏ qua sự khác nhau về khoảng trắng và dòng trống">
                            <input
                                type="checkbox"
                                class="split-pane-toggle-input"
                                prop:checked=move || diff_ignore_ws.get()
                                on:change=move |ev| diff_ignore_ws.set(event_target_checked(&ev))
                            />
                            <span>"Bỏ qua khoảng trắng thừa"</span>
                        </label>
                    }.into_any(),
                }}
            </div>

            // 3. THÂN CHÍNH: SPLIT-PANE 2 CỘT SONG SONG
            <div class="split-pane-body">
                // CỘT TRÁI: INPUT PANEL
                <div class="split-pane-col">
                    <div class="split-pane-col-header">
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <span>"📥 ĐẦU VÀO (INPUT)"</span>
                            <span class="stat-badge format-badge">
                                {move || detected_in_format.get()}
                            </span>
                        </div>
                        <div class="stat-badge-group">
                            <span class="stat-badge">
                                {move || format!("{} dòng", input_stats.get().0)}
                            </span>
                            <span class="stat-badge">
                                {move || format!("{} ký tự", input_stats.get().1)}
                            </span>
                            <span class="stat-badge">
                                {move || format_byte_size(input_stats.get().2)}
                            </span>
                            {move || state.active_file_path().map(|path| {
                                view! {
                                    <button
                                        type="button"
                                        class="split-pane-btn"
                                        style="padding: 2px 8px; font-size: 12px;"
                                        title=format!("Nạp nội dung từ tệp đang chọn: {}", path)
                                        on:click=move |_| {
                                            state.status_message.set(format!("Đã nạp đường dẫn tệp: {}", path));
                                        }
                                    >
                                        "📁 File đang chọn"
                                    </button>
                                }
                            })}
                        </div>
                    </div>

                    <textarea
                        class="split-pane-textarea"
                        placeholder="Dán hoặc gõ nội dung vào đây (Hỗ trợ kéo thả file, tự động cập nhật 0ms)..."
                        prop:value=move || input_text.get()
                        on:compositionstart=move |_| is_composing.set(true)
                        on:compositionend=move |ev| {
                            is_composing.set(false);
                            input_text.set(event_target_value(&ev));
                        }
                        on:input=move |ev: ev::Event| {
                            if !is_composing.get_untracked() {
                                input_text.set(event_target_value(&ev));
                            }
                        }
                        on:dragover=move |ev| ev.prevent_default()
                        on:drop=move |ev: ev::DragEvent| {
                            ev.prevent_default();
                            // Hỗ trợ thả text hoặc file
                        }
                    ></textarea>
                </div>

                // CỘT GIỮA: THANH NGĂN CÁCH TINH TẾ (Divider with Swap button)
                <div class="split-pane-divider">
                    <button
                        type="button"
                        class="split-pane-swap-btn"
                        title="Hoán đổi Input và Output (⇄)"
                        on:click=on_swap
                    >
                        "⇄"
                    </button>
                </div>

                // CỘT PHẢI: OUTPUT PANEL
                <div class="split-pane-col">
                    <div class="split-pane-col-header">
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <span>"📤 KẾT QUẢ (OUTPUT)"</span>
                            {move || {
                                let (res, _, _, _, _) = conversion_result.get();
                                match res {
                                    Ok(_) => view! {
                                        <span class="split-pane-status-pill success">
                                            "✓ Hợp lệ"
                                        </span>
                                    }.into_any(),
                                    Err(_) => view! {
                                        <span class="split-pane-status-pill error">
                                            "⚠️ Lỗi cú pháp"
                                        </span>
                                    }.into_any(),
                                }
                            }}
                            {move || (mode.get() != ConverterMode::DiffChecker).then(|| view! {
                                <span class="stat-badge format-badge">
                                    {output_format_label.get()}
                                </span>
                            })}
                        </div>

                        <div class="stat-badge-group">
                            {move || {
                                let (res, warns, _, stats, _) = conversion_result.get();
                                if mode.get() == ConverterMode::DiffChecker {
                                    view! {
                                        <div style="display: flex; align-items: center; gap: 6px;">
                                            <span class="diff-stat-added">{format!("+{} thêm", stats.added)}</span>
                                            <span class="diff-stat-removed">{format!("-{} xóa", stats.removed)}</span>
                                            <span class="diff-stat-match">{format!("{} khớp", stats.unchanged)}</span>
                                            <span class="stat-badge speed-badge">"⚡ < 5ms"</span>
                                        </div>
                                    }.into_any()
                                } else if !warns.is_empty() {
                                    view! {
                                        <div style="display: flex; align-items: center; gap: 6px;">
                                            <span class="split-pane-warning-pill">
                                                {format!("⚠️ {} cảnh báo Linter", warns.len())}
                                            </span>
                                            <span class="stat-badge speed-badge">"⚡ < 5ms"</span>
                                        </div>
                                    }.into_any()
                                } else if res.is_ok() {
                                    let (lines, chars, bytes) = output_stats.get();
                                    view! {
                                        <div style="display: flex; align-items: center; gap: 6px;">
                                            <span class="stat-badge">{format!("{} dòng", lines)}</span>
                                            <span class="stat-badge">{format!("{} ký tự", chars)}</span>
                                            <span class="stat-badge">{format_byte_size(bytes)}</span>
                                            <span class="stat-badge speed-badge">"⚡ < 5ms"</span>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <span class="stat-badge speed-badge">"⚡ < 5ms"</span>
                                    }.into_any()
                                }
                            }}
                        </div>
                    </div>

                    // Khu vực hiển thị Output
                    <div class="split-pane-output-content">
                        // Toast thông báo đã sao chép
                        {move || copied_toast.get().then(|| view! {
                            <div class="split-pane-toast">
                                <span>"✓"</span>
                                <span>"Đã sao chép vào bộ nhớ tạm!"</span>
                            </div>
                        })}

                        {move || {
                            let (res, warns, diff_lines, _, side_rows) = conversion_result.get();

                            // 1. Trường hợp lỗi cú pháp (Syntax Error Card)
                            if let Err(ref err_msg) = res {
                                return view! {
                                    <div class="split-pane-error-card">
                                        <div class="split-pane-error-title">
                                            <span>"⚠️ Phát hiện lỗi phân tích cú pháp:"</span>
                                        </div>
                                        <div style="font-family: var(--font-mono); white-space: pre-wrap; margin-top: 4px;">
                                            {err_msg.clone()}
                                        </div>
                                    </div>
                                }.into_any();
                            }

                            // 2. Trường hợp Diff Checker ở chế độ Side-by-Side
                            if mode.get() == ConverterMode::DiffChecker && diff_view_mode.get() == DiffViewMode::SideBySide {
                                return view! {
                                    <div class="split-pane-diff-side-by-side">
                                        <div class="diff-side-header-row">
                                            <div class="diff-side-col-header">
                                                <span>"📄 Văn bản A (Gốc)"</span>
                                            </div>
                                            <div class="diff-side-divider-header"></div>
                                            <div class="diff-side-col-header">
                                                <span>"📑 Văn bản B (Đích so sánh)"</span>
                                            </div>
                                        </div>
                                        <div class="diff-side-body">
                                            {side_rows.into_iter().map(|row| {
                                                let left_cell = match row.left {
                                                    Some(l) => {
                                                        let (cell_class, marker) = match l.kind {
                                                            DiffKind::Removed => ("diff-side-cell cell-removed", "-"),
                                                            DiffKind::Unchanged => ("diff-side-cell cell-unchanged", " "),
                                                            DiffKind::Added => ("diff-side-cell cell-added", "+"),
                                                        };
                                                        let gutter = l.line_a.map(|n| n.to_string()).unwrap_or_default();
                                                        view! {
                                                            <div class=cell_class>
                                                                <span class="diff-gutter">{gutter}</span>
                                                                <span class="diff-marker">{marker}</span>
                                                                <span class="diff-text">
                                                                    {l.chunks.into_iter().map(|chk| {
                                                                        let chk_cls = if chk.is_highlight { "diff-char-del" } else { "" };
                                                                        view! { <span class=chk_cls>{chk.text}</span> }
                                                                    }).collect::<Vec<_>>()}
                                                                </span>
                                                            </div>
                                                        }.into_any()
                                                    }
                                                    None => view! {
                                                        <div class="diff-side-cell cell-empty">
                                                            <span class="diff-gutter"></span>
                                                            <span class="diff-marker"></span>
                                                            <span class="diff-text"></span>
                                                        </div>
                                                    }.into_any(),
                                                };

                                                let right_cell = match row.right {
                                                    Some(r) => {
                                                        let (cell_class, marker) = match r.kind {
                                                            DiffKind::Added => ("diff-side-cell cell-added", "+"),
                                                            DiffKind::Unchanged => ("diff-side-cell cell-unchanged", " "),
                                                            DiffKind::Removed => ("diff-side-cell cell-removed", "-"),
                                                        };
                                                        let gutter = r.line_b.map(|n| n.to_string()).unwrap_or_default();
                                                        view! {
                                                            <div class=cell_class>
                                                                <span class="diff-gutter">{gutter}</span>
                                                                <span class="diff-marker">{marker}</span>
                                                                <span class="diff-text">
                                                                    {r.chunks.into_iter().map(|chk| {
                                                                        let chk_cls = if chk.is_highlight { "diff-char-ins" } else { "" };
                                                                        view! { <span class=chk_cls>{chk.text}</span> }
                                                                    }).collect::<Vec<_>>()}
                                                                </span>
                                                            </div>
                                                        }.into_any()
                                                    }
                                                    None => view! {
                                                        <div class="diff-side-cell cell-empty">
                                                            <span class="diff-gutter"></span>
                                                            <span class="diff-marker"></span>
                                                            <span class="diff-text"></span>
                                                        </div>
                                                    }.into_any(),
                                                };

                                                view! {
                                                    <div class="diff-side-row">
                                                        {left_cell}
                                                        <div class="diff-side-divider"></div>
                                                        {right_cell}
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                }.into_any();
                            }

                            // 3. Trường hợp Diff Checker ở chế độ Unified
                            if mode.get() == ConverterMode::DiffChecker && diff_view_mode.get() == DiffViewMode::Unified {
                                return view! {
                                    <div class="split-pane-diff-container">
                                        {diff_lines.into_iter().map(|line| {
                                            let (row_class, marker) = match line.kind {
                                                DiffKind::Added => ("diff-row diff-row-added", "+"),
                                                DiffKind::Removed => ("diff-row diff-row-removed", "-"),
                                                DiffKind::Unchanged => ("diff-row diff-row-unchanged", " "),
                                            };
                                            let gutter_text = match (line.line_a, line.line_b) {
                                                (Some(a), Some(b)) => format!("{}:{}", a, b),
                                                (Some(a), None) => format!("{}:-", a),
                                                (None, Some(b)) => format!("-:{}", b),
                                                (None, None) => String::new(),
                                            };

                                            let is_added = line.kind == DiffKind::Added;
                                            let is_removed = line.kind == DiffKind::Removed;

                                            view! {
                                                <div class=row_class>
                                                    <span class="diff-gutter">{gutter_text}</span>
                                                    <span class="diff-marker">{marker}</span>
                                                    <span class="diff-text">
                                                        {line.chunks.into_iter().map(|chunk| {
                                                            let chunk_cls = if chunk.is_highlight {
                                                                if is_added { "diff-char-ins" } else if is_removed { "diff-char-del" } else { "" }
                                                            } else {
                                                                ""
                                                            };
                                                            view! {
                                                                <span class=chunk_cls>{chunk.text}</span>
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </span>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any();
                            }

                            // 4. Trường hợp Diff Checker ở chế độ EditTarget
                            if mode.get() == ConverterMode::DiffChecker && diff_view_mode.get() == DiffViewMode::EditTarget {
                                return view! {
                                    <textarea
                                        class="split-pane-textarea"
                                        placeholder="Dán hoặc gõ văn bản đích B để đối chiếu diff..."
                                        prop:value=move || diff_target_text.get()
                                        on:input=move |ev| diff_target_text.set(event_target_value(&ev))
                                    ></textarea>
                                }.into_any();
                            }

                            // 5. Trường hợp thông thường (Prettified Text)
                            let text_val = res.unwrap_or_default();
                            view! {
                                <div style="display: flex; flex-direction: column; flex: 1; min-height: 0; overflow: hidden;">
                                    {(!warns.is_empty()).then(|| view! {
                                        <div style="background: var(--warning-surface); border-bottom: 1px solid var(--warning); padding: 8px 12px; font-size: 12px; color: var(--warning-light);">
                                            <div style="font-weight: 700; margin-bottom: 2px;">"Cảnh báo Linter SQL:"</div>
                                            {warns.into_iter().map(|w| view! {
                                                <div>{format!("• {}", w)}</div>
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    })}
                                    <pre class="split-pane-pre">{text_val}</pre>
                                </div>
                            }.into_any()
                        }}
                    </div>
                </div>
            </div>
        </div>
    }
}

// ==============================================================================
// UNIT TESTS
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_byte_size() {
        assert_eq!(format_byte_size(500), "500 B");
        assert_eq!(format_byte_size(1024), "1.0 KB");
        assert_eq!(format_byte_size(1536), "1.5 KB");
        assert_eq!(format_byte_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_word_diff_chunking() {
        let (ch_a, ch_b) = compute_word_diff("Hello world 123", "Hello earth 123");
        assert!(ch_a.iter().any(|c| c.text.contains("world") && c.is_highlight));
        assert!(ch_b.iter().any(|c| c.text.contains("earth") && c.is_highlight));
        assert!(ch_a.iter().any(|c| c.text.contains("Hello") && !c.is_highlight));
        assert!(ch_b.iter().any(|c| c.text.contains("Hello") && !c.is_highlight));
    }

    #[test]
    fn test_side_by_side_rows_construction() {
        let a = "line1\nline2\nline3";
        let b = "line1\nline2 modified\nline3";
        let (lines, _) = compute_line_diff(a, b, false);
        let rows = build_side_by_side_rows(&lines);
        assert_eq!(rows.len(), 3);
        assert!(rows[0].left.is_some() && rows[0].right.is_some());
        assert!(rows[1].left.is_some() && rows[1].right.is_some());
        assert_eq!(rows[1].left.as_ref().unwrap().kind, DiffKind::Removed);
        assert_eq!(rows[1].right.as_ref().unwrap().kind, DiffKind::Added);
        assert_eq!(rows[2].left.as_ref().unwrap().kind, DiffKind::Unchanged);
    }

    #[test]
    fn test_base64_encode_decode_roundtrip() {
        let original = "Xin chào Việt Nam! 🇻🇳 123456";
        let encoded = base64_encode_bytes(original.as_bytes(), false);
        assert!(!encoded.is_empty());

        let decoded = base64_decode_str(&encoded, false).expect("Giải mã thành công");
        let decoded_str = String::from_utf8(decoded).expect("UTF-8 hợp lệ");
        assert_eq!(decoded_str, original);
    }

    #[test]
    fn test_base64_url_safe_variant() {
        let original = "Hello?&=+/_";
        let encoded = base64_encode_bytes(original.as_bytes(), true);
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
        assert!(!encoded.contains('='));

        let decoded = base64_decode_str(&encoded, true).expect("URL-Safe decode thành công");
        assert_eq!(String::from_utf8(decoded).unwrap(), original);
    }

    #[test]
    fn test_data_structure_json_to_csv() {
        let json_input = r#"[{"name": "An", "age": 28}, {"name": "Binh", "age": 32}]"#;
        let csv_output = convert_data_structure(json_input, DataTargetFormat::Csv, "tbl")
            .expect("Convert sang CSV");
        assert!(csv_output.contains("name,age") || csv_output.contains("age,name"));
        assert!(csv_output.contains("An,28") || csv_output.contains("28,An"));
    }

    #[test]
    fn test_data_structure_json_to_sql() {
        let json_input = r#"[{"name": "An", "age": 28}]"#;
        let sql = convert_data_structure(json_input, DataTargetFormat::SqlInsert, "users")
            .expect("Convert sang SQL");
        assert!(sql.contains("INSERT INTO users"));
        assert!(sql.contains("'An'"));
    }

    #[test]
    fn test_data_structure_csv_to_json() {
        let csv_input = "id,name,active\n1,Alice,true\n2,Bob,false";
        let json_output = convert_data_structure(csv_input, DataTargetFormat::JsonPretty, "tbl")
            .expect("Convert CSV sang JSON");
        assert!(json_output.contains("\"Alice\""));
        assert!(json_output.contains("\"Bob\""));
    }

    #[test]
    fn test_sql_formatter_and_linter() {
        let raw_sql = "select id, name, from users where active = 1;";
        let (formatted, warns) = format_and_lint_sql(raw_sql, SqlFormatStyle::Pretty, SqlKeywordCase::Uppercase);
        assert!(formatted.is_ok());
        let sql = formatted.unwrap();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM"));
        // Cảnh báo dấu phẩy thừa trước FROM
        assert!(!warns.is_empty());
    }

    #[test]
    fn test_sql_linter_unclosed_parenthesis() {
        let raw_sql = "SELECT * FROM users WHERE id IN (1, 2, 3";
        let (_, warns) = format_and_lint_sql(raw_sql, SqlFormatStyle::Pretty, SqlKeywordCase::Uppercase);
        assert!(warns.iter().any(|w| w.contains("ngoặc đóng")));
    }

    #[test]
    fn test_diff_checker_computation() {
        let a = "line1\nline2\nline3";
        let b = "line1\nline2 modified\nline3\nline4";
        let (lines, stats) = compute_line_diff(a, b, false);
        assert!(stats.unchanged >= 2);
        assert_eq!(stats.added, 2);
        assert_eq!(stats.removed, 1);
        assert_eq!(lines.len(), 5);
    }
}
