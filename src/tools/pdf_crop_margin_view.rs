use leptos::prelude::*;

#[component]
pub fn PdfCropMarginView() -> impl IntoView {
    let (margin_padding_pt, set_margin_padding_pt) = signal(10.0f32);
    let (uniform_all, set_uniform_all) = signal(false);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">
                    {move || format!("Khoảng đệm an toàn sau khi cắt: {:.0} pt", margin_padding_pt.get())}
                </label>
                <input
                    type="range"
                    min="0"
                    max="50"
                    step="2"
                    prop:value=move || margin_padding_pt.get().to_string()
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                            set_margin_padding_pt.set(v);
                        }
                    }
                    class="form-range"
                />
            </div>

            <div class="form-group">
                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=uniform_all
                        on:change=move |ev| set_uniform_all.set(event_target_checked(&ev))
                    />
                    "Áp dụng đồng bộ khung cắt (CropBox) lớn nhất cho toàn bộ các trang"
                </label>
            </div>
        </div>
    }
}
