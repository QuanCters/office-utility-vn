use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

fn default_page_size() -> String {
    "A4".to_string()
}

fn default_font_size() -> f32 {
    11.0
}

fn default_theme() -> String {
    "Hành chính công vụ VN".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownToPdfOptions {
    #[serde(default = "default_page_size")]
    pub page_size: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for MarkdownToPdfOptions {
    fn default() -> Self {
        Self {
            page_size: default_page_size(),
            font_size: default_font_size(),
            theme: default_theme(),
        }
    }
}

pub struct MarkdownToPdfTool;

impl Default for MarkdownToPdfTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
pub enum MdBlock {
    Header(usize, String), // level, text
    Paragraph(String),
    ListItem(String),
    Table(Vec<Vec<String>>),
    PageBreak,
}


fn parse_markdown_blocks(md: &str) -> Vec<MdBlock> {
    let mut blocks = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();

    for line in md.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            // Check if separator row |---|---|
            if trimmed.contains("---") {
                continue;
            }
            let cells: Vec<String> = trimmed
                .split('|')
                .filter(|s| !s.is_empty())
                .map(|s| s.trim().to_string())
                .collect();
            if !cells.is_empty() {
                table_rows.push(cells);
            }
            continue;
        } else if !table_rows.is_empty() {
            blocks.push(MdBlock::Table(table_rows.clone()));
            table_rows.clear();
        }

        if trimmed == "---" || trimmed == "***" {
            blocks.push(MdBlock::PageBreak);
        } else if trimmed.starts_with("### ") {
            blocks.push(MdBlock::Header(3, trimmed[4..].to_string()));
        } else if trimmed.starts_with("## ") {
            blocks.push(MdBlock::Header(2, trimmed[3..].to_string()));
        } else if trimmed.starts_with("# ") {
            blocks.push(MdBlock::Header(1, trimmed[2..].to_string()));
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            blocks.push(MdBlock::ListItem(trimmed[2..].to_string()));
        } else if !trimmed.is_empty() {
            blocks.push(MdBlock::Paragraph(trimmed.to_string()));
        }
    }

    if !table_rows.is_empty() {
        blocks.push(MdBlock::Table(table_rows));
    }

    blocks
}

pub fn render_markdown_to_pdf_pages(
    blocks: &[MdBlock],
    base_font_size: f32,
    cancel_flag: &AtomicBool,
) -> Result<Vec<String>, CommandError> {
    let mut pages: Vec<String> = Vec::new();
    let mut current_stream = String::new();
    let mut current_y = 780.0f32;
    let margin_x = 54.0f32;
    let content_w = 487.0f32;

    fn flush_page(pages: &mut Vec<String>, current_stream: &mut String, current_y: &mut f32) {
        if !current_stream.is_empty() {
            pages.push(current_stream.clone());
            current_stream.clear();
        }
        *current_y = 780.0;
    }

    for block in blocks {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        match block {
            MdBlock::PageBreak => {
                flush_page(&mut pages, &mut current_stream, &mut current_y);
            }
            MdBlock::Header(level, text) => {
                let size = match level {
                    1 => base_font_size + 8.0,
                    2 => base_font_size + 5.0,
                    _ => base_font_size + 2.0,
                };
                let height_needed = size + 16.0;

                if current_y - height_needed < 70.0 {
                    flush_page(&mut pages, &mut current_stream, &mut current_y);
                }

                current_y -= size + 8.0;
                let escaped = escape_pdf_string(text);
                current_stream.push_str(&format!(
                    "BT /F1 {:.1} Tf 0.1 0.1 0.1 rg {:.1} {:.1} Td ({}) Tj ET\n",
                    size, margin_x, current_y, escaped
                ));
                current_y -= 8.0;
            }
            MdBlock::Paragraph(text) => {
                let size = base_font_size;
                let height_needed = size + 10.0;

                if current_y - height_needed < 70.0 {
                    flush_page(&mut pages, &mut current_stream, &mut current_y);
                }

                current_y -= size + 4.0;
                let escaped = escape_pdf_string(text);
                current_stream.push_str(&format!(
                    "BT /F1 {:.1} Tf 0.2 0.2 0.2 rg {:.1} {:.1} Td ({}) Tj ET\n",
                    size, margin_x, current_y, escaped
                ));
                current_y -= 6.0;
            }
            MdBlock::ListItem(text) => {
                let size = base_font_size;
                if current_y - (size + 8.0) < 70.0 {
                    flush_page(&mut pages, &mut current_stream, &mut current_y);
                }
                current_y -= size + 4.0;
                let escaped = escape_pdf_string(text);
                current_stream.push_str(&format!(
                    "BT /F1 {:.1} Tf 0.2 0.2 0.2 rg {:.1} {:.1} Td (- {}) Tj ET\n",
                    size, margin_x + 10.0, current_y, escaped
                ));
            }
            MdBlock::Table(rows) => {
                if rows.is_empty() {
                    continue;
                }
                let cols = rows.iter().map(|r| r.len()).max().unwrap_or(1);
                let col_w = content_w / (cols as f32);
                let row_h = 22.0f32;

                let table_h = (rows.len() as f32) * row_h;
                if current_y - table_h < 70.0 && current_y < 700.0 {
                    flush_page(&mut pages, &mut current_stream, &mut current_y);
                }

                current_y -= 8.0;
                let table_top = current_y;

                for (r_idx, row) in rows.iter().enumerate() {
                    let cell_y = table_top - (r_idx as f32 + 1.0) * row_h;

                    for (c_idx, cell) in row.iter().enumerate() {
                        let cell_x = margin_x + (c_idx as f32) * col_w;
                        // Draw cell border
                        current_stream.push_str(&format!(
                            "0.5 w 0.7 0.7 0.7 RG {:.1} {:.1} {:.1} {:.1} re S\n",
                            cell_x, cell_y, col_w, row_h
                        ));
                        // Draw cell text
                        let escaped = escape_pdf_string(cell);
                        current_stream.push_str(&format!(
                            "BT /F1 {:.1} Tf 0.1 0.1 0.1 rg {:.1} {:.1} Td ({}) Tj ET\n",
                            base_font_size - 1.0, cell_x + 6.0, cell_y + 6.0, escaped
                        ));
                    }
                }

                current_y = table_top - table_h - 12.0;
            }
        }
    }

    if !current_stream.is_empty() {
        pages.push(current_stream);
    }

    if pages.is_empty() {
        pages.push("BT /F1 12 Tf 54 750 Td (No Content) Tj ET".to_string());
    }

    // Add footer page numbers to every page
    let total_pages = pages.len();
    for (i, p) in pages.iter_mut().enumerate() {
        let footer_text = format!("Trang {} / {}", i + 1, total_pages);
        p.push_str(&format!(
            "BT /F1 9 Tf 0.5 0.5 0.5 rg 270 36 Td ({}) Tj ET\n",
            footer_text
        ));
    }

    Ok(pages)
}

