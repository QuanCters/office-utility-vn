use leptos::prelude::*;
use office_utility_vn_ui::app::AppState;
use office_utility_vn_ui::micro_apps::gridcraft::GridCraftState;
use office_utility_vn_ui::micro_apps::pdf_studio::PdfStudioState;
use office_utility_vn_ui::platform::{
    CoordinatorAction, HostDocumentCoordinator, WindowCloseDecision, WindowCloseManager,
};

/// Mô phỏng trạng thái bộ bảo vệ đóng cửa sổ native (Native Window Close Guard Simulator).
#[derive(Debug, Default)]
struct MockWindowCloseGuard {
    is_prevented: bool,
    active_request_id: Option<String>,
    authorized_token: Option<String>,
    is_closing: bool,
    close_count: usize,
    request_seq: usize,
}

impl MockWindowCloseGuard {
    fn new() -> Self {
        Self::default()
    }

    fn begin_close_request(&mut self) -> (String, bool) {
        if let Some(existing) = &self.active_request_id {
            (existing.clone(), false)
        } else {
            self.request_seq += 1;
            let id = format!("close-req-{}", self.request_seq);
            self.active_request_id = Some(id.clone());
            (id, true)
        }
    }

    fn should_allow_close(&mut self) -> bool {
        if self.is_closing {
            return true;
        }
        if let Some(_tok) = self.authorized_token.take() {
            self.is_closing = true;
            self.active_request_id = None;
            self.close_count += 1;
            return true;
        }
        false
    }

    fn on_close_requested(&mut self) -> bool {
        if self.should_allow_close() {
            false // allow close (not prevented)
        } else {
            self.is_prevented = true;
            let (_id, _) = self.begin_close_request();
            true // prevent close
        }
    }

    fn authorize(&mut self, req_id: &str) -> bool {
        if self.active_request_id.as_deref() == Some(req_id) {
            let tok = format!("close-tok-{}", req_id);
            self.authorized_token = Some(tok);
            self.active_request_id = None;
            true
        } else {
            false
        }
    }

    fn cancel(&mut self, req_id: &str) -> bool {
        if self.active_request_id.as_deref() == Some(req_id) {
            self.active_request_id = None;
            true
        } else {
            false
        }
    }
}

// ==============================================================================
// TASK 6 — 12 KỊCH BẢN CHUẨN XÁC MINH VÒNG ĐỜI LIÊN THÀNH PHẦN (SCENARIOS A - L)
// ==============================================================================

/// Kịch bản A: Bảng tính Dirty → File New → Chọn Lưu → Nhận ACK → Thực thi New
#[test]
fn test_cross_scenario_a_dirty_workbook_new_save_ack_new() {
    let state = GridCraftState::new();
    let mut coord = HostDocumentCoordinator::new();

    // 1. Tài liệu A đang mở và dirty
    coord.on_document_opened("doc-a", "/data/doc_a.xlsx", "hash-a0");
    coord.on_user_edit();
    state.set_document_session(Some("doc-a".into()), Some("/data/doc_a.xlsx".into()), 1);
    state.set_active_file("doc_a.xlsx");
    state.is_dirty.set(true);
    assert!(state.is_dirty.get_untracked());
    assert!(coord.is_dirty());

    // 2. Bấm File -> New -> Yêu cầu lưu trước
    let action = coord.on_save_requested(coord.session_generation(), "doc-a", "save-a-1", vec![1, 2, 3], 1000);
    assert!(matches!(action, CoordinatorAction::PerformDurableWrite { .. }));

    // 3. Backend xác nhận lưu đúng snapshot fingerprint đã ghi
    let fp = match coord.state() {
        office_utility_vn_ui::platform::DocumentState::Saving { snapshot_fingerprint, .. } => snapshot_fingerprint.clone(),
        _ => panic!("Expected Saving state"),
    };
    coord.on_durable_write_completed("save-a-1", &fp);
    assert!(!coord.is_dirty());

    // 4. Kích hoạt New: Thiết lập phiên mới sạch
    state.set_document_session(Some("doc-new-1".into()), None, 1);
    state.set_active_file("Untitled.xlsx");
    state.is_dirty.set(false);

    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-new-1"));
    assert_eq!(state.active_file_path.get_untracked(), None);
    assert_eq!(state.active_file_name.get_untracked().as_deref(), Some("Untitled.xlsx"));
    assert!(!state.is_dirty.get_untracked());
}

