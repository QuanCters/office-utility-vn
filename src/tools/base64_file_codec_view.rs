use leptos::prelude::*;

#[component]
pub fn Base64FileCodecView(
    #[prop(default = signal("encode".to_string()).0)] mode: ReadSignal<String>,
    #[prop(default = signal("encode".to_string()).1)] set_mode: WriteSignal<String>,
    #[prop(default = signal(false).0)] data_uri_prefix: ReadSignal<bool>,
    #[prop(default = signal(false).1)] set_data_uri_prefix: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">"Chế độ xử lý Base64:"</label>
                    <div style="display: flex; gap: 16px; margin-top: 6px;">
                        <label class="radio-item">
                            <input
                                type="radio"
                                name="b64_mode"
                                checked=move || mode.get() == "encode"
                                on:change=move |_| set_mode.set("encode".to_string())
                            />
                            "Mã hóa Tệp sang Base64 / Data URI"
                        </label>
                        <label class="radio-item">
                            <input
                                type="radio"
                                name="b64_mode"
                                checked=move || mode.get() == "decode"
                                on:change=move |_| set_mode.set("decode".to_string())
                            />
                            "Giải mã Base64 về Tệp gốc"
                        </label>
                    </div>
                </div>

                <div class="form-group">
                    <label class="form-label">"Định dạng Data URI:"</label>
                    <div style="display: flex; gap: 12px; margin-top: 6px;">
                        <label class="radio-item">
                            <input
                                type="checkbox"
                                checked=data_uri_prefix
                                on:change=move |ev| set_data_uri_prefix.set(event_target_checked(&ev))
                                prop:disabled=move || mode.get() == "decode"
                            />
                            "Thêm tiền tố Data URI (VD: data:image/png;base64,...)"
                        </label>
                    </div>
                </div>
            </div>

            <div style="font-size: 12px; color: var(--text-muted); margin-top: 8px;">
                "💡 Xử lý theo luồng (Streaming Codec) với bộ đệm cố định 48 KB, hỗ trợ tệp dung lượng lên đến hàng trăm Megabytes mà không làm tăng vọt RAM."
            </div>
        </div>
    }
}
