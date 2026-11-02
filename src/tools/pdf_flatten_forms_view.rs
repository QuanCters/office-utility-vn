use leptos::prelude::*;

#[component]
pub fn PdfFlattenFormsView() -> impl IntoView {
    let (remove_interactive, set_remove_interactive) = signal(true);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Làm phẳng biểu mẫu nhập liệu (Flatten AcroForms):"</label>
                <div style="margin-top: 6px;">
                    <label class="radio-item">
                        <input
                            type="checkbox"
                            checked=remove_interactive
                            on:change=move |ev| set_remove_interactive.set(event_target_checked(&ev))
                        />
                        "Hợp nhất nội dung các ô biểu mẫu vào trang tĩnh và gỡ bỏ hoàn toàn trường tương tác"
                    </label>
                </div>
            </div>
        </div>
    }
}
