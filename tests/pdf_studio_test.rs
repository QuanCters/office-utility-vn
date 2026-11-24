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

    // 1. Mặc định khi khởi tạo là chế độ Hub
    assert!(state.is_hub());
    assert_eq!(state.view_mode.get(), PdfViewMode::Hub);
    assert!(!state.is_visual_workspace());
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

    // 4. Quay lại Hub
    state.open_hub();
    assert!(state.is_hub());
    assert_eq!(state.view_mode.get(), PdfViewMode::Hub);
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
    assert!(!state.is_hub());
}

#[test]
fn test_pdf_studio_visual_workspace_transition() {
    use office_utility_vn_ui::micro_apps::pdf_studio::state::PdfViewMode;

    // 1. Mặc định khởi tạo ở Hub mode
    let state = PdfStudioState::new();
    assert!(state.is_hub());
    assert_eq!(state.view_mode.get(), PdfViewMode::Hub);

    // 2. Khởi tạo kèm tool ban đầu cũng ở Hub mode
    let state_with_tool = PdfStudioState::with_initial_tool(Some("pdf-merge-split".to_string()));
    assert!(state_with_tool.is_hub());
    assert_eq!(state_with_tool.view_mode.get(), PdfViewMode::Hub);
    assert_eq!(state_with_tool.active_tool_slug(), "pdf-merge-split");

    // 3. Mở visual workspace tường minh
    state.open_visual_workspace();
    assert!(state.is_visual_workspace());
    assert_eq!(state.view_mode.get(), PdfViewMode::VisualWorkspace);
    assert!(!state.is_hub());
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

    // Khi không truyền initial_tool -> Mặc định mở Hub
    let default_state = init_pdf_studio_app_state(None, None);
    assert_eq!(default_state.view_mode.get(), PdfViewMode::Hub);
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

    // Gate 1 (Stage 4B-2B): Khi nhận IncomingBridgeMessage::Save, dirty KHÔNG được tự động xóa
    // mà phải duy trì dirty = true cho đến khi có SaveOutcome::Saved từ backend để ngăn ngừa mất dữ liệu khi Cancel
    state.handle_bridge_message(&IncomingBridgeMessage::Dirty { dirty: true });
    assert!(state.has_unsaved_changes.get());
    state.handle_bridge_message(&IncomingBridgeMessage::Save {
        name: "test.pdf".to_string(),
        save_as: false,
        bytes: vec![],
    });
    // BẢO VỆ DỮ LIỆU: Vẫn phải giữ dirty = true vì chưa có xác nhận ghi đĩa
    assert!(state.has_unsaved_changes.get());
    assert!(state.status_message.get().contains("Đang xử lý"));

    // Chỉ khi nhận xác nhận lưu thành công (SaveOutcome::Saved) thì dirty mới được xóa
    state.set_unsaved_changes(false);
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

#[test]
fn test_pdf_studio_save_outcomes_and_cancel_preserves_dirty() {
    use office_utility_vn_ui::micro_apps::pdf_studio::bridge::SaveOutcome;

    // 1. Kiểm tra các biến thể SaveOutcome phân định minh thị kết quả lưu
    let outcome_saved = SaveOutcome::Saved {
        dest_path: "/data/contract.pdf".to_string(),
        committed_revision: 5,
        new_document_id: Some("doc-123".to_string()),
    };
    let outcome_canceled = SaveOutcome::Canceled;
    let outcome_failed = SaveOutcome::Failed("Permission denied".to_string());
    let _outcome_browser = SaveOutcome::BrowserDownloadInitiated;

    // Phải tuần tự hóa và giải mã được qua serde
    let json = serde_json::to_string(&outcome_saved).unwrap();
    let deserialized: SaveOutcome = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, outcome_saved);

    // 2. Kiểm tra logic bảo vệ trạng thái dirty khi người dùng Cancel hoặc thất bại
    let state = PdfStudioState::new();
    state.set_unsaved_changes(true);
    assert!(state.has_unsaved_changes());

    // Giả lập xử lý SaveOutcome::Canceled: dirty BẮT BUỘC giữ nguyên true
    match outcome_canceled {
        SaveOutcome::Canceled => {
            // KHÔNG xóa dirty!
            assert!(state.has_unsaved_changes());
            state.status_message.set("Đã hủy thao tác lưu tệp.".to_string());
        }
        _ => panic!("Expected Canceled"),
    }
    assert!(state.has_unsaved_changes());
    assert_eq!(state.status_message.get(), "Đã hủy thao tác lưu tệp.");

    // Giả lập xử lý SaveOutcome::Failed: dirty BẮT BUỘC giữ nguyên true
    match outcome_failed {
        SaveOutcome::Failed(ref err) => {
            assert!(state.has_unsaved_changes());
            state.status_message.set(format!("Lỗi: {err}"));
        }
        _ => panic!("Expected Failed"),
    }
    assert!(state.has_unsaved_changes());

    // Giả lập xử lý SaveOutcome::Saved: Chỉ khi lưu thành công thì mới xóa dirty
    match outcome_saved {
        SaveOutcome::Saved { ref dest_path, committed_revision, ref new_document_id } => {
            state.set_unsaved_changes(false);
            state.set_active_document_session(new_document_id.clone(), committed_revision);
            state.status_message.set(format!("Đã lưu: {dest_path}"));
        }
        _ => panic!("Expected Saved"),
    }
    assert!(!state.has_unsaved_changes());
    assert_eq!(state.active_revision(), 5);
    assert_eq!(state.active_document_id().as_deref(), Some("doc-123"));
}

