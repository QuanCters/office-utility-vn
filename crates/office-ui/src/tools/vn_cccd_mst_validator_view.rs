use leptos::prelude::*;

#[component]
pub fn VnCccdMstValidatorView(
    input_value: ReadSignal<String>,
    set_input_value: WriteSignal<String>,
    result_json: ReadSignal<String>,
    set_is_ime_composing: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Nhập số CCCD (12 chữ số) hoặc Mã số thuế (10 hoặc 13 chữ số):"</label>
                <input
                    class="form-input"
                    type="text"
                    prop:value=input_value
                    on:compositionstart=move |_| set_is_ime_composing.set(true)
                    on:compositionend=move |_| set_is_ime_composing.set(false)
                    on:input=move |ev| {
                        set_input_value.set(event_target_value(&ev));
                    }
                    placeholder="Ví dụ: 001205012345 hoặc 0100109106"
                />
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
