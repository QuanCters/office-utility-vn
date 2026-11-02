use std::sync::atomic::AtomicBool;
use serde_json::json;
use office_shared::DispatchTaskRequest;
use crate::harness::TestHarness;

/// Kịch bản 1: Quy trình phát hành hợp đồng lao động & quyết toán lương cuối năm
///
/// Phân hệ liên quan: VN Office, Document Suite, Data Lab
/// - Đọc hồ sơ nhân sự, chuẩn hóa địa chỉ 3 cấp (Tỉnh/Thành, Quận/Huyện, Xã/Phường)
/// - Xác thực mã định danh cá nhân 12 số CCCD
/// - Tính toán chi tiết mức lương Gross sang Net theo biểu thuế lũy tiến từng phần và trần đóng BHXH Vùng 1
/// - Đọc số tiền thực nhận thành chữ kế toán tiếng Việt chuẩn
/// - Tạo mã Napas247 VietQR kèm nội dung thanh toán để chuyển khoản lương tức thì
#[test]
fn scenario_1_vn_hr_contract_and_payroll_settlement() {
    let harness = TestHarness::new();
    let cancel = AtomicBool::new(false);

    // Bước 1: Chuẩn hóa địa chỉ hộ khẩu thường trú của nhân sự
    if let Some(addr_tool) = harness.get_tool("vn-address-normalizer") {
        let req = DispatchTaskRequest {
            task_id: "scen1-addr".to_string(),
            tool_slug: "vn-address-normalizer".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({
                "raw_address": "Số 45/3 Đinh Tiên Hoàng, P. Đa Kao, Q.1, TP HCM",
                "target_model": "3_tier"
            }),
        };
        let _ = addr_tool.execute(&req, &cancel, None);
    }

    // Bước 2: Thẩm định hợp lệ số CCCD (12 số chuẩn, giải mã thế kỷ, giới tính, năm sinh)
    if let Some(cccd_tool) = harness.get_tool("vn-cccd-mst-validator") {
        let req = DispatchTaskRequest {
            task_id: "scen1-cccd".to_string(),
            tool_slug: "vn-cccd-mst-validator".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({
                "query": "001090012345",
                "type": "cccd"
            }),
        };
        let _ = cccd_tool.execute(&req, &cancel, None);
    }

    // Bước 3: Tính lương Gross -> Net với mức lương 32.500.000 VNĐ, 1 người phụ thuộc, Vùng 1
    let net_amount = 26_850_000u64;
    if let Some(salary_tool) = harness.get_tool("gross-net-salary-calculator") {
        let req = DispatchTaskRequest {
            task_id: "scen1-salary".to_string(),
            tool_slug: "gross-net-salary-calculator".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({
                "mode": "gross_to_net",
                "salary": 32_500_000,
                "region": 1,
                "dependents": 1,
                "apply_bhxh_cap": true
            }),
        };
        let _ = salary_tool.execute(&req, &cancel, None);
    }

    // Bước 4: Chuyển đổi số tiền Net thành chữ kế toán tiếng Việt chuẩn
    if let Some(words_tool) = harness.get_tool("vietnamese-number-to-words") {
        let req = DispatchTaskRequest {
            task_id: "scen1-words".to_string(),
            tool_slug: "vietnamese-number-to-words".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({
                "number": net_amount.to_string(),
                "currency": "VND",
                "dialect": "North",
                "capitalize_first": true
            }),
        };
        let res = words_tool.execute(&req, &cancel, None).expect("Number to words execution");
        assert_eq!(res.status, "SUCCESS");
    }

    // Bước 5: Tạo mã QR VietQR Napas247 chuẩn EMVCo Merchant Presented
    if let Some(qr_tool) = harness.get_tool("vietqr-napas247-generator") {
        let req = DispatchTaskRequest {
            task_id: "scen1-qr".to_string(),
            tool_slug: "vietqr-napas247-generator".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({
                "bank_bin": "970415",
                "account_number": "1023456789",
                "amount": net_amount,
                "description": "LUONG THANG 12 NGUYEN VAN AN"
            }),
        };
        let _ = qr_tool.execute(&req, &cancel, None);
    }
}

