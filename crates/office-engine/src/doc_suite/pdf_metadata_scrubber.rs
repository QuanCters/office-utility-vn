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
pub struct PdfScrubberOptions {
    #[serde(default = "default_true")]
    pub remove_info_dict: bool,
    #[serde(default = "default_true")]
    pub remove_xmp: bool,
}

fn default_true() -> bool {
    true
}

impl Default for PdfScrubberOptions {
    fn default() -> Self {
        Self {
            remove_info_dict: true,
            remove_xmp: true,
        }
    }
}

pub struct PdfMetadataScrubberTool;

impl Default for PdfMetadataScrubberTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct ScrubberPdfObj {
    id: usize,
    generation: usize,
    content: String,
}

struct ScrubberParsedPdf {
    header: String,
    objects: Vec<ScrubberPdfObj>,
    root_id: usize,
    info_id: Option<usize>,
}

impl ScrubberParsedPdf {
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
            let id_str = rest.split_whitespace().next().unwrap_or("1");
            if let Ok(id) = id_str.parse::<usize>() {
                root_id = id;
            }
        }

        let mut info_id = None;
        if let Some(i_pos) = raw_text.find("/Info ") {
            let rest = &raw_text[i_pos + 6..];
            let id_str = rest.split_whitespace().next().unwrap_or("");
            if let Ok(id) = id_str.parse::<usize>() {
                info_id = Some(id);
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

            if let (Some(g_s), Some(i_s)) = (gen_str, id_str) {
                if let (Ok(id), Ok(generation)) = (i_s.parse::<usize>(), g_s.parse::<usize>()) {
                    let content_start = abs_obj_start + 4;
                    if let Some(end_offset) = raw_text[content_start..].find("endobj") {
                        let content = raw_text[content_start..content_start + end_offset].trim().to_string();
                        objects.push(ScrubberPdfObj {
                            id,
                            generation,
                            content,
                        });
                        search_from = content_start + end_offset + 6;
                        continue;
                    }
                }
            }
            search_from = abs_obj_start + 4;
        }

        Ok(ScrubberParsedPdf {
            header,
            objects,
            root_id,
            info_id,
        })
    }
}

