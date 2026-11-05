use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Hành động áp dụng cho một trang PDF cụ thể
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageAction {
    #[serde(default)]
    pub source_page: usize, // 1-based index
    #[serde(default)]
    pub rotation_deg: i32,  // 0, 90, 180, 270
}

/// Tùy chọn cấu hình cho công cụ Sắp xếp & Đảo trang PDF
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PdfPageOrganizerOptions {
    /// Danh sách các trang được giữ lại theo thứ tự mới kèm góc xoay
    #[serde(default)]
    pub pages: Vec<PageAction>,
}

/// Thông tin hình học và định hướng của từng trang PDF
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageInfo {
    pub page_index: usize, // 1-based index
    pub width_pt: f64,
    pub height_pt: f64,
    pub rotation_deg: i32,
    pub is_landscape: bool,
}

pub struct PdfPageOrganizerTool;

impl Default for PdfPageOrganizerTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct PageOrganizerPdfObj {
    id: usize,
    generation: usize,
    content: String,
}

struct PageOrganizerParsedPdf {
    header: String,
    objects: Vec<PageOrganizerPdfObj>,
    root_id: usize,
}

impl PageOrganizerParsedPdf {
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
        let header = match raw_text.lines().next() {
            Some(line) => line.trim().to_string(),
            None => "%PDF-1.4".to_string(),
        };

