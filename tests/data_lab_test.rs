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

#[test]
fn test_visual_canvas_default_nodes_and_dag_structure() {
    use office_utility_vn_ui::micro_apps::data_lab::visual_canvas_workflow::{
        create_default_canvas_nodes, CanvasNodeColor, CanvasNodeKind, CanvasNodeStatus,
    };

    // 1. Mặc định khởi tạo không truyền active_file -> dùng danh_sach_nhan_vien.xlsx
    let default_nodes = create_default_canvas_nodes(None);
    assert_eq!(default_nodes.len(), 5, "Bảng vẽ DAG ban đầu phải chứa chính xác 5 Node");

    // Node 1: Input Source
    let n1 = &default_nodes[0];
    assert_eq!(n1.id, "node-input");
    assert_eq!(n1.name, "Tệp Dữ liệu Đầu vào");
    assert_eq!(n1.icon, "📥");
    assert_eq!(n1.kind, CanvasNodeKind::InputSource);
    assert_eq!(n1.color, CanvasNodeColor::Blue);
    assert_eq!(n1.status, CanvasNodeStatus::Completed);
    assert!(n1.summary.contains("danh_sach_nhan_vien.xlsx"));
    assert!(n1.summary.contains("1,200 dòng"));

    // Node 2: Transform Node 1 (Clean)
    let n2 = &default_nodes[1];
    assert_eq!(n2.id, "node-transform-1");
    assert_eq!(n2.name, "Làm sạch bảng tính & Tách Họ Tên");
    assert_eq!(n2.icon, "🧹");
    assert_eq!(n2.kind, CanvasNodeKind::Transform);
    assert_eq!(n2.color, CanvasNodeColor::Green);
    assert_eq!(n2.status, CanvasNodeStatus::Completed);
    assert!(n2.summary.contains("Họ và tên"));

    // Node 3: Transform Node 2 (Fuzzy)
    let n3 = &default_nodes[2];
    assert_eq!(n3.id, "node-transform-2");
    assert_eq!(n3.name, "Gom cụm Fuzzy Cột Tỉnh Thành");
    assert_eq!(n3.icon, "✨");
    assert_eq!(n3.kind, CanvasNodeKind::Transform);
    assert_eq!(n3.color, CanvasNodeColor::Purple);
    assert_eq!(n3.status, CanvasNodeStatus::Completed);
    assert!(n3.summary.contains("Tỉnh / Thành phố"));

    // Node 4: Transform Node 3 (Split)
    let n4 = &default_nodes[3];
    assert_eq!(n4.id, "node-transform-3");
    assert_eq!(n4.name, "Tách File theo Cột Phòng Ban");
    assert_eq!(n4.icon, "🗂️");
    assert_eq!(n4.kind, CanvasNodeKind::Transform);
    assert_eq!(n4.color, CanvasNodeColor::Orange);
    assert_eq!(n4.status, CanvasNodeStatus::Running);
    assert!(n4.summary.contains("Phòng ban"));

    // Node 5: Output Destination Node
    let n5 = &default_nodes[4];
    assert_eq!(n5.id, "node-output");
    assert_eq!(n5.name, "Thư mục Xuất Kết quả");
    assert_eq!(n5.icon, "📁");
    assert_eq!(n5.kind, CanvasNodeKind::OutputDestination);
    assert_eq!(n5.color, CanvasNodeColor::Blue);
    assert_eq!(n5.status, CanvasNodeStatus::Ready);
    assert!(n5.summary.contains("/exports/data_lab_output/"));

    // 2. Khởi tạo với tệp đang chọn trong DataLabState
    let custom_nodes = create_default_canvas_nodes(Some("bang_luong_q4.csv".to_string()));
    assert!(custom_nodes[0].summary.contains("bang_luong_q4.csv"));
}

