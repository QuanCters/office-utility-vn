use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Chế độ xử lý PDF
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeSplitMode {
    Merge,
    Split,
    Rotate,
}

/// Tùy chọn cấu hình cho công cụ Ghép/Tách/Xoay PDF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfMergeSplitOptions {
    pub mode: MergeSplitMode,
    #[serde(default)]
    pub page_ranges: Option<String>,
    #[serde(default)]
    pub rotation_deg: Option<i32>,
    #[serde(default)]
    pub max_size_mb: Option<f64>,
    #[serde(default)]
    pub insert_blank_pages: Option<Vec<usize>>,
}

pub struct PdfMergeSplitTool;

impl Default for PdfMergeSplitTool {
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

/// Bộ phân tích cú pháp PDF đơn giản, an toàn bộ nhớ (100% on-device)
struct MiniPdf {
    header: String,
    objects: Vec<PdfObject>,
}

impl MiniPdf {
    fn parse(bytes: &[u8]) -> Result<Self, CommandError> {
        // Kiểm tra magic bytes %PDF-
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

        while let Some(obj_start) = raw_text[search_from..].find(" obj") {
            let abs_obj_start = search_from + obj_start;
            let prefix = raw_text[search_from..abs_obj_start].trim_end();
            let mut parts = prefix.split_whitespace().rev();
            let gen_str = parts.next();
            let id_str = parts.next();

            if let (Some(g_s), Some(i_s)) = (gen_str, id_str)
                && let (Ok(id), Ok(generation_num)) = (i_s.parse::<usize>(), g_s.parse::<usize>()) {
                    let content_start = abs_obj_start + 4;
                    if let Some(end_offset) = raw_text[content_start..].find("endobj") {
                        let content = &raw_text[content_start..content_start + end_offset];
                        objects.push(PdfObject {
                            id,
                            generation: generation_num,
                            content: content.trim().to_string(),
                        });
                        search_from = content_start + end_offset + 6;
                        continue;
                    }
                }
            search_from = abs_obj_start + 4;
        }

        Ok(MiniPdf { header, objects })
    }

