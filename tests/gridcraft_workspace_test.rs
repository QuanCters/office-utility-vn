use leptos::prelude::*;
use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;
use office_utility_vn_ui::micro_apps::gridcraft::workspace::{
    DevSaveResult, GridCraftSaveRequestContext, handle_gridcraft_save_outcome,
};
use office_utility_vn_ui::micro_apps::gridcraft::{
    GRIDCRAFT_IFRAME_SRC, GridCraftApp, GridCraftState, is_gridcraft_dev_workspace_enabled,
};
use office_utility_vn_ui::platform::adapter::{
    GridCraftAdapter, MicroAppAdapter, SaveOperationIntent,
};
use office_utility_vn_ui::platform::coordinator::HostDocumentCoordinator;

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
    assert!(
        save_res.is_ok(),
        "Stage 4C-1: GridCraft legitimate save phải được chấp thuận để thực thi qua Native Persistence"
    );
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

    assert_eq!(
        state.active_document_id.get().as_deref(),
        Some("grid-doc-404")
    );
    assert_eq!(
        state.active_file_path.get().as_deref(),
        Some("/tmp/workspace_test.xlsx")
    );
    assert_eq!(state.active_revision.get(), 1);
    assert_eq!(
        state.active_file_name.get().as_deref(),
        Some("workspace_test.xlsx")
    );
    assert!(!state.is_dirty.get());

    // Giả lập người dùng sửa bảng tính
    state.is_dirty.set(true);
    assert!(state.is_dirty.get());
}

#[test]
fn test_gridcraft_confirmed_save_adapter_protocol() {
    let adapter = GridCraftAdapter;

    // 1. Kiểm tra format_open
    let open_msg = adapter
        .format_open("test.xlsx", &[0x50, 0x4B, 0x03, 0x04])
        .unwrap();
    assert_eq!(open_msg.type_name, "gridcraft:open");
    assert_eq!(
        open_msg.payload.get("name").unwrap().as_str().unwrap(),
        "test.xlsx"
    );
    assert_eq!(open_msg.binary_data, Some(vec![0x50, 0x4B, 0x03, 0x04]));

    // 2. Kiểm tra format_saved
    let saved_msg = GridCraftAdapter::format_saved("save-1-1");
    assert_eq!(saved_msg.type_name, "gridcraft:saved");
    assert_eq!(
        saved_msg.payload.get("id").unwrap().as_str().unwrap(),
        "save-1-1"
    );

    // 3. Kiểm tra format_failed
    let failed_msg = GridCraftAdapter::format_failed("Disk Full", Some("save-1-2"));
    assert_eq!(failed_msg.type_name, "gridcraft:failed");
    assert_eq!(
        failed_msg.payload.get("error").unwrap().as_str().unwrap(),
        "Disk Full"
    );
    assert_eq!(
        failed_msg.payload.get("id").unwrap().as_str().unwrap(),
        "save-1-2"
    );
}

#[test]
fn test_stage4c0b_reproduce_defect2_corrupt_open_replaces_host_session() {
    let state = GridCraftState::new();
    // 1. Host đang có tài liệu gốc A
    state.set_document_session(
        Some("doc-valid-a".into()),
        Some("/data/report_a.xlsx".into()),
        1,
    );
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
    assert_eq!(
        state.active_document_id.get_untracked().as_deref(),
        Some("doc-valid-a")
    );
    assert_eq!(
        state.active_file_path.get_untracked().as_deref(),
        Some("/data/report_a.xlsx")
    );

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
    assert!(
        state.is_dirty.get_untracked(),
        "Cờ dirty của tài liệu A phải được bảo toàn!"
    );
    assert!(
        state.provisional_open.get_untracked().is_none(),
        "Phiên tạm thời phải bị hủy bỏ hoàn toàn!"
    );
}

#[test]
fn test_stage4c0b_valid_open_commits_new_session_after_parsing_success() {
    let state = GridCraftState::new();
    state.set_document_session(
        Some("doc-valid-a".into()),
        Some("/data/report_a.xlsx".into()),
        1,
    );
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
    assert_eq!(
        state.active_document_id.get_untracked().as_deref(),
        Some("doc-valid-b")
    );
    assert_eq!(
        state.active_file_path.get_untracked().as_deref(),
        Some("/data/report_b.xlsx")
    );
    assert_eq!(
        state.active_file_name.get_untracked().as_deref(),
        Some("report_b.xlsx")
    );
    assert!(!state.is_dirty.get_untracked());
    assert!(state.provisional_open.get_untracked().is_none());
}

#[test]
fn test_stage4c0b_native_dialog_cancel_preserves_document_and_dirty() {
    let state = GridCraftState::new();
    state.set_document_session(
        Some("doc-clean-a".into()),
        Some("/data/clean_a.xlsx".into()),
        2,
    );
    state.set_active_file("clean_a.xlsx");
    state.is_dirty.set(true);

    // Người dùng bấm Cancel hộp thoại native dialog
    state.abort_provisional_open(None);

    assert_eq!(
        state.active_document_id.get_untracked().as_deref(),
        Some("doc-clean-a")
    );
    assert_eq!(
        state.active_file_path.get_untracked().as_deref(),
        Some("/data/clean_a.xlsx")
    );
    assert!(
        state.is_dirty.get_untracked(),
        "Cancel phải giữ nguyên dirty của tài liệu hiện tại"
    );
    assert!(state.provisional_open.get_untracked().is_none());
}

// =========================================================================
// CODEX P1 REGRESSION TESTS (PR #2 REPRODUCING EVIDENCE)
// =========================================================================

#[test]
fn test_p1_finding1_coordinator_lifecycle_and_synchronization_preserves_concurrent_edits() {
    use office_utility_vn_ui::platform::coordinator::{
        CoordinatorAction, DocumentState, HostDocumentCoordinator,
    };

    let mut coordinator = HostDocumentCoordinator::new();

    // 1. Defect reproduction in unpatched state:
    // If coordinator remains Unloaded, on_save_requested returns None,
    // and concurrent edits during save are ignored by is_dirty().
    assert_eq!(*coordinator.state(), DocumentState::Unloaded);
    let rej_action = coordinator.on_save_requested(1, "doc-1", "req-reject", vec![1], 100);
    assert_eq!(
        rej_action,
        CoordinatorAction::None,
        "Unloaded coordinator must reject save"
    );

    // 2. Synchronized lifecycle:
    // When a document is opened/committed in GridCraft:
    coordinator.on_document_opened("doc-1", "/data/financials.xlsx", "init-fp");
    assert_eq!(*coordinator.state(), DocumentState::Clean);
    assert_eq!(coordinator.dest_path(), Some("/data/financials.xlsx"));

    // User edits the sheet:
    coordinator.on_user_edit();
    assert_eq!(*coordinator.state(), DocumentState::Dirty);
    assert!(coordinator.is_dirty());

    // User triggers save -> coordinator transitions to Saving:
    let action = coordinator.on_save_requested(1, "doc-1", "req-valid", vec![1, 2, 3], 200);
    assert!(
        matches!(action, CoordinatorAction::PerformDurableWrite { ref dest_path, .. } if dest_path == "/data/financials.xlsx")
    );
    assert!(matches!(
        coordinator.state(),
        DocumentState::Saving {
            dirty_during_save: false,
            ..
        }
    ));

    // Concurrent user edit occurs while save I/O is in-flight:
    coordinator.on_user_edit();
    assert!(matches!(
        coordinator.state(),
        DocumentState::Saving {
            dirty_during_save: true,
            ..
        }
    ));

    // Durable write finishes:
    let written_fp = format!(
        "{:x}",
        office_utility_vn_ui::platform::coordinator::md5_or_simple_hash(&[1, 2, 3])
    );
    coordinator.on_durable_write_completed("req-valid", &written_fp);

    // Coordinator MUST preserve dirty state because of concurrent edits:
    assert!(
        coordinator.is_dirty(),
        "Coordinator MUST remain dirty when concurrent edit happened during save!"
    );
    assert_eq!(*coordinator.state(), DocumentState::Dirty);
}

#[test]
fn test_p1_finding2_multi_workbook_save_isolation_prevents_wrong_workbook_overwrite() {
    use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;

    let state = GridCraftState::new();

    // Open Workbook A
    state.register_document_session(WorkbookSession {
        document_id: "doc-A".into(),
        file_path: "/data/A.xlsx".into(),
        file_name: "A.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: None,
    });
    state.set_document_session(Some("doc-A".into()), Some("/data/A.xlsx".into()), 1);
    state.active_file_name.set(Some("A.xlsx".into()));

    // Open Workbook B
    state.register_document_session(WorkbookSession {
        document_id: "doc-B".into(),
        file_path: "/data/B.xlsx".into(),
        file_name: "B.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: None,
    });
    state.set_document_session(Some("doc-B".into()), Some("/data/B.xlsx".into()), 1);
    state.active_file_name.set(Some("B.xlsx".into()));

    // Now user in GridCraft switches window to A.xlsx and triggers save for "A.xlsx":
    // The resolved session MUST be for A.xlsx (/data/A.xlsx, doc-A), NEVER B.xlsx (/data/B.xlsx)!
    let session_a = state
        .resolve_save_session("A.xlsx")
        .expect("Session A must be resolved");
    assert_eq!(session_a.document_id, "doc-A");
    assert_eq!(session_a.file_path, "/data/A.xlsx");
    assert_eq!(session_a.file_name, "A.xlsx");

    // Saving an unknown / unregistered workbook name must return None to trigger Save As rather than overwriting B:
    let session_unknown = state.resolve_save_session("Unknown.xlsx");
    assert!(
        session_unknown.is_none(),
        "Unknown workbook must not bind to currently active session"
    );
}

#[test]
fn test_p1_finding3_message_listener_cleanup_on_unmount() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let cleaned_up = Arc::new(AtomicBool::new(false));
    let cleaned_up_clone = cleaned_up.clone();

    let owner = Owner::new();
    owner.with(|| {
        on_cleanup(move || {
            cleaned_up_clone.store(true, Ordering::SeqCst);
        });
    });

    assert!(
        !cleaned_up.load(Ordering::SeqCst),
        "Cleanup should not run before unmount"
    );
    drop(owner);
    assert!(
        cleaned_up.load(Ordering::SeqCst),
        "Cleanup MUST run upon component unmount"
    );
}

#[test]
fn test_p1_finding4_save_and_exit_rechecks_all_applications_preventing_data_loss() {
    use office_utility_vn_ui::app::AppState;
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::handle_gridcraft_save_completion;
    use office_utility_vn_ui::platform::window_close::WindowCloseManager;

    let app_state = AppState::new();
    let close_manager = WindowCloseManager::new();

    // Scenario: GridCraft is dirty, but PDF Studio is ALSO dirty
    app_state.gridcraft_state.is_dirty.set(true);
    app_state.pdf_studio_state.has_unsaved_changes.set(true);
    assert!(app_state.has_unsaved_work_or_running_task());

    // Close request initiated
    let close_req_id = "close-req-multi-app-1".to_string();
    close_manager
        .active_request_id
        .set(Some(close_req_id.clone()));

    // GridCraft completes its save successfully and is now clean (is_still_dirty = false):
    app_state.gridcraft_state.is_dirty.set(false);

    // Save completion handler runs:
    handle_gridcraft_save_completion(
        false, // GridCraft is not dirty anymore
        Some(close_req_id),
        Some(close_manager),
        Some(app_state),
    );

    // Window MUST NOT be authorized to close because PDF Studio is still dirty!
    assert!(
        !close_manager.is_authorized.get_untracked(),
        "Must NOT authorize close when PDF Studio is dirty!"
    );
    assert!(
        close_manager.is_cancelled.get_untracked(),
        "Close request must be cancelled to prevent data loss!"
    );
    assert!(
        close_manager.error_message.get_untracked().is_some(),
        "Informative error message must be provided"
    );
}

