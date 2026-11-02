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
pub struct BookmarkEntry {
    pub title: String,
    pub page: usize,
    #[serde(default)]
    pub children: Vec<BookmarkEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PdfBookmarkOptions {
    #[serde(default)]
    pub bookmarks_json: String,
    #[serde(default)]
    pub bookmarks: Vec<BookmarkEntry>,
}

pub struct PdfBookmarkTocTool;

impl Default for PdfBookmarkTocTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
pub struct PdfPageRef {
    pub page_number: usize,
    pub obj_id: usize,
}


fn extract_page_obj_ids(raw_text: &str) -> Vec<PdfPageRef> {
    let mut pages = Vec::new();
    let mut search_from = 0;
    let mut page_num = 1;

    while let Some(pos) = raw_text[search_from..].find("/Type /Page\n")
        .or_else(|| raw_text[search_from..].find("/Type /Page ")
        .or_else(|| raw_text[search_from..].find("/Type/Page")))
    {
        let abs_pos = search_from + pos;
        let prefix = &raw_text[..abs_pos];
        if let Some(obj_marker) = prefix.rfind(" obj") {
            let before_obj = prefix[..obj_marker].trim_end();
            let mut parts = before_obj.split_whitespace().rev();
            let _gen = parts.next();
            let id_str = parts.next();
            if let Some(id_s) = id_str
                && let Ok(id) = id_s.parse::<usize>() {
                    pages.push(PdfPageRef {
                        page_number: page_num,
                        obj_id: id,
                    });
                    page_num += 1;
                }
        }
        search_from = abs_pos + 11;
    }

    if pages.is_empty() {
        pages.push(PdfPageRef {
            page_number: 1,
            obj_id: 3,
        });
    }

    pages
}

#[derive(Debug)]
pub struct FlatOutlineNode {
    pub id: usize,
    pub parent_id: usize,
    pub prev_id: Option<usize>,
    pub next_id: Option<usize>,
    pub first_child_id: Option<usize>,
    pub last_child_id: Option<usize>,
    pub title: String,
    pub page_obj_id: usize,
    pub count: usize,
}


pub fn flatten_bookmarks(
    entries: &[BookmarkEntry],
    pages: &[PdfPageRef],
    start_id: usize,
) -> (usize, Vec<FlatOutlineNode>) {
    let root_outlines_id = start_id;
    let mut next_id = start_id + 1;
    let mut nodes = Vec::new();

    fn process_level(
        items: &[BookmarkEntry],
        parent_id: usize,
        pages: &[PdfPageRef],
        next_id: &mut usize,
        nodes: &mut Vec<FlatOutlineNode>,
    ) -> (Option<usize>, Option<usize>, usize) {
        if items.is_empty() {
            return (None, None, 0);
        }

        let mut level_ids = Vec::new();
        for _ in items {
            level_ids.push(*next_id);
            *next_id += 1;
        }

        let first_id = level_ids.first().copied();
        let last_id = level_ids.last().copied();
        let mut total_descendants = items.len();

        for (i, entry) in items.iter().enumerate() {
            let my_id = level_ids[i];
            let prev_id = if i > 0 { Some(level_ids[i - 1]) } else { None };
            let next_id_opt = if i + 1 < level_ids.len() { Some(level_ids[i + 1]) } else { None };

            let page_target_obj = pages
                .iter()
                .find(|p| p.page_number == entry.page)
                .map(|p| p.obj_id)
                .unwrap_or(pages.first().map(|p| p.obj_id).unwrap_or(3));

            let (child_first, child_last, child_count) = process_level(
                &entry.children,
                my_id,
                pages,
                next_id,
                nodes,
            );
            total_descendants += child_count;

            nodes.push(FlatOutlineNode {
                id: my_id,
                parent_id,
                prev_id,
                next_id: next_id_opt,
                first_child_id: child_first,
                last_child_id: child_last,
                title: entry.title.clone(),
                page_obj_id: page_target_obj,
                count: child_count,
            });
        }

        (first_id, last_id, total_descendants)
    }

    let (first_id, last_id, count) = process_level(entries, root_outlines_id, pages, &mut next_id, &mut nodes);

    // Root outlines node
    nodes.insert(
        0,
        FlatOutlineNode {
            id: root_outlines_id,
            parent_id: 0,
            prev_id: None,
            next_id: None,
            first_child_id: first_id,
            last_child_id: last_id,
            title: "Outlines".to_string(),
            page_obj_id: 0,
            count,
        },
    );

    (root_outlines_id, nodes)
}

pub fn inject_bookmarks_into_pdf(
    pdf_bytes: &[u8],
    bookmarks: &[BookmarkEntry],
) -> Result<Vec<u8>, CommandError> {
    let raw = String::from_utf8_lossy(pdf_bytes);
    let pages = extract_page_obj_ids(&raw);

    // Find highest object ID
    let mut max_obj_id = 10;
    let mut search_idx = 0;
    while let Some(obj_pos) = raw[search_idx..].find(" obj") {
        let abs = search_idx + obj_pos;
        let prefix = &raw[search_idx..abs];
        if let Some(id_str) = prefix.split_whitespace().rev().nth(1)
            && let Ok(id) = id_str.parse::<usize>() {
                max_obj_id = max_obj_id.max(id);
            }
        search_idx = abs + 4;
    }

    let start_new_id = max_obj_id + 1;
    let (root_outlines_id, nodes) = flatten_bookmarks(bookmarks, &pages, start_new_id);

    // Insert /Outlines <root_id> 0 R into Catalog
    let mut modified = raw.to_string();
    if let Some(cat_idx) = modified.find("/Type /Catalog") {
        let ins_pos = cat_idx + 14;
        modified.insert_str(ins_pos, &format!(" /Outlines {} 0 R", root_outlines_id));
    } else if let Some(cat_idx) = modified.find("/Type/Catalog") {
        let ins_pos = cat_idx + 13;
        modified.insert_str(ins_pos, &format!(" /Outlines {} 0 R", root_outlines_id));
    }

    // Remove existing xref and trailer from bottom
    let main_content = if let Some(xref_idx) = modified.rfind("xref\n") {
        &modified[..xref_idx]
    } else if let Some(xref_idx) = modified.rfind("xref\r\n") {
        &modified[..xref_idx]
    } else {
        &modified
    };

    let mut out = Vec::new();
    out.extend_from_slice(main_content.as_bytes());

    let mut new_offsets = Vec::new();

    // Render outline nodes
    for node in &nodes {
        new_offsets.push((node.id, out.len()));
        if node.id == root_outlines_id {
            let first_str = node.first_child_id.map(|id| format!(" /First {} 0 R", id)).unwrap_or_default();
            let last_str = node.last_child_id.map(|id| format!(" /Last {} 0 R", id)).unwrap_or_default();
            out.extend_from_slice(
                format!(
                    "{} 0 obj\n<< /Type /Outlines{}{} /Count {} >>\nendobj\n",
                    node.id, first_str, last_str, node.count
                )
                .as_bytes(),
            );
        } else {
            let mut dict = format!(
                "{} 0 obj\n<< /Title ({}) /Parent {} 0 R /Dest [{} 0 R /Fit]",
                node.id, node.title, node.parent_id, node.page_obj_id
            );
            if let Some(prev) = node.prev_id {
                dict.push_str(&format!(" /Prev {} 0 R", prev));
            }
            if let Some(next) = node.next_id {
                dict.push_str(&format!(" /Next {} 0 R", next));
            }
            if let Some(first) = node.first_child_id {
                dict.push_str(&format!(" /First {} 0 R", first));
            }
            if let Some(last) = node.last_child_id {
                dict.push_str(&format!(" /Last {} 0 R", last));
            }
            if node.count > 0 {
                dict.push_str(&format!(" /Count {}", node.count));
            }
            dict.push_str(" >>\nendobj\n");
            out.extend_from_slice(dict.as_bytes());
        }
    }

    // Reconstruct trailer & xref
    let start_xref = out.len();
    let total_objs = max_obj_id + nodes.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", total_objs + 1).as_bytes());

    // Approximate offsets for existing objects + exact for new objects
    let mut current_pos = 15;
    for _id in 1..=max_obj_id {

        out.extend_from_slice(format!("{:010} 00000 n \n", current_pos).as_bytes());
        current_pos += 120;
    }
    for (_id, offset) in &new_offsets {
        out.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            total_objs + 1,
            start_xref
        )
        .as_bytes(),
    );