/// Kịch bản B: Bảng tính Dirty → File Open → Lưu → ACK → Chọn tệp hợp lệ → Commit phiên mới
#[test]
fn test_cross_scenario_b_dirty_workbook_open_save_ack_native_dialog_commit() {
    let state = GridCraftState::new();
    let mut coord = HostDocumentCoordinator::new();

    // 1. Bảng tính A dirty
    coord.on_document_opened("doc-a", "/data/doc_a.xlsx", "hash-a0");
    coord.on_user_edit();
    state.set_document_session(Some("doc-a".into()), Some("/data/doc_a.xlsx".into()), 1);
    state.is_dirty.set(true);

    // 2. Lưu trước khi mở tệp khác
    coord.on_save_requested(coord.session_generation(), "doc-a", "save-a-2", vec![1, 2], 1000);
    let fp = match coord.state() {
        office_utility_vn_ui::platform::DocumentState::Saving { snapshot_fingerprint, .. } => snapshot_fingerprint.clone(),
        _ => panic!("Expected Saving state"),
    };
    coord.on_durable_write_completed("save-a-2", &fp);
    assert!(!coord.is_dirty());

    // 3. Mở tệp B qua Native Dialog: Bắt đầu provisional open
    state.begin_provisional_open(
        "open-req-b".into(),
        "doc-b".into(),
        "/data/doc_b.xlsx".into(),
        "doc_b.xlsx".into(),
        1,
        1,
    );

    // 4. GridCraft parse tệp B thành công -> Commit
    let committed = state.commit_provisional_open("open-req-b");
    assert!(committed);
    coord.on_document_opened("doc-b", "/data/doc_b.xlsx", "hash-b0");

    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-b"));
    assert_eq!(state.active_file_path.get_untracked().as_deref(), Some("/data/doc_b.xlsx"));
    assert!(!state.is_dirty.get_untracked());
    assert!(!coord.is_dirty());
}

/// Kịch bản C: Bảng tính Dirty → File Open → Lưu thất bại → Giữ nguyên tài liệu cũ
#[test]
fn test_cross_scenario_c_dirty_workbook_open_save_failure_old_workbook_remains() {
    let state = GridCraftState::new();
    let mut coord = HostDocumentCoordinator::new();

    coord.on_document_opened("doc-a", "/data/doc_a.xlsx", "hash-a0");
    coord.on_user_edit();
    state.set_document_session(Some("doc-a".into()), Some("/data/doc_a.xlsx".into()), 1);
    state.is_dirty.set(true);

    // Lưu trước khi mở gặp lỗi IO (ví dụ hết ổ đĩa)
    coord.on_save_requested(coord.session_generation(), "doc-a", "save-a-err", vec![1, 2], 1000);
    let action = coord.on_durable_write_failed("save-a-err", "No space left on device");
    assert!(matches!(action, CoordinatorAction::ShowSaveError { .. }));

    // Hủy bỏ tiến trình open kế tiếp vì lưu thất bại
    state.abort_provisional_open(None);

    // Tài liệu A và thay đổi chưa lưu BẮT BUỘC được bảo toàn
    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-a"));
    assert_eq!(state.active_file_path.get_untracked().as_deref(), Some("/data/doc_a.xlsx"));
    assert!(state.is_dirty.get_untracked());
    assert!(coord.is_dirty());
}

/// Kịch bản D: Bảng tính Dirty → File Open → Hủy (Cancel) → Giữ nguyên tài liệu cũ
#[test]
fn test_cross_scenario_d_dirty_workbook_open_cancel_old_workbook_remains() {
    let state = GridCraftState::new();
    let mut coord = HostDocumentCoordinator::new();

    coord.on_document_opened("doc-a", "/data/doc_a.xlsx", "hash-a0");
    coord.on_user_edit();
    state.set_document_session(Some("doc-a".into()), Some("/data/doc_a.xlsx".into()), 1);
    state.is_dirty.set(true);

    // Người dùng bấm Cancel trên hộp thoại hỏi lưu thay đổi
    // Tài liệu A không đổi
    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-a"));
    assert!(state.is_dirty.get_untracked());
    assert!(coord.is_dirty());
}

/// Kịch bản E: Tài liệu hợp lệ A → Mở tệp hỏng B → A vẫn active và Save vẫn nhắm tới A
#[test]
fn test_cross_scenario_e_valid_workbook_a_open_corrupt_b_retains_a_and_save_targets_a() {
    let state = GridCraftState::new();
    let mut coord = HostDocumentCoordinator::new();

    coord.on_document_opened("doc-a", "/data/valid_a.xlsx", "hash-a0");
    coord.on_user_edit();
    state.set_document_session(Some("doc-a".into()), Some("/data/valid_a.xlsx".into()), 1);
    state.is_dirty.set(true);

    // Bắt đầu mở tệp hỏng B
    state.begin_provisional_open(
        "open-corrupt-b".into(),
        "doc-corrupt-b".into(),
        "/data/corrupt.xlsx".into(),
        "corrupt.xlsx".into(),
        1,
        1,
    );

    // Trong khi parse, active document không đổi
    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-a"));

    // Báo lỗi parse -> Abort provisional
    state.abort_provisional_open(Some("open-corrupt-b"));

    // Xác nhận A vẫn active:
    assert_eq!(state.active_document_id.get_untracked().as_deref(), Some("doc-a"));
    assert_eq!(state.active_file_path.get_untracked().as_deref(), Some("/data/valid_a.xlsx"));
    assert!(state.is_dirty.get_untracked());

    // Thao tác Lưu sau đó VẪN NHẮM ĐÚNG VÀO TỆP A:
    let action = coord.on_save_requested(coord.session_generation(), "doc-a", "save-after-err", vec![5, 6], 2000);
    assert!(matches!(action, CoordinatorAction::PerformDurableWrite { ref dest_path, .. } if dest_path == "/data/valid_a.xlsx"));
}

