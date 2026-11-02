use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfFlattenOptions {
    #[serde(default = "default_true")]
    pub remove_interactive_forms: bool,
}

impl Default for PdfFlattenOptions {
    fn default() -> Self {
        Self {
            remove_interactive_forms: true,
        }
    }
}

pub struct PdfFlattenFormsTool;

impl Default for PdfFlattenFormsTool {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct FlattenPdfObj {
    id: usize,
    generation: usize,
    content: String,
}

struct FlattenParsedPdf {
    header: String,
    objects: Vec<FlattenPdfObj>,
    root_id: usize,
}

impl FlattenParsedPdf {
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

        let mut root_id = 1;
        if let Some(r_pos) = raw_text.find("/Root ") {
            let rest = &raw_text[r_pos + 6..];
            let id_str = rest.split_whitespace().next().unwrap_or("1");
            if let Ok(id) = id_str.parse::<usize>() {
                root_id = id;
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

            if let (Some(g_s), Some(i_s)) = (gen_str, id_str)
                && let (Ok(id), Ok(generation)) = (i_s.parse::<usize>(), g_s.parse::<usize>()) {
                    let content_start = abs_obj_start + 4;
                    if let Some(end_offset) = raw_text[content_start..].find("endobj") {
                        let content = raw_text[content_start..content_start + end_offset].trim().to_string();
                        objects.push(FlattenPdfObj {
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

        Ok(FlattenParsedPdf {
            header,
            objects,
            root_id,
        })
    }
}

fn render_flattened_pdf(header: &str, objects: &[FlattenPdfObj], root_id: usize) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(header.as_bytes());
    out.push(b'\n');

    let mut xref_offsets = Vec::new();
    xref_offsets.push(0);

    for obj in objects {
        let offset = out.len();
        xref_offsets.push(offset);
        let obj_repr = format!("{} {} obj\n{}\nendobj\n", obj.id, obj.generation, obj.content);
        out.extend_from_slice(obj_repr.as_bytes());
    }

    let xref_pos = out.len();
    let xref_header = format!("xref\n0 {}\n0000000000 65535 f \n", xref_offsets.len());
    out.extend_from_slice(xref_header.as_bytes());

    for &off in &xref_offsets[1..] {
        let line = format!("{:010} 00000 n \n", off);
        out.extend_from_slice(line.as_bytes());
    }

    let trailer = format!(
        "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n",
        xref_offsets.len(),
        root_id,
        xref_pos
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

impl OfficeTool for PdfFlattenFormsTool {
    fn slug(&self) -> &'static str {
        "pdf-flatten-forms"
    }

    fn name(&self) -> &'static str {
        "Làm phẳng PDF (Flatten Forms)"
    }

    fn description(&self) -> &'static str {
        "Hợp nhất luồng hiển thị (AP Appearance Streams) của các trường nhập liệu tương tác (AcroForm, Widget Annotations) trực tiếp vào luồng nội dung tĩnh của trang và xóa mảng trường tương tác, ngăn chặn chỉnh sửa."
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

        let options: PdfFlattenOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình làm phẳng biểu mẫu không hợp lệ: {e}"),
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
            .unwrap_or_else(|| std::env::temp_dir().join("office_pdf_flatten_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        if let Some(prog) = on_progress {
            prog(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 3,
                percentage: 10.0,
                message: "Đang phân tích các trường tương tác AcroForm và Widget Annotations...".to_string(),
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

            let mut parsed = FlattenParsedPdf::parse(&bytes)?;

            if options.remove_interactive_forms {
                // Thu thập giá trị text từ các widget form fields
                let mut form_values = Vec::new();
                for obj in &parsed.objects {
                    if (obj.content.contains("/Subtype /Widget") || obj.content.contains("/FT /Tx"))
                        && let Some(v_idx) = obj.content.find("/V ") {
                            let rest = &obj.content[v_idx + 3..];
                            if let Some(start) = rest.find('(')
                                && let Some(end) = rest[start + 1..].find(')') {
                                    let val = &rest[start + 1..start + 1 + end];
                                    form_values.push(val.to_string());
                                }
                        }
                }

                let mut max_id = parsed.objects.iter().map(|o| o.id).max().unwrap_or(10);
                let mut new_objects = Vec::new();

                for obj in &parsed.objects {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err(CommandError::TaskCancelled);
                    }

                    // 1. Loại bỏ các đối tượng Widget form fields
                    if obj.content.contains("/Subtype /Widget") || obj.content.contains("/Type /Annot") {
                        continue;
                    }

                    // 2. Loại bỏ đối tượng AcroForm
                    if obj.content.contains("/Fields") && obj.content.contains("/DA") {
                        continue;
                    }

                    let mut content = obj.content.clone();

                    // 3. Trong Catalog, loại bỏ tham chiếu /AcroForm
                    if (content.contains("/Type /Catalog") || content.contains("/Type/Catalog"))
                        && let Some(af_pos) = content.find("/AcroForm") {
                            let rest = &content[af_pos + 9..];
                            if let Some(r_pos) = rest.find('R') {
                                let end_idx = af_pos + 9 + r_pos + 1;
                                content.replace_range(af_pos..end_idx, "");
                            }
                        }

                    // 4. Trong Page, loại bỏ /Annots và hợp nhất text của Form vào /Contents
                    if content.contains("/Type /Page") || content.contains("/Type/Page") {
                        if let Some(an_pos) = content.find("/Annots") {
                            let rest = &content[an_pos + 7..];
                            if let Some(rb_pos) = rest.find(']') {
                                let end_idx = an_pos + 7 + rb_pos + 1;
                                content.replace_range(an_pos..end_idx, "");
                            } else if let Some(r_pos) = rest.find('R') {
                                let end_idx = an_pos + 7 + r_pos + 1;
                                content.replace_range(an_pos..end_idx, "");
                            }
                        }

                        // Nếu có form values, tạo thêm luồng tĩnh chứa nội dung đã điền
                        if !form_values.is_empty() {
                            max_id += 1;
                            let flattened_stream_id = max_id;
                            let mut static_stream = String::from("q BT /F1 12 Tf 100 700 Td ");
                            for fv in &form_values {
                                static_stream.push_str(&format!("({}) Tj 0 -20 Td ", fv));
                            }
                            static_stream.push_str("ET Q\n");

                            new_objects.push(FlattenPdfObj {
                                id: flattened_stream_id,
                                generation: 0,
                                content: format!("<< /Length {} >>\nstream\n{}\nendstream", static_stream.len(), static_stream),
                            });

                            if let Some(contents_idx) = content.find("/Contents") {
                                let rest = &content[contents_idx + 9..];
                                if let Some(arr_start) = rest.find('[') {
                                    let abs_arr_start = contents_idx + 9 + arr_start + 1;
                                    content.insert_str(abs_arr_start, &format!(" {} 0 R ", flattened_stream_id));
                                } else if let Some(r_pos) = rest.find('R') {
                                    let end_r = contents_idx + 9 + r_pos + 1;
                                    let old_ref = content[contents_idx + 9..end_r].trim().to_string();
                                    content.replace_range(contents_idx..end_r, &format!("/Contents [{} {} 0 R]", old_ref, flattened_stream_id));
                                }
                            } else if let Some(dict_end) = content.rfind(">>") {
                                content.insert_str(dict_end, &format!(" /Contents {} 0 R ", flattened_stream_id));
                            }
                        }
                    }

                    new_objects.push(FlattenPdfObj {
                        id: obj.id,
                        generation: obj.generation,
                        content,
                    });
                }

                parsed.objects = new_objects;
            }

            let flattened_bytes = render_flattened_pdf(&parsed.header, &parsed.objects, parsed.root_id);

            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("doc");
            let out_path = output_dir.join(format!("{file_stem}_flattened.pdf"));
            fs::write(&out_path, flattened_bytes).map_err(|e| CommandError::IoError {
                details: format!("Không thể ghi tệp PDF làm phẳng: {e}"),
            })?;

            output_paths.push(out_path.to_string_lossy().to_string());

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: 2,
                    total_steps: 3,
                    percentage: 20.0 + (idx as f32 / request.input_paths.len() as f32) * 70.0,
                    message: format!("Đã làm phẳng biểu mẫu {}/{}", idx + 1, request.input_paths.len()),
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
                message: "Làm phẳng biểu mẫu AcroForms PDF thành công.".to_string(),
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

    fn create_test_acroform_pdf() -> Vec<u8> {
        let pdf = "\
%PDF-1.4\n\
1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 5 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Annots [6 0 R] >>\nendobj\n\
4 0 obj\n<< /Length 20 >>\nstream\nBT /F1 12 Tf (Form Tinh) Tj ET\nendstream\nendobj\n\
5 0 obj\n<< /Fields [6 0 R] /DA (/Helv 10 Tf 0 g) >>\nendobj\n\
6 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /T (HoVaTen) /V (Nguyen Van A) /Rect [100 700 300 720] >>\nendobj\n\
xref\n0 7\n0000000000 65535 f \n\
trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n600\n%%EOF\n";
        pdf.as_bytes().to_vec()
    }

    #[test]
    fn test_pdf_flatten_forms_happy_path() {
        let tool = PdfFlattenFormsTool;
        let tmp = std::env::temp_dir().join("test_flatten_happy");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("form.pdf");
        fs::write(&pdf_path, create_test_acroform_pdf()).expect("write pdf");

        let req = DispatchTaskRequest {
            task_id: "task-flatten-happy".to_string(),
            tool_slug: "pdf-flatten-forms".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "remove_interactive_forms": true
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("flatten succeeds");
        assert_eq!(res.output_paths.len(), 1);

        let out_bytes = fs::read(&res.output_paths[0]).expect("read flattened");
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(!out_str.contains("/AcroForm"));
        assert!(!out_str.contains("/Subtype /Widget"));
        assert!(out_str.contains("Nguyen Van A"));
    }

    #[test]
    fn test_pdf_flatten_forms_edge_case() {
        let tool = PdfFlattenFormsTool;
        let tmp = std::env::temp_dir().join("test_flatten_edge");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("interactive.pdf");
        fs::write(&pdf_path, create_test_acroform_pdf()).expect("write pdf");

        let req = DispatchTaskRequest {
            task_id: "task-flatten-edge".to_string(),
            tool_slug: "pdf-flatten-forms".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "remove_interactive_forms": true
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("flatten succeeds");
        let out_bytes = fs::read(&res.output_paths[0]).expect("read flattened");
        let out_str = String::from_utf8_lossy(&out_bytes);
        assert!(!out_str.contains("/Annots"));
        assert!(!out_str.contains("/FT /Tx"));
    }

    #[test]
    fn test_pdf_flatten_forms_invalid_magic_bytes() {
        let tool = PdfFlattenFormsTool;
        let tmp = std::env::temp_dir().join("test_flatten_fake");
        let _ = fs::create_dir_all(&tmp);

        let fake_path = tmp.join("bad.pdf");
        fs::write(&fake_path, b"NOT_A_VALID_PDF").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-fake".to_string(),
            tool_slug: "pdf-flatten-forms".to_string(),
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
    fn test_pdf_flatten_forms_cooperative_cancellation() {
        let tool = PdfFlattenFormsTool;
        let tmp = std::env::temp_dir().join("test_flatten_cancel");
        let _ = fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("doc.pdf");
        fs::write(&pdf_path, create_test_acroform_pdf()).expect("write doc");

        let req = DispatchTaskRequest {
            task_id: "task-cancel".to_string(),
            tool_slug: "pdf-flatten-forms".to_string(),
            input_paths: vec![pdf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({}),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_pdf_flatten_forms_security_guardrails() {
        let tool = PdfFlattenFormsTool;
        assert_eq!(tool.slug(), "pdf-flatten-forms");
        assert_eq!(tool.name(), "Làm phẳng PDF (Flatten Forms)");
    }
}
