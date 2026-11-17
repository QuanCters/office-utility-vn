use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Cấu hình tùy chọn cho Bác sĩ Bảng mã & Khôi phục Lỗi chữ Tiếng Việt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VnFontEncodingDocOptions {
    #[serde(default = "default_true")]
    pub auto_detect: bool,
    #[serde(default)]
    pub target_font_serif: Option<String>,
    #[serde(default)]
    pub target_font_sans: Option<String>,
}

impl Default for VnFontEncodingDocOptions {
    fn default() -> Self {
        Self {
            auto_detect: true,
            target_font_serif: Some("Times New Roman".to_string()),
            target_font_sans: Some("Arial".to_string()),
        }
    }
}

fn default_true() -> bool {
    true
}

pub struct VnFontEncodingDocTool;

impl Default for VnFontEncodingDocTool {
    fn default() -> Self {
        Self
    }
}

// ============================================================================
// 1. VIETNAMESE SYLLABLE SCORER & PHONOTACTICS ENGINE
// ============================================================================

/// Kiểm tra ký tự thuộc bảng chữ cái tiếng Việt hợp lệ (Unicode NFC)
fn is_vietnamese_char(c: char) -> bool {
    c.is_ascii_alphabetic()
        || matches!(
            c,
            'đ' | 'Đ'
                | 'ă' | 'ằ' | 'ắ' | 'ẳ' | 'ẵ' | 'ặ' | 'Ă' | 'Ằ' | 'Ắ' | 'Ẳ' | 'Ẵ' | 'Ặ'
                | 'â' | 'ầ' | 'ấ' | 'ẩ' | 'ẫ' | 'ậ' | 'Â' | 'Ầ' | 'Ấ' | 'Ẩ' | 'Ẫ' | 'Ậ'
                | 'ê' | 'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' | 'Ê' | 'Ề' | 'Ế' | 'Ể' | 'Ễ' | 'Ệ'
                | 'ô' | 'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' | 'Ô' | 'Ồ' | 'Ố' | 'Ổ' | 'Ỗ' | 'Ộ'
                | 'ơ' | 'ờ' | 'ớ' | 'ở' | 'ỡ' | 'ợ' | 'Ơ' | 'Ờ' | 'Ớ' | 'Ở' | 'Ỡ' | 'Ợ'
                | 'ư' | 'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' | 'Ư' | 'Ừ' | 'Ứ' | 'Ử' | 'Ữ' | 'Ự'
                | 'à' | 'á' | 'ả' | 'ã' | 'ạ' | 'À' | 'Á' | 'Ả' | 'Ã' | 'Ạ'
                | 'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' | 'È' | 'É' | 'Ẻ' | 'Ẽ' | 'Ẹ'
                | 'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' | 'Ì' | 'Í' | 'Ỉ' | 'Ĩ' | 'Ị'
                | 'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' | 'Ò' | 'Ó' | 'Ỏ' | 'Õ' | 'Ọ'
                | 'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'Ù' | 'Ú' | 'Ủ' | 'Ũ' | 'Ụ'
                | 'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' | 'Ỳ' | 'Ý' | 'Ỷ' | 'Ỹ' | 'Ỵ'
        )
}

/// Kiểm tra ký tự dị dạng / Mojibake / TCVN3 ngoài bảng Unicode tiếng Việt chuẩn
fn is_corrupted_char(c: char) -> bool {
    matches!(
        c,
        '»' | '™' | '²' | '£' | '§' | '©' | '‡' | 'ß' | '·' | 'µ' | '¶' | '¸' | '¹' | '¨'
            | '¼' | '½' | '¾' | 'Æ' | 'Ç' | 'È' | 'Ë' | 'Ì' | 'Î' | 'Ï' | 'Ð' | 'Ñ' | 'ª'
            | 'Ò' | 'Ô' | 'Õ' | 'Ö' | '×' | 'Ø' | 'Ù' | 'Ü' | 'Ý' | 'Þ' | 'ä' | 'å' | 'æ'
            | 'ç' | 'ë' | 'î' | 'ï' | 'ð' | 'ñ' | 'ö' | '÷' | 'ø' | 'û' | 'ü' | 'ÿ'
    )
}

/// Kiểm tra từ có chứa các cặp tổ hợp đặc thù của lỗi mã VNI
fn has_vni_corrupted_patterns(s: &str) -> bool {
    let vni_patterns = [
        "aø", "aù", "aû", "aõ", "aï", "eø", "eù", "eû", "eõ", "eï",
        "iø", "iù", "iû", "iõ", "iï", "oø", "où", "oû", "oõ", "oï",
        "uø", "uù", "uû", "uõ", "uï", "yø", "yù", "yû", "yõ",
        "aê", "aè", "aé", "aú", "aü", "aë",
        "aâ", "aà", "aá", "aå", "aã", "aä",
        "eâ", "eà", "eá", "eå", "eã", "eä",
        "oâ", "oà", "oá", "oå", "oã", "oä",
        "ôø", "ôù", "ôû", "ôõ", "ôï",
        "öø", "öù", "öû", "öõ", "öï",
    ];
    for &pat in &vni_patterns {
        if s.contains(pat) {
            return true;
        }
    }
    false
}