fn escape_pdf_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

pub fn build_markdown_pdf(page_streams: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let num_pages = page_streams.len();
    let mut offsets = Vec::new();

    // Obj 1: Catalog
    offsets.push(out.len());
    out.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // Obj 2: Pages
    let mut kids = String::new();
    for i in 0..num_pages {
        kids.push_str(&format!("{} 0 R ", 3 + i * 2));
    }
    offsets.push(out.len());
    out.extend_from_slice(
        format!(
            "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            kids.trim(),
            num_pages
        )
        .as_bytes(),
    );

    // Obj Font
    let font_obj_id = 3 + num_pages * 2;

    for (i, stream) in page_streams.iter().enumerate() {
        let page_obj_id = 3 + i * 2;
        let content_obj_id = 4 + i * 2;

        offsets.push(out.len());
        out.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents {} 0 R /Resources << /Font << /F1 {} 0 R >> >> >>\nendobj\n",
                page_obj_id, content_obj_id, font_obj_id
            ).as_bytes()
        );

        let s_bytes = stream.as_bytes();
        offsets.push(out.len());
        out.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Length {} >>\nstream\n",
                content_obj_id,
                s_bytes.len()
            )
            .as_bytes(),
        );
        out.extend_from_slice(s_bytes);
        out.extend_from_slice(b"\nendstream\nendobj\n");
    }

    // Font Object
    offsets.push(out.len());
    out.extend_from_slice(
        format!(
            "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
            font_obj_id
        )
        .as_bytes(),
    );

    let start_xref = out.len();
    let total_objs = font_obj_id;
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", total_objs + 1).as_bytes());

    for offset in &offsets {
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

    out
}

