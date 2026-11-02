use leptos::prelude::*;

#[component]
pub fn PdfMetadataScrubberView() -> impl IntoView {
    let (remove_info, set_remove_info) = signal(true);
    let (remove_xmp, set_remove_xmp) = signal(true);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Tùy chọn tẩy sạch metadata ẩn (Bảo mật thông tin):"</label>
                <div style="display: flex; flex-direction: column; gap: 8px; margin-top: 6px;">
                    <label class="radio-item">
                        <input
                            type="checkbox"
                            checked=remove_info
                            on:change=move |ev| set_remove_info.set(event_target_checked(&ev))
                        />
                        "Xóa sạch từ điển Info trong Trailer (Tác giả, Phần mềm tạo, Ngày tạo, Lịch sử sửa đổi)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="checkbox"
                            checked=remove_xmp
                            on:change=move |ev| set_remove_xmp.set(event_target_checked(&ev))
                        />
                        "Gỡ bỏ hoàn toàn luồng XML metadata XMP nhạy cảm khỏi PDF Catalog"
                    </label>
                </div>
            </div>
        </div>
    }
}