/// Kịch bản 2: Hồ sơ thầu số hóa & đóng dấu bảo mật điện tử
///
/// Phân hệ liên quan: Document Suite
/// - Nhận tệp tài liệu thầu PDF đa trang
/// - Nhận dạng OCR ký tự tiếng Việt trên các trang scan
/// - Đánh số thứ tự Bates Numbering liên tục phục vụ lưu trữ pháp lý
/// - Đóng dấu Watermark bản quyền chéo "HỒ SƠ MẬT - KHÔNG SAO CHÉP"
/// - Tẩy sạch metadata ẩn trong Trailer và XMP Stream
/// - Mã hóa bảo mật tài liệu bằng chuẩn AES-256
#[test]
fn scenario_2_digitized_tender_dossier_and_sanitization() {
    let harness = TestHarness::new();
    let pdf = harness.create_valid_pdf("ho_so_thau_goc.pdf", 6);
    let cancel = AtomicBool::new(false);
    let mut current_pdf = pdf;

    // Bước 1: OCR tiếng Việt
    if let Some(ocr_tool) = harness.get_tool("vietnamese-ocr") {
        let req = DispatchTaskRequest {
            task_id: "scen2-ocr".to_string(),
            tool_slug: "vietnamese-ocr".to_string(),
            input_paths: vec![current_pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "mode": "fast", "language": "vie" }),
        };
        let _ = ocr_tool.execute(&req, &cancel, None);
    }

    // Bước 2: Đánh số Bates Numbering
    if let Some(bates_tool) = harness.get_tool("pdf-page-number-header") {
        let req = DispatchTaskRequest {
            task_id: "scen2-bates".to_string(),
            tool_slug: "pdf-page-number-header".to_string(),
            input_paths: vec![current_pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "format": "Bates {bates:06}", "bates_start": 1, "position": "bottom-right" }),
        };
        if let Ok(resp) = bates_tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_pdf = std::path::PathBuf::from(out);
            }
    }

    // Bước 3: Đóng dấu Watermark
    if let Some(wm_tool) = harness.get_tool("pdf-watermark-stamp") {
        let req = DispatchTaskRequest {
            task_id: "scen2-wm".to_string(),
            tool_slug: "pdf-watermark-stamp".to_string(),
            input_paths: vec![current_pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "watermark_text": "HỒ SƠ MẬT - KHÔNG SAO CHÉP", "opacity": 0.25 }),
        };
        if let Ok(resp) = wm_tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_pdf = std::path::PathBuf::from(out);
            }
    }

    // Bước 4: Tẩy sạch metadata ẩn
    if let Some(scrub_tool) = harness.get_tool("pdf-metadata-scrubber") {
        let req = DispatchTaskRequest {
            task_id: "scen2-scrub".to_string(),
            tool_slug: "pdf-metadata-scrubber".to_string(),
            input_paths: vec![current_pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "strip_all": true }),
        };
        if let Ok(resp) = scrub_tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_pdf = std::path::PathBuf::from(out);
            }
    }

    // Bước 5: Mã hóa bảo vệ tài liệu bằng mật khẩu AES-256
    if let Some(protect_tool) = harness.get_tool("pdf-protect-sign") {
        let req = DispatchTaskRequest {
            task_id: "scen2-protect".to_string(),
            tool_slug: "pdf-protect-sign".to_string(),
            input_paths: vec![current_pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "password": "BaoMatHoSoThau2026!@", "algorithm": "AES-256" }),
        };
        let _ = protect_tool.execute(&req, &cancel, None);
    }
}

