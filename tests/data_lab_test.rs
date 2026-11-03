use leptos::prelude::*;
use office_utility_vn_ui::micro_apps::data_lab::state::{category_for_tool, DataLabCategory, DataLabState};
use office_utility_vn_ui::micro_apps::data_lab::toolbar::data_lab_tool_icon;
use office_utility_vn_ui::micro_apps::{tool_slug_to_micro_app, MicroAppId, DATA_LAB_TOOLS};

const EXPECTED_DATA_LAB_SLUGS: &[&str] = &[
    "excel-cleaner-normalizer",
    "diff-checker-text-sheets",
    "data-structure-converter",
    "excel-split-by-column",
    "excel-multi-merger",
    "excel-visual-joiner",
    "excel-sheet-unlocker",
    "list-set-operations",
    "text-pii-anonymizer",
    "sql-formatter-linter",
    "base64-file-codec",
    "docx-template-mailmerge",
    "universal-file-converter-hub",
];

#[test]
fn test_data_lab_state_default_initialization() {
    let state = DataLabState::new();

    // 1. Mặc định công cụ ban đầu là "excel-cleaner-normalizer"
    assert_eq!(state.active_tool_slug(), "excel-cleaner-normalizer");

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
    assert_eq!(state.active_file_extension(), None);

    // 5. Danh mục mặc định ban đầu là Spreadsheet (Bảng tính & Excel)
    assert_eq!(state.active_category.get(), DataLabCategory::Spreadsheet);

    // 6. Trang tính mặc định là Sheet1
    assert_eq!(state.active_sheet_name(), "Sheet1");
}

#[test]
fn test_data_lab_state_with_custom_initial_tool() {
    // 1. Khởi tạo với công cụ thuộc TransformMerge
    let state_conv = DataLabState::with_initial_tool(Some("data-structure-converter".to_string()));
    assert_eq!(state_conv.active_tool_slug(), "data-structure-converter");
    assert_eq!(state_conv.active_category.get(), DataLabCategory::TransformMerge);
    assert!(!state_conv.is_workflow());

    // 2. Khởi tạo với docx-template-mailmerge (được chuyển giao từ doc-suite)
    let state_mailmerge = DataLabState::with_initial_tool(Some("docx-template-mailmerge".to_string()));
    assert_eq!(state_mailmerge.active_tool_slug(), "docx-template-mailmerge");
    assert_eq!(state_mailmerge.active_category.get(), DataLabCategory::TransformMerge);

    // 3. Khởi tạo với universal-file-converter-hub (được chuyển giao từ sys-hub)
    let state_hub = DataLabState::with_initial_tool(Some("universal-file-converter-hub".to_string()));
    assert_eq!(state_hub.active_tool_slug(), "universal-file-converter-hub");
    assert_eq!(state_hub.active_category.get(), DataLabCategory::TransformMerge);

    // 4. Khởi tạo với công cụ thuộc TextStructure
    let state_diff = DataLabState::with_initial_tool(Some("diff-checker-text-sheets".to_string()));
    assert_eq!(state_diff.active_tool_slug(), "diff-checker-text-sheets");
    assert_eq!(state_diff.active_category.get(), DataLabCategory::TextStructure);

    let state_sql = DataLabState::with_initial_tool(Some("sql-formatter-linter".to_string()));
    assert_eq!(state_sql.active_tool_slug(), "sql-formatter-linter");
    assert_eq!(state_sql.active_category.get(), DataLabCategory::TextStructure);

    // 5. Khởi tạo với None hoặc chuỗi rỗng -> Fallback về "excel-cleaner-normalizer"
    let state_none = DataLabState::with_initial_tool(None);
    assert_eq!(state_none.active_tool_slug(), "excel-cleaner-normalizer");
    assert_eq!(state_none.active_category.get(), DataLabCategory::Spreadsheet);

    let state_empty = DataLabState::with_initial_tool(Some("   ".to_string()));
    assert_eq!(state_empty.active_tool_slug(), "excel-cleaner-normalizer");
}

