use office_engine::OfficeTool;
use office_engine::vn_office::{VietnameseNumberToWordsTool, VnCccdMstValidatorTool};
use office_shared::{
    AddonStatusResponse, CommandError, DispatchTaskRequest, DispatchTaskResponse,
    FileMetadataResponse, TaskHistoryRecord, TaskProgress, ToolPreset, ToolSettingRecord,
    WorkflowDag,
};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::{AppHandle, Manager, State};

pub mod updater;
pub use updater::*;

use crate::db;
use crate::state::AppState;

pub const ENABLED_TOOL_ALLOWLIST: &[&str] =
    &["vietnamese-number-to-words", "vn-cccd-mst-validator"];

pub fn is_emergency_kill_switch_active() -> bool {
    match std::env::var("OFFICE_UTILITY_EMERGENCY_FAIL_CLOSED") {
        Ok(val) => {
            let v = val.trim();
            v == "1" || v.eq_ignore_ascii_case("true")
        }
        Err(_) => false,
    }
}

pub fn is_tool_enabled(slug: &str) -> bool {
    ENABLED_TOOL_ALLOWLIST.contains(&slug)
}

/// Thu nhận kết nối CSDL SQLite với cơ chế đánh giá an toàn Mutex Poisoning
/// Không mặc nhiên coi into_inner() là thành công, mà phải kiểm tra tính toàn vẹn (ROLLBACK transaction dở dang và sanity ping)
pub fn acquire_db_connection<'a>(
    db_mutex: &'a std::sync::Mutex<rusqlite::Connection>,
) -> Result<std::sync::MutexGuard<'a, rusqlite::Connection>, CommandError> {
    match db_mutex.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            eprintln!(
                "[acquire_db_connection] Cảnh báo an ninh: Mutex CSDL bị poisoned do luồng trước panic"
            );
            let guard = poisoned.into_inner();
            // Đánh giá tính an toàn của kết nối SQLite:
            // 1. Nếu luồng cũ panic trong một transaction dở dang (is_autocommit() == false), thực hiện ROLLBACK
            if !guard.is_autocommit() {
                eprintln!(
                    "[acquire_db_connection] Phát hiện transaction dở dang sau panic. Đang khôi phục ROLLBACK..."
                );
                if let Err(rollback_err) = guard.execute_batch("ROLLBACK;") {
                    eprintln!("[acquire_db_connection] ROLLBACK thất bại: {rollback_err}");
                    return Err(CommandError::DatabaseError {
                        details: format!(
                            "Kết nối CSDL không thể phục hồi sau Mutex Poison (ROLLBACK thất bại): {rollback_err}"
                        ),
                    });
                }
            }
            // 2. Thực hiện sanity check kết nối
            if let Err(sanity_err) = guard.query_row("SELECT 1", [], |_| Ok(())) {
                eprintln!("[acquire_db_connection] Sanity check thất bại sau poison: {sanity_err}");
                return Err(CommandError::DatabaseError {
                    details: format!("Kết nối CSDL bị hỏng sau Mutex Poison: {sanity_err}"),
                });
            }
            eprintln!("[acquire_db_connection] Phục hồi và khử nhiễm kết nối CSDL thành công");
            Ok(guard)
        }
    }
}

