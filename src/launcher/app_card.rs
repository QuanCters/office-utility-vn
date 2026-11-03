use leptos::prelude::*;
use crate::micro_apps::{MicroAppId, MicroAppMetadata};

/// Card đồ họa lớn cho 4 micro-app độc lập:
/// - Icon nổi bật (🏢, 📄, 🎨, 📊)
/// - Tiêu đề & huy hiệu số lượng công cụ
/// - Mô tả vắn tắt nghiệp vụ
/// - Hiệu ứng chuyển động mượt mà (smooth hover transition)
/// Khi người dùng nhấp chọn card, kích hoạt callback `on_select(micro_app_id)`.
#[component]
pub fn AppCard(
    /// Định danh MicroApp tương ứng
    app_id: MicroAppId,
    /// Callback được kích hoạt khi chọn MicroApp
    #[prop(into)]
    on_select: Callback<MicroAppId>,
) -> impl IntoView {
    let meta: MicroAppMetadata = app_id.metadata();

    let card_class_modifier = match app_id {
        MicroAppId::VnOffice => "launcher-card-vn-office",
        MicroAppId::PdfStudio => "launcher-card-pdf-studio",
        MicroAppId::MediaStudio => "launcher-card-media-studio",
        MicroAppId::DataLab => "launcher-card-data-lab",
    };

    view! {
        <div
            class=format!("launcher-app-card {}", card_class_modifier)
            role="button"
            tabindex="0"
            title=format!("Mở không gian {}", meta.name)
            on:click=move |_| on_select.run(app_id)
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "Enter" || ev.key() == " " {
                    ev.prevent_default();
                    on_select.run(app_id);
                }
            }
        >
            <div class="launcher-app-card-top">
                <div class="launcher-app-card-icon-wrap">
                    <span class="launcher-app-card-icon" aria-hidden="true">{meta.icon}</span>
                </div>
                <span class="launcher-app-card-badge">
                    {format!("{} công cụ", meta.tool_count)}
                </span>
            </div>

            <div class="launcher-app-card-body">
                <h3 class="launcher-app-card-title">{meta.name}</h3>
                <p class="launcher-app-card-desc">{meta.description}</p>
            </div>

            <div class="launcher-app-card-footer">
                <span class="launcher-app-card-action-text">"Vào không gian"</span>
                <span class="launcher-app-card-arrow" aria-hidden="true">"→"</span>
            </div>
        </div>
    }
}
