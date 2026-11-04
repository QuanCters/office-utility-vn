use leptos::prelude::*;

#[component]
pub fn PdfPkiSignView() -> impl IntoView {
    let (action, set_action) = signal("sign".to_string());
    let (_signer_name, set_signer_name) = signal("Văn Phòng Điện Tử".to_string());
    let (_reason, set_reason) = signal("Xác nhận phê duyệt tài liệu".to_string());
    let (_location, set_location) = signal("Hà Nội, Việt Nam".to_string());
    let (_cert_cn, set_cert_cn) = signal("VN-PKI Qualified Digital Signature".to_string());

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Tác vụ Chữ ký số:"</label>
                <div style="display: flex; gap: 16px; margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="pki_action"
                            value="sign"
                            checked=move || action.get() == "sign"
                            on:change=move |_| set_action.set("sign".to_string())
                        />
                        "Gắn chữ ký số PKI (Sign)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="pki_action"
                            value="verify"
                            checked=move || action.get() == "verify"
                            on:change=move |_| set_action.set("verify".to_string())
                        />
                        "Xác thực & Giám định Toàn vẹn (Verify)"
                    </label>
                </div>
            </div>

            {move || {
                if action.get() == "sign" {
                    view! {
                        <div class="pki-sign-fields">
                            <div class="form-group">
                                <label class="form-label">"Tên Người ký / Đơn vị ký:"</label>
                                <input
                                    type="text"
                                    class="text-input"
                                    placeholder="Ví dụ: Giám đốc Nguyễn Văn A"
                                    on:input=move |ev| set_signer_name.set(event_target_value(&ev))
                                />
                            </div>

                            <div class="form-group">
                                <label class="form-label">"Lý do ký (Reason):"</label>
                                <input
                                    type="text"
                                    class="text-input"
                                    placeholder="Ví dụ: Phê duyệt quyết toán năm 2026"
                                    on:input=move |ev| set_reason.set(event_target_value(&ev))
                                />
                            </div>

                            <div class="form-group">
                                <label class="form-label">"Địa điểm ký (Location):"</label>
                                <input
                                    type="text"
                                    class="text-input"
                                    placeholder="Ví dụ: Hà Nội, Việt Nam"
                                    on:input=move |ev| set_location.set(event_target_value(&ev))
                                />
                            </div>

                            <div class="form-group">
                                <label class="form-label">"Chứng thư số PKI (Certificate CN):"</label>
                                <select
                                    class="preset-select"
                                    on:change=move |ev| set_cert_cn.set(event_target_value(&ev))
                                >
                                    <option value="VN-PKI Qualified Digital Signature">"VN-PKI Qualified Digital Signature (Chuẩn SHA-256)"</option>
                                    <option value="Ban Co Yeu Chinh Phu - VGCA">"Ban Cơ Yếu Chính Phủ - Chữ ký số chuyên dùng công vụ"</option>
                                    <option value="Viettel-CA / VNPT-CA / FPT-CA Token">"USB Token Doanh nghiệp (Viettel/VNPT/FPT-CA)"</option>
                                </select>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="pki-verify-notice">
                            <p class="description">
                                "Hệ thống sẽ đối soát hàm băm SHA-256 theo khoảng /ByteRange quy chuẩn ISO 32000-1, so sánh với chữ ký số nhúng trong /Contents và lập báo cáo chi tiết phát hiện tài liệu có bị can thiệp sau khi ký hay không."
                            </p>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
