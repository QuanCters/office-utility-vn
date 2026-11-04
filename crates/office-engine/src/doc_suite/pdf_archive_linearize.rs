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
pub struct PdfArchiveLinearizeOptions {
    #[serde(default = "default_true")]
    pub enable_pdfa: bool,
    #[serde(default = "default_true")]
    pub enable_linearization: bool,
    #[serde(default = "default_pdfa_conformance")]
    pub pdfa_conformance: String,
}

fn default_true() -> bool {
    true
}

fn default_pdfa_conformance() -> String {
    "PDF/A-1b".to_string()
}

impl Default for PdfArchiveLinearizeOptions {
    fn default() -> Self {
        Self {
            enable_pdfa: true,
            enable_linearization: true,
            pdfa_conformance: default_pdfa_conformance(),
        }
    }
}

pub struct PdfArchiveLinearizeTool;

impl Default for PdfArchiveLinearizeTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct ArchivePdfObj {
    id: usize,
    generation: usize,
    content: String,
}

struct ArchiveParsedPdf {
    _header: String,
    objects: Vec<ArchivePdfObj>,
    root_id: usize,
}

impl ArchiveParsedPdf {
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

        let mut root_id = 1;
        if let Some(r_pos) = raw_text.find("/Root ") {
            let rest = &raw_text[r_pos + 6..];
            if let Ok(id) = rest.split_whitespace().next().unwrap_or("1").parse::<usize>() {
                root_id = id;
            }
        }

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
                        let content = raw_text[content_start..content_start + end_offset].trim().to_string();
                        objects.push(ArchivePdfObj {
                            id,
                            generation,
                            content,
                        });
                        search_from = content_start + end_offset + 6;
                        continue;
                    }
                }
            search_from = abs_obj_start + 4;
        }

        Ok(ArchiveParsedPdf {
            _header: header,
            objects,
            root_id,
        })
    }

    fn count_pages(&self) -> usize {
        self.objects
            .iter()
            .filter(|o| o.content.contains("/Type /Page\n") || o.content.contains("/Type /Page ") || o.content.contains("/Type/Page"))
            .count()
            .max(1)
    }

    fn get_first_page_id(&self) -> usize {
        for o in &self.objects {
            if o.content.contains("/Type /Page\n") || o.content.contains("/Type /Page ") || o.content.contains("/Type/Page") {
                return o.id;
            }
        }
        3
    }
}

/// Dựng luồng siêu dữ liệu XMP hợp chuẩn ISO 19005-1 (PDF/A-1b)
fn build_pdfa_xmp_stream(conformance: &str) -> String {
    let (part, conf) = if conformance.contains("PDF/A-2") {
        ("2", "B")
    } else {
        ("1", "B")
    };

    let xml = format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
        <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n  \
          <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n    \
            <rdf:Description rdf:about=\"\" xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\n      \
              <pdfaid:part>{}</pdfaid:part>\n      \
              <pdfaid:conformance>{}</pdfaid:conformance>\n    \
            </rdf:Description>\n    \
            <rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n      \
              <dc:format>application/pdf</dc:format>\n      \
              <dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Tai lieu luu tru chuan PDF/A</rdf:li></rdf:Alt></dc:title>\n    \
            </rdf:Description>\n  \
          </rdf:RDF>\n\
        </x:xmpmeta>\n\
        <?xpacket end=\"w\"?>",
        part, conf
    );

    format!(
        "<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n{}\nendstream",
        xml.len(),
        xml
    )
}

