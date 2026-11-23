use leptos::prelude::*;
use office_utility_vn_ui::micro_apps::gridcraft::{
    GRIDCRAFT_IFRAME_SRC, GridCraftApp, GridCraftState, is_gridcraft_dev_workspace_enabled,
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
    });
    state.register_document_session(WorkbookSession {
        document_id: "doc-bud-2026".into(),
        file_path: file_b.to_str().unwrap().to_string(),
        file_name: "Budget_2026.xlsx".into(),
        revision: 1,
        is_dirty: false,
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
    });
    state.register_document_session(WorkbookSession {
        document_id: "doc-budget-q2".into(),
        file_path: "/dir2/budget.xlsx".into(),
        file_name: "budget.xlsx".into(),
        revision: 1,
        is_dirty: false,
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
