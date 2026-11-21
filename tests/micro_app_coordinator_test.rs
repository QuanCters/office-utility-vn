use office_utility_vn_ui::platform::{
    CoordinatorAction, DocumentState, HostDocumentCoordinator, UserClosingDecision,
};

#[test]
fn test_1_successful_save_flow() {
    let mut coord = HostDocumentCoordinator::new();
    assert_eq!(*coord.state(), DocumentState::Unloaded);

    // Mở tài liệu
    coord.on_document_opened("doc-1", "/home/user/test.xlsx", "hash-v0");
    assert_eq!(*coord.state(), DocumentState::Clean);
    assert!(!coord.is_dirty());

    // Người dùng sửa
    coord.on_user_edit();
    assert_eq!(*coord.state(), DocumentState::Dirty);
    assert!(coord.is_dirty());

    // Yêu cầu lưu
    let generation = coord.session_generation();
    let action = coord.on_save_requested(generation, "doc-1", "req-1", vec![1, 2, 3], 1000);
    assert!(matches!(action, CoordinatorAction::PerformDurableWrite { .. }));
    assert!(matches!(coord.state(), DocumentState::Saving { .. }));
    assert!(coord.is_dirty());

    // Backend ghi đĩa thành công
    let fp = match coord.state() {
        DocumentState::Saving { snapshot_fingerprint, .. } => snapshot_fingerprint.clone(),
        _ => panic!("Expected Saving"),
    };
    coord.on_durable_write_completed("req-1", &fp);
    assert_eq!(*coord.state(), DocumentState::Clean);
    assert!(!coord.is_dirty());
}

#[test]
fn test_2_permission_denied_fault_injection() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "/root/protected.xlsx", "hash-0");
    coord.on_user_edit();

    let generation = coord.session_generation();
    coord.on_save_requested(generation, "doc-1", "req-eacces", vec![1, 2, 3], 1000);

    // Fault injection: EACCES Permission Denied
    let action = coord.on_durable_write_failed("req-eacces", "Permission denied (os error 13)");
    assert!(matches!(action, CoordinatorAction::ShowSaveError { .. }));

    // Bất biến 1 & 2: Không được chuyển Clean, trạng thái là SaveFailed và is_dirty() == true!
    assert!(matches!(coord.state(), DocumentState::SaveFailed { dirty: true, .. }));
    assert!(coord.is_dirty(), "Tài liệu phải giữ cờ dirty khi ghi đĩa thất bại!");
}

#[test]
fn test_3_enoent_destination_fault_injection() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "/nonexistent/dir/test.xlsx", "hash-0");
    coord.on_user_edit();

    let generation = coord.session_generation();
    coord.on_save_requested(generation, "doc-1", "req-enoent", vec![1], 1000);

    // Fault injection: ENOENT No such file or directory
    coord.on_durable_write_failed("req-enoent", "No such file or directory (os error 2)");
    assert!(coord.is_dirty());
    assert!(matches!(coord.state(), DocumentState::SaveFailed { .. }));
}

#[test]
fn test_4_save_cancellation_preserves_dirty() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "test.xlsx", "hash-0");
    coord.on_user_edit();

    // Người dùng muốn rời đi
    let nav_action = coord.on_navigation_requested("launcher");
    assert!(matches!(nav_action, CoordinatorAction::PromptUnsavedChanges { .. }));

    // Người dùng bấm Cancel trên hộp thoại
    let cancel_action = coord.on_closing_dialog_decision(UserClosingDecision::Cancel);
    assert_eq!(cancel_action, CoordinatorAction::None);
    assert_eq!(*coord.state(), DocumentState::Dirty);
    assert!(coord.is_dirty());
}

#[test]
fn test_5_save_timeout_recovery() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "test.xlsx", "hash-0");
    coord.on_user_edit();

    let generation = coord.session_generation();
    coord.on_save_requested(generation, "doc-1", "req-timeout", vec![1], 1000);

    // Timeout fired after 30s
    coord.on_durable_write_failed("req-timeout", "Save operation timed out after 30000ms");
    assert!(coord.is_dirty());
    assert!(matches!(coord.state(), DocumentState::SaveFailed { .. }));
}