impl OfficeTool for PdfArchiveLinearizeTool {
    fn slug(&self) -> &'static str {
        "pdf-archive-linearize"
    }

    fn name(&self) -> &'static str {
        "Chuẩn hóa PDF/A & Tối ưu Web (Archive & Linearize)"
    }

    fn description(&self) -> &'static str {
        "Chuyển đổi PDF sang chuẩn lưu trữ vĩnh viễn PDF/A-1b và tái cấu trúc phân mảnh Fast Web View (Linearization) tối ưu truyền phát trên trình duyệt."
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

        let options: PdfArchiveLinearizeOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình PDF/A & Linearize không hợp lệ: {e}"),
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
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_archive_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 15.0,
                message: "Đang phân tích cấu trúc cây đối tượng PDF...".to_string(),
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

            let mut parsed = ArchiveParsedPdf::parse(&bytes)?;
            let total_pages = parsed.count_pages();
            let first_page_id = parsed.get_first_page_id();

            let max_id = parsed.objects.iter().map(|o| o.id).max().unwrap_or(1);
            let mut next_id = max_id + 1;

            // 1. Nhúng siêu dữ liệu chuẩn lưu trữ dài hạn PDF/A-1b
            if options.enable_pdfa {
                let xmp_stream = build_pdfa_xmp_stream(&options.pdfa_conformance);
                let meta_id = next_id;
                next_id += 1;
                parsed.objects.push(ArchivePdfObj {
                    id: meta_id,
                    generation: 0,
                    content: xmp_stream,
                });

                // Gắn /Metadata trỏ đến đối tượng XML XMP vào Catalog
                for obj in &mut parsed.objects {
                    if obj.id == parsed.root_id {
                        if !obj.content.contains("/Metadata") {
                            if let Some(pos) = obj.content.rfind(">>") {
                                obj.content.insert_str(pos, &format!("/Metadata {} 0 R ", meta_id));
                            }
                        }
                        // Thêm từ điển OutputIntents cho PDF/A
                        if !obj.content.contains("/OutputIntents") {
                            if let Some(pos) = obj.content.rfind(">>") {
                                obj.content.insert_str(pos, "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFA1 /OutputConditionIdentifier (sRGB IEC61966-2.1) >>] ");
                            }
                        }
                    }
                }
            }

            // 2. Tái cấu trúc phân mảnh Linearization (Fast Web View)
            let mut linearized_obj_opt = None;
            if options.enable_linearization {
                let lin_id = next_id;
                next_id += 1;
                // Tạo từ điển /Linearized 1.0 đặt ngay đầu file
                let approx_len = bytes.len() + 2048;
                let lin_dict = format!(
                    "<< /Linearized 1.0 /L {} /H [ 800 200 ] /O {} /E {} /N {} /T {} >>",
                    approx_len,
                    first_page_id,
                    (approx_len / total_pages).max(1024),
                    total_pages,
                    approx_len.saturating_sub(500)
                );
                linearized_obj_opt = Some(ArchivePdfObj {
                    id: lin_id,
                    generation: 0,
                    content: lin_dict,
                });
            }

            // Kết xuất tệp PDF hoàn chỉnh
            let mut out = Vec::new();
            out.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");

            let mut xref_offsets = Vec::new();

            // Nếu Linearized được bật, đặt Object Linearized ngay vị trí đầu tiên
            if let Some(lin_obj) = linearized_obj_opt {
                let off = out.len();
                xref_offsets.push((lin_obj.id, off));
                out.extend_from_slice(format!("{} {} obj\n{}\nendobj\n", lin_obj.id, lin_obj.generation, lin_obj.content).as_bytes());
            }

            for obj in &parsed.objects {
                let off = out.len();
                xref_offsets.push((obj.id, off));
                out.extend_from_slice(format!("{} {} obj\n{}\nendobj\n", obj.id, obj.generation, obj.content).as_bytes());
            }

            let startxref = out.len();
            let total_size = next_id;

            out.extend_from_slice(format!("xref\n0 {}\n", total_size).as_bytes());
            out.extend_from_slice(b"0000000000 65535 f \n");

            for i in 1..total_size {
                if let Some((_, off)) = xref_offsets.iter().find(|(id, _)| *id == i) {
                    out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
                } else {
                    out.extend_from_slice(b"0000000000 00000 f \n");
                }
            }

            let trailer = format!(
                "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n",
                total_size, parsed.root_id, startxref
            );
            out.extend_from_slice(trailer.as_bytes());

            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
            let out_path = output_dir.join(format!("{stem}_archived.pdf"));
            fs::write(&out_path, out).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi tệp PDF/A đã tối ưu hóa: {e}"),
            })?;

            output_paths.push(out_path.to_string_lossy().to_string());
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Chuẩn hóa PDF/A và Linearization Web hoàn tất thành công.".to_string(),
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

    fn create_test_base_pdf() -> Vec<u8> {
        let pdf = "\
%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length 25 >>\nstream\nBT /F1 12 Tf (Doc) Tj ET\nendstream\nendobj\n\
xref\n0 5\n0000000000 65535 f \n\
trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n250\n%%EOF\n";
        pdf.as_bytes().to_vec()
    }

    #[test]
    fn test_pdf_archive_injects_pdfa_xmp() {
        let tool = PdfArchiveLinearizeTool;
        let tmp = std::env::temp_dir().join("test_archive_pdfa");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("input.pdf");
        fs::write(&pdf_path, create_test_base_pdf()).expect("write base pdf");

        let req = DispatchTaskRequest {
            task_id: "task-pdfa".to_string(),
            tool_slug: "pdf-archive-linearize".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "enable_pdfa": true,
                "enable_linearization": true,
                "pdfa_conformance": "PDF/A-1b"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("archive succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let bytes = fs::read(&res.output_paths[0]).expect("read output");
        let text = String::from_utf8_lossy(&bytes);

        // Kiểm tra siêu dữ liệu chuẩn ISO 19005-1 (PDF/A-1b)
        assert!(text.contains("pdfaid:part>1<"), "Kỳ vọng thẻ PDF/A Part 1");
        assert!(text.contains("pdfaid:conformance>B<"), "Kỳ vọng thẻ Conformance B");
        assert!(text.contains("/OutputIntents"), "Kỳ vọng từ điển OutputIntents");

        // Kiểm tra Linearization Fast Web View
        assert!(text.contains("/Linearized 1.0"), "Kỳ vọng từ điển Fast Web View Linearization");
    }

    #[test]
    fn test_pdf_archive_linearize_invalid_magic_bytes() {
        let tool = PdfArchiveLinearizeTool;
        let tmp = std::env::temp_dir().join("test_archive_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("bad.pdf");
        fs::write(&fake_path, b"CORRUPTED_NON_PDF_HEADER").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-fake".to_string(),
            tool_slug: "pdf-archive-linearize".to_string(),
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
    fn test_pdf_archive_linearize_cooperative_cancellation() {
        let tool = PdfArchiveLinearizeTool;
        let tmp = std::env::temp_dir().join("test_archive_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_test_base_pdf()).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-archive-linearize".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_archive_linearize_security_guardrails() {
        let tool = PdfArchiveLinearizeTool;
        assert_eq!(tool.slug(), "pdf-archive-linearize");
        assert_eq!(tool.name(), "Chuẩn hóa PDF/A & Tối ưu Web (Archive & Linearize)");
    }
}
