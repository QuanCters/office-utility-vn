#![allow(unused_imports, unused_variables, dead_code)]

use crate::micro_apps::gridcraft::state::GridCraftState;
use crate::platform::adapter::{DocumentStateEvent, GridCraftAdapter, MicroAppAdapter};
use crate::platform::coordinator::HostDocumentCoordinator;
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

/// Địa chỉ nạp iframe GridCraft chuẩn hóa cho Stage 4A & 4B
pub const GRIDCRAFT_IFRAME_SRC: &str =
    "/gridcraft/index.html?host=parent&save=confirmed&chrome=notitlebar";

/// ID phần tử iframe
pub const GRIDCRAFT_IFRAME_ID: &str = "gridcraft-workspace-frame";

/// Kết quả của thao tác lưu trong môi trường kiểm thử nội bộ (Dev-only Save Result)
#[derive(Debug, Clone)]
pub enum DevSaveResult {
    Saved {
        dest_path: String,
        new_doc_id: String,
        committed_rev: u64,
    },
    Canceled,
    Failed(String),
}

/// Xử lý hoàn tất lưu tệp và kiểm tra an toàn toàn bộ ứng dụng trước khi ủy quyền đóng cửa sổ (Finding 4)
pub fn handle_gridcraft_save_completion(
    is_still_dirty: bool,
    close_req_id: Option<String>,
    close_manager: Option<crate::platform::window_close::WindowCloseManager>,
    app_state: Option<crate::app::AppState>,
) {
    if let Some(req_id) = close_req_id {
        if !is_still_dirty {
            let other_dirty = app_state
                .as_ref()
                .map(|st| st.has_unsaved_work_or_running_task())
                .unwrap_or(false);
            if other_dirty {
                if let Some(cm) = close_manager {
                    cm.cancel_close(&req_id);
                    cm.error_message.set(Some(
                        "Vui lòng hoàn tất lưu tệp trong các ứng dụng khác trước khi đóng cửa sổ."
                            .into(),
                    ));
                }
            } else if let Some(cm) = close_manager {
                cm.authorize_close(&req_id);
            }
        } else if let Some(cm) = close_manager {
            cm.cancel_close(&req_id);
            cm.error_message.set(Some(
                "Phát hiện thay đổi mới trong quá trình lưu, đã hủy đóng cửa sổ để bảo vệ dữ liệu."
                    .into(),
            ));
        }
    }
}

