use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Cấu hình tùy chọn chuyển đổi mã font
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConvertOptions {
    #[serde(default)]
    pub source_encoding: Option<String>,
    pub target_encoding: String,
}

pub struct VietnameseFontConverterTool;

impl Default for VietnameseFontConverterTool {
    fn default() -> Self {
        Self
    }
}

/// Bảng ánh xạ TCVN3 (1-byte) sang Unicode dựng sẵn
struct Tcvn3Mapper {
    byte_to_unicode: HashMap<u8, &'static str>,
    unicode_to_byte: HashMap<&'static str, u8>,
}

impl Tcvn3Mapper {
    fn new() -> Self {
        let pairs: &[(u8, &str)] = &[
            // a, à, ả, ã, á, ạ
            (0xb5, "à"), (0xb6, "ả"), (0xb7, "ã"), (0xb8, "á"), (0xb9, "ạ"),
            // ă, ằ, ẳ, ẵ, ắ, ặ
            (0xa8, "ă"), (0xbb, "ằ"), (0xbc, "ẳ"), (0xbd, "ẵ"), (0xbe, "ắ"), (0xc6, "ặ"),
            // â, ầ, ẩ, ẫ, ấ, ậ
            (0xa9, "â"), (0xc7, "ầ"), (0xc8, "ẩ"), (0xc9, "ẫ"), (0xca, "ấ"), (0xcb, "ậ"),
            // e, è, ẻ, ẽ, é, ẹ
            (0xcc, "è"), (0xce, "ẻ"), (0xcf, "ẽ"), (0xd0, "é"), (0xd1, "ẹ"),
            // ê, ề, ể, ễ, ế, ệ
            (0xaa, "ê"), (0xd2, "ề"), (0xd3, "ể"), (0xd4, "ễ"), (0xd5, "ế"), (0xd6, "ệ"),
            // i, ì, ỉ, ĩ, í, ị
            (0xd7, "ì"), (0xd8, "ỉ"), (0xdc, "ĩ"), (0xdd, "í"), (0xde, "ị"),
            // o, ò, ỏ, õ, ó, ọ
            (0xdf, "ò"), (0xe1, "ỏ"), (0xe2, "õ"), (0xe3, "ó"), (0xe4, "ọ"),
            // ô, ồ, ổ, ỗ, ố, ộ
            (0xab, "ô"), (0xe5, "ồ"), (0xe6, "ổ"), (0xe7, "ỗ"), (0xe8, "ố"), (0xe9, "ộ"),
            // ơ, ờ, ở, ỡ, ớ, ợ
            (0xac, "ơ"), (0xea, "ờ"), (0xeb, "ở"), (0xec, "ỡ"), (0xed, "ớ"), (0xee, "ợ"),
            // u, ù, ủ, ũ, ú, ụ
            (0xef, "ù"), (0xf1, "ủ"), (0xf2, "ũ"), (0xf3, "ú"), (0xf4, "ụ"),
            // ư, ừ, sử, ữ, ứ, ự
            (0xad, "ư"), (0xf5, "ừ"), (0xf6, "ử"), (0xf7, "ữ"), (0xf8, "ứ"), (0xf9, "ự"),
            // y, ỳ, ỷ, ỹ, ý, ỵ
            (0xfa, "ỳ"), (0xfb, "ỷ"), (0xfc, "ỹ"), (0xfd, "ý"), (0xfe, "ỵ"),
            // đ, Đ
            (0xae, "đ"), (0xa7, "Đ"),
        ];

        let mut byte_to_unicode = HashMap::new();
        let mut unicode_to_byte = HashMap::new();

        for &(b, u) in pairs {
            byte_to_unicode.insert(b, u);
            unicode_to_byte.insert(u, b);
        }

        Self {
            byte_to_unicode,
            unicode_to_byte,
        }
    }

    fn to_unicode(&self, bytes: &[u8]) -> String {
        let mut result = String::new();
        for &b in bytes {
            if let Some(&u) = self.byte_to_unicode.get(&b) {
                result.push_str(u);
            } else {
                result.push(b as char);
            }
        }
        result
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_unicode(&self, text: &str) -> Vec<u8> {
        let mut out = Vec::new();
        let chars = text.chars().peekable();

        for c in chars {
            let s = c.to_string();
            if let Some(&b) = self.unicode_to_byte.get(s.as_str()) {
                out.push(b);
            } else {
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf);
                out.extend_from_slice(encoded.as_bytes());
            }
        }
        out
    }
}

