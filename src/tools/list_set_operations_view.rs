use leptos::prelude::*;

#[component]
pub fn ListSetOperationsView(
    #[prop(default = signal("intersection".to_string()).0)] operation: ReadSignal<String>,
    #[prop(default = signal("intersection".to_string()).1)] set_operation: WriteSignal<String>,
    #[prop(default = signal(true).0)] case_sensitive: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_case_sensitive: WriteSignal<bool>,
    #[prop(default = signal(true).0)] trim_items: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_trim_items: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Phép toán tập hợp:"</label>
                <select
                    class="form-select"
                    on:change=move |ev| set_operation.set(event_target_value(&ev))
                    prop:value=move || operation.get()
                >
                    <option value="intersection">"Giao nhau (A ∩ B - Phần tử có ở CẢ HAI danh sách)"</option>
                    <option value="union">"Hợp nhất (A ∪ B - Toàn bộ phần tử, lọc trùng lặp)"</option>
                    <option value="difference">"Hiệu (A \\ B - Chỉ có ở danh sách A, không có ở B)"</option>
                    <option value="symmetric_difference">"Hiệu đối xứng (A △ B - Chỉ có ở một trong hai danh sách)"</option>
                </select>
            </div>

            <div class="options-row" style="margin-top: 10px; display: flex; flex-direction: column; gap: 8px;">
                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || case_sensitive.get()
                        on:change=move |ev| set_case_sensitive.set(event_target_checked(&ev))
                    />
                    <span>"Phân biệt chữ hoa / chữ thường (Case Sensitive)"</span>
                </label>

                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || trim_items.get()
                        on:change=move |ev| set_trim_items.set(event_target_checked(&ev))
                    />
                    <span>"Tự động xóa khoảng trắng thừa đầu và cuối mỗi dòng"</span>
                </label>
            </div>

            <div style="font-size: 11px; color: var(--text-muted); margin-top: 10px;">
                "💡 Xử lý tập hợp cực nhanh cho hàng trăm ngàn email, SĐT, mã định danh chỉ trong vài mili-giây."
            </div>
        </div>
    }
}
