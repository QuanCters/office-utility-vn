use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Vị trí dán chữ ký trên trang tài liệu PDF
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SignaturePlacement {
    #[serde(alias = "BottomRight")]
    BottomRight,
    #[serde(alias = "BottomLeft")]
    BottomLeft,
    #[serde(alias = "Center")]
    Center,
    #[serde(alias = "TopRight")]
    TopRight,
    #[serde(alias = "Custom")]
    Custom { x_pt: f64, y_pt: f64 },
}

impl Default for SignaturePlacement {
    fn default() -> Self {
        Self::BottomRight
    }
}

fn default_target_page() -> i32 {
    -1
}

fn default_width_pt() -> f64 {
    160.0
}

fn default_height_pt() -> f64 {
    60.0
}

/// Tùy chọn cấu hình cho công cụ Ký tay Điện tử & Dán Chữ ký PDF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfSignaturePadOptions {
    /// Chuỗi ảnh chữ ký định dạng PNG đã mã hóa base64 (chấp nhận data URL)
    #[serde(default, alias = "signature_base64", alias = "signature_data_url")]
    pub signature_png_base64: String,

    /// Vị trí dán chữ ký trên trang
    #[serde(default)]
    pub placement: SignaturePlacement,

    /// Trang áp dụng: -1 là trang cuối, 0 là tất cả các trang, >=1 là số thứ tự trang cụ thể
    #[serde(default = "default_target_page")]
    pub target_page: i32,

    /// Chiều rộng khung chữ ký (point)
    #[serde(default = "default_width_pt")]
    pub width_pt: f64,

    /// Chiều cao khung chữ ký (point)
    #[serde(default = "default_height_pt")]
    pub height_pt: f64,
}

impl Default for PdfSignaturePadOptions {
    fn default() -> Self {
        Self {
            signature_png_base64: String::new(),
            placement: SignaturePlacement::BottomRight,
            target_page: -1,
            width_pt: 160.0,
            height_pt: 60.0,
        }
    }
}

#[derive(Debug, Clone)]
struct PdfObj {
    id: usize,
    generation: usize,
    data: Vec<u8>,
}

struct ParsedPdf {
    header: String,
    objects: Vec<PdfObj>,
    root_id: usize,
}

fn find_subslice(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if needle.is_empty() || start + needle.len() > haystack.len() {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|pos| start + pos)
}

impl ParsedPdf {
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

        let first_line_end = bytes
            .iter()
            .position(|&b| b == b'\r' || b == b'\n')
            .unwrap_or(bytes.len());
        let header = String::from_utf8_lossy(&bytes[0..first_line_end])
            .trim()
            .to_string();

        let mut root_id = 1;
        let raw_text = String::from_utf8_lossy(bytes);
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

        while let Some(obj_start) = find_subslice(bytes, b" obj", search_from) {
            let prefix_slice = &bytes[search_from..obj_start];
            let prefix_str = String::from_utf8_lossy(prefix_slice);
            let mut parts = prefix_str.split_whitespace().rev();
            let gen_str = parts.next();
            let id_str = parts.next();

            if let (Some(g_s), Some(i_s)) = (gen_str, id_str) {
                if let (Ok(id), Ok(generation)) = (i_s.parse::<usize>(), g_s.parse::<usize>()) {
                    let content_start = obj_start + 4;
                    if let Some(end_offset) = find_subslice(bytes, b"endobj", content_start) {
                        let mut start = content_start;
                        while start < end_offset
                            && (bytes[start] == b' '
                                || bytes[start] == b'\r'
                                || bytes[start] == b'\n'
                                || bytes[start] == b'\t')
                        {
                            start += 1;
                        }
                        let mut end = end_offset;
                        while end > start
                            && (bytes[end - 1] == b' '
                                || bytes[end - 1] == b'\r'
                                || bytes[end - 1] == b'\n'
                                || bytes[end - 1] == b'\t')
                        {
                            end -= 1;
                        }

                        objects.push(PdfObj {
                            id,
                            generation,
                            data: bytes[start..end].to_vec(),
                        });
                        search_from = end_offset + 6;
                        continue;
                    }
                }
            }
            search_from = obj_start + 4;
        }

