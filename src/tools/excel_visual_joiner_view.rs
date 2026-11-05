use leptos::prelude::*;

#[component]
pub fn ExcelVisualJoinerView(
    #[prop(default = signal("inner".to_string()).0)] join_type: ReadSignal<String>,
    #[prop(default = signal("inner".to_string()).1)] set_join_type: WriteSignal<String>,
    #[prop(default = signal("MaNV".to_string()).0)] key_a: ReadSignal<String>,
    #[prop(default = signal("MaNV".to_string()).1)] set_key_a: WriteSignal<String>,
    #[prop(default = signal("MaNV".to_string()).0)] key_b: ReadSignal<String>,
    #[prop(default = signal("MaNV".to_string()).1)] set_key_b: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Kiểu kết hợp (Join Type):"</label>
                <select
                    class="form-select"
                    on:change=move |ev| set_join_type.set(event_target_value(&ev))
                    prop:value=move || join_type.get()
                >
                    <option value="inner">"INNER JOIN (Chỉ lấy các dòng khớp khóa ở cả 2 bảng)"</option>
                    <option value="left">"LEFT JOIN (Giữ nguyên toàn bộ Bảng A, điền trống nếu Bảng B không có)"</option>
                    <option value="full_outer">"FULL OUTER JOIN (Lấy toàn bộ dòng của cả 2 bảng)"</option>
                    <option value="anti">"ANTI JOIN (Tìm các dòng có trong Bảng A nhưng thiếu trong Bảng B)"</option>
                </select>
            </div>

            <div class="options-row" style="margin-top: 10px;">
                <div class="form-group">
                    <label class="form-label">"Tên cột khóa ở Bảng A:"</label>
                    <input
                        class="form-input"
                        type="text"
                        prop:value=move || key_a.get()
                        on:input=move |ev| set_key_a.set(event_target_value(&ev))
                        placeholder="Ví dụ: Mã NV, ID, SĐT..."
                    />
                </div>

                <div class="form-group">
                    <label class="form-label">"Tên cột khóa ở Bảng B:"</label>
                    <input
                        class="form-input"
                        type="text"
                        prop:value=move || key_b.get()
                        on:input=move |ev| set_key_b.set(event_target_value(&ev))
                        placeholder="Ví dụ: Mã NV, CustomerID..."
                    />
                </div>
            </div>

            <div style="font-size: 12px; color: var(--text-muted); margin-top: 10px;">
                "💡 Thuật toán Hash Join siêu tốc trong bộ nhớ thay thế hoàn toàn các hàm VLOOKUP/XLOOKUP phức tạp."
            </div>
        </div>
    }
}
