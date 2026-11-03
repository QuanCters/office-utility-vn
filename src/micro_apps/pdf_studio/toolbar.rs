use leptos::prelude::*;
use crate::micro_apps::pdf_studio::state::{PdfCategory, PdfStudioState};

/// Ánh xạ tĩnh từ slug công cụ PDF sang icon emoji đặc trưng.
pub fn pdf_studio_tool_icon(slug: &str) -> &'static str {
    match slug {
        // Ghép / Tách
        "pdf-merge-split" => "📑",
        "pdf-crop-margin" => "✂️",
        "pdf-booklet-imposition" => "📖",
        "pdf-bookmark-toc" => "📑",
        "pdf-visual-diff" => "⚖️",

        // Bảo mật & Tối ưu
        "pdf-protect-sign" => "🔒",
        "pdf-compress" => "🗜️",
        "pdf-metadata-scrubber" => "🧹",
        "pdf-flatten-forms" => "📋",
        "pdf-repair-corrupt" => "🩹",

        // Định dạng & Trang
        "pdf-page-number-header" => "🔢",
        "pdf-watermark-stamp" => "🔖",
        "pdf-grayscale-print" => "🖨️",

        // Chuyển đổi & OCR
        "vietnamese-ocr" => "🔍",
        "pdf-to-markdown" => "📝",
        "pdf-to-long-image" => "🖼️",
        "pdf-table-extractor" => "📊",
        "markdown-to-styled-pdf" => "📄",

        _ => "📄",
    }
}

/// Thanh công cụ phân loại theo 4 tab nghiệp vụ PDF + 1 tab Workflow Macro DAG.
#[component]
pub fn PdfStudioToolbar(
    state: PdfStudioState,
) -> impl IntoView {
    view! {
        <nav class="pdf-studio-toolbar" aria-label="Thanh công cụ PDF Studio">
            // Hàng 1: Các tab danh mục nghiệp vụ và nút Workflow
            <div class="pdf-studio-toolbar-tabs">
                <div class="pdf-studio-category-tabs">
                    {PdfCategory::ALL.into_iter().map(|cat| {
                        let is_active = move || !state.is_workflow() && state.active_category.get() == cat;
                        let count = cat.slugs().len();
                        view! {
                            <button
                                type="button"
                                class="pdf-studio-tab-btn"
                                class:active=is_active
                                title=format!("Xem các công cụ thuộc nhóm {}", cat.label())
                                on:click=move |_| state.select_category(cat)
                            >
                                <span class="pdf-tab-icon" aria-hidden="true">{cat.icon()}</span>
                                <span class="pdf-tab-label">{cat.label()}</span>
                                <span class="pdf-tab-count">{count}</span>
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                </div>

                <div class="pdf-studio-toolbar-tabs-right">
                    <button
                        type="button"
                        class="pdf-studio-tab-btn pdf-studio-workflow-tab-btn"
                        class:active=move || state.is_workflow()
                        title="Mở trình kết nối chuỗi quy trình Macro DAG cho tài liệu PDF"
                        on:click=move |_| state.open_workflow()
                    >
                        <span class="pdf-tab-icon" aria-hidden="true">"⚡"</span>
                        <span class="pdf-tab-label">"Chuỗi Workflow"</span>
                        <span class="pdf-tab-badge">"Macro DAG"</span>
                    </button>
                </div>
            </div>

            // Hàng 2: Danh sách các công cụ (Tool Chips) trong tab đang chọn
            <div class="pdf-studio-toolbar-subrow">
                {move || {
                    if state.is_workflow() {
                        view! {
                            <div class="pdf-studio-subrow-workflow-info">
                                <span class="workflow-info-badge">"⚡ Chế độ Tự động hóa"</span>
                                <span class="workflow-info-text">
                                    "Kéo ghép 18 công cụ PDF thành chuỗi xử lý liên hoàn tự động on-device."
                                </span>
                            </div>
                        }.into_any()
                    } else {
                        let cur_cat = state.active_category.get();
                        let slugs = cur_cat.slugs();
                        view! {
                            <div class="pdf-studio-tool-chips-scroll">
                                {slugs.iter().map(|&slug| {
                                    let tool = state.find_tool(slug);
                                    let name = tool.map(|t| t.name).unwrap_or(slug);
                                    let desc = tool.map(|t| t.description).unwrap_or("");
                                    let icon = pdf_studio_tool_icon(slug);
                                    let is_active = move || state.active_tool.get() == slug;

                                    view! {
                                        <button
                                            type="button"
                                            class="pdf-studio-tool-chip"
                                            class:active=is_active
                                            title=desc
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
