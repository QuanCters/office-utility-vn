use leptos::prelude::*;

#[component]
pub fn PdfPageNumberHeaderView() -> impl IntoView {
    let (pattern, set_pattern) = signal("Trang {page}/{total}".to_string());
    let (position, set_position) = signal("bottom_center".to_string());
    let (skip_cover, set_skip_cover) = signal(false);
    let (_is_composing, set_is_composing) = signal(false);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Mẫu định dạng số trang / Bates:"</label>
                <input
                    class="form-input"
                    type="text"
                    prop:value=pattern
                    on:compositionstart=move |_| set_is_composing.set(true)
                    on:compositionend=move |_| set_is_composing.set(false)
                    on:input=move |ev| set_pattern.set(event_target_value(&ev))
                    placeholder="Trang {page}/{total} hoặc Bates {number:06}"
                />
            </div>

            <div class="form-group">
                <label class="form-label">"Vị trí hiển thị số trang:"</label>
                <select
                    class="preset-select"
                    on:change=move |ev| set_position.set(event_target_value(&ev))
                >
                    <option value="bottom_center">"Chính giữa phía dưới (Footer Center)"</option>
                    <option value="bottom_right">"Góc dưới bên phải (Footer Right)"</option>
                    <option value="bottom_left">"Góc dưới bên trái (Footer Left)"</option>
                    <option value="top_right">"Góc trên bên phải (Header Right)"</option>
                    <option value="top_center">"Chính giữa phía trên (Header Center)"</option>
                </select>
            </div>

            <div class="form-group">
                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=skip_cover
                        on:change=move |ev| set_skip_cover.set(event_target_checked(&ev))
                    />
                    "Bỏ qua trang bìa đầu tiên (Không đánh số trang 1)"
                </label>
            </div>
        </div>
    }
}
