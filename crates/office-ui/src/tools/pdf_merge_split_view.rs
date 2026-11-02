use leptos::prelude::*;

#[component]
pub fn PdfMergeSplitView() -> impl IntoView {
    let (mode, set_mode) = signal("merge".to_string());
    let (page_ranges, set_page_ranges) = signal("1-3, 5".to_string());
    let (_rotation_deg, set_rotation_deg) = signal("90".to_string());
    let (_is_composing, set_is_composing) = signal(false);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Chế độ xử lý:"</label>
                <div style="display: flex; gap: 12px; margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="merge_split_mode"
                            checked=move || mode.get() == "merge"
                            on:change=move |_| set_mode.set("merge".to_string())
                        />
                        "Ghép tệp (Merge)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="merge_split_mode"
                            checked=move || mode.get() == "split"
                            on:change=move |_| set_mode.set("split".to_string())
                        />
                        "Tách trang (Split)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="merge_split_mode"
                            checked=move || mode.get() == "rotate"
                            on:change=move |_| set_mode.set("rotate".to_string())
                        />
                        "Xoay trang (Rotate)"
                    </label>
                </div>
            </div>

            {move || {
                if mode.get() == "split" {
                    view! {
                        <div class="form-group">
                            <label class="form-label">"Dải trang cần trích xuất (VD: 1-3, 5, 8-10):"</label>
                            <input
                                class="form-input"
                                type="text"
                                prop:value=page_ranges
                                on:compositionstart=move |_| set_is_composing.set(true)
                                on:compositionend=move |_| set_is_composing.set(false)
                                on:input=move |ev| set_page_ranges.set(event_target_value(&ev))
                                placeholder="1-3, 5"
                            />
                        </div>
                    }.into_any()
                } else if mode.get() == "rotate" {
                    view! {
                        <div class="form-group">
                            <label class="form-label">"Góc xoay chiều kim đồng hồ:"</label>
                            <select
                                class="preset-select"
                                on:change=move |ev| set_rotation_deg.set(event_target_value(&ev))
                            >
                                <option value="90">"90°"</option>
                                <option value="180">"180°"</option>
                                <option value="270">"270°"</option>
                            </select>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
