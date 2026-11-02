use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Cấu hình tùy chọn sửa lỗi Mojibake
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MojibakeFixOptions {
    #[serde(default = "default_true")]
    pub auto_detect: bool,
}

fn default_true() -> bool {
    true
}

pub struct EncodingMojibakeFixerTool;

impl Default for EncodingMojibakeFixerTool {
    fn default() -> Self {
        Self
    }
}

/// Ánh xạ ký tự Windows-1252 đặc biệt (0x80 ..= 0x9F) về byte tương ứng
fn cp1252_char_to_byte(c: char) -> Option<u8> {
    match c {
        '\u{20AC}' => Some(0x80), // €
        '\u{201A}' => Some(0x82), // ‚
        '\u{0192}' => Some(0x83), // ƒ
        '\u{201E}' => Some(0x84), // „
        '\u{2026}' => Some(0x85), // …
        '\u{2020}' => Some(0x86), // †
        '\u{2021}' => Some(0x87), // ‡
        '\u{02C6}' => Some(0x88), // ˆ
        '\u{2030}' => Some(0x89), // ‰
        '\u{0160}' => Some(0x8A), // Š
        '\u{2039}' => Some(0x8B), // ‹
        '\u{0152}' => Some(0x8C), // Œ
        '\u{017D}' => Some(0x8E), // Ž
        '\u{2018}' => Some(0x91), // ‘
        '\u{2019}' => Some(0x92), // ’
        '\u{201C}' => Some(0x93), // “
        '\u{201D}' => Some(0x94), // ”
        '\u{2022}' => Some(0x95), // •
        '\u{2013}' => Some(0x96), // –
        '\u{2014}' => Some(0x97), // —
        '\u{02DC}' => Some(0x98), // ˜
        '\u{2122}' => Some(0x99), // ™
        '\u{0161}' => Some(0x9A), // š
        '\u{203A}' => Some(0x9B), // ›
        '\u{0153}' => Some(0x9C), // œ
        '\u{017E}' => Some(0x9E), // ž
        '\u{0178}' => Some(0x9F), // Ÿ
        _ => None,
    }
}

/// Chuyển đổi chuỗi ký tự bị giải mã sai thành mảng byte ban đầu
fn text_to_cp1252_or_latin1_bytes(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len());
    for c in text.chars() {
        if let Some(b) = cp1252_char_to_byte(c) {
            bytes.push(b);
        } else if (c as u32) <= 0xFF {
            bytes.push(c as u8);
        } else {
            // Với các ký tự khác ngoài bảng mã 1-byte, encode utf-8
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            bytes.extend_from_slice(s.as_bytes());
        }
    }
    bytes
}

/// Sửa lỗi trên một token/từ đơn lẻ
fn fix_single_token(token: &str) -> String {
    let mut current = token.to_string();
    for _ in 0..3 {
        let has_mojibake = current.contains("áº")
            || current.contains("á»")
            || current.contains("Ã")
            || current.contains("Ä")
            || current.contains("Â")
            || current.contains("Å");
        if !has_mojibake {
            break;
        }
        let raw = text_to_cp1252_or_latin1_bytes(&current);
        if let Ok(rec) = std::str::from_utf8(&raw)
            && rec != current {
                current = rec.to_string();
                continue;
            }
        break;
    }
    current
}

/// Sửa lỗi hiển thị Mojibake tiếng Việt (hoàn nguyên UTF-8 bị đọc nhầm theo CP1252 / ISO-8859-1)
pub fn fix_mojibake_text(corrupted: &str, cancel_flag: &AtomicBool) -> Result<String, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    // Thử khôi phục toàn bộ chuỗi trước
    let mut whole_candidate = corrupted.to_string();
    for _ in 0..3 {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let has_mojibake = whole_candidate.contains("áº")
            || whole_candidate.contains("á»")
            || whole_candidate.contains("Ã")
            || whole_candidate.contains("Ä")
            || whole_candidate.contains("Â")
            || whole_candidate.contains("Å");

        if !has_mojibake {
            break;
        }

        let raw_bytes = text_to_cp1252_or_latin1_bytes(&whole_candidate);
        if let Ok(recovered) = std::str::from_utf8(&raw_bytes)
            && recovered != whole_candidate {
                whole_candidate = recovered.to_string();
                continue;
            }
        break;
    }

    // Nếu toàn bộ chuỗi đã được khôi phục sạch sẽ
    if !whole_candidate.contains("áº")
        && !whole_candidate.contains("á»")
        && !whole_candidate.contains("Ã")
        && !whole_candidate.contains("Ä")
        && !whole_candidate.contains("Â")
    {
        return Ok(whole_candidate);
    }

    // Nếu chuỗi chứa văn bản hỗn hợp (vừa có từ đúng vừa có từ lỗi), xử lý từng từ
    let mut words = Vec::new();
    for word in corrupted.split(' ') {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }
        words.push(fix_single_token(word));
    }

    Ok(words.join(" "))
}

