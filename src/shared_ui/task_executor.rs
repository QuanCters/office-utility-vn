use leptos::prelude::*;
use crate::launcher::format_elapsed_time;
use crate::shared_ui::keyboard_shortcuts::{use_cancel_shortcut, use_run_shortcut};
use crate::shared_ui::progress_bar::ProgressBar;
use crate::shared_ui::workflow_runner::WorkflowDag;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

/// Mở thư mục đầu ra mặc định (Downloads) qua Tauri IPC command `open_output_folder`.
pub fn open_output_folder_ipc() {
    #[cfg(target_arch = "wasm32")]
    leptos::task::spawn_local(async move {
        let empty = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
        let _ = invoke("open_output_folder", empty).await;
    });
}

/// Hủy một tác vụ đang chạy theo `task_id` qua Tauri IPC command `cancel_task`.
pub fn cancel_task_ipc(task_id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let tid = task_id.to_string();
        leptos::task::spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "task_id": tid })).unwrap();
            let _ = invoke("cancel_task", args).await;
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = task_id;
}

/// Thực thi chuỗi tự động hóa Workflow DAG qua Tauri IPC `execute_workflow_pipeline`:
/// 1. Tự động chuyển đổi và lưu định nghĩa Workflow vào SQLite qua `save_workflow`
/// 2. Kích hoạt lệnh IPC `execute_workflow_pipeline` với danh sách tệp đầu vào
/// 3. Cập nhật tín hiệu tiến độ và kết quả thực thi
pub fn execute_workflow_ipc(
    dag: WorkflowDag,
    input_files: Vec<String>,
    is_running: RwSignal<bool>,
    status_message: RwSignal<String>,
) {
    is_running.set(true);
    let step_count = dag.steps.len();
    let title = dag.title.clone();
    status_message.set(format!(
        "Đang khởi chạy chuỗi tự động hóa '{}' ({} bước On-Device)...",
        title, step_count
    ));

    #[cfg(target_arch = "wasm32")]
    leptos::task::spawn_local(async move {
        let backend_dag = serde_json::json!({
            "id": dag.id,
            "name": dag.title,
            "description": Some(format!("Chuỗi quy trình gồm {} bước", step_count)),
            "nodes": dag.steps.iter().enumerate().map(|(i, s)| {
                serde_json::json!({
                    "id": s.id,
                    "tool_slug": s.tool_slug,
                    "label": s.name,
                    "options": {},
                    "input_mimes": [],
                    "output_mimes": [],
                    "position_x": (i as f64) * 160.0,
                    "position_y": 100.0,
                })
            }).collect::<Vec<_>>(),
            "edges": if dag.steps.len() > 1 {
                (0..dag.steps.len() - 1).map(|i| {
                    serde_json::json!({
                        "id": format!("edge-{}-{}", i, i + 1),
                        "source_node_id": dag.steps[i].id,
                        "target_node_id": dag.steps[i + 1].id,
                    })
                }).collect::<Vec<_>>()
            } else {
                vec![]
            }
        });

        // 1. Lưu workflow vào SQLite để execute_workflow_pipeline có thể tìm thấy theo workflow_id
        let save_args = match serde_wasm_bindgen::to_value(&serde_json::json!({ "workflow": backend_dag })) {
            Ok(v) => v,
            Err(e) => {
                is_running.set(false);
                status_message.set(format!("Lỗi chuẩn bị quy trình: {e}"));
                return;
            }
        };
        let _ = invoke("save_workflow", save_args).await;

        // 2. Thực thi chuỗi workflow pipeline
        let exec_args = match serde_wasm_bindgen::to_value(&serde_json::json!({
            "workflow_id": dag.id,
            "input_paths": input_files,
            "output_directory": null,
        })) {
            Ok(v) => v,
            Err(e) => {
                is_running.set(false);
                status_message.set(format!("Lỗi tham số pipeline: {e}"));
                return;
            }
        };

        let resp = invoke("execute_workflow_pipeline", exec_args).await;
        if let Ok(res_obj) = serde_wasm_bindgen::from_value::<serde_json::Value>(resp.clone()) {
            let status = res_obj.get("status").and_then(|v| v.as_str()).unwrap_or("SUCCESS");
            if status == "FAILED" {
                let reason = res_obj.get("failed_items")
                    .and_then(|arr| arr.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|item| item.get("reason"))
                    .and_then(|r| r.as_str())
                    .unwrap_or("Thực thi chuỗi thất bại");
                status_message.set(format!("❌ Chuỗi tự động hóa thất bại: {reason}"));
            } else {
                let elapsed_ms = res_obj.get("elapsed_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                status_message.set(format!(
                    "✓ Đã hoàn tất thành công chuỗi quy trình '{}' ({} bước, {} ms)!",
                    title, step_count, elapsed_ms
                ));
            }
        } else {
            let err_msg = serde_wasm_bindgen::from_value::<String>(resp)
                .unwrap_or_else(|_| "Lỗi thực thi chuỗi workflow pipeline".to_string());
            status_message.set(format!("❌ {err_msg}"));
        }
        is_running.set(false);
    });

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = input_files;
        is_running.set(false);
        status_message.set(format!(
            "✓ Đã hoàn tất thành công chuỗi quy trình '{}' ({} bước)!",
            title, step_count
        ));
    }
}

