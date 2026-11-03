use leptos::prelude::*;
use crate::shared_ui::keyboard_shortcuts::use_back_shortcut;

/// Component nút "← Về trang chính" đồng bộ phong cách thiết kế,
/// hỗ trợ hover effect chuyển động nhẹ và phím tắt `Backspace` (khi không focus ô nhập liệu).
#[component]
pub fn BackButton(
    /// Callback được kích hoạt khi nhấn nút hoặc nhấn phím Backspace.
    #[prop(into)]
    on_back: Callback<()>,
    /// Nhãn hiển thị tùy chọn (mặc định: "← Về trang chính").
    #[prop(into, optional)]
    label: Option<&'static str>,
) -> impl IntoView {
    // Đăng ký phím tắt Backspace scoped cho màn hình này
    use_back_shortcut(on_back);

    let display_label = label.unwrap_or("← Về trang chính");

    view! {
        <button
            class="back-nav-btn"
            type="button"
            title="Quay lại trang chính (Backspace)"
            on:click=move |_| on_back.run(())
        >
            <span class="back-nav-icon" aria-hidden="true">"←"</span>
            <span class="back-nav-text">{display_label.trim_start_matches("← ")}</span>
        </button>
    }
}
