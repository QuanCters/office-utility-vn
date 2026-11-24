use serde::{Deserialize, Serialize};
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

/// Thông điệp gửi từ PDFCraft Canvas (iframe) tới Host (Leptos App).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IncomingBridgeMessage {
    /// PDFCraft Canvas đã khởi tạo xong WebGL & WebAssembly và sẵn sàng nhận tài liệu.
    #[serde(rename = "pdfcraft:ready")]
    Ready {
        #[serde(default)]
        version: String,
    },
    /// PDFCraft Canvas thông báo trạng thái có thay đổi chưa lưu (dirty = true) hoặc đã lưu xong (dirty = false).
    #[serde(rename = "pdfcraft:dirty")]
    Dirty {
        dirty: bool,
    },
    /// PDFCraft Canvas gửi yêu cầu lưu tài liệu kèm mảng bytes PDF mới nhất.
    #[serde(rename = "pdfcraft:save")]
    Save {
        name: String,
        #[serde(rename = "saveAs", default)]
        save_as: bool,
        #[serde(default)]
        bytes: Vec<u8>,
    },
    /// PDFCraft Canvas thông báo lỗi khởi động (ví dụ: trình duyệt không hỗ trợ WebGL).
    #[serde(rename = "pdfcraft:failed")]
    Failed {
        error: String,
    },
}

/// Kết quả phản hồi minh thị cho hành động lưu tài liệu trong PDF Studio (Gate 1 - Stage 4B-2B).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveOutcome {
    /// Đã lưu thành công bền vững xuống đĩa với đường dẫn và revision xác định
    Saved {
        dest_path: String,
        committed_revision: u64,
        #[serde(default)]
        new_document_id: Option<String>,
    },
    /// Người dùng bấm Hủy (Cancel) trên hộp thoại hệ điều hành -> Giữ nguyên dirty và session
    Canceled,
    /// Thao tác lưu thất bại có thông báo lỗi chi tiết -> Giữ nguyên dirty
    Failed(String),
    /// Kích hoạt tải về qua trình duyệt Web thuần túy (chỉ áp dụng khi không chạy trong Tauri)
    BrowserDownloadInitiated,
}

/// DTO phân quyền lưu đĩa có kiểu dữ liệu chặt chẽ (P1 #4676016514: Strict Save Grant Validation)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeSaveGrantDto {
    pub grant_id: String,
    pub document_id: String,
    pub app_instance_id: String,
    pub session_generation: u64,
    pub authorized_dest_path: String,
    pub expected_revision: u64,
    #[serde(default)]
    pub request_id: Option<String>,
}

static MUT_GRANT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[doc(hidden)]
pub fn set_mut_grant(active: bool) {
    MUT_GRANT.store(active, std::sync::atomic::Ordering::SeqCst);
}

#[doc(hidden)]
pub fn is_mut_grant_active() -> bool {
    MUT_GRANT.load(std::sync::atomic::Ordering::SeqCst)
}

static MUT_SAVE_AS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[doc(hidden)]
pub fn set_mut_save_as(active: bool) {
    MUT_SAVE_AS.store(active, std::sync::atomic::Ordering::SeqCst);
}

#[doc(hidden)]
pub fn is_mut_save_as_active() -> bool {
    MUT_SAVE_AS.load(std::sync::atomic::Ordering::SeqCst)
}

static AUTHORIZED_PDF_CACHE: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// Lưu nội dung nhị phân vào bộ đệm tài liệu đã được ủy quyền trong phiên
pub fn cache_authorized_pdf_bytes(path: &str, bytes: Vec<u8>) {
    if let Ok(mut cache) = AUTHORIZED_PDF_CACHE.lock() {
        cache.insert(path.to_string(), bytes);
    }
}

/// Lấy nội dung nhị phân từ bộ đệm tài liệu đã được ủy quyền
pub fn get_authorized_pdf_bytes(path: &str) -> Option<Vec<u8>> {
    AUTHORIZED_PDF_CACHE.lock().ok().and_then(|c| c.get(path).cloned())
}

impl IncomingBridgeMessage {
    /// Phân tích thông điệp từ chuỗi JSON.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Xuất thông điệp ra chuỗi JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Thông điệp Host (Leptos App) gửi tới PDFCraft Canvas (iframe).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutgoingBridgeMessage {
    /// Yêu cầu PDFCraft mở một tệp PDF dạng binary bytes.
    #[serde(rename = "pdfcraft:open")]
    Open {
        name: String,
        #[serde(default)]
        bytes: Vec<u8>,
    },
}

impl OutgoingBridgeMessage {
    /// Tạo thông điệp mở tệp mới với tên và nội dung nhị phân.
    pub fn open(name: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self::Open {
            name: name.into(),
            bytes,
        }
    }

