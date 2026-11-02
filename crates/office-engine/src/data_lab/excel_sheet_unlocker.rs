use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Cấu hình tùy chọn gỡ khóa bảo vệ sheet Excel
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExcelUnlockOptions {}

pub struct ExcelSheetUnlockerTool;

impl Default for ExcelSheetUnlockerTool {
    fn default() -> Self {
        Self
    }
}

/// Loại bỏ các thẻ bảo vệ sheetProtection và workbookProtection khỏi chuỗi XML
pub fn strip_protection_tags(xml_content: &str) -> String {
    // Loại bỏ thẻ tự đóng: <sheetProtection ... /> hoặc <workbookProtection ... />
    let re_self_closing = match Regex::new(r"<(sheetProtection|workbookProtection)[^>]*?/>") {
        Ok(r) => r,
        Err(_) => return xml_content.to_string(),
    };
    let step1 = re_self_closing.replace_all(xml_content, "");

    // Loại bỏ thẻ mở và đóng: <sheetProtection ...>...</sheetProtection>
    let re_paired = match Regex::new(r"<(sheetProtection|workbookProtection)[^>]*?>.*?</\1>") {
        Ok(r) => r,
        Err(_) => return step1.into_owned(),
    };
    re_paired.replace_all(&step1, "").into_owned()
}

/// Cấu trúc đại diện một tệp tin bên trong ZIP archive
#[derive(Debug, Clone)]
pub struct ZipEntry {
    pub name: String,
    pub compression_method: u16,
    pub mod_time: u16,
    pub mod_date: u16,
    pub data: Vec<u8>,
}

/// Bộ phân tích cú pháp ZIP đơn giản, an toàn bộ nhớ
pub fn parse_zip_archive(raw: &[u8], cancel_flag: &AtomicBool) -> Result<Vec<ZipEntry>, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let mut entries = Vec::new();
    let mut offset = 0;

    while offset + 30 <= raw.len() {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        // Kiểm tra magic header: 0x04034b50 (PK\x03\x04)
        if &raw[offset..offset + 4] != b"PK\x03\x04" {
            break;
        }

        let compression = u16::from_le_bytes([raw[offset + 8], raw[offset + 9]]);
        let mod_time = u16::from_le_bytes([raw[offset + 10], raw[offset + 11]]);
        let mod_date = u16::from_le_bytes([raw[offset + 12], raw[offset + 13]]);
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
            // Deflate decompression
            let mut decoder = DeflateDecoder::new(comp_data);
            let mut decomp = Vec::new();
            if decoder.read_to_end(&mut decomp).is_err() {
                comp_data.to_vec()
            } else {
                decomp
            }
        } else {
            // Uncompressed (Stored)
            comp_data.to_vec()
        };

        entries.push(ZipEntry {
            name,
            compression_method: compression,
            mod_time,
            mod_date,
            data: decompressed_data,
        });

        offset = data_end;
    }

    Ok(entries)
}

