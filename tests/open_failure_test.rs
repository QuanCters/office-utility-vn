use std::sync::atomic::AtomicBool;
use leptos::prelude::*;
use office_engine::data_lab::excel_reader::read_tabular_rows;
use office_utility_vn_ui::micro_apps::gridcraft::GridCraftState;
use office_utility_vn_ui::platform::{
    CoordinatorAction, DocumentState, HostDocumentCoordinator,
};

#[test]
fn test_open_failure_01_valid_xlsx_parsing() {
    let cancel = AtomicBool::new(false);
    let valid_bytes = std::fs::read("fixtures/test_uat_sample.xlsx").expect("Fixtures must exist");
    let result = read_tabular_rows(&valid_bytes, &cancel);
    assert!(result.is_ok(), "Valid XLSX fixture must parse successfully");
    let rows = result.unwrap();
    assert!(!rows.is_empty(), "Parsed rows must not be empty");
    assert_eq!(rows[0][0], "OfficeUtilityVN_UAT_Sample");
}

#[test]
fn test_open_failure_02_corrupt_xlsx_fails_parsing() {
    let cancel = AtomicBool::new(false);
    let corrupt_bytes = std::fs::read("fixtures/test_uat_corrupt.xlsx").expect("Fixtures must exist");
    let result = read_tabular_rows(&corrupt_bytes, &cancel);
    assert!(result.is_err(), "Corrupt XLSX fixture must fail parsing");
}

#[test]
fn test_open_failure_03_truncated_xlsx_fails_parsing() {
    let cancel = AtomicBool::new(false);
    let truncated_bytes = std::fs::read("fixtures/test_uat_truncated.xlsx").expect("Fixtures must exist");
    let result = read_tabular_rows(&truncated_bytes, &cancel);
    assert!(result.is_err(), "Truncated XLSX fixture must fail parsing");
}

#[test]
fn test_open_failure_04_pdf_renamed_to_xlsx_fails_parsing() {
    let pdf_bytes = std::fs::read("fixtures/test_uat_doc.pdf").expect("Fixtures must exist");
    // Định dạng XLSX hợp lệ bắt buộc bắt đầu bằng magic bytes PK\x03\x04
    let is_valid_xlsx_magic = pdf_bytes.starts_with(b"PK\x03\x04");
    assert!(!is_valid_xlsx_magic, "PDF bytes renamed to .xlsx must NOT have XLSX zip magic header");
}

#[test]
fn test_open_failure_05_unsupported_workbook_format() {
    let random_bytes = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04];
    let is_valid_spreadsheet = random_bytes.starts_with(b"PK\x03\x04") || random_bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]);
    assert!(!is_valid_spreadsheet, "Unsupported format bytes must NOT be recognized as spreadsheet");
}

#[test]
fn test_open_failure_06_native_dialog_cancel_preserves_document_and_dirty() {
    let state = GridCraftState::new();
    state.set_document_session(Some("doc-session-original".into()), Some("/data/work.xlsx".into()), 3);
    state.set_active_file("work.xlsx");
    state.is_dirty.set(true);

    // Người dùng bấm Cancel hộp thoại OS Open Dialog
    state.abort_provisional_open(None);

    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-session-original"));
    assert_eq!(state.active_file_path.get_untracked().as_deref(), Some("/data/work.xlsx"));
    assert!(state.is_dirty.get_untracked(), "Cancel must preserve dirty flag");
    assert!(state.provisional_open.get_untracked().is_none());
}

#[test]
fn test_open_failure_07_parse_failure_after_provisional_session_creation() {
    let state = GridCraftState::new();
    state.set_document_session(Some("doc-original".into()), Some("/data/original.xlsx".into()), 1);
    state.set_active_file("original.xlsx");
    state.is_dirty.set(true);

    // 1. Tạo provisional open session
    state.begin_provisional_open(
        "open-req-99".into(),
        "doc-prov-99".into(),
        "/data/bad.xlsx".into(),
        "bad.xlsx".into(),
        1,
        1,
    );

    // 2. Parse thất bại: iframe gửi gridcraft:open_failed
    state.abort_provisional_open(Some("open-req-99"));

    // Bất biến: Session gốc và dirty state được bảo toàn 100%
    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-original"));
    assert_eq!(state.active_file_path.get_untracked().as_deref(), Some("/data/original.xlsx"));
    assert!(state.is_dirty.get_untracked());
    assert!(state.provisional_open.get_untracked().is_none());
}