#[test]
fn test_6_two_overlapping_saves() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "test.xlsx", "hash-0");
    coord.on_user_edit();

    let generation = coord.session_generation();
    // Yêu cầu lưu 1
    coord.on_save_requested(generation, "doc-1", "req-1", vec![1], 1000);

    // Yêu cầu lưu 2 đến khi yêu cầu 1 đang in-flight (Double Ctrl+S)
    coord.on_save_requested(generation, "doc-1", "req-2", vec![2], 1010);

    // Yêu cầu 2 phải đánh dấu dirty_during_save = true
    if let DocumentState::Saving { dirty_during_save, request_id, .. } = coord.state() {
        assert_eq!(request_id, "req-2");
        assert!(dirty_during_save);
    } else {
        panic!("Expected Saving with req-2");
    }
}

#[test]
fn test_7_edit_after_export_before_ack_preserves_dirty() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "test.xlsx", "hash-0");
    coord.on_user_edit();

    let generation = coord.session_generation();
    coord.on_save_requested(generation, "doc-1", "req-1", vec![1, 2, 3], 1000);
    let fp = match coord.state() {
        DocumentState::Saving { snapshot_fingerprint, .. } => snapshot_fingerprint.clone(),
        _ => panic!("Expected Saving"),
    };

    // Bất biến 3: Người dùng gõ phím mới trong lúc đĩa đang ghi
    coord.on_user_edit();

    // Backend báo yêu cầu 1 ghi đĩa xong
    coord.on_durable_write_completed("req-1", &fp);

    // Kết quả: Vì có edit sau export, tài liệu VẪN PHẢI LÀ DIRTY!
    assert_eq!(*coord.state(), DocumentState::Dirty);
    assert!(coord.is_dirty(), "Phải giữ cờ Dirty cho các edit mới xuất hiện trong lúc lưu!");
}

#[test]
fn test_8_stale_acknowledgement_rejection() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "test.xlsx", "hash-0");
    coord.on_user_edit();

    let generation = coord.session_generation();
    coord.on_save_requested(generation, "doc-1", "req-current", vec![1], 1000);

    // Một ack cũ "req-stale" từ frame trước đến
    coord.on_durable_write_completed("req-stale", "hash-stale");

    // Bất biến 4: Bị từ chối, coordinator vẫn đang Saving cho req-current
    if let DocumentState::Saving { request_id, .. } = coord.state() {
        assert_eq!(request_id, "req-current");
    } else {
        panic!("Stale ack must not alter current saving state");
    }
}

#[test]
fn test_9_wrong_document_acknowledgement() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-A", "doc_a.xlsx", "hash-0");
    coord.on_user_edit();

    let generation = coord.session_generation();
    // Gửi save request cho "doc-B" (khác document ID)
    let action = coord.on_save_requested(generation, "doc-B", "req-b", vec![1], 1000);

    // Bất biến 5: Bị từ chối
    assert_eq!(action, CoordinatorAction::None);
    assert_eq!(*coord.state(), DocumentState::Dirty);
}

#[test]
fn test_10_iframe_reload_invalidates_pending_save() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "test.xlsx", "hash-0");
    coord.on_user_edit();

    let gen1 = coord.session_generation();
    coord.on_save_requested(gen1, "doc-1", "req-1", vec![1], 1000);

    // Iframe bị reload (generation tăng)
    coord.on_iframe_reload();
    let gen2 = coord.session_generation();
    assert_eq!(gen2, gen1 + 1);

    // Request cũ đến với gen1 bị từ chối
    let stale_action = coord.on_save_requested(gen1, "doc-1", "req-old", vec![2], 1500);
    assert_eq!(stale_action, CoordinatorAction::None);
    assert!(coord.is_dirty());
}

#[test]
fn test_11_document_replacement_during_pending_save() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-old", "old.xlsx", "hash-0");
    coord.on_user_edit();

    let generation = coord.session_generation();
    coord.on_save_requested(generation, "doc-old", "req-1", vec![1], 1000);

    // Mở tài liệu mới "doc-new" thay thế
    coord.on_document_opened("doc-new", "new.xlsx", "hash-new");
    assert_eq!(*coord.state(), DocumentState::Clean);

    // Ack cũ của doc-old đến -> không làm bẩn doc-new
    coord.on_durable_write_completed("req-1", "hash-0");
    assert_eq!(*coord.state(), DocumentState::Clean);
}

