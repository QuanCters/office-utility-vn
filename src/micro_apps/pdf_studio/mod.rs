pub mod file_list_panel;
pub mod layout;
pub mod state;
pub mod toolbar;

pub use file_list_panel::PdfStudioFileListPanel;
pub use layout::PdfStudioLayout;
pub use state::{category_for_tool, PdfCategory, PdfStudioState};
pub use toolbar::{pdf_studio_tool_icon, PdfStudioToolbar};

use leptos::prelude::*;

/// Micro-App 2: PDF Studio — Xưởng Xử lý, Biên tập, Bảo mật & Số hóa Tài liệu PDF chuyên nghiệp.
///
/// Quản lý 18 công cụ tài liệu PDF:
/// 1. `pdf-merge-split` (Ghép, Tách & Xoay trang PDF)
/// 2. `pdf-crop-margin` (Cắt Viền Trắng Thừa - Auto-Crop)
/// 3. `pdf-booklet-imposition` (Dàn trang In Sách Gấp đôi & N-up)
/// 4. `pdf-bookmark-toc` (Tạo Cây Mục lục PDF - Bookmark TOC)
/// 5. `pdf-visual-diff` (So sánh Đối soát 2 Bản PDF)
/// 6. `pdf-protect-sign` (Bảo mật & Mở khóa PDF)
/// 7. `pdf-compress` (Nén giảm Dung lượng PDF)
/// 8. `pdf-metadata-scrubber` (Tẩy sạch Metadata ẩn PDF)
/// 9. `pdf-flatten-forms` (Làm phẳng PDF - Flatten Forms)
/// 10. `pdf-form-manager` (Quản lý & Điền Biểu mẫu AcroForm)
/// 11. `pdf-redaction` (Xóa Dữ liệu Nhạy cảm & Che Đen Stream)
/// 12. `pdf-archive-linearize` (Chuẩn hóa Lưu trữ PDF/A & Fast Web View)
/// 13. `pdf-repair-corrupt` (Phục hồi File PDF Lỗi Cấu trúc)
/// 14. `pdf-page-number-header` (Đánh số trang & Bates Numbering)
/// 15. `pdf-watermark-stamp` (Đóng dấu Watermark & Con dấu đỏ)
/// 16. `pdf-grayscale-print` (Chuyển PDF sang Đen trắng)
/// 17. `vietnamese-ocr` (OCR Nhận dạng Ký tự Tiếng Việt)
/// 18. `pdf-to-markdown` (Chuyển đổi PDF sang Markdown)
/// 19. `pdf-to-long-image` (Ghép PDF thành Ảnh Dọc Dài)
/// 20. `pdf-table-extractor` (Trích xuất Bảng từ PDF ra Excel)
/// 21. `markdown-to-styled-pdf` (Biên dịch Markdown sang PDF)
#[component]
pub fn PdfStudioApp(
    #[prop(into, optional)] state: Option<PdfStudioState>,
    #[prop(into, optional)] initial_tool: Option<String>,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    let app_state = state.unwrap_or_else(|| PdfStudioState::with_initial_tool(initial_tool.clone()));
    if let Some(ref slug) = initial_tool {
        app_state.select_tool(slug);
    }

    view! {
        <layout::PdfStudioLayout state=app_state on_back=on_back />
    }
}
