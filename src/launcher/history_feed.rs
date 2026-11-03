use leptos::prelude::*;
use crate::app::{TaskHistoryUi, ALL_TOOLS};
use crate::micro_apps::{tool_slug_to_micro_app, MicroAppId};

#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

/// Định dạng thời gian thực thi (milliseconds) thành chuỗi hiển thị thân thiện.
pub fn format_elapsed_time(ms: u64) -> String {
    if ms < 1000 {
        format!("{} ms", ms)
    } else {
        format!("{:.1} s", ms as f64 / 1000.0)
    }
}

/// Trả về nhãn trạng thái và CSS class tương ứng cho bản ghi lịch sử tác vụ.
pub fn format_history_status(status: &str) -> (&'static str, &'static str) {
    match status.to_lowercase().as_str() {
        "success" | "ok" | "done" => ("Thành công", "status-success"),
        "failed" | "error" => ("Thất bại", "status-failed"),
        "running" | "in_progress" => ("Đang chạy", "status-running"),
        _ => ("Đã xong", "status-other"),
    }
}

/// Component hiển thị danh sách lịch sử tác vụ gần đây từ SQLite `task_history`.
///
/// Tự động suy luận `MicroAppId` từ `tool_slug` thông qua hàm mapping tĩnh `tool_slug_to_micro_app`.
/// Khi nhấp vào một bản ghi, kích hoạt callback `on_deep_link(micro_app_id, tool_slug)`.
#[component]
pub fn HistoryFeed(
    /// Callback điều hướng thẳng vào công cụ trong MicroApp tương ứng
    #[prop(into)]
    on_deep_link: Callback<(MicroAppId, String)>,
    /// Tín hiệu danh sách lịch sử tùy chọn (tiện lợi cho unit test hoặc truyền từ ngoài)
    #[prop(optional)]
    history: Option<Signal<Vec<TaskHistoryUi>>>,
) -> impl IntoView {
    // Nếu không có history signal truyền vào, khởi tạo internal signal
    let (internal_history, _set_internal_history) = signal(Vec::<TaskHistoryUi>::new());

    #[cfg(target_arch = "wasm32")]
    if history.is_none() {
        spawn_local(async move {
            #[derive(serde::Serialize)]
            struct GetHistoryArgs {
                limit: i64,
                offset: i64,
            }
            if let Ok(args) = serde_wasm_bindgen::to_value(&GetHistoryArgs { limit: 20, offset: 0 }) {
                let resp = invoke("get_task_history", args).await;
                if let Ok(records) = serde_wasm_bindgen::from_value::<Vec<TaskHistoryUi>>(resp) {
                    _set_internal_history.set(records);
                }
            }
        });
    }

    let records_signal = move || {
        if let Some(h) = history {
            h.get()
        } else {
            internal_history.get()
        }
    };

    let find_tool_name = |slug: &str| -> String {
        ALL_TOOLS
            .iter()
            .find(|t| t.slug == slug)
            .map(|t| t.name.to_string())
            .unwrap_or_else(|| slug.replace('-', " "))
    };

    view! {
        <div class="launcher-history-container">
            <div class="launcher-history-header">
                <div class="launcher-history-title-group">
                    <span class="launcher-history-icon" aria-hidden="true">"📋"</span>
                    <span class="launcher-history-title">"LỊCH SỬ TÁC VỤ GẦN ĐÂY"</span>
                </div>
                <span class="launcher-history-count">
                    {move || format!("{} tác vụ", records_signal().len())}
                </span>
            </div>

            <div class="launcher-history-feed">
                {move || {
                    let items = records_signal();
                    if items.is_empty() {
                        view! {
                            <div class="launcher-history-empty">
                                <div class="launcher-history-empty-icon">"☕"</div>
                                <div class="launcher-history-empty-title">"Chưa có tác vụ nào được ghi nhận"</div>
                                <p class="launcher-history-empty-desc">
                                    "Chọn một Micro-App phía trên hoặc nhấn Ctrl+K để bắt đầu xử lý tài liệu đầu tiên."
                                </p>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="launcher-history-list">
                                {items.into_iter().map(|item| {
                                    let slug_opt = item.tool_slug.clone();
                                    let app_opt = slug_opt.as_deref().and_then(tool_slug_to_micro_app);
                                    let (status_text, status_class) = format_history_status(&item.status);
                                    let elapsed_text = format_elapsed_time(item.elapsed_ms);
                                    let tool_name = slug_opt.as_deref().map(find_tool_name).unwrap_or_else(|| "Tác vụ tổng hợp".to_string());

                                    let on_item_click = {
                                        let slug_clone = slug_opt.clone();
                                        move |_| {
                                            if let (Some(app), Some(slug)) = (app_opt, slug_clone.as_ref()) {
                                                on_deep_link.run((app, slug.clone()));
                                            }
                                        }
                                    };

                                    let has_deep_link = app_opt.is_some() && slug_opt.is_some();

                                    view! {
                                        <div
                                            class=format!(
                                                "launcher-history-item {}",
                                                if has_deep_link { "clickable" } else { "" }
                                            )
                                            role=if has_deep_link { "button" } else { "presentation" }
                                            tabindex=if has_deep_link { "0" } else { "-1" }
                                            title=if has_deep_link { "Mở lại công cụ này" } else { "" }
                                            on:click=on_item_click
                                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                if has_deep_link && (ev.key() == "Enter" || ev.key() == " ") {
                                                    ev.prevent_default();
                                                    if let (Some(app), Some(slug)) = (app_opt, slug_opt.as_ref()) {
                                                        on_deep_link.run((app, slug.clone()));
                                                    }
                                                }
                                            }
                                        >
                                            <div class="launcher-history-item-left">
                                                {if let Some(app) = app_opt {
                                                    view! {
                                                        <span class="launcher-history-app-badge" title=app.name()>
                                                            <span class="app-badge-icon">{app.icon()}</span>
                                                            <span class="app-badge-name">{app.name()}</span>
                                                        </span>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <span class="launcher-history-app-badge neutral">
                                                            <span class="app-badge-icon">"⚡"</span>
                                                            <span class="app-badge-name">"Hệ thống"</span>
                                                        </span>
                                                    }.into_any()
                                                }}
                                                <span class="launcher-history-tool-name">{tool_name}</span>
                                            </div>

                                            <div class="launcher-history-item-right">
                                                <span class=format!("launcher-history-badge {}", status_class)>
                                                    {status_text}
                                                </span>
                                                <span class="launcher-history-time" title=item.created_at.clone()>
                                                    {elapsed_text}
                                                </span>
                                                {if has_deep_link {
                                                    view! {
                                                        <span class="launcher-history-arrow" aria-hidden="true">"→"</span>
                                                    }.into_any()
                                                } else {
                                                    view! { <span class="launcher-history-arrow-placeholder"></span> }.into_any()
                                                }}
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