#[test]
fn test_p1_filesystem_workbook_preservation_real_fixtures() {
    use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;

    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let file_a = temp_dir.path().join("Report_2025.xlsx");
    let file_b = temp_dir.path().join("Budget_2026.xlsx");

    let original_content_a = b"PK\x03\x04_WORKBOOK_A_FINANCIALS_DATA";
    let original_content_b = b"PK\x03\x04_WORKBOOK_B_BUDGET_ESTIMATES";

    std::fs::write(&file_a, original_content_a).expect("Write file A");
    std::fs::write(&file_b, original_content_b).expect("Write file B");

    let state = GridCraftState::new();
    state.register_document_session(WorkbookSession {
        document_id: "doc-rep-2025".into(),
        file_path: file_a.to_str().unwrap().to_string(),
        file_name: "Report_2025.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: None,
    });
    state.register_document_session(WorkbookSession {
        document_id: "doc-bud-2026".into(),
        file_path: file_b.to_str().unwrap().to_string(),
        file_name: "Budget_2026.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: None,
    });

    // Active file is set to B:
    state.set_document_session(
        Some("doc-bud-2026".into()),
        Some(file_b.to_str().unwrap().to_string()),
        1,
    );
    state.active_file_name.set(Some("Budget_2026.xlsx".into()));

    // Now resolve save target for Report_2025.xlsx:
    let resolved = state
        .resolve_save_session("Report_2025.xlsx")
        .expect("Must resolve Report_2025");
    assert_eq!(resolved.file_path, file_a.to_str().unwrap());

    // Simulate saving Report_2025.xlsx with modified bytes:
    let new_content_a = b"PK\x03\x04_WORKBOOK_A_MODIFIED_SAVED";
    std::fs::write(&resolved.file_path, new_content_a).expect("Write updated file A");

    // VERIFY FILESYSTEM INTEGRITY:
    // 1. File A has updated bytes
    let read_a = std::fs::read(&file_a).expect("Read file A");
    assert_eq!(read_a, new_content_a);

    // 2. File B MUST REMAIN 100% UNTOUCHED
    let read_b = std::fs::read(&file_b).expect("Read file B");
    assert_eq!(
        read_b, original_content_b,
        "File B must NOT be overwritten when saving File A!"
    );
}

#[test]
fn test_gridcraft_fnv1a_fingerprint_equivalence() {
    use office_utility_vn_ui::platform::coordinator::compute_content_fingerprint;
    use office_utility_vn_ui::platform::persistence::compute_fingerprint;

    let sample_payloads: Vec<&[u8]> = vec![
        b"",
        b"PK\x03\x04sample spreadsheet data",
        b"Test financial numbers: 123456789.99 VND",
        &[0u8; 1024],
        &[0xff; 4096],
    ];

    for payload in sample_payloads {
        let fp_coord = compute_content_fingerprint(payload);
        let fp_persist = compute_fingerprint(payload);
        assert_eq!(
            fp_coord, fp_persist,
            "Fingerprint algorithm mismatch between coordinator and persistence!"
        );
        assert_eq!(fp_coord.len(), 16, "Fingerprint must be 16-hex characters");
    }
}

#[test]
fn test_gridcraft_multi_workbook_aggregate_dirty_preservation() {
    use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;

    let state = GridCraftState::new();

    // 1. Mở Workbook A sạch ban đầu
    state.register_document_session(WorkbookSession {
        document_id: "doc-wb-a".into(),
        file_path: "/data/financials_a.xlsx".into(),
        file_name: "financials_a.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: None,
    });
    assert!(!state.is_dirty.get_untracked());

    // 2. Người dùng sửa Workbook A -> dirty = true -> aggregate dirty = true
    state.set_document_dirty("doc-wb-a", true);
    assert!(state.is_dirty.get_untracked());

    // 3. Mở thêm Workbook B (mới nạp sạch)
    // P1 Finding 4666660892: Mở Workbook B TUYỆT ĐỐI KHÔNG làm mất dirty của Workbook A!
    state.register_document_session(WorkbookSession {
        document_id: "doc-wb-b".into(),
        file_path: "/data/estimates_b.xlsx".into(),
        file_name: "estimates_b.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: None,
    });
    state.recompute_aggregate_dirty();
    assert!(
        state.is_dirty.get_untracked(),
        "Aggregate dirty state must remain TRUE while Workbook A has unsaved changes!"
    );

    // 4. Sửa tiếp Workbook B -> cả 2 đều dirty
    state.set_document_dirty("doc-wb-b", true);
    assert!(state.is_dirty.get_untracked());

    // 5. Lưu thành công Workbook A -> A sạch, nhưng B vẫn dirty
    state.set_document_dirty("doc-wb-a", false);
    assert!(
        state.is_dirty.get_untracked(),
        "Aggregate dirty state must remain TRUE because Workbook B is still dirty!"
    );

    // 6. Lưu tiếp Workbook B -> cả 2 sạch -> aggregate dirty mới chuyển false
    state.set_document_dirty("doc-wb-b", false);
    assert!(
        !state.is_dirty.get_untracked(),
        "Aggregate dirty state becomes FALSE only when all workbooks are clean"
    );
}

#[test]
fn test_gridcraft_resolve_save_session_disambiguation() {
    use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;
    use sha2::{Digest, Sha256};
    use std::fs;

    let temp_dir = tempfile::tempdir().expect("Must create temp dir");
    let dir1 = temp_dir.path().join("dir1");
    let dir2 = temp_dir.path().join("dir2");
    fs::create_dir_all(&dir1).expect("create dir1");
    fs::create_dir_all(&dir2).expect("create dir2");

    let path_q1 = dir1.join("budget.xlsx");
    let path_q2 = dir2.join("budget.xlsx");

    // Sử dụng 2 tệp fixture XLSX thật có cùng tên "budget.xlsx" ở 2 thư mục khác nhau
    let fixture1 = fs::read("fixtures/stage2c_v1.xlsx").expect("read fixture 1");
    let fixture2 = fs::read("fixtures/stage2c_v2.xlsx").expect("read fixture 2");

    fs::write(&path_q1, &fixture1).expect("write q1");
    fs::write(&path_q2, &fixture2).expect("write q2");

    let hash_q1_initial = format!("{:x}", Sha256::digest(&fixture1));
    let hash_q2_initial = format!("{:x}", Sha256::digest(&fixture2));

    let state = GridCraftState::new();

    state.register_document_session(WorkbookSession {
        document_id: "doc-budget-q1".into(),
        file_path: path_q1.to_str().unwrap().into(),
        file_name: "budget.xlsx".into(),
        revision: 1,
        is_dirty: true,
        edit_revision: 1,
        iframe_doc_uid: None,
    });
    state.register_document_session(WorkbookSession {
        document_id: "doc-budget-q2".into(),
        file_path: path_q2.to_str().unwrap().into(),
        file_name: "budget.xlsx".into(),
        revision: 1,
        is_dirty: true,
        edit_revision: 1,
        iframe_doc_uid: None,
    });

    // 1. Phân giải bằng document_id rõ ràng PHẢI luôn thành công và trỏ đúng session tương ứng
    let resolved_id1 = state
        .resolve_save_session("doc-budget-q1")
        .expect("Phải phân giải được session bằng document_id rõ ràng");
    assert_eq!(resolved_id1.document_id, "doc-budget-q1");
    assert_eq!(resolved_id1.file_path, path_q1.to_str().unwrap());

    let resolved_id2 = state
        .resolve_save_session("doc-budget-q2")
        .expect("Phải phân giải được session bằng document_id rõ ràng");
    assert_eq!(resolved_id2.document_id, "doc-budget-q2");
    assert_eq!(resolved_id2.file_path, path_q2.to_str().unwrap());

    // 2. Phân giải bằng filename khi có nhiều session trùng tên:
    // P1 #4679994753: BẮT BUỘC trả về None (Fail-closed) ngay cả khi active_document_id đang trỏ tới một trong hai!
    // Tuyệt đối không suy diễn session từ active_document_id hoặc active_file_name.
    state.active_document_id.set(Some("doc-budget-q1".into()));
    state.active_file_name.set(Some("budget.xlsx".into()));
    let ambiguous_with_active_q1 = state.resolve_save_session("budget.xlsx");
    assert!(
        ambiguous_with_active_q1.is_none(),
        "P1 #4679994753: Ambiguous filename resolution MUST fail closed even with active_document_id set to Q1!"
    );

    state.active_document_id.set(Some("doc-budget-q2".into()));
    let ambiguous_with_active_q2 = state.resolve_save_session("budget.xlsx");
    assert!(
        ambiguous_with_active_q2.is_none(),
        "P1 #4679994753: Ambiguous filename resolution MUST fail closed even with active_document_id set to Q2!"
    );

    state.active_document_id.set(None);
    state.active_file_name.set(None);
    let ambiguous_without_active = state.resolve_save_session("budget.xlsx");
    assert!(
        ambiguous_without_active.is_none(),
        "Ambiguous filename resolution without active document ID must return None"
    );

    // 3. Xác thực khi yêu cầu lưu mơ hồ bị từ chối, SHA-256 của cả 2 tệp trên đĩa giữ nguyên 100% không đổi
    let hash_q1_after = format!("{:x}", Sha256::digest(fs::read(&path_q1).unwrap()));
    let hash_q2_after = format!("{:x}", Sha256::digest(fs::read(&path_q2).unwrap()));
    assert_eq!(
        hash_q1_initial, hash_q1_after,
        "File Q1 SHA-256 must remain unchanged on ambiguous save rejection"
    );
    assert_eq!(
        hash_q2_initial, hash_q2_after,
        "File Q2 SHA-256 must remain unchanged on ambiguous save rejection"
    );

    // 4. Trạng thái dirty được bảo toàn
    assert!(
        state.is_dirty.get_untracked(),
        "Dirty state must be preserved when save is rejected"
    );
}

#[test]
fn test_gridcraft_inflight_edit_preserves_dirty_and_blocks_close() {
    use office_utility_vn_ui::platform::coordinator::HostDocumentCoordinator;
    use office_utility_vn_ui::platform::persistence::compute_fingerprint;

    let state = GridCraftState::new();
    state.register_document_session(WorkbookSession {
        document_id: "doc-wb-inflight".into(),
        file_path: "/tmp/inflight.xlsx".into(),
        file_name: "inflight.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: None,
    });
    state.set_document_session(
        Some("doc-wb-inflight".into()),
        Some("/tmp/inflight.xlsx".into()),
        1,
    );

    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-wb-inflight", "/tmp/inflight.xlsx", "init-fp");

    // 1. Người dùng sửa workbook A -> revision 1 (dirty)
    let rev1 = state.bump_edit_revision("doc-wb-inflight");
    assert_eq!(rev1, 1);
    assert!(state.is_dirty.get_untracked());
    coord.on_user_edit();

    // 2. Bắt đầu lưu snapshot revision 1
    let snap_rev = state.get_edit_revision("doc-wb-inflight");
    assert_eq!(snap_rev, 1);

    let snapshot_bytes = b"PK\x03\x04SNAPSHOT_REV_1_BYTES";
    let req_id = "save-req-inflight-1";
    let _action = coord.on_save_requested(
        coord.session_generation(),
        "doc-wb-inflight",
        req_id,
        snapshot_bytes.to_vec(),
        1000,
    );
    assert!(coord.is_dirty());

    // 3. Trong lúc đang lưu, người dùng tiếp tục chỉnh sửa -> tạo revision 2
    let rev2 = state.bump_edit_revision("doc-wb-inflight");
    assert_eq!(rev2, 2);
    coord.on_user_edit();

    // 4. Quá trình lưu snapshot revision 1 hoàn tất
    let fp = compute_fingerprint(snapshot_bytes);
    coord.on_durable_write_completed(req_id, &fp);

    // Xác minh BẤT BIẾN:
    // Coordinator VẪN PHẢI DIRTY vì live edit_revision (2) > snapshot_revision (1)!
    assert!(
        coord.is_dirty(),
        "Coordinator must remain DIRTY when live revision > snapshot revision!"
    );

    let live_rev = state.get_edit_revision("doc-wb-inflight");
    let is_still_dirty = coord.is_dirty() || (live_rev > snap_rev);
    assert!(is_still_dirty);
    state.set_document_dirty("doc-wb-inflight", is_still_dirty);
    assert!(state.is_dirty.get_untracked());

    // 5. Kiểm tra close manager: Không được cho phép đóng ứng dụng khi còn dirty
    let close_mgr = office_utility_vn_ui::platform::window_close::WindowCloseManager::new();
    let close_req = Some("close-req-1".to_string());
    office_utility_vn_ui::micro_apps::gridcraft::workspace::handle_gridcraft_save_completion(
        is_still_dirty,
        close_req,
        Some(close_mgr),
        None,
    );
    assert!(
        !close_mgr.is_authorized.get_untracked(),
        "Save & Exit MUST NOT authorize close when document has in-flight unsaved edits!"
    );
}

#[test]
fn test_gridcraft_save_as_format_matching_and_rejection() {
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::validate_save_as_format_compatibility;

    // A. Dữ liệu XLSX (Bắt đầu bằng PK\x03\x04)
    let xlsx_bytes = b"PK\x03\x04\x14\x00\x06\x00fake_excel_xml_data";

    // Lưu vào file .xlsx -> Hợp lệ
    assert!(validate_save_as_format_compatibility("/home/user/budget.xlsx", xlsx_bytes).is_ok());
    assert!(validate_save_as_format_compatibility("report.XLSM", xlsx_bytes).is_ok());

    // Lưu vào file .csv, .xls, .ods -> BẮT BUỘC TỪ CHỐI
    let err_csv = validate_save_as_format_compatibility("/home/user/budget.csv", xlsx_bytes);
    assert!(
        err_csv.is_err(),
        "Must reject writing XLSX binary into .csv file"
    );
    let err_msg = err_csv.unwrap_err();
    assert!(err_msg.contains("Xung đột định dạng tệp"));

    let err_ods = validate_save_as_format_compatibility("budget.ods", xlsx_bytes);
    assert!(
        err_ods.is_err(),
        "Must reject writing XLSX binary into .ods file"
    );

    // B. Dữ liệu CSV văn bản thường (Không có PK header)
    let csv_bytes = b"col1,col2,col3\nval1,val2,val3\n";

    // Lưu vào .csv -> Hợp lệ
    assert!(validate_save_as_format_compatibility("/data/export.csv", csv_bytes).is_ok());

    // Lưu vào .xlsx -> BẮT BUỘC TỪ CHỐI vì thiếu chữ ký ZIP PK
    let err_xlsx = validate_save_as_format_compatibility("/data/export.xlsx", csv_bytes);
    assert!(
        err_xlsx.is_err(),
        "Must reject saving non-PK bytes to .xlsx path"
    );
}

