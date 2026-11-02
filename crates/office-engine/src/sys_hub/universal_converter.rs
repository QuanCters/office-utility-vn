use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use flate2::read::DeflateDecoder;
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::data_lab::{DataStructureConverterTool, VietnameseFontConverterTool};
use crate::doc_suite::{
    MarkdownToPdfTool, PdfMergeSplitTool, PdfTableExtractorTool, PdfToLongImageTool,
    PdfToMarkdownTool, VietnameseOcrTool,
};
use crate::media_studio::ImageBatchTool;
use crate::OfficeTool;

/// Cấu hình tùy chọn trạm chuyển đổi tệp vạn năng
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalConverterOptions {
    pub target_extension: String,
}

/// Cấu trúc đại diện một tệp tin bên trong ZIP archive
#[derive(Debug, Clone)]
pub struct ZipEntry {
    pub name: String,
    pub data: Vec<u8>,
}

/// Đọc các entry từ mảng byte ZIP thuần Rust, an toàn chống Zip Bomb (< 50MB per entry)
pub fn parse_zip_entries(raw: &[u8], cancel_flag: &AtomicBool) -> Result<Vec<ZipEntry>, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let mut entries = Vec::new();
    let mut offset = 0;
    const MAX_ENTRY_SIZE: usize = 50 * 1024 * 1024; // 50MB per entry

    while offset + 30 <= raw.len() {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        if &raw[offset..offset + 4] != b"PK\x03\x04" {
            break;
        }

        let compression = u16::from_le_bytes([raw[offset + 8], raw[offset + 9]]);
        let comp_size = u32::from_le_bytes([
            raw[offset + 18],
            raw[offset + 19],
            raw[offset + 20],
            raw[offset + 21],
        ]) as usize;
        let name_len = u16::from_le_bytes([raw[offset + 26], raw[offset + 27]]) as usize;
        let extra_len = u16::from_le_bytes([raw[offset + 28], raw[offset + 29]]) as usize;

        let name_start = offset + 30;
        let name_end = name_start + name_len;
        if name_end > raw.len() {
            break;
        }

        let name = String::from_utf8_lossy(&raw[name_start..name_end]).to_string();
        let data_start = name_end + extra_len;
        let data_end = data_start + comp_size;

        if data_end > raw.len() {
            break;
        }

        let comp_data = &raw[data_start..data_end];

        let decompressed_data = if compression == 8 {
            let mut decoder = DeflateDecoder::new(comp_data).take(MAX_ENTRY_SIZE as u64);
            let mut decomp = Vec::new();
            if decoder.read_to_end(&mut decomp).is_err() {
                comp_data.to_vec()
            } else {
                decomp
            }
        } else {
            comp_data.to_vec()
        };

        entries.push(ZipEntry {
            name,
            data: decompressed_data,
        });

        offset = data_end;
    }

    Ok(entries)
}

