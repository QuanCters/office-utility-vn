use office_utility_vn_ui::app::{TaskHistoryUi, ALL_TOOLS};
use office_utility_vn_ui::launcher::{
    filter_tools, format_elapsed_time, format_history_status, remove_vietnamese_diacritics,
};
use office_utility_vn_ui::micro_apps::{tool_slug_to_micro_app, MicroAppId};

#[test]
fn test_remove_vietnamese_diacritics() {
    // 1. Kiểm tra chữ có dấu thông dụng
    assert_eq!(
        remove_vietnamese_diacritics("Đọc Số Tiền Thành Chữ Chuẩn"),
        "doc so tien thanh chu chuan"
    );

    // 2. Kiểm tra dấu hỏi, ngã, nặng, huyền, sắc
    assert_eq!(
        remove_vietnamese_diacritics("à á ả ã ạ â ầ ấ ẩ ẫ ậ ă ằ ắ ẳ ẵ ặ"),
        "a a a a a a a a a a a a a a a a a"
    );
    assert_eq!(
        remove_vietnamese_diacritics("è é ẻ ẽ ẹ ê ề ế ể ễ ệ"),
        "e e e e e e e e e e e"
    );
    assert_eq!(
        remove_vietnamese_diacritics("ì í ỉ ĩ ị"),
        "i i i i i"
    );
    assert_eq!(
        remove_vietnamese_diacritics("ò ó ỏ õ ọ ô ồ ố ổ ỗ ộ ơ ờ ớ ở ỡ ợ"),
        "o o o o o o o o o o o o o o o o o"
    );
    assert_eq!(
        remove_vietnamese_diacritics("ù ú ủ ũ ụ ư ừ ứ ử ữ ự"),
        "u u u u u u u u u u u"
    );
    assert_eq!(
        remove_vietnamese_diacritics("ỳ ý ỷ ỹ ỵ"),
        "y y y y y"
    );

    // 3. Chữ Đ / đ
    assert_eq!(
        remove_vietnamese_diacritics("Đường Đời"),
        "duong doi"
    );

    // 4. Chuỗi tiếng Anh hoặc không có dấu được giữ nguyên (lowercase)
    assert_eq!(
        remove_vietnamese_diacritics("PDF Studio 2026 - Data Lab"),
        "pdf studio 2026 - data lab"
    );
}

#[test]
fn test_filter_tools_empty_query() {
    let all = filter_tools(ALL_TOOLS, "");
    assert_eq!(all.len(), ALL_TOOLS.len());
    assert_eq!(all.len(), 58);

    let whitespace = filter_tools(ALL_TOOLS, "   ");
    assert_eq!(whitespace.len(), ALL_TOOLS.len());
}

#[test]
fn test_filter_tools_unaccented_search() {
    // Tìm kiếm gõ không dấu "luong" phải tìm ra công cụ Tính Lương Gross - Net
    let res = filter_tools(ALL_TOOLS, "luong");
    assert!(!res.is_empty(), "Should find salary tool with 'luong'");
    assert!(res.iter().any(|t| t.slug == "gross-net-salary-calculator"));

    // Tìm kiếm gõ không dấu "doc so"
    let res = filter_tools(ALL_TOOLS, "doc so");
    assert!(res.iter().any(|t| t.slug == "vietnamese-number-to-words"));

    // Tìm kiếm gõ không dấu "hoa don"
    let res = filter_tools(ALL_TOOLS, "hoa don");
    assert!(res.iter().any(|t| t.slug == "vn-einvoice-xml-reader"));

    // Tìm kiếm gõ không dấu "ghep"
    let res = filter_tools(ALL_TOOLS, "ghep");
    assert!(res.iter().any(|t| t.slug == "pdf-merge-split"));

    // Tìm kiếm gõ không dấu "anh the"
    let res = filter_tools(ALL_TOOLS, "anh the");
    assert!(res.iter().any(|t| t.slug == "photo-id-crop"));
}

#[test]
fn test_filter_tools_accented_search() {
    // Tìm kiếm gõ chuẩn tiếng Việt có dấu "hóa đơn"
    let res = filter_tools(ALL_TOOLS, "hóa đơn");
    assert!(res.iter().any(|t| t.slug == "vn-einvoice-xml-reader"));

    // Tìm kiếm "Lương"
    let res = filter_tools(ALL_TOOLS, "Lương");
    assert!(res.iter().any(|t| t.slug == "gross-net-salary-calculator"));

    // Tìm kiếm "bảo mật"
    let res = filter_tools(ALL_TOOLS, "bảo mật");
    assert!(res.iter().any(|t| t.slug == "pdf-protect-sign"));
}

#[test]
fn test_filter_tools_by_slug_and_domain() {
    // Tìm theo slug
    let res = filter_tools(ALL_TOOLS, "vietqr");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].slug, "vietqr-napas247-generator");

    // Tìm kiếm không tồn tại
    let res = filter_tools(ALL_TOOLS, "non_existent_tool_xyz_123");
    assert!(res.is_empty());
}

#[test]
fn test_all_tools_have_valid_micro_app_mapping() {
    // Đảm bảo mọi công cụ trong ALL_TOOLS đều được ánh xạ về một MicroAppId hợp lệ
    for tool in ALL_TOOLS.iter() {
        let app_opt = tool_slug_to_micro_app(tool.slug);
        assert!(
            app_opt.is_some(),
            "Tool '{}' must be mapped to a MicroAppId",
            tool.slug
        );
    }
}

