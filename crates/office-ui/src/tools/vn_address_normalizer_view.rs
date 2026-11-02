use leptos::prelude::*;

#[component]
pub fn VnAddressNormalizerView(
    address_input: ReadSignal<String>,
    set_address_input: WriteSignal<String>,
    target_model: ReadSignal<String>,
    set_target_model: WriteSignal<String>,
    result_json: ReadSignal<String>,
    set_is_ime_composing: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Địa chỉ gõ tự do / viết tắt (ví dụ: hn, q.tb, tphcm):"</label>
                <input
                    class="form-input"
                    type="text"
                    prop:value=address_input
                    on:compositionstart=move |_| set_is_ime_composing.set(true)
                    on:compositionend=move |_| set_is_ime_composing.set(false)
                    on:input=move |ev| {
                        set_address_input.set(event_target_value(&ev));
                    }
                    placeholder="Ví dụ: 123 Hoàng Hoa Thám, q.tb, tphcm"
                />
            </div>

            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">"Mô hình địa giới hành chính:"</label>
                    <div style="display: flex; gap: 16px; margin-top: 4px;">
                        <label class="radio-item">
                            <input
                                type="radio"
                                name="target_model"
                                checked=move || target_model.get() == "ThreeTier"
                                on:change=move |_| set_target_model.set("ThreeTier".to_string())
                            />
                            "Mô hình 3 cấp truyền thống (Tỉnh - Huyện - Xã)"
                        </label>
                        <label class="radio-item">
                            <input
                                type="radio"
                                name="target_model"
                                checked=move || target_model.get() == "TwoTier"
                                on:change=move |_| set_target_model.set("TwoTier".to_string())
                            />
                            "Mô hình 2 cấp sau sáp nhập (Tỉnh - Xã mới)"
                        </label>
                    </div>
                </div>
            </div>

            { move || {
                let res = result_json.get();
                if !res.is_empty() {
                    view! {
                        <div style="margin-top: 12px; padding: 14px; background: rgba(30, 41, 59, 0.7); border: 1px solid rgba(148, 163, 184, 0.2); border-radius: 6px; font-family: monospace; white-space: pre-wrap; font-size: 13px; color: #E2E8F0;">
                            {res}
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