#[test]
fn test_visual_canvas_tool_color_mapping_and_statuses() {
    use office_utility_vn_ui::micro_apps::data_lab::visual_canvas_workflow::{
        color_for_slug, CanvasNodeColor, CanvasNodeStatus,
    };

    // Kiểm tra ánh xạ màu sắc theo nhóm nghiệp vụ
    // Xanh lá (Clean)
    assert_eq!(color_for_slug("excel-cleaner-normalizer"), CanvasNodeColor::Green);
    assert_eq!(color_for_slug("excel-sheet-unlocker"), CanvasNodeColor::Green);

    // Cam (Split / Merge / Join / Set)
    assert_eq!(color_for_slug("excel-split-by-column"), CanvasNodeColor::Orange);
    assert_eq!(color_for_slug("excel-multi-merger"), CanvasNodeColor::Orange);
    assert_eq!(color_for_slug("excel-visual-joiner"), CanvasNodeColor::Orange);
    assert_eq!(color_for_slug("diff-checker-text-sheets"), CanvasNodeColor::Orange);
    assert_eq!(color_for_slug("list-set-operations"), CanvasNodeColor::Orange);

    // Tím (Fuzzy / Transform / Mailmerge / Codec)
    assert_eq!(color_for_slug("data-structure-converter"), CanvasNodeColor::Purple);
    assert_eq!(color_for_slug("docx-template-mailmerge"), CanvasNodeColor::Purple);
    assert_eq!(color_for_slug("text-pii-anonymizer"), CanvasNodeColor::Purple);
    assert_eq!(color_for_slug("sql-formatter-linter"), CanvasNodeColor::Purple);
    assert_eq!(color_for_slug("base64-file-codec"), CanvasNodeColor::Purple);

    // Kiểm tra nhãn và icon của status
    assert_eq!(CanvasNodeStatus::Ready.label(), "Sẵn sàng");
    assert_eq!(CanvasNodeStatus::Ready.icon(), "⏱");

    assert_eq!(CanvasNodeStatus::Running.label(), "Đang chạy...");
    assert_eq!(CanvasNodeStatus::Running.icon(), "⚡");

    assert_eq!(CanvasNodeStatus::Completed.label(), "Hoàn thành");
    assert_eq!(CanvasNodeStatus::Completed.icon(), "✓");

    assert_eq!(CanvasNodeStatus::Error("err".to_string()).label(), "Lỗi");
    assert_eq!(CanvasNodeStatus::Error("err".to_string()).icon(), "✕");
}

// ==============================================================================
// TEST SUITE: SPREADSHEET GRID & PROFILING FACET VIEW (OpenRefine Parity)
// ==============================================================================

#[test]
fn test_spreadsheet_grid_demo_table_initialization() {
    use office_utility_vn_ui::micro_apps::data_lab::spreadsheet_grid_view::{
        create_demo_table, ColumnType,
    };

    let (columns, rows) = create_demo_table();

    // 1. Kiểm tra 6 cột chuẩn
    assert_eq!(columns.len(), 6);
    assert_eq!(columns[0].name, "Mã NV");
    assert_eq!(columns[0].col_type, ColumnType::Number);
    assert_eq!(columns[0].col_type.icon(), "123");

    assert_eq!(columns[1].name, "Họ và tên");
    assert_eq!(columns[1].col_type, ColumnType::Text);
    assert_eq!(columns[1].col_type.icon(), "ABC");

    assert_eq!(columns[2].name, "Ngày sinh");
    assert_eq!(columns[2].col_type, ColumnType::Date);
    assert_eq!(columns[2].col_type.icon(), "📅");

    assert_eq!(columns[3].name, "Tỉnh thành");
    assert_eq!(columns[4].name, "Phòng ban");
    assert_eq!(columns[5].name, "Lương cơ bản");

    // 2. Kiểm tra chính xác 25 dòng dữ liệu mẫu
    assert_eq!(rows.len(), 25);
    for row in &rows {
        assert_eq!(row.len(), 6, "Mỗi dòng trong bảng phải có đủ 6 ô");
    }
}