/// Trích xuất toàn bộ văn bản từ tệp DOCX (OpenXML word/document.xml)
pub fn extract_text_from_docx_bytes(bytes: &[u8], cancel_flag: &AtomicBool) -> Result<String, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let entries = parse_zip_entries(bytes, cancel_flag)?;
    let doc_entry = entries
        .iter()
        .find(|e| e.name == "word/document.xml" || e.name.ends_with("/document.xml"))
        .ok_or_else(|| CommandError::ValidationFailed {
            reason: "Không tìm thấy cấu trúc word/document.xml trong tệp DOCX".to_string(),
        })?;

    let xml = String::from_utf8_lossy(&doc_entry.data);
    let mut result_lines = Vec::new();

    // Tách theo từng paragraph <w:p>
    for p_chunk in xml.split("<w:p") {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }
        let mut p_text = String::new();
        let mut cur = p_chunk;
        while let Some(start_t) = cur.find("<w:t") {
            let after_tag = &cur[start_t..];
            if let Some(close_tag) = after_tag.find('>') {
                let content_start = &after_tag[close_tag + 1..];
                if let Some(end_t) = content_start.find("</w:t>") {
                    let text = &content_start[..end_t];
                    let unescaped = text
                        .replace("&amp;", "&")
                        .replace("&lt;", "<")
                        .replace("&gt;", ">")
                        .replace("&quot;", "\"")
                        .replace("&apos;", "'");
                    p_text.push_str(&unescaped);
                    cur = &content_start[end_t + 6..];
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let trimmed = p_text.trim();
        if !trimmed.is_empty() {
            result_lines.push(trimmed.to_string());
        }
    }

    Ok(result_lines.join("\n\n"))
}

/// Trích xuất bảng dữ liệu từ tệp XLSX (OpenXML xl/sharedStrings.xml & xl/worksheets/sheet1.xml)
pub fn extract_csv_from_xlsx_bytes(bytes: &[u8], cancel_flag: &AtomicBool) -> Result<String, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let entries = parse_zip_entries(bytes, cancel_flag)?;

    // 1. Đọc sharedStrings nếu có
    let mut shared_strings: Vec<String> = Vec::new();
    if let Some(sst_entry) = entries.iter().find(|e| e.name.ends_with("sharedStrings.xml")) {
        let sst_xml = String::from_utf8_lossy(&sst_entry.data);
        for si_chunk in sst_xml.split("<si>") {
            let mut s_text = String::new();
            let mut cur = si_chunk;
            while let Some(start_t) = cur.find("<t") {
                let after_tag = &cur[start_t..];
                if let Some(close_tag) = after_tag.find('>') {
                    let content_start = &after_tag[close_tag + 1..];
                    if let Some(end_t) = content_start.find("</t>") {
                        let text = &content_start[..end_t];
                        s_text.push_str(text);
                        cur = &content_start[end_t + 4..];
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            if !s_text.is_empty() {
                let unescaped = s_text
                    .replace("&amp;", "&")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&quot;", "\"")
                    .replace("&apos;", "'");
                shared_strings.push(unescaped);
            }
        }
    }

    // 2. Đọc sheet1.xml
    let sheet_entry = entries
        .iter()
        .find(|e| e.name.ends_with("sheet1.xml") || (e.name.contains("sheet") && e.name.ends_with(".xml")))
        .ok_or_else(|| CommandError::ValidationFailed {
            reason: "Không tìm thấy trang tính (worksheet) trong tệp XLSX".to_string(),
        })?;

    let sheet_xml = String::from_utf8_lossy(&sheet_entry.data);
    let mut csv_rows: Vec<String> = Vec::new();

    for row_chunk in sheet_xml.split("<row ") {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let mut row_vals: Vec<String> = Vec::new();
        for c_chunk in row_chunk.split("<c ") {
            if !c_chunk.contains("</c>") {
                continue;
            }
            let is_shared = c_chunk.contains("t=\"s\"");
            if let Some(v_start) = c_chunk.find("<v>") {
                let after_v = &c_chunk[v_start + 3..];
                if let Some(v_end) = after_v.find("</v>") {
                    let raw_val = &after_v[..v_end];
                    let val = if is_shared {
                        if let Ok(idx) = raw_val.parse::<usize>() {
                            shared_strings.get(idx).cloned().unwrap_or_default()
                        } else {
                            raw_val.to_string()
                        }
                    } else {
                        raw_val.to_string()
                    };

                    if val.contains(',') || val.contains('"') || val.contains('\n') {
                        let escaped = val.replace('"', "\"\"");
                        row_vals.push(format!("\"{escaped}\""));
                    } else {
                        row_vals.push(val);
                    }
                }
            }
        }

        if !row_vals.is_empty() {
            csv_rows.push(row_vals.join(","));
        }
    }

    if csv_rows.is_empty() {
        return Err(CommandError::ValidationFailed {
            reason: "Trang tính XLSX không có dữ liệu".to_string(),
        });
    }

    Ok(csv_rows.join("\n"))
}

pub struct UniversalConverterTool {
    pdf_tool: PdfMergeSplitTool,
    pdf_to_md_tool: PdfToMarkdownTool,
    pdf_to_img_tool: PdfToLongImageTool,
    pdf_ocr_tool: VietnameseOcrTool,
    pdf_table_tool: PdfTableExtractorTool,
    md_to_pdf_tool: MarkdownToPdfTool,
    image_tool: ImageBatchTool,
    font_tool: VietnameseFontConverterTool,
    data_struct_tool: DataStructureConverterTool,
}

impl Default for UniversalConverterTool {
    fn default() -> Self {
        Self {
            pdf_tool: PdfMergeSplitTool,
            pdf_to_md_tool: PdfToMarkdownTool,
            pdf_to_img_tool: PdfToLongImageTool,
            pdf_ocr_tool: VietnameseOcrTool,
            pdf_table_tool: PdfTableExtractorTool,
            md_to_pdf_tool: MarkdownToPdfTool,
            image_tool: ImageBatchTool,
            font_tool: VietnameseFontConverterTool,
            data_struct_tool: DataStructureConverterTool,
        }
    }
}

/// Loại định dạng đã phân tích qua magic bytes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileCategory {
    DocumentPdf,
    DocumentDocx,
    TableXlsx,
    TableCsv,
    DocumentMarkdown,
    ImageMedia(String),
    TextData,
    Unsupported(String),
}

/// Nhận diện loại tệp qua chữ ký magic bytes bằng crate `infer`
pub fn inspect_file_category(path: &Path) -> Result<FileCategory, CommandError> {
    if !path.exists() {
        return Err(CommandError::FileNotFound {
            path: path.to_string_lossy().to_string(),
        });
    }

    let bytes = fs::read(path).map_err(|e| CommandError::IoError {
        details: format!("Không thể đọc tệp: {e}"),
    })?;

    if bytes.is_empty() {
        return Err(CommandError::InvalidFileSignature {
            expected: "valid header".to_string(),
            actual: "Tệp rỗng (0 bytes)".to_string(),
        });
    }

    // Kiểm tra PDF
    if bytes.starts_with(b"%PDF-") {
        return Ok(FileCategory::DocumentPdf);
    }

    // Kiểm tra ZIP OpenXML (DOCX hoặc XLSX)
    if bytes.starts_with(b"PK\x03\x04") {
        if bytes.windows(5).any(|w| w == b"word/") {
            return Ok(FileCategory::DocumentDocx);
        }
        if bytes.windows(3).any(|w| w == b"xl/") {
            return Ok(FileCategory::TableXlsx);
        }
        return Ok(FileCategory::Unsupported("ZIP không thuộc OpenXML".to_string()));
    }

    // Kiểm tra ảnh bằng infer
    if let Some(kind) = infer::get(&bytes) {
        let mime = kind.mime_type();
        if mime.starts_with("image/") {
            return Ok(FileCategory::ImageMedia(kind.extension().to_string()));
        }
    }

    // Nhận diện thủ công JPEG, BMP, PPM, WebP
    if bytes.starts_with(&[0xFF, 0xD8]) {
        return Ok(FileCategory::ImageMedia("jpeg".to_string()));
    }
    if bytes.starts_with(b"BM") {
        return Ok(FileCategory::ImageMedia("bmp".to_string()));
    }
    if bytes.starts_with(b"P6") {
        return Ok(FileCategory::ImageMedia("ppm".to_string()));
    }
    if bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
        return Ok(FileCategory::ImageMedia("webp".to_string()));
    }

    // Kiểm tra nếu là tệp văn bản hợp lệ UTF-8 không chứa byte null
    if !bytes.contains(&0) && let Ok(txt_content) = std::str::from_utf8(&bytes) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ext == "csv" || (txt_content.contains(',') && txt_content.contains('\n')) {
            return Ok(FileCategory::TableCsv);
        }
        if ext == "md" || ext == "mdx" || txt_content.starts_with("# ") || txt_content.contains("\n# ") {
            return Ok(FileCategory::DocumentMarkdown);
        }
        return Ok(FileCategory::TextData);
    }

    let sample_hex = bytes
        .iter()
        .take(8)
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(FileCategory::Unsupported(sample_hex))
}