#[test]
fn test_micro_app_cards_metadata_consistency() {
    let all_apps = MicroAppId::ALL;
    assert_eq!(all_apps.len(), 4);

    let vn_office = MicroAppId::VnOffice.metadata();
    assert_eq!(vn_office.name, "VN Office");
    assert_eq!(vn_office.icon, "🏢");
    assert_eq!(vn_office.tool_count, 11);

    let pdf_studio = MicroAppId::PdfStudio.metadata();
    assert_eq!(pdf_studio.name, "PDF Studio");
    assert_eq!(pdf_studio.icon, "📄");
    assert_eq!(pdf_studio.tool_count, 25);

    let media_studio = MicroAppId::MediaStudio.metadata();
    assert_eq!(media_studio.name, "Media Studio");
    assert_eq!(media_studio.icon, "🎨");
    assert_eq!(media_studio.tool_count, 9);

    let data_lab = MicroAppId::DataLab.metadata();
    assert_eq!(data_lab.name, "Data Lab");
    assert_eq!(data_lab.icon, "📊");
    assert_eq!(data_lab.tool_count, 13);

    let total_tools: usize = all_apps.iter().map(|a| a.metadata().tool_count).sum();
    assert_eq!(total_tools, 58, "Sum of micro-app tools must equal 58");
}

#[test]
fn test_format_elapsed_time() {
    assert_eq!(format_elapsed_time(0), "0 ms");
    assert_eq!(format_elapsed_time(450), "450 ms");
    assert_eq!(format_elapsed_time(999), "999 ms");
    assert_eq!(format_elapsed_time(1000), "1.0 s");
    assert_eq!(format_elapsed_time(1500), "1.5 s");
    assert_eq!(format_elapsed_time(12345), "12.3 s");
}

#[test]
fn test_format_history_status() {
    let (label, class) = format_history_status("success");
    assert_eq!(label, "Thành công");
    assert_eq!(class, "status-success");

    let (label, class) = format_history_status("ok");
    assert_eq!(label, "Thành công");
    assert_eq!(class, "status-success");

    let (label, class) = format_history_status("failed");
    assert_eq!(label, "Thất bại");
    assert_eq!(class, "status-failed");

    let (label, class) = format_history_status("error");
    assert_eq!(label, "Thất bại");
    assert_eq!(class, "status-failed");

    let (label, class) = format_history_status("running");
    assert_eq!(label, "Đang chạy");
    assert_eq!(class, "status-running");

    let (label, class) = format_history_status("anything_else");
    assert_eq!(label, "Đã xong");
    assert_eq!(class, "status-other");
}

#[test]
fn test_history_deep_link_derivation() {
    // 1. Bản ghi lịch sử của PDF Studio
    let history_pdf = TaskHistoryUi {
        id: "task-1".to_string(),
        tool_slug: Some("pdf-merge-split".to_string()),
        workflow_id: None,
        status: "success".to_string(),
        elapsed_ms: 1200,
        created_at: "2026-11-03 10:00:00".to_string(),
        error_message: None,
    };
    let slug = history_pdf.tool_slug.as_ref().unwrap();
    let app = tool_slug_to_micro_app(slug);
    assert_eq!(app, Some(MicroAppId::PdfStudio));

    // 2. Bản ghi lịch sử của VN Office
    let history_vn = TaskHistoryUi {
        id: "task-2".to_string(),
        tool_slug: Some("vietqr-napas247-generator".to_string()),
        workflow_id: None,
        status: "success".to_string(),
        elapsed_ms: 350,
        created_at: "2026-11-03 10:05:00".to_string(),
        error_message: None,
    };
    let slug = history_vn.tool_slug.as_ref().unwrap();
    let app = tool_slug_to_micro_app(slug);
    assert_eq!(app, Some(MicroAppId::VnOffice));

    // 3. Bản ghi lịch sử của Media Studio
    let history_media = TaskHistoryUi {
        id: "task-3".to_string(),
        tool_slug: Some("image-batch".to_string()),
        workflow_id: None,
        status: "success".to_string(),
        elapsed_ms: 4500,
        created_at: "2026-11-03 10:10:00".to_string(),
        error_message: None,
    };
    let slug = history_media.tool_slug.as_ref().unwrap();
    let app = tool_slug_to_micro_app(slug);
    assert_eq!(app, Some(MicroAppId::MediaStudio));

    // 4. Bản ghi lịch sử của Data Lab
    let history_data = TaskHistoryUi {
        id: "task-4".to_string(),
        tool_slug: Some("excel-cleaner-normalizer".to_string()),
        workflow_id: None,
        status: "failed".to_string(),
        elapsed_ms: 800,
        created_at: "2026-11-03 10:15:00".to_string(),
        error_message: Some("Invalid header".to_string()),
    };
    let slug = history_data.tool_slug.as_ref().unwrap();
    let app = tool_slug_to_micro_app(slug);
    assert_eq!(app, Some(MicroAppId::DataLab));

    // 5. Bản ghi không có tool_slug (ví dụ workflow tổng hợp)
    let history_empty_slug = TaskHistoryUi {
        id: "task-5".to_string(),
        tool_slug: None,
        workflow_id: Some("custom-pipeline".to_string()),
        status: "ok".to_string(),
        elapsed_ms: 2000,
        created_at: "2026-11-03 10:20:00".to_string(),
        error_message: None,
    };
    let app = history_empty_slug.tool_slug.as_deref().and_then(tool_slug_to_micro_app);
    assert_eq!(app, None);
}
