use leptos::prelude::*;

#[component]
pub fn PdfToMarkdownView() -> impl IntoView {
    let (extract_tables, set_extract_tables) = signal(true);
    let (heading_scale, set_heading_scale) = signal(1.5f32);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Tùy chọn trích xuất bảng biểu:"</label>
                <div style="margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="checkbox"
                            checked=extract_tables
                            on:change=move |ev| set_extract_tables.set(event_target_checked(&ev))
                        />
                        "Tự động nhận diện và tái tạo bảng biểu sang cú pháp kẻ bảng Markdown"
                    </label>
                </div>
            </div>

            <div class="form-group">
                <label class="form-label">
                    {move || format!("Ngưỡng tỷ lệ font tiêu đề (Heading Scale): {:.1}x", heading_scale.get())}
                </label>
                <input
                    type="range"
                    min="1.0"
                    max="3.0"
                    step="0.1"
                    prop:value=move || heading_scale.get().to_string()
                    on:input=move |ev| {
                        if let Ok(val) = event_target_value(&ev).parse::<f32>() {
                            set_heading_scale.set(val);
                        }
                    }
                    class="form-range"
                />
            </div>
        </div>
    }
}
