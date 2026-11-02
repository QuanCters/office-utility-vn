use leptos::prelude::*;

#[component]
pub fn VnWorkingDaysCalcView(
    start_date: ReadSignal<String>,
    set_start_date: WriteSignal<String>,
    end_date: ReadSignal<String>,
    set_end_date: WriteSignal<String>,
    exclude_saturday: ReadSignal<bool>,
    set_exclude_saturday: WriteSignal<bool>,
    result_json: ReadSignal<String>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">"Ngày bắt đầu (YYYY-MM-DD):"</label>
                    <input
                        class="form-input"
                        type="date"
                        prop:value=start_date
                        on:input=move |ev| {
                            set_start_date.set(event_target_value(&ev));
                        }
                    />
                </div>

                <div class="form-group">
                    <label class="form-label">"Ngày kết thúc (YYYY-MM-DD):"</label>
                    <input
                        class="form-input"
                        type="date"
                        prop:value=end_date
                        on:input=move |ev| {
                            set_end_date.set(event_target_value(&ev));
                        }
                    />
                </div>
            </div>

            <div class="options-row">
                <div class="form-group">
                    <label class="radio-item">
                        <input
                            type="checkbox"
                            checked=exclude_saturday
                            on:change=move |ev| set_exclude_saturday.set(event_target_checked(&ev))
                        />
                        "Nghỉ Thứ Bảy và Chủ Nhật (bỏ chọn nếu làm việc Thứ Bảy)"
                    </label>
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
