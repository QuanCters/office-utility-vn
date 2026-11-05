use leptos::prelude::*;

#[component]
pub fn SqlFormatterLinterView(
    #[prop(default = signal("postgres".to_string()).0)] dialect: ReadSignal<String>,
    #[prop(default = signal("postgres".to_string()).1)] set_dialect: WriteSignal<String>,
    #[prop(default = signal(true).0)] uppercase_keywords: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_uppercase_keywords: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">"Phương ngữ SQL (Dialect):"</label>
                    <select
                        class="form-select"
                        on:change=move |ev| {
                            set_dialect.set(event_target_value(&ev));
                        }
                        prop:value=move || dialect.get()
                    >
                        <option value="postgres">"PostgreSQL"</option>
                        <option value="mysql">"MySQL / MariaDB"</option>
                        <option value="sqlite">"SQLite"</option>
                        <option value="tsql">"Microsoft SQL Server (T-SQL)"</option>
                    </select>
                </div>

                <div class="form-group">
                    <label class="form-label">"Tùy chọn chuẩn hóa từ khóa:"</label>
                    <div style="display: flex; gap: 12px; margin-top: 6px;">
                        <label class="radio-item">
                            <input
                                type="checkbox"
                                checked=uppercase_keywords
                                on:change=move |ev| set_uppercase_keywords.set(event_target_checked(&ev))
                            />
                            "Viết hoa toàn bộ từ khóa SQL (SELECT, FROM, JOIN,...)"
                        </label>
                    </div>
                </div>
            </div>

            <div style="font-size: 12px; color: var(--text-muted); margin-top: 8px;">
                "💡 Linter tích hợp tự động phân tích cú pháp AST, phát hiện ngoặc đơn lồng nhau chưa đóng, lỗi vị trí WHERE/FROM và các chuỗi literal chưa đóng nháy."
            </div>
        </div>
    }
}
