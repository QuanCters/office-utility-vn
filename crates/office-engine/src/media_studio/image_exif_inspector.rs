use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Tùy chọn cấu hình kiểm tra & tước bỏ EXIF
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExifInspectorOptions {
    #[serde(default)]
    pub strip_all: bool,
    #[serde(default)]
    pub action: Option<String>,
}

/// Dữ liệu tổng hợp metadata EXIF
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ExifMetadataSummary {
    pub make: Option<String>,
    pub model: Option<String>,
    pub date_time: Option<String>,
    pub software: Option<String>,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
    pub gps_latitude: Option<f64>,
    pub gps_longitude: Option<f64>,
}

pub struct ImageExifInspectorTool;

impl Default for ImageExifInspectorTool {
    fn default() -> Self {
        Self
    }
}

// Helper đọc số theo Endian
fn read_u16(bytes: &[u8], offset: usize, is_le: bool) -> Option<u16> {
    if offset + 2 > bytes.len() {
        return None;
    }
    let b = [bytes[offset], bytes[offset + 1]];
    Some(if is_le {
        u16::from_le_bytes(b)
    } else {
        u16::from_be_bytes(b)
    })
}

fn read_u32(bytes: &[u8], offset: usize, is_le: bool) -> Option<u32> {
    if offset + 4 > bytes.len() {
        return None;
    }
    let b = [
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ];
    Some(if is_le {
        u32::from_le_bytes(b)
    } else {
        u32::from_be_bytes(b)
    })
}

fn read_rational(bytes: &[u8], offset: usize, is_le: bool) -> Option<f64> {
    let num = read_u32(bytes, offset, is_le)? as f64;
    let den = read_u32(bytes, offset + 4, is_le)? as f64;
    if den == 0.0 {
        None
    } else {
        Some(num / den)
    }
}

fn read_ascii(bytes: &[u8], offset: usize, len: usize) -> Option<String> {
    if offset + len > bytes.len() {
        return None;
    }
    let raw = &bytes[offset..offset + len];
    let trimmed = match raw.iter().position(|&b| b == 0) {
        Some(p) => &raw[..p],
        None => raw,
    };
    Some(String::from_utf8_lossy(trimmed).trim().to_string())
}

/// Trích xuất Exif Metadata từ mảng byte JPEG
pub fn parse_jpeg_exif(bytes: &[u8]) -> Result<ExifMetadataSummary, CommandError> {
    if bytes.len() < 4 || !bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(CommandError::InvalidFileSignature {
            expected: "JPEG (FF D8)".to_string(),
            actual: "Không phải header JPEG hợp lệ".to_string(),
        });
    }

    let mut summary = ExifMetadataSummary::default();
    let mut cursor = 2;

    while cursor + 4 <= bytes.len() {
        if bytes[cursor] != 0xFF {
            break;
        }

        let marker = bytes[cursor + 1];
        if marker == 0xD9 || marker == 0xDA {
            // EOI hoặc SOS (kết thúc các marker tiêu đề)
            break;
        }

        if marker == 0x00 || (0xD0..=0xD7).contains(&marker) {
            cursor += 2;
            continue;
        }

        let len = u16::from_be_bytes([bytes[cursor + 2], bytes[cursor + 3]]) as usize;
        let segment_start = cursor + 4;
        let segment_end = cursor + 2 + len;

        if segment_end > bytes.len() {
            break;
        }

        // Marker SOF0 (0xC0) hoặc SOF2 (0xC2) lấy kích thước ảnh
        if (marker == 0xC0 || marker == 0xC2)
            && segment_start + 7 <= bytes.len() {
                let h = u16::from_be_bytes([bytes[segment_start + 1], bytes[segment_start + 2]]) as u32;
                let w = u16::from_be_bytes([bytes[segment_start + 3], bytes[segment_start + 4]]) as u32;

                // NFR-SEC-05: Image Bomb Defense
                if w > 16384 || h > 16384 {
                    return Err(CommandError::ValidationFailed {
                        reason: format!("Kích thước ảnh ({w}x{h}) vượt quá giới hạn an toàn 16384x16384"),
                    });
                }

                summary.image_width = Some(w);
                summary.image_height = Some(h);
            }

        // Marker APP1 (0xE1): EXIF
        if marker == 0xE1 && segment_end > segment_start + 6 {
            let header = &bytes[segment_start..segment_start + 6];
            if header == b"Exif\0\0" {
                let tiff_base = segment_start + 6;
                let tiff_data = &bytes[tiff_base..segment_end];
                parse_tiff_data(tiff_data, &mut summary);
            }
        }

        cursor = segment_end;
    }

    Ok(summary)
}

