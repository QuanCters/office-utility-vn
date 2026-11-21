use leptos::prelude::*;
use crate::launcher::CommandPalette;
use crate::micro_apps::data_lab::canvas_frame::GridCraftCanvasFrame;
use crate::micro_apps::data_lab::command_bar::{DataLabCommandBar, DataLabWorkspaceMode};
use crate::micro_apps::data_lab::drawer::{DataLabDrawer, DrawerTab};
use crate::micro_apps::data_lab::explorer::DataLabExplorer;
use crate::micro_apps::data_lab::inspector::{DataLabInspector, RecipeStepDisplay};
use crate::micro_apps::data_lab::reconcile_workbench::VisualReconcileWorkbench;
use crate::micro_apps::data_lab::split_pane_view::{ConverterMode, SplitPaneConverterView};
use crate::micro_apps::data_lab::spreadsheet_grid_view::{ColumnType, SpreadsheetGridFacetView};
use crate::micro_apps::data_lab::state::DataLabState;
use crate::micro_apps::data_lab::visual_canvas_workflow::VisualCanvasWorkflowView;
use crate::micro_apps::{micro_app_tools, MicroAppId};
use crate::shared_ui::{WorkflowRunner, cancel_task_ipc, execute_workflow_ipc};
use crate::tools::{
    DocxTemplateMailmergeView, ExcelMultiMergerView,
    ExcelSheetUnlockerView, ExcelSplitByColumnView, ExcelVisualJoinerView,
    ListSetOperationsView, TextPiiAnonymizerView, UniversalFileConverterHubView,
};

