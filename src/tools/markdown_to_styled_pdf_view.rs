use leptos::prelude::*;

#[component]
pub fn MarkdownToStyledPdfView() -> impl IntoView {
    let (page_size, set_page_size) = signal("A4".to_string());
    let (theme, set_theme) = signal("Hành chính công vụ VN".to_string());

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Khổ trang in:"</label>
                <select
                    class="form-select"
                    prop:value=page_size
                    on:change=move |ev| set_page_size.set(event_target_value(&ev))
                >
                    <option value="A4">"A4 (Chuẩn hành chính văn phòng)"</option>
                    <option value="A5">"A5 (Sổ tay, tờ rơi)"</option>
                    <option value="Letter">"Letter (Chuẩn quốc tế)"</option>
                </select>
            </div>

            <div class="form-group">
                <label class="form-label">"Chủ đề dàn trang (Theme):"</label>
                <select
                    class="form-select"
                    prop:value=theme
                    on:change=move |ev| set_theme.set(event_target_value(&ev))
                >
                    <option value="Hành chính công vụ VN">"Hành chính công vụ VN (Times/Serif, căn lề chuẩn)"</option>
                    <option value="Tài liệu kỹ thuật hiện đại">"Tài liệu kỹ thuật hiện đại (Sans-serif, code highlight)"</option>
                    <option value="Sách báo">"Sách báo & Tạp chí (Trang trọng, tiêu đề lớn)"</option>
                </select>
            </div>
            <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">
                "💡 Tự động nhận dạng phông chữ tiếng Việt Unicode, chia cột bảng và đánh số trang chân trang."
            </div>
        </div>
    }
}