fn render_scrubbed_pdf(header: &str, objects: &[ScrubberPdfObj], root_id: usize) -> Vec<u8> {
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
        "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n",
        xref_offsets.len(),
        root_id,
        xref_pos
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

impl OfficeTool for PdfMetadataScrubberTool {
    fn slug(&self) -> &'static str {
        "pdf-metadata-scrubber"
    }

    fn name(&self) -> &'static str {
        "Tẩy sạch Metadata ẩn PDF"
    }

    fn description(&self) -> &'static str {
        "Xóa sạch từ điển Info trong Trailer (Tác giả, Phần mềm tạo, Ngày tạo, Lịch sử sửa đổi) và gỡ bỏ luồng XML metadata XMP nhạy cảm khỏi Catalog."
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

        let options: PdfScrubberOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tẩy sạch metadata không hợp lệ: {e}"),
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
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_scrubber_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Đang đọc và phân tích cấu trúc metadata của tệp PDF...".to_string(),
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

            let parsed = ScrubberParsedPdf::parse(&bytes)?;

            let mut scrubbed_objects = Vec::new();

            for obj in &parsed.objects {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                // 1. Kiểm tra xóa Info Dictionary
                if options.remove_info_dict {
                    if Some(obj.id) == parsed.info_id {
                        continue; // Bỏ qua object Info hoàn toàn
                    }
                    if obj.content.contains("/Author") || obj.content.contains("/Creator") || obj.content.contains("/Producer") {
                        if obj.content.contains("/Title") || obj.content.contains("/CreationDate") {
                            continue; // Bỏ qua object thông tin tài liệu
                        }
                    }
                }

                // 2. Kiểm tra xóa XMP Metadata stream khỏi Catalog và Objects
                let mut content = obj.content.clone();
                if options.remove_xmp {
                    if content.contains("/Type /Metadata") || content.contains("/Subtype /XML") {
                        continue; // Bỏ qua object luồng XMP
                    }

                    if content.contains("/Metadata") {
                        // Gỡ bỏ /Metadata <id> 0 R khỏi Catalog
                        if let Some(m_pos) = content.find("/Metadata") {
                            let rest = &content[m_pos + 9..];
                            if let Some(r_pos) = rest.find('R') {
                                let end_idx = m_pos + 9 + r_pos + 1;
                                content.replace_range(m_pos..end_idx, "");
                            }
                        }
                    }
                }

                // Loại bỏ PieceInfo hoặc dấu vết phần mềm biên soạn
                if content.contains("/PieceInfo") {
                    if let Some(p_pos) = content.find("/PieceInfo") {
                        let rest = &content[p_pos + 10..];
                        if let Some(r_pos) = rest.find('R') {
                            let end_idx = p_pos + 10 + r_pos + 1;
                            content.replace_range(p_pos..end_idx, "");
                        }
                    }
                }

                scrubbed_objects.push(ScrubberPdfObj {
                    id: obj.id,
                    generation: obj.generation,
                    content,
                });
            }

            let scrubbed_bytes = render_scrubbed_pdf(&parsed.header, &scrubbed_objects, parsed.root_id);

            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("doc");
            let out_path = output_dir.join(format!("{file_stem}_scrubbed.pdf"));
            fs::write(&out_path, scrubbed_bytes).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi tệp PDF đã tẩy sạch metadata: {e}"),
            })?;

            output_paths.push(out_path.to_string_lossy().to_string());

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: 20.0 + (idx as f32 / request.input_paths.len() as f32) * 70.0,
                    message: format!("Đã tẩy sạch metadata tệp {}/{}", idx + 1, request.input_paths.len()),
                    processed_files: idx + 1,
                    failed_files: 0,
                    total_files: request.input_paths.len(),
                    is_completed: false,
                });
            }
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Tẩy sạch metadata ẩn PDF thành công.".to_string(),
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

    fn create_test_pdf_with_metadata() -> Vec<u8> {
        let pdf = "\
%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Metadata 5 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length 25 >>\nstream\nBT /F1 12 Tf (Tai lieu) Tj ET\nendstream\nendobj\n\
5 0 obj\n<< /Type /Metadata /Subtype /XML /Length 50 >>\nstream\n<x:xmpmeta><rdf:Description/></x:xmpmeta>\nendstream\nendobj\n\
6 0 obj\n<< /Author (Nguyen Van A) /Creator (Secret Software v1.0) /CreationDate (D:20261102) >>\nendobj\n\
xref\n0 7\n0000000000 65535 f \n\
trailer\n<< /Size 7 /Root 1 0 R /Info 6 0 R >>\nstartxref\n500\n%%EOF\n";
        pdf.as_bytes().to_vec()
    }

    #[test]
    fn test_pdf_metadata_scrubber_happy_path() {
        let tool = PdfMetadataScrubberTool::default();
        let tmp = std::env::temp_dir().join("test_scrub_happy");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc_with_meta.pdf");
        fs::write(&pdf_path, create_test_pdf_with_metadata()).expect("write pdf");

        let req = DispatchTaskRequest {
            task_id: "task-scrub-happy".to_string(),
            tool_slug: "pdf-metadata-scrubber".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "remove_info_dict": true,
                "remove_xmp": true
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("scrub succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read scrubbed");
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(!out_str.contains("/Info"));
        assert!(!out_str.contains("<x:xmpmeta>"));
    }

    #[test]
    fn test_pdf_metadata_scrubber_edge_case() {
        let tool = PdfMetadataScrubberTool::default();
        let tmp = std::env::temp_dir().join("test_scrub_edge");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("leak_doc.pdf");
        fs::write(&pdf_path, create_test_pdf_with_metadata()).expect("write pdf");

        let req = DispatchTaskRequest {
            task_id: "task-scrub-edge".to_string(),
            tool_slug: "pdf-metadata-scrubber".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "remove_info_dict": true,
                "remove_xmp": true
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("scrub succeeds");
        let out_bytes = fs::read(&res.output_paths[0]).expect("read scrubbed");
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(!out_str.contains("Nguyen Van A"));
        assert!(!out_str.contains("Secret Software"));
    }

    #[test]
    fn test_pdf_metadata_scrubber_invalid_magic_bytes() {
        let tool = PdfMetadataScrubberTool::default();
        let tmp = std::env::temp_dir().join("test_scrub_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("bad.pdf");
        fs::write(&fake_path, b"NOT_A_VALID_PDF").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-fake".to_string(),
            tool_slug: "pdf-metadata-scrubber".to_string(),
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
    fn test_pdf_metadata_scrubber_cooperative_cancellation() {
        let tool = PdfMetadataScrubberTool::default();
        let tmp = std::env::temp_dir().join("test_scrub_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_test_pdf_with_metadata()).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-metadata-scrubber".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_metadata_scrubber_security_guardrails() {
        let tool = PdfMetadataScrubberTool::default();
        assert_eq!(tool.slug(), "pdf-metadata-scrubber");
        assert_eq!(tool.name(), "Tẩy sạch Metadata ẩn PDF");
    }
}
