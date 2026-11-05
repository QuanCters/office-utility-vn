use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::app::ToolItem;
use crate::shared_ui::keyboard_shortcuts::{use_cancel_shortcut, use_run_shortcut};

/// Một bước (node) thực thi trong chuỗi Workflow DAG.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub tool_slug: String,
    pub name: String,
    pub description: String,
}

/// Mô hình chuỗi Workflow DAG độc lập chứa danh sách các bước tuần tự.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowDag {
    pub id: String,
    pub title: String,
    pub steps: Vec<WorkflowStep>,
}

/// Component WorkflowRunner dùng chung cho các micro-app:
/// - Nhận danh sách `available_tools: &'static [ToolItem]` và CHỈ cho phép chọn công cụ trong danh sách này.
/// - Hiển thị chuỗi DAG pipeline trực quan kèm cổng kết nối giữa các bước.
/// - Cho phép sắp xếp thứ tự (Lên/Xuống), xóa bước và thêm bước mới.
/// - Kích hoạt callback `on_run` khi bấm nút chạy hoặc nhấn tổ hợp phím `Ctrl+Enter`.
#[component]
pub fn WorkflowRunner(
    /// Danh sách công cụ khả dụng của Micro-App hiện tại.
    available_tools: &'static [ToolItem],
    /// Callback khi kích hoạt chạy pipeline DAG.
    #[prop(into)]
    on_run: Callback<WorkflowDag>,
    /// Tiêu đề tùy chỉnh cho chuỗi tự động hóa.
    #[prop(into, optional)]
    title: Option<&'static str>,
    /// Danh sách bước ban đầu tùy chọn.
    #[prop(into, optional)]
    initial_steps: Option<Vec<WorkflowStep>>,
    /// Tín hiệu trạng thái đang chạy tác vụ.
    #[prop(into, optional)]
    is_running: Option<Signal<bool>>,
    /// Thông điệp kết quả hoặc tiến trình của workflow.
    #[prop(into, optional)]
    status_message: Option<Signal<String>>,
    /// Callback hủy chuỗi tác vụ khi đang chạy.
    #[prop(into, optional)]
    on_cancel: Option<Callback<()>>,
) -> impl IntoView {
    let header_title = title.unwrap_or("⚡ Thiết kế Chuỗi Tự động hóa Macro DAG");

    // Khởi tạo các bước mặc định lấy từ available_tools
    let default_steps = initial_steps.unwrap_or_else(|| {
        available_tools
            .iter()
            .take(3)
            .enumerate()
            .map(|(idx, tool)| WorkflowStep {
                id: format!("step-{}", idx + 1),
                tool_slug: tool.slug.to_string(),
                name: tool.name.to_string(),
                description: tool.description.to_string(),
            })
            .collect()
    });

    let (steps, set_steps) = signal(default_steps);
    let (is_picker_open, set_is_picker_open) = signal(false);

    let is_running_sig = is_running.unwrap_or_else(|| Signal::derive(|| false));
    let status_msg_sig = status_message.unwrap_or_else(|| Signal::derive(String::new));

    // Thực thi chuỗi pipeline
    let run_pipeline = move || {
        if is_running_sig.get() {
            return;
        }
        let current_steps = steps.get();
        if current_steps.is_empty() {
            return;
        }
        let dag = WorkflowDag {
            id: format!("dag-{}", current_steps.len()),
            title: header_title.to_string(),
            steps: current_steps,
        };
        on_run.run(dag);
    };

    // Đăng ký phím tắt Ctrl+Enter để chạy
    use_run_shortcut(Callback::new(move |_| run_pipeline()));

    // Đăng ký phím tắt Esc để hủy nếu có callback on_cancel
    if let Some(cancel_cb) = on_cancel {
        use_cancel_shortcut(cancel_cb);
    }

    // Xóa một bước tại chỉ mục index
    let remove_step = move |index: usize| {
        set_steps.update(|s| {
            if index < s.len() {
                s.remove(index);
                // Cập nhật lại id
                for (i, step) in s.iter_mut().enumerate() {
                    step.id = format!("step-{}", i + 1);
                }
            }
        });
    };

    // Di chuyển bước sang trái (lên trên)
    let move_up = move |index: usize| {
        if index > 0 {
            set_steps.update(|s| {
                s.swap(index, index - 1);
                for (i, step) in s.iter_mut().enumerate() {
                    step.id = format!("step-{}", i + 1);
                }
            });
        }
    };

    // Di chuyển bước sang phải (xuống dưới)
    let move_down = move |index: usize| {
        set_steps.update(|s| {
            if index + 1 < s.len() {
                s.swap(index, index + 1);
                for (i, step) in s.iter_mut().enumerate() {
                    step.id = format!("step-{}", i + 1);
                }
            }
        });
    };

    // Thêm một bước từ available_tools
    let add_tool_step = move |tool: &'static ToolItem| {
        set_steps.update(|s| {
            let next_idx = s.len() + 1;
            s.push(WorkflowStep {
                id: format!("step-{}", next_idx),
                tool_slug: tool.slug.to_string(),
                name: tool.name.to_string(),
                description: tool.description.to_string(),
            });
        });
        set_is_picker_open.set(false);
    };

    view! {
        <div class="workflow-workspace shared-workflow-runner">
            <div class="workflow-header">
                <div>
                    <h2 style="font-size: 18px; color: #F8FAFC; margin-bottom: 4px;">
                        {header_title}
                    </h2>
                    <p style="font-size: 12px; color: var(--text-muted);">
                        "Ghép nối các công cụ độc lập thuộc micro-app thành chuỗi Macro DAG tự động xử lý On-Device (Ctrl+Enter để chạy)."
                    </p>
                </div>

                <div style="display: flex; gap: 8px; align-items: center;">
                    <button
                        type="button"
                        class="btn-primary"
                        prop:disabled=move || is_running_sig.get() || steps.with(|s| s.is_empty())
                        on:click=move |_| run_pipeline()
                    >
                        {move || {
                            if is_running_sig.get() {
                                "⏳ Đang chạy chuỗi...".to_string()
                            } else {
                                let count = steps.with(|s| s.len());
                                format!("▶ Chạy Chuỗi Pipeline ({} Bước)", count)
                            }
                        }}
                    </button>
                </div>
            </div>

            // Thông báo kết quả / trạng thái
            {move || {
                let msg = status_msg_sig.get();
                if !msg.is_empty() {
                    view! {
                        <div style="padding: 12px; margin-bottom: 16px; background: rgba(16, 185, 129, 0.1); border: 1px solid rgba(16, 185, 129, 0.3); border-radius: 8px; color: #34D399; font-weight: 600;">
                            {msg}
                        </div>
                    }.into_any()
                } else {
                    view! { <span style="display: none;"></span> }.into_any()
                }
            }}

            // Tuyến đường ống DAG (Pipeline Track)
            <div class="dag-pipeline-track">
                {move || {
                    let current = steps.get();
                    let total = current.len();

                    if current.is_empty() {
                        view! {
                            <div class="dag-empty-placeholder" style="color: var(--text-muted); padding: 24px 32px; border: 1px dashed #334155; border-radius: 8px;">
                                "Chưa có bước nào trong chuỗi. Hãy thêm công cụ bằng nút bên dưới."
                            </div>
                        }.into_any()
                    } else {
                        current
                            .into_iter()
                            .enumerate()
                            .map(|(idx, step)| {
                                let step_id = step.id.clone();
                                let is_first = idx == 0;
                                let is_last = idx + 1 == total;

                                view! {
                                    <div class="dag-step-group" style="display: flex; align-items: center; gap: 12px;">
                                        <div class="dag-node-card">
                                            <div class="dag-node-header">
                                                <span class="dag-node-title">{format!("{}. {}", idx + 1, step.name)}</span>
                                                <span style="font-size: 12px; color: #10B981; font-weight: 600;">{format!("Bước {}", idx + 1)}</span>
                                            </div>

                                            <div class="dag-node-ports">
                                                <div style="font-size: 12px; color: var(--text-muted); line-height: 1.4; margin-bottom: 8px;">
                                                    {step.description}
                                                </div>
                                                <div style="display: flex; gap: 6px;">
                                                    <span class="dag-port-badge in">{format!("ID: {}", step_id)}</span>
                                                    <span class="dag-port-badge out">{step.tool_slug}</span>
                                                </div>
                                            </div>

                                            // Bộ nút điều khiển vị trí & xóa bước
                                            <div class="dag-node-actions" style="display: flex; justify-content: space-between; align-items: center; margin-top: 10px; border-top: 1px solid #334155; padding-top: 6px;">
                                                <div style="display: flex; gap: 4px;">
                                                    <button
                                                        type="button"
                                                        class="btn-secondary dag-reorder-btn"
                                                        style="padding: 2px 6px; font-size: 12px;"
                                                        title="Di chuyển sang trước"
                                                        prop:disabled=is_first
                                                        on:click=move |_| move_up(idx)
                                                    >
                                                        "←"
                                                    </button>
                                                    <button
                                                        type="button"
                                                        class="btn-secondary dag-reorder-btn"
                                                        style="padding: 2px 6px; font-size: 12px;"
                                                        title="Di chuyển sang sau"
                                                        prop:disabled=is_last
                                                        on:click=move |_| move_down(idx)
                                                    >
                                                        "→"
                                                    </button>
                                                </div>

                                                <button
                                                    type="button"
                                                    class="btn-danger dag-remove-btn"
                                                    style="padding: 2px 8px; font-size: 12px;"
                                                    title="Xóa bước này khỏi chuỗi"
                                                    on:click=move |_| remove_step(idx)
                                                >
                                                    "✕ Xóa"
                                                </button>
                                            </div>
                                        </div>

                                        // Mũi tên connector giữa các node
                                        {if !is_last {
                                            view! {
                                                <div class="dag-connector">
                                                    <span>"======>"</span>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <span style="display: none;"></span> }.into_any()
                                        }}
                                    </div>
                                }
                            })
                            .collect::<Vec<_>>()
                            .into_any()
                    }
                }}

                // Nút mở bộ chọn công cụ
                <div style="position: relative; display: inline-block;">
                    <button
                        type="button"
                        class="add-node-btn"
                        title="Thêm công cụ từ danh sách khả dụng của Micro-App"
                        on:click=move |_| set_is_picker_open.update(|v| *v = !*v)
                    >
                        "+ Thêm bước công cụ..."
                    </button>

                    // Panel danh sách công cụ khả dụng
                    {move || {
                        if is_picker_open.get() {
                            view! {
                                <div
                                    class="dag-tool-picker-menu"
                                    style="position: absolute; top: calc(100% + 8px); left: 0; min-width: 280px; max-height: 320px; overflow-y: auto; background: #0F172A; border: 1px solid #334155; border-radius: 8px; box-shadow: 0 10px 25px rgba(0,0,0,0.5); z-index: 50; padding: 6px;"
                                >
                                    <div style="padding: 6px 10px; font-size: 12px; font-weight: 600; color: #CBD5E1; border-bottom: 1px solid #1E293B; margin-bottom: 4px;">
                                        "Chọn công cụ để thêm vào chuỗi:"
                                    </div>
                                    {available_tools.iter().map(|tool| {
                                        view! {
                                            <button
                                                type="button"
                                                class="dag-picker-item"
                                                style="display: block; width: 100%; text-align: left; padding: 8px 10px; background: transparent; border: none; border-radius: 6px; cursor: pointer; color: #F8FAFC; transition: background 0.15s ease;"
                                                on:click=move |_| add_tool_step(tool)
                                            >
                                                <div style="font-size: 12px; font-weight: 600;">{tool.name}</div>
                                                <div style="font-size: 12px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                                    {tool.description}
                                                </div>
                                            </button>
                                        }
                                    }).collect::<Vec<_>>() }
                                </div>
                            }.into_any()
                        } else {
                            view! { <span style="display: none;"></span> }.into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
