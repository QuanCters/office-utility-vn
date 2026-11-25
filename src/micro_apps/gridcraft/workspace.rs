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
    AmbiguousCommit {
        dest_path: String,
        new_doc_id: String,
        committed_rev: u64,
        message: String,
    },
    Canceled,
    Failed(String),
}

/// Bối cảnh yêu cầu lưu bất biến cho GridCraft, được chụp khi yêu cầu lưu được chấp nhận
/// (P1 #4684931088: Bảo lưu danh tính tài liệu nguồn, iframe UID, intent, correlation ID và snapshot revision)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridCraftSaveRequestContext {
    pub request_id: String,
    pub operation_intent: crate::platform::adapter::SaveOperationIntent,
    pub originating_document_id: Option<String>,
    pub originating_path: Option<String>,
    pub originating_name: String,
    pub iframe_doc_uid: Option<u64>,
    pub snapshot_edit_revision: u64,
    pub base_committed_revision: u64,
    pub session_generation: u64,
}

/// Đồng bộ trạng thái coordinator dựa trên thông báo dirty/revision thực tế (P2 #4690136471).
/// Khi đang `DocumentState::Saving`, chỉ kích hoạt `on_user_edit()` (đặt `dirty_during_save = true`)
/// nếu revision của chính workbook đang lưu thực sự tăng thêm — tuyệt đối không kích hoạt theo snapshot lặp định kỳ!
pub fn sync_coordinator_on_dirty_notification(
    c: &mut HostDocumentCoordinator,
    dirty: bool,
    has_documents_snapshot: bool,
    advanced_doc_ids: &[String],
    active_doc_id: Option<&str>,
    active_file_path: Option<&str>,
) {
    if !dirty {
        return;
    }

    if *c.state() == crate::platform::coordinator::DocumentState::Unloaded {
        let cur_doc = active_doc_id.unwrap_or("gridcraft-doc");
        let cur_path = active_file_path.unwrap_or("workbook.xlsx");
        c.on_document_opened(cur_doc, cur_path, "dirty-init");
    }

    match c.state().clone() {
        crate::platform::coordinator::DocumentState::Saving { document_id, .. } => {
            if has_documents_snapshot {
                if advanced_doc_ids.iter().any(|id| id == &document_id) {
                    c.on_user_edit();
                }
            } else {
                c.on_user_edit();
            }
        }
        crate::platform::coordinator::DocumentState::Clean => {
            c.on_user_edit();
        }
        crate::platform::coordinator::DocumentState::Dirty
        | crate::platform::coordinator::DocumentState::SaveFailed { .. } => {
            if !has_documents_snapshot
                || c.active_document_id()
                    .map(|id| advanced_doc_ids.iter().any(|adv| adv == id))
                    .unwrap_or(false)
            {
                c.on_user_edit();
            }
        }
        _ => {}
    }
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

/// Xử lý kết quả lưu tài liệu GridCraft theo các bất biến sản phẩm (Finding 4, P1 #4676016506 & P1 #4684931088)
pub fn handle_gridcraft_save_outcome(
    s: &GridCraftState,
    coordinator: Option<&leptos::prelude::StoredValue<HostDocumentCoordinator>>,
    outcome: Result<DevSaveResult, String>,
    ctx: &GridCraftSaveRequestContext,
    bytes: &[u8],
    close_manager: Option<crate::platform::window_close::WindowCloseManager>,
    app_state: Option<crate::app::AppState>,
) {
    s.clear_pending_save_request(&ctx.request_id);

    let intent = ctx.operation_intent;
    let name = ctx.originating_name.as_str();
    let req_id = ctx.request_id.as_str();
    let snapshot_edit_rev = ctx.snapshot_edit_revision;

    // P1 #4684931088: Danh tính tài liệu nguồn và iframe UID bắt buộc lấy từ ngữ cảnh bất biến `ctx`,
    // TUYỆT ĐỐI KHÔNG suy diễn từ `s.active_document_id` tại thời điểm hoàn tất bất đồng bộ.
    let originating_doc_id = ctx.originating_document_id.clone();
    let originating_uid = ctx.iframe_doc_uid.or_else(|| {
        originating_doc_id.as_deref().and_then(|id| {
            s.document_sessions
                .with_untracked(|m| m.get(id).and_then(|sess| sess.iframe_doc_uid))
        })
    });

    let lookup_origin_live_edit_rev = || -> u64 {
        originating_doc_id
            .as_deref()
            .and_then(|id| {
                s.document_sessions
                    .with_untracked(|m| m.get(id).map(|sess| sess.edit_revision))
            })
            .or_else(|| {
                originating_uid.and_then(|uid| {
                    s.document_sessions
                        .with_untracked(|m| {
                            m.values()
                                .find(|sess| sess.iframe_doc_uid == Some(uid))
                                .map(|sess| sess.edit_revision)
                        })
                        .or_else(|| s.get_unmanaged_edit_revision(uid))
                })
            })
            .unwrap_or(snapshot_edit_rev)
    };

    let mark_origin_dirty = || {
        if let Some(ref orig_id) = originating_doc_id {
            s.set_document_dirty(orig_id, true);
        } else if let Some(uid) = originating_uid {
            s.sync_workbook_dirty_and_revision(Some(name), Some(uid), true, None);
        } else {
            s.is_dirty.set(true);
        }
    };

    match outcome {
        Ok(DevSaveResult::Saved {
            dest_path,
            new_doc_id,
            committed_rev,
        }) => {
            if intent == crate::platform::adapter::SaveOperationIntent::Export {
                // P1 #4676016506: Xuất file (Export) TUYỆT ĐỐI KHÔNG:
                // - Xóa trạng thái dirty của workbook nguồn
                // - Thay thế session tài liệu native đang hoạt động
                // - Thay đổi committed revision của workbook
                // - Cho phép Save & Exit
                // - Xem file export là workbook gốc
                send_saved_to_iframe(GRIDCRAFT_IFRAME_ID, req_id, None, None, None);
                return;
            }

            let actual_file_name = std::path::Path::new(&dest_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(name)
                .to_string();

            let live_edit_rev = lookup_origin_live_edit_rev();
            let mut coord_dirty_for_origin = false;

            if let Some(coord) = coordinator {
                coord.update_value(|c| {
                    let fp = crate::platform::persistence::compute_fingerprint(bytes);
                    c.on_durable_write_completed(req_id, &fp);
                    let coord_matches_origin = c.active_document_id().is_none()
                        || c.active_document_id() == originating_doc_id.as_deref()
                        || c.active_document_id() == Some(new_doc_id.as_str());
                    if coord_matches_origin {
                        coord_dirty_for_origin = c.is_dirty();
                        // P1 #4679604052: Đồng bộ hóa Coordinator với session và đường dẫn mới sau Save As
                        c.on_document_opened(&new_doc_id, &dest_path, &fp);
                        if coord_dirty_for_origin || live_edit_rev > snapshot_edit_rev {
                            c.on_user_edit();
                        }
                    }
                });
            }

            let is_still_dirty = coord_dirty_for_origin || (live_edit_rev > snapshot_edit_rev);

            let cur_active_id = s.active_document_id.get_untracked();
            let should_update_active_signals = cur_active_id.is_none()
                || cur_active_id.as_deref() == originating_doc_id.as_deref()
                || cur_active_id.as_deref() == Some(new_doc_id.as_str());

            if intent == crate::platform::adapter::SaveOperationIntent::SaveAs {
                // P1 #4683798594 & #4684931088: Di chuyển phiên nguồn đã chụp từ ctx sang danh tính mới,
                // tuyệt đối không động vào các phiên workbook khác (ví dụ B khi đang Save As A -> C)
                let old_id = originating_doc_id.as_deref().unwrap_or(&new_doc_id);
                s.migrate_document_session(
                    old_id,
                    crate::micro_apps::gridcraft::state::WorkbookSession {
                        document_id: new_doc_id.clone(),
                        file_path: dest_path.clone(),
                        file_name: actual_file_name.clone(),
                        revision: committed_rev,
                        is_dirty: is_still_dirty,
                        edit_revision: live_edit_rev,
                        iframe_doc_uid: originating_uid,
                    },
                );
            } else {
                s.register_document_session(crate::micro_apps::gridcraft::state::WorkbookSession {
                    document_id: new_doc_id.clone(),
                    file_path: dest_path.clone(),
                    file_name: actual_file_name.clone(),
                    revision: committed_rev,
                    is_dirty: is_still_dirty,
                    edit_revision: live_edit_rev,
                    iframe_doc_uid: originating_uid,
                });
            }

            if should_update_active_signals {
                s.set_document_session(
                    Some(new_doc_id.clone()),
                    Some(dest_path.clone()),
                    committed_rev,
                );
                s.active_file_name.set(Some(actual_file_name.clone()));
            }

            send_saved_to_iframe(
                GRIDCRAFT_IFRAME_ID,
                req_id,
                Some(&actual_file_name),
                Some(&dest_path),
                Some(&new_doc_id),
            );

            let close_req = s.pending_close_request_id.get_untracked();
            s.pending_close_request_id.set(None);
            handle_gridcraft_save_completion(is_still_dirty, close_req, close_manager, app_state);
        }
        Ok(DevSaveResult::AmbiguousCommit {
            dest_path,
            new_doc_id,
            committed_rev,
            message,
        }) => {
            if intent == crate::platform::adapter::SaveOperationIntent::Export {
                // P1 #4676016506 / Finding 4679068393: Xuất file thất bại hoặc ambiguous
                // TUYỆT ĐỐI KHÔNG:
                // - Thay thế session tài liệu native đang hoạt động
                // - Đăng ký export dest_path làm session workbook
                // - Thay đổi committed revision hay active_file_name của workbook
                // - Thay đổi trạng thái dirty của workbook
                s.save_blocked_detail.set(format!(
                    "⚠️ Xuất tệp thất bại (Ambiguous Commit): {message}"
                ));
                s.is_save_blocked_alert.set(true);
                send_save_failed_to_iframe(GRIDCRAFT_IFRAME_ID, &message, Some(req_id));
                if let Some(close_req_id) = s.pending_close_request_id.get_untracked() {
                    s.pending_close_request_id.set(None);
                    if let Some(cm) = close_manager {
                        cm.cancel_close(&close_req_id);
                        cm.error_message
                            .set(Some(format!("Lỗi xuất tệp (Ambiguous Commit): {message}")));
                    }
                }
                return;
            }

            if let Some(coord) = coordinator {
                coord.update_value(|c| {
                    let _ = c.on_durable_write_failed(req_id, &message);
                });
            }
            let actual_file_name = std::path::Path::new(&dest_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(name)
                .to_string();
            let live_edit_rev = lookup_origin_live_edit_rev();

            let cur_active_id = s.active_document_id.get_untracked();
            let should_update_active_signals = cur_active_id.is_none()
                || cur_active_id.as_deref() == originating_doc_id.as_deref()
                || cur_active_id.as_deref() == Some(new_doc_id.as_str());

            let session = crate::micro_apps::gridcraft::state::WorkbookSession {
                document_id: new_doc_id.clone(),
                file_path: dest_path.clone(),
                file_name: actual_file_name.clone(),
                revision: committed_rev,
                is_dirty: true,
                edit_revision: live_edit_rev,
                iframe_doc_uid: originating_uid,
            };

            if intent == crate::platform::adapter::SaveOperationIntent::SaveAs {
                let old_id = originating_doc_id.as_deref().unwrap_or(&new_doc_id);
                s.migrate_document_session(old_id, session);
            } else {
                s.register_document_session(session);
            }

            if should_update_active_signals {
                s.set_document_session(Some(new_doc_id), Some(dest_path), committed_rev);
                s.active_file_name.set(Some(actual_file_name));
            }
            s.is_dirty.set(true);

            s.save_blocked_detail.set(format!(
                "⚠️ Hoán đổi tệp thành công nhưng đồng bộ thư mục cha thất bại (Ambiguous Commit): {message}"
            ));
            s.is_save_blocked_alert.set(true);
            send_save_failed_to_iframe(GRIDCRAFT_IFRAME_ID, &message, Some(req_id));

            if let Some(close_req_id) = s.pending_close_request_id.get_untracked() {
                s.pending_close_request_id.set(None);
                if let Some(cm) = close_manager {
                    cm.cancel_close(&close_req_id);
                    cm.error_message
                        .set(Some(format!("Lỗi ghi đĩa (Ambiguous Commit): {message}")));
                }
            }
        }
        Ok(DevSaveResult::Canceled) => {
            if let Some(coord) = coordinator {
                coord.update_value(|c| {
                    c.on_save_cancelled(req_id);
                });
            }
            if intent != crate::platform::adapter::SaveOperationIntent::Export {
                mark_origin_dirty();
            }
            send_save_failed_to_iframe(GRIDCRAFT_IFRAME_ID, "Đã hủy thao tác lưu", Some(req_id));
            if let Some(close_req_id) = s.pending_close_request_id.get_untracked() {
                s.pending_close_request_id.set(None);
                if let Some(cm) = close_manager {
                    cm.cancel_close(&close_req_id);
                }
            }
        }
        Ok(DevSaveResult::Failed(err)) => {
            if intent != crate::platform::adapter::SaveOperationIntent::Export {
                if let Some(coord) = coordinator {
                    coord.update_value(|c| {
                        let _ = c.on_durable_write_failed(req_id, &err);
                    });
                }
                mark_origin_dirty();
            }
            s.save_blocked_detail.set(format!("❌ Lỗi ghi đĩa: {err}"));
            s.is_save_blocked_alert.set(true);
            send_save_failed_to_iframe(GRIDCRAFT_IFRAME_ID, &err, Some(req_id));
            if let Some(close_req_id) = s.pending_close_request_id.get_untracked() {
                s.pending_close_request_id.set(None);
                if let Some(cm) = close_manager {
                    cm.cancel_close(&close_req_id);
                    cm.error_message.set(Some(format!("Lỗi ghi đĩa: {err}")));
                }
            }
        }
        Err(err) => {
            if intent != crate::platform::adapter::SaveOperationIntent::Export {
                if let Some(coord) = coordinator {
                    coord.update_value(|c| {
                        let _ = c.on_durable_write_failed(req_id, &err);
                    });
                }
                mark_origin_dirty();
            }
            s.save_blocked_detail.set(format!("❌ Lỗi hệ thống: {err}"));
            s.is_save_blocked_alert.set(true);
            send_save_failed_to_iframe(GRIDCRAFT_IFRAME_ID, &err, Some(req_id));
            if let Some(close_req_id) = s.pending_close_request_id.get_untracked() {
                s.pending_close_request_id.set(None);
                if let Some(cm) = close_manager {
                    cm.cancel_close(&close_req_id);
                    cm.error_message.set(Some(format!("Lỗi hệ thống: {err}")));
                }
            }
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
            let pending_save_metadata: std::rc::Rc<
                std::cell::RefCell<std::collections::HashMap<String, (u64, u64)>>,
            > = std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashMap::new()));
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
                                let doc_name = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("name"),
                                )
                                .ok()
                                .and_then(|v| v.as_string());
                                let doc_id = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("doc_id"),
                                )
                                .ok()
                                .and_then(|v| v.as_string());
                                let documents = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("documents"),
                                )
                                .ok()
                                .and_then(|v| v.dyn_into::<js_sys::Array>().ok());

                                // 1. Đồng bộ trạng thái dirty chi tiết của từng workbook nếu mảng documents có mặt
                                let has_documents_snapshot = documents.is_some();
                                let mut live_uids: Vec<u64> = Vec::new();
                                let mut advanced_doc_ids: Vec<String> = Vec::new();
                                if let Some(docs) = documents {
                                    for i in 0..docs.length() {
                                        let item = docs.get(i);
                                        if let Ok(obj) = item.dyn_into::<js_sys::Object>() {
                                            let n = js_sys::Reflect::get(
                                                &obj,
                                                &wasm_bindgen::JsValue::from_str("name"),
                                            )
                                            .ok()
                                            .and_then(|v| v.as_string());
                                            let d = js_sys::Reflect::get(
                                                &obj,
                                                &wasm_bindgen::JsValue::from_str("dirty"),
                                            )
                                            .ok()
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false);
                                            let u = js_sys::Reflect::get(
                                                &obj,
                                                &wasm_bindgen::JsValue::from_str("uid"),
                                            )
                                            .ok()
                                            .and_then(|v| v.as_f64())
                                            .map(|v| v as u64)
                                            .or_else(|| {
                                                js_sys::Reflect::get(
                                                    &obj,
                                                    &wasm_bindgen::JsValue::from_str("doc_uid"),
                                                )
                                                .ok()
                                                .and_then(|v| v.as_f64())
                                                .map(|v| v as u64)
                                            });
                                            let r = js_sys::Reflect::get(
                                                &obj,
                                                &wasm_bindgen::JsValue::from_str("revision"),
                                            )
                                            .ok()
                                            .and_then(|v| v.as_f64())
                                            .map(|v| v as u64);

                                            if let Some(uid_val) = u {
                                                live_uids.push(uid_val);
                                            }
                                            if let Some((matched_doc_id, true)) = s
                                                .sync_workbook_dirty_and_revision_tracked(
                                                    n.as_deref(),
                                                    u,
                                                    d,
                                                    r,
                                                )
                                            {
                                                advanced_doc_ids.push(matched_doc_id);
                                            }
                                        }
                                    }
                                }

                                // 2. Đối soát các workbook đã bị đóng/hủy trong iframe (P2 #4683038858)
                                // Chạy ngay cả khi live_uids rỗng (workbook cuối cùng bị đóng với "Don't Save")
                                if has_documents_snapshot {
                                    s.reconcile_closed_sessions(&live_uids);
                                }

                                // 3. Đồng bộ coordinator và aggregate dirty (P2 #4690136471: chỉ báo edit cho coordinator khi revision thực sự tăng)
                                if dirty {
                                    s.is_dirty.set(true);
                                    let cur_doc = s.active_document_id.get_untracked();
                                    let cur_path = s.active_file_path.get_untracked();
                                    coordinator.update_value(|c| {
                                        sync_coordinator_on_dirty_notification(
                                            c,
                                            dirty,
                                            has_documents_snapshot,
                                            &advanced_doc_ids,
                                            cur_doc.as_deref(),
                                            cur_path.as_deref(),
                                        );
                                    });
                                } else {
                                    s.recompute_aggregate_dirty();
                                    if !s.is_dirty.get_untracked()
                                        && s.pending_save_requests.with_untracked(|m| m.is_empty())
                                    {
                                        coordinator.update_value(|c| c.reset_if_dirty_on_close());
                                    }
                                }
                            }
                            "gridcraft:save_metadata" => {
                                let meta_id = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("id"),
                                )
                                .ok()
                                .and_then(|v| v.as_string());
                                let meta_uid = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("doc_uid"),
                                )
                                .ok()
                                .and_then(|v| v.as_f64())
                                .map(|v| v as u64)
                                .or_else(|| {
                                    js_sys::Reflect::get(
                                        &data,
                                        &wasm_bindgen::JsValue::from_str("uid"),
                                    )
                                    .ok()
                                    .and_then(|v| v.as_f64())
                                    .map(|v| v as u64)
                                });
                                let meta_rev = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("revision"),
                                )
                                .ok()
                                .and_then(|v| v.as_f64())
                                .map(|v| v as u64)
                                .unwrap_or(0);
                                if let (Some(id), Some(uid)) = (meta_id, meta_uid) {
                                    pending_save_metadata
                                        .borrow_mut()
                                        .insert(id, (uid, meta_rev));
                                }
                            }
                            "gridcraft:save" | "gridcraft:export" => {
                                let is_export_msg = msg_type == "gridcraft:export";
                                let name = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("name"),
                                )
                                .ok()
                                .and_then(|v| v.as_string())
                                .unwrap_or_else(|| {
                                    if is_export_msg {
                                        "export.csv".to_string()
                                    } else {
                                        "workbook.xlsx".to_string()
                                    }
                                });
                                let req_id = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("id"),
                                )
                                .ok()
                                .and_then(|v| v.as_string());
                                let incoming_doc_id = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("doc_id"),
                                )
                                .ok()
                                .and_then(|v| v.as_string())
                                .or_else(|| {
                                    js_sys::Reflect::get(
                                        &data,
                                        &wasm_bindgen::JsValue::from_str("document_id"),
                                    )
                                    .ok()
                                    .and_then(|v| v.as_string())
                                });
                                let explicit_intent_str = js_sys::Reflect::get(
                                    &data,
                                    &wasm_bindgen::JsValue::from_str("intent"),
                                )
                                .ok()
                                .and_then(|v| v.as_string());
                                let parsed_intent = if is_export_msg {
                                    Some(crate::platform::adapter::SaveOperationIntent::Export)
                                } else {
                                    match explicit_intent_str.as_deref() {
                                        Some("export") => Some(
                                            crate::platform::adapter::SaveOperationIntent::Export,
                                        ),
                                        Some("save_as") => Some(
                                            crate::platform::adapter::SaveOperationIntent::SaveAs,
                                        ),
                                        Some("save") => Some(
                                            crate::platform::adapter::SaveOperationIntent::Save,
                                        ),
                                        Some(_) => None,
                                        None => {
                                            let save_as_flag = js_sys::Reflect::get(
                                                &data,
                                                &wasm_bindgen::JsValue::from_str("saveAs"),
                                            )
                                            .ok()
                                            .and_then(|v| v.as_bool());
                                            let has_id = js_sys::Reflect::get(
                                                &data,
                                                &wasm_bindgen::JsValue::from_str("id"),
                                            )
                                            .ok()
                                            .and_then(|v| v.as_string())
                                            .is_some();
                                            if save_as_flag == Some(true) {
                                                Some(crate::platform::adapter::SaveOperationIntent::SaveAs)
                                            } else if save_as_flag == Some(false) && has_id {
                                                Some(crate::platform::adapter::SaveOperationIntent::Save)
                                            } else {
                                                None
                                            }
                                        }
                                    }
                                };
                                let intent = match parsed_intent {
                                    Some(i) => i,
                                    None => {
                                        if let Some(close_req_id) =
                                            s.pending_close_request_id.get_untracked()
                                        {
                                            s.pending_close_request_id.set(None);
                                            if let Some(cm) = close_manager {
                                                cm.cancel_close(&close_req_id);
                                                cm.error_message.set(Some("Yêu cầu lưu bị từ chối: Intent không hợp lệ hoặc mơ hồ.".into()));
                                            }
                                        }
                                        send_save_failed_to_iframe(
                                            GRIDCRAFT_IFRAME_ID,
                                            "Yêu cầu lưu bị từ chối: Intent không hợp lệ hoặc mơ hồ.",
                                            req_id.as_deref(),
                                        );
                                        return;
                                    }
                                };
                                let save_as = intent
                                    == crate::platform::adapter::SaveOperationIntent::SaveAs
                                    || intent
                                        == crate::platform::adapter::SaveOperationIntent::Export;

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
                                    // Finding 2, P1 #4679604052, P2 #4682914297 & P1 #4684931088:
                                    // Phân giải chính xác phiên workbook đích bằng metadata, doc_uid, doc_id hoặc name
                                    let meta_pair = req_id.as_deref().and_then(|id| {
                                        pending_save_metadata.borrow_mut().remove(id)
                                    });
                                    let incoming_uid = js_sys::Reflect::get(
                                        &data,
                                        &wasm_bindgen::JsValue::from_str("doc_uid"),
                                    )
                                    .ok()
                                    .and_then(|v| v.as_f64())
                                    .map(|v| v as u64)
                                    .or_else(|| {
                                        js_sys::Reflect::get(
                                            &data,
                                            &wasm_bindgen::JsValue::from_str("uid"),
                                        )
                                        .ok()
                                        .and_then(|v| v.as_f64())
                                        .map(|v| v as u64)
                                    })
                                    .or_else(|| meta_pair.map(|(uid, _)| uid))
                                    .or_else(|| {
                                        req_id.as_deref().and_then(|id| {
                                            if id.starts_with("save-") {
                                                let parts: Vec<&str> = id.split('-').collect();
                                                if parts.len() >= 3 {
                                                    parts[1].parse::<u64>().ok()
                                                } else {
                                                    None
                                                }
                                            } else {
                                                None
                                            }
                                        })
                                    });

                                    let lookup_key = incoming_doc_id.as_deref().unwrap_or(&name);
                                    let resolved_session =
                                        s.resolve_save_session_with_uid(lookup_key, incoming_uid);
                                    let (
                                        target_doc_id,
                                        target_path,
                                        target_rev,
                                        resolved_uid,
                                        resolved_edit_rev,
                                    ) = if let Some(ref sess) = resolved_session {
                                        (
                                            Some(sess.document_id.clone()),
                                            Some(sess.file_path.clone()),
                                            sess.revision,
                                            sess.iframe_doc_uid.or(incoming_uid),
                                            Some(sess.edit_revision),
                                        )
                                    } else if save_as {
                                        (None, None, 0, incoming_uid, None)
                                    } else {
                                        // Tệp lạ không thuộc phiên đã đăng ký -> buộc Save As để bảo vệ file hiện hành
                                        (None, None, 0, incoming_uid, None)
                                    };

                                    let effective_doc_id = target_doc_id
                                        .as_deref()
                                        .or(incoming_doc_id.as_deref())
                                        .unwrap_or("gridcraft-doc");
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
                                    let bytes_clone = bytes.clone();
                                    let req_clone = req_id_str.clone();

                                    let is_export_op = intent
                                        == crate::platform::adapter::SaveOperationIntent::Export;

                                    if !is_export_op {
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
                                    }

                                    // Finding 2 & P1 #4684931088: Chụp ngữ cảnh yêu cầu lưu bất biến tại thời điểm chấp nhận yêu cầu
                                    let snapshot_edit_rev = resolved_edit_rev
                                        .or_else(|| meta_pair.map(|(_, rev)| rev))
                                        .or_else(|| {
                                            resolved_uid
                                                .and_then(|uid| s.get_unmanaged_edit_revision(uid))
                                        })
                                        .unwrap_or_else(|| s.get_edit_revision(effective_doc_id));
                                    let effective_intent = if requires_format_safe_save_as(
                                        intent,
                                        target_path.as_deref(),
                                        &bytes_clone,
                                    ) {
                                        crate::platform::adapter::SaveOperationIntent::SaveAs
                                    } else {
                                        intent
                                    };
                                    let save_ctx = GridCraftSaveRequestContext {
                                        request_id: req_clone.clone(),
                                        operation_intent: effective_intent,
                                        originating_document_id: target_doc_id
                                            .clone()
                                            .or_else(|| incoming_doc_id.clone()),
                                        originating_path: target_path.clone(),
                                        originating_name: name.clone(),
                                        iframe_doc_uid: resolved_uid,
                                        snapshot_edit_revision: snapshot_edit_rev,
                                        base_committed_revision: target_rev,
                                        session_generation: session_gen,
                                    };
                                    if !is_export_op {
                                        s.register_pending_save_request(
                                            &save_ctx.request_id,
                                            save_ctx.originating_document_id.as_deref(),
                                        );
                                    }

                                    wasm_bindgen_futures::spawn_local(async move {
                                        let outcome =
                                            execute_gridcraft_save(&save_ctx, &bytes_clone).await;

                                        handle_gridcraft_save_outcome(
                                            &s,
                                            Some(&coordinator),
                                            outcome,
                                            &save_ctx,
                                            &bytes_clone,
                                            close_manager,
                                            app_state,
                                        );
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

/// Gửi thông báo xác nhận lưu thành công vào GridCraft iframe (Bản vá 0001 confirmed save & P1 #4679604052)
#[cfg(target_arch = "wasm32")]
pub fn send_saved_to_iframe(
    iframe_id: &str,
    req_id: &str,
    new_name: Option<&str>,
    dest_path: Option<&str>,
    new_doc_id: Option<&str>,
) {
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
                        if let Some(n) = new_name {
                            let _ = js_sys::Reflect::set(
                                &obj,
                                &wasm_bindgen::JsValue::from_str("name"),
                                &wasm_bindgen::JsValue::from_str(n),
                            );
                        }
                        if let Some(p) = dest_path {
                            let _ = js_sys::Reflect::set(
                                &obj,
                                &wasm_bindgen::JsValue::from_str("path"),
                                &wasm_bindgen::JsValue::from_str(p),
                            );
                        }
                        if let Some(d) = new_doc_id {
                            let _ = js_sys::Reflect::set(
                                &obj,
                                &wasm_bindgen::JsValue::from_str("doc_id"),
                                &wasm_bindgen::JsValue::from_str(d),
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
pub fn send_saved_to_iframe(
    _iframe_id: &str,
    _req_id: &str,
    _new_name: Option<&str>,
    _dest_path: Option<&str>,
    _new_doc_id: Option<&str>,
) {
}

/// Gửi thông điệp yêu cầu iframe GridCraft tuần tự hóa và lưu tệp chủ động (Stage 4C-0C Task 1)
/// Trả về true nếu đã gửi thành công tới iframe, false nếu iframe không tồn tại trong DOM (Finding 4)
#[cfg(target_arch = "wasm32")]
pub fn send_request_save_to_iframe(iframe_id: &str, req_id: Option<&str>, save_as: bool) -> bool {
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
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(not(target_arch = "wasm32"))]
static MOCK_IFRAME_AVAILABLE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);

#[cfg(not(target_arch = "wasm32"))]
pub fn set_mock_iframe_available(available: bool) {
    MOCK_IFRAME_AVAILABLE.store(available, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn send_request_save_to_iframe(
    _iframe_id: &str,
    _req_id: Option<&str>,
    _save_as: bool,
) -> bool {
    MOCK_IFRAME_AVAILABLE.load(std::sync::atomic::Ordering::SeqCst)
}

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

/// Thực thi quy trình lưu đĩa bảo mật chính thức của GridCraft (Stage 4C-1 Product Cutover & P1 #4684931088)
#[cfg(target_arch = "wasm32")]
pub async fn execute_gridcraft_save(
    ctx: &GridCraftSaveRequestContext,
    bytes: &[u8],
) -> Result<DevSaveResult, String> {
    let name = ctx.originating_name.as_str();
    let file_path = ctx.originating_path.as_deref();
    let document_id = ctx.originating_document_id.as_deref();
    let expected_rev = ctx.base_committed_revision + 1;
    let session_gen = ctx.session_generation;
    let save_as = ctx.operation_intent == crate::platform::adapter::SaveOperationIntent::SaveAs
        || ctx.operation_intent == crate::platform::adapter::SaveOperationIntent::Export;
    let req_id = ctx.request_id.as_str();

    // 1. Regular Save: Có sẵn file_path, document_id đã đăng ký VÀ định dạng nhị phân tương thích 100% với tệp đích (P1 #4689587087)
    // Nếu mở CSV/XLS nhưng snapshot serialize ra XLSX ZIP (PK\x03\x04), TUYỆT ĐỐI không ghi đè tệp CSV/XLS mà bắt buộc Save As sang .xlsx!
    if !save_as
        && let (Some(_path), Some(doc_id)) = (file_path, document_id)
        && validate_save_as_format_compatibility(_path, bytes).is_ok()
    {
        let grant_args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "documentId": doc_id,
            "appInstanceId": "gridcraft-workspace-instance",
            "sessionGeneration": session_gen,
            "requestId": req_id,
            "expectedRevision": expected_rev,
        }))
        .map_err(|e| format!("{e:?}"))?;

        let grant_val = match invoke("request_save_grant", grant_args).await {
            Ok(val) => val,
            Err(e) => {
                return Ok(DevSaveResult::Failed(format!(
                    "Yêu cầu cấp phép ghi đĩa thất bại: {e:?}"
                )));
            }
        };

        let grant: crate::platform::NativeSaveGrantDto =
            match serde_wasm_bindgen::from_value(grant_val) {
                Ok(g) => g,
                Err(e) => {
                    return Ok(DevSaveResult::Failed(format!(
                        "Lỗi bất biến: Grant bị thiếu trường bắt buộc hoặc sai định dạng: {e:?}"
                    )));
                }
            };

        let grant_ctx = crate::platform::SaveRequestContext {
            document_id: doc_id.to_string(),
            app_instance_id: "gridcraft-workspace-instance".to_string(),
            session_generation: session_gen,
            authorized_dest_path: Some(_path.to_string()),
            expected_revision: expected_rev,
            request_id: req_id.to_string(),
        };

        if let Err(err) = crate::platform::validate_native_save_grant(&grant, &grant_ctx) {
            return Ok(DevSaveResult::Failed(format!("Lỗi bất biến: {err}")));
        }

        let save_args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "grantId": grant.grant_id,
            "requestId": grant.request_id,
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
                let err_str = format!("{e:?}");
                if err_str.contains("AmbiguousCommit") {
                    return Ok(DevSaveResult::AmbiguousCommit {
                        dest_path: _path.to_string(),
                        new_doc_id: doc_id.to_string(),
                        committed_rev: expected_rev,
                        message: err_str,
                    });
                }
                return Ok(DevSaveResult::Failed(format!("Lỗi ghi đĩa: {e:?}")));
            }
        }
    }

    // 2. Save As (hoặc Lưu lần đầu) thông qua Native Dialog
    // Finding 3 & P2 #4679604061: Giới hạn định dạng hộp thoại Save As nghiêm ngặt theo chữ ký dữ liệu đã serialize (XLSX, CSV, PDF)
    let (filter_name, extensions) = if bytes.starts_with(b"PK\x03\x04") {
        ("Bảng tính Excel (*.xlsx)", vec!["xlsx"])
    } else if bytes.starts_with(b"%PDF-") {
        ("Tài liệu PDF (*.pdf)", vec!["pdf"])
    } else if name.ends_with(".csv") || std::str::from_utf8(bytes).is_ok() {
        ("Tệp CSV (*.csv)", vec!["csv"])
    } else {
        return Ok(DevSaveResult::Failed(
            "Định dạng dữ liệu không được hỗ trợ".to_string(),
        ));
    };

    let default_save_name = format_safe_default_save_name(name, bytes);
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "documentId": document_id,
        "appInstanceId": "gridcraft-workspace-instance",
        "sessionGeneration": session_gen,
        "defaultName": default_save_name,
        "filterName": filter_name,
        "extensions": extensions,
    }))
    .unwrap_or(wasm_bindgen::JsValue::UNDEFINED);

    match invoke("native_dialog_save_as", args).await {
        Ok(js_res) => {
            if let Ok(Some(handle)) =
                serde_wasm_bindgen::from_value::<Option<crate::platform::SaveAsHandleDto>>(js_res)
            {
                let dest_path = handle.file_path;

                // Finding 3: Xác thực định dạng đích tương thích chữ ký nhị phân
                if let Err(err) = validate_save_as_format_compatibility(&dest_path, bytes) {
                    return Ok(DevSaveResult::Failed(err));
                }

                let grant_ctx = crate::platform::SaveRequestContext {
                    document_id: handle.document_id.clone(),
                    app_instance_id: "gridcraft-workspace-instance".to_string(),
                    session_generation: session_gen,
                    authorized_dest_path: Some(dest_path.clone()),
                    expected_revision: 1,
                    request_id: handle.grant.request_id.clone(),
                };

                if let Err(err) =
                    crate::platform::validate_native_save_grant(&handle.grant, &grant_ctx)
                {
                    return Ok(DevSaveResult::Failed(format!(
                        "Lỗi bất biến Save As: {err}"
                    )));
                }

                let save_args = serde_wasm_bindgen::to_value(&serde_json::json!({
                    "grantId": handle.grant.grant_id,
                    "requestId": handle.grant.request_id,
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
                        return Ok(DevSaveResult::Saved {
                            dest_path,
                            new_doc_id: handle.document_id,
                            committed_rev: rev,
                        });
                    }
                    Err(e) => {
                        let err_str = format!("{e:?}");
                        if err_str.contains("AmbiguousCommit") {
                            return Ok(DevSaveResult::AmbiguousCommit {
                                dest_path,
                                new_doc_id: handle.document_id,
                                committed_rev: 1,
                                message: err_str,
                            });
                        }
                        return Ok(DevSaveResult::Failed(format!("Lỗi ghi đĩa: {e:?}")));
                    }
                }
            } else {
                Ok(DevSaveResult::Canceled)
            }
        }
        Err(e) => Ok(DevSaveResult::Failed(format!("Lỗi hộp thoại: {e:?}"))),
    }
}