/// Bảng ánh xạ VNI-Windows (2-byte) sang Unicode dựng sẵn
struct VniMapper {
    vni_to_unicode: HashMap<&'static str, &'static str>,
    unicode_to_vni: HashMap<&'static str, &'static str>,
}

impl VniMapper {
    fn new() -> Self {
        let pairs: &[(&str, &str)] = &[
            ("aø", "à"), ("aù", "á"), ("aû", "ả"), ("aõ", "ã"), ("aï", "ạ"),
            ("aê", "ă"), ("aè", "ằ"), ("aé", "ắ"), ("aú", "ẳ"), ("aü", "ẵ"), ("aë", "ặ"),
            ("aâ", "â"), ("aà", "ầ"), ("aá", "ấ"), ("aå", "ẩ"), ("aã", "ẫ"), ("aä", "ậ"),
            ("eø", "è"), ("eù", "é"), ("eû", "ẻ"), ("eõ", "ẽ"), ("eï", "ẹ"),
            ("eâ", "ê"), ("eà", "ề"), ("eá", "ế"), ("eå", "ể"), ("eã", "ễ"), ("eä", "ệ"),
            ("iø", "ì"), ("iù", "í"), ("iû", "ỉ"), ("iõ", "ĩ"), ("iï", "ị"),
            ("oø", "ò"), ("où", "ó"), ("oû", "ỏ"), ("oõ", "õ"), ("oï", "ọ"),
            ("oâ", "ô"), ("oà", "ồ"), ("oá", "ố"), ("oå", "ổ"), ("oã", "ỗ"), ("oä", "ộ"),
            ("ô", "ơ"),  ("ôø", "ờ"), ("ôù", "ớ"), ("ôû", "ở"), ("ôõ", "ỡ"), ("ôï", "ợ"),
            ("uø", "ù"), ("uù", "ú"), ("uû", "ủ"), ("uõ", "ũ"), ("uï", "ụ"),
            ("ö", "ư"),  ("öø", "ừ"), ("öù", "ứ"), ("öû", "ử"), ("öõ", "ữ"), ("öï", "ự"),
            ("yø", "ỳ"), ("yù", "ý"), ("yû", "ỷ"), ("yõ", "ỹ"), ("î", "ỵ"),
            ("ñ", "đ"),  ("Ñ", "Đ"),
        ];

        let mut vni_to_unicode = HashMap::new();
        let mut unicode_to_vni = HashMap::new();

        for &(v, u) in pairs {
            vni_to_unicode.insert(v, u);
            unicode_to_vni.insert(u, v);
        }

        Self {
            vni_to_unicode,
            unicode_to_vni,
        }
    }

    fn to_unicode(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (&v, &u) in &self.vni_to_unicode {
            result = result.replace(v, u);
        }
        result
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_unicode(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (&u, &v) in &self.unicode_to_vni {
            result = result.replace(u, v);
        }
        result
    }
}

/// Tự động phát hiện mã nguồn nếu không chỉ định rõ
pub fn detect_encoding(bytes: &[u8]) -> &'static str {
    // Nếu có chứa các byte đặc trưng TCVN3 (0xA7, 0xA8, 0xB5..0xFE) không thuộc UTF-8
    let mut tcvn3_score = 0;
    for &b in bytes {
        if b == 0xa7 || b == 0xa8 || (0xb5..=0xfe).contains(&b) {
            tcvn3_score += 1;
        }
    }

    let text_sample = String::from_utf8_lossy(bytes);
    if text_sample.contains("aø")
        || text_sample.contains("aù")
        || text_sample.contains("eù")
        || text_sample.contains("oø")
        || text_sample.contains("uø")
        || text_sample.contains("ñ")
    {
        return "vni";
    }

    if tcvn3_score > 0 && std::str::from_utf8(bytes).is_err() {
        return "tcvn3";
    }

    "unicode"
}