/// Điểm kết nối điều phối sản xuất chung (Enforced Minimal Allowlist Deny-by-Default)
pub fn dispatch_tool_execution(
    request: &DispatchTaskRequest,
    cancel_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    progress_callback: Option<&(dyn Fn(TaskProgress) + Send + Sync)>,
) -> Result<DispatchTaskResponse, CommandError> {
    // 1. Kiểm tra biến môi trường khóa khẩn cấp toàn hệ thống (Emergency Kill-Switch)
    if is_emergency_kill_switch_active() {
        return Err(CommandError::ValidationFailed {
            reason: "Hệ thống đang trong chế độ khóa khẩn cấp (Emergency Kill-Switch Active)"
                .to_string(),
        });
    }

    // 2. Kiểm tra cờ hủy hợp tác tức thì (< 100ms)
    if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let tool_slug = request.tool_slug.as_str();

    // 3. Chính sách Deny-by-Default: Chỉ cho phép các công cụ nằm trong Allowlist đã kiểm chứng
    if !is_tool_enabled(tool_slug) {
        return Err(CommandError::ValidationFailed {
            reason: format!(
                "Công cụ '{tool_slug}' tạm thời bị khóa hoặc chưa khả dụng trong phiên bản hiện tại."
            ),
        });
    }

    // 4. Tuyến thực thi các công cụ đã được cấp phép và kiểm định hành vi
    match tool_slug {
        "vietnamese-number-to-words" => {
            let tool = VietnameseNumberToWordsTool::default();
            tool.execute(
                request,
                cancel_flag,
                progress_callback.map(|cb| cb as &dyn Fn(TaskProgress)),
            )
        }
        "vn-cccd-mst-validator" => {
            let tool = VnCccdMstValidatorTool::default();
            tool.execute(
                request,
                cancel_flag,
                progress_callback.map(|cb| cb as &dyn Fn(TaskProgress)),
            )
        }
        _ => Err(CommandError::ValidationFailed {
            reason: format!(
                "Công cụ '{tool_slug}' không có trong danh sách được cấp phép thực thi."
            ),
        }),
    }
}

/// Điểm kết nối workflow sản xuất chung (Fail-Closed: Tạm thời vô hiệu hóa để bảo đảm an toàn dữ liệu)
pub fn execute_workflow_execution(
    workflow_id: &str,
    _input_paths: Vec<String>,
    _output_directory: Option<String>,
    _progress_callback: Option<&(dyn Fn(TaskProgress) + Send + Sync)>,
) -> Result<DispatchTaskResponse, CommandError> {
    if is_emergency_kill_switch_active() {
        return Err(CommandError::ValidationFailed {
            reason: "Hệ thống đang trong chế độ khóa khẩn cấp (Emergency Kill-Switch Active)"
                .to_string(),
        });
    }

    Err(CommandError::ValidationFailed {
        reason: format!(
            "Tính năng thực thi Workflow Pipeline ('{workflow_id}') tạm thời bị khóa để bảo đảm an toàn dữ liệu."
        ),
    })
}

/// 1. Điều phối thực thi tác vụ công cụ đơn lẻ (Non-blocking Rayon/Tokio)
#[tauri::command]
pub async fn dispatch_tool_task(
    _app_handle: AppHandle,
    state: State<'_, AppState>,
    request: DispatchTaskRequest,
    on_progress: tauri::ipc::Channel<TaskProgress>,
) -> Result<DispatchTaskResponse, CommandError> {
    let task_id = request.task_id.clone();
    let cancel_flag = state.register_task(&task_id);
    let start_time = Instant::now();

    let req_clone = request.clone();
    let cancel_clone = std::sync::Arc::clone(&cancel_flag);

    let progress_sender = on_progress.clone();
    let progress_callback = Box::new(move |p: TaskProgress| {
        let _ = progress_sender.send(p);
    });

    // Thực thi trên threadpool nền để giữ Main UI 60 FPS
    let result = tauri::async_runtime::spawn_blocking(move || {
        dispatch_tool_execution(&req_clone, &cancel_clone, Some(&*progress_callback))
    })
    .await
    .map_err(|e| CommandError::ProcessExecutionError {
        message: format!("Lỗi runtime luồng nền: {e}"),
    })?;

    state.unregister_task(&task_id);

    // Ghi nhật ký vào SQLite
    let elapsed = start_time.elapsed().as_millis() as u64;
    let (status, err_msg) = match &result {
        Ok(resp) => (resp.status.clone(), None),
        Err(e) => ("FAILED".to_string(), Some(e.to_string())),
    };

    let history_record = TaskHistoryRecord {
        id: task_id,
        parent_task_id: None,
        tool_slug: Some(request.tool_slug.clone()),
        workflow_id: None,
        input_files_json: serde_json::to_string(&request.input_paths).unwrap_or_default(),
        output_files_json: serde_json::to_string(
            &result.as_ref().map(|r| &r.output_paths).unwrap_or(&vec![]),
        )
        .unwrap_or_default(),
        options_used_json: serde_json::to_string(&request.options).unwrap_or_default(),
        status,
        elapsed_ms: elapsed,
        error_message: err_msg,
        created_at: String::new(),
    };

    // Ghi nhật ký vào SQLite kèm cơ chế kiểm chứng an toàn Mutex Poisoning
    match acquire_db_connection(&state.db) {
        Ok(conn) => {
            if let Err(e) = db::add_task_history(&conn, &history_record) {
                eprintln!("[dispatch_tool_task] Lỗi ghi nhận task_history: {e}");
            }
            if let Err(e) =
                db::record_tool_usage(&conn, &request.tool_slug, &history_record.options_used_json)
            {
                eprintln!("[dispatch_tool_task] Lỗi ghi nhận tool_usage: {e}");
            }
        }
        Err(db_err) => {
            eprintln!(
                "[dispatch_tool_task] Cảnh báo kiểm toán: Không thể ghi nhận task_history do lỗi CSDL: {db_err}"
            );
        }
    }

    result
}

