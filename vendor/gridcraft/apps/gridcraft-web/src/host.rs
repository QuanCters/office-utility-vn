//! Host modes: a page that runs GridCraft (as an iframe, or in its own document) opens workbooks
//! in it and stores what it saves instead of the browser downloading. The messages and the two
//! transports are `craft-web-host`'s (storytold/craft-libs), the same contract as the other Craft
//! apps' bridges, with the `gridcraft:` prefix. This module is GridCraft's part of it.
//!
//! - `?host=parent` (an `<iframe>`, such as a Nextcloud app) talks to the same-origin parent over
//!   `postMessage`; `data-host="page"` on the canvas (the page's own document, no iframe) uses
//!   DOM `CustomEvent`s on the canvas instead. `?host=parent` wins when both are present. With no
//!   parent, or a parent on another origin, there is no bridge: the normal app with downloads.
//! - `gridcraft:ready` once the app is listening; send a file now. GridCraft reads bytes, so it
//!   says `file: false` and a host sends `bytes`.
//! - `gridcraft:open` (from the host): `{ name, bytes }`. It opens like a workbook picked with
//!   File › Open or dropped on the window; a file over [`MAX_OPEN_BYTES`] is refused before it is
//!   copied.
//! - `gridcraft:save` (to the host): `{ name, saveAs, bytes }` for every file GridCraft would
//!   otherwise download: Save and Save As (an .xlsx workbook) and Export as PDF. The host stores
//!   it and reports failures itself. GridCraft doesn't tell the host which saves make a file the
//!   document, so `saveAs` is always `false`.
//! - `gridcraft:dirty` whenever unsaved changes appear or go away, `gridcraft:failed` when the
//!   app couldn't start.
//!
//! In both host modes the page starts with an empty workbook instead of a sample, saves and PDF
//! exports are handed to the page rather than downloaded, and `?author=` names the user for new
//! notes and comments.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use craft_web_host::web::{FileData, Host, Message};
use gridcraft_engine::model::Workbook;
use gridcraft_ui_egui::{Inbox, Services, SheetApp};
use wasm_bindgen::JsCast as _;
use web_sys::js_sys;

use crate::hostmode::open_name;

/// The app id in the host messages (`gridcraft:ready`).
const APP: &str = "gridcraft";

/// Saved files that count as a workbook: the document is then clean under the name it was saved as.
const WORKBOOK_EXTS: &[&str] = &["xlsx"];

/// The largest workbook taken from the host page, refused before it is copied.
const MAX_OPEN_BYTES: u64 = 512 << 20;

/// The longest user name taken from `?author=`.
const MAX_AUTHOR_CHARS: usize = 200;

struct PendingSave {
    doc_uid: u64,
    revision: u64,
    snapshot: Arc<Workbook>,
    name: String,
}

#[derive(Clone, Debug)]
struct HostSaveIntent {
    correlation_id: Option<String>,
    save_as: bool,
}

#[derive(Clone)]
pub struct Bridge {
    host: Rc<Host>,
    /// Names of the files handed to the host since the last frame (unconfirmed mode).
    saved: Rc<RefCell<Vec<String>>>,
    confirmed: bool,
    next_save_id: Rc<Cell<u64>>,
    pending_saves: Rc<RefCell<HashMap<String, PendingSave>>>,
    confirmed_saved_ids: Rc<RefCell<Vec<String>>>,
    last_saved_revisions: Rc<RefCell<HashMap<u64, u64>>>,
    pending_downloads: Rc<RefCell<Vec<(String, Vec<u8>)>>>,
    current_open_id: Rc<RefCell<Option<String>>>,
    failed_save_happened: Rc<Cell<bool>>,
    pending_save_intents: Rc<RefCell<Vec<HostSaveIntent>>>,
}

