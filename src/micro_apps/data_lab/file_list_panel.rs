use leptos::prelude::*;
use crate::micro_apps::data_lab::state::DataLabState;

/// Panel bên trái quản lý danh sách các tệp dữ liệu đã nạp vào workspace (Shared Context).
///
/// Các tệp ở đây được lưu giữ và sử dụng chung cho toàn bộ 13 công cụ Data Lab.
#[component]
pub fn DataLabFileListPanel(
    state: DataLabState,
) -> impl IntoView {
    let is_collapsed = RwSignal::new(false);

    // Kích hoạt hộp thoại chọn file của hệ điều hành
    let on_add_file_click = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = js_sys::eval("document.getElementById('data-lab-native-picker')?.click()");
        }
    };

    // Lắng nghe sự kiện người dùng chọn tệp từ thẻ input
    let on_file_input_change = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Ok(js_val) = js_sys::eval("Array.from(document.getElementById('data-lab-native-picker')?.files || []).map(f => f.path || f.name)") {
                if let Ok(names) = serde_wasm_bindgen::from_value::<Vec<String>>(js_val) {
                    if !names.is_empty() {
                        state.add_files(names);
                        // Reset input value để cho phép chọn lại cùng file
                        let _ = js_sys::eval("var el = document.getElementById('data-lab-native-picker'); if (el) el.value = '';");
                    }
                }
            }
        }
    };

    view! {
        <aside
            class="data-lab-file-panel"
            class:collapsed=move || is_collapsed.get()
            aria-label="Quản lý danh sách tệp dữ liệu"
        >
            // Thẻ input ẩn chọn tệp dữ liệu đa định dạng
            <input
                type="file"
                id="data-lab-native-picker"
                accept=".xlsx,.xls,.csv,.tsv,.json,.sql,.txt,.docx"
                multiple=true
                style="display: none;"
                on:change=on_file_input_change
            />

            // Header của panel
            <div class="data-panel-header">
                <div class="data-panel-title-row">
                    <div class="data-panel-title-group">
                        <span class="data-panel-icon" aria-hidden="true">"📊"</span>
                        <h3 class="data-panel-title">"Dữ liệu & Bảng tính"</h3>
                    </div>
                    <div style="display: flex; align-items: center; gap: 4px;">
                        <span class="data-panel-count-badge">
                            {move || format!("{} tệp", state.loaded_files_count())}
                        </span>
                        <button
                            type="button"
                            class="data-panel-collapse-btn"
                            title=move || if is_collapsed.get() { "Mở rộng danh sách tệp (290px)" } else { "Thu gọn danh sách tệp" }
                            on:click=move |_| is_collapsed.update(|c| *c = !*c)
                        >
                            {move || if is_collapsed.get() { "▶" } else { "◀" }}
                        </button>
                    </div>
                </div>

                <div class="data-panel-actions-row">
                    <button
                        type="button"
                        class="data-panel-add-btn"
                        title="Thêm tệp dữ liệu (Excel, CSV, JSON, DOCX...) từ máy tính"
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
                                    class="data-panel-clear-btn"
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
            <div class="data-panel-body">
                {move || {
                    if !state.has_files() {
                        // Trạng thái trống
                        view! {
                            <div class="data-panel-empty-state">
                                <div class="empty-icon-wrap" aria-hidden="true">
                                    <span class="empty-icon">"📊"</span>
                                </div>
                                <h4 class="empty-title">"Chưa có tệp dữ liệu nào"</h4>
                                <p class="empty-desc">
                                    "Nhấn '+ Thêm file' để nạp bảng tính Excel (.xlsx, .xls), CSV, JSON hoặc DOCX vào phòng thí nghiệm."
                                </p>
                                <button
                                    type="button"
                                    class="empty-add-btn"
                                    on:click=on_add_file_click
                                >
                                    "+ Chọn tệp dữ liệu"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        // Danh sách các tệp đã nạp
                        let files = state.loaded_files.get();
                        let active_idx = state.active_file_index.get();

                        view! {
                            <ul class="data-panel-file-list" role="list">
                                {files.into_iter().enumerate().map(|(idx, path)| {
                                    let is_active = active_idx == Some(idx);
                                    let name = path.rsplit(['/', '\\']).next().unwrap_or(&path).to_string();
                                    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
                                    let (format_label, format_class) = match ext.as_str() {
                                        "xlsx" | "xls" => ("XLSX", "format-pill-excel"),
                                        "csv" | "tsv" => ("CSV", "format-pill-csv"),
                                        "json" => ("JSON", "format-pill-json"),
                                        "sql" => ("SQL", "format-pill-sql"),
                                        "docx" => ("DOCX", "format-pill-docx"),
                                        "txt" => ("TXT", "format-pill-txt"),
                                        _ => ("FILE", "format-pill-default"),
                                    };
                                    let is_spreadsheet = matches!(ext.as_str(), "xlsx" | "xls" | "csv" | "tsv");

                                    view! {
                                        <li
                                            class="data-panel-file-item"
                                            class:active=is_active
                                            on:click=move |_| state.select_file(idx)
                                        >
                                            <div class="file-item-header">
                                                <div class="file-item-left">
                                                    <span class=format!("format-pill {}", format_class)>
                                                        {format_label}
                                                    </span>
                                                    <span class="file-name" title=path.clone()>
                                                        {name.clone()}
                                                    </span>
                                                </div>

                                                <button
                                                    type="button"
                                                    class="file-remove-btn"
                                                    title="Xóa tệp khỏi danh sách"
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        state.remove_file(idx);
                                                    }
                                                >
                                                    "✕"
                                                </button>
                                            </div>

                                            {if is_active && is_spreadsheet {
                                                view! {
                                                    <div class="file-item-sheet-row" on:click=move |ev| ev.stop_propagation()>
                                                        <label class="sheet-label">"Trang tính:"</label>
                                                        <select
                                                            class="sheet-dropdown"
                                                            prop:value=move || state.active_sheet.get()
                                                            on:change=move |ev| state.set_sheet(event_target_value(&ev))
                                                        >
                                                            {state.available_sheets.get().into_iter().map(|s| {
                                                                let s_val = s.clone();
                                                                view! {
                                                                    <option value=s_val>{s}</option>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </select>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <span style="display: none;"></span> }.into_any()
                                            }}
                                        </li>
                                    }
                                }).collect::<Vec<_>>()}
                            </ul>
                        }.into_any()
                    }
                }}
            </div>
        </aside>
    }
}
