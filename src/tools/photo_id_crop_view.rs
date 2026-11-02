use leptos::prelude::*;

#[component]
pub fn PhotoIdCropView(
    #[prop(into)] id_size: RwSignal<String>,
    #[prop(into)] sheet_layout: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Kích thước ảnh thẻ:"</label>
                <div style="display: flex; gap: 12px; margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="id_size"
                            checked=move || id_size.get() == "3x4"
                            on:change=move |_| id_size.set("3x4".to_string())
                        />
                        "3x4 cm (Hồ sơ, bằng lái xe)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="id_size"
                            checked=move || id_size.get() == "4x6"
                            on:change=move |_| id_size.set("4x6".to_string())
                        />
                        "4x6 cm (Hộ chiếu, CCCD)"
                    </label>
                </div>
            </div>

            <div class="form-group">
                <label class="form-label">"Dàn trang in:"</label>
                <select
                    class="form-select"
                    prop:value=sheet_layout
                    on:change=move |ev| sheet_layout.set(event_target_value(&ev))
                >
                    <option value="single">"Chỉ xuất 1 ảnh đơn lẻ"</option>
                    <option value="sheet_10x15">"Ghép nhiều ảnh lên khổ in 10x15 cm (300 DPI)"</option>
                </select>
            </div>
        </div>
    }
}
