use leptos::prelude::*;
use crate::app::ToolItem;
use crate::micro_apps::vn_office::state::VnOfficeState;

/// Trả về biểu tượng đại diện trực quan cho từng công cụ thuộc VN Office.
pub fn vn_office_tool_icon(slug: &str) -> &'static str {
    match slug {
        "vietnamese-number-to-words" => "💰",
        "gross-net-salary-calculator" => "💵",
        "vietqr-napas247-generator" => "📱",
        "vn-einvoice-xml-reader" => "🧾",
        "vn-cccd-mst-validator" => "🪪",
        "vn-address-normalizer" => "📍",
        "vn-working-days-calc" => "📅",
        "vn-loan-amortization" => "🏦",
        "vn-bank-bin-mapper" => "🏛️",
        "vietnamese-font-converter" => "🔠",
        "encoding-mojibake-fixer" => "🛠️",
        _ => "💼",
    }
}

/// Sidebar nội bộ của VN Office:
/// - Liệt kê thanh mảnh, gọn gàng 11 công cụ nghiệp vụ
/// - Chỉ báo công cụ đang hoạt động (active indicator)
/// - Nút "⚡ Chuỗi Quy trình (Workflow)" mở trình thiết kế DAG
/// - Ô lọc nhanh công cụ theo từ khóa
#[component]
pub fn VnOfficeSidebar(
    state: VnOfficeState,
) -> impl IntoView {
    let filter_text = state.sidebar_filter;

    // Lọc danh sách công cụ theo từ khóa tìm kiếm (không phân biệt hoa thường)
    let filtered_tools = move || {
        let query = filter_text.get().trim().to_lowercase();
        let all_tools = state.tools();
        if query.is_empty() {
            all_tools.to_vec()
        } else {
            all_tools
                .iter()
                .filter(|t| {
                    t.name.to_lowercase().contains(&query)
                        || t.slug.to_lowercase().contains(&query)
                        || t.description.to_lowercase().contains(&query)
                })
                .cloned()
                .collect::<Vec<ToolItem>>()
        }
    };

    view! {
        <aside class="vn-office-sidebar" aria-label="Thanh điều hướng VN Office">
            // Tiêu đề phân mục & ô tìm kiếm
            <div class="vn-office-sidebar-header">
                <div class="vn-office-sidebar-title-row">
                    <span class="vn-office-sidebar-badge">"VN OFFICE"</span>
                    <span class="vn-office-sidebar-count">"11 Công cụ"</span>
                </div>
                <div class="vn-office-search-box">
                    <span class="vn-office-search-icon" aria-hidden="true">"🔍"</span>
                    <input
                        type="text"
                        class="vn-office-search-input"
                        placeholder="Lọc công cụ nghiệp vụ..."
                        prop:value=move || filter_text.get()
                        on:input=move |ev| filter_text.set(event_target_value(&ev))
                    />
                    {move || {
                        if !filter_text.get().is_empty() {
                            view! {
                                <button
                                    type="button"
                                    class="vn-office-search-clear"
                                    title="Xóa tìm kiếm"
                                    on:click=move |_| filter_text.set(String::new())
                                >
                                    "✕"
                                </button>
                            }.into_any()
                        } else {
                            view! { <span style="display: none;"></span> }.into_any()
                        }
                    }}
                </div>
            </div>

            // Nút bấm chuỗi quy trình Workflow DAG chuyên biệt
            <div class="vn-office-workflow-section">
                <button
                    type="button"
                    class="vn-office-workflow-btn"
                    class:active=move || state.is_workflow_active.get()
                    title="Mở trình kết nối chuỗi tự động hóa Macro DAG"
                    on:click=move |_| state.open_workflow()
                >
                    <span class="vn-office-workflow-icon" aria-hidden="true">"⚡"</span>
                    <div class="vn-office-workflow-text">
                        <span class="vn-office-workflow-name">"Chuỗi Quy trình (Workflow)"</span>
                        <span class="vn-office-workflow-desc">"Tự động hóa Macro DAG"</span>
                    </div>
                </button>
            </div>

            <div class="vn-office-section-divider">
                <span>"NGHIỆP VỤ & QUY CHUẨN"</span>
            </div>

            // Danh sách 11 công cụ
            <nav class="vn-office-tools-list">
                {move || {
                    let tools = filtered_tools();
                    if tools.is_empty() {
                        view! {
                            <div class="vn-office-empty-filter">
                                "Không tìm thấy công cụ phù hợp."
                            </div>
                        }.into_any()
                    } else {
                        tools
                            .into_iter()
                            .map(|tool| {
                                let slug = tool.slug.to_string();
                                let slug_for_click = slug.clone();
                                let slug_for_active = slug.clone();
                                let icon = vn_office_tool_icon(&slug);

                                view! {
                                    <button
                                        type="button"
                                        class="vn-office-tool-item"
                                        class:active=move || {
                                            !state.is_workflow_active.get() && state.active_tool.get() == slug_for_active
                                        }
                                        title=tool.description.to_string()
                                        on:click=move |_| state.select_tool(&slug_for_click)
                                    >
                                        <span class="vn-office-tool-item-icon" aria-hidden="true">{icon}</span>
                                        <div class="vn-office-tool-item-content">
                                            <span class="vn-office-tool-item-name">{tool.name}</span>
                                            <span class="vn-office-tool-item-slug">{slug}</span>
                                        </div>
                                    </button>
                                }
                            })
                            .collect::<Vec<_>>()
                            .into_any()
                    }
                }}
            </nav>
        </aside>
    }
}