/// Danh sách các âm tiết tiếng Việt cực kỳ thông dụng (Lexicon Cache)
fn is_common_vietnamese_word(w: &str) -> bool {
    let lower = w.to_lowercase();
    matches!(
        lower.as_str(),
        "cộng" | "hòa" | "xã" | "hội" | "chủ" | "nghĩa" | "việt" | "nam" | "độc" | "lập" | "tự" | "do"
            | "hạnh" | "phúc" | "bác" | "sĩ" | "bảng" | "mã" | "khôi" | "phục" | "lỗi" | "chữ" | "tiếng"
            | "văn" | "bản" | "báo" | "cáo" | "ngày" | "tháng" | "năm" | "thực" | "hiện" | "công" | "ty"
            | "trách" | "nhiệm" | "hữu" | "hạn" | "tài" | "khoản" | "ngân" | "hàng" | "tiền" | "lương"
            | "thuế" | "hóa" | "đơn" | "khách" | "hợp" | "đồng" | "quyết" | "định" | "thông" | "tư"
            | "nghị" | "luật" | "chính" | "phủ" | "bộ" | "ban" | "ngành" | "phòng" | "giám" | "đốc"
            | "trưởng" | "nhân" | "viên" | "kế" | "toán" | "kiểm" | "tra" | "xác" | "nhận" | "kính" | "gửi"
            | "chúc" | "sức" | "khỏe" | "thành" | "hà" | "nội" | "hồ" | "chí" | "minh" | "đà" | "nẵng"
            | "hải" | "cần" | "thơ" | "quản" | "lý" | "dữ" | "liệu" | "tin" | "hệ" | "thống" | "phần"
            | "mềm" | "ứng" | "dụng" | "danh" | "sách" | "chi" | "tiết" | "tổng" | "số" | "lượng" | "giá"
            | "ghi" | "chú" | "địa" | "chỉ" | "điện" | "thoại" | "người" | "đại" | "diện" | "pháp" | "ký"
            | "tên" | "đóng" | "dấu" | "mua" | "bán" | "chuyển" | "nhượng" | "vay" | "vốn" | "lãi" | "suất"
            | "kỳ" | "kiệm" | "đầu" | "kinh" | "doanh" | "thương" | "mại" | "dịch" | "vụ"
            | "sản" | "xuất" | "xây" | "dựng" | "phát" | "triển" | "quốc" | "gia" | "dân" | "tộc"
            | "nhất" | "bảo" | "hiểm" | "học" | "tập" | "sinh" | "hoạt" | "tác" | "lao" | "động" | "việc"
            | "làm" | "nghề" | "nghiệp" | "thời" | "gian" | "điểm" | "mục" | "tiêu" | "kết" | "quả" | "đánh"
            | "xếp" | "loại" | "sắc" | "tốt" | "khá" | "trung" | "bình" | "khen" | "thưởng"
    )
}

/// Tính tỷ lệ âm tiết / từ hợp lệ tiếng Việt trong chuỗi văn bản (Vietnamese Syllable Scorer)
pub fn score_vietnamese_text(text: &str) -> f32 {
    let mut total_words = 0usize;
    let mut score_sum = 0.0f32;

    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| c.is_ascii_punctuation() || matches!(c, '“'|'”'|'‘'|'’'|'«'|'»'|'–'|'—'));
        if clean.is_empty() {
            continue;
        }
        total_words += 1;

        // 1. Chặn ký tự dị dạng / Mojibake / TCVN3 ngoài bảng Unicode tiếng Việt
        if clean.chars().any(is_corrupted_char) {
            continue;
        }

        // 2. Chặn các cụm lỗi mã VNI
        let lower = clean.to_lowercase();
        if has_vni_corrupted_patterns(&lower) {
            continue;
        }

        // 3. Chặn các chuỗi Mojibake UTF-8 điển hình và các vần bất hợp lệ
        if clean.contains('Ã') || clean.contains('Ä') || lower.contains("ei") || lower.contains("éng") {
            continue;
        }

        // 4. Kiểm tra mọi ký tự phải thuộc bảng chữ cái tiếng Việt
        if clean.chars().any(|c| !is_vietnamese_char(c)) {
            continue;
        }

        // 5. Quy tắc chính tả tiếng Việt: ngh chỉ đi với i, e, ê, y
        if lower.starts_with("ngh") {
            let after = &lower[3..];
            if let Some(c) = after.chars().next() {
                if !matches!(c, 'i'|'í'|'ì'|'ỉ'|'ĩ'|'ị'|'e'|'é'|'è'|'ẻ'|'ẽ'|'ẹ'|'ê'|'ế'|'ề'|'ể'|'ễ'|'ệ'|'y'|'ý'|'ỳ'|'ỷ'|'ỹ'|'ỵ') {
                    continue;
                }
            }
        }

        if is_common_vietnamese_word(clean) {
            score_sum += 1.0;
        } else {
            score_sum += 0.8;
        }
    }

    if total_words == 0 {
        return 0.0;
    }

    (score_sum / total_words as f32).clamp(0.0, 1.0)
}

// ============================================================================
// 2. CÁC ĐỘNG CƠ GIẢI MÃ BẢNG MÃ CỐT LÕI (TCVN3, VNI, MOJIBAKE)
// ============================================================================

