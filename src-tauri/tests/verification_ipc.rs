//! src-tauri/tests/verification_ipc.rs
//! Bộ kiểm thử hồi quy chốt an toàn PR-01 (Safety Gate Hardening Verification)
//! Thực thi trực tiếp logic điều phối sản xuất từ tầng Khung Xương Sống

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use office_shared::{CommandError, DispatchTaskRequest, TaskProgress};
use office_utility_vn_lib::commands::{
    dispatch_tool_execution, dispatch_tool_with_persistence, execute_workflow_execution,
    execute_workflow_with_persistence, is_emergency_kill_switch_active, is_tool_enabled,
};
use office_utility_vn_lib::db;
use office_utility_vn_lib::state::AppState;

/// Khóa đồng bộ bảo vệ biến môi trường giữa các luồng test chạy song song
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Lấy đường dẫn tuyệt đối đến fixture độc lập với thư mục chạy lệnh Cargo
fn get_fixture_path(rel: &str) -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = manifest_dir.join("..").join(rel);
    if p.exists() {
        p
    } else {
        manifest_dir.join(rel)
    }
}

/// Tính mã băm SHA-256 của tệp tin trên ổ đĩa
fn compute_file_sha256(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("Không thể đọc tệp {:?}: {e}", path));
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

// =============================================================================
// TEST 1: UNKNOWN TOOL PHẢI BỊ TỪ CHỐI FAIL-CLOSED
// =============================================================================
#[test]
fn test_01_unknown_tool_fails_closed_never_reports_success() {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let unknown_slugs = vec![
        "unknown-random-tool-12345",
        "malicious_tool_payload",
        "admin-backdoor",
        "",
    ];

    for slug in unknown_slugs {
        let req = DispatchTaskRequest {
            task_id: format!("test-unknown-{slug}"),
            tool_slug: slug.to_string(),
            input_paths: vec![],
            output_directory: None,
            options: serde_json::json!({}),
        };

        let result = dispatch_tool_execution(&req, &cancel_flag, None);
        assert!(
            result.is_err(),
            "LỖI BẢO MẬT: Slug lạ '{}' không được phép trả về Ok(SUCCESS)",
            slug
        );

        match result {
            Err(CommandError::ValidationFailed { reason }) => {
                assert!(
                    reason.contains("chưa khả dụng")
                        || reason.contains("bị khóa")
                        || reason.contains("không có trong danh sách"),
                    "Lý do từ chối phải rõ ràng: {reason}"
                );
            }
            other => panic!(
                "Kỳ vọng ValidationFailed cho slug lạ, nhận được: {:?}",
                other
            ),
        }
    }
}

// =============================================================================
// TEST 2: DISABLED FAKE ENGINES PHẢI BỊ CHẶN TUYỆT ĐỐI (DÙ ĐẦU VÀO HỢP LỆ)
// =============================================================================
#[test]
fn test_02_disabled_fake_engines_reject_execution() {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let fixture_pdf = get_fixture_path("fixtures/test_uat_doc.pdf");
    assert!(
        fixture_pdf.is_file(),
        "Fixture test_uat_doc.pdf phải tồn tại"
    );
    let input_paths = vec![fixture_pdf.to_str().unwrap().to_string()];

    let fake_slugs = vec![
        "vietnamese-ocr",     // Fake hardcoded text (FAKE001)
        "pdf-to-long-image",  // Fake gray bitmap (FAKE003)
        "video-to-gif-webp",  // Fake gradient animation (FAKE002)
        "pdf-protect-sign",   // Fake XOR cipher (FAKE005)
        "pdf-pki-sign",       // Fake PKCS#7 hex string (PDF003)
        "pdf-visual-diff",    // Fake visual comparison
        "pdf-to-images-pack", // Fake image pack
    ];

    for slug in fake_slugs {
        let req = DispatchTaskRequest {
            task_id: format!("test-fake-{slug}"),
            tool_slug: slug.to_string(),
            input_paths: input_paths.clone(),
            output_directory: None,
            options: serde_json::json!({}),
        };

        let result = dispatch_tool_execution(&req, &cancel_flag, None);
        assert!(
            result.is_err(),
            "LỖI AN TOÀN: Động cơ giả mạo '{}' không được phép trả về Ok(SUCCESS)",
            slug
        );

        match result {
            Err(CommandError::ValidationFailed { reason }) => {
                assert!(
                    reason.contains("chưa khả dụng")
                        || reason.contains("bị khóa")
                        || reason.contains("không có trong danh sách"),
                    "Lý do từ chối động cơ giả mạo phải chỉ rõ trạng thái khóa an toàn: {reason}"
                );
            }
            other => panic!(
                "Động cơ giả mạo '{}' không trả về ValidationFailed, nhận được: {:?}",
                slug, other
            ),
        }
    }
}

