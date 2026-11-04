use leptos::prelude::*;
use office_utility_vn_ui::micro_apps::pdf_studio::state::{category_for_tool, PdfCategory, PdfStudioState};
use office_utility_vn_ui::micro_apps::pdf_studio::toolbar::pdf_studio_tool_icon;
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
    // 1. Số lượng công cụ chính xác 22
    assert_eq!(PDF_STUDIO_TOOLS.len(), 22);
    assert_eq!(EXPECTED_PDF_STUDIO_SLUGS.len(), 22);

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

    // 2. Tổng số công cụ trên 4 danh mục phải đúng bằng 22
    assert_eq!(total_slugs, 22);

    // 3. Kiểm tra cụ thể từng nhóm danh mục theo thiết kế
    assert_eq!(
        PdfCategory::MergeSplit.slugs(),
        &[
            "pdf-merge-split",
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

    // 1. Mặc định khi khởi tạo là chế độ Hub (Portal View)
    assert_eq!(state.view_mode.get(), PdfViewMode::Hub);
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

    // 4. Quay lại Hub
    state.open_hub();
    assert_eq!(state.view_mode.get(), PdfViewMode::Hub);
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

    // 7. Mở lại Hub
    state.open_hub();
    assert_eq!(state.view_mode.get(), PdfViewMode::Hub);
    assert!(state.is_hub());
}

#[test]
fn test_pdf_studio_hub_filter() {
    use office_utility_vn_ui::micro_apps::pdf_studio::hub_view::{filter_pdf_studio_tools, matches_pdf_tool};

    let state = PdfStudioState::new();
    let tools = state.tools();
    assert_eq!(tools.len(), 22);

    // 1. Chuỗi tìm kiếm rỗng -> Khớp toàn bộ 22 công cụ
    let all_filtered = filter_pdf_studio_tools(tools, "");
    assert_eq!(all_filtered.len(), 22);

    let all_spaces = filter_pdf_studio_tools(tools, "   ");
    assert_eq!(all_spaces.len(), 22);

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

    // 7. Từ khóa hoàn toàn không tồn tại -> Trả về danh sách rỗng
    let not_found = filter_pdf_studio_tools(tools, "xyz_not_found_random_token_999");
    assert!(not_found.is_empty());
}