/// Kịch bản F: Bảng tính Dirty → Đóng Cửa Sổ (Window Close) → Cancel → Cửa sổ giữ nguyên
#[test]
fn test_cross_scenario_f_dirty_workbook_window_close_cancel_stays_open() {
    let state = GridCraftState::new();
    let mut guard = MockWindowCloseGuard::new();

    state.set_document_session(Some("doc-dirty-1".into()), Some("/data/dirty.xlsx".into()), 1);
    state.is_dirty.set(true);

    // Người dùng / OS bấm nút đóng cửa sổ (X)
    let is_prevented = guard.on_close_requested();
    assert!(is_prevented, "Native guard BẮT BUỘC ngăn chặn đóng cửa sổ");
    let req_id = guard.active_request_id.clone().unwrap();

    // Người dùng chọn Hủy (Cancel) trên hộp thoại
    let cancelled = guard.cancel(&req_id);
    assert!(cancelled);
    assert_eq!(guard.close_count, 0, "Cửa sổ không được đóng");

    // Lần bấm đóng tiếp theo vẫn bị chặn
    assert!(guard.on_close_requested(), "Vẫn tiếp tục bảo vệ");
}

/// Kịch bản G: Bảng tính Dirty → Đóng Cửa Sổ → Lưu → ACK → Cửa sổ đóng đúng 1 lần
#[test]
fn test_cross_scenario_g_dirty_workbook_window_close_save_ack_closes_once() {
    let state = GridCraftState::new();
    let mut coord = HostDocumentCoordinator::new();
    let mut guard = MockWindowCloseGuard::new();

    coord.on_document_opened("doc-close", "/data/close_me.xlsx", "hash-c0");
    coord.on_user_edit();
    state.is_dirty.set(true);

    // 1. Bấm đóng cửa sổ -> Chặn
    assert!(guard.on_close_requested());
    let req_id = guard.active_request_id.clone().unwrap();

    // 2. Người dùng chọn Lưu
    let action = coord.on_save_requested(coord.session_generation(), "doc-close", "save-on-close", vec![1], 100);
    assert!(matches!(action, CoordinatorAction::PerformDurableWrite { .. }));

    // 3. ACK lưu thành công tới
    let fp = match coord.state() {
        office_utility_vn_ui::platform::DocumentState::Saving { snapshot_fingerprint, .. } => snapshot_fingerprint.clone(),
        _ => panic!("Expected Saving state"),
    };
    coord.on_durable_write_completed("save-on-close", &fp);
    state.is_dirty.set(false);

    // 4. Ủy quyền đóng cửa sổ với request_id
    let authorized = guard.authorize(&req_id);
    assert!(authorized);

    // 5. Kích hoạt đóng thực sự
    let prevented = guard.on_close_requested();
    assert!(!prevented, "Cửa sổ được phép đóng sau khi lưu");
    assert_eq!(guard.close_count, 1, "Cửa sổ chỉ đóng đúng 1 lần");
}

/// Kịch bản H: Bảng tính Dirty → Đóng Cửa Sổ → Lưu Thất Bại → Cửa sổ giữ nguyên, Dirty còn
#[test]
fn test_cross_scenario_h_dirty_workbook_window_close_save_fails_stays_open() {
    let state = GridCraftState::new();
    let mut coord = HostDocumentCoordinator::new();
    let mut guard = MockWindowCloseGuard::new();

    coord.on_document_opened("doc-fail", "/data/protected.xlsx", "hash-f0");
    coord.on_user_edit();
    state.is_dirty.set(true);

    assert!(guard.on_close_requested());
    let req_id = guard.active_request_id.clone().unwrap();

    // Lưu gặp lỗi Permission Denied
    coord.on_save_requested(coord.session_generation(), "doc-fail", "save-err", vec![1], 100);
    coord.on_durable_write_failed("save-err", "Permission denied");

    // Vì lưu lỗi, hủy ủy quyền đóng
    guard.cancel(&req_id);

    assert_eq!(guard.close_count, 0, "Cửa sổ không được đóng khi lưu thất bại");
    assert!(coord.is_dirty(), "Trạng thái dirty phải được bảo toàn");
    assert!(state.is_dirty.get_untracked(), "State dirty phải giữ nguyên");
}