/// 2. Hủy tác vụ đang chạy (< 100ms)
#[tauri::command]
pub async fn cancel_task(state: State<'_, AppState>, task_id: String) -> Result<(), CommandError> {
    if state.cancel_task(&task_id) {
        Ok(())
    } else {
        Err(CommandError::ValidationFailed {
            reason: format!("Không tìm thấy tác vụ đang chạy với ID: {task_id}"),
        })
    }
}

/// Thực thi workflow kèm ghi nhận lịch sử vào SQLite với chính sách kiểm toán bất biến (Audit Trail Integrity)
/// Tuân thủ quy chuẩn:
/// 1. Khôi phục an toàn Mutex Poisoning có kiểm định ROLLBACK và Sanity Ping
/// 2. Không nuốt lỗi SQL bằng unwrap_or(false) khi kiểm tra Foreign Key
/// 3. Định danh thực thi duy nhất UUIDv4 ngăn ngừa collision
/// 4. Thất bại ghi nhật ký kiểm toán phải báo lỗi về luồng gọi thay vì âm thầm bỏ qua
pub fn execute_workflow_with_persistence(
    db_mutex: &std::sync::Mutex<rusqlite::Connection>,
    workflow_id: &str,
    input_paths: Vec<String>,
    output_directory: Option<String>,
    progress_callback: Option<&(dyn Fn(TaskProgress) + Send + Sync)>,
) -> Result<DispatchTaskResponse, CommandError> {
    let start_time = Instant::now();

    // 1. Thực thi core logic (luôn fail-closed trong PR-01)
    let res = execute_workflow_execution(
        workflow_id,
        input_paths.clone(),
        output_directory,
        progress_callback,
    );

    let elapsed = start_time.elapsed().as_millis() as u64;
    let (status, err_msg) = match &res {
        Ok(resp) => (resp.status.clone(), None),
        Err(e) => ("FAILED".to_string(), Some(e.to_string())),
    };

    // 2. Định danh duy nhất cho từng lần chạy để không vi phạm PRIMARY KEY của task_history
    let execution_id = uuid::Uuid::new_v4().to_string();

    // 3. Khôi phục an toàn Mutex CSDL có kiểm chứng toàn vẹn kết nối
    let conn = acquire_db_connection(db_mutex)?;

    // 4. Kiểm tra toàn vẹn Foreign Key: Tuyệt đối không nuốt lỗi SQL bằng unwrap_or(false)
    let workflow_fk = if workflow_id.trim().is_empty() {
        None
    } else {
        let fk_exists = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM workflows WHERE id = ?1)",
                rusqlite::params![workflow_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|e| CommandError::DatabaseError {
                details: format!("Lỗi truy vấn toàn vẹn khóa ngoại workflow '{workflow_id}': {e}"),
            })?;

        if fk_exists {
            Some(workflow_id.to_string())
        } else {
            eprintln!(
                "[execute_workflow_with_persistence] Cảnh báo: workflow_id '{workflow_id}' không tồn tại trong CSDL"
            );
            None
        }
    };

    let history_record = TaskHistoryRecord {
        id: execution_id,
        parent_task_id: None,
        tool_slug: None,
        workflow_id: workflow_fk,
        input_files_json: serde_json::to_string(&input_paths).unwrap_or_default(),
        output_files_json: serde_json::to_string(
            &res.as_ref().map(|r| &r.output_paths).unwrap_or(&vec![]),
        )
        .unwrap_or_default(),
        options_used_json: "{}".to_string(),
        status,
        elapsed_ms: elapsed,
        error_message: err_msg,
        created_at: String::new(),
    };

    // 5. Ghi nhận Task History: Thất bại ghi nhận lịch sử kiểm toán phải được báo cáo, không nuốt ngầm
    if let Err(e) = db::add_task_history(&conn, &history_record) {
        eprintln!("[execute_workflow_with_persistence] Lỗi ghi nhận task_history: {e}");
        return Err(CommandError::DatabaseError {
            details: format!("Thất bại ghi nhận lịch sử kiểm toán tác vụ: {e}"),
        });
    }

    res
}