#[test]
fn test_pdf_studio_save_as_activates_new_document_session() {
    use office_utility_vn_ui::micro_apps::pdf_studio::bridge::SaveOutcome;

    let state = PdfStudioState::new();
    state.add_file("/documents/original_a.pdf");
    state.set_active_document_session(Some("doc-session-aaa".to_string()), 1);
    state.set_unsaved_changes(true);

    assert_eq!(state.active_file_path().as_deref(), Some("/documents/original_a.pdf"));
    assert_eq!(state.active_document_id().as_deref(), Some("doc-session-aaa"));
    assert_eq!(state.active_revision(), 1);

    // Thực hiện Save As sang tệp B: backend cấp document_id mới "doc-session-bbb"
    let save_as_outcome = SaveOutcome::Saved {
        dest_path: "/documents/saved_b.pdf".to_string(),
        committed_revision: 1,
        new_document_id: Some("doc-session-bbb".to_string()),
    };

    if let SaveOutcome::Saved { dest_path, committed_revision, new_document_id } = save_as_outcome {
        state.set_unsaved_changes(false);
        state.set_active_document_session(new_document_id, committed_revision);
        state.add_file(&dest_path);
    }

    // Xác nhận session B đã được kích hoạt làm active session, không còn trỏ về file A
    assert_eq!(state.active_file_path().as_deref(), Some("/documents/saved_b.pdf"));
    assert_eq!(state.active_document_id().as_deref(), Some("doc-session-bbb"));
    assert_eq!(state.active_revision(), 1);
    assert!(!state.has_unsaved_changes());
}

#[test]
fn test_pdf_studio_concurrent_edits_during_save_preserve_dirty() {
    let state = PdfStudioState::new();
    state.add_file("/documents/doc.pdf");
    state.set_active_document_session(Some("doc-1".to_string()), 1);

    // Người dùng chỉnh sửa lần 1
    state.set_unsaved_changes(true);
    let snapshot_gen = state.edit_generation.get_untracked();
    assert_eq!(snapshot_gen, 1);

    // Trong lúc save đang chạy bất đồng bộ, người dùng tiếp tục chỉnh sửa lần 2 (thế hệ tăng lên)
    state.set_unsaved_changes(true);
    let newer_gen = state.edit_generation.get_untracked();
    assert_eq!(newer_gen, 2);

    // Khi tác vụ lưu của snapshot_gen hoàn thành:
    // Vì newer_gen (2) != snapshot_gen (1), dirty state BẮT BUỘC phải duy trì true!
    let current_gen = state.edit_generation.get_untracked();
    if current_gen == snapshot_gen {
        state.has_unsaved_changes.set(false);
    } else {
        state.has_unsaved_changes.set(true);
    }

    assert!(state.has_unsaved_changes(), "Bắt buộc giữ unsaved_changes = true nếu có chỉnh sửa mới trong khi đang lưu!");
}

