use leptos::prelude::*;

#[component]
pub fn VnEinvoiceXmlReaderView(
    xml_content: ReadSignal<String>,
    set_xml_content: WriteSignal<String>,
    verify_signature: ReadSignal<bool>,
    set_verify_signature: WriteSignal<bool>,
    invoice_json: ReadSignal<String>,
    set_is_ime_composing: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Nội dung hoặc dán XML hóa đơn điện tử:"</label>
                <textarea
                    class="form-textarea"
                    rows="6"
                    prop:value=xml_content
                    on:compositionstart=move |_| set_is_ime_composing.set(true)
                    on:compositionend=move |_| set_is_ime_composing.set(false)
                    on:input=move |ev| {
                        set_xml_content.set(event_target_value(&ev));
                    }
                    placeholder="Dán nội dung XML hóa đơn (Thông tư 78/2021 hoặc Nghị định 123/2020)..."
                />
            </div>

            <div class="options-row">
                <div class="form-group">
                    <label class="radio-item">
                        <input
                            type="checkbox"
                            checked=verify_signature
                            on:change=move |ev| set_verify_signature.set(event_target_checked(&ev))
                        />
                        "Kiểm tra tính hợp lệ thẻ chữ ký số <ds:Signature>"
                    </label>
                </div>
            </div>

            { move || {
                let res = invoice_json.get();
                if !res.is_empty() {
                    view! {
                        <div style="margin-top: 12px; padding: 14px; background: rgba(30, 41, 59, 0.7); border: 1px solid rgba(148, 163, 184, 0.2); border-radius: 6px; font-family: monospace; white-space: pre-wrap; font-size: 13px; color: #E2E8F0; max-height: 250px; overflow-y: auto;">
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
