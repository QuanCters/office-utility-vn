use leptos::prelude::*;
use office_utility_vn_ui::micro_apps::gridcraft::{
    is_gridcraft_dev_workspace_enabled, GridCraftApp, GridCraftState, GRIDCRAFT_IFRAME_SRC,
};
use office_utility_vn_ui::platform::adapter::{GridCraftAdapter, MicroAppAdapter};

#[test]
fn test_gridcraft_dev_gate_enabled_in_debug() {
    // Stage 4A Security Gate: Dev-only gate MUST be active in debug assertions
    assert!(
        is_gridcraft_dev_workspace_enabled(),
        "Development-only gate must be enabled in debug mode"
    );
}

#[test]
fn test_gridcraft_state_initialization() {
    let state = GridCraftState::new();

    // Khởi tạo ban đầu: canvas chưa sẵn sàng, chưa có thay đổi, chưa có lỗi
    assert!(!state.is_canvas_ready.get());
    assert!(!state.is_dirty.get());
    assert_eq!(state.active_file_name.get(), None);
    assert_eq!(state.error_message.get(), None);
    assert!(!state.is_save_blocked_alert.get());
}

#[test]
fn test_gridcraft_workspace_iframe_url_and_params() {
    // 1. Phải chứa host=parent để kích hoạt cầu nối postMessage
    assert!(
        GRIDCRAFT_IFRAME_SRC.contains("host=parent"),
        "URL phải có ?host=parent"
    );

    // 2. Phải chứa save=confirmed để kích hoạt bản vá 0001
    assert!(
        GRIDCRAFT_IFRAME_SRC.contains("save=confirmed"),
        "URL phải có save=confirmed"
    );

    // 3. Phải dùng chrome=notitlebar để giữ Ribbon và File tab, loại bỏ title bar giả lập
    assert!(
        GRIDCRAFT_IFRAME_SRC.contains("chrome=notitlebar"),
        "URL phải dùng chrome=notitlebar"
    );

    // 4. TUYỆT ĐỐI KHÔNG dùng chrome=none hoặc chrome=nofiletab vì sẽ làm mất File tab
    assert!(
        !GRIDCRAFT_IFRAME_SRC.contains("chrome=none"),
        "URL không được dùng chrome=none"
    );
    assert!(
        !GRIDCRAFT_IFRAME_SRC.contains("chrome=nofiletab"),
        "URL không được dùng chrome=nofiletab"
    );
}

#[test]
fn test_gridcraft_app_component_instantiation() {
    // Kiểm tra khởi tạo component GridCraftApp không panic
    let _view = view! {
        <GridCraftApp on_back=Callback::new(|_| {}) />
    };
}

#[test]
fn test_gridcraft_stage_4c1_production_save_authorized() {
    let state = GridCraftState::new();

    // Stage 4C-1: Production Save được mở khóa hợp thức qua Native Persistence
    let save_res = state.handle_save_requested("budget.xlsx", Some("req-1"), &[1, 2, 3]);
    assert!(save_res.is_ok(), "Stage 4C-1: GridCraft legitimate save phải được chấp thuận để thực thi qua Native Persistence");
    assert!(!state.is_save_blocked_alert.get());
}

#[test]
fn test_gridcraft_lifecycle_session_and_coordinator_integration() {
    let state = GridCraftState::new();
    assert_eq!(state.active_document_id.get(), None);
    assert_eq!(state.active_file_path.get(), None);
    assert_eq!(state.active_revision.get(), 0);

    // Mở tài liệu phiên mới
    state.set_document_session(
        Some("grid-doc-404".to_string()),
        Some("/tmp/workspace_test.xlsx".to_string()),
        1,
    );
    state.set_active_file("workspace_test.xlsx");

    assert_eq!(state.active_document_id.get().as_deref(), Some("grid-doc-404"));
    assert_eq!(state.active_file_path.get().as_deref(), Some("/tmp/workspace_test.xlsx"));
    assert_eq!(state.active_revision.get(), 1);
    assert_eq!(state.active_file_name.get().as_deref(), Some("workspace_test.xlsx"));
    assert!(!state.is_dirty.get());

    // Giả lập người dùng sửa bảng tính
    state.is_dirty.set(true);
    assert!(state.is_dirty.get());
}