fn parse_tiff_data(tiff: &[u8], summary: &mut ExifMetadataSummary) {
    if tiff.len() < 8 {
        return;
    }

    let is_le = match &tiff[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return,
    };

    let Some(magic) = read_u16(tiff, 2, is_le) else { return };
    if magic != 42 {
        return;
    }

    let Some(first_ifd_offset) = read_u32(tiff, 4, is_le) else { return };
    let ifd_offset = first_ifd_offset as usize;
    let mut gps_ifd_offset = None;

    // Duyệt IFD0
    if ifd_offset + 2 <= tiff.len() {
        let Some(num_entries) = read_u16(tiff, ifd_offset, is_le) else { return };
        let mut entry_cursor = ifd_offset + 2;

        for _ in 0..num_entries {
            if entry_cursor + 12 > tiff.len() {
                break;
            }

            let Some(tag) = read_u16(tiff, entry_cursor, is_le) else { break };
            let Some(count) = read_u32(tiff, entry_cursor + 4, is_le) else { break };
            let value_or_offset = match read_u32(tiff, entry_cursor + 8, is_le) {
                Some(v) => v as usize,
                None => break,
            };

            match tag {
                0x010F => {
                    // Make
                    summary.make = read_ascii(tiff, value_or_offset, count as usize);
                }
                0x0110 => {
                    // Model
                    summary.model = read_ascii(tiff, value_or_offset, count as usize);
                }
                0x0132 => {
                    // DateTime
                    summary.date_time = read_ascii(tiff, value_or_offset, count as usize);
                }
                0x0131 => {
                    // Software
                    summary.software = read_ascii(tiff, value_or_offset, count as usize);
                }
                0x8825 => {
                    // GPS IFD Pointer
                    gps_ifd_offset = Some(value_or_offset);
                }
                _ => {}
            }

            entry_cursor += 12;
        }
    }

    // Duyệt GPS IFD nếu có
    if let Some(gps_offset) = gps_ifd_offset
        && gps_offset + 2 <= tiff.len() {
            let Some(num_entries) = read_u16(tiff, gps_offset, is_le) else { return };
            let mut entry_cursor = gps_offset + 2;

            let mut lat_ref = 'N';
            let mut lon_ref = 'E';
            let mut lat_values = None;
            let mut lon_values = None;

            for _ in 0..num_entries {
                if entry_cursor + 12 > tiff.len() {
                    break;
                }

                let Some(tag) = read_u16(tiff, entry_cursor, is_le) else { break };
                let value_or_offset = match read_u32(tiff, entry_cursor + 8, is_le) {
                    Some(v) => v as usize,
                    None => break,
                };

                match tag {
                    0x0001 => {
                        // GPSLatitudeRef
                        if entry_cursor + 8 < tiff.len() {
                            lat_ref = tiff[entry_cursor + 8] as char;
                        }
                    }
                    0x0002 => {
                        // GPSLatitude (3 rationals)
                        if value_or_offset + 24 <= tiff.len() {
                            let deg = read_rational(tiff, value_or_offset, is_le).unwrap_or(0.0);
                            let min = read_rational(tiff, value_or_offset + 8, is_le).unwrap_or(0.0);
                            let sec = read_rational(tiff, value_or_offset + 16, is_le).unwrap_or(0.0);
                            lat_values = Some(deg + min / 60.0 + sec / 3600.0);
                        }
                    }
                    0x0003 => {
                        // GPSLongitudeRef
                        if entry_cursor + 8 < tiff.len() {
                            lon_ref = tiff[entry_cursor + 8] as char;
                        }
                    }
                    0x0004
                        // GPSLongitude (3 rationals)
                        if value_or_offset + 24 <= tiff.len() => {
                            let deg = read_rational(tiff, value_or_offset, is_le).unwrap_or(0.0);
                            let min = read_rational(tiff, value_or_offset + 8, is_le).unwrap_or(0.0);
                            let sec = read_rational(tiff, value_or_offset + 16, is_le).unwrap_or(0.0);
                            lon_values = Some(deg + min / 60.0 + sec / 3600.0);
                        }
                    _ => {}
                }

                entry_cursor += 12;
            }

            if let Some(lat) = lat_values {
                summary.gps_latitude = Some(if lat_ref == 'S' { -lat } else { lat });
            }
            if let Some(lon) = lon_values {
                summary.gps_longitude = Some(if lon_ref == 'W' { -lon } else { lon });
            }
        }
}

