use leptos::prelude::*;

#[component]
pub fn VietQrNapas247GeneratorView(
    bank_bin: ReadSignal<String>,
    set_bank_bin: WriteSignal<String>,
    account_number: ReadSignal<String>,
    set_account_number: WriteSignal<String>,
    amount: ReadSignal<String>,
    set_amount: WriteSignal<String>,
    message: ReadSignal<String>,
    set_message: WriteSignal<String>,
    qr_payload: ReadSignal<String>,
    set_is_ime_composing: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">"Ngân hàng thụ hưởng:"</label>
                    <select
                        class="form-select"
                        on:change=move |ev| {
                            set_bank_bin.set(event_target_value(&ev));
                        }
                    >
                        <option value="970436" selected=move || bank_bin.get() == "970436">"Vietcombank (970436)"</option>
                        <option value="970422" selected=move || bank_bin.get() == "970422">"MBBank (970422)"</option>
                        <option value="970407" selected=move || bank_bin.get() == "970407">"Techcombank (970407)"</option>
                        <option value="970415" selected=move || bank_bin.get() == "970415">"VietinBank (970415)"</option>
                        <option value="970418" selected=move || bank_bin.get() == "970418">"BIDV (970418)"</option>
                        <option value="970416" selected=move || bank_bin.get() == "970416">"ACB (970416)"</option>
                        <option value="970423" selected=move || bank_bin.get() == "970423">"TPBank (970423)"</option>
                        <option value="970432" selected=move || bank_bin.get() == "970432">"VPBank (970432)"</option>
                    </select>
                </div>

                <div class="form-group">
                    <label class="form-label">"Số tài khoản thụ hưởng:"</label>
                    <input
                        class="form-input"
                        type="text"
                        prop:value=account_number
                        on:input=move |ev| {
                            set_account_number.set(event_target_value(&ev));
                        }
                        placeholder="Ví dụ: 0011001234567"
                    />
                </div>
            </div>

            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">"Số tiền (VND - Tùy chọn):"</label>
                    <input
                        class="form-input"
                        type="text"
                        prop:value=amount
                        on:input=move |ev| {
                            set_amount.set(event_target_value(&ev));
                        }
                        placeholder="Ví dụ: 500000"
                    />
                </div>

                <div class="form-group">
                    <label class="form-label">"Nội dung chuyển khoản (Tùy chọn):"</label>
                    <input
                        class="form-input"
                        type="text"
                        prop:value=message
                        on:compositionstart=move |_| set_is_ime_composing.set(true)
                        on:compositionend=move |_| set_is_ime_composing.set(false)
                        on:input=move |ev| {
                            set_message.set(event_target_value(&ev));
                        }
                        placeholder="Ví dụ: Chuyen tien hoc"
                    />
                </div>
            </div>

            { move || {
                let res = qr_payload.get();
                if !res.is_empty() {
                    view! {
                        <div style="margin-top: 12px; padding: 14px; background: rgba(5, 150, 105, 0.1); border: 1px solid rgba(5, 150, 105, 0.3); border-radius: 6px;">
                            <div style="font-size: 11px; color: #10B981; font-weight: 600; margin-bottom: 4px;">"CHUỖI EMVCO VIETQR CHUẨN:"</div>
                            <div style="font-size: 13px; font-family: monospace; word-break: break-all; color: #F8FAFC;">{res}</div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
