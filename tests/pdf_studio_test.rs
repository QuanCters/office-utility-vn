use leptos::prelude::*;
use office_utility_vn_ui::micro_apps::pdf_studio::bridge::{
    IncomingBridgeMessage, OutgoingBridgeMessage, PdfCraftBridge,
};
use office_utility_vn_ui::micro_apps::pdf_studio::canvas_frame::PdfCraftCanvasFrame;
use office_utility_vn_ui::micro_apps::pdf_studio::state::{category_for_tool, PdfCategory, PdfStudioState};
use office_utility_vn_ui::micro_apps::pdf_studio::toolbar::pdf_studio_tool_icon;
use office_utility_vn_ui::micro_apps::pdf_studio::vn_ribbon::PdfVnExtensionRibbon;
use office_utility_vn_ui::micro_apps::{tool_slug_to_micro_app, MicroAppId, PDF_STUDIO_TOOLS};

const EXPECTED_PDF_STUDIO_SLUGS: &[&str] = &[
    "pdf-merge-split",
    "pdf-protect-sign",
    "pdf-to-markdown",
    "pdf-compress",
    "vietnamese-ocr",
    "pdf-watermark-stamp",
    "pdf-page-number-header",
    "pdf-metadata-scrubber",
    "pdf-flatten-forms",
    "pdf-form-manager",
    "pdf-redaction",
    "pdf-archive-linearize",
    "pdf-pki-sign",
    "pdf-crop-margin",
    "pdf-booklet-imposition",
    "pdf-visual-diff",
    "pdf-bookmark-toc",
    "pdf-to-long-image",
    "pdf-grayscale-print",
    "pdf-repair-corrupt",
    "markdown-to-styled-pdf",
    "pdf-table-extractor",
    "pdf-page-organizer",
    "pdf-signature-pad",
    "pdf-to-images-pack",
];

#[test]
fn test_pdf_studio_state_default_initialization() {
    let state = PdfStudioState::new();

    // 1. Mặc định công cụ ban đầu là "pdf-merge-split"
    assert_eq!(state.active_tool_slug(), "pdf-merge-split");

    // 2. Chế độ workflow ban đầu phải là false
    assert!(!state.is_workflow());

    // 3. Trạng thái thực thi ban đầu phải là false
    assert!(!state.is_running());

    // 4. Danh sách tệp ban đầu rỗng
    assert_eq!(state.loaded_files_count(), 0);
    assert!(!state.has_files());
    assert_eq!(state.active_file_index.get(), None);
    assert_eq!(state.active_file_path(), None);
    assert_eq!(state.active_file_name(), None);

    // 5. Danh mục mặc định ban đầu là MergeSplit (Ghép/Tách)
    assert_eq!(state.active_category.get(), PdfCategory::MergeSplit);
}

#[test]
fn test_pdf_studio_state_with_custom_initial_tool() {
    // 1. Khởi tạo với công cụ thuộc SecurityOptimize
    let state_sec = PdfStudioState::with_initial_tool(Some("pdf-compress".to_string()));
    assert_eq!(state_sec.active_tool_slug(), "pdf-compress");
    assert_eq!(state_sec.active_category.get(), PdfCategory::SecurityOptimize);
    assert!(!state_sec.is_workflow());

    // 2. Khởi tạo với công cụ thuộc ConvertOcr
    let state_ocr = PdfStudioState::with_initial_tool(Some("vietnamese-ocr".to_string()));
    assert_eq!(state_ocr.active_tool_slug(), "vietnamese-ocr");
    assert_eq!(state_ocr.active_category.get(), PdfCategory::ConvertOcr);

    // 3. Khởi tạo với công cụ thuộc FormatPage
    let state_fmt = PdfStudioState::with_initial_tool(Some("pdf-page-number-header".to_string()));
    assert_eq!(state_fmt.active_tool_slug(), "pdf-page-number-header");
    assert_eq!(state_fmt.active_category.get(), PdfCategory::FormatPage);

    // 4. Khởi tạo với None hoặc chuỗi rỗng -> Fallback về "pdf-merge-split"
    let state_none = PdfStudioState::with_initial_tool(None);
    assert_eq!(state_none.active_tool_slug(), "pdf-merge-split");
    assert_eq!(state_none.active_category.get(), PdfCategory::MergeSplit);

    let state_empty = PdfStudioState::with_initial_tool(Some("   ".to_string()));
    assert_eq!(state_empty.active_tool_slug(), "pdf-merge-split");
}