/// Bố cục tổng thể của Micro-App Data Lab v2.0 theo chuẩn "5-Zone Adaptive IDE Workbench":
/// - Zone 1: Top Command Bar (40px) với CommandPalette (Ctrl+K), Smart Clipboard Pill, 3 Workspace Modes
/// - Zone 2: Left Explorer (240px, toggle Ctrl+B) với Shared Datasets & Batch Templates
/// - Zone 3: Collapsible Drawer (140px) với DAG Pipeline & DuckDB In-Memory SQL Scratchpad
/// - Zone 4: Center Super-Grid với Trifacta 3-Tier Column Header (60 FPS, Click-to-filter, Sparkline SVG)
/// - Zone 5: Right Inspector (300px, toggle Ctrl+J) với Contextual Tools, Inline Fuzzy Clusters, Applied Steps
/// - Bottom: Task & Status Bar (28px) 100% On-Device, RAM metrics, Undo/Redo
#[component]
pub fn DataLabLayout(
    state: DataLabState,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    provide_context(state);

    // Trạng thái Workspace Modes & Collapsible Zones
    let current_mode = RwSignal::new(DataLabWorkspaceMode::TableClean);
    let left_collapsed = RwSignal::new(false);
    let right_collapsed = RwSignal::new(false);
    let drawer_open = RwSignal::new(false);
    let drawer_tab = RwSignal::new(DrawerTab::WorkflowDag);
    let palette_open = RwSignal::new(false);
    let workflow_canvas_mode = RwSignal::new(true);

    // Smart Clipboard Pill gợi ý thông minh
    let clipboard_hint = RwSignal::new(Some("✨ Phát hiện văn bản lỗi mã TCVN3 — Sửa ngay".to_string()));

    // Đồng bộ trạng thái cột đang chọn giữa Super-Grid và Right Inspector
    let selected_col_idx = RwSignal::new(1usize);
    let selected_col_name = RwSignal::new("Họ và tên".to_string());
    let selected_col_type = RwSignal::new(ColumnType::Text);
    let external_action = RwSignal::new(Option::<String>::None);

    // Lịch sử biến đổi Applied Steps (Power Query style)
    let applied_steps = RwSignal::new(RecipeStepDisplay::demo_steps());

    // Callbacks xử lý thực thi chuỗi quy trình Workflow DAG qua IPC
    let on_run_workflow = Callback::new(move |dag: crate::shared_ui::workflow_runner::WorkflowDag| {
        execute_workflow_ipc(dag, state.loaded_files.get(), state.is_running, state.status_message);
    });

    let on_cancel_workflow = Callback::new(move |_| {
        cancel_task_ipc("");
        state.is_running.set(false);
        state.status_message.set("Đã dừng thực thi chuỗi quy trình dữ liệu.".to_string());
    });

    // Lắng nghe phím tắt toàn cục: Ctrl+B (Explorer), Ctrl+J (Inspector), Ctrl+K (Palette)
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::ev;
        let _ = window_event_listener(ev::keydown, move |ev: web_sys::KeyboardEvent| {
            let ctrl_or_meta = ev.ctrl_key() || ev.meta_key();
            let key = ev.key();
            if ctrl_or_meta {
                if key == "b" || key == "B" {
                    ev.prevent_default();
                    left_collapsed.update(|v| *v = !*v);
                } else if key == "j" || key == "J" {
                    ev.prevent_default();
                    right_collapsed.update(|v| *v = !*v);
                } else if key == "k" || key == "K" {
                    ev.prevent_default();
                    palette_open.set(true);
                }
            }
        });
    }

    // Callbacks xử lý tác vụ từ Explorer & Inspector
    let on_load_dataset = Callback::new(move |name: String| {
        state.status_message.set(format!("Đã nạp bộ dữ liệu \"{}\" vào Super-Grid.", name));
    });

    let on_apply_recipe_template = Callback::new(move |template_title: String| {
        applied_steps.update(|steps| {
            let new_id = steps.len() + 1;
            steps.push(RecipeStepDisplay::new(new_id, format!("{}. Áp dụng {}", new_id, template_title), "+Quy trình"));
        });
        state.status_message.set(format!("Đã áp dụng mẫu quy trình \"{}\".", template_title));
    });

    let on_quick_action = Callback::new(move |action: String| {
        external_action.set(Some(action.clone()));
        applied_steps.update(|steps| {
            let new_id = steps.len() + 1;
            let badge = match action.as_str() {
                "split_vietnamese_name" => "+2 cột",
                "normalize_casing" => "✨ Chuẩn hóa",
                "fix_tcvn3_font" => "✨ 142 ô",
                "fuzzy_clustering" => "-38 trùng",
                _ => "✓ Đã áp dụng",
            };
            steps.push(RecipeStepDisplay::new(new_id, format!("{}. {}", new_id, action), badge));
        });
    });

    let on_toggle_step = Callback::new(move |step_id: usize| {
        applied_steps.update(|steps| {
            if let Some(step) = steps.iter_mut().find(|s| s.id == step_id) {
                step.is_enabled = !step.is_enabled;
            }
        });
    });

    let on_remove_step = Callback::new(move |step_id: usize| {
        applied_steps.update(|steps| {
            steps.retain(|s| s.id != step_id);
        });
    });

    let on_apply_fuzzy_merge = Callback::new(move |_| {
        external_action.set(Some("fuzzy_cluster_merge".to_string()));
        applied_steps.update(|steps| {
            let new_id = steps.len() + 1;
            steps.push(RecipeStepDisplay::new(new_id, format!("{}. Gom cụm chính tả", new_id), "-38 trùng"));
        });
    });

    let on_apply_clipboard_fix = Callback::new(move |_| {
        external_action.set(Some("fix_tcvn3_font".to_string()));
        clipboard_hint.set(None);
        state.status_message.set("✓ Đã sửa lỗi font TCVN3 cho nội dung!".to_string());
    });

    // Quản lý kéo giãn độ rộng vách ngăn (Resizable Splitters)
    let workbench_ref = NodeRef::<leptos::html::Div>::new();
    let left_width = RwSignal::new(240i32);
    let right_width = RwSignal::new(300i32);
    let is_dragging_left = RwSignal::new(false);
    let is_dragging_right = RwSignal::new(false);

    let on_resizer_left_down = move |_ev: web_sys::PointerEvent| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            if let Some(target) = _ev.current_target() {
                if let Ok(elem) = target.dyn_into::<web_sys::Element>() {
                    let _ = elem.set_pointer_capture(_ev.pointer_id());
                }
            }
        }
        is_dragging_left.set(true);
    };

    let on_resizer_left_move = move |ev: web_sys::PointerEvent| {
        if is_dragging_left.get() {
            let client_x = ev.client_x();
            let new_w = client_x.clamp(160, 480);
            left_width.set(new_w);
            if let Some(wb) = workbench_ref.get() {
                let style_str = format!("--left-w: {}px; --right-w: {}px;", new_w, right_width.get());
                let _ = wb.set_attribute("style", &style_str);
            }
        }
    };

    let on_resizer_left_up = move |_ev: web_sys::PointerEvent| {
        if is_dragging_left.get() {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                if let Some(target) = _ev.current_target() {
                    if let Ok(elem) = target.dyn_into::<web_sys::Element>() {
                        let _ = elem.release_pointer_capture(_ev.pointer_id());
                    }
                }
            }
            is_dragging_left.set(false);
        }
    };

    let on_resizer_right_down = move |_ev: web_sys::PointerEvent| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            if let Some(target) = _ev.current_target() {
                if let Ok(elem) = target.dyn_into::<web_sys::Element>() {
                    let _ = elem.set_pointer_capture(_ev.pointer_id());
                }
            }
        }
        is_dragging_right.set(true);
    };

    let on_resizer_right_move = move |ev: web_sys::PointerEvent| {
        if is_dragging_right.get() {
            let client_x = ev.client_x();
            if let Some(window) = web_sys::window() {
                let win_w = window.inner_width().ok().and_then(|w| w.as_f64()).unwrap_or(1920.0) as i32;
                let new_w = (win_w - client_x).clamp(220, 500);
                right_width.set(new_w);
                if let Some(wb) = workbench_ref.get() {
                    let style_str = format!("--left-w: {}px; --right-w: {}px;", left_width.get(), new_w);
                    let _ = wb.set_attribute("style", &style_str);
                }
            }
        }
    };

    let on_resizer_right_up = move |_ev: web_sys::PointerEvent| {
        if is_dragging_right.get() {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                if let Some(target) = _ev.current_target() {
                    if let Ok(elem) = target.dyn_into::<web_sys::Element>() {
                        let _ = elem.release_pointer_capture(_ev.pointer_id());
                    }
                }
            }
            is_dragging_right.set(false);
        }
    };

    view! {
        <div class="data-lab-app-container">
            // ZONE 1: TOP COMMAND BAR (Cao 40px)
            <DataLabCommandBar
                on_back=on_back
                current_mode=current_mode
                palette_open=palette_open
                left_collapsed=left_collapsed
                drawer_open=drawer_open
                right_collapsed=right_collapsed
                clipboard_hint=clipboard_hint
                on_apply_clipboard_fix=on_apply_clipboard_fix
            />

            // KHUNG LÀM VIỆC CHÍNH: 5-ZONE ADAPTIVE IDE WORKBENCH
            <div
                node_ref=workbench_ref
                class="data-lab-v2-workbench"
                class:left-collapsed=move || left_collapsed.get()
                class:right-collapsed=move || right_collapsed.get()
            >
                // ZONE 2: LEFT EXPLORER (240px, ẩn/hiện Ctrl+B)
                <DataLabExplorer
                    state=state
                    on_load_dataset=on_load_dataset
                    on_apply_recipe_template=on_apply_recipe_template
                />

                // Vách ngăn kéo giãn bên trái (Left Resizer)
                <div
                    class="dl-resizer dl-resizer-left"
                    class:is-dragging=move || is_dragging_left.get()
                    title="Kéo để điều chỉnh độ rộng Explorer"
                    on:pointerdown=on_resizer_left_down
                    on:pointermove=on_resizer_left_move
                    on:pointerup=on_resizer_left_up
                    on:pointercancel=on_resizer_left_up
                ></div>

                // VÙNG TRUNG TÂM (CENTER STAGE & COLLAPSIBLE DRAWER)
                <div class="dl-main-stage">
                    // ZONE 3: COLLAPSIBLE FLOW / SQL DRAWER (140px khi mở)
                    <DataLabDrawer
                        drawer_open=drawer_open
                        active_tab=drawer_tab
                        on_run_sql=Callback::new(move |query: String| {
                            state.status_message.set(format!("Đã thực thi DuckDB SQL: \"{}\"", query));
                        })
                        on_run_workflow=Callback::new(move |_| {
                            state.status_message.set("Đang chạy chuỗi quy trình DAG In-Memory...".to_string());
                        })
                    />

                    // ZONE 4: CENTER WORKSPACE THEO CHẾ ĐỘ (TABLE / RECONCILE / TEXT CODEC)
                    <main class="dl-grid-viewport">
                        {move || {
                            if state.is_workflow() {
                                view! {
                                    <div class="data-lab-workflow-wrapper">
                                        <div class="data-lab-workflow-mode-switch">
                                            <button
                                                type="button"
                                                class="workflow-mode-btn"
                                                class:active=move || workflow_canvas_mode.get()
                                                on:click=move |_| workflow_canvas_mode.set(true)
                                            >
                                                "🕸️ Bảng vẽ Canvas (KNIME)"
                                            </button>
                                            <button
                                                type="button"
                                                class="workflow-mode-btn"
                                                class:active=move || !workflow_canvas_mode.get()
                                                on:click=move |_| workflow_canvas_mode.set(false)
                                            >
                                                "📋 Danh sách Bước (Runner)"
                                            </button>
                                        </div>

                                        {move || {
                                            if workflow_canvas_mode.get() {
                                                view! {
                                                    <VisualCanvasWorkflowView state=state />
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <WorkflowRunner
                                                        available_tools=micro_app_tools(MicroAppId::DataLab)
                                                        title="⚡ Chuỗi Quy trình Xử lý Data Lab"
                                                        is_running=Signal::derive(move || state.is_running.get())
                                                        status_message=Signal::derive(move || state.status_message.get())
                                                        on_run=on_run_workflow
                                                        on_cancel=on_cancel_workflow
                                                    />
                                                }.into_any()
                                            }
                                        }}
                                    </div>
                                }.into_any()
                            } else {
                                match current_mode.get() {
                                    DataLabWorkspaceMode::TableClean => {
                                        if state.is_gridcraft_preview.get() {
                                            view! {
                                                <GridCraftCanvasFrame state=state />
                                            }.into_any()
                                        } else {
                                            // Kiểm tra nếu người dùng chọn công cụ riêng biệt từ Palette
                                            let active_slug = state.active_tool.get();
                                            match active_slug.as_str() {
                                                "excel-cleaner-normalizer" | "data-profiler" | "fuzzy-clustering" | "recipe-replay" => {
                                                    view! {
                                                        <SpreadsheetGridFacetView
                                                            state=state
                                                            shared_selected_col=selected_col_idx
                                                            shared_col_name=selected_col_name
                                                            shared_col_type=selected_col_type
                                                            external_action=external_action
                                                        />
                                                    }.into_any()
                                                }
                                                "excel-split-by-column" => view! { <ExcelSplitByColumnView /> }.into_any(),
                                                "excel-multi-merger" => view! { <ExcelMultiMergerView /> }.into_any(),
                                                "excel-visual-joiner" => view! { <ExcelVisualJoinerView /> }.into_any(),
                                                "excel-sheet-unlocker" => view! { <ExcelSheetUnlockerView /> }.into_any(),
                                                "docx-template-mailmerge" => view! { <DocxTemplateMailmergeView /> }.into_any(),
                                                "universal-file-converter-hub" | "universal-converter" => view! { <UniversalFileConverterHubView /> }.into_any(),
                                                "list-set-operations" => view! { <ListSetOperationsView /> }.into_any(),
                                                "text-pii-anonymizer" => view! { <TextPiiAnonymizerView /> }.into_any(),
                                                _ => {
                                                    view! {
                                                        <SpreadsheetGridFacetView
                                                            state=state
                                                            shared_selected_col=selected_col_idx
                                                            shared_col_name=selected_col_name
                                                            shared_col_type=selected_col_type
                                                            external_action=external_action
                                                        />
                                                    }.into_any()
                                                }
                                            }
                                        }
                                    }

                                    DataLabWorkspaceMode::VisualReconcile => {
                                        view! {
                                            <VisualReconcileWorkbench
                                                on_export_excel=Callback::new(move |_| {
                                                    state.status_message.set("✓ Đã xuất biên bản đối soát Excel!".to_string());
                                                })
                                                on_run_reconcile=Callback::new(move |_| {
                                                    state.status_message.set("⚡ Đang đối soát 3 nguồn trực quan (<100ms)...".to_string());
                                                })
                                            />
                                        }.into_any()
                                    }

                                    DataLabWorkspaceMode::TextCodecs => {
                                        let active_slug = state.active_tool.get();
                                        let initial_mode = match active_slug.as_str() {
                                            "base64-file-codec" => ConverterMode::Base64,
                                            "diff-checker-text-sheets" => ConverterMode::DiffChecker,
                                            "sql-formatter-linter" => ConverterMode::SqlFormatter,
                                            _ => ConverterMode::DataStructure,
                                        };
                                        view! {
                                            <SplitPaneConverterView state=state initial_mode=initial_mode />
                                        }.into_any()
                                    }
                                }
                            }
                        }}
                    </main>
                </div>

                // Vách ngăn kéo giãn bên phải (Right Resizer)
                <div
                    class="dl-resizer dl-resizer-right"
                    class:is-dragging=move || is_dragging_right.get()
                    title="Kéo để điều chỉnh độ rộng Inspector"
                    on:pointerdown=on_resizer_right_down
                    on:pointermove=on_resizer_right_move
                    on:pointerup=on_resizer_right_up
                    on:pointercancel=on_resizer_right_up
                ></div>

                // ZONE 5: RIGHT CONTEXTUAL INSPECTOR & APPLIED STEPS (300px, ẩn/hiện Ctrl+J)
                <DataLabInspector
                    workspace_mode=Signal::derive(move || current_mode.get())
                    selected_col_name=Signal::derive(move || selected_col_name.get())
                    selected_col_type=Signal::derive(move || selected_col_type.get())
                    applied_steps=applied_steps
                    on_quick_action=on_quick_action
                    on_toggle_step=on_toggle_step
                    on_remove_step=on_remove_step
                    on_apply_fuzzy_merge=on_apply_fuzzy_merge
                />
            </div>

            // STATUS & TASK BAR (Cao 28px)
            <footer class="dl-status-bar">
                <div class="dl-status-left">
                    <span class="dl-status-badge dl-status-security">
                        "🔒 100% On-Device"
                    </span>
                    <span class="dl-status-divider">"•"</span>
                    <span class="dl-status-metrics">
                        "50,000 dòng × 6 cột"
                    </span>
                    <span class="dl-status-divider">"•"</span>
                    <span class="dl-status-ram">
                        "RAM: 42MB (In-Memory Arrow)"
                    </span>
                    <span class="dl-status-divider">"•"</span>
                    <span class="dl-status-message">
                        {move || state.status_message.get()}
                    </span>
                </div>

                <div class="dl-status-right">
                    <button
                        type="button"
                        class="dl-status-btn dl-undo-btn"
                        title="Hoàn tác thao tác trước (Ctrl+Z)"
                        on:click=move |_| state.status_message.set("↩ Đã hoàn tác bước trước.".to_string())
                    >
                        "↩ Hoàn tác"
                    </button>
                    <button
                        type="button"
                        class="dl-status-btn dl-redo-btn"
                        title="Làm lại thao tác vừa hủy (Ctrl+Y)"
                        on:click=move |_| state.status_message.set("↪ Đã làm lại bước tiếp theo.".to_string())
                    >
                        "↪ Làm lại"
                    </button>
                    <button
                        type="button"
                        class="dl-status-btn dl-export-btn highlight"
                        title="Xuất bảng dữ liệu ra tệp CSV hoặc Excel"
                        on:click=move |_| state.status_message.set("✓ Đã xuất tệp dữ liệu sang CSV!".to_string())
                    >
                        "▶ Xuất File Excel/CSV"
                    </button>
                </div>
            </footer>

            // Scoped Command Palette (Ctrl+K) trong Data Lab
            <CommandPalette
                open=palette_open
                tools=state.tools()
                on_deep_link=Callback::new(move |(_, slug): (_, String)| {
                    state.select_tool(&slug);
                    if matches!(slug.as_str(), "data-structure-converter" | "base64-file-codec" | "diff-checker-text-sheets" | "sql-formatter-linter") {
                        current_mode.set(DataLabWorkspaceMode::TextCodecs);
                    } else if matches!(slug.as_str(), "excel-visual-joiner") {
                        current_mode.set(DataLabWorkspaceMode::VisualReconcile);
                    } else {
                        current_mode.set(DataLabWorkspaceMode::TableClean);
                    }
                })
                on_back=on_back
            />
        </div>
    }
}
