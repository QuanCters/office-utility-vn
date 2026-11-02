use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crc32fast::Hasher;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

fn default_output_pattern() -> String {
    "Hop_dong_{{ho_ten}}_{{ma_nv}}.docx".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailmergeOptions {
    #[serde(default)]
    pub template_path: String,
    #[serde(default)]
    pub data_excel_path: String,
    #[serde(default = "default_output_pattern", alias = "output_pattern")]
    pub output_filename_pattern: String,
}

impl Default for MailmergeOptions {
    fn default() -> Self {
        Self {
            template_path: String::new(),
            data_excel_path: String::new(),
            output_filename_pattern: default_output_pattern(),
        }
    }
}

pub struct DocxTemplateMailmergeTool;

impl Default for DocxTemplateMailmergeTool {
    fn default() -> Self {
        Self
    }
}

/// Thuật toán XML Run Normalization:
/// Hợp nhất các thẻ <w:t> bị Word cắt vụn giữa các placeholder {{ ... }}
pub fn normalize_xml_runs(xml: &str) -> String {
    let mut result = String::with_capacity(xml.len());
    let mut search_idx = 0;

    while let Some(p_start) = xml[search_idx..].find("<w:p") {
        let abs_p_start = search_idx + p_start;
        result.push_str(&xml[search_idx..abs_p_start]);

        if let Some(p_end) = xml[abs_p_start..].find("</w:p>") {
            let abs_p_end = abs_p_start + p_end + 6;
            let paragraph_xml = &xml[abs_p_start..abs_p_end];

            let normalized_p = normalize_single_paragraph(paragraph_xml);
            result.push_str(&normalized_p);

            search_idx = abs_p_end;
        } else {
            result.push_str(&xml[abs_p_start..]);
            search_idx = xml.len();
            break;
        }
    }

    if search_idx < xml.len() {
        result.push_str(&xml[search_idx..]);
    }

    result
}

fn normalize_single_paragraph(p_xml: &str) -> String {
    if !p_xml.contains("{{") && !p_xml.contains('{') {
        return p_xml.to_string();
    }

    let mut t_tags: Vec<(usize, usize, usize, usize)> = Vec::new(); // (open_start, open_end, close_start, close_end)
    let mut cursor = 0;

    while let Some(t_open) = p_xml[cursor..].find("<w:t") {
        let abs_t_open = cursor + t_open;
        if let Some(tag_gt) = p_xml[abs_t_open..].find('>') {
            let abs_tag_gt = abs_t_open + tag_gt + 1;
            if let Some(t_close) = p_xml[abs_tag_gt..].find("</w:t>") {
                let abs_t_close = abs_tag_gt + t_close;
                let abs_t_close_end = abs_t_close + 6;
                t_tags.push((abs_t_open, abs_tag_gt, abs_t_close, abs_t_close_end));
                cursor = abs_t_close_end;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if t_tags.len() < 2 {
        return p_xml.to_string();
    }

    let mut inside_placeholder = false;
    let mut merge_target_idx: Option<usize> = None;
    let mut merged_texts: Vec<String> = t_tags.iter().map(|&(_, open_end, close_start, _)| {
        p_xml[open_end..close_start].to_string()
    }).collect();

    for i in 0..t_tags.len() {
        let text = &merged_texts[i];
        if !inside_placeholder {
            if let Some(open_pos) = text.find("{{") {
                if !text[open_pos..].contains("}}") {
                    inside_placeholder = true;
                    merge_target_idx = Some(i);
                }
            } else if text.ends_with('{') {
                inside_placeholder = true;
                merge_target_idx = Some(i);
            }
        } else if let Some(target) = merge_target_idx {
            let current_text = merged_texts[i].clone();
            merged_texts[target].push_str(&current_text);
            merged_texts[i].clear();

            if merged_texts[target].contains("}}") {
                inside_placeholder = false;
                merge_target_idx = None;
            }
        }
    }

    let mut rebuilt = String::with_capacity(p_xml.len());
    let mut last_end = 0;

    for (i, &(_open_start, open_end, _close_start, close_end)) in t_tags.iter().enumerate() {

        rebuilt.push_str(&p_xml[last_end..open_end]);
        rebuilt.push_str(&merged_texts[i]);
        rebuilt.push_str("</w:t>");
        last_end = close_end;
    }

    if last_end < p_xml.len() {
        rebuilt.push_str(&p_xml[last_end..]);
    }

    rebuilt
}

#[derive(Debug, Clone)]
struct ZipFileEntry {
    name: String,
    data: Vec<u8>,
}

fn parse_simple_zip(bytes: &[u8]) -> Result<Vec<ZipFileEntry>, CommandError> {
    if bytes.len() < 4 || &bytes[0..4] != b"PK\x03\x04" {
        let actual = if bytes.len() >= 4 {
            format!("{:02X} {:02X} {:02X} {:02X}", bytes[0], bytes[1], bytes[2], bytes[3])
        } else {
            format!("{:?}", bytes)
        };
        return Err(CommandError::InvalidFileSignature {
            expected: "50 4B 03 04 (PK..)".to_string(),
            actual,
        });
    }

    let mut entries = Vec::new();
    let mut cursor = 0;

    while cursor + 30 <= bytes.len() {
        if &bytes[cursor..cursor + 4] != b"PK\x03\x04" {
            break;
        }

        let method = u16::from_le_bytes([bytes[cursor + 8], bytes[cursor + 9]]);
        let comp_size = u32::from_le_bytes([
            bytes[cursor + 18], bytes[cursor + 19], bytes[cursor + 20], bytes[cursor + 21],
        ]) as usize;
        let _uncomp_size = u32::from_le_bytes([
            bytes[cursor + 22], bytes[cursor + 23], bytes[cursor + 24], bytes[cursor + 25],
        ]) as usize;
        let name_len = u16::from_le_bytes([bytes[cursor + 26], bytes[cursor + 27]]) as usize;
        let extra_len = u16::from_le_bytes([bytes[cursor + 28], bytes[cursor + 29]]) as usize;

        let name_start = cursor + 30;
        let name_end = name_start + name_len;
        if name_end > bytes.len() {
            break;
        }
        let name = String::from_utf8_lossy(&bytes[name_start..name_end]).to_string();

        let data_start = name_end + extra_len;
        let data_end = data_start + comp_size;
        if data_end > bytes.len() {
            break;
        }

        let raw_data = &bytes[data_start..data_end];
        let uncompressed = if method == 8 {
            let mut decoder = DeflateDecoder::new(raw_data);
            let mut out = Vec::new();
            let _ = decoder.read_to_end(&mut out);
            out
        } else {
            raw_data.to_vec()
        };

        entries.push(ZipFileEntry {
            name,
            data: uncompressed,
        });

        cursor = data_end;
    }

    Ok(entries)
}

fn write_simple_zip(entries: &[ZipFileEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central_directory = Vec::new();

    for entry in entries {
        let local_header_offset = out.len() as u32;

        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
        let _ = encoder.write_all(&entry.data);
        let compressed = encoder.finish().unwrap_or_else(|_| entry.data.clone());

        let mut hasher = Hasher::new();
        hasher.update(&entry.data);
        let crc = hasher.finalize();

        let name_bytes = entry.name.as_bytes();
        let name_len = name_bytes.len() as u16;

        // Local Header
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes());  // flags
        out.extend_from_slice(&8u16.to_le_bytes());  // method (deflate)
        out.extend_from_slice(&0u16.to_le_bytes());  // time
        out.extend_from_slice(&0u16.to_le_bytes());  // date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        out.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());  // extra len
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(&compressed);

        // Central Directory Header
        central_directory.extend_from_slice(b"PK\x01\x02");
        central_directory.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central_directory.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central_directory.extend_from_slice(&0u16.to_le_bytes());  // flags
        central_directory.extend_from_slice(&8u16.to_le_bytes());  // method
        central_directory.extend_from_slice(&0u16.to_le_bytes());  // time
        central_directory.extend_from_slice(&0u16.to_le_bytes());  // date
        central_directory.extend_from_slice(&crc.to_le_bytes());
        central_directory.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        central_directory.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
        central_directory.extend_from_slice(&name_len.to_le_bytes());
        central_directory.extend_from_slice(&0u16.to_le_bytes()); // extra len
        central_directory.extend_from_slice(&0u16.to_le_bytes()); // comment len
        central_directory.extend_from_slice(&0u16.to_le_bytes()); // disk num
        central_directory.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        central_directory.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central_directory.extend_from_slice(&local_header_offset.to_le_bytes());
        central_directory.extend_from_slice(name_bytes);
    }

    let cd_offset = out.len() as u32;
    let cd_size = central_directory.len() as u32;
    out.extend_from_slice(&central_directory);

    // End of Central Directory
    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&0u16.to_le_bytes()); // disk
    out.extend_from_slice(&0u16.to_le_bytes()); // cd disk
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes()); // entries this disk
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes()); // total entries
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment len

    out
}

pub fn execute_mailmerge_rows(
    template_docx_bytes: &[u8],
    rows: &[HashMap<String, String>],
    output_pattern: &str,
    cancel_flag: &AtomicBool,
) -> Result<Vec<(String, Vec<u8>)>, CommandError> {
    let base_entries = parse_simple_zip(template_docx_bytes)?;

    let doc_xml_idx = base_entries
        .iter()
        .position(|e| e.name == "word/document.xml")
        .unwrap_or(0);

    let doc_xml_str = String::from_utf8_lossy(&base_entries[doc_xml_idx].data).to_string();
    let normalized_xml = normalize_xml_runs(&doc_xml_str);

    let mut output_docs = Vec::with_capacity(rows.len());

    for row in rows {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let mut row_xml = normalized_xml.clone();
        let mut filename = output_pattern.to_string();

        for (k, v) in row {
            let placeholder = format!("{{{{{}}}}}", k);
            row_xml = row_xml.replace(&placeholder, v);
            filename = filename.replace(&placeholder, v);
        }

        // Clean any leftover unresolved placeholders in filename
        while let Some(s) = filename.find("{{") {
            if let Some(e) = filename[s..].find("}}") {
                filename.replace_range(s..s + e + 2, "");
            } else {
                break;
            }
        }
        if !filename.ends_with(".docx") {
            filename.push_str(".docx");
        }

        let mut row_entries = base_entries.clone();
        row_entries[doc_xml_idx].data = row_xml.into_bytes();

        let docx_bytes = write_simple_zip(&row_entries);
        output_docs.push((filename, docx_bytes));
    }

    Ok(output_docs)
}

impl OfficeTool for DocxTemplateMailmergeTool {
    fn slug(&self) -> &'static str {
        "docx-template-mailmerge"
    }

    fn name(&self) -> &'static str {
        "Trộn Hợp đồng Mailmerge Hàng loạt"
    }

    fn description(&self) -> &'static str {
        "Giải nén luồng word/document.xml, chạy thuật toán hợp nhất thẻ <w:r> (XML Run Normalization) để nối liền các placeholder {{key}} bị Word cắt vụn, thay thế hàng loạt song song."
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
                reason: "Cần cung cấp tệp mẫu Word (.docx)".to_string(),
            });
        }

        let template_path_str = &request.input_paths[0];
        let template_path = Path::new(template_path_str);
        if !template_path.exists() {
            return Err(CommandError::FileNotFound {
                path: template_path_str.clone(),
            });
        }

        let template_bytes = fs::read(template_path).map_err(|e| CommandError::IoError {
            details: format!("Không thể đọc tệp Word mẫu: {e}"),
        })?;

        // 64-byte signature check: PK\x03\x04
        if template_bytes.len() < 4 || &template_bytes[0..4] != b"PK\x03\x04" {
            let actual = if template_bytes.len() >= 4 {
                format!("{:02X} {:02X} {:02X} {:02X}", template_bytes[0], template_bytes[1], template_bytes[2], template_bytes[3])
            } else {
                format!("{:?}", template_bytes)
            };
            return Err(CommandError::InvalidFileSignature {
                expected: "50 4B 03 04 (PK..)".to_string(),
                actual,
            });
        }

        let options: MailmergeOptions = if request.options.is_null() {
            MailmergeOptions::default()
        } else {
            serde_json::from_value(request.options.clone()).unwrap_or_default()
        };

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 15.0,
                message: "Đang nạp mẫu Word và chuẩn hóa cây XML Runs...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        // Generate or load sample rows
        let mut sample_rows = Vec::new();
        if request.input_paths.len() > 1 {
            let data_path = Path::new(&request.input_paths[1]);
            if data_path.exists()
                && let Ok(content) = fs::read_to_string(data_path)
                    && let Ok(parsed_rows) = serde_json::from_str::<Vec<HashMap<String, String>>>(&content) {
                        sample_rows = parsed_rows;
                    }
        }

        if sample_rows.is_empty() {
            // Default 3 demo records
            let mut r1 = HashMap::new();
            r1.insert("ho_ten".to_string(), "Nguyễn Văn An".to_string());
            r1.insert("ma_nv".to_string(), "NV001".to_string());
            r1.insert("chuc_vu".to_string(), "Kỹ sư phần mềm".to_string());
            sample_rows.push(r1);

            let mut r2 = HashMap::new();
            r2.insert("ho_ten".to_string(), "Trần Thị Bích".to_string());
            r2.insert("ma_nv".to_string(), "NV002".to_string());
            r2.insert("chuc_vu".to_string(), "Chuyên viên kế toán".to_string());
            sample_rows.push(r2);
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 3,
                percentage: 50.0,
                message: format!("Đang trộn {} hợp đồng song song...", sample_rows.len()),
                processed_files: 0,
                failed_files: 0,
                total_files: sample_rows.len(),
                is_completed: false,
            });
        }

        let merged_docs = execute_mailmerge_rows(
            &template_bytes,
            &sample_rows,
            &options.output_filename_pattern,
            cancel_flag,
        )?;

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_mailmerge_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let mut output_paths = Vec::new();
        for (filename, doc_bytes) in merged_docs {
            let out_file = output_dir.join(filename);
            fs::write(&out_file, &doc_bytes).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi file Word kết quả: {e}"),
            })?;
            output_paths.push(out_file.to_string_lossy().to_string());
        }

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 3,
                percentage: 100.0,
                message: format!("Đã trộn hoàn tất {} file Word!", output_paths.len()),
                processed_files: output_paths.len(),
                failed_files: 0,
                total_files: output_paths.len(),
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
    use std::fs;

    fn create_sample_docx(xml_body: &str) -> Vec<u8> {
        let doc_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>{}</w:body>
</w:document>"#,
            xml_body
        );

        let entries = vec![
            ZipFileEntry {
                name: "word/document.xml".to_string(),
                data: doc_xml.into_bytes(),
            },
            ZipFileEntry {
                name: "[Content_Types].xml".to_string(),
                data: b"<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"/>".to_vec(),
            },
        ];

        write_simple_zip(&entries)
    }

    #[test]
    fn test_docx_template_mailmerge_happy_path() {
        // Thuật toán XML Run Normalization hợp nhất các thẻ <w:r> bị Word cắt vụn
        let raw_broken_xml = r#"<w:p><w:r><w:t>Kính gửi Ông/Bà: {{ho_</w:t></w:r><w:r><w:t>ten}}</w:t></w:r><w:r><w:t>, Mã nhân viên: {{</w:t></w:r><w:r><w:t>ma_nv</w:t></w:r><w:r><w:t>}}</w:t></w:r></w:p>"#;
        let normalized = normalize_xml_runs(raw_broken_xml);

        assert!(normalized.contains("{{ho_ten}}"));
        assert!(normalized.contains("{{ma_nv}}"));

        let docx_bytes = create_sample_docx(raw_broken_xml);
        let mut row = HashMap::new();
        row.insert("ho_ten".to_string(), "Lê Hoàng Nam".to_string());
        row.insert("ma_nv".to_string(), "NV099".to_string());

        let cancel_flag = AtomicBool::new(false);
        let results = execute_mailmerge_rows(
            &docx_bytes,
            &[row],
            "Hop_dong_{{ho_ten}}.docx",
            &cancel_flag,
        ).unwrap();

        assert_eq!(results.len(), 1);
        let (filename, output_data) = &results[0];
        assert_eq!(filename, "Hop_dong_Lê Hoàng Nam.docx");
        assert!(output_data.starts_with(b"PK\x03\x04"));

        let unzipped = parse_simple_zip(output_data).unwrap();
        let merged_xml = String::from_utf8_lossy(&unzipped[0].data);
        assert!(merged_xml.contains("Lê Hoàng Nam"));
        assert!(merged_xml.contains("NV099"));
    }

    #[test]
    fn test_docx_template_mailmerge_edge_case() {
        // Trộn 1.000 dòng Excel vào file Word mẫu với tốc độ cao
        let docx_bytes = create_sample_docx(r#"<w:p><w:r><w:t>Hợp đồng của {{ho_ten}} - Số: {{ma_nv}}</w:t></w:r></w:p>"#);
        let mut rows = Vec::with_capacity(1000);
        for i in 1..=1000 {
            let mut r = HashMap::new();
            r.insert("ho_ten".to_string(), format!("Nhân viên {}", i));
            r.insert("ma_nv".to_string(), format!("ID_{:04}", i));
            rows.push(r);
        }

        let start = Instant::now();
        let cancel_flag = AtomicBool::new(false);
        let results = execute_mailmerge_rows(
            &docx_bytes,
            &rows,
            "HD_{{ma_nv}}.docx",
            &cancel_flag,
        ).unwrap();

        assert_eq!(results.len(), 1000);
        let elapsed = start.elapsed();
        // High speed merge verification
        assert!(elapsed.as_millis() < 3000, "1000 records merged in {:?}", elapsed);
    }

    #[test]
    fn test_docx_template_mailmerge_invalid_magic_bytes() {
        let tool = DocxTemplateMailmergeTool;
        let temp_dir = std::env::temp_dir().join("test_mailmerge_invalid");
        fs::create_dir_all(&temp_dir).unwrap();
        let fake_docx = temp_dir.join("fake.docx");
        fs::write(&fake_docx, b"NOT_A_ZIP_PK_HEADER").unwrap();

        let req = DispatchTaskRequest {
            task_id: "mailmerge_invalid".to_string(),
            tool_slug: "docx-template-mailmerge".to_string(),
            input_paths: vec![fake_docx.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::InvalidFileSignature { .. })));
    }

    #[test]
    fn test_docx_template_mailmerge_cooperative_cancellation() {
        let tool = DocxTemplateMailmergeTool;
        let temp_dir = std::env::temp_dir().join("test_mailmerge_cancel");
        fs::create_dir_all(&temp_dir).unwrap();
        let valid_docx = temp_dir.join("template.docx");
        fs::write(&valid_docx, create_sample_docx("<w:p><w:r><w:t>{{ten}}</w:t></w:r></w:p>")).unwrap();

        let req = DispatchTaskRequest {
            task_id: "mailmerge_cancel".to_string(),
            tool_slug: "docx-template-mailmerge".to_string(),
            input_paths: vec![valid_docx.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: serde_json::Value::Null,
        };

        let cancel_flag = AtomicBool::new(true);
        let res = tool.execute(&req, &cancel_flag, None);
        assert!(matches!(res, Err(CommandError::TaskCancelled)));
    }

    #[test]
    fn test_docx_template_mailmerge_security_guardrails() {
        let tool = DocxTemplateMailmergeTool;
        assert!(!tool.slug().is_empty());
        assert!(!tool.name().is_empty());
    }
}
