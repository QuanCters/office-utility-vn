use leptos::prelude::*;
use crate::app::{ToolItem, ALL_TOOLS};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidebarDestination {
    Home,
    WorkflowDag,
    History,
}

pub struct DomainMeta {
    pub key: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
}

pub const DOMAIN_GROUPS: &[DomainMeta] = &[
    DomainMeta { key: "vn-office", icon: "🇻🇳", label: "Nghiệp vụ VN" },
    DomainMeta { key: "doc-suite", icon: "📄", label: "Document Suite" },
    DomainMeta { key: "media-studio", icon: "🎨", label: "Media Studio" },
    DomainMeta { key: "data-lab", icon: "📊", label: "Data Lab" },
    DomainMeta { key: "sys-hub", icon: "⚙️", label: "System Hub" },
];

#[component]
pub fn Sidebar<FSelect, FNav, FToggleFav>(
    current_slug: ReadSignal<String>,
    is_home_active: Signal<bool>,
    is_workflow_active: Signal<bool>,
    is_history_active: Signal<bool>,
    favorite_slugs: ReadSignal<Vec<String>>,
    recent_slugs: ReadSignal<Vec<String>>,
    on_select_tool: FSelect,
    on_navigate: FNav,
    on_toggle_favorite: FToggleFav,
) -> impl IntoView
where
    FSelect: Fn(String) + Send + Sync + Copy + 'static,
    FNav: Fn(SidebarDestination) + Send + Sync + Copy + 'static,
    FToggleFav: Fn(String) + Send + Sync + Copy + 'static,
{
    let (is_collapsed, set_is_collapsed) = signal(false);
    let (search_query, set_search_query) = signal(String::new());
    let (_is_ime_composing, set_is_ime_composing) = signal(false);

    // Trạng thái mở/đóng từng nhóm domain
    let (open_vn, set_open_vn) = signal(true);
    let (open_doc, set_open_doc) = signal(true);
    let (open_media, set_open_media) = signal(false);
    let (open_data, set_open_data) = signal(false);
    let (open_sys, set_open_sys) = signal(false);

    let get_domain_signal = move |key: &str| -> (Signal<bool>, Callback<()>) {
        match key {
            "vn-office" => (
                open_vn.into(),
                Callback::new(move |_| set_open_vn.update(|v| *v = !*v)),
            ),
            "doc-suite" => (
                open_doc.into(),
                Callback::new(move |_| set_open_doc.update(|v| *v = !*v)),
            ),
            "media-studio" => (
                open_media.into(),
                Callback::new(move |_| set_open_media.update(|v| *v = !*v)),
            ),
            "data-lab" => (
                open_data.into(),
                Callback::new(move |_| set_open_data.update(|v| *v = !*v)),
            ),
            _ => (
                open_sys.into(),
                Callback::new(move |_| set_open_sys.update(|v| *v = !*v)),
            ),
        }
    };

    let filtered_tools = move || {
        let q = search_query.get().trim().to_lowercase();
        if q.is_empty() {
            Vec::new()
        } else {
            ALL_TOOLS
                .iter()
                .filter(|t| {
                    t.name.to_lowercase().contains(&q)
                        || t.slug.to_lowercase().contains(&q)
                        || t.description.to_lowercase().contains(&q)
                })
                .cloned()
                .collect::<Vec<ToolItem>>()
        }
    };

    let find_tool = |slug: &str| -> Option<ToolItem> {
        ALL_TOOLS.iter().find(|t| t.slug == slug).cloned()
    };

    view! {
        <aside class=move || if is_collapsed.get() { "app-sidebar collapsed" } else { "app-sidebar" }>
            // Header thương hiệu mini & nút thu gọn
            <div class="sidebar-top-bar">
                <div class="sidebar-brand-mini" on:click=move |_| on_navigate(SidebarDestination::Home) title="Về trang chủ">
                    <span class="sidebar-logo-icon">"⚡"</span>
                    <span class="sidebar-brand-text">"Office Utility"</span>
                </div>
                <button
                    class="sidebar-collapse-toggle-btn"
                    title=move || if is_collapsed.get() { "Mở rộng thanh bên" } else { "Thu gọn thanh bên" }
                    on:click=move |_| set_is_collapsed.update(|c| *c = !*c)
                >
                    { move || if is_collapsed.get() { "»" } else { "«" } }
                </button>
            </div>

            // Ô tìm kiếm nhanh inline
            <div class="sidebar-search-container">
                <div class="sidebar-search-box">
                    <span class="sidebar-search-icon">"🔍"</span>
                    <input
                        class="sidebar-search-input"
                        type="text"
                        placeholder="Tìm công cụ..."
                        prop:value=search_query
                        on:compositionstart=move |_| set_is_ime_composing.set(true)
                        on:compositionend=move |_| set_is_ime_composing.set(false)
                        on:input=move |ev| set_search_query.set(event_target_value(&ev))
                    />
                    { move || {
                        if !search_query.get().is_empty() {
                            view! {
                                <button class="sidebar-search-clear" on:click=move |_| set_search_query.set(String::new())>
                                    "✕"
                                </button>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                </div>
            </div>

            // Nội dung chính của Sidebar: Danh sách cuộn
            <div class="sidebar-scrollable-content">
                { move || {
                    let results = filtered_tools();
                    if !search_query.get().trim().is_empty() {
                        // Chế độ hiển thị kết quả tìm kiếm trực tiếp
                        view! {
                            <div class="sidebar-section">
                                <div class="sidebar-section-header">
                                    <span>"KẾT QUẢ TÌM KIẾM"</span>
                                    <span class="sidebar-badge">{results.len()}</span>
                                </div>
                                <div class="sidebar-items-list">
                                    {
                                        if results.is_empty() {
                                            view! {
                                                <div class="sidebar-empty-hint">"Không thấy công cụ phù hợp"</div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div>
                                                    { results.into_iter().map(|tool| {
                                                        let s = tool.slug.to_string();
                                                        let s_fav = tool.slug.to_string();
                                                        let is_cur = current_slug.get() == tool.slug;
                                                        let is_fav = favorite_slugs.get().contains(&s_fav);
                                                        view! {
                                                            <div class=if is_cur { "sidebar-tool-item active" } else { "sidebar-tool-item" }>
                                                                <button
                                                                    class="sidebar-tool-name-btn"
                                                                    title=tool.description
                                                                    on:click=move |_| on_select_tool(s.clone())
                                                                >
                                                                    <span class="tool-bullet">"•"</span>
                                                                    <span class="sidebar-tool-label">{tool.name}</span>
                                                                </button>
                                                                <button
                                                                    class=if is_fav { "sidebar-fav-btn active" } else { "sidebar-fav-btn" }
                                                                    title=if is_fav { "Bỏ yêu thích" } else { "Thêm vào yêu thích" }
                                                                    on:click=move |_| on_toggle_favorite(s_fav.clone())
                                                                >
                                                                    "★"
                                                                </button>
                                                            </div>
                                                        }
                                                    }).collect_view() }
                                                </div>
                                            }.into_any()
                                        }
                                    }
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        // Chế độ phân cấp tiêu chuẩn: Yêu thích, Gần đây, 5 Domain
                        view! {
                            <div>
                                // 1. Section: Yêu thích (Favorites)
                                { move || {
                                    let favs = favorite_slugs.get();
                                    if favs.is_empty() {
                                        view! { <div></div> }.into_any()
                                    } else {
                                        view! {
                                            <div class="sidebar-section">
                                                <div class="sidebar-section-header">
                                                    <span>"★ YÊU THÍCH"</span>
                                                    <span class="sidebar-badge">{favs.len()}</span>
                                                </div>
                                                <div class="sidebar-items-list">
                                                    { favs.into_iter().filter_map(|s| find_tool(&s)).map(|tool| {
                                                        let s_tool = tool.slug.to_string();
                                                        let s_fav = tool.slug.to_string();
                                                        let is_cur = current_slug.get() == tool.slug;
                                                        view! {
                                                            <div class=if is_cur { "sidebar-tool-item active" } else { "sidebar-tool-item" }>
                                                                <button
                                                                    class="sidebar-tool-name-btn"
                                                                    title=tool.description
                                                                    on:click=move |_| on_select_tool(s_tool.clone())
                                                                >
                                                                    <span class="tool-bullet" style="color: #F59E0B;">"★"</span>
                                                                    <span class="sidebar-tool-label">{tool.name}</span>
                                                                </button>
                                                                <button
                                                                    class="sidebar-fav-btn active"
                                                                    title="Bỏ yêu thích"
                                                                    on:click=move |_| on_toggle_favorite(s_fav.clone())
                                                                >
                                                                    "✕"
                                                                </button>
                                                            </div>
                                                        }
                                                    }).collect_view() }
                                                </div>
                                            </div>
                                        }.into_any()
                                    }
                                }}

                                // 2. Section: Vừa dùng gần đây (Recents)
                                { move || {
                                    let recents = recent_slugs.get();
                                    if recents.is_empty() {
                                        view! { <div></div> }.into_any()
                                    } else {
                                        view! {
                                            <div class="sidebar-section">
                                                <div class="sidebar-section-header">
                                                    <span>"🕒 GẦN ĐÂY"</span>
                                                    <span class="sidebar-badge">{recents.len().min(5)}</span>
                                                </div>
                                                <div class="sidebar-items-list">
                                                    { recents.into_iter().take(5).filter_map(|s| find_tool(&s)).map(|tool| {
                                                        let s_tool = tool.slug.to_string();
                                                        let is_cur = current_slug.get() == tool.slug;
                                                        view! {
                                                            <div class=if is_cur { "sidebar-tool-item active" } else { "sidebar-tool-item" }>
                                                                <button
                                                                    class="sidebar-tool-name-btn"
                                                                    title=tool.description
                                                                    on:click=move |_| on_select_tool(s_tool.clone())
                                                                >
                                                                    <span class="tool-bullet" style="color: #38BDF8;">"•"</span>
                                                                    <span class="sidebar-tool-label">{tool.name}</span>
                                                                </button>
                                                            </div>
                                                        }
                                                    }).collect_view() }
                                                </div>
                                            </div>
                                        }.into_any()
                                    }
                                }}

                                <div class="sidebar-divider"></div>

                                // 3. 5 Domain Groups Phân tầng
                                <div class="sidebar-domain-groups-container">
                                    {
                                        DOMAIN_GROUPS.iter().map(|domain| {
                                            let key = domain.key;
                                            let icon = domain.icon;
                                            let label = domain.label;
                                            let (is_open, toggle_open) = get_domain_signal(key);
                                            let tools: Vec<ToolItem> = ALL_TOOLS
                                                .iter()
                                                .filter(|t| t.domain == key)
                                                .cloned()
                                                .collect();
                                            let count = tools.len();

                                            view! {
                                                <div class="sidebar-domain-group">
                                                    <button class="sidebar-domain-header" on:click=move |_| toggle_open.run(())>
                                                        <div class="sidebar-domain-title">
                                                            <span class="sidebar-domain-icon">{icon}</span>
                                                            <span class="sidebar-domain-label">{label}</span>
                                                        </div>
                                                        <div class="sidebar-domain-meta">
                                                            <span class="sidebar-badge">{count}</span>
                                                            <span class="sidebar-arrow">{ move || if is_open.get() { "▾" } else { "▸" } }</span>
                                                        </div>
                                                    </button>

                                                    { move || {
                                                        if is_open.get() {
                                                            let tools_copy = tools.clone();
                                                            view! {
                                                                <div class="sidebar-domain-items">
                                                                    { tools_copy.into_iter().map(|tool| {
                                                                        let s_tool = tool.slug.to_string();
                                                                        let s_fav = tool.slug.to_string();
                                                                        let is_cur = current_slug.get() == tool.slug;
                                                                        let is_fav = favorite_slugs.get().contains(&s_fav);
                                                                        view! {
                                                                            <div class=if is_cur { "sidebar-tool-item active" } else { "sidebar-tool-item" }>
                                                                                <button
                                                                                    class="sidebar-tool-name-btn"
                                                                                    title=tool.description
                                                                                    on:click=move |_| on_select_tool(s_tool.clone())
                                                                                >
                                                                                    <span class="tool-bullet">"•"</span>
                                                                                    <span class="sidebar-tool-label">{tool.name}</span>
                                                                                </button>
                                                                                <button
                                                                                    class=if is_fav { "sidebar-fav-btn active" } else { "sidebar-fav-btn" }
                                                                                    title=if is_fav { "Bỏ yêu thích" } else { "Thêm vào yêu thích" }
                                                                                    on:click=move |_| on_toggle_favorite(s_fav.clone())
                                                                                >
                                                                                    "★"
                                                                                </button>
                                                                            </div>
                                                                        }
                                                                    }).collect_view() }
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            view! { <div></div> }.into_any()
                                                        }
                                                    }}
                                                </div>
                                            }
                                        }).collect_view()
                                    }
                                </div>
                            </div>
                        }.into_any()
                    }
                }}
            </div>

            // Bottom Navigation: Trang chủ, Chuỗi Workflow, Lịch sử tác vụ
            <div class="sidebar-bottom-nav">
                <button
                    class=move || if is_home_active.get() { "sidebar-nav-btn active" } else { "sidebar-nav-btn" }
                    on:click=move |_| on_navigate(SidebarDestination::Home)
                    title="Về màn hình Dashboard Tổng quan"
                >
                    <span class="sidebar-nav-icon">"🏠"</span>
                    <span class="sidebar-nav-label">"Trang Chủ"</span>
                </button>

                <button
                    class=move || if is_workflow_active.get() { "sidebar-nav-btn active" } else { "sidebar-nav-btn" }
                    on:click=move |_| on_navigate(SidebarDestination::WorkflowDag)
                    title="Chuỗi Tự động hóa Pipeline DAG"
                >
                    <span class="sidebar-nav-icon">"⚡"</span>
                    <span class="sidebar-nav-label">"Chuỗi Workflow"</span>
                </button>

                <button
                    class=move || if is_history_active.get() { "sidebar-nav-btn active" } else { "sidebar-nav-btn" }
                    on:click=move |_| on_navigate(SidebarDestination::History)
                    title="Nhật ký Lịch sử Tác vụ Cục bộ"
                >
                    <span class="sidebar-nav-icon">"⏱️"</span>
                    <span class="sidebar-nav-label">"Lịch Sử Tác Vụ"</span>
                </button>
            </div>
        </aside>
    }
}
