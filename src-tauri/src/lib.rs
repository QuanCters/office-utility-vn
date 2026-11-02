#![allow(clippy::default_constructed_unit_structs)]

pub mod commands;
pub mod db;
pub mod state;

use std::path::PathBuf;
use tauri::Manager;
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
            // Lệnh cập nhật ứng dụng (Updater Lifecycle)
            commands::check_app_update,
            commands::install_app_update,
            commands::set_update_action
        ])
        .run(tauri::generate_context!())
        .expect("Lỗi khởi chạy ứng dụng desktop Office Utility VN");
}
