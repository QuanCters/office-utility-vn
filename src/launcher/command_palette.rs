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

/// Tự động cuộn phần tử active trong Command Palette vào vùng nhìn thấy (nearest block).
pub fn scroll_active_item_into_view() {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        leptos::task::spawn_local(async {
            if let Some(win) = web_sys::window() {
                if let Some(doc) = win.document() {
                    if let Ok(Some(elem)) = doc.query_selector(".palette-item.active") {
                        let obj = js_sys::Object::new();
                        let _ = js_sys::Reflect::set(
                            &obj,
                            &wasm_bindgen::JsValue::from_str("block"),
                            &wasm_bindgen::JsValue::from_str("nearest"),
                        );
                        let scroll_fn = js_sys::Reflect::get(
                            &elem,
                            &wasm_bindgen::JsValue::from_str("scrollIntoView"),
                        )
                        .unwrap_or(wasm_bindgen::JsValue::UNDEFINED);

                        if scroll_fn.is_function() {
                            if let Ok(func) = scroll_fn.dyn_into::<js_sys::Function>() {
                                let _ = func.call1(&elem, &obj);
                                return;
                            }
                        }
                        elem.scroll_into_view();
                    }
                }
            }
        });
    }
}

/// Global Command Palette (Ctrl+K):
/// - Tìm kiếm nhanh trên toàn bộ công cụ (51 công cụ) hoặc phạm vi Micro-App
/// - Hỗ trợ gõ tiếng Việt có dấu và không dấu
/// - Điều hướng bằng phím mũi tên Lên/Xuống
/// - Enter để chọn công cụ và kích hoạt `on_deep_link(micro_app_id, tool_slug)`
/// - Hỗ trợ tùy chọn quay về Launcher (`on_back`) khi chạy trong ngữ cảnh Micro-App
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
    /// Callback quay về Launcher (nếu có)
    #[prop(into, optional)]
    on_back: Option<Callback<()>>,
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

    #[derive(Clone, Copy)]
    enum PaletteItem<'a> {
        Back,
        Tool(&'a ToolItem),
    }

    let filtered_items = move || -> Vec<PaletteItem<'static>> {
        let q = search_query.get();
        let q_trimmed = q.trim();
        let q_lower = q_trimmed.to_lowercase();
        let q_unaccented = remove_vietnamese_diacritics(&q_lower);

        let mut items = Vec::new();

        if on_back.is_some() {
            let matches_back = q_trimmed.is_empty()
                || "trang chính launcher quay lại về home back".contains(&q_lower)
                || "trang chinh launcher quay lai ve home back".contains(&q_unaccented);
            if matches_back {
                items.push(PaletteItem::Back);
            }
        }

        let tools_matched = filter_tools(tool_list, &q);
        for t in tools_matched {
            items.push(PaletteItem::Tool(t));
        }

        items
    };

    let select_item = {
        move |item: PaletteItem| {
            match item {
                PaletteItem::Back => {
                    open.set(false);
                    if let Some(cb) = on_back {
                        cb.run(());
                    }
                }
                PaletteItem::Tool(t) => {
                    if let Some(app_id) = tool_slug_to_micro_app(t.slug) {
                        open.set(false);
                        on_deep_link.run((app_id, t.slug.to_string()));
                    }
                }
            }
        }
    };

    // Tự động cuộn theo dõi khi selected_index thay đổi
    Effect::new(move |_| {
        let _ = selected_index.get();
        scroll_active_item_into_view();
    });

    // Xử lý phím điều hướng khi đang mở palette
    let on_palette_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        if key == "Escape" {
            ev.prevent_default();
            open.set(false);
            return;
        }

        let items = filtered_items();
        let total = items.len();
        if total == 0 {
            return;
        }

        if key == "ArrowDown" {
            ev.prevent_default();
            selected_index.update(|idx| {
                *idx = (*idx + 1) % total;
            });
            scroll_active_item_into_view();
        } else if key == "ArrowUp" {
            ev.prevent_default();
            selected_index.update(|idx| {
                *idx = if *idx == 0 { total - 1 } else { *idx - 1 };
            });
            scroll_active_item_into_view();
        } else if key == "Enter" {
            // Không kích hoạt chọn nếu người dùng đang trong phiên gõ dấu Tiếng Việt (IME composition)
            if !is_ime_composing.get() && !ev.is_composing() {
                ev.prevent_default();
                let current_idx = selected_index.get();
                if let Some(item) = items.get(current_idx) {
                    select_item(*item);
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
                                let results = filtered_items();
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
                                                <span class="palette-result-count">{format!("{} mục", total)}</span>
                                            </div>
                                            {results.into_iter().enumerate().map(|(idx, item)| {
                                                let is_active = idx == current_idx;
                                                let item_class = format!(
                                                    "palette-item {}",
                                                    if is_active { "active" } else { "" }
                                                );

                                                match item {
                                                    PaletteItem::Back => view! {
                                                        <div
                                                            class=item_class
                                                            role="option"
                                                            aria-selected=is_active.to_string()
                                                            on:click=move |_| select_item(PaletteItem::Back)
                                                            on:mouseenter=move |_| selected_index.set(idx)
                                                        >
                                                            <div class="palette-item-content">
                                                                <div class="palette-item-name">
                                                                    <span class="palette-item-app-icon">"🏠"</span>
                                                                    <span>"← Về trang chính (Launcher)"</span>
                                                                </div>
                                                                <div class="palette-item-desc">
                                                                    "Quay trở lại trung tâm điều phối tổng thể"
                                                                </div>
                                                            </div>
                                                            <span class="palette-item-badge app-badge" style="background: rgba(79, 70, 229, 0.2); color: #818CF8;">
                                                                "Launcher"
                                                            </span>
                                                        </div>
                                                    }.into_any(),
                                                    PaletteItem::Tool(tool) => {
                                                        let slug = tool.slug;
                                                        let app_opt = tool_slug_to_micro_app(slug);

                                                        view! {
                                                            <div
                                                                class=item_class
                                                                role="option"
                                                                aria-selected=is_active.to_string()
                                                                on:click=move |_| select_item(PaletteItem::Tool(tool))
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
                                                        }.into_any()
                                                    }
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