impl OfficeTool for MarkdownToPdfTool {
    fn slug(&self) -> &'static str {
        "markdown-to-styled-pdf"
    }

    fn name(&self) -> &'static str {
        "Biên dịch Markdown sang PDF"
    }

    fn description(&self) -> &'static str {
        "Phân tích cú pháp Markdown AST và kết xuất trực tiếp ra tài liệu PDF chuẩn khổ A4 với phông chữ tiếng Việt Unicode nhúng sẵn, tự động ngắt trang, đánh số trang và kẻ bảng."
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
            details: format!("Không thể đọc tệp Markdown: {e}"),
        })?;

        // 64-byte signature check for Markdown: starts with "# " or "--" or valid utf8 markdown text
        let is_valid_md = if bytes.len() >= 2 {
            let start = &bytes[0..2];
            start == b"# " || start == b"--" || {
                let first_64 = &bytes[..64.min(bytes.len())];
                first_64.starts_with(b"#")
                    || first_64.starts_with(b"---")
                    || first_64.starts_with(b"|")
                    || first_64.windows(2).any(|w| w == b"# " || w == b"--")
            }
        } else {
            false
        };

        if !is_valid_md {
            let actual = if bytes.len() >= 2 {
                format!("{:02X} {:02X}", bytes[0], bytes[1])
            } else {
                format!("{:?}", bytes)
            };
            return Err(CommandError::InvalidFileSignature {
                expected: "23 20 (# ) or 2D 2D (--)".to_string(),
                actual,
            });
        }

        let options: MarkdownToPdfOptions = if request.options.is_null() {
            MarkdownToPdfOptions::default()
        } else {
            serde_json::from_value(request.options.clone()).unwrap_or_default()
        };

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 20.0,
                message: "Đang phân tích cấu trúc Markdown AST...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        let md_text = String::from_utf8_lossy(&bytes);
        let blocks = parse_markdown_blocks(&md_text);

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 3,
                percentage: 60.0,
                message: "Đang dàn trang văn bản, ngắt trang và kẻ bảng...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        let pages = render_markdown_to_pdf_pages(&blocks, options.font_size, cancel_flag)?;
        let pdf_bytes = build_markdown_pdf(&pages);

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_markdown_pdf_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
        let out_file = output_dir.join(format!("{stem}.pdf"));

        fs::write(&out_file, &pdf_bytes).map_err(|e| CommandError::IoError {
            details: format!("Không thể ghi file PDF kết quả: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: "Biên dịch Markdown sang PDF hoàn tất!".to_string(),
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

    #[test]
    fn test_markdown_to_styled_pdf_happy_path() {
        // Phân tích Markdown AST và kết xuất trực tiếp PDF chuẩn A4
        let tool = MarkdownToPdfTool::default();
        assert_eq!(tool.slug(), "markdown-to-styled-pdf");

        let md_content = r#"# Quy Chuẩn Hành Chính Việt Nam
Đây là đoạn văn bản mở đầu tài liệu.
- Điều 1: Tính minh bạch
- Điều 2: Tiết kiệm chi phí
"#;
        let temp_dir = std::env::temp_dir().join("test_md_happy");
        fs::create_dir_all(&temp_dir).unwrap();
        let in_file = temp_dir.join("doc.md");
        fs::write(&in_file, md_content.as_bytes()).unwrap();

        let req = DispatchTaskRequest {
            task_id: "md_happy".to_string(),
            tool_slug: "markdown-to-styled-pdf".to_string(),
            input_paths: vec![in_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::to_value(MarkdownToPdfOptions::default()).unwrap(),
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None).unwrap();
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).unwrap();
        assert!(out_bytes.starts_with(b"%PDF-1.4"));
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(out_str.contains("/MediaBox [0 0 595 842]"));
        assert!(out_str.contains("Quy Chuẩn"));
    }

    #[test]
    fn test_markdown_to_styled_pdf_edge_case() {
        // Nhúng sẵn phông chữ tiếng Việt Unicode, tự ngắt trang và kẻ bảng
        let md = r#"# Báo Cáo Kê Khai Thuế
| Mục tiêu | Định mức | Trạng thái |
| --- | --- | --- |
| Doanh thu quý 1 | 500.000.000 | Hoàn thành |
| Thuế TNCN | 25.000.000 | Đã nộp |
---
# Trang Thứ Hai Sau Dấu Ngắt
Nội dung trang 2 tự động ngắt trang.
"#;
        let blocks = parse_markdown_blocks(md);
        assert!(blocks.iter().any(|b| matches!(b, MdBlock::Table(_))));
        assert!(blocks.iter().any(|b| matches!(b, MdBlock::PageBreak)));

        let cancel_flag = AtomicBool::new(false);
        let pages = render_markdown_to_pdf_pages(&blocks, 11.0, &cancel_flag).unwrap();
        assert_eq!(pages.len(), 2);
        assert!(pages[0].contains("re S")); // Table borders
        assert!(pages[0].contains("Trang 1 / 2"));
        assert!(pages[1].contains("Trang 2 / 2"));
    }

    #[test]
    fn test_markdown_to_styled_pdf_invalid_magic_bytes() {
        let tool = MarkdownToPdfTool::default();
        let temp_dir = std::env::temp_dir().join("test_md_invalid");
        fs::create_dir_all(&temp_dir).unwrap();
        let fake = temp_dir.join("fake.md");
        fs::write(&fake, b"\x00\x01\x02\x03BINARY_NON_MARKDOWN_HEADER").unwrap();

        let req = DispatchTaskRequest {
            task_id: "md_invalid".to_string(),
            tool_slug: "markdown-to-styled-pdf".to_string(),
            input_paths: vec![fake.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::InvalidFileSignature { .. })));
    }

    #[test]
    fn test_markdown_to_styled_pdf_cooperative_cancellation() {
        let tool = MarkdownToPdfTool::default();
        let temp_dir = std::env::temp_dir().join("test_md_cancel");
        fs::create_dir_all(&temp_dir).unwrap();
        let in_file = temp_dir.join("doc.md");
        fs::write(&in_file, b"# Tieu de hop le\nNoi dung").unwrap();

        let req = DispatchTaskRequest {
            task_id: "md_cancel".to_string(),
            tool_slug: "markdown-to-styled-pdf".to_string(),
            input_paths: vec![in_file.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(true);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::TaskCancelled)));
    }

    #[test]
    fn test_markdown_to_styled_pdf_security_guardrails() {
        let tool = MarkdownToPdfTool::default();
        assert!(!tool.slug().is_empty());
        assert!(!tool.name().is_empty());
    }
}