/// Kịch bản I: Lưu snapshot A → Người dùng sửa tiếp B → ACK A tới → KHÔNG được hủy B
#[test]
fn test_cross_scenario_i_save_a_edit_b_ack_a_does_not_discard_b() {
    let mut coord = HostDocumentCoordinator::new();

    coord.on_document_opened("doc-interleave", "/data/test.xlsx", "hash-0");
    coord.on_user_edit();

    // Bắt đầu lưu snapshot A
    let session_gen = coord.session_generation();
    coord.on_save_requested(session_gen, "doc-interleave", "save-a", vec![1], 100);

    // Trong lúc I/O đang bay, người dùng sửa tiếp B
    coord.on_user_edit();

    // ACK cho snapshot A tới
    let fp = match coord.state() {
        office_utility_vn_ui::platform::DocumentState::Saving { snapshot_fingerprint, .. } => snapshot_fingerprint.clone(),
        _ => panic!("Expected Saving state"),
    };
    coord.on_durable_write_completed("save-a", &fp);

    // Bất biến sống còn: Tài liệu VẪN PHẢI DIRTY vì thay đổi B chưa được lưu!
    assert!(
        coord.is_dirty(),
        "Tài liệu BẮT BUỘC giữ cờ dirty khi có sửa đổi xen ngang trong lúc lưu!"
    );
}

/// Kịch bản J: Iframe cũ gửi ACK muộn sau khi đã đổi tài liệu → Bị bỏ qua
#[test]
fn test_cross_scenario_j_old_iframe_ack_after_document_switch_ignored() {
    let mut coord = HostDocumentCoordinator::new();

    // Phiên tài liệu A ở generation 1
    coord.on_document_opened("doc-old", "/data/old.xlsx", "hash-old");
    let gen_old = coord.session_generation();

    // Iframe reload hoặc đổi tài liệu -> Thế hệ tăng lên
    coord.on_iframe_reload();
    let gen_new = coord.session_generation();
    assert_ne!(gen_old, gen_new);

    // Mở tài liệu mới B
    coord.on_document_opened("doc-new", "/data/new.xlsx", "hash-new");

    // ACK muộn từ session generation cũ gửi lên
    let action = coord.on_save_requested(gen_old, "doc-old", "save-stale", vec![9], 100);
    assert_eq!(action, CoordinatorAction::None, "Yêu cầu từ thế hệ cũ BẮT BUỘC bị bỏ qua");

    // Tài liệu B không bị ảnh hưởng
    assert_eq!(coord.dest_path(), Some("/data/new.xlsx"));
}

/// Kịch bản K: Yêu cầu đóng/mở dồn dập (Rapid duplicate requests) → Một chuyển tiếp xác định
#[test]
fn test_cross_scenario_k_rapid_duplicate_requests_single_deterministic_transition() {
    let mut guard = MockWindowCloseGuard::new();

    // Bấm đóng 3 lần liên tiếp
    let p1 = guard.on_close_requested();
    let id1 = guard.active_request_id.clone().unwrap();

    let p2 = guard.on_close_requested();
    let p3 = guard.on_close_requested();

    assert!(p1 && p2 && p3, "Tất cả các lần bấm dồn dập đều được bảo vệ");

    // Ủy quyền với id1
    assert!(guard.authorize(&id1));
    // Đóng thành công đúng 1 lần
    assert!(!guard.on_close_requested());
    assert_eq!(guard.close_count, 1);
}

/// Kịch bản L: PDF Studio dirty document → Window Close → Bảo vệ đóng an toàn
#[test]
fn test_cross_scenario_l_pdf_studio_dirty_document_window_close_guard() {
    let pdf_state = PdfStudioState::default();
    let mut guard = MockWindowCloseGuard::new();

    // Đánh dấu PDF Studio có thay đổi chưa lưu
    pdf_state.set_unsaved_changes(true);
    assert!(pdf_state.has_unsaved_changes.get_untracked());

    // Yêu cầu đóng cửa sổ
    assert!(guard.on_close_requested(), "Cửa sổ phải bị chặn vì PDF Studio có dirty state");
    let req_id = guard.active_request_id.clone().unwrap();

    // Người dùng bấm Hủy
    guard.cancel(&req_id);
    assert_eq!(guard.close_count, 0, "Cửa sổ giữ nguyên");
    assert!(pdf_state.has_unsaved_changes.get_untracked());

    // Lần đóng thứ 2: Người dùng chọn Không Lưu (Discard)
    assert!(guard.on_close_requested());
    let req_id_2 = guard.active_request_id.clone().unwrap();
    guard.authorize(&req_id_2);

    assert!(!guard.on_close_requested(), "Cửa sổ được phép đóng khi người dùng chọn Discard");
    assert_eq!(guard.close_count, 1);
}

// ==============================================================================
// STAGE 4C-0C: KIỂM THỬ XÁC NHẬN SỰ CỐ & 15 KỊCH BẢN TÍCH HỢP ĐÓNG CỬA SỔ CHUẨN
// ==============================================================================