impl Bridge {
    /// The bridge for this page, or `None` when GridCraft isn't hosted: no `?host=parent` and no
    /// `data-host="page"` on the canvas, an unframed `?host=parent` page (it stays the normal app
    /// with downloads), or a parent on another origin.
    pub fn detect(canvas: &web_sys::HtmlCanvasElement) -> Option<Self> {
        let confirmed = web_sys::window()
            .and_then(|w| w.location().search().ok())
            .map(|q| {
                let s = web_sys::UrlSearchParams::new_with_str(&q).ok();
                s.as_ref().and_then(|p| p.get("save")).as_deref() == Some("confirmed")
            })
            .unwrap_or(false);
        Some(Self {
            host: Rc::new(Host::connect(APP, canvas)?),
            saved: Rc::default(),
            confirmed,
            next_save_id: Rc::new(Cell::new(1)),
            pending_saves: Rc::default(),
            confirmed_saved_ids: Rc::default(),
            last_saved_revisions: Rc::default(),
            pending_downloads: Rc::default(),
            current_open_id: Rc::default(),
            failed_save_happened: Rc::new(Cell::new(false)),
            pending_save_intents: Rc::default(),
        })
    }

    /// Send saved and exported files to the host instead of downloading them, and hear files
    /// from it.
    pub fn connect(&self, services: &mut Services, inbox: Inbox, ctx: egui::Context) {
        self.listen(inbox, ctx);
        let bridge = self.clone();
        services.download = Some(Box::new(move |name: &str, bytes: &[u8]| {
            if bridge.confirmed {
                bridge.pending_downloads.borrow_mut().push((name.to_string(), bytes.to_vec()));
            } else if let Err(e) = bridge.post_save(name, bytes) {
                log::error!("couldn't hand {name} to the page: {e}");
            }
        }));
        if self.confirmed {
            services.open_async = Some(Box::new(move || {
                if let Some(w) = web_sys::window()
                    && let Ok(Some(parent)) = w.parent()
                {
                    let obj = js_sys::Object::new();
                    let _ = js_sys::Reflect::set(
                        &obj,
                        &wasm_bindgen::JsValue::from_str("type"),
                        &wasm_bindgen::JsValue::from_str("gridcraft:request_open"),
                    );
                    let _ = parent.post_message(&obj, "*");
                }
            }));

            let cur_open_succeed = self.current_open_id.clone();
            services.open_succeeded = Some(Box::new(move |name: &str| {
                if let Some(w) = web_sys::window()
                    && let Ok(Some(parent)) = w.parent()
                {
                    let obj = js_sys::Object::new();
                    let _ = js_sys::Reflect::set(&obj, &wasm_bindgen::JsValue::from_str("type"), &wasm_bindgen::JsValue::from_str("gridcraft:open_succeeded"));
                    let _ = js_sys::Reflect::set(&obj, &wasm_bindgen::JsValue::from_str("name"), &wasm_bindgen::JsValue::from_str(name));
                    if let Some(ref id) = *cur_open_succeed.borrow() {
                        let _ = js_sys::Reflect::set(&obj, &wasm_bindgen::JsValue::from_str("open_id"), &wasm_bindgen::JsValue::from_str(id));
                    }
                    let _ = parent.post_message(&obj, "*");
                }
            }));

            let cur_open_fail = self.current_open_id.clone();
            services.open_failed = Some(Box::new(move |name: &str, error: &str| {
                if let Some(w) = web_sys::window()
                    && let Ok(Some(parent)) = w.parent()
                {
                    let obj = js_sys::Object::new();
                    let _ = js_sys::Reflect::set(&obj, &wasm_bindgen::JsValue::from_str("type"), &wasm_bindgen::JsValue::from_str("gridcraft:open_failed"));
                    let _ = js_sys::Reflect::set(&obj, &wasm_bindgen::JsValue::from_str("name"), &wasm_bindgen::JsValue::from_str(name));
                    let _ = js_sys::Reflect::set(&obj, &wasm_bindgen::JsValue::from_str("error"), &wasm_bindgen::JsValue::from_str(error));
                    if let Some(ref id) = *cur_open_fail.borrow() {
                        let _ = js_sys::Reflect::set(&obj, &wasm_bindgen::JsValue::from_str("open_id"), &wasm_bindgen::JsValue::from_str(id));
                    }
                    let _ = parent.post_message(&obj, "*");
                }
            }));
        }
    }

    /// `?author=`: the host's name for the user signs new notes and comments.
    pub fn set_author(app: &mut SheetApp, author: &str) {
        let author: String = author.trim().chars().take(MAX_AUTHOR_CHARS).collect();
        if !author.is_empty() {
            app.session.prefs.user_name = author;
        }
    }

    pub fn post_ready(&self) {
        let _ = self.host.post_ready(env!("CARGO_PKG_VERSION"), false);
    }