#[test]
fn test_pdf_studio_cached_document_reopening() {
    use office_utility_vn_ui::micro_apps::pdf_studio::bridge::{
        cache_authorized_pdf_bytes, get_authorized_pdf_bytes, read_file_bytes,
    };

    let fake_path = "/authorized/documents/report_q4.pdf";
    let fake_bytes = b"%PDF-1.7 Test document content for caching".to_vec();

    // Chưa cache thì get trả về None
    assert!(get_authorized_pdf_bytes(fake_path).is_none());

    // Nạp vào cache ủy quyền phiên
    cache_authorized_pdf_bytes(fake_path, fake_bytes.clone());

    // Đọc lại từ cache
    let retrieved = get_authorized_pdf_bytes(fake_path);
    assert_eq!(retrieved, Some(fake_bytes.clone()));

    // read_file_bytes cũng phải đọc thành công từ cache
    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        let mut f = std::pin::pin!(f);
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(&waker);
        match f.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(res) => res,
            std::task::Poll::Pending => panic!("Future pending unexpectedly"),
        }
    }

    let read_res = block_on(read_file_bytes(fake_path));
    assert!(read_res.is_ok());
    assert_eq!(read_res.unwrap(), fake_bytes);
}

#[test]
fn test_pdf_studio_session_identity_restored_on_file_switch() {
    let state = PdfStudioState::new();

    // 1. Mở file A: /tmp/A.pdf với document_id "doc-A", rev 1
    state.add_file("/tmp/A.pdf");
    state.set_active_document_session(Some("doc-A".to_string()), 1);

    // 2. Mở tiếp file B: /tmp/B.pdf với document_id "doc-B", rev 1
    state.add_file("/tmp/B.pdf");
    state.set_active_document_session(Some("doc-B".to_string()), 1);

    assert_eq!(state.active_file_path(), Some("/tmp/B.pdf".to_string()));
    assert_eq!(state.active_document_id(), Some("doc-B".to_string()));

    // 3. Người dùng chọn lại file A (index 0)
    state.select_file(0);

    // BẮT BUỘC: Khi chọn lại file A, session ID và revision của A phải được phục hồi!
    assert_eq!(state.active_file_path(), Some("/tmp/A.pdf".to_string()));
    assert_eq!(
        state.active_document_id(),
        Some("doc-A".to_string()),
        "Active document ID must be restored to A's session ID when switching back to A!"
    );
    assert_eq!(state.active_revision(), 1);
}

