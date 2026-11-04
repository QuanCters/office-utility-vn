use leptos::prelude::*;

#[component]
pub fn PdfRedactionView() -> impl IntoView {
    let (sensitive_patterns, set_sensitive_patterns) = signal("CCCD, Lương, Mật khẩu".to_string());
    let (blackout, set_blackout) = signal(true);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Từ khóa / Chuỗi ký tự nhạy cảm cần xóa vĩnh viễn (phân cách bằng dấu phẩy):"</label>
                <input
                    class="form-input"
                    type="text"
                    prop:value=sensitive_patterns
                    on:input=move |ev| set_sensitive_patterns.set(event_target_value(&ev))
                    placeholder="VD: 079090012345, matkhau123, luong_chinh"
                />
            </div>

            <div class="form-group">
                <label class="form-label">"Tùy chọn hiển thị bảo mật:"</label>
                <div style="margin-top: 6px;">
                    <label class="radio-item">
                        <input
                            type="checkbox"
                            checked=blackout
                            on:change=move |ev| set_blackout.set(event_target_checked(&ev))
                        />
                        "Phủ hộp đen (Blackout Box) che chắn các vùng hình học đã che giấu"
                    </label>
                </div>
            </div>

            <div style="color: var(--text-muted); font-size: 13px; line-height: 1.5; margin-top: 8px;">
                "⚠️ Cơ chế Redaction On-Device sẽ loại bỏ triệt để các byte chuỗi ký tự khỏi mã nguồn PDF stream, không chỉ phủ màu bề mặt, đảm bảo không thể bôi đen hay khôi phục ngược."
            </div>
        </div>
    }
}