    pub fn post_failed(&self, error: &str) {
        let _ = self.host.post_failed(error, None);
    }

    /// After each frame: a workbook handed to the host this frame counts as saved under the
    /// name it was saved as, and the host hears when unsaved changes appear or go away (the
    /// crate does not repeat a state it already reported).
    pub fn sync(&self, app: &mut SheetApp) {
        if !self.confirmed {
            let saved = std::mem::take(&mut *self.saved.borrow_mut());
            if let Some(name) = saved.iter().rev().find(|name| is_workbook(name))
                && let Some(doc) = app.session.active_mut()
            {
                doc.saved = doc.wb.clone();
                if doc.path.is_none() {
                    doc.title = name.clone();
                }
            }
            self.host.post_dirty(app.session.documents().iter().any(|d| d.is_dirty()));
            return;
        }

        if self.failed_save_happened.take() {
            app.pending_deferred_action = None;
        }

        // --- Host-initiated save intents (Stage 4C-0C) ---
        let save_intents = std::mem::take(&mut *self.pending_save_intents.borrow_mut());
        for intent in save_intents {
            if app.session.active().is_some() {
                app.commit_edit(0, 0, false, false);
                app.store_view_state();

                if let Some(doc) = app.session.active() {
                    let raw_name = doc.display_title();
                    let name = if raw_name.ends_with(".xlsx") {
                        raw_name
                    } else {
                        format!("{raw_name}.xlsx")
                    };

                    let id_num = self.next_save_id.get();
                    self.next_save_id.set(id_num + 1);
                    let id = intent.correlation_id.unwrap_or_else(|| format!("save-{}-{}", doc.uid, id_num));
                    let snapshot = doc.wb.clone();
                    let revision = doc.revision;
                    let doc_uid = doc.uid;
                    let is_save_as = intent.save_as;

                    let bytes = match gridcraft_engine::io::save_bytes(&snapshot, &name) {
                        Ok(b) => b,
                        Err(e) => {
                            log::error!("couldn't serialize confirmed workbook snapshot '{name}': {e}");
                            let _ = self.host.post_failed(&format!("Lỗi tuần tự hóa workbook '{name}': {e}"), None);
                            continue;
                        }
                    };

                    self.pending_saves.borrow_mut().insert(
                        id.clone(),
                        PendingSave {
                            doc_uid,
                            revision,
                            snapshot,
                            name: file_name(&name),
                        },
                    );
                    if let Err(e) = self.host.post_save(&name, &bytes, Some(&id), is_save_as) {
                        log::error!("couldn't hand confirmed {name} ({id}) to the page: {e}");
                    }
                }
            }
        }

        // --- Confirmed save mode ---
        let downloads = std::mem::take(&mut *self.pending_downloads.borrow_mut());
        for (name, downloaded_bytes) in downloads {
            let is_wb = is_workbook(&name);
            if is_wb && let Some(doc) = app.session.active() {
                let id_num = self.next_save_id.get();
                self.next_save_id.set(id_num + 1);
                let id = format!("save-{}-{}", doc.uid, id_num);
                let snapshot = doc.wb.clone();
                let revision = doc.revision;
                let doc_uid = doc.uid;
                // Bất biến Task 2 & 3 (Stage 2C Closure): Mảng bytes gửi xuống host BẮT BUỘC
                // được tuần tự hóa trực tiếp từ chính snapshot được chụp.
                // TUYỆT ĐỐI KHÔNG fallback về downloaded_bytes nếu thất bại, vì downloaded_bytes
                // có thể là phiên bản cũ, vi phạm tính đồng nhất snapshot/bytes.
                let is_save_as = app.dialog.as_ref().is_some_and(|d| d.name == "saveAs")
                    || app.session.journal.last().is_some_and(|(cmd, _)| cmd == "file.saveAs");
                let bytes = match gridcraft_engine::io::save_bytes(&snapshot, &name) {
                    Ok(b) => b,
                    Err(e) => {
                        log::error!("couldn't serialize confirmed workbook snapshot '{name}': {e}");
                        let _ = self.host.post_failed(&format!("Lỗi tuần tự hóa workbook '{name}': {e}"), None);
                        continue;
                    }
                };
                self.pending_saves.borrow_mut().insert(
                    id.clone(),
                    PendingSave {
                        doc_uid,
                        revision,
                        snapshot,
                        name: file_name(&name),
                    },
                );
                if let Err(e) = self.host.post_save(&name, &bytes, Some(&id), is_save_as) {
                    log::error!("couldn't hand confirmed {name} ({id}) to the page: {e}");
                }
            } else if let Err(e) = self.host.post_save(&name, &downloaded_bytes, None, false) {
                log::error!("couldn't hand {name} to the page: {e}");
            }
        }

        let confirmed_ids = std::mem::take(&mut *self.confirmed_saved_ids.borrow_mut());
        for id in confirmed_ids {
            let mut pending_saves = self.pending_saves.borrow_mut();
            if let Some(pending) = pending_saves.remove(&id) {
                let mut last_revs = self.last_saved_revisions.borrow_mut();
                let last_rev = last_revs.get(&pending.doc_uid).copied().unwrap_or(0);
                if pending.revision >= last_rev {
                    last_revs.insert(pending.doc_uid, pending.revision);
                    if let Some(i) = app.session.document_index(pending.doc_uid) {
                        let cur = app.session.active_index();
                        if cur == i {
                            if let Some(doc) = app.session.active_mut() {
                                doc.saved = pending.snapshot.clone();
                                if doc.path.is_none() {
                                    doc.title = pending.name;
                                }
                            }
                        } else {
                            app.session.set_active(i);
                            if let Some(doc) = app.session.active_mut() {
                                doc.saved = pending.snapshot.clone();
                                if doc.path.is_none() {
                                    doc.title = pending.name;
                                }
                            }
                            app.session.set_active(cur);
                        }
                    }
                    // Stage 4C-0B Task 2: Resume deferred action if one is pending for this document
                    if let Some(action) = app.pending_deferred_action.take() {
                        if action.doc_uid == pending.doc_uid {
                            if let Some(active_doc) = app.session.active() {
                                if active_doc.uid == pending.doc_uid && std::sync::Arc::ptr_eq(&active_doc.wb, &pending.snapshot) {
                                    match action.intent {
                                        gridcraft_ui_egui::DeferredActionIntent::Close => {
                                            app.run_or_alert("file.close", serde_json::json!({"force": true, "document": true}));
                                            if let Some(w) = web_sys::window()
                                                && let Ok(Some(parent)) = w.parent()
                                            {
                                                let obj = js_sys::Object::new();
                                                let _ = js_sys::Reflect::set(&obj, &wasm_bindgen::JsValue::from_str("type"), &wasm_bindgen::JsValue::from_str("gridcraft:close_confirmed"));
                                                let _ = parent.post_message(&obj, "*");
                                            }
                                        }
                                        gridcraft_ui_egui::DeferredActionIntent::Open => {
                                            app.run_or_alert("file.close", serde_json::json!({"force": true, "document": true}));
                                            if let Some(start) = &app.services.open_async {
                                                start();
                                            }
                                        }
                                        gridcraft_ui_egui::DeferredActionIntent::New { params } => {
                                            app.run_or_alert("file.close", serde_json::json!({"force": true, "document": true}));
                                            let mut p = params.clone();
                                            p["force"] = serde_json::json!(true);
                                            app.run_or_alert("file.new", p);
                                        }
                                    }
                                } else {
                                    log::warn!("Live workbook edited while saving; preserving edits and cancelling deferred action");
                                }
                            }
                        }
                    }
                    // Task 6: Dọn dẹp các pending save cũ hơn hoặc bằng revision đã chốt
                    pending_saves.retain(|_, p| {
                        !(p.doc_uid == pending.doc_uid && p.revision <= pending.revision)
                    });
                } else {
                    log::warn!(
                        "ignoring stale save acknowledgment for id {id}: revision {} < last saved revision {last_rev}",
                        pending.revision
                    );
                }
            } else {
                log::warn!("received unexpected or duplicate save acknowledgment for id {id}");
            }
        }

        self.host.post_dirty(app.session.documents().iter().any(|d| d.is_dirty()));
    }

