pub mod bridge;
pub mod canvas_frame;
pub mod file_list_panel;
pub mod hub_view;
pub mod layout;
pub mod state;
pub mod toolbar;
pub mod vn_ribbon;

pub use bridge::{IncomingBridgeMessage, OutgoingBridgeMessage, PdfCraftBridge};
pub use canvas_frame::PdfCraftCanvasFrame;
pub use file_list_panel::PdfStudioFileListPanel;
pub use hub_view::{filter_pdf_studio_tools, matches_pdf_tool, PdfStudioHubView};
pub use layout::PdfStudioLayout;
pub use state::{category_for_tool, PdfCategory, PdfStudioState, PdfViewMode};
pub use toolbar::{pdf_studio_tool_icon, PdfStudioToolbar};
pub use vn_ribbon::{PdfVnExtensionRibbon, VnRibbonTool};

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
/// 13. `pdf-pki-sign` (Chữ ký số PKI & Xác thực Toàn vẹn)
/// 14. `pdf-repair-corrupt` (Phục hồi File PDF Lỗi Cấu trúc)
/// 15. `pdf-page-number-header` (Đánh số trang & Bates Numbering)
/// 16. `pdf-watermark-stamp` (Đóng dấu Watermark & Con dấu đỏ)
/// 17. `pdf-grayscale-print` (Chuyển PDF sang Đen trắng)
/// 18. `vietnamese-ocr` (OCR Nhận dạng Ký tự Tiếng Việt)
/// 19. `pdf-to-markdown` (Chuyển đổi PDF sang Markdown)
/// 20. `pdf-to-long-image` (Ghép PDF thành Ảnh Dọc Dài)
/// 21. `pdf-table-extractor` (Trích xuất Bảng từ PDF ra Excel)
/// 22. `markdown-to-styled-pdf` (Biên dịch Markdown sang PDF)
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