// =============================================================================
// TEST 3: UNSAFE ROUTED PDF ENGINES PHẢI BỊ KHÓA TRƯỚC KHI ĐỤNG VÀO FILE
// =============================================================================
#[test]
fn test_03_unsafe_routed_pdf_tools_are_blocked() {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let fixture_pdf = get_fixture_path("fixtures/test_uat_doc.pdf");
    assert!(
        fixture_pdf.is_file(),
        "Fixture test_uat_doc.pdf phải tồn tại"
    );
    let input_paths = vec![fixture_pdf.to_str().unwrap().to_string()];

    let unsafe_routed_pdf = vec![
        "pdf-booklet-imposition",
        "pdf-grayscale-print",
        "pdf-repair-corrupt",
        "pdf-table-extractor",
    ];

    for slug in unsafe_routed_pdf {
        let req = DispatchTaskRequest {
            task_id: format!("test-unsafe-pdf-{slug}"),
            tool_slug: slug.to_string(),
            input_paths: input_paths.clone(),
            output_directory: None,
            options: serde_json::json!({}),
        };

        let result = dispatch_tool_execution(&req, &cancel_flag, None);
        assert!(
            result.is_err(),
            "LỖI AN TOÀN DỮ LIỆU: Công cụ PDF không an toàn '{}' không được phép trả về Ok(SUCCESS)",
            slug
        );

        match result {
            Err(CommandError::ValidationFailed { reason }) => {
                assert!(
                    reason.contains("chưa khả dụng")
                        || reason.contains("bị khóa")
                        || reason.contains("không có trong danh sách"),
                    "Thông điệp chặn PDF phải chỉ rõ trạng thái khóa an toàn: {reason}"
                );
            }
            other => panic!(
                "Công cụ PDF '{}' không trả về ValidationFailed, nhận được: {:?}",
                slug, other
            ),
        }
    }
}

