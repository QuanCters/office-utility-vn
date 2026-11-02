use leptos::prelude::*;

#[component]
pub fn ExcelSheetUnlockerView() -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div style="padding: 12px; background: rgba(59, 130, 246, 0.1); border: 1px solid rgba(59, 130, 246, 0.3); border-radius: 6px;">
                <div style="font-size: 13px; font-weight: 600; color: #60A5FA; margin-bottom: 4px;">
                    "🔓 Mở khóa bảo vệ trang tính tự động (Zero Password Required)"
                </div>
                <div style="font-size: 12px; color: var(--text-muted); line-height: 1.5;">
                    "Công cụ sẽ tự động phân tích cấu trúc OpenXML và gỡ bỏ hoàn toàn các thẻ "<code style="color: #F8FAFC;">"sheetProtection"</code>" và "<code style="color: #F8FAFC;">"workbookProtection"</code>" bị khóa bởi Excel mà không làm thay đổi hay mất mát bất kỳ công thức, dữ liệu, định dạng nào."
                </div>
            </div>

            <div style="font-size: 11px; color: var(--text-muted); margin-top: 10px;">
                "⚠️ Lưu ý: Tính năng này gỡ bỏ khóa bảo vệ chỉnh sửa ô tính (Protect Sheet). Không áp dụng cho tệp bị mã hóa mật khẩu khi mở tệp (Password to Open)."
            </div>
        </div>
    }
}