        let mut root_id = 1;
        if let Some(r_pos) = raw_text.find("/Root") {
            let rest = &raw_text[r_pos + 5..];
            let mut parts = rest.split_whitespace();
            if let Some(first) = parts.next() {
                if let Ok(id) = first.parse::<usize>() {
                    root_id = id;
                }
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
                        let content = raw_text[content_start..content_start + end_offset]
                            .trim()
                            .to_string();
                        objects.push(PageOrganizerPdfObj {
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

        if let Some(cat_obj) = objects.iter().find(|o| {
            o.content.contains("/Type") && o.content.contains("/Catalog")
        }) {
            root_id = cat_obj.id;
        }

        Ok(PageOrganizerParsedPdf {
            header,
            objects,
            root_id,
        })
    }

    fn get_page_ids(&self) -> Vec<usize> {
        let mut obj_map = HashMap::new();
        for obj in &self.objects {
            obj_map.insert(obj.id, obj);
        }

        let mut page_ids = Vec::new();
        let mut visited = HashSet::new();

        if let Some(pages_root) = find_pages_root_id(&self.objects, self.root_id) {
            resolve_pages_tree(pages_root, &obj_map, &mut visited, &mut page_ids);
        }

        if page_ids.is_empty() {
            for obj in &self.objects {
                if obj.content.contains("/Type")
                    && (obj.content.contains("/Page ")
                        || obj.content.contains("/Page\n")
                        || obj.content.contains("/Page/")
                        || obj.content.contains("/Page>"))
                    && !obj.content.contains("/Pages")
                {
                    page_ids.push(obj.id);
                }
            }
        }

        page_ids
    }
}

fn find_pages_root_id(objects: &[PageOrganizerPdfObj], root_id: usize) -> Option<usize> {
    if let Some(cat) = objects.iter().find(|o| o.id == root_id) {
        if let Some(pos) = cat.content.find("/Pages") {
            let rest = &cat.content[pos + 6..];
            for token in rest.split_whitespace() {
                if let Ok(id) = token.parse::<usize>() {
                    return Some(id);
                }
            }
        }
    }

    objects.iter().find_map(|o| {
        if o.content.contains("/Type")
            && (o.content.contains("/Pages ")
                || o.content.contains("/Pages\n")
                || o.content.contains("/Pages/"))
        {
            Some(o.id)
        } else {
            None
        }
    })
}

fn resolve_pages_tree(
    obj_id: usize,
    obj_map: &HashMap<usize, &PageOrganizerPdfObj>,
    visited: &mut HashSet<usize>,
    page_ids: &mut Vec<usize>,
) {
    if !visited.insert(obj_id) {
        return;
    }

    if let Some(obj) = obj_map.get(&obj_id) {
        let is_page = obj.content.contains("/Type")
            && (obj.content.contains("/Page ")
                || obj.content.contains("/Page\n")
                || obj.content.contains("/Page/")
                || obj.content.contains("/Page>"))
            && !obj.content.contains("/Pages");

        let is_pages = obj.content.contains("/Type")
            && (obj.content.contains("/Pages ")
                || obj.content.contains("/Pages\n")
                || obj.content.contains("/Pages/")
                || obj.content.contains("/Pages>"));

        if is_page {
            page_ids.push(obj_id);
        } else if is_pages || obj.content.contains("/Kids") {
            if let Some(kids_pos) = obj.content.find("/Kids") {
                let rest = &obj.content[kids_pos + 5..];
                if let Some(start_bracket) = rest.find('[') {
                    if let Some(end_bracket) = rest[start_bracket + 1..].find(']') {
                        let inner = &rest[start_bracket + 1..start_bracket + 1 + end_bracket];
                        let tokens: Vec<&str> = inner.split_whitespace().collect();
                        let mut i = 0;
                        while i < tokens.len() {
                            if let Ok(child_id) = tokens[i].parse::<usize>() {
                                if i + 2 < tokens.len() && tokens[i + 2] == "R" {
                                    resolve_pages_tree(child_id, obj_map, visited, page_ids);
                                    i += 3;
                                    continue;
                                } else if i + 1 < tokens.len() && tokens[i + 1] == "R" {
                                    resolve_pages_tree(child_id, obj_map, visited, page_ids);
                                    i += 2;
                                    continue;
                                }
                            }
                            i += 1;
                        }
                    }
                }
            }
        }
    }
}

fn parse_box(content: &str, box_key: &str) -> Option<(f64, f64)> {
    let key_pos = content.find(box_key)?;
    let rest = &content[key_pos + box_key.len()..];
    let bracket_start = rest.find('[')?;
    let bracket_end = rest[bracket_start + 1..].find(']')?;
    let inner = &rest[bracket_start + 1..bracket_start + 1 + bracket_end];
    let nums: Vec<f64> = inner
        .split_whitespace()
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();
    if nums.len() >= 4 {
        let width = (nums[2] - nums[0]).abs();
        let height = (nums[3] - nums[1]).abs();
        if width > 0.0 && height > 0.0 {
            return Some((width, height));
        }
    }
    None
}

fn parse_rotation(content: &str) -> i32 {
    if let Some(pos) = content.find("/Rotate") {
        let after = &content[pos + 7..];
        let trimmed = after.trim_start();
        let num_str: String = trimmed
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        if let Ok(rot) = num_str.parse::<i32>() {
            return rot.rem_euclid(360);
        }
    }
    0
}

/// Đọc nhanh thông tin hình học, tỷ lệ khổ giấy và góc xoay của toàn bộ các trang PDF
pub fn get_pdf_page_info(path: &Path) -> Result<Vec<PageInfo>, CommandError> {
    if !path.exists() {
        return Err(CommandError::FileNotFound {
            path: path.to_string_lossy().to_string(),
        });
    }

    let bytes = fs::read(path).map_err(|e| CommandError::IoError {
        details: format!("Không thể đọc tệp {}: {e}", path.display()),
    })?;

    let parsed = PageOrganizerParsedPdf::parse(&bytes)?;
    let page_ids = parsed.get_page_ids();

    let mut obj_map = HashMap::new();
    for obj in &parsed.objects {
        obj_map.insert(obj.id, obj);
    }

    let mut page_infos = Vec::with_capacity(page_ids.len());
    for (idx, page_id) in page_ids.iter().enumerate() {
        let default_content = String::new();
        let content = match obj_map.get(page_id) {
            Some(obj) => &obj.content,
            None => &default_content,
        };

        let (width_pt, height_pt) = match parse_box(content, "/MediaBox") {
            Some(dims) => dims,
            None => match parse_box(content, "/CropBox") {
                Some(dims) => dims,
                None => (595.0, 842.0),
            },
        };

        let rotation_deg = parse_rotation(content);
        let effective_width = if (rotation_deg / 90) % 2 != 0 {
            height_pt
        } else {
            width_pt
        };
        let effective_height = if (rotation_deg / 90) % 2 != 0 {
            width_pt
        } else {
            height_pt
        };
        let is_landscape = effective_width > effective_height;

        page_infos.push(PageInfo {
            page_index: idx + 1,
            width_pt,
            height_pt,
            rotation_deg,
            is_landscape,
        });
    }

    Ok(page_infos)
}

fn render_organizer_pdf(
    header: &str,
    objects: &[PageOrganizerPdfObj],
    root_id: usize,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(header.as_bytes());
    out.push(b'\n');

    let mut max_id = 0;
    for obj in objects {
        if obj.id > max_id {
            max_id = obj.id;
        }
    }

    let mut offsets = vec![None; max_id + 1];

    for obj in objects {
        let offset = out.len();
        if obj.id <= max_id {
            offsets[obj.id] = Some(offset);
        }
        let obj_repr = format!(
            "{} {} obj\n{}\nendobj\n",
            obj.id, obj.generation, obj.content
        );
        out.extend_from_slice(obj_repr.as_bytes());
    }

    let xref_pos = out.len();
    let xref_header = format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len());
    out.extend_from_slice(xref_header.as_bytes());

    for off_opt in &offsets[1..] {
        let line = match off_opt {
            Some(off) => format!("{:010} 00000 n \n", off),
            None => "0000000000 65535 f \n".to_string(),
        };
        out.extend_from_slice(line.as_bytes());
    }

    let trailer = format!(
        "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n",
        offsets.len(),
        root_id,
        xref_pos
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

impl OfficeTool for PdfPageOrganizerTool {
    fn slug(&self) -> &'static str {
        "pdf-page-organizer"
    }

    fn name(&self) -> &'static str {
        "Sắp xếp & Đảo trang PDF Trực quan"
    }

    fn description(&self) -> &'static str {
        "Tổ chức, sắp xếp lại thứ tự trang trực quan, xoay từng trang độc lập 90/180/270 độ hoặc xóa trang không mong muốn trong tài liệu PDF."
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

        let options: PdfPageOrganizerOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình sắp xếp trang PDF không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp đầu vào trống".to_string(),
            });
        }

        let output_dir = match request.output_directory.as_deref() {
            Some(dir) => PathBuf::from(dir),
            None => std::env::temp_dir().join("office_pdf_organizer_output"),
        };

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Đang đọc cấu trúc cây đối tượng PDF...".to_string(),
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

            let parsed = PageOrganizerParsedPdf::parse(&bytes)?;
            let original_page_ids = parsed.get_page_ids();
            let total_pages = original_page_ids.len();

            if total_pages == 0 {
                return Err(CommandError::ValidationFailed {
                    reason: "Tệp PDF không chứa trang hợp lệ nào".to_string(),
                });
            }

            let actions: Vec<PageAction> = if options.pages.is_empty() {
                (1..=total_pages)
                    .map(|p| PageAction {
                        source_page: p,
                        rotation_deg: 0,
                    })
                    .collect()
            } else {
                options.pages.clone()
            };

            for action in &actions {
                if action.source_page == 0 || action.source_page > total_pages {
                    return Err(CommandError::ValidationFailed {
                        reason: format!(
                            "Trang nguồn {} không hợp lệ (tổng số trang: {})",
                            action.source_page, total_pages
                        ),
                    });
                }
                if action.rotation_deg % 90 != 0 {
                    return Err(CommandError::ValidationFailed {
                        reason: format!(
                            "Góc xoay phải là bội số của 90 (nhận được: {})",
                            action.rotation_deg
                        ),
                    });
                }
            }

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: 40.0 + (idx as f32 / request.input_paths.len() as f32) * 40.0,
                    message: format!(
                        "Đang tái tổ chức thứ tự và xoay trang tệp {}/{}...",
                        idx + 1,
                        request.input_paths.len()
                    ),
                    processed_files: idx,
                    failed_files: 0,
                    total_files: request.input_paths.len(),
                    is_completed: false,
                });
            }

            let mut max_id = 0;
            for obj in &parsed.objects {
                if obj.id > max_id {
                    max_id = obj.id;
                }
            }

            let pages_root_id = match find_pages_root_id(&parsed.objects, parsed.root_id) {
                Some(id) => id,
                None => {
                    max_id += 1;
                    max_id
                }
            };
            let catalog_id = parsed.root_id;

            let mut original_page_objs = HashMap::new();
            for &pid in &original_page_ids {
                if let Some(obj) = parsed.objects.iter().find(|o| o.id == pid) {
                    original_page_objs.insert(pid, obj.clone());
                }
            }

            let mut used_page_ids = HashSet::new();
            let mut final_page_objects = Vec::new();
            let mut final_page_ids = Vec::new();

            for action in &actions {
                let orig_id = original_page_ids[action.source_page - 1];
                let page_obj_template = match original_page_objs.get(&orig_id) {
                    Some(obj) => obj.clone(),
                    None => continue,
                };

                let target_id = if used_page_ids.insert(orig_id) {
                    orig_id
                } else {
                    max_id += 1;
                    max_id
                };
                final_page_ids.push(target_id);

                let mut page_obj = page_obj_template;
                page_obj.id = target_id;

                // 1. Cập nhật /Parent trỏ về nút gốc Pages
                let parent_ref = format!("/Parent {} 0 R", pages_root_id);
                if let Some(parent_pos) = page_obj.content.find("/Parent") {
                    let end_of_parent = page_obj.content[parent_pos..]
                        .find('R')
                        .map(|pos| parent_pos + pos + 1)
                        .unwrap_or_else(|| {
                            let len = page_obj.content[parent_pos..]
                                .find('>')
                                .unwrap_or(15);
                            parent_pos + len
                        });
                    page_obj
                        .content
                        .replace_range(parent_pos..end_of_parent, &parent_ref);
                } else if let Some(dict_end) = page_obj.content.rfind(">>") {
                    page_obj
                        .content
                        .insert_str(dict_end, &format!(" {} ", parent_ref));
                } else {
                    page_obj.content.push_str(&format!(" {}", parent_ref));
                }

                // 2. Cập nhật /Rotate cộng dồn mod 360
                let existing_rot = parse_rotation(&page_obj.content);
                let final_rot = (existing_rot + action.rotation_deg).rem_euclid(360);
                if let Some(rot_pos) = page_obj.content.find("/Rotate") {
                    let after_rot = &page_obj.content[rot_pos + 7..];
                    let ws_len = after_rot.len() - after_rot.trim_start().len();
                    let digits_part = &after_rot[ws_len..];
                    let digits_len = match digits_part
                        .find(|c: char| !c.is_ascii_digit() && c != '-')
                    {
                        Some(len) => len,
                        None => digits_part.len(),
                    };
                    page_obj.content.replace_range(
                        rot_pos..rot_pos + 7 + ws_len + digits_len,
                        &format!("/Rotate {final_rot}"),
                    );
                } else if let Some(dict_end) = page_obj.content.rfind(">>") {
                    page_obj
                        .content
                        .insert_str(dict_end, &format!(" /Rotate {final_rot} "));
                } else {
                    page_obj
                        .content
                        .push_str(&format!(" /Rotate {final_rot}"));
                }

                final_page_objects.push(page_obj);
            }

            // Xây dựng nút gốc /Pages mới với danh sách Kids chính xác
            let kids_str = final_page_ids
                .iter()
                .map(|id| format!("{} 0 R", id))
                .collect::<Vec<_>>()
                .join(" ");
            let pages_root_content = format!(
                "<< /Type /Pages /Kids [{}] /Count {} >>",
                kids_str,
                final_page_ids.len()
            );
            let pages_root_obj = PageOrganizerPdfObj {
                id: pages_root_id,
                generation: 0,
                content: pages_root_content,
            };

            // Đảm bảo Catalog trỏ tới /Pages root
            let catalog_obj = match parsed
                .objects
                .iter()
                .find(|o| o.id == catalog_id)
                .cloned()
            {
                Some(mut cat) => {
                    let pages_ref = format!("/Pages {} 0 R", pages_root_id);
                    if let Some(pages_pos) = cat.content.find("/Pages") {
                        let end_of_pages = cat.content[pages_pos..]
                            .find('R')
                            .map(|p| pages_pos + p + 1)
                            .unwrap_or_else(|| {
                                let len = cat.content[pages_pos..].find('>').unwrap_or(15);
                                pages_pos + len
                            });
                        cat.content
                            .replace_range(pages_pos..end_of_pages, &pages_ref);
                    } else if let Some(dict_end) = cat.content.rfind(">>") {
                        cat.content.insert_str(dict_end, &format!(" {} ", pages_ref));
                    } else {
                        cat.content.push_str(&format!(" {}", pages_ref));
                    }
                    cat
                }
                None => PageOrganizerPdfObj {
                    id: catalog_id,
                    generation: 0,
                    content: format!("<< /Type /Catalog /Pages {} 0 R >>", pages_root_id),
                },
            };

            // Giữ lại các đối tượng không phải trang hoặc cây phân cấp cũ (fonts, streams, metadata...)
            let mut other_objects = Vec::new();
            for obj in &parsed.objects {
                if obj.id == catalog_id || obj.id == pages_root_id {
                    continue;
                }
                if original_page_ids.contains(&obj.id) {
                    continue;
                }
                if obj.content.contains("/Type")
                    && (obj.content.contains("/Pages ")
                        || obj.content.contains("/Pages\n")
                        || obj.content.contains("/Pages/"))
                {
                    continue;
                }
                other_objects.push(obj.clone());
            }

            let mut all_objects = vec![catalog_obj, pages_root_obj];
            all_objects.extend(final_page_objects);
            all_objects.extend(other_objects);

            let organized_bytes =
                render_organizer_pdf(&parsed.header, &all_objects, catalog_id);

            let stem = Path::new(file_path_str)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("document");
            let out_file_name = format!("{stem}_organized_{idx}.pdf");
            let out_path = output_dir.join(out_file_name);

            fs::write(&out_path, organized_bytes).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi tệp PDF sắp xếp: {e}"),
            })?;

            output_paths.push(out_path.to_string_lossy().to_string());
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Tổ chức và sắp xếp trang PDF hoàn tất thành công.".to_string(),
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

    fn create_dummy_pdf(num_pages: usize) -> Vec<u8> {
        let mut kids = Vec::new();
        let mut objects = Vec::new();

        for i in 1..=num_pages {
            let page_id = 2 + i;
            kids.push(format!("{page_id} 0 R"));
            objects.push(format!(
                "{page_id} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n"
            ));
        }

        let catalog = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
        let pages_root = format!(
            "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            kids.join(" "),
            num_pages
        );

        let mut pdf = String::from("%PDF-1.4\n");
        pdf.push_str(catalog);
        pdf.push_str(&pages_root);
        for obj in objects {
            pdf.push_str(&obj);
        }
        pdf.push_str("xref\n0 6\n0000000000 65535 f \ntrailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n500\n%%EOF\n");
        pdf.into_bytes()
    }

    #[test]
    fn test_pdf_page_organizer_happy_path() {
        let tool = PdfPageOrganizerTool;
        let tmp = std::env::temp_dir().join("test_organizer_happy");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc3p.pdf");
        fs::write(&pdf_path, create_dummy_pdf(3)).expect("write dummy pdf");

        let req = DispatchTaskRequest {
            task_id: "task-organizer-happy".to_string(),
            tool_slug: "pdf-page-organizer".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "pages": [
                    { "source_page": 3, "rotation_deg": 0 },
                    { "source_page": 1, "rotation_deg": 90 },
                    { "source_page": 2, "rotation_deg": 0 }
                ]
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("organizer succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let out_path = PathBuf::from(&res.output_paths[0]);
        let page_infos = get_pdf_page_info(&out_path).expect("read output page infos");
        assert_eq!(page_infos.len(), 3);
        assert_eq!(page_infos[0].page_index, 1);
        assert_eq!(page_infos[0].rotation_deg, 0);

        assert_eq!(page_infos[1].page_index, 2);
        assert_eq!(page_infos[1].rotation_deg, 90);
        assert!(page_infos[1].is_landscape, "Rotated 90 deg portrait should be landscape");

        assert_eq!(page_infos[2].page_index, 3);
        assert_eq!(page_infos[2].rotation_deg, 0);
    }

    #[test]
    fn test_pdf_page_organizer_invalid_magic_bytes() {
        let tool = PdfPageOrganizerTool;
        let tmp = std::env::temp_dir().join("test_organizer_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_path = tmp.join("not_a_pdf.pdf");
        fs::write(&bad_path, b"NOT_A_PDF_DOCUMENT_SIGNATURE").expect("write bad file");

        let req = DispatchTaskRequest {
            task_id: "task-organizer-bad".to_string(),
            tool_slug: "pdf-page-organizer".to_string(),
            input_paths: vec![bad_path.to_string_lossy().to_string()],
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

        let info_err = get_pdf_page_info(&bad_path).unwrap_err();
        match info_err {
            CommandError::InvalidFileSignature { expected, .. } => {
                assert_eq!(expected, "%PDF-");
            }
            other => panic!("Expected InvalidFileSignature from get_pdf_page_info, got {:?}", other),
        }
    }

    #[test]
    fn test_pdf_page_organizer_cooperative_cancellation() {
        let tool = PdfPageOrganizerTool;
        let tmp = std::env::temp_dir().join("test_organizer_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc2p.pdf");
        fs::write(&pdf_path, create_dummy_pdf(2)).expect("write dummy pdf");

        let req = DispatchTaskRequest {
            task_id: "task-organizer-cancel".to_string(),
            tool_slug: "pdf-page-organizer".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_page_organizer_security_guardrails() {
        let tool = PdfPageOrganizerTool;
        assert_eq!(tool.slug(), "pdf-page-organizer");
        assert_eq!(tool.name(), "Sắp xếp & Đảo trang PDF Trực quan");

        let tmp = std::env::temp_dir().join("test_organizer_guardrails");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc3p_guard.pdf");
        fs::write(&pdf_path, create_dummy_pdf(3)).expect("write dummy pdf");

        // Test out of bounds source page (page 99 > 3)
        let req_oob = DispatchTaskRequest {
            task_id: "task-guard-oob".to_string(),
            tool_slug: "pdf-page-organizer".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "pages": [{ "source_page": 99, "rotation_deg": 0 }]
            }),
        };
        let cancel = AtomicBool::new(false);
        let err_oob = tool.execute(&req_oob, &cancel, None).unwrap_err();
        assert!(matches!(err_oob, CommandError::ValidationFailed { .. }));

        // Test source page 0
        let req_zero = DispatchTaskRequest {
            task_id: "task-guard-zero".to_string(),
            tool_slug: "pdf-page-organizer".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "pages": [{ "source_page": 0, "rotation_deg": 0 }]
            }),
        };
        let err_zero = tool.execute(&req_zero, &cancel, None).unwrap_err();
        assert!(matches!(err_zero, CommandError::ValidationFailed { .. }));

        // Test invalid rotation degree (not multiple of 90)
        let req_rot = DispatchTaskRequest {
            task_id: "task-guard-rot".to_string(),
            tool_slug: "pdf-page-organizer".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "pages": [{ "source_page": 1, "rotation_deg": 45 }]
            }),
        };
        let err_rot = tool.execute(&req_rot, &cancel, None).unwrap_err();
        assert!(matches!(err_rot, CommandError::ValidationFailed { .. }));
    }

    #[test]
    fn test_pdf_page_organizer_edge_case() {
        let tool = PdfPageOrganizerTool;
        let tmp = std::env::temp_dir().join("test_organizer_edge");
        let _ = fs::create_dir_all(&tmp);

        // 1-page PDF
        let pdf1_path = tmp.join("doc1p.pdf");
        fs::write(&pdf1_path, create_dummy_pdf(1)).expect("write dummy 1p");

        let req1 = DispatchTaskRequest {
            task_id: "task-organizer-1p".to_string(),
            tool_slug: "pdf-page-organizer".to_string(),
            input_paths: vec![pdf1_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "pages": [{ "source_page": 1, "rotation_deg": 180 }]
            }),
        };

        let cancel = AtomicBool::new(false);
        let res1 = tool.execute(&req1, &cancel, None).expect("1p organize succeeds");
        let infos1 = get_pdf_page_info(&PathBuf::from(&res1.output_paths[0])).expect("infos 1p");
        assert_eq!(infos1.len(), 1);
        assert_eq!(infos1[0].rotation_deg, 180);

        // Empty pages in options preserves original order
        let req_empty = DispatchTaskRequest {
            task_id: "task-organizer-empty-opts".to_string(),
            tool_slug: "pdf-page-organizer".to_string(),
            input_paths: vec![pdf1_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "pages": [] }),
        };
        let res_empty = tool.execute(&req_empty, &cancel, None).expect("empty opts succeeds");
        let infos_empty = get_pdf_page_info(&PathBuf::from(&res_empty.output_paths[0])).expect("infos empty");
        assert_eq!(infos_empty.len(), 1);

        // Non-existent input file
        let req_missing = DispatchTaskRequest {
            task_id: "task-organizer-missing".to_string(),
            tool_slug: "pdf-page-organizer".to_string(),
            input_paths: vec![tmp.join("non_existent_file.pdf").to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };
        let err_missing = tool.execute(&req_missing, &cancel, None).unwrap_err();
        assert!(matches!(err_missing, CommandError::FileNotFound { .. }));
    }

    #[test]
    fn test_pdf_page_organizer_accumulate_existing_rotation() {
        let tool = PdfPageOrganizerTool;
        let tmp = std::env::temp_dir().join("test_organizer_accumulate_rot");
        let _ = fs::create_dir_all(&tmp);

        // Tạo PDF với trang 1 đã có sẵn /Rotate 90
        let pdf_content = "\
%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate 90 >>\nendobj\n\
xref\n0 4\n0000000000 65535 f \ntrailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n250\n%%EOF\n";

        let pdf_path = tmp.join("doc_with_existing_rot.pdf");
        fs::write(&pdf_path, pdf_content.as_bytes()).expect("write pdf with rot");

        let req = DispatchTaskRequest {
            task_id: "task-accumulate-rot".to_string(),
            tool_slug: "pdf-page-organizer".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "pages": [
                    { "source_page": 1, "rotation_deg": 90 }
                ]
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("organizer succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read output file");
        let out_str = String::from_utf8_lossy(&out_bytes);

        // Đảm bảo chứa /Rotate 180 và KHÔNG chứa "/Rotate 180 90" hoặc " 90" sau /Rotate
        assert!(out_str.contains("/Rotate 180"), "Phải chứa /Rotate 180");
        assert!(!out_str.contains("/Rotate 180 90"), "Không được để sót giá trị 90 cũ");
        assert!(!out_str.contains("/Rotate 90"), "Không còn /Rotate 90");

        let infos = get_pdf_page_info(&PathBuf::from(&res.output_paths[0])).expect("infos");
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].rotation_deg, 180);
    }
}