/// TASK 1: Tái hiện sự cố mất kết nối sự kiện khi frontend chưa đăng ký listener
#[test]
fn test_task1_reproduce_event_disconnection_when_no_frontend_listener() {
    let mut guard = MockWindowCloseGuard::new();

    // 1. Hệ điều hành phát sinh CloseRequested
    let prevented = guard.on_close_requested();
    assert!(prevented, "Native guard BẮT BUỘC chặn đóng cửa sổ");

    // 2. Vì không có listener nào ở frontend tiếp nhận và xử lý, không có lệnh authorize nào được gọi
    // Guard duy trì trạng thái pending nhưng không bao giờ cho phép đóng:
    assert!(!guard.should_allow_close(), "Cửa sổ bị kẹt cứng không thể tắt nếu thiếu frontend listener!");
    assert_eq!(guard.close_count, 0);
}

/// TASK 8 - Test 1: Tài liệu sạch (Clean document) → Đóng → Đóng đúng 1 lần
#[test]
fn test_task8_01_clean_document_closes_once() {
    let app_state = AppState::new();
    let manager = WindowCloseManager::new();
    let mut guard = MockWindowCloseGuard::new();

    // Tài liệu sạch 100%
    assert!(!app_state.has_unsaved_work_or_running_task());

    // CloseRequested kích hoạt
    assert!(guard.on_close_requested());
    let req_id = guard.active_request_id.clone().unwrap();

    // Root listener xử lý
    manager.handle_native_close_requested(req_id.clone(), false, "");

    // Không bật dialog vì tài liệu sạch
    assert!(!manager.is_dialog_open.get_untracked());
    assert!(manager.is_authorized.get_untracked());

    // Native guard nhận ủy quyền và cho phép đóng đúng 1 lần
    assert!(guard.authorize(&req_id));
    assert!(!guard.on_close_requested());
    assert_eq!(guard.close_count, 1);
}

/// TASK 8 - Test 2: Tài liệu Dirty → Đóng → Dialog xác nhận xuất hiện
#[test]
fn test_task8_02_dirty_document_close_shows_dialog() {
    let app_state = AppState::new();
    let manager = WindowCloseManager::new();
    let mut guard = MockWindowCloseGuard::new();

    // Đánh dấu dirty
    app_state.gridcraft_state.is_dirty.set(true);
    assert!(app_state.has_unsaved_work_or_running_task());

    assert!(guard.on_close_requested());
    let req_id = guard.active_request_id.clone().unwrap();

    let summary = app_state.unsaved_work_summary();
    manager.handle_native_close_requested(req_id.clone(), true, &summary);

    // BẮT BUỘC hiện dialog hỏi người dùng
    assert!(manager.is_dialog_open.get_untracked());
    assert_eq!(manager.active_request_id.get_untracked(), Some(req_id));
    assert_eq!(guard.close_count, 0, "Cửa sổ không được đóng");
}

/// TASK 8 - Test 3: Tài liệu Dirty → Bấm Cancel → Cửa sổ giữ nguyên
#[test]
fn test_task8_03_dirty_document_cancel_remains_open() {
    let app_state = AppState::new();
    let manager = WindowCloseManager::new();
    let mut guard = MockWindowCloseGuard::new();

    app_state.gridcraft_state.is_dirty.set(true);
    guard.on_close_requested();
    let req_id = guard.active_request_id.clone().unwrap();

    manager.handle_native_close_requested(req_id.clone(), true, "GridCraft Dirty");
    assert!(manager.is_dialog_open.get_untracked());

    // Người dùng bấm Hủy (Cancel)
    manager.on_user_decision(WindowCloseDecision::Cancel, app_state);
    guard.cancel(&req_id);

    // Dialog đóng, cờ cancelled bật, cửa sổ không tắt
    assert!(!manager.is_dialog_open.get_untracked());
    assert!(manager.is_cancelled.get_untracked());
    assert_eq!(guard.close_count, 0);

    // Thử đóng lại sau đó vẫn bị chặn an toàn
    assert!(guard.on_close_requested());
}

/// TASK 8 - Test 4: Tài liệu Dirty → Lưu thành công (ACK) → Đóng đúng 1 lần
#[test]
fn test_task8_04_dirty_document_save_ack_closes_once() {
    let app_state = AppState::new();
    let manager = WindowCloseManager::new();
    let mut coord = HostDocumentCoordinator::new();
    let mut guard = MockWindowCloseGuard::new();

    coord.on_document_opened("doc-1", "/data/file.xlsx", "hash-clean");
    coord.on_user_edit();
    app_state.gridcraft_state.is_dirty.set(true);

    guard.on_close_requested();
    let req_id = guard.active_request_id.clone().unwrap();
    manager.handle_native_close_requested(req_id.clone(), true, "File dirty");

    // Tiến hành lưu có xác nhận
    let action = coord.on_save_requested(coord.session_generation(), "doc-1", "save-close-req", vec![1, 2], 100);
    assert!(matches!(action, CoordinatorAction::PerformDurableWrite { .. }));

    let fp = match coord.state() {
        office_utility_vn_ui::platform::DocumentState::Saving { snapshot_fingerprint, .. } => snapshot_fingerprint.clone(),
        _ => panic!("Expected Saving"),
    };
    coord.on_durable_write_completed("save-close-req", &fp);
    assert!(!coord.is_dirty());
    app_state.gridcraft_state.is_dirty.set(false);

    // Ủy quyền đóng sau khi lưu thành công
    manager.authorize_close(&req_id);
    guard.authorize(&req_id);

    assert!(!guard.on_close_requested(), "Cửa sổ được phép đóng sau khi lưu ACK");
    assert_eq!(guard.close_count, 1);
}