/// Ánh xạ byte / ký tự TCVN3 (ABC 1-byte) sang Unicode dựng sẵn
pub fn convert_tcvn3_to_unicode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());

    for c in text.chars() {
        if (c as u32) <= 0xFF {
            let b = c as u8;
            let mapped = match b {
                0xb5 => "à", 0xb6 => "ả", 0xb7 => "ã", 0xb8 => "á", 0xb9 => "ạ",
                0xa8 => "ă", 0xbb => "ằ", 0xbc => "ẳ", 0xbd => "ẵ", 0xbe => "ắ", 0xc6 => "ặ",
                0xa9 => "â", 0xc7 => "ầ", 0xc8 => "ẩ", 0xc9 => "ẫ", 0xca => "ấ", 0xcb => "ậ",
                0xcc => "è", 0xce => "ẻ", 0xcf => "ẽ", 0xd0 => "é", 0xd1 => "ẹ",
                0xaa => "ê", 0xd2 => "ề", 0xd3 => "ể", 0xd4 => "ễ", 0xd5 => "ế", 0xd6 => "ệ",
                0xd7 => "ì", 0xd8 => "ỉ", 0xdc => "ĩ", 0xdd => "í", 0xde => "ị",
                0xdf => "ò", 0xe1 => "ỏ", 0xe2 => "õ", 0xe3 => "ó", 0xe4 => "ọ",
                0xab => "ô", 0xe5 => "ồ", 0xe6 => "ổ", 0xe7 => "ỗ", 0xe8 => "ố", 0xe9 => "ộ",
                0xac => "ơ", 0xea => "ờ", 0xeb => "ở", 0xec => "ỡ", 0xed => "ớ", 0xee => "ợ",
                0xef => "ù", 0xf1 => "ủ", 0xf2 => "ũ", 0xf3 => "ú", 0xf4 => "ụ",
                0xad => "ư", 0xf5 => "ừ", 0xf6 => "ử", 0xf7 => "ữ", 0xf8 => "ứ", 0xf9 => "ự",
                0xfa => "ỳ", 0xfb => "ỷ", 0xfc => "ỹ", 0xfd => "ý", 0xfe => "ỵ",
                0xae => "đ", 0xa7 => "Đ",
                0xa1 => "Ă", 0xa2 => "Â", 0xa3 => "Ê", 0xa4 => "Ô", 0xa5 => "Ơ", 0xa6 => "Ư",
                _ => "",
            };

            if !mapped.is_empty() {
                out.push_str(mapped);
            } else {
                out.push(c);
            }
        } else {
            out.push(c);
        }
    }

    out
}

/// Ánh xạ VNI-Windows (2-byte / 1-byte) sang Unicode dựng sẵn
pub fn convert_vni_to_unicode(text: &str) -> String {
    let mut s = text.to_string();

    let pairs: &[(&str, &str)] = &[
        ("aø", "à"), ("aù", "á"), ("aû", "ả"), ("aõ", "ã"), ("aï", "ạ"),
        ("Aø", "À"), ("Aù", "Á"), ("Aû", "Ả"), ("Aõ", "Ã"), ("Aï", "Ạ"),
        ("AØ", "À"), ("AÙ", "Á"), ("AÛ", "Ả"), ("AÕ", "Ã"), ("AÏ", "Ạ"),
        ("aê", "ă"), ("aè", "ằ"), ("aé", "ắ"), ("aú", "ẳ"), ("aü", "ẵ"), ("aë", "ặ"),
        ("Aê", "Ă"), ("Aè", "Ằ"), ("Aé", "Ắ"), ("Aú", "Ẳ"), ("Aü", "Ẵ"), ("Aë", "Ặ"),
        ("AÊ", "Ă"), ("AÈ", "Ằ"), ("AÉ", "Ắ"), ("AÚ", "Ẳ"), ("AÜ", "Ẵ"), ("AË", "Ặ"),
        ("aâ", "â"), ("aà", "ầ"), ("aá", "ấ"), ("aå", "ẩ"), ("aã", "ẫ"), ("aä", "ậ"),
        ("Aâ", "Â"), ("Aà", "Ầ"), ("Aá", "Ấ"), ("Aå", "Ẩ"), ("Aã", "Ẫ"), ("Aä", "Ậ"),
        ("AÂ", "Â"), ("AÀ", "Ầ"), ("AÁ", "Ấ"), ("AÅ", "Ẩ"), ("AÃ", "Ẫ"), ("AÄ", "Ậ"),
        ("eø", "è"), ("eù", "é"), ("eû", "ẻ"), ("eõ", "ẽ"), ("eï", "ẹ"),
        ("Eø", "È"), ("Eù", "É"), ("Eû", "Ẻ"), ("Eõ", "Ẽ"), ("Eï", "Ẹ"),
        ("EØ", "È"), ("EÙ", "É"), ("EÛ", "Ẻ"), ("EÕ", "Ẽ"), ("EÏ", "Ẹ"),
        ("eâ", "ê"), ("eà", "ề"), ("eá", "ế"), ("eå", "ể"), ("eã", "ễ"), ("eä", "ệ"),
        ("Eâ", "Ê"), ("Eà", "Ề"), ("Eá", "Ế"), ("Eå", "Ể"), ("Eã", "Ễ"), ("Eä", "Ệ"),
        ("EÂ", "Ê"), ("EÀ", "Ề"), ("EÁ", "Ế"), ("EÅ", "Ể"), ("EÃ", "Ễ"), ("EÄ", "Ệ"),
        ("iø", "ì"), ("iù", "í"), ("iû", "ỉ"), ("iõ", "ĩ"), ("iï", "ị"),
        ("Iø", "Ì"), ("Iù", "Í"), ("Iû", "Ỉ"), ("Iõ", "Ĩ"), ("Iï", "Ị"),
        ("IØ", "Ì"), ("IÙ", "Í"), ("IÛ", "Ỉ"), ("IÕ", "Ĩ"), ("IÏ", "Ị"),
        ("oø", "ò"), ("où", "ó"), ("oû", "ỏ"), ("oõ", "õ"), ("oï", "ọ"),
        ("Oø", "Ò"), ("Où", "Ó"), ("Oû", "Ỏ"), ("Oõ", "Õ"), ("Oï", "Ọ"),
        ("OØ", "Ò"), ("OÙ", "Ó"), ("OÛ", "Ỏ"), ("OÕ", "Õ"), ("OÏ", "Ọ"),
        ("oâ", "ô"), ("oà", "ồ"), ("oá", "ố"), ("oå", "ổ"), ("oã", "ỗ"), ("oä", "ộ"),
        ("Oâ", "Ô"), ("Oà", "Ồ"), ("Oá", "Ố"), ("Oå", "Ổ"), ("Oã", "Ỗ"), ("Oä", "Ộ"),
        ("OÂ", "Ô"), ("OÀ", "Ồ"), ("OÁ", "Ố"), ("OÅ", "Ổ"), ("OÃ", "Ỗ"), ("OÄ", "Ộ"),
        ("ôø", "ờ"), ("ôù", "ớ"), ("ôû", "ở"), ("ôõ", "ỡ"), ("ôï", "ợ"),
        ("Ôø", "Ờ"), ("Ôù", "Ớ"), ("Ôû", "Ở"), ("Ôõ", "Ỡ"), ("ÔÏ", "Ợ"),
        ("ÔØ", "Ờ"), ("ÔÙ", "Ớ"), ("ÔÛ", "Ở"), ("ÔÕ", "Ỡ"), ("ÔÏ", "Ợ"),
        ("uø", "ù"), ("uù", "ú"), ("uû", "ủ"), ("uõ", "ũ"), ("uï", "ụ"),
        ("Uø", "Ù"), ("Uù", "Ú"), ("Uû", "Ủ"), ("Uõ", "Ũ"), ("Uï", "Ụ"),
        ("UØ", "Ù"), ("UÙ", "Ú"), ("UÛ", "Ủ"), ("UÕ", "Ũ"), ("UÏ", "Ụ"),
        ("öø", "ừ"), ("öù", "ứ"), ("öû", "ử"), ("öõ", "ữ"), ("öï", "ự"),
        ("Öø", "Ừ"), ("Öù", "Ứ"), ("Öû", "Ử"), ("Öõ", "Ữ"), ("Öï", "Ự"),
        ("ÖØ", "Ừ"), ("ÖÙ", "Ứ"), ("ÖÛ", "Ử"), ("ÖÕ", "Ữ"), ("ÖÏ", "Ự"),
        ("yø", "ỳ"), ("yù", "ý"), ("yû", "ỷ"), ("yõ", "ỹ"),
        ("Yø", "Ỳ"), ("Yù", "Ý"), ("Yû", "Ỷ"), ("Yõ", "Ỹ"),
        ("YØ", "Ỳ"), ("YÙ", "Ý"), ("YÛ", "Ỷ"), ("YÕ", "Ỹ"),
    ];

    for &(v, u) in pairs {
        s = s.replace(v, u);
    }

    let single_pairs: &[(&str, &str)] = &[
        ("ó", "ĩ"),
        ("Ó", "Ĩ"),
        ("ô", "ơ"), ("Ô", "Ơ"),
        ("ö", "ư"), ("Ö", "Ư"),
        ("ñ", "đ"), ("Ñ", "Đ"),
        ("î", "ỵ"), ("Î", "Ỵ"),
    ];

    for &(v, u) in single_pairs {
        s = s.replace(v, u);
    }

    s
}

