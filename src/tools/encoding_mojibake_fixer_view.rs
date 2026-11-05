use leptos::prelude::*;

#[component]
pub fn EncodingMojibakeFixerView(
    #[prop(default = signal(true).0)] auto_detect: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_auto_detect: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || auto_detect.get()
                        on:change=move |ev| set_auto_detect.set(event_target_checked(&ev))
                    />
                    <span>"Tự động nhận diện và đảo ngược chu kỳ giải mã nhầm UTF-8 / Windows-1252 / ISO-8859-1"</span>
                </label>
            </div>

            <div style="font-size: 12px; color: var(--text-muted); margin-top: 10px;">
                "💡 Sửa triệt để các văn bản dính lỗi font như "<code style="color: #F8FAFC;">"Tiáº¿ng Viá»‡t"</code>" về lại "<code style="color: #10B981;">"Tiếng Việt"</code>" chuẩn đọc được."
            </div>
        </div>
    }
}