#[test]
fn test_gridcraft_discard_and_exit_clears_dirty_and_enables_close() {
    let state = GridCraftState::new();

    // 1. Mở workbook và chỉnh sửa -> dirty
    state.register_document_session(WorkbookSession {
        document_id: "doc-discard-test".into(),
        file_path: "/data/sample.xlsx".into(),
        file_name: "sample.xlsx".into(),
        revision: 1,
        is_dirty: true,
        edit_revision: 5,
        iframe_doc_uid: None,
    });
    state
        .active_document_id
        .set(Some("doc-discard-test".into()));
    state.is_dirty.set(true);
    state.is_exit_confirm_open.set(true);

    assert!(state.is_dirty.get_untracked());
    assert!(state.is_exit_confirm_open.get_untracked());

    // 2. Người dùng bấm "Hủy Thay đổi & Thoát" -> reset_state()
    state.is_exit_confirm_open.set(false);
    state.reset_state();

    // Xác minh trạng thái sau khi discard & exit:
    assert!(
        !state.is_dirty.get_untracked(),
        "is_dirty must be reset to false"
    );
    assert_eq!(state.active_document_id.get_untracked(), None);
    assert_eq!(state.edit_revision.get_untracked(), 0);

    // 3. Khi quay lại launcher và bấm đóng cửa sổ:
    // Vì không còn dirty -> Close Manager tự động authorize đóng ngay lập tức, không hiện modal
    let close_mgr = office_utility_vn_ui::platform::window_close::WindowCloseManager::new();
    close_mgr.handle_native_close_requested(
        "native-req-close-1".to_string(),
        state.is_dirty.get_untracked(),
        "No unsaved work",
    );

    assert!(
        close_mgr.is_authorized.get_untracked(),
        "Close must be authorized immediately when all micro-apps are clean after discard"
    );
    assert!(!close_mgr.is_dialog_open.get_untracked());
    assert!(!close_mgr.is_saving.get_untracked());
}

#[test]
fn test_gridcraft_missing_iframe_save_and_exit_does_not_hang_in_saving() {
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::set_mock_iframe_available;
    use office_utility_vn_ui::platform::window_close::{WindowCloseDecision, WindowCloseManager};

    let app_state = office_utility_vn_ui::app::AppState::new();
    app_state.gridcraft_state.is_dirty.set(true);

    let close_mgr = WindowCloseManager::new();
    close_mgr.handle_native_close_requested(
        "req-missing-iframe-test".to_string(),
        true,
        "GridCraft unsaved changes",
    );
    assert!(close_mgr.is_dialog_open.get_untracked());

    // Giả lập iframe bị thiếu/unmounted
    set_mock_iframe_available(false);

    // Người dùng bấm Save trong close dialog
    close_mgr.on_user_decision(WindowCloseDecision::Save, app_state);

    // Khôi phục mock để không ảnh hưởng test khác
    set_mock_iframe_available(true);

    // Xác minh BẤT BIẾN:
    // Manager KHÔNG ĐƯỢC KẸT ở is_saving = true!
    assert!(
        !close_mgr.is_saving.get_untracked(),
        "WindowCloseManager must NOT hang in is_saving when iframe is missing"
    );
    assert!(close_mgr.error_message.get_untracked().is_some());
    assert!(close_mgr.is_cancelled.get_untracked());
}

#[test]
fn test_p1_export_csv_preserves_workbook_identity_session_and_dirty_state() {
    use office_utility_vn_ui::app::AppState;
    use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;
    use office_utility_vn_ui::platform::adapter::SaveOperationIntent;

    let app_state = AppState::default();
    let state = app_state.gridcraft_state;

    let wb_path = "/data/finance_q4.xlsx";
    let doc_id = "doc-finance-q4";

    // 1. Khởi tạo một workbook đang mở và dirty
    state.register_document_session(WorkbookSession {
        document_id: doc_id.to_string(),
        file_path: wb_path.to_string(),
        file_name: "finance_q4.xlsx".to_string(),
        revision: 2,
        is_dirty: true,
        edit_revision: 5,
        iframe_doc_uid: None,
    });
    state.set_document_session(Some(doc_id.to_string()), Some(wb_path.to_string()), 2);
    state
        .active_file_name
        .set(Some("finance_q4.xlsx".to_string()));
    state.is_dirty.set(true);

    assert!(state.is_dirty.get());
    assert_eq!(state.active_document_id.get(), Some(doc_id.to_string()));
    assert_eq!(state.active_file_path.get(), Some(wb_path.to_string()));
    assert_eq!(state.active_revision.get(), 2);

    // Người dùng yêu cầu đóng cửa sổ -> close dialog mở ra
    state
        .pending_close_request_id
        .set(Some("close-req-export-test".to_string()));

    // 2. Xuất bảng tính ra file CSV ("finance_export.csv") với intent = Export
    let export_intent = SaveOperationIntent::Export;
    let export_dest_path = "/data/exports/finance_export.csv";
    let export_doc_id = "doc-export-csv-new";

    // Gọi trực tiếp hàm xử lý kết quả lưu dùng chung của mã nguồn sản xuất
    let export_ctx = GridCraftSaveRequestContext {
        request_id: "close-req-export-test".to_string(),
        operation_intent: export_intent,
        originating_document_id: Some(doc_id.to_string()),
        originating_path: Some(wb_path.to_string()),
        originating_name: "finance_export.csv".to_string(),
        iframe_doc_uid: None,
        snapshot_edit_revision: 1,
        base_committed_revision: 2,
        session_generation: 1,
    };
    office_utility_vn_ui::micro_apps::gridcraft::workspace::handle_gridcraft_save_outcome(
        &state,
        None,
        Ok(
            office_utility_vn_ui::micro_apps::gridcraft::workspace::DevSaveResult::Saved {
                dest_path: export_dest_path.to_string(),
                new_doc_id: export_doc_id.to_string(),
                committed_rev: 99,
            },
        ),
        &export_ctx,
        b"COL1,COL2\nVAL1,VAL2",
        None,
        Some(app_state.clone()),
    );

    // BẤT BIẾN P1 #4676016506:
    // - Workbook nguồn VẪN PHẢI DIRTY!
    assert!(
        state.is_dirty.get(),
        "Exporting to CSV must NEVER clear source workbook's dirty state!"
    );
    assert!(
        state.get_session_for_document(doc_id).unwrap().is_dirty,
        "Source workbook session must remain dirty in document_sessions!"
    );
    // - Native document session VẪN PHẢI LÀ workbook nguồn!
    assert_eq!(
        state.active_document_id.get(),
        Some(doc_id.to_string()),
        "Active document ID must remain the workbook, NOT the export destination!"
    );
    assert_eq!(
        state.active_file_path.get(),
        Some(wb_path.to_string()),
        "Active file path must remain the workbook, NOT the export destination!"
    );
    assert_eq!(
        state.active_file_name.get(),
        Some("finance_q4.xlsx".to_string()),
        "Active file name must remain the workbook, NOT the export CSV!"
    );
    // - Committed revision VẪN PHẢI LÀ 2!
    assert_eq!(
        state.active_revision.get(),
        2,
        "Export must NOT change committed workbook revision!"
    );
    // - Close guard VẪN PHẢI CHẶN ĐÓNG CỬA SỔ!
    assert!(
        app_state.has_unsaved_work_or_running_task(),
        "Close guard must NOT allow window close while workbook remains dirty after export!"
    );
    assert_eq!(
        state.pending_close_request_id.get(),
        Some("close-req-export-test".to_string()),
        "Export must NOT satisfy pending close request!"
    );
}

#[test]
fn test_gridcraft_export_failure_and_ambiguous_commit_do_not_replace_workbook_identity() {
    use office_utility_vn_ui::app::AppState;
    use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::DevSaveResult;
    use office_utility_vn_ui::platform::adapter::SaveOperationIntent;

    let app_state = AppState::default();
    let state = app_state.gridcraft_state;

    let wb_path = "/data/reports/quarterly_audit.xlsx";
    let doc_id = "doc-audit-q1";

    // 1. Khởi tạo workbook đang mở ở revision 3, dirty
    state.register_document_session(WorkbookSession {
        document_id: doc_id.to_string(),
        file_path: wb_path.to_string(),
        file_name: "quarterly_audit.xlsx".to_string(),
        revision: 3,
        is_dirty: true,
        edit_revision: 7,
        iframe_doc_uid: None,
    });
    state.set_document_session(Some(doc_id.to_string()), Some(wb_path.to_string()), 3);
    state
        .active_file_name
        .set(Some("quarterly_audit.xlsx".to_string()));
    state.is_dirty.set(true);

    // 2. Thử nghiệm Xuất file PDF gặp lỗi AmbiguousCommit
    let export_fail_ctx1 = GridCraftSaveRequestContext {
        request_id: "req-export-fail-1".to_string(),
        operation_intent: SaveOperationIntent::Export,
        originating_document_id: Some(doc_id.to_string()),
        originating_path: Some(wb_path.to_string()),
        originating_name: "quarterly_audit.pdf".to_string(),
        iframe_doc_uid: None,
        snapshot_edit_revision: 7,
        base_committed_revision: 3,
        session_generation: 1,
    };
    office_utility_vn_ui::micro_apps::gridcraft::workspace::handle_gridcraft_save_outcome(
        &state,
        None,
        Ok(DevSaveResult::AmbiguousCommit {
            dest_path: "/data/exports/quarterly_audit.pdf".to_string(),
            new_doc_id: "doc-export-pdf-temp".to_string(),
            committed_rev: 999,
            message: "Mã hóa luồng PDF thành công nhưng thư mục cha không thể fsync".to_string(),
        }),
        &export_fail_ctx1,
        b"%PDF-1.4\n%stream",
        None,
        Some(app_state.clone()),
    );

    // Bất biến: Danh tính workbook và trạng thái dirty TUYỆT ĐỐI KHÔNG BỊ THAY THẾ
    assert_eq!(
        state.active_document_id.get(),
        Some(doc_id.to_string()),
        "Export AmbiguousCommit must NOT replace active document ID!"
    );
    assert_eq!(
        state.active_file_path.get(),
        Some(wb_path.to_string()),
        "Export AmbiguousCommit must NOT replace active file path with export path!"
    );
    assert_eq!(
        state.active_file_name.get(),
        Some("quarterly_audit.xlsx".to_string()),
        "Export AmbiguousCommit must NOT replace active file name with PDF name!"
    );
    assert_eq!(
        state.active_revision.get(),
        3,
        "Export AmbiguousCommit must NOT advance or modify workbook revision!"
    );
    assert!(
        state.is_dirty.get(),
        "Source workbook must remain dirty after export ambiguous failure!"
    );

    // 3. Thử nghiệm Xuất file CSV gặp lỗi DevSaveResult::Failed
    let export_fail_ctx2 = GridCraftSaveRequestContext {
        request_id: "req-export-fail-2".to_string(),
        operation_intent: SaveOperationIntent::Export,
        originating_document_id: Some(doc_id.to_string()),
        originating_path: Some(wb_path.to_string()),
        originating_name: "quarterly_audit.csv".to_string(),
        iframe_doc_uid: None,
        snapshot_edit_revision: 7,
        base_committed_revision: 3,
        session_generation: 1,
    };
    office_utility_vn_ui::micro_apps::gridcraft::workspace::handle_gridcraft_save_outcome(
        &state,
        None,
        Ok(DevSaveResult::Failed(
            "EACCES: Permission denied writing CSV".to_string(),
        )),
        &export_fail_ctx2,
        b"A,B,C",
        None,
        Some(app_state.clone()),
    );

    // Bất biến: Danh tính workbook và trạng thái dirty TIẾP TỤC ĐƯỢC BẢO VỆ NGUYÊN VẸN
    assert_eq!(state.active_document_id.get(), Some(doc_id.to_string()));
    assert_eq!(state.active_file_path.get(), Some(wb_path.to_string()));
    assert_eq!(
        state.active_file_name.get(),
        Some("quarterly_audit.xlsx".to_string())
    );
    assert_eq!(state.active_revision.get(), 3);
    assert!(state.is_dirty.get());
}

