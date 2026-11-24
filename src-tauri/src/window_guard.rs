use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tauri::Window;

/// Payload phát đi khi cửa sổ nhận được yêu cầu đóng từ hệ thống hoặc người dùng (CloseRequested).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseRequestedPayload {
    pub request_id: String,
}

/// Trạng thái nội bộ của bộ bảo vệ đóng cửa sổ native (WindowCloseState).
#[derive(Debug, Default)]
pub struct WindowCloseState {
    /// Mã yêu cầu đóng cửa sổ đang chờ xử lý từ phía người dùng/frontend.
    pub active_request_id: Option<String>,
    /// Mã token ủy quyền đóng cửa sổ một lần (single-use token).
    pub authorized_token: Option<String>,
    /// Đang trong quá trình đóng cửa sổ đã ủy quyền (ngăn ngừa đệ quy / vòng lặp đóng).
    pub is_closing: bool,
}

/// Bộ bảo vệ đóng cửa sổ native (WindowCloseGuard).
/// Đảm bảo không bao giờ đóng cửa sổ và hủy hoại dữ liệu chưa lưu nếu chưa có quyết định rõ ràng từ người dùng.
#[derive(Clone, Default)]
pub struct WindowCloseGuard {
    state: Arc<Mutex<WindowCloseState>>,
}

impl WindowCloseGuard {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(WindowCloseState::default())),
        }
    }

    /// Đánh giá xem sự kiện CloseRequested có được phép đóng cửa sổ hay không.
    /// Trả về true nếu đã được ủy quyền hợp lệ.
    /// Trả về false nếu chưa được ủy quyền và cần gọi api.prevent_close().
    pub fn should_allow_close(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.is_closing {
            return true;
        }
        if let Some(_token) = state.authorized_token.take() {
            state.is_closing = true;
            state.active_request_id = None;
            return true;
        }
        false
    }

    /// Khởi tạo một phiên yêu cầu đóng cửa sổ mới và trả về (request_id, is_new).
    /// Nếu đã có một yêu cầu đóng cửa sổ đang chờ (duplicate requests), trả về id hiện tại và is_new = false.
    pub fn begin_close_request(&self) -> (String, bool) {
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = &state.active_request_id {
            (existing.clone(), false)
        } else {
            let id = format!("close-req-{}", Uuid::new_v4());
            state.active_request_id = Some(id.clone());
            (id, true)
        }
    }

    /// Ủy quyền đóng cửa sổ dựa trên request_id hợp lệ.
    /// Trả về token ủy quyền một lần nếu thành công.
    pub fn authorize(&self, request_id: &str) -> Result<String, String> {
        let mut state = self.state.lock().unwrap();
        if let Some(active) = &state.active_request_id {
            if active == request_id {
                let token = format!("close-tok-{}", Uuid::new_v4());
                state.authorized_token = Some(token.clone());
                state.active_request_id = None;
                return Ok(token);
            }
        }
        Err("Mã yêu cầu đóng cửa sổ không hợp lệ hoặc đã hết hạn".to_string())
    }

    /// Hủy bỏ yêu cầu đóng cửa sổ hiện tại (khi người dùng bấm Hủy).
    pub fn cancel(&self, request_id: &str) -> bool {
        let mut state = self.state.lock().unwrap();
        if let Some(active) = &state.active_request_id {
            if active == request_id {
                state.active_request_id = None;
                return true;
            }
        }
        false
    }

    /// Kiểm tra xem cửa sổ có đang trong giai đoạn đóng hay không.
    pub fn is_closing(&self) -> bool {
        self.state.lock().unwrap().is_closing
    }

    /// Reset trạng thái (dùng cho testing).
    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        state.active_request_id = None;
        state.authorized_token = None;
        state.is_closing = false;
    }
}

/// Lệnh IPC Tauri: Người dùng hoặc coordinator xác nhận đóng cửa sổ sau khi đã lưu hoặc chọn bỏ lưu.
#[tauri::command]
pub async fn authorize_window_close(
    request_id: String,
    window: Window,
    guard: tauri::State<'_, WindowCloseGuard>,
) -> Result<(), String> {
    guard.authorize(&request_id)?;
    window.close().map_err(|e| format!("Không thể đóng cửa sổ: {e}"))
}

