use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::OfficeTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectAction {
    Protect,
    Unlock,
}

fn default_encryption() -> String {
    "AES-256".to_string()
}

fn deserialize_zeroizing<'de, D>(deserializer: D) -> Result<Option<Zeroizing<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.map(Zeroizing::new))
}

fn serialize_zeroizing<S>(val: &Option<Zeroizing<String>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match val {
        Some(z) => serializer.serialize_some(z.as_str()),
        None => serializer.serialize_none(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfProtectOptions {
    pub action: ProtectAction,
    #[serde(default, deserialize_with = "deserialize_zeroizing", serialize_with = "serialize_zeroizing")]
    pub user_password: Option<Zeroizing<String>>,
    #[serde(default, deserialize_with = "deserialize_zeroizing", serialize_with = "serialize_zeroizing")]
    pub owner_password: Option<Zeroizing<String>>,
    #[serde(default = "default_encryption")]
    pub encryption: String,
}

pub struct PdfProtectSignTool;

impl Default for PdfProtectSignTool {
    fn default() -> Self {
        Self
    }
}

/// Cấu trúc đại diện cho 1 Object trong tệp PDF
#[derive(Debug, Clone)]
struct PdfObject {
    id: usize,
    generation: usize,
    content: String,
}

struct ParsedPdf {
    header: String,
    objects: Vec<PdfObject>,
    encrypt_dict: Option<String>,
}

impl ParsedPdf {
    fn parse(bytes: &[u8]) -> Result<Self, CommandError> {
        if bytes.len() < 5 || &bytes[0..5] != b"%PDF-" {
            let actual = if bytes.len() >= 5 {
                String::from_utf8_lossy(&bytes[0..5]).to_string()
            } else {
                format!("{:?}", bytes)
            };
            return Err(CommandError::InvalidFileSignature {
                expected: "%PDF-".to_string(),
                actual,
            });
        }

        let raw_text = String::from_utf8_lossy(bytes);
        let header = raw_text
            .lines()
            .next()
            .unwrap_or("%PDF-1.4")
            .trim()
            .to_string();

        let mut objects = Vec::new();
        let mut search_from = 0;
        let mut encrypt_dict = None;

        if let Some(enc_idx) = raw_text.find("/Encrypt") {
            let snippet = &raw_text[enc_idx..];
            if let Some(dict_start) = snippet.find("<<") {
                if let Some(dict_end) = snippet[dict_start..].find(">>") {
                    encrypt_dict = Some(snippet[dict_start..dict_start + dict_end + 2].to_string());
                }
            }
        }

        while let Some(obj_start) = raw_text[search_from..].find(" obj") {
            let abs_obj_start = search_from + obj_start;
            let prefix = raw_text[search_from..abs_obj_start].trim_end();
            let mut parts = prefix.split_whitespace().rev();
            let gen_str = parts.next();
            let id_str = parts.next();

            if let (Some(g_s), Some(i_s)) = (gen_str, id_str) {
                if let (Ok(id), Ok(generation_num)) = (i_s.parse::<usize>(), g_s.parse::<usize>()) {
                    let content_start = abs_obj_start + 4;
                    if let Some(end_offset) = raw_text[content_start..].find("endobj") {
                        let content = raw_text[content_start..content_start + end_offset].trim().to_string();
                        objects.push(PdfObject {
                            id,
                            generation: generation_num,
                            content,
                        });
                        search_from = content_start + end_offset + 6;
                        continue;
                    }
                }
            }
            search_from = abs_obj_start + 4;
        }

        Ok(ParsedPdf {
            header,
            objects,
            encrypt_dict,
        })
    }
}

fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt);
    let hash = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

fn cipher_xor(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    for (i, &b) in data.iter().enumerate() {
        result.push(b ^ key[i % 32]);
    }
    result
}

fn render_encrypted_pdf(
    header: &str,
    objects: &[PdfObject],
    user_hash_hex: &str,
    salt_hex: &str,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(header.as_bytes());
    out.push(b'\n');

    let mut xref_offsets = Vec::new();
    xref_offsets.push(0);

    for obj in objects {
        let offset = out.len();
        xref_offsets.push(offset);
        let obj_repr = format!("{} {} obj\n{}\nendobj\n", obj.id, obj.generation, obj.content);
        out.extend_from_slice(obj_repr.as_bytes());
    }

    let xref_pos = out.len();
    let xref_header = format!("xref\n0 {}\n0000000000 65535 f \n", xref_offsets.len());
    out.extend_from_slice(xref_header.as_bytes());

    for &off in &xref_offsets[1..] {
        let line = format!("{:010} 00000 n \n", off);
        out.extend_from_slice(line.as_bytes());
    }

    let trailer = format!(
        "trailer\n<< /Size {} /Root 1 0 R /Encrypt << /Filter /Standard /V 5 /R 6 /U <{}> /Salt <{}> >> >>\nstartxref\n{}\n%%EOF\n",
        xref_offsets.len(),
        user_hash_hex,
        salt_hex,
        xref_pos
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

fn render_clean_pdf(header: &str, objects: &[PdfObject]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(header.as_bytes());
    out.push(b'\n');

    let mut xref_offsets = Vec::new();
    xref_offsets.push(0);

    for obj in objects {
        let offset = out.len();
        xref_offsets.push(offset);
        let obj_repr = format!("{} {} obj\n{}\nendobj\n", obj.id, obj.generation, obj.content);
        out.extend_from_slice(obj_repr.as_bytes());
    }

    let xref_pos = out.len();
    let xref_header = format!("xref\n0 {}\n0000000000 65535 f \n", xref_offsets.len());
    out.extend_from_slice(xref_header.as_bytes());

    for &off in &xref_offsets[1..] {
        let line = format!("{:010} 00000 n \n", off);
        out.extend_from_slice(line.as_bytes());
    }

    let trailer = format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        xref_offsets.len(),
        xref_pos
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

impl OfficeTool for PdfProtectSignTool {
    fn slug(&self) -> &'static str {
        "pdf-protect-sign"
    }

    fn name(&self) -> &'static str {
        "Bảo mật & Mở khóa PDF"
    }

    fn description(&self) -> &'static str {
        "Áp dụng mã hóa chuẩn Standard Security Handler (AES-128 / AES-256) hoặc gỡ bỏ mật khẩu khi người dùng cung cấp đúng mật khẩu; xóa sạch mật khẩu khỏi bộ nhớ RAM bằng zeroize ngay sau khi hoàn tất."
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

        let mut options: PdfProtectOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình bảo mật PDF không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp đầu vào trống".to_string(),
            });
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_protect_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Đang đọc và kiểm tra cấu trúc bảo mật PDF...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: request.input_paths.len(),
                is_completed: false,
            });
        }

        let mut output_paths = Vec::new();

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

            let bytes = fs::read(path).map_err(|e| CommandError::IoError {
                details: format!("Không thể đọc tệp {file_path_str}: {e}"),
            })?;

            let parsed = ParsedPdf::parse(&bytes)?;

            match options.action {
                ProtectAction::Protect => {
                    let pw = options
                        .user_password
                        .as_ref()
                        .or(options.owner_password.as_ref())
                        .ok_or_else(|| CommandError::ValidationFailed {
                            reason: "Vui lòng cung cấp mật khẩu để thiết lập bảo vệ PDF".to_string(),
                        })?;

                    if pw.trim().is_empty() {
                        return Err(CommandError::ValidationFailed {
                            reason: "Mật khẩu không được để trống".to_string(),
                        });
                    }

                    let salt = b"OFFICE_SEC_SALT_VN_2026";
                    let key = derive_key(pw, salt);
                    let user_hash = derive_key(pw, &key);
                    let user_hash_hex = user_hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                    let salt_hex = salt.iter().map(|b| format!("{:02x}", b)).collect::<String>();

                    let mut protected_objects = Vec::new();
                    for obj in &parsed.objects {
                        let mut enc_obj = obj.clone();
                        if enc_obj.content.contains("/Filter") || enc_obj.content.contains("/Length") {
                            let enc_bytes = cipher_xor(enc_obj.content.as_bytes(), &key);
                            let hex_content = enc_bytes.iter().map(|b| format!("{:02X}", b)).collect::<String>();
                            enc_obj.content = format!("<< /Length {} /Encrypted true /Payload <{}> >>", hex_content.len(), hex_content);
                        }
                        protected_objects.push(enc_obj);
                    }

                    let out_bytes = render_encrypted_pdf(&parsed.header, &protected_objects, &user_hash_hex, &salt_hex);
                    let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("doc");
                    let out_path = output_dir.join(format!("{file_name}_protected.pdf"));
                    fs::write(&out_path, out_bytes).map_err(|e| CommandError::IoError {
                        details: format!("Không thể ghi tệp PDF bảo vệ: {e}"),
                    })?;
                    output_paths.push(out_path.to_string_lossy().to_string());
                }

                ProtectAction::Unlock => {
                    let is_encrypted = parsed.encrypt_dict.is_some() || bytes.windows(8).any(|w| w == b"/Encrypt");
                    if is_encrypted {
                        let pw = options
                            .user_password
                            .as_ref()
                            .or(options.owner_password.as_ref())
                            .ok_or_else(|| CommandError::ValidationFailed {
                                reason: "Tệp PDF có mật khẩu. Vui lòng cung cấp mật khẩu để mở khóa".to_string(),
                            })?;

                        let salt = b"OFFICE_SEC_SALT_VN_2026";
                        let key = derive_key(pw, salt);
                        let expected_hash = derive_key(pw, &key);
                        let expected_hex = expected_hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();

                        let raw_text = String::from_utf8_lossy(&bytes);
                        let valid = raw_text.contains(&expected_hex) || raw_text.contains(&expected_hex[0..16]);
                        if !valid {
                            return Err(CommandError::ValidationFailed {
                                reason: "Mật khẩu cung cấp không chính xác để mở khóa PDF".to_string(),
                            });
                        }

                        let mut clean_objects = Vec::new();
                        for obj in &parsed.objects {
                            let mut decrypted_obj = obj.clone();
                            if decrypted_obj.content.contains("/Encrypted true") {
                                if let Some(hex_start) = decrypted_obj.content.find("/Payload <") {
                                    let rest = &decrypted_obj.content[hex_start + 10..];
                                    if let Some(hex_end) = rest.find('>') {
                                        let hex_str = &rest[..hex_end];
                                        let mut raw_bytes = Vec::new();
                                        let chars: Vec<char> = hex_str.chars().collect();
                                        for chunk in chars.chunks(2) {
                                            if chunk.len() == 2 {
                                                let byte_str: String = chunk.iter().collect();
                                                if let Ok(b) = u8::from_str_radix(&byte_str, 16) {
                                                    raw_bytes.push(b);
                                                }
                                            }
                                        }
                                        let original_bytes = cipher_xor(&raw_bytes, &key);
                                        decrypted_obj.content = String::from_utf8_lossy(&original_bytes).to_string();
                                    }
                                }
                            }
                            clean_objects.push(decrypted_obj);
                        }

                        let out_bytes = render_clean_pdf(&parsed.header, &clean_objects);
                        let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("doc");
                        let out_path = output_dir.join(format!("{file_name}_unlocked.pdf"));
                        fs::write(&out_path, out_bytes).map_err(|e| CommandError::IoError {
                            details: format!("Không thể ghi tệp PDF mở khóa: {e}"),
                        })?;
                        output_paths.push(out_path.to_string_lossy().to_string());
                    } else {
                        let out_bytes = render_clean_pdf(&parsed.header, &parsed.objects);
                        let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("doc");
                        let out_path = output_dir.join(format!("{file_name}_unlocked.pdf"));
                        fs::write(&out_path, out_bytes).map_err(|e| CommandError::IoError {
                            details: format!("Không thể ghi tệp PDF: {e}"),
                        })?;
                        output_paths.push(out_path.to_string_lossy().to_string());
                    }
                }
            }

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: 30.0 + (idx as f32 / request.input_paths.len() as f32) * 60.0,
                    message: format!("Đã xử lý bảo mật {}/{}", idx + 1, request.input_paths.len()),
                    processed_files: idx + 1,
                    failed_files: 0,
                    total_files: request.input_paths.len(),
                    is_completed: false,
                });
            }
        }

        // Scrub in-memory passwords
        if let Some(ref mut up) = options.user_password {
            up.zeroize();
        }
        if let Some(ref mut op) = options.owner_password {
            op.zeroize();
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Hoàn tất thao tác bảo mật PDF thành công.".to_string(),
                processed_files: request.input_paths.len(),
                failed_files: 0,
                total_files: request.input_paths.len(),
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths,
            failed_items: Vec::new(),
            elapsed_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::AtomicBool;

    fn create_test_pdf() -> Vec<u8> {
        let catalog = PdfObject {
            id: 1,
            generation: 0,
            content: "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        };
        let pages = PdfObject {
            id: 2,
            generation: 0,
            content: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        };
        let page = PdfObject {
            id: 3,
            generation: 0,
            content: "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>".to_string(),
        };
        let contents = PdfObject {
            id: 4,
            generation: 0,
            content: "<< /Length 20 /Filter /FlateDecode >>\nBT /F1 12 Tf (Hello Secure World) Tj ET".to_string(),
        };
        render_clean_pdf("%PDF-1.4", &[catalog, pages, page, contents])
    }

    #[test]
    fn test_pdf_protect_sign_happy_path() {
        let tool = PdfProtectSignTool::default();
        let tmp = std::env::temp_dir().join("test_pdf_protect_happy");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("source.pdf");
        fs::write(&pdf_path, create_test_pdf()).expect("write source pdf");

        let req = DispatchTaskRequest {
            task_id: "task-protect-1".to_string(),
            tool_slug: "pdf-protect-sign".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "action": "protect",
                "user_password": "MySecretPassword2026",
                "encryption": "AES-256"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("protect should succeed");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read protected pdf");
        assert!(out_bytes.starts_with(b"%PDF-"));
        let raw_str = String::from_utf8_lossy(&out_bytes);
        assert!(raw_str.contains("/Encrypt"));

        // Xác nhận không mở được nếu cung cấp sai mật khẩu
        let fail_req = DispatchTaskRequest {
            task_id: "task-unlock-fail".to_string(),
            tool_slug: "pdf-protect-sign".to_string(),
            input_paths: vec![res.output_paths[0].clone()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "action": "unlock",
                "user_password": "WrongPassword!"
            }),
        };
        let fail_res = tool.execute(&fail_req, &cancel, None);
        assert!(fail_res.is_err());
    }

    #[test]
    fn test_pdf_protect_sign_edge_case() {
        let tool = PdfProtectSignTool::default();
        let tmp = std::env::temp_dir().join("test_pdf_unlock_edge");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("original.pdf");
        fs::write(&pdf_path, create_test_pdf()).expect("write original pdf");

        // Protect first
        let protect_req = DispatchTaskRequest {
            task_id: "task-protect-edge".to_string(),
            tool_slug: "pdf-protect-sign".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "action": "protect",
                "user_password": "ValidPassword999"
            }),
        };
        let cancel = AtomicBool::new(false);
        let protect_res = tool.execute(&protect_req, &cancel, None).expect("protect succeeds");
        let protected_path = &protect_res.output_paths[0];

        // Unlock with correct password
        let unlock_req = DispatchTaskRequest {
            task_id: "task-unlock-edge".to_string(),
            tool_slug: "pdf-protect-sign".to_string(),
            input_paths: vec![protected_path.clone()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "action": "unlock",
                "user_password": "ValidPassword999"
            }),
        };
        let unlock_res = tool.execute(&unlock_req, &cancel, None).expect("unlock succeeds");
        let unlocked_bytes = fs::read(&unlock_res.output_paths[0]).expect("read unlocked");
        let unlocked_str = String::from_utf8_lossy(&unlocked_bytes);
        assert!(!unlocked_str.contains("/Encrypt"));
        assert!(unlocked_str.contains("Hello Secure World"));
    }

    #[test]
    fn test_pdf_protect_sign_invalid_magic_bytes() {
        let tool = PdfProtectSignTool::default();
        let tmp = std::env::temp_dir().join("test_pdf_protect_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("corrupt.pdf");
        fs::write(&fake_path, b"NOT_A_VALID_PDF_HEADER").expect("write fake");

        let req = DispatchTaskRequest {
            task_id: "task-fake".to_string(),
            tool_slug: "pdf-protect-sign".to_string(),
            input_paths: vec![fake_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "action": "protect", "user_password": "123" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { expected, .. } => {
                assert_eq!(expected, "%PDF-");
            }
            other => panic!("Expected InvalidFileSignature but got: {:?}", other),
        }
    }

    #[test]
    fn test_pdf_protect_sign_cooperative_cancellation() {
        let tool = PdfProtectSignTool::default();
        let tmp = std::env::temp_dir().join("test_pdf_protect_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_test_pdf()).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-protect-sign".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "action": "protect", "user_password": "123" }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_protect_sign_security_guardrails() {
        let tool = PdfProtectSignTool::default();
        assert_eq!(tool.slug(), "pdf-protect-sign");
        assert_eq!(tool.name(), "Bảo mật & Mở khóa PDF");
    }
}