/// Component `GridCraftWorkspace`:
/// Chiếm trọn 100% không gian làm việc (Full-Workspace),
/// nhúng trực tiếp giao diện gốc GridCraft WebAssembly mà không có bất kỳ thành phần nào của Data Lab cũ.
#[component]
pub fn GridCraftWorkspace(#[prop(into)] state: GridCraftState) -> impl IntoView {
    let coordinator = StoredValue::new(HostDocumentCoordinator::new());
    let close_manager =
        leptos::prelude::use_context::<crate::platform::window_close::WindowCloseManager>();
    let app_state = leptos::prelude::use_context::<crate::app::AppState>();

    // Finding 1: Đồng bộ hóa Coordinator ngay khi mở Workspace nếu State đã có tài liệu active
    let init_doc = state.active_document_id.get_untracked();
    let init_path = state.active_file_path.get_untracked();
    if let (Some(doc_id), Some(path)) = (init_doc, init_path) {
        coordinator.update_value(|c| {
            c.on_document_opened(&doc_id, &path, "init-mount");
            if state.is_dirty.get_untracked() {
                c.on_user_edit();
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window();
        if let Some(win) = window {
            let s = state;
            let listener = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(
                move |ev: web_sys::MessageEvent| {
                    let data = ev.data();
                    let type_val =
                        match js_sys::Reflect::get(&data, &wasm_bindgen::JsValue::from_str("type"))
                        {
                            Ok(v) => v.as_string(),
                            Err(_) => None,
                        };

                    if let Some(msg_type) = type_val {
                        match msg_type.as_str() {
                            "gridcraft:ready" => {
                                s.is_canvas_ready.set(true);
                                s.error_message.set(None);
                                #[cfg(debug_assertions)]
                                {
                                    if s.active_file_path.get_untracked().is_some() {
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let args =
                                                serde_wasm_bindgen::to_value(&serde_json::json!({
                                                    "fixtureId": "tc04_sample",
                                                }))
                                                .unwrap();
                                            if let Ok(val) = invoke("load_dev_fixture", args).await
                                            {
                                                if let Ok(handle) = serde_wasm_bindgen::from_value::<
                                                    serde_json::Value,
                                                >(
                                                    val
                                                ) {
                                                    let file_name = handle
                                                        .get("file_name")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("tc04_sample.xlsx")
                                                        .to_string();
                                                    let bytes_val = handle
                                                        .get("bytes")
                                                        .cloned()
                                                        .unwrap_or(serde_json::Value::Null);
                                                    let bytes: Vec<u8> =
                                                        serde_json::from_value(bytes_val)
                                                            .unwrap_or_default();
                                                    if !bytes.is_empty() {
                                                        send_open_to_iframe(
                                                            GRIDCRAFT_IFRAME_ID,
                                                            &file_name,
                                                            &bytes,
                                                            None,
                                                        );
                                                    }
                                                }
                                            }
                                        });
                                    }
                                }
                            }
                            "gridcraft:dirty" => {
                                let dirty = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("dirty"),
                                )
                                .ok()
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                                if let Some(active_id) = s.active_document_id.get_untracked() {
                                    s.set_document_dirty(&active_id, dirty);
                                } else {
                                    s.is_dirty.set(dirty);
                                }
                                if dirty {
                                    if coordinator.read_value().state()
                                        == &crate::platform::coordinator::DocumentState::Unloaded
                                    {
                                        let cur_doc = s
                                            .active_document_id
                                            .get_untracked()
                                            .unwrap_or_else(|| "gridcraft-doc".into());
                                        let cur_path = s
                                            .active_file_path
                                            .get_untracked()
                                            .unwrap_or_else(|| "workbook.xlsx".into());
                                        coordinator.update_value(|c| {
                                            c.on_document_opened(&cur_doc, &cur_path, "dirty-init");
                                        });
                                    }
                                    coordinator.update_value(|c| c.on_user_edit());
                                }
                            }
                            "gridcraft:save" => {
                                let name = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("name"),
                                )
                                .ok()
                                .and_then(|v| v.as_string())
                                .unwrap_or_else(|| "workbook.xlsx".to_string());
                                let req_id = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("id"),
                                )
                                .ok()
                                .and_then(|v| v.as_string());
                                let save_as = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("saveAs"),
                                )
                                .ok()
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);

                                let bytes_val = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("bytes"),
                                )
                                .unwrap_or(wasm_bindgen::JsValue::UNDEFINED);
                                let bytes = if let Some(array) =
                                    bytes_val.dyn_ref::<js_sys::Uint8Array>()
                                {
                                    array.to_vec()
                                } else if bytes_val.is_instance_of::<js_sys::ArrayBuffer>() {
                                    js_sys::Uint8Array::new(&bytes_val).to_vec()
                                } else {
                                    Vec::new()
                                };

                                let gate_res =
                                    s.handle_save_requested(&name, req_id.as_deref(), &bytes);

                                if gate_res.is_err() {
                                    // Gating an ninh (Gate 5): Vô hiệu hóa Production Save
                                    if let Some(ref id) = req_id {
                                        send_save_failed_to_iframe(
                                            GRIDCRAFT_IFRAME_ID,
                                            "Tính năng lưu tệp Production bị vô hiệu hóa an toàn.",
                                            Some(id),
                                        );
                                    }
                                    if let Some(close_req_id) =
                                        s.pending_close_request_id.get_untracked()
                                    {
                                        s.pending_close_request_id.set(None);
                                        if let Some(cm) = close_manager {
                                            cm.cancel_close(&close_req_id);
                                            cm.error_message.set(Some("Lưu thất bại: Tính năng lưu tệp Production bị vô hiệu hóa an toàn.".into()));
                                        }
                                    }
                                } else {
                                    // Finding 2: Phân giải chính xác phiên workbook đích để tránh ghi đè workbook khác
                                    let resolved_session = s.resolve_save_session(&name);
                                    let (target_doc_id, target_path, target_rev) =
                                        if let Some(ref sess) = resolved_session {
                                            (
                                                Some(sess.document_id.clone()),
                                                Some(sess.file_path.clone()),
                                                sess.revision,
                                            )
                                        } else if save_as {
                                            (None, None, 0)
                                        } else {
                                            // Tệp lạ không thuộc phiên đã đăng ký -> buộc Save As để bảo vệ file hiện hành
                                            (None, None, 0)
                                        };

                                    let effective_doc_id =
                                        target_doc_id.as_deref().unwrap_or("gridcraft-doc");
                                    let effective_path = target_path.as_deref().unwrap_or(&name);

                                    // Finding 1: Đồng bộ hóa Coordinator với workbook này trước khi phát lệnh lưu
                                    if coordinator.read_value().state()
                                        == &crate::platform::coordinator::DocumentState::Unloaded
                                        || coordinator.read_value().dest_path()
                                            != Some(effective_path)
                                    {
                                        coordinator.update_value(|c| {
                                            c.on_document_opened(
                                                effective_doc_id,
                                                effective_path,
                                                "save-sync",
                                            );
                                            if s.is_dirty.get() {
                                                c.on_user_edit();
                                            }
                                        });
                                    }

                                    static SAVE_REQ_COUNTER: std::sync::atomic::AtomicU64 =
                                        std::sync::atomic::AtomicU64::new(1);
                                    let session_gen = coordinator.read_value().session_generation();
                                    let counter = SAVE_REQ_COUNTER
                                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    let req_id_str = req_id.unwrap_or_else(|| {
                                        format!(
                                            "save-g{}-{}-{}",
                                            session_gen,
                                            js_sys::Date::now() as u64,
                                            counter
                                        )
                                    });
                                    let name_clone = name.clone();
                                    let bytes_clone = bytes.clone();
                                    let req_clone = req_id_str.clone();

                                    let mut action =
                                        crate::platform::coordinator::CoordinatorAction::None;
                                    coordinator.update_value(|c| {
                                        action = c.on_save_requested(
                                            session_gen,
                                            effective_doc_id,
                                            &req_clone,
                                            bytes_clone.clone(),
                                            js_sys::Date::now() as u64,
                                        );
                                    });

                                    // Finding 1: Nếu coordinator từ chối lưu (khác gen, sai doc ID), fail-closed ngay lập tức!
                                    if matches!(
                                        action,
                                        crate::platform::coordinator::CoordinatorAction::None
                                    ) {
                                        s.save_blocked_detail.set(
                                            "Yêu cầu lưu bị từ chối bởi bộ điều phối an toàn."
                                                .into(),
                                        );
                                        s.is_save_blocked_alert.set(true);
                                        send_save_failed_to_iframe(
                                            GRIDCRAFT_IFRAME_ID,
                                            "Yêu cầu lưu bị từ chối",
                                            Some(&req_clone),
                                        );
                                        if let Some(close_req_id) =
                                            s.pending_close_request_id.get_untracked()
                                        {
                                            s.pending_close_request_id.set(None);
                                            if let Some(cm) = close_manager {
                                                cm.cancel_close(&close_req_id);
                                                cm.error_message.set(Some("Yêu cầu lưu bị từ chối bởi bộ điều phối an toàn.".into()));
                                            }
                                        }
                                        return;
                                    }

                                    wasm_bindgen_futures::spawn_local(async move {
                                        let outcome = execute_gridcraft_save(
                                            &name_clone,
                                            target_path.as_deref(),
                                            target_doc_id.as_deref(),
                                            target_rev + 1,
                                            session_gen,
                                            save_as,
                                            &req_clone,
                                            &bytes_clone,
                                        )
                                        .await;

                                        match outcome {
                                            Ok(DevSaveResult::Saved {
                                                dest_path,
                                                new_doc_id,
                                                committed_rev,
                                            }) => {
                                                coordinator.update_value(|c| {
                                                let fp = crate::platform::persistence::compute_fingerprint(&bytes_clone);
                                                c.on_durable_write_completed(&req_clone, &fp);
                                            });
                                                let is_still_dirty =
                                                    coordinator.read_value().is_dirty();
                                                s.set_document_dirty(&new_doc_id, is_still_dirty);
                                                s.register_document_session(crate::micro_apps::gridcraft::state::WorkbookSession {
                                                    document_id: new_doc_id.clone(),
                                                    file_path: dest_path.clone(),
                                                    file_name: name_clone.clone(),
                                                    revision: committed_rev,
                                                    is_dirty: is_still_dirty,
                                                });
                                                s.set_document_session(
                                                    Some(new_doc_id),
                                                    Some(dest_path.clone()),
                                                    committed_rev,
                                                );
                                                s.active_file_name.set(Some(name_clone));
                                                send_saved_to_iframe(
                                                    GRIDCRAFT_IFRAME_ID,
                                                    &req_clone,
                                                );

                                                // Finding 4: Kiểm tra toàn bộ ứng dụng trước khi ủy quyền đóng cửa sổ
                                                let close_req =
                                                    s.pending_close_request_id.get_untracked();
                                                s.pending_close_request_id.set(None);
                                                handle_gridcraft_save_completion(
                                                    is_still_dirty,
                                                    close_req,
                                                    close_manager,
                                                    app_state,
                                                );
                                            }
                                            Ok(DevSaveResult::Canceled) => {
                                                coordinator.update_value(|c| {
                                                    c.on_save_cancelled(&req_clone)
                                                });
                                                s.is_dirty.set(true);
                                                send_save_failed_to_iframe(
                                                    GRIDCRAFT_IFRAME_ID,
                                                    "Đã hủy thao tác lưu",
                                                    Some(&req_clone),
                                                );
                                                if let Some(close_req_id) =
                                                    s.pending_close_request_id.get_untracked()
                                                {
                                                    s.pending_close_request_id.set(None);
                                                    if let Some(cm) = close_manager {
                                                        cm.cancel_close(&close_req_id);
                                                    }
                                                }
                                            }
                                            Ok(DevSaveResult::Failed(err)) => {
                                                coordinator.update_value(|c| {
                                                    let _ =
                                                        c.on_durable_write_failed(&req_clone, &err);
                                                });
                                                s.is_dirty.set(true);
                                                s.save_blocked_detail
                                                    .set(format!("❌ Lỗi ghi đĩa: {err}"));
                                                s.is_save_blocked_alert.set(true);
                                                send_save_failed_to_iframe(
                                                    GRIDCRAFT_IFRAME_ID,
                                                    &err,
                                                    Some(&req_clone),
                                                );
                                                if let Some(close_req_id) =
                                                    s.pending_close_request_id.get_untracked()
                                                {
                                                    s.pending_close_request_id.set(None);
                                                    if let Some(cm) = close_manager {
                                                        cm.cancel_close(&close_req_id);
                                                        cm.error_message.set(Some(format!(
                                                            "Lỗi ghi đĩa: {err}"
                                                        )));
                                                    }
                                                }
                                            }
                                            Err(err) => {
                                                coordinator.update_value(|c| {
                                                    let _ =
                                                        c.on_durable_write_failed(&req_clone, &err);
                                                });
                                                s.is_dirty.set(true);
                                                s.save_blocked_detail
                                                    .set(format!("❌ Lỗi hệ thống: {err}"));
                                                s.is_save_blocked_alert.set(true);
                                                send_save_failed_to_iframe(
                                                    GRIDCRAFT_IFRAME_ID,
                                                    &err,
                                                    Some(&req_clone),
                                                );
                                                if let Some(close_req_id) =
                                                    s.pending_close_request_id.get_untracked()
                                                {
                                                    s.pending_close_request_id.set(None);
                                                    if let Some(cm) = close_manager {
                                                        cm.cancel_close(&close_req_id);
                                                        cm.error_message.set(Some(format!(
                                                            "Lỗi hệ thống: {err}"
                                                        )));
                                                    }
                                                }
                                            }
                                        }
                                    });
                                }
                            }
                            "gridcraft:request_open" => {
                                let session_gen = coordinator.read_value().session_generation();
                                wasm_bindgen_futures::spawn_local(async move {
                                    let _ = open_file_into_gridcraft(
                                        GRIDCRAFT_IFRAME_ID,
                                        s,
                                        session_gen,
                                    )
                                    .await;
                                });
                            }
                            "gridcraft:open_succeeded" => {
                                let open_id = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("open_id"),
                                )
                                .ok()
                                .and_then(|v| v.as_string());
                                if let Some(id) = open_id {
                                    let cur_gen = coordinator.read_value().session_generation();
                                    if let Some(prov) = s.provisional_open.get_untracked() {
                                        if prov.open_id == id && prov.generation == cur_gen {
                                            let doc_id = prov.document_id.clone();
                                            let path = prov.file_path.clone();
                                            coordinator.update_value(|c| {
                                                c.on_document_opened(
                                                    &doc_id,
                                                    &path,
                                                    "open-succeeded",
                                                );
                                            });
                                        }
                                    }
                                    s.commit_provisional_open_with_generation(&id, cur_gen);
                                }
                            }
                            "gridcraft:open_failed" => {
                                let open_id = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("open_id"),
                                )
                                .ok()
                                .and_then(|v| v.as_string());
                                let err = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("error"),
                                )
                                .ok()
                                .and_then(|v| v.as_string())
                                .unwrap_or_else(|| "Tệp không hợp lệ".to_string());
                                s.abort_provisional_open(open_id.as_deref());
                                s.save_blocked_detail
                                    .set(format!("❌ Không thể mở tệp: {err}"));
                                s.is_save_blocked_alert.set(true);
                            }
                            "gridcraft:failed" => {
                                let error = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("error"),
                                )
                                .ok()
                                .and_then(|v| v.as_string())
                                .unwrap_or_else(|| "Lỗi GridCraft".to_string());
                                s.error_message.set(Some(error));
                            }
                            _ => {}
                        }
                    }
                },
            );

            let _ =
                win.add_event_listener_with_callback("message", listener.as_ref().unchecked_ref());
            let win_clone = win.clone();
            let listener_js = listener.as_ref().clone();
            on_cleanup(move || {
                let _ = win_clone
                    .remove_event_listener_with_callback("message", listener_js.unchecked_ref());
            });
            listener.forget();
        }
    }

    view! {
        <div class="gridcraft-workspace-root w-full h-full flex flex-col flex-1 relative overflow-hidden" style="width: 100%; height: 100%; min-height: 0; display: flex; flex-direction: column; flex: 1; background: #18181B;">
            // 1. CẢNH BÁO AN NINH KHI CỐ GẮNG LƯU (Stage 4A Security Gate Alert)
            {move || {
                if state.is_save_blocked_alert.get() {
                    let msg = state.save_blocked_detail.get();
                    view! {
                        <div
                            class="gridcraft-security-banner"
                            style="background: #7F1D1D; color: #FEE2E2; padding: 6px 16px; font-size: 12px; display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid #EF4444; z-index: 20;"
                            role="alert"
                        >
                            <span>{msg}</span>
                            <button
                                type="button"
                                style="background: transparent; border: none; color: #FEE2E2; font-size: 14px; cursor: pointer; padding: 0 6px;"
                                on:click=move |_| state.is_save_blocked_alert.set(false)
                            >
                                "✕"
                            </button>
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }
            }}

            // 2. VÙNG KHỞI TẠO / LOADING SKELETON (Tự unmount khi canvas ready)
            {move || {
                if !state.is_canvas_ready.get() {
                    view! {
                        <div
                            class="gridcraft-loading-overlay"
                            style="position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; background: #0F172A; color: #E2E8F0; z-index: 10;"
                        >
                            {if let Some(err) = state.error_message.get() {
                                view! {
                                    <div style="display: flex; flex-direction: column; align-items: center; gap: 12px; text-align: center; max-width: 480px; padding: 24px;">
                                        <span style="font-size: 32px;">"⚠️"</span>
                                        <div style="font-size: 16px; font-weight: 600; color: #EF4444;">"Khởi động GridCraft Thất bại"</div>
                                        <div style="font-size: 12px; color: #94A3B8;">{err}</div>
                                        <button
                                            type="button"
                                            style="margin-top: 12px; background: #3B82F6; color: #FFFFFF; border: none; border-radius: 6px; padding: 8px 16px; cursor: pointer; font-size: 13px;"
                                            on:click=move |_| reload_iframe(GRIDCRAFT_IFRAME_ID)
                                        >
                                            "Thử lại"
                                        </button>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div style="display: flex; flex-direction: column; align-items: center; gap: 12px;">
                                        <div
                                            class="animate-spin"
                                            style="width: 36px; height: 36px; border: 3px solid #334155; border-top-color: #10B981; border-radius: 50%;"
                                        ></div>
                                        <div style="font-size: 14px; font-weight: 600; color: #F1F5F9;">"Đang nạp Ứng dụng Bảng tính GridCraft..."</div>
                                        <div style="font-size: 12px; color: #94A3B8;">"WebAssembly & WebGL2 Engine (100% On-Device)"</div>
                                    </div>
                                }.into_any()
                            }}
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }
            }}

            // 3. KHUNG NHÚNG IFRAME FULL-WORKSPACE
            <iframe
                id=GRIDCRAFT_IFRAME_ID
                src=GRIDCRAFT_IFRAME_SRC
                class="gridcraft-iframe w-full h-full border-none flex-1"
                style="width: 100%; height: 100%; border: none; display: block; flex: 1; min-height: 0;"
                allow="clipboard-read; clipboard-write"
                title="GridCraft Full Spreadsheet Application"
                on:load=move |_| {
                    state.is_canvas_ready.set(true);
                }
            ></iframe>
        </div>
    }
}

/// Gửi thông báo lỗi lưu ngược vào iframe
#[cfg(target_arch = "wasm32")]
pub fn send_save_failed_to_iframe(iframe_id: &str, error: &str, req_id: Option<&str>) {
    use web_sys::window;
    if let Some(win) = window() {
        if let Some(doc) = win.document() {
            if let Some(elem) = doc.get_element_by_id(iframe_id) {
                if let Ok(iframe) = elem.dyn_into::<web_sys::HtmlIFrameElement>() {
                    if let Some(cw) = iframe.content_window() {
                        let obj = js_sys::Object::new();
                        let _ = js_sys::Reflect::set(
                            &obj,
                            &wasm_bindgen::JsValue::from_str("type"),
                            &wasm_bindgen::JsValue::from_str("gridcraft:failed"),
                        );
                        let _ = js_sys::Reflect::set(
                            &obj,
                            &wasm_bindgen::JsValue::from_str("error"),
                            &wasm_bindgen::JsValue::from_str(error),
                        );
                        if let Some(id) = req_id {
                            let _ = js_sys::Reflect::set(
                                &obj,
                                &wasm_bindgen::JsValue::from_str("id"),
                                &wasm_bindgen::JsValue::from_str(id),
                            );
                        }
                        let _ = cw.post_message(&obj, "*");
                    }
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn send_save_failed_to_iframe(_iframe_id: &str, _error: &str, _req_id: Option<&str>) {}

/// Gửi thông báo xác nhận lưu thành công vào GridCraft iframe (Bản vá 0001 confirmed save)
#[cfg(target_arch = "wasm32")]
pub fn send_saved_to_iframe(iframe_id: &str, req_id: &str) {
    use web_sys::window;
    if let Some(win) = window() {
        if let Some(doc) = win.document() {
            if let Some(elem) = doc.get_element_by_id(iframe_id) {
                if let Ok(iframe) = elem.dyn_into::<web_sys::HtmlIFrameElement>() {
                    if let Some(cw) = iframe.content_window() {
                        let obj = js_sys::Object::new();
                        let _ = js_sys::Reflect::set(
                            &obj,
                            &wasm_bindgen::JsValue::from_str("type"),
                            &wasm_bindgen::JsValue::from_str("gridcraft:saved"),
                        );
                        let _ = js_sys::Reflect::set(
                            &obj,
                            &wasm_bindgen::JsValue::from_str("id"),
                            &wasm_bindgen::JsValue::from_str(req_id),
                        );
                        let _ = cw.post_message(&obj, "*");
                    }
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn send_saved_to_iframe(_iframe_id: &str, _req_id: &str) {}

/// Gửi thông điệp yêu cầu iframe GridCraft tuần tự hóa và lưu tệp chủ động (Stage 4C-0C Task 1)
#[cfg(target_arch = "wasm32")]
pub fn send_request_save_to_iframe(iframe_id: &str, req_id: Option<&str>, save_as: bool) {
    use web_sys::window;
    if let Some(win) = window() {
        if let Some(doc) = win.document() {
            if let Some(elem) = doc.get_element_by_id(iframe_id) {
                if let Ok(iframe) = elem.dyn_into::<web_sys::HtmlIFrameElement>() {
                    if let Some(cw) = iframe.content_window() {
                        let obj = js_sys::Object::new();
                        let _ = js_sys::Reflect::set(
                            &obj,
                            &wasm_bindgen::JsValue::from_str("type"),
                            &wasm_bindgen::JsValue::from_str("gridcraft:request_save"),
                        );
                        if let Some(id) = req_id {
                            let _ = js_sys::Reflect::set(
                                &obj,
                                &wasm_bindgen::JsValue::from_str("id"),
                                &wasm_bindgen::JsValue::from_str(id),
                            );
                            let _ = js_sys::Reflect::set(
                                &obj,
                                &wasm_bindgen::JsValue::from_str("correlation_id"),
                                &wasm_bindgen::JsValue::from_str(id),
                            );
                        }
                        let _ = js_sys::Reflect::set(
                            &obj,
                            &wasm_bindgen::JsValue::from_str("saveAs"),
                            &wasm_bindgen::JsValue::from_bool(save_as),
                        );
                        let _ = cw.post_message(&obj, "*");
                    }
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn send_request_save_to_iframe(_iframe_id: &str, _req_id: Option<&str>, _save_as: bool) {}

/// Gửi thông điệp nạp tài liệu workbook vào GridCraft iframe
#[cfg(target_arch = "wasm32")]
pub fn send_open_to_iframe(iframe_id: &str, name: &str, bytes: &[u8], open_id: Option<&str>) {
    use web_sys::window;
    if let Some(win) = window() {
        if let Some(doc) = win.document() {
            if let Some(elem) = doc.get_element_by_id(iframe_id) {
                if let Ok(iframe) = elem.dyn_into::<web_sys::HtmlIFrameElement>() {
                    if let Some(cw) = iframe.content_window() {
                        let uint8_array = js_sys::Uint8Array::from(bytes);
                        let obj = js_sys::Object::new();
                        let _ = js_sys::Reflect::set(
                            &obj,
                            &wasm_bindgen::JsValue::from_str("type"),
                            &wasm_bindgen::JsValue::from_str("gridcraft:open"),
                        );
                        let _ = js_sys::Reflect::set(
                            &obj,
                            &wasm_bindgen::JsValue::from_str("name"),
                            &wasm_bindgen::JsValue::from_str(name),
                        );
                        let _ = js_sys::Reflect::set(
                            &obj,
                            &wasm_bindgen::JsValue::from_str("bytes"),
                            &uint8_array,
                        );
                        if let Some(id) = open_id {
                            let _ = js_sys::Reflect::set(
                                &obj,
                                &wasm_bindgen::JsValue::from_str("open_id"),
                                &wasm_bindgen::JsValue::from_str(id),
                            );
                        }
                        let _ = cw.post_message(&obj, "*");
                    }
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn send_open_to_iframe(_iframe_id: &str, _name: &str, _bytes: &[u8], _open_id: Option<&str>) {}

/// Thực thi quy trình lưu đĩa bảo mật chính thức của GridCraft (Stage 4C-1 Product Cutover)
#[cfg(target_arch = "wasm32")]
pub async fn execute_gridcraft_save(
    name: &str,
    file_path: Option<&str>,
    document_id: Option<&str>,
    expected_rev: u64,
    session_gen: u64,
    save_as: bool,
    req_id: &str,
    bytes: &[u8],
) -> Result<DevSaveResult, String> {
    // 1. Regular Save: Có sẵn file_path và document_id đã đăng ký
    if !save_as && let (Some(_path), Some(doc_id)) = (file_path, document_id) {
        let grant_args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "documentId": doc_id,
            "appInstanceId": "gridcraft-workspace-instance",
            "sessionGeneration": session_gen,
            "requestId": req_id,
            "expectedRevision": expected_rev,
        }))
        .map_err(|e| format!("{e:?}"))?;

        if let Ok(grant_val) = invoke("request_save_grant", grant_args).await {
            if let Ok(grant) = serde_wasm_bindgen::from_value::<serde_json::Value>(grant_val) {
                if let (Some(grant_id), Some(req)) = (
                    grant.get("grant_id").and_then(|v| v.as_str()),
                    grant.get("request_id").and_then(|v| v.as_str()),
                ) {
                    let save_args = serde_wasm_bindgen::to_value(&serde_json::json!({
                        "grantId": grant_id,
                        "requestId": req,
                        "bytes": bytes,
                    }))
                    .map_err(|e| format!("{e:?}"))?;

                    match invoke("secure_save_document", save_args).await {
                        Ok(save_res) => {
                            let (dest, rev) = if let Ok(resp) =
                                serde_wasm_bindgen::from_value::<serde_json::Value>(save_res)
                            {
                                (
                                    resp.get("dest_path")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or(_path)
                                        .to_string(),
                                    resp.get("committed_revision")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(expected_rev),
                                )
                            } else {
                                (_path.to_string(), expected_rev)
                            };
                            return Ok(DevSaveResult::Saved {
                                dest_path: dest,
                                new_doc_id: doc_id.to_string(),
                                committed_rev: rev,
                            });
                        }
                        Err(e) => {
                            return Ok(DevSaveResult::Failed(format!("Lỗi ghi đĩa: {e:?}")));
                        }
                    }
                }
            }
        }
    }

    // 2. Save As (hoặc Lưu lần đầu) thông qua Native Dialog
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "documentId": document_id,
        "appInstanceId": "gridcraft-workspace-instance",
        "sessionGeneration": session_gen,
        "defaultName": name,
        "filterName": "Bảng tính Excel (*.xlsx)",
        "extensions": ["xlsx", "xlsm", "xls", "ods", "csv"],
    }))
    .unwrap_or(wasm_bindgen::JsValue::UNDEFINED);

    match invoke("native_dialog_save_as", args).await {
        Ok(js_res) => {
            if let Ok(Some(handle)) =
                serde_wasm_bindgen::from_value::<Option<serde_json::Value>>(js_res)
            {
                if let Some(grant) = handle.get("grant") {
                    if let (Some(grant_id), Some(g_req_id)) = (
                        grant.get("grant_id").and_then(|v| v.as_str()),
                        grant.get("request_id").and_then(|v| v.as_str()),
                    ) {
                        let save_args = serde_wasm_bindgen::to_value(&serde_json::json!({
                            "grantId": grant_id,
                            "requestId": g_req_id,
                            "bytes": bytes,
                        }))
                        .map_err(|e| format!("{e:?}"))?;

                        match invoke("secure_save_document", save_args).await {
                            Ok(save_res) => {
                                let rev = if let Ok(resp) =
                                    serde_wasm_bindgen::from_value::<serde_json::Value>(save_res)
                                {
                                    resp.get("committed_revision")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(1)
                                } else {
                                    1
                                };
                                let dest_path = handle
                                    .get("file_path")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(name)
                                    .to_string();
                                let new_doc_id = handle
                                    .get("document_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("doc-gridcraft-new")
                                    .to_string();
                                return Ok(DevSaveResult::Saved {
                                    dest_path,
                                    new_doc_id,
                                    committed_rev: rev,
                                });
                            }
                            Err(e) => {
                                return Ok(DevSaveResult::Failed(format!(
                                    "Ghi đĩa thất bại: {e:?}"
                                )));
                            }
                        }
                    }
                }
                Ok(DevSaveResult::Failed(
                    "Phản hồi Save As không hợp lệ".to_string(),
                ))
            } else {
                // Người dùng Cancel
                Ok(DevSaveResult::Canceled)
            }
        }
        Err(e) => Ok(DevSaveResult::Failed(format!("Lỗi hộp thoại: {e:?}"))),
    }
}

/// Giữ bí danh tương thích cho môi trường dev và kiểm thử tự động
#[cfg(target_arch = "wasm32")]
pub async fn execute_gridcraft_dev_save(
    name: &str,
    file_path: Option<&str>,
    document_id: Option<&str>,
    expected_rev: u64,
    save_as: bool,
    req_id: &str,
    bytes: &[u8],
) -> Result<DevSaveResult, String> {
    execute_gridcraft_save(
        name,
        file_path,
        document_id,
        expected_rev,
        1,
        save_as,
        req_id,
        bytes,
    )
    .await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn execute_gridcraft_save(
    _name: &str,
    _file_path: Option<&str>,
    _document_id: Option<&str>,
    _expected_rev: u64,
    _session_gen: u64,
    _save_as: bool,
    _req_id: &str,
    _bytes: &[u8],
) -> Result<DevSaveResult, String> {
    Ok(DevSaveResult::Canceled)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn execute_gridcraft_dev_save(
    _name: &str,
    _file_path: Option<&str>,
    _document_id: Option<&str>,
    _expected_rev: u64,
    _save_as: bool,
    _req_id: &str,
    _bytes: &[u8],
) -> Result<DevSaveResult, String> {
    Ok(DevSaveResult::Canceled)
}

/// Mở tệp bảng tính qua hộp thoại OS Native File Dialog và nạp vào GridCraft (Stage 4C-1 Transactional Open)
#[cfg(target_arch = "wasm32")]
pub async fn open_file_into_gridcraft(
    iframe_id: &str,
    state: GridCraftState,
    session_gen: u64,
) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "appInstanceId": "gridcraft-workspace-instance",
        "sessionGeneration": session_gen,
        "filterName": "Bảng tính Excel (*.xlsx)",
        "extensions": ["xlsx", "xlsm", "xls", "ods", "csv"],
    }))
    .map_err(|e| format!("{e:?}"))?;

    if let Ok(js_res) = invoke("native_dialog_open_file", args).await {
        if let Ok(Some(handle)) =
            serde_wasm_bindgen::from_value::<Option<serde_json::Value>>(js_res)
        {
            let doc_id = handle
                .get("document_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let file_name = handle
                .get("file_name")
                .and_then(|v| v.as_str())
                .unwrap_or("workbook.xlsx")
                .to_string();
            let file_path = handle
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let rev = handle
                .get("initial_revision")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let bytes_val = handle
                .get("bytes")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let bytes: Vec<u8> = serde_json::from_value(bytes_val).unwrap_or_default();

            // Workstream 4: Audit Open request ID generation (kết hợp generation, timestamp và counter đơn điệu)
            static OPEN_REQ_COUNTER: std::sync::atomic::AtomicU64 =
                std::sync::atomic::AtomicU64::new(1);
            let counter = OPEN_REQ_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let open_req_id = format!(
                "open-g{}-{}-{}",
                session_gen,
                js_sys::Date::now() as u64,
                counter
            );

            state.begin_provisional_open(
                open_req_id.clone(),
                doc_id,
                file_path,
                file_name.clone(),
                rev,
                session_gen,
            );

            send_open_to_iframe(iframe_id, &file_name, &bytes, Some(&open_req_id));
            return Ok(());
        } else {
            state.abort_provisional_open(None);
        }
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn open_file_into_gridcraft(
    _iframe_id: &str,
    _state: GridCraftState,
    _session_gen: u64,
) -> Result<(), String> {
    Ok(())
}

/// Tải lại iframe khi cần thử lại
#[cfg(target_arch = "wasm32")]
fn reload_iframe(iframe_id: &str) {
    use web_sys::window;
    if let Some(win) = window() {
        if let Some(doc) = win.document() {
            if let Some(elem) = doc.get_element_by_id(iframe_id) {
                if let Ok(iframe) = elem.dyn_into::<web_sys::HtmlIFrameElement>() {
                    let _ = iframe.set_src(GRIDCRAFT_IFRAME_SRC);
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn reload_iframe(_iframe_id: &str) {}
