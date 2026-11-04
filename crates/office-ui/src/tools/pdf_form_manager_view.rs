use leptos::prelude::*;

#[component]
pub fn PdfFormManagerView() -> impl IntoView {
    let (action, set_action) = signal("inspect".to_string());
    let (_export_fmt, set_export_fmt) = signal("json".to_string());
    let (fill_json, set_fill_json) = signal("{\n  \"HoVaTen\": \"Nguyen Van A\",\n  \"MaSoThue\": \"0101234567\"\n}".to_string());

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Tác vụ xử lý biểu mẫu (AcroForm):"</label>
                <div style="display: flex; gap: 12px; margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="form_action"
                            checked=move || action.get() == "inspect"
                            on:change=move |_| set_action.set("inspect".to_string())
                        />
                        "Khảo sát trường (Inspect)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="form_action"
                            checked=move || action.get() == "export_data"
                            on:change=move |_| set_action.set("export_data".to_string())
                        />
                        "Xuất dữ liệu (Export)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="form_action"
                            checked=move || action.get() == "auto_fill"
                            on:change=move |_| set_action.set("auto_fill".to_string())
                        />
                        "Điền tự động (Auto-fill)"
                    </label>
                </div>
            </div>

            {move || {
                if action.get() == "export_data" {
                    view! {
                        <div class="form-group">
                            <label class="form-label">"Định dạng xuất dữ liệu:"</label>
                            <select
                                class="preset-select"
                                on:change=move |ev| set_export_fmt.set(event_target_value(&ev))
                            >
                                <option value="json">"JSON (.json)"</option>
                                <option value="csv">"CSV (.csv)"</option>
                            </select>
                        </div>
                    }.into_any()
                } else if action.get() == "auto_fill" {
                    view! {
                        <div class="form-group">
                            <label class="form-label">"Dữ liệu mẫu cần điền (JSON key-value):"</label>
                            <textarea
                                class="form-input"
                                rows="4"
                                prop:value=fill_json
                                on:input=move |ev| set_fill_json.set(event_target_value(&ev))
                            />
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div style="color: var(--text-muted); font-size: 13px;">
                            "Tự động phát hiện toàn bộ các trường nhập liệu (Text, Checkbox, Choice), tọa độ Rect và giá trị hiện tại."
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