impl OfficeTool for UniversalConverterTool {
    fn slug(&self) -> &'static str {
        "universal-converter"
    }

    fn name(&self) -> &'static str {
        "Trạm Chuyển đổi Tệp Vạn năng"
    }

    fn description(&self) -> &'static str {
        "Tự động nhận diện định dạng tệp thông qua magic bytes (infer) và điều phối tác vụ chuyển đổi đến phân hệ chuyên trách tương ứng."
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

        let options: UniversalConverterOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình UniversalConverter không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp đầu vào trống".to_string(),
            });
        }

        let target_ext = options.target_extension.trim().trim_start_matches('.').to_lowercase();
        if target_ext.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Chưa cung cấp định dạng đầu ra (target_extension)".to_string(),
            });
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Bắt đầu nhận diện magic bytes các tệp đầu vào...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: request.input_paths.len(),
                is_completed: false,
            });
        }

        let mut output_paths = Vec::new();
        let mut failed_items = Vec::new();
        let total_files = request.input_paths.len();

        for (idx, file_path_str) in request.input_paths.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let path = Path::new(file_path_str);
            let category = match inspect_file_category(path) {
                Ok(cat) => cat,
                Err(err) => {
                    if total_files == 1 {
                        return Err(err);
                    }
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("{:?}", err),
                    });
                    continue;
                }
            };

            let single_req = DispatchTaskRequest {
                task_id: format!("{}-{}", request.task_id, idx),
                tool_slug: request.tool_slug.clone(),
                input_paths: vec![file_path_str.clone()],
                output_directory: request.output_directory.clone(),
                options: serde_json::json!({}),
            };

            let sub_res: Result<DispatchTaskResponse, CommandError> = match (&category, target_ext.as_str()) {
                // 1. Documents: PDF conversions
                (FileCategory::DocumentPdf, "md" | "markdown") => {
                    let mut pdf_req = single_req.clone();
                    pdf_req.tool_slug = "pdf-to-markdown".to_string();
                    pdf_req.options = serde_json::json!({});
                    self.pdf_to_md_tool.execute(&pdf_req, cancel_flag, None)
                }

                (FileCategory::DocumentPdf, "png" | "jpg" | "jpeg" | "image") => {
                    let mut img_req = single_req.clone();
                    img_req.tool_slug = "pdf-to-long-image".to_string();
                    img_req.options = serde_json::json!({});
                    self.pdf_to_img_tool.execute(&img_req, cancel_flag, None)
                }

                (FileCategory::DocumentPdf, "txt" | "text" | "ocr") => {
                    let mut ocr_req = single_req.clone();
                    ocr_req.tool_slug = "vietnamese-ocr".to_string();
                    ocr_req.options = serde_json::json!({});
                    self.pdf_ocr_tool.execute(&ocr_req, cancel_flag, None)
                }

                (FileCategory::DocumentPdf, "pdf") => {
                    let mut pdf_req = single_req.clone();
                    pdf_req.tool_slug = "pdf-merge-split".to_string();
                    pdf_req.options = serde_json::json!({
                        "mode": "split",
                        "page_ranges": "1"
                    });
                    self.pdf_tool.execute(&pdf_req, cancel_flag, None)
                }

                (FileCategory::DocumentPdf, "xlsx" | "excel") => {
                    let mut req = single_req.clone();
                    req.tool_slug = "pdf-table-extractor".to_string();
                    req.options = serde_json::json!({
                        "output_csv": false,
                        "page_range": None::<String>,
                    });
                    self.pdf_table_tool.execute(&req, cancel_flag, None)
                }

                (FileCategory::DocumentPdf, "csv") => {
                    let mut req = single_req.clone();
                    req.tool_slug = "pdf-table-extractor".to_string();
                    req.options = serde_json::json!({
                        "output_csv": true,
                        "page_range": None::<String>,
                    });
                    let mut res = self.pdf_table_tool.execute(&req, cancel_flag, None)?;
                    res.output_paths.retain(|p| p.ends_with(".csv"));
                    Ok(res)
                }

                // 2. Documents: Markdown & Text -> PDF
                (FileCategory::DocumentMarkdown | FileCategory::TextData, "pdf") => {
                    let mut md_req = single_req.clone();
                    md_req.tool_slug = "markdown-to-styled-pdf".to_string();
                    md_req.options = serde_json::json!({
                        "theme": "Hành chính công vụ VN",
                        "embed_vietnamese_font": true
                    });
                    self.md_to_pdf_tool.execute(&md_req, cancel_flag, None)
                }

                // 3. Documents: DOCX -> Markdown / Text
                (FileCategory::DocumentDocx, "md" | "markdown" | "txt" | "text") => {
                    let raw_bytes = fs::read(path).map_err(|e| CommandError::IoError {
                        details: e.to_string(),
                    })?;
                    let text = extract_text_from_docx_bytes(&raw_bytes, cancel_flag)?;
                    let out_dir = match &request.output_directory {
                        Some(d) => PathBuf::from(d),
                        None => path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from(".")),
                    };
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
                    let ext = if target_ext.starts_with("md") { "md" } else { "txt" };
                    let out_path = out_dir.join(format!("{stem}.{ext}"));
                    fs::write(&out_path, text.as_bytes()).map_err(|e| CommandError::IoError {
                        details: e.to_string(),
                    })?;
                    Ok(DispatchTaskResponse {
                        task_id: single_req.task_id.clone(),
                        status: "SUCCESS".to_string(),
                        output_paths: vec![out_path.to_string_lossy().to_string()],
                        failed_items: vec![],
                        elapsed_ms: start_time.elapsed().as_millis() as u64,
                    })
                }

                // 4. Tables: CSV -> JSON / SQL / Markdown
                (FileCategory::TableCsv, "json" | "sql" | "md" | "markdown") => {
                    let mut req = single_req.clone();
                    req.tool_slug = "data-structure-converter".to_string();
                    req.options = serde_json::json!({
                        "target_format": target_ext
                    });
                    self.data_struct_tool.execute(&req, cancel_flag, None)
                }

                // 5. Tables: XLSX -> CSV / JSON / SQL / Markdown
                (FileCategory::TableXlsx, "csv") => {
                    let raw_bytes = fs::read(path).map_err(|e| CommandError::IoError {
                        details: e.to_string(),
                    })?;
                    let csv_content = extract_csv_from_xlsx_bytes(&raw_bytes, cancel_flag)?;
                    let out_dir = match &request.output_directory {
                        Some(d) => PathBuf::from(d),
                        None => path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from(".")),
                    };
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("table");
                    let out_path = out_dir.join(format!("{stem}.csv"));
                    fs::write(&out_path, csv_content.as_bytes()).map_err(|e| CommandError::IoError {
                        details: e.to_string(),
                    })?;
                    Ok(DispatchTaskResponse {
                        task_id: single_req.task_id.clone(),
                        status: "SUCCESS".to_string(),
                        output_paths: vec![out_path.to_string_lossy().to_string()],
                        failed_items: vec![],
                        elapsed_ms: start_time.elapsed().as_millis() as u64,
                    })
                }

                (FileCategory::TableXlsx, "json" | "sql" | "md" | "markdown") => {
                    let raw_bytes = fs::read(path).map_err(|e| CommandError::IoError {
                        details: e.to_string(),
                    })?;
                    let csv_content = extract_csv_from_xlsx_bytes(&raw_bytes, cancel_flag)?;
                    let mut req = single_req.clone();
                    req.tool_slug = "data-structure-converter".to_string();
                    req.options = serde_json::json!({
                        "target_format": target_ext,
                        "text": csv_content
                    });
                    self.data_struct_tool.execute(&req, cancel_flag, None)
                }

                // 6. Images: PNG, JPEG, BMP, WebP cross-conversions
                (FileCategory::ImageMedia(_), "png" | "jpg" | "jpeg" | "bmp" | "webp") => {
                    let mut img_req = single_req.clone();
                    img_req.tool_slug = "image-batch".to_string();
                    img_req.options = serde_json::json!({
                        "target_format": target_ext,
                        "quality": 90,
                        "strip_exif": true
                    });
                    self.image_tool.execute(&img_req, cancel_flag, None)
                }

                // 7. Text encoding fallback
                (FileCategory::TextData | FileCategory::TableCsv | FileCategory::DocumentMarkdown, "txt" | "text" | "unicode") => {
                    let mut font_req = single_req.clone();
                    font_req.tool_slug = "vietnamese-font-converter".to_string();
                    font_req.options = serde_json::json!({
                        "source_encoding": "auto",
                        "target_encoding": "unicode"
                    });
                    self.font_tool.execute(&font_req, cancel_flag, None)
                }

                // 8. Định dạng không được hỗ trợ
                (FileCategory::Unsupported(hex_sig), _) => {
                    let err = CommandError::ValidationFailed {
                        reason: format!(
                            "Định dạng tệp không được hỗ trợ (Chữ ký: {hex_sig}). Ứng dụng hỗ trợ PDF, DOCX, XLSX, CSV, Markdown, TXT và Hình ảnh (PNG, JPEG, BMP, WebP)."
                        ),
                    };
                    if total_files == 1 {
                        return Err(err);
                    }
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("{:?}", err),
                    });
                    continue;
                }

                // 9. Cặp chuyển đổi chưa khả thi
                (cat, target) => {
                    let err = CommandError::ValidationFailed {
                        reason: format!(
                            "Chưa hỗ trợ chuyển đổi từ định dạng {:?} sang .{}",
                            cat, target
                        ),
                    };
                    if total_files == 1 {
                        return Err(err);
                    }
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("{:?}", err),
                    });
                    continue;
                }
            };

            match sub_res {
                Ok(resp) => {
                    output_paths.extend(resp.output_paths);
                }
                Err(err) => {
                    if total_files == 1 {
                        return Err(err);
                    }
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("{:?}", err),
                    });
                }
            }

            if let Some(prog) = on_progress {
                let pct = 10.0 + ((idx + 1) as f32 / total_files as f32) * 85.0;
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: pct,
                    message: format!("Đã điều phối và chuyển đổi {}/{} tệp...", idx + 1, total_files),
                    processed_files: idx + 1,
                    failed_files: failed_items.len(),
                    total_files,
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
                message: "Hoàn tất điều phối chuyển đổi vạn năng.".to_string(),
                processed_files: output_paths.len(),
                failed_files: failed_items.len(),
                total_files,
                is_completed: true,
            });
        }

        let status = if failed_items.is_empty() {
            "SUCCESS".to_string()
        } else if !output_paths.is_empty() {
            "PARTIAL_SUCCESS".to_string()
        } else {
            "FAILED".to_string()
        };

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status,
            output_paths,
            failed_items,
            elapsed_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use serde_json::json;

    fn create_dummy_bmp() -> Vec<u8> {
        let raw = crate::media_studio::image_batch::RawImage {
            width: 16,
            height: 16,
            pixels: vec![120; 16 * 16 * 3],
        };
        crate::media_studio::image_batch::encode_bmp(&raw).expect("encode bmp")
    }

    #[test]
    fn test_universal_converter_happy_path() {
        let tool = UniversalConverterTool::default();
        let tmp = std::env::temp_dir().join("test_univ_happy");
        let _ = fs::create_dir_all(&tmp);

        let img_path = tmp.join("sample.bmp");
        fs::write(&img_path, create_dummy_bmp()).expect("write bmp");

        let req = DispatchTaskRequest {
            task_id: "task-univ-happy".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![img_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "target_extension": "png"
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("dispatch ok");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);
        let out_bytes = fs::read(&res.output_paths[0]).expect("read converted");
        assert!(out_bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn test_universal_converter_edge_case() {
        let tool = UniversalConverterTool::default();
        let tmp = std::env::temp_dir().join("test_univ_edge");
        let _ = fs::create_dir_all(&tmp);

        // Tệp định dạng không được hỗ trợ (chứa byte lạ)
        let unk_path = tmp.join("unknown.dat");
        fs::write(&unk_path, b"\x00\x01\x02\x03\x04\x05\x06\x07").expect("write unk");

        let req = DispatchTaskRequest {
            task_id: "task-univ-edge".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![unk_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_extension": "pdf" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("không được hỗ trợ"));
            }
            other => panic!("Kỳ vọng ValidationFailed nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_universal_converter_invalid_magic_bytes() {
        let tool = UniversalConverterTool::default();
        let tmp = std::env::temp_dir().join("test_univ_invalid");
        let _ = fs::create_dir_all(&tmp);

        // Tệp rỗng 0 byte
        let empty_path = tmp.join("empty.bin");
        fs::write(&empty_path, b"").expect("write empty");

        let req = DispatchTaskRequest {
            task_id: "task-univ-empty".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![empty_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_extension": "png" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature với tệp rỗng, nhận: {:?}", other),
        }
    }

    #[test]
    fn test_universal_converter_cooperative_cancellation() {
        let tool = UniversalConverterTool::default();
        let req = DispatchTaskRequest {
            task_id: "task-cancel-univ".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec!["fake.jpg".to_string()],
            output_directory: None,
            options: json!({ "target_extension": "png" }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_universal_converter_security_guardrails() {
        let tool = UniversalConverterTool::default();
        assert_eq!(tool.slug(), "universal-converter");
        assert_eq!(tool.name(), "Trạm Chuyển đổi Tệp Vạn năng");
    }

    fn create_test_zip(filename: &str, data: &[u8]) -> Vec<u8> {
        let mut zip = Vec::new();
        zip.extend_from_slice(b"PK\x03\x04");
        zip.extend_from_slice(&[20, 0]);
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(&[0, 0]); // Stored
        zip.extend_from_slice(&[0, 0, 0, 0]);
        zip.extend_from_slice(&[0, 0, 0, 0]);
        let size = (data.len() as u32).to_le_bytes();
        zip.extend_from_slice(&size);
        zip.extend_from_slice(&size);
        let name_bytes = filename.as_bytes();
        let name_len = (name_bytes.len() as u16).to_le_bytes();
        zip.extend_from_slice(&name_len);
        zip.extend_from_slice(&[0, 0]);
        zip.extend_from_slice(name_bytes);
        zip.extend_from_slice(data);
        zip
    }

    #[test]
    fn test_inspect_file_category_extended() {
        let tmp = std::env::temp_dir().join("test_univ_inspect_ext");
        let _ = fs::create_dir_all(&tmp);

        // PDF
        let pdf_file = tmp.join("sample.pdf");
        fs::write(&pdf_file, b"%PDF-1.7 sample content").expect("write pdf");
        assert_eq!(inspect_file_category(&pdf_file).unwrap(), FileCategory::DocumentPdf);

        // DOCX (ZIP chứa word/)
        let docx_file = tmp.join("sample.docx");
        fs::write(&docx_file, create_test_zip("word/document.xml", b"<w:document/>")).expect("write docx");
        assert_eq!(inspect_file_category(&docx_file).unwrap(), FileCategory::DocumentDocx);

        // XLSX (ZIP chứa xl/)
        let xlsx_file = tmp.join("sample.xlsx");
        fs::write(&xlsx_file, create_test_zip("xl/workbook.xml", b"<workbook/>")).expect("write xlsx");
        assert_eq!(inspect_file_category(&xlsx_file).unwrap(), FileCategory::TableXlsx);

        // CSV
        let csv_file = tmp.join("sample.csv");
        fs::write(&csv_file, "ID,Name,Amount\n1,Alice,100\n").expect("write csv");
        assert_eq!(inspect_file_category(&csv_file).unwrap(), FileCategory::TableCsv);

        // Markdown
        let md_file = tmp.join("sample.md");
        fs::write(&md_file, "# Main Title\n\nSome paragraphs").expect("write md");
        assert_eq!(inspect_file_category(&md_file).unwrap(), FileCategory::DocumentMarkdown);

        // PNG Image
        let png_file = tmp.join("sample.png");
        fs::write(&png_file, b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR").expect("write png");
        assert!(matches!(inspect_file_category(&png_file).unwrap(), FileCategory::ImageMedia(ext) if ext == "png"));

        // BMP Image
        let bmp_file = tmp.join("sample.bmp");
        fs::write(&bmp_file, b"BM123456").expect("write bmp");
        assert_eq!(inspect_file_category(&bmp_file).unwrap(), FileCategory::ImageMedia("bmp".to_string()));
    }

    #[test]
    fn test_docx_and_xlsx_extractors() {
        let cancel = AtomicBool::new(false);

        // 1. DOCX test
        let docx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:t>Tiêu đề hợp đồng</w:t></w:p>
    <w:p><w:t>Điều 1: Phạm vi công việc.</w:t></w:p>
  </w:body>
</w:document>"#;
        let docx_bytes = create_test_zip("word/document.xml", docx_xml.as_bytes());
        let extracted_text = extract_text_from_docx_bytes(&docx_bytes, &cancel).expect("extract docx");
        assert!(extracted_text.contains("Tiêu đề hợp đồng"));
        assert!(extracted_text.contains("Điều 1: Phạm vi công việc."));

        // 2. XLSX test
        let shared_strings_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <si><t>Họ Tên</t></si>
  <si><t>Lương</t></si>
  <si><t>Nguyễn Văn A</t></si>
</sst>"#;
        let sheet_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="s"><v>0</v></c>
      <c r="B1" t="s"><v>1</v></c>
    </row>
    <row r="2">
      <c r="A2" t="s"><v>2</v></c>
      <c r="B2"><v>15000000</v></c>
    </row>
  </sheetData>
</worksheet>"#;
        let mut xlsx_zip = create_test_zip("xl/sharedStrings.xml", shared_strings_xml.as_bytes());
        let sheet_zip = create_test_zip("xl/worksheets/sheet1.xml", sheet_xml.as_bytes());
        xlsx_zip.extend_from_slice(&sheet_zip);

        let extracted_csv = extract_csv_from_xlsx_bytes(&xlsx_zip, &cancel).expect("extract xlsx");
        assert!(extracted_csv.contains("Họ Tên,Lương"));
        assert!(extracted_csv.contains("Nguyễn Văn A,15000000"));
    }

    #[test]
    fn test_matrix_markdown_to_pdf() {
        let tool = UniversalConverterTool::default();
        let tmp = std::env::temp_dir().join("test_univ_md_to_pdf");
        let _ = fs::create_dir_all(&tmp);

        let md_file = tmp.join("doc.md");
        fs::write(&md_file, "# Tiêu đề văn bản\n\nNội dung đoạn văn.").expect("write md");

        let req = DispatchTaskRequest {
            task_id: "task-matrix-md-pdf".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![md_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_extension": "pdf" }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("convert md to pdf");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);
        let pdf_bytes = fs::read(&res.output_paths[0]).expect("read pdf");
        assert!(pdf_bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn test_matrix_csv_to_json_and_sql() {
        let tool = UniversalConverterTool::default();
        let tmp = std::env::temp_dir().join("test_univ_csv_json");
        let _ = fs::create_dir_all(&tmp);

        let csv_file = tmp.join("data.csv");
        fs::write(&csv_file, "id,name,val\n1,Alpha,100\n2,Beta,200\n").expect("write csv");

        let req_json = DispatchTaskRequest {
            task_id: "task-matrix-csv-json".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![csv_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_extension": "json" }),
        };

        let cancel = AtomicBool::new(false);
        let res_json = tool.execute(&req_json, &cancel, None).expect("convert csv to json");
        assert_eq!(res_json.status, "SUCCESS");
        let json_text = fs::read_to_string(&res_json.output_paths[0]).expect("read json");
        assert!(json_text.contains("\"name\":\"Alpha\"") || json_text.contains("\"name\": \"Alpha\""));
    }

    #[test]
    fn test_matrix_docx_and_xlsx_dispatch() {
        let tool = UniversalConverterTool::default();
        let tmp = std::env::temp_dir().join("test_univ_docx_xlsx");
        let _ = fs::create_dir_all(&tmp);

        // DOCX -> MD
        let docx_xml = r#"<w:document><w:body><w:p><w:t>Kính gửi quý khách hàng</w:t></w:p></w:body></w:document>"#;
        let docx_file = tmp.join("letter.docx");
        fs::write(&docx_file, create_test_zip("word/document.xml", docx_xml.as_bytes())).expect("write docx");

        let req_docx = DispatchTaskRequest {
            task_id: "task-matrix-docx-md".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![docx_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_extension": "md" }),
        };

        let cancel = AtomicBool::new(false);
        let res_docx = tool.execute(&req_docx, &cancel, None).expect("convert docx to md");
        assert_eq!(res_docx.status, "SUCCESS");
        let md_text = fs::read_to_string(&res_docx.output_paths[0]).expect("read md");
        assert!(md_text.contains("Kính gửi quý khách hàng"));
    }

    #[test]
    fn test_matrix_unsupported_format_graceful_error() {
        let tool = UniversalConverterTool::default();
        let tmp = std::env::temp_dir().join("test_univ_unsupported");
        let _ = fs::create_dir_all(&tmp);

        let ole_file = tmp.join("sample.doc");
        // Magic byte OLE2: D0 CF 11 E0
        fs::write(&ole_file, b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1 binary data").expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-matrix-unsupp".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![ole_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_extension": "pdf" }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("chưa được hỗ trợ") || reason.contains("không được hỗ trợ"));
            }
            other => panic!("Kỳ vọng ValidationFailed, nhận được {:?}", other),
        }
    }

    #[test]
    fn test_matrix_pdf_to_xlsx_and_csv_dispatch() {
        let tool = UniversalConverterTool::default();
        let tmp = std::env::temp_dir().join("test_univ_pdf_table");
        let _ = fs::create_dir_all(&tmp);

        // Tạo PDF mẫu hợp lệ tối thiểu
        let sample_pdf = tmp.join("sample.pdf");
        let minimal_pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << >> >>\nendobj\n4 0 obj\n<< /Length 44 >>\nstream\nBT /F1 12 Tf 100 700 Td (Ma Hang | Don Gia) Tj ET\nendstream\nendobj\nxref\n0 5\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n0000000115 00000 n \n0000000214 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n308\n%%EOF\n";
        fs::write(&sample_pdf, minimal_pdf).expect("write sample pdf");

        // 1. PDF -> XLSX
        let req_xlsx = DispatchTaskRequest {
            task_id: "task-pdf-xlsx".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![sample_pdf.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_extension": "xlsx" }),
        };
        let cancel = AtomicBool::new(false);
        let res_xlsx = tool.execute(&req_xlsx, &cancel, None).expect("convert pdf to xlsx");
        assert_eq!(res_xlsx.status, "SUCCESS");
        assert_eq!(res_xlsx.output_paths.len(), 1);
        assert!(res_xlsx.output_paths[0].ends_with(".xlsx"));

        // 2. PDF -> CSV
        let req_csv = DispatchTaskRequest {
            task_id: "task-pdf-csv".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![sample_pdf.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_extension": "csv" }),
        };
        let res_csv = tool.execute(&req_csv, &cancel, None).expect("convert pdf to csv");
        assert_eq!(res_csv.status, "SUCCESS");
        assert_eq!(res_csv.output_paths.len(), 1);
        assert!(res_csv.output_paths[0].ends_with(".csv"));
    }
}
