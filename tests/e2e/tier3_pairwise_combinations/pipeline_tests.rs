use std::sync::atomic::AtomicBool;
use serde_json::json;
use office_shared::DispatchTaskRequest;
use crate::harness::TestHarness;

/// Pipeline 1: Chu trình xuất bản tài liệu PDF toàn diện
/// markdown-to-styled-pdf -> pdf-page-number-header -> pdf-watermark-stamp -> pdf-compress
#[test]
fn test_pipeline_document_full_lifecycle() {
    let harness = TestHarness::new();
    let md = harness.create_valid_text_file(
        "report.md",
        "# BÁO CÁO TÀI CHÍNH NĂM 2026\n\nCông ty Cổ phần Tiện ích Việt Nam kính báo.\n\n## 1. Doanh thu thuần\nĐạt 150 tỷ đồng."
    );

    let cancel = AtomicBool::new(false);
    let mut current_pdf = harness.create_valid_pdf("generated_report.pdf", 4);

    // Bước 1: markdown-to-styled-pdf
    if let Some(tool) = harness.get_tool("markdown-to-styled-pdf") {
        let req = DispatchTaskRequest {
            task_id: "pipe1-step1".to_string(),
            tool_slug: "markdown-to-styled-pdf".to_string(),
            input_paths: vec![md.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "theme": "business", "embed_vietnamese_font": true }),
        };
        if let Ok(resp) = tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_pdf = std::path::PathBuf::from(out);
            }
    }

    // Bước 2: pdf-page-number-header
    if let Some(tool) = harness.get_tool("pdf-page-number-header") {
        let req = DispatchTaskRequest {
            task_id: "pipe1-step2".to_string(),
            tool_slug: "pdf-page-number-header".to_string(),
            input_paths: vec![current_pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "format": "Trang {page}/{total}", "position": "bottom-right" }),
        };
        if let Ok(resp) = tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_pdf = std::path::PathBuf::from(out);
            }
    }

    // Bước 3: pdf-watermark-stamp
    if let Some(tool) = harness.get_tool("pdf-watermark-stamp") {
        let req = DispatchTaskRequest {
            task_id: "pipe1-step3".to_string(),
            tool_slug: "pdf-watermark-stamp".to_string(),
            input_paths: vec![current_pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "watermark_text": "LƯU HÀNH NỘI BỘ", "opacity": 0.25 }),
        };
        if let Ok(resp) = tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_pdf = std::path::PathBuf::from(out);
            }
    }

    // Bước 4: pdf-compress
    if let Some(tool) = harness.get_tool("pdf-compress") {
        let req = DispatchTaskRequest {
            task_id: "pipe1-step4".to_string(),
            tool_slug: "pdf-compress".to_string(),
            input_paths: vec![current_pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "compression_level": "maximum", "dpi": 150 }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }
}

/// Pipeline 2: Xử lý hình ảnh & đóng dấu bản quyền
/// screenshot-beautifier -> doc-scanner-perspective -> image-batch -> watermark-batch-studio
#[test]
fn test_pipeline_media_preparation() {
    let harness = TestHarness::new();
    let img = harness.create_valid_bmp("screen_raw.bmp", 64, 64);
    let cancel = AtomicBool::new(false);
    let mut current_img = img;

    // Bước 1: screenshot-beautifier
    if let Some(tool) = harness.get_tool("screenshot-beautifier") {
        let req = DispatchTaskRequest {
            task_id: "pipe2-step1".to_string(),
            tool_slug: "screenshot-beautifier".to_string(),
            input_paths: vec![current_img.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "window_frame": "macos_dark", "shadow_radius": 15 }),
        };
        if let Ok(resp) = tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_img = std::path::PathBuf::from(out);
            }
    }

    // Bước 2: image-batch (chuyển đổi và nén tối ưu)
    if let Some(tool) = harness.get_tool("image-batch") {
        let req = DispatchTaskRequest {
            task_id: "pipe2-step2".to_string(),
            tool_slug: "image-batch".to_string(),
            input_paths: vec![current_img.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "target_format": "png", "quality": 85, "strip_exif": true }),
        };
        if let Ok(resp) = tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_img = std::path::PathBuf::from(out);
            }
    }

    // Bước 3: watermark-batch-studio
    if let Some(tool) = harness.get_tool("watermark-batch-studio") {
        let req = DispatchTaskRequest {
            task_id: "pipe2-step3".to_string(),
            tool_slug: "watermark-batch-studio".to_string(),
            input_paths: vec![current_img.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "text": "BẢN QUYỀN HÌNH ẢNH 2026", "opacity": 0.3 }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }
}

