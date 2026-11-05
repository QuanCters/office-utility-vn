use leptos::prelude::*;

#[component]
pub fn PdfRepairCorruptView() -> impl IntoView {
    let (scan_depth, set_scan_depth) = signal("linear_xref".to_string());

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Chế độ phục hồi file PDF lỗi:"</label>
                <div style="display: flex; gap: 16px; margin-top: 6px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="scan_depth"
                            checked=move || scan_depth.get() == "linear_xref"
                            on:change=move |_| set_scan_depth.set("linear_xref".to_string())
                        />
                        "Quét tuyến tính tái tạo bảng xref & trailer"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="scan_depth"
                            checked=move || scan_depth.get() == "salvage_objs"
                            on:change=move |_| set_scan_depth.set("salvage_objs".to_string())
                        />
                        "Trích xuất và bảo lưu các trang nguyên vẹn"
                    </label>
                </div>
            </div>
            <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">
                "💡 Thuật toán tự động tìm các cặp thẻ obj / endobj và tái tạo lại toàn bộ bảng tham chiếu chéo chỉ mục."
            </div>
        </div>
    }
}