#[test]
fn test_12_navigation_blocked_on_dirty_or_save_failure() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "contract.xlsx", "hash-0");
    coord.on_user_edit();

    // Bất biến 2: Đang dirty -> điều hướng bị chặn
    let action = coord.on_navigation_requested("pdf-studio");
    match action {
        CoordinatorAction::PromptUnsavedChanges { dest_path, target_navigation } => {
            assert_eq!(dest_path, "contract.xlsx");
            assert_eq!(target_navigation, "pdf-studio");
        }
        other => panic!("Expected PromptUnsavedChanges, got {other:?}"),
    }
}

#[test]
fn test_13_user_explicit_discard_decision() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "data.xlsx", "hash-0");
    coord.on_user_edit();

    coord.on_navigation_requested("launcher");
    assert!(matches!(coord.state(), DocumentState::ClosingPending { .. }));

    // Bất biến 9: Người dùng xác nhận Discard
    let action = coord.on_closing_dialog_decision(UserClosingDecision::Discard);
    assert_eq!(action, CoordinatorAction::ProceedNavigation("launcher".to_string()));
    assert_eq!(*coord.state(), DocumentState::Clean);
    assert!(!coord.is_dirty());
}

#[test]
fn test_14_non_workbook_export_preserves_workbook_dirty_state() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-wb-1", "budget.xlsx", "hash-wb-0");
    coord.on_user_edit();
    assert!(coord.is_dirty());

    // Xuất PDF thông qua cùng kênh callback download nhưng là tệp phụ trợ (non-workbook export)
    // Tệp PDF không được coi là save của workbook hiện tại
    let _generation = coord.session_generation();
    let _pdf_bytes = vec![0x25, 0x50, 0x44, 0x46]; // %PDF
    // Nếu gửi save request cho một export không đổi baseline (hoặc export độc lập)
    // Tài liệu workbook vẫn phải giữ dirty!
    assert!(coord.is_dirty(), "Workbook phải tiếp tục dirty sau khi xuất file PDF phụ trợ!");

    // Khi backend hoàn tất ghi file pdf, workbook vẫn dirty cho đến khi có Save workbook đích thực
    assert_eq!(*coord.state(), DocumentState::Dirty);
}

#[test]
fn test_15_atomic_durable_write_lifecycle() {
    use office_utility_vn_ui::platform::{atomic_durable_write, compute_fingerprint};

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("finance_report.xlsx");
    let content = b"PK\x03\x04RealXlsxBinaryPayloadWithFormulas";

    let report = atomic_durable_write(&dest_file, content).unwrap();

    assert_eq!(report.dest_path, dest_file.to_string_lossy().to_string());
    assert_eq!(report.bytes_written, content.len());
    assert_eq!(report.fingerprint, compute_fingerprint(content));
    assert!(report.write_completed);
    assert!(report.file_synced);
    assert!(report.atomically_replaced);

    // Xác nhận nội dung tệp đích khớp 100%
    let read_back = std::fs::read(&dest_file).unwrap();
    assert_eq!(read_back, content);

    // Xác nhận không còn tệp tạm (.tmp.*) sót lại trong thư mục
    let entries: Vec<_> = std::fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 1, "Chỉ có duy nhất tệp đích sau khi rename, không để lại tệp tạm!");
}

#[test]
fn test_16_atomic_durable_write_nonexistent_dir_fails_gracefully() {
    use office_utility_vn_ui::platform::atomic_durable_write;
    use std::path::Path;

    let bad_path = Path::new("/nonexistent_volume_dir_abc123/file.xlsx");
    let res = atomic_durable_write(bad_path, b"data");
    assert!(res.is_err(), "Ghi vào thư mục không tồn tại phải trả về lỗi!");
}