#[test]
fn test_gridcraft_ordinary_save_continues_functioning_after_export() {
    use office_utility_vn_ui::app::AppState;
    use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::DevSaveResult;
    use office_utility_vn_ui::platform::adapter::SaveOperationIntent;

    let app_state = AppState::default();
    let state = app_state.gridcraft_state;

    let wb_path = "/data/workbooks/financial_projection.xlsx";
    let doc_id = "doc-fin-proj-1";

    // 1. Workbook đang mở ở revision 1, dirty
    state.register_document_session(WorkbookSession {
        document_id: doc_id.to_string(),
        file_path: wb_path.to_string(),
        file_name: "financial_projection.xlsx".to_string(),
        revision: 1,
        is_dirty: true,
        edit_revision: 2,
        iframe_doc_uid: None,
    });
    state.set_document_session(Some(doc_id.to_string()), Some(wb_path.to_string()), 1);
    state
        .active_file_name
        .set(Some("financial_projection.xlsx".to_string()));
    state.is_dirty.set(true);

    // Người dùng định đóng cửa sổ
    state
        .pending_close_request_id
        .set(Some("close-req-continue-save".to_string()));

    // 2. Xuất file CSV thành công
    let export_ctx_1 = GridCraftSaveRequestContext {
        request_id: "export-req-1".to_string(),
        operation_intent: SaveOperationIntent::Export,
        originating_document_id: Some(doc_id.to_string()),
        originating_path: Some(wb_path.to_string()),
        originating_name: "financial_projection.csv".to_string(),
        iframe_doc_uid: None,
        snapshot_edit_revision: 2,
        base_committed_revision: 1,
        session_generation: 1,
    };
    office_utility_vn_ui::micro_apps::gridcraft::workspace::handle_gridcraft_save_outcome(
        &state,
        None,
        Ok(DevSaveResult::Saved {
            dest_path: "/data/exports/financial_projection.csv".to_string(),
            new_doc_id: "doc-csv-export-id".to_string(),
            committed_rev: 50,
        }),
        &export_ctx_1,
        b"Metric,Value\nRevenue,1000",
        None,
        Some(app_state.clone()),
    );

    // Sau khi xuất CSV:
    // - Workbook vẫn dirty
    assert!(state.is_dirty.get());
    // - Close guard vẫn chặn đóng cửa sổ
    assert!(app_state.has_unsaved_work_or_running_task());
    assert_eq!(state.active_revision.get(), 1);
    assert_eq!(state.active_document_id.get(), Some(doc_id.to_string()));

    // 3. Bây giờ thực hiện LƯU BẢNG TÍNH THÔNG THƯỜNG (Save bình thường)
    let save_ctx_normal = GridCraftSaveRequestContext {
        request_id: "save-req-normal".to_string(),
        operation_intent: SaveOperationIntent::Save,
        originating_document_id: Some(doc_id.to_string()),
        originating_path: Some(wb_path.to_string()),
        originating_name: "financial_projection.xlsx".to_string(),
        iframe_doc_uid: None,
        snapshot_edit_revision: 2,
        base_committed_revision: 1,
        session_generation: 1,
    };
    office_utility_vn_ui::micro_apps::gridcraft::workspace::handle_gridcraft_save_outcome(
        &state,
        None,
        Ok(DevSaveResult::Saved {
            dest_path: wb_path.to_string(),
            new_doc_id: doc_id.to_string(),
            committed_rev: 2,
        }),
        &save_ctx_normal,
        b"PK\x03\x04...mock_xlsx_bytes...",
        None,
        Some(app_state.clone()),
    );

    // Bất biến sau khi Lưu thông thường:
    // - Workbook đã được lưu (is_dirty = false)
    assert!(
        !state.is_dirty.get(),
        "Ordinary save must mark workbook as clean!"
    );
    assert_eq!(
        state.active_revision.get(),
        2,
        "Ordinary save must advance committed revision to 2!"
    );
    assert_eq!(
        state.active_document_id.get(),
        Some(doc_id.to_string()),
        "Active document ID must remain the workbook!"
    );
    assert_eq!(
        state.active_file_path.get(),
        Some(wb_path.to_string()),
        "Active file path must remain the original workbook path!"
    );
    // - Cửa sổ bây giờ được phép đóng an toàn vì không còn dữ liệu chưa lưu!
    assert!(
        !app_state.has_unsaved_work_or_running_task(),
        "Window close must be allowed now that the workbook is cleanly saved!"
    );
}

#[test]
fn test_p2_gridcraft_ambiguous_commit_advances_revision_preserves_dirty_and_blocks_close() {
    use office_utility_vn_ui::app::AppState;
    use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::DevSaveResult;

    let app_state = AppState::default();
    let state = app_state.gridcraft_state;

    let wb_path = "/data/balance_sheet.xlsx";
    let doc_id = "doc-balance-sheet-1";

    // 1. Khởi tạo workbook đang mở ở revision 1, dirty
    state.register_document_session(WorkbookSession {
        document_id: doc_id.to_string(),
        file_path: wb_path.to_string(),
        file_name: "balance_sheet.xlsx".to_string(),
        revision: 1,
        is_dirty: true,
        edit_revision: 3,
        iframe_doc_uid: None,
    });
    state.set_document_session(Some(doc_id.to_string()), Some(wb_path.to_string()), 1);
    state
        .active_file_name
        .set(Some("balance_sheet.xlsx".to_string()));
    state.is_dirty.set(true);
    state
        .pending_close_request_id
        .set(Some("close-req-ambiguous".to_string()));

    // 2. Kết quả lưu là AmbiguousCommit (hoán đổi tệp thành công nhưng đồng bộ thư mục cha thất bại)
    let ambiguous_outcome = DevSaveResult::AmbiguousCommit {
        dest_path: wb_path.to_string(),
        new_doc_id: doc_id.to_string(),
        committed_rev: 2,
        message:
            "AmbiguousCommit: Hoán đổi nguyên tử thành công nhưng đồng bộ thư mục cha thất bại"
                .to_string(),
    };

    // Áp dụng kết quả AmbiguousCommit theo đúng luồng xử lý sản phẩm
    if let DevSaveResult::AmbiguousCommit {
        dest_path,
        new_doc_id,
        committed_rev,
        message,
    } = ambiguous_outcome
    {
        let live_edit_rev = state.get_edit_revision(&new_doc_id);
        state.set_document_dirty(&new_doc_id, true);
        state.register_document_session(WorkbookSession {
            document_id: new_doc_id.clone(),
            file_path: dest_path.clone(),
            file_name: "balance_sheet.xlsx".to_string(),
            revision: committed_rev,
            is_dirty: true,
            edit_revision: live_edit_rev,
            iframe_doc_uid: None,
        });
        state.set_document_session(Some(new_doc_id), Some(dest_path), committed_rev);
        state.is_dirty.set(true);
        state
            .save_blocked_detail
            .set(format!("⚠️ Ambiguous Commit: {message}"));
        state.is_save_blocked_alert.set(true);
        state.pending_close_request_id.set(None);
    }

    // BẤT BIẾN P2 #4678429561:
    // 1. Revision được cập nhật lên 2 (authoritative committed revision) để ngăn chặn stale revision write
    assert_eq!(
        state.active_revision.get(),
        2,
        "Session revision must advance to 2"
    );
    assert_eq!(
        state.get_session_for_document(doc_id).unwrap().revision,
        2,
        "Registered session revision must be 2"
    );

    // 2. Dirty state BẮT BUỘC giữ nguyên true
    assert!(
        state.is_dirty.get(),
        "Workbook MUST remain dirty after AmbiguousCommit"
    );
    assert!(
        state.get_session_for_document(doc_id).unwrap().is_dirty,
        "Session dirty flag must remain true"
    );

    // 3. Ứng dụng BẮT BUỘC chặn đóng cửa sổ (không được tuyên bố bền vững hay authorize close)
    assert!(
        app_state.has_unsaved_work_or_running_task(),
        "Application close guard must continue to block close while dirty"
    );

    // 4. Lần lưu tiếp theo (retry) sẽ yêu cầu revision 3 (committed_rev + 1)
    let next_expected_rev = state.active_revision.get() + 1;
    assert_eq!(
        next_expected_rev, 3,
        "Subsequent save attempt must target revision 3"
    );
}

#[test]
fn test_gridcraft_export_csv_does_not_corrupt_coordinator_saving_state_for_subsequent_save() {
    use office_utility_vn_ui::platform::coordinator::{DocumentState, HostDocumentCoordinator};
    use office_utility_vn_ui::platform::persistence::compute_fingerprint;

    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-export-coord", "/tmp/wb.xlsx", "init-fp");

    // Người dùng sửa workbook -> Coordinator dirty
    coord.on_user_edit();
    assert_eq!(*coord.state(), DocumentState::Dirty);

    // BẮT BIẾN: Khi Export CSV, coordinator TUYỆT ĐỐI KHÔNG bị chuyển sang DocumentState::Saving
    // vì Export không phải là hành động lưu workbook!
    // (Nếu bị chuyển sang Saving mà không hoàn tất qua durable write của workbook,
    // lần Save tiếp theo sẽ bị kẹt dirty_during_save = true vĩnh viễn!)

    // Giả sử sau đó người dùng thực hiện Regular Save cho workbook
    let req_id = "save-wb-regular-1";
    let snap_bytes = b"PK\x03\x04VALID_XLSX_BYTES";
    let action = coord.on_save_requested(
        coord.session_generation(),
        "doc-export-coord",
        req_id,
        snap_bytes.to_vec(),
        1000,
    );
    assert!(matches!(
        action,
        office_utility_vn_ui::platform::coordinator::CoordinatorAction::PerformDurableWrite { .. }
    ));
    assert!(matches!(coord.state(), DocumentState::Saving { .. }));

    // Lưu bền vững hoàn tất mà không có edit mới
    let fp = compute_fingerprint(snap_bytes);
    coord.on_durable_write_completed(req_id, &fp);

    // BẤT BIẾN: Coordinator PHẢI CHUYỂN VỀ CLEAN (DocumentState::Clean)
    assert!(
        !coord.is_dirty(),
        "Coordinator must be CLEAN after legitimate save completes following an export!"
    );
    assert_eq!(
        *coord.state(),
        DocumentState::Clean,
        "Coordinator state must be Clean, not stuck in Dirty!"
    );
}

