use serde::{Deserialize, Serialize};

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

            // Xử lý lưu tệp tự động nếu nhận được Save message
            if let IncomingBridgeMessage::Save { ref name, save_as, ref bytes } = msg {
                let name_clone = name.clone();
                let bytes_clone = bytes.clone();
                let active_path = if !save_as { state.active_file_path() } else { None };
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = save_file_to_disk_or_download(&name_clone, active_path.as_deref(), &bytes_clone).await;
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

/// Đọc toàn bộ nội dung nhị phân của tệp (ưu tiên Tauri IPC, fallback web fetch).
#[cfg(target_arch = "wasm32")]
pub async fn read_file_bytes(file_path: &str) -> Result<Vec<u8>, String> {
    // 0. Tuân thủ nghiêm ngặt NFR-SEC-01 (Zero-Cloud): Nghiêm cấm tải file từ URL http:// hoặc https://
    if file_path.starts_with("http://") || file_path.starts_with("https://") {
        return Err("NFR-SEC-01: Không cho phép tải file qua mạng bên ngoài (Chính sách Zero-Cloud Local-Only)".to_string());
    }

    // 1. Thử gọi qua Tauri IPC `read_file_binary`
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "filePath": file_path
    }))
    .map_err(|e| format!("{e:?}"))?;

    if let Ok(js_val) = invoke("read_file_binary", args).await {
        if let Ok(bytes) = serde_wasm_bindgen::from_value::<Vec<u8>>(js_val) {
            if !bytes.is_empty() {
                return Ok(bytes);
            }
        }
    }

    // 2. Thử fetch qua web nếu file_path là local path nội bộ (ví dụ: /assets/... hoặc blob:)
    if file_path.starts_with('/') || file_path.starts_with("blob:") {
        if let Some(win) = web_sys::window() {
            if let Ok(resp_val) = wasm_bindgen_futures::JsFuture::from(win.fetch_with_str(file_path)).await {
                if let Ok(resp) = resp_val.dyn_into::<web_sys::Response>() {
                    if let Ok(ab_val) = resp.array_buffer() {
                        if let Ok(buffer_val) = wasm_bindgen_futures::JsFuture::from(ab_val).await {
                            let u8_arr = js_sys::Uint8Array::new(&buffer_val);
                            return Ok(u8_arr.to_vec());
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
    if file_path.starts_with("http://") || file_path.starts_with("https://") {
        return Err("NFR-SEC-01: Không cho phép tải file qua mạng bên ngoài (Chính sách Zero-Cloud Local-Only)".to_string());
    }
    std::fs::read(file_path).map_err(|e| format!("Không thể đọc tệp: {e}"))
}

/// Lưu nội dung nhị phân xuống đĩa (qua Tauri IPC) hoặc kích hoạt tải về qua trình duyệt.
#[cfg(target_arch = "wasm32")]
pub async fn save_file_to_disk_or_download(name: &str, file_path: Option<&str>, bytes: &[u8]) -> Result<(), String> {
    // 1. Nếu có file_path cụ thể, thử lưu qua Tauri command
    if let Some(path) = file_path {
        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "filePath": path,
            "bytes": bytes,
        }))
        .map_err(|e| format!("{e:?}"))?;

        if invoke("save_file_binary", args).await.is_ok() {
            return Ok(());
        }
    }

    // 2. Fallback: Kích hoạt tải về qua trình duyệt (Blob download)
    trigger_browser_download(name, bytes)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn save_file_to_disk_or_download(_name: &str, file_path: Option<&str>, bytes: &[u8]) -> Result<(), String> {
    if let Some(path) = file_path {
        std::fs::write(path, bytes).map_err(|e| format!("Không thể ghi tệp: {e}"))?;
    }
    Ok(())
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