/// Đóng gói lại các entry thành tệp ZIP hoàn chỉnh với Central Directory
pub fn build_zip_archive(entries: &[ZipEntry], cancel_flag: &AtomicBool) -> Result<Vec<u8>, CommandError> {
    let mut output = Vec::new();
    let mut central_directory = Vec::new();

    for entry in entries {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let local_header_offset = output.len() as u32;

        let mut crc_hasher = crc32fast::Hasher::new();
        crc_hasher.update(&entry.data);
        let crc = crc_hasher.finalize();

        let uncompressed_size = entry.data.len() as u32;

        let (comp_data, comp_method) = if entry.compression_method == 8 {
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
            if encoder.write_all(&entry.data).is_ok() {
                match encoder.finish() {
                    Ok(compressed) => (compressed, 8u16),
                    Err(_) => (entry.data.clone(), 0u16),
                }
            } else {
                (entry.data.clone(), 0u16)
            }
        } else {
            (entry.data.clone(), 0u16)
        };

        let compressed_size = comp_data.len() as u32;
        let name_bytes = entry.name.as_bytes();

        // 1. Ghi Local File Header (30 bytes)
        output.extend_from_slice(b"PK\x03\x04");
        output.extend_from_slice(&20u16.to_le_bytes()); // Version needed (2.0)
        output.extend_from_slice(&0u16.to_le_bytes());  // General purpose bit flag
        output.extend_from_slice(&comp_method.to_le_bytes());
        output.extend_from_slice(&entry.mod_time.to_le_bytes());
        output.extend_from_slice(&entry.mod_date.to_le_bytes());
        output.extend_from_slice(&crc.to_le_bytes());
        output.extend_from_slice(&compressed_size.to_le_bytes());
        output.extend_from_slice(&uncompressed_size.to_le_bytes());
        output.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        output.extend_from_slice(&0u16.to_le_bytes()); // Extra field length
        output.extend_from_slice(name_bytes);
        output.extend_from_slice(&comp_data);

        // 2. Ghi Central Directory Header (46 bytes + name)
        central_directory.extend_from_slice(b"PK\x01\x02");
        central_directory.extend_from_slice(&20u16.to_le_bytes()); // Version made by
        central_directory.extend_from_slice(&20u16.to_le_bytes()); // Version needed
        central_directory.extend_from_slice(&0u16.to_le_bytes());  // Flag
        central_directory.extend_from_slice(&comp_method.to_le_bytes());
        central_directory.extend_from_slice(&entry.mod_time.to_le_bytes());
        central_directory.extend_from_slice(&entry.mod_date.to_le_bytes());
        central_directory.extend_from_slice(&crc.to_le_bytes());
        central_directory.extend_from_slice(&compressed_size.to_le_bytes());
        central_directory.extend_from_slice(&uncompressed_size.to_le_bytes());
        central_directory.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        central_directory.extend_from_slice(&0u16.to_le_bytes()); // Extra field length
        central_directory.extend_from_slice(&0u16.to_le_bytes()); // File comment length
        central_directory.extend_from_slice(&0u16.to_le_bytes()); // Disk number start
        central_directory.extend_from_slice(&0u16.to_le_bytes()); // Internal file attributes
        central_directory.extend_from_slice(&0u32.to_le_bytes()); // External file attributes
        central_directory.extend_from_slice(&local_header_offset.to_le_bytes());
        central_directory.extend_from_slice(name_bytes);
    }

    let cd_offset = output.len() as u32;
    let cd_size = central_directory.len() as u32;
    let entry_count = entries.len() as u16;

    output.extend_from_slice(&central_directory);

    // 3. Ghi End of Central Directory Record (22 bytes)
    output.extend_from_slice(b"PK\x05\x06");
    output.extend_from_slice(&0u16.to_le_bytes()); // Disk number
    output.extend_from_slice(&0u16.to_le_bytes()); // Start disk
    output.extend_from_slice(&entry_count.to_le_bytes());
    output.extend_from_slice(&entry_count.to_le_bytes());
    output.extend_from_slice(&cd_size.to_le_bytes());
    output.extend_from_slice(&cd_offset.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes()); // Comment length

    Ok(output)
}

/// Gỡ khóa toàn bộ tệp Excel .xlsx (OpenXML container)
pub fn unlock_excel_bytes(raw_bytes: &[u8], cancel_flag: &AtomicBool) -> Result<Vec<u8>, CommandError> {
    if raw_bytes.starts_with(b"\x7fELF") || raw_bytes.starts_with(b"MZ") {
        return Err(CommandError::InvalidFileSignature {
            expected: "Tệp Excel XLSX (PK..)".to_string(),
            actual: "Tệp nhị phân thực thi bị chặn".to_string(),
        });
    }

    // Nếu là tệp XML thuần (ví dụ kiểm thử hoặc worksheet đơn lẻ)
    if raw_bytes.starts_with(b"<?xml") || raw_bytes.starts_with(b"<worksheet") {
        let xml_str = String::from_utf8_lossy(raw_bytes);
        let stripped = strip_protection_tags(&xml_str);
        return Ok(stripped.into_bytes());
    }

    if !raw_bytes.starts_with(b"PK\x03\x04") {
        return Err(CommandError::InvalidFileSignature {
            expected: "Tệp Excel OpenXML (PK..)".to_string(),
            actual: "Chữ ký tệp không khớp OpenXML".to_string(),
        });
    }

    let mut entries = parse_zip_archive(raw_bytes, cancel_flag)?;
    let mut modified = false;

    for entry in &mut entries {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        // Kiểm tra các tệp worksheet và workbook
        if entry.name.contains("worksheets/sheet") || entry.name.ends_with("workbook.xml") {
            let content_str = String::from_utf8_lossy(&entry.data);
            if content_str.contains("sheetProtection") || content_str.contains("workbookProtection") {
                let stripped = strip_protection_tags(&content_str);
                entry.data = stripped.into_bytes();
                modified = true;
            }
        }
    }

    if modified {
        build_zip_archive(&entries, cancel_flag)
    } else {
        Ok(raw_bytes.to_vec())
    }
}