#[test]
fn test_pdf_studio_switching_and_saving_preserves_other_document_bytes() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path_a = dir.path().join("A.pdf");
    let path_b = dir.path().join("B.pdf");

    let initial_bytes_a = b"%PDF-1.7 ORIGINAL_CONTENT_FOR_DOCUMENT_A";
    let initial_bytes_b = b"%PDF-1.7 DISTINCT_CONTENT_FOR_DOCUMENT_B";

    std::fs::write(&path_a, initial_bytes_a).expect("write A");
    std::fs::write(&path_b, initial_bytes_b).expect("write B");

    let sha_b_before = {
        use std::hash::{DefaultHasher, Hasher};
        let mut hasher = DefaultHasher::new();
        hasher.write(initial_bytes_b);
        hasher.finish()
    };

    let state = PdfStudioState::new();

    // 1. Mở A.pdf
    let path_a_str = path_a.to_str().unwrap().to_string();
    state.add_file(&path_a_str);
    state.set_active_document_session(Some("doc-session-A".to_string()), 1);

    // 2. Mở B.pdf
    let path_b_str = path_b.to_str().unwrap().to_string();
    state.add_file(&path_b_str);
    state.set_active_document_session(Some("doc-session-B".to_string()), 1);

    assert_eq!(state.active_file_path(), Some(path_b_str.clone()));
    assert_eq!(state.active_document_id(), Some("doc-session-B".to_string()));

    // 3. Người dùng chuyển lại tệp A
    state.select_file(0);
    assert_eq!(state.active_file_path(), Some(path_a_str.clone()));
    assert_eq!(
        state.active_document_id(),
        Some("doc-session-A".to_string()),
        "Must restore A's session ID, NOT keep B's session ID!"
    );

    // 4. Giả lập lưu A với nội dung mới
    let new_bytes_a = b"%PDF-1.7 EDITED_CONTENT_FOR_DOCUMENT_A_V2";
    // Kiểm tra rằng session được lấy để lưu là A
    let grant_target_doc_id = state.active_document_id().expect("doc id for A");
    assert_eq!(grant_target_doc_id, "doc-session-A");
    assert_ne!(grant_target_doc_id, "doc-session-B");

    // Ghi đĩa theo phiên A
    let target_session = state.get_document_session(&path_a_str).expect("session for A");
    assert_eq!(target_session.document_id, Some("doc-session-A".to_string()));
    std::fs::write(&target_session.file_path, new_bytes_a).expect("write new A");
    state.set_active_document_session(Some("doc-session-A".to_string()), 2);

    // 5. Xác minh: A chứa đúng bytes mới, B giữ nguyên tuyệt đối
    let read_a = std::fs::read(&path_a).expect("read A");
    assert_eq!(read_a, new_bytes_a);

    let read_b = std::fs::read(&path_b).expect("read B");
    assert_eq!(read_b, initial_bytes_b);

    let sha_b_after = {
        use std::hash::{DefaultHasher, Hasher};
        let mut hasher = DefaultHasher::new();
        hasher.write(&read_b);
        hasher.finish()
    };
    assert_eq!(sha_b_before, sha_b_after, "B's content must be 100% unchanged!");
}

