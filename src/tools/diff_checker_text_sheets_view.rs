use leptos::prelude::*;

#[component]
pub fn DiffCheckerTextSheetsView(
    #[prop(default = signal(false).0)] ignore_whitespace: ReadSignal<bool>,
    #[prop(default = signal(false).1)] set_ignore_whitespace: WriteSignal<bool>,
    #[prop(default = signal(true).0)] case_sensitive: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_case_sensitive: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row" style="display: flex; flex-direction: column; gap: 8px;">
                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || ignore_whitespace.get()
                        on:change=move |ev| set_ignore_whitespace.set(event_target_checked(&ev))
                    />
                    <span>"Bỏ qua khoảng trắng thừa (Ignore Whitespace)"</span>
                </label>

                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || case_sensitive.get()
                        on:change=move |ev| set_case_sensitive.set(event_target_checked(&ev))
                    />
                    <span>"Phân biệt chữ hoa / chữ thường (Case Sensitive)"</span>
                </label>
            </div>

            <div style="font-size: 11px; color: var(--text-muted); margin-top: 10px;">
                "💡 Thuật toán Myers/Patience Diff nhận diện chính xác từng ô bảng tính bị thêm mới (+), xóa bỏ (-) hoặc sửa đổi (~)."
            </div>
        </div>
    }
}
