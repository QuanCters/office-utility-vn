use leptos::ev::MouseEvent;
use leptos::prelude::*;
use crate::app::{ToolItem, ALL_TOOLS};
use crate::micro_apps::{tool_slug_to_micro_app, MicroAppId};

/// Chuyển đổi chuỗi tiếng Việt có dấu thành không dấu (lowercase ASCII),
/// giúp tìm kiếm mượt mà không phân biệt dấu và chữ hoa/thường.
pub fn remove_vietnamese_diacritics(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'a' | 'à' | 'á' | 'ả' | 'ã' | 'ạ' | 'ă' | 'ằ' | 'ắ' | 'ẳ' | 'ẵ' | 'ặ' | 'â' | 'ầ'
            | 'ấ' | 'ẩ' | 'ẫ' | 'ậ' | 'A' | 'À' | 'Á' | 'Ả' | 'Ã' | 'Ạ' | 'Ă' | 'Ằ' | 'Ắ'
            | 'Ẳ' | 'Ẵ' | 'Ặ' | 'Â' | 'Ầ' | 'Ấ' | 'Ẩ' | 'Ẫ' | 'Ậ' => 'a',
            'd' | 'đ' | 'D' | 'Đ' => 'd',
            'e' | 'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' | 'E' | 'È'
            | 'É' | 'Ẻ' | 'Ẽ' | 'Ẹ' | 'Ê' | 'Ề' | 'Ế' | 'Ể' | 'Ễ' | 'Ệ' => 'e',
            'i' | 'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' | 'I' | 'Ì' | 'Í' | 'Ỉ' | 'Ĩ' | 'Ị' => 'i',
            'o' | 'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ờ'
            | 'ớ' | 'ở' | 'ỡ' | 'ợ' | 'O' | 'Ò' | 'Ó' | 'Ỏ' | 'Õ' | 'Ọ' | 'Ô' | 'Ồ' | 'Ố'
            | 'Ổ' | 'Ỗ' | 'Ộ' | 'Ơ' | 'Ờ' | 'Ớ' | 'Ở' | 'Ỡ' | 'Ợ' => 'o',
            'u' | 'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' | 'U' | 'Ù'
            | 'Ú' | 'Ủ' | 'Ũ' | 'Ụ' | 'Ư' | 'Ừ' | 'Ứ' | 'Ử' | 'Ữ' | 'Ự' => 'u',
            'y' | 'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' | 'Y' | 'Ỳ' | 'Ý' | 'Ỷ' | 'Ỹ' | 'Ỵ' => 'y',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

/// Lọc danh sách công cụ theo từ khóa tìm kiếm (hỗ trợ có dấu và không dấu).
pub fn filter_tools<'a>(tools: &'a [ToolItem], query: &str) -> Vec<&'a ToolItem> {
    let q_trimmed = query.trim();
    if q_trimmed.is_empty() {
        return tools.iter().collect();
    }

    let q_lower = q_trimmed.to_lowercase();
    let q_unaccented = remove_vietnamese_diacritics(&q_lower);

    tools
        .iter()
        .filter(|t| {
            // Kiểm tra khớp trực tiếp (case-insensitive)
            let name_lower = t.name.to_lowercase();
            let desc_lower = t.description.to_lowercase();
            let slug_lower = t.slug.to_lowercase();
            let domain_lower = t.domain.to_lowercase();

            if name_lower.contains(&q_lower)
                || desc_lower.contains(&q_lower)
                || slug_lower.contains(&q_lower)
                || domain_lower.contains(&q_lower)
            {
                return true;
            }

            // Kiểm tra khớp không dấu (unaccented diacritics stripping)
            let name_unaccented = remove_vietnamese_diacritics(t.name);
            let desc_unaccented = remove_vietnamese_diacritics(t.description);

            name_unaccented.contains(&q_unaccented)
                || desc_unaccented.contains(&q_unaccented)
                || slug_lower.contains(&q_unaccented)
        })
        .collect()
}

