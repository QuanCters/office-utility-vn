use std::sync::atomic::{AtomicBool, Ordering};
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Phương ngữ tiếng Việt khi phát âm số
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Dialect {
    /// Chuẩn miền Bắc: "nghìn", "lẻ", "mốt", "tư"
    #[default]
    North,
    /// Chuẩn miền Nam: "ngàn", "linh", "tư" hoặc "bốn"
    South,
}

/// Đơn vị tiền tệ
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurrencyUnit {
    /// Việt Nam Đồng: "đồng", "đồng chẵn"
    VND,
    /// Đô la Mỹ: "đô la Mỹ", "xu"
    USD,
    /// Euro: "euro", "cents"
    EUR,
    /// Đơn vị tùy chỉnh (ví dụ: "lượng vàng", "mét khối")
    Custom {
        main: String,
        sub: Option<String>,
    },
    /// Không kèm đơn vị tiền tệ (đọc số thuần túy)
    None,
}

impl Default for CurrencyUnit {
    fn default() -> Self {
        Self::VND
    }
}

/// Cấu hình tham số cho công cụ Đọc số thành chữ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumberToWordsConfig {
    #[serde(default)]
    pub dialect: Dialect,
    #[serde(default)]
    pub currency: CurrencyUnit,
    #[serde(default)]
    pub use_dong_chan: bool,
    #[serde(default)]
    pub capitalize_first: bool,
    #[serde(default = "default_negative_prefix")]
    pub negative_prefix: String,
}

fn default_negative_prefix() -> String {
    "âm".to_string()
}

impl Default for NumberToWordsConfig {
    fn default() -> Self {
        Self {
            dialect: Dialect::North,
            currency: CurrencyUnit::VND,
            use_dong_chan: false,
            capitalize_first: false,
            negative_prefix: default_negative_prefix(),
        }
    }
}

/// Đọc một bộ 3 chữ số thành chữ
fn read_three_digits(digits: [u8; 3], is_highest_group: bool, dialect: Dialect) -> Vec<String> {
    let [h, t, u] = digits;
    let mut words = Vec::new();

    let le_linh = match dialect {
        Dialect::North => "lẻ",
        Dialect::South => "linh",
    };

    // Đọc hàng trăm
    if !is_highest_group || h > 0 {
        words.push(digit_to_word(h).to_string());
        words.push("trăm".to_string());
    }

    // Đọc hàng chục
    if t == 0 {
        if u > 0 && (!is_highest_group || h > 0) {
            words.push(le_linh.to_string());
        }
    } else if t == 1 {
        words.push("mười".to_string());
    } else {
        words.push(digit_to_word(t).to_string());
        words.push("mươi".to_string());
    }

    // Đọc hàng đơn vị
    if u > 0 || (is_highest_group && h == 0 && t == 0) {
        if t == 0 {
            words.push(digit_to_word(u).to_string());
        } else if t == 1 {
            if u == 5 {
                words.push("lăm".to_string());
            } else {
                words.push(digit_to_word(u).to_string());
            }
        } else {
            // t >= 2
            if u == 1 {
                words.push("mốt".to_string());
            } else if u == 4 {
                match dialect {
                    Dialect::North => words.push("tư".to_string()),
                    Dialect::South => words.push("bốn".to_string()),
                }
            } else if u == 5 {
                words.push("lăm".to_string());
            } else {
                words.push(digit_to_word(u).to_string());
            }
        }
    }

    words
}

fn digit_to_word(d: u8) -> &'static str {
    match d {
        0 => "không",
        1 => "một",
        2 => "hai",
        3 => "ba",
        4 => "bốn",
        5 => "năm",
        6 => "sáu",
        7 => "bảy",
        8 => "tám",
        9 => "chín",
        _ => "",
    }
}

/// Phân tách chuỗi số lớn thành các nhóm 3 chữ số từ phải sang trái
fn split_into_groups(s: &str) -> Vec<[u8; 3]> {
    let bytes = s.as_bytes();
    let mut groups = Vec::new();
    let mut i = bytes.len();

    while i > 0 {
        let start = if i >= 3 { i - 3 } else { 0 };
        let slice = &bytes[start..i];
        let mut group = [0u8; 3];
        if slice.len() == 3 {
            group[0] = slice[0] - b'0';
            group[1] = slice[1] - b'0';
            group[2] = slice[2] - b'0';
        } else if slice.len() == 2 {
            group[0] = 0;
            group[1] = slice[0] - b'0';
            group[2] = slice[1] - b'0';
        } else if slice.len() == 1 {
            group[0] = 0;
            group[1] = 0;
            group[2] = slice[0] - b'0';
        }
        groups.push(group);
        i = start;
    }

    groups
}