    /// Phân tích thông điệp từ chuỗi JSON.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Xuất thông điệp ra chuỗi JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

pub type OnReadyCb = std::sync::Arc<dyn Fn(String) + Send + Sync>;
pub type OnDirtyCb = std::sync::Arc<dyn Fn(bool) + Send + Sync>;
pub type OnSaveCb = std::sync::Arc<dyn Fn(String, bool, Vec<u8>) + Send + Sync>;
pub type OnFailedCb = std::sync::Arc<dyn Fn(String) + Send + Sync>;

/// Cầu nối giao tiếp hai chiều giữa Leptos Host và PDFCraft iframe.
#[derive(Clone, Default)]
pub struct PdfCraftBridge {
    pub iframe_id: String,
    pub state: Option<crate::micro_apps::pdf_studio::state::PdfStudioState>,
    pub on_ready_cb: Option<OnReadyCb>,
    pub on_dirty_cb: Option<OnDirtyCb>,
    pub on_save_cb: Option<OnSaveCb>,
    pub on_failed_cb: Option<OnFailedCb>,
}

impl PdfCraftBridge {
    /// Khởi tạo cầu nối với id của iframe mục tiêu (mặc định "pdfcraft-frame").
    pub fn new(iframe_id: impl Into<String>) -> Self {
        Self {
            iframe_id: iframe_id.into(),
            state: None,
            on_ready_cb: None,
            on_dirty_cb: None,
            on_save_cb: None,
            on_failed_cb: None,
        }
    }

    /// Gắn trạng thái phản ứng `PdfStudioState` vào cầu nối.
    pub fn with_state(mut self, state: crate::micro_apps::pdf_studio::state::PdfStudioState) -> Self {
        self.state = Some(state);
        self
    }

    /// Đăng ký callback khi PDFCraft sẵn sàng hoạt động.
    pub fn on_ready<F: Fn(String) + Send + Sync + 'static>(&mut self, cb: F) -> &mut Self {
        self.on_ready_cb = Some(std::sync::Arc::new(cb));
        self
    }

    /// Đăng ký callback khi PDFCraft thay đổi trạng thái dirty (có thay đổi chưa lưu).
    pub fn on_dirty<F: Fn(bool) + Send + Sync + 'static>(&mut self, cb: F) -> &mut Self {
        self.on_dirty_cb = Some(std::sync::Arc::new(cb));
        self
    }

    /// Đăng ký callback khi PDFCraft kích hoạt thao tác lưu tài liệu.
    pub fn on_save<F: Fn(String, bool, Vec<u8>) + Send + Sync + 'static>(&mut self, cb: F) -> &mut Self {
        self.on_save_cb = Some(std::sync::Arc::new(cb));
        self
    }

    /// Đăng ký callback khi PDFCraft gặp lỗi khởi động.
    pub fn on_failed<F: Fn(String) + Send + Sync + 'static>(&mut self, cb: F) -> &mut Self {
        self.on_failed_cb = Some(std::sync::Arc::new(cb));
        self
    }

    /// Điều phối thông điệp nhận được tới `state` và các callbacks đã đăng ký.
    pub fn dispatch_incoming(&self, msg: &IncomingBridgeMessage) {
        if let Some(state) = &self.state {
            state.handle_bridge_message(msg);
        }

        match msg {
            IncomingBridgeMessage::Ready { version } => {
                if let Some(cb) = &self.on_ready_cb {
                    cb(version.clone());
                }
            }
            IncomingBridgeMessage::Dirty { dirty } => {
                if let Some(cb) = &self.on_dirty_cb {
                    cb(*dirty);
                }
            }
            IncomingBridgeMessage::Save { name, save_as, bytes } => {
                if let Some(cb) = &self.on_save_cb {
                    cb(name.clone(), *save_as, bytes.clone());
                }
            }
            IncomingBridgeMessage::Failed { error } => {
                if let Some(cb) = &self.on_failed_cb {
                    cb(error.clone());
                }
            }
        }
    }

    /// Gửi lệnh mở tài liệu PDF sang iframe bằng bytes nhị phân.
    pub fn open_document(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
        send_open_file(&self.iframe_id, name, bytes)
    }

    /// Nạp và mở tài liệu từ đường dẫn đĩa sang canvas.
    pub async fn open_file_path(&self, file_path: &str) -> Result<(), String> {
        open_file_into_canvas(&self.iframe_id, file_path).await
    }
}