/// Tước bỏ không tổn hao (Lossless EXIF Stripping) cho tệp JPEG mà không giải nén lại ảnh
pub fn strip_jpeg_metadata(bytes: &[u8]) -> Result<Vec<u8>, CommandError> {
    if bytes.len() < 4 || !bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(CommandError::InvalidFileSignature {
            expected: "JPEG (FF D8)".to_string(),
            actual: "Không phải header JPEG hợp lệ".to_string(),
        });
    }

    let mut clean_bytes = Vec::with_capacity(bytes.len());
    clean_bytes.extend_from_slice(&[0xFF, 0xD8]); // SOI

    let mut cursor = 2;
    while cursor + 4 <= bytes.len() {
        if bytes[cursor] != 0xFF {
            clean_bytes.extend_from_slice(&bytes[cursor..]);
            break;
        }

        let marker = bytes[cursor + 1];
        if marker == 0xD9 {
            clean_bytes.extend_from_slice(&[0xFF, 0xD9]);
            break;
        }

        if marker == 0x00 || (0xD0..=0xD7).contains(&marker) {
            clean_bytes.push(bytes[cursor]);
            clean_bytes.push(bytes[cursor + 1]);
            cursor += 2;
            continue;
        }

        let len = u16::from_be_bytes([bytes[cursor + 2], bytes[cursor + 3]]) as usize;
        let segment_end = cursor + 2 + len;
        if segment_end > bytes.len() {
            clean_bytes.extend_from_slice(&bytes[cursor..]);
            break;
        }

        // Bỏ qua APP1 (0xE1: Exif/XMP), APP13 (0xED: Photoshop IPTC), COM (0xFE)
        let is_metadata = marker == 0xE1 || marker == 0xED || marker == 0xFE;
        if !is_metadata {
            clean_bytes.extend_from_slice(&bytes[cursor..segment_end]);
        }

        cursor = segment_end;

        if marker == 0xDA {
            // SOS: phần dữ liệu quét nén
            clean_bytes.extend_from_slice(&bytes[cursor..]);
            break;
        }
    }

    Ok(clean_bytes)
}

/// Tước bỏ không tổn hao cho tệp PNG (loại bỏ tEXt, zTXt, iTXt, eXIf)
pub fn strip_png_metadata(bytes: &[u8]) -> Result<Vec<u8>, CommandError> {
    if bytes.len() < 8 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(CommandError::InvalidFileSignature {
            expected: "PNG".to_string(),
            actual: "Không phải header PNG hợp lệ".to_string(),
        });
    }

    let mut clean_bytes = Vec::with_capacity(bytes.len());
    clean_bytes.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    let mut cursor = 8;
    while cursor + 8 <= bytes.len() {
        let chunk_len = u32::from_be_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        let chunk_type = &bytes[cursor + 4..cursor + 8];
        let total_chunk_len = 12 + chunk_len;

        if cursor + total_chunk_len > bytes.len() {
            clean_bytes.extend_from_slice(&bytes[cursor..]);
            break;
        }

        let is_meta = chunk_type == b"tEXt"
            || chunk_type == b"zTXt"
            || chunk_type == b"iTXt"
            || chunk_type == b"eXIf";

        if !is_meta {
            clean_bytes.extend_from_slice(&bytes[cursor..cursor + total_chunk_len]);
        }

        if chunk_type == b"IEND" {
            break;
        }

        cursor += total_chunk_len;
    }

    Ok(clean_bytes)
}