#[test]
fn test_spreadsheet_grid_column_profiling_and_metrics() {
    use office_utility_vn_ui::micro_apps::data_lab::spreadsheet_grid_view::{
        calculate_column_profile, create_demo_table,
    };

    let (_, rows) = create_demo_table();

    // Cột 0: Mã NV (100% hợp lệ, 0% rỗng)
    let prof_id = calculate_column_profile(0, &rows);
    assert_eq!(prof_id.total, 25);
    assert_eq!(prof_id.valid_count, 25);
    assert_eq!(prof_id.null_count, 0);
    assert_eq!(prof_id.valid_percentage, 100.0);
    assert_eq!(prof_id.null_percentage, 0.0);

    // Cột 3: Tỉnh thành (có 1 dòng rỗng: NV021 Nguyễn Du)
    let prof_prov = calculate_column_profile(3, &rows);
    assert_eq!(prof_prov.total, 25);
    assert_eq!(prof_prov.valid_count, 24);
    assert_eq!(prof_prov.null_count, 1);
    assert_eq!(prof_prov.valid_percentage, 96.0);
    assert_eq!(prof_prov.null_percentage, 4.0);

    // Kiểm tra danh sách phân phối tần suất top values
    assert!(!prof_prov.top_values.is_empty());
    // Hà Nội có nhiều dòng nhất
    let top_val = &prof_prov.top_values[0];
    assert_eq!(top_val.0, "Hà Nội");
    assert!(top_val.1 >= 4);
}

#[test]
fn test_spreadsheet_grid_vietnamese_name_splitting() {
    use office_utility_vn_ui::micro_apps::data_lab::spreadsheet_grid_view::split_vietnamese_full_name;

    // 1. Tên họ đơn thông thường
    let (h1, d1, t1) = split_vietnamese_full_name("Nguyễn Văn An");
    assert_eq!(h1, "Nguyễn");
    assert_eq!(d1, "Văn");
    assert_eq!(t1, "An");

    // 2. Họ kép 2 từ: Phan Huy
    let (h2, d2, t2) = split_vietnamese_full_name("Phan Huy Chú");
    assert_eq!(h2, "Phan Huy");
    assert_eq!(d2, "");
    assert_eq!(t2, "Chú");

    // 3. Họ kép 2 từ có tên đệm: Hoàng Phủ Ngọc Tường
    let (h3, d3, t3) = split_vietnamese_full_name("Hoàng Phủ Ngọc Tường");
    assert_eq!(h3, "Hoàng Phủ");
    assert_eq!(d3, "Ngọc");
    assert_eq!(t3, "Tường");

    // 4. Họ kép: Võ Đình Tú
    let (h4, d4, t4) = split_vietnamese_full_name("Võ Đình Tú");
    assert_eq!(h4, "Võ Đình");
    assert_eq!(d4, "");
    assert_eq!(t4, "Tú");

    // 5. Họ kép 4 từ: Nguyễn Phước Vĩnh Lộc
    let (h5, d5, t5) = split_vietnamese_full_name("Nguyễn Phước Vĩnh Lộc");
    assert_eq!(h5, "Nguyễn Phước");
    assert_eq!(d5, "Vĩnh");
    assert_eq!(t5, "Lộc");

    // 6. Tên chỉ có 2 từ
    let (h6, d6, t6) = split_vietnamese_full_name("Lê Lợi");
    assert_eq!(h6, "Lê");
    assert_eq!(d6, "");
    assert_eq!(t6, "Lợi");
}

