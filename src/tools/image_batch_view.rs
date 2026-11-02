use leptos::prelude::*;

#[component]
pub fn ImageBatchView(
    #[prop(into)] target_format: RwSignal<String>,
    #[prop(into)] quality: RwSignal<u32>,
    #[prop(into)] max_width: RwSignal<String>,
    #[prop(into)] strip_exif: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Định dạng đích:"</label>
                <div style="display: flex; gap: 12px; margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="target_format"
                            checked=move || target_format.get() == "webp"
                            on:change=move |_| target_format.set("webp".to_string())
                        />
                        "WebP (Khuyến nghị web)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="target_format"
                            checked=move || target_format.get() == "avif"
                            on:change=move |_| target_format.set("avif".to_string())
                        />
                        "AVIF (Nén tối đa)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="target_format"
                            checked=move || target_format.get() == "jpg"
                            on:change=move |_| target_format.set("jpg".to_string())
                        />
                        "JPG (Tương thích)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="target_format"
                            checked=move || target_format.get() == "png"
                            on:change=move |_| target_format.set("png".to_string())
                        />
                        "PNG"
                    </label>
                </div>
            </div>

            <div class="options-row">
                <div class="form-group" style="flex: 1;">
                    <label class="form-label">
                        {move || format!("Chất lượng nén: {}%", quality.get())}
                    </label>
                    <input
                        type="range"
                        min="50"
                        max="100"
                        prop:value=quality
                        on:input=move |ev| {
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                quality.set(val);
                            }
                        }
                    />
                </div>

                <div class="form-group" style="flex: 1;">
                    <label class="form-label">"Thu nhỏ kích thước (Resize):"</label>
                    <select
                        class="form-select"
                        prop:value=max_width
                        on:change=move |ev| max_width.set(event_target_value(&ev))
                    >
                        <option value="none">"Giữ nguyên gốc"</option>
                        <option value="1920">"Full HD (1920px)"</option>
                        <option value="1280">"HD (1280px)"</option>
                        <option value="800">"Nhỏ (800px)"</option>
                    </select>
                </div>
            </div>

            <div class="form-group">
                <label class="checkbox-item" style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
                    <input
                        type="checkbox"
                        checked=strip_exif
                        on:change=move |ev| strip_exif.set(event_target_checked(&ev))
                    />
                    <span>"Xóa thông tin EXIF/GPS bảo vệ quyền riêng tư"</span>
                </label>
            </div>
        </div>
    }
}