/// 3. Thực thi chuỗi tự động hóa Workflow (Macro DAG)
#[tauri::command]
pub async fn execute_workflow_pipeline(
    _app_handle: AppHandle,
    state: State<'_, AppState>,
    workflow_id: String,
    input_paths: Vec<String>,
    output_directory: Option<String>,
    on_progress: tauri::ipc::Channel<TaskProgress>,
) -> Result<DispatchTaskResponse, CommandError> {
    let progress_sender = on_progress.clone();
    let progress_callback = Box::new(move |p: TaskProgress| {
        let _ = progress_sender.send(p);
    });

    execute_workflow_with_persistence(
        &state.db,
        &workflow_id,
        input_paths,
        output_directory,
        Some(&*progress_callback),
    )
}

/// 4. Kiểm tra nhanh chữ ký và MIME-type tệp (Deep Magic Bytes qua `infer`)
#[tauri::command]
pub async fn inspect_file_metadata(
    file_path: String,
) -> Result<FileMetadataResponse, CommandError> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err(CommandError::FileNotFound {
            path: file_path.clone(),
        });
    }

    let metadata = std::fs::metadata(path).map_err(|e| CommandError::IoError {
        details: format!("Không thể đọc metadata: {e}"),
    })?;

    // Đọc nhanh tối đa 64 bytes đầu tiên
    let mut header = [0u8; 64];
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| CommandError::IoError {
        details: format!("Không thể mở file: {e}"),
    })?;
    let bytes_read = file.read(&mut header).unwrap_or(0);

    let (mime, ext) = if let Some(kind) = infer::get(&header[..bytes_read]) {
        (kind.mime_type().to_string(), kind.extension().to_string())
    } else {
        ("application/octet-stream".to_string(), "bin".to_string())
    };

    let magic_hex: Vec<String> = header[..bytes_read.min(8)]
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect();

    Ok(FileMetadataResponse {
        file_path,
        mime_type: mime,
        extension: ext,
        file_size_bytes: metadata.len(),
        is_supported: true,
        magic_bytes_hex: magic_hex.join(" "),
    })
}

/// Đọc toàn bộ nội dung nhị phân (binary bytes) của một tệp từ đĩa cục bộ.
/// [BẢO MẬT STAGE 4C-0D CLOSURE]: Vô hiệu hóa vĩnh viễn lỗ hổng raw-path read bypass (Fail-Closed).
/// Không chấp nhận bất kỳ đường dẫn nào do JavaScript cung cấp theo NFR-SEC-01.
/// Mọi thao tác nạp tài liệu sản xuất bắt buộc phải thông qua 'native_dialog_open_file' với OS Dialog.
#[tauri::command]
pub async fn read_file_binary(_file_path: String) -> Result<Vec<u8>, CommandError> {
    Err(CommandError::ValidationFailed {
        reason: "Lệnh cũ 'read_file_binary' đã bị vô hiệu hóa vĩnh viễn theo NFR-SEC-01 để ngăn chặn lỗ hổng Arbitrary File Read. Mọi thao tác nạp tệp bắt buộc phải thông qua 'native_dialog_open_file' với hộp thoại OS do người dùng trực tiếp chọn.".to_string(),
    })
}

#[cfg(debug_assertions)]
const APPROVED_DEV_FIXTURES: &[(&str, &str)] = &[
    ("tc04_sample", "/tmp/office-utility-uat/tc04_sample.xlsx"),
    (
        "test_uat_sample",
        "/tmp/office-utility-uat/test_uat_sample.xlsx",
    ),
];

