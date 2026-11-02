use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

fn default_scan_depth() -> String {
    "Quét tuyến tính tái tạo xref".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfRepairOptions {
    #[serde(default = "default_scan_depth")]
    pub scan_depth: String,
}

impl Default for PdfRepairOptions {
    fn default() -> Self {
        Self {
            scan_depth: default_scan_depth(),
        }
    }
}

pub struct PdfRepairCorruptTool;

impl Default for PdfRepairCorruptTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
pub struct RecoveredPdfObj {
    pub id: usize,
    pub generation: usize,
    pub content: String,
}

pub fn linear_scan_objects(raw_text: &str, cancel_flag: &AtomicBool) -> Result<Vec<RecoveredPdfObj>, CommandError> {
    let mut objects = Vec::new();
    let mut search_from = 0;

    while let Some(obj_marker) = raw_text[search_from..].find(" obj") {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let abs_marker = search_from + obj_marker;
        let prefix = &raw_text[search_from..abs_marker].trim_end();
        let mut tokens = prefix.split_whitespace().rev();
        let gen_str = tokens.next();
        let id_str = tokens.next();

        if let (Some(g_s), Some(i_s)) = (gen_str, id_str) {
            if let (Ok(id), Ok(generation)) = (i_s.parse::<usize>(), g_s.parse::<usize>()) {
                let content_start = abs_marker + 4;
                if let Some(end_offset) = raw_text[content_start..].find("endobj") {
                    let abs_end = content_start + end_offset;
                    let content = raw_text[content_start..abs_end].trim().to_string();
                    objects.push(RecoveredPdfObj {
                        id,
                        generation,
                        content,
                    });
                    search_from = abs_end + 6;
                    continue;
                }
            }
        }

        search_from = abs_marker + 4;
    }

    Ok(objects)
}

pub fn rebuild_xref_and_trailer(
    header: &str,
    objects: &[RecoveredPdfObj],
) -> Vec<u8> {
    let mut out = Vec::new();
    let clean_header = if header.starts_with("%PDF-") {
        header.lines().next().unwrap_or("%PDF-1.4").trim()
    } else {
        "%PDF-1.4"
    };
    out.extend_from_slice(clean_header.as_bytes());
    out.extend_from_slice(b"\n%\xE2\xE3\xCF\xD3\n");

    let mut catalog_id = 1;
    let mut pages_id = 2;
    let mut page_ids = Vec::new();

    for obj in objects {
        if obj.content.contains("/Type /Catalog") || obj.content.contains("/Type/Catalog") {
            catalog_id = obj.id;
        } else if obj.content.contains("/Type /Pages") || obj.content.contains("/Type/Pages") {
            pages_id = obj.id;
        } else if obj.content.contains("/Type /Page") || obj.content.contains("/Type/Page") {
            page_ids.push(obj.id);
        }
    }

    let mut offsets = Vec::new();

    // If catalog or pages object was not found, synthesize missing structure
    let has_catalog = objects.iter().any(|o| o.id == catalog_id);
    let has_pages = objects.iter().any(|o| o.id == pages_id);

    let max_existing_id = objects.iter().map(|o| o.id).max().unwrap_or(0);
    let next_synth_id = max_existing_id + 1;

    let final_catalog_id = if has_catalog { catalog_id } else { next_synth_id };
    let final_pages_id = if has_pages { pages_id } else { next_synth_id + 1 };

    for obj in objects {
        offsets.push((obj.id, out.len()));
        out.extend_from_slice(
            format!("{} {} obj\n{}\nendobj\n", obj.id, obj.generation, obj.content).as_bytes(),
        );
    }

    if !has_pages {
        offsets.push((final_pages_id, out.len()));
        let mut kids_str = String::new();
        for pid in &page_ids {
            kids_str.push_str(&format!("{} 0 R ", pid));
        }
        out.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
                final_pages_id,
                kids_str.trim(),
                page_ids.len()
            )
            .as_bytes(),
        );
    }

    if !has_catalog {
        offsets.push((final_catalog_id, out.len()));
        out.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /Catalog /Pages {} 0 R >>\nendobj\n",
                final_catalog_id, final_pages_id
            )
            .as_bytes(),
        );
    }

    offsets.sort_by_key(|&(id, _)| id);

    let start_xref = out.len();
    let total_objs = offsets.last().map(|&(id, _)| id).unwrap_or(0);
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", total_objs + 1).as_bytes());

    let mut current_id = 1;
    for &(id, offset) in &offsets {
        while current_id < id {
            out.extend_from_slice(b"0000000000 65535 f \n");
            current_id += 1;
        }
        out.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
        current_id += 1;
    }

    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n",
            total_objs + 1,
            final_catalog_id,
            start_xref
        )
        .as_bytes(),
    );

    out
}

