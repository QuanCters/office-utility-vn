use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Quyết định của người dùng từ hộp thoại xác nhận đóng cửa sổ khi có thay đổi chưa lưu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowCloseDecision {
    /// Lưu tài liệu trước khi đóng
    Save,
    /// Bỏ qua thay đổi chưa lưu và đóng ngay lập tức
    Discard,
    /// Hủy bỏ yêu cầu đóng và ở lại ứng dụng
    Cancel,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

/// Bộ điều phối đóng cửa sổ an toàn cấp ứng dụng (WindowCloseManager).
/// Kết nối sự kiện `native:close_requested` từ Tauri backend với trạng thái dirty của các micro-apps.
#[derive(Clone, Copy, Debug)]
pub struct WindowCloseManager {
    /// Trạng thái mở/đóng của hộp thoại xác nhận Unsaved Changes
    pub is_dialog_open: RwSignal<bool>,
    /// Mã yêu cầu đóng cửa sổ hiện hành từ native guard
    pub active_request_id: RwSignal<Option<String>>,
    /// Mô tả chi tiết dữ liệu/tác vụ chưa lưu để hiển thị trên UI
    pub dirty_summary: RwSignal<String>,
    /// Đang trong quá trình lưu dữ liệu trước khi đóng
    pub is_saving: RwSignal<bool>,
    /// Thông báo lỗi nếu lưu thất bại
    pub error_message: RwSignal<Option<String>>,
    /// Đã ủy quyền đóng cửa sổ thành công
    pub is_authorized: RwSignal<bool>,
    /// Đã hủy bỏ yêu cầu đóng cửa sổ
    pub is_cancelled: RwSignal<bool>,
}

impl Default for WindowCloseManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowCloseManager {
    pub fn new() -> Self {
        Self {
            is_dialog_open: RwSignal::new(false),
            active_request_id: RwSignal::new(None),
            dirty_summary: RwSignal::new(String::new()),
            is_saving: RwSignal::new(false),
            error_message: RwSignal::new(None),
            is_authorized: RwSignal::new(false),
            is_cancelled: RwSignal::new(false),
        }
    }

    /// Xử lý sự kiện `native:close_requested` từ native backend.
    /// Nếu tài liệu sạch (không có unsaved work) -> tự động ủy quyền đóng ngay, không hiện dialog.
    /// Nếu tài liệu dirty -> kích hoạt dialog xác nhận Save / Discard / Cancel.
    pub fn handle_native_close_requested(
        &self,
        request_id: String,
        has_unsaved_work: bool,
        summary: &str,
    ) {
        // Task 5: Không cho phép duplicate requests sinh ra overlapping dialogs
        if self.is_dialog_open.get_untracked() || self.active_request_id.with_untracked(|id| id.is_some()) {
            return;
        }

        if !has_unsaved_work {
            // Task 3: Tài liệu sạch và không có tác vụ dở dang -> Tự động ủy quyền đóng ngay
            self.authorize_close(&request_id);
        } else {
            // Task 4: Có thay đổi chưa lưu -> Bật dialog xác nhận
            self.active_request_id.set(Some(request_id));
            self.dirty_summary.set(summary.to_string());
            self.is_dialog_open.set(true);
            self.error_message.set(None);
            self.is_saving.set(false);
            self.is_cancelled.set(false);
        }
    }

    /// Ủy quyền cho phép đóng cửa sổ native với request_id tương ứng
    pub fn authorize_close(&self, request_id: &str) {
        self.is_authorized.set(true);
        self.is_dialog_open.set(false);
        self.active_request_id.set(None);
        self.is_saving.set(false);
        let _ = request_id;

        #[cfg(target_arch = "wasm32")]
        {
            let req_id_str = request_id.to_string();
            wasm_bindgen_futures::spawn_local(async move {
                let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "requestId": req_id_str })).unwrap();
                let _ = invoke("authorize_window_close", args).await;
            });
        }
    }

    /// Hủy bỏ yêu cầu đóng cửa sổ (người dùng chọn Hủy / Cancel)
    pub fn cancel_close(&self, request_id: &str) {
        self.is_cancelled.set(true);
        self.is_dialog_open.set(false);
        self.active_request_id.set(None);
        self.is_saving.set(false);
        let _ = request_id;

        #[cfg(target_arch = "wasm32")]
        {
            let req_id_str = request_id.to_string();
            wasm_bindgen_futures::spawn_local(async move {
                let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "requestId": req_id_str })).unwrap();
                let _ = invoke("cancel_window_close", args).await;
            });
        }
    }

    /// Xử lý quyết định từ giao diện người dùng
    pub fn on_user_decision(&self, decision: WindowCloseDecision, app_state: crate::app::AppState) {
        let req_id = match self.active_request_id.get_untracked() {
            Some(id) => id,
            None => return,
        };

        match decision {
            WindowCloseDecision::Cancel => {
                app_state.gridcraft_state.pending_close_request_id.set(None);
                self.cancel_close(&req_id);
            }
            WindowCloseDecision::Discard => {
                app_state.gridcraft_state.pending_close_request_id.set(None);
                // Người dùng xác nhận bỏ lưu: Đóng cửa sổ không ghi đĩa
                self.authorize_close(&req_id);
            }
            WindowCloseDecision::Save => {
                self.is_saving.set(true);
                self.error_message.set(None);
                let manager_clone = *self;
                let req_clone = req_id.clone();

                // 1. Kiểm tra GridCraft
                if app_state.gridcraft_state.is_dirty.get_untracked() {
                    // [Stage 4C-1 Product Cutover]: Kích hoạt Host-Initiated Save tới GridCraft iframe
                    app_state
                        .gridcraft_state
                        .pending_close_request_id
                        .set(Some(req_clone.clone()));

                    let dispatched = crate::micro_apps::gridcraft::workspace::send_request_save_to_iframe(
                        crate::micro_apps::gridcraft::workspace::GRIDCRAFT_IFRAME_ID,
                        Some(&req_clone),
                        false,
                    );

                    if !dispatched {
                        self.is_saving.set(false);
                        app_state.gridcraft_state.pending_close_request_id.set(None);
                        self.error_message.set(Some(
                            "Không tìm thấy khung tài liệu GridCraft để lưu tự động. Vui lòng mở lại tài liệu hoặc hủy thay đổi trước khi đóng.".to_string(),
                        ));
                        self.cancel_close(&req_clone);
                        return;
                    }
                    // Giữ is_saving = true để hiển thị trạng thái đang lưu khi đã gửi thành công tới iframe
                    return;
                }

                // 2. Nếu có tác vụ hoặc micro-app khác (PDF Studio, Data Lab) còn dữ liệu chưa lưu:
                if app_state.has_unsaved_work_or_running_task() {
                    self.is_saving.set(false);
                    self.error_message.set(Some(
                        "Vui lòng hoàn tất lưu tệp trực tiếp trong ứng dụng trước khi đóng cửa sổ."
                            .to_string(),
                    ));
                    self.cancel_close(&req_clone);
                    return;
                }

                // 3. Nếu tài liệu hoàn toàn Clean:
                manager_clone.authorize_close(&req_clone);
            }
        }
    }
}

