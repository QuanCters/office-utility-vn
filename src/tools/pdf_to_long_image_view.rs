use leptos::prelude::*;

#[component]
pub fn PdfToLongImageView() -> impl IntoView {
    let (output_format, set_output_format) = signal("png".to_string());
    let (dpi, set_dpi) = signal(150u32);
    let (add_divider, set_add_divider) = signal(true);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Định dạng ảnh đầu ra:"</label>
                <select
                    class="form-select"
                    prop:value=output_format
                    on:change=move |ev| set_output_format.set(event_target_value(&ev))
                >
                    <option value="png">"PNG (Độ nét cao, không nén)"</option>
                    <option value="webp">"WebP (Dung lượng siêu nhẹ)"</option>
                    <option value="jpg">"JPG (Tương thích phổ thông)"</option>
                </select>
            </div>

            <div class="form-group">
                <label class="form-label">
                    {move || format!("Độ phân giải kết xuất: {} DPI", dpi.get())}
                </label>
                <input
                    type="range"
                    min="100"
                    max="300"
                    step="25"
                    prop:value=move || dpi.get().to_string()
                    on:input=move |ev| {
                        if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                            set_dpi.set(val);
                        }
                    }
                    class="form-range"
                />
            </div>

            <div class="form-group">
                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=add_divider
                        on:change=move |ev| set_add_divider.set(event_target_checked(&ev))
                    />
                    "Thêm đường kẻ mảnh phân cách giữa các trang"
                </label>
            </div>
        </div>
    }
}
