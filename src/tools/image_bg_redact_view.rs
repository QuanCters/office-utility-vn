use leptos::prelude::*;

#[component]
pub fn ImageBgRedactView(
    #[prop(into)] redact_type: RwSignal<String>,
    #[prop(into)] auto_detect: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Kiểu che mờ dữ liệu nhạy cảm:"</label>
                <div style="display: flex; gap: 12px; margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="redact_type"
                            checked=move || redact_type.get() == "pixelate"
                            on:change=move |_| redact_type.set("pixelate".to_string())
                        />
                        "Pixelate (Khảm điểm ô vuông)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="redact_type"
                            checked=move || redact_type.get() == "blur"
                            on:change=move |_| redact_type.set("blur".to_string())
                        />
                        "Gaussian Blur (Mờ mượt)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="redact_type"
                            checked=move || redact_type.get() == "blackout"
                            on:change=move |_| redact_type.set("blackout".to_string())
                        />
                        "Hộp đen che tuyệt đối"
                    </label>
                </div>
            </div>

            <div class="form-group">
                <label class="checkbox-item" style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
                    <input
                        type="checkbox"
                        checked=auto_detect
                        on:change=move |ev| auto_detect.set(event_target_checked(&ev))
                    />
                    <span>"Tự động nhận diện vùng số CMND/CCCD và mã QR để che mờ"</span>
                </label>
            </div>
        </div>
    }
}