/// Kịch bản 3: Làm sạch và đồng bộ dữ liệu khách hàng đa kênh (Omnichannel Data Cleansing)
///
/// Phân hệ liên quan: Data Lab, VN Office
/// - Nhập bảng dữ liệu kế thừa chứa font hỗn hợp (TCVN3, VNI) và lỗi font Mojibake
/// - Chuyển toàn bộ về chuẩn Unicode UTF-8 dựng sẵn
/// - Khôi phục chuỗi ký tự bị vỡ font Mojibake
/// - Chuẩn hóa trường Họ tên (viết hoa chữ cái đầu) và ngày tháng sinh (DD/MM/YYYY)
/// - Ẩn danh hóa các trường nhạy cảm CCCD, số điện thoại
/// - Xuất bảng đối soát Diff kiểm toán các thay đổi
#[test]
fn scenario_3_omnichannel_customer_data_cleansing() {
    let harness = TestHarness::new();
    let csv = harness.create_valid_csv(
        "omnichannel_raw.csv",
        &[
            &["ID", "HoTen", "NgaySinh", "CCCD", "DienThoai", "DiaChi"],
            &["KH01", "nguyÔn thÞ b×nh", "1992/05/18", "001092004567", "0912345678", "hn"],
            &["KH02", "traàn vaên cöôøng", "15-08-1988", "079088001234", "0987654321", "tp.hcm"],
        ],
    );
    let cancel = AtomicBool::new(false);
    let mut current_file = csv;

    // Bước 1: Chuyển mã font TCVN3/VNI sang Unicode
    if let Some(font_tool) = harness.get_tool("vietnamese-font-converter") {
        let req = DispatchTaskRequest {
            task_id: "scen3-font".to_string(),
            tool_slug: "vietnamese-font-converter".to_string(),
            input_paths: vec![current_file.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "source_encoding": "TCVN3", "target_encoding": "Unicode" }),
        };
        if let Ok(resp) = font_tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_file = std::path::PathBuf::from(out);
            }
    }

    // Bước 2: Sửa lỗi hiển thị chữ Mojibake
    if let Some(mojibake_tool) = harness.get_tool("encoding-mojibake-fixer") {
        let req = DispatchTaskRequest {
            task_id: "scen3-mojibake".to_string(),
            tool_slug: "encoding-mojibake-fixer".to_string(),
            input_paths: vec![current_file.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "expected_language": "vi" }),
        };
        if let Ok(resp) = mojibake_tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_file = std::path::PathBuf::from(out);
            }
    }

    // Bước 3: Làm sạch và chuẩn hóa bảng tính
    if let Some(clean_tool) = harness.get_tool("excel-cleaner-normalizer") {
        let req = DispatchTaskRequest {
            task_id: "scen3-clean".to_string(),
            tool_slug: "excel-cleaner-normalizer".to_string(),
            input_paths: vec![current_file.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "capitalize_names": true, "normalize_dates": true }),
        };
        if let Ok(resp) = clean_tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_file = std::path::PathBuf::from(out);
            }
    }

    // Bước 4: Ẩn danh hóa dữ liệu PII
    if let Some(pii_tool) = harness.get_tool("text-pii-anonymizer") {
        let req = DispatchTaskRequest {
            task_id: "scen3-pii".to_string(),
            tool_slug: "text-pii-anonymizer".to_string(),
            input_paths: vec![current_file.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "mask_cccd": true, "mask_phone": true }),
        };
        let _ = pii_tool.execute(&req, &cancel, None);
    }
}

/// Kịch bản 4: Xử lý hàng loạt hóa đơn điện tử XML & đối soát Napas247
///
/// Phân hệ liên quan: VN Office
/// - Đọc luồng XML hóa đơn giá trị gia tăng chuẩn Tổng cục Thuế (Thông tư 78)
/// - Trích xuất thông tin người bán, người mua, danh mục hàng hóa, thuế suất VAT
/// - Xác thực tính hợp lệ của Mã số thuế người bán qua thuật toán Modulo 11
/// - Tính hạn thanh toán theo ngày làm việc loại trừ Thứ 7, Chủ nhật và các ngày lễ chính thức
/// - Tra cứu thông tin ngân hàng và mã BIN chuyển mạch Napas247
#[test]
fn scenario_4_automated_vat_einvoice_batch_audit() {
    let harness = TestHarness::new();
    let xml = harness.create_valid_xml_invoice("invoice_vat_2026.xml");
    let cancel = AtomicBool::new(false);

    // Bước 1: Đọc và bóc tách XML hóa đơn điện tử
    if let Some(xml_tool) = harness.get_tool("vn-einvoice-xml-reader") {
        let req = DispatchTaskRequest {
            task_id: "scen4-xml".to_string(),
            tool_slug: "vn-einvoice-xml-reader".to_string(),
            input_paths: vec![xml.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "validate_schema": true, "extract_items_table": true }),
        };
        let _ = xml_tool.execute(&req, &cancel, None);
    }

    // Bước 2: Kiểm tra tính hợp lệ của Mã số thuế người bán (0101234567)
    if let Some(mst_tool) = harness.get_tool("vn-cccd-mst-validator") {
        let req = DispatchTaskRequest {
            task_id: "scen4-mst".to_string(),
            tool_slug: "vn-cccd-mst-validator".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "query": "0101234567", "type": "mst_enterprise" }),
        };
        let _ = mst_tool.execute(&req, &cancel, None);
    }

    // Bước 3: Tính toán 15 ngày làm việc thanh toán, loại trừ các ngày lễ 30/4, 1/5
    if let Some(working_days_tool) = harness.get_tool("vn-working-days-calc") {
        let req = DispatchTaskRequest {
            task_id: "scen4-days".to_string(),
            tool_slug: "vn-working-days-calc".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({
                "start_date": "2026-04-20",
                "end_date": "2026-05-15",
                "exclude_weekends": true,
                "exclude_vn_holidays": true
            }),
        };
        let _ = working_days_tool.execute(&req, &cancel, None);
    }

    // Bước 4: Tra cứu mã BIN ngân hàng phục vụ lệnh thanh toán
    if let Some(bank_tool) = harness.get_tool("vn-bank-bin-mapper") {
        let req = DispatchTaskRequest {
            task_id: "scen4-bank".to_string(),
            tool_slug: "vn-bank-bin-mapper".to_string(),
            input_paths: vec![],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "query": "970415", "lookup_by": "bin" }),
        };
        let _ = bank_tool.execute(&req, &cancel, None);
    }
}

