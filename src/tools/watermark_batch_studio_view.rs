use leptos::prelude::*;

#[component]
pub fn WatermarkBatchStudioView(
    #[prop(into)] watermark_type: RwSignal<String>,
    #[prop(into)] opacity: RwSignal<f32>,
    #[prop(into)] text: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Kiểu đóng dấu bản quyền:"</label>
                <div style="display: flex; gap: 12px; margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="watermark_type"
                            checked=move || watermark_type.get() == "tiled"
                            on:change=move |_| watermark_type.set("tiled".to_string())
                        />
                        "Lưới chéo lặp lại toàn bức ảnh (Tiled)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="watermark_type"
                            checked=move || watermark_type.get() == "single_anchor"
                            on:change=move |_| watermark_type.set("single_anchor".to_string())
                        />
                        "Ở góc dưới bên phải (Bottom Right)"
                    </label>
                </div>
            </div>

            <div class="form-group">
                <label class="form-label">"Nội dung chữ bản quyền:"</label>
                <input
                    class="form-input"
                    type="text"
                    prop:value=text
                    on:input=move |ev| text.set(event_target_value(&ev))
                    placeholder="Ví dụ: BẢN QUYỀN - KHÔNG SAO CHÉP"
                />
            </div>

            <div class="form-group">
                <label class="form-label">
                    {move || format!("Độ mờ Alpha: {:.0}%", opacity.get() * 100.0)}
                </label>
                <input
                    type="range"
                    min="10"
                    max="100"
                    prop:value=move || (opacity.get() * 100.0) as u32
                    on:input=move |ev| {
                        if let Ok(val) = event_target_value(&ev).parse::<f32>() {
                            opacity.set(val / 100.0);
                        }
                    }
                />
            </div>
        </div>
    }
}