// =============================================================================
// TEST 4: SOURCE PRESERVATION — SHA-256 CỦA TỆP GỐC KHÔNG ĐỔI & KHÔNG TẠO TỆP RÁC
// =============================================================================
#[test]
fn test_04_source_preservation_sha256_unmodified() {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let fixture_pdf = get_fixture_path("fixtures/test_uat_doc.pdf");
    assert!(
        fixture_pdf.is_file(),
        "Fixture test_uat_doc.pdf phải tồn tại"
    );

    let sha256_before = compute_file_sha256(&fixture_pdf);

    let parent_dir = fixture_pdf.parent().unwrap();
    let entries_before: Vec<_> = fs::read_dir(parent_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();

    let req = DispatchTaskRequest {
        task_id: "test-preservation-01".to_string(),
        tool_slug: "pdf-merge-split".to_string(),
        input_paths: vec![fixture_pdf.to_str().unwrap().to_string()],
        output_directory: None,
        options: serde_json::json!({}),
    };

    let result = dispatch_tool_execution(&req, &cancel_flag, None);
    assert!(result.is_err(), "Tác vụ unrouted phải bị từ chối");

    let sha256_after = compute_file_sha256(&fixture_pdf);
    assert_eq!(
        sha256_before, sha256_after,
        "VI PHẠM INV-02: Tệp nguồn bị sửa đổi nội dung khi tác vụ bị từ chối!"
    );

    let entries_after: Vec<_> = fs::read_dir(parent_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(
        entries_before.len(),
        entries_after.len(),
        "VI PHẠM INV-02: Xuất hiện tệp rác trên ổ đĩa khi tác vụ bị từ chối!"
    );
}

// =============================================================================
// TEST 5: HONEST PROGRESS — KHÔNG PHÁT SỰ KIỆN HOÀN TẤT KHI THẤT BẠI
// =============================================================================
#[test]
fn test_05_honest_progress_never_emits_completion_on_failure() {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let recorded_events: Arc<Mutex<Vec<TaskProgress>>> = Arc::new(Mutex::new(Vec::new()));

    let events_clone = Arc::clone(&recorded_events);
    let progress_callback = move |p: TaskProgress| {
        events_clone.lock().unwrap().push(p);
    };

    let req = DispatchTaskRequest {
        task_id: "test-progress-01".to_string(),
        tool_slug: "unrouted-test-tool".to_string(),
        input_paths: vec![],
        output_directory: None,
        options: serde_json::json!({}),
    };

    let result = dispatch_tool_execution(&req, &cancel_flag, Some(&progress_callback));
    assert!(result.is_err());

    let events = recorded_events.lock().unwrap();
    for p in events.iter() {
        assert!(
            !p.is_completed,
            "VI PHẠM INV-01: Phát sự kiện is_completed=true khi tác vụ thất bại!"
        );
        assert!(
            p.percentage < 100.0,
            "VI PHẠM INV-01: Phát sự kiện percentage=100.0 khi tác vụ thất bại!"
        );
        assert!(
            !p.message.contains("Hoàn tất thành công"),
            "VI PHẠM INV-01: Phát thông điệp 'Hoàn tất thành công' giả mạo!"
        );
    }
}

// =============================================================================
// TEST 6: VERIFIED TOOL COMPATIBILITY & HỢP ĐỒNG HÀNH VI THỰC TẾ
// =============================================================================
#[test]
fn test_06_verified_tool_compatibility_and_behavioral_contract() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let cancel_flag = Arc::new(AtomicBool::new(false));

    // 1. Kiểm thử hành vi thực tế của vietnamese-number-to-words
    {
        let req_valid = DispatchTaskRequest {
            task_id: "test-words-01".to_string(),
            tool_slug: "vietnamese-number-to-words".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: serde_json::json!({
                "number": "1250000",
                "currency": "VND",
                "use_dong_chan": true,
                "capitalize_first": true
            }),
        };
        let res = dispatch_tool_execution(&req_valid, &cancel_flag, None)
            .expect("Chuyển đổi số phải thành công");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);
        assert_eq!(
            res.output_paths[0], "Một triệu hai trăm năm mươi nghìn đồng chẵn",
            "Oracle đối chiếu: Chuỗi tiếng Việt đọc số không khớp chuẩn kế toán"
        );

        let req_neg = DispatchTaskRequest {
            task_id: "test-words-neg".to_string(),
            tool_slug: "vietnamese-number-to-words".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: serde_json::json!({
                "number": "-500000",
                "currency": "VND",
                "use_dong_chan": false,
                "capitalize_first": true
            }),
        };
        let res_neg = dispatch_tool_execution(&req_neg, &cancel_flag, None).expect("Đọc số âm");
        assert!(res_neg.output_paths[0].starts_with("Âm năm trăm"));

        let req_invalid = DispatchTaskRequest {
            task_id: "test-words-err".to_string(),
            tool_slug: "vietnamese-number-to-words".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: serde_json::json!({
                "number": "not_a_valid_number_string"
            }),
        };
        let res_err = dispatch_tool_execution(&req_invalid, &cancel_flag, None);
        assert!(
            res_err.is_err(),
            "Đầu vào chuỗi chữ phải trả về lỗi ValidationFailed"
        );
    }

    // 2. Kiểm thử hành vi thực tế của vn-cccd-mst-validator
    {
        let req_cccd = DispatchTaskRequest {
            task_id: "test-cccd-01".to_string(),
            tool_slug: "vn-cccd-mst-validator".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: serde_json::json!({
                "input_string": "001205012345"
            }),
        };
        let res_cccd =
            dispatch_tool_execution(&req_cccd, &cancel_flag, None).expect("Giải mã CCCD");
        assert_eq!(res_cccd.status, "SUCCESS");
        let decoded_json: serde_json::Value =
            serde_json::from_str(&res_cccd.output_paths[0]).expect("Parse JSON output");
        assert_eq!(decoded_json["is_valid"], true);
        assert_eq!(decoded_json["province_name"], "Hà Nội");
        assert_eq!(decoded_json["gender"], "Nam");
        assert_eq!(decoded_json["birth_year"], 2005);

        let req_mst = DispatchTaskRequest {
            task_id: "test-mst-01".to_string(),
            tool_slug: "vn-cccd-mst-validator".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: serde_json::json!({
                "input_string": "0100109106"
            }),
        };
        let res_mst = dispatch_tool_execution(&req_mst, &cancel_flag, None).expect("Kiểm tra MST");
        let mst_json: serde_json::Value =
            serde_json::from_str(&res_mst.output_paths[0]).expect("Parse MST JSON");
        assert_eq!(mst_json["is_valid"], true);

        let req_bad = DispatchTaskRequest {
            task_id: "test-cccd-bad".to_string(),
            tool_slug: "vn-cccd-mst-validator".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: serde_json::json!({
                "input_string": "12345"
            }),
        };
        let res_bad =
            dispatch_tool_execution(&req_bad, &cancel_flag, None).expect("Chạy validator");
        let bad_json: serde_json::Value =
            serde_json::from_str(&res_bad.output_paths[0]).expect("Parse bad JSON");
        assert_eq!(bad_json["type"], "Invalid");
        assert!(bad_json["reason"].as_str().unwrap().contains("không khớp"));
    }
}

// =============================================================================
// TEST 7: REGISTRY COMPLETENESS — TOÀN BỘ 58 SLUGS KHÔNG RƠI VÀO DEFAULT SUCCESS
// =============================================================================
#[test]
fn test_07_registry_completeness_all_58_slugs() {
    let inv_path = get_fixture_path("docs/engineering-baseline/tool_inventory.json");
    let content = fs::read_to_string(&inv_path).expect("Đọc tool_inventory.json");
    let inventory: Vec<serde_json::Value> =
        serde_json::from_str(&content).expect("Parse inventory");
    assert_eq!(
        inventory.len(),
        58,
        "Inventory máy đọc phải có đúng 58 công cụ"
    );

    for tool in inventory {
        let slug = tool["slug"].as_str().expect("slug");
        let enabled = is_tool_enabled(slug);

        if slug == "vietnamese-number-to-words" || slug == "vn-cccd-mst-validator" {
            assert!(
                enabled,
                "Công cụ '{}' phải được kích hoạt trong Allowlist",
                slug
            );
        } else {
            assert!(
                !enabled,
                "Công cụ '{}' chưa được cấp phép không được nằm trong Allowlist",
                slug
            );
        }
    }
}