/// Đánh giá liệu thao tác Save thường lên `target_path` với payload `bytes` có bắt buộc nâng cấp
/// thành Save As (ví dụ: mở `.csv`/`.xls` nhưng snapshot serialize ra XLSX `PK\x03\x04`) hay không (P1 #4689587087).
pub fn requires_format_safe_save_as(
    intent: crate::platform::adapter::SaveOperationIntent,
    target_path: Option<&str>,
    bytes: &[u8],
) -> bool {
    if intent != crate::platform::adapter::SaveOperationIntent::Save {
        return false;
    }
    match target_path {
        Some(path) => validate_save_as_format_compatibility(path, bytes).is_err(),
        None => true,
    }
}

/// Tên tệp gợi ý an toàn cho hộp thoại Save As khớp định dạng nhị phân thực tế (P1 #4689587087).
pub fn format_safe_default_save_name(name: &str, bytes: &[u8]) -> String {
    let lower = name.to_ascii_lowercase();
    if bytes.starts_with(b"PK\x03\x04") {
        if lower.ends_with(".xlsx") || lower.ends_with(".xlsm") {
            name.to_string()
        } else {
            let stem = std::path::Path::new(name)
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("workbook");
            format!("{stem}.xlsx")
        }
    } else if bytes.starts_with(b"%PDF-") {
        if lower.ends_with(".pdf") {
            name.to_string()
        } else {
            let stem = std::path::Path::new(name)
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("export");
            format!("{stem}.pdf")
        }
    } else if lower.ends_with(".csv") || lower.ends_with(".txt") {
        name.to_string()
    } else {
        let stem = std::path::Path::new(name)
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("export");
        format!("{stem}.csv")
    }
}

