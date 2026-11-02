use leptos::prelude::*;

#[component]
pub fn VietnameseOcrView() -> impl IntoView {
    let (engine, set_engine) = signal("fast_os_native".to_string());
    let (format, set_format) = signal("txt".to_string());

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Động cơ nhận diện ký tự quang học (OCR):"</label>
                <div style="display: flex; gap: 12px; margin-top: 4px;">
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="ocr_engine"
                            checked=move || engine.get() == "fast_os_native"
                            on:change=move |_| set_engine.set("fast_os_native".to_string())
                        />
                        "OS Native OCR (0 MB, cực tốc)"
                    </label>
                    <label class="radio-item">
                        <input
                            type="radio"
                            name="ocr_engine"
                            checked=move || engine.get() == "accurate_onnx"
                            on:change=move |_| set_engine.set("accurate_onnx".to_string())
                        />
                        "VietOCR On-Demand ONNX (~12 MB, chính xác cao)"
                    </label>
                </div>
            </div>

            <div class="form-group">
                <label class="form-label">"Định dạng tệp xuất kết quả:"</label>
                <select
                    class="preset-select"
                    on:change=move |ev| set_format.set(event_target_value(&ev))
                >
                    <option value="txt">".txt (Văn bản thuần UTF-8)"</option>
                    <option value="docx">".docx (Microsoft Word)"</option>
                    <option value="md">".md (Markdown)"</option>
                </select>
            </div>
        </div>
    }
}