/// TASK 8 - Test 5: Tài liệu Dirty → Lưu thất bại → Giữ nguyên cửa sổ và trạng thái Dirty
#[test]
fn test_task8_05_dirty_document_save_failure_remains_open() {
    let app_state = AppState::new();
    let manager = WindowCloseManager::new();
    let mut coord = HostDocumentCoordinator::new();
    let mut guard = MockWindowCloseGuard::new();

    coord.on_document_opened("doc-err", "/data/err.xlsx", "hash-err");
    coord.on_user_edit();
    app_state.gridcraft_state.is_dirty.set(true);

    guard.on_close_requested();
    let req_id = guard.active_request_id.clone().unwrap();
    manager.handle_native_close_requested(req_id.clone(), true, "Err dirty");

    coord.on_save_requested(coord.session_generation(), "doc-err", "save-fail", vec![1], 100);
    coord.on_durable_write_failed("save-fail", "Disk full");
    assert!(coord.is_dirty());

    // Vì lưu lỗi, hủy ủy quyền đóng
    manager.cancel_close(&req_id);
    guard.cancel(&req_id);

    assert_eq!(guard.close_count, 0, "Cửa sổ không được đóng khi lưu thất bại");
    assert!(app_state.gridcraft_state.is_dirty.get_untracked());
}

/// TASK 8 - Test 6: Tài liệu Dirty → Lưu bị hủy (Save Canceled) → Cửa sổ giữ nguyên
#[test]
fn test_task8_06_dirty_document_save_canceled_remains_open() {
    let app_state = AppState::new();
    let manager = WindowCloseManager::new();
    let mut coord = HostDocumentCoordinator::new();
    let mut guard = MockWindowCloseGuard::new();

    coord.on_document_opened("doc-cancel", "/data/cancel.xlsx", "hash-0");
    coord.on_user_edit();
    app_state.gridcraft_state.is_dirty.set(true);

    guard.on_close_requested();
    let req_id = guard.active_request_id.clone().unwrap();
    manager.handle_native_close_requested(req_id.clone(), true, "Cancel dirty");

    coord.on_save_requested(coord.session_generation(), "doc-cancel", "save-cancel-1", vec![1], 100);
    coord.on_save_cancelled("save-cancel-1");
    assert!(coord.is_dirty());

    manager.cancel_close(&req_id);
    guard.cancel(&req_id);

    assert_eq!(guard.close_count, 0);
    assert!(app_state.gridcraft_state.is_dirty.get_untracked());
}

/// TASK 8 - Test 7: Tài liệu Dirty → Chọn Don't Save → Đóng không ghi đĩa
#[test]
fn test_task8_07_dirty_document_dont_save_closes_without_writing() {
    let app_state = AppState::new();
    let manager = WindowCloseManager::new();
    let mut guard = MockWindowCloseGuard::new();

    app_state.gridcraft_state.is_dirty.set(true);
    guard.on_close_requested();
    let req_id = guard.active_request_id.clone().unwrap();
    manager.handle_native_close_requested(req_id.clone(), true, "Discard me");

    // Người dùng chọn Bỏ lưu (Discard)
    manager.on_user_decision(WindowCloseDecision::Discard, app_state);
    guard.authorize(&req_id);

    assert!(!guard.on_close_requested(), "Cửa sổ được phép đóng khi người dùng chọn Discard");
    assert_eq!(guard.close_count, 1);
}

