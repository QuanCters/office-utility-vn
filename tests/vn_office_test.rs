use leptos::prelude::*;
use office_utility_vn_ui::micro_apps::vn_office::sidebar::vn_office_tool_icon;
use office_utility_vn_ui::micro_apps::vn_office::state::{POPULAR_VN_BANKS, VnOfficeState};
use office_utility_vn_ui::micro_apps::{tool_slug_to_micro_app, MicroAppId, VN_OFFICE_TOOLS};

const EXPECTED_VN_OFFICE_SLUGS: &[&str] = &[
    "vietnamese-number-to-words",
    "gross-net-salary-calculator",
    "vietqr-napas247-generator",
    "vn-einvoice-xml-reader",
    "vn-cccd-mst-validator",
    "vn-address-normalizer",
    "vn-working-days-calc",
    "vn-loan-amortization",
    "vn-bank-bin-mapper",
    "vietnamese-font-converter",
    "encoding-mojibake-fixer",
];

#[test]
fn test_vn_office_state_default_tool() {
    let state = VnOfficeState::new();

    // 1. Mặc định công cụ ban đầu là "vietnamese-number-to-words"
    assert_eq!(state.active_tool_slug(), "vietnamese-number-to-words");

    // 2. Chế độ workflow ban đầu phải là false
    assert!(!state.is_workflow());

    // 3. Trạng thái thực thi ban đầu phải là false
    assert!(!state.is_running.get());

    // 4. Ô lọc sidebar ban đầu rỗng
    assert_eq!(state.sidebar_filter.get(), "");

    // 5. Panel quy định pháp lý mặc định được mở
    assert!(state.is_legal_panel_open.get());
}

#[test]
fn test_vn_office_state_with_custom_initial_tool() {
    // 1. Khởi tạo với slug cụ thể
    let state_salary = VnOfficeState::with_initial_tool(Some("gross-net-salary-calculator".to_string()));
    assert_eq!(state_salary.active_tool_slug(), "gross-net-salary-calculator");
    assert!(!state_salary.is_workflow());

    let state_qr = VnOfficeState::with_initial_tool(Some("vietqr-napas247-generator".to_string()));
    assert_eq!(state_qr.active_tool_slug(), "vietqr-napas247-generator");

    // 2. Khởi tạo với chuỗi rỗng hoặc toàn khoảng trắng -> fallback về mặc định
    let state_empty = VnOfficeState::with_initial_tool(Some("   ".to_string()));
    assert_eq!(state_empty.active_tool_slug(), "vietnamese-number-to-words");

    let state_none = VnOfficeState::with_initial_tool(None);
    assert_eq!(state_none.active_tool_slug(), "vietnamese-number-to-words");
}

#[test]
fn test_vn_office_state_tool_switching_and_workflow_toggle() {
    let state = VnOfficeState::new();
    assert_eq!(state.active_tool_slug(), "vietnamese-number-to-words");
    assert!(!state.is_workflow());

    // 1. Chuyển sang hóa đơn điện tử
    state.select_tool("vn-einvoice-xml-reader");
    assert_eq!(state.active_tool_slug(), "vn-einvoice-xml-reader");
    assert!(!state.is_workflow());

    // 2. Mở chế độ Workflow DAG
    state.open_workflow();
    assert!(state.is_workflow());

    // 3. Khi đang ở Workflow DAG, chọn một công cụ mới -> phải tự động tắt Workflow DAG
    state.select_tool("vn-cccd-mst-validator");
    assert_eq!(state.active_tool_slug(), "vn-cccd-mst-validator");
    assert!(!state.is_workflow());

    // 4. Kiểm tra đóng mở workflow thủ công
    state.open_workflow();
    assert!(state.is_workflow());
    state.close_workflow();
    assert!(!state.is_workflow());
    assert_eq!(state.active_tool_slug(), "vn-cccd-mst-validator");
}