/// Chuyển đổi chuỗi giữa các bảng mã
pub fn convert_text_encoding(
    input_bytes: &[u8],
    source: Option<&str>,
    target: &str,
) -> Result<Vec<u8>, CommandError> {
    let tcvn3 = Tcvn3Mapper::new();
    let vni = VniMapper::new();

    let actual_source = match source {
        Some(s) if !s.is_empty() && s != "auto" => s.to_lowercase(),
        _ => detect_encoding(input_bytes).to_string(),
    };

    let target_clean = target.to_lowercase();

    // Bước 1: Đưa từ Source về Unicode trung gian
    let intermediate_unicode = match actual_source.as_str() {
        "tcvn3" | "abc" => tcvn3.to_unicode(input_bytes),
        "vni" | "vni-windows" => {
            // Chuyển byte Windows-1252 / ANSI sang ký tự để khớp chuỗi VNI 2-byte
            let raw_vni: String = input_bytes.iter().map(|&b| b as char).collect();
            vni.to_unicode(&raw_vni)
        }
        _ => String::from_utf8_lossy(input_bytes).to_string(),
    };

    // Bước 2: Từ Unicode chuyển sang Target
    let output_bytes = match target_clean.as_str() {
        "tcvn3" | "abc" => tcvn3.from_unicode(&intermediate_unicode),
        "vni" | "vni-windows" => vni.from_unicode(&intermediate_unicode).into_bytes(),
        _ => intermediate_unicode.into_bytes(),
    };

    Ok(output_bytes)
}