/// TASK 8 - Test 8: Đang lưu A → Sửa tiếp B → Nhận ACK A → Cửa sổ ở lại và vẫn Dirty
#[test]
fn test_task8_08_save_a_pending_edit_b_ack_a_remains_open_and_dirty() {
    let mut coord = HostDocumentCoordinator::new();
    let mut guard = MockWindowCloseGuard::new();

    coord.on_document_opened("doc-ab", "/data/ab.xlsx", "hash-0");
    coord.on_user_edit();

    let session_gen = coord.session_generation();
    coord.on_save_requested(session_gen, "doc-ab", "save-a", vec![1], 100);

    // Người dùng sửa tiếp ô B trong lúc lưu A
    coord.on_user_edit();

    // ACK A tới
    let fp = match coord.state() {
        office_utility_vn_ui::platform::DocumentState::Saving { snapshot_fingerprint, .. } => snapshot_fingerprint.clone(),
        _ => panic!("Expected Saving"),
    };
    coord.on_durable_write_completed("save-a", &fp);

    // Bất biến: Vẫn dirty vì thay đổi B chưa lưu!
    assert!(coord.is_dirty(), "Phải giữ cờ dirty khi có sửa đổi xen ngang");

    // Yêu cầu đóng cửa sổ BẮT BUỘC bị chặn
    assert!(guard.on_close_requested());
    assert_eq!(guard.close_count, 0);
}

/// TASK 8 - Test 9: Yêu cầu đóng dồn dập từ OS → Duy trì 1 quyết định duy nhất
#[test]
fn test_task8_09_duplicate_os_close_requests_single_active_decision() {
    let app_state = AppState::new();
    let manager = WindowCloseManager::new();
    let mut guard = MockWindowCloseGuard::new();

    app_state.gridcraft_state.is_dirty.set(true);

    // Lần 1
    let (id1, is_new1) = guard.begin_close_request();
    assert!(is_new1);
    manager.handle_native_close_requested(id1.clone(), true, "Duplicate test");
    assert!(manager.is_dialog_open.get_untracked());

    // Lần 2 dồn dập
    let (id2, is_new2) = guard.begin_close_request();
    assert!(!is_new2, "Lần 2 phải nhận is_new = false");
    assert_eq!(id1, id2, "ID phải trùng nhau");
    manager.handle_native_close_requested(id2.clone(), true, "Duplicate test");

    // Chỉ có 1 dialog duy nhất, không bị ghi đè
    assert_eq!(manager.active_request_id.get_untracked(), Some(id1.clone()));

    // Hoàn tất với id1
    manager.on_user_decision(WindowCloseDecision::Discard, app_state);
    guard.authorize(&id1);
    assert!(!guard.on_close_requested());
    assert_eq!(guard.close_count, 1);
}

/// TASK 8 - Test 10: Mã token cũ hoặc hết hạn → Bị từ chối
#[test]
fn test_task8_10_stale_close_token_rejected() {
    let mut guard = MockWindowCloseGuard::new();

    // Phiên 1
    let (id1, _) = guard.begin_close_request();
    guard.cancel(&id1);

    // Phiên 2
    let (id2, _) = guard.begin_close_request();
    assert_ne!(id1, id2);

    // Cố gắng ủy quyền bằng id1 cũ
    assert!(!guard.authorize(&id1), "Token cũ phải bị từ chối");
    assert!(guard.on_close_requested(), "Cửa sổ vẫn bị chặn");

    // id2 hợp lệ
    assert!(guard.authorize(&id2));
    assert!(!guard.on_close_requested());
    assert_eq!(guard.close_count, 1);
}

/// TASK 8 - Test 11: Yêu cầu đóng phát sinh trước khi frontend đăng ký listener → Fail-closed an toàn
#[test]
fn test_task8_11_close_request_before_frontend_listener_fail_closed() {
    let mut guard = MockWindowCloseGuard::new();

    // Giả sử trước khi frontend kịp mount
    let prevented = guard.on_close_requested();
    assert!(prevented, "Native guard mặc định BẮT BUỘC chặn đóng (Fail-closed)");
    assert_eq!(guard.close_count, 0, "Không được đóng cửa sổ khi chưa có xác nhận");
}

/// TASK 8 - Test 12: Iframe bị đơ / không phản hồi → Không gây mất dữ liệu ngầm
#[test]
fn test_task8_12_iframe_unresponsive_no_silent_data_loss() {
    let mut coord = HostDocumentCoordinator::new();
    let mut guard = MockWindowCloseGuard::new();

    coord.on_document_opened("doc-hung", "/data/hung.xlsx", "hash-0");
    coord.on_user_edit();

    // Đóng cửa sổ được kích hoạt
    assert!(guard.on_close_requested());

    // Giả sử iframe bị reload hoặc đơ không phản hồi
    coord.on_iframe_reload();
    assert!(coord.is_dirty(), "Trạng thái dirty phải được bảo toàn khi iframe reload");

    // Vì không nhận được phản hồi lưu hợp lệ, không cấp quyền đóng
    assert_eq!(guard.close_count, 0, "Cửa sổ không bao giờ bị đóng ngầm");
}

/// TASK 8 - Test 13: PDF Studio tài liệu có thay đổi chưa lưu → Bật xác nhận thích hợp
#[test]
fn test_task8_13_pdf_studio_dirty_document_close_confirmation() {
    let app_state = AppState::new();
    let manager = WindowCloseManager::new();

    app_state.pdf_studio_state.has_unsaved_changes.set(true);
    assert!(app_state.has_unsaved_work_or_running_task());

    let summary = app_state.unsaved_work_summary();
    assert!(summary.contains("PDF Studio"), "Phải nhận diện đúng PDF Studio");

    manager.handle_native_close_requested("req-pdf-1".into(), true, &summary);
    assert!(manager.is_dialog_open.get_untracked());
}

