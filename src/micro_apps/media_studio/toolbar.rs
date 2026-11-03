use leptos::prelude::*;
use crate::micro_apps::media_studio::state::{media_studio_tool_icon, MediaCategory, MediaStudioState};

/// Thanh công cụ phân loại theo 2 nhóm nghiệp vụ Media + 1 tab Workflow Macro DAG.
#[component]
pub fn MediaStudioToolbar(
    state: MediaStudioState,
) -> impl IntoView {
    view! {
        <nav class="media-studio-toolbar" aria-label="Thanh công cụ Media Studio">
            // Hàng 1: Các tab danh mục nghiệp vụ và nút Workflow
            <div class="media-studio-toolbar-tabs">
                <div class="media-studio-category-tabs">
                    {MediaCategory::ALL.into_iter().map(|cat| {
                        let is_active = move || !state.is_workflow() && state.active_category.get() == cat;
                        let count = cat.slugs().len();
                        view! {
                            <button
                                type="button"
                                class="media-studio-tab-btn"
                                class:active=is_active
                                title=format!("Xem các công cụ thuộc nhóm {}", cat.label())
                                on:click=move |_| state.select_category(cat)
                            >
                                <span class="media-tab-icon" aria-hidden="true">{cat.icon()}</span>
                                <span class="media-tab-label">{cat.label()}</span>
                                <span class="media-tab-count">{count}</span>
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                </div>

                <div class="media-studio-toolbar-tabs-right">
                    <button
                        type="button"
                        class="media-studio-tab-btn media-studio-workflow-tab-btn"
                        class:active=move || state.is_workflow()
                        title="Mở trình kết nối chuỗi quy trình Macro DAG cho đa phương tiện"
                        on:click=move |_| state.open_workflow()
                    >
                        <span class="media-tab-icon" aria-hidden="true">"⚡"</span>
                        <span class="media-tab-label">"Chuỗi Workflow"</span>
                        <span class="media-tab-badge">"Macro DAG"</span>
                    </button>
                </div>
            </div>

            // Hàng 2: Danh sách các công cụ (Tool Chips) trong tab đang chọn
            <div class="media-studio-toolbar-subrow">
                {move || {
                    if state.is_workflow() {
                        view! {
                            <div class="media-studio-subrow-workflow-info">
                                <span class="workflow-info-badge">"⚡ Chế độ Tự động hóa"</span>
                                <span class="workflow-info-text">
                                    "Kéo ghép 9 công cụ Media Studio thành chuỗi xử lý liên hoàn tự động on-device."
                                </span>
                            </div>
                        }.into_any()
                    } else {
                        let cur_cat = state.active_category.get();
                        let slugs = cur_cat.slugs();
                        view! {
                            <div class="media-studio-tool-chips-scroll">
                                {slugs.iter().map(|&slug| {
                                    let tool = state.find_tool(slug);
                                    let name = tool.map(|t| t.name).unwrap_or(slug);
                                    let icon = media_studio_tool_icon(slug);
                                    let is_active = move || !state.is_workflow() && state.active_tool.get() == slug;

                                    view! {
                                        <button
                                            type="button"
                                            class="media-studio-tool-chip"
                                            class:active=is_active
                                            title=tool.map(|t| t.description).unwrap_or("")
                                            on:click=move |_| state.select_tool(slug)
                                        >
                                            <span class="tool-chip-icon" aria-hidden="true">{icon}</span>
                                            <span class="tool-chip-name">{name}</span>
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </nav>
    }
}