/// Tải fixture phục vụ kiểm thử phát triển / UAT cục bộ (Debug Only).
/// Tuyệt đối KHÔNG nhận đường dẫn tùy ý từ JavaScript; chỉ chấp nhận fixture_id được định nghĩa sẵn.
/// Bị loại bỏ hoàn toàn trong bản dựng Release.
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn load_dev_fixture(
    fixture_id: String,
) -> Result<crate::safe_persistence::DocumentHandle, CommandError> {
    let matched = APPROVED_DEV_FIXTURES
        .iter()
        .find(|(id, _)| *id == fixture_id)
        .ok_or_else(|| CommandError::ValidationFailed {
            reason: format!(
                "Mã kiểm thử '{}' không nằm trong danh mục fixture phát triển được cho phép",
                fixture_id
            ),
        })?;

    let path = std::path::Path::new(matched.1);

    if !path.is_file() {
        return Err(CommandError::FileNotFound {
            path: matched.1.to_string(),
        });
    }

    // Kiểm tra không được là liên kết tượng trưng (symlink)
    let meta = std::fs::symlink_metadata(path).map_err(|e| CommandError::IoError {
        details: format!("Không thể kiểm tra metadata fixture: {e}"),
    })?;
    if meta.file_type().is_symlink() {
        return Err(CommandError::ValidationFailed {
            reason: "Từ chối fixture dạng liên kết tượng trưng (symlink)".to_string(),
        });
    }

    // Giới hạn kích thước an toàn tối đa 5MB
    if meta.len() > 5 * 1024 * 1024 {
        return Err(CommandError::ValidationFailed {
            reason: "Kích thước fixture vượt quá giới hạn an toàn 5MB".to_string(),
        });
    }

    // Đọc dữ liệu và xác thực magic header định dạng XLSX (PK\x03\x04)
    let bytes = std::fs::read(path).map_err(|e| CommandError::IoError {
        details: format!("Không thể đọc nội dung fixture: {e}"),
    })?;
    if !bytes.starts_with(b"PK\x03\x04") {
        return Err(CommandError::ValidationFailed {
            reason: "Tệp fixture không phải là tệp bảng tính XLSX hợp lệ (thiếu header PK)"
                .to_string(),
        });
    }

    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "tc04_sample.xlsx".to_string());
    let file_path = path.to_string_lossy().to_string();
    let doc_id = "doc-uat-04".to_string();

    let _ = crate::safe_persistence::GLOBAL_SAVE_GRANT_MANAGER.register_document_session(
        &doc_id,
        "gridcraft-dev-instance",
        1,
        path,
        0,
    );

    Ok(crate::safe_persistence::DocumentHandle {
        document_id: doc_id,
        app_instance_id: "gridcraft-dev-instance".to_string(),
        session_generation: 1,
        file_name,
        file_path,
        initial_revision: 0,
        bytes,
    })
}

/// Ghi dữ liệu nhị phân (binary bytes) ra tệp trên đĩa cục bộ.
/// [BẢO MẬT STAGE 4B-2A CLOSURE]: Khóa hoàn toàn vĩnh viễn lỗ hổng raw-path write bypass (Fail-Closed).
/// Không chấp nhận bất kỳ đường dẫn nào do JavaScript cung cấp.
/// Mọi thao tác ghi đĩa bắt buộc phải thông qua 'secure_save_document' với Save Grant hợp lệ từ Native Dialog.
#[tauri::command]
pub async fn save_file_binary(_file_path: String, _bytes: Vec<u8>) -> Result<(), CommandError> {
    Err(CommandError::ValidationFailed {
        reason: "Lệnh cũ 'save_file_binary' đã bị vô hiệu hóa vĩnh viễn theo NFR-SEC-01. Mọi thao tác lưu tệp bắt buộc phải thông qua 'secure_save_document' với Save Grant hợp lệ từ Native Dialog.".to_string(),
    })
}

