use leptos::prelude::*;

#[component]
pub fn GrossNetSalaryCalculatorView(
    salary_input: ReadSignal<String>,
    set_salary_input: WriteSignal<String>,
    region: ReadSignal<u8>,
    set_region: WriteSignal<u8>,
    dependents: ReadSignal<u32>,
    set_dependents: WriteSignal<u32>,
    result_json: ReadSignal<String>,
    set_is_ime_composing: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Mức lương Gross (VND):"</label>
                <input
                    class="form-input"
                    type="text"
                    prop:value=salary_input
                    on:compositionstart=move |_| set_is_ime_composing.set(true)
                    on:compositionend=move |_| set_is_ime_composing.set(false)
                    on:input=move |ev| {
                        set_salary_input.set(event_target_value(&ev));
                    }
                    placeholder="Ví dụ: 30000000"
                />
            </div>

            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">"Vùng lương tối thiểu:"</label>
                    <select
                        class="form-select"
                        on:change=move |ev| {
                            if let Ok(val) = event_target_value(&ev).parse::<u8>() {
                                set_region.set(val);
                            }
                        }
                    >
                        <option value="1" selected=move || region.get() == 1>"Vùng I (Hà Nội, TP.HCM, Hải Phòng...)"</option>
                        <option value="2" selected=move || region.get() == 2>"Vùng II (Đà Nẵng, Cần Thơ, Nha Trang...)"</option>
                        <option value="3" selected=move || region.get() == 3>"Vùng III (Các tỉnh/thành còn lại)"</option>
                        <option value="4" selected=move || region.get() == 4>"Vùng IV (Vùng nông thôn, khó khăn)"</option>
                    </select>
                </div>

                <div class="form-group">
                    <label class="form-label">"Số người phụ thuộc:"</label>
                    <input
                        class="form-input"
                        type="number"
                        min="0"
                        prop:value=dependents
                        on:input=move |ev| {
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                set_dependents.set(val);
                            }
                        }
                    />
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