    /// Lấy danh sách các object là Page (Type /Page)
    fn get_page_indices(&self) -> Vec<usize> {
        self.objects
            .iter()
            .enumerate()
            .filter_map(|(idx, obj)| {
                if obj.content.contains("/Type")
                    && (obj.content.contains("/Page ")
                        || obj.content.contains("/Page\n")
                        || obj.content.contains("/Page/")
                        || obj.content.contains("/Page>"))
                    && !obj.content.contains("/Pages")
                {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Phân giải dải trang (VD: "1-3, 5, 8-10") thành tập hợp các chỉ số trang 1-based
pub fn parse_page_ranges(range_str: &str, total_pages: usize) -> Result<BTreeSet<usize>, CommandError> {
    let mut pages = BTreeSet::new();
    let parts = range_str.split(',');

    for part in parts {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some((start_str, end_str)) = trimmed.split_once('-') {
            let start = start_str
                .trim()
                .parse::<usize>()
                .map_err(|_| CommandError::ValidationFailed {
                    reason: format!("Dải trang không hợp lệ: '{trimmed}'"),
                })?;
            let end = end_str
                .trim()
                .parse::<usize>()
                .map_err(|_| CommandError::ValidationFailed {
                    reason: format!("Dải trang không hợp lệ: '{trimmed}'"),
                })?;

            if start == 0 || end == 0 || start > end {
                return Err(CommandError::ValidationFailed {
                    reason: format!("Thứ tự trang không hợp lệ: {start}-{end}"),
                });
            }

            for p in start..=end {
                if p <= total_pages {
                    pages.insert(p);
                }
            }
        } else {
            let single = trimmed
                .parse::<usize>()
                .map_err(|_| CommandError::ValidationFailed {
                    reason: format!("Số trang không hợp lệ: '{trimmed}'"),
                })?;
            if single > 0 && single <= total_pages {
                pages.insert(single);
            }
        }
    }

    if pages.is_empty() {
        return Err(CommandError::ValidationFailed {
            reason: "Không có trang hợp lệ nào được chọn".to_string(),
        });
    }

    Ok(pages)
}

/// Sinh tệp PDF hợp lệ từ danh sách objects
fn render_pdf(header: &str, objects: &[PdfObject], root_id: usize) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(header.as_bytes());
    out.push(b'\n');

    let mut xref_offsets = Vec::new();
    xref_offsets.push(0); // Object 0

    let mut id_map = std::collections::HashMap::new();

    for (idx, obj) in objects.iter().enumerate() {
        id_map.insert(obj.id, idx + 1);
    }

    for obj in objects {
        let new_id = id_map.get(&obj.id).copied().unwrap_or(obj.id);
        let offset = out.len();
        xref_offsets.push(offset);

        let mut modified_content = obj.content.clone();
        for (old_id, target_id) in &id_map {
            let old_ref = format!("{} 0 R", old_id);
            let new_ref = format!("{} 0 R", target_id);
            modified_content = modified_content.replace(&old_ref, &new_ref);
        }

        let obj_repr = format!("{} {} obj\n{}\nendobj\n", new_id, obj.generation, modified_content);
        out.extend_from_slice(obj_repr.as_bytes());
    }

    let xref_pos = out.len();
    let xref_header = format!("xref\n0 {}\n0000000000 65535 f \n", xref_offsets.len());
    out.extend_from_slice(xref_header.as_bytes());

    for &off in &xref_offsets[1..] {
        let line = format!("{:010} 00000 n \n", off);
        out.extend_from_slice(line.as_bytes());
    }

    let mapped_root = id_map.get(&root_id).copied().unwrap_or(root_id);
    let trailer = format!(
        "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n",
        xref_offsets.len(),
        mapped_root,
        xref_pos
    );
    out.extend_from_slice(trailer.as_bytes());

    out
}

fn render_selected_pages(
    pdf: &MiniPdf,
    selected_pages: &[usize],
    insert_blanks: &Option<Vec<usize>>,
) -> Vec<u8> {
    let page_indices = pdf.get_page_indices();
    let total_pages = page_indices.len();
    let mut split_page_objects = Vec::new();
    let mut split_refs = Vec::new();
    let mut new_id_counter = 10;

    for &page_num in selected_pages {
        if page_num > 0 && page_num <= total_pages {
            let original_idx = page_indices[page_num - 1];
            if original_idx < pdf.objects.len() {
                let mut original_page = pdf.objects[original_idx].clone();
                let this_id = new_id_counter;
                new_id_counter += 1;
                split_refs.push(this_id);

                original_page.id = this_id;
                if let Some(parent_pos) = original_page.content.find("/Parent") {
                    let end_of_parent = original_page.content[parent_pos..]
                        .find('R')
                        .map(|pos| parent_pos + pos + 1)
                        .unwrap_or(parent_pos + 15);
                    original_page.content.replace_range(parent_pos..end_of_parent, "/Parent 2 0 R");
                } else {
                    original_page.content.push_str(" /Parent 2 0 R");
                }

                split_page_objects.push(original_page);

                if let Some(blanks) = insert_blanks
                    && blanks.contains(&page_num) {
                        let blank_id = new_id_counter;
                        new_id_counter += 1;
                        split_refs.push(blank_id);
                        split_page_objects.push(PdfObject {
                            id: blank_id,
                            generation: 0,
                            content: "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << >> >>".to_string(),
                        });
                    }
            }
        }
    }

    let kids_str = split_refs
        .iter()
        .map(|id| format!("{} 0 R", id))
        .collect::<Vec<_>>()
        .join(" ");

    let catalog = PdfObject {
        id: 1,
        generation: 0,
        content: "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
    };
    let pages_root = PdfObject {
        id: 2,
        generation: 0,
        content: format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids_str,
            split_refs.len()
        ),
    };

    let mut all_objects = vec![catalog, pages_root];
    all_objects.extend(split_page_objects);
    render_pdf(&pdf.header, &all_objects, 1)
}

impl OfficeTool for PdfMergeSplitTool {
    fn slug(&self) -> &'static str {
        "pdf-merge-split"
    }

    fn name(&self) -> &'static str {
        "Ghép, Tách & Xoay trang PDF"
    }

    fn description(&self) -> &'static str {
        "Đọc và xử lý cây đối tượng PDF; trích xuất trang theo dải (1-3, 5, 8-10), thay đổi thuộc tính Rotate (90/180/270 độ), đánh số lại Object ID, gộp các bảng XREF và xuất tệp tức thì."
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

        let options: PdfMergeSplitOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình PDF không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp đầu vào trống".to_string(),
            });
        }

        // Báo cáo bắt đầu tác vụ
        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Đang kiểm tra chữ ký tệp và cấu trúc PDF...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: request.input_paths.len(),
                is_completed: false,
            });
        }

        // Đọc và kiểm tra tất cả các tệp đầu vào
        let mut parsed_pdfs = Vec::new();
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

            let pdf = MiniPdf::parse(&bytes)?;
            parsed_pdfs.push((file_path_str, pdf));

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 1,
                    total_steps: 3,
                    percentage: 10.0 + (idx as f32 / request.input_paths.len() as f32) * 20.0,
                    message: format!("Đã phân tích tệp {}/{}", idx + 1, request.input_paths.len()),
                    processed_files: idx + 1,
                    failed_files: 0,
                    total_files: request.input_paths.len(),
                    is_completed: false,
                });
            }
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let mut output_paths = Vec::new();

        match options.mode {
            MergeSplitMode::Merge => {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                let mut combined_objects = Vec::new();
                let mut page_refs = Vec::new();
                let mut max_id = 10;

                for (_path, pdf) in &parsed_pdfs {
                    let page_idxs = pdf.get_page_indices();
                    for idx in page_idxs {
                        let page_obj = &pdf.objects[idx];
                        let new_id = max_id;
                        max_id += 1;
                        page_refs.push(new_id);

                        let mut page_content = page_obj.content.clone();
                        if let Some(parent_pos) = page_content.find("/Parent") {
                            let end_of_parent = page_content[parent_pos..]
                                .find('R')
                                .map(|pos| parent_pos + pos + 1)
                                .unwrap_or(parent_pos + 15);
                            page_content.replace_range(parent_pos..end_of_parent, "/Parent 2 0 R");
                        } else {
                            page_content.push_str(" /Parent 2 0 R");
                        }

                        combined_objects.push(PdfObject {
                            id: new_id,
                            generation: 0,
                            content: page_content,
                        });

                        let current_total_pages = page_refs.len();
                        if let Some(ref blanks) = options.insert_blank_pages
                            && blanks.contains(&current_total_pages) {
                                let blank_id = max_id;
                                max_id += 1;
                                page_refs.push(blank_id);
                                combined_objects.push(PdfObject {
                                    id: blank_id,
                                    generation: 0,
                                    content: "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << >> >>".to_string(),
                                });
                            }
                    }
                }

                let kids_str = page_refs
                    .iter()
                    .map(|id| format!("{} 0 R", id))
                    .collect::<Vec<_>>()
                    .join(" ");

                let catalog = PdfObject {
                    id: 1,
                    generation: 0,
                    content: "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
                };
                let pages_root = PdfObject {
                    id: 2,
                    generation: 0,
                    content: format!(
                        "<< /Type /Pages /Kids [{}] /Count {} >>",
                        kids_str,
                        page_refs.len()
                    ),
                };

                let mut all_objects = vec![catalog, pages_root];
                all_objects.extend(combined_objects);

                let merged_bytes = render_pdf("%PDF-1.4", &all_objects, 1);
                let out_path = output_dir.join(format!("{}_merged.pdf", request.task_id));
                fs::write(&out_path, merged_bytes).map_err(|e| CommandError::IoError {
                    details: format!("Không thể ghi tệp PDF ghép: {e}"),
                })?;

                output_paths.push(out_path.to_string_lossy().to_string());
            }

            MergeSplitMode::Split => {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                let (_first_path, first_pdf) = parsed_pdfs
                    .first()
                    .ok_or_else(|| CommandError::ValidationFailed {
                        reason: "Cần ít nhất một tệp để tách".to_string(),
                    })?;

                let page_indices = first_pdf.get_page_indices();
                let total_pages = page_indices.len();

                let selected_pages: Vec<usize> = if let Some(ref range_str) = options.page_ranges {
                    parse_page_ranges(range_str, total_pages)?.into_iter().collect()
                } else {
                    (1..=total_pages).collect()
                };

                if let Some(max_mb) = options.max_size_mb
                    && max_mb > 0.0 {
                        // Chia nhỏ theo ngưỡng dung lượng byte (max_size_mb)
                        let max_bytes = (max_mb * 1024.0 * 1024.0).max(100.0) as usize;
                        let mut current_chunk = Vec::new();
                        let mut chunk_idx = 1;

                        for &p in &selected_pages {
                            current_chunk.push(p);
                            let rendered = render_selected_pages(first_pdf, &current_chunk, &options.insert_blank_pages);
                            if rendered.len() > max_bytes && current_chunk.len() > 1 {
                                current_chunk.pop();
                                let chunk_bytes = render_selected_pages(first_pdf, &current_chunk, &options.insert_blank_pages);
                                let out_path = output_dir.join(format!("{}_part{}.pdf", request.task_id, chunk_idx));
                                fs::write(&out_path, chunk_bytes).map_err(|e| CommandError::IoError {
                                    details: format!("Không thể ghi tệp PDF tách theo dung lượng: {e}"),
                                })?;
                                output_paths.push(out_path.to_string_lossy().to_string());
                                chunk_idx += 1;
                                current_chunk = vec![p];
                            }
                        }

                        if !current_chunk.is_empty() {
                            let chunk_bytes = render_selected_pages(first_pdf, &current_chunk, &options.insert_blank_pages);
                            let out_path = output_dir.join(format!("{}_part{}.pdf", request.task_id, chunk_idx));
                            fs::write(&out_path, chunk_bytes).map_err(|e| CommandError::IoError {
                                details: format!("Không thể ghi tệp PDF tách theo dung lượng: {e}"),
                            })?;
                            output_paths.push(out_path.to_string_lossy().to_string());
                        }
                    } else {
                        let split_bytes = render_selected_pages(first_pdf, &selected_pages, &options.insert_blank_pages);
                        let out_path = output_dir.join(format!("{}_split.pdf", request.task_id));
                        fs::write(&out_path, split_bytes).map_err(|e| CommandError::IoError {
                            details: format!("Không thể ghi tệp PDF tách: {e}"),
                        })?;
                        output_paths.push(out_path.to_string_lossy().to_string());
                    }
            }

            MergeSplitMode::Rotate => {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                let deg = options.rotation_deg.unwrap_or(90);
                if deg % 90 != 0 {
                    return Err(CommandError::ValidationFailed {
                        reason: format!("Góc xoay phải là bội số của 90 (nhận được: {deg})"),
                    });
                }

                for (idx, (file_path_str, pdf)) in parsed_pdfs.iter().enumerate() {
                    let mut modified_objects = pdf.objects.clone();
                    let page_indices = pdf.get_page_indices();

                    for &p_idx in &page_indices {
                        let page_obj = &mut modified_objects[p_idx];
                        if let Some(rot_pos) = page_obj.content.find("/Rotate") {
                            let after_rot = &page_obj.content[rot_pos + 7..];
                            let num_end = after_rot
                                .find(|c: char| !c.is_ascii_digit() && c != '-')
                                .unwrap_or(after_rot.len());
                            page_obj.content.replace_range(rot_pos..rot_pos + 7 + num_end, &format!("/Rotate {deg}"));
                        } else {
                            if let Some(dict_end) = page_obj.content.rfind(">>") {
                                page_obj.content.insert_str(dict_end, &format!(" /Rotate {deg} "));
                            } else {
                                page_obj.content.push_str(&format!(" /Rotate {deg}"));
                            }
                        }
                    }

                    let rotated_bytes = render_pdf(&pdf.header, &modified_objects, 1);
                    let file_name = Path::new(file_path_str)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("document");
                    let out_path = output_dir.join(format!("{file_name}_rotated_{deg}deg_{idx}.pdf"));
                    fs::write(&out_path, rotated_bytes).map_err(|e| CommandError::IoError {
                        details: format!("Không thể ghi tệp PDF xoay: {e}"),
                    })?;

                    output_paths.push(out_path.to_string_lossy().to_string());
                }
            }
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Hoàn tất xử lý PDF thành công.".to_string(),
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
    use std::sync::atomic::AtomicBool;
    use serde_json::json;

    fn create_dummy_pdf(num_pages: usize) -> Vec<u8> {
        let mut objects = Vec::new();
        let mut kids = Vec::new();

        for i in 1..=num_pages {
            let page_id = 2 + i;
            kids.push(format!("{page_id} 0 R"));
            objects.push(PdfObject {
                id: page_id,
                generation: 0,
                content: "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>".to_string(),
            });
        }

        let catalog = PdfObject {
            id: 1,
            generation: 0,
            content: "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        };
        let pages_root = PdfObject {
            id: 2,
            generation: 0,
            content: format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), num_pages),
        };

        let mut all = vec![catalog, pages_root];
        all.extend(objects);

        render_pdf("%PDF-1.4", &all, 1)
    }

    #[test]
    fn test_pdf_merge_split_happy_path() {
        let tool = PdfMergeSplitTool;
        let tmp = std::env::temp_dir().join("test_pdf_merge_happy");
        let _ = fs::create_dir_all(&tmp);

        let pdf1_path = tmp.join("doc1.pdf");
        let pdf2_path = tmp.join("doc2.pdf");

        fs::write(&pdf1_path, create_dummy_pdf(2)).expect("write pdf1");
        fs::write(&pdf2_path, create_dummy_pdf(3)).expect("write pdf2");

        let req = DispatchTaskRequest {
            task_id: "task-merge-happy".to_string(),
            tool_slug: "pdf-merge-split".to_string(),
            input_paths: vec![
                pdf1_path.to_string_lossy().to_string(),
                pdf2_path.to_string_lossy().to_string(),
            ],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "mode": "merge"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("merge should succeed");
        assert_eq!(res.output_paths.len(), 1);
        let out_bytes = fs::read(&res.output_paths[0]).expect("read merged");
        assert!(out_bytes.starts_with(b"%PDF-"));

        let merged_pdf = MiniPdf::parse(&out_bytes).expect("parse merged pdf");
        assert_eq!(merged_pdf.get_page_indices().len(), 5);
    }

    #[test]
    fn test_pdf_merge_split_edge_case() {
        let tool = PdfMergeSplitTool;
        let tmp = std::env::temp_dir().join("test_pdf_split_edge");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("multi.pdf");
        fs::write(&pdf_path, create_dummy_pdf(5)).expect("write multi");

        let req = DispatchTaskRequest {
            task_id: "task-split-edge".to_string(),
            tool_slug: "pdf-merge-split".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "mode": "split",
                "page_ranges": "1-2, 4"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("split should succeed");
        assert_eq!(res.output_paths.len(), 1);
        let out_bytes = fs::read(&res.output_paths[0]).expect("read split");
        let parsed = MiniPdf::parse(&out_bytes).expect("parse split pdf");
        assert_eq!(parsed.get_page_indices().len(), 3);
    }

    #[test]
    fn test_pdf_merge_split_invalid_magic_bytes() {
        let tool = PdfMergeSplitTool;
        let tmp = std::env::temp_dir().join("test_pdf_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("fake.pdf");
        fs::write(&fake_path, b"NOT_A_PDF_FILE_HEADER").expect("write fake");

        let req = DispatchTaskRequest {
            task_id: "task-fake-sig".to_string(),
            tool_slug: "pdf-merge-split".to_string(),
            input_paths: vec![fake_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "mode": "split" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { expected, .. } => {
                assert_eq!(expected, "%PDF-");
            }
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_pdf_merge_split_cooperative_cancellation() {
        let tool = PdfMergeSplitTool;
        let tmp = std::env::temp_dir().join("test_pdf_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_dummy_pdf(2)).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-merge-split".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "mode": "rotate", "rotation_deg": 90 }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_merge_split_security_guardrails() {
        let tool = PdfMergeSplitTool;
        assert_eq!(tool.slug(), "pdf-merge-split");
        assert_eq!(tool.name(), "Ghép, Tách & Xoay trang PDF");

        let invalid = parse_page_ranges("abc", 5);
        assert!(invalid.is_err());
        let zero = parse_page_ranges("0", 5);
        assert!(zero.is_err());
    }

    #[test]
    fn test_backward_compatible_options_deserialization() {
        // Cấu hình cũ từ tool_presets hoặc workflow không có max_size_mb hay insert_blank_pages
        let old_json = json!({
            "mode": "split",
            "page_ranges": "1-3"
        });
        let opts: PdfMergeSplitOptions = serde_json::from_value(old_json).expect("deserialize old options");
        assert_eq!(opts.mode, MergeSplitMode::Split);
        assert_eq!(opts.page_ranges, Some("1-3".to_string()));
        assert_eq!(opts.max_size_mb, None);
        assert_eq!(opts.insert_blank_pages, None);
    }

    #[test]
    fn test_split_by_max_size_mb() {
        let tool = PdfMergeSplitTool;
        let tmp = std::env::temp_dir().join("test_pdf_split_size");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("multi_10p.pdf");
        fs::write(&pdf_path, create_dummy_pdf(6)).expect("write multi_10p");

        let req = DispatchTaskRequest {
            task_id: "task-split-size".to_string(),
            tool_slug: "pdf-merge-split".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "mode": "split",
                "max_size_mb": 0.0005 // Rất nhỏ (~500 bytes) để buộc chia thành nhiều part
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("split by size should succeed");
        assert!(res.output_paths.len() > 1, "Kỳ vọng chia thành nhiều phần, thực tế: {}", res.output_paths.len());
        for path in &res.output_paths {
            let bytes = fs::read(path).expect("read chunk");
            assert!(bytes.starts_with(b"%PDF-"));
        }
    }

    #[test]
    fn test_insert_blank_pages() {
        let tool = PdfMergeSplitTool;
        let tmp = std::env::temp_dir().join("test_pdf_insert_blank");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc_3p.pdf");
        fs::write(&pdf_path, create_dummy_pdf(3)).expect("write doc_3p");

        let req = DispatchTaskRequest {
            task_id: "task-insert-blank".to_string(),
            tool_slug: "pdf-merge-split".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "mode": "split",
                "page_ranges": "1-3",
                "insert_blank_pages": [1, 2] // Chèn 2 trang trắng sau trang 1 và 2 -> tổng cộng 5 trang
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("split with blanks should succeed");
        assert_eq!(res.output_paths.len(), 1);
        let bytes = fs::read(&res.output_paths[0]).expect("read output");
        let parsed = MiniPdf::parse(&bytes).expect("parse output");
        assert_eq!(parsed.get_page_indices().len(), 5);
    }
}