/// Global Command Palette (Ctrl+K):
/// - Tìm kiếm nhanh trên toàn bộ công cụ (51 công cụ)
/// - Hỗ trợ gõ tiếng Việt có dấu và không dấu
/// - Điều hướng bằng phím mũi tên Lên/Xuống
/// - Enter để chọn công cụ và kích hoạt `on_deep_link(micro_app_id, tool_slug)`
/// - Phím `Esc` hoặc bấm vùng ngoài để đóng
#[component]
pub fn CommandPalette(
    /// Tín hiệu đóng/mở palette
    open: RwSignal<bool>,
    /// Callback điều hướng deep-link
    #[prop(into)]
    on_deep_link: Callback<(MicroAppId, String)>,
    /// Danh sách công cụ tùy chọn (mặc định ALL_TOOLS)
    #[prop(into, optional)]
    tools: Option<&'static [ToolItem]>,
) -> impl IntoView {
    let search_query = RwSignal::new(String::new());
    let selected_index = RwSignal::new(0usize);
    let is_ime_composing = RwSignal::new(false);

    let tool_list = tools.unwrap_or(ALL_TOOLS);

    // Đăng ký phím tắt toàn cục Ctrl+K / Cmd+K để bật/tắt palette
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::ev::keydown;
        let _ = window_event_listener(keydown, move |ev: web_sys::KeyboardEvent| {
            let ctrl_or_meta = ev.ctrl_key() || ev.meta_key();
            let key = ev.key();

            if ctrl_or_meta && (key == "k" || key == "K") {
                ev.prevent_default();
                open.update(|is_open| {
                    *is_open = !*is_open;
                    if *is_open {
                        search_query.set(String::new());
                        selected_index.set(0);
                    }
                });
            } else if open.get() && key == "Escape" {
                ev.prevent_default();
                open.set(false);
            }
        });
    }

    let filtered = move || {
        let q = search_query.get();
        filter_tools(tool_list, &q)
    };

    let select_tool = {
        move |slug: &str| {
            if let Some(app_id) = tool_slug_to_micro_app(slug) {
                open.set(false);
                on_deep_link.run((app_id, slug.to_string()));
            }
        }
    };

    // Xử lý phím điều hướng khi đang mở palette
    let on_palette_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        if key == "Escape" {
            ev.prevent_default();
            open.set(false);
            return;
        }

        let items = filtered();
        let total = items.len();
        if total == 0 {
            return;
        }

        if key == "ArrowDown" {
            ev.prevent_default();
            selected_index.update(|idx| {
                *idx = (*idx + 1) % total;
            });
        } else if key == "ArrowUp" {
            ev.prevent_default();
            selected_index.update(|idx| {
                *idx = if *idx == 0 { total - 1 } else { *idx - 1 };
            });
        } else if key == "Enter" {
            // Không kích hoạt chọn nếu người dùng đang trong phiên gõ dấu Tiếng Việt (IME composition)
            if !is_ime_composing.get() && !ev.is_composing() {
                ev.prevent_default();
                let current_idx = selected_index.get();
                if let Some(tool) = items.get(current_idx) {
                    select_tool(tool.slug);
                }
            }
        }
    };

    view! {
        {move || {
            if !open.get() {
                return None;
            }

            Some(view! {
                <div
                    class="modal-overlay"
                    on:click=move |_| open.set(false)
                >
                    <div
                        class="command-palette"
                        on:click=move |ev: MouseEvent| ev.stop_propagation()
                        on:keydown=on_palette_keydown
                    >
                        <div class="palette-input-box">
                            <span class="palette-search-icon" aria-hidden="true">"🔍"</span>
                            <input
                                class="palette-input"
                                type="text"
                                placeholder="Tìm kiếm nhanh công cụ theo tên, chức năng hoặc nghiệp vụ..."
                                autofocus=true
                                prop:value=move || search_query.get()
                                on:input=move |ev| {
                                    search_query.set(event_target_value(&ev));
                                    selected_index.set(0);
                                }
                                on:compositionstart=move |_| is_ime_composing.set(true)
                                on:compositionend=move |_| is_ime_composing.set(false)
                            />
                            <span class="palette-esc-hint">"Esc để đóng"</span>
                        </div>

                        <div class="palette-list" role="listbox">
                            {move || {
                                let results = filtered();
                                let total = results.len();
                                let current_idx = selected_index.get();

                                if results.is_empty() {
                                    view! {
                                        <div class="palette-empty-state">
                                            <div class="palette-empty-icon">"🔍"</div>
                                            <div class="palette-empty-title">"Không tìm thấy công cụ phù hợp"</div>
                                            <div class="palette-empty-desc">
                                                "Thử tìm bằng từ khóa khác (ví dụ: pdf, vietqr, hoá đơn, lương, excel, ảnh...)"
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div>
                                            <div class="palette-section-header">
                                                <span>"KẾT QUẢ TÌM KIẾM"</span>
                                                <span class="palette-result-count">{format!("{} công cụ", total)}</span>
                                            </div>
                                            {results.into_iter().enumerate().map(|(idx, tool)| {
                                                let slug = tool.slug;
                                                let app_opt = tool_slug_to_micro_app(slug);
                                                let is_active = idx == current_idx;

                                                let item_class = format!(
                                                    "palette-item {}",
                                                    if is_active { "active" } else { "" }
                                                );

                                                view! {
                                                    <div
                                                        class=item_class
                                                        role="option"
                                                        aria-selected=is_active.to_string()
                                                        on:click=move |_| select_tool(slug)
                                                        on:mouseenter=move |_| selected_index.set(idx)
                                                    >
                                                        <div class="palette-item-content">
                                                            <div class="palette-item-name">
                                                                {if let Some(app) = app_opt {
                                                                    view! {
                                                                        <span class="palette-item-app-icon" title=app.name()>
                                                                            {app.icon()}
                                                                        </span>
                                                                    }.into_any()
                                                                } else {
                                                                    view! { <span class="palette-item-app-icon">"⚡"</span> }.into_any()
                                                                }}
                                                                <span>{tool.name}</span>
                                                            </div>
                                                            <div class="palette-item-desc">{tool.description}</div>
                                                        </div>

                                                        {if let Some(app) = app_opt {
                                                            view! {
                                                                <span class="palette-item-badge app-badge">
                                                                    {app.name()}
                                                                </span>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <span class="palette-item-badge">{tool.domain}</span>
                                                            }.into_any()
                                                        }}
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                }
                            }}
                        </div>
                    </div>
                </div>
            })
        }}
    }
}
