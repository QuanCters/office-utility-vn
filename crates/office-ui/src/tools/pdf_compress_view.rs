use leptos::prelude::*;

#[component]
pub fn PdfCompressView() -> impl IntoView {
    let (target_dpi, set_target_dpi) = signal(150u32);
    let (strip_unreferenced, set_strip_unreferenced) = signal(true);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Mức nén và tái lấy mẫu ảnh nhúng (DPI):"</label>
                <div style="display: flex; gap: 12px; margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="compress_dpi"
                            checked=move || target_dpi.get() == 72
                            on:change=move |_| set_target_dpi.set(72)
                        />
                        "72 DPI (Siêu nhẹ, gửi Email/Web)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="compress_dpi"
                            checked=move || target_dpi.get() == 150
                            on:change=move |_| set_target_dpi.set(150)
                        />
                        "150 DPI (Chuẩn màn hình sắc nét)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="compress_dpi"
                            checked=move || target_dpi.get() == 300
                            on:change=move |_| set_target_dpi.set(300)
                        />
                        "300 DPI (In ấn chất lượng cao)"
                    </label>
                </div>
            </div>

            <div class="form-group">
                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=strip_unreferenced
                        on:change=move |ev| set_strip_unreferenced.set(event_target_checked(&ev))
                    />
                    "Loại bỏ các đối tượng mồ côi (Unreferenced Objects) để tối ưu dung lượng"
                </label>
            </div>
        </div>
    }
}