/// Bộ đóng bọc an toàn thread-safe cho dọn dẹp WebAssembly (WasmCleanupHandle).
/// Trong môi trường WebAssembly trình duyệt đơn luồng, struct này giúp thỏa mãn bound Send + Sync của Leptos on_cleanup.
#[cfg(target_arch = "wasm32")]
pub struct WasmCleanupHandle(Box<dyn FnOnce()>);

#[cfg(target_arch = "wasm32")]
unsafe impl Send for WasmCleanupHandle {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for WasmCleanupHandle {}

#[cfg(target_arch = "wasm32")]
impl WasmCleanupHandle {
    pub fn new<F: FnOnce() + 'static>(f: F) -> Self {
        Self(Box::new(f))
    }

    pub fn run(self) {
        (self.0)();
    }
}

#[cfg(target_arch = "wasm32")]
pub fn js_log(msg: &str) {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(msg));
}

#[cfg(target_arch = "wasm32")]
static IS_WINDOW_CLOSE_LISTENER_REGISTERED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Đăng ký listener duy nhất tại tầng gốc (App-level) nhận sự kiện `native:close_requested` từ Tauri.
/// Tự động dọn dẹp khi component unmount, hỗ trợ đầy đủ vòng đời bất đồng bộ và hủy listener.
#[cfg(target_arch = "wasm32")]
pub fn register_window_close_listener(
    manager: WindowCloseManager,
    app_state: crate::app::AppState,
) -> Option<WasmCleanupHandle> {
    use wasm_bindgen::JsCast;
    use std::sync::atomic::Ordering;

    // 1. Chống đăng ký trùng lặp (Duplicate registration prevention)
    if IS_WINDOW_CLOSE_LISTENER_REGISTERED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return None;
    }

    let window = match web_sys::window() {
        Some(w) => w,
        None => {
            IS_WINDOW_CLOSE_LISTENER_REGISTERED.store(false, Ordering::SeqCst);
            return None;
        }
    };
    let tauri = match js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__")) {
        Ok(t) => t,
        Err(_) => {
            IS_WINDOW_CLOSE_LISTENER_REGISTERED.store(false, Ordering::SeqCst);
            return None;
        }
    };
    if tauri.is_undefined() || tauri.is_null() {
        IS_WINDOW_CLOSE_LISTENER_REGISTERED.store(false, Ordering::SeqCst);
        return None;
    }

    let event_module = match js_sys::Reflect::get(&tauri, &JsValue::from_str("event")) {
        Ok(m) => m,
        Err(_) => {
            IS_WINDOW_CLOSE_LISTENER_REGISTERED.store(false, Ordering::SeqCst);
            return None;
        }
    };
    if event_module.is_undefined() || event_module.is_null() {
        IS_WINDOW_CLOSE_LISTENER_REGISTERED.store(false, Ordering::SeqCst);
        return None;
    }
    let listen_fn = match js_sys::Reflect::get(&event_module, &JsValue::from_str("listen")) {
        Ok(f) => f,
        Err(_) => {
            IS_WINDOW_CLOSE_LISTENER_REGISTERED.store(false, Ordering::SeqCst);
            return None;
        }
    };
    if !listen_fn.is_function() {
        IS_WINDOW_CLOSE_LISTENER_REGISTERED.store(false, Ordering::SeqCst);
        return None;
    }
    let listen_func: js_sys::Function = listen_fn.unchecked_into();

    let closure = Closure::<dyn FnMut(JsValue)>::new(move |event_val: JsValue| {
        let payload = js_sys::Reflect::get(&event_val, &JsValue::from_str("payload"))
            .unwrap_or(event_val.clone());
        let req_id = if let Ok(val) = js_sys::Reflect::get(&payload, &JsValue::from_str("request_id")) {
            val.as_string()
        } else {
            None
        }.or_else(|| {
            js_sys::Reflect::get(&payload, &JsValue::from_str("requestId")).ok().and_then(|v| v.as_string())
        }).unwrap_or_default();

        if !req_id.is_empty() {
            let has_dirty = app_state.has_unsaved_work_or_running_task();
            let summary = app_state.unsaved_work_summary();
            manager.handle_native_close_requested(req_id, has_dirty, &summary);
        }
    });

    let promise_res = listen_func.call2(
        &event_module,
        &JsValue::from_str("native:close_requested"),
        closure.as_ref().unchecked_ref(),
    );

    let promise_val = match promise_res {
        Ok(p) => p,
        Err(e) => {
            IS_WINDOW_CLOSE_LISTENER_REGISTERED.store(false, Ordering::SeqCst);
            web_sys::console::error_1(&format!("[WINDOW_CLOSE] Error calling listen: {e:?}").into());
            return None;
        }
    };

    let is_unmounted = std::rc::Rc::new(std::cell::Cell::new(false));
    let unlisten_fn_holder = std::rc::Rc::new(std::cell::RefCell::new(None::<js_sys::Function>));
    let closure_holder = std::rc::Rc::new(std::cell::RefCell::new(Some(closure)));

    if let Ok(promise) = promise_val.dyn_into::<js_sys::Promise>() {
        let is_unmounted_async = is_unmounted.clone();
        let unlisten_holder_async = unlisten_fn_holder.clone();

        wasm_bindgen_futures::spawn_local(async move {
            match wasm_bindgen_futures::JsFuture::from(promise).await {
                Ok(unlisten_val) => {
                    if let Ok(func) = unlisten_val.dyn_into::<js_sys::Function>() {
                        if is_unmounted_async.get() {
                            // Thành phần đã bị unmount trước khi Promise hoàn tất: gọi dọn dẹp ngay lập tức
                            let _ = func.call0(&JsValue::UNDEFINED);
                        } else {
                            *unlisten_holder_async.borrow_mut() = Some(func);
                        }
                    }
                }
                Err(err) => {
                    web_sys::console::error_1(
                        &format!("[WINDOW_CLOSE] event.listen Promise rejected: {err:?}").into(),
                    );
                }
            }
        });
    }

    let unmount_flag = is_unmounted;
    let unlisten_holder = unlisten_fn_holder;
    let closure_cell = closure_holder;

    // P2 #4683415016: Frontend-ready handshake / query pending close request.
    // Nếu native guard đã phát sinh close_requested trước khi listener kịp đăng ký,
    // tái phân phối (redeliver) yêu cầu đóng ngay lập tức cho manager xử lý fail-closed.
    let manager_startup = manager;
    let app_state_startup = app_state;
    wasm_bindgen_futures::spawn_local(async move {
        let args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap_or(JsValue::NULL);
        if let Ok(res) = invoke("query_pending_close_request", args).await {
            if let Ok(Some(req_id)) = serde_wasm_bindgen::from_value::<Option<String>>(res) {
                if !req_id.is_empty() {
                    let has_dirty = app_state_startup.has_unsaved_work_or_running_task();
                    let summary = app_state_startup.unsaved_work_summary();
                    manager_startup.handle_native_close_requested(req_id, has_dirty, &summary);
                }
            }
        }
    });

    Some(WasmCleanupHandle::new(move || {
        unmount_flag.set(true);
        IS_WINDOW_CLOSE_LISTENER_REGISTERED.store(false, Ordering::SeqCst);
        if let Some(f) = unlisten_holder.borrow_mut().take() {
            let _ = f.call0(&JsValue::UNDEFINED);
        }
        let _ = closure_cell.borrow_mut().take();
    }))
}
