use chrono::{DateTime, Utc};
use office_shared::{CommandError, UpdateCheckResponse, UpdaterPreference};
use tauri::{AppHandle, State};
use crate::db;
use crate::state::AppState;

/// Đánh giá xem có nên bật popup nhắc cập nhật cho người dùng không
pub fn evaluate_should_prompt(
    pref: &UpdaterPreference,
    _current_version: &str,
    latest_version: &str,
    manual: bool,
) -> bool {
    // 1. Kiểm tra thủ công: Luôn luôn hiển thị kết quả trực tiếp cho người dùng
    if manual {
        return true;
    }

    // 2. Bỏ qua phiên bản: Nếu người dùng đã chọn skip phiên bản này
    if let Some(skipped) = &pref.skipped_version {
        if skipped == latest_version {
            return false;
        }
    }

    // 3. Nhắc lại sau: Kiểm tra thời hạn 24 giờ
    if let Some(remind_until_str) = &pref.remind_later_until {
        if let Ok(remind_until) = DateTime::parse_from_rfc3339(remind_until_str) {
            if Utc::now() < remind_until.with_timezone(&Utc) {
                return false;
            }
        }
    }

    true
}

/// 1. Kiểm tra bản cập nhật mới (Tự động ngầm hoặc Thủ công)
#[tauri::command(rename_all = "snake_case")]
pub async fn check_app_update(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    manual: bool,
) -> Result<UpdateCheckResponse, CommandError> {
    let current_version = app_handle.package_info().version.to_string();

    // Đọc tùy chọn lưu trong CSDL SQLite
    let pref = {
        let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
            details: format!("Lỗi khóa CSDL: {e}"),
        })?;
        db::get_updater_preference(&conn)?
    };

    // Kiểm tra bản cập nhật:
    // Hỗ trợ graceful fallback nếu chạy ở môi trường offline hoặc dev test
    let (has_update, latest_version, release_notes, release_date) = {
        #[cfg(feature = "updater-plugin")]
        {
            use tauri_plugin_updater::UpdaterExt;
            match app_handle.updater() {
                Ok(updater_builder) => match updater_builder.check().await {
                    Ok(Some(update)) => {
                        let latest = update.version.clone();
                        let notes = update.body.clone();
                        let date = update.date.map(|d| d.to_string());
                        (true, latest, notes, date)
                    }
                    Ok(None) => (false, current_version.clone(), None, None),
                    Err(err) => {
                        if manual {
                            return Err(CommandError::ProcessExecutionError {
                                message: format!("Không thể kết nối đến máy chủ cập nhật: {err}"),
                            });
                        }
                        (false, current_version.clone(), None, None)
                    }
                },
                Err(_) => (false, current_version.clone(), None, None),
            }
        }
        #[cfg(not(feature = "updater-plugin"))]
        {
            // Trong chế độ phát triển nội bộ hoặc kiểm thử không bật updater-plugin:
            // Mặc định phản hồi sạch dựa trên phiên bản hiện tại
            (false, current_version.clone(), None, None)
        }
    };

    let should_prompt = if has_update {
        evaluate_should_prompt(&pref, &current_version, &latest_version, manual)
    } else {
        false
    };

    // Cập nhật mốc last_checked_at vào SQLite
    {
        let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
            details: format!("Lỗi khóa CSDL: {e}"),
        })?;
        let mut updated_pref = pref.clone();
        updated_pref.last_checked_at = Some(Utc::now().to_rfc3339());
        db::save_updater_preference(&conn, &updated_pref)?;
    }

    Ok(UpdateCheckResponse {
        has_update,
        current_version,
        latest_version,
        release_notes,
        release_date,
        should_prompt,
    })
}

