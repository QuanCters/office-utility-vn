use leptos::prelude::*;

#[component]
pub fn ExcelMultiMergerView(
    #[prop(default = signal("append_rows".to_string()).0)] mode: ReadSignal<String>,
    #[prop(default = signal("append_rows".to_string()).1)] set_mode: WriteSignal<String>,
    #[prop(default = signal(true).0)] add_source_column: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_add_source_column: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Kiểu gộp bảng tính:"</label>
                <div style="display: flex; gap: 16px; margin-top: 6px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="merge_mode"
                            checked=move || mode.get() == "append_rows"
                            on:change=move |_| set_mode.set("append_rows".to_string())
                        />
                        <span>"Gộp nối tiếp dòng (Append Rows - cùng cột)"</span>
                    </label>

                    <label class="radio-item">
                        <input
                            type="radio"
                            name="merge_mode"
                            checked=move || mode.get() == "multi_sheet"
                            on:change=move |_| set_mode.set("multi_sheet".to_string())
                        />
                        <span>"Gộp thành nhiều Sheet trong 1 Workbook"</span>
                    </label>
                </div>
            </div>

            <div class="form-group" style="margin-top: 10px;">
                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=move || add_source_column.get()
                        on:change=move |ev| set_add_source_column.set(event_target_checked(&ev))
                    />
                    <span>"Thêm cột truy vết tên file nguồn vào đầu mỗi dòng"</span>
                </label>
            </div>

            <div style="font-size: 12px; color: var(--text-muted); margin-top: 10px;">
                "💡 Khi gộp nối tiếp dòng, dòng tiêu đề của các tệp sau sẽ tự động được bỏ qua để đảm bảo tính toàn vẹn dữ liệu."
            </div>
        </div>
    }
}