#[test]
fn test_spreadsheet_grid_date_normalization_to_dd_mm_yyyy() {
    use office_utility_vn_ui::micro_apps::data_lab::spreadsheet_grid_view::normalize_date_cell;

    // YYYY-MM-DD -> DD/MM/YYYY
    assert_eq!(normalize_date_cell("1990-05-15"), "15/05/1990");
    assert_eq!(normalize_date_cell("1985-12-01"), "01/12/1985");

    // YYYY/MM/DD -> DD/MM/YYYY
    assert_eq!(normalize_date_cell("1988/04/25"), "25/04/1988");
    assert_eq!(normalize_date_cell("1995/11/03"), "03/11/1995");

    // DD/MM/YYYY giữ nguyên
    assert_eq!(normalize_date_cell("12/08/1992"), "12/08/1992");
    assert_eq!(normalize_date_cell("20/09/1996"), "20/09/1996");

    // Chuỗi rỗng hoặc không phải ngày
    assert_eq!(normalize_date_cell(""), "");
    assert_eq!(normalize_date_cell("   "), "");
    assert_eq!(normalize_date_cell("khong_phai_ngay"), "khong_phai_ngay");
}

#[test]
fn test_spreadsheet_grid_fuzzy_clustering_detects_vietnamese_variants() {
    use office_utility_vn_ui::micro_apps::data_lab::spreadsheet_grid_view::{
        create_demo_table, detect_fuzzy_clusters,
    };

    let (_, rows) = create_demo_table();

    // Cột 3 là Tỉnh thành chứa các biến thể:
    // Hà Nội, Ha Noi, Hà Nôi, Hanoi
    // TP. Hồ Chí Minh, TP HCM, Ho Chi Minh, TP.HCM
    // Đà Nẵng, Da Nang, Đa Nang
    let clusters = detect_fuzzy_clusters(3, &rows);
    assert!(!clusters.is_empty(), "Phải phát hiện ít nhất 1 cụm chính tả trong cột Tỉnh thành");

    let hanoi_cluster = clusters.iter().find(|c| c.canonical == "Hà Nội");
    assert!(hanoi_cluster.is_some(), "Cụm Hà Nội phải được phát hiện");

    let hanoi = hanoi_cluster.unwrap();
    let variation_names: Vec<&str> = hanoi.variations.iter().map(|(v, _)| v.as_str()).collect();
    assert!(variation_names.contains(&"Hà Nội"));
    assert!(variation_names.contains(&"Ha Noi") || variation_names.contains(&"Hà Nôi") || variation_names.contains(&"Hanoi"));
}

#[test]
fn test_spreadsheet_grid_table_to_csv_utf8_bom() {
    use office_utility_vn_ui::micro_apps::data_lab::spreadsheet_grid_view::{
        create_demo_table, table_to_csv,
    };

    let (cols, rows) = create_demo_table();
    let csv = table_to_csv(&cols, &rows);

    // Phải bắt đầu bằng UTF-8 BOM \u{FEFF} để Excel mở tiếng Việt không bị lỗi font
    assert!(csv.starts_with('\u{FEFF}'));

    // Kiểm tra dòng tiêu đề
    assert!(csv.contains("\"Mã NV\",\"Họ và tên\",\"Ngày sinh\",\"Tỉnh thành\",\"Phòng ban\",\"Lương cơ bản\""));

    // Kiểm tra dòng dữ liệu có dấu tiếng Việt
    assert!(csv.contains("Nguyễn   Văn An"));
    assert!(csv.contains("Hà Nội"));
}

#[test]
fn test_spreadsheet_grid_recipe_step_json_roundtrip() {
    use office_utility_vn_ui::micro_apps::data_lab::spreadsheet_grid_view::RecipeStep;

    let steps = vec![
        RecipeStep {
            id: "step-1".to_string(),
            action: "trim_spaces".to_string(),
            description: "Làm sạch khoảng trắng thừa".to_string(),
            target_col: None,
            timestamp: "12:00:00".to_string(),
        },
        RecipeStep {
            id: "step-2".to_string(),
            action: "normalize_dates".to_string(),
            description: "Chuẩn hóa ngày sang DD/MM/YYYY".to_string(),
            target_col: Some("Ngày sinh".to_string()),
            timestamp: "12:01:00".to_string(),
        },
    ];

    let json = serde_json::to_string(&steps).expect("Serialization phải thành công");
    let deserialized: Vec<RecipeStep> = serde_json::from_str(&json).expect("Deserialization phải thành công");

    assert_eq!(steps, deserialized);
}