impl OfficeTool for ExcelSheetUnlockerTool {
    fn slug(&self) -> &'static str {
        "excel-sheet-unlocker"
    }

    fn name(&self) -> &'static str {
        "Gỡ Khóa Bảo vệ Chỉnh sửa Sheet Excel"
    }

    fn description(&self) -> &'static str {
        "Đọc luồng nén OpenXML, loại bỏ thẻ sheetProtection và workbookProtection để mở khóa chỉnh sửa trang tính mà không cần mật khẩu."
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

        let input_bytes = if let Some(first_path) = request.input_paths.first() {
            let p = Path::new(first_path);
            if !p.exists() {
                return Err(CommandError::FileNotFound {
                    path: first_path.clone(),
                });
            }
            fs::read(p).map_err(|e| CommandError::IoError {
                details: e.to_string(),
            })?
        } else if let Some(xml_text) = request.options.get("xml").and_then(|v| v.as_str()) {
            xml_text.as_bytes().to_vec()
        } else {
            return Err(CommandError::ValidationFailed {
                reason: "Không có tệp đầu vào hoặc dữ liệu XML bảng tính".to_string(),
            });
        };

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 50.0,
                message: "Đang phân tích OpenXML và gỡ khóa trang tính...".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        let unlocked_bytes = unlock_excel_bytes(&input_bytes, cancel_flag)?;

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("excel_unlock_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        let out_file = out_dir.join("unlocked_workbook.xlsx");
        fs::write(&out_file, &unlocked_bytes).map_err(|e| CommandError::IoError {
            details: e.to_string(),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Mở khóa trang tính thành công 100%".to_string(),
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
            failed_items: vec![],
            elapsed_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_excel_sheet_unlocker_happy_path() {
        let tool = ExcelSheetUnlockerTool::default();
        assert_eq!(tool.slug(), "excel-sheet-unlocker");

        // Giả lập luồng XML OpenXML có chứa thẻ sheetProtection
        let mock_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
    <sheetViews><sheetView tabSelected="1" workbookViewId="0"/></sheetViews>
    <sheetProtection algorithmName="SHA-512" hashValue="abc123xyz==" saltValue="randomSalt==" spinCount="100000" sheet="1" objects="1" scenarios="1"/>
    <sheetData><row r="1"><c r="A1" t="s"><v>100</v></c></row></sheetData>
</worksheet>"#;

        let stripped = strip_protection_tags(mock_xml);

        // Khẳng định thẻ sheetProtection đã bị gỡ hoàn toàn
        assert!(!stripped.contains("<sheetProtection"));
        assert!(!stripped.contains("SHA-512"));
        // Các dữ liệu khác trong sheet vẫn được bảo toàn nguyên vẹn
        assert!(stripped.contains("<sheetData>"));
        assert!(stripped.contains("<v>100</v>"));
    }

    #[test]
    fn test_excel_sheet_unlocker_edge_case() {
        // Kiểm thử đóng gói tệp ZIP hoàn chỉnh với sheetProtection bên trong
        let cancel = AtomicBool::new(false);
        let sheet_content = b"<worksheet><sheetProtection password=\"CCCD\"/></worksheet>";

        let entry = ZipEntry {
            name: "xl/worksheets/sheet1.xml".to_string(),
            compression_method: 8,
            mod_time: 0,
            mod_date: 0,
            data: sheet_content.to_vec(),
        };

        let zip_bytes = build_zip_archive(&[entry], &cancel).expect("build zip");
        assert!(zip_bytes.starts_with(b"PK\x03\x04"));

        // Chạy gỡ khóa trên tệp ZIP này
        let unlocked = unlock_excel_bytes(&zip_bytes, &cancel).expect("unlock zip ok");
        assert!(unlocked.starts_with(b"PK\x03\x04"));

        // Giải nén kiểm tra lại
        let unzipped = parse_zip_archive(&unlocked, &cancel).expect("unzip ok");
        assert_eq!(unzipped.len(), 1);
        let unzipped_text = String::from_utf8_lossy(&unzipped[0].data);
        assert!(!unzipped_text.contains("sheetProtection"));
        assert!(unzipped_text.contains("<worksheet></worksheet>"));
    }

    #[test]
    fn test_excel_sheet_unlocker_invalid_magic_bytes() {
        let tool = ExcelSheetUnlockerTool::default();
        let tmp = std::env::temp_dir().join("test_unlock_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("bad_exec.bin");
        fs::write(&bad_file, b"\x7fELF\x02\x01\x01\x00").expect("write elf");

        let req = DispatchTaskRequest {
            task_id: "task-fake-unlock".to_string(),
            tool_slug: "excel-sheet-unlocker".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_excel_sheet_unlocker_cooperative_cancellation() {
        let tool = ExcelSheetUnlockerTool::default();
        let req = DispatchTaskRequest {
            task_id: "task-cancel-unlock".to_string(),
            tool_slug: "excel-sheet-unlocker".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "xml": "<worksheet><sheetProtection/></worksheet>"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_excel_sheet_unlocker_security_guardrails() {
        // Không có thẻ protection -> giữ nguyên không làm hỏng file
        let clean_xml = "<worksheet><sheetData><row/></sheetData></worksheet>";
        let stripped = strip_protection_tags(clean_xml);
        assert_eq!(stripped, clean_xml);

        // Kiểm thử huỷ tác vụ trong quá trình phân tích ZIP
        let cancel_true = AtomicBool::new(true);
        let res = parse_zip_archive(b"PK\x03\x041234567890", &cancel_true);
        assert_eq!(res.unwrap_err(), CommandError::TaskCancelled);
    }
}
