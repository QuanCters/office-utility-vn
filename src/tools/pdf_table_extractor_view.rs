use leptos::prelude::*;

#[component]
pub fn PdfTableExtractorView() -> impl IntoView {
    let (page_range, set_page_range) = signal(String::new());
    let (output_csv, set_output_csv) = signal(true);
    let (is_ime_composing, set_is_ime_composing) = signal(false);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Khoảng trang cần trích xuất bảng:"</label>
                <input
                    type="text"
                    class="form-input"
                    prop:value=page_range
                    on:compositionstart=move |_| set_is_ime_composing.set(true)
                    on:compositionend=move |_| set_is_ime_composing.set(false)
                    on:input=move |ev| {
                        if !is_ime_composing.get() {
                            set_page_range.set(event_target_value(&ev));
                        }
                    }
                    placeholder="Để trống để trích xuất toàn bộ trang (hoặc: 1-3, 5)"
                />
            </div>

            <div class="form-group">
                <label class="radio-item">
                    <input
                        type="checkbox"
                        checked=output_csv
                        on:change=move |ev| set_output_csv.set(event_target_checked(&ev))
                    />
                    "Đồng thời xuất thêm bản sao tệp dữ liệu CSV (.csv)"
                </label>
            </div>
            <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">
                "💡 Tự động phân tích tọa độ đường kẻ vector và phân nhóm dữ liệu thành cột/hàng bảng tính Excel (.xlsx)."
            </div>
        </div>
    }
}