impl OfficeTool for EncodingMojibakeFixerTool {
    fn slug(&self) -> &'static str {
        "encoding-mojibake-fixer"
    }

    fn name(&self) -> &'static str {
        "Sửa Lỗi Hiển thị Chữ Mojibake"
    }

    fn description(&self) -> &'static str {
        "Tự động khôi phục chuỗi văn bản tiếng Việt bị lỗi hiển thị do giải mã sai bảng mã UTF-8 và Windows-1252 / ISO-8859-1."
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
                    expected: "Tệp văn bản thuần".to_string(),
                    actual: "Tệp nhị phân thực thi bị chặn".to_string(),
                });
            }

            String::from_utf8_lossy(&raw_bytes).to_string()
        } else if let Some(text_val) = request.options.get("text").and_then(|v| v.as_str()) {
            text_val.to_string()
        } else {
            return Err(CommandError::ValidationFailed {
                reason: "Không có văn bản hoặc tệp đầu vào cần sửa lỗi Mojibake".to_string(),
            });
        };

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 50.0,
                message: "Đang đảo ngược chu kỳ bảng mã UTF-8 / Windows-1252...".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        let fixed_text = fix_mojibake_text(&content, cancel_flag)?;

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("mojibake_fix_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        let out_file = out_dir.join("mojibake_fixed.txt");
        fs::write(&out_file, fixed_text.as_bytes()).map_err(|e| CommandError::IoError {
            details: e.to_string(),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Đã phục hồi văn bản tiếng Việt đọc được".to_string(),
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
    fn test_encoding_mojibake_fixer_happy_path() {
        let tool = EncodingMojibakeFixerTool;
        assert_eq!(tool.slug(), "encoding-mojibake-fixer");

        // Kiểm thử khôi phục chuỗi 'Tiáº¿ng Viá»‡t' về 'Tiếng Việt'
        let corrupted = "Tiáº¿ng Viá»‡t";
        let cancel = AtomicBool::new(false);
        let fixed = fix_mojibake_text(corrupted, &cancel).expect("fix ok");
        assert_eq!(fixed, "Tiếng Việt");
    }

    #[test]
    fn test_encoding_mojibake_fixer_edge_case() {
        // Kiểm thử xử lý chuỗi lỗi bảng mã ISO-8859-1 (ví dụ: 'Cá»™ng hÃ²a' -> 'Cộng hòa')
        let corrupted = "Cá»™ng hÃ²a XÃ£ há»™i ch\u{00fa} nghÄ©a Viá»‡t Nam";
        let cancel = AtomicBool::new(false);
        let fixed = fix_mojibake_text(corrupted, &cancel).expect("fix edge ok");
        assert_eq!(fixed, "Cộng hòa Xã hội chú nghĩa Việt Nam");
    }

    #[test]
    fn test_encoding_mojibake_fixer_invalid_magic_bytes() {
        let tool = EncodingMojibakeFixerTool;
        let tmp = std::env::temp_dir().join("test_mojibake_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("bad_exec.bin");
        fs::write(&bad_file, b"\x7fELF\x02\x01\x01\x00").expect("write elf");

        let req = DispatchTaskRequest {
            task_id: "task-fake-mojibake".to_string(),
            tool_slug: "encoding-mojibake-fixer".to_string(),
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
    fn test_encoding_mojibake_fixer_cooperative_cancellation() {
        let tool = EncodingMojibakeFixerTool;
        let req = DispatchTaskRequest {
            task_id: "task-cancel-mojibake".to_string(),
            tool_slug: "encoding-mojibake-fixer".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "text": "Tiáº¿ng Viá»‡t"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_encoding_mojibake_fixer_security_guardrails() {
        // Chuỗi văn bản bình thường không bị lỗi font -> giữ nguyên 100%
        let cancel = AtomicBool::new(false);
        let normal_text = "Hà Nội mùa thu tuyệt đẹp 2026!";
        let fixed = fix_mojibake_text(normal_text, &cancel).expect("keep normal text");
        assert_eq!(fixed, normal_text);
    }
}