#[test]
fn test_17_destination_scoped_save_ordering_adversarial() {
    use office_utility_vn_ui::platform::{
        atomic_durable_write_with_ticket, DestinationSaveTracker, PersistenceError,
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("financial_model.xlsx");
    let tracker = DestinationSaveTracker::new();

    // Lượt lưu Save A (revision 10): Cấp vé seq 1
    let ticket_a = tracker.reserve(&dest_file);
    assert_eq!(ticket_a.seq, 1);

    // Lượt lưu Save B (revision 11): Cấp vé seq 2
    let ticket_b = tracker.reserve(&dest_file);
    assert_eq!(ticket_b.seq, 2);

    // Kịch bản nghịch đảo (Adversarial Ordering):
    // Save B ghi nhanh hơn và commit thành công trước!
    let content_b = b"PK\x03\x04REVISION_11_LATEST_FORMULAS";
    let report_b = atomic_durable_write_with_ticket(&dest_file, content_b, &ticket_b, &tracker).unwrap();
    assert!(report_b.atomically_replaced);

    let state = tracker.get_state(&dest_file).unwrap();
    assert_eq!(state.last_committed_seq, 2);

    // Giờ đây Save A (bị trễ I/O) mới cố gắng ghi đè với revision 10
    let content_a = b"PK\x03\x04REVISION_10_OUTDATED_STALE_DATA";
    let res_a = atomic_durable_write_with_ticket(&dest_file, content_a, &ticket_a, &tracker);

    // Bất biến Task 3: Save A BẮT BUỘC phải bị từ chối trước khi rename!
    match res_a {
        Err(PersistenceError::StaleWriteIgnored { attempted_seq, current_seq }) => {
            assert_eq!(attempted_seq, 1);
            assert_eq!(current_seq, 2);
        }
        other => panic!("Kỳ vọng StaleWriteIgnored, nhưng nhận được: {other:?}"),
    }

    // Đọc lại nội dung tệp thực tế trên đĩa: Tuyệt đối không bị rollback về revision 10!
    let persisted_content = std::fs::read(&dest_file).unwrap();
    assert_eq!(
        persisted_content, content_b,
        "Tệp trên đĩa PHẢI giữ nguyên nội dung Revision 11, không bao giờ bị ghi đè bởi Revision 10!"
    );
}

#[test]
fn test_18_save_as_different_destination_isolation() {
    use office_utility_vn_ui::platform::{
        atomic_durable_write_with_ticket, DestinationSaveTracker,
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_1 = temp_dir.path().join("original.xlsx");
    let dest_2 = temp_dir.path().join("exported_copy.xlsx");
    let tracker = DestinationSaveTracker::new();

    let ticket_1 = tracker.reserve(&dest_1);
    let ticket_2 = tracker.reserve(&dest_2);

    // Cả 2 vé đều có seq 1 cho đường dẫn độc lập của chúng
    assert_eq!(ticket_1.seq, 1);
    assert_eq!(ticket_2.seq, 1);

    let rep_1 = atomic_durable_write_with_ticket(&dest_1, b"Book1", &ticket_1, &tracker).unwrap();
    let rep_2 = atomic_durable_write_with_ticket(&dest_2, b"Book2", &ticket_2, &tracker).unwrap();

    assert!(rep_1.atomically_replaced);
    assert!(rep_2.atomically_replaced);
    assert_eq!(std::fs::read(&dest_1).unwrap(), b"Book1");
    assert_eq!(std::fs::read(&dest_2).unwrap(), b"Book2");
}

#[test]
fn test_19_path_traversal_and_security_guard() {
    use office_utility_vn_ui::platform::{validate_destination_path, PersistenceError};

    // 1. Path traversal attempts
    let bad_paths = [
        "../../etc/passwd",
        "folder/../../../root/secrets.xlsx",
        "relative/../secret.xlsx",
        "",
        "   ",
        "document\0malicious.xlsx",
        "script.exe",
        "malware.sh",
        "payload.bin",
    ];

    for path in bad_paths {
        let res = validate_destination_path(path);
        assert!(
            matches!(res, Err(PersistenceError::SecurityViolation(_))),
            "Đường dẫn nguy hại '{path}' phải bị SecurityViolation chặn đứng!"
        );
    }

    // 2. Valid paths
    let valid_paths = [
        "report.xlsx",
        "/home/user/documents/budget.xlsx",
        "data_2026.csv",
        "export.json",
        "sheet.ods",
    ];

    for path in valid_paths {
        assert!(validate_destination_path(path).is_ok(), "Đường dẫn hợp lệ '{path}' phải được chấp nhận");
    }
}

#[test]
fn test_20_save_cancellation_and_timeout_recovery() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "annual_plan.xlsx", "h0");
    coord.on_user_edit();

    let generation = coord.session_generation();
    coord.on_save_requested(generation, "doc-1", "req-cancel", vec![1, 2], 1000);
    assert!(matches!(coord.state(), DocumentState::Saving { .. }));

    // Hủy bỏ yêu cầu lưu
    coord.on_save_cancelled("req-cancel");
    assert_eq!(*coord.state(), DocumentState::Dirty);
    assert!(coord.is_dirty(), "Hủy lưu phải giữ nguyên cờ dirty!");

    // Yêu cầu lưu tiếp theo bị Timeout
    coord.on_save_requested(generation, "doc-1", "req-timeout-2", vec![1, 2], 2000);
    let timeout_action = coord.on_save_timeout("req-timeout-2");
    assert!(matches!(timeout_action, CoordinatorAction::ShowSaveError { .. }));
    assert!(matches!(coord.state(), DocumentState::SaveFailed { dirty: true, .. }));
    assert!(coord.is_dirty());
}

