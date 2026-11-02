use leptos::prelude::*;

#[component]
pub fn DataStructureConverterView(
    #[prop(default = signal("json".to_string()).0)] target_format: ReadSignal<String>,
    #[prop(default = signal("json".to_string()).1)] set_target_format: WriteSignal<String>,
    #[prop(default = signal("my_table".to_string()).0)] table_name: ReadSignal<String>,
    #[prop(default = signal("my_table".to_string()).1)] set_table_name: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">"Định dạng chuyển đổi đích:"</label>
                    <select
                        class="form-select"
                        on:change=move |ev| set_target_format.set(event_target_value(&ev))
                        prop:value=move || target_format.get()
                    >
                        <option value="json">"Mảng đối tượng JSON (JSON Array of Objects)"</option>
                        <option value="sql">"Câu lệnh SQL INSERT INTO"</option>
                        <option value="markdown">"Bảng kẻ chuẩn Markdown"</option>
                    </select>
                </div>

                { move || {
                    if target_format.get() == "sql" {
                        view! {
                            <div class="form-group">
                                <label class="form-label">"Tên bảng SQL:"</label>
                                <input
                                    class="form-input"
                                    type="text"
                                    prop:value=move || table_name.get()
                                    on:input=move |ev| set_table_name.set(event_target_value(&ev))
                                    placeholder="Ví dụ: users, products..."
                                />
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
            </div>

            <div style="font-size: 11px; color: var(--text-muted); margin-top: 10px;">
                "💡 Tự động nhận diện kiểu số, boolean, chuỗi và thoát ký tự SQL nháy đơn an toàn."
            </div>
        </div>
    }
}