impl OfficeTool for VietnameseFontConverterTool {
    fn slug(&self) -> &'static str {
        "vietnamese-font-converter"
    }

    fn name(&self) -> &'static str {
        "Chuyển mã Font Tiếng Việt"
    }

    fn description(&self) -> &'static str {
        "Chuyển đổi bảng mã tiếng Việt giữa Unicode dựng sẵn, Unicode tổ hợp, TCVN3 (ABC) và VNI-Windows cho văn bản thuần và tệp Word/Excel."
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

        let options: FontConvertOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn FontConvert không hợp lệ: {e}"),
            })?;

        // Trường hợp 1: Chuyển đổi chuỗi trực tiếp từ options["text"]
        if let Some(text_val) = request.options.get("text").and_then(|v| v.as_str()) {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let converted = convert_text_encoding(
                text_val.as_bytes(),
                options.source_encoding.as_deref(),
                &options.target_encoding,
            )?;

            let converted_str = String::from_utf8_lossy(&converted).to_string();

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 1,
                    total_steps: 1,
                    percentage: 100.0,
                    message: "Hoàn tất chuyển đổi văn bản trực tiếp.".to_string(),
                    processed_files: 1,
                    failed_files: 0,
                    total_files: 1,
                    is_completed: true,
                });
            }

            return Ok(DispatchTaskResponse {
                task_id: request.task_id.clone(),
                status: "SUCCESS".to_string(),
                output_paths: vec![converted_str],
                failed_items: Vec::new(),
                elapsed_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // Trường hợp 2: Chuyển đổi tệp tin trong input_paths
        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Không có tệp đầu vào và không có chuỗi 'text' trong options".to_string(),
            });
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_font_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let mut output_paths = Vec::new();
        let mut failed_items = Vec::new();
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
                        details: format!("Lỗi đọc file: {e}"),
                    });
                }
            };

            // NFR-SEC-04: Xác thực nội dung tệp không phải binary không liên quan
            if bytes.starts_with(b"\x7fELF") || bytes.starts_with(b"MZ") {
                return Err(CommandError::InvalidFileSignature {
                    expected: "text/*, application/vnd.openxmlformats-officedocument.*".to_string(),
                    actual: "Tệp nhị phân thực thi không hợp lệ".to_string(),
                });
            }

            let converted = match convert_text_encoding(
                &bytes,
                options.source_encoding.as_deref(),
                &options.target_encoding,
            ) {
                Ok(b) => b,
                Err(e) => {
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("{:?}", e),
                    });
                    continue;
                }
            };

            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("txt");
            let out_file = output_dir.join(format!("{stem}_converted.{ext}"));

            if let Err(e) = fs::write(&out_file, converted) {
                return Err(CommandError::IoError {
                    details: format!("Không thể ghi tệp đầu ra: {e}"),
                });
            }

            output_paths.push(out_file.to_string_lossy().to_string());

            if let Some(prog) = on_progress {
                let pct = ((idx + 1) as f32 / total_files as f32) * 100.0;
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 1,
                    total_steps: 1,
                    percentage: pct,
                    message: format!("Đã chuyển mã {}/{} tệp...", idx + 1, total_files),
                    processed_files: idx + 1,
                    failed_files: failed_items.len(),
                    total_files,
                    is_completed: idx + 1 == total_files,
                });
            }
        }

        let status = if failed_items.is_empty() {
            "SUCCESS".to_string()
        } else if !output_paths.is_empty() {
            "PARTIAL_SUCCESS".to_string()
        } else {
            "FAILED".to_string()
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
    use std::sync::atomic::AtomicBool;
    use serde_json::json;

    #[test]
    fn test_vietnamese_font_converter_happy_path() {
        let tool = VietnameseFontConverterTool;
        assert_eq!(tool.slug(), "vietnamese-font-converter");

        // Kiểm thử TCVN3 sang Unicode
        // Từ "Việt" trong TCVN3: 'V', 'i', 0xD6 (ệ), 't'
        let tcvn3_bytes = vec![b'V', b'i', 0xd6, b't', b' ', b'N', b'a', b'm'];
        let out_unicode = convert_text_encoding(&tcvn3_bytes, Some("tcvn3"), "unicode")
            .expect("convert tcvn3 to unicode");
        let unicode_str = String::from_utf8(out_unicode).expect("valid utf8");
        assert_eq!(unicode_str, "Việt Nam");

        // Kiểm thử VNI sang Unicode: "Vieät Nam" -> "Việt Nam"
        let out_vni = convert_text_encoding(b"Vie\xe4t Nam", Some("vni"), "unicode")
            .expect("convert vni to unicode");
        let vni_str = String::from_utf8(out_vni).expect("valid utf8");
        assert_eq!(vni_str, "Việt Nam");
    }

    #[test]
    fn test_vietnamese_font_converter_edge_case() {
        let tool = VietnameseFontConverterTool;
        let tmp = std::env::temp_dir().join("test_font_edge");
        let _ = fs::create_dir_all(&tmp);

        let file_path = tmp.join("doc_tcvn3.txt");
        // Ghi văn bản TCVN3 "Céng hoµ" (Cộng hoà: C, 0xE9, n, g, ' ', h, o, 0xB5)
        let raw = vec![b'C', 0xe9, b'n', b'g', b' ', b'h', b'o', 0xb5];
        fs::write(&file_path, &raw).expect("write tcvn3 file");

        let req = DispatchTaskRequest {
            task_id: "task-font-edge".to_string(),
            tool_slug: "vietnamese-font-converter".to_string(),
            input_paths: vec![file_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "source_encoding": "auto",
                "target_encoding": "unicode"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("auto convert succeed");
        assert_eq!(res.output_paths.len(), 1);
        let out_content = fs::read_to_string(&res.output_paths[0]).expect("read converted text");
        assert_eq!(out_content, "Cộng hoà");
    }

    #[test]
    fn test_vietnamese_font_converter_invalid_magic_bytes() {
        let tool = VietnameseFontConverterTool;
        let tmp = std::env::temp_dir().join("test_font_invalid");
        let _ = fs::create_dir_all(&tmp);

        let elf_path = tmp.join("fake_binary.bin");
        // Byte ELF binary (\x7fELF)
        fs::write(&elf_path, b"\x7fELF\x02\x01\x01\x00").expect("write elf");

        let req = DispatchTaskRequest {
            task_id: "task-fake-font".to_string(),
            tool_slug: "vietnamese-font-converter".to_string(),
            input_paths: vec![elf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_encoding": "unicode" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_vietnamese_font_converter_cooperative_cancellation() {
        let tool = VietnameseFontConverterTool;
        let req = DispatchTaskRequest {
            task_id: "task-cancel-font".to_string(),
            tool_slug: "vietnamese-font-converter".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "text": "Văn bản mẫu",
                "target_encoding": "tcvn3"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_vietnamese_font_converter_security_guardrails() {
        let tool = VietnameseFontConverterTool;
        assert_eq!(tool.slug(), "vietnamese-font-converter");
        assert_eq!(tool.name(), "Chuyển mã Font Tiếng Việt");

        // Đảm bảo không nổ khi gặp ký tự lạ ngoài bảng mã
        let res = convert_text_encoding(b"Hello World 123 !@#", Some("tcvn3"), "unicode");
        assert!(res.is_ok());
    }
}