/// Format số nguyên thành chữ tiếng Việt
fn format_integer_part(s: &str, dialect: Dialect) -> String {
    let clean = s.trim_start_matches('0');
    if clean.is_empty() {
        return "không".to_string();
    }

    let groups = split_into_groups(clean);
    let n = groups.len();

    let nghin_ngan = match dialect {
        Dialect::North => "nghìn",
        Dialect::South => "ngàn",
    };

    let scale_names = ["", nghin_ngan, "triệu"];

    let mut result_words = Vec::new();

    for (idx, group) in groups.iter().enumerate().rev() {
        let is_highest = idx == n - 1;
        let is_all_zero = group[0] == 0 && group[1] == 0 && group[2] == 0;

        if !is_all_zero {
            let group_words = read_three_digits(*group, is_highest, dialect);
            result_words.extend(group_words);

            let scale_idx = idx % 3;
            if scale_idx > 0 {
                result_words.push(scale_names[scale_idx].to_string());
            }
        }

        if idx > 0 && idx % 3 == 0 {
            // Kiểm tra xem có cần xuất "tỷ"
            let has_higher_in_block = (idx..n).any(|i| {
                let g = groups[i];
                g[0] > 0 || g[1] > 0 || g[2] > 0
            });
            if has_higher_in_block {
                result_words.push("tỷ".to_string());
            }
        }
    }

    result_words.join(" ")
}

/// Hàm chính: Đọc chuỗi số thành chữ tiếng Việt theo chuẩn kế toán và ngữ pháp
pub fn format_vietnamese_number(
    input: &str,
    config: &NumberToWordsConfig,
) -> Result<String, CommandError> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(CommandError::ValidationFailed {
            reason: "Chuỗi số tiền không được để trống".to_string(),
        });
    }

    let is_negative = raw.starts_with('-');
    let unsigned_raw = if is_negative || raw.starts_with('+') {
        &raw[1..]
    } else {
        raw
    };

    if unsigned_raw.is_empty() {
        return Err(CommandError::ValidationFailed {
            reason: "Không có chữ số hợp lệ".to_string(),
        });
    }

    // Kiểm tra ký tự không hợp lệ
    for c in unsigned_raw.chars() {
        if !c.is_ascii_digit() && c != '.' && c != ',' {
            return Err(CommandError::ValidationFailed {
                reason: format!("Ký tự không hợp lệ: '{c}'"),
            });
        }
    }

    // Đếm dấu chấm và dấu phẩy
    let dot_count = unsigned_raw.chars().filter(|&c| c == '.').count();
    let comma_count = unsigned_raw.chars().filter(|&c| c == ',').count();

    if dot_count > 1 && comma_count > 1 {
        return Err(CommandError::ValidationFailed {
            reason: "Quá nhiều dấu phân cách".to_string(),
        });
    }

    let (integer_str, decimal_str) = if comma_count == 1 && dot_count <= 1 {
        // Định dạng 125,50 hoặc 1.250,50
        let parts: Vec<&str> = unsigned_raw.split(',').collect();
        let int_clean = parts[0].replace('.', "");
        (int_clean, Some(parts[1].to_string()))
    } else if dot_count == 1 && comma_count == 0 {
        // Có 1 dấu chấm: Kiểm tra xem là phân cách hàng nghìn (1.000) hay số thập phân (125.50)
        let parts: Vec<&str> = unsigned_raw.split('.').collect();
        if parts[1].len() == 3 && parts[0].len() <= 3 && config.currency == CurrencyUnit::VND {
            // Có khả năng là phân cách hàng nghìn kiểu 1.000 hoặc 250.000
            (unsigned_raw.replace('.', ""), None)
        } else {
            (parts[0].to_string(), Some(parts[1].to_string()))
        }
    } else if dot_count > 1 && comma_count == 0 {
        // Nhiều dấu chấm: phân cách hàng nghìn kiểu VN (1.250.000)
        (unsigned_raw.replace('.', ""), None)
    } else if comma_count > 1 && dot_count == 0 {
        // Nhiều dấu phẩy: phân cách hàng nghìn kiểu Mỹ (1,250,000)
        (unsigned_raw.replace(',', ""), None)
    } else {
        (unsigned_raw.to_string(), None)
    };

    if integer_str.is_empty() || !integer_str.chars().all(|c| c.is_ascii_digit()) {
        return Err(CommandError::ValidationFailed {
            reason: "Phần nguyên không hợp lệ".to_string(),
        });
    }

    let int_words = format_integer_part(&integer_str, config.dialect);

    let mut result = int_words;

    // Gắn tiền tệ chính
    match &config.currency {
        CurrencyUnit::VND => {
            result.push_str(" đồng");
            if config.use_dong_chan && decimal_str.is_none() {
                result.push_str(" chẵn");
            }
        }
        CurrencyUnit::USD => {
            result.push_str(" đô la Mỹ");
        }
        CurrencyUnit::EUR => {
            result.push_str(" euro");
        }
        CurrencyUnit::Custom { main, .. } => {
            result.push(' ');
            result.push_str(main);
        }
        CurrencyUnit::None => {}
    }

    // Đọc phần thập phân nếu có
    if let Some(dec) = decimal_str {
        let dec_clean = dec.trim_end_matches('0');
        if !dec_clean.is_empty() {
            let dec_val: u32 = dec.parse().unwrap_or(0);
            if dec_val > 0 {
                let dec_words = format_integer_part(&dec, config.dialect);
                result.push_str(" và ");
                result.push_str(&dec_words);

                match &config.currency {
                    CurrencyUnit::USD => result.push_str(" xu"),
                    CurrencyUnit::EUR => result.push_str(" cents"),
                    CurrencyUnit::Custom { sub: Some(s), .. } => {
                        result.push(' ');
                        result.push_str(s);
                    }
                    _ => {}
                }
            }
        }
    }

    // Tiền tố số âm
    if is_negative {
        result = format!("{} {}", config.negative_prefix, result);
    }

    // Viết hoa chữ cái đầu tiên
    if config.capitalize_first {
        let mut chars = result.chars();
        if let Some(first) = chars.next() {
            result = first.to_uppercase().collect::<String>() + chars.as_str();
        }
    }

    Ok(result)
}

