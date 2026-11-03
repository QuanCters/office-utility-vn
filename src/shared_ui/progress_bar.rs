use leptos::prelude::*;
use crate::shared_ui::keyboard_shortcuts::use_cancel_shortcut;

/// Component hiển thị tiến trình task thực thi từ Tauri IPC channel,
/// hiển thị trạng thái chi tiết, thanh phần trăm động và hỗ trợ nút hủy tác vụ hoặc phím `Esc`.
#[component]
pub fn ProgressBar(
    /// Tiến trình từ 0.0 -> 1.0 (hoặc 0 -> 100).
    #[prop(into)]
    progress: Signal<f32>,
    /// Thông điệp trạng thái chi tiết (ví dụ: "Đang trích xuất trang 3/10...").
    #[prop(into)]
    status: Signal<String>,
    /// Callback hủy tác vụ tùy chọn khi người dùng bấm nút Hủy hoặc nhấn Esc.
    #[prop(into, optional)]
    on_cancel: Option<Callback<()>>,
) -> impl IntoView {
    if let Some(cancel_cb) = on_cancel {
        use_cancel_shortcut(cancel_cb);
    }

    let normalized_percent = move || {
        let p = progress.get();
        let val = if p <= 1.0 { p * 100.0 } else { p };
        val.clamp(0.0, 100.0)
    };

    let width_style = move || {
        format!("width: {:.1}%", normalized_percent())
    };

    let percent_label = move || {
        format!("{:.0}%", normalized_percent())
    };

    view! {
        <div class="progress-container shared-progress-bar">
            <div class="progress-track" role="progressbar" aria-valuenow=move || normalized_percent() as u32 aria-valuemin="0" aria-valuemax="100">
                <div class="progress-bar" style=width_style></div>
            </div>
            <div class="progress-meta">
                <span class="progress-status" title=move || status.get()>
                    {move || {
                        let s = status.get();
                        if s.trim().is_empty() {
                            "Đang xử lý trên thiết bị...".to_string()
                        } else {
                            s
                        }
                    }}
                </span>
                <span class="progress-percentage">{percent_label}</span>
            </div>

            {move || {
                if let Some(cancel_cb) = on_cancel {
                    view! {
                        <div class="progress-cancel-row" style="display: flex; justify-content: flex-end; margin-top: 6px;">
                            <button
                                type="button"
                                class="btn-danger btn-cancel-task"
                                title="Hủy tác vụ đang chạy (phím Esc)"
                                on:click=move |_| cancel_cb.run(())
                            >
                                <span>"✕ Hủy tác vụ (Esc)"</span>
                            </button>
                        </div>
                    }.into_any()
                } else {
                    view! { <span style="display: none;"></span> }.into_any()
                }
            }}
        </div>
    }
}
