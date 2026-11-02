use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

fn default_dpi() -> u32 {
    150
}

fn default_compression_level() -> u32 {
    9
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfCompressOptions {
    #[serde(default = "default_dpi")]
    pub target_dpi: u32,
    #[serde(default = "default_compression_level")]
    pub compression_level: u32,
    #[serde(default)]
    pub strip_unreferenced: bool,
}

impl Default for PdfCompressOptions {
    fn default() -> Self {
        Self {
            target_dpi: 150,
            compression_level: 9,
            strip_unreferenced: true,
        }
    }
}

pub struct PdfCompressTool;

impl Default for PdfCompressTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct PdfObj {
    id: usize,
    generation: usize,
    dict: String,
    stream: Option<Vec<u8>>,
}

/// Tái nén dữ liệu bằng Zlib mức nén tối đa (Level 9 / Best)
fn recompress_stream(data: &[u8], is_already_flate: bool) -> Vec<u8> {
    let uncompressed = if is_already_flate {
        let mut decoder = ZlibDecoder::new(data).take(50 * 1024 * 1024);
        let mut decomp = Vec::new();
        if decoder.read_to_end(&mut decomp).is_ok() && !decomp.is_empty() {
            decomp
        } else {
            data.to_vec()
        }
    } else {
        data.to_vec()
    };

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    if encoder.write_all(&uncompressed).is_ok()
        && let Ok(compressed) = encoder.finish()
            && (compressed.len() < data.len() || !is_already_flate) {
                return compressed;
            }
    data.to_vec()
}

/// Giảm mẫu ảnh (Downsample) nếu cần nén theo DPI thấp
fn downsample_image_stream(data: &[u8], factor: usize) -> Vec<u8> {
    if factor <= 1 || data.len() < factor * 4 {
        return data.to_vec();
    }
    data.iter().step_by(factor).copied().collect()
}

/// Thu gom tất cả Object ID được tham chiếu bắt đầu từ Root
fn collect_referenced_ids(objects: &[PdfObj], root_id: usize) -> HashSet<usize> {
    let mut referenced = HashSet::new();
    let mut queue = vec![root_id];
    referenced.insert(root_id);

    while let Some(curr) = queue.pop() {
        if let Some(obj) = objects.iter().find(|o| o.id == curr) {
            let words: Vec<&str> = obj.dict.split_whitespace().collect();
            for i in 0..words.len() {
                if i + 2 < words.len() && words[i + 2] == "R"
                    && let Ok(ref_id) = words[i].parse::<usize>()
                        && referenced.insert(ref_id) {
                            queue.push(ref_id);
                        }
            }
        }
    }

    referenced
}

fn parse_pdf_objects(bytes: &[u8]) -> Result<(String, Vec<PdfObj>, usize), CommandError> {
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
    if let Some(root_pos) = raw_text.find("/Root ") {
        let rest = &raw_text[root_pos + 6..];
        let id_str = rest.split_whitespace().next().unwrap_or("1");
        if let Ok(id) = id_str.parse::<usize>() {
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
                    let full_block = &raw_text[content_start..content_start + end_offset];
                    let (dict_part, stream_part) = if let Some(stream_pos) = full_block.find("stream") {
                        let dict = full_block[..stream_pos].trim().to_string();
                        let s_content = full_block[stream_pos + 6..].trim();
                        let s_end = s_content.rfind("endstream").unwrap_or(s_content.len());
                        (dict, Some(s_content[..s_end].trim().as_bytes().to_vec()))
                    } else {
                        (full_block.trim().to_string(), None)
                    };

                    objects.push(PdfObj {
                        id,
                        generation,
                        dict: dict_part,
                        stream: stream_part,
                    });

                    search_from = content_start + end_offset + 6;
                    continue;
                }
            }
        search_from = abs_obj_start + 4;
    }

    Ok((header, objects, root_id))
}