#[test]
fn test_vn_office_11_tools_exact_list() {
    let state = VnOfficeState::new();
    let tools = state.tools();

    // 1. Số lượng công cụ chính xác là 11
    assert_eq!(tools.len(), 11);
    assert_eq!(VN_OFFICE_TOOLS.len(), 11);

    // 2. Khớp chính xác 11 slugs theo yêu cầu thiết kế
    let actual_slugs: Vec<&str> = tools.iter().map(|t| t.slug).collect();
    assert_eq!(actual_slugs, EXPECTED_VN_OFFICE_SLUGS);

    // 3. Toàn bộ 11 công cụ phải định tuyến hợp lệ sang MicroAppId::VnOffice
    for slug in EXPECTED_VN_OFFICE_SLUGS.iter() {
        assert_eq!(
            tool_slug_to_micro_app(slug),
            Some(MicroAppId::VnOffice),
            "Slug {} phải thuộc MicroAppId::VnOffice",
            slug
        );

        let found = state.find_tool(slug);
        assert!(found.is_some(), "Công cụ {} phải tìm thấy qua find_tool", slug);
        let tool = found.unwrap();
        assert!(!tool.name.is_empty(), "Tên công cụ {} không được rỗng", slug);
        assert!(!tool.description.is_empty(), "Mô tả công cụ {} không được rỗng", slug);
    }

    // 4. Tìm kiếm slug không tồn tại trả về None
    assert!(state.find_tool("non-existent-tool").is_none());
}

#[test]
fn test_vn_office_legal_regulations_defaults() {
    let state = VnOfficeState::new();
    let regs = state.regulations.get();

    // 1. Giảm trừ gia cảnh theo Nghị quyết 954/2020/UBTVQH14
    assert_eq!(regs.personal_deduction, 11_000_000);
    assert_eq!(regs.dependent_deduction, 4_400_000);

    // 2. Tỷ lệ trích nộp bảo hiểm người lao động
    assert_eq!(regs.bhxh_employee_rate, 0.08); // 8%
    assert_eq!(regs.bhyt_employee_rate, 0.015); // 1.5%
    assert_eq!(regs.bhtn_employee_rate, 0.01); // 1.0%

    // 3. Tỷ lệ trích nộp người sử dụng lao động
    assert_eq!(regs.bhxh_employer_rate, 0.175); // 17.5%
    assert_eq!(regs.bhyt_employer_rate, 0.03); // 3.0%
    assert_eq!(regs.bhtn_employer_rate, 0.01); // 1.0%

    // 4. Lương cơ sở mới nhất theo Nghị định 73/2024/NĐ-CP
    assert_eq!(regs.base_salary, 2_340_000);
    assert_eq!(regs.max_insurable_salary, 46_800_000); // 20 lần lương cơ sở
}

#[test]
fn test_vn_office_bank_references() {
    let state = VnOfficeState::new();
    let banks = state.bank_references();

    assert_eq!(banks, POPULAR_VN_BANKS);
    assert!(banks.len() >= 45, "Danh sách ngân hàng phải bao gồm ít nhất 45 ngân hàng VN");

    // Kiểm tra các ngân hàng trọng yếu
    let has_vcb = banks.iter().any(|b| b.bin == "970436" && b.short_name == "VCB");
    let has_bidv = banks.iter().any(|b| b.bin == "970418" && b.short_name == "BIDV");
    let has_ctg = banks.iter().any(|b| b.bin == "970415" && b.short_name == "CTG");
    let has_tcb = banks.iter().any(|b| b.bin == "970407" && b.short_name == "TCB");
    let has_mb = banks.iter().any(|b| b.bin == "970422" && b.short_name == "MB");

    assert!(has_vcb, "Phải có Vietcombank");
    assert!(has_bidv, "Phải có BIDV");
    assert!(has_ctg, "Phải có VietinBank");
    assert!(has_tcb, "Phải có Techcombank");
    assert!(has_mb, "Phải có MBBank");
}

#[test]
fn test_vn_office_tool_icon_mapping() {
    for slug in EXPECTED_VN_OFFICE_SLUGS.iter() {
        let icon = vn_office_tool_icon(slug);
        assert!(!icon.is_empty(), "Icon cho slug {} không được rỗng", slug);
        assert_ne!(icon, "💼", "Slug {} phải có icon chuyên biệt chứ không dùng icon mặc định", slug);
    }

    // Icon fallback cho slug lạ
    assert_eq!(vn_office_tool_icon("unknown-slug"), "💼");
}
