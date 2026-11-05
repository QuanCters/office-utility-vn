use leptos::prelude::*;

#[component]
pub fn TextPiiAnonymizerView(
    #[prop(default = signal(true).0)] mask_cccd: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_mask_cccd: WriteSignal<bool>,
    #[prop(default = signal(true).0)] mask_phone: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_mask_phone: WriteSignal<bool>,
    #[prop(default = signal(true).0)] mask_mst: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_mask_mst: WriteSignal<bool>,
    #[prop(default = signal(true).0)] mask_bank: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_mask_bank: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row" style="display: flex; flex-direction: column; gap: 8px;">
                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || mask_cccd.get()
                        on:change=move |ev| set_mask_cccd.set(event_target_checked(&ev))
                    />
                    <span>"Ẩn số CMND / CCCD 12 số ("[CCCD_ẨN]")"</span>
                </label>

                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || mask_phone.get()
                        on:change=move |ev| set_mask_phone.set(event_target_checked(&ev))
                    />
                    <span>"Ẩn số điện thoại di động VN 10 số 09x, 08x, 03x, 07x ("[SĐT_ẨN]")"</span>
                </label>

                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || mask_mst.get()
                        on:change=move |ev| set_mask_mst.set(event_target_checked(&ev))
                    />
                    <span>"Ẩn Mã số thuế 10/13 số ("[MST_ẨN]")"</span>
                </label>

                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || mask_bank.get()
                        on:change=move |ev| set_mask_bank.set(event_target_checked(&ev))
                    />
                    <span>"Ẩn số tài khoản ngân hàng ("[STK_ẨN]")"</span>
                </label>
            </div>

            <div style="font-size: 12px; color: var(--text-muted); margin-top: 10px;">
                "🔒 Cam kết 100% On-Device: Dữ liệu nhạy cảm được xóa sạch khỏi bộ nhớ RAM (Zeroize Memory Scrubbing) ngay sau khi xử lý."
            </div>
        </div>
    }
}
