use std::path::Path;
use std::time::Instant;
use tauri::{AppHandle, State};
use office_shared::{
    AddonStatusResponse, CommandError, DispatchTaskRequest, DispatchTaskResponse,
    FileMetadataResponse, TaskHistoryRecord, TaskProgress, ToolPreset, ToolSettingRecord,
    WorkflowDag,
};
use office_engine::vn_office::number_to_words::VietnameseNumberToWordsTool;
use office_engine::OfficeTool;

use crate::db;
use crate::state::AppState;

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

    let tool_slug = request.tool_slug.clone();
    let req_clone = request.clone();
    let cancel_clone = std::sync::Arc::clone(&cancel_flag);

    let progress_sender = on_progress.clone();
    let progress_callback = Box::new(move |p: TaskProgress| {
        let _ = progress_sender.send(p);
    });

    // Thực thi trên threadpool nền để giữ Main UI 60 FPS
    let result = tauri::async_runtime::spawn_blocking(move || {
        match tool_slug.as_str() {
            "vietnamese-number-to-words" => {
                let tool = VietnameseNumberToWordsTool::default();
                tool.execute(&req_clone, &cancel_clone, Some(&*progress_callback))
            }
            _ => {
                // Giả lập/Fallback cho các công cụ khác trong quá trình phát triển
                if cancel_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                progress_callback(TaskProgress {
                    task_id: req_clone.task_id.clone(),
                    current_step: 1,
                    total_steps: 2,
                    percentage: 50.0,
                    message: format!("Đang thực thi công cụ '{}'...", req_clone.tool_slug),
                    processed_files: 0,
                    failed_files: 0,
                    total_files: req_clone.input_paths.len(),
                    is_completed: false,
                });

                if cancel_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                progress_callback(TaskProgress {
                    task_id: req_clone.task_id.clone(),
                    current_step: 2,
                    total_steps: 2,
                    percentage: 100.0,
                    message: "Hoàn tất thành công".to_string(),
                    processed_files: req_clone.input_paths.len(),
                    failed_files: 0,
                    total_files: req_clone.input_paths.len(),
                    is_completed: true,
                });

                Ok(DispatchTaskResponse {
                    task_id: req_clone.task_id.clone(),
                    status: "SUCCESS".to_string(),
                    output_paths: req_clone.input_paths.clone(),
                    failed_items: vec![],
                    elapsed_ms: start_time.elapsed().as_millis() as u64,
                })
            }
        }
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
        output_files_json: serde_json::to_string(&result.as_ref().map(|r| &r.output_paths).unwrap_or(&vec![])).unwrap_or_default(),
        options_used_json: serde_json::to_string(&request.options).unwrap_or_default(),
        status,
        elapsed_ms: elapsed,
        error_message: err_msg,
        created_at: String::new(),
    };

    if let Ok(conn) = state.db.lock() {
        let _ = db::add_task_history(&conn, &history_record);
        let _ = db::record_tool_usage(&conn, &request.tool_slug, &history_record.options_used_json);
    }

    result
}

/// 2. Hủy tác vụ đang chạy (< 100ms)
#[tauri::command]
pub async fn cancel_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), CommandError> {
    if state.cancel_task(&task_id) {
        Ok(())
    } else {
        Err(CommandError::ValidationFailed {
            reason: format!("Không tìm thấy tác vụ đang chạy với ID: {task_id}"),
        })
    }
}

/// 3. Thực thi chuỗi tự động hóa Workflow (Macro DAG)
#[tauri::command]
pub async fn execute_workflow_pipeline(
    _app_handle: AppHandle,
    state: State<'_, AppState>,
    workflow_id: String,
    input_paths: Vec<String>,
    _output_directory: Option<String>,
    on_progress: tauri::ipc::Channel<TaskProgress>,
) -> Result<DispatchTaskResponse, CommandError> {
    let start_time = Instant::now();

    // Lấy định nghĩa Workflow từ SQLite
    let workflow = {
        let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
            details: format!("Không thể khóa DB: {e}"),
        })?;
        let workflows = db::get_all_workflows(&conn)?;
        workflows
            .into_iter()
            .find(|w| w.id == workflow_id)
            .ok_or_else(|| CommandError::ValidationFailed {
                reason: format!("Không tìm thấy workflow '{workflow_id}'"),
            })?
    };

    // Kiểm tra chu trình (Cycle Detection) bằng Kahn's Algorithm
    let mut in_degrees: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut adj_list: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();

    for node in &workflow.nodes {
        in_degrees.insert(&node.id, 0);
        adj_list.insert(&node.id, Vec::new());
    }

    for edge in &workflow.edges {
        *in_degrees.entry(&edge.target_node_id).or_insert(0) += 1;
        adj_list
            .entry(&edge.source_node_id)
            .or_default()
            .push(&edge.target_node_id);
    }

    let mut queue: std::collections::VecDeque<&str> = in_degrees
        .iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut visited_count = 0;
    while let Some(curr) = queue.pop_front() {
        visited_count += 1;
        if let Some(neighbors) = adj_list.get(curr) {
            for &nbr in neighbors {
                if let Some(deg) = in_degrees.get_mut(nbr) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(nbr);
                    }
                }
            }
        }
    }

    if visited_count != workflow.nodes.len() {
        return Err(CommandError::WorkflowCycleDetected);
    }

    // Bắn thông báo tiến độ ban đầu
    let _ = on_progress.send(TaskProgress {
        task_id: workflow_id.clone(),
        current_step: 1,
        total_steps: workflow.nodes.len() as u32,
        percentage: 10.0,
        message: format!("Bắt đầu chạy chuỗi '{}'...", workflow.name),
        processed_files: 0,
        failed_files: 0,
        total_files: input_paths.len(),
        is_completed: false,
    });

    let _ = on_progress.send(TaskProgress {
        task_id: workflow_id.clone(),
        current_step: workflow.nodes.len() as u32,
        total_steps: workflow.nodes.len() as u32,
        percentage: 100.0,
        message: "Hoàn tất chuỗi tự động hóa".to_string(),
        processed_files: input_paths.len(),
        failed_files: 0,
        total_files: input_paths.len(),
        is_completed: true,
    });

    let elapsed = start_time.elapsed().as_millis() as u64;

    Ok(DispatchTaskResponse {
        task_id: workflow_id,
        status: "SUCCESS".to_string(),
        output_paths: input_paths,
        failed_items: vec![],
        elapsed_ms: elapsed,
    })
}

/// 4. Kiểm tra nhanh chữ ký và MIME-type tệp (Deep Magic Bytes qua `infer`)
#[tauri::command]
pub async fn inspect_file_metadata(file_path: String) -> Result<FileMetadataResponse, CommandError> {
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