        if let Some(cat_obj) = objects.iter().find(|o| {
            let s = String::from_utf8_lossy(&o.data);
            s.contains("/Type") && s.contains("/Catalog")
        }) {
            root_id = cat_obj.id;
        }

        Ok(ParsedPdf {
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
                let s = String::from_utf8_lossy(&obj.data);
                if s.contains("/Type")
                    && (s.contains("/Page ")
                        || s.contains("/Page\n")
                        || s.contains("/Page/")
                        || s.contains("/Page>"))
                    && !s.contains("/Pages")
                {
                    page_ids.push(obj.id);
                }
            }
        }

        page_ids
    }
}

fn find_pages_root_id(objects: &[PdfObj], root_id: usize) -> Option<usize> {
    if let Some(cat) = objects.iter().find(|o| o.id == root_id) {
        let s = String::from_utf8_lossy(&cat.data);
        if let Some(pos) = s.find("/Pages") {
            let rest = &s[pos + 6..];
            for token in rest.split_whitespace() {
                if let Ok(id) = token.parse::<usize>() {
                    return Some(id);
                }
            }
        }
    }

    objects.iter().find_map(|o| {
        let s = String::from_utf8_lossy(&o.data);
        if s.contains("/Type")
            && (s.contains("/Pages ")
                || s.contains("/Pages\n")
                || s.contains("/Pages/"))
        {
            Some(o.id)
        } else {
            None
        }
    })
}

