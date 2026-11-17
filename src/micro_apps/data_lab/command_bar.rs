use leptos::prelude::*;

/// 3 chế độ không gian làm việc chính trong Data Lab v2.0
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum DataLabWorkspaceMode {
    /// Bảng tính linh hoạt & Làm sạch dữ liệu chuyên sâu
    #[default]
    TableClean,
    /// Đối soát trực quan & So sánh 2 bảng (Reconciliation)
    VisualReconcile,
    /// Xử lý văn bản, chuyển đổi cấu trúc & bộ mã
    TextCodecs,
}

impl DataLabWorkspaceMode {
    pub fn label(&self) -> &'static str {
        match self {
            DataLabWorkspaceMode::TableClean => "⊞ Bảng & Làm sạch",
            DataLabWorkspaceMode::VisualReconcile => "☍ Đối soát & So sánh",
            DataLabWorkspaceMode::TextCodecs => "⇄ Văn bản & Chuyển mã",
        }
    }
}

/// Thanh lệnh chỉ huy trung tâm Data Lab v2.0 (Header Command Bar)
///
/// Hỗ trợ chuyển đổi chế độ làm việc nhanh, kích hoạt tìm kiếm lệnh CommandPalette,
/// toggle các panel (Explorer, Drawer, Inspector) và Smart Clipboard Quick-Fix.
#[component]
pub fn DataLabCommandBar(
    #[prop(into)] on_back: Callback<()>,
    current_mode: RwSignal<DataLabWorkspaceMode>,
    palette_open: RwSignal<bool>,
    left_collapsed: RwSignal<bool>,
    drawer_open: RwSignal<bool>,
    right_collapsed: RwSignal<bool>,
    clipboard_hint: RwSignal<Option<String>>,
    #[prop(into)] on_apply_clipboard_fix: Callback<()>,
) -> impl IntoView {
    view! {
        <header class="data-lab-command-bar">
            // Nhóm 1: Nút điều hướng Back & Nhận diện Data Lab v2.0
            <div class="command-bar-left">
                <button
                    type="button"
                    class="cmd-bar-back-btn"
                    title="Quay lại Launcher tổng quan (Esc)"
                    on:click=move |_| on_back.run(())
                >
                    <span class="cmd-bar-back-icon">"←"</span>
                    <span class="cmd-bar-back-label">"Launcher"</span>
                </button>

                <div class="cmd-bar-brand">
                    <span class="cmd-bar-logo-icon">"📊"</span>
                    <span class="cmd-bar-title">"DATA LAB"</span>
                    <span class="cmd-bar-version-badge">"v2.0"</span>
                </div>
            </div>

            // Nhóm 2: Command Palette Trigger & Smart Clipboard Pill
            <div class="command-bar-center">
                <button
                    type="button"
                    class="cmd-bar-search-trigger"
                    title="Mở bảng tìm kiếm lệnh và 13 công cụ dữ liệu (Ctrl+K)"
                    on:click=move |_| palette_open.set(true)
                >
                    <span class="cmd-bar-search-icon">"🔍"</span>
                    <span class="cmd-bar-search-placeholder">"Ctrl+K: Tìm 13+ công cụ / Gõ lệnh..."</span>
                </button>

                // Smart Clipboard Pill (nếu có clipboard hint)
                {move || {
                    clipboard_hint.get().map(|hint| {
                        view! {
                            <button
                                type="button"
                                class="cmd-bar-clipboard-pill"
                                title="Bấm để tự động áp dụng xử lý cho nội dung trong Clipboard"
                                on:click=move |_| on_apply_clipboard_fix.run(())
                            >
                                <span class="clipboard-pill-icon">"✨"</span>
                                <span class="clipboard-pill-text">{hint}</span>
                                <span class="clipboard-pill-badge">"Sửa ngay"</span>
                            </button>
                        }
                    })
                }}
            </div>

            // Nhóm 3: Bộ 3 Workspace Mode Switcher
            <div class="command-bar-modes">
                <div class="workspace-mode-segmented-control">
                    <button
                        type="button"
                        class="mode-segment-btn"
                        class:active=move || current_mode.get() == DataLabWorkspaceMode::TableClean
                        on:click=move |_| current_mode.set(DataLabWorkspaceMode::TableClean)
                        title="Không gian Bảng tính & Làm sạch dữ liệu"
                    >
                        <span class="mode-icon">"⊞"</span>
                        <span class="mode-text">"Bảng & Làm sạch"</span>
                    </button>
                    <button
                        type="button"
                        class="mode-segment-btn"
                        class:active=move || current_mode.get() == DataLabWorkspaceMode::VisualReconcile
                        on:click=move |_| current_mode.set(DataLabWorkspaceMode::VisualReconcile)
                        title="Không gian Đối soát & So sánh Bảng tính"
                    >
                        <span class="mode-icon">"☍"</span>
                        <span class="mode-text">"Đối soát & So sánh"</span>
                    </button>
                    <button
                        type="button"
                        class="mode-segment-btn"
                        class:active=move || current_mode.get() == DataLabWorkspaceMode::TextCodecs
                        on:click=move |_| current_mode.set(DataLabWorkspaceMode::TextCodecs)
                        title="Không gian Văn bản & Chuyển mã"
                    >
                        <span class="mode-icon">"⇄"</span>
                        <span class="mode-text">"Văn bản & Chuyển mã"</span>
                    </button>
                </div>
            </div>

            // Nhóm 4: Nút Toggle Panel & Badge On-Device
            <div class="command-bar-right">
                <button
                    type="button"
                    class="cmd-bar-toggle-btn"
                    class:active=move || !left_collapsed.get()
                    title="Ẩn / Hiện Explorer Tệp (Ctrl+B)"
                    on:click=move |_| left_collapsed.update(|v| *v = !*v)
                >
                    <span class="toggle-icon">"📁"</span>
                    <span class="toggle-label">"Ctrl+B"</span>
                </button>

                <button
                    type="button"
                    class="cmd-bar-toggle-btn"
                    class:active=move || drawer_open.get()
                    title="Mở / Đóng Ngăn Kéo Quy trình DAG & SQL Scratchpad"
                    on:click=move |_| drawer_open.update(|v| *v = !*v)
                >
                    <span class="toggle-icon">"⚡"</span>
                    <span class="toggle-label">"Luồng / 💻 SQL"</span>
                </button>

                <button
                    type="button"
                    class="cmd-bar-toggle-btn"
                    class:active=move || !right_collapsed.get()
                    title="Ẩn / Hiện Right Inspector Công cụ & Lịch sử (Ctrl+J)"
                    on:click=move |_| right_collapsed.update(|v| *v = !*v)
                >
                    <span class="toggle-icon">"🛠️"</span>
                    <span class="toggle-label">"Ctrl+J"</span>
                </button>

                <span class="data-lab-privacy-badge">
                    "🔒 100% On-Device"
                </span>
            </div>
        </header>
    }
}
