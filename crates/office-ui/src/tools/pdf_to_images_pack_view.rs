use leptos::prelude::*;

#[component]
pub fn PdfToImagesPackView() -> impl IntoView {
    let (format, set_format) = signal("png".to_string());
    let (dpi, set_dpi) = signal(150u32);
    let (quality, set_quality) = signal(85u8);
    let (prefix, set_prefix) = signal("trang_".to_string());

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Định dạng ảnh đầu ra:"</label>
                <div style="display: flex; gap: 16px; margin-top: 6px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="image_format"
                            checked=move || format.get() == "png"
                            on:change=move |_| set_format.set("png".to_string())
                        />
                        "PNG (Sắc nét, không suy hao, lý tưởng cho đồ họa & văn bản)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="image_format"
                            checked=move || format.get() == "jpeg"
                            on:change=move |_| set_format.set("jpeg".to_string())
                        />
                        "JPEG (Dung lượng nhẹ, tối ưu tài liệu scan)"
                    </label>
                </div>
            </div>

            <div class="form-group">
                <label class="form-label">"Độ phân giải kết xuất (DPI):"</label>
                <div style="display: flex; gap: 16px; margin-top: 6px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="image_dpi"
                            checked=move || dpi.get() == 150
                            on:change=move |_| set_dpi.set(150)
                        />
                        "150 DPI (Tiêu chuẩn màn hình / Trình chiếu)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="image_dpi"
                            checked=move || dpi.get() == 300
                            on:change=move |_| set_dpi.set(300)
                        />
                        "300 DPI (Chuẩn in ấn sắc nét cao)"
                    </label>
                </div>
            </div>

            {move || {
                if format.get() == "jpeg" {
                    view! {
                        <div class="form-group">
                            <label class="form-label">
                                {move || format!("Chất lượng nén JPEG: {}%", quality.get())}
                            </label>
                            <div style="display: flex; align-items: center; gap: 12px;">
                                <input
                                    type="range"
                                    min="60"
                                    max="100"
                                    step="5"
                                    prop:value=move || quality.get().to_string()
                                    on:input=move |ev| {
                                        if let Ok(val) = event_target_value(&ev).parse::<u8>() {
                                            set_quality.set(val);
                                        }
                                    }
                                    class="form-range"
                                    style="flex: 1;"
                                />
                                <span style="min-width: 45px; font-weight: 500; font-size: 13px; color: var(--text-secondary);">
                                    {move || format!("{}%", quality.get())}
                                </span>
                            </div>
                            <span class="form-hint" style="font-size: 12px; color: var(--text-muted); margin-top: 2px;">
                                "Mức đề xuất 80% - 90% cho sự cân bằng tốt nhất giữa kích thước và độ rõ nét."
                            </span>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            <div class="form-group">
                <label class="form-label">"Tiền tố tên tệp trong tệp nén ZIP:"</label>
                <div style="display: flex; align-items: center; gap: 8px; margin-top: 4px;">
                    <input
                        type="text"
                        class="form-control"
                        placeholder="Ví dụ: trang_, page_, ban_ve_"
                        prop:value=prefix
                        on:input=move |ev| set_prefix.set(event_target_value(&ev))
                        style="max-width: 280px;"
                    />
                    <span style="font-size: 13px; color: var(--text-muted);">
                        {move || {
                            let p = prefix.get();
                            let ext = if format.get() == "png" { "png" } else { "jpg" };
                            let p_str = if p.is_empty() { "trang_" } else { &p };
                            format!("=> {p_str}001.{ext}, {p_str}002.{ext}...")
                        }}
                    </span>
                </div>
                <span class="form-hint" style="font-size: 12px; color: var(--text-muted); margin-top: 4px; display: block;">
                    "Toàn bộ các tệp ảnh kết xuất sẽ được đóng gói tự động trong tệp .zip duy nhất để tiện tải về và chia sẻ."
                </span>
            </div>
        </div>
    }
}