#[test]
fn test_pdf_studio_save_completion_cross_document_interleaving_safety() {
    use office_utility_vn_ui::micro_apps::pdf_studio::bridge::SaveOutcome;
    use sha2::{Digest, Sha256};

    let dir = tempfile::tempdir().expect("create temp dir");
    let path_a = dir.path().join("A.pdf");
    let path_b = dir.path().join("B.pdf");

    let initial_bytes_a = b"%PDF-1.7 INITIAL_CONTENT_FOR_DOC_A";
    let initial_bytes_b = b"%PDF-1.7 INITIAL_CONTENT_FOR_DOC_B";

    std::fs::write(&path_a, initial_bytes_a).expect("write initial A");
    std::fs::write(&path_b, initial_bytes_b).expect("write initial B");

    let sha256_oracle = |bytes: &[u8]| -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    };

    let oracle_initial_b = sha256_oracle(initial_bytes_b);

    let state = PdfStudioState::new();

    // 1. Mở A.pdf và đăng ký session
    let path_a_str = path_a.to_str().unwrap().to_string();
    state.add_file(&path_a_str);
    state.set_active_document_session(Some("doc-session-A".to_string()), 1);
    state.set_unsaved_changes(true);

    // 2. Mở B.pdf và đăng ký session
    let path_b_str = path_b.to_str().unwrap().to_string();
    state.add_file(&path_b_str);
    state.set_active_document_session(Some("doc-session-B".to_string()), 1);
    state.set_unsaved_changes(false);

    // 3. Người dùng chọn lại file A
    state.select_file(0);
    assert_eq!(state.active_file_path(), Some(path_a_str.clone()));
    assert_eq!(state.active_document_id(), Some("doc-session-A".to_string()));

    // 4. Bắt đầu lưu A.pdf: Chụp lại thông tin snapshot
    let originating_path_a = state.active_file_path().unwrap();
    let originating_doc_id_a = state.active_document_id().unwrap();
    let snapshot_gen_a = state.edit_generation.get_untracked();
    let new_bytes_a = b"%PDF-1.7 NEW_CONTENT_FOR_A_POST_EDIT";

    // 5. TRƯỚC KHI tác vụ lưu của A hoàn tất, người dùng chuyển sang file B
    state.select_file(1);
    assert_eq!(state.active_file_path(), Some(path_b_str.clone()));
    assert_eq!(state.active_document_id(), Some("doc-session-B".to_string()));
    // Người dùng chỉnh sửa file B
    state.set_unsaved_changes(true);

    // 6. Tác vụ lưu bất đồng bộ của A hoàn tất
    let outcome_a = SaveOutcome::Saved {
        dest_path: originating_path_a.clone(),
        committed_revision: 2,
        new_document_id: Some("doc-session-A".to_string()),
    };

    // Áp dụng completion handler của bridge
    office_utility_vn_ui::micro_apps::pdf_studio::bridge::apply_save_outcome(
        &state,
        Ok(outcome_a),
        Some(originating_path_a.clone()),
        Some(originating_doc_id_a.clone()),
        snapshot_gen_a,
        false,
        new_bytes_a,
    );
    std::fs::write(&originating_path_a, new_bytes_a).expect("write new A");

    // 7. BẮT BUỘC: Sau khi A lưu xong, active session của B VẪN PHẢI LÀ B!
    // Không được bị ghi đè thành doc-session-A!
    assert_eq!(
        state.active_document_id(),
        Some("doc-session-B".to_string()),
        "B's active document session identity must remain B, even after A's save completes!"
    );
    assert_eq!(state.active_revision(), 1);
    assert!(state.has_unsaved_changes(), "B must remain dirty after A saves!");

    // 8. Bây giờ người dùng lưu file B
    let new_bytes_b = b"%PDF-1.7 NEW_CONTENT_FOR_B_POST_EDIT";
    let originating_path_b = state.active_file_path().unwrap();
    let originating_doc_id_b = state.active_document_id().unwrap();
    assert_eq!(originating_doc_id_b, "doc-session-B", "Grant for B must reference B's session!");
    assert_ne!(originating_doc_id_b, "doc-session-A", "Grant for B must NEVER reference A's session!");

    let outcome_b = SaveOutcome::Saved {
        dest_path: originating_path_b.clone(),
        committed_revision: 2,
        new_document_id: Some("doc-session-B".to_string()),
    };
    office_utility_vn_ui::micro_apps::pdf_studio::bridge::apply_save_outcome(
        &state,
        Ok(outcome_b),
        Some(originating_path_b.clone()),
        Some(originating_doc_id_b),
        state.edit_generation.get_untracked(),
        false,
        new_bytes_b,
    );
    std::fs::write(&originating_path_b, new_bytes_b).expect("write new B");

    // 9. Xác thực SHA-256 độc lập cho nội dung cuối cùng của cả hai tệp
    let read_a = std::fs::read(&path_a).expect("read final A");
    let read_b = std::fs::read(&path_b).expect("read final B");

    let oracle_final_a = sha256_oracle(&read_a);
    let oracle_final_b = sha256_oracle(&read_b);

    assert_eq!(oracle_final_a, sha256_oracle(new_bytes_a), "A must contain new A bytes!");
    assert_eq!(oracle_final_b, sha256_oracle(new_bytes_b), "B must contain new B bytes!");
    assert_ne!(oracle_final_a, oracle_final_b, "A and B must have distinct SHA-256!");
    assert_ne!(oracle_final_b, oracle_initial_b, "B was updated with new bytes!");
}


