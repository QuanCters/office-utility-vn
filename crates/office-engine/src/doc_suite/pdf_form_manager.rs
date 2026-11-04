use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FormAction {
    #[default]
    Inspect,
    ExportData,
    AutoFill,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PdfFormManagerOptions {
    #[serde(default)]
    pub action: FormAction,
    #[serde(default)]
    pub export_format: Option<String>, // "json" hoặc "csv"
    #[serde(default)]
    pub fill_values: Option<HashMap<String, String>>,
}

pub struct PdfFormManagerTool;

impl Default for PdfFormManagerTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFieldInfo {
    pub name: String,
    pub field_type: String,
    pub value: String,
    pub rect: Option<[f32; 4]>,
}

#[derive(Debug, Clone)]
struct FormPdfObj {
    id: usize,
    generation: usize,
    content: String,
}

struct FormParsedPdf {
    header: String,
    objects: Vec<FormPdfObj>,
}

impl FormParsedPdf {
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
        let header = raw_text.lines().next().unwrap_or("%PDF-1.4").trim().to_string();

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
                        let content = raw_text[content_start..content_start + end_offset].trim().to_string();
                        objects.push(FormPdfObj {
                            id,
                            generation,
                            content,
                        });
                        search_from = content_start + end_offset + 6;
                        continue;
                    }
                }
            search_from = abs_obj_start + 4;
        }

        Ok(FormParsedPdf { header, objects })
    }

    fn extract_fields(&self) -> Vec<FormFieldInfo> {
        let mut fields = Vec::new();

        for obj in &self.objects {
            if obj.content.contains("/Subtype /Widget")
                || obj.content.contains("/Subtype/Widget")
                || obj.content.contains("/FT ")
            {
                let name = extract_pdf_string(&obj.content, "/T");
                if name.is_empty() {
                    continue;
                }

                let ft = if obj.content.contains("/FT /Tx") || obj.content.contains("/FT/Tx") {
                    "Text".to_string()
                } else if obj.content.contains("/FT /Btn") || obj.content.contains("/FT/Btn") {
                    "Button/Checkbox".to_string()
                } else if obj.content.contains("/FT /Ch") || obj.content.contains("/FT/Ch") {
                    "Choice".to_string()
                } else if obj.content.contains("/FT /Sig") || obj.content.contains("/FT/Sig") {
                    "Signature".to_string()
                } else {
                    "Unknown".to_string()
                };

                let value = extract_pdf_string(&obj.content, "/V");
                let rect = extract_pdf_rect(&obj.content);

                fields.push(FormFieldInfo {
                    name,
                    field_type: ft,
                    value,
                    rect,
                });
            }
        }

        fields
    }

    fn auto_fill_fields(&mut self, fill_map: &HashMap<String, String>) -> usize {
        let mut filled_count = 0;

        for obj in &mut self.objects {
            if obj.content.contains("/Subtype /Widget")
                || obj.content.contains("/Subtype/Widget")
                || obj.content.contains("/FT ")
            {
                let name = extract_pdf_string(&obj.content, "/T");
                if let Some(new_val) = fill_map.get(&name) {
                    // Cập nhật hoặc chèn thẻ /V (giá trị mới)
                    if let Some(v_pos) = obj.content.find("/V") {
                        let rest = &obj.content[v_pos..];
                        if let Some(open_p) = rest.find('(')
                            && let Some(close_p) = rest[open_p..].find(')') {
                                let abs_open = v_pos + open_p;
                                let abs_close = abs_open + close_p;
                                obj.content.replace_range(abs_open + 1..abs_close, new_val);
                                filled_count += 1;
                                continue;
                            }
                    }
                    // Nếu chưa có /V, chèn vào trước dấu kết thúc từ điển >>
                    if let Some(close_dict) = obj.content.rfind(">>") {
                        let clean_val = new_val.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
                        obj.content.insert_str(close_dict, &format!("/V ({clean_val}) "));
                        filled_count += 1;
                    }
                }
            }
        }

        filled_count
    }

    fn render_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(self.header.as_bytes());
        out.push(b'\n');

        let mut xref_offsets = Vec::new();
        let mut root_id = 1;

        for obj in &self.objects {
            if obj.content.contains("/Root ") {
                let rest = &obj.content[obj.content.find("/Root ").unwrap_or(0) + 6..];
                if let Ok(id) = rest.split_whitespace().next().unwrap_or("1").parse::<usize>() {
                    root_id = id;
                }
            }
        }

        for obj in &self.objects {
            let offset = out.len();
            xref_offsets.push((obj.id, offset));
            let s = format!("{} {} obj\n{}\nendobj\n", obj.id, obj.generation, obj.content);
            out.extend_from_slice(s.as_bytes());
        }

        let xref_pos = out.len();
        let max_id = self.objects.iter().map(|o| o.id).max().unwrap_or(1);
        let size = max_id + 1;

        out.extend_from_slice(format!("xref\n0 {}\n", size).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");

        for i in 1..size {
            if let Some((_, off)) = xref_offsets.iter().find(|(id, _)| *id == i) {
                out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
            } else {
                out.extend_from_slice(b"0000000000 00000 f \n");
            }
        }

        let trailer = format!(
            "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n",
            size, root_id, xref_pos
        );
        out.extend_from_slice(trailer.as_bytes());
        out
    }
}

