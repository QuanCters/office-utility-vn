use leptos::prelude::*;

#[component]
pub fn VnLoanAmortizationView(
    principal: ReadSignal<String>,
    set_principal: WriteSignal<String>,
    annual_rate: ReadSignal<String>,
    set_annual_rate: WriteSignal<String>,
    term_months: ReadSignal<u32>,
    set_term_months: WriteSignal<u32>,
    method: ReadSignal<String>,
    set_method: WriteSignal<String>,
    result_json: ReadSignal<String>,
    _set_is_ime_composing: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">"Số tiền vay (VND):"</label>
                    <input
                        class="form-input"
                        type="text"
                        prop:value=principal
                        on:input=move |ev| {
                            set_principal.set(event_target_value(&ev));
                        }
                        placeholder="Ví dụ: 1000000000"
                    />
                </div>

                <div class="form-group">
                    <label class="form-label">"Lãi suất (%/năm):"</label>
                    <input
                        class="form-input"
                        type="text"
                        prop:value=annual_rate
                        on:input=move |ev| {
                            set_annual_rate.set(event_target_value(&ev));
                        }
                        placeholder="Ví dụ: 8.5"
                    />
                </div>

                <div class="form-group">
                    <label class="form-label">"Kỳ hạn vay (tháng):"</label>
                    <input
                        class="form-input"
                        type="number"
                        min="1"
                        prop:value=term_months
                        on:input=move |ev| {
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                set_term_months.set(val);
                            }
                        }
                    />
                </div>
            </div>

            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">"Phương thức trả nợ:"</label>
                    <div style="display: flex; gap: 16px; margin-top: 4px;">
                        <label class="radio-item">
                            <input
                                type="radio"
                                name="loan_method"
                                checked=move || method.get() == "ReducingBalance"
                                on:change=move |_| set_method.set("ReducingBalance".to_string())
                            />
                            "Dư nợ giảm dần (Gốc đều, lãi giảm)"
                        </label>
                        <label class="radio-item">
                            <input
                                type="radio"
                                name="loan_method"
                                checked=move || method.get() == "FixedAnnuity"
                                on:change=move |_| set_method.set("FixedAnnuity".to_string())
                            />
                            "Niên kim cố định (Trả góp đều mỗi tháng)"
                        </label>
                    </div>
                </div>
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
