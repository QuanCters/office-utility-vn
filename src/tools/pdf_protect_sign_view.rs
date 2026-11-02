use leptos::prelude::*;

#[component]
pub fn PdfProtectSignView() -> impl IntoView {
    let (action, set_action) = signal("protect".to_string());
    let (password, set_password) = signal(String::new());
    let (_encryption, set_encryption) = signal("AES-256".to_string());
    let (_is_composing, set_is_composing) = signal(false);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Hành động:"</label>
                <div style="display: flex; gap: 12px; margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="protect_action"
                            checked=move || action.get() == "protect"
                            on:change=move |_| set_action.set("protect".to_string())
                        />
                        "Đặt mật khẩu bảo vệ"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="protect_action"
                            checked=move || action.get() == "unlock"
                            on:change=move |_| set_action.set("unlock".to_string())
                        />
                        "Gỡ bỏ mật khẩu (Mở khóa)"
                    </label>
                </div>
            </div>

            <div class="form-group">
                <label class="form-label">
                    {move || if action.get() == "protect" {
                        "Mật khẩu bảo vệ cần thiết lập:"
                    } else {
                        "Mật khẩu hiện tại để mở khóa:"
                    }}
                </label>
                <input
                    class="form-input"
                    type="password"
                    prop:value=password
                    on:compositionstart=move |_| set_is_composing.set(true)
                    on:compositionend=move |_| set_is_composing.set(false)
                    on:input=move |ev| set_password.set(event_target_value(&ev))
                    placeholder="Nhập mật khẩu..."
                />
            </div>

            {move || {
                if action.get() == "protect" {
                    view! {
                        <div class="form-group">
                            <label class="form-label">"Chuẩn mã hóa:"</label>
                            <select
                                class="preset-select"
                                on:change=move |ev| set_encryption.set(event_target_value(&ev))
                            >
                                <option value="AES-256">"AES-256 (Khuyến nghị chuẩn quân sự)"</option>
                                <option value="AES-128">"AES-128"</option>
                            </select>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
