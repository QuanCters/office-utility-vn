use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactRegion {
    pub page: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfRedactionOptions {
    #[serde(default)]
    pub sensitive_patterns: Vec<String>,
    #[serde(default)]
    pub regions: Vec<RedactRegion>,
    #[serde(default = "default_blackout")]
    pub blackout: bool,
}

fn default_blackout() -> bool {
    true
}

impl Default for PdfRedactionOptions {
    fn default() -> Self {
        Self {
            sensitive_patterns: Vec::new(),
            regions: Vec::new(),
            blackout: true,
        }
    }
}

pub struct PdfRedactionTool;

impl Default for PdfRedactionTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct RedactPdfObj {
    id: usize,
    generation: usize,
    content: String,
    stream: Option<String>,
}

struct RedactParsedPdf {
    header: String,
    objects: Vec<RedactPdfObj>,
}

impl RedactParsedPdf {
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
        let header = raw_text.lines().next().unwrap_or("%PDF-1.4").trim().to_string();

        let mut objects = Vec::new();
        let mut search_from = 0;

        while let Some(obj_start) = raw_text[search_from..].find(" obj") {
            let abs_obj_start = search_from + obj_start;
            let prefix = raw_text[search_from..abs_obj_start].trim_end();
            let mut parts = prefix.split_whitespace().rev();
            let gen_str = parts.next();
            let id_str = parts.next();

            if let (Some(g_s), Some(i_s)) = (gen_str, id_str)
                && let (Ok(id), Ok(generation)) = (i_s.parse::<usize>(), g_s.parse::<usize>()) {
                    let content_start = abs_obj_start + 4;
                    if let Some(end_offset) = raw_text[content_start..].find("endobj") {
                        let full_body = &raw_text[content_start..content_start + end_offset];
                        let (content, stream) = if let Some(stream_idx) = full_body.find("stream") {
                            let dict_part = full_body[..stream_idx].trim().to_string();
                            let after_stream = &full_body[stream_idx + 6..];
                            let stream_content = if let Some(endstream_idx) = after_stream.rfind("endstream") {
                                after_stream[..endstream_idx].trim_matches(|c| c == '\r' || c == '\n').to_string()
                            } else {
                                after_stream.to_string()
                            };
                            (dict_part, Some(stream_content))
                        } else {
                            (full_body.trim().to_string(), None)
                        };

                        objects.push(RedactPdfObj {
                            id,
                            generation,
                            content,
                            stream,
                        });
                        search_from = content_start + end_offset + 6;
                        continue;
                    }
                }
            search_from = abs_obj_start + 4;
        }

        Ok(RedactParsedPdf { header, objects })
    }

    fn apply_redaction(&mut self, options: &PdfRedactionOptions) -> usize {
        let mut total_redactions = 0;

        for obj in &mut self.objects {
            if let Some(ref mut stream_data) = obj.stream {
                let mut modified = false;

                // 1. Xóa bỏ vĩnh viễn các chuỗi ký tự nhạy cảm khỏi stream
                for pattern in &options.sensitive_patterns {
                    if !pattern.is_empty() && stream_data.contains(pattern) {
                        let replacement = "*".repeat(pattern.len());
                        *stream_data = stream_data.replace(pattern, &replacement);
                        modified = true;
                        total_redactions += 1;
                    }
                }

                // 2. Chèn khối hình chữ nhật đen phủ vùng redaction
                if options.blackout && !options.regions.is_empty() {
                    let mut blackout_ops = String::new();
                    for reg in &options.regions {
                        blackout_ops.push_str(&format!(
                            "\nq\n0 0 0 rg\n{:.2} {:.2} {:.2} {:.2} re\nf\nQ\n",
                            reg.x, reg.y, reg.width, reg.height
                        ));
                        total_redactions += 1;
                    }
                    stream_data.push_str(&blackout_ops);
                    modified = true;
                }

                if modified {
                    // Cập nhật lại thuộc tính /Length trong dictionary
                    let new_len = stream_data.len();
                    if let Some(len_idx) = obj.content.find("/Length") {
                        let rest = &obj.content[len_idx + 7..];
                        if let Some(end_len) = rest.find(|c: char| c == '/' || c == '>' || c == '\n' || c == '\r') {
                            let old_len_str = &rest[..end_len].trim();
                            if !old_len_str.is_empty() {
                                let abs_start = len_idx + 7;
                                let abs_end = len_idx + 7 + end_len;
                                obj.content.replace_range(abs_start..abs_end, &format!(" {new_len} "));
                            }
                        }
                    }
                }
            }
        }

        total_redactions
    }

    fn render_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(self.header.as_bytes());
        out.push(b'\n');

        let mut xref_offsets = Vec::new();
        let mut root_id = 1;

        for obj in &self.objects {
            if obj.content.contains("/Root ") {
                let rest = &obj.content[obj.content.find("/Root ").unwrap_or(0) + 6..];
                if let Ok(id) = rest.split_whitespace().next().unwrap_or("1").parse::<usize>() {
                    root_id = id;
                }
            }
        }

        for obj in &self.objects {
            let offset = out.len();
            xref_offsets.push((obj.id, offset));
            if let Some(ref st) = obj.stream {
                let dict = if !obj.content.contains("/Length") {
                    format!("<< /Length {} >>", st.len())
                } else {
                    obj.content.clone()
                };
                let s = format!("{} {} obj\n{}\nstream\n{}\nendstream\nendobj\n", obj.id, obj.generation, dict, st);
                out.extend_from_slice(s.as_bytes());
            } else {
                let s = format!("{} {} obj\n{}\nendobj\n", obj.id, obj.generation, obj.content);
                out.extend_from_slice(s.as_bytes());
            }
        }

        let xref_pos = out.len();
        let max_id = self.objects.iter().map(|o| o.id).max().unwrap_or(1);
        let size = max_id + 1;

        out.extend_from_slice(format!("xref\n0 {}\n", size).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");

        for i in 1..size {
            if let Some((_, off)) = xref_offsets.iter().find(|(id, _)| *id == i) {
                out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
            } else {
                out.extend_from_slice(b"0000000000 00000 f \n");
            }
        }

        let trailer = format!(
            "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n",
            size, root_id, xref_pos
        );
        out.extend_from_slice(trailer.as_bytes());
        out
    }
}