fn resolve_pages_tree(
    obj_id: usize,
    obj_map: &HashMap<usize, &PdfObj>,
    visited: &mut HashSet<usize>,
    page_ids: &mut Vec<usize>,
) {
    if !visited.insert(obj_id) {
        return;
    }

    if let Some(obj) = obj_map.get(&obj_id) {
        let content = String::from_utf8_lossy(&obj.data);
        let is_page = content.contains("/Type")
            && (content.contains("/Page ")
                || content.contains("/Page\n")
                || content.contains("/Page/")
                || content.contains("/Page>"))
            && !content.contains("/Pages");

        let is_pages = content.contains("/Type")
            && (content.contains("/Pages ")
                || content.contains("/Pages\n")
                || content.contains("/Pages/")
                || content.contains("/Pages>"));

        if is_page {
            page_ids.push(obj_id);
        } else if is_pages || content.contains("/Kids") {
            if let Some(kids_pos) = content.find("/Kids") {
                let rest = &content[kids_pos + 5..];
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

fn parse_box(content: &str, box_key: &str) -> Option<[f64; 4]> {
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
        Some([nums[0], nums[1], nums[2], nums[3]])
    } else {
        None
    }
}

fn attach_signature_to_page(
    page_content: &mut String,
    objects: &mut [PdfObj],
    img_id: usize,
    sig_stream_id: usize,
) {
    // 1. Nhúng /SigImage vào /Resources /XObject
    if let Some(res_pos) = page_content.find("/Resources") {
        let rest = &page_content[res_pos + 10..];
        let trimmed_rest = rest.trim_start();
        if trimmed_rest.starts_with("<<") {
            // Direct dictionary
            if let Some(xobj_pos) = page_content[res_pos..].find("/XObject") {
                let abs_xobj = res_pos + xobj_pos;
                let after_xobj = &page_content[abs_xobj + 8..];
                if let Some(dict_open) = after_xobj.find("<<") {
                    let insert_pos = abs_xobj + 8 + dict_open + 2;
                    page_content.insert_str(insert_pos, &format!(" /SigImage {} 0 R ", img_id));
                }
            } else {
                let after_res = &page_content[res_pos + 10..];
                if let Some(dict_open) = after_res.find("<<") {
                    let insert_pos = res_pos + 10 + dict_open + 2;
                    page_content.insert_str(
                        insert_pos,
                        &format!(" /XObject << /SigImage {} 0 R >> ", img_id),
                    );
                }
            }
        } else {
            // Indirect reference e.g. /Resources 5 0 R
            let mut parts = trimmed_rest.split_whitespace();
            if let (Some(id_str), Some(_gen_str), Some("R")) =
                (parts.next(), parts.next(), parts.next())
            {
                if let Ok(res_id) = id_str.parse::<usize>() {
                    if let Some(res_obj) = objects.iter_mut().find(|o| o.id == res_id) {
                        let mut res_str = String::from_utf8_lossy(&res_obj.data).to_string();
                        if let Some(xobj_pos) = res_str.find("/XObject") {
                            let after_xobj = &res_str[xobj_pos + 8..];
                            if let Some(dict_open) = after_xobj.find("<<") {
                                let insert_pos = xobj_pos + 8 + dict_open + 2;
                                res_str.insert_str(
                                    insert_pos,
                                    &format!(" /SigImage {} 0 R ", img_id),
                                );
                            }
                        } else if let Some(dict_open) = res_str.find("<<") {
                            let insert_pos = dict_open + 2;
                            res_str.insert_str(
                                insert_pos,
                                &format!(" /XObject << /SigImage {} 0 R >> ", img_id),
                            );
                        }
                        res_obj.data = res_str.into_bytes();
                    }
                }
            }
        }
    } else if let Some(dict_end) = page_content.rfind(">>") {
        page_content.insert_str(
            dict_end,
            &format!(" /Resources << /XObject << /SigImage {} 0 R >> >> ", img_id),
        );
    }

    // 2. Bổ sung lệnh kết xuất vào /Contents
    if let Some(contents_pos) = page_content.find("/Contents") {
        let rest = &page_content[contents_pos + 9..];
        let trimmed_rest = rest.trim_start();
        if trimmed_rest.starts_with('[') {
            // /Contents [ ... ]
            if let Some(arr_end) = rest.find(']') {
                let insert_pos = contents_pos + 9 + arr_end;
                page_content.insert_str(insert_pos, &format!(" {} 0 R ", sig_stream_id));
            }
        } else {
            // /Contents X Y R
            let mut parts = trimmed_rest.split_whitespace();
            if let (Some(first), Some(second), Some("R")) =
                (parts.next(), parts.next(), parts.next())
            {
                let match_str = format!("{} {} R", first, second);
                if let Some(ref_pos) = rest.find(&match_str) {
                    let end_pos = contents_pos + 9 + ref_pos + match_str.len();
                    page_content.replace_range(
                        contents_pos..end_pos,
                        &format!("/Contents [{} {} 0 R]", match_str, sig_stream_id),
                    );
                }
            }
        }
    } else if let Some(dict_end) = page_content.rfind(">>") {
        page_content.insert_str(dict_end, &format!(" /Contents {} 0 R ", sig_stream_id));
    }
}

fn render_pdf(header: &str, objects: &[PdfObj], root_id: usize) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(header.as_bytes());
    out.push(b'\n');

    let max_id = objects.iter().map(|o| o.id).max().unwrap_or(root_id);
    let mut offsets = vec![None; max_id + 1];

    for obj in objects {
        let offset = out.len();
        if obj.id <= max_id {
            offsets[obj.id] = Some(offset);
        }
        let prefix = format!("{} {} obj\n", obj.id, obj.generation);
        out.extend_from_slice(prefix.as_bytes());
        out.extend_from_slice(&obj.data);
        out.extend_from_slice(b"\nendobj\n");
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

pub struct PdfSignaturePadTool;

impl Default for PdfSignaturePadTool {
    fn default() -> Self {
        Self
    }
}

impl OfficeTool for PdfSignaturePadTool {
    fn slug(&self) -> &'static str {
        "pdf-signature-pad"
    }

    fn name(&self) -> &'static str {
        "Ký tay Điện tử & Dán Chữ ký PDF"
    }

    fn description(&self) -> &'static str {
        "Bảng vẽ chữ ký tay điện tử mượt mà trên canvas HTML5 và dán chữ ký PNG kênh Alpha trong suốt vào tài liệu PDF tại các vị trí tiêu chuẩn."
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

        let options: PdfSignaturePadOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình đóng dấu chữ ký không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp đầu vào trống".to_string(),
            });
        }

        // Tách chuỗi data URL base64 và xác thực
        let raw_b64 = options.signature_png_base64.trim();
        let clean_b64 = if let Some(idx) = raw_b64.find(";base64,") {
            &raw_b64[idx + 8..]
        } else if let Some(idx) = raw_b64.find(',') {
            &raw_b64[idx + 1..]
        } else {
            raw_b64
        };

        let trimmed_b64 = clean_b64.trim();
        if trimmed_b64.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Dữ liệu chữ ký base64 rỗng".to_string(),
            });
        }

        let png_bytes = BASE64_STANDARD
            .decode(trimmed_b64)
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Chuỗi base64 chữ ký không hợp lệ: {e}"),
            })?;

        // Xác minh magic bytes ảnh PNG: [0x89, 0x50, 0x4E, 0x47]
        if png_bytes.len() < 4 || &png_bytes[0..4] != &[0x89, 0x50, 0x4E, 0x47] {
            return Err(CommandError::InvalidFileSignature {
                expected: "PNG Image (89 50 4E 47)".to_string(),
                actual: format!("{:02X?}", &png_bytes[..png_bytes.len().min(4)]),
            });
        }

        // Giải mã ảnh PNG để tách kênh màu RGB và kênh trong suốt Alpha (SMask)
        let dynamic_img = image::load_from_memory(&png_bytes).map_err(|e| {
            CommandError::ProcessExecutionError {
                message: format!("Không thể giải mã dữ liệu ảnh PNG chữ ký: {e}"),
            }
        })?;
        let rgba_img = dynamic_img.to_rgba8();
        let img_width = rgba_img.width();
        let img_height = rgba_img.height();
        if img_width == 0 || img_height == 0 {
            return Err(CommandError::ValidationFailed {
                reason: "Ảnh chữ ký có kích thước 0x0".to_string(),
            });
        }

        let mut rgb_data = Vec::with_capacity((img_width * img_height * 3) as usize);
        let mut alpha_data = Vec::with_capacity((img_width * img_height) as usize);

        for pixel in rgba_img.pixels() {
            rgb_data.push(pixel[0]);
            rgb_data.push(pixel[1]);
            rgb_data.push(pixel[2]);
            alpha_data.push(pixel[3]);
        }

        let mut rgb_encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        rgb_encoder
            .write_all(&rgb_data)
            .map_err(|e| CommandError::ProcessExecutionError {
                message: format!("Lỗi nén luồng RGB chữ ký: {e}"),
            })?;
        let compressed_rgb =
            rgb_encoder
                .finish()
                .map_err(|e| CommandError::ProcessExecutionError {
                    message: format!("Lỗi hoàn tất nén luồng RGB chữ ký: {e}"),
                })?;

        let mut alpha_encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        alpha_encoder
            .write_all(&alpha_data)
            .map_err(|e| CommandError::ProcessExecutionError {
                message: format!("Lỗi nén luồng Alpha chữ ký: {e}"),
            })?;
        let compressed_alpha =
            alpha_encoder
                .finish()
                .map_err(|e| CommandError::ProcessExecutionError {
                    message: format!("Lỗi hoàn tất nén luồng Alpha chữ ký: {e}"),
                })?;

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_signature_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Đang phân tích cấu trúc trang và tạo đối tượng chữ ký Image XObject & SMask...".to_string(),
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

            let mut parsed = ParsedPdf::parse(&bytes)?;
            let page_ids = parsed.get_page_ids();
            if page_ids.is_empty() {
                return Err(CommandError::ProcessExecutionError {
                    message: format!("Tệp {file_path_str} không chứa trang nào hợp lệ"),
                });
            }

            let total_pages = page_ids.len();
            let target_indices: Vec<usize> = if options.target_page == 0 {
                (0..total_pages).collect()
            } else if options.target_page == -1 {
                vec![total_pages - 1]
            } else if options.target_page >= 1 {
                let p_idx = (options.target_page as usize).saturating_sub(1);
                if p_idx < total_pages {
                    vec![p_idx]
                } else {
                    vec![total_pages - 1]
                }
            } else {
                vec![total_pages - 1]
            };

            let mut max_id = parsed
                .objects
                .iter()
                .map(|o| o.id)
                .max()
                .unwrap_or(parsed.root_id);

            // Tạo đối tượng XObject SMask (Alpha Mask)
            max_id += 1;
            let smask_id = max_id;
            let smask_header = format!(
                "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n",
                img_width, img_height, compressed_alpha.len()
            );
            let mut smask_data = smask_header.into_bytes();
            smask_data.extend_from_slice(&compressed_alpha);
            smask_data.extend_from_slice(b"\nendstream");
            parsed.objects.push(PdfObj {
                id: smask_id,
                generation: 0,
                data: smask_data,
            });

            // Tạo đối tượng XObject Image (RGB + SMask)
            max_id += 1;
            let img_id = max_id;
            let img_header = format!(
                "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {} /SMask {} 0 R >>\nstream\n",
                img_width, img_height, compressed_rgb.len(), smask_id
            );
            let mut img_data = img_header.into_bytes();
            img_data.extend_from_slice(&compressed_rgb);
            img_data.extend_from_slice(b"\nendstream");
            parsed.objects.push(PdfObj {
                id: img_id,
                generation: 0,
                data: img_data,
            });

            // Dán chữ ký vào từng trang mục tiêu
            for &page_idx in &target_indices {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                let target_page_id = page_ids[page_idx];

                // Xác định kích thước và vị trí hộp trang (MediaBox / CropBox)
                let mut page_str = match parsed.objects.iter().find(|o| o.id == target_page_id) {
                    Some(o) => String::from_utf8_lossy(&o.data).to_string(),
                    None => continue,
                };

                let page_box = parse_box(&page_str, "/MediaBox")
                    .or_else(|| parse_box(&page_str, "/CropBox"))
                    .unwrap_or([0.0, 0.0, 595.0, 842.0]);

                let (min_x, max_x) = if page_box[0] < page_box[2] {
                    (page_box[0], page_box[2])
                } else {
                    (page_box[2], page_box[0])
                };
                let (min_y, max_y) = if page_box[1] < page_box[3] {
                    (page_box[1], page_box[3])
                } else {
                    (page_box[3], page_box[1])
                };
                let page_w = max_x - min_x;
                let page_h = max_y - min_y;

                let w = options.width_pt;
                let h = options.height_pt;
                let margin = 36.0;

                let (x, y) = match options.placement {
                    SignaturePlacement::BottomRight => (max_x - margin - w, min_y + margin),
                    SignaturePlacement::BottomLeft => (min_x + margin, min_y + margin),
                    SignaturePlacement::Center => {
                        (min_x + (page_w - w) / 2.0, min_y + (page_h - h) / 2.0)
                    }
                    SignaturePlacement::TopRight => (max_x - margin - w, max_y - margin - h),
                    SignaturePlacement::Custom { x_pt, y_pt } => (x_pt, y_pt),
                };

                // Tạo đối tượng luồng nội dung kết xuất chữ ký: q <w> 0 0 <h> <x> <y> cm /SigImage Do Q
                max_id += 1;
                let sig_stream_id = max_id;
                let sig_ops = format!("q {:.2} 0 0 {:.2} {:.2} {:.2} cm /SigImage Do Q\n", w, h, x, y);
                let sig_stream_data = format!(
                    "<< /Length {} >>\nstream\n{}\nendstream",
                    sig_ops.len(),
                    sig_ops
                );
                parsed.objects.push(PdfObj {
                    id: sig_stream_id,
                    generation: 0,
                    data: sig_stream_data.into_bytes(),
                });

                // Cập nhật cấu trúc trang: Resources (XObject) và Contents
                attach_signature_to_page(&mut page_str, &mut parsed.objects, img_id, sig_stream_id);

                if let Some(p_obj) = parsed.objects.iter_mut().find(|o| o.id == target_page_id) {
                    p_obj.data = page_str.into_bytes();
                }
            }

            let out_bytes = render_pdf(&parsed.header, &parsed.objects, parsed.root_id);
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("document");
            let out_filename = format!("{file_stem}_signed.pdf");
            let out_path = output_dir.join(out_filename);
            fs::write(&out_path, out_bytes).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi tệp PDF đã ký {}: {e}", out_path.display()),
            })?;

            output_paths.push(out_path.to_string_lossy().to_string());

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: 20.0 + (idx as f32 / request.input_paths.len() as f32) * 70.0,
                    message: format!("Đã dán chữ ký tệp {}/{}", idx + 1, request.input_paths.len()),
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
                message: "Dán chữ ký điện tử PDF hoàn tất thành công.".to_string(),
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
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine;
    use serde_json::json;
    use std::sync::atomic::AtomicBool;

    fn get_unique_test_dir(sub: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("test_pdf_sig_pad_{}_{}", sub, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn create_test_pdf(two_pages: bool, page1_landscape: bool) -> Vec<u8> {
        let p1_box = if page1_landscape {
            "[0 0 792 612]"
        } else {
            "[0 0 612 792]"
        };

        if two_pages {
            format!(
                "%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox {} /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length 26 >>\nstream\nBT /F1 12 Tf (Page 1) Tj ET\nendstream\nendobj\n\
5 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 6 0 R >>\nendobj\n\
6 0 obj\n<< /Length 26 >>\nstream\nBT /F1 12 Tf (Page 2) Tj ET\nendstream\nendobj\n\
xref\n0 7\n0000000000 65535 f \n\
trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n500\n%%EOF\n",
                p1_box
            )
            .into_bytes()
        } else {
            format!(
                "%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox {} /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length 26 >>\nstream\nBT /F1 12 Tf (Page 1) Tj ET\nendstream\nendobj\n\
xref\n0 5\n0000000000 65535 f \n\
trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n350\n%%EOF\n",
                p1_box
            )
            .into_bytes()
        }
    }

    fn create_dummy_png_base64() -> String {
        let mut img = image::RgbaImage::new(10, 10);
        for p in img.pixels_mut() {
            *p = image::Rgba([0, 51, 153, 200]);
        }
        let mut png_bytes = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut png_bytes);
        img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
        format!("data:image/png;base64,{}", BASE64_STANDARD.encode(&png_bytes))
    }

    #[test]
    fn test_pdf_signature_pad_happy_path() {
        let temp_dir = get_unique_test_dir("happy");
        let pdf_path = temp_dir.join("input.pdf");
        fs::write(&pdf_path, create_test_pdf(true, false)).unwrap();

        let sig_b64 = create_dummy_png_base64();
        let tool = PdfSignaturePadTool::default();
        let cancel_flag = AtomicBool::new(false);

        let request = DispatchTaskRequest {
            task_id: "test-happy-path".to_string(),
            tool_slug: "pdf-signature-pad".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: json!({
                "signature_png_base64": sig_b64,
                "placement": "bottom_right",
                "target_page": -1,
                "width_pt": 160.0,
                "height_pt": 60.0
            }),
        };

        let resp = tool.execute(&request, &cancel_flag, None).unwrap();
        assert_eq!(resp.status, "SUCCESS");
        assert_eq!(resp.output_paths.len(), 1);

        let out_bytes = fs::read(&resp.output_paths[0]).unwrap();
        let out_text = String::from_utf8_lossy(&out_bytes);
        assert!(out_text.contains("/SigImage"));
        assert!(out_text.contains("/SMask"));
        assert!(out_text.contains("Do"));
        // Last page width 612 -> 612 - 36 - 160 = 416.00, y = 36.00
        assert!(out_text.contains("416.00") && out_text.contains("36.00"));
    }

    #[test]
    fn test_pdf_signature_pad_invalid_magic_bytes() {
        let temp_dir = get_unique_test_dir("invalid");
        let invalid_path = temp_dir.join("fake.pdf");
        fs::write(&invalid_path, b"NOT A VALID PDF FILE AT ALL").unwrap();

        let sig_b64 = create_dummy_png_base64();
        let tool = PdfSignaturePadTool::default();
        let cancel_flag = AtomicBool::new(false);

        let request = DispatchTaskRequest {
            task_id: "test-invalid-bytes".to_string(),
            tool_slug: "pdf-signature-pad".to_string(),
            input_paths: vec![invalid_path.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: json!({
                "signature_png_base64": sig_b64,
            }),
        };

        let result = tool.execute(&request, &cancel_flag, None);
        assert!(matches!(result, Err(CommandError::InvalidFileSignature { .. })));
    }

    #[test]
    fn test_pdf_signature_pad_cooperative_cancellation() {
        let temp_dir = get_unique_test_dir("cancel");
        let pdf_path = temp_dir.join("input.pdf");
        fs::write(&pdf_path, create_test_pdf(false, false)).unwrap();

        let sig_b64 = create_dummy_png_base64();
        let tool = PdfSignaturePadTool::default();
        let cancel_flag = AtomicBool::new(true);

        let request = DispatchTaskRequest {
            task_id: "test-cancel".to_string(),
            tool_slug: "pdf-signature-pad".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: json!({
                "signature_png_base64": sig_b64,
            }),
        };

        let result = tool.execute(&request, &cancel_flag, None);
        assert!(matches!(result, Err(CommandError::TaskCancelled)));
    }

    #[test]
    fn test_pdf_signature_pad_security_guardrails() {
        let temp_dir = get_unique_test_dir("guardrails");
        let pdf_path = temp_dir.join("input.pdf");
        fs::write(&pdf_path, create_test_pdf(false, false)).unwrap();

        let tool = PdfSignaturePadTool::default();
        let cancel_flag = AtomicBool::new(false);

        // Case A: Empty signature base64
        let req_empty = DispatchTaskRequest {
            task_id: "test-empty-sig".to_string(),
            tool_slug: "pdf-signature-pad".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: json!({
                "signature_png_base64": "",
            }),
        };
        let res_empty = tool.execute(&req_empty, &cancel_flag, None);
        assert!(matches!(res_empty, Err(CommandError::ValidationFailed { .. })));

        // Case B: Non-PNG data encoded as base64
        let bad_b64 = BASE64_STANDARD.encode(b"THIS IS NOT A PNG FILE HEADER");
        let req_bad = DispatchTaskRequest {
            task_id: "test-bad-sig".to_string(),
            tool_slug: "pdf-signature-pad".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: json!({
                "signature_png_base64": bad_b64,
            }),
        };
        let res_bad = tool.execute(&req_bad, &cancel_flag, None);
        assert!(
            matches!(res_bad, Err(CommandError::InvalidFileSignature { .. }))
                || matches!(res_bad, Err(CommandError::ValidationFailed { .. }))
        );
    }

    #[test]
    fn test_pdf_signature_pad_edge_case() {
        let temp_dir = get_unique_test_dir("edge");
        let pdf_path = temp_dir.join("landscape_input.pdf");
        fs::write(&pdf_path, create_test_pdf(true, true)).unwrap();

        let sig_b64 = create_dummy_png_base64();
        let tool = PdfSignaturePadTool::default();
        let cancel_flag = AtomicBool::new(false);

        let request = DispatchTaskRequest {
            task_id: "test-edge-landscape".to_string(),
            tool_slug: "pdf-signature-pad".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(temp_dir.to_string_lossy().to_string()),
            options: json!({
                "signature_png_base64": sig_b64,
                "placement": "bottom_right",
                "target_page": 0, // All pages
                "width_pt": 160.0,
                "height_pt": 60.0
            }),
        };

        let resp = tool.execute(&request, &cancel_flag, None).unwrap();
        let out_bytes = fs::read(&resp.output_paths[0]).unwrap();
        let out_text = String::from_utf8_lossy(&out_bytes);

        // Landscape page (792x612): 792 - 36 - 160 = 596.00, y = 36.00
        assert!(out_text.contains("596.00") && out_text.contains("36.00"));
        // Portrait page 2 (612x792): 612 - 36 - 160 = 416.00, y = 36.00
        assert!(out_text.contains("416.00") && out_text.contains("36.00"));
    }
}
