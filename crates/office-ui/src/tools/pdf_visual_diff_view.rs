use leptos::prelude::*;

#[component]
pub fn PdfVisualDiffView() -> impl IntoView {
    let (mode, set_mode) = signal("text_diff".to_string());
    let (highlight_color, set_highlight_color) = signal("#rose-600".to_string());

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Chế độ đối soát tài liệu:"</label>
                <div style="display: flex; gap: 16px; margin-top: 6px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="diff_mode"
                            checked=move || mode.get() == "text_diff"
                            on:change=move |_| set_mode.set("text_diff".to_string())
                        />
                        "Text Diff (So sánh câu chữ nhanh)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="diff_mode"
                            checked=move || mode.get() == "visual_pixel_diff"
                            on:change=move |_| set_mode.set("visual_pixel_diff".to_string())
                        />
                        "Visual Pixel Diff (So sánh hình ảnh từng pixel)"
                    </label>
                </div>
            </div>

            <div class="form-group">
                <label class="form-label">"Màu tô nổi bật sai khác:"</label>
                <select
                    class="form-select"
                    prop:value=highlight_color
                    on:change=move |ev| set_highlight_color.set(event_target_value(&ev))
                >
                    <option value="#rose-600">"Đỏ cảnh báo (#rose-600)"</option>
                    <option value="#amber-500">"Vàng cam (#amber-500)"</option>
                </select>
            </div>
        </div>
    }
}