#[test]
fn test_data_lab_13_tools_exact_list() {
    assert_eq!(
        DATA_LAB_TOOLS.len(),
        13,
        "Data Lab phải quản lý chính xác 13 công cụ dữ liệu"
    );

    let registered_slugs: Vec<&str> = DATA_LAB_TOOLS.iter().map(|t| t.slug).collect();

    for &expected_slug in EXPECTED_DATA_LAB_SLUGS {
        assert!(
            registered_slugs.contains(&expected_slug),
            "Công cụ {} phải có mặt trong DATA_LAB_TOOLS",
            expected_slug
        );
        assert_eq!(
            tool_slug_to_micro_app(expected_slug),
            Some(MicroAppId::DataLab),
            "Công cụ {} phải được route về MicroAppId::DataLab",
            expected_slug
        );
    }

    // Alias universal-converter cũng phải route về DataLab
    assert_eq!(
        tool_slug_to_micro_app("universal-converter"),
        Some(MicroAppId::DataLab)
    );
}

#[test]
fn test_data_lab_category_grouping() {
    assert_eq!(DataLabCategory::ALL.len(), 3);

    // Tổng số slugs của 3 category phải bằng đúng 13
    let total_cat_slugs: usize = DataLabCategory::ALL.iter().map(|c| c.slugs().len()).sum();
    assert_eq!(total_cat_slugs, 13);

    // Kiểm tra từng category
    assert_eq!(DataLabCategory::Spreadsheet.slugs().len(), 5);
    assert_eq!(DataLabCategory::TransformMerge.slugs().len(), 4);
    assert_eq!(DataLabCategory::TextStructure.slugs().len(), 4);

    // Kiểm tra ánh xạ category_for_tool
    assert_eq!(
        category_for_tool("excel-cleaner-normalizer"),
        Some(DataLabCategory::Spreadsheet)
    );
    assert_eq!(
        category_for_tool("excel-split-by-column"),
        Some(DataLabCategory::Spreadsheet)
    );
    assert_eq!(
        category_for_tool("excel-multi-merger"),
        Some(DataLabCategory::Spreadsheet)
    );
    assert_eq!(
        category_for_tool("excel-visual-joiner"),
        Some(DataLabCategory::Spreadsheet)
    );
    assert_eq!(
        category_for_tool("excel-sheet-unlocker"),
        Some(DataLabCategory::Spreadsheet)
    );

    assert_eq!(
        category_for_tool("data-structure-converter"),
        Some(DataLabCategory::TransformMerge)
    );
    assert_eq!(
        category_for_tool("docx-template-mailmerge"),
        Some(DataLabCategory::TransformMerge)
    );
    assert_eq!(
        category_for_tool("universal-file-converter-hub"),
        Some(DataLabCategory::TransformMerge)
    );
    assert_eq!(
        category_for_tool("universal-converter"),
        Some(DataLabCategory::TransformMerge)
    );
    assert_eq!(
        category_for_tool("base64-file-codec"),
        Some(DataLabCategory::TransformMerge)
    );

    assert_eq!(
        category_for_tool("diff-checker-text-sheets"),
        Some(DataLabCategory::TextStructure)
    );
    assert_eq!(
        category_for_tool("list-set-operations"),
        Some(DataLabCategory::TextStructure)
    );
    assert_eq!(
        category_for_tool("text-pii-anonymizer"),
        Some(DataLabCategory::TextStructure)
    );
    assert_eq!(
        category_for_tool("sql-formatter-linter"),
        Some(DataLabCategory::TextStructure)
    );

    assert_eq!(category_for_tool("unknown-tool"), None);
}

#[test]
fn test_data_lab_file_management() {
    let state = DataLabState::new();

    // 1. Thêm một tệp đơn lẻ
    state.add_file("/home/user/bang_luong_t10.xlsx");
    assert_eq!(state.loaded_files_count(), 1);
    assert!(state.has_files());
    assert_eq!(state.active_file_index.get(), Some(0));
    assert_eq!(
        state.active_file_path(),
        Some("/home/user/bang_luong_t10.xlsx".to_string())
    );
    assert_eq!(state.active_file_name(), Some("bang_luong_t10.xlsx".to_string()));
    assert_eq!(state.active_file_extension(), Some("xlsx".to_string()));

    // 2. Thêm nhiều tệp tiếp theo
    state.add_files(vec![
        "/data/customers.csv".to_string(),
        "/exports/schema.sql".to_string(),
        "contract.docx".to_string(),
    ]);
    assert_eq!(state.loaded_files_count(), 4);
    assert_eq!(state.active_file_index.get(), Some(0));

    // 3. Chọn file thứ hai
    state.select_file(1);
    assert_eq!(state.active_file_index.get(), Some(1));
    assert_eq!(state.active_file_name(), Some("customers.csv".to_string()));
    assert_eq!(state.active_file_extension(), Some("csv".to_string()));

    // 4. Bỏ qua tệp rỗng
    state.add_file("");
    state.add_file("   ");
    assert_eq!(state.loaded_files_count(), 4);
}