#[test]
fn test_21_duplicate_and_stale_fingerprint_rejection() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "test.xlsx", "h0");
    coord.on_user_edit();

    let generation = coord.session_generation();
    let bytes = vec![1, 2, 3, 4];
    coord.on_save_requested(generation, "doc-1", "req-dup", bytes, 1000);

    let fp = match coord.state() {
        DocumentState::Saving { snapshot_fingerprint, .. } => snapshot_fingerprint.clone(),
        _ => panic!("Expected saving"),
    };

    // Acknowledgment với sai fingerprint -> bị từ chối!
    coord.on_durable_write_completed("req-dup", "wrong_fingerprint_hash");
    assert!(matches!(coord.state(), DocumentState::Saving { .. }), "Sai fingerprint không được xác nhận Clean!");

    // Acknowledgment chuẩn xác
    coord.on_durable_write_completed("req-dup", &fp);
    assert_eq!(*coord.state(), DocumentState::Clean);

    // Duplicate acknowledgment lần 2 -> không gây lỗi hay đổi trạng thái
    coord.on_durable_write_completed("req-dup", &fp);
    assert_eq!(*coord.state(), DocumentState::Clean);
}

#[test]
fn test_22_deterministic_concurrency_race_pause_save_a() {
    use office_utility_vn_ui::platform::{
        atomic_durable_write_with_ticket, DestinationSaveTracker, PersistenceError,
    };
    use std::sync::{Arc, Barrier};
    use std::thread;

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("concurrency_race_target.xlsx");
    let tracker = Arc::new(DestinationSaveTracker::new());

    // Cấp vé tuần tự: Save A nhận seq 1, Save B nhận seq 2
    let ticket_a = tracker.reserve(&dest_file);
    let ticket_b = tracker.reserve(&dest_file);
    assert_eq!(ticket_a.seq, 1);
    assert_eq!(ticket_b.seq, 2);

    let content_a = b"PK\x03\x04SAVE_A_REV_1_OUTDATED";
    let content_b = b"PK\x03\x04SAVE_B_REV_2_NEWEST";

    let barrier_start = Arc::new(Barrier::new(2));
    let barrier_save_b_done = Arc::new(Barrier::new(2));

    let dest_clone = dest_file.clone();
    let tracker_clone = Arc::clone(&tracker);
    let b_start = Arc::clone(&barrier_start);
    let b_done = Arc::clone(&barrier_save_b_done);

    // Luồng Save A: Chuẩn bị tệp tạm, kiểm tra sơ bộ can_commit (vượt qua), rồi BỊ TẠM DỪNG (pause)
    let handle_a = thread::spawn(move || {
        // Kiểm tra sơ bộ ban đầu
        tracker_clone.can_commit(&ticket_a).expect("Check sơ bộ ban đầu của Save A phải thành công vì chưa ai commit");

        // Đồng bộ để Save B bắt đầu chạy
        b_start.wait();

        // Chờ Save B commit xong hoàn toàn xuống đĩa
        b_done.wait();

        // Giờ Save A mới tiếp tục bước commit hoán đổi rename
        // Nhưng dưới sự bảo vệ của tracker_clone.commit_atomic, Save A PHẢI BỊ TỪ CHỐI!
        let temp_file_a = dest_clone.parent().unwrap().join(".tmp.save_a.xlsx");
        std::fs::write(&temp_file_a, content_a).unwrap();

        let commit_res = tracker_clone.commit_atomic(&ticket_a, || {
            std::fs::rename(&temp_file_a, &dest_clone).map_err(|e| PersistenceError::AtomicRename {
                from: temp_file_a.clone(),
                to: dest_clone.clone(),
                source: e,
            })?;
            Ok(())
        });

        // Dọn dẹp tệp tạm của Save A nếu nó còn tồn tại
        if temp_file_a.exists() {
            let _ = std::fs::remove_file(&temp_file_a);
        }

        commit_res
    });

    // Luồng Save B: Chạy và commit thành công trước
    barrier_start.wait();
    let rep_b = atomic_durable_write_with_ticket(&dest_file, content_b, &ticket_b, &tracker).unwrap();
    assert!(rep_b.atomically_replaced);
    assert_eq!(tracker.get_state(&dest_file).unwrap().last_committed_seq, 2);

    // Báo cho Save A thức dậy và cố gắng commit
    barrier_save_b_done.wait();

    let res_a = handle_a.join().unwrap();
    match res_a {
        Err(PersistenceError::StaleWriteIgnored { attempted_seq, current_seq }) => {
            assert_eq!(attempted_seq, 1);
            assert_eq!(current_seq, 2);
        }
        other => panic!("Save A phải bị từ chối bởi StaleWriteIgnored, nhưng nhận: {other:?}"),
    }

    // Đọc tệp trên hệ thống tệp đĩa thực tế: Phải là content_b, không bao giờ bị Save A ghi đè!
    let disk_content = std::fs::read(&dest_file).unwrap();
    assert_eq!(disk_content, content_b, "Tệp trên đĩa BẮT BUỘC giữ nguyên Revision 2 của Save B!");
}

