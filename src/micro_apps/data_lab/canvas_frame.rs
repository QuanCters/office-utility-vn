#![allow(unused_imports, unused_variables, dead_code)]

use leptos::prelude::*;
use crate::micro_apps::data_lab::state::DataLabState;
use crate::micro_apps::MicroAppId;
use crate::platform::adapter::{DocumentStateEvent, GridCraftAdapter, MicroAppAdapter};
use crate::platform::coordinator::HostDocumentCoordinator;
use crate::platform::descriptor::MicroAppRegistry;
use crate::platform::transport::validate_structure;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Component `GridCraftCanvasFrame` nhúng bảng tính trực quan GridCraft WebAssembly
/// vào Micro-App Data Lab dưới dạng Chế độ Xem trước Thử nghiệm (Development-Only Preview).
///
/// Tuân thủ nghiêm ngặt các rào chắn kiến trúc Stage 3A:
/// 1. Tái sử dụng trọn vẹn tầng platform: MicroAppRegistry, GridCraftAdapter, HostDocumentCoordinator.
/// 2. BẢO MẬT TUYỆT ĐỐI (Security Gate): Toàn bộ thao tác ghi đĩa bền vững (File Persistence)
///    bị KHÓA CHẶT trong chế độ xem trước. Khi GridCraft phát sự kiện `gridcraft:save`,
///    Host sẽ chặn đứng, hiển thị cảnh báo an ninh và KHÔNG BAO GIỜ gọi `save_file_binary`.
/// 3. Hỗ trợ giao tiếp 2 chiều: Nạp tệp nhị phân (`gridcraft:open`), theo dõi dirty state (`gridcraft:dirty`).
/// 4. Giữ nguyên dải ruy-băng placeholder ở trạng thái vô hiệu hóa (Disabled Ribbon Placeholder).
#[component]
pub fn GridCraftCanvasFrame(
    #[prop(into, optional)] state: Option<DataLabState>,
) -> impl IntoView {
    let lab_state = state.or_else(use_context::<DataLabState>);

    // Tín hiệu nội bộ theo dõi trạng thái xem trước
    let is_canvas_ready = RwSignal::new(false);
    let is_dirty = RwSignal::new(false);
    let security_alert = RwSignal::new(Option::<String>::None);
    let current_doc_name = RwSignal::new("workbook.xlsx".to_string());

    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = lab_state {
            // Lấy mô tả tích hợp từ Registry
            let _descriptor = MicroAppRegistry::get(MicroAppId::DataLab);

            // Khởi tạo Coordinator với định danh tài liệu
            let coordinator = StoredValue::new(HostDocumentCoordinator::new());

            // Thiết lập Message Listener nhận sự kiện từ iframe qua window postMessage
            let window = web_sys::window();
            if let Some(win) = window {
                let adapter = GridCraftAdapter;
                let listener = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |ev: web_sys::MessageEvent| {
                    let data = ev.data();
                    let json_str = match js_sys::JSON::stringify(&data) {
                        Ok(s) => String::from(s),
                        Err(_) => return,
                    };

                    let json_val: serde_json::Value = match serde_json::from_str(&json_str) {
                        Ok(v) => v,
                        Err(_) => return,
                    };

                    let raw = match validate_structure(&json_val) {
                        Ok(mut msg) => {
                            // Trích xuất binary data nếu có mảng byte
                            if let Some(arr) = json_val.get("bytes").and_then(|b| b.as_array()) {
                                let bytes: Vec<u8> = arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect();
                                msg.binary_data = Some(bytes);
                            }
                            msg
                        }
                        Err(_) => return,
                    };

                    // Phân tích sự kiện qua GridCraftAdapter
                    if let Ok(event) = adapter.parse_incoming(&raw) {
                        match event {
                            DocumentStateEvent::Ready { version } => {
                                is_canvas_ready.set(true);
                                s.is_canvas_ready.set(true);
                                s.status_message.set(format!("✓ GridCraft Canvas sẵn sàng (v{}).", version));
                            }
                            DocumentStateEvent::Dirty { dirty } => {
                                is_dirty.set(dirty);
                                if dirty {
                                    coordinator.update_value(|c| c.on_user_edit());
                                    s.status_message.set("📝 Bảng tính GridCraft có thay đổi chưa lưu (Preview Mode).".to_string());
                                } else {
                                    s.status_message.set("✓ Bảng tính GridCraft ở trạng thái sạch.".to_string());
                                }
                            }
                            DocumentStateEvent::SaveRequested { name, intent: _, bytes, request_id } => {
                                // =========================================================================
                                // 🔒 SECURITY GATE: PERSISTENCE BỊ KHÓA HOÀN TOÀN TRONG CHẾ ĐỘ PREVIEW
                                // Tuyệt đối KHÔNG gọi save_file_binary hoặc ghi đĩa cục bộ!
                                // =========================================================================
                                let byte_count = bytes.len();
                                let req_id = request_id.unwrap_or_else(|| "untracked".to_string());
                                let warning = format!(
                                    "🔒 [Security Gate Active] Đã chặn yêu cầu lưu tệp '{}' ({} bytes, ID: {})! Tính năng ghi đĩa bị khóa an toàn trong Chế độ Xem trước Thử nghiệm (Preview Mode).",
                                    name, byte_count, req_id
                                );
                                security_alert.set(Some(warning));
                                s.status_message.set("🔒 Lưu tệp bị chặn: Chế độ xem trước không ghi đĩa.".to_string());
                                coordinator.update_value(|c| {
                                    let generation = c.session_generation();
                                    let now = js_sys::Date::now() as u64;
                                    let _ = c.on_save_requested(generation, "gridcraft-preview-doc", &req_id, bytes, now);
                                });
                            }
                            DocumentStateEvent::Failed { error } => {
                                s.status_message.set(format!("❌ Lỗi GridCraft Canvas: {}", error));
                            }
                        }
                    }
                });

                let _ = win.add_event_listener_with_callback("message", listener.as_ref().unchecked_ref());
                listener.forget();
            }

            // Theo dõi tệp đang chọn và tự động nạp vào GridCraft Canvas khi sẵn sàng
            let last_opened = StoredValue::new(None::<String>);
            Effect::new(move |_| {
                let ready = is_canvas_ready.get();
                if ready {
                    if let Some(path) = s.active_file_path() {
                        if last_opened.get_value().as_deref() != Some(&path) {
                            last_opened.set_value(Some(path.clone()));
                            current_doc_name.set(path.clone());
                            // Gửi thông điệp mở tệp vào iframe
                            send_open_command_to_iframe("gridcraft-frame", &path);
                        }
                    }
                }
            });
        }
    }

    view! {
        <div class="gridcraft-preview-container relative w-full h-full flex flex-col flex-1 overflow-hidden" style="width: 100%; height: 100%; min-height: 0; display: flex; flex-direction: column; flex: 1;">
            // 1. THANH CẢNH BÁO AN NINH & ĐIỀU HƯỚNG PREVIEW (Preview Security & Mode Bar)
            <div
                class="gridcraft-preview-bar"
                style="background: #1E293B; border-bottom: 1px solid #334155; padding: 6px 16px; display: flex; align-items: center; justify-content: space-between; gap: 12px; z-index: 10;"
            >
                <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-size: 14px;">"🧪"</span>
                    <span style="font-size: 12px; font-weight: 600; color: #F59E0B; text-transform: uppercase; letter-spacing: 0.05em;">
                        "Chế độ Xem trước Thử nghiệm (GridCraft Canvas Preview)"
                    </span>
                    <span
                        style="font-size: 12px; background: rgba(239, 68, 68, 0.2); color: #FCA5A5; border: 1px solid #EF4444; border-radius: 4px; padding: 2px 6px; font-weight: 500;"
                    >
                        "🔒 Persistence Disabled"
                    </span>
                    {move || {
                        if is_dirty.get() {
                            view! {
                                <span style="font-size: 12px; background: rgba(245, 158, 11, 0.2); color: #FCD34D; border: 1px solid #F59E0B; border-radius: 4px; padding: 2px 6px;">
                                    "● Có thay đổi chưa lưu"
                                </span>
                            }.into_any()
                        } else {
                            view! {
                                <span style="font-size: 12px; color: #94A3B8;">
                                    "○ Trạng thái sạch"
                                </span>
                            }.into_any()
                        }
                    }}
                </div>

                <div style="display: flex; align-items: center; gap: 8px;">
                    <button
                        type="button"
                        class="btn-switch-classic"
                        style="background: #334155; color: #F1F5F9; border: 1px solid #475569; border-radius: 4px; padding: 4px 10px; font-size: 12px; font-weight: 500; cursor: pointer; transition: all 0.15s ease;"
                        on:click=move |_| {
                            if let Some(s) = lab_state {
                                s.is_gridcraft_preview.set(false);
                            }
                        }
                    >
                        "⊞ Quay lại Bảng tính Cổ điển"
                    </button>
                </div>
            </div>

            // 2. BANNER CẢNH BÁO KHI PHÁT HIỆN YÊU CẦU LƯU (Security Gate Alert Banner)
            {move || {
                security_alert.get().map(|msg| {
                    view! {
                        <div
                            class="gridcraft-security-gate-alert"
                            style="background: #7F1D1D; color: #FEE2E2; padding: 8px 16px; font-size: 12px; display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid #EF4444;"
                            role="alert"
                        >
                            <span>{msg}</span>
                            <button
                                type="button"
                                style="background: transparent; border: none; color: #FEE2E2; font-size: 14px; cursor: pointer; padding: 0 4px;"
                                on:click=move |_| security_alert.set(None)
                            >
                                "✕"
                            </button>
                        </div>
                    }
                })
            }}

            // 3. PLACEHOLDER DẢI RUY-BĂNG BỔ TRỢ (Data Extension Ribbon - Bị vô hiệu hóa)
            <div
                class="data-extension-ribbon-placeholder"
                style="background: #0F172A; border-bottom: 1px solid #1E293B; padding: 4px 16px; display: flex; align-items: center; gap: 8px; opacity: 0.6;"
                title="Dải ruy-băng công cụ mở rộng (Sắp ra mắt — Vô hiệu hóa trong Preview)"
            >
                <span style="font-size: 12px; color: #64748B; font-weight: 500; margin-right: 8px;">
                    "Dải Ruy-băng Mở rộng (Stage 5 Placeholder):"
                </span>
                <button
                    type="button"
                    disabled=true
                    style="background: #1E293B; color: #64748B; border: 1px dashed #334155; border-radius: 4px; padding: 3px 8px; font-size: 12px; cursor: not-allowed; pointer-events: none;"
                >
                    "✨ Chuẩn hóa Cột (Tắt)"
                </button>
                <button
                    type="button"
                    disabled=true
                    style="background: #1E293B; color: #64748B; border: 1px dashed #334155; border-radius: 4px; padding: 3px 8px; font-size: 12px; cursor: not-allowed; pointer-events: none;"
                >
                    "🔍 Gom cụm Trùng lặp (Tắt)"
                </button>
                <button
                    type="button"
                    disabled=true
                    style="background: #1E293B; color: #64748B; border: 1px dashed #334155; border-radius: 4px; padding: 3px 8px; font-size: 12px; cursor: not-allowed; pointer-events: none;"
                >
                    "📊 Hồ sơ Dữ liệu (Tắt)"
                </button>
            </div>

            // 4. VÙNG HIỂN THỊ CHÍNH: LOADING SKELETON & WASM IFRAME
            <div class="relative w-full h-full flex-1 overflow-hidden" style="position: relative; width: 100%; height: 100%; min-height: 0; display: flex; flex: 1;">
                // Trạng thái loading skeleton trong lúc WASM canvas đang khởi động
                <div
                    id="gridcraft-skeleton"
                    class="gridcraft-loading-skeleton"
                    class:hidden=move || is_canvas_ready.get()
                    style="position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; background: #0F172A; color: #E2E8F0; z-index: 5;"
                    aria-live="polite"
                >
                    <div style="display: flex; flex-direction: column; align-items: center; gap: 12px; text-align: center;">
                        <div
                            class="animate-spin"
                            style="width: 32px; height: 32px; border: 3px solid #334155; border-top-color: #3B82F6; border-radius: 50%;"
                            aria-hidden="true"
                        ></div>
                        <div>
                            <div style="font-size: 14px; font-weight: 600; color: #F1F5F9;">
                                "Đang nạp GridCraft Canvas..."
                            </div>
                            <div style="font-size: 12px; color: #94A3B8; margin-top: 4px;">
                                "Khởi tạo WebAssembly & WebGL 100% On-Device"
                            </div>
                        </div>
                        <div style="margin-top: 12px;">
                            <button
                                type="button"
                                style="background: #3B82F6; color: #FFFFFF; border: none; border-radius: 6px; padding: 6px 14px; font-size: 12px; font-weight: 500; cursor: pointer;"
                                on:click=move |_| {
                                    if let Some(s) = lab_state {
                                        s.is_gridcraft_preview.set(false);
                                    }
                                }
                            >
                                "⚡ Quay lại Bảng tính Cổ điển (Classic Spreadsheet)"
                            </button>
                        </div>
                    </div>
                </div>

                // Khung nhúng iframe GridCraft WebAssembly Canvas
                <iframe
                    id="gridcraft-frame"
                    src="/gridcraft/index.html?host=parent&theme=dark&lang=vi"
                    class="w-full h-full border-none"
                    style="width: 100%; height: 100%; border: none; display: block; flex: 1;"
                    allow="clipboard-read; clipboard-write"
                    title="GridCraft Spreadsheet Canvas"
                ></iframe>
            </div>
        </div>
    }
}

/// Gửi thông điệp `gridcraft:open` tới iframe qua contentWindow.postMessage
#[cfg(target_arch = "wasm32")]
fn send_open_command_to_iframe(iframe_id: &str, file_name: &str) {
    use web_sys::window;
    if let Some(win) = window() {
        if let Some(doc) = win.document() {
            if let Some(elem) = doc.get_element_by_id(iframe_id) {
                if let Ok(iframe) = elem.dyn_into::<web_sys::HtmlIFrameElement>() {
                    if let Some(content_win) = iframe.content_window() {
                        let payload = serde_json::json!({
                            "type": "gridcraft:open",
                            "name": file_name,
                        });
                        let js_payload = wasm_bindgen::JsValue::from_str(&payload.to_string());
                        let _ = content_win.post_message(&js_payload, "*");
                    }
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn send_open_command_to_iframe(_iframe_id: &str, _file_name: &str) {}