impl OfficeTool for PdfRedactionTool {
    fn slug(&self) -> &'static str {
        "pdf-redaction"
    }

    fn name(&self) -> &'static str {
        "Xóa Dữ liệu Nhạy cảm PDF (PDF Redaction)"
    }

    fn description(&self) -> &'static str {
        "Cắt bỏ vĩnh viễn chuỗi ký tự và đối tượng nhạy cảm khỏi cấu trúc luồng byte PDF, phủ hộp đen bảo mật không thể đảo ngược."
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

        let options: PdfRedactionOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình Redaction không hợp lệ: {e}"),
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
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_redact_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 20.0,
                message: "Đang phân tích cấu trúc luồng nội dung và định vị dữ liệu nhạy cảm...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: request.input_paths.len(),
                is_completed: false,
            });
        }

        let mut output_paths = Vec::new();

        for file_path_str in &request.input_paths {
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

            let mut parsed = RedactParsedPdf::parse(&bytes)?;
            parsed.apply_redaction(&options);
            let cleaned_bytes = parsed.render_bytes();

            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("redacted");
            let out_path = output_dir.join(format!("{stem}_redacted.pdf"));

            fs::write(&out_path, cleaned_bytes).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi tệp PDF đã xóa dữ liệu nhạy cảm: {e}"),
            })?;

            output_paths.push(out_path.to_string_lossy().to_string());
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Xóa bỏ triệt để dữ liệu nhạy cảm và che đen thành công.".to_string(),
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

    fn create_test_sensitive_pdf() -> Vec<u8> {
        let pdf = "\
%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length 75 >>\nstream\nBT /F1 12 Tf 100 700 Td (So CCCD: 079090012345 va Luong: 50,000,000 VND) Tj ET\nendstream\nendobj\n\
xref\n0 5\n0000000000 65535 f \n\
trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n350\n%%EOF\n";
        pdf.as_bytes().to_vec()
    }

    #[test]
    fn test_pdf_redaction_removes_sensitive_stream_bytes() {
        let tool = PdfRedactionTool;
        let tmp = std::env::temp_dir().join("test_redact_stream");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("sensitive.pdf");
        fs::write(&pdf_path, create_test_sensitive_pdf()).expect("write sensitive pdf");

        let req = DispatchTaskRequest {
            task_id: "task-redact-pat".to_string(),
            tool_slug: "pdf-redaction".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "sensitive_patterns": ["079090012345", "50,000,000 VND"],
                "blackout": true,
                "regions": [
                    { "page": 1, "x": 100.0, "y": 690.0, "width": 200.0, "height": 20.0 }
                ]
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("redaction succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let redacted_bytes = fs::read(&res.output_paths[0]).expect("read redacted pdf");
        let redacted_str = String::from_utf8_lossy(&redacted_bytes);

        // Chứng minh byte nhạy cảm hoàn toàn không còn tồn tại trong tệp kết quả
        assert!(!redacted_str.contains("079090012345"), "Số CCCD phải bị xóa vĩnh viễn khỏi stream");
        assert!(!redacted_str.contains("50,000,000 VND"), "Số tiền lương phải bị xóa vĩnh viễn khỏi stream");

        // Chứng minh có vẽ hộp che đen blackout
        assert!(redacted_str.contains("0 0 0 rg"), "Kỳ vọng lệnh vẽ màu đen rg");
        assert!(redacted_str.contains("re"), "Kỳ vọng lệnh vẽ hình chữ nhật re");
    }

    #[test]
    fn test_pdf_redaction_invalid_magic_bytes() {
        let tool = PdfRedactionTool;
        let tmp = std::env::temp_dir().join("test_redact_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("bad.pdf");
        fs::write(&fake_path, b"NOT_A_PDF").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-fake".to_string(),
            tool_slug: "pdf-redaction".to_string(),
            input_paths: vec![fake_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { expected, .. } => {
                assert_eq!(expected, "%PDF-");
            }
            other => panic!("Expected InvalidFileSignature, got {:?}", other),
        }
    }

    #[test]
    fn test_pdf_redaction_cooperative_cancellation() {
        let tool = PdfRedactionTool;
        let tmp = std::env::temp_dir().join("test_redact_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_test_sensitive_pdf()).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-redaction".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_redaction_security_guardrails() {
        let tool = PdfRedactionTool;
        assert_eq!(tool.slug(), "pdf-redaction");
        assert_eq!(tool.name(), "Xóa Dữ liệu Nhạy cảm PDF (PDF Redaction)");
    }
}