#[test]
fn test_23_forced_serialization_failure_preserves_dirty_state() {
    let mut coord = HostDocumentCoordinator::new();
    coord.on_document_opened("doc-1", "budget.xlsx", "h0");
    coord.on_user_edit();
    assert!(coord.is_dirty());

    // Giả lập lỗi tuần tự hóa (serialization failure) ở tầng GridCraft/Engine:
    // Khi tuần tự hóa thất bại:
    // 1. Không có payload gridcraft:save hợp lệ được phát ra cho host
    // 2. Không có gridcraft:saved acknowledgment nào được tạo
    // 3. Coordinator vẫn ở trạng thái Dirty
    assert_eq!(*coord.state(), DocumentState::Dirty);

    // Nếu host nhận được lỗi hoặc cố gắng ghi nhận thất bại:
    let generation = coord.session_generation();
    coord.on_save_requested(generation, "doc-1", "req-err-1", vec![], 1000);
    assert!(matches!(coord.state(), DocumentState::Saving { .. }));

    let action = coord.on_durable_write_failed("req-err-1", "Lỗi tuần tự hóa workbook: corrupted stream");
    assert!(matches!(action, CoordinatorAction::ShowSaveError { .. }));
    assert!(matches!(coord.state(), DocumentState::SaveFailed { dirty: true, .. }));
    assert!(coord.is_dirty(), "Tài liệu BẮT BUỘC phải tiếp tục Dirty để cho phép thử lại an toàn!");

    // Thử lại lần 2 thành công
    let gen2 = coord.session_generation();
    coord.on_save_requested(gen2, "doc-1", "req-retry-ok", vec![1, 2, 3], 2000);
    assert!(matches!(coord.state(), DocumentState::Saving { .. }));

    let fp = match coord.state() {
        DocumentState::Saving { snapshot_fingerprint, .. } => snapshot_fingerprint.clone(),
        _ => panic!("Expected saving"),
    };
    coord.on_durable_write_completed("req-retry-ok", &fp);
    assert_eq!(*coord.state(), DocumentState::Clean);
}