/// Lệnh IPC Tauri: Người dùng bấm Hủy (Cancel) trên hộp thoại hỏi lưu thay đổi.
#[tauri::command]
pub async fn cancel_window_close(
    request_id: String,
    guard: tauri::State<'_, WindowCloseGuard>,
) -> Result<(), String> {
    if guard.cancel(&request_id) {
        Ok(())
    } else {
        Err("Không tìm thấy yêu cầu đóng cửa sổ tương ứng để hủy".to_string())
    }
}

/// Lệnh IPC Tauri: Frontend kiểm tra yêu cầu đóng cửa sổ đang chờ xử lý (P2 #4683415016 Startup Safety).
/// Trả về mã request_id nếu có yêu cầu đóng phát sinh trước khi frontend kịp mount và đăng ký listener.
#[tauri::command]
pub async fn query_pending_close_request(
    guard: tauri::State<'_, WindowCloseGuard>,
) -> Result<Option<String>, String> {
    let state = guard.state.lock().unwrap();
    Ok(state.active_request_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_initially_prevents_unauthorized_close() {
        let guard = WindowCloseGuard::new();
        // Ban đầu chưa có ủy quyền, guard BẮT BUỘC ngăn chặn đóng cửa sổ
        assert!(!guard.should_allow_close(), "Cửa sổ không được phép đóng khi chưa ủy quyền");
    }

    #[test]
    fn test_guard_begin_request_and_forged_authorization_rejection() {
        let guard = WindowCloseGuard::new();
        let (valid_req, is_new) = guard.begin_close_request();
        assert!(is_new);
        assert!(valid_req.starts_with("close-req-"));

        // Thử ủy quyền với mã giả mạo
        let res = guard.authorize("forged-req-token-12345");
        assert!(res.is_err(), "Mã giả mạo phải bị từ chối");
        assert!(!guard.should_allow_close(), "Vẫn không được phép đóng sau khi giả mạo thất bại");
    }

    #[test]
    fn test_guard_duplicate_requests_single_active_id() {
        let guard = WindowCloseGuard::new();
        let (req1, is_new1) = guard.begin_close_request();
        assert!(is_new1);

        // Yêu cầu đóng thứ 2 trong khi yêu cầu 1 chưa giải quyết
        let (req2, is_new2) = guard.begin_close_request();
        assert!(!is_new2, "Yêu cầu trùng lặp phải trả về is_new = false");
        assert_eq!(req1, req2, "ID yêu cầu phải được tái sử dụng để không làm hỏng dialog");

        // Ủy quyền với req1 hợp lệ
        assert!(guard.authorize(&req1).is_ok());
        assert!(guard.should_allow_close());
    }

    #[test]
    fn test_guard_cancel_invalidates_close_request() {
        let guard = WindowCloseGuard::new();
        let (valid_req, _) = guard.begin_close_request();

        // Người dùng bấm Hủy
        assert!(guard.cancel(&valid_req));
        // Thử ủy quyền sau khi đã hủy
        let res = guard.authorize(&valid_req);
        assert!(res.is_err(), "Yêu cầu đã hủy không thể được ủy quyền tiếp");
        assert!(!guard.should_allow_close(), "Cửa sổ không được đóng sau khi hủy");
    }

    #[test]
    fn test_guard_valid_authorization_allows_close_and_sets_is_closing() {
        let guard = WindowCloseGuard::new();
        let (valid_req, _) = guard.begin_close_request();

        let token_res = guard.authorize(&valid_req);
        assert!(token_res.is_ok(), "Ủy quyền hợp lệ phải thành công");
        let token = token_res.unwrap();
        assert!(token.starts_with("close-tok-"));

        // Khi CloseRequested được kích hoạt sau đó:
        assert!(guard.should_allow_close(), "Phải cho phép đóng sau khi ủy quyền");
        assert!(guard.is_closing(), "Trạng thái closing phải được kích hoạt để tránh lặp vô tận");

        // Gọi lại should_allow_close() trong lúc is_closing vẫn trả về true
        assert!(guard.should_allow_close(), "Đang trong tiến trình closing phải cho phép thoát");
    }

    #[test]
    fn test_guard_single_use_authorization_lifecycle() {
        let guard = WindowCloseGuard::new();
        let (req1, _) = guard.begin_close_request();
        let _ = guard.authorize(&req1);

        // Lần đóng thứ nhất tiêu thụ token
        assert!(guard.should_allow_close());

        // Giả sử sau đó trạng thái được reset cho một phiên mới:
        guard.reset();
        assert!(!guard.should_allow_close(), "Sau reset, guard phải bảo vệ trở lại");
        assert!(!guard.is_closing());
    }
}
