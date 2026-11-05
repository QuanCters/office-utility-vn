use leptos::prelude::*;

#[component]
pub fn DocxTemplateMailmergeView() -> impl IntoView {
    let (output_pattern, set_output_pattern) = signal("Hop_dong_{{ho_ten}}_{{ma_nv}}.docx".to_string());
    let (is_ime_composing, set_is_ime_composing) = signal(false);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Quy tắc đặt tên tệp Word sau khi trộn:"</label>
                <input
                    type="text"
                    class="form-input"
                    prop:value=output_pattern
                    on:compositionstart=move |_| set_is_ime_composing.set(true)
                    on:compositionend=move |_| set_is_ime_composing.set(false)
                    on:input=move |ev| {
                        if !is_ime_composing.get() {
                            set_output_pattern.set(event_target_value(&ev));
                        }
                    }
                    placeholder="Ví dụ: Hop_dong_{{ho_ten}}_{{ma_nv}}.docx"
                />
            </div>
            <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">
                "💡 Các biến trong {{...}} sẽ tự động được thay thế tương ứng với tên cột trong bảng tính dữ liệu."
            </div>
        </div>
    }
}