/// Ánh xạ ký tự Windows-1252 đặc biệt về byte tương ứng
fn cp1252_char_to_byte(c: char) -> Option<u8> {
    match c {
        '\u{20AC}' => Some(0x80),
        '\u{201A}' => Some(0x82),
        '\u{0192}' => Some(0x83),
        '\u{201E}' => Some(0x84),
        '\u{2026}' => Some(0x85),
        '\u{2020}' => Some(0x86),
        '\u{2021}' => Some(0x87),
        '\u{02C6}' => Some(0x88),
        '\u{2030}' => Some(0x89),
        '\u{0160}' => Some(0x8A),
        '\u{2039}' => Some(0x8B),
        '\u{0152}' => Some(0x8C),
        '\u{017D}' => Some(0x8E),
        '\u{2018}' => Some(0x91),
        '\u{2019}' => Some(0x92),
        '\u{201C}' => Some(0x93),
        '\u{201D}' => Some(0x94),
        '\u{2022}' => Some(0x95),
        '\u{2013}' => Some(0x96),
        '\u{2014}' => Some(0x97),
        '\u{02DC}' => Some(0x98),
        '\u{2122}' => Some(0x99),
        '\u{0161}' => Some(0x9A),
        '\u{203A}' => Some(0x9B),
        '\u{0153}' => Some(0x9C),
        '\u{017E}' => Some(0x9E),
        '\u{0178}' => Some(0x9F),
        _ => None,
    }
}

/// Chuyển chuỗi Mojibake Windows-1252/ISO-8859-1 về mảng byte raw
fn text_to_cp1252_bytes(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len());
    for c in text.chars() {
        if let Some(b) = cp1252_char_to_byte(c) {
            bytes.push(b);
        } else if (c as u32) <= 0xFF {
            bytes.push(c as u8);
        } else {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            bytes.extend_from_slice(s.as_bytes());
        }
    }
    bytes
}

/// Khôi phục chuỗi Mojibake UTF-8 (bị đọc nhầm thành CP1252 hoặc ISO-8859-1)
pub fn convert_mojibake_to_unicode(text: &str) -> String {
    let mut current = text.to_string();

    for _ in 0..2 {
        let raw = text_to_cp1252_bytes(&current);
        if let Ok(decoded) = std::str::from_utf8(&raw) {
            if decoded != current {
                current = decoded.to_string();
                continue;
            }
        }
        break;
    }

    current
}

// ============================================================================
// 3. TỰ ĐỘNG PHÁT HIỆN & KHÔI PHỤC Ở CẤP ĐỘ TỪNG Ô (CELL-LEVEL AUTO-DETECTION)
// ============================================================================

