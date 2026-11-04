use leptos::prelude::*;
use crate::micro_apps::pdf_studio::state::PdfStudioState;

/// Panel bên trái quản lý danh sách các tệp PDF đã nạp vào workspace (Shared Context).
///
/// Hỗ trợ 2 chế độ hiển thị:
/// 1. **Mini Sidebar (56px)**: Tự động thu gọn khi người dùng đang ở trong không gian làm việc tập trung,
///    giải phóng > 90% diện tích màn hình cho Dropzone và cấu hình.
/// 2. **Expanded Sidebar (280px)**: Mở rộng chi tiết cho phép xem danh sách tệp, xóa từng tệp, xóa tất cả.
#[component]
pub fn PdfStudioFileListPanel(
    state: PdfStudioState,
) -> impl IntoView {
    // Kích hoạt hộp thoại chọn file của hệ điều hành
    let on_add_file_click = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = js_sys::eval("document.getElementById('pdf-studio-native-picker')?.click()");
        }
    };

    // Lắng nghe sự kiện người dùng chọn tệp từ thẻ input
    let on_file_input_change = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Ok(js_val) = js_sys::eval("Array.from(document.getElementById('pdf-studio-native-picker')?.files || []).map(f => f.path || f.name)") {
                if let Ok(names) = serde_wasm_bindgen::from_value::<Vec<String>>(js_val) {
                    if !names.is_empty() {
                        state.add_files(names);
                        // Reset input value để cho phép chọn lại cùng file
                        let _ = js_sys::eval("var el = document.getElementById('pdf-studio-native-picker'); if (el) el.value = '';");
                    }
                }
            }
        }
    };

    view! {
        <aside
            class="pdf-studio-file-panel"
            class:collapsed=move || state.sidebar_collapsed.get()
            class:expanded=move || !state.sidebar_collapsed.get()
            aria-label="Quản lý danh sách tệp PDF"
        >
            // Thẻ input ẩn chọn tệp PDF (luôn sẵn sàng cho cả chế độ mini và expanded)
            <input
                type="file"
                id="pdf-studio-native-picker"
                accept=".pdf,application/pdf"
                multiple=true
                style="display: none;"
                on:change=on_file_input_change
            />

            {move || {
                if state.sidebar_collapsed.get() {
                    // =========================================================
                    // 1. GIAO DIỆN MINI SIDEBAR (56px)
                    // =========================================================
                    view! {
                        <div class="pdf-panel-mini" role="toolbar" aria-label="Thanh bên tệp thu gọn">
                            // Nút Toggle mở rộng
                            <button
                                type="button"
                                class="pdf-mini-toggle-btn"
                                title="Mở rộng danh sách tệp đầy đủ (Expand)"
                                on:click=move |_| state.toggle_sidebar()
                            >
                                <span class="mini-toggle-icon" aria-hidden="true">"▶"</span>
                            </button>

                            // Nút thêm tệp nhanh (+)
                            <button
                                type="button"
                                class="pdf-mini-add-btn"
                                title="Thêm tệp PDF mới từ máy tính"
                                on:click=on_add_file_click
                            >
                                <span class="mini-add-icon" aria-hidden="true">"+"</span>
                            </button>

                            // Badge số lượng tệp đã nạp
                            <div
                                class="pdf-mini-badge-wrap"
                                title=format!("{} tệp đã nạp", state.loaded_files_count())
                            >
                                <span class="pdf-mini-badge">
                                    {move || format!("{}", state.loaded_files_count())}
                                </span>
                            </div>

                            <div class="pdf-mini-divider"></div>

                            // Danh sách icon file thu gọn
                            <div class="pdf-mini-file-list" role="list">
                                {move || {
                                    let files = state.loaded_files.get();
                                    let active_idx = state.active_file_index.get();

                                    files.into_iter().enumerate().map(|(idx, path)| {
                                        let filename = path
                                            .rsplit(['/', '\\'])
                                            .next()
                                            .unwrap_or(&path)
                                            .to_string();
                                        let is_active = active_idx == Some(idx);
                                        view! {
                                            <button
                                                type="button"
                                                class="pdf-mini-file-item"
                                                class:active=is_active
                                                role="listitem"
                                                title=format!("#{}: {}", idx + 1, filename)
                                                on:click=move |_| state.select_file(idx)
                                            >
                                                <span class="mini-item-icon">"📄"</span>
                                                <span class="mini-item-num">{format!("{}", idx + 1)}</span>
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()
                                }}
                            </div>
                        </div>
                    }.into_any()
                } else {
                    // =========================================================
                    // 2. GIAO DIỆN EXPANDED SIDEBAR (280px)
                    // =========================================================
                    view! {
                        <div class="pdf-panel-expanded">
                            // Header của panel
                            <div class="pdf-panel-header">
                                <div class="pdf-panel-title-row">
                                    <div class="pdf-panel-title-group">
                                        <span class="pdf-panel-icon" aria-hidden="true">"📑"</span>
                                        <h3 class="pdf-panel-title">"Tài liệu PDF"</h3>
                                    </div>
                                    <div class="pdf-panel-header-actions">
                                        <span class="pdf-panel-count-badge">
                                            {move || format!("{} tệp", state.loaded_files_count())}
                                        </span>
                                        <button
                                            type="button"
                                            class="pdf-panel-collapse-btn"
                                            title="Thu gọn danh sách tệp (Mini Sidebar)"
                                            on:click=move |_| state.toggle_sidebar()
                                        >
                                            <span class="collapse-icon" aria-hidden="true">"◀"</span>
                                        </button>
                                    </div>
                                </div>

                                <div class="pdf-panel-actions-row">
                                    <button
                                        type="button"
                                        class="pdf-panel-add-btn"
                                        title="Thêm tệp PDF từ máy tính để xử lý"
                                        on:click=on_add_file_click
                                    >
                                        <span aria-hidden="true">"+"</span>
                                        <span>"Thêm file"</span>
                                    </button>

                                    {move || {
                                        if state.has_files() {
                                            view! {
                                                <button
                                                    type="button"
                                                    class="pdf-panel-clear-btn"
                                                    title="Xóa toàn bộ tệp trong danh sách"
                                                    on:click=move |_| state.clear_files()
                                                >
                                                    "Xóa hết"
                                                </button>
                                            }.into_any()
                                        } else {
                                            view! { <span style="display: none;"></span> }.into_any()
                                        }
                                    }}
                                </div>
                            </div>

                            // Thân danh sách tệp
                            <div class="pdf-panel-body">
                                {move || {
                                    if !state.has_files() {
                                        // Trạng thái trống
                                        view! {
                                            <div class="pdf-panel-empty-state">
                                                <div class="empty-icon-wrap" aria-hidden="true">
                                                    <span class="empty-icon">"📄"</span>
                                                </div>
                                                <h4 class="empty-title">"Chưa có tệp PDF nào"</h4>
                                                <p class="empty-desc">
                                                    "Nạp tệp một lần để dùng chung cho toàn bộ các công cụ PDF Studio."
                                                </p>
                                                <button
                                                    type="button"
                                                    class="empty-add-btn"
                                                    on:click=on_add_file_click
                                                >
                                                    "+ Chọn tệp PDF..."
                                                </button>
                                            </div>
                                        }.into_any()
                                    } else {
                                        // Danh sách các tệp đã nạp
                                        let files = state.loaded_files.get();
                                        let active_idx = state.active_file_index.get();

                                        view! {
                                            <div class="pdf-panel-file-list" role="list">
                                                {files.into_iter().enumerate().map(|(idx, path)| {
                                                    let filename = path
                                                        .rsplit(['/', '\\'])
                                                        .next()
                                                        .unwrap_or(&path)
                                                        .to_string();
                                                    let is_active = active_idx == Some(idx);
                                                    let full_path = path.clone();

                                                    view! {
                                                        <div
                                                            class="pdf-file-item"
                                                            class:active=is_active
                                                            role="listitem"
                                                            tabindex="0"
                                                            title=full_path
                                                            on:click=move |_| state.select_file(idx)
                                                        >
                                                            <span class="pdf-file-idx" aria-hidden="true">
                                                                {format!("#{}", idx + 1)}
                                                            </span>
                                                            <span class="pdf-file-icon" aria-hidden="true">"📄"</span>
                                                            <div class="pdf-file-info">
                                                                <span class="pdf-file-name">{filename}</span>
                                                                {if is_active {
                                                                    view! {
                                                                        <span class="pdf-file-active-badge">
                                                                            "Đang kích hoạt"
                                                                        </span>
                                                                    }.into_any()
                                                                } else {
                                                                    view! { <span style="display:none"></span> }.into_any()
                                                                }}
                                                            </div>
                                                            <button
                                                                type="button"
                                                                class="pdf-file-remove-btn"
                                                                title="Xóa tệp này khỏi danh sách"
                                                                on:click=move |ev| {
                                                                    ev.stop_propagation();
                                                                    state.remove_file(idx);
                                                                }
                                                            >
                                                                <span aria-hidden="true">"✕"</span>
                                                            </button>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>

                            // Footer hiển thị tệp đang kích hoạt
                            {move || {
                                if let Some(active_name) = state.active_file_name() {
                                    let total = state.loaded_files_count();
                                    let current_num = state.active_file_index.get().map(|i| i + 1).unwrap_or(0);
                                    view! {
                                        <div class="pdf-panel-footer">
                                            <div class="pdf-panel-active-indicator">
                                                <span class="active-label">"Tệp đang xử lý:"</span>
                                                <div class="active-file-row">
                                                    <span class="active-file-title" title=state.active_file_path().unwrap_or_default()>
                                                        {active_name}
                                                    </span>
                                                    <span class="active-file-pos">
                                                        {format!("({}/{})", current_num, total)}
                                                    </span>
                                                </div>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span style="display: none;"></span> }.into_any()
                                }
                            }}
                        </div>
                    }.into_any()
                }
            }}
        </aside>
    }
}