#[test]
fn test_pdf_studio_file_management() {
    let state = PdfStudioState::new();

    // 1. Thêm một tệp đầu tiên
    state.add_file("/home/user/documents/report_q4.pdf");
    assert_eq!(state.loaded_files_count(), 1);
    assert!(state.has_files());
    assert_eq!(state.active_file_index.get(), Some(0));
    assert_eq!(state.active_file_path(), Some("/home/user/documents/report_q4.pdf".to_string()));
    assert_eq!(state.active_file_name(), Some("report_q4.pdf".to_string()));

    // 2. Thêm nhiều tệp tiếp theo
    state.add_files(vec![
        "/home/user/downloads/contract_signed.pdf".to_string(),
        "invoice_vat_2026.pdf".to_string(),
    ]);
    assert_eq!(state.loaded_files_count(), 3);
    // Active file index vẫn giữ nguyên ở tệp đầu tiên
    assert_eq!(state.active_file_index.get(), Some(0));

    // 3. Bỏ qua tệp có đường dẫn rỗng
    state.add_file("");
    state.add_file("   ");
    assert_eq!(state.loaded_files_count(), 3);

    // 4. Chọn tệp khác theo index
    state.select_file(1);
    assert_eq!(state.active_file_index.get(), Some(1));
    assert_eq!(state.active_file_name(), Some("contract_signed.pdf".to_string()));

    state.select_file(2);
    assert_eq!(state.active_file_index.get(), Some(2));
    assert_eq!(state.active_file_name(), Some("invoice_vat_2026.pdf".to_string()));

    // 5. Chọn index vượt quá giới hạn -> Không thay đổi
    state.select_file(99);
    assert_eq!(state.active_file_index.get(), Some(2));
}

#[test]
fn test_pdf_studio_remove_file_and_adjust_index() {
    let state = PdfStudioState::new();

    state.add_files(vec![
        "file_a.pdf".to_string(),
        "file_b.pdf".to_string(),
        "file_c.pdf".to_string(),
    ]);
    assert_eq!(state.loaded_files_count(), 3);

    // Trường hợp 1: Đang chọn file_c (index 2), xóa file_b (index 1)
    state.select_file(2);
    assert_eq!(state.active_file_name(), Some("file_c.pdf".to_string()));
    state.remove_file(1);
    assert_eq!(state.loaded_files_count(), 2);
    // Index của file_c giảm từ 2 xuống 1
    assert_eq!(state.active_file_index.get(), Some(1));
    assert_eq!(state.active_file_name(), Some("file_c.pdf".to_string()));

    // Trường hợp 2: Đang chọn tệp ở cuối danh sách (index 1 = file_c) và xóa chính nó
    state.remove_file(1);
    assert_eq!(state.loaded_files_count(), 1);
    // Index tự động lùi về tệp liền trước (index 0 = file_a)
    assert_eq!(state.active_file_index.get(), Some(0));
    assert_eq!(state.active_file_name(), Some("file_a.pdf".to_string()));

    // Trường hợp 3: Xóa tệp duy nhất còn lại
    state.remove_file(0);
    assert_eq!(state.loaded_files_count(), 0);
    assert_eq!(state.active_file_index.get(), None);
    assert_eq!(state.active_file_path(), None);
    assert_eq!(state.active_file_name(), None);

    // Trường hợp 4: Xóa khi danh sách rỗng hoặc chỉ số vượt quá giới hạn -> Không panic
    state.remove_file(0);
    state.remove_file(99);
    assert_eq!(state.loaded_files_count(), 0);
}