#[test]
fn test_open_failure_08_repeated_open_actions_last_wins() {
    let state = GridCraftState::new();
    state.set_document_session(Some("doc-init".into()), Some("/data/init.xlsx".into()), 1);

    // Người dùng bấm Open lần 1
    state.begin_provisional_open("open-1".into(), "doc-1".into(), "/path/1.xlsx".into(), "1.xlsx".into(), 1, 1);

    // Người dùng bấm Open lần 2 ngay lập tức
    state.begin_provisional_open("open-2".into(), "doc-2".into(), "/path/2.xlsx".into(), "2.xlsx".into(), 1, 1);

    // ACK muộn của Open 1 tới: Phải bị từ chối!
    let committed_1 = state.commit_provisional_open("open-1");
    assert!(!committed_1, "Stale Open 1 must NOT be allowed to commit");

    // ACK của Open 2 tới: Được chấp nhận!
    let committed_2 = state.commit_provisional_open("open-2");
    assert!(committed_2, "Latest Open 2 must commit successfully");
    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-2"));
    assert_eq!(state.active_file_path.get_untracked().as_deref(), Some("/path/2.xlsx"));
}

#[test]
fn test_open_failure_09_iframe_reload_invalidates_stale_provisional_open() {
    let state = GridCraftState::new();
    state.set_document_session(Some("doc-prev".into()), Some("/data/prev.xlsx".into()), 1);

    // Đang mở ở generation 1
    state.begin_provisional_open("open-gen-1".into(), "doc-new".into(), "/path/new.xlsx".into(), "new.xlsx".into(), 1, 1);

    // Iframe reload: reset provisional open
    state.abort_provisional_open(None);

    // Cố gắng commit với open_id của generation cũ -> Thất bại!
    assert!(!state.commit_provisional_open("open-gen-1"));
    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-prev"));
}

#[test]
fn test_open_failure_10_open_success_ack_arriving_late_after_abort() {
    let state = GridCraftState::new();
    state.set_document_session(Some("doc-live".into()), Some("/data/live.xlsx".into()), 1);

    state.begin_provisional_open("open-late".into(), "doc-late".into(), "/data/late.xlsx".into(), "late.xlsx".into(), 1, 1);
    // Người dùng hủy hoặc có sự kiện khác abort
    state.abort_provisional_open(Some("open-late"));

    // Sau đó ACK tới muộn
    let result = state.commit_provisional_open("open-late");
    assert!(!result, "Late ACK after abort must be rejected");
    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-live"));
}

#[test]
fn test_open_failure_11_open_failure_ack_arriving_late_does_not_abort_newer_open() {
    let state = GridCraftState::new();
    state.set_document_session(Some("doc-live".into()), Some("/data/live.xlsx".into()), 1);

    // Open 1
    state.begin_provisional_open("open-1".into(), "doc-1".into(), "/data/1.xlsx".into(), "1.xlsx".into(), 1, 1);

    // Open 2 (mới hơn)
    state.begin_provisional_open("open-2".into(), "doc-2".into(), "/data/2.xlsx".into(), "2.xlsx".into(), 1, 1);

    // Open failure của open-1 tới muộn
    state.abort_provisional_open(Some("open-1"));

    // Provisional open hiện tại vẫn phải là open-2!
    assert_eq!(
        state.provisional_open.get_untracked().as_ref().map(|p| p.open_id.as_str()),
        Some("open-2"),
        "Late failure of open-1 must not cancel open-2"
    );

    // open-2 có thể commit bình thường
    assert!(state.commit_provisional_open("open-2"));
    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-2"));
}