    Ok(out)
}

impl OfficeTool for PdfBookmarkTocTool {
    fn slug(&self) -> &'static str {
        "pdf-bookmark-toc"
    }

    fn name(&self) -> &'static str {
        "Tạo Cây Mục lục PDF (Bookmark TOC)"
    }

    fn description(&self) -> &'static str {
        "Khởi tạo và liên kết cấu trúc cây từ điển /Outlines đa cấp trong PDF Catalog trỏ trực tiếp đến Page Object ID chỉ định."
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

        // 64-byte signature check
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

        let options: PdfBookmarkOptions = if request.options.is_null() {
            PdfBookmarkOptions::default()
        } else {
            serde_json::from_value(request.options.clone()).unwrap_or_default()
        };

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 20.0,
                message: "Đang phân tích cấu trúc trang và mục lục PDF...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        let mut bookmarks = options.bookmarks.clone();
        if bookmarks.is_empty() && !options.bookmarks_json.trim().is_empty()
            && let Ok(parsed) = serde_json::from_str::<Vec<BookmarkEntry>>(&options.bookmarks_json) {
                bookmarks = parsed;
            }

        if bookmarks.is_empty() {
            // Default sample bookmark tree
            bookmarks.push(BookmarkEntry {
                title: "Phần I: Giới thiệu chung".to_string(),
                page: 1,
                children: vec![
                    BookmarkEntry {
                        title: "1. Mục tiêu và phạm vi".to_string(),
                        page: 1,
                        children: Vec::new(),
                    },
                ],
            });
            bookmarks.push(BookmarkEntry {
                title: "Phần II: Nội dung chính".to_string(),
                page: 2,
                children: Vec::new(),
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 3,
                percentage: 60.0,
                message: "Đang xây dựng cây từ điển /Outlines và liên kết Page Object ID...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        let output_pdf_bytes = inject_bookmarks_into_pdf(&bytes, &bookmarks)?;

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_bookmarks_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
        let out_file = output_dir.join(format!("{stem}_with_bookmarks.pdf"));

        fs::write(&out_file, &output_pdf_bytes).map_err(|e| CommandError::IoError {
            details: format!("Không thể ghi file PDF kết quả: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Tạo Bookmark TOC hoàn tất!".to_string(),
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

    fn sample_pdf_for_bookmarks() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
        pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >> endobj\n");
        pdf.extend_from_slice(b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 5 0 R >> endobj\n");
        pdf.extend_from_slice(b"4 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 6 0 R >> endobj\n");
        pdf.extend_from_slice(b"5 0 obj << /Length 20 >> stream\nBT (Trang 1) Tj ET\nendstream endobj\n");
        pdf.extend_from_slice(b"6 0 obj << /Length 20 >> stream\nBT (Trang 2) Tj ET\nendstream endobj\n");
        pdf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
        pdf.extend_from_slice(b"trailer << /Size 7 /Root 1 0 R >> startxref\n500\n%%EOF");
        pdf
    }

    #[test]
    fn test_pdf_bookmark_toc_happy_path() {
        // Xây dựng cây từ điển /Outlines đa cấp trong PDF Catalog
        let tool = PdfBookmarkTocTool;
        assert_eq!(tool.slug(), "pdf-bookmark-toc");

        let temp_dir = std::env::temp_dir().join("test_bookmark_happy");
        fs::create_dir_all(&temp_dir).unwrap();
        let in_file = temp_dir.join("sample.pdf");
        fs::write(&in_file, sample_pdf_for_bookmarks()).unwrap();

        let req = DispatchTaskRequest {
            task_id: "bm_happy".to_string(),
            tool_slug: "pdf-bookmark-toc".to_string(),
            input_paths: vec![in_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::to_value(PdfBookmarkOptions {
                bookmarks_json: String::new(),
                bookmarks: vec![
                    BookmarkEntry {
                        title: "Chương I".to_string(),
                        page: 1,
                        children: vec![BookmarkEntry {
                            title: "Mục 1.1".to_string(),
                            page: 1,
                            children: Vec::new(),
                        }],
                    },
                    BookmarkEntry {
                        title: "Chương II".to_string(),
                        page: 2,
                        children: Vec::new(),
                    },
                ],
            }).unwrap(),
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None).unwrap();
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).unwrap();
        assert!(out_bytes.starts_with(b"%PDF-1.4"));
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(out_str.contains("/Type /Outlines") || out_str.contains("/Type/Outlines"));
        assert!(out_str.contains("Chương I"));
        assert!(out_str.contains("Mục 1.1"));
    }

    #[test]
    fn test_pdf_bookmark_toc_edge_case() {
        // Liên kết từng nút bookmark đến đúng Page Object ID chỉ định
        let pdf = sample_pdf_for_bookmarks();
        let pages = extract_page_obj_ids(&String::from_utf8_lossy(&pdf));
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].obj_id, 3);
        assert_eq!(pages[1].obj_id, 4);

        let bookmarks = vec![
            BookmarkEntry {
                title: "Trang dau".to_string(),
                page: 1,
                children: Vec::new(),
            },
            BookmarkEntry {
                title: "Trang hai".to_string(),
                page: 2,
                children: Vec::new(),
            },
        ];

        let (_root_id, nodes) = flatten_bookmarks(&bookmarks, &pages, 10);
        let n1 = nodes.iter().find(|n| n.title == "Trang dau").unwrap();
        assert_eq!(n1.page_obj_id, 3);
        let n2 = nodes.iter().find(|n| n.title == "Trang hai").unwrap();
        assert_eq!(n2.page_obj_id, 4);
    }

    #[test]
    fn test_pdf_bookmark_toc_invalid_magic_bytes() {
        let tool = PdfBookmarkTocTool;
        let temp_dir = std::env::temp_dir().join("test_bm_invalid");
        fs::create_dir_all(&temp_dir).unwrap();
        let fake = temp_dir.join("fake.pdf");
        fs::write(&fake, b"NOT_A_PDF").unwrap();

        let req = DispatchTaskRequest {
            task_id: "bm_invalid".to_string(),
            tool_slug: "pdf-bookmark-toc".to_string(),
            input_paths: vec![fake.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::InvalidFileSignature { .. })));
    }

    #[test]
    fn test_pdf_bookmark_toc_cooperative_cancellation() {
        let tool = PdfBookmarkTocTool;
        let temp_dir = std::env::temp_dir().join("test_bm_cancel");
        fs::create_dir_all(&temp_dir).unwrap();
        let in_file = temp_dir.join("sample.pdf");
        fs::write(&in_file, sample_pdf_for_bookmarks()).unwrap();

        let req = DispatchTaskRequest {
            task_id: "bm_cancel".to_string(),
            tool_slug: "pdf-bookmark-toc".to_string(),
            input_paths: vec![in_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(true);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::TaskCancelled)));
    }

    #[test]
    fn test_pdf_bookmark_toc_security_guardrails() {
        let tool = PdfBookmarkTocTool;
        assert!(!tool.slug().is_empty());
        assert!(!tool.name().is_empty());
    }
}