#[test]
fn test_pdf_studio_clear_files() {
    let state = PdfStudioState::new();
    state.add_files(vec!["doc1.pdf", "doc2.pdf", "doc3.pdf"]);
    assert_eq!(state.loaded_files_count(), 3);
    assert!(state.has_files());

    state.clear_files();
    assert_eq!(state.loaded_files_count(), 0);
    assert!(!state.has_files());
    assert_eq!(state.active_file_index.get(), None);
}

#[test]
fn test_pdf_studio_all_tools_exact_list() {
    // 1. Số lượng công cụ chính xác 25
    assert_eq!(PDF_STUDIO_TOOLS.len(), 25);
    assert_eq!(EXPECTED_PDF_STUDIO_SLUGS.len(), 25);

    for slug in EXPECTED_PDF_STUDIO_SLUGS {
        // 2. Mọi slug phải tồn tại trong PDF_STUDIO_TOOLS
        let found = PDF_STUDIO_TOOLS.iter().find(|t| t.slug == *slug);
        assert!(found.is_some(), "Công cụ '{}' thiếu trong PDF_STUDIO_TOOLS", slug);

        // 3. Mọi slug phải được ánh xạ đúng về MicroAppId::PdfStudio
        assert_eq!(
            tool_slug_to_micro_app(slug),
            Some(MicroAppId::PdfStudio),
            "Slug '{}' không định tuyến đúng về PdfStudio",
            slug
        );

        // 4. Mọi slug phải thuộc đúng 1 nhóm PdfCategory
        let cat = category_for_tool(slug);
        assert!(cat.is_some(), "Slug '{}' không được gán danh mục PdfCategory nào", slug);

        // 5. Mọi slug phải có icon emoji trực quan
        let icon = pdf_studio_tool_icon(slug);
        assert!(!icon.is_empty(), "Slug '{}' không có icon", slug);
    }
}

#[test]
fn test_pdf_studio_category_grouping() {
    // 1. Kiểm tra 4 danh mục của PDF Studio
    let categories = PdfCategory::ALL;
    assert_eq!(categories.len(), 4);

    let mut total_slugs = 0;
    for cat in categories {
        assert!(!cat.label().is_empty());
        assert!(!cat.icon().is_empty());
        let slugs = cat.slugs();
        assert!(!slugs.is_empty());
        total_slugs += slugs.len();

        for slug in slugs {
            assert_eq!(category_for_tool(slug), Some(cat));
        }
    }

    // 2. Tổng số công cụ trên 4 danh mục phải đúng bằng 25
    assert_eq!(total_slugs, 25);

    // 3. Kiểm tra cụ thể từng nhóm danh mục theo thiết kế
    assert_eq!(
        PdfCategory::MergeSplit.slugs(),
        &[
            "pdf-merge-split",
            "pdf-page-organizer",
            "pdf-crop-margin",
            "pdf-booklet-imposition",
            "pdf-bookmark-toc",
            "pdf-visual-diff",
        ]
    );

    assert_eq!(
        PdfCategory::SecurityOptimize.slugs(),
        &[
            "pdf-protect-sign",
            "pdf-signature-pad",
            "pdf-compress",
            "pdf-metadata-scrubber",
            "pdf-flatten-forms",
            "pdf-form-manager",
            "pdf-redaction",
            "pdf-archive-linearize",
            "pdf-pki-sign",
            "pdf-repair-corrupt",
        ]
    );

    assert_eq!(
        PdfCategory::FormatPage.slugs(),
        &[
            "pdf-page-number-header",
            "pdf-watermark-stamp",
            "pdf-grayscale-print",
        ]
    );

    assert_eq!(
        PdfCategory::ConvertOcr.slugs(),
        &[
            "vietnamese-ocr",
            "pdf-to-markdown",
            "pdf-to-long-image",
            "pdf-to-images-pack",
            "pdf-table-extractor",
            "markdown-to-styled-pdf",
        ]
    );
}

