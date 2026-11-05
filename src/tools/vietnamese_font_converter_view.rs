use leptos::prelude::*;

#[component]
pub fn VietnameseFontConverterView(
    #[prop(default = signal("auto".to_string()).0)] source_encoding: ReadSignal<String>,
    #[prop(default = signal("auto".to_string()).1)] set_source_encoding: WriteSignal<String>,
    #[prop(default = signal("unicode".to_string()).0)] target_encoding: ReadSignal<String>,
    #[prop(default = signal("unicode".to_string()).1)] set_target_encoding: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">"Bảng mã nguồn:"</label>
                    <select
                        class="form-select"
                        on:change=move |ev| {
                            set_source_encoding.set(event_target_value(&ev));
                        }
                        prop:value=move || source_encoding.get()
                    >
                        <option value="auto">"Tự động nhận diện (Khuyến nghị)"</option>
                        <option value="tcvn3">"TCVN3 (ABC) - .VNTime"</option>
                        <option value="vni">"VNI-Windows - VNI-Times"</option>
                        <option value="vietware">"Vietware"</option>
                        <option value="nfd">"Unicode tổ hợp (NFD)"</option>
                    </select>
                </div>

                <div class="form-group">
                    <label class="form-label">"Bảng mã đích:"</label>
                    <select
                        class="form-select"
                        on:change=move |ev| {
                            set_target_encoding.set(event_target_value(&ev));
                        }
                        prop:value=move || target_encoding.get()
                    >
                        <option value="unicode">"Unicode dựng sẵn (NFC - Chuẩn quốc tế)"</option>
                        <option value="tcvn3">"TCVN3 (ABC)"</option>
                        <option value="vni">"VNI-Windows"</option>
                    </select>
                </div>
            </div>

            <div style="font-size: 12px; color: var(--text-muted); margin-top: 8px;">
                "💡 Giữ nguyên 100% định dạng in đậm, in nghiêng, màu sắc và công thức khi chuyển đổi tệp .docx và .xlsx."
            </div>
        </div>
    }
}