/// Tự động phát hiện và khôi phục ô dữ liệu lỗi font hoặc lỗi mã
pub fn recover_cell_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return text.to_string();
    }

    let orig_score = score_vietnamese_text(text);
    if orig_score >= 0.85 {
        return text.to_string();
    }

    let cand_mojibake = convert_mojibake_to_unicode(text);
    let cand_tcvn3 = convert_tcvn3_to_unicode(text);
    let cand_vni = convert_vni_to_unicode(text);

    let cand_moji_tcvn3 = convert_tcvn3_to_unicode(&cand_mojibake);
    let cand_moji_vni = convert_vni_to_unicode(&cand_mojibake);

    let candidates = [
        (text, orig_score),
        (cand_mojibake.as_str(), score_vietnamese_text(&cand_mojibake)),
        (cand_tcvn3.as_str(), score_vietnamese_text(&cand_tcvn3)),
        (cand_vni.as_str(), score_vietnamese_text(&cand_vni)),
        (cand_moji_tcvn3.as_str(), score_vietnamese_text(&cand_moji_tcvn3)),
        (cand_moji_vni.as_str(), score_vietnamese_text(&cand_moji_vni)),
    ];

    let mut best_str = text;
    let mut best_score = orig_score;

    for (c_str, c_score) in candidates {
        if c_score > best_score {
            best_score = c_score;
            best_str = c_str;
        }
    }

    if best_score > orig_score + 0.1 && best_score >= 0.4 {
        best_str.to_string()
    } else {
        text.to_string()
    }
}

// ============================================================================
// 4. BẢO TOÀN & TÁI CẤU TRÚC FONT TRONG OPENXML (DOCX & XLSX)
// ============================================================================

fn map_target_font(font_name: &str, serif: &str, sans: &str) -> Option<String> {
    let lower = font_name.to_lowercase();
    if lower.contains("vntime") || lower.contains("vni-times") || lower.contains("vni-time") {
        Some(serif.to_string())
    } else if lower.contains("vnarial") || lower.contains("vni-helve") || lower.contains("vni-arial") {
        Some(sans.to_string())
    } else if lower.starts_with(".vn") || lower.starts_with("vni-") || lower.starts_with("vni_") {
        Some(serif.to_string())
    } else {
        None
    }
}