impl OfficeTool for ImageExifInspectorTool {
    fn slug(&self) -> &'static str {
        "image-exif-inspector"
    }

    fn name(&self) -> &'static str {
        "Kiểm tra & Xóa Exif/GPS Ảnh"
    }

    fn description(&self) -> &'static str {
        "Đọc tọa độ GPS, ngày giờ, thông số ống kính máy ảnh; sử dụng img-parts để tước bỏ hoàn toàn các phân đoạn APP1/EXIF/XMP mà không cần giải mã/nén lại dữ liệu điểm ảnh (Lossless Exif Stripping)."
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

        let mut options: ExifInspectorOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn ExifInspector không hợp lệ: {e}"),
            })?;
        if options.action.as_deref() == Some("strip_all") {
            options.strip_all = true;
        }

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp ảnh đầu vào trống".to_string(),
            });
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_exif_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let mut output_paths = Vec::new();
        let mut failed_items = Vec::new();
        let total_files = request.input_paths.len();

        for (idx, file_path_str) in request.input_paths.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let path = Path::new(file_path_str);
            if !path.exists() {
                if total_files == 1 {
                    return Err(CommandError::FileNotFound {
                        path: file_path_str.clone(),
                    });
                }
                failed_items.push(FileFailureDetail {
                    input_path: file_path_str.clone(),
                    reason: "Tệp không tồn tại".to_string(),
                });
                continue;
            }

            let bytes = match fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    if total_files == 1 {
                        return Err(CommandError::IoError {
                            details: format!("Lỗi đọc tệp: {e}"),
                        });
                    }
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("Lỗi đọc: {e}"),
                    });
                    continue;
                }
            };

            if bytes.len() < 4 {
                if total_files == 1 {
                    return Err(CommandError::InvalidFileSignature {
                        expected: "image/jpeg, image/png".to_string(),
                        actual: "Tệp quá nhỏ".to_string(),
                    });
                }
                failed_items.push(FileFailureDetail {
                    input_path: file_path_str.clone(),
                    reason: "Tệp quá nhỏ".to_string(),
                });
                continue;
            }

            let is_jpeg = bytes.starts_with(&[0xFF, 0xD8]);
            let is_png = bytes.starts_with(b"\x89PNG\r\n\x1a\n");

            if !is_jpeg && !is_png {
                if total_files == 1 {
                    return Err(CommandError::InvalidFileSignature {
                        expected: "image/jpeg, image/png".to_string(),
                        actual: format!("{:02X?}", &bytes[0..bytes.len().min(4)]),
                    });
                }
                failed_items.push(FileFailureDetail {
                    input_path: file_path_str.clone(),
                    reason: "Chữ ký file không hỗ trợ".to_string(),
                });
                continue;
            }

            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            // Kiểm tra metadata & Image bomb
            if is_jpeg {
                let _meta = match parse_jpeg_exif(&bytes) {
                    Ok(m) => m,
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
            }

            // Xử lý tước bỏ EXIF nếu strip_all được bật
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
            let ext = if is_jpeg { "jpg" } else { "png" };

            let out_bytes = if options.strip_all {
                if is_jpeg {
                    match strip_jpeg_metadata(&bytes) {
                        Ok(b) => b,
                        Err(e) => {
                            if total_files == 1 {
                                return Err(e);
                            }
                            failed_items.push(FileFailureDetail {
                                input_path: file_path_str.clone(),
                                reason: format!("{:?}", e),
                            });
                            continue;
                        }
                    }
                } else {
                    match strip_png_metadata(&bytes) {
                        Ok(b) => b,
                        Err(e) => {
                            if total_files == 1 {
                                return Err(e);
                            }
                            failed_items.push(FileFailureDetail {
                                input_path: file_path_str.clone(),
                                reason: format!("{:?}", e),
                            });
                            continue;
                        }
                    }
                }
            } else {
                bytes
            };

            let out_name = format!("{stem}_stripped.{ext}");
            let out_path = output_dir.join(out_name);

            if let Err(e) = fs::write(&out_path, out_bytes) {
                if total_files == 1 {
                    return Err(CommandError::IoError {
                        details: format!("Lỗi ghi file đầu ra: {e}"),
                    });
                }
                failed_items.push(FileFailureDetail {
                    input_path: file_path_str.clone(),
                    reason: format!("Lỗi ghi: {e}"),
                });
                continue;
            }

            output_paths.push(out_path.to_string_lossy().to_string());

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: (idx + 1) as u32,
                    total_steps: total_files as u32,
                    percentage: ((idx + 1) as f32 / total_files as f32) * 100.0,
                    message: format!("Đã xử lý EXIF {}/{}", idx + 1, total_files),
                    processed_files: idx + 1,
                    failed_files: failed_items.len(),
                    total_files,
                    is_completed: idx + 1 == total_files,
                });
            }
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
    use serde_json::json;

    /// Tạo một tệp JPEG giả lập chứa thông tin EXIF đầy đủ (Make, Model, DateTime, GPS)
    fn create_test_jpeg_with_exif() -> Vec<u8> {
        let mut jpeg = Vec::new();
        jpeg.extend_from_slice(&[0xFF, 0xD8]); // SOI

        // APP1 Exif segment
        let mut tiff = Vec::new();
        tiff.extend_from_slice(b"II"); // Little Endian
        tiff.extend_from_slice(&42u16.to_le_bytes()); // Magic
        tiff.extend_from_slice(&8u32.to_le_bytes()); // Offset to IFD0 (8)

        // IFD0: 4 entries (Make, Model, DateTime, GPS IFD pointer)
        let num_entries = 4u16;
        tiff.extend_from_slice(&num_entries.to_le_bytes());

        // Offset dữ liệu chuỗi đặt sau IFD0 và GPS IFD
        // IFD0 size = 2 + 4 * 12 + 4 (next ifd) = 54 bytes -> offset 62
        let make_str = b"Sony\0";
        let model_str = b"Alpha 7 IV\0";
        let date_str = b"2026:11:02 10:00:00\0";

        let make_offset = 120u32;
        let model_offset = make_offset + make_str.len() as u32;
        let date_offset = model_offset + model_str.len() as u32;
        let gps_ifd_offset = 64u32;

        // Entry 1: Make (0x010F, ASCII=2)
        tiff.extend_from_slice(&0x010Fu16.to_le_bytes());
        tiff.extend_from_slice(&2u16.to_le_bytes());
        tiff.extend_from_slice(&(make_str.len() as u32).to_le_bytes());
        tiff.extend_from_slice(&make_offset.to_le_bytes());

        // Entry 2: Model (0x0110, ASCII=2)
        tiff.extend_from_slice(&0x0110u16.to_le_bytes());
        tiff.extend_from_slice(&2u16.to_le_bytes());
        tiff.extend_from_slice(&(model_str.len() as u32).to_le_bytes());
        tiff.extend_from_slice(&model_offset.to_le_bytes());

        // Entry 3: DateTime (0x0132, ASCII=2)
        tiff.extend_from_slice(&0x0132u16.to_le_bytes());
        tiff.extend_from_slice(&2u16.to_le_bytes());
        tiff.extend_from_slice(&(date_str.len() as u32).to_le_bytes());
        tiff.extend_from_slice(&date_offset.to_le_bytes());

        // Entry 4: GPS IFD Pointer (0x8825, LONG=4)
        tiff.extend_from_slice(&0x8825u16.to_le_bytes());
        tiff.extend_from_slice(&4u16.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&gps_ifd_offset.to_le_bytes());

        tiff.extend_from_slice(&0u32.to_le_bytes()); // Next IFD (0)
        tiff.extend_from_slice(&[0, 0]); // Canh lề 2 bytes để GPS IFD đúng offset 64

        // GPS IFD (offset 64): 4 entries (LatRef, Lat, LonRef, Lon)
        let gps_entries = 4u16;
        tiff.extend_from_slice(&gps_entries.to_le_bytes());

        let lat_vals_offset = date_offset + date_str.len() as u32;
        let lon_vals_offset = lat_vals_offset + 24;

        // GPS LatRef (0x0001, ASCII=2, count 2)
        tiff.extend_from_slice(&0x0001u16.to_le_bytes());
        tiff.extend_from_slice(&2u16.to_le_bytes());
        tiff.extend_from_slice(&2u32.to_le_bytes());
        tiff.extend_from_slice(&[b'N', 0, 0, 0]);

        // GPS Lat (0x0002, RATIONAL=5, count 3)
        tiff.extend_from_slice(&0x0002u16.to_le_bytes());
        tiff.extend_from_slice(&5u16.to_le_bytes());
        tiff.extend_from_slice(&3u32.to_le_bytes());
        tiff.extend_from_slice(&lat_vals_offset.to_le_bytes());

        // GPS LonRef (0x0003, ASCII=2, count 2)
        tiff.extend_from_slice(&0x0003u16.to_le_bytes());
        tiff.extend_from_slice(&2u16.to_le_bytes());
        tiff.extend_from_slice(&2u32.to_le_bytes());
        tiff.extend_from_slice(&[b'E', 0, 0, 0]);

        // GPS Lon (0x0004, RATIONAL=5, count 3)
        tiff.extend_from_slice(&0x0004u16.to_le_bytes());
        tiff.extend_from_slice(&5u16.to_le_bytes());
        tiff.extend_from_slice(&3u32.to_le_bytes());
        tiff.extend_from_slice(&lon_vals_offset.to_le_bytes());

        tiff.extend_from_slice(&0u32.to_le_bytes()); // Next IFD (0)

        // Điền dữ liệu chuỗi vào đúng vị trí
        while tiff.len() < make_offset as usize {
            tiff.push(0);
        }
        tiff.extend_from_slice(make_str);
        tiff.extend_from_slice(model_str);
        tiff.extend_from_slice(date_str);

        // Lat: 10 độ, 45 phút, 0 giây (TP.HCM xấp xỉ 10.75 N)
        // 10/1, 45/1, 0/1
        tiff.extend_from_slice(&10u32.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&45u32.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&0u32.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());

        // Lon: 106 độ, 40 phút, 0 giây (TP.HCM xấp xỉ 106.666 E)
        // 106/1, 40/1, 0/1
        tiff.extend_from_slice(&106u32.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&40u32.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&0u32.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());

        // Ghép APP1 vào JPEG
        let app1_len = 2 + 6 + tiff.len();
        jpeg.extend_from_slice(&[0xFF, 0xE1]);
        jpeg.extend_from_slice(&(app1_len as u16).to_be_bytes());
        jpeg.extend_from_slice(b"Exif\0\0");
        jpeg.extend_from_slice(&tiff);

        // SOF0 (kích thước 1920 x 1080)
        jpeg.extend_from_slice(&[0xFF, 0xC0]);
        jpeg.extend_from_slice(&17u16.to_be_bytes());
        jpeg.push(8); // precision
        jpeg.extend_from_slice(&1080u16.to_be_bytes()); // height
        jpeg.extend_from_slice(&1920u16.to_be_bytes()); // width
        jpeg.push(3); // components
        jpeg.extend_from_slice(&[1, 0x11, 0, 2, 0x11, 1, 3, 0x11, 1]);

        // SOS & Scan Data
        jpeg.extend_from_slice(&[0xFF, 0xDA]);
        jpeg.extend_from_slice(&8u16.to_be_bytes());
        jpeg.extend_from_slice(&[1, 1, 0, 0, 63, 0]);
        jpeg.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]); // Dữ liệu nén giả lập
        jpeg.extend_from_slice(&[0xFF, 0xD9]); // EOI

        jpeg
    }

    #[test]
    fn test_image_exif_inspector_happy_path() {
        let tool = ImageExifInspectorTool;
        let tmp = std::env::temp_dir().join("test_exif_happy");
        let _ = fs::create_dir_all(&tmp);

        let img_file = tmp.join("camera_photo.jpg");
        fs::write(&img_file, create_test_jpeg_with_exif()).expect("write test jpeg");

        let meta = parse_jpeg_exif(&fs::read(&img_file).expect("read")).expect("parse exif");
        assert_eq!(meta.make.as_deref(), Some("Sony"));
        assert_eq!(meta.model.as_deref(), Some("Alpha 7 IV"));
        assert_eq!(meta.date_time.as_deref(), Some("2026:11:02 10:00:00"));
        assert_eq!(meta.image_width, Some(1920));
        assert_eq!(meta.image_height, Some(1080));

        let lat = meta.gps_latitude.expect("latitude");
        assert!((lat - 10.75).abs() < 0.001);

        let lon = meta.gps_longitude.expect("longitude");
        assert!((lon - 106.666).abs() < 0.01);

        let req = DispatchTaskRequest {
            task_id: "task-exif-read".to_string(),
            tool_slug: "image-exif-inspector".to_string(),
            input_paths: vec![img_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "strip_all": false }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("execute");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);
    }

    #[test]
    fn test_image_exif_inspector_edge_case() {
        let tool = ImageExifInspectorTool;
        let tmp = std::env::temp_dir().join("test_exif_strip_edge");
        let _ = fs::create_dir_all(&tmp);

        let img_file = tmp.join("raw_with_gps.jpg");
        let raw_bytes = create_test_jpeg_with_exif();
        fs::write(&img_file, &raw_bytes).expect("write");

        // Tước bỏ toàn bộ EXIF (Lossless Stripping)
        let req = DispatchTaskRequest {
            task_id: "task-exif-strip".to_string(),
            tool_slug: "image-exif-inspector".to_string(),
            input_paths: vec![img_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "strip_all": true }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("strip should succeed");
        assert_eq!(res.status, "SUCCESS");

        let stripped_bytes = fs::read(&res.output_paths[0]).expect("read stripped");
        assert!(stripped_bytes.starts_with(&[0xFF, 0xD8]));
        assert!(stripped_bytes.ends_with(&[0xFF, 0xD9]));

        // Xác nhận không còn marker APP1 (0xFF, 0xE1)
        let has_app1 = stripped_bytes.windows(2).any(|w| w == [0xFF, 0xE1]);
        assert!(!has_app1, "Tệp sạch không được chứa phân đoạn APP1");

        // Kích thước tệp đã giảm do bóc tách EXIF
        assert!(stripped_bytes.len() < raw_bytes.len());
    }

    #[test]
    fn test_image_exif_inspector_invalid_magic_bytes() {
        let tool = ImageExifInspectorTool;
        let tmp = std::env::temp_dir().join("test_exif_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("not_an_image.bin");
        fs::write(&bad_file, b"NON_IMAGE_CORRUPTED_BYTES").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-bad-exif".to_string(),
            tool_slug: "image-exif-inspector".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "strip_all": false }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_image_exif_inspector_cooperative_cancellation() {
        let tool = ImageExifInspectorTool;
        let tmp = std::env::temp_dir().join("test_exif_cancel");
        let _ = fs::create_dir_all(&tmp);

        let file = tmp.join("photo_cancel.jpg");
        fs::write(&file, create_test_jpeg_with_exif()).expect("write");

        let req = DispatchTaskRequest {
            task_id: "task-cancel-exif".to_string(),
            tool_slug: "image-exif-inspector".to_string(),
            input_paths: vec![file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "strip_all": true }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_image_exif_inspector_security_guardrails() {
        let tool = ImageExifInspectorTool;
        assert_eq!(tool.slug(), "image-exif-inspector");
        assert_eq!(tool.name(), "Kiểm tra & Xóa Exif/GPS Ảnh");

        // Image bomb defense (>16384 px)
        let mut bomb_jpeg = Vec::new();
        bomb_jpeg.extend_from_slice(&[0xFF, 0xD8]);
        bomb_jpeg.extend_from_slice(&[0xFF, 0xC0]);
        bomb_jpeg.extend_from_slice(&17u16.to_be_bytes());
        bomb_jpeg.push(8);
        bomb_jpeg.extend_from_slice(&20000u16.to_be_bytes()); // height > 16384
        bomb_jpeg.extend_from_slice(&20000u16.to_be_bytes()); // width > 16384
        bomb_jpeg.push(3);
        bomb_jpeg.extend_from_slice(&[1, 0x11, 0, 2, 0x11, 1, 3, 0x11, 1]);
        bomb_jpeg.extend_from_slice(&[0xFF, 0xD9]);

        let err = parse_jpeg_exif(&bomb_jpeg).unwrap_err();
        match err {
            CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("16384x16384"));
            }
            other => panic!("Kỳ vọng ValidationFailed nhưng nhận được: {:?}", other),
        }
    }
}