/// Component TaskExecutor dùng chung cho cả 4 Micro-Apps:
/// - Nút "🚀 Thực thi tác vụ" (phím tắt `Ctrl+Enter`)
/// - Nút "✕ Hủy tác vụ" (phím tắt `Esc`)
/// - Thanh tiến trình `ProgressBar` real-time
/// - Bảng hiển thị kết quả (thời gian chạy, kết quả đầu ra, lỗi)
/// - Nút "📂 Mở thư mục đầu ra" (`open_output_folder`)
#[component]
pub fn TaskExecutor(
    /// Slug của công cụ cần thực thi
    #[prop(into)]
    tool_slug: Signal<String>,
    /// Danh sách tệp đầu vào (từ workspace dùng chung của micro-app)
    #[prop(into, optional)]
    input_files: Option<Signal<Vec<String>>>,
    /// Hàm xây dựng options JSON tùy biến cho công cụ
    #[prop(into, optional)]
    options_builder: Option<Callback<(), serde_json::Value>>,
    /// Trạng thái đang chạy từ bên ngoài tùy chọn (được đồng bộ với micro-app)
    #[prop(into, optional)]
    external_is_running: Option<RwSignal<bool>>,
    /// Callback khi tác vụ thành công
    #[prop(into, optional)]
    on_success: Option<Callback<serde_json::Value>>,
    /// Callback khi tác vụ thất bại
    #[prop(into, optional)]
    on_error: Option<Callback<String>>,
    /// Tùy chọn nhãn nút thực thi (mặc định: "🚀 Thực thi tác vụ")
    #[prop(into, optional)]
    run_label: Option<&'static str>,
) -> impl IntoView {
    let btn_label = run_label.unwrap_or("🚀 Thực thi tác vụ");

    let is_running = RwSignal::new(false);
    let active_task_id = RwSignal::new(None::<String>);
    let progress = RwSignal::new(0.0f32);
    let progress_msg = RwSignal::new(String::new());
    let result_msg = RwSignal::new(None::<String>);
    let error_msg = RwSignal::new(None::<String>);
    let elapsed_ms = RwSignal::new(None::<u64>);

    let input_files_sig = input_files.unwrap_or_else(|| Signal::derive(Vec::new));

    // Xử lý hủy tác vụ
    let do_cancel = move || {
        if is_running.get() {
            if let Some(tid) = active_task_id.get() {
                cancel_task_ipc(&tid);
            }
            is_running.set(false);
            if let Some(ext) = external_is_running {
                ext.set(false);
            }
            progress.set(0.0);
            progress_msg.set("Đã dừng tác vụ".to_string());
            error_msg.set(Some("Tác vụ đã bị người dùng hủy bỏ.".to_string()));
        }
    };

    // Đăng ký phím tắt Esc để hủy
    use_cancel_shortcut(Callback::new(move |_| do_cancel()));

    // Xử lý thực thi dispatch_tool_task
    let do_dispatch = move || {
        if is_running.get() {
            return;
        }

        let slug = tool_slug.get();
        let backend_slug = if slug == "universal-file-converter-hub" {
            "universal-converter".to_string()
        } else {
            slug.clone()
        };

        let files = input_files_sig.get();
        let opts = options_builder
            .map(|cb| cb.run(()))
            .unwrap_or_else(|| serde_json::json!({}));

        #[cfg(target_arch = "wasm32")]
        let task_id_str = format!("task-{}", js_sys::Date::now() as u64);
        #[cfg(not(target_arch = "wasm32"))]
        let task_id_str = "task-local-1".to_string();

        active_task_id.set(Some(task_id_str.clone()));
        is_running.set(true);
        if let Some(ext) = external_is_running {
            ext.set(true);
        }
        error_msg.set(None);
        result_msg.set(None);
        elapsed_ms.set(None);
        progress.set(30.0);
        progress_msg.set(format!("Đang thực thi công cụ '{}' On-Device...", slug));

        #[cfg(target_arch = "wasm32")]
        leptos::task::spawn_local(async move {
            let request_payload = serde_json::json!({
                "task_id": task_id_str,
                "tool_slug": backend_slug,
                "input_paths": files,
                "output_directory": null,
                "options": opts
            });

            let args = match serde_wasm_bindgen::to_value(&serde_json::json!({ "request": request_payload })) {
                Ok(v) => v,
                Err(e) => {
                    is_running.set(false);
                    if let Some(ext) = external_is_running {
                        ext.set(false);
                    }
                    error_msg.set(Some(format!("Lỗi tham số yêu cầu: {e}")));
                    return;
                }
            };

            let resp = invoke("dispatch_tool_task", args).await;
            if let Ok(res_obj) = serde_wasm_bindgen::from_value::<serde_json::Value>(resp.clone()) {
                let status = res_obj.get("status").and_then(|v| v.as_str()).unwrap_or("SUCCESS");
                let failed_items = res_obj.get("failed_items").and_then(|v| v.as_array());

                if status == "FAILED"
                    || (failed_items.is_some()
                        && !failed_items.unwrap().is_empty()
                        && res_obj
                            .get("output_paths")
                            .and_then(|v| v.as_array())
                            .map_or(true, |a| a.is_empty()))
                {
                    let err_reason = failed_items
                        .and_then(|arr| arr.first())
                        .and_then(|item| item.get("reason"))
                        .and_then(|r| r.as_str())
                        .unwrap_or("Không thể hoàn tất tác vụ với định dạng này.");
                    error_msg.set(Some(err_reason.to_string()));
                    result_msg.set(None);
                    progress.set(0.0);
                    progress_msg.set("Thực thi thất bại".to_string());
                    if let Some(cb) = on_error {
                        cb.run(err_reason.to_string());
                    }
                } else {
                    let ms = res_obj.get("elapsed_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                    elapsed_ms.set(Some(ms));
                    if let Some(first_output) = res_obj
                        .get("output_paths")
                        .and_then(|v| v.as_array())
                        .and_then(|outputs| outputs.first())
                        .and_then(|v| v.as_str())
                    {
                        result_msg.set(Some(first_output.to_string()));
                    } else {
                        result_msg.set(Some("Tác vụ xử lý hoàn tất thành công!".to_string()));
                    }
                    error_msg.set(None);
                    progress.set(100.0);
                    progress_msg.set("Đã hoàn thành xuất sắc 100%".to_string());
                    if let Some(cb) = on_success {
                        cb.run(res_obj);
                    }
                }
            } else {
                let err_str = serde_wasm_bindgen::from_value::<String>(resp)
                    .unwrap_or_else(|_| "Lỗi thực thi tác vụ: định dạng tệp không tương thích hoặc bị từ chối.".to_string());
                error_msg.set(Some(err_str.clone()));
                result_msg.set(None);
                progress.set(0.0);
                progress_msg.set("Có lỗi phát sinh".to_string());
                if let Some(cb) = on_error {
                    cb.run(err_str);
                }
            }

            is_running.set(false);
            if let Some(ext) = external_is_running {
                ext.set(false);
            }
        });

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (&backend_slug, &files, &opts, &on_error);
            progress.set(100.0);
            progress_msg.set("Đã hoàn thành xuất sắc 100%".to_string());
            result_msg.set(Some("Tác vụ xử lý hoàn tất thành công (Native simulation)".to_string()));
            elapsed_ms.set(Some(12));
            is_running.set(false);
            if let Some(ext) = external_is_running {
                ext.set(false);
            }
            if let Some(cb) = on_success {
                cb.run(serde_json::json!({
                    "status": "SUCCESS",
                    "output_paths": ["result_simulated"],
                    "elapsed_ms": 12
                }));
            }
        }
    };

    // Đăng ký phím tắt Ctrl+Enter để chạy
    use_run_shortcut(Callback::new(move |_| do_dispatch()));

    view! {
        <div class="task-executor-bar">
            <div class="task-executor-controls">
                <button
                    type="button"
                    class="btn-primary task-run-btn"
                    disabled=move || is_running.get()
                    on:click=move |_| do_dispatch()
                >
                    <span class="btn-icon">"🚀"</span>
                    <span>{btn_label}</span>
                    <kbd class="kbd-hint">"Ctrl+Enter"</kbd>
                </button>

                {move || if is_running.get() {
                    view! {
                        <button
                            type="button"
                            class="btn-danger task-cancel-btn"
                            title="Hủy tác vụ đang chạy (phím Esc)"
                            on:click=move |_| do_cancel()
                        >
                            <span>"✕ Hủy tác vụ"</span>
                            <kbd class="kbd-hint">"Esc"</kbd>
                        </button>
                    }.into_any()
                } else {
                    view! { <span style="display: none;"></span> }.into_any()
                }}

                <button
                    type="button"
                    class="btn-secondary task-open-output-btn"
                    title="Mở thư mục xuất tệp mặc định (Downloads)"
                    on:click=move |_| open_output_folder_ipc()
                >
                    <span class="btn-icon">"📂"</span>
                    <span>"Mở thư mục đầu ra"</span>
                </button>
            </div>

            // Thanh tiến trình hiển thị khi đang chạy hoặc vừa chạy xong
            {move || if is_running.get() || progress.get() > 0.0 {
                view! {
                    <ProgressBar
                        progress=Signal::derive(move || progress.get())
                        status=Signal::derive(move || progress_msg.get())
                        on_cancel=Callback::new(move |_| do_cancel())
                    />
                }.into_any()
            } else {
                view! { <span style="display: none;"></span> }.into_any()
            }}

            // Khối hiển thị kết quả hoặc thông báo lỗi
            {move || {
                if let Some(err) = error_msg.get() {
                    view! {
                        <div class="task-executor-result error">
                            <div class="result-header">
                                <span class="result-icon">"⚠️"</span>
                                <span class="result-title">"Thực thi thất bại"</span>
                            </div>
                            <div class="result-body">{err}</div>
                        </div>
                    }.into_any()
                } else if let Some(res) = result_msg.get() {
                    let elapsed_label = elapsed_ms.get()
                        .map(|ms| format!(" ({})", format_elapsed_time(ms)))
                        .unwrap_or_default();

                    view! {
                        <div class="task-executor-result success">
                            <div class="result-header">
                                <span class="result-icon">"✓"</span>
                                <span class="result-title">{format!("Hoàn tất thành công{}", elapsed_label)}</span>
                            </div>
                            <div class="result-body selectable-text">{res}</div>
                            <div class="result-actions">
                                <button
                                    type="button"
                                    class="btn-secondary btn-sm"
                                    on:click=move |_| open_output_folder_ipc()
                                >
                                    "📂 Mở thư mục đầu ra"
                                </button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span style="display: none;"></span> }.into_any()
                }
            }}
        </div>
    }
}
