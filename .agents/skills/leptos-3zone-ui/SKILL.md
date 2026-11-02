---
name: leptos-3zone-ui
description: Khuôn mẫu chuẩn xây dựng giao diện công cụ bằng Leptos 0.8 (CSR WebAssembly) tuân thủ bố cục Unified 3-Zone Workspace, phím tắt toàn cục và an toàn bộ gõ tiếng Việt (IME-Safe).
---

# Quy chuẩn Giao diện Leptos 0.8 & Unified 3-Zone Workspace (`crates/office-ui`)

## 1. Quy tắc Cú pháp Leptos 0.8 (Chống Trôi dạt Phiên bản)
- Sử dụng cú pháp Reactive mới của **Leptos 0.8**: `let (value, set_value) = signal(initial_val);` hoặc `RwSignal::new(...)`.
- **TUYỆT ĐỐI CẤM** sử dụng cú pháp Leptos 0.5/0.6 đã lỗi thời như `create_signal(cx, ...)`, `create_effect(cx, ...)`, hoặc truyền tham số `cx: Scope` vào `#[component]`.

## 2. Xử lý An toàn Bộ gõ Tiếng Việt (IME-Safe Input - `NFR-COM-02`)
Trên Windows (UniKey, EVKey) và macOS IME, nếu cập nhật Signal trực tiếp trên mọi sự kiện `on:input` rồi ép render lại giá trị `prop:value` ngay giữa lúc bộ gõ đang ghép dấu (ví dụ gõ `t` + `i` + `e` + `e` + `n` + `g` -> `tiếng`), con trỏ sẽ bị nhảy hoặc mất dấu. Bắt buộc dùng pattern `ImeSafeInput` sau:

```rust
use leptos::prelude::*;
use leptos::ev;

#[component]
pub fn ImeSafeTextInput(
    value: RwSignal<String>,
    #[prop(into)] placeholder: String,
) -> impl IntoView {
    let is_composing = RwSignal::new(false);

    view! {
        <input
            type="text"
            class="w-full rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-2 text-sm font-sans focus:border-indigo-600 focus:outline-none"
            placeholder=placeholder
            prop:value=move || value.get()
            on:compositionstart=move |_| is_composing.set(true)
            on:compositionend=move |ev| {
                is_composing.set(false);
                value.set(event_target_value(&ev));
            }
            on:input=move |ev: ev::Event| {
                if !is_composing.get_untracked() {
                    value.set(event_target_value(&ev));
                }
            }
        />
    }
}