/// Ghi tài liệu an toàn tuyệt đối thông qua Native Save Grant (Stage 4B-2A Revised API).
/// Đường dẫn đích và quyền hạn được sở hữu và xác thực 100% bởi Native Backend từ grant_id.
#[tauri::command]
pub async fn secure_save_document(
    grant_id: String,
    request_id: String,
    bytes: Vec<u8>,
) -> Result<crate::safe_persistence::SaveSuccessResponse, CommandError> {
    let res = tauri::async_runtime::spawn_blocking(move || {
        crate::safe_persistence::GLOBAL_SAVE_GRANT_MANAGER.execute_granted_save(
            &grant_id,
            &request_id,
            &bytes,
        )
    })
    .await
    .map_err(|e| CommandError::ProcessExecutionError {
        message: format!("Lỗi runtime khi điều phối ghi tệp có ủy quyền: {e}"),
    })??;
    Ok(res)
}

/// Mở tệp thông qua hộp thoại OS Native File Dialog.
/// Đường dẫn được chọn trực tiếp bởi người dùng qua hệ điều hành.
#[tauri::command]
pub async fn native_dialog_open_file(
    app_instance_id: String,
    session_generation: u64,
    filter_name: String,
    extensions: Vec<String>,
) -> Result<Option<crate::safe_persistence::DocumentHandle>, CommandError> {
    let exts: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
    crate::safe_persistence::GLOBAL_SAVE_GRANT_MANAGER.open_file_via_native_dialog(
        &app_instance_id,
        session_generation,
        &filter_name,
        &exts,
    )
}

/// Lưu tệp mới (Save As) thông qua hộp thoại OS Native File Dialog.
/// Cấp phát ngay một Save Grant hợp lệ cho đường dẫn vừa chọn.
#[tauri::command]
pub async fn native_dialog_save_as(
    document_id: Option<String>,
    app_instance_id: String,
    session_generation: u64,
    default_name: Option<String>,
    filter_name: String,
    extensions: Vec<String>,
) -> Result<Option<crate::safe_persistence::SaveAsHandle>, CommandError> {
    let exts: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
    crate::safe_persistence::GLOBAL_SAVE_GRANT_MANAGER.save_as_via_native_dialog(
        document_id.as_deref(),
        &app_instance_id,
        session_generation,
        default_name.as_deref(),
        &filter_name,
        &exts,
    )
}

/// Yêu cầu phát hành Save Grant cho tài liệu hiện hành đã được người dùng ủy quyền
#[tauri::command]
pub async fn request_save_grant(
    document_id: String,
    app_instance_id: String,
    session_generation: u64,
    request_id: String,
    expected_revision: u64,
) -> Result<crate::safe_persistence::NativeSaveGrant, CommandError> {
    let grant = crate::safe_persistence::GLOBAL_SAVE_GRANT_MANAGER.issue_save_grant(
        &document_id,
        &app_instance_id,
        session_generation,
        &request_id,
        expected_revision,
        crate::safe_persistence::GrantOperationType::Save,
        crate::safe_persistence::OverwritePolicy::AllowOverwrite,
        30,
    )?;
    Ok(grant)
}

/// 5. Kiểm tra trạng thái gói mở rộng ngoại tuyến (Addon Package)
#[tauri::command]
pub async fn get_addon_status(
    _app_handle: AppHandle,
    addon_id: String,
) -> Result<AddonStatusResponse, CommandError> {
    Ok(AddonStatusResponse {
        addon_id,
        is_installed: false,
        version: None,
        local_path: None,
        size_mb: 45.0,
    })
}

/// 6. Lấy toàn bộ cấu hình công cụ & trạng thái ghim từ SQLite
#[tauri::command]
pub async fn get_all_tool_settings(
    state: State<'_, AppState>,
) -> Result<Vec<ToolSettingRecord>, CommandError> {
    let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
        details: format!("Không thể khóa DB: {e}"),
    })?;
    db::get_all_tool_settings(&conn)
}

/// 7. Ghim / Bỏ ghim công cụ lên thanh truy cập nhanh
#[tauri::command]
pub async fn set_tool_pinned(
    state: State<'_, AppState>,
    slug: String,
    is_pinned: bool,
) -> Result<(), CommandError> {
    let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
        details: format!("Không thể khóa DB: {e}"),
    })?;
    db::set_tool_pinned(&conn, &slug, is_pinned)
}

/// 8. Lưu chuỗi Workflow DAG mới vào SQLite
#[tauri::command]
pub async fn save_workflow(
    state: State<'_, AppState>,
    workflow: WorkflowDag,
) -> Result<String, CommandError> {
    let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
        details: format!("Không thể khóa DB: {e}"),
    })?;
    db::save_workflow(&conn, &workflow)
}

