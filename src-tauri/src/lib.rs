#![allow(clippy::default_constructed_unit_structs)]

pub mod commands;
pub mod db;
pub mod safe_persistence;
pub mod state;
pub mod window_guard;

use std::path::PathBuf;
use tauri::{Emitter, Manager};
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Xác định đường dẫn CSDL SQLite cục bộ: {app_data_dir}/office_utility.db
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("./data"));

            let db_path = app_data_dir.join("office_utility.db");
            let conn = match db::init_db(&db_path) {
                Ok(c) => c,
                Err(err) => {
                    eprintln!("Cảnh báo: Không thể khởi tạo CSDL file: {err}. Chuyển sang SQLite in-memory.");
                    db::init_in_memory().expect("Khởi tạo SQLite fallback in-memory thất bại")
                }
            };

            app.manage(AppState::new(conn));
            app.manage(window_guard::WindowCloseGuard::new());

            if std::env::var("OFFICE_UTILITY_ROLLBACK_LEGACY_DATALAB").is_ok() {
                if let Some(main_win) = app.get_webview_window("main") {
                    let _ = main_win.eval("localStorage.setItem('OFFICE_UTILITY_ROLLBACK_LEGACY_DATALAB', '1');");
                }
            } else if let Some(main_win) = app.get_webview_window("main") {
                let _ = main_win.eval("localStorage.removeItem('OFFICE_UTILITY_ROLLBACK_LEGACY_DATALAB');");
            }

            #[cfg(debug_assertions)]
            {
                let fixture_path = std::path::Path::new("/tmp/office-utility-uat/tc04_sample.xlsx");
                if fixture_path.exists() {
                    let _ = safe_persistence::GLOBAL_SAVE_GRANT_MANAGER.register_document_session(
                        "doc-uat-04",
                        "gridcraft-dev-instance",
                        1,
                        fixture_path,
                        0,
                    );
                }
            }

            // Cấu hình System Tray nếu được hỗ trợ
            #[cfg(desktop)]
            {
                use tauri::tray::TrayIconBuilder;
                let _ = TrayIconBuilder::new()
                    .tooltip("Office Utility VN - 100% On-Device")
                    .build(app);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let guard = window.app_handle().state::<window_guard::WindowCloseGuard>();
                let allow = guard.should_allow_close();
                if !allow {
                    api.prevent_close();
                    // P2 #4683415016: Tái phát (Redelivery) yêu cầu đóng với req_id hiện hành,
                    // bảo toàn tính chống trùng lặp (deduplication) mà không bỏ sót sự kiện nếu frontend vừa mount
                    let (req_id, _is_new) = guard.begin_close_request();
                    let _ = window.emit("native:close_requested", window_guard::CloseRequestedPayload {
                        request_id: req_id.clone(),
                    });
                    let _ = window.app_handle().emit("native:close_requested", window_guard::CloseRequestedPayload {
                        request_id: req_id,
                    });
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            // 10 Lệnh tiêu chuẩn theo 04-database-api.md
            commands::dispatch_tool_task,
            commands::cancel_task,
            commands::execute_workflow_pipeline,
            commands::inspect_file_metadata,
            commands::get_addon_status,
            commands::get_all_tool_settings,
            commands::set_tool_pinned,
            commands::save_workflow,
            commands::get_all_workflows,
            commands::get_task_history,
            // Các lệnh bổ trợ
            commands::clear_task_history,
            commands::get_tool_presets,
            commands::save_tool_preset,
            commands::delete_tool_preset,
            commands::toggle_always_on_top,
            commands::minimize_window,
            commands::toggle_maximize_window,
            commands::close_window,
            commands::open_output_folder,
            // Lệnh đọc/ghi tệp nhị phân có kiểm soát an ninh (Stage 4B-2A / Stage 4C-0D Closure)
            #[cfg(debug_assertions)]
            commands::read_file_binary,
            #[cfg(debug_assertions)]
            commands::load_dev_fixture,
            commands::save_file_binary,
            commands::secure_save_document,
            commands::native_dialog_open_file,
            commands::native_dialog_save_as,
            commands::request_save_grant,
            // Lệnh bảo vệ đóng cửa sổ an toàn (Stage 4C-0B Task 5 & P2 #4683415016)
            window_guard::authorize_window_close,
            window_guard::cancel_window_close,
            window_guard::query_pending_close_request,
            // Lệnh cập nhật ứng dụng (Updater Lifecycle)
            commands::check_app_update,
            commands::install_app_update,
            commands::set_update_action
        ])
        .run(tauri::generate_context!())
        .expect("Lỗi khởi chạy ứng dụng desktop Office Utility VN");
}