#[test]
fn test_pdf_studio_tool_and_category_switching() {
    let state = PdfStudioState::new();

    // 1. Chọn công cụ thuộc nhóm Chuyển đổi & OCR
    state.select_tool("pdf-to-markdown");
    assert_eq!(state.active_tool_slug(), "pdf-to-markdown");
    assert_eq!(state.active_category.get(), PdfCategory::ConvertOcr);
    assert!(!state.is_workflow());

    // 2. Chuyển sang danh mục FormatPage (Định dạng & Trang)
    state.select_category(PdfCategory::FormatPage);
    assert_eq!(state.active_category.get(), PdfCategory::FormatPage);
    // Tự động chọn công cụ đầu tiên của tab mới
    assert_eq!(state.active_tool_slug(), "pdf-page-number-header");

    // 3. Chọn công cụ khác cùng trong nhóm FormatPage
    state.select_tool("pdf-watermark-stamp");
    assert_eq!(state.active_tool_slug(), "pdf-watermark-stamp");
    // Nhấp lại vào tab FormatPage không đổi công cụ đã chọn
    state.select_category(PdfCategory::FormatPage);
    assert_eq!(state.active_tool_slug(), "pdf-watermark-stamp");
}

#[test]
fn test_pdf_studio_workflow_toggle() {
    let state = PdfStudioState::new();

    // 1. Mở chế độ Workflow
    state.open_workflow();
    assert!(state.is_workflow());

    // 2. Chọn một công cụ bất kỳ -> Tự động đóng chế độ Workflow
    state.select_tool("pdf-compress");
    assert!(!state.is_workflow());
    assert_eq!(state.active_tool_slug(), "pdf-compress");

    // 3. Mở lại Workflow và chọn danh mục -> Tự động đóng Workflow
    state.open_workflow();
    assert!(state.is_workflow());
    state.select_category(PdfCategory::MergeSplit);
    assert!(!state.is_workflow());

    // 4. Đóng thủ công
    state.open_workflow();
    assert!(state.is_workflow());
    state.close_workflow();
    assert!(!state.is_workflow());
}

#[test]
fn test_pdf_studio_view_mode_and_sidebar_toggle() {
    use office_utility_vn_ui::micro_apps::pdf_studio::state::PdfViewMode;

    let state = PdfStudioState::new();

    // 1. Mặc định khi khởi tạo là chế độ Visual Workspace (thay thế Hub cũ)
    assert!(state.is_visual_workspace());
    assert_eq!(state.view_mode.get(), PdfViewMode::VisualWorkspace);
    assert!(state.is_hub());
    assert!(!state.is_tool_workspace());

    // 2. Mở một công cụ cụ thể -> Chuyển sang ToolWorkspace và thu gọn Sidebar (mini)
    state.open_tool("pdf-compress");
    assert_eq!(state.view_mode.get(), PdfViewMode::ToolWorkspace);
    assert_eq!(state.active_tool_slug(), "pdf-compress");
    assert!(state.is_tool_workspace());
    assert!(!state.is_hub());
    assert!(state.sidebar_collapsed.get(), "Sidebar phải tự động thu gọn về dạng Mini khi vào công cụ");

    // 3. Toggle sidebar mở rộng và thu gọn
    state.toggle_sidebar();
    assert!(!state.sidebar_collapsed.get(), "Sau khi toggle, sidebar phải mở rộng (expanded)");
    state.toggle_sidebar();
    assert!(state.sidebar_collapsed.get(), "Sau khi toggle lần 2, sidebar phải thu gọn lại");

    // 4. Quay lại Visual Workspace (thay thế Hub cũ)
    state.open_hub();
    assert!(state.is_visual_workspace());
    assert_eq!(state.view_mode.get(), PdfViewMode::VisualWorkspace);
    assert!(state.is_hub());
    assert!(!state.is_tool_workspace());

    // 5. Mở Workflow
    state.open_workflow();
    assert_eq!(state.view_mode.get(), PdfViewMode::Workflow);
    assert!(state.is_workflow());

    // 6. Đóng Workflow
    state.close_workflow();
    assert!(!state.is_workflow());
    assert!(state.is_tool_workspace());

    // 7. Mở lại Visual Workspace tường minh
    state.open_visual_workspace();
    assert!(state.is_visual_workspace());
    assert_eq!(state.view_mode.get(), PdfViewMode::VisualWorkspace);
    assert!(state.is_hub());
}