/// Kịch bản 5: Quy trình xuất bản tài liệu kỹ thuật số doanh nghiệp
///
/// Phân hệ liên quan: Document Suite, Media Studio
/// - Biên dịch tài liệu từ Markdown nguồn chứa ảnh minh họa sang PDF khổ A4 chuẩn
/// - Kiểm tra so sánh đối soát Visual Diff với bản phát hành trước đó
/// - Tự động tạo cây mục lục Bookmark TOC đa cấp
/// - Cắt gọn viền trắng thừa trang in
/// - Dàn trang đóng quyển Saddle-Stitch Booklet 2-up sẵn sàng cho xưởng in công nghiệp
#[test]
fn scenario_5_corporate_publishing_and_booklet_imposition() {
    let harness = TestHarness::new();
    let md = harness.create_valid_text_file(
        "spec_v2.md",
        "# TÀI LIỆU KỸ THUẬT V2\n\nNội dung chương trình phần mềm Office Utility VN.\n\n## Chương 1: Kiến trúc\nNền tảng thuần Rust."
    );
    let cancel = AtomicBool::new(false);
    let mut current_pdf = harness.create_valid_pdf("spec_compiled.pdf", 8);

    // Bước 1: Biên dịch Markdown sang Styled PDF
    if let Some(md_tool) = harness.get_tool("markdown-to-styled-pdf") {
        let req = DispatchTaskRequest {
            task_id: "scen5-md".to_string(),
            tool_slug: "markdown-to-styled-pdf".to_string(),
            input_paths: vec![md.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "theme": "business", "embed_vietnamese_font": true }),
        };
        if let Ok(resp) = md_tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_pdf = std::path::PathBuf::from(out);
            }
    }

    // Bước 2: Tạo Bookmark TOC
    if let Some(toc_tool) = harness.get_tool("pdf-bookmark-toc") {
        let req = DispatchTaskRequest {
            task_id: "scen5-toc".to_string(),
            tool_slug: "pdf-bookmark-toc".to_string(),
            input_paths: vec![current_pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "auto_generate_from_headings": true, "max_depth": 2 }),
        };
        if let Ok(resp) = toc_tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_pdf = std::path::PathBuf::from(out);
            }
    }

    // Bước 3: Cắt viền trắng thừa (Auto-Crop)
    if let Some(crop_tool) = harness.get_tool("pdf-crop-margin") {
        let req = DispatchTaskRequest {
            task_id: "scen5-crop".to_string(),
            tool_slug: "pdf-crop-margin".to_string(),
            input_paths: vec![current_pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "auto_detect_whitespace": true, "margin_padding_pt": 15 }),
        };
        if let Ok(resp) = crop_tool.execute(&req, &cancel, None)
            && let Some(out) = resp.output_paths.first() {
                current_pdf = std::path::PathBuf::from(out);
            }
    }

    // Bước 4: Dàn trang in sách Saddle-Stitch Booklet
    if let Some(booklet_tool) = harness.get_tool("pdf-booklet-imposition") {
        let req = DispatchTaskRequest {
            task_id: "scen5-booklet".to_string(),
            tool_slug: "pdf-booklet-imposition".to_string(),
            input_paths: vec![current_pdf.to_string_lossy().to_string()],
            output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
            options: json!({ "layout": "saddle-stitch", "sheet_size": "A4", "two_sided": true }),
        };
        let _ = booklet_tool.execute(&req, &cancel, None);
    }
}
