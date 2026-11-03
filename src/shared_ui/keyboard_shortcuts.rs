use leptos::prelude::*;

/// Các hành động phím tắt scoped được hỗ trợ trong micro-app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopedShortcutAction {
    /// `Ctrl+Enter` hoặc `Cmd+Enter`: Chạy tác vụ / pipeline.
    Run,
    /// `Escape`: Hủy tác vụ đang chạy hoặc đóng modal.
    Cancel,
    /// `Backspace`: Quay lại màn hình chính / launcher (chỉ khi không focus ô nhập liệu).
    Back,
    /// Không khớp phím tắt nào.
    None,
}

/// Pure helper phân tích phím bấm và trả về hành động tương ứng.
///
/// Quy tắc:
/// - `Ctrl+Enter` / `Cmd+Enter` => `Run`
/// - `Escape` => `Cancel`
/// - `Backspace` (khi `!is_input_focused`) => `Back`
/// - `Backspace` (khi `is_input_focused`) => `None` (cho phép xóa ký tự bình thường)
/// - Mọi phím khác => `None`
pub fn match_scoped_shortcut(key: &str, ctrl_or_meta: bool, is_input_focused: bool) -> ScopedShortcutAction {
    if ctrl_or_meta && key == "Enter" {
        ScopedShortcutAction::Run
    } else if key == "Escape" {
        ScopedShortcutAction::Cancel
    } else if key == "Backspace" && !is_input_focused {
        ScopedShortcutAction::Back
    } else {
        ScopedShortcutAction::None
    }
}

/// Kiểm tra xem con trỏ chuột/focus hiện tại có đang nằm trong ô nhập liệu (input, textarea, select, contenteditable) hay không.
pub fn is_input_focused_event(ev: &web_sys::KeyboardEvent) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(target) = ev.target() {
            if let Ok(tag_val) = js_sys::Reflect::get(&target, &wasm_bindgen::JsValue::from_str("tagName")) {
                if let Some(tag) = tag_val.as_string() {
                    let tag_lower = tag.to_ascii_lowercase();
                    if tag_lower == "input" || tag_lower == "textarea" || tag_lower == "select" {
                        return true;
                    }
                }
            }
            if let Ok(is_editable) = js_sys::Reflect::get(&target, &wasm_bindgen::JsValue::from_str("isContentEditable")) {
                if is_editable.as_bool() == Some(true) {
                    return true;
                }
            }
        }
        false
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = ev;
        false
    }
}

/// Tập hợp các callback xử lý phím tắt scoped.
#[derive(Clone, Copy, Default)]
pub struct ScopedShortcutsHandlers {
    pub on_run: Option<Callback<()>>,
    pub on_cancel: Option<Callback<()>>,
    pub on_back: Option<Callback<()>>,
}

/// Xử lý một sự kiện `KeyboardEvent` theo cấu hình `ScopedShortcutsHandlers`.
/// Trả về `true` nếu sự kiện đã được kích hoạt và chặn lan truyền (`prevent_default`).
pub fn handle_scoped_keydown_event(ev: &web_sys::KeyboardEvent, handlers: &ScopedShortcutsHandlers) -> bool {
    let key = ev.key();
    let ctrl_or_meta = ev.ctrl_key() || ev.meta_key();
    let is_input = is_input_focused_event(ev);

    let action = match_scoped_shortcut(&key, ctrl_or_meta, is_input);
    match action {
        ScopedShortcutAction::Run => {
            if let Some(cb) = handlers.on_run {
                ev.prevent_default();
                cb.run(());
                return true;
            }
        }
        ScopedShortcutAction::Cancel => {
            if let Some(cb) = handlers.on_cancel {
                ev.prevent_default();
                cb.run(());
                return true;
            }
        }
        ScopedShortcutAction::Back => {
            if let Some(cb) = handlers.on_back {
                ev.prevent_default();
                cb.run(());
                return true;
            }
        }
        ScopedShortcutAction::None => {}
    }
    false
}

/// Hook Leptos đăng ký window keydown listener cho phạm vi scoped:
/// - `Ctrl+Enter` / `Cmd+Enter` => Chạy `on_run`
/// - `Escape` => Hủy `on_cancel`
/// - `Backspace` => Quay lại `on_back` (khi không focus ô nhập văn bản)
pub fn use_scoped_keyboard_shortcuts(
    on_run: Option<Callback<()>>,
    on_cancel: Option<Callback<()>>,
    on_back: Option<Callback<()>>,
) {
    let handlers = ScopedShortcutsHandlers {
        on_run,
        on_cancel,
        on_back,
    };

    #[cfg(target_arch = "wasm32")]
    {
        use leptos::ev::keydown;
        let _ = window_event_listener(keydown, move |ev: web_sys::KeyboardEvent| {
            handle_scoped_keydown_event(&ev, &handlers);
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = handlers;
    }
}

/// Helper tiện ích chỉ đăng ký phím tắt `Backspace` để quay lại.
pub fn use_back_shortcut(on_back: Callback<()>) {
    use_scoped_keyboard_shortcuts(None, None, Some(on_back));
}

/// Helper tiện ích chỉ đăng ký phím tắt `Escape` để hủy tác vụ.
pub fn use_cancel_shortcut(on_cancel: Callback<()>) {
    use_scoped_keyboard_shortcuts(None, Some(on_cancel), None);
}

/// Helper tiện ích chỉ đăng ký phím tắt `Ctrl+Enter` để thực thi tác vụ.
pub fn use_run_shortcut(on_run: Callback<()>) {
    use_scoped_keyboard_shortcuts(Some(on_run), None, None);
}