/// 9. Lấy danh sách tất cả chuỗi Workflow từ SQLite
#[tauri::command]
pub async fn get_all_workflows(
    state: State<'_, AppState>,
) -> Result<Vec<WorkflowDag>, CommandError> {
    let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
        details: format!("Không thể khóa DB: {e}"),
    })?;
    db::get_all_workflows(&conn)
}

/// 10. Truy vấn nhật ký lịch sử tác vụ phân trang từ SQLite
#[tauri::command]
pub async fn get_task_history(
    state: State<'_, AppState>,
    limit: u32,
    offset: u32,
) -> Result<Vec<TaskHistoryRecord>, CommandError> {
    let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
        details: format!("Không thể khóa DB: {e}"),
    })?;
    db::get_task_history(&conn, limit, offset)
}

// ============================================================================
// CÁC LỆNH BỔ TRỢ QUẢN LÝ PRESETS & CỬA SỔ
// ============================================================================

#[tauri::command]
pub async fn clear_task_history(state: State<'_, AppState>) -> Result<(), CommandError> {
    let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
        details: format!("Không thể khóa DB: {e}"),
    })?;
    db::clear_task_history(&conn)
}

#[tauri::command]
pub async fn get_tool_presets(
    state: State<'_, AppState>,
    tool_slug: String,
) -> Result<Vec<ToolPreset>, CommandError> {
    let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
        details: format!("Không thể khóa DB: {e}"),
    })?;
    db::get_tool_presets(&conn, &tool_slug)
}

#[tauri::command]
pub async fn save_tool_preset(
    state: State<'_, AppState>,
    preset: ToolPreset,
) -> Result<String, CommandError> {
    let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
        details: format!("Không thể khóa DB: {e}"),
    })?;
    db::save_tool_preset(&conn, &preset)
}

#[tauri::command]
pub async fn delete_tool_preset(
    state: State<'_, AppState>,
    preset_id: String,
) -> Result<(), CommandError> {
    let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
        details: format!("Không thể khóa DB: {e}"),
    })?;
    db::delete_tool_preset(&conn, &preset_id)
}

#[tauri::command]
pub async fn toggle_always_on_top(
    window: tauri::Window,
    enabled: bool,
) -> Result<(), CommandError> {
    window
        .set_always_on_top(enabled)
        .map_err(|e| CommandError::ProcessExecutionError {
            message: format!("Không thể bật Always-on-Top: {e}"),
        })?;
    Ok(())
}

#[tauri::command]
pub async fn minimize_window(window: tauri::Window) -> Result<(), CommandError> {
    window
        .minimize()
        .map_err(|e| CommandError::ProcessExecutionError {
            message: format!("Không thể thu nhỏ cửa sổ: {e}"),
        })
}

#[tauri::command]
pub async fn toggle_maximize_window(window: tauri::Window) -> Result<(), CommandError> {
    if window.is_maximized().unwrap_or(false) {
        window
            .unmaximize()
            .map_err(|e| CommandError::ProcessExecutionError {
                message: format!("Không thể thu nhỏ cửa sổ: {e}"),
            })
    } else {
        window
            .maximize()
            .map_err(|e| CommandError::ProcessExecutionError {
                message: format!("Không thể phóng to cửa sổ: {e}"),
            })
    }
}

#[tauri::command]
pub async fn close_window(window: tauri::Window) -> Result<(), CommandError> {
    window
        .close()
        .map_err(|e| CommandError::ProcessExecutionError {
            message: format!("Không thể đóng cửa sổ: {e}"),
        })
}

#[tauri::command]
pub async fn open_output_folder(app: AppHandle) -> Result<(), CommandError> {
    use tauri_plugin_opener::OpenerExt;
    let download_dir = app
        .path()
        .download_dir()
        .unwrap_or_else(|_| PathBuf::from("./"));
    app.opener()
        .open_path(download_dir.to_string_lossy(), None::<&str>)
        .map_err(|e| CommandError::ProcessExecutionError {
            message: format!("Không thể mở thư mục kết quả: {e}"),
        })
}
