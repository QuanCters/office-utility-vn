use leptos::prelude::*;

#[component]
pub fn PdfArchiveLinearizeView() -> impl IntoView {
    let (enable_pdfa, set_enable_pdfa) = signal(true);
    let (_pdfa_conformance, set_pdfa_conformance) = signal("PDF/A-1b".to_string());
    let (enable_linearization, set_enable_linearization) = signal(true);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Chuẩn hóa lưu trữ dài hạn PDF/A:"</label>
                <div style="margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="checkbox"
                            checked=enable_pdfa
                            on:change=move |ev| set_enable_pdfa.set(event_target_checked(&ev))
                        />
                        "Nhúng siêu dữ liệu XML XMP và OutputIntents theo chuẩn quốc tế ISO 19005"
                    </label>
                </div>
            </div>

            {move || {
                if enable_pdfa.get() {
                    view! {
                        <div class="form-group">
                            <label class="form-label">"Cấp độ tuân thủ (Conformance Level):"</label>
                            <select
                                class="preset-select"
                                on:change=move |ev| set_pdfa_conformance.set(event_target_value(&ev))
                            >
                                <option value="PDF/A-1b">"PDF/A-1b (Visual Preservation - Khuyến nghị cho văn phòng VN)"</option>
                                <option value="PDF/A-2b">"PDF/A-2b (Hỗ trợ nén JPEG2000 và layer đồ họa trong suốt)"</option>
                            </select>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            <div class="form-group">
                <label class="form-label">"Tối ưu hóa phân phối Web (Linearization):"</label>
                <div style="margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="checkbox"
                            checked=enable_linearization
                            on:change=move |ev| set_enable_linearization.set(event_target_checked(&ev))
                        />
                        "Bật Fast Web View (Tái cấu trúc file để trình duyệt render tức thì trang đầu tiên khi đang tải)"
                    </label>
                </div>
            </div>
        </div>
    }
}
