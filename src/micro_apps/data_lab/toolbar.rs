use leptos::prelude::*;
use crate::micro_apps::data_lab::state::{DataLabCategory, DataLabState};

/// Ánh xạ tĩnh từ slug công cụ Data Lab sang icon emoji đặc trưng.
pub fn data_lab_tool_icon(slug: &str) -> &'static str {
    match slug {
        // Bảng tính & Excel
        "excel-cleaner-normalizer" => "🧹",
        "excel-split-by-column" => "🗂️",
        "excel-multi-merger" => "📑",
        "excel-visual-joiner" => "🔗",
        "excel-sheet-unlocker" => "🔓",

        // Chuyển đổi & Trộn tệp
        "data-structure-converter" => "🔄",
        "docx-template-mailmerge" => "✉️",
        "universal-file-converter-hub" | "universal-converter" => "⚡",
        "base64-file-codec" => "🔐",

        // Văn bản & Cấu trúc
        "diff-checker-text-sheets" => "⚖️",
        "list-set-operations" => "🔢",
        "text-pii-anonymizer" => "🛡️",
        "sql-formatter-linter" => "💻",

        _ => "📊",
    }
}

/// Thanh công cụ phân loại theo 3 tab nghiệp vụ Data Lab + 1 tab Workflow Macro DAG.
#[component]
pub fn DataLabToolbar(
    state: DataLabState,
) -> impl IntoView {
    view! {
        <nav class="data-lab-toolbar" aria-label="Thanh công cụ Data Lab">
            // Hàng 1: Các tab danh mục nghiệp vụ và nút Workflow
            <div class="data-lab-toolbar-tabs">
                <div class="data-lab-category-tabs">
                    {DataLabCategory::ALL.into_iter().map(|cat| {
                        let is_active = move || !state.is_workflow() && state.active_category.get() == cat;
                        let count = cat.slugs().len();
                        view! {
                            <button
                                type="button"
                                class="data-lab-tab-btn"
                                class:active=is_active
                                title=format!("Xem các công cụ thuộc nhóm {}", cat.label())
                                on:click=move |_| state.select_category(cat)
                            >
                                <span class="data-tab-icon" aria-hidden="true">{cat.icon()}</span>
                                <span class="data-tab-label">{cat.label()}</span>
                                <span class="data-tab-count">{count}</span>
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                </div>

                <div class="data-lab-toolbar-tabs-right">
                    <button
                        type="button"
                        class="data-lab-tab-btn data-lab-workflow-tab-btn"
                        class:active=move || state.is_workflow()
                        title="Mở trình kết nối chuỗi quy trình Macro DAG cho dữ liệu bảng tính"
                        on:click=move |_| state.open_workflow()
                    >
                        <span class="data-tab-icon" aria-hidden="true">"⚡"</span>
                        <span class="data-tab-label">"Chuỗi Workflow"</span>
                        <span class="data-tab-badge">"Macro DAG"</span>
                    </button>
                </div>
            </div>

            // Hàng 2: Danh sách các công cụ (Tool Chips) trong tab đang chọn
            <div class="data-lab-toolbar-subrow">
                {move || {
                    if state.is_workflow() {
                        view! {
                            <div class="data-lab-subrow-workflow-info">
                                <span class="workflow-info-badge">"⚡ Chế độ Tự động hóa"</span>
                                <span class="workflow-info-text">
                                    "Kéo ghép 13 công cụ Data Lab thành chuỗi xử lý liên hoàn tự động on-device."
                                </span>
                            </div>
                        }.into_any()
                    } else {
                        let cur_cat = state.active_category.get();
                        let slugs = cur_cat.slugs();
                        view! {
                            <div class="data-lab-tool-chips-scroll">
                                {slugs.iter().map(|&slug| {
                                    let tool = state.find_tool(slug);
                                    let name = tool.map(|t| t.name).unwrap_or(slug);
                                    let icon = data_lab_tool_icon(slug);
                                    let is_active = move || state.active_tool.get() == slug;

                                    view! {
                                        <button
                                            type="button"
                                            class="data-lab-tool-chip"
                                            class:active=is_active
                                            title=format!("Mở công cụ {}", name)
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
