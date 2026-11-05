use leptos::prelude::*;

#[component]
pub fn PdfBookmarkTocView() -> impl IntoView {
    let (bookmarks_input, set_bookmarks_input) = signal(
        "Chương 1: Giới thiệu - Trang 1\n  1.1 Mục tiêu - Trang 1\nChương 2: Nội dung - Trang 2".to_string()
    );
    let (is_ime_composing, set_is_ime_composing) = signal(false);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Cây mục lục (Bookmark TOC):"</label>
                <textarea
                    class="form-input"
                    style="height: 120px; font-family: monospace; font-size: 12px;"
                    prop:value=bookmarks_input
                    on:compositionstart=move |_| set_is_ime_composing.set(true)
                    on:compositionend=move |_| set_is_ime_composing.set(false)
                    on:input=move |ev| {
                        if !is_ime_composing.get() {
                            set_bookmarks_input.set(event_target_value(&ev));
                        }
                    }
                    placeholder="Nhập cây mục lục, ví dụ:\nChương 1: Mở đầu - Trang 1\n  1.1 Mục tiêu - Trang 1\nChương 2: Nội dung - Trang 2"
                />
            </div>
            <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">
                "💡 Thụt lề 2 dấu cách để tạo cấp mục lục con (Nested Outlines)."
            </div>
        </div>
    }
}
