use leptos::prelude::*;

#[component]
pub fn PdfGrayscalePrintView() -> impl IntoView {
    let (convert_images, set_convert_images) = signal(true);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=convert_images
                        on:change=move |ev| set_convert_images.set(event_target_checked(&ev))
                    />
                    "Chuyển đổi toàn bộ hình ảnh nhúng (XObject) sang DeviceGray 8-bit"
                </label>
            </div>
            <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">
                "💡 Tiết kiệm tối đa mực in văn phòng bằng cách chuyển các luồng RGB/CMYK sang đơn sắc DeviceGray chuẩn."
            </div>
        </div>
    }
}
