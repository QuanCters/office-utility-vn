use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Thông điệp dạng thô trao đổi qua tầng transport postMessage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawBridgeMessage {
    pub type_name: String,
    pub payload: serde_json::Value,
    #[serde(skip)]
    pub binary_data: Option<Vec<u8>>,
}

/// Các lỗi phát sinh tại tầng giao vận PostMessage Transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// Nguồn gốc (Origin) của thông điệp không khớp với domain của Host.
    InvalidOrigin {
        expected: String,
        actual: String,
    },
    /// Cửa sổ nguồn (event.source) không phải là contentWindow của iframe mục tiêu.
    InvalidSource,
    /// Cấu trúc dữ liệu JSON không hợp lệ hoặc thiếu trường "type".
    MalformedStructure(String),
    /// Không tìm thấy phần tử iframe trên DOM để kết nối.
    MissingIframe(String),
    /// Gửi thông điệp postMessage thất bại.
    PostMessageFailed(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOrigin { expected, actual } => {
                write!(f, "Invalid origin: expected '{expected}', got '{actual}'")
            }
            Self::InvalidSource => write!(f, "Message event source does not match target iframe content window"),
            Self::MalformedStructure(msg) => write!(f, "Malformed bridge message: {msg}"),
            Self::MissingIframe(id) => write!(f, "Target iframe with id '{id}' was not found"),
            Self::PostMessageFailed(err) => write!(f, "Failed to post message to iframe: {err}"),
        }
    }
}

impl std::error::Error for TransportError {}

/// Hàm kiểm tra tính hợp lệ của nguồn gốc thông điệp (Origin Validation Guard).
pub fn validate_origin(expected: &str, actual: &str) -> Result<(), TransportError> {
    if expected.is_empty() || actual.is_empty() || expected != actual {
        Err(TransportError::InvalidOrigin {
            expected: expected.to_string(),
            actual: actual.to_string(),
        })
    } else {
        Ok(())
    }
}

/// Hàm kiểm tra cấu trúc thông điệp cơ bản (Structure Validation Guard).
pub fn validate_structure(json_value: &serde_json::Value) -> Result<RawBridgeMessage, TransportError> {
    let obj = json_value
        .as_object()
        .ok_or_else(|| TransportError::MalformedStructure("Root value must be a JSON object".to_string()))?;

    let type_val = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| TransportError::MalformedStructure("Missing or non-string 'type' property".to_string()))?;

    if type_val.trim().is_empty() {
        return Err(TransportError::MalformedStructure("'type' property cannot be empty".to_string()));
    }

    Ok(RawBridgeMessage {
        type_name: type_val.to_string(),
        payload: json_value.clone(),
        binary_data: None,
    })
}

/// Tầng giao vận mỏng dùng chung cho các ứng dụng Micro-App nhúng qua iframe.
pub struct CraftHostTransport {
    expected_origin: String,
    iframe_element_id: String,
    #[cfg(target_arch = "wasm32")]
    active_listener: Option<Closure<dyn FnMut(web_sys::MessageEvent)>>,
}

impl CraftHostTransport {
    /// Khởi tạo một phiên transport mới với origin kỳ vọng và iframe element ID.
    pub fn new(expected_origin: impl Into<String>, iframe_element_id: impl Into<String>) -> Self {
        Self {
            expected_origin: expected_origin.into(),
            iframe_element_id: iframe_element_id.into(),
            #[cfg(target_arch = "wasm32")]
            active_listener: None,
        }
    }

    /// Lấy origin kỳ vọng được bảo vệ.
    pub fn expected_origin(&self) -> &str {
        &self.expected_origin
    }

    /// Lấy ID của phần tử iframe đích.
    pub fn iframe_id(&self) -> &str {
        &self.iframe_element_id
    }

    /// Kiểm tra xem listener hiện tại có đang được gắn hay không.
    pub fn is_attached(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            self.active_listener.is_some()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            false
        }
    }

    /// Hủy đăng ký listener cũ để tránh trùng lặp sự kiện khi iframe reload/reinitialize.
    pub fn detach(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(listener) = self.active_listener.take() {
                if let Some(window) = web_sys::window() {
                    let _ = window.remove_event_listener_with_callback(
                        "message",
                        listener.as_ref().unchecked_ref(),
                    );
                }
            }
        }
    }

    /// Gửi thông điệp tới iframe với targetOrigin tường minh và hỗ trợ chuyển nhượng ArrayBuffer.
    pub fn post_to_iframe(
        &self,
        type_name: &str,
        payload: serde_json::Value,
        binary_data: Option<&[u8]>,
    ) -> Result<(), TransportError> {
        if self.expected_origin == "*" || self.expected_origin.trim().is_empty() {
            return Err(TransportError::PostMessageFailed(
                "Wildcard or empty targetOrigin is strictly forbidden".to_string(),
            ));
        }

        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().ok_or_else(|| {
                TransportError::PostMessageFailed("No browser window available".to_string())
            })?;
            let document = window.document().ok_or_else(|| {
                TransportError::PostMessageFailed("No DOM document available".to_string())
            })?;
            let element = document
                .get_element_by_id(&self.iframe_element_id)
                .ok_or_else(|| TransportError::MissingIframe(self.iframe_element_id.clone()))?;

            let iframe = element
                .dyn_into::<web_sys::HtmlIFrameElement>()
                .map_err(|_| TransportError::MissingIframe(self.iframe_element_id.clone()))?;

            let target_win = iframe.content_window().ok_or_else(|| {
                TransportError::PostMessageFailed("Iframe content window is null".to_string())
            })?;

            let mut final_obj = match payload {
                serde_json::Value::Object(map) => map,
                _ => serde_json::Map::new(),
            };
            final_obj.insert("type".to_string(), serde_json::Value::String(type_name.to_string()));

            if let Some(bytes) = binary_data {
                let uint8 = js_sys::Uint8Array::from(bytes);
                let buffer = uint8.buffer();
                let transfer = js_sys::Array::of1(&buffer);

                let js_msg = serde_wasm_bindgen::to_value(&final_obj).map_err(|e| {
                    TransportError::PostMessageFailed(format!("Serialization error: {e:?}"))
                })?;
                js_sys::Reflect::set(&js_msg, &JsValue::from_str("bytes"), &buffer).ok();

                target_win
                    .post_message_with_transfer(&js_msg, &self.expected_origin, &transfer)
                    .map_err(|e| TransportError::PostMessageFailed(format!("postMessage error: {e:?}")))?;
            } else {
                let js_msg = serde_wasm_bindgen::to_value(&final_obj).map_err(|e| {
                    TransportError::PostMessageFailed(format!("Serialization error: {e:?}"))
                })?;
                target_win
                    .post_message(&js_msg, &self.expected_origin)
                    .map_err(|e| TransportError::PostMessageFailed(format!("postMessage error: {e:?}")))?;
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (type_name, payload, binary_data);
        }
        Ok(())
    }
}

impl Drop for CraftHostTransport {
    fn drop(&mut self) {
        self.detach();
    }
}