fn extract_pdf_string(content: &str, key: &str) -> String {
    if let Some(pos) = content.find(key) {
        let rest = &content[pos + key.len()..];
        if let Some(start) = rest.find('(')
            && let Some(end) = rest[start + 1..].find(')') {
                return rest[start + 1..start + 1 + end].to_string();
            }
    }
    String::new()
}

fn extract_pdf_rect(content: &str) -> Option<[f32; 4]> {
    if let Some(pos) = content.find("/Rect") {
        let rest = &content[pos + 5..];
        if let Some(start) = rest.find('[')
            && let Some(end) = rest[start + 1..].find(']') {
                let inner = &rest[start + 1..start + 1 + end];
                let nums: Vec<f32> = inner
                    .split_whitespace()
                    .filter_map(|s| s.parse::<f32>().ok())
                    .collect();
                if nums.len() == 4 {
                    return Some([nums[0], nums[1], nums[2], nums[3]]);
                }
            }
    }
    None
}

impl OfficeTool for PdfFormManagerTool {
    fn slug(&self) -> &'static str {
        "pdf-form-manager"
    }

    fn name(&self) -> &'static str {
        "Quản lý & Điền Biểu mẫu PDF (AcroForm Manager)"
    }

    fn description(&self) -> &'static str {
        "Nhận diện cấu trúc AcroForm, trích xuất dữ liệu form sang JSON/CSV và tự động điền giá trị (Auto-fill) vào các trường biểu mẫu."
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

        let options: PdfFormManagerOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình Form Manager không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp đầu vào trống".to_string(),
            });
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_form_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 15.0,
                message: "Đang phân tích cấu trúc đối tượng AcroForm...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: request.input_paths.len(),
                is_completed: false,
            });
        }

        let mut output_paths = Vec::new();

        for file_path_str in &request.input_paths {
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

            let mut parsed = FormParsedPdf::parse(&bytes)?;
            let fields = parsed.extract_fields();
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("document");

            match options.action {
                FormAction::Inspect => {
                    let out_path = output_dir.join(format!("{stem}_fields.json"));
                    let json_data = serde_json::to_string_pretty(&fields).map_err(|e| CommandError::ValidationFailed {
                        reason: format!("Không thể tuần tự hóa thông tin trường: {e}"),
                    })?;
                    fs::write(&out_path, json_data.as_bytes()).map_err(|e| CommandError::IoError {
                        details: format!("Không thể ghi tệp inspect JSON: {e}"),
                    })?;
                    output_paths.push(out_path.to_string_lossy().to_string());
                }

                FormAction::ExportData => {
                    let format = options.export_format.as_deref().unwrap_or("json");
                    if format.eq_ignore_ascii_case("csv") {
                        let out_path = output_dir.join(format!("{stem}_data.csv"));
                        let mut csv_str = String::from("field_name,field_type,value\n");
                        for f in &fields {
                            let clean_val = f.value.replace('"', "\"\"");
                            csv_str.push_str(&format!("\"{}\",\"{}\",\"{}\"\n", f.name, f.field_type, clean_val));
                        }
                        fs::write(&out_path, csv_str.as_bytes()).map_err(|e| CommandError::IoError {
                            details: format!("Không thể ghi tệp CSV: {e}"),
                        })?;
                        output_paths.push(out_path.to_string_lossy().to_string());
                    } else {
                        let out_path = output_dir.join(format!("{stem}_data.json"));
                        let mut map = HashMap::new();
                        for f in &fields {
                            map.insert(f.name.clone(), f.value.clone());
                        }
                        let json_data = serde_json::to_string_pretty(&map).map_err(|e| CommandError::ValidationFailed {
                            reason: format!("Không thể tuần tự hóa dữ liệu form: {e}"),
                        })?;
                        fs::write(&out_path, json_data.as_bytes()).map_err(|e| CommandError::IoError {
                            details: format!("Không thể ghi tệp data JSON: {e}"),
                        })?;
                        output_paths.push(out_path.to_string_lossy().to_string());
                    }
                }

                FormAction::AutoFill => {
                    let fill_map = options.fill_values.clone().unwrap_or_default();
                    parsed.auto_fill_fields(&fill_map);
                    let filled_bytes = parsed.render_bytes();

                    let out_path = output_dir.join(format!("{stem}_filled.pdf"));
                    fs::write(&out_path, filled_bytes).map_err(|e| CommandError::IoError {
                        details: format!("Không thể ghi tệp PDF đã điền: {e}"),
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
                message: "Xử lý biểu mẫu AcroForm hoàn tất thành công.".to_string(),
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

    fn create_test_form_pdf() -> Vec<u8> {
        let pdf = "\
%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 5 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Annots [6 0 R 7 0 R] >>\nendobj\n\
4 0 obj\n<< /Length 20 >>\nstream\nBT /F1 12 Tf (Form Test) Tj ET\nendstream\nendobj\n\
5 0 obj\n<< /Fields [6 0 R 7 0 R] /DA (/Helv 10 Tf 0 g) >>\nendobj\n\
6 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /T (HoVaTen) /V (Nguyen Van A) /Rect [100 700 300 720] >>\nendobj\n\
7 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /T (MaSoThue) /V (0101234567) /Rect [100 650 300 670] >>\nendobj\n\
xref\n0 8\n0000000000 65535 f \n\
trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n650\n%%EOF\n";
        pdf.as_bytes().to_vec()
    }

    #[test]
    fn test_pdf_form_manager_inspect_fields() {
        let tool = PdfFormManagerTool;
        let tmp = std::env::temp_dir().join("test_form_inspect");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("form_doc.pdf");
        fs::write(&pdf_path, create_test_form_pdf()).expect("write test pdf");

        let req = DispatchTaskRequest {
            task_id: "task-form-inspect".to_string(),
            tool_slug: "pdf-form-manager".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "action": "inspect" }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("inspect succeeds");
        assert_eq!(res.output_paths.len(), 1);
        let content = fs::read_to_string(&res.output_paths[0]).expect("read json");
        assert!(content.contains("HoVaTen"));
        assert!(content.contains("MaSoThue"));
    }

    #[test]
    fn test_pdf_form_manager_auto_fill() {
        let tool = PdfFormManagerTool;
        let tmp = std::env::temp_dir().join("test_form_autofill");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("input_form.pdf");
        fs::write(&pdf_path, create_test_form_pdf()).expect("write test pdf");

        let mut fill_map = HashMap::new();
        fill_map.insert("HoVaTen".to_string(), "Tran Thi B".to_string());
        fill_map.insert("MaSoThue".to_string(), "0319876543".to_string());

        let req = DispatchTaskRequest {
            task_id: "task-form-fill".to_string(),
            tool_slug: "pdf-form-manager".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "action": "auto_fill",
                "fill_values": fill_map
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("autofill succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let filled_bytes = fs::read(&res.output_paths[0]).expect("read filled pdf");
        let filled_str = String::from_utf8_lossy(&filled_bytes);
        assert!(filled_str.contains("Tran Thi B"));
        assert!(filled_str.contains("0319876543"));
    }

    #[test]
    fn test_pdf_form_manager_invalid_magic_bytes() {
        let tool = PdfFormManagerTool;
        let tmp = std::env::temp_dir().join("test_form_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("bad.pdf");
        fs::write(&fake_path, b"NOT_A_VALID_PDF_FORMAT").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-fake".to_string(),
            tool_slug: "pdf-form-manager".to_string(),
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
    fn test_pdf_form_manager_cooperative_cancellation() {
        let tool = PdfFormManagerTool;
        let tmp = std::env::temp_dir().join("test_form_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_test_form_pdf()).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-form-manager".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_form_manager_security_guardrails() {
        let tool = PdfFormManagerTool;
        assert_eq!(tool.slug(), "pdf-form-manager");
        assert_eq!(tool.name(), "Quản lý & Điền Biểu mẫu PDF (AcroForm Manager)");
    }
}