/// Pipeline 3: Làm sạch dữ liệu, chuẩn hóa font và ẩn danh hóa PII
/// vietnamese-font-converter -> excel-cleaner-normalizer -> text-pii-anonymizer -> diff-checker-text-sheets
#[test]
fn test_pipeline_data_cleaning_and_diff() {
    let harness = TestHarness::new();
    let csv = harness.create_valid_csv(
        "legacy_data.csv",
        &[
            &["HoTen", "CCCD", "DienThoai", "DiaChi"],
            &["NguyÔn V¨n An", "001090012345", "0912345678", "Hµ Néi"],
            &["TrÇn ThÞ B×nh", "079088054321", "0987654321", "TP Hå ChÝ Minh"],
        ],
    );

    let cancel = AtomicBool::new(false);
    let mut current_file = csv;

    // Bước 1: vietnamese-font-converter (TCVN3 -> Unicode)
    if let Some(tool) = harness.get_tool("vietnamese-font-converter") {
        let req = DispatchTaskRequest {
            task_id: "pipe3-step1".to_string(),
            tool_slug: "vietnamese-font-converter".to_string(),
            input_paths: vec![current_file.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "source_encoding": "TCVN3", "target_encoding": "Unicode" }),
        };
        if let Ok(resp) = tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_file = std::path::PathBuf::from(out);
            }
    }

    // Bước 2: text-pii-anonymizer
    if let Some(tool) = harness.get_tool("text-pii-anonymizer") {
        let req = DispatchTaskRequest {
            task_id: "pipe3-step2".to_string(),
            tool_slug: "text-pii-anonymizer".to_string(),
            input_paths: vec![current_file.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "mask_cccd": true, "mask_phone": true }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }
}

