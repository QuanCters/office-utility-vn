use leptos::prelude::*;

#[component]
pub fn ExcelCleanerNormalizerView(
    #[prop(default = signal(true).0)] normalize_dates: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_normalize_dates: WriteSignal<bool>,
    #[prop(default = signal(true).0)] split_vietnamese_names: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_split_vietnamese_names: WriteSignal<bool>,
    #[prop(default = signal(true).0)] trim_spaces: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_trim_spaces: WriteSignal<bool>,
    #[prop(default = signal(true).0)] deduplicate: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_deduplicate: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row" style="display: flex; flex-direction: column; gap: 8px;">
                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || normalize_dates.get()
                        on:change=move |ev| set_normalize_dates.set(event_target_checked(&ev))
                    />
                    <span>"Đưa ngày tháng hỗn hợp về chuẩn DD/MM/YYYY"</span>
                </label>

                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || split_vietnamese_names.get()
                        on:change=move |ev| set_split_vietnamese_names.set(event_target_checked(&ev))
                    />
                    <span>"Tách cột Họ / Tên đệm / Tên theo quy tắc họ kép tiếng Việt"</span>
                </label>

                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || trim_spaces.get()
                        on:change=move |ev| set_trim_spaces.set(event_target_checked(&ev))
                    />
                    <span>"Xóa khoảng trắng thừa và viết hoa chữ cái đầu tên riêng"</span>
                </label>

                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || deduplicate.get()
                        on:change=move |ev| set_deduplicate.set(event_target_checked(&ev))
                    />
                    <span>"Loại bỏ dòng dữ liệu trùng lặp"</span>
                </label>
            </div>

            <div style="font-size: 11px; color: var(--text-muted); margin-top: 10px;">
                "💡 Nhận diện chính xác họ kép: Nguyễn Phước, Phan Huy, Hoàng Phủ, Võ Đình, Đặng Vũ..."
            </div>
        </div>
    }
}
