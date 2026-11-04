use leptos::prelude::*;
use crate::micro_apps::media_studio::state::MediaStudioState;

/// Biểu tượng trực quan dựa trên định dạng tệp media.
fn format_media_icon(file_name: &str) -> &'static str {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".mp4") || lower.ends_with(".mov") || lower.ends_with(".avi") || lower.ends_with(".mkv") || lower.ends_with(".webm") {
        "🎞️"
    } else if lower.ends_with(".mp3") || lower.ends_with(".wav") || lower.ends_with(".m4a") || lower.ends_with(".flac") || lower.ends_with(".aac") {
        "🎙️"
    } else {
        "🖼️"
    }
}

/// Panel bên trái quản lý danh sách các tệp đa phương tiện đã nạp vào Media Studio (Shared Context).
///
/// Hỗ trợ 2 chế độ hiển thị:
/// 1. **Mini Gallery Sidebar (56px)**: Tự động thu gọn khi ở trong không gian làm việc tập trung,
///    giải phóng > 90% diện tích màn hình cho Dropzone, Canvas và thanh Batch Queue.
/// 2. **Expanded Gallery Sidebar (280px)**: Mở rộng chi tiết cho phép xem danh sách tệp, enqueue hàng loạt, xóa từng tệp.
#[component]
pub fn MediaStudioGalleryPanel(
    state: MediaStudioState,
) -> impl IntoView {
    // Kích hoạt hộp thoại chọn file của hệ điều hành
    let on_add_file_click = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = js_sys::eval("document.getElementById('media-studio-native-picker')?.click()");
        }
    };

    // Lắng nghe sự kiện người dùng chọn tệp từ thẻ input
    let on_file_input_change = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Ok(js_val) = js_sys::eval("Array.from(document.getElementById('media-studio-native-picker')?.files || []).map(f => f.path || f.name)") {
                if let Ok(names) = serde_wasm_bindgen::from_value::<Vec<String>>(js_val) {
                    if !names.is_empty() {
                        state.add_files(names);
                        let _ = js_sys::eval("var el = document.getElementById('media-studio-native-picker'); if (el) el.value = '';");
                    }
                }
            }
        }
    };

    view! {
        <aside
            class="media-studio-gallery-panel"
            class:collapsed=move || state.sidebar_collapsed.get()
            class:expanded=move || !state.sidebar_collapsed.get()
            aria-label="Quản lý danh sách tệp Media"
        >
            // Thẻ input ẩn chọn tệp Media (luôn sẵn sàng cho cả chế độ mini và expanded)
            <input
                type="file"
                id="media-studio-native-picker"
                accept="image/*,audio/*,video/*,.webp,.avif,.jpg,.jpeg,.png,.gif,.mp3,.wav,.mp4,.mov"
                multiple=true
                style="display: none;"
                on:change=on_file_input_change
            />

            {move || {
                if state.sidebar_collapsed.get() {
                    // =========================================================
                    // 1. GIAO DIỆN MINI GALLERY SIDEBAR (56px)
                    // =========================================================
                    view! {
                        <div class="media-panel-mini" role="toolbar" aria-label="Thanh bên media thu gọn">
                            // Nút Toggle mở rộng
                            <button
                                type="button"
                                class="media-mini-toggle-btn"
                                title="Mở rộng thư viện tệp đầy đủ (Expand)"
                                on:click=move |_| state.toggle_sidebar()
                            >
                                <span class="mini-toggle-icon" aria-hidden="true">"▶"</span>
                            </button>

                            // Nút thêm tệp nhanh (+)
                            <button
                                type="button"
                                class="media-mini-add-btn"
                                title="Thêm tệp ảnh, video, âm thanh mới từ máy tính"
                                on:click=on_add_file_click
                            >
                                <span class="mini-add-icon" aria-hidden="true">"+"</span>
                            </button>

                            // Badge số lượng tệp đã nạp
                            <div
                                class="media-mini-badge-wrap"
                                title=format!("{} tệp đã nạp", state.loaded_files_count())
                            >
                                <span class="media-mini-badge">
                                    {move || format!("{}", state.loaded_files_count())}
                                </span>
                            </div>

                            <div class="media-mini-divider"></div>

                            // Danh sách icon tệp thu gọn
                            <div class="media-mini-file-list" role="list">
                                {move || {
                                    let files = state.loaded_files.get();
                                    let active_idx = state.active_file_index.get();

                                    files.into_iter().enumerate().map(|(idx, path)| {
                                        let filename = path
                                            .rsplit(['/', '\\'])
                                            .next()
                                            .unwrap_or(&path)
                                            .to_string();
                                        let icon = format_media_icon(&filename);
                                        let is_active = active_idx == Some(idx);
                                        view! {
                                            <button
                                                type="button"
                                                class="media-mini-file-item"
                                                class:active=is_active
                                                role="listitem"
                                                title=format!("#{}: {}", idx + 1, filename)
                                                on:click=move |_| state.select_file(idx)
                                            >
                                                <span class="mini-item-icon">{icon}</span>
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
                    // 2. GIAO DIỆN EXPANDED GALLERY SIDEBAR (280px)
                    // =========================================================
                    view! {
                        <div class="media-panel-expanded">
                            // Header của gallery panel
                            <div class="media-panel-header">
                                <div class="media-panel-title-row">
                                    <div class="media-panel-title-group">
                                        <span class="media-panel-icon" aria-hidden="true">"🎞️"</span>
                                        <h3 class="media-panel-title">"Thư viện Media"</h3>
                                    </div>
                                    <div class="media-panel-header-actions">
                                        <span class="media-panel-count-badge">
                                            {move || format!("{} tệp", state.loaded_files_count())}
                                        </span>
                                        <button
                                            type="button"
                                            class="media-panel-collapse-btn"
                                            title="Thu gọn danh sách tệp (Mini Sidebar)"
                                            on:click=move |_| state.toggle_sidebar()
                                        >
                                            <span class="collapse-icon" aria-hidden="true">"◀"</span>
                                        </button>
                                    </div>
                                </div>

                                <div class="media-panel-actions-row">
                                    <button
                                        type="button"
                                        class="media-panel-add-btn"
                                        title="Thêm tệp ảnh, video, âm thanh từ máy tính"
                                        on:click=on_add_file_click
                                    >
                                        <span aria-hidden="true">"+"</span>
                                        <span>"Thêm file"</span>
                                    </button>

                                    {move || {
                                        if state.has_files() {
                                            view! {
                                                <div class="media-panel-bulk-btns">
                                                    <button
                                                        type="button"
                                                        class="media-panel-enqueue-all-btn"
                                                        title="Đưa toàn bộ tệp vào hàng đợi xử lý hàng loạt"
                                                        on:click=move |_| {
                                                            let files = state.loaded_files.get();
                                                            for f in files {
                                                                state.enqueue_batch_item(f);
                                                            }
                                                        }
                                                    >
                                                        "⚡ Enqueue All"
                                                    </button>
                                                    <button
                                                        type="button"
                                                        class="media-panel-clear-btn"
                                                        title="Xóa toàn bộ tệp trong danh sách"
                                                        on:click=move |_| state.clear_files()
                                                    >
                                                        "Xóa hết"
                                                    </button>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <span style="display: none;"></span> }.into_any()
                                        }
                                    }}
                                </div>
                            </div>

                            // Thân danh sách tệp
                            <div class="media-panel-body">
                                {move || {
                                    if !state.has_files() {
                                        // Trạng thái trống
                                        view! {
                                            <div class="media-panel-empty-state">
                                                <div class="empty-icon-wrap" aria-hidden="true">
                                                    <span class="empty-icon">"🎨"</span>
                                                </div>
                                                <h4 class="empty-title">"Chưa có tệp Media nào"</h4>
                                                <p class="empty-desc">
                                                    "Thêm tệp ảnh, âm thanh hoặc video từ máy tính để bắt đầu xử lý."
                                                </p>
                                                <button
                                                    type="button"
                                                    class="media-empty-add-btn"
                                                    on:click=on_add_file_click
                                                >
                                                    <span>"📁 Chọn tệp từ máy..."</span>
                                                </button>
                                            </div>
                                        }.into_any()
                                    } else {
                                        // Danh sách các tệp media đã nạp
                                        let files = state.loaded_files.get();
                                        let active_idx = state.active_file_index.get();

                                        view! {
                                            <div class="media-file-items-list" role="list">
                                                {files.into_iter().enumerate().map(|(idx, file_path)| {
                                                    let is_active = active_idx == Some(idx);
                                                    let path_clone = file_path.clone();
                                                    let path_enqueue = file_path.clone();
                                                    let basename = file_path
                                                        .split('/')
                                                        .next_back()
                                                        .or_else(|| file_path.split('\\').next_back())
                                                        .unwrap_or(&file_path)
                                                        .to_string();
                                                    let icon = format_media_icon(&basename);

                                                    view! {
                                                        <div
                                                            class="media-file-item"
                                                            class:active=is_active
                                                            role="listitem"
                                                            on:click=move |_| state.select_file(idx)
                                                        >
                                                            <div class="media-file-item-left">
                                                                <span class="media-file-icon" aria-hidden="true">{icon}</span>
                                                                <div class="media-file-info">
                                                                    <span class="media-file-name" title=path_clone.clone()>
                                                                        {basename}
                                                                    </span>
                                                                    <span class="media-file-index-tag">
                                                                        {format!("#{}", idx + 1)}
                                                                    </span>
                                                                </div>
                                                            </div>

                                                            <div class="media-file-item-right">
                                                                {if is_active {
                                                                    view! {
                                                                        <span class="media-active-chip" title="Tệp đang được áp dụng cho công cụ">
                                                                            "Đang mở"
                                                                        </span>
                                                                    }.into_any()
                                                                } else {
                                                                    view! { <span style="display: none;"></span> }.into_any()
                                                                }}

                                                                <button
                                                                    type="button"
                                                                    class="media-file-enqueue-btn"
                                                                    title="Đưa tệp này vào hàng đợi xử lý hàng loạt"
                                                                    on:click=move |ev| {
                                                                        ev.stop_propagation();
                                                                        state.enqueue_batch_item(path_enqueue.clone());
                                                                    }
                                                                >
                                                                    "⚡"
                                                                </button>

                                                                <button
                                                                    type="button"
                                                                    class="media-file-remove-btn"
                                                                    title="Xóa tệp khỏi danh sách"
                                                                    on:click=move |ev| {
                                                                        ev.stop_propagation();
                                                                        state.remove_file(idx);
                                                                    }
                                                                >
                                                                    "✕"
                                                                </button>
                                                            </div>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>

                            // Footer hiển thị tệp đang active
                            <div class="media-panel-footer">
                                {move || {
                                    if let Some(active_name) = state.active_file_name() {
                                        let total = state.loaded_files_count();
                                        let current_idx = state.active_file_index.get().map(|i| i + 1).unwrap_or(0);
                                        view! {
                                            <div class="media-panel-status-active">
                                                <span class="status-indicator">"●"</span>
                                                <span class="status-text" title=active_name.clone()>
                                                    {format!("{}/{} • {}", current_idx, total, active_name)}
                                                </span>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="media-panel-status-none">
                                                <span class="status-muted">"Chưa có tệp nào được kích hoạt"</span>
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </aside>
    }
}