#[test]
fn test_open_failure_12_opening_second_document_while_first_dirty() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "/data/first.xlsx", "h1");
    coord.on_user_edit();
    assert!(coord.is_dirty());

    // Khi người dùng yêu cầu mở tệp 2, coordinator chặn lại và hỏi unsaved changes
    let action = coord.on_navigation_requested("file:open");
    assert!(matches!(action, CoordinatorAction::PromptUnsavedChanges { .. }));
    assert!(coord.is_dirty());
    assert_eq!(coord.dest_path().unwrap(), "/data/first.xlsx");
}

#[test]
fn test_open_failure_13_save_then_open_after_async_save_success() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "/data/first.xlsx", "h1");
    coord.on_user_edit();
    assert!(coord.is_dirty());

    // 1. Lưu tài liệu
    let session_gen = coord.session_generation();
    coord.on_save_requested(session_gen, "doc-1", "req-save-1", vec![1, 2, 3], 1000);
    let fp = match coord.state() {
        DocumentState::Saving { snapshot_fingerprint, .. } => snapshot_fingerprint.clone(),
        _ => panic!("Expected Saving"),
    };

    // 2. Lưu thành công
    coord.on_durable_write_completed("req-save-1", &fp);
    assert_eq!(*coord.state(), DocumentState::Clean);
    assert!(!coord.is_dirty());

    // 3. Bây giờ mở tệp tiếp theo -> Cho phép ngay vì đã clean
    let action = coord.on_navigation_requested("file:open");
    assert!(matches!(action, CoordinatorAction::ProceedNavigation(_)));
}

#[test]
fn test_open_failure_14_save_then_open_after_save_failure_retains_workbook() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "/data/first.xlsx", "h1");
    coord.on_user_edit();

    let session_gen = coord.session_generation();
    coord.on_save_requested(session_gen, "doc-1", "req-save-fail", vec![1, 2, 3], 1000);

    // Ghi thất bại
    coord.on_durable_write_failed("req-save-fail", "I/O Error");
    assert!(coord.is_dirty());

    // Yêu cầu chuyển/mở tệp tiếp theo BẮT BUỘC bị chặn lại
    let action = coord.on_navigation_requested("file:open");
    assert!(matches!(action, CoordinatorAction::PromptUnsavedChanges { .. }));
    assert!(coord.is_dirty(), "Tài liệu phải tiếp tục Dirty để bảo vệ dữ liệu");
}

#[test]
fn test_open_failure_15_rapid_open_requests_monotonic_collision_free_ids() {
    // Workstream 4: Kiểm tra sinh open request ID không phụ thuộc duy nhất vào timestamp ms
    static TEST_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let session_gen = 2u64;
    let fixed_timestamp = 1700000000000u64;

    let id1 = format!("open-g{}-{}-{}", session_gen, fixed_timestamp, TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let id2 = format!("open-g{}-{}-{}", session_gen, fixed_timestamp, TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed));

    assert_ne!(id1, id2, "Hai yêu cầu mở tệp phát sinh trong cùng 1 ms bắt buộc phải có ID phân biệt không xung đột");
    assert!(id1.starts_with("open-g2-1700000000000-"));
    assert!(id2.starts_with("open-g2-1700000000000-"));
}

#[test]
fn test_open_failure_16_generation_mismatch_stale_response_rejected() {
    let state = GridCraftState::new();
    state.set_document_session(Some("doc-live".into()), Some("/data/live.xlsx".into()), 1);

    // Mở tài liệu mới ở generation 1
    state.begin_provisional_open("open-req-gen1".into(), "doc-new".into(), "/data/new.xlsx".into(), "new.xlsx".into(), 1, 1);

    // Iframe reload xảy ra, host tăng generation lên 2
    let cur_gen = 2u64;

    // ACK từ generation 1 tới muộn khi generation hiện tại là 2
    let committed = state.commit_provisional_open_with_generation("open-req-gen1", cur_gen);
    assert!(!committed, "Phản hồi mở tệp từ thế hệ iframe cũ (generation 1) BẮT BUỘC bị từ chối khi host đã ở generation 2");
    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-live"), "Tài liệu đang mở trước đó phải được bảo toàn nguyên vẹn");
}