// =============================================================================
// TEST 8: WORKFLOW PIPELINE PHẢI FAIL-CLOSED TUYỆT ĐỐI
// =============================================================================
#[test]
fn test_08_execute_workflow_pipeline_fails_closed() {
    let fixture_pdf = get_fixture_path("fixtures/test_uat_doc.pdf");
    assert!(fixture_pdf.is_file(), "Fixture phải tồn tại");

    let sha256_before = compute_file_sha256(&fixture_pdf);

    let recorded_events: Arc<Mutex<Vec<TaskProgress>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&recorded_events);
    let progress_callback = move |p: TaskProgress| {
        events_clone.lock().unwrap().push(p);
    };

    let result = execute_workflow_execution(
        "wf-test-accounting-01",
        vec![fixture_pdf.to_str().unwrap().to_string()],
        None,
        Some(&progress_callback),
    );

    assert!(
        result.is_err(),
        "VI PHẠM BẢO MẬT: execute_workflow_pipeline không được trả về Ok(SUCCESS) giả mạo!"
    );

    match result {
        Err(CommandError::ValidationFailed { reason }) => {
            assert!(
                reason.contains("tạm thời bị khóa") || reason.contains("Workflow"),
                "Lý do từ chối workflow phải rõ ràng: {reason}"
            );
        }
        other => panic!(
            "Kỳ vọng ValidationFailed cho workflow, nhận được: {:?}",
            other
        ),
    }

    let sha256_after = compute_file_sha256(&fixture_pdf);
    assert_eq!(
        sha256_before, sha256_after,
        "Tệp nguồn không được phép bị thay đổi khi workflow bị từ chối"
    );

    let events = recorded_events.lock().unwrap();
    for p in events.iter() {
        assert!(
            !p.is_completed,
            "Workflow bị từ chối không được phát is_completed=true"
        );
        assert!(
            p.percentage < 100.0,
            "Workflow bị từ chối không được phát percentage=100.0"
        );
    }
}

// =============================================================================
// TEST 9: EMERGENCY KILL-SWITCH — HỢP ĐỒNG GIÁ TRỊ VÀ MÔ PHỎNG KHỞI ĐỘNG KHÓA
// =============================================================================
#[test]
fn test_09_emergency_kill_switch_behavior() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let env_key = "OFFICE_UTILITY_EMERGENCY_FAIL_CLOSED";

    let req = DispatchTaskRequest {
        task_id: "test-kill-switch".to_string(),
        tool_slug: "vietnamese-number-to-words".to_string(),
        input_paths: vec![],
        output_directory: None,
        options: serde_json::json!({
            "number": "1000",
            "currency": "VND",
            "use_dong_chan": true,
            "capitalize_first": true
        }),
    };

    // 1. Trường hợp unset (Default / Inactive)
    unsafe {
        std::env::remove_var(env_key);
    }
    assert!(
        !is_emergency_kill_switch_active(),
        "Khi unset, kill-switch phải inactive"
    );
    assert!(dispatch_tool_execution(&req, &cancel_flag, None).is_ok());

    // 2. Các giá trị Inactive rõ ràng: "0", "false", "FaLsE", ""
    for val in &["0", "false", "FaLsE", "FALSE", ""] {
        unsafe {
            std::env::set_var(env_key, val);
        }
        assert!(
            !is_emergency_kill_switch_active(),
            "Giá trị '{val}' không được kích hoạt kill-switch"
        );
        assert!(
            dispatch_tool_execution(&req, &cancel_flag, None).is_ok(),
            "Giá trị '{val}' phải cho phép dispatch bình thường"
        );
    }

    // 3. Các giá trị không hợp lệ (Invalid Values): không được kích hoạt
    for invalid_val in &["2", "-1", "invalid", "null", "undefined", "yes", "disabled"] {
        unsafe {
            std::env::set_var(env_key, invalid_val);
        }
        assert!(
            !is_emergency_kill_switch_active(),
            "Giá trị không hợp lệ '{invalid_val}' không được kích hoạt kill-switch"
        );
        assert!(
            dispatch_tool_execution(&req, &cancel_flag, None).is_ok(),
            "Giá trị không hợp lệ '{invalid_val}' không được vô tình khóa hệ thống"
        );
    }

    // 4. Các giá trị kích hoạt hợp lệ (Active): "1", "true", "TrUe", "TRUE", kèm khoảng trắng trim
    for active_val in &["1", "true", "TrUe", "TRUE", " 1 ", " true "] {
        unsafe {
            std::env::set_var(env_key, active_val);
        }
        assert!(
            is_emergency_kill_switch_active(),
            "Giá trị '{active_val}' phải kích hoạt kill-switch thành công"
        );
        let kill_res = dispatch_tool_execution(&req, &cancel_flag, None);
        assert!(
            kill_res.is_err(),
            "Khi kill-switch active với '{active_val}', mọi dispatch phải bị từ chối"
        );
        match kill_res {
            Err(CommandError::ValidationFailed { reason }) => {
                assert!(
                    reason.contains("khóa khẩn cấp") || reason.contains("Emergency Kill-Switch"),
                    "Thông điệp từ chối phải nêu rõ lý do khóa khẩn cấp: {reason}"
                );
            }
            other => panic!("Kỳ vọng ValidationFailed, nhận được: {:?}", other),
        }
    }

    // 5. Mô phỏng khởi động ứng dụng với biến môi trường khóa sẵn (Process Startup with Lockdown)
    unsafe {
        std::env::set_var(env_key, "1");
    }
    // Ngay từ lệnh gọi đầu tiên khi ứng dụng vừa khởi động, mọi yêu cầu đều phải fail-closed
    let startup_res = dispatch_tool_execution(&req, &cancel_flag, None);
    assert!(
        startup_res.is_err(),
        "Khởi động với lockdown phải chặn ngay lập tức"
    );

    // Dọn dẹp biến môi trường
    unsafe {
        std::env::remove_var(env_key);
    }
    assert!(
        !is_emergency_kill_switch_active(),
        "Sau khi dọn dẹp, kill-switch phải inactive"
    );
}

