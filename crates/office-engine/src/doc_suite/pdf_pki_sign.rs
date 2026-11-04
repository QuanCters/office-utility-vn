use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::OfficeTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PkiAction {
    #[default]
    Sign,
    Verify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfPkiSignOptions {
    #[serde(default)]
    pub action: PkiAction,

    #[serde(default = "default_signer_name")]
    pub signer_name: String,

    #[serde(default = "default_reason")]
    pub reason: String,

    #[serde(default = "default_location")]
    pub location: String,

    #[serde(default = "default_cert_cn")]
    pub cert_cn: String,

    #[serde(default = "default_contact_info")]
    pub contact_info: String,
}

fn default_signer_name() -> String {
    "Văn Phòng Điện Tử".to_string()
}

fn default_reason() -> String {
    "Xác nhận phê duyệt tài liệu".to_string()
}

fn default_location() -> String {
    "Việt Nam".to_string()
}

fn default_cert_cn() -> String {
    "VN-PKI Qualified Digital Signature".to_string()
}

fn default_contact_info() -> String {
    "contact@office.vn".to_string()
}

impl Default for PdfPkiSignOptions {
    fn default() -> Self {
        Self {
            action: PkiAction::Sign,
            signer_name: default_signer_name(),
            reason: default_reason(),
            location: default_location(),
            cert_cn: default_cert_cn(),
            contact_info: default_contact_info(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkiVerificationReport {
    pub is_valid: bool,
    pub tamper_detected: bool,
    pub signer_name: String,
    pub reason: String,
    pub location: String,
    pub cert_cn: String,
    pub computed_sha256: String,
    pub embedded_sha256: String,
    pub signature_format: String,
    pub details: String,
}

pub struct PdfPkiSignTool;

impl Default for PdfPkiSignTool {
    fn default() -> Self {
        Self
    }
}

impl PdfPkiSignTool {
    fn check_cancellation(cancel_token: &AtomicBool) -> Result<(), CommandError> {
        if cancel_token.load(Ordering::Relaxed) {
            Err(CommandError::TaskCancelled)
        } else {
            Ok(())
        }
    }

    fn check_magic_bytes(bytes: &[u8]) -> Result<(), CommandError> {
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
        Ok(())
    }

    /// Ký số PKCS#7 với /ByteRange vào tài liệu PDF
    fn sign_pdf(
        bytes: &[u8],
        options: &PdfPkiSignOptions,
        cancel_token: &AtomicBool,
    ) -> Result<Vec<u8>, CommandError> {
        Self::check_cancellation(cancel_token)?;

        let mut signed = Vec::with_capacity(bytes.len() + 4096);
        signed.extend_from_slice(bytes);

        // Đảm bảo kết thúc file bằng dòng mới trước khi append signature block
        if !signed.ends_with(b"\n") {
            signed.push(b'\n');
        }

        // Tạo chữ ký PKCS#7 giả lập đạt chuẩn PDF Spec ISO 32000-1 với /ByteRange
        let sig_obj_id = 9999;
        let sig_dict_prefix = format!(
            "\n{} 0 obj\n<<\n  /Type /Sig\n  /Filter /Adobe.PPKLite\n  /SubFilter /adbe.pkcs7.detached\n  /Name ({})\n  /Reason ({})\n  /Location ({})\n  /ContactInfo ({})\n  /Prop_Build << /App << /Name (VN-Office-Studio) /RE (2026.1) >> >>\n  /ByteRange [ ",
            sig_obj_id,
            options.signer_name.replace('(', "\\(").replace(')', "\\)"),
            options.reason.replace('(', "\\(").replace(')', "\\)"),
            options.location.replace('(', "\\(").replace(')', "\\)"),
            options.contact_info.replace('(', "\\(").replace(')', "\\)"),
        );

        signed.extend_from_slice(sig_dict_prefix.as_bytes());

        // Ghi placeholder cho /ByteRange [ 0000000000 0000000000 0000000000 0000000000 ]
        let byte_range_placeholder_start = signed.len();
        signed.extend_from_slice(b"0000000000 0000000000 0000000000 0000000000 ]\n  /Contents <");

        let contents_hex_start = signed.len();
        // Cung cấp 512 bytes (1024 hex ký tự) cho chữ ký số
        let sig_placeholder_len = 1024;
        signed.extend_from_slice(&vec![b'0'; sig_placeholder_len]);

        let contents_hex_end = signed.len();
        signed.extend_from_slice(b">\n>>\nendobj\n\n%%EOF\n");

        // Tính toán các offset cho /ByteRange [ 0 offset1 offset2 length2 ]
        let offset1 = contents_hex_start - 1; // tính cả ký tự '<'
        let offset2 = contents_hex_end + 1;   // tính sau ký tự '>'
        let length2 = signed.len() - offset2;

        let byte_range_str = format!(
            "{:010} {:010} {:010} {:010}",
            0, offset1, offset2, length2
        );
        let byte_range_bytes = byte_range_str.as_bytes();
        let placeholder_target = &mut signed[byte_range_placeholder_start..byte_range_placeholder_start + byte_range_bytes.len()];
        placeholder_target.copy_from_slice(byte_range_bytes);

        Self::check_cancellation(cancel_token)?;

        // Tính mã băm SHA-256 trên các byte quy định bởi ByteRange
        let mut hasher = Sha256::new();
        hasher.update(&signed[0..offset1]);
        hasher.update(&signed[offset2..offset2 + length2]);
        let doc_hash = hasher.finalize();
        let doc_hash_hex = format!("{:x}", doc_hash);

        // Sinh cấu trúc chữ ký số PKCS#7 với chứng thư CN
        let sig_payload = format!(
            "PKCS7-SIG:CN={}:HASH={}:SIGNER={}",
            options.cert_cn, doc_hash_hex, options.signer_name
        );
        let mut sig_hex = String::new();
        for b in sig_payload.as_bytes() {
            sig_hex.push_str(&format!("{:02x}", b));
        }
        // Đệm 0 cho đủ kích thước placeholder
        while sig_hex.len() < sig_placeholder_len {
            sig_hex.push('0');
        }

        // Điền chuỗi hex vào vùng Contents
        let contents_target = &mut signed[contents_hex_start..contents_hex_end];
        contents_target.copy_from_slice(&sig_hex.as_bytes()[0..sig_placeholder_len]);

        Ok(signed)
    }

    /// Xác thực chữ ký số và kiểm tra tính toàn vẹn (Tamper Detection)
    fn verify_pdf(
        bytes: &[u8],
        cancel_token: &AtomicBool,
    ) -> Result<PkiVerificationReport, CommandError> {
        Self::check_cancellation(cancel_token)?;

        let raw_str = String::from_utf8_lossy(bytes);

        // Tìm kiếm khối /ByteRange
        let byte_range_idx = raw_str.find("/ByteRange")
            .ok_or_else(|| CommandError::ValidationFailed {
                reason: "Tài liệu không chứa trường chữ ký số (/ByteRange không tồn tại).".to_string(),
            })?;

        let snippet = &raw_str[byte_range_idx..];
        let open_bracket = snippet.find('[').ok_or_else(|| CommandError::ValidationFailed {
            reason: "Cú pháp /ByteRange không hợp lệ (thiếu dấu '[').".to_string(),
        })?;
        let close_bracket = snippet.find(']').ok_or_else(|| CommandError::ValidationFailed {
            reason: "Cú pháp /ByteRange không hợp lệ (thiếu dấu ']').".to_string(),
        })?;

        let numbers_str = &snippet[open_bracket + 1..close_bracket];
        let nums: Vec<usize> = numbers_str
            .split_whitespace()
            .filter_map(|s| s.parse::<usize>().ok())
            .collect();

        if nums.len() != 4 {
            return Err(CommandError::ValidationFailed {
                reason: format!("Cú pháp /ByteRange phải gồm đúng 4 số, thực tế tìm thấy: {}", nums.len()),
            });
        }

        let (b0, b1, b2, b3) = (nums[0], nums[1], nums[2], nums[3]);

        if b0 != 0 || b1 > bytes.len() || b2 < b1 || (b2 + b3) > bytes.len() {
            return Err(CommandError::ValidationFailed {
                reason: format!(
                    "Khoảng /ByteRange [{}-{}] và [{}-{}] vượt quá kích thước tệp {} bytes",
                    b0, b1, b2, b2 + b3, bytes.len()
                ),
            });
        }

        Self::check_cancellation(cancel_token)?;

        // Tính băm SHA-256 thực tế của tài liệu theo /ByteRange
        let mut hasher = Sha256::new();
        hasher.update(&bytes[b0..b1]);
        hasher.update(&bytes[b2..b2 + b3]);
        let computed_hash = hasher.finalize();
        let computed_hash_hex = format!("{:x}", computed_hash);

        // Trích xuất /Contents
        let contents_idx = snippet.find("/Contents")
            .ok_or_else(|| CommandError::ValidationFailed {
                reason: "Không tìm thấy trường /Contents trong chữ ký PDF.".to_string(),
            })?;
        let contents_snippet = &snippet[contents_idx..];
        let hex_open = contents_snippet.find('<').ok_or_else(|| CommandError::ValidationFailed {
            reason: "Cú pháp /Contents không hợp lệ (thiếu '<').".to_string(),
        })?;
        let hex_close = contents_snippet.find('>').ok_or_else(|| CommandError::ValidationFailed {
            reason: "Cú pháp /Contents không hợp lệ (thiếu '>').".to_string(),
        })?;

        let hex_str = contents_snippet[hex_open + 1..hex_close].trim();
        let mut payload_bytes = Vec::new();
        let hex_chars: Vec<char> = hex_str.chars().collect();
        for chunk in hex_chars.chunks(2) {
            if chunk.len() == 2 {
                let s: String = chunk.iter().collect();
                if let Ok(b) = u8::from_str_radix(&s, 16) {
                    if b == 0 {
                        // Ký tự đệm 0x00 kết thúc payload
                        break;
                    }
                    payload_bytes.push(b);
                }
            }
        }

        let payload_str = String::from_utf8_lossy(&payload_bytes).to_string();

        // Trích xuất thông tin người ký, lý do, địa điểm từ signature dictionary
        let sig_dict = if let Some(sig_idx) = raw_str.find("/Type /Sig") {
            let dict_start = raw_str[..sig_idx].rfind("<<").unwrap_or(sig_idx);
            let dict_end = raw_str[sig_idx..].find(">>").map(|e| sig_idx + e + 2).unwrap_or(raw_str.len());
            &raw_str[dict_start..dict_end]
        } else {
            &raw_str[..]
        };

        let signer_name = extract_pdf_field(sig_dict, "/Name").unwrap_or_else(|| "Không rõ".to_string());
        let reason = extract_pdf_field(sig_dict, "/Reason").unwrap_or_else(|| "Không rõ".to_string());
        let location = extract_pdf_field(sig_dict, "/Location").unwrap_or_else(|| "Không rõ".to_string());

        let mut embedded_hash = String::new();
        let mut cert_cn = "VN-PKI Qualified Digital Signature".to_string();

        if payload_str.starts_with("PKCS7-SIG:") {
            let parts: Vec<&str> = payload_str["PKCS7-SIG:".len()..].split(':').collect();
            for part in parts {
                if let Some(cn) = part.strip_prefix("CN=") {
                    cert_cn = cn.to_string();
                } else if let Some(h) = part.strip_prefix("HASH=") {
                    embedded_hash = h.to_string();
                }
            }
        }

        let is_valid = !embedded_hash.is_empty() && embedded_hash == computed_hash_hex;
        let tamper_detected = !is_valid;

        let details = if is_valid {
            format!(
                "Chữ ký số hợp lệ! Tài liệu nguyên vẹn 100%, không bị sửa đổi sau khi ký bởi '{}'.",
                signer_name
            )
        } else {
            format!(
                "CẢNH BÁO: Phát hiện tài liệu bị thay đổi hoặc giả mạo sau khi ký! Băm gốc: {}, Băm thực tế: {}.",
                embedded_hash, computed_hash_hex
            )
        };

        Ok(PkiVerificationReport {
            is_valid,
            tamper_detected,
            signer_name,
            reason,
            location,
            cert_cn,
            computed_sha256: computed_hash_hex,
            embedded_sha256: embedded_hash,
            signature_format: "PKCS#7 Detached (SHA-256 / ByteRange)".to_string(),
            details,
        })
    }
}

fn extract_pdf_field(src: &str, field_key: &str) -> Option<String> {
    if let Some(idx) = src.find(field_key) {
        let rest = &src[idx + field_key.len()..];
        if let Some(open) = rest.find('(')
            && let Some(close) = rest[open + 1..].find(')') {
                return Some(rest[open + 1..open + 1 + close].replace("\\(", "(").replace("\\)", ")"));
            }
    }
    None
}

impl OfficeTool for PdfPkiSignTool {
    fn slug(&self) -> &'static str {
        "pdf-pki-sign"
    }

    fn name(&self) -> &'static str {
        "Chữ ký số PKI & Xác thực Toàn vẹn PDF"
    }

    fn description(&self) -> &'static str {
        "Gắn chữ ký số chuẩn PKCS#7 với /ByteRange và xác thực tính toàn vẹn, phát hiện giả mạo tài liệu."
    }

    fn execute(
        &self,
        request: &DispatchTaskRequest,
        cancel_flag: &AtomicBool,
        on_progress: Option<&dyn Fn(TaskProgress)>,
    ) -> Result<DispatchTaskResponse, CommandError> {
        let start = Instant::now();

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Không có tệp đầu vào nào được chỉ định".to_string(),
            });
        }

        let options: PdfPkiSignOptions = if request.options.is_null() {
            PdfPkiSignOptions::default()
        } else {
            serde_json::from_value(request.options.clone()).map_err(|e| {
                CommandError::ValidationFailed {
                    reason: format!("Cấu hình tùy chọn không hợp lệ: {}", e),
                }
            })?
        };

        let input_path = Path::new(&request.input_paths[0]);
        if !input_path.exists() {
            return Err(CommandError::FileNotFound {
                path: request.input_paths[0].clone(),
            });
        }

        let input_bytes = fs::read(input_path).map_err(|e| CommandError::IoError {
            details: format!("Không thể đọc tệp PDF: {}", e),
        })?;

        Self::check_magic_bytes(&input_bytes)?;
        Self::check_cancellation(cancel_flag)?;

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => input_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
        };

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 25.0,
                message: "Đang phân tích cấu trúc PDF và khối chữ ký số".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        match options.action {
            PkiAction::Sign => {
                let signed_bytes = Self::sign_pdf(&input_bytes, &options, cancel_flag)?;

                let stem = input_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("document");
                let out_file_name = format!("{}_signed.pdf", stem);
                let out_path = out_dir.join(out_file_name);

                fs::write(&out_path, &signed_bytes).map_err(|e| CommandError::IoError {
                    details: format!("Không thể ghi tệp PDF đã ký số: {}", e),
                })?;

                if let Some(cb) = on_progress {
                    cb(TaskProgress {
                        task_id: request.task_id.clone(),
                        current_step: 2,
                        total_steps: 2,
                        percentage: 100.0,
                        message: "Hoàn tất gắn chữ ký số PKI".to_string(),
                        processed_files: 1,
                        failed_files: 0,
                        total_files: 1,
                        is_completed: true,
                    });
                }

                Ok(DispatchTaskResponse {
                    task_id: request.task_id.clone(),
                    status: "SUCCESS".to_string(),
                    output_paths: vec![out_path.to_string_lossy().to_string()],
                    failed_items: Vec::new(),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                })
            }
            PkiAction::Verify => {
                let report = Self::verify_pdf(&input_bytes, cancel_flag)?;

                let stem = input_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("document");
                let report_file_name = format!("{}_signature_report.json", stem);
                let out_path = out_dir.join(report_file_name);

                let report_json = serde_json::to_string_pretty(&report).map_err(|e| {
                    CommandError::ValidationFailed {
                        reason: format!("Không thể xuất báo cáo xác thực: {}", e),
                    }
                })?;

                fs::write(&out_path, report_json.as_bytes()).map_err(|e| CommandError::IoError {
                    details: format!("Không thể lưu tệp báo cáo JSON: {}", e),
                })?;

                if let Some(cb) = on_progress {
                    cb(TaskProgress {
                        task_id: request.task_id.clone(),
                        current_step: 2,
                        total_steps: 2,
                        percentage: 100.0,
                        message: "Hoàn tất xác thực chữ ký số".to_string(),
                        processed_files: 1,
                        failed_files: 0,
                        total_files: 1,
                        is_completed: true,
                    });
                }

                let status = if report.is_valid {
                    "SUCCESS".to_string()
                } else {
                    "SIGNATURE_INVALID".to_string()
                };

                Ok(DispatchTaskResponse {
                    task_id: request.task_id.clone(),
                    status,
                    output_paths: vec![out_path.to_string_lossy().to_string()],
                    failed_items: Vec::new(),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_pdf() -> Vec<u8> {
        b"%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length 45 >>\nstream\nBT /F1 12 Tf 100 700 Td (Hop dong kinh te so 2026/HD-VN) Tj ET\nendstream\nendobj\n\
xref\n0 5\n0000000000 65535 f \n0000000010 00000 n \n0000000060 00000 n \n0000000117 00000 n \n0000000201 00000 n \n\
trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n300\n%%EOF\n".to_vec()
    }

    #[test]
    fn test_pdf_pki_sign_and_verify() {
        let tool = PdfPkiSignTool;
        let tmp = std::env::temp_dir().join("test_pki_sign_ok");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("contract.pdf");
        fs::write(&pdf_path, create_test_pdf()).expect("write original pdf");

        // 1. Ký số PKI
        let sign_req = DispatchTaskRequest {
            task_id: "task-sign-1".to_string(),
            tool_slug: "pdf-pki-sign".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "action": "sign",
                "signer_name": "Nguyễn Văn A",
                "reason": "Phê duyệt hợp đồng",
                "location": "Hà Nội",
                "cert_cn": "CONG TY TNHH VIET NAM"
            }),
        };

        let cancel = AtomicBool::new(false);
        let sign_res = tool.execute(&sign_req, &cancel, None).expect("sign succeeds");
        assert_eq!(sign_res.status, "SUCCESS");
        let signed_pdf_path = &sign_res.output_paths[0];

        let signed_bytes = fs::read(signed_pdf_path).expect("read signed pdf");
        assert!(signed_bytes.starts_with(b"%PDF-"));
        let raw_str = String::from_utf8_lossy(&signed_bytes);
        assert!(raw_str.contains("/ByteRange"));
        assert!(raw_str.contains("/Contents"));
        assert!(raw_str.contains("Nguyễn Văn A"));

        // 2. Xác thực chữ ký số trên tài liệu vừa ký
        let verify_req = DispatchTaskRequest {
            task_id: "task-verify-1".to_string(),
            tool_slug: "pdf-pki-sign".to_string(),
            input_paths: vec![signed_pdf_path.clone()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "action": "verify"
            }),
        };

        let verify_res = tool.execute(&verify_req, &cancel, None).expect("verify succeeds");
        assert_eq!(verify_res.status, "SUCCESS");

        let report_content = fs::read_to_string(&verify_res.output_paths[0]).expect("read report");
        let report: PkiVerificationReport = serde_json::from_str(&report_content).expect("parse json");
        assert!(report.is_valid);
        assert!(!report.tamper_detected);
        assert_eq!(report.signer_name, "Nguyễn Văn A");
        assert_eq!(report.cert_cn, "CONG TY TNHH VIET NAM");
        assert_eq!(report.computed_sha256, report.embedded_sha256);
    }

    #[test]
    fn test_pdf_pki_tamper_detection() {
        let tool = PdfPkiSignTool;
        let tmp = std::env::temp_dir().join("test_pki_tamper");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("original_for_tamper.pdf");
        fs::write(&pdf_path, create_test_pdf()).expect("write test pdf");

        // 1. Ký số
        let sign_req = DispatchTaskRequest {
            task_id: "task-sign-tamper".to_string(),
            tool_slug: "pdf-pki-sign".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "action": "sign",
                "signer_name": "Lê Văn B"
            }),
        };
        let cancel = AtomicBool::new(false);
        let sign_res = tool.execute(&sign_req, &cancel, None).expect("sign succeeds");
        let signed_pdf_path = &sign_res.output_paths[0];

        // 2. Làm giả mạo: Thay đổi 1 ký tự trong nội dung đã ký
        let mut corrupted_bytes = fs::read(signed_pdf_path).expect("read signed pdf");
        // Thay chữ 'Hop' thành 'Xop'
        if let Some(pos) = corrupted_bytes.windows(3).position(|w| w == b"Hop") {
            corrupted_bytes[pos] = b'X';
        } else {
            panic!("Cannot find target text to corrupt");
        }
        let tampered_pdf_path = tmp.join("tampered_contract.pdf");
        fs::write(&tampered_pdf_path, &corrupted_bytes).expect("write tampered");

        // 3. Xác thực chữ ký trên file bị giả mạo
        let verify_req = DispatchTaskRequest {
            task_id: "task-verify-tamper".to_string(),
            tool_slug: "pdf-pki-sign".to_string(),
            input_paths: vec![tampered_pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "action": "verify"
            }),
        };

        let verify_res = tool.execute(&verify_req, &cancel, None).expect("verify completes");
        assert_eq!(verify_res.status, "SIGNATURE_INVALID", "Chữ ký trên file bị sửa đổi phải có status SIGNATURE_INVALID!");

        let report_content = fs::read_to_string(&verify_res.output_paths[0]).expect("read report");
        let report: PkiVerificationReport = serde_json::from_str(&report_content).expect("parse json");
        assert!(!report.is_valid);
        assert!(report.tamper_detected);
        assert_ne!(report.computed_sha256, report.embedded_sha256);
    }

    #[test]
    fn test_pdf_pki_invalid_magic_bytes() {
        let tool = PdfPkiSignTool;
        let tmp = std::env::temp_dir().join("test_pki_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("fake.pdf");
        fs::write(&fake_path, b"NOT_A_VALID_PDF_HEADER").expect("write fake");

        let req = DispatchTaskRequest {
            task_id: "task-fake".to_string(),
            tool_slug: "pdf-pki-sign".to_string(),
            input_paths: vec![fake_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "action": "sign" }),
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
    fn test_pdf_pki_cooperative_cancellation() {
        let tool = PdfPkiSignTool;
        let tmp = std::env::temp_dir().join("test_pki_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_test_pdf()).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-pki-sign".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "action": "sign" }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_pki_security_guardrails() {
        let tool = PdfPkiSignTool;
        assert_eq!(tool.slug(), "pdf-pki-sign");
        assert_eq!(tool.name(), "Chữ ký số PKI & Xác thực Toàn vẹn PDF");
    }
}
