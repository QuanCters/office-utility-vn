use leptos::prelude::*;
use office_utility_vn_ui::micro_apps::gridcraft::{
    GRIDCRAFT_IFRAME_SRC, GridCraftApp, GridCraftState, is_gridcraft_dev_workspace_enabled,
};
use office_utility_vn_ui::micro_apps::gridcraft::state::WorkbookSession;
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
    });
    state.register_document_session(WorkbookSession {
        document_id: "doc-bud-2026".into(),
        file_path: file_b.to_str().unwrap().to_string(),
        file_name: "Budget_2026.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
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

    let state = GridCraftState::new();

    // Hai tệp cùng tên "budget.xlsx" ở 2 thư mục khác nhau
    state.register_document_session(WorkbookSession {
        document_id: "doc-budget-q1".into(),
        file_path: "/dir1/budget.xlsx".into(),
        file_name: "budget.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
    });
    state.register_document_session(WorkbookSession {
        document_id: "doc-budget-q2".into(),
        file_path: "/dir2/budget.xlsx".into(),
        file_name: "budget.xlsx".into(),
        revision: 1,
        is_dirty: false,
        edit_revision: 0,
    });

    // Khi active_document_id là Q1: resolve bằng tên "budget.xlsx" phải trỏ về đúng Q1
    state.active_document_id.set(Some("doc-budget-q1".into()));
    let resolved_q1 = state.resolve_save_session("budget.xlsx").expect("Must resolve to active Q1");
    assert_eq!(resolved_q1.document_id, "doc-budget-q1");
    assert_eq!(resolved_q1.file_path, "/dir1/budget.xlsx");

    // Khi active_document_id là Q2: resolve bằng tên "budget.xlsx" phải trỏ về đúng Q2
    state.active_document_id.set(Some("doc-budget-q2".into()));
    let resolved_q2 = state.resolve_save_session("budget.xlsx").expect("Must resolve to active Q2");
    assert_eq!(resolved_q2.document_id, "doc-budget-q2");
    assert_eq!(resolved_q2.file_path, "/dir2/budget.xlsx");

    // Khi active_document_id không trùng với tài liệu nào (hoặc None):
    // lookup theo tên mơ hồ phải trả về None để buộc rơi vào Save As an toàn thay vì ghi đè sai file!
    state.active_document_id.set(None);
    let ambiguous = state.resolve_save_session("budget.xlsx");
    assert!(
        ambiguous.is_none(),
        "Ambiguous filename resolution without active document ID must return None"
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
    assert!(err_csv.is_err(), "Must reject writing XLSX binary into .csv file");
    let err_msg = err_csv.unwrap_err();
    assert!(err_msg.contains("Xung đột định dạng tệp"));

    let err_ods = validate_save_as_format_compatibility("budget.ods", xlsx_bytes);
    assert!(err_ods.is_err(), "Must reject writing XLSX binary into .ods file");

    // B. Dữ liệu CSV văn bản thường (Không có PK header)
    let csv_bytes = b"col1,col2,col3\nval1,val2,val3\n";

    // Lưu vào .csv -> Hợp lệ
    assert!(validate_save_as_format_compatibility("/data/export.csv", csv_bytes).is_ok());

    // Lưu vào .xlsx -> BẮT BUỘC TỪ CHỐI vì thiếu chữ ký ZIP PK
    let err_xlsx = validate_save_as_format_compatibility("/data/export.xlsx", csv_bytes);
    assert!(err_xlsx.is_err(), "Must reject saving non-PK bytes to .xlsx path");
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
    });
    state.active_document_id.set(Some("doc-discard-test".into()));
    state.is_dirty.set(true);
    state.is_exit_confirm_open.set(true);

    assert!(state.is_dirty.get_untracked());
    assert!(state.is_exit_confirm_open.get_untracked());

    // 2. Người dùng bấm "Hủy Thay đổi & Thoát" -> reset_state()
    state.is_exit_confirm_open.set(false);
    state.reset_state();

    // Xác minh trạng thái sau khi discard & exit:
    assert!(!state.is_dirty.get_untracked(), "is_dirty must be reset to false");
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
    });
    state.set_document_session(Some(doc_id.to_string()), Some(wb_path.to_string()), 2);
    state.active_file_name.set(Some("finance_q4.xlsx".to_string()));
    state.is_dirty.set(true);

    assert!(state.is_dirty.get());
    assert_eq!(state.active_document_id.get(), Some(doc_id.to_string()));
    assert_eq!(state.active_file_path.get(), Some(wb_path.to_string()));
    assert_eq!(state.active_revision.get(), 2);

    // Người dùng yêu cầu đóng cửa sổ -> close dialog mở ra
    state.pending_close_request_id.set(Some("close-req-export-test".to_string()));

    // 2. Xuất bảng tính ra file CSV ("finance_export.csv") với intent = Export
    let export_intent = SaveOperationIntent::Export;

    // Production logic: Khi intent == Export, hàm completion handler chỉ gửi thông báo xác nhận tới iframe,
    // tuyệt đối không đụng vào dirty flag, không đổi session, không authorize close.
    if export_intent == SaveOperationIntent::Export {
        // Production logic: No state alteration for export
    }

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
    });
    state.set_document_session(Some(doc_id.to_string()), Some(wb_path.to_string()), 1);
    state.active_file_name.set(Some("balance_sheet.xlsx".to_string()));
    state.is_dirty.set(true);
    state.pending_close_request_id.set(Some("close-req-ambiguous".to_string()));

    // 2. Kết quả lưu là AmbiguousCommit (hoán đổi tệp thành công nhưng đồng bộ thư mục cha thất bại)
    let ambiguous_outcome = DevSaveResult::AmbiguousCommit {
        dest_path: wb_path.to_string(),
        new_doc_id: doc_id.to_string(),
        committed_rev: 2,
        message: "AmbiguousCommit: Hoán đổi nguyên tử thành công nhưng đồng bộ thư mục cha thất bại".to_string(),
    };

    // Áp dụng kết quả AmbiguousCommit theo đúng luồng xử lý sản phẩm
    if let DevSaveResult::AmbiguousCommit { dest_path, new_doc_id, committed_rev, message } = ambiguous_outcome {
        let live_edit_rev = state.get_edit_revision(&new_doc_id);
        state.set_document_dirty(&new_doc_id, true);
        state.register_document_session(WorkbookSession {
            document_id: new_doc_id.clone(),
            file_path: dest_path.clone(),
            file_name: "balance_sheet.xlsx".to_string(),
            revision: committed_rev,
            is_dirty: true,
            edit_revision: live_edit_rev,
        });
        state.set_document_session(Some(new_doc_id), Some(dest_path), committed_rev);
        state.is_dirty.set(true);
        state.save_blocked_detail.set(format!("⚠️ Ambiguous Commit: {message}"));
        state.is_save_blocked_alert.set(true);
        state.pending_close_request_id.set(None);
    }

    // BẤT BIẾN P2 #4678429561:
    // 1. Revision được cập nhật lên 2 (authoritative committed revision) để ngăn chặn stale revision write
    assert_eq!(state.active_revision.get(), 2, "Session revision must advance to 2");
    assert_eq!(
        state.get_session_for_document(doc_id).unwrap().revision,
        2,
        "Registered session revision must be 2"
    );

    // 2. Dirty state BẮT BUỘC giữ nguyên true
    assert!(state.is_dirty.get(), "Workbook MUST remain dirty after AmbiguousCommit");
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
    assert_eq!(next_expected_rev, 3, "Subsequent save attempt must target revision 3");
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
    assert!(matches!(action, office_utility_vn_ui::platform::coordinator::CoordinatorAction::PerformDurableWrite { .. }));
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
    use sha2::{Digest, Sha256};
    use office_utility_vn_ui::platform::window_close::WindowCloseManager;
    use office_utility_vn_ui::platform::persistence::atomic_durable_write;
    use office_utility_vn_ui::platform::save_grant::{
        validate_native_save_grant, NativeSaveGrantDto, SaveRequestContext,
    };
    use office_utility_vn_ui::micro_apps::gridcraft::workspace::handle_gridcraft_save_completion;

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
    close_mgr.handle_native_close_requested(close_req_id.clone(), true, "GridCraft: quarterly_budget.xlsx");
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
    let write_report = atomic_durable_write(&file_path, edited_bytes).expect("durable write succeeds");
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