#[test]
fn test_pdf_studio_visual_workspace_transition() {
    use office_utility_vn_ui::micro_apps::pdf_studio::state::PdfViewMode;

    // 1. Mặc định khởi tạo ở Visual Workspace mode
    let state = PdfStudioState::new();
    assert!(state.is_visual_workspace());
    assert_eq!(state.view_mode.get(), PdfViewMode::VisualWorkspace);

    // 2. Khởi tạo kèm tool ban đầu cũng ở Visual Workspace mode
    let state_with_tool = PdfStudioState::with_initial_tool(Some("pdf-merge-split".to_string()));
    assert!(state_with_tool.is_visual_workspace());
    assert_eq!(state_with_tool.view_mode.get(), PdfViewMode::VisualWorkspace);
    assert_eq!(state_with_tool.active_tool_slug(), "pdf-merge-split");

    // 3. Mở visual workspace tường minh
    state.open_visual_workspace();
    assert!(state.is_visual_workspace());
    assert_eq!(state.view_mode.get(), PdfViewMode::VisualWorkspace);
}

#[test]
fn test_pdf_studio_hub_filter() {
    use office_utility_vn_ui::micro_apps::pdf_studio::hub_view::{filter_pdf_studio_tools, matches_pdf_tool};

    let state = PdfStudioState::new();
    let tools = state.tools();
    assert_eq!(tools.len(), 25);

    // 1. Chuỗi tìm kiếm rỗng -> Khớp toàn bộ 25 công cụ
    let all_filtered = filter_pdf_studio_tools(tools, "");
    assert_eq!(all_filtered.len(), 25);

    let all_spaces = filter_pdf_studio_tools(tools, "   ");
    assert_eq!(all_spaces.len(), 25);

    // 2. Tìm kiếm có dấu tiếng Việt "nén" -> Tìm ra pdf-compress
    let nen_results = filter_pdf_studio_tools(tools, "nén");
    assert!(nen_results.iter().any(|t| t.slug == "pdf-compress"));

    // 3. Tìm kiếm không dấu tiếng Việt "nen" -> Vẫn tìm ra pdf-compress
    let unaccented_nen = filter_pdf_studio_tools(tools, "nen");
    assert!(unaccented_nen.iter().any(|t| t.slug == "pdf-compress"));

    // 4. Tìm kiếm theo slug "ocr" -> Tìm ra vietnamese-ocr
    let ocr_results = filter_pdf_studio_tools(tools, "ocr");
    assert!(ocr_results.iter().any(|t| t.slug == "vietnamese-ocr"));

    // 5. Tìm kiếm theo từ khóa "gộp" hoặc "tách"
    let gop_results = filter_pdf_studio_tools(tools, "gộp");
    assert!(gop_results.iter().any(|t| t.slug == "pdf-merge-split"));
    let merge_tool = state.find_tool("pdf-merge-split").expect("pdf-merge-split must exist");
    assert!(matches_pdf_tool(merge_tool, "gộp"));
    assert!(matches_pdf_tool(merge_tool, "tách"));
    assert!(matches_pdf_tool(merge_tool, "merge"));

    // 6. Tìm kiếm trích xuất bảng biểu / Excel
    let table_results = filter_pdf_studio_tools(tools, "bảng");
    assert!(table_results.iter().any(|t| t.slug == "pdf-table-extractor"));

    // 7. Tìm kiếm 3 công cụ mở rộng Phase 1
    let organizer_results = filter_pdf_studio_tools(tools, "sắp xếp");
    assert!(organizer_results.iter().any(|t| t.slug == "pdf-page-organizer"));
    let organizer_tool = state.find_tool("pdf-page-organizer").expect("pdf-page-organizer must exist");
    assert!(matches_pdf_tool(organizer_tool, "organizer"));
    assert!(matches_pdf_tool(organizer_tool, "đảo trang"));

    let sig_results = filter_pdf_studio_tools(tools, "ký tay");
    assert!(sig_results.iter().any(|t| t.slug == "pdf-signature-pad"));
    let sig_tool = state.find_tool("pdf-signature-pad").expect("pdf-signature-pad must exist");
    assert!(matches_pdf_tool(sig_tool, "signature"));
    assert!(matches_pdf_tool(sig_tool, "chữ ký"));

    let zip_results = filter_pdf_studio_tools(tools, "gói ảnh");
    assert!(zip_results.iter().any(|t| t.slug == "pdf-to-images-pack"));
    let pack_tool = state.find_tool("pdf-to-images-pack").expect("pdf-to-images-pack must exist");
    assert!(matches_pdf_tool(pack_tool, "zip"));
    assert!(matches_pdf_tool(pack_tool, "xuất ảnh"));

    // 8. Từ khóa hoàn toàn không tồn tại -> Trả về danh sách rỗng
    let not_found = filter_pdf_studio_tools(tools, "xyz_not_found_random_token_999");
    assert!(not_found.is_empty());
}