// =============================================================================
// TEST 10: WORKFLOW PERSISTENCE — THỰC THI NHIỀU LẦN, DUY NHẤT ID, GHI NHẬN FAILED VÀ PHỤC HỒI MUTEX POISON
// =============================================================================
#[test]
fn test_10_workflow_persistence_failed_records_and_mutex_poison_recovery() {
    let fixture_pdf = get_fixture_path("fixtures/test_uat_doc.pdf");
    assert!(
        fixture_pdf.is_file(),
        "Fixture test_uat_doc.pdf phải tồn tại"
    );
    let input_paths = vec![fixture_pdf.to_str().unwrap().to_string()];

    // 1. Khởi tạo SQLite in-memory độc lập
    let conn = db::init_in_memory().expect("Khởi tạo SQLite in-memory");
    let workflow_id = "wf-test-accounting-01";

    // Đăng ký trước workflow vào bảng workflows để thỏa mãn FOREIGN KEY constraint
    let wf_dag = office_shared::WorkflowDag {
        id: workflow_id.to_string(),
        name: "Quy trình Kế toán Tự động".to_string(),
        description: Some("Quy trình kiểm thử".to_string()),
        nodes: vec![],
        edges: vec![],
    };
    db::save_workflow(&conn, &wf_dag).expect("Lưu workflow vào CSDL");

    let app_state = AppState::new(conn);

    // 2. Chạy lần 1: Bị từ chối fail-closed
    let res1 = execute_workflow_with_persistence(
        &app_state.db,
        workflow_id,
        input_paths.clone(),
        None,
        None,
    );
    assert!(res1.is_err(), "Lần 1: Workflow phải bị từ chối fail-closed");

    // 3. Chạy lần 2 trên CÙNG workflow_id: Bị từ chối fail-closed (Thử thách PRIMARY KEY collision)
    let res2 = execute_workflow_with_persistence(
        &app_state.db,
        workflow_id,
        input_paths.clone(),
        None,
        None,
    );
    assert!(res2.is_err(), "Lần 2: Workflow phải bị từ chối fail-closed");

    // 4. Kiểm tra CSDL SQLite: Xác nhận tồn tại đúng 2 bản ghi FAILED với ID khác nhau
    {
        let conn_guard = app_state.db.lock().unwrap();
        let history = db::get_task_history(&conn_guard, 50, 0).expect("Truy vấn task_history");
        assert_eq!(
            history.len(),
            2,
            "VI PHẠM BẢO TOÀN PERSISTENCE: Phải có đúng 2 bản ghi lịch sử tác vụ cho 2 lần chạy!"
        );

        // Khẳng định 2 ID phân biệt, không xung đột PRIMARY KEY
        assert_ne!(
            history[0].id, history[1].id,
            "VI PHẠM PRIMARY KEY: ID của 2 lần thực thi lặp lại không được trùng nhau!"
        );

        for record in &history {
            assert_eq!(
                record.status, "FAILED",
                "VI PHẠM INV-01: Bản ghi lịch sử tác vụ bị từ chối phải có status=FAILED!"
            );
            assert_ne!(
                record.status, "SUCCESS",
                "VI PHẠM INV-01: Tuyệt đối không được ghi nhận SUCCESS khi tác vụ chưa thực thi!"
            );
            assert_eq!(
                record.workflow_id,
                Some(workflow_id.to_string()),
                "workflow_id phải khớp với workflow được gọi"
            );
            assert!(
                record
                    .error_message
                    .as_ref()
                    .unwrap()
                    .contains("tạm thời bị khóa"),
                "error_message phải giải thích lý do từ chối trung thực"
            );
        }
    }

    // 5. Kiểm tra an toàn khi Mutex bị poisoned (Worker thread panic)
    let db_clone = Arc::clone(&app_state.db);
    let _ = std::panic::catch_unwind(move || {
        let _guard = db_clone.lock().unwrap();
        panic!("Mô phỏng worker thread gặp sự cố và làm poison SQLite mutex");
    });

    assert!(
        app_state.db.is_poisoned(),
        "Mutex CSDL phải ở trạng thái poisoned sau khi luồng giữ lock bị crash"
    );

    // 6. Chạy lần 3 khi Mutex bị poisoned: Vẫn phải thực thi an toàn và phục hồi ghi nhận vào CSDL
    let res3 =
        execute_workflow_with_persistence(&app_state.db, workflow_id, input_paths, None, None);
    assert!(
        res3.is_err(),
        "Lần 3: Workflow vẫn phải bị từ chối fail-closed"
    );

    // 7. Xác nhận bản ghi thứ 3 được phục hồi và lưu trữ thành công
    let conn_recovered = match app_state.db.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let history_after_poison =
        db::get_task_history(&conn_recovered, 50, 0).expect("Truy vấn sau poison recovery");
    assert_eq!(
        history_after_poison.len(),
        3,
        "VI PHẠM POISON RECOVERY: Bản ghi lần 3 phải được lưu thành công vào CSDL sau khi phục hồi poison!"
    );
    assert_eq!(history_after_poison[0].status, "FAILED");
}