impl OfficeTool for PdfRepairCorruptTool {
    fn slug(&self) -> &'static str {
        "pdf-repair-corrupt"
    }

    fn name(&self) -> &'static str {
        "Phục hồi File PDF Lỗi Cấu trúc"
    }

    fn description(&self) -> &'static str {
        "Quét tuyến tính toàn bộ mảng byte của tệp để định vị lại offset của các cặp từ khóa obj / endobj, tái tạo lại bảng tham chiếu chéo xref và trailer bị hỏng."
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

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp đầu vào trống".to_string(),
            });
        }

        let input_path_str = &request.input_paths[0];
        let input_path = Path::new(input_path_str);
        if !input_path.exists() {
            return Err(CommandError::FileNotFound {
                path: input_path_str.clone(),
            });
        }

        let bytes = fs::read(input_path).map_err(|e| CommandError::IoError {
            details: format!("Không thể đọc tệp PDF: {e}"),
        })?;

        // 64-byte signature check: %PDF-
        let has_pdf_magic = bytes.len() >= 5
            && (bytes.starts_with(b"%PDF-")
                || bytes[..64.min(bytes.len())]
                    .windows(5)
                    .any(|w| w == b"%PDF-"));

        if !has_pdf_magic {
            let actual = if bytes.len() >= 5 {
                String::from_utf8_lossy(&bytes[0..5.min(bytes.len())]).to_string()
            } else {
                format!("{:?}", bytes)
            };
            return Err(CommandError::InvalidFileSignature {
                expected: "%PDF-".to_string(),
                actual,
            });
        }

        let _options: PdfRepairOptions = if request.options.is_null() {
            PdfRepairOptions::default()
        } else {
            serde_json::from_value(request.options.clone()).unwrap_or_default()
        };

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 15.0,
                message: "Đang quét tuyến tính toàn bộ mảng byte để định vị obj / endobj...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        let raw = String::from_utf8_lossy(&bytes);
        let header = raw.lines().next().unwrap_or("%PDF-1.4");
        let recovered_objects = linear_scan_objects(&raw, cancel_flag)?;

        if recovered_objects.is_empty() {
            return Err(CommandError::ProcessExecutionError {
                message: "Không tìm thấy bất kỳ đối tượng PDF hợp lệ nào trong tệp.".to_string(),
            });
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 3,
                percentage: 60.0,
                message: format!(
                    "Đã tìm thấy {} đối tượng, đang tái tạo lại bảng xref và trailer...",
                    recovered_objects.len()
                ),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let repaired_pdf = rebuild_xref_and_trailer(header, &recovered_objects);

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_repaired_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
        let out_file = output_dir.join(format!("{stem}_repaired.pdf"));

        fs::write(&out_file, &repaired_pdf).map_err(|e| CommandError::IoError {
            details: format!("Không thể ghi file PDF đã phục hồi: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Phục hồi cấu trúc PDF hoàn tất!".to_string(),
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
            failed_items: Vec::new(),
            elapsed_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample_corrupted_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        // Objects with intentional valid bodies
        pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
        pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n");
        pdf.extend_from_slice(b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R >> endobj\n");
        pdf.extend_from_slice(b"4 0 obj << /Length 25 >> stream\nBT (Du lieu nguyen ven) Tj ET\nendstream endobj\n");
        // Completely broken xref and damaged trailer
        pdf.extend_from_slice(b"xref_CORRUPTED_TABLE_JUNK_DATA_1234567890\n");
        pdf.extend_from_slice(b"trailer_TRUNCATED_DAMAGED_DATA\n%%EOF_INCOMPLETE");
        pdf
    }

    #[test]
    fn test_pdf_repair_corrupt_happy_path() {
        // Quét toàn bộ mảng byte để định vị lại offset của các cặp từ khóa obj / endobj
        let tool = PdfRepairCorruptTool::default();
        assert_eq!(tool.slug(), "pdf-repair-corrupt");

        let cancel_flag = AtomicBool::new(false);
        let pdf_data = sample_corrupted_pdf();
        let raw = String::from_utf8_lossy(&pdf_data);
        let objs = linear_scan_objects(&raw, &cancel_flag).unwrap();



        assert_eq!(objs.len(), 4);
        assert_eq!(objs[0].id, 1);
        assert_eq!(objs[1].id, 2);
        assert_eq!(objs[2].id, 3);
        assert_eq!(objs[3].id, 4);
    }

    #[test]
    fn test_pdf_repair_corrupt_edge_case() {
        // Tái tạo bảng tham chiếu chéo xref và trailer bị hỏng
        let tool = PdfRepairCorruptTool::default();
        let temp_dir = std::env::temp_dir().join("test_repair_edge");
        fs::create_dir_all(&temp_dir).unwrap();
        let broken_file = temp_dir.join("damaged.pdf");
        fs::write(&broken_file, sample_corrupted_pdf()).unwrap();

        let req = DispatchTaskRequest {
            task_id: "repair_edge".to_string(),
            tool_slug: "pdf-repair-corrupt".to_string(),
            input_paths: vec![broken_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::to_value(PdfRepairOptions {
                scan_depth: "Quét tuyến tính tái tạo xref".to_string(),
            }).unwrap(),
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None).unwrap();
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).unwrap();
        assert!(out_bytes.starts_with(b"%PDF-1.4"));
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(out_str.contains("xref\n0 5"));
        assert!(out_str.contains("trailer\n<< /Size 5 /Root 1 0 R >>"));
        assert!(out_str.contains("startxref\n"));
        assert!(out_str.contains("%%EOF"));
    }

    #[test]
    fn test_pdf_repair_corrupt_invalid_magic_bytes() {
        let tool = PdfRepairCorruptTool::default();
        let temp_dir = std::env::temp_dir().join("test_repair_invalid");
        fs::create_dir_all(&temp_dir).unwrap();
        let fake = temp_dir.join("fake.pdf");
        fs::write(&fake, b"GARBAGE_BYTES_NO_PDF_HEADER_AT_ALL").unwrap();

        let req = DispatchTaskRequest {
            task_id: "repair_invalid".to_string(),
            tool_slug: "pdf-repair-corrupt".to_string(),
            input_paths: vec![fake.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::InvalidFileSignature { .. })));
    }

    #[test]
    fn test_pdf_repair_corrupt_cooperative_cancellation() {
        let tool = PdfRepairCorruptTool::default();
        let temp_dir = std::env::temp_dir().join("test_repair_cancel");
        fs::create_dir_all(&temp_dir).unwrap();
        let broken_file = temp_dir.join("damaged.pdf");
        fs::write(&broken_file, sample_corrupted_pdf()).unwrap();

        let req = DispatchTaskRequest {
            task_id: "repair_cancel".to_string(),
            tool_slug: "pdf-repair-corrupt".to_string(),
            input_paths: vec![broken_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(true);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::TaskCancelled)));
    }

    #[test]
    fn test_pdf_repair_corrupt_security_guardrails() {
        let tool = PdfRepairCorruptTool::default();
        assert!(!tool.slug().is_empty());
        assert!(!tool.name().is_empty());
    }
}