#[test]
fn test_pdfcraft_canvas_frame_renders() {
    let html = view! { <PdfCraftCanvasFrame /> }.to_html();
    assert!(html.contains("<iframe"));
    assert!(html.contains("id=\"pdfcraft-frame\""));
    assert!(html.contains("src=\"/pdfcraft/index.html?host=parent"));
    assert!(html.contains("allow=\"clipboard-read; clipboard-write\""));
    assert!(html.contains("w-full h-full border-none"));
    assert!(html.contains("pdfcraft-loading-skeleton"));
    assert!(html.contains("Đang tải PDF Studio"));
}

#[test]
fn test_pdf_studio_app_initial_tool_preserves_tool_workspace() {
    use office_utility_vn_ui::micro_apps::pdf_studio::init_pdf_studio_app_state;
    use office_utility_vn_ui::micro_apps::pdf_studio::PdfViewMode;

    let state = PdfStudioState::new();
    let state = init_pdf_studio_app_state(Some(state), Some("pdf-to-images-pack".to_string()));
    assert_eq!(state.active_tool_slug(), "pdf-to-images-pack");
    assert_eq!(state.view_mode.get(), PdfViewMode::ToolWorkspace);

    // Khi không truyền initial_tool -> Mặc định mở VisualWorkspace
    let default_state = init_pdf_studio_app_state(None, None);
    assert_eq!(default_state.view_mode.get(), PdfViewMode::VisualWorkspace);
}

#[test]
fn test_canvas_frame_renders_fallback_action() {
    let html = view! { <PdfCraftCanvasFrame /> }.to_html();
    assert!(html.contains("pdfcraft-fallback-btn"));
    assert!(html.contains("Chuyển sang Giao diện Công cụ"));
}

