pub mod batch_queue;
pub mod gallery_panel;
pub mod layout;
pub mod state;
pub mod toolbar;

pub use batch_queue::MediaStudioBatchQueue;
pub use gallery_panel::MediaStudioGalleryPanel;
pub use layout::MediaStudioLayout;
pub use state::{category_for_tool, media_studio_tool_icon, MediaBatchItem, MediaCategory, MediaStudioState};
pub use toolbar::MediaStudioToolbar;

use leptos::prelude::*;

/// Micro-App 3: Media Studio — Phòng thu Đa phương tiện, xử lý ảnh thẻ, làm đẹp ảnh, xóa siêu dữ liệu và tối ưu đa phương tiện.
///
/// Quản lý 9 công cụ đa phương tiện:
/// 1. `image-batch` (Nén & đổi định dạng ảnh hàng loạt)
/// 2. `photo-id-crop` (Cắt ảnh thẻ 3x4, 4x6 300 DPI)
/// 3. `image-bg-redact` (Làm mờ dữ liệu nhạy cảm giấy tờ)
/// 4. `image-exif-inspector` (Kiểm tra & xóa Exif/GPS)
/// 5. `watermark-batch-studio` (Đóng dấu bản quyền ảnh hàng loạt)
/// 6. `screenshot-beautifier` (Làm đẹp ảnh chụp màn hình)
/// 7. `doc-scanner-perspective` (Phẳng hóa tài liệu chụp nghiêng)
/// 8. `audio-silence-trimmer` (Cắt bỏ khoảng lặng ghi âm)
/// 9. `video-to-gif-webp` (Chuyển đổi video ngắn sang GIF/WebP)
#[component]
pub fn MediaStudioApp(
    #[prop(into, optional)] state: Option<MediaStudioState>,
    #[prop(into, optional)] initial_tool: Option<String>,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    let app_state = state.unwrap_or_else(|| MediaStudioState::with_initial_tool(initial_tool.clone()));
    if let Some(ref slug) = initial_tool {
        app_state.select_tool(slug);
    }

    view! {
        <layout::MediaStudioLayout state=app_state on_back=on_back />
    }
}