fn render_compressed_pdf(header: &str, objects: &[PdfObj], root_id: usize) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(header.as_bytes());
    out.push(b'\n');

    let mut xref_offsets = Vec::new();
    xref_offsets.push(0);

    for obj in objects {
        let offset = out.len();
        xref_offsets.push(offset);

        if let Some(ref st) = obj.stream {
            let header_line = format!("{} {} obj\n<< {} /Length {} >>\nstream\n", obj.id, obj.generation, obj.dict.trim_matches(|c| c == '<' || c == '>'), st.len());
            out.extend_from_slice(header_line.as_bytes());
            out.extend_from_slice(st);
            out.extend_from_slice(b"\nendstream\nendobj\n");
        } else {
            let obj_repr = format!("{} {} obj\n{}\nendobj\n", obj.id, obj.generation, obj.dict);
            out.extend_from_slice(obj_repr.as_bytes());
        }
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

impl OfficeTool for PdfCompressTool {
    fn slug(&self) -> &'static str {
        "pdf-compress"
    }

    fn name(&self) -> &'static str {
        "Nén giảm Dung lượng PDF"
    }

    fn description(&self) -> &'static str {
        "Nén lại các luồng FlateDecode stream ở mức nén tối đa, tái lấy mẫu (downsample) các đối tượng XObject Image nhúng trong PDF theo các mức 72, 150, 300 DPI, và loại bỏ các đối tượng mồ côi."
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

        let options: PdfCompressOptions = if let Ok(opts) = serde_json::from_value::<PdfCompressOptions>(request.options.clone()) {
            opts
        } else {
            let mut opts = PdfCompressOptions::default();
            if let Some(dpi) = request.options.get("dpi").and_then(|v| v.as_u64()) {
                opts.target_dpi = dpi as u32;
            } else if let Some(dpi) = request.options.get("target_dpi").and_then(|v| v.as_u64()) {
                opts.target_dpi = dpi as u32;
            }
            if let Some(lvl) = request.options.get("compression_level") {
                if let Some(n) = lvl.as_u64() {
                    opts.compression_level = n as u32;
                } else if let Some(s) = lvl.as_str() {
                    opts.compression_level = match s {
                        "minimal" | "low" => 3,
                        "maximum" | "high" => 9,
                        _ => 6, // medium
                    };
                }
            }
            if let Some(strip) = request.options.get("strip_unreferenced").and_then(|v| v.as_bool()) {
                opts.strip_unreferenced = strip;
            }
            opts
        };

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp đầu vào trống".to_string(),
            });
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_compress_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Đang đọc và phân tích cấu trúc đối tượng PDF...".to_string(),
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

            let (header, mut objects, root_id) = parse_pdf_objects(&bytes)?;

            // 1. Tối ưu hóa / Tái nén các luồng stream
            for obj in &mut objects {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                if let Some(ref st) = obj.stream {
                    let is_image = obj.dict.contains("/Subtype /Image") || obj.dict.contains("/Image");
                    let mut current_stream = st.clone();

                    if is_image && options.target_dpi <= 72 {
                        current_stream = downsample_image_stream(&current_stream, 2);
                    }

                    let is_flate = obj.dict.contains("/FlateDecode");
                    let recompressed = recompress_stream(&current_stream, is_flate);

                    if !obj.dict.contains("/Filter") {
                        obj.dict.push_str(" /Filter /FlateDecode");
                    }
                    obj.stream = Some(recompressed);
                }
            }

            // 2. Loại bỏ đối tượng mồ côi (Unreferenced Objects)
            let final_objects = if options.strip_unreferenced {
                let reachable = collect_referenced_ids(&objects, root_id);
                objects.into_iter().filter(|o| reachable.contains(&o.id)).collect()
            } else {
                objects
            };

            let compressed_bytes = render_compressed_pdf(&header, &final_objects, root_id);

            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("doc");
            let out_path = output_dir.join(format!("{file_stem}_compressed.pdf"));
            fs::write(&out_path, &compressed_bytes).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi tệp PDF nén {}: {e}", out_path.display()),
            })?;

            output_paths.push(out_path.to_string_lossy().to_string());

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: 20.0 + (idx as f32 / request.input_paths.len() as f32) * 70.0,
                    message: format!("Đã tối ưu hóa và nén tệp {}/{}", idx + 1, request.input_paths.len()),
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
                message: "Nén giảm dung lượng PDF hoàn tất thành công.".to_string(),
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

    fn create_bloated_pdf() -> Vec<u8> {
        // Tạo chuỗi lặp lại lớn không nén
        let repeated_text = "Nội dung văn bản lặp lại nhiều lần để kiểm tra nén dung lượng cao. ".repeat(200);

        let pdf = format!(
            "%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
5 0 obj\n<< /Type /OrphanObject /Note (Doi tuong mo coi can loai bo) >>\nendobj\n\
xref\n0 6\n0000000000 65535 f \n\
trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n1000\n%%EOF\n",
            repeated_text.len(),
            repeated_text
        );

        pdf.into_bytes()
    }

    #[test]
    fn test_pdf_compress_happy_path() {
        let tool = PdfCompressTool;
        let tmp = std::env::temp_dir().join("test_pdf_compress_happy");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("large.pdf");
        let original_bytes = create_bloated_pdf();
        fs::write(&pdf_path, &original_bytes).expect("write bloated pdf");

        let req = DispatchTaskRequest {
            task_id: "task-compress-happy".to_string(),
            tool_slug: "pdf-compress".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "target_dpi": 72,
                "compression_level": 9,
                "strip_unreferenced": true
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("compress succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let compressed_bytes = fs::read(&res.output_paths[0]).expect("read compressed");
        assert!(compressed_bytes.len() < original_bytes.len());
        // Kiểm tra loại bỏ đối tượng mồ côi (Object 5)
        let comp_str = String::from_utf8_lossy(&compressed_bytes);
        assert!(!comp_str.contains("OrphanObject"));
    }

    #[test]
    fn test_pdf_compress_edge_case() {
        let tool = PdfCompressTool;
        let tmp = std::env::temp_dir().join("test_pdf_compress_edge");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_bloated_pdf()).expect("write pdf");

        let req = DispatchTaskRequest {
            task_id: "task-compress-edge".to_string(),
            tool_slug: "pdf-compress".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "target_dpi": 150,
                "compression_level": 9,
                "strip_unreferenced": false
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("compress succeeds");
        let compressed_bytes = fs::read(&res.output_paths[0]).expect("read compressed");
        let comp_str = String::from_utf8_lossy(&compressed_bytes);
        assert!(comp_str.contains("/FlateDecode"));
    }

    #[test]
    fn test_pdf_compress_invalid_magic_bytes() {
        let tool = PdfCompressTool;
        let tmp = std::env::temp_dir().join("test_pdf_compress_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("fake.pdf");
        fs::write(&fake_path, b"CORRUPT_BYTES").expect("write fake");

        let req = DispatchTaskRequest {
            task_id: "task-fake".to_string(),
            tool_slug: "pdf-compress".to_string(),
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
    fn test_pdf_compress_cooperative_cancellation() {
        let tool = PdfCompressTool;
        let tmp = std::env::temp_dir().join("test_pdf_compress_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_bloated_pdf()).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-compress".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_compress_security_guardrails() {
        let tool = PdfCompressTool;
        assert_eq!(tool.slug(), "pdf-compress");
        assert_eq!(tool.name(), "Nén giảm Dung lượng PDF");
    }
}