pub fn patch_docx_xml(xml: &str, serif: &str, sans: &str) -> String {
    let re_rfonts = match Regex::new(r#"<w:rFonts[^>]*>"#) {
        Ok(r) => r,
        Err(_) => return xml.to_string(),
    };
    let step1 = re_rfonts.replace_all(xml, |caps: &regex::Captures| {
        let tag = &caps[0];
        let re_attr = match Regex::new(r#"(w:(?:ascii|hAnsi|cs|eastAsia))="([^"]+)""#) {
            Ok(r) => r,
            Err(_) => return tag.to_string(),
        };
        let rep = re_attr.replace_all(tag, |acaps: &regex::Captures| {
            let attr = &acaps[1];
            let font = &acaps[2];
            if let Some(target) = map_target_font(font, serif, sans) {
                format!(r#"{attr}="{target}""#)
            } else {
                acaps[0].to_string()
            }
        });
        rep.into_owned()
    }).into_owned();

    let re_text = match Regex::new(r#"(?s)(<w:t[^>]*>)(.*?)(</w:t>)"#) {
        Ok(r) => r,
        Err(_) => return step1,
    };
    re_text.replace_all(&step1, |caps: &regex::Captures| {
        let open_tag = &caps[1];
        let content = &caps[2];
        let close_tag = &caps[3];
        let recovered = recover_cell_text(content);
        format!("{open_tag}{recovered}{close_tag}")
    }).into_owned()
}

pub fn patch_xlsx_shared_strings(xml: &str, serif: &str, sans: &str) -> String {
    let re_rfont = match Regex::new(r#"<rFont[^>]*>"#) {
        Ok(r) => r,
        Err(_) => return xml.to_string(),
    };
    let step1 = re_rfont.replace_all(xml, |caps: &regex::Captures| {
        let tag = &caps[0];
        let re_val = match Regex::new(r#"val="([^"]+)""#) {
            Ok(r) => r,
            Err(_) => return tag.to_string(),
        };
        let rep = re_val.replace(tag, |vcaps: &regex::Captures| {
            let font_val = &vcaps[1];
            if let Some(target) = map_target_font(font_val, serif, sans) {
                format!(r#"val="{target}""#)
            } else {
                vcaps[0].to_string()
            }
        });
        rep.into_owned()
    }).into_owned();

    let re_t = match Regex::new(r#"(?s)(<t[^>]*>)(.*?)(</t>)"#) {
        Ok(r) => r,
        Err(_) => return step1,
    };
    re_t.replace_all(&step1, |caps: &regex::Captures| {
        let open_tag = &caps[1];
        let content = &caps[2];
        let close_tag = &caps[3];
        let recovered = recover_cell_text(content);
        format!("{open_tag}{recovered}{close_tag}")
    }).into_owned()
}

pub fn patch_xlsx_styles(xml: &str, serif: &str, sans: &str) -> String {
    let re_name = match Regex::new(r#"<name[^>]*>"#) {
        Ok(r) => r,
        Err(_) => return xml.to_string(),
    };

    re_name.replace_all(xml, |caps: &regex::Captures| {
        let tag = &caps[0];
        let re_val = match Regex::new(r#"val="([^"]+)""#) {
            Ok(r) => r,
            Err(_) => return tag.to_string(),
        };
        let rep = re_val.replace(tag, |vcaps: &regex::Captures| {
            let font_val = &vcaps[1];
            if let Some(target) = map_target_font(font_val, serif, sans) {
                format!(r#"val="{target}""#)
            } else {
                vcaps[0].to_string()
            }
        });
        rep.into_owned()
    }).into_owned()
}

pub fn patch_xlsx_worksheet(xml: &str) -> String {
    let re_t = match Regex::new(r#"(?s)(<t[^>]*>)(.*?)(</t>)"#) {
        Ok(r) => r,
        Err(_) => return xml.to_string(),
    };

    re_t.replace_all(xml, |caps: &regex::Captures| {
        let open_tag = &caps[1];
        let content = &caps[2];
        let close_tag = &caps[3];

        let recovered = recover_cell_text(content);
        format!("{open_tag}{recovered}{close_tag}")
    }).into_owned()
}

// ============================================================================
// 5. ZIP PARSER & BUILDER AN TOÀN BỘ NHỚ
// ============================================================================

#[derive(Debug, Clone)]
pub struct ZipEntry {
    pub name: String,
    pub compression_method: u16,
    pub mod_time: u16,
    pub mod_date: u16,
    pub data: Vec<u8>,
}

pub fn parse_zip_archive(raw: &[u8], cancel_flag: &AtomicBool) -> Result<Vec<ZipEntry>, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let mut entries = Vec::new();
    let mut offset = 0;

    while offset + 30 <= raw.len() {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        if &raw[offset..offset + 4] != b"PK\x03\x04" {
            break;
        }

        let compression = u16::from_le_bytes([raw[offset + 8], raw[offset + 9]]);
        let mod_time = u16::from_le_bytes([raw[offset + 10], raw[offset + 11]]);
        let mod_date = u16::from_le_bytes([raw[offset + 12], raw[offset + 13]]);
        let comp_size = u32::from_le_bytes([
            raw[offset + 18],
            raw[offset + 19],
            raw[offset + 20],
            raw[offset + 21],
        ]) as usize;
        let name_len = u16::from_le_bytes([raw[offset + 26], raw[offset + 27]]) as usize;
        let extra_len = u16::from_le_bytes([raw[offset + 28], raw[offset + 29]]) as usize;

        let name_start = offset + 30;
        let name_end = name_start + name_len;
        if name_end > raw.len() {
            break;
        }

        let name = String::from_utf8_lossy(&raw[name_start..name_end]).to_string();
        let data_start = name_end + extra_len;
        let data_end = data_start + comp_size;

        if data_end > raw.len() {
            break;
        }

        let comp_data = &raw[data_start..data_end];

        let decompressed_data = if compression == 8 {
            let mut decoder = DeflateDecoder::new(comp_data).take(250 * 1024 * 1024);
            let mut decomp = Vec::new();
            if decoder.read_to_end(&mut decomp).is_err() {
                comp_data.to_vec()
            } else {
                decomp
            }
        } else {
            comp_data.to_vec()
        };

        entries.push(ZipEntry {
            name,
            compression_method: compression,
            mod_time,
            mod_date,
            data: decompressed_data,
        });

        offset = data_end;
    }

    Ok(entries)
}

pub fn build_zip_archive(entries: &[ZipEntry], cancel_flag: &AtomicBool) -> Result<Vec<u8>, CommandError> {
    let mut output = Vec::new();
    let mut central_directory = Vec::new();

    for entry in entries {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let local_header_offset = output.len() as u32;

        let mut crc_hasher = crc32fast::Hasher::new();
        crc_hasher.update(&entry.data);
        let crc = crc_hasher.finalize();

        let uncompressed_size = entry.data.len() as u32;

        let (comp_data, comp_method) = if entry.compression_method == 8 {
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
            if encoder.write_all(&entry.data).is_ok() {
                match encoder.finish() {
                    Ok(compressed) => (compressed, 8u16),
                    Err(_) => (entry.data.clone(), 0u16),
                }
            } else {
                (entry.data.clone(), 0u16)
            }
        } else {
            (entry.data.clone(), 0u16)
        };

        let compressed_size = comp_data.len() as u32;
        let name_bytes = entry.name.as_bytes();

        // 1. Local File Header
        output.extend_from_slice(b"PK\x03\x04");
        output.extend_from_slice(&20u16.to_le_bytes());
        output.extend_from_slice(&0u16.to_le_bytes());
        output.extend_from_slice(&comp_method.to_le_bytes());
        output.extend_from_slice(&entry.mod_time.to_le_bytes());
        output.extend_from_slice(&entry.mod_date.to_le_bytes());
        output.extend_from_slice(&crc.to_le_bytes());
        output.extend_from_slice(&compressed_size.to_le_bytes());
        output.extend_from_slice(&uncompressed_size.to_le_bytes());
        output.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        output.extend_from_slice(&0u16.to_le_bytes());
        output.extend_from_slice(name_bytes);
        output.extend_from_slice(&comp_data);

        // 2. Central Directory Header
        central_directory.extend_from_slice(b"PK\x01\x02");
        central_directory.extend_from_slice(&20u16.to_le_bytes());
        central_directory.extend_from_slice(&20u16.to_le_bytes());
        central_directory.extend_from_slice(&0u16.to_le_bytes());
        central_directory.extend_from_slice(&comp_method.to_le_bytes());
        central_directory.extend_from_slice(&entry.mod_time.to_le_bytes());
        central_directory.extend_from_slice(&entry.mod_date.to_le_bytes());
        central_directory.extend_from_slice(&crc.to_le_bytes());
        central_directory.extend_from_slice(&compressed_size.to_le_bytes());
        central_directory.extend_from_slice(&uncompressed_size.to_le_bytes());
        central_directory.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        central_directory.extend_from_slice(&0u16.to_le_bytes());
        central_directory.extend_from_slice(&0u16.to_le_bytes());
        central_directory.extend_from_slice(&0u16.to_le_bytes());
        central_directory.extend_from_slice(&0u16.to_le_bytes());
        central_directory.extend_from_slice(&0u32.to_le_bytes());
        central_directory.extend_from_slice(&local_header_offset.to_le_bytes());
        central_directory.extend_from_slice(name_bytes);
    }

    let cd_offset = output.len() as u32;
    let cd_size = central_directory.len() as u32;
    let entry_count = entries.len() as u16;

    output.extend_from_slice(&central_directory);

    // 3. End of Central Directory
    output.extend_from_slice(b"PK\x05\x06");
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&entry_count.to_le_bytes());
    output.extend_from_slice(&entry_count.to_le_bytes());
    output.extend_from_slice(&cd_size.to_le_bytes());
    output.extend_from_slice(&cd_offset.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());

    Ok(output)
}

// ============================================================================
// 6. THỰC THI TRAIT OfficeTool
// ============================================================================

impl OfficeTool for VnFontEncodingDocTool {
    fn slug(&self) -> &'static str {
        "vn-font-encoding-doc"
    }

    fn name(&self) -> &'static str {
        "Bác sĩ Bảng mã & Khôi phục Lỗi chữ Tiếng Việt"
    }

    fn description(&self) -> &'static str {
        "Tự động phát hiện và khôi phục ô dữ liệu lỗi font TCVN3/VNI-Windows kết hợp sửa lỗi Mojibake UTF-8 (Windows-1252/ISO-8859-1) và ánh xạ lại font chữ trong file OpenXML Word/Excel."
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

        let options: VnFontEncodingDocOptions = serde_json::from_value(request.options.clone())
            .unwrap_or_default();

        let serif = options.target_font_serif.as_deref().unwrap_or("Times New Roman");
        let sans = options.target_font_sans.as_deref().unwrap_or("Arial");

        // Trường hợp 1: Chuyển đổi trực tiếp chuỗi 'text' trong options
        if let Some(text_val) = request.options.get("text").and_then(|v| v.as_str()) {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let recovered = recover_cell_text(text_val);

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 1,
                    total_steps: 1,
                    percentage: 100.0,
                    message: "Hoàn tất khôi phục văn bản trực tiếp.".to_string(),
                    processed_files: 1,
                    failed_files: 0,
                    total_files: 1,
                    is_completed: true,
                });
            }

            return Ok(DispatchTaskResponse {
                task_id: request.task_id.clone(),
                status: "SUCCESS".to_string(),
                output_paths: vec![recovered],
                failed_items: Vec::new(),
                elapsed_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // Trường hợp 2: Chuyển đổi danh sách tệp tin đầu vào
        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Không có tệp đầu vào và không có chuỗi 'text' trong options".to_string(),
            });
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("vn_font_doc_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let mut output_paths = Vec::new();
        let mut failed_items: Vec<FileFailureDetail> = Vec::new();
        let total_files = request.input_paths.len();

        for (idx, file_path_str) in request.input_paths.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let path = Path::new(file_path_str);
            if !path.exists() {
                return Err(CommandError::FileNotFound {
                    path: file_path_str.clone(),
                });
            }

            let bytes = match fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    return Err(CommandError::IoError {
                        details: format!("Lỗi đọc tệp {file_path_str}: {e}"),
                    });
                }
            };

            // NFR-SEC-04: Chặn các tệp nhị phân thực thi độc hại
            if bytes.starts_with(b"\x7fELF") || bytes.starts_with(b"MZ") {
                return Err(CommandError::InvalidFileSignature {
                    expected: "OpenXML (.docx/.xlsx) hoặc tệp văn bản".to_string(),
                    actual: "Tệp nhị phân thực thi không hợp lệ".to_string(),
                });
            }

            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
            let file_ext = path.extension().and_then(|s| s.to_str()).unwrap_or("txt").to_lowercase();

            let out_file_name = format!("{}_recovered.{}", file_stem, file_ext);
            let target_path = output_dir.join(out_file_name);

            if bytes.starts_with(b"PK\x03\x04") {
                match parse_zip_archive(&bytes, cancel_flag) {
                    Ok(mut entries) => {
                        let is_docx = entries.iter().any(|e| e.name.starts_with("word/"));
                        let is_xlsx = entries.iter().any(|e| e.name.starts_with("xl/"));

                        for entry in &mut entries {
                            if cancel_flag.load(Ordering::Relaxed) {
                                return Err(CommandError::TaskCancelled);
                            }

                            if is_docx && entry.name.starts_with("word/") && entry.name.ends_with(".xml") {
                                let xml_str = String::from_utf8_lossy(&entry.data);
                                let patched = patch_docx_xml(&xml_str, serif, sans);
                                entry.data = patched.into_bytes();
                            } else if is_xlsx {
                                if entry.name == "xl/sharedStrings.xml" {
                                    let xml_str = String::from_utf8_lossy(&entry.data);
                                    let patched = patch_xlsx_shared_strings(&xml_str, serif, sans);
                                    entry.data = patched.into_bytes();
                                } else if entry.name == "xl/styles.xml" {
                                    let xml_str = String::from_utf8_lossy(&entry.data);
                                    let patched = patch_xlsx_styles(&xml_str, serif, sans);
                                    entry.data = patched.into_bytes();
                                } else if entry.name.starts_with("xl/worksheets/") && entry.name.ends_with(".xml") {
                                    let xml_str = String::from_utf8_lossy(&entry.data);
                                    let patched = patch_xlsx_worksheet(&xml_str);
                                    entry.data = patched.into_bytes();
                                }
                            }
                        }

                        match build_zip_archive(&entries, cancel_flag) {
                            Ok(repacked) => {
                                if let Err(e) = fs::write(&target_path, repacked) {
                                    failed_items.push(FileFailureDetail {
                                        input_path: file_path_str.clone(),
                                        reason: format!("Lỗi ghi file đầu ra: {e}"),
                                    });
                                    continue;
                                }
                                output_paths.push(target_path.to_string_lossy().to_string());
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    Err(e) => return Err(e),
                }
            } else if bytes.starts_with(b"<?xml") || file_ext == "xml" {
                let xml_str = String::from_utf8_lossy(&bytes);
                let patched = patch_docx_xml(&xml_str, serif, sans);
                if let Err(e) = fs::write(&target_path, patched.into_bytes()) {
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("Lỗi ghi file đầu ra: {e}"),
                    });
                    continue;
                }
                output_paths.push(target_path.to_string_lossy().to_string());
            } else {
                let text_content = String::from_utf8_lossy(&bytes);
                let mut lines_out = Vec::new();
                for line in text_content.lines() {
                    lines_out.push(recover_cell_text(line));
                }
                let final_text = lines_out.join("\n");
                if let Err(e) = fs::write(&target_path, final_text.into_bytes()) {
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("Lỗi ghi file đầu ra: {e}"),
                    });
                    continue;
                }
                output_paths.push(target_path.to_string_lossy().to_string());
            }

            if let Some(prog) = on_progress {
                let current_step = (idx + 1) as u32;
                let total_steps = total_files as u32;
                let percentage = (current_step as f32 / total_steps as f32) * 100.0;
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step,
                    total_steps,
                    percentage,
                    message: format!("Đã xử lý xong tệp {}/{}", current_step, total_steps),
                    processed_files: idx + 1,
                    failed_files: failed_items.len(),
                    total_files,
                    is_completed: current_step == total_steps,
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

// ============================================================================
// 7. KIỂM THỬ ĐƠN VỊ & BẢO MẬT (TDD)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_vn_font_encoding_doc_syllable_scorer() {
        let valid_text = "Cộng hòa Xã hội Chủ nghĩa Việt Nam - Độc lập Tự do Hạnh phúc";
        let valid_score = score_vietnamese_text(valid_text);
        assert!(
            valid_score >= 0.85,
            "Điểm tiếng Việt chuẩn phải cao (>= 0.85), thực tế: {}",
            valid_score
        );

        let tcvn3_text = "Céng hßa X· héi Chñ nghÜa ViÖt Nam";
        let tcvn3_score = score_vietnamese_text(tcvn3_text);
        assert!(
            tcvn3_score <= 0.25,
            "Điểm TCVN3 lỗi mã phải thấp (<= 0.25), thực tế: {}",
            tcvn3_score
        );

        let vni_text = "Coäng hoøa Xaõ hoäi Chuû nghóa Vieät Nam";
        let vni_score = score_vietnamese_text(vni_text);
        assert!(
            vni_score <= 0.25,
            "Điểm VNI lỗi mã phải thấp (<= 0.25), thực tế: {}",
            vni_score
        );

        let mojibake_text = "Cá»™ng hÃ²a XÃ£ há»™i Chá»§ nghÄ©a Viá»‡t Nam";
        let mojibake_score = score_vietnamese_text(mojibake_text);
        assert!(
            mojibake_score <= 0.25,
            "Điểm Mojibake lỗi mã phải thấp (<= 0.25), thực tế: {}",
            mojibake_score
        );
    }

    #[test]
    fn test_vn_font_encoding_doc_tcvn3_autodetect() {
        let corrupted = "Céng hßa X· héi Chñ nghÜa ViÖt Nam";
        let recovered = recover_cell_text(corrupted);
        assert_eq!(recovered, "Cộng hòa Xã hội Chủ nghĩa Việt Nam");
    }

    #[test]
    fn test_vn_font_encoding_doc_vni_autodetect() {
        let corrupted = "Coäng hoøa Xaõ hoäi Chuû nghóa Vieät Nam";
        let recovered = recover_cell_text(corrupted);
        assert_eq!(recovered, "Cộng hòa Xã hội Chủ nghĩa Việt Nam");
    }

    #[test]
    fn test_vn_font_encoding_doc_mojibake_autodetect() {
        let corrupted = "Cá»™ng hÃ²a XÃ£ há»™i Chá»§ nghÄ©a Viá»‡t Nam";
        let recovered = recover_cell_text(corrupted);
        assert_eq!(recovered, "Cộng hòa Xã hội Chủ nghĩa Việt Nam");
    }

    #[test]
    fn test_vn_font_encoding_doc_cooperative_cancellation() {
        let tool = VnFontEncodingDocTool::default();
        let cancel_flag = AtomicBool::new(true);
        let request = DispatchTaskRequest {
            task_id: "test-cancel".to_string(),
            tool_slug: "vn-font-encoding-doc".to_string(),
            input_paths: vec!["non_existent.docx".to_string()],
            output_directory: None,
            options: serde_json::json!({ "text": "Céng hßa" }),
        };
        let res = tool.execute(&request, &cancel_flag, None);
        assert_eq!(res.unwrap_err(), CommandError::TaskCancelled);
    }

    #[test]
    fn test_vn_font_encoding_doc_invalid_magic_bytes() {
        let tool = VnFontEncodingDocTool::default();
        let cancel_flag = AtomicBool::new(false);
        let temp_dir = std::env::temp_dir().join("vn_font_test_sec");
        let _ = fs::create_dir_all(&temp_dir);
        let bad_file = temp_dir.join("bad_exec.docx");
        fs::write(&bad_file, b"\x7fELF\x02\x01\x01\x00malicious").unwrap();

        let request = DispatchTaskRequest {
            task_id: "test-sec".to_string(),
            tool_slug: "vn-font-encoding-doc".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::json!({}),
        };
        let res = tool.execute(&request, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::InvalidFileSignature { .. })));
        let _ = fs::remove_file(bad_file);
    }

    #[test]
    fn test_vn_font_encoding_doc_openxml_docx_xml() {
        let xml_input = r#"<w:p><w:r><w:rPr><w:rFonts w:ascii=".VnTime" w:hAnsi=".VnTime"/></w:rPr><w:t>Céng hßa X· héi</w:t></w:r></w:p>"#;
        let patched = patch_docx_xml(xml_input, "Times New Roman", "Arial");
        assert!(patched.contains(r#"w:ascii="Times New Roman""#));
        assert!(patched.contains(r#"<w:t>Cộng hòa Xã hội</w:t>"#));
    }

    #[test]
    fn test_vn_font_encoding_doc_openxml_xlsx_xml() {
        let sst_input = r#"<sst><si><r><rPr><rFont val="VNI-Times"/></rPr><t>Coäng hoøa</t></r></si></sst>"#;
        let patched = patch_xlsx_shared_strings(sst_input, "Times New Roman", "Arial");
        assert!(patched.contains(r#"<rFont val="Times New Roman"/>"#));
        assert!(patched.contains(r#"<t>Cộng hòa</t>"#));
    }
}
