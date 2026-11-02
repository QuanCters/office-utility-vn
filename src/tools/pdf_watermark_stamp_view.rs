use leptos::prelude::*;

#[component]
pub fn PdfWatermarkStampView() -> impl IntoView {
    let (text, set_text) = signal("BẢN QUYỀN - KHÔNG SAO CHÉP".to_string());
    let (opacity, set_opacity) = signal(0.3f32);
    let (angle, set_angle) = signal(45.0f32);
    let (_position, set_position) = signal("center".to_string());
    let (_is_composing, set_is_composing) = signal(false);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Nội dung chữ Watermark bản quyền:"</label>
                <input
                    class="form-input"
                    type="text"
                    prop:value=text
                    on:compositionstart=move |_| set_is_composing.set(true)
                    on:compositionend=move |_| set_is_composing.set(false)
                    on:input=move |ev| set_text.set(event_target_value(&ev))
                    placeholder="BẢN QUYỀN - KHÔNG SAO CHÉP"
                />
            </div>

            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">
                        {move || format!("Độ mờ đục (Opacity): {:.0}%", opacity.get() * 100.0)}
                    </label>
                    <input
                        type="range"
                        min="0.05"
                        max="1.0"
                        step="0.05"
                        prop:value=move || opacity.get().to_string()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                set_opacity.set(v);
                            }
                        }
                        class="form-range"
                    />
                </div>

                <div class="form-group">
                    <label class="form-label">
                        {move || format!("Góc xoay chữ: {:.0}°", angle.get())}
                    </label>
                    <input
                        type="range"
                        min="0"
                        max="90"
                        step="5"
                        prop:value=move || angle.get().to_string()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                set_angle.set(v);
                            }
                        }
                        class="form-range"
                    />
                </div>
            </div>

            <div class="form-group">
                <label class="form-label">"Vị trí con dấu / watermark:"</label>
                <select
                    class="preset-select"
                    on:change=move |ev| set_position.set(event_target_value(&ev))
                >
                    <option value="center">"Chính giữa trang (Center)"</option>
                    <option value="bottom_right">"Góc dưới bên phải (Bottom Right)"</option>
                    <option value="bottom_left">"Góc dưới bên trái (Bottom Left)"</option>
                </select>
            </div>
        </div>
    }
}