/// Xác thực tính tương thích giữa định dạng phần mở rộng của đường dẫn đích và nội dung nhị phân (Finding 3 & P2 #4679604061: XLSX, CSV, PDF)
pub fn validate_save_as_format_compatibility(dest_path: &str, bytes: &[u8]) -> Result<(), String> {
    let dest_lower = dest_path.to_lowercase();
    let is_zip_xlsx = bytes.starts_with(b"PK\x03\x04");
    let is_pdf = bytes.starts_with(b"%PDF-");

    if is_zip_xlsx {
        if !dest_lower.ends_with(".xlsx") && !dest_lower.ends_with(".xlsm") {
            return Err(format!(
                "Xung đột định dạng tệp: Dữ liệu nhị phân là Excel ZIP/OpenXML nhưng tệp đích là '{dest_path}'. Bị từ chối để tránh hỏng dữ liệu."
            ));
        }
    } else if is_pdf {
        if !dest_lower.ends_with(".pdf") {
            return Err(format!(
                "Xung đột định dạng tệp: Dữ liệu nhị phân là PDF nhưng tệp đích là '{dest_path}'. Bị từ chối để tránh hỏng dữ liệu."
            ));
        }
    } else {
        if dest_lower.ends_with(".xlsx") || dest_lower.ends_with(".xlsm") {
            return Err(format!(
                "Xung đột định dạng tệp: Tệp đích là Excel '{dest_path}' nhưng dữ liệu nhị phân không chứa chữ ký PK ZIP hợp lệ."
            ));
        }
        if dest_lower.ends_with(".pdf") {
            return Err(format!(
                "Xung đột định dạng tệp: Tệp đích là PDF '{dest_path}' nhưng dữ liệu nhị phân không chứa chữ ký %PDF- hợp lệ."
            ));
        }
        if !dest_lower.ends_with(".csv") && !dest_lower.ends_with(".txt") {
            return Err(format!(
                "Xung đột định dạng tệp: Tệp đích '{dest_path}' không tương thích với định dạng văn bản/CSV."
            ));
        }
    }
    Ok(())
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
    let ctx = GridCraftSaveRequestContext {
        request_id: req_id.to_string(),
        operation_intent: if save_as {
            crate::platform::adapter::SaveOperationIntent::SaveAs
        } else {
            crate::platform::adapter::SaveOperationIntent::Save
        },
        originating_document_id: document_id.map(str::to_string),
        originating_path: file_path.map(str::to_string),
        originating_name: name.to_string(),
        iframe_doc_uid: None,
        snapshot_edit_revision: 0,
        base_committed_revision: expected_rev.saturating_sub(1),
        session_generation: 1,
    };
    execute_gridcraft_save(&ctx, bytes).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn execute_gridcraft_save(
    _ctx: &GridCraftSaveRequestContext,
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