// =============================================================================
// TEST 11: SQLITE AUDIT PERSISTENCE FAILURE INJECTION & POISON SAFETY
// =============================================================================
#[test]
fn test_11_workflow_sqlite_audit_failure_injection_and_poison_safety() {
    let fixture_pdf = get_fixture_path("fixtures/test_uat_doc.pdf");
    assert!(
        fixture_pdf.is_file(),
        "Fixture test_uat_doc.pdf phải tồn tại"
    );
    let input_paths = vec![fixture_pdf.to_str().unwrap().to_string()];

    // --- Subtest 11.1: Query Failure Injection (Bảng workflows bị hỏng / drop) ---
    {
        let conn = db::init_in_memory().expect("Khởi tạo SQLite in-memory");
        // Giả lập hỏng truy vấn: xóa bảng workflows
        conn.execute_batch("DROP TABLE workflows;")
            .expect("Drop bảng workflows");

        let db_mutex = Arc::new(Mutex::new(conn));
        let res = execute_workflow_with_persistence(
            &db_mutex,
            "wf-corrupted-query",
            input_paths.clone(),
            None,
            None,
        );

        assert!(
            res.is_err(),
            "Khi truy vấn FK gặp lỗi SQL, không được âm thầm nuốt lỗi"
        );
        match res {
            Err(CommandError::DatabaseError { details }) => {
                assert!(
                    details.contains("Lỗi truy vấn toàn vẹn khóa ngoại workflow")
                        || details.contains("no such table"),
                    "Lỗi phải phản ánh đúng DatabaseError mà không giả lập false: {details}"
                );
            }
            other => panic!(
                "Kỳ vọng DatabaseError khi truy vấn SQL thất bại, nhận được: {:?}",
                other
            ),
        }
    }

    // --- Subtest 11.2: Write Failure Injection (Trigger chặn ghi task_history) ---
    {
        let conn = db::init_in_memory().expect("Khởi tạo SQLite in-memory");
        let wf_dag = office_shared::WorkflowDag {
            id: "wf-write-fail".to_string(),
            name: "Test Write Fail".to_string(),
            description: None,
            nodes: vec![],
            edges: vec![],
        };
        db::save_workflow(&conn, &wf_dag).expect("Lưu workflow");

        // Giả lập lỗi ghi dữ liệu: cài trigger ABORT trên INSERT vào task_history
        conn.execute_batch(
            "CREATE TRIGGER abort_history BEFORE INSERT ON task_history \
             BEGIN SELECT RAISE(ABORT, 'INJECTED_DISK_WRITE_FAILURE'); END;",
        )
        .expect("Tạo trigger abort");

        let db_mutex = Arc::new(Mutex::new(conn));
        let res = execute_workflow_with_persistence(
            &db_mutex,
            "wf-write-fail",
            input_paths.clone(),
            None,
            None,
        );

        assert!(
            res.is_err(),
            "Khi ghi task_history thất bại, không được nuốt ngầm"
        );
        match res {
            Err(CommandError::DatabaseError { details }) => {
                assert!(
                    details.contains("Thất bại ghi nhận lịch sử kiểm toán")
                        && details.contains("INJECTED_DISK_WRITE_FAILURE"),
                    "Chi tiết lỗi ghi nhận kiểm toán phải rõ ràng: {details}"
                );
            }
            other => panic!(
                "Kỳ vọng DatabaseError khi ghi task_history thất bại, nhận được: {:?}",
                other
            ),
        }
    }

    // --- Subtest 11.3: Mutex Poison Safety với Dangling Transaction ---
    {
        let conn = db::init_in_memory().expect("Khởi tạo SQLite in-memory");
        let wf_dag = office_shared::WorkflowDag {
            id: "wf-poison-tx".to_string(),
            name: "Test Poison TX".to_string(),
            description: None,
            nodes: vec![],
            edges: vec![],
        };
        db::save_workflow(&conn, &wf_dag).expect("Lưu workflow");

        let db_mutex = Arc::new(Mutex::new(conn));

        // Mô phỏng luồng khác mở transaction nhưng panic trước khi COMMIT
        let db_clone = Arc::clone(&db_mutex);
        let _ = std::panic::catch_unwind(move || {
            let guard = db_clone.lock().unwrap();
            guard.execute_batch("BEGIN TRANSACTION;").unwrap();
            assert!(!guard.is_autocommit(), "Transaction đang mở");
            panic!("Worker thread panic trong lúc transaction đang dở dang");
        });

        assert!(db_mutex.is_poisoned());

        // Kiểm tra acquire_db_connection tự động ROLLBACK và khôi phục
        let res = execute_workflow_with_persistence(
            &db_mutex,
            "wf-poison-tx",
            input_paths.clone(),
            None,
            None,
        );

        assert!(res.is_err());
        // Bản ghi FAILED phải được lưu trữ sau khi rollback thành công
        let guard = db_mutex.lock().unwrap_or_else(|p| p.into_inner());
        assert!(
            guard.is_autocommit(),
            "Sau khi phục hồi, kết nối phải ở trạng thái autocommit"
        );
        let history = db::get_task_history(&guard, 10, 0).expect("Truy vấn history");
        assert_eq!(
            history.len(),
            1,
            "Bản ghi phải được lưu an toàn sau khi rollback transaction dở dang"
        );
    }
}