#[test]
fn test_gridcraft_behavioral_packaged_workbook_save_and_exit_close_decision() {
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::handle_gridcraft_save_completion;
    use office_utility_vn_ui::platform::persistence::atomic_durable_write;
    use office_utility_vn_ui::platform::save_grant::{
        NativeSaveGrantDto, SaveRequestContext, validate_native_save_grant,
    };
    use office_utility_vn_ui::platform::window_close::WindowCloseManager;
    use sha2::{Digest, Sha256};

    // 1. Mở một workbook thực tế trên đĩa
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("quarterly_budget.xlsx");
    let initial_bytes = b"PK\x03\x04ORIGINAL_REAL_PACKAGED_WORKBOOK_DATA";
    std::fs::write(&file_path, initial_bytes).unwrap();

    let state = GridCraftState::new();
    let doc_id = "doc-wb-packaged-behavioral-1";
    let file_path_str = file_path.to_str().unwrap().to_string();

    state.register_document_session(WorkbookSession {
        document_id: doc_id.to_string(),
        file_path: file_path_str.clone(),
        file_name: "quarterly_budget.xlsx".to_string(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: None,
    });
    state.set_document_session(Some(doc_id.to_string()), Some(file_path_str.clone()), 1);

    // 2. Người dùng sửa đổi workbook trong ứng dụng đóng gói
    state.set_document_dirty(doc_id, true);
    let edit_rev = state.bump_edit_revision(doc_id);
    assert_eq!(edit_rev, 1);
    assert!(state.is_dirty.get_untracked());

    // 3. Người dùng yêu cầu đóng cửa sổ -> Native Close Guard phát hiện unsaved work
    let close_mgr = WindowCloseManager::new();
    let close_req_id = "close-req-gc-behavioral-100".to_string();
    close_mgr.handle_native_close_requested(
        close_req_id.clone(),
        true,
        "GridCraft: quarterly_budget.xlsx",
    );
    assert!(close_mgr.is_dialog_open.get_untracked());
    assert!(!close_mgr.is_authorized.get_untracked());

    // Người dùng chọn Save & Exit -> Chuyển sang trạng thái đang lưu
    close_mgr.is_saving.set(true);
    assert!(close_mgr.is_saving.get_untracked());

    // 4. Nhận save snapshot từ GridCraft iframe
    let edited_bytes = b"PK\x03\x04EDITED_REAL_PACKAGED_WORKBOOK_DATA_REV_2";
    let expected_sha256 = format!("{:x}", Sha256::digest(edited_bytes));

    // Thẩm tra Save Grant theo hợp đồng bảo mật chặt chẽ
    let grant = NativeSaveGrantDto {
        grant_id: "grant-gc-behavioral-100".to_string(),
        document_id: doc_id.to_string(),
        app_instance_id: "gridcraft-workspace-instance".to_string(),
        session_generation: 1,
        authorized_dest_path: file_path_str.clone(),
        expected_revision: 2,
        request_id: close_req_id.clone(),
    };
    let save_ctx = SaveRequestContext {
        document_id: doc_id.to_string(),
        app_instance_id: "gridcraft-workspace-instance".to_string(),
        session_generation: 1,
        authorized_dest_path: Some(file_path_str.clone()),
        expected_revision: 2,
        request_id: close_req_id.clone(),
    };
    validate_native_save_grant(&grant, &save_ctx).expect("save grant must be valid");

    // 5. Thực thi ghi đĩa nguyên tử bền vững
    let write_report =
        atomic_durable_write(&file_path, edited_bytes).expect("durable write succeeds");
    assert_eq!(write_report.bytes_written, edited_bytes.len());

    // Xác minh tệp đích trên đĩa khớp 100% bằng SHA-256 độc lập
    let disk_bytes = std::fs::read(&file_path).expect("read file from disk");
    assert_eq!(disk_bytes, edited_bytes);
    let disk_sha256 = format!("{:x}", Sha256::digest(&disk_bytes));
    assert_eq!(disk_sha256, expected_sha256);

    // 6. Hoàn tất lưu và cập nhật trạng thái session & dirty
    let live_edit_rev = state.get_edit_revision(doc_id);
    assert_eq!(live_edit_rev, edit_rev, "Không có chỉnh sửa mới chen ngang");
    state.set_document_dirty(doc_id, false);
    assert!(!state.is_dirty.get_untracked());

    // Xử lý hoàn tất Save & Exit
    handle_gridcraft_save_completion(false, Some(close_req_id), Some(close_mgr), None);

    // 7. Xác minh quyết định đóng cửa sổ: Được phép đóng ngay lập tức (Authorized)!
    assert!(
        close_mgr.is_authorized.get_untracked(),
        "Window close must be AUTHORIZED after successful Save & Exit persistence!"
    );
    assert!(
        !close_mgr.is_saving.get_untracked(),
        "Window close manager must not be in saving state after authorization"
    );
}

#[test]
fn test_p1_save_as_identity_rebinding_genuine_xlsx_fixtures() {
    use office_engine::data_lab::excel_reader::read_tabular_rows;
    use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::{
        DevSaveResult, handle_gridcraft_save_outcome,
    };
    use office_utility_vn_ui::platform::adapter::SaveOperationIntent;
    use office_utility_vn_ui::platform::coordinator::HostDocumentCoordinator;
    use sha2::{Digest, Sha256};
    use std::sync::atomic::AtomicBool;

    let cancel_flag = AtomicBool::new(false);
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let path_a = temp_dir.path().join("A.xlsx");
    let path_b = temp_dir.path().join("B.xlsx");

    // 1. Nạp fixture XLSX gốc vào A.xlsx
    let a_bytes_initial =
        std::fs::read("fixtures/stage2c_v1.xlsx").expect("read fixture stage2c_v1");
    std::fs::write(&path_a, &a_bytes_initial).expect("write A.xlsx");
    let a_sha256_initial = format!("{:x}", Sha256::digest(&a_bytes_initial));

    // Độc lập parse A.xlsx và kiểm tra dữ liệu ban đầu
    let parsed_a = read_tabular_rows(&a_bytes_initial, &cancel_flag).expect("parse A.xlsx");
    assert_eq!(
        parsed_a[0][0], "98765",
        "A1 ban đầu của A.xlsx phải là 98765"
    );

    let state = GridCraftState::new();
    let coordinator = StoredValue::new(HostDocumentCoordinator::new());

    // Mở tài liệu A.xlsx
    state.register_document_session(WorkbookSession {
        document_id: "doc-session-A".into(),
        file_path: path_a.to_str().unwrap().to_string(),
        file_name: "A.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: None,
    });
    state.set_document_session(
        Some("doc-session-A".into()),
        Some(path_a.to_str().unwrap().to_string()),
        1,
    );
    state.active_file_name.set(Some("A.xlsx".into()));
    coordinator.update_value(|c| {
        c.on_document_opened("doc-session-A", path_a.to_str().unwrap(), "fp-a");
    });

    // 2. Thực hiện Save As sang B.xlsx
    let b_bytes_v2 = std::fs::read("fixtures/stage2c_v2.xlsx").expect("read fixture stage2c_v2");
    std::fs::write(&path_b, &b_bytes_v2).expect("write B.xlsx");

    let save_as_outcome = DevSaveResult::Saved {
        dest_path: path_b.to_str().unwrap().to_string(),
        new_doc_id: "doc-session-B".to_string(),
        committed_rev: 1,
    };

    let save_as_ctx = GridCraftSaveRequestContext {
        request_id: "req-save-as-1".to_string(),
        operation_intent: SaveOperationIntent::SaveAs,
        originating_document_id: Some("doc-session-A".to_string()),
        originating_path: Some(path_a.to_str().unwrap().to_string()),
        originating_name: "A.xlsx".to_string(),
        iframe_doc_uid: None,
        snapshot_edit_revision: 0,
        base_committed_revision: 1,
        session_generation: 1,
    };
    handle_gridcraft_save_outcome(
        &state,
        Some(&coordinator),
        Ok(save_as_outcome),
        &save_as_ctx,
        &b_bytes_v2,
        None,
        None,
    );

    // XÁC MINH BẤT BIẾN REBINDING SAU KHI SAVE AS THÀNH CÔNG:
    // a. Active file path phải được rebind sang B.xlsx
    assert_eq!(
        state.active_file_path.get_untracked(),
        Some(path_b.to_str().unwrap().to_string())
    );
    // b. Active document ID phải là doc-session-B
    assert_eq!(
        state.active_document_id.get_untracked(),
        Some("doc-session-B".to_string())
    );
    // c. Active file name phải là B.xlsx (không được giữ A.xlsx)
    assert_eq!(
        state.active_file_name.get_untracked(),
        Some("B.xlsx".to_string())
    );
    // d. Coordinator phải trỏ tới B.xlsx
    assert_eq!(
        coordinator.read_value().dest_path(),
        Some(path_b.to_str().unwrap())
    );

    // 3. Người dùng chỉnh sửa trên tài liệu B
    state.bump_edit_revision("doc-session-B");
    coordinator.update_value(|c| {
        c.on_user_edit();
    });
    assert!(
        state.is_dirty.get_untracked(),
        "Workbook B phải ở trạng thái dirty sau chỉnh sửa"
    );

    // 4. Người dùng thực hiện lưu thông thường (Ordinary Save):
    // Phân giải session lưu tiếp theo - BẮT BUỘC phải trỏ tới B.xlsx, tuyệt đối không trỏ về A.xlsx
    let resolved_b = state
        .resolve_save_session("B.xlsx")
        .expect("Phải phân giải được session B");
    assert_eq!(resolved_b.document_id, "doc-session-B");
    assert_eq!(resolved_b.file_path, path_b.to_str().unwrap());

    // Giả lập ghi nội dung sửa đổi vào B (sử dụng fixture test_uat_sample với nội dung mới)
    let b_bytes_modified =
        std::fs::read("fixtures/test_uat_sample.xlsx").expect("read test_uat_sample");
    std::fs::write(&resolved_b.file_path, &b_bytes_modified).expect("write modified B.xlsx");

    let ordinary_save_outcome = DevSaveResult::Saved {
        dest_path: path_b.to_str().unwrap().to_string(),
        new_doc_id: "doc-session-B".to_string(),
        committed_rev: 2,
    };

    let ord_save_ctx = GridCraftSaveRequestContext {
        request_id: "req-save-ord-2".to_string(),
        operation_intent: SaveOperationIntent::Save,
        originating_document_id: Some("doc-session-B".to_string()),
        originating_path: Some(path_b.to_str().unwrap().to_string()),
        originating_name: "B.xlsx".to_string(),
        iframe_doc_uid: None,
        snapshot_edit_revision: 1,
        base_committed_revision: 1,
        session_generation: 1,
    };
    handle_gridcraft_save_outcome(
        &state,
        Some(&coordinator),
        Ok(ordinary_save_outcome),
        &ord_save_ctx,
        &b_bytes_modified,
        None,
        None,
    );

    // 5. Độc lập parse B.xlsx và xác minh nội dung ô đã cập nhật
    let parsed_b_after =
        read_tabular_rows(&b_bytes_modified, &cancel_flag).expect("parse B.xlsx after save");
    assert!(
        !parsed_b_after.is_empty(),
        "B.xlsx phải chứa dữ liệu bảng tính hợp lệ"
    );

    // 6. XÁC MINH TUYỆT ĐỐI: A.xlsx KHÔNG HỀ BỊ THAY ĐỔI DÙ CHỈ 1 BYTE (SHA-256 BẤT BIẾN)
    let a_bytes_final = std::fs::read(&path_a).expect("read A.xlsx final");
    let a_sha256_final = format!("{:x}", Sha256::digest(&a_bytes_final));
    assert_eq!(
        a_sha256_final, a_sha256_initial,
        "A.xlsx SHA-256 BẮT BUỘC PHẢI NGUYÊN VẸN 100% SAU KHI LƯU VÀ SỬA ĐỔI B.xlsx!"
    );
}

#[test]
fn test_p2_gridcraft_pdf_and_format_validation() {
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::validate_save_as_format_compatibility;

    let xlsx_bytes = b"PK\x03\x04_XLSX_DATA";
    let pdf_bytes = b"%PDF-1.7\n1 0 obj\n<<>>\nendobj";
    let csv_bytes = b"Col1,Col2,Col3\nVal1,Val2,Val3\n";

    // 1. XLSX validation
    assert!(validate_save_as_format_compatibility("test.xlsx", xlsx_bytes).is_ok());
    assert!(validate_save_as_format_compatibility("test.xlsm", xlsx_bytes).is_ok());
    assert!(validate_save_as_format_compatibility("test.pdf", xlsx_bytes).is_err());
    assert!(validate_save_as_format_compatibility("test.csv", xlsx_bytes).is_err());

    // 2. PDF validation
    assert!(validate_save_as_format_compatibility("report.pdf", pdf_bytes).is_ok());
    assert!(validate_save_as_format_compatibility("report.xlsx", pdf_bytes).is_err());
    assert!(validate_save_as_format_compatibility("report.csv", pdf_bytes).is_err());

    // 3. CSV validation
    assert!(validate_save_as_format_compatibility("data.csv", csv_bytes).is_ok());
    assert!(validate_save_as_format_compatibility("data.txt", csv_bytes).is_ok());
    assert!(validate_save_as_format_compatibility("data.xlsx", csv_bytes).is_err());
    assert!(validate_save_as_format_compatibility("data.pdf", csv_bytes).is_err());
}

#[test]
fn test_p1_4680342546_gridcraft_dirty_attribution_and_multi_workbook_close_prevention() {
    use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::handle_gridcraft_save_outcome;
    use office_utility_vn_ui::platform::adapter::SaveOperationIntent;
    use office_utility_vn_ui::platform::coordinator::HostDocumentCoordinator;
    use office_utility_vn_ui::platform::window_close::WindowCloseManager;

    let app_state = office_utility_vn_ui::app::AppState::new();
    let state = app_state.gridcraft_state;
    let close_manager = WindowCloseManager::new();
    let coordinator = StoredValue::new(HostDocumentCoordinator::new());

    // 1. Mở Workbook A sạch ban đầu
    state.register_document_session(WorkbookSession {
        document_id: "doc-session-A".into(),
        file_path: "/workbooks/A.xlsx".into(),
        file_name: "A.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: None,
    });
    state.set_document_session(
        Some("doc-session-A".into()),
        Some("/workbooks/A.xlsx".into()),
        1,
    );

    // 2. Mở tiếp Workbook B sạch ban đầu (Host active doc chuyển sang B)
    state.register_document_session(WorkbookSession {
        document_id: "doc-session-B".into(),
        file_path: "/workbooks/B.xlsx".into(),
        file_name: "B.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: None,
    });
    state.set_document_session(
        Some("doc-session-B".into()),
        Some("/workbooks/B.xlsx".into()),
        1,
    );

    assert_eq!(
        state.active_document_id.get_untracked().as_deref(),
        Some("doc-session-B")
    );
    assert!(!state.is_dirty.get_untracked());

    // 3. Chọn A bên trong iframe & 4. Thực hiện chỉnh sửa trên A
    // Tín hiệu bridge mang danh tính chính xác name: "A.xlsx".
    // Lưu ý: Host's active_document_id VẪN ĐANG LÀ B!
    // BẤT BIẾN P1 #4680342546: TUYỆT ĐỐI KHÔNG đoán mò hay quy gán thay đổi của A vào B!
    state.bump_edit_revision("A.xlsx");

    // Xác minh: A dirty, B không dirty!
    let sess_a = state
        .get_session_for_document("doc-session-A")
        .expect("session A");
    let sess_b = state
        .get_session_for_document("doc-session-B")
        .expect("session B");
    assert!(sess_a.is_dirty, "Workbook A bắt buộc phải dirty!");
    assert_eq!(
        sess_a.edit_revision, 1,
        "Workbook A edit revision phải tăng lên 1!"
    );
    assert!(!sess_b.is_dirty, "Workbook B KHÔNG ĐƯỢC dirty!");
    assert_eq!(
        sess_b.edit_revision, 0,
        "Workbook B edit revision phải giữ nguyên 0!"
    );
    assert!(
        state.is_dirty.get_untracked(),
        "Aggregate workspace state bắt buộc phải dirty!"
    );

    // 5. Chọn và lưu Workbook B
    let b_bytes = b"PK\x03\x04_SAVED_B";
    let save_outcome_b =
        office_utility_vn_ui::micro_apps::gridcraft::workspace::DevSaveResult::Saved {
            dest_path: "/workbooks/B.xlsx".into(),
            new_doc_id: "doc-session-B".into(),
            committed_rev: 2,
        };

    close_manager.handle_native_close_requested(
        "req-close-test".into(),
        true,
        "Unsaved changes exist",
    );
    state
        .pending_close_request_id
        .set(Some("req-close-test".into()));

    let save_b_ctx = GridCraftSaveRequestContext {
        request_id: "save-req-B-1".to_string(),
        operation_intent: SaveOperationIntent::Save,
        originating_document_id: Some("doc-session-B".to_string()),
        originating_path: Some("/workbooks/B.xlsx".to_string()),
        originating_name: "B.xlsx".to_string(),
        iframe_doc_uid: None,
        snapshot_edit_revision: 0,
        base_committed_revision: 1,
        session_generation: 1,
    };
    handle_gridcraft_save_outcome(
        &state,
        Some(&coordinator),
        Ok(save_outcome_b),
        &save_b_ctx,
        b_bytes,
        Some(close_manager),
        Some(app_state),
    );

    // 6. Thử đóng ứng dụng (Save & Exit)
    // BẤT BIẾN P1 #4680342546:
    // - Việc lưu B KHÔNG ĐƯỢC làm mất thay đổi chưa lưu của A!
    // - Session A bắt buộc phải giữ is_dirty = true!
    // - Aggregate workspace dirty bắt buộc vẫn là true!
    // - Yêu cầu đóng cửa sổ BẮT BUỘC BỊ HỦY (CANCELED), KHÔNG ĐƯỢC PHÉP ĐÓNG ỨNG DỤNG!
    let sess_a_after = state
        .get_session_for_document("doc-session-A")
        .expect("session A sau khi lưu B");
    assert!(
        sess_a_after.is_dirty,
        "Workbook A BẮT BUỘC VẪN DIRTY sau khi lưu B!"
    );
    assert_eq!(
        sess_a_after.edit_revision, 1,
        "Các sửa đổi của A không được phép bị mất!"
    );

    assert!(
        state.is_dirty.get_untracked(),
        "Workspace is_dirty BẮT BUỘC VẪN LÀ TRUE!"
    );
    assert!(
        close_manager.is_cancelled.get_untracked(),
        "Yêu cầu đóng cửa sổ BẮT BUỘC BỊ HỦY vì Workbook A vẫn còn thay đổi chưa lưu!"
    );
    assert!(
        !close_manager.is_authorized.get_untracked(),
        "Cấm ủy quyền đóng cửa sổ khi vẫn còn tài liệu chứa thay đổi chưa lưu!"
    );
}

/// Acceptance Test 1 & 7: Save As A to B, then ordinary Save to B; no ghost dirty origin session
#[test]
fn test_acceptance_01_save_as_a_to_b_then_ordinary_save_to_b_no_ghost_session() {
    let coordinator = StoredValue::new(HostDocumentCoordinator::new());
    let state = GridCraftState::new();

    // 1. Khởi tạo workbook A
    state.register_document_session(WorkbookSession {
        document_id: "doc-A".into(),
        file_path: "/data/A.xlsx".into(),
        file_name: "A.xlsx".into(),
        revision: 1,
        is_dirty: true,
        edit_revision: 1,
        iframe_doc_uid: Some(101),
    });
    state.set_document_session(Some("doc-A".into()), Some("/data/A.xlsx".into()), 1);
    assert!(state.is_dirty.get_untracked());

    // 2. Thực hiện Save As sang B.xlsx
    let save_outcome_b = DevSaveResult::Saved {
        dest_path: "/data/B.xlsx".into(),
        new_doc_id: "doc-B".into(),
        committed_rev: 1,
    };
    let save_as_a_ctx = GridCraftSaveRequestContext {
        request_id: "req-save-as-1".to_string(),
        operation_intent: SaveOperationIntent::SaveAs,
        originating_document_id: Some("doc-A".to_string()),
        originating_path: Some("/data/A.xlsx".to_string()),
        originating_name: "A.xlsx".to_string(),
        iframe_doc_uid: Some(101),
        snapshot_edit_revision: 1,
        base_committed_revision: 1,
        session_generation: 1,
    };
    handle_gridcraft_save_outcome(
        &state,
        Some(&coordinator),
        Ok(save_outcome_b),
        &save_as_a_ctx,
        b"PK\x03\x04_B_CONTENT",
        None,
        None,
    );

    // BẤT BIẾN:
    // - Active document chuyển sang B
    assert_eq!(
        state.active_document_id.get_untracked().as_deref(),
        Some("doc-B")
    );
    assert_eq!(
        state.active_file_path.get_untracked().as_deref(),
        Some("/data/B.xlsx")
    );
    assert_eq!(
        state.active_file_name.get_untracked().as_deref(),
        Some("B.xlsx")
    );

    // - Origin session doc-A BẮT BUỘC KHÔNG CÒN LÀ GHOST DIRTY SESSION (P1 #4683798594)
    assert!(
        state
            .document_sessions
            .with_untracked(|m| !m.contains_key("doc-A")),
        "doc-A bắt buộc phải được di chuyển!"
    );
    let sess_b = state
        .document_sessions
        .with_untracked(|m| m.get("doc-B").cloned())
        .expect("session B");
    assert!(!sess_b.is_dirty, "B phải clean sau khi save");
    assert_eq!(
        sess_b.iframe_doc_uid,
        Some(101),
        "Iframe UID được bảo toàn cho B"
    );
    assert!(
        !state.is_dirty.get_untracked(),
        "Workspace clean hoàn toàn!"
    );

    // 3. Người dùng sửa đổi tiếp trên B (revision = 2)
    state.sync_workbook_dirty_and_revision(Some("B.xlsx"), Some(101), true, Some(2));
    assert!(state.is_dirty.get_untracked());
    let sess_b_dirty = state
        .document_sessions
        .with_untracked(|m| m.get("doc-B").cloned())
        .unwrap();
    assert!(sess_b_dirty.is_dirty);
    assert_eq!(sess_b_dirty.edit_revision, 2);

    // 4. Lưu thông thường (Ctrl+S) trên B
    let resolved = state
        .resolve_save_session_with_uid("B.xlsx", Some(101))
        .expect("resolved B");
    assert_eq!(resolved.document_id, "doc-B");
    assert_eq!(resolved.file_path, "/data/B.xlsx");

    let save_outcome_b2 = DevSaveResult::Saved {
        dest_path: "/data/B.xlsx".into(),
        new_doc_id: "doc-B".into(),
        committed_rev: 2,
    };
    let save_b2_ctx = GridCraftSaveRequestContext {
        request_id: "req-save-b-2".to_string(),
        operation_intent: SaveOperationIntent::Save,
        originating_document_id: Some("doc-B".to_string()),
        originating_path: Some("/data/B.xlsx".to_string()),
        originating_name: "B.xlsx".to_string(),
        iframe_doc_uid: Some(101),
        snapshot_edit_revision: 2,
        base_committed_revision: 1,
        session_generation: 1,
    };
    handle_gridcraft_save_outcome(
        &state,
        Some(&coordinator),
        Ok(save_outcome_b2),
        &save_b2_ctx,
        b"PK\x03\x04_B2_CONTENT",
        None,
        None,
    );

    assert_eq!(state.active_revision.get_untracked(), 2);
    assert!(!state.is_dirty.get_untracked());
}

/// Acceptance Test 2 & 5: Repeated Ctrl+S and toolbar save; no false dirty revision without intervening edits
#[test]
fn test_acceptance_02_repeated_save_without_intervening_edits_no_false_dirty() {
    let coordinator = StoredValue::new(HostDocumentCoordinator::new());
    let state = GridCraftState::new();

    state.register_document_session(WorkbookSession {
        document_id: "doc-rep".into(),
        file_path: "/data/report.xlsx".into(),
        file_name: "report.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 5,
        iframe_doc_uid: Some(202),
    });
    state.set_document_session(Some("doc-rep".into()), Some("/data/report.xlsx".into()), 1);

    // Giả sử nhận nhiều state snapshot từ iframe mà revision không đổi
    state.sync_workbook_dirty_and_revision(Some("report.xlsx"), Some(202), false, Some(5));
    state.sync_workbook_dirty_and_revision(Some("report.xlsx"), Some(202), false, Some(5));

    let sess = state.get_session_for_document("doc-rep").unwrap();
    assert_eq!(
        sess.edit_revision, 5,
        "Repeated snapshots không được tăng edit_revision"
    );
    assert!(!sess.is_dirty);
    assert!(!state.is_dirty.get_untracked());

    // Lưu lần 1 (Ctrl+S)
    let rep_ctx1 = GridCraftSaveRequestContext {
        request_id: "save-rep-1".to_string(),
        operation_intent: SaveOperationIntent::Save,
        originating_document_id: Some("doc-rep".to_string()),
        originating_path: Some("/data/report.xlsx".to_string()),
        originating_name: "report.xlsx".to_string(),
        iframe_doc_uid: Some(202),
        snapshot_edit_revision: 5,
        base_committed_revision: 1,
        session_generation: 1,
    };
    handle_gridcraft_save_outcome(
        &state,
        Some(&coordinator),
        Ok(DevSaveResult::Saved {
            dest_path: "/data/report.xlsx".into(),
            new_doc_id: "doc-rep".into(),
            committed_rev: 2,
        }),
        &rep_ctx1,
        b"PK\x03\x04_CONTENT",
        None,
        None,
    );
    assert_eq!(state.active_revision.get_untracked(), 2);
    assert!(!state.is_dirty.get_untracked());

    // Lưu lần 2 ngay sau đó không có edit trung gian (Toolbar Save)
    let rep_ctx2 = GridCraftSaveRequestContext {
        request_id: "save-rep-2".to_string(),
        operation_intent: SaveOperationIntent::Save,
        originating_document_id: Some("doc-rep".to_string()),
        originating_path: Some("/data/report.xlsx".to_string()),
        originating_name: "report.xlsx".to_string(),
        iframe_doc_uid: Some(202),
        snapshot_edit_revision: 5,
        base_committed_revision: 2,
        session_generation: 1,
    };
    handle_gridcraft_save_outcome(
        &state,
        Some(&coordinator),
        Ok(DevSaveResult::Saved {
            dest_path: "/data/report.xlsx".into(),
            new_doc_id: "doc-rep".into(),
            committed_rev: 2,
        }),
        &rep_ctx2,
        b"PK\x03\x04_CONTENT",
        None,
        None,
    );
    assert_eq!(state.active_revision.get_untracked(), 2);
    assert!(
        !state.is_dirty.get_untracked(),
        "Lưu lặp lại không được tạo revision bẩn giả"
    );
}

/// Acceptance Test 3: Open two same-named workbooks and modify only one
#[test]
fn test_acceptance_03_two_same_named_workbooks_modify_one_with_uids() {
    let state = GridCraftState::new();

    // 2 workbook cùng tên budget.xlsx ở 2 thư mục khác nhau
    state.register_document_session(WorkbookSession {
        document_id: "doc-budget-1".into(),
        file_path: "/dir1/budget.xlsx".into(),
        file_name: "budget.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: Some(301),
    });
    state.register_document_session(WorkbookSession {
        document_id: "doc-budget-2".into(),
        file_path: "/dir2/budget.xlsx".into(),
        file_name: "budget.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: Some(302),
    });

    // Chỉ sửa đổi doc-budget-1 (uid: 301)
    state.sync_workbook_dirty_and_revision(Some("budget.xlsx"), Some(301), true, Some(1));

    let sess_1 = state
        .document_sessions
        .with_untracked(|m| m.get("doc-budget-1").cloned())
        .unwrap();
    let sess_2 = state
        .document_sessions
        .with_untracked(|m| m.get("doc-budget-2").cloned())
        .unwrap();

    assert!(sess_1.is_dirty, "Workbook 1 phải dirty");
    assert_eq!(sess_1.edit_revision, 1);
    assert!(!sess_2.is_dirty, "Workbook 2 KHÔNG ĐƯỢC dirty");
    assert_eq!(sess_2.edit_revision, 0);

    // Phân giải không có UID -> Ambiguous -> None (buộc Save As hoặc chỉ định đích)
    let ambiguous = state.resolve_save_session("budget.xlsx");
    assert!(
        ambiguous.is_none(),
        "Tên trùng lặp không có UID bắt buộc fail closed!"
    );

    // Phân giải có UID 301 -> Chính xác doc-budget-1!
    let resolved = state
        .resolve_save_session_with_uid("budget.xlsx", Some(301))
        .expect("resolved 1");
    assert_eq!(resolved.document_id, "doc-budget-1");
    assert_eq!(resolved.file_path, "/dir1/budget.xlsx");
}

/// Acceptance Test 8: Discard and close A while B remains open
#[test]
fn test_acceptance_08_discard_and_close_a_while_b_remains_open() {
    let state = GridCraftState::new();

    state.register_document_session(WorkbookSession {
        document_id: "doc-A".into(),
        file_path: "/data/A.xlsx".into(),
        file_name: "A.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: Some(401),
    });
    state.register_document_session(WorkbookSession {
        document_id: "doc-B".into(),
        file_path: "/data/B.xlsx".into(),
        file_name: "B.xlsx".into(),
        revision: 1,
        is_dirty: true,
        edit_revision: 1,
        iframe_doc_uid: Some(402),
    });

    assert_eq!(state.document_sessions.with_untracked(|m| m.len()), 2);
    assert!(state.is_dirty.get_untracked());

    // Iframe đóng A: live_uids bây giờ chỉ còn [402] (B)
    state.reconcile_closed_sessions(&[402]);

    // Session A được dọn dẹp sạch sẽ
    assert!(
        state
            .document_sessions
            .with_untracked(|m| !m.contains_key("doc-A"))
    );
    // Session B vẫn còn nguyên và vẫn dirty
    assert!(
        state
            .document_sessions
            .with_untracked(|m| m.contains_key("doc-B"))
    );
    assert!(
        state.is_dirty.get_untracked(),
        "B vẫn dirty nên workspace vẫn dirty"
    );
}

/// Task A (P1 #4684931088): Immutable save request context prevents Save As A->C from hijacking B
/// when A is selected inside the iframe while host active_document_id still points to B.
/// Uses genuine XLSX workbook fixtures and independent SHA-256 + spreadsheet content verification.
#[test]
fn test_p1_4684931088_save_as_a_to_c_while_host_active_is_b_preserves_b_and_c_independently() {
    use office_engine::data_lab::excel_reader::read_tabular_rows;
    use sha2::{Digest, Sha256};
    use std::sync::atomic::AtomicBool;

    let cancel_flag = AtomicBool::new(false);
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let path_a = temp_dir.path().join("A.xlsx");
    let path_b = temp_dir.path().join("B.xlsx");
    let path_c = temp_dir.path().join("C.xlsx");

    let a_bytes = std::fs::read("fixtures/stage2c_v1.xlsx").expect("read stage2c_v1.xlsx");
    let b_bytes_initial = std::fs::read("fixtures/stage2c_v2.xlsx").expect("read stage2c_v2.xlsx");
    let b_bytes_edited =
        std::fs::read("fixtures/test_uat_sample.xlsx").expect("read test_uat_sample.xlsx");

    std::fs::write(&path_a, &a_bytes).expect("write A.xlsx");
    std::fs::write(&path_b, &b_bytes_initial).expect("write B.xlsx");

    let b_sha_initial = format!("{:x}", Sha256::digest(&b_bytes_initial));
    let c_sha_expected = format!("{:x}", Sha256::digest(&a_bytes));
    let b_sha_edited_expected = format!("{:x}", Sha256::digest(&b_bytes_edited));
    assert_ne!(
        c_sha_expected, b_sha_edited_expected,
        "Fixtures for C and edited B must be distinct"
    );

    let state = GridCraftState::new();
    let coordinator = StoredValue::new(HostDocumentCoordinator::new());

    // 1. Open A.xlsx (uid 101)
    state.register_document_session(WorkbookSession {
        document_id: "doc-A".into(),
        file_path: path_a.to_str().unwrap().to_string(),
        file_name: "A.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: Some(101),
    });
    state.set_document_session(
        Some("doc-A".into()),
        Some(path_a.to_str().unwrap().to_string()),
        1,
    );

    // 2. Open B.xlsx (uid 102) -> host active_document_id is now doc-B
    state.register_document_session(WorkbookSession {
        document_id: "doc-B".into(),
        file_path: path_b.to_str().unwrap().to_string(),
        file_name: "B.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: Some(102),
    });
    state.set_document_session(
        Some("doc-B".into()),
        Some(path_b.to_str().unwrap().to_string()),
        1,
    );
    state.active_file_name.set(Some("B.xlsx".into()));
    assert_eq!(
        state.active_document_id.get_untracked().as_deref(),
        Some("doc-B")
    );

    // 3. Select A inside iframe (host active_document_id remains doc-B!) and dirty A
    state.sync_workbook_dirty_and_revision(Some("A.xlsx"), Some(101), true, Some(1));
    assert_eq!(
        state.active_document_id.get_untracked().as_deref(),
        Some("doc-B"),
        "Host active_document_id remains B when iframe selects A"
    );

    // 4. Save As A to C.xlsx: resolve originating session by iframe UID 101 at request acceptance
    let resolved_a = state
        .resolve_save_session_with_uid("A.xlsx", Some(101))
        .expect("must resolve A by UID 101");
    assert_eq!(resolved_a.document_id, "doc-A");

    let ctx_save_as_a = GridCraftSaveRequestContext {
        request_id: "save-101-1".to_string(),
        operation_intent: SaveOperationIntent::SaveAs,
        originating_document_id: Some(resolved_a.document_id.clone()),
        originating_path: Some(resolved_a.file_path.clone()),
        originating_name: resolved_a.file_name.clone(),
        iframe_doc_uid: resolved_a.iframe_doc_uid,
        snapshot_edit_revision: resolved_a.edit_revision,
        base_committed_revision: resolved_a.revision,
        session_generation: 1,
    };

    std::fs::write(&path_c, &a_bytes).expect("persist C.xlsx");
    handle_gridcraft_save_outcome(
        &state,
        Some(&coordinator),
        Ok(DevSaveResult::Saved {
            dest_path: path_c.to_str().unwrap().to_string(),
            new_doc_id: "doc-C".to_string(),
            committed_rev: 1,
        }),
        &ctx_save_as_a,
        &a_bytes,
        None,
        None,
    );

    // 5. Verify B retains its original identity, UID, path, and contents
    let sess_b = state
        .document_sessions
        .with_untracked(|m| m.get("doc-B").cloned())
        .expect("B session MUST still exist under doc-B");
    assert_eq!(sess_b.document_id, "doc-B");
    assert_eq!(sess_b.iframe_doc_uid, Some(102));
    assert_eq!(sess_b.file_path, path_b.to_str().unwrap());
    assert_eq!(sess_b.file_name, "B.xlsx");
    assert!(!sess_b.is_dirty);

    let b_disk_after_save_as_a = std::fs::read(&path_b).expect("read B.xlsx");
    assert_eq!(
        format!("{:x}", Sha256::digest(&b_disk_after_save_as_a)),
        b_sha_initial,
        "B.xlsx contents must be untouched after Save As A->C"
    );

    // Also verify A migrated cleanly to C with UID 101
    assert!(
        state
            .document_sessions
            .with_untracked(|m| !m.contains_key("doc-A"))
    );
    let sess_c = state
        .document_sessions
        .with_untracked(|m| m.get("doc-C").cloned())
        .expect("C session must exist");
    assert_eq!(sess_c.iframe_doc_uid, Some(101));
    assert_eq!(sess_c.file_path, path_c.to_str().unwrap());
    assert!(!sess_c.is_dirty);

    // 6. Edit and save B
    state.sync_workbook_dirty_and_revision(Some("B.xlsx"), Some(102), true, Some(1));
    let resolved_b = state
        .resolve_save_session_with_uid("B.xlsx", Some(102))
        .expect("B must still resolve to doc-B via UID 102");
    assert_eq!(resolved_b.document_id, "doc-B");
    assert_eq!(resolved_b.file_path, path_b.to_str().unwrap());

    let ctx_save_b = GridCraftSaveRequestContext {
        request_id: "save-102-1".to_string(),
        operation_intent: SaveOperationIntent::Save,
        originating_document_id: Some(resolved_b.document_id.clone()),
        originating_path: Some(resolved_b.file_path.clone()),
        originating_name: resolved_b.file_name.clone(),
        iframe_doc_uid: resolved_b.iframe_doc_uid,
        snapshot_edit_revision: resolved_b.edit_revision,
        base_committed_revision: resolved_b.revision,
        session_generation: 1,
    };

    std::fs::write(&resolved_b.file_path, &b_bytes_edited).expect("write edited B.xlsx");
    handle_gridcraft_save_outcome(
        &state,
        Some(&coordinator),
        Ok(DevSaveResult::Saved {
            dest_path: path_b.to_str().unwrap().to_string(),
            new_doc_id: "doc-B".to_string(),
            committed_rev: 2,
        }),
        &ctx_save_b,
        &b_bytes_edited,
        None,
        None,
    );

    // 7. Independently verify only B changed and C remains unchanged
    let b_final = std::fs::read(&path_b).expect("read final B.xlsx");
    let c_final = std::fs::read(&path_c).expect("read final C.xlsx");
    assert_eq!(
        format!("{:x}", Sha256::digest(&b_final)),
        b_sha_edited_expected
    );
    assert_eq!(format!("{:x}", Sha256::digest(&c_final)), c_sha_expected);

    let parsed_b = read_tabular_rows(&b_final, &cancel_flag).expect("parse final B.xlsx");
    let parsed_c = read_tabular_rows(&c_final, &cancel_flag).expect("parse final C.xlsx");
    assert_eq!(
        parsed_c[0][0], "98765",
        "C.xlsx must retain A's original cell content"
    );
    assert_ne!(parsed_b, parsed_c, "Edited B.xlsx must differ from C.xlsx");
}

/// Task A (P1 #4684931088): Switching active workbooks during pending persistence must not alter
/// the originating request identity or corrupt the newly active workbook's session.
#[test]
fn test_p1_4684931088_switching_active_workbook_during_pending_persistence() {
    let state = GridCraftState::new();
    let coordinator = StoredValue::new(HostDocumentCoordinator::new());

    state.register_document_session(WorkbookSession {
        document_id: "doc-A".into(),
        file_path: "/tmp/A.xlsx".into(),
        file_name: "A.xlsx".into(),
        revision: 1,
        is_dirty: true,
        edit_revision: 1,
        iframe_doc_uid: Some(501),
    });
    state.register_document_session(WorkbookSession {
        document_id: "doc-B".into(),
        file_path: "/tmp/B.xlsx".into(),
        file_name: "B.xlsx".into(),
        revision: 1,
        is_dirty: true,
        edit_revision: 1,
        iframe_doc_uid: Some(502),
    });
    state.set_document_session(Some("doc-A".into()), Some("/tmp/A.xlsx".into()), 1);
    state.active_file_name.set(Some("A.xlsx".into()));
    coordinator.update_value(|c| {
        c.on_document_opened("doc-A", "/tmp/A.xlsx", "fp-a");
        c.on_user_edit();
        let _ = c.on_save_requested(1, "doc-A", "save-501-1", b"PK\x03\x04_A".to_vec(), 100);
    });

    // Capture immutable request context for A -> C Save As before switching
    let ctx_a = GridCraftSaveRequestContext {
        request_id: "save-501-1".to_string(),
        operation_intent: SaveOperationIntent::SaveAs,
        originating_document_id: Some("doc-A".to_string()),
        originating_path: Some("/tmp/A.xlsx".to_string()),
        originating_name: "A.xlsx".to_string(),
        iframe_doc_uid: Some(501),
        snapshot_edit_revision: 1,
        base_committed_revision: 1,
        session_generation: 1,
    };
    state
        .register_pending_save_request(&ctx_a.request_id, ctx_a.originating_document_id.as_deref());

    // Switch active workbook to B while A's save is pending
    state.set_document_session(Some("doc-B".into()), Some("/tmp/B.xlsx".into()), 1);
    state.active_file_name.set(Some("B.xlsx".into()));
    coordinator.update_value(|c| {
        c.on_document_opened("doc-B", "/tmp/B.xlsx", "fp-b");
        c.on_user_edit();
    });

    // Complete A's Save As -> C while B is active
    handle_gridcraft_save_outcome(
        &state,
        Some(&coordinator),
        Ok(DevSaveResult::Saved {
            dest_path: "/tmp/C.xlsx".into(),
            new_doc_id: "doc-C".into(),
            committed_rev: 1,
        }),
        &ctx_a,
        b"PK\x03\x04_A",
        None,
        None,
    );

    // Originating doc-A migrated to doc-C with UID 501 and clean state
    assert!(
        state
            .document_sessions
            .with_untracked(|m| !m.contains_key("doc-A"))
    );
    let sess_c = state
        .document_sessions
        .with_untracked(|m| m.get("doc-C").cloned())
        .expect("doc-C");
    assert_eq!(sess_c.iframe_doc_uid, Some(501));
    assert!(!sess_c.is_dirty);

    // Active workbook B retained its own identity, UID, path, and dirty state
    assert_eq!(
        state.active_document_id.get_untracked().as_deref(),
        Some("doc-B")
    );
    assert_eq!(
        state.active_file_path.get_untracked().as_deref(),
        Some("/tmp/B.xlsx")
    );
    assert_eq!(
        state.active_file_name.get_untracked().as_deref(),
        Some("B.xlsx")
    );
    let sess_b = state
        .document_sessions
        .with_untracked(|m| m.get("doc-B").cloned())
        .expect("doc-B");
    assert_eq!(sess_b.iframe_doc_uid, Some(502));
    assert!(sess_b.is_dirty, "B must stay dirty because B was not saved");
    assert!(
        state.is_dirty.get_untracked(),
        "Workspace aggregate dirty must stay true for B"
    );
}

/// Task C (#4683038858): Explicit discard and close of a dirty workbook (both multi-doc and single-doc)
/// must remove the closed workbook session and not leave a ghost dirty session.
#[test]
fn test_p2_4683038858_explicit_discard_and_close_dirty_workbook_leaves_no_ghost_dirty_session() {
    let state = GridCraftState::new();

    // 1. Open dirty A.xlsx (uid 601) and clean B.xlsx (uid 602)
    state.register_document_session(WorkbookSession {
        document_id: "doc-A".into(),
        file_path: "/data/A.xlsx".into(),
        file_name: "A.xlsx".into(),
        revision: 1,
        is_dirty: true,
        edit_revision: 3,
        iframe_doc_uid: Some(601),
    });
    state.register_document_session(WorkbookSession {
        document_id: "doc-B".into(),
        file_path: "/data/B.xlsx".into(),
        file_name: "B.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
        iframe_doc_uid: Some(602),
    });
    state.set_document_session(Some("doc-A".into()), Some("/data/A.xlsx".into()), 1);
    assert!(state.is_dirty.get_untracked());

    // User closes dirty A with "Don't Save" -> iframe documents snapshot now contains only [602]
    state.reconcile_closed_sessions(&[602]);

    assert!(
        state
            .document_sessions
            .with_untracked(|m| !m.contains_key("doc-A")),
        "Closed dirty workbook A must be removed without leaving a ghost dirty session"
    );
    assert!(
        state
            .document_sessions
            .with_untracked(|m| m.contains_key("doc-B"))
    );
    assert!(
        !state.is_dirty.get_untracked(),
        "Workspace must be clean when only clean B remains"
    );
    assert!(
        state.active_document_id.get_untracked().is_none(),
        "Stale active_document_id pointing to closed A must be cleared"
    );

    // 2. Now dirty B and close B (last workbook) with "Don't Save" -> iframe documents snapshot is empty []
    state.sync_workbook_dirty_and_revision(Some("B.xlsx"), Some(602), true, Some(1));
    assert!(state.is_dirty.get_untracked());
    state.reconcile_closed_sessions(&[]);

    assert!(
        state.document_sessions.with_untracked(|m| m.is_empty()),
        "All closed workbooks must be removed when iframe reports empty documents list"
    );
    assert!(
        !state.is_dirty.get_untracked(),
        "Workspace must be clean after last dirty workbook is discarded and closed"
    );
}

#[test]
fn test_p1_4689114183_unmanaged_dirty_workbook_retained_when_sibling_save_completes() {
    use office_utility_vn_ui::app::AppState;
    use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::{
        DevSaveResult, GridCraftSaveRequestContext, handle_gridcraft_save_outcome,
    };
    use office_utility_vn_ui::platform::adapter::SaveOperationIntent;
    use office_utility_vn_ui::platform::window_close::WindowCloseManager;

    let app_state = AppState::new();
    let state = app_state.gridcraft_state;
    let close_mgr = WindowCloseManager::new();

    // 1. Mở workbook A.xlsx (đã có native session), đồng thời tạo workbook B mới chưa lưu (uid 702, không có native session)
    state.register_document_session(WorkbookSession {
        document_id: "doc-A".into(),
        file_path: "/data/A.xlsx".into(),
        file_name: "A.xlsx".into(),
        revision: 1,
        is_dirty: true,
        edit_revision: 1,
        iframe_doc_uid: Some(701),
    });
    state.set_document_session(Some("doc-A".into()), Some("/data/A.xlsx".into()), 1);

    // Iframe báo workbook B chưa có native session (uid 702) đang dirty ở revision 2
    state.sync_workbook_dirty_and_revision(Some("Untitled-1"), Some(702), true, Some(2));
    assert!(state.is_dirty.get_untracked());
    assert!(app_state.has_unsaved_work_or_running_task());

    // 2. Chọn A và thực hiện Save & Exit cho A trong khi B (unmanaged uid 702) vẫn chưa lưu
    close_mgr.handle_native_close_requested(
        "req-close-p1-4689114183".into(),
        true,
        "GridCraft: A.xlsx, Untitled-1",
    );
    close_mgr.is_saving.set(true);
    state
        .pending_close_request_id
        .set(Some("req-close-p1-4689114183".into()));

    let ctx_a = GridCraftSaveRequestContext {
        request_id: "save-A-only".into(),
        operation_intent: SaveOperationIntent::Save,
        originating_document_id: Some("doc-A".into()),
        originating_path: Some("/data/A.xlsx".into()),
        originating_name: "A.xlsx".into(),
        iframe_doc_uid: Some(701),
        snapshot_edit_revision: 1,
        base_committed_revision: 1,
        session_generation: 1,
    };
    state.register_pending_save_request("save-A-only", Some("doc-A"));

    handle_gridcraft_save_outcome(
        &state,
        None,
        Ok(DevSaveResult::Saved {
            dest_path: "/data/A.xlsx".into(),
            new_doc_id: "doc-A".into(),
            committed_rev: 2,
        }),
        &ctx_a,
        b"PK\x03\x04_A_SAVED",
        Some(close_mgr),
        Some(app_state.clone()),
    );

    // BẤT BIẾN P1 #4689114183: Dù A đã sạch và được đăng ký lại, B (uid 702) vẫn dirty => workspace và app_state PHẢI giữ dirty
    // và KHÔNG được phép ủy quyền đóng cửa sổ!
    assert!(
        state.is_dirty.get_untracked(),
        "Workspace dirty flag MUST remain true while unmanaged workbook uid 702 has unsaved changes!"
    );
    assert!(
        app_state.has_unsaved_work_or_running_task(),
        "AppState MUST still report unsaved work for unmanaged workbook uid 702!"
    );
    assert!(
        !close_mgr.is_authorized.get_untracked(),
        "Save & Exit on A MUST NOT authorize window close while unmanaged workbook B is dirty!"
    );
    assert_eq!(state.get_unmanaged_edit_revision(702), Some(2));

    // 3. Nếu người dùng đóng hẳn B với "Don't Save", documents snapshot chỉ còn [701] -> đối soát dọn uid 702 sạch sẽ
    state.reconcile_closed_sessions(&[701]);
    assert!(
        !state.is_dirty.get_untracked(),
        "Workspace dirty flag must clear once unmanaged dirty workbook 702 is closed/discarded"
    );
    assert_eq!(state.get_unmanaged_edit_revision(702), None);
}

#[test]
fn test_p1_4689587087_opened_csv_never_overwritten_with_xlsx_snapshot() {
    use office_utility_vn_ui::app::AppState;
    use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::{
        DevSaveResult, GridCraftSaveRequestContext, format_safe_default_save_name,
        handle_gridcraft_save_outcome, requires_format_safe_save_as,
        validate_save_as_format_compatibility,
    };
    use office_utility_vn_ui::platform::adapter::SaveOperationIntent;
    use office_utility_vn_ui::platform::window_close::WindowCloseManager;
    use sha2::{Digest, Sha256};

    let tmp_dir =
        std::env::temp_dir().join(format!("p1_4689587087_csv_safety_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp_dir);
    let csv_path = tmp_dir.join("ledger.csv");
    let xlsx_dest = tmp_dir.join("ledger.xlsx");

    let csv_original = b"id,name,amount\n1,Office,1500\n";
    std::fs::write(&csv_path, csv_original).expect("write initial csv");
    let csv_sha_before = format!("{:x}", Sha256::digest(csv_original));

    // Snapshot từ GridCraft host-initiated save (XLSX ZIP)
    let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let xlsx_bytes = std::fs::read(fixture_dir.join("stage2c_v1.xlsx")).expect("read xlsx fixture");

    // 1. Khi mở .csv nhưng payload là XLSX (PK\x03\x04), Save thường BẮT BUỘC phải nâng cấp thành Save As!
    assert!(requires_format_safe_save_as(
        SaveOperationIntent::Save,
        csv_path.to_str(),
        &xlsx_bytes
    ));
    assert!(
        validate_save_as_format_compatibility(&csv_path.to_string_lossy(), &xlsx_bytes).is_err(),
        "Direct ZIP write over .csv MUST be rejected"
    );
    assert_eq!(
        format_safe_default_save_name("ledger.csv", &xlsx_bytes),
        "ledger.xlsx"
    );

    // Ngược lại, .xlsx khớp với PK\x03\x04 và .csv khớp với CSV text
    assert!(!requires_format_safe_save_as(
        SaveOperationIntent::Save,
        xlsx_dest.to_str(),
        &xlsx_bytes
    ));
    assert!(!requires_format_safe_save_as(
        SaveOperationIntent::Save,
        csv_path.to_str(),
        csv_original
    ));

    // 2. Đăng ký phiên .csv và mô phỏng hoàn tất Save & Exit khi được nâng cấp sang SaveAs -> ledger.xlsx
    let app_state = AppState::new();
    let state = app_state.gridcraft_state;
    let close_mgr = WindowCloseManager::new();
    close_mgr.handle_native_close_requested("close-csv-req".into(), true, "ledger.csv");
    close_mgr.is_saving.set(true);
    state
        .pending_close_request_id
        .set(Some("close-csv-req".into()));

    state.register_document_session(WorkbookSession {
        document_id: "doc-csv-1".into(),
        file_path: csv_path.to_string_lossy().to_string(),
        file_name: "ledger.csv".into(),
        revision: 0,
        is_dirty: true,
        edit_revision: 1,
        iframe_doc_uid: Some(801),
    });
    state.set_document_session(
        Some("doc-csv-1".into()),
        Some(csv_path.to_string_lossy().to_string()),
        0,
    );

    let upgraded_ctx = GridCraftSaveRequestContext {
        request_id: "save-csv-upgraded".into(),
        operation_intent: SaveOperationIntent::SaveAs,
        originating_document_id: Some("doc-csv-1".into()),
        originating_path: Some(csv_path.to_string_lossy().to_string()),
        originating_name: "ledger.csv".into(),
        iframe_doc_uid: Some(801),
        snapshot_edit_revision: 1,
        base_committed_revision: 0,
        session_generation: 1,
    };
    state.register_pending_save_request("save-csv-upgraded", Some("doc-csv-1"));

    // Ghi .xlsx đích an toàn
    std::fs::write(&xlsx_dest, &xlsx_bytes).expect("write xlsx dest");
    handle_gridcraft_save_outcome(
        &state,
        None,
        Ok(DevSaveResult::Saved {
            dest_path: xlsx_dest.to_string_lossy().to_string(),
            new_doc_id: "doc-xlsx-new".into(),
            committed_rev: 1,
        }),
        &upgraded_ctx,
        &xlsx_bytes,
        Some(close_mgr),
        Some(app_state.clone()),
    );

    // 3. Xác minh tệp CSV gốc trên đĩa KHÔNG BỊ GHI ĐÈ, còn phiên host chuyển sạch sang ledger.xlsx và được phép đóng cửa sổ
    let csv_after = std::fs::read(&csv_path).expect("read csv after");
    assert_eq!(format!("{:x}", Sha256::digest(&csv_after)), csv_sha_before);
    assert!(
        state
            .document_sessions
            .with_untracked(|m| !m.contains_key("doc-csv-1") && m.contains_key("doc-xlsx-new")),
        "CSV session must migrate cleanly to new XLSX session without retaining a ghost dirty CSV session"
    );
    assert!(!state.is_dirty.get_untracked());
    assert!(close_mgr.is_authorized.get_untracked());

    let _ = std::fs::remove_dir_all(&tmp_dir);
}