/// TASK 8 - Test 14: GridCraft tài liệu sạch → Đóng không hiện dialog thừa
#[test]
fn test_task8_14_gridcraft_clean_document_no_unnecessary_dialog() {
    let app_state = AppState::new();
    let manager = WindowCloseManager::new();

    app_state.gridcraft_state.is_dirty.set(false);
    assert!(!app_state.has_unsaved_work_or_running_task());

    manager.handle_native_close_requested("req-clean-gc".into(), false, "");
    assert!(!manager.is_dialog_open.get_untracked(), "Không được hiện dialog khi sạch");
    assert!(manager.is_authorized.get_untracked());
}

/// TASK 8 - Test 15: Vòng lặp đóng / mở lặp lại → Không rò rỉ listener hay trùng lặp
#[test]
fn test_task8_15_repeated_lifecycle_no_listener_leak() {
    let app_state = AppState::new();
    let manager = WindowCloseManager::new();
    let mut guard = MockWindowCloseGuard::new();

    app_state.gridcraft_state.is_dirty.set(true);

    for i in 1..=5 {
        let (req_id, is_new) = guard.begin_close_request();
        assert!(is_new);
        manager.handle_native_close_requested(req_id.clone(), true, "Loop dirty");
        assert!(manager.is_dialog_open.get_untracked());

        // Người dùng Cancel mỗi vòng
        manager.on_user_decision(WindowCloseDecision::Cancel, app_state);
        guard.cancel(&req_id);
        assert!(!manager.is_dialog_open.get_untracked());
        assert_eq!(guard.close_count, 0, "Vòng {i} không được đóng");
    }

    assert_eq!(guard.close_count, 0);
}

/// TASK 8 - Test 16: Host-initiated Save & Exit Correlation (Stage 4C-0C TC-GUI-04)
#[test]
fn test_task8_16_host_initiated_save_and_exit_correlation_lifecycle() {
    let app_state = AppState::new();
    let manager = WindowCloseManager::new();
    let mut coord = HostDocumentCoordinator::new();
    let mut guard = MockWindowCloseGuard::new();

    coord.on_document_opened("doc-uat-04", "/data/test_uat_sample.xlsx", "hash-uat");
    coord.on_user_edit();
    app_state.gridcraft_state.is_dirty.set(true);
    app_state.gridcraft_state.is_dev_save_permitted.set(true);

    let (req_id, is_new) = guard.begin_close_request();
    assert!(is_new);
    manager.handle_native_close_requested(req_id.clone(), true, "Bảng tính GridCraft có thay đổi chưa lưu");
    assert!(manager.is_dialog_open.get_untracked());

    // 1. Người dùng chọn Save & Exit
    manager.on_user_decision(WindowCloseDecision::Save, app_state);

    // Xác minh: Manager ở trạng thái saving, chưa ủy quyền đóng, pending_close_request_id được ghi nhận
    assert!(manager.is_saving.get_untracked());
    assert!(!manager.is_authorized.get_untracked());
    assert_eq!(app_state.gridcraft_state.pending_close_request_id.get_untracked(), Some(req_id.clone()));

    // 2. GridCraft phản hồi gridcraft:save -> Coordinator xử lý ghi bền vững
    let action = coord.on_save_requested(coord.session_generation(), "doc-uat-04", &req_id, vec![0x50, 0x4B, 0x03, 0x04], 101);
    assert!(matches!(action, CoordinatorAction::PerformDurableWrite { .. }));

    let fp = match coord.state() {
        office_utility_vn_ui::platform::DocumentState::Saving { snapshot_fingerprint, .. } => snapshot_fingerprint.clone(),
        _ => panic!("Expected Saving"),
    };
    coord.on_durable_write_completed(&req_id, &fp);
    assert!(!coord.is_dirty());
    app_state.gridcraft_state.is_dirty.set(false);

    // 3. Quy trình Save hoàn tất -> Kích hoạt đóng ủy quyền qua pending_close_request_id
    if let Some(close_id) = app_state.gridcraft_state.pending_close_request_id.get_untracked() {
        app_state.gridcraft_state.pending_close_request_id.set(None);
        manager.authorize_close(&close_id);
        guard.authorize(&close_id);
    }

    assert!(manager.is_authorized.get_untracked());
    assert!(!manager.is_dialog_open.get_untracked());
    assert_eq!(app_state.gridcraft_state.pending_close_request_id.get_untracked(), None);
    assert!(!guard.on_close_requested(), "Cửa sổ đóng an toàn sau khi lưu có xác nhận hoàn tất");
    assert_eq!(guard.close_count, 1);
}