/// Công cụ `FR-VNO-01` thực thi trait `OfficeTool`
#[derive(Default)]
pub struct VietnameseNumberToWordsTool;

impl OfficeTool for VietnameseNumberToWordsTool {
    fn slug(&self) -> &'static str {
        "vietnamese-number-to-words"
    }

    fn name(&self) -> &'static str {
        "Đọc Số Tiền Thành Chữ Chuẩn"
    }

    fn description(&self) -> &'static str {
        "Chuyển đổi số tiền hoặc số bất kỳ thành chữ tiếng Việt chuẩn xác theo quy tắc kế toán và ngữ pháp."
    }

    fn execute(
        &self,
        request: &DispatchTaskRequest,
        cancel_flag: &AtomicBool,
        on_progress: Option<&dyn Fn(TaskProgress)>,
    ) -> Result<DispatchTaskResponse, CommandError> {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let options: NumberToWordsConfig = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn không hợp lệ: {e}"),
            })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 1,
                percentage: 50.0,
                message: "Đang chuyển đổi số thành chữ...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 0,
                is_completed: false,
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        // Lấy số cần đọc từ options hoặc trường phụ
        let number_str = request
            .options
            .get("number")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        let words = format_vietnamese_number(number_str, &options)?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 1,
                percentage: 100.0,
                message: "Hoàn tất".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths: vec![words],
            failed_items: vec![],
            elapsed_ms: 1,
        })
    }
}

#[cfg(test)]
mod vietnamese_number_to_words_tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_single_digits_and_zero() {
        let config = NumberToWordsConfig {
            dialect: Dialect::North,
            currency: CurrencyUnit::VND,
            use_dong_chan: false,
            capitalize_first: false,
            negative_prefix: "âm".to_string(),
        };