// =============================================================================
// TEST 12: MUTATION COVERAGE — PHÁT HIỆN ĐƯỜNG THỰC THI THẬT CỦA CÔNG CỤ PDF BỊ KHÓA
// =============================================================================
#[test]
fn test_12_unsafe_pdf_engine_execution_bypass_detection() {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let fixture_pdf = get_fixture_path("fixtures/test_uat_doc.pdf");
    assert!(
        fixture_pdf.is_file(),
        "Fixture test_uat_doc.pdf phải tồn tại"
    );

    let sha_before = compute_file_sha256(&fixture_pdf);
    let temp_out_dir = tempfile::tempdir().expect("Tạo thư mục tạm");
    let out_dir_str = temp_out_dir.path().to_str().unwrap().to_string();

    // Thử thách điều phối đối với pdf-rotate (công cụ PDF nhạy cảm)
    let req = DispatchTaskRequest {
        task_id: "test-pdf-rotate-bypass".to_string(),
        tool_slug: "pdf-rotate".to_string(),
        input_paths: vec![fixture_pdf.to_str().unwrap().to_string()],
        output_directory: Some(out_dir_str),
        options: serde_json::json!({
            "mode": "rotate",
            "rotation": 90,
            "page_selection": "all"
        }),
    };

    let result = dispatch_tool_execution(&req, &cancel_flag, None);

    // Khẳng định 1: Tuyệt đối không được trả về Ok(DispatchTaskResponse)
    assert!(
        result.is_err(),
        "VI PHẠM BẢO MẬT: Engine PDF không an toàn đã được kích hoạt chạy thật trên tệp hợp lệ!"
    );

    // Khẳng định 2: Lỗi phải đến từ cổng an toàn Deny-by-Default
    match result {
        Err(CommandError::ValidationFailed { reason }) => {
            assert!(
                reason.contains("tạm thời bị khóa")
                    || reason.contains("chưa khả dụng")
                    || reason.contains("Deny-by-Default"),
                "Lý do từ chối phải là cổng Deny-by-Default, không phải lỗi từ engine: {reason}"
            );
        }
        other => panic!(
            "Kỳ vọng ValidationFailed từ Deny-by-Default gate, nhận được: {:?}",
            other
        ),
    }

    // Khẳng định 3: Thư mục đích không được chứa bất kỳ tệp kết xuất nào từ engine PDF
    let output_entries = std::fs::read_dir(temp_out_dir.path())
        .expect("Đọc thư mục xuất")
        .count();
    assert_eq!(
        output_entries, 0,
        "VI PHẠM TOÀN VẸN DỮ LIỆU: Engine PDF đã tạo tệp đầu ra trong thư mục đích dù bị khóa!"
    );

    // Khẳng định 4: Tệp nguồn không bị biến đổi
    let sha_after = compute_file_sha256(&fixture_pdf);
    assert_eq!(
        sha_before, sha_after,
        "VI PHẠM BẤT BIẾN TỆP NGUỒN: Tệp PDF nguồn đã bị sửa đổi!"
    );
}