/// 2. Tiến hành tải và cài đặt bản cập nhật, sau đó khởi động lại
#[tauri::command]
pub async fn install_app_update(_app_handle: AppHandle) -> Result<(), CommandError> {
    #[cfg(feature = "updater-plugin")]
    {
        use tauri_plugin_updater::UpdaterExt;
        let updater = _app_handle
            .updater()
            .map_err(|e| CommandError::ProcessExecutionError {
                message: format!("Không thể khởi tạo updater: {e}"),
            })?;

        if let Some(update) = updater.check().await.map_err(|e| CommandError::ProcessExecutionError {
            message: format!("Kiểm tra bản cập nhật thất bại: {e}"),
        })? {
            update
                .download_and_install(|_chunk_len, _total_len| {}, || {})
                .await
                .map_err(|e| CommandError::ProcessExecutionError {
                    message: format!("Tải và áp dụng bản cập nhật thất bại: {e}"),
                })?;
            _app_handle.restart();
        }
        Ok(())
    }
    #[cfg(not(feature = "updater-plugin"))]
    {
        Ok(())
    }
}

/// 3. Thiết lập lựa chọn của người dùng (NOW, LATER, SKIP)
#[tauri::command(rename_all = "snake_case")]
pub async fn set_update_action(
    state: State<'_, AppState>,
    action: String,
    target_version: Option<String>,
) -> Result<(), CommandError> {
    let conn = state.db.lock().map_err(|e| CommandError::DatabaseError {
        details: format!("Lỗi khóa CSDL: {e}"),
    })?;

    let mut pref = db::get_updater_preference(&conn)?;
    match action.to_uppercase().as_str() {
        "SKIP" => {
            pref.skipped_version = target_version;
        }
        "LATER" => {
            // Hẹn nhắc lại sau 24 giờ
            let next_day = Utc::now() + chrono::Duration::hours(24);
            pref.remind_later_until = Some(next_day.to_rfc3339());
        }
        "NOW" => {
            // Xóa mốc hoãn nếu người dùng chủ động cập nhật
            pref.remind_later_until = None;
        }
        _ => {}
    }
    pref.last_checked_at = Some(Utc::now().to_rfc3339());
    db::save_updater_preference(&conn, &pref)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_prompt_decision_logic() {
        let current_version = "0.1.0";
        let latest_version = "0.2.0";

        // Case 1: Brand new update, no preference -> should prompt
        let pref = UpdaterPreference::default();
        assert!(evaluate_should_prompt(&pref, current_version, latest_version, false));

        // Case 2: Skipped this version -> should not prompt on background check
        let pref_skipped = UpdaterPreference {
            skipped_version: Some("0.2.0".to_string()),
            remind_later_until: None,
            last_checked_at: None,
        };
        assert!(!evaluate_should_prompt(&pref_skipped, current_version, latest_version, false));
        // But manual check overrides skip!
        assert!(evaluate_should_prompt(&pref_skipped, current_version, latest_version, true));

        // Case 3: Remind later in the future -> should not prompt on background check
        let in_future = (Utc::now() + chrono::Duration::hours(12)).to_rfc3339();
        let pref_later = UpdaterPreference {
            skipped_version: None,
            remind_later_until: Some(in_future),
            last_checked_at: None,
        };
        assert!(!evaluate_should_prompt(&pref_later, current_version, latest_version, false));
        // But manual check overrides remind later!
        assert!(evaluate_should_prompt(&pref_later, current_version, latest_version, true));

        // Case 4: Remind later in the past (expired) -> should prompt
        let in_past = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let pref_expired = UpdaterPreference {
            skipped_version: None,
            remind_later_until: Some(in_past),
            last_checked_at: None,
        };
        assert!(evaluate_should_prompt(&pref_expired, current_version, latest_version, false));
    }

    #[test]
    fn test_set_update_action_logic() {
        let conn = db::init_in_memory().unwrap();
        let state = AppState::new(conn);

        // Test SKIP action
        {
            let conn = state.db.lock().unwrap();
            let mut pref = db::get_updater_preference(&conn).unwrap();
            pref.skipped_version = Some("0.3.0".to_string());
            db::save_updater_preference(&conn, &pref).unwrap();
        }

        {
            let conn = state.db.lock().unwrap();
            let loaded = db::get_updater_preference(&conn).unwrap();
            assert_eq!(loaded.skipped_version.as_deref(), Some("0.3.0"));
        }
    }
}