        assert_eq!(format_vietnamese_number("0", &config).unwrap(), "không đồng");
        assert_eq!(format_vietnamese_number("1", &config).unwrap(), "một đồng");
        assert_eq!(format_vietnamese_number("5", &config).unwrap(), "năm đồng");
    }

    #[test]
    fn test_tens_and_special_pronunciations() {
        let north_config = NumberToWordsConfig {
            dialect: Dialect::North,
            currency: CurrencyUnit::None,
            use_dong_chan: false,
            capitalize_first: false,
            negative_prefix: "âm".to_string(),
        };

        assert_eq!(format_vietnamese_number("10", &north_config).unwrap(), "mười");
        assert_eq!(format_vietnamese_number("11", &north_config).unwrap(), "mười một");
        assert_eq!(format_vietnamese_number("14", &north_config).unwrap(), "mười bốn");
        assert_eq!(format_vietnamese_number("15", &north_config).unwrap(), "mười lăm");

        assert_eq!(format_vietnamese_number("20", &north_config).unwrap(), "hai mươi");
        assert_eq!(format_vietnamese_number("21", &north_config).unwrap(), "hai mươi mốt");
        assert_eq!(format_vietnamese_number("24", &north_config).unwrap(), "hai mươi tư");
        assert_eq!(format_vietnamese_number("25", &north_config).unwrap(), "hai mươi lăm");
        assert_eq!(format_vietnamese_number("55", &north_config).unwrap(), "năm mươi lăm");

        let south_config = NumberToWordsConfig {
            dialect: Dialect::South,
            currency: CurrencyUnit::None,
            use_dong_chan: false,
            capitalize_first: false,
            negative_prefix: "âm".to_string(),
        };
        assert_eq!(format_vietnamese_number("24", &south_config).unwrap(), "hai mươi bốn");
    }

    #[test]
    fn test_hundreds_le_and_linh() {
        let north_config = NumberToWordsConfig {
            dialect: Dialect::North,
            currency: CurrencyUnit::None,
            use_dong_chan: false,
            capitalize_first: false,
            negative_prefix: "âm".to_string(),
        };

        assert_eq!(format_vietnamese_number("100", &north_config).unwrap(), "một trăm");
        assert_eq!(format_vietnamese_number("101", &north_config).unwrap(), "một trăm lẻ một");
        assert_eq!(format_vietnamese_number("105", &north_config).unwrap(), "một trăm lẻ năm");
        assert_eq!(format_vietnamese_number("110", &north_config).unwrap(), "một trăm mười");
        assert_eq!(format_vietnamese_number("115", &north_config).unwrap(), "một trăm mười lăm");

        let south_config = NumberToWordsConfig {
            dialect: Dialect::South,
            currency: CurrencyUnit::None,
            use_dong_chan: false,
            capitalize_first: false,
            negative_prefix: "âm".to_string(),
        };
        assert_eq!(format_vietnamese_number("101", &south_config).unwrap(), "một trăm linh một");
        assert_eq!(format_vietnamese_number("105", &south_config).unwrap(), "một trăm linh năm");
    }

    #[test]
    fn test_thousands_millions_billions() {
        let north_config = NumberToWordsConfig {
            dialect: Dialect::North,
            currency: CurrencyUnit::VND,
            use_dong_chan: false,
            capitalize_first: false,
            negative_prefix: "âm".to_string(),
        };

        assert_eq!(format_vietnamese_number("1000", &north_config).unwrap(), "một nghìn đồng");
        assert_eq!(
            format_vietnamese_number("1005", &north_config).unwrap(),
            "một nghìn không trăm lẻ năm đồng"
        );
        assert_eq!(format_vietnamese_number("1000000", &north_config).unwrap(), "một triệu đồng");
        assert_eq!(format_vietnamese_number("1000000000", &north_config).unwrap(), "một tỷ đồng");
        assert_eq!(
            format_vietnamese_number("1000000000000", &north_config).unwrap(),
            "một nghìn tỷ đồng"
        );
        assert_eq!(
            format_vietnamese_number("1000000000000000", &north_config).unwrap(),
            "một triệu tỷ đồng"
        );
        assert_eq!(
            format_vietnamese_number("1000000000000000000", &north_config).unwrap(),
            "một tỷ tỷ đồng"
        );

        let south_config = NumberToWordsConfig {
            dialect: Dialect::South,
            currency: CurrencyUnit::VND,
            use_dong_chan: false,
            capitalize_first: false,
            negative_prefix: "âm".to_string(),
        };
        assert_eq!(format_vietnamese_number("1000", &south_config).unwrap(), "một ngàn đồng");
        assert_eq!(
            format_vietnamese_number("1005", &south_config).unwrap(),
            "một ngàn không trăm linh năm đồng"
        );
        assert_eq!(
            format_vietnamese_number("1000000000000", &south_config).unwrap(),
            "một ngàn tỷ đồng"
        );
    }

    #[test]
    fn test_arbitrary_large_numbers_no_overflow() {
        let config = NumberToWordsConfig {
            dialect: Dialect::North,
            currency: CurrencyUnit::VND,
            use_dong_chan: false,
            capitalize_first: true,
            negative_prefix: "âm".to_string(),
        };

        let huge_number = "1000000000000000000000000000000000000"; // 37 chữ số
        let result = format_vietnamese_number(huge_number, &config);
        assert!(result.is_ok(), "Không được tràn số hoặc panic với số cực lớn");
    }

    #[test]
    fn test_accounting_suffix_dong_chan_and_capitalization() {
        let config = NumberToWordsConfig {
            dialect: Dialect::North,
            currency: CurrencyUnit::VND,
            use_dong_chan: true,
            capitalize_first: true,
            negative_prefix: "âm".to_string(),
        };

        assert_eq!(
            format_vietnamese_number("100000", &config).unwrap(),
            "Một trăm nghìn đồng chẵn"
        );

        assert_eq!(
            format_vietnamese_number("1.250.000", &config).unwrap(),
            "Một triệu hai trăm năm mươi nghìn đồng chẵn"
        );
    }

    #[test]
    fn test_negative_amounts() {
        let config = NumberToWordsConfig {
            dialect: Dialect::North,
            currency: CurrencyUnit::VND,
            use_dong_chan: true,
            capitalize_first: true,
            negative_prefix: "âm".to_string(),
        };

        assert_eq!(
            format_vietnamese_number("-50000", &config).unwrap(),
            "Âm năm mươi nghìn đồng chẵn"
        );
    }

    #[test]
    fn test_decimal_foreign_currencies() {
        let usd_config = NumberToWordsConfig {
            dialect: Dialect::North,
            currency: CurrencyUnit::USD,
            use_dong_chan: false,
            capitalize_first: true,
            negative_prefix: "âm".to_string(),
        };

        assert_eq!(
            format_vietnamese_number("125.50", &usd_config).unwrap(),
            "Một trăm hai mươi lăm đô la Mỹ và năm mươi xu"
        );
        assert_eq!(
            format_vietnamese_number("125,50", &usd_config).unwrap(),
            "Một trăm hai mươi lăm đô la Mỹ và năm mươi xu"
        );
    }

    #[test]
    fn test_invalid_input_validation_failed() {
        let config = NumberToWordsConfig::default();

        assert!(matches!(
            format_vietnamese_number("", &config),
            Err(CommandError::ValidationFailed { .. })
        ));

        assert!(matches!(
            format_vietnamese_number("abc", &config),
            Err(CommandError::ValidationFailed { .. })
        ));
        assert!(matches!(
            format_vietnamese_number("123a456", &config),
            Err(CommandError::ValidationFailed { .. })
        ));

        assert!(matches!(
            format_vietnamese_number("1.2.3.4,5,6", &config),
            Err(CommandError::ValidationFailed { .. })
        ));
    }

    #[test]
    fn test_office_tool_metadata() {
        let tool = VietnameseNumberToWordsTool::default();
        assert_eq!(tool.slug(), "vietnamese-number-to-words");
        assert_eq!(tool.name(), "Đọc Số Tiền Thành Chữ Chuẩn");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_task_cancellation_under_100ms() {
        let tool = VietnameseNumberToWordsTool::default();
        let cancel_flag = AtomicBool::new(true);

        let request = DispatchTaskRequest {
            task_id: "test-task-cancel-1".to_string(),
            tool_slug: "vietnamese-number-to-words".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: serde_json::to_value(NumberToWordsConfig::default()).unwrap(),
        };

        let start = std::time::Instant::now();
        let result = tool.execute(&request, &cancel_flag, None);
        let elapsed = start.elapsed();

        assert!(
            matches!(result, Err(CommandError::TaskCancelled)),
            "Phải trả về Err(TaskCancelled) khi cancel_flag bật"
        );
        assert!(
            elapsed.as_millis() < 100,
            "Hủy tác vụ phải hoàn tất dưới 100ms (NFR-REL-01, NFR-PERF-02)"
        );
    }
}