    fn post_save(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let name = file_name(path);
        self.host.post_save(&name, bytes, None, false)?;
        self.saved.borrow_mut().push(name);
        Ok(())
    }

    fn listen(&self, inbox: Inbox, ctx: egui::Context) {
        let confirmed = self.confirmed;
        let confirmed_saved_ids = self.confirmed_saved_ids.clone();
        let pending_saves_for_fail = self.pending_saves.clone();
        let app_ctx_for_open = ctx.clone();
        let app_ctx_for_saved = ctx.clone();
        let app_ctx_for_fail = ctx.clone();
        let inbox_for_listen = inbox.clone();
        self.host.listen(move |message| match message {
            Message::Open { name, data } => {
                deliver_open(&inbox_for_listen, &app_ctx_for_open, &name, &data);
            }
            Message::Saved { id } => {
                if confirmed {
                    confirmed_saved_ids.borrow_mut().push(id);
                    app_ctx_for_saved.request_repaint();
                }
            }
            _ => {}
        });
        if confirmed && let Some(win) = web_sys::window() {
            let failed_flag = self.failed_save_happened.clone();
            let cur_open_store = self.current_open_id.clone();
            let app_ctx_for_msgs = ctx.clone();
            let inbox_for_msgs = inbox.clone();
            let save_intents_store = self.pending_save_intents.clone();
            let app_ctx_for_intents = ctx.clone();
            let on_fail = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::MessageEvent)>::new(
                move |event: web_sys::MessageEvent| {
                    let data = event.data();
                    if let Some(t) = js_sys::Reflect::get(&data, &wasm_bindgen::JsValue::from_str("type"))
                        .ok()
                        .and_then(|v| v.as_string())
                    {
                        if t == "gridcraft:failed" || t == "gridcraft:save_failed" {
                            failed_flag.set(true);
                            if let Some(id) = js_sys::Reflect::get(&data, &wasm_bindgen::JsValue::from_str("id"))
                                .ok()
                                .and_then(|v| v.as_string())
                            {
                                pending_saves_for_fail.borrow_mut().remove(&id);
                                app_ctx_for_fail.request_repaint();
                            }
                        } else if t == "gridcraft:open" {
                            let open_id = js_sys::Reflect::get(&data, &wasm_bindgen::JsValue::from_str("open_id"))
                                .ok()
                                .and_then(|v| v.as_string());
                            *cur_open_store.borrow_mut() = open_id;
                            // Ghi nhận open_id để gửi xác nhận ACK, không push đúp nội dung vào inbox
                            // vì self.host.listen đã xử lý mở tệp duy nhất một lần.
                            app_ctx_for_msgs.request_repaint();
                        } else if t == "gridcraft:request_save" {
                            let save_id = js_sys::Reflect::get(&data, &wasm_bindgen::JsValue::from_str("id"))
                                .ok()
                                .and_then(|v| v.as_string())
                                .or_else(|| {
                                    js_sys::Reflect::get(&data, &wasm_bindgen::JsValue::from_str("correlation_id"))
                                        .ok()
                                        .and_then(|v| v.as_string())
                                });
                            let save_as = js_sys::Reflect::get(&data, &wasm_bindgen::JsValue::from_str("saveAs"))
                                .ok()
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            save_intents_store.borrow_mut().push(HostSaveIntent {
                                correlation_id: save_id,
                                save_as,
                            });
                            app_ctx_for_intents.request_repaint();
                        }
                    }
                },
            );
            let _ = win.add_event_listener_with_callback("message", on_fail.as_ref().unchecked_ref());
            on_fail.forget();
        }
    }
}

/// The file an open message carries, pushed onto the inbox for the next frame, like a dropped
/// file. A host's file is untrusted input: an oversized one is refused before it is copied into
/// the app's memory (the data's own length, not the host's claim).
fn deliver_open(inbox: &Inbox, ctx: &egui::Context, name: &str, data: &FileData) {
    let FileData::Bytes(array) = data else {
        log::warn!("host file {name} came as a File; GridCraft reads bytes only; not opened");
        return;
    };
    if data.size() > MAX_OPEN_BYTES as f64 {
        log::warn!("host file is {} bytes, over the {} MiB host open limit; not opened", data.size(), MAX_OPEN_BYTES >> 20);
        return;
    }
    inbox.lock().unwrap_or_else(|e| e.into_inner()).push((open_name(name), array.to_vec()));
    ctx.request_repaint();
}

fn is_workbook(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, ext)| WORKBOOK_EXTS.iter().any(|e| ext.eq_ignore_ascii_case(e)))
}

fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string())
}