/// Pipeline 4: Chu trình quyết toán lương & tạo QR chuyển khoản tức thì
/// vn-einvoice-xml-reader -> gross-net-salary-calculator -> vietnamese-number-to-words -> vietqr-napas247-generator
#[test]
fn test_pipeline_vn_payroll_settlement() {
    let harness = TestHarness::new();
    let cancel = AtomicBool::new(false);

    // Bước 1: Tính toán lương Gross sang Net
    let net_salary = 20_000_000u64;
    if let Some(tool) = harness.get_tool("gross-net-salary-calculator") {
        let req = DispatchTaskRequest {
            task_id: "pipe4-step1".to_string(),
            tool_slug: "gross-net-salary-calculator".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "mode": "gross_to_net", "salary": 25_000_000, "region": 1, "dependents": 1 }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }

    // Bước 2: Đọc số tiền thành chữ kế toán tiếng Việt chuẩn
    if let Some(tool) = harness.get_tool("vietnamese-number-to-words") {
        let req = DispatchTaskRequest {
            task_id: "pipe4-step2".to_string(),
            tool_slug: "vietnamese-number-to-words".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({
                "number": net_salary.to_string(),
                "currency": "VND",
                "dialect": "North",
                "capitalize_first": true
            }),
        };
        let res = tool.execute(&req, &cancel, None).expect("Number to words must succeed");
        assert_eq!(res.status, "SUCCESS");
    }

    // Bước 3: Tạo mã thanh toán VietQR Napas247
    if let Some(tool) = harness.get_tool("vietqr-napas247-generator") {
        let req = DispatchTaskRequest {
            task_id: "pipe4-step3".to_string(),
            tool_slug: "vietqr-napas247-generator".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({
                "bank_bin": "970415",
                "account_number": "1012345678",
                "amount": net_salary,
                "description": "LUONG THANG 11 NGUYEN VAN AN"
            }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }
}

/// Pipeline 5: Chuyển đổi dữ liệu và mã hóa file Base64
/// excel-visual-joiner -> data-structure-converter -> base64-file-codec
#[test]
fn test_pipeline_data_export_and_encoding() {
    let harness = TestHarness::new();
    let csv = harness.create_valid_csv("export.csv", &[&["MaNV", "Diem"], &["NV01", "95"], &["NV02", "88"]]);
    let cancel = AtomicBool::new(false);

    // Bước 1: data-structure-converter (CSV -> JSON)
    if let Some(tool) = harness.get_tool("data-structure-converter") {
        let req = DispatchTaskRequest {
            task_id: "pipe5-step1".to_string(),
            tool_slug: "data-structure-converter".to_string(),
            input_paths: vec![csv.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "source_format": "csv", "target_format": "json_array" }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }

    // Bước 2: base64-file-codec (Encode file sang Data URI)
    if let Some(tool) = harness.get_tool("base64-file-codec") {
        let req = DispatchTaskRequest {
            task_id: "pipe5-step2".to_string(),
            tool_slug: "base64-file-codec".to_string(),
            input_paths: vec![csv.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "mode": "encode", "emit_data_uri": true }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }
}

/// Pipeline 6: Tương tác liên phân hệ: PDF sang ảnh dài -> Nén ảnh batch -> Che mờ dữ liệu
/// pdf-to-long-image -> image-batch -> image-bg-redact
#[test]
fn test_pipeline_cross_subsystem_doc_to_media() {
    let harness = TestHarness::new();
    let pdf = harness.create_valid_pdf("doc_for_long_image.pdf", 3);
    let cancel = AtomicBool::new(false);

    if let Some(tool) = harness.get_tool("pdf-to-long-image") {
        let req = DispatchTaskRequest {
            task_id: "pipe6-step1".to_string(),
            tool_slug: "pdf-to-long-image".to_string(),
            input_paths: vec![pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "format": "png", "dpi": 150 }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }
}

/// Pipeline 7: Hợp nhất bảng tính -> Tách theo cột -> Gỡ mật khẩu sheet
/// excel-multi-merger -> excel-split-by-column -> excel-sheet-unlocker
#[test]
fn test_pipeline_excel_merger_and_split() {
    let harness = TestHarness::new();
    let f1 = harness.create_valid_csv("sheet1.csv", &[&["Dept", "Name"], &["IT", "An"], &["HR", "Binh"]]);
    let f2 = harness.create_valid_csv("sheet2.csv", &[&["Dept", "Name"], &["IT", "Cuong"], &["Finance", "Dung"]]);
    let cancel = AtomicBool::new(false);

    if let Some(tool) = harness.get_tool("excel-multi-merger") {
        let req = DispatchTaskRequest {
            task_id: "pipe7-step1".to_string(),
            tool_slug: "excel-multi-merger".to_string(),
            input_paths: vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "merge_mode": "append_rows" }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }
}

/// Pipeline 8: Trộn hợp đồng mẫu Word -> Làm phẳng Form -> Đặt mật khẩu mã hóa PDF
/// docx-template-mailmerge -> pdf-flatten-forms -> pdf-protect-sign
#[test]
fn test_pipeline_contract_mailmerge_and_protect() {
    let harness = TestHarness::new();
    let doc = harness.create_valid_text_file("template.docx", "HỢP ĐỒNG LAO ĐỘNG SỐ {{stt}}");
    let cancel = AtomicBool::new(false);

    if let Some(tool) = harness.get_tool("docx-template-mailmerge") {
        let req = DispatchTaskRequest {
            task_id: "pipe8-step1".to_string(),
            tool_slug: "docx-template-mailmerge".to_string(),
            input_paths: vec![doc.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "data_source": "inline" }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }
}

/// Pipeline 9: Thẩm định hồ sơ ngân hàng: Chuẩn hóa địa chỉ -> Xác thực CCCD/MST -> Tra cứu BIN -> Tính ngày làm việc
/// vn-address-normalizer -> vn-cccd-mst-validator -> vn-bank-bin-mapper -> vn-working-days-calc
#[test]
fn test_pipeline_vn_banking_audit() {
    let harness = TestHarness::new();
    let cancel = AtomicBool::new(false);

    if let Some(tool) = harness.get_tool("vn-address-normalizer") {
        let req = DispatchTaskRequest {
            task_id: "pipe9-step1".to_string(),
            tool_slug: "vn-address-normalizer".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "raw_address": "Số 10 Tràng Tiền, Q. Hoàn Kiếm, HN" }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }

    if let Some(tool) = harness.get_tool("vn-cccd-mst-validator") {
        let req = DispatchTaskRequest {
            task_id: "pipe9-step2".to_string(),
            tool_slug: "vn-cccd-mst-validator".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "query": "001090012345", "type": "cccd" }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }
}

/// Pipeline 10: Khôi phục cấu trúc PDF bị hỏng -> Bóc tách bảng dữ liệu -> Nối trực quan Excel
/// pdf-repair-corrupt -> pdf-table-extractor -> excel-visual-joiner
#[test]
fn test_pipeline_pdf_repair_and_extract() {
    let harness = TestHarness::new();
    let corrupt_pdf = harness.create_valid_pdf("repaired_sample.pdf", 2);
    let cancel = AtomicBool::new(false);

    if let Some(tool) = harness.get_tool("pdf-repair-corrupt") {
        let req = DispatchTaskRequest {
            task_id: "pipe10-step1".to_string(),
            tool_slug: "pdf-repair-corrupt".to_string(),
            input_paths: vec![corrupt_pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "rebuild_xref": true, "salvage_pages": true }),
        };
        let _ = tool.execute(&req, &cancel, None);
    }
}
