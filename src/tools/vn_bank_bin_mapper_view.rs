use leptos::prelude::*;

#[component]
pub fn VnBankBinMapperView(
    query: ReadSignal<String>,
    set_query: WriteSignal<String>,
    result_json: ReadSignal<String>,
    set_is_ime_composing: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Tìm kiếm ngân hàng (Nhập mã BIN, tên viết tắt VCB/MB/TCB, SWIFT hoặc tên đầy đủ):"</label>
                <input
                    class="form-input"
                    type="text"
                    prop:value=query
                    on:compositionstart=move |_| set_is_ime_composing.set(true)
                    on:compositionend=move |_| set_is_ime_composing.set(false)
                    on:input=move |ev| {
                        set_query.set(event_target_value(&ev));
                    }
                    placeholder="Ví dụ: 970436 hoặc Vietcombank hoặc BFTVVNVX..."
                />
            </div>

            { move || {
                let res = result_json.get();
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