/// Gửi thông điệp `pdfcraft:open` sang iframe qua postMessage kèm transfer ArrayBuffer.
#[cfg(target_arch = "wasm32")]
pub fn send_open_file(iframe_id: &str, name: &str, bytes: &[u8]) -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| "Không tìm thấy window".to_string())?;
    let document = window
        .document()
        .ok_or_else(|| "Không tìm thấy document".to_string())?;
    let iframe_el = document
        .get_element_by_id(iframe_id)
        .ok_or_else(|| format!("Không tìm thấy iframe với id: {iframe_id}"))?;
    let iframe = iframe_el
        .dyn_into::<web_sys::HtmlIFrameElement>()
        .map_err(|_| "Phần tử không phải HtmlIFrameElement".to_string())?;
    let content_window = iframe
        .content_window()
        .ok_or_else(|| "Không thể lấy content_window của iframe".to_string())?;

    let array_buffer = js_sys::Uint8Array::from(bytes).buffer();
    let msg = js_sys::Object::new();
    js_sys::Reflect::set(
        &msg,
        &wasm_bindgen::JsValue::from_str("type"),
        &wasm_bindgen::JsValue::from_str("pdfcraft:open"),
    )
    .map_err(|e| format!("{e:?}"))?;
    js_sys::Reflect::set(
        &msg,
        &wasm_bindgen::JsValue::from_str("name"),
        &wasm_bindgen::JsValue::from_str(name),
    )
    .map_err(|e| format!("{e:?}"))?;
    js_sys::Reflect::set(
        &msg,
        &wasm_bindgen::JsValue::from_str("bytes"),
        &array_buffer,
    )
    .map_err(|e| format!("{e:?}"))?;

    let transfer = js_sys::Array::of1(&array_buffer);
    let origin = window.location().origin().unwrap_or_else(|_| "*".to_string());
    content_window
        .post_message_with_transfer(&msg, &origin, &transfer)
        .map_err(|e| format!("postMessage failed: {e:?}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn send_open_file(_iframe_id: &str, _name: &str, _bytes: &[u8]) -> Result<(), String> {
    Ok(())
}

/// Phân tích thông điệp nhận từ JS `MessageEvent.data`.
#[cfg(target_arch = "wasm32")]
pub fn parse_incoming_js_value(data: &wasm_bindgen::JsValue) -> Option<IncomingBridgeMessage> {
    let type_val = js_sys::Reflect::get(data, &wasm_bindgen::JsValue::from_str("type")).ok()?;
    let msg_type = type_val.as_string()?;
    match msg_type.as_str() {
        "pdfcraft:ready" => {
            let version = js_sys::Reflect::get(data, &wasm_bindgen::JsValue::from_str("version"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            Some(IncomingBridgeMessage::Ready { version })
        }
        "pdfcraft:dirty" => {
            let dirty = js_sys::Reflect::get(data, &wasm_bindgen::JsValue::from_str("dirty"))
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Some(IncomingBridgeMessage::Dirty { dirty })
        }
        "pdfcraft:save" => {
            let name = js_sys::Reflect::get(data, &wasm_bindgen::JsValue::from_str("name"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| "document.pdf".to_string());
            let save_as = js_sys::Reflect::get(data, &wasm_bindgen::JsValue::from_str("saveAs"))
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let bytes_val = js_sys::Reflect::get(data, &wasm_bindgen::JsValue::from_str("bytes")).ok()?;
            let bytes = if let Some(array) = bytes_val.dyn_ref::<js_sys::Uint8Array>() {
                array.to_vec()
            } else if bytes_val.is_instance_of::<js_sys::ArrayBuffer>() {
                js_sys::Uint8Array::new(&bytes_val).to_vec()
            } else if let Ok(arr) = serde_wasm_bindgen::from_value::<Vec<u8>>(bytes_val) {
                arr
            } else {
                Vec::new()
            };
            Some(IncomingBridgeMessage::Save { name, save_as, bytes })
        }
        "pdfcraft:failed" => {
            let error = js_sys::Reflect::get(data, &wasm_bindgen::JsValue::from_str("error"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| "Lỗi khởi động PDFCraft".to_string());
            Some(IncomingBridgeMessage::Failed { error })
        }
        _ => None,
    }
}

/// Đăng ký listener lắng nghe sự kiện `message` từ window cho PDFCraft Canvas iframe.
#[cfg(target_arch = "wasm32")]
pub fn setup_bridge_listener(
    iframe_id: String,
    state: crate::micro_apps::pdf_studio::state::PdfStudioState,
    bridge: Option<PdfCraftBridge>,
) -> Result<(), String> {
    use wasm_bindgen::closure::Closure;

    let window = web_sys::window().ok_or_else(|| "Không tìm thấy window".to_string())?;
    let expected_origin = window.location().origin().unwrap_or_default();

    let on_message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
        // 1. Kiểm tra Origin (chấp nhận same-origin, origin rỗng "null", hoặc các custom protocol của Tauri/WebView2)
        let ev_origin = event.origin();
        let is_valid_origin = expected_origin.is_empty()
            || ev_origin == expected_origin
            || ev_origin == "null"
            || ev_origin.is_empty()
            || ev_origin.contains("tauri.localhost")
            || ev_origin.starts_with("tauri://")
            || ev_origin.starts_with("http://127.0.0.1")
            || ev_origin.starts_with("http://localhost");

        if !is_valid_origin {
            return;
        }

        // 2. Kiểm tra Source Window khớp với iframe con nếu tìm thấy iframe
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(iframe_el) = doc.get_element_by_id(&iframe_id) {
                if let Ok(iframe) = iframe_el.dyn_into::<web_sys::HtmlIFrameElement>() {
                    if let Some(cw) = iframe.content_window() {
                        if let Some(source) = event.source() {
                            if !js_sys::Object::is(&source, &cw) {
                                return;
                            }
                        }
                    }
                }
            }
        }

        // 3. Phân tích và xử lý thông điệp
        let data = event.data();
        if let Some(msg) = parse_incoming_js_value(&data) {
            state.handle_bridge_message(&msg);
            if let Some(ref b) = bridge {
                b.dispatch_incoming(&msg);
            }

            // Xử lý lưu tệp tự động nếu nhận được Save message (Gate 1 - Stage 4B-2B)
            if let IncomingBridgeMessage::Save { ref name, save_as, ref bytes } = msg {
                let name_clone = name.clone();
                let bytes_clone = bytes.clone();
                let orig_path = state.active_file_path();
                let orig_doc_id = state.active_document_id();
                let cur_rev = state.active_revision();
                let snapshot_gen = state.edit_generation.get_untracked();
                static REQ_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                let count = REQ_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let req_id = format!("pdf-save-{}-{}", snapshot_gen, count);
                let fp = crate::platform::persistence::compute_fingerprint(&bytes_clone);

                let strip_orig = is_mut_save_as_active();

                let ctx = crate::micro_apps::pdf_studio::state::SaveRequestContext {
                    request_id: req_id.clone(),
                    operation_intent: if save_as {
                        crate::platform::adapter::SaveOperationIntent::SaveAs
                    } else {
                        crate::platform::adapter::SaveOperationIntent::Save
                    },
                    originating_document_id: if strip_orig { None } else { orig_doc_id.clone() },
                    originating_path: if strip_orig { None } else { orig_path.clone() },
                    session_generation: 1,
                    snapshot_edit_generation: snapshot_gen,
                    expected_committed_revision: cur_rev + 1,
                    snapshot_fingerprint: fp,
                    authorized_dest_path: if !save_as { orig_path.clone() } else { None },
                };

                state.register_pending_save(req_id.clone(), ctx.clone());
                let ctx_clone = ctx.clone();
                let s = state;
                wasm_bindgen_futures::spawn_local(async move {
                    let outcome = save_file_to_disk_or_download(
                        &name_clone,
                        if !save_as { orig_path.as_deref() } else { None },
                        orig_doc_id.as_deref(),
                        cur_rev + 1,
                        save_as,
                        &bytes_clone,
                    )
                    .await;
                    apply_save_outcome(&s, outcome, &ctx_clone, &bytes_clone);
                });
            }
        }
    });

    window
        .add_event_listener_with_callback("message", on_message.as_ref().unchecked_ref())
        .map_err(|e| format!("Không thể đăng ký message listener: {e:?}"))?;

    on_message.forget();
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn setup_bridge_listener(
    _iframe_id: String,
    _state: crate::micro_apps::pdf_studio::state::PdfStudioState,
    _bridge: Option<PdfCraftBridge>,
) -> Result<(), String> {
    Ok(())
}

/// Áp dụng kết quả của tác vụ lưu tệp PDF (P1 #4676016520 & #4676016514)
pub fn apply_save_outcome(
    s: &crate::micro_apps::pdf_studio::state::PdfStudioState,
    outcome: Result<SaveOutcome, String>,
    ctx: &crate::micro_apps::pdf_studio::state::SaveRequestContext,
    bytes: &[u8],
) {
    s.clear_pending_save(&ctx.request_id);

    match outcome {
        Ok(SaveOutcome::Saved { dest_path, committed_revision, new_document_id }) => {
            let target_doc_id = new_document_id.or_else(|| ctx.originating_document_id.clone());
            let is_save_as = ctx.operation_intent == crate::platform::adapter::SaveOperationIntent::SaveAs;

            let is_still_active_target = s.active_file_path().as_deref() == ctx.originating_path.as_deref()
                || s.active_file_path().as_deref() == Some(&dest_path);

            // 1. Luôn cập nhật phiên và bộ nhớ cache cho tài liệu đã được lưu TRƯỚC khi chuyển active file
            let mut map = s.document_sessions.get_untracked();
            if let Some(sess) = map.get_mut(&dest_path) {
                sess.document_id = target_doc_id.clone();
                sess.revision = committed_revision;
                if sess.edit_generation == ctx.snapshot_edit_generation {
                    sess.is_dirty = false;
                }
            } else {
                map.insert(
                    dest_path.clone(),
                    crate::micro_apps::pdf_studio::state::PdfDocumentSession {
                        file_path: dest_path.clone(),
                        document_id: target_doc_id.clone(),
                        revision: committed_revision,
                        is_dirty: false,
                        edit_generation: ctx.snapshot_edit_generation,
                    },
                );
            }
            s.document_sessions.set(map);

            cache_authorized_pdf_bytes(&dest_path, bytes.to_vec());
            s.cache_document_bytes(&dest_path, bytes.to_vec());

            // 2. Thêm hoặc chọn tệp mới lưu
            if is_save_as || ctx.originating_path.is_none() {
                if is_still_active_target {
                    s.add_file(&dest_path);
                } else {
                    // Thêm tệp vào danh sách nhưng giữ nguyên tệp đang active
                    let cur_idx = s.active_file_index.get_untracked();
                    s.loaded_files.update(|f| {
                        if !f.iter().any(|p| p == &dest_path) {
                            f.push(dest_path.clone());
                        }
                    });
                    s.active_file_index.set(cur_idx);
                }
            }

            // 3. Chỉ cập nhật tín hiệu active NẾU tài liệu đang chọn VẪN LÀ tài liệu nguồn hoặc tệp vừa lưu
            if is_still_active_target {
                s.active_document_id.set(target_doc_id);
                s.active_revision.set(committed_revision);
                let current_gen = s.edit_generation.get_untracked();
                if current_gen == ctx.snapshot_edit_generation {
                    s.has_unsaved_changes.set(false);
                } else {
                    s.has_unsaved_changes.set(true);
                }
            } else {
                // Người dùng đã chuyển sang tài liệu khác trong khi lưu!
                // Giữ nguyên active state của tài liệu đang chọn.
            }
            s.status_message.set(format!("✓ Đã lưu tệp: {dest_path}"));
        }
        Ok(SaveOutcome::Canceled) => {
            if let Some(ref path) = ctx.originating_path {
                let mut map = s.document_sessions.get_untracked();
                if let Some(sess) = map.get_mut(path) {
                    sess.is_dirty = true;
                }
                s.document_sessions.set(map);
            }
            let is_still_active = match ctx.originating_path.as_deref() {
                Some(p) => s.active_file_path().as_deref() == Some(p),
                None => false,
            };
            if is_still_active {
                s.has_unsaved_changes.set(true);
            }
            s.status_message.set("Đã hủy thao tác lưu tệp.".to_string());
        }
        Ok(SaveOutcome::Failed(err)) => {
            if let Some(ref path) = ctx.originating_path {
                let mut map = s.document_sessions.get_untracked();
                if let Some(sess) = map.get_mut(path) {
                    sess.is_dirty = true;
                }
                s.document_sessions.set(map);
            }
            let is_still_active = match ctx.originating_path.as_deref() {
                Some(p) => s.active_file_path().as_deref() == Some(p),
                None => false,
            };
            if is_still_active {
                s.has_unsaved_changes.set(true);
            }
            s.status_message.set(format!("❌ Lưu tệp thất bại: {err}"));
        }
        Ok(SaveOutcome::BrowserDownloadInitiated) => {
            let is_still_active = match ctx.originating_path.as_deref() {
                Some(p) => s.active_file_path().as_deref() == Some(p),
                None => false,
            };
            if is_still_active {
                let current_gen = s.edit_generation.get_untracked();
                if current_gen == ctx.snapshot_edit_generation {
                    s.has_unsaved_changes.set(false);
                } else {
                    s.has_unsaved_changes.set(true);
                }
            }
            s.status_message.set("Đã bắt đầu tải về qua trình duyệt.".to_string());
        }
        Err(err) => {
            if let Some(ref path) = ctx.originating_path {
                let mut map = s.document_sessions.get_untracked();
                if let Some(sess) = map.get_mut(path) {
                    sess.is_dirty = true;
                }
                s.document_sessions.set(map);
            }
            let is_still_active = match ctx.originating_path.as_deref() {
                Some(p) => s.active_file_path().as_deref() == Some(p),
                None => false,
            };
            if is_still_active {
                s.has_unsaved_changes.set(true);
            }
            s.status_message.set(format!("❌ Lỗi hệ thống: {err}"));
        }
    }
}

/// Nạp tệp PDF thông qua Native OS File Dialog có kiểm soát an ninh (Stage 4C-0D UAT Support).
#[cfg(target_arch = "wasm32")]
pub async fn open_file_into_pdf_studio(
    iframe_id: &str,
    state: crate::micro_apps::pdf_studio::state::PdfStudioState,
) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "appInstanceId": "pdf-studio-instance",
        "sessionGeneration": 1,
        "filterName": "Tài liệu PDF (*.pdf)",
        "extensions": ["pdf"],
    }))
    .map_err(|e| format!("{e:?}"))?;

    if let Ok(js_res) = invoke("native_dialog_open_file", args).await {
        if let Ok(Some(handle)) = serde_wasm_bindgen::from_value::<Option<serde_json::Value>>(js_res) {
            let doc_id = handle.get("document_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let file_path = handle.get("file_path").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let file_name = handle.get("file_name").and_then(|v| v.as_str()).unwrap_or("document.pdf").to_string();
            let rev = handle.get("initial_revision").and_then(|v| v.as_u64()).unwrap_or(0);

            let bytes_val = handle.get("bytes").cloned().unwrap_or(serde_json::Value::Null);
            let bytes: Vec<u8> = serde_json::from_value(bytes_val).unwrap_or_default();

            if !file_path.is_empty() {
                cache_authorized_pdf_bytes(&file_path, bytes.clone());
                state.cache_document_bytes(&file_path, bytes.clone());
                state.add_file(&file_path);
                state.set_active_document_session(doc_id, rev);
                if !bytes.is_empty() {
                    let _ = send_open_file(iframe_id, &file_name, &bytes);
                }
                return Ok(());
            }
        }
    }

    // Fallback nếu chạy trong môi trường web không có Tauri IPC
    let _ = js_sys::eval("document.getElementById('pdf-studio-native-picker')?.click()");
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn open_file_into_pdf_studio(
    _iframe_id: &str,
    _state: crate::micro_apps::pdf_studio::state::PdfStudioState,
) -> Result<(), String> {
    Ok(())
}

/// Đọc nội dung nhị phân của tệp nội bộ (ưu tiên cache tài liệu đã ủy quyền, web fetch đối với /assets/... hoặc blob: URL).
/// [BẢO MẬT STAGE 4C-0D CLOSURE]: Loại bỏ hoàn toàn lệnh gọi IPC 'read_file_binary' raw-path.
#[cfg(target_arch = "wasm32")]
pub async fn read_file_bytes(file_path: &str) -> Result<Vec<u8>, String> {
    // 0. Kiểm tra bộ đệm tài liệu đã được ủy quyền trong phiên
    if let Some(cached) = get_authorized_pdf_bytes(file_path) {
        return Ok(cached);
    }

    // 1. Tuân thủ nghiêm ngặt NFR-SEC-01 (Zero-Cloud): Nghiêm cấm tải file từ URL http:// hoặc https://
    if file_path.starts_with("http://") || file_path.starts_with("https://") {
        return Err("NFR-SEC-01: Không cho phép tải file qua mạng bên ngoài (Chính sách Zero-Cloud Local-Only)".to_string());
    }

    // 2. Fetch qua web nếu file_path là local web path nội bộ (ví dụ: /assets/... hoặc blob:)
    if file_path.starts_with('/') || file_path.starts_with("blob:") {
        if let Some(win) = web_sys::window() {
            if let Ok(resp_val) = wasm_bindgen_futures::JsFuture::from(win.fetch_with_str(file_path)).await {
                if let Ok(resp) = resp_val.dyn_into::<web_sys::Response>() {
                    if let Ok(ab_val) = resp.array_buffer() {
                        if let Ok(buffer_val) = wasm_bindgen_futures::JsFuture::from(ab_val).await {
                            let u8_arr = js_sys::Uint8Array::new(&buffer_val);
                            let bytes = u8_arr.to_vec();
                            cache_authorized_pdf_bytes(file_path, bytes.clone());
                            return Ok(bytes);
                        }
                    }
                }
            }
        }
    }

    Err(format!("Không thể đọc nội dung file từ đường dẫn: {file_path}"))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn read_file_bytes(file_path: &str) -> Result<Vec<u8>, String> {
    if let Some(cached) = get_authorized_pdf_bytes(file_path) {
        return Ok(cached);
    }
    if file_path.starts_with("http://") || file_path.starts_with("https://") {
        return Err("NFR-SEC-01: Không cho phép tải file qua mạng bên ngoài (Chính sách Zero-Cloud Local-Only)".to_string());
    }
    std::fs::read(file_path).map_err(|e| format!("Không thể đọc tệp: {e}"))
}

/// Kiểm tra môi trường thực thi có phải là Tauri Desktop hay không.
#[cfg(target_arch = "wasm32")]
pub fn is_tauri_runtime() -> bool {
    if let Some(win) = web_sys::window() {
        if let Ok(tauri_val) = js_sys::Reflect::get(&win, &wasm_bindgen::JsValue::from_str("__TAURI__")) {
            return !tauri_val.is_undefined() && !tauri_val.is_null();
        }
    }
    false
}

/// Lưu nội dung nhị phân xuống đĩa (qua Tauri IPC có thẩm quyền Native Dialog) hoặc kích hoạt tải về qua trình duyệt.
#[cfg(target_arch = "wasm32")]
pub async fn save_file_to_disk_or_download(
    name: &str,
    file_path: Option<&str>,
    document_id: Option<&str>,
    expected_rev: u64,
    save_as: bool,
    bytes: &[u8],
) -> Result<SaveOutcome, String> {
    if is_tauri_runtime() {
        // Desktop Mode: TUYỆT ĐỐI KHÔNG FALLBACK VỀ TRÌNH DUYỆT

        // 1. Nếu là Regular Save và đã có phiên tài liệu được ủy quyền
        if !save_as && let (Some(_path), Some(doc_id)) = (file_path, document_id) {
            let req_id = format!("pdf-save-{}", js_sys::Date::now() as u64);
            let grant_args = serde_wasm_bindgen::to_value(&serde_json::json!({
                "documentId": doc_id,
                "appInstanceId": "pdf-studio-instance",
                "sessionGeneration": 1,
                "requestId": req_id,
                "expectedRevision": expected_rev,
            }))
            .map_err(|e| format!("{e:?}"))?;

            let grant_val = match invoke("request_save_grant", grant_args).await {
                Ok(val) => val,
                Err(e) => {
                    return Ok(SaveOutcome::Failed(format!("Yêu cầu cấp phép ghi đĩa thất bại: {e:?}")));
                }
            };

            let grant: NativeSaveGrantDto = match serde_wasm_bindgen::from_value(grant_val) {
                Ok(g) => g,
                Err(e) => {
                    return Ok(SaveOutcome::Failed(format!(
                        "Lỗi bất biến: Grant bị thiếu trường bắt buộc hoặc sai định dạng: {e:?}"
                    )));
                }
            };

            let skip_dest = is_mut_grant_active();

            if !skip_dest && grant.authorized_dest_path != _path {
                return Ok(SaveOutcome::Failed(format!(
                    "Lỗi bất biến phiên tài liệu: Grant cấp cho {} nhưng tệp đang chọn là {}",
                    grant.authorized_dest_path, _path
                )));
            }

            if grant.grant_id.trim().is_empty() {
                return Ok(SaveOutcome::Failed("Lỗi bất biến: Grant ID rỗng".into()));
            }
            if grant.document_id != doc_id {
                return Ok(SaveOutcome::Failed(format!(
                    "Lỗi bất biến: Grant document_id ({}) không khớp doc_id ({doc_id})",
                    grant.document_id
                )));
            }
            if grant.expected_revision != expected_rev {
                return Ok(SaveOutcome::Failed(format!(
                    "Lỗi bất biến: Grant expected_revision ({}) không khớp expected_rev ({expected_rev})",
                    grant.expected_revision
                )));
            }
            if grant.app_instance_id != "pdf-studio-instance" {
                return Ok(SaveOutcome::Failed(format!(
                    "Lỗi bất biến: Grant app_instance_id ({}) không khớp",
                    grant.app_instance_id
                )));
            }
            if let Some(ref req) = grant.request_id {
                if req != &req_id {
                    return Ok(SaveOutcome::Failed(format!(
                        "Lỗi bất biến: Request ID ({req}) không khớp ({req_id})"
                    )));
                }
            }

            let save_args = serde_wasm_bindgen::to_value(&serde_json::json!({
                "grantId": grant.grant_id,
                "requestId": grant.request_id.unwrap_or(req_id),
                "bytes": bytes,
            }))
            .map_err(|e| format!("{e:?}"))?;

            match invoke("secure_save_document", save_args).await {
                Ok(save_res) => {
                    let (dest, rev) = if let Ok(resp) = serde_wasm_bindgen::from_value::<serde_json::Value>(save_res) {
                        (
                            resp.get("dest_path").and_then(|v| v.as_str()).unwrap_or(_path).to_string(),
                            resp.get("committed_revision").and_then(|v| v.as_u64()).unwrap_or(expected_rev),
                        )
                    } else {
                        (_path.to_string(), expected_rev)
                    };
                    return Ok(SaveOutcome::Saved {
                        dest_path: dest,
                        committed_revision: rev,
                        new_document_id: Some(doc_id.to_string()),
                    });
                }
                Err(e) => {
                    return Ok(SaveOutcome::Failed(format!("Ghi đĩa thất bại: {e:?}")));
                }
            }
        }

        // 2. Luồng Save As (hoặc Lưu lần đầu chưa có phiên ủy quyền)
        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "documentId": document_id,
            "appInstanceId": "pdf-studio-instance",
            "sessionGeneration": 1,
            "defaultName": name,
            "filterName": "Tài liệu PDF (*.pdf)",
            "extensions": ["pdf"],
        }))
        .unwrap_or(wasm_bindgen::JsValue::UNDEFINED);

        match invoke("native_dialog_save_as", args).await {
            Ok(js_res) => {
                if let Ok(Some(handle)) = serde_wasm_bindgen::from_value::<Option<serde_json::Value>>(js_res) {
                    let new_doc_id = handle.get("document_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    if let Some(grant) = handle.get("grant") {
                        if let (Some(grant_id), Some(req_id)) = (
                            grant.get("grant_id").and_then(|v| v.as_str()),
                            grant.get("request_id").and_then(|v| v.as_str()),
                        ) {
                            let save_args = serde_wasm_bindgen::to_value(&serde_json::json!({
                                "grantId": grant_id,
                                "requestId": req_id,
                                "bytes": bytes,
                            }))
                            .map_err(|e| format!("{e:?}"))?;

                            match invoke("secure_save_document", save_args).await {
                                Ok(save_res) => {
                                    let rev = if let Ok(resp) = serde_wasm_bindgen::from_value::<serde_json::Value>(save_res) {
                                        resp.get("committed_revision").and_then(|v| v.as_u64()).unwrap_or(1)
                                    } else {
                                        1
                                    };
                                    let dest_path = handle.get("file_path").and_then(|v| v.as_str()).unwrap_or(name).to_string();
                                    return Ok(SaveOutcome::Saved {
                                        dest_path,
                                        committed_revision: rev,
                                        new_document_id: new_doc_id,
                                    });
                                }
                                Err(e) => {
                                    return Ok(SaveOutcome::Failed(format!("Ghi đĩa thất bại: {e:?}")));
                                }
                            }
                        }
                    }
                    Ok(SaveOutcome::Failed("Cấu trúc phản hồi từ native dialog không hợp lệ".to_string()))
                } else {
                    // Người dùng chủ động bấm Hủy (Cancel) trên hộp thoại hệ điều hành -> KHÔNG LƯU, GIỮ NGUYÊN DIRTY
                    Ok(SaveOutcome::Canceled)
                }
            }
            Err(e) => Ok(SaveOutcome::Failed(format!("Lỗi khi mở hộp thoại lưu tệp: {e:?}"))),
        }
    } else {
        // Fallback: Chỉ kích hoạt tải về trình duyệt khi chạy trên Web thuần túy
        trigger_browser_download(name, bytes)?;
        Ok(SaveOutcome::BrowserDownloadInitiated)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn save_file_to_disk_or_download(
    _name: &str,
    file_path: Option<&str>,
    document_id: Option<&str>,
    _expected_rev: u64,
    _save_as: bool,
    bytes: &[u8],
) -> Result<SaveOutcome, String> {
    if let Some(path) = file_path {
        std::fs::write(path, bytes).map_err(|e| format!("Không thể ghi tệp: {e}"))?;
        cache_authorized_pdf_bytes(path, bytes.to_vec());
        Ok(SaveOutcome::Saved {
            dest_path: path.to_string(),
            committed_revision: 1,
            new_document_id: document_id.map(|s| s.to_string()),
        })
    } else {
        Ok(SaveOutcome::Canceled)
    }
}

/// Kích hoạt tải tệp về thông qua Blob URL trong trình duyệt.
#[cfg(target_arch = "wasm32")]
pub fn trigger_browser_download(name: &str, bytes: &[u8]) -> Result<(), String> {
    let uint8_array = js_sys::Uint8Array::from(bytes);
    let window = web_sys::window().ok_or_else(|| "Không tìm thấy window".to_string())?;
    let global = js_sys::Function::new_with_args(
        "name, u8",
        r#"
        const blob = new Blob([u8], { type: 'application/pdf' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = name;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        "#,
    );
    let _ = global.call2(&window, &wasm_bindgen::JsValue::from_str(name), &uint8_array);
    Ok(())
}

/// Đọc tệp từ đường dẫn đĩa và nạp trực tiếp vào PDFCraft iframe canvas.
pub async fn open_file_into_canvas(iframe_id: &str, file_path: &str) -> Result<(), String> {
    let bytes = read_file_bytes(file_path).await?;
    let name = file_path.rsplit(['/', '\\']).next().unwrap_or(file_path);
    send_open_file(iframe_id, name, &bytes)?;
    Ok(())
}