// =============================================================================
// TEST 13: SINGLE-TOOL MANDATORY AUDIT PERSISTENCE VÀ INJECTED FAILURE PROPAGATION
// =============================================================================
#[test]
fn test_13_single_tool_sqlite_audit_persistence_mandatory_and_failure_injection() {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let conn = db::init_in_memory().expect("Khởi tạo SQLite in-memory");
    let db_mutex = Arc::new(Mutex::new(conn));

    // 1. Happy Path: Dispatch công cụ allowlisted thành công và lưu vết task_history
    let req_ok = DispatchTaskRequest {
        task_id: "test-single-tool-audit-ok-01".to_string(),
        tool_slug: "vietnamese-number-to-words".to_string(),
        input_paths: vec![],
        output_directory: None,
        options: serde_json::json!({
            "number": "1250000",
            "currency": "VND",
            "use_dong_chan": true,
            "capitalize_first": true
        }),
    };

    let res_ok = dispatch_tool_with_persistence(&db_mutex, &req_ok, &cancel_flag, None);
    assert!(res_ok.is_ok(), "Dispatch allowlisted tool phải trả về Ok");

    {
        let guard = db_mutex.lock().unwrap();
        let history = db::get_task_history(&guard, 10, 0).expect("Truy vấn task_history");
        assert_eq!(history.len(), 1, "Kỳ vọng 1 bản ghi task_history");
        assert_eq!(history[0].id, "test-single-tool-audit-ok-01");
        assert_eq!(history[0].status, "SUCCESS");
        assert_eq!(
            history[0].tool_slug.as_deref(),
            Some("vietnamese-number-to-words")
        );
    }

    // 2. Failure Injection: Giả lập lỗi ghi CSDL (Injected Disk Write Failure)
    // Cài đặt trigger chặn INSERT vào bảng task_history
    {
        let guard = db_mutex.lock().unwrap();
        guard
            .execute_batch(
                "CREATE TRIGGER fail_single_history BEFORE INSERT ON task_history \
                 BEGIN SELECT RAISE(ABORT, 'INJECTED_DISK_WRITE_FAILURE'); END;",
            )
            .expect("Cài đặt trigger mô phỏng lỗi ghi");
    }

    let req_fail = DispatchTaskRequest {
        task_id: "test-single-tool-audit-fail-02".to_string(),
        tool_slug: "vietnamese-number-to-words".to_string(),
        input_paths: vec![],
        output_directory: None,
        options: serde_json::json!({
            "number": "500000",
        }),
    };

    let res_fail = dispatch_tool_with_persistence(&db_mutex, &req_fail, &cancel_flag, None);

    // BẮT BUỘC FAIL-CLOSED: Không được nuốt ngầm lỗi ghi kiểm toán
    assert!(
        res_fail.is_err(),
        "VI PHẠM CHÍNH SÁCH KIỂM TOÁN: Lỗi ghi SQLite task_history đã bị nuốt ngầm!"
    );

    match res_fail {
        Err(CommandError::DatabaseError { details }) => {
            assert!(
                details.contains("Thất bại ghi nhận lịch sử kiểm toán")
                    && details.contains("INJECTED_DISK_WRITE_FAILURE"),
                "Chi tiết lỗi DatabaseError phải thể hiện rõ nguyên nhân lỗi ghi: {details}"
            );
        }
        other => panic!(
            "Kỳ vọng CommandError::DatabaseError khi ghi kiểm toán thất bại, nhận được: {:?}",
            other
        ),
    }

    // 3. Phục hồi và hoàn tất kiểm toán sau khi lỗi được khắc phục
    {
        let guard = db_mutex.lock().unwrap();
        guard
            .execute_batch("DROP TRIGGER fail_single_history;")
            .expect("Hủy bỏ trigger mô phỏng lỗi");
    }

    let req_recovered = DispatchTaskRequest {
        task_id: "test-single-tool-audit-recovered-03".to_string(),
        tool_slug: "vietnamese-number-to-words".to_string(),
        input_paths: vec![],
        output_directory: None,
        options: serde_json::json!({
            "number": "750000",
        }),
    };

    let res_recovered =
        dispatch_tool_with_persistence(&db_mutex, &req_recovered, &cancel_flag, None);
    assert!(
        res_recovered.is_ok(),
        "Sau khi sửa lỗi CSDL, dispatch phải hoạt động trở lại"
    );

    {
        let guard = db_mutex.lock().unwrap();
        let history = db::get_task_history(&guard, 10, 0).expect("Truy vấn task_history");
        assert_eq!(
            history.len(),
            2,
            "Chỉ 2 bản ghi thành công được lưu trữ; bản ghi thất bại không được để lại trạng thái rác"
        );
        let ids: Vec<&str> = history.iter().map(|h| h.id.as_str()).collect();
        assert!(ids.contains(&"test-single-tool-audit-ok-01"));
        assert!(ids.contains(&"test-single-tool-audit-recovered-03"));
        assert!(!ids.contains(&"test-single-tool-audit-fail-02"));
    }
}

// =============================================================================
// ALIASES BẮT BUỘC ĐỂ THỎA MÃN source_test_inventory.json
// =============================================================================
#[test]
fn unknown_and_disabled_tools_never_return_success() {
    test_01_unknown_tool_fails_closed_never_reports_success();
    test_02_disabled_fake_engines_reject_execution();
    test_03_unsafe_routed_pdf_tools_are_blocked();
}

#[test]
fn every_advertised_tool_has_registry_entry() {
    test_07_registry_completeness_all_58_slugs();
}
