use leptos::prelude::*;

#[component]
pub fn PdfBookletImpositionView() -> impl IntoView {
    let (mode, set_mode) = signal("saddle_stitch".to_string());
    let (paper_size, set_paper_size) = signal("A4".to_string());

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Kiểu dàn trang in:"</label>
                <select
                    class="form-select"
                    prop:value=mode
                    on:change=move |ev| set_mode.set(event_target_value(&ev))
                >
                    <option value="saddle_stitch">"Đóng quyển Saddle-Stitch (A4 -> A5)"</option>
                    <option value="two_up">"Ghép 2 trang trên 1 mặt (2-up)"</option>
                    <option value="four_up">"Ghép 4 trang trên 1 mặt (4-up)"</option>
                </select>
            </div>

            <div class="form-group">
                <label class="form-label">"Khổ giấy in đích:"</label>
                <select
                    class="form-select"
                    prop:value=paper_size
                    on:change=move |ev| set_paper_size.set(event_target_value(&ev))
                >
                    <option value="A4">"A4 (297 x 210 mm)"</option>
                    <option value="A3">"A3 (420 x 297 mm)"</option>
                </select>
            </div>
        </div>
    }
}
