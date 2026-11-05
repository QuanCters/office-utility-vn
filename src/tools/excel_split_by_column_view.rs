use leptos::prelude::*;

#[component]
pub fn ExcelSplitByColumnView(
    #[prop(default = signal("MaPhongBan".to_string()).0)] split_column_name: ReadSignal<String>,
    #[prop(default = signal("MaPhongBan".to_string()).1)] set_split_column_name: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Tên cột dùng làm khóa phân nhóm tách tệp:"</label>
                <input
                    class="form-input"
                    type="text"
                    prop:value=move || split_column_name.get()
                    on:input=move |ev| set_split_column_name.set(event_target_value(&ev))
                    placeholder="Ví dụ: Mã Phòng Ban, Khu Vực, Chi Nhánh, Tỉnh Thành..."
                />
            </div>

            <div style="font-size: 12px; color: var(--text-muted); margin-top: 10px;">
                "💡 Mỗi giá trị duy nhất trong cột này sẽ được tách thành một file Excel/CSV riêng biệt, bảo tồn nguyên vẹn dòng tiêu đề gốc."
            </div>
        </div>
    }
}