#[test]
fn test_gridcraft_confirmed_save_adapter_protocol() {
    let adapter = GridCraftAdapter;

    // 1. Kiểm tra format_open
    let open_msg = adapter.format_open("test.xlsx", &[0x50, 0x4B, 0x03, 0x04]).unwrap();
    assert_eq!(open_msg.type_name, "gridcraft:open");
    assert_eq!(open_msg.payload.get("name").unwrap().as_str().unwrap(), "test.xlsx");
    assert_eq!(open_msg.binary_data, Some(vec![0x50, 0x4B, 0x03, 0x04]));

    // 2. Kiểm tra format_saved
    let saved_msg = GridCraftAdapter::format_saved("save-1-1");
    assert_eq!(saved_msg.type_name, "gridcraft:saved");
    assert_eq!(saved_msg.payload.get("id").unwrap().as_str().unwrap(), "save-1-1");

    // 3. Kiểm tra format_failed
    let failed_msg = GridCraftAdapter::format_failed("Disk Full", Some("save-1-2"));
    assert_eq!(failed_msg.type_name, "gridcraft:failed");
    assert_eq!(failed_msg.payload.get("error").unwrap().as_str().unwrap(), "Disk Full");
    assert_eq!(failed_msg.payload.get("id").unwrap().as_str().unwrap(), "save-1-2");
}

#[test]
fn test_stage4c0b_reproduce_defect2_corrupt_open_replaces_host_session() {
    let state = GridCraftState::new();
    // 1. Host đang có tài liệu gốc A
    state.set_document_session(Some("doc-valid-a".into()), Some("/data/report_a.xlsx".into()), 1);
    state.set_active_file("report_a.xlsx");
    state.is_dirty.set(true);

    // 2. Mở tệp hỏng B: Bắt đầu giao dịch provisional open
    state.begin_provisional_open(
        "open-corrupt-1".into(),
        "doc-corrupt-b".into(),
        "/data/corrupt.xlsx".into(),
        "corrupt.xlsx".into(),
        1,
        1,
    );

    // Trong khi parse, active document VẪN LÀ A:
    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-valid-a"));
    assert_eq!(state.active_file_path.get_untracked().as_deref(), Some("/data/report_a.xlsx"));

    // 3. Iframe báo parse lỗi! Gọi abort_provisional_open:
    state.abort_provisional_open(Some("open-corrupt-1"));

    // Sau khi abort, tài liệu gốc A BẮT BUỘC ĐƯỢC BẢO TOÀN 100%!
    assert_eq!(
        state.active_document_id.get_untracked().as_deref(),
        Some("doc-valid-a"),
        "Tài liệu A phải được bảo toàn khi mở tệp B bị lỗi!"
    );
    assert_eq!(
        state.active_file_path.get_untracked().as_deref(),
        Some("/data/report_a.xlsx"),
        "Đường dẫn của tài liệu A phải được bảo toàn khi mở tệp B bị lỗi!"
    );
    assert!(state.is_dirty.get_untracked(), "Cờ dirty của tài liệu A phải được bảo toàn!");
    assert!(state.provisional_open.get_untracked().is_none(), "Phiên tạm thời phải bị hủy bỏ hoàn toàn!");
}

#[test]
fn test_stage4c0b_valid_open_commits_new_session_after_parsing_success() {
    let state = GridCraftState::new();
    state.set_document_session(Some("doc-valid-a".into()), Some("/data/report_a.xlsx".into()), 1);
    state.set_active_file("report_a.xlsx");

    // 1. Bắt đầu provisional open
    state.begin_provisional_open(
        "open-valid-2".into(),
        "doc-valid-b".into(),
        "/data/report_b.xlsx".into(),
        "report_b.xlsx".into(),
        1,
        1,
    );

    // 2. Iframe báo parse thành công!
    let committed = state.commit_provisional_open("open-valid-2");
    assert!(committed);

    // 3. Bây giờ session mới chính thức được chuyển sang B:
    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-valid-b"));
    assert_eq!(state.active_file_path.get_untracked().as_deref(), Some("/data/report_b.xlsx"));
    assert_eq!(state.active_file_name.get_untracked().as_deref(), Some("report_b.xlsx"));
    assert!(!state.is_dirty.get_untracked());
    assert!(state.provisional_open.get_untracked().is_none());
}

#[test]
fn test_stage4c0b_native_dialog_cancel_preserves_document_and_dirty() {
    let state = GridCraftState::new();
    state.set_document_session(Some("doc-clean-a".into()), Some("/data/clean_a.xlsx".into()), 2);
    state.set_active_file("clean_a.xlsx");
    state.is_dirty.set(true);

    // Người dùng bấm Cancel hộp thoại native dialog
    state.abort_provisional_open(None);

    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-clean-a"));
    assert_eq!(state.active_file_path.get_untracked().as_deref(), Some("/data/clean_a.xlsx"));
    assert!(state.is_dirty.get_untracked(), "Cancel phải giữ nguyên dirty của tài liệu hiện tại");
    assert!(state.provisional_open.get_untracked().is_none());
}
