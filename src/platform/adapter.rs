use crate::micro_apps::MicroAppId;
use crate::platform::transport::{RawBridgeMessage, TransportError};

/// Trạng thái sự kiện tài liệu chuẩn hóa từ góc nhìn của Micro-App Adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentStateEvent {
    /// Ứng dụng nhúng sẵn sàng nhận tệp tài liệu.
    Ready { version: String },
    /// Ứng dụng nhúng thông báo trạng thái có thay đổi chưa lưu.
    Dirty { dirty: bool },
    /// Ứng dụng nhúng yêu cầu lưu tệp kèm dữ liệu nhị phân.
    SaveRequested {
        name: String,
        save_as: bool,
        bytes: Vec<u8>,
        request_id: Option<String>,
    },
    /// Ứng dụng nhúng gặp sự cố khởi tạo hoặc thực thi.
    Failed { error: String },
}

/// Boundary Contract cho Adapter của từng Micro-App nhúng.
pub trait MicroAppAdapter: Send + Sync {
    /// Định danh Micro-App tương ứng.
    fn app_id(&self) -> MicroAppId;

    /// Tiền tố thông điệp (message prefix, ví dụ: "pdfcraft" hoặc "gridcraft").
    fn prefix(&self) -> &'static str;

    /// Phân tích thông điệp thô sang sự kiện vòng đời chuẩn tắc.
    fn parse_incoming(&self, raw: &RawBridgeMessage) -> Result<DocumentStateEvent, TransportError>;

    /// Định dạng thông điệp mở tệp chuẩn cho ứng dụng nhúng.
    fn format_open(&self, name: &str, bytes: &[u8]) -> Result<RawBridgeMessage, TransportError>;
}

/// Adapter đặc thù cho PDFCraft (tuân thủ giao thức pdfcraft:*).
/// Được định nghĩa làm ranh giới chuẩn bị cho việc di chuyển trong Stage 3.
pub struct PdfCraftAdapter;

impl MicroAppAdapter for PdfCraftAdapter {
    fn app_id(&self) -> MicroAppId {
        MicroAppId::PdfStudio
    }

    fn prefix(&self) -> &'static str {
        "pdfcraft"
    }

    fn parse_incoming(&self, raw: &RawBridgeMessage) -> Result<DocumentStateEvent, TransportError> {
        match raw.type_name.as_str() {
            "pdfcraft:ready" => {
                let version = raw
                    .payload
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(DocumentStateEvent::Ready { version })
            }
            "pdfcraft:dirty" => {
                let dirty = raw
                    .payload
                    .get("dirty")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Ok(DocumentStateEvent::Dirty { dirty })
            }
            "pdfcraft:save" => {
                let name = raw
                    .payload
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("document.pdf")
                    .to_string();
                let save_as = raw
                    .payload
                    .get("saveAs")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let request_id = raw
                    .payload
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let bytes = raw.binary_data.clone().unwrap_or_default();
                Ok(DocumentStateEvent::SaveRequested {
                    name,
                    save_as,
                    bytes,
                    request_id,
                })
            }
            "pdfcraft:failed" => {
                let error = raw
                    .payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown PDFCraft failure")
                    .to_string();
                Ok(DocumentStateEvent::Failed { error })
            }
            other => Err(TransportError::MalformedStructure(format!(
                "Unrecognized PDFCraft message type: {other}"
            ))),
        }
    }

    fn format_open(&self, name: &str, bytes: &[u8]) -> Result<RawBridgeMessage, TransportError> {
        Ok(RawBridgeMessage {
            type_name: "pdfcraft:open".to_string(),
            payload: serde_json::json!({
                "name": name,
            }),
            binary_data: Some(bytes.to_vec()),
        })
    }
}

/// Adapter đặc thù cho GridCraft (tuân thủ giao thức gridcraft:*).
/// Được định nghĩa làm ranh giới chuẩn bị cho việc tích hợp Data Lab trong Stage 4.
pub struct GridCraftAdapter;

impl MicroAppAdapter for GridCraftAdapter {
    fn app_id(&self) -> MicroAppId {
        MicroAppId::DataLab
    }

    fn prefix(&self) -> &'static str {
        "gridcraft"
    }

    fn parse_incoming(&self, raw: &RawBridgeMessage) -> Result<DocumentStateEvent, TransportError> {
        match raw.type_name.as_str() {
            "gridcraft:ready" => {
                let version = raw
                    .payload
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(DocumentStateEvent::Ready { version })
            }
            "gridcraft:dirty" => {
                let dirty = raw
                    .payload
                    .get("dirty")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Ok(DocumentStateEvent::Dirty { dirty })
            }
            "gridcraft:save" => {
                let name = raw
                    .payload
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("workbook.xlsx")
                    .to_string();
                let save_as = raw
                    .payload
                    .get("saveAs")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let request_id = raw
                    .payload
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let bytes = raw.binary_data.clone().unwrap_or_default();
                Ok(DocumentStateEvent::SaveRequested {
                    name,
                    save_as,
                    bytes,
                    request_id,
                })
            }
            "gridcraft:failed" => {
                let error = raw
                    .payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown GridCraft failure")
                    .to_string();
                Ok(DocumentStateEvent::Failed { error })
            }
            other => Err(TransportError::MalformedStructure(format!(
                "Unrecognized GridCraft message type: {other}"
            ))),
        }
    }

    fn format_open(&self, name: &str, bytes: &[u8]) -> Result<RawBridgeMessage, TransportError> {
        Ok(RawBridgeMessage {
            type_name: "gridcraft:open".to_string(),
            payload: serde_json::json!({
                "name": name,
            }),
            binary_data: Some(bytes.to_vec()),
        })
    }
}

impl GridCraftAdapter {
    /// Định dạng thông điệp xác nhận lưu thành công gửi ngược vào GridCraft iframe.
    pub fn format_saved(id: &str) -> RawBridgeMessage {
        RawBridgeMessage {
            type_name: "gridcraft:saved".to_string(),
            payload: serde_json::json!({
                "id": id,
            }),
            binary_data: None,
        }
    }

    /// Định dạng thông điệp báo lỗi lưu gửi ngược vào GridCraft iframe.
    pub fn format_failed(error: &str, id: Option<&str>) -> RawBridgeMessage {
        let mut payload = serde_json::json!({
            "error": error,
        });
        if let Some(id) = id {
            payload["id"] = serde_json::Value::String(id.to_string());
        }
        RawBridgeMessage {
            type_name: "gridcraft:failed".to_string(),
            payload,
            binary_data: None,
        }
    }
}