#[test]
fn test_bridge_protocol() {
    // 1. Test deserialization of incoming messages from PDFCraft
    // 1a. pdfcraft:ready
    let ready_json = r#"{"type":"pdfcraft:ready","version":"0.2.0"}"#;
    let msg: IncomingBridgeMessage = serde_json::from_str(ready_json).expect("deserialize ready message");
    match &msg {
        IncomingBridgeMessage::Ready { version } => assert_eq!(version, "0.2.0"),
        _ => panic!("Expected Ready variant, got {:?}", msg),
    }

    // 1b. pdfcraft:dirty
    let dirty_json = r#"{"type":"pdfcraft:dirty","dirty":true}"#;
    let msg: IncomingBridgeMessage = serde_json::from_str(dirty_json).expect("deserialize dirty message");
    match &msg {
        IncomingBridgeMessage::Dirty { dirty } => assert!(*dirty),
        _ => panic!("Expected Dirty variant, got {:?}", msg),
    }

    // 1c. pdfcraft:save
    let save_json = r#"{"type":"pdfcraft:save","name":"bao_cao_tai_chinh.pdf","saveAs":false,"bytes":[37,80,68,70]}"#;
    let msg: IncomingBridgeMessage = serde_json::from_str(save_json).expect("deserialize save message");
    match &msg {
        IncomingBridgeMessage::Save { name, save_as, bytes } => {
            assert_eq!(name, "bao_cao_tai_chinh.pdf");
            assert!(!*save_as);
            assert_eq!(bytes, &[37, 80, 68, 70]);
        }
        _ => panic!("Expected Save variant, got {:?}", msg),
    }

    // 1d. pdfcraft:failed
    let failed_json = r#"{"type":"pdfcraft:failed","error":"WebGL not supported"}"#;
    let msg: IncomingBridgeMessage = serde_json::from_str(failed_json).expect("deserialize failed message");
    match &msg {
        IncomingBridgeMessage::Failed { error } => assert_eq!(error, "WebGL not supported"),
        _ => panic!("Expected Failed variant, got {:?}", msg),
    }

    // 2. Test serialization of outgoing messages to PDFCraft
    // 2a. pdfcraft:open
    let open_msg = OutgoingBridgeMessage::Open {
        name: "hop_dong_kinh_te.pdf".to_string(),
        bytes: vec![37, 80, 68, 70, 45, 49, 46, 55],
    };
    let open_json = serde_json::to_string(&open_msg).expect("serialize open message");
    assert!(open_json.contains(r#""type":"pdfcraft:open""#));
    assert!(open_json.contains(r#""name":"hop_dong_kinh_te.pdf""#));
    assert!(open_json.contains(r#""bytes":[37,80,68,70,45,49,46,55]"#));

    // Deserialization roundtrip for open message
    let parsed_open: OutgoingBridgeMessage = serde_json::from_str(&open_json).expect("roundtrip open message");
    assert_eq!(parsed_open, open_msg);

    // 3. Test state integration with bridge messages
    let state = PdfStudioState::new();
    assert!(!state.is_canvas_ready.get());
    assert!(!state.has_unsaved_changes.get());

    // When ready event arrives -> is_canvas_ready becomes true
    state.handle_bridge_message(&IncomingBridgeMessage::Ready {
        version: "0.2.0".to_string(),
    });
    assert!(state.is_canvas_ready.get());

    // When dirty(true) event arrives -> has_unsaved_changes becomes true
    state.handle_bridge_message(&IncomingBridgeMessage::Dirty { dirty: true });
    assert!(state.has_unsaved_changes.get());

    // When dirty(false) event arrives -> has_unsaved_changes becomes false
    state.handle_bridge_message(&IncomingBridgeMessage::Dirty { dirty: false });
    assert!(!state.has_unsaved_changes.get());

    // When save event arrives -> has_unsaved_changes is cleared
    state.handle_bridge_message(&IncomingBridgeMessage::Dirty { dirty: true });
    assert!(state.has_unsaved_changes.get());
    state.handle_bridge_message(&IncomingBridgeMessage::Save {
        name: "test.pdf".to_string(),
        save_as: false,
        bytes: vec![],
    });
    assert!(!state.has_unsaved_changes.get());

    // 4. Test PdfCraftBridge callback dispatching
    let mut bridge = PdfCraftBridge::new("pdfcraft-frame");
    let ready_dispatched = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ready_clone = ready_dispatched.clone();
    bridge.on_ready(move |_ver| {
        ready_clone.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    let save_dispatched = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let save_clone = save_dispatched.clone();
    bridge.on_save(move |_name, _save_as, _bytes| {
        save_clone.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    bridge.dispatch_incoming(&IncomingBridgeMessage::Ready {
        version: "0.2.0".to_string(),
    });
    assert!(ready_dispatched.load(std::sync::atomic::Ordering::Relaxed));

    bridge.dispatch_incoming(&IncomingBridgeMessage::Save {
        name: "test.pdf".to_string(),
        save_as: false,
        bytes: vec![1, 2, 3],
    });
    assert!(save_dispatched.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn test_vn_extension_ribbon_renders() {
    let state = PdfStudioState::new();
    let html = view! { <PdfVnExtensionRibbon state=state /> }.to_html();

    // 1. Kiểm tra container của ribbon
    assert!(html.contains("pdf-vn-extension-ribbon"));

    // 2. Kiểm tra đầy đủ 6 nút bấm công cụ độc bản Việt Nam
    // 2.1. 🇻🇳 Nhận dạng OCR tiếng Việt (vietnamese-ocr)
    assert!(html.contains("vietnamese-ocr"));
    assert!(html.contains("🇻🇳"));
    assert!(html.contains("Nhận dạng OCR tiếng Việt"));

    // 2.2. 📊 Xuất bảng biểu sang Excel (pdf-table-extractor)
    assert!(html.contains("pdf-table-extractor"));
    assert!(html.contains("📊"));
    assert!(html.contains("Xuất bảng biểu sang Excel"));

    // 2.3. 📜 Ghép ảnh dài Infographic (pdf-to-long-image)
    assert!(html.contains("pdf-to-long-image"));
    assert!(html.contains("📜"));
    assert!(html.contains("Ghép ảnh dài Infographic"));

    // 2.4. 📖 In sách đóng kim Booklet (pdf-booklet-imposition)
    assert!(html.contains("pdf-booklet-imposition"));
    assert!(html.contains("📖"));
    assert!(html.contains("In sách đóng kim Booklet"));

    // 2.5. 🖨️ In tiết kiệm mực Grayscale (pdf-grayscale-print)
    assert!(html.contains("pdf-grayscale-print"));
    assert!(html.contains("🖨️"));
    assert!(html.contains("In tiết kiệm mực Grayscale"));

    // 2.6. 🛠️ Khôi phục PDF lỗi (pdf-repair-corrupt)
    assert!(html.contains("pdf-repair-corrupt"));
    assert!(html.contains("🛠️"));
    assert!(html.contains("Khôi phục PDF lỗi"));
}

#[test]
fn test_read_file_bytes_zero_cloud_rejection() {
    use std::future::Future;

    let mut fut_http = Box::pin(office_utility_vn_ui::micro_apps::pdf_studio::bridge::read_file_bytes(
        "http://evil.com/remote.pdf",
    ));
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    let res_http = match fut_http.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(res) => res,
        std::task::Poll::Pending => panic!("Expected Ready"),
    };
    assert!(res_http.is_err());
    assert!(res_http.unwrap_err().contains("NFR-SEC-01"));

    let mut fut_https = Box::pin(office_utility_vn_ui::micro_apps::pdf_studio::bridge::read_file_bytes(
        "https://evil.com/remote.pdf",
    ));
    let res_https = match fut_https.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(res) => res,
        std::task::Poll::Pending => panic!("Expected Ready"),
    };
    assert!(res_https.is_err());
    assert!(res_https.unwrap_err().contains("NFR-SEC-01"));
}