#[test]
fn test_data_lab_remove_file_and_adjust_index() {
    let state = DataLabState::new();
    state.add_files(vec![
        "file_a.xlsx".to_string(),
        "file_b.csv".to_string(),
        "file_c.json".to_string(),
    ]);

    // Chọn file index 1 ("file_b.csv")
    state.select_file(1);
    assert_eq!(state.active_file_index.get(), Some(1));

    // Xóa file 0 ("file_a.xlsx") -> active file phải lùi về index 0 ("file_b.csv")
    state.remove_file(0);
    assert_eq!(state.loaded_files_count(), 2);
    assert_eq!(state.active_file_index.get(), Some(0));
    assert_eq!(state.active_file_name(), Some("file_b.csv".to_string()));

    // Xóa file 1 ("file_c.json") -> active file giữ nguyên index 0
    state.remove_file(1);
    assert_eq!(state.loaded_files_count(), 1);
    assert_eq!(state.active_file_index.get(), Some(0));

    // Xóa file cuối cùng -> index trở thành None
    state.remove_file(0);
    assert_eq!(state.loaded_files_count(), 0);
    assert_eq!(state.active_file_index.get(), None);

    // Xóa khi mảng rỗng không gây panic
    state.remove_file(99);
}

#[test]
fn test_data_lab_clear_files() {
    let state = DataLabState::new();
    state.add_files(vec!["file1.xlsx".to_string(), "file2.csv".to_string()]);
    assert_eq!(state.loaded_files_count(), 2);

    state.clear_files();
    assert_eq!(state.loaded_files_count(), 0);
    assert!(!state.has_files());
    assert_eq!(state.active_file_index.get(), None);
    assert_eq!(state.active_file_path(), None);
}

#[test]
fn test_data_lab_sheet_management() {
    let state = DataLabState::new();
    assert_eq!(state.active_sheet_name(), "Sheet1");

    state.set_sheet("BaoCaoQ4");
    assert_eq!(state.active_sheet_name(), "BaoCaoQ4");

    state.select_sheet("DanhSachKhachHang");
    assert_eq!(state.active_sheet_name(), "DanhSachKhachHang");
}

#[test]
fn test_data_lab_tool_and_category_switching() {
    let state = DataLabState::new();

    // 1. Chọn tool thuộc nhóm TextStructure
    state.select_tool("diff-checker-text-sheets");
    assert_eq!(state.active_tool_slug(), "diff-checker-text-sheets");
    assert_eq!(state.active_category.get(), DataLabCategory::TextStructure);
    assert!(!state.is_workflow());

    // 2. Chuyển category sang TransformMerge -> tự động chọn công cụ đầu tiên của nhóm
    state.select_category(DataLabCategory::TransformMerge);
    assert_eq!(state.active_category.get(), DataLabCategory::TransformMerge);
    assert_eq!(state.active_tool_slug(), "data-structure-converter");

    // 3. Chọn tool base64-file-codec trong cùng nhóm
    state.select_tool("base64-file-codec");
    assert_eq!(state.active_tool_slug(), "base64-file-codec");

    // Chọn lại category TransformMerge -> tool không bị reset vì đã thuộc category đó
    state.select_category(DataLabCategory::TransformMerge);
    assert_eq!(state.active_tool_slug(), "base64-file-codec");
}

#[test]
fn test_data_lab_workflow_toggle() {
    let state = DataLabState::new();
    assert!(!state.is_workflow());

    state.open_workflow();
    assert!(state.is_workflow());

    state.close_workflow();
    assert!(!state.is_workflow());

    // Mở workflow rồi chọn tool đơn lẻ -> tự động tắt workflow
    state.open_workflow();
    assert!(state.is_workflow());
    state.select_tool("excel-multi-merger");
    assert!(!state.is_workflow());
    assert_eq!(state.active_tool_slug(), "excel-multi-merger");
}

#[test]
fn test_data_lab_tool_icon_mapping() {
    for &slug in EXPECTED_DATA_LAB_SLUGS {
        let icon = data_lab_tool_icon(slug);
        assert!(!icon.is_empty(), "Icon cho slug {} không được rỗng", slug);
        assert_ne!(icon, "❓");
    }

    assert_eq!(data_lab_tool_icon("universal-converter"), "⚡");
    assert_eq!(data_lab_tool_icon("non-existent-tool"), "📊");
}
