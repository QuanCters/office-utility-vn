pub mod app_card;
pub mod command_palette;
pub mod history_feed;

pub use app_card::AppCard;
pub use command_palette::{
    filter_tools, remove_vietnamese_diacritics, scroll_active_item_into_view, CommandPalette,
};
pub use history_feed::{format_elapsed_time, format_history_status, HistoryFeed};

use leptos::prelude::*;
use crate::app::TaskHistoryUi;
use crate::micro_apps::MicroAppId;

/// Màn hình Launcher trung tâm (Command Center) của Office Utility VN:
/// - Header: Logo, tiêu đề, badge bảo mật "100% On-Device Native · Zero-Cloud Security" và thanh tìm kiếm nhanh Ctrl+K
/// - 4 Micro-App Cards: VN Office (11 tools), PDF Studio (18 tools), Media Studio (9 tools), Data Lab (13 tools)
/// - Unified Task History Feed: Lịch sử tác vụ tổng hợp, suy luận icon MicroApp từ `tool_slug` và hỗ trợ deep-link
/// - Global Command Palette: Tìm kiếm nhanh trên 51 công cụ với hỗ trợ tiếng Việt không dấu, phím mũi tên và Enter
#[component]
pub fn LauncherView(
    /// Callback khi người dùng nhấp chọn 1 trong 4 Micro-Apps
    #[prop(into)]
    on_select_app: Callback<MicroAppId>,
    /// Callback khi người dùng click vào lịch sử hoặc chọn tool từ Command Palette để mở thẳng công cụ
    #[prop(into)]
    on_deep_link: Callback<(MicroAppId, String)>,
    /// Tín hiệu lịch sử tác vụ tùy chọn
    #[prop(optional)]
    history: Option<Signal<Vec<TaskHistoryUi>>>,
) -> impl IntoView {
    let palette_open = RwSignal::new(false);

    view! {
        <div class="launcher-view-container">
            // Header & Hero Banner
            <header class="launcher-header">
                <div class="launcher-hero-badge">
                    <span class="launcher-pulse-dot" aria-hidden="true">"🟢"</span>
                    <span>"100% On-Device Native · Zero-Cloud Security"</span>
                </div>

                <div class="launcher-title-row">
                    <div>
                        <h1 class="launcher-main-title">
                            <span class="launcher-title-icon" aria-hidden="true">"⚡"</span>
                            <span>"Office Utility VN"</span>
                        </h1>
                        <p class="launcher-subtitle">
                            "Trung tâm điều khiển 4 không gian làm việc chuyên sâu chuẩn nghiệp vụ Việt Nam"
                        </p>
                    </div>

                    // Quick Search Trigger (Ctrl+K)
                    <div
                        class="launcher-search-trigger"
                        role="button"
                        tabindex="0"
                        title="Tìm kiếm nhanh công cụ (Ctrl+K)"
                        on:click=move |_| palette_open.set(true)
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" || ev.key() == " " {
                                ev.prevent_default();
                                palette_open.set(true);
                            }
                        }
                    >
                        <span class="launcher-search-icon" aria-hidden="true">"🔍"</span>
                        <span class="launcher-search-placeholder">"Tìm nhanh 51 công cụ..."</span>
                        <kbd class="launcher-search-kbd">"Ctrl+K"</kbd>
                    </div>
                </div>
            </header>

            // Section 1: 4 Micro-App Cards
            <section class="launcher-section" aria-labelledby="launcher-apps-title">
                <div class="launcher-section-header">
                    <h2 id="launcher-apps-title" class="launcher-section-title">
                        "🏢 KHÔNG GIAN LÀM VIỆC CHUYÊN SÂU"
                    </h2>
                    <span class="launcher-section-desc">
                        "Chọn không gian để làm việc liên tục trên cùng tập tài liệu"
                    </span>
                </div>

                <div class="launcher-cards-grid">
                    <AppCard
                        app_id=MicroAppId::VnOffice
                        on_select=on_select_app
                    />
                    <AppCard
                        app_id=MicroAppId::PdfStudio
                        on_select=on_select_app
                    />
                    <AppCard
                        app_id=MicroAppId::MediaStudio
                        on_select=on_select_app
                    />
                    <AppCard
                        app_id=MicroAppId::DataLab
                        on_select=on_select_app
                    />
                </div>
            </section>

            // Section 2: Task History Feed
            <section class="launcher-section" aria-labelledby="launcher-history-title">
                {if let Some(h) = history {
                    view! {
                        <HistoryFeed
                            on_deep_link=on_deep_link
                            history=h
                        />
                    }.into_any()
                } else {
                    view! {
                        <HistoryFeed
                            on_deep_link=on_deep_link
                        />
                    }.into_any()
                }}
            </section>

            // Section 3: Keyboard shortcuts hint bar
            <footer class="launcher-shortcuts-footer">
                <div class="launcher-shortcut-item">
                    <kbd class="kbd">"Ctrl"</kbd>
                    <span>"+"</span>
                    <kbd class="kbd">"K"</kbd>
                    <span class="shortcut-desc">"Tìm nhanh công cụ"</span>
                </div>
                <div class="launcher-shortcut-item">
                    <kbd class="kbd">"Ctrl"</kbd>
                    <span>"+"</span>
                    <kbd class="kbd">"Enter"</kbd>
                    <span class="shortcut-desc">"Thực thi tác vụ"</span>
                </div>
                <div class="launcher-shortcut-item">
                    <kbd class="kbd">"Ctrl"</kbd>
                    <span>"+"</span>
                    <kbd class="kbd">"P"</kbd>
                    <span class="shortcut-desc">"Ghim Always-on-Top"</span>
                </div>
                <div class="launcher-shortcut-item">
                    <kbd class="kbd">"Esc"</kbd>
                    <span class="shortcut-desc">"Đóng bảng / Hủy"</span>
                </div>
            </footer>

            // Global Command Palette Modal
            <CommandPalette
                open=palette_open
                on_deep_link=on_deep_link
            />
        </div>
    }
}
