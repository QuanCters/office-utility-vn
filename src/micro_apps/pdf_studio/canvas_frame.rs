use leptos::prelude::*;
use crate::micro_apps::pdf_studio::state::PdfStudioState;

/// Component `PdfCraftCanvasFrame` nhúng giao diện đồ họa trực quan của PDFCraft (WebAssembly Canvas)
/// thông qua thẻ `iframe`.
///
/// Hỗ trợ:
/// - `allow="clipboard-read; clipboard-write"` để thao tác sao chép / dán nội dung trong canvas.
/// - Chiều rộng & chiều cao 100% chiếm trọn không gian container (`w-full h-full border-none`).
/// - Hiển thị trạng thái loading skeleton trong lúc WebAssembly và WebGL Canvas đang khởi tạo.
/// - Tự động ẩn skeleton khi nhận thông điệp `pdfcraft:ready` từ bridge.
/// - Tự động nạp tệp PDF đang kích hoạt vào Canvas khi sẵn sàng.
#[component]
pub fn PdfCraftCanvasFrame(
    #[prop(into, optional)] state: Option<PdfStudioState>,
) -> impl IntoView {
    let studio_state = state.or_else(use_context::<PdfStudioState>);

    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = studio_state {
            // Đăng ký message listener nhận sự kiện từ iframe
            let _ = crate::micro_apps::pdf_studio::bridge::setup_bridge_listener(
                "pdfcraft-frame".to_string(),
                s,
                None,
            );

            // Theo dõi tệp đang chọn và tự động nạp vào canvas khi iframe đã sẵn sàng
            let last_opened = StoredValue::new(None::<String>);
            Effect::new(move |_| {
                let is_ready = s.is_canvas_ready.get();
                if is_ready {
                    if let Some(path) = s.active_file_path() {
                        if last_opened.get_value().as_deref() != Some(&path) {
                            last_opened.set_value(Some(path.clone()));
                            wasm_bindgen_futures::spawn_local(async move {
                                let _ = crate::micro_apps::pdf_studio::bridge::open_file_into_canvas(
                                    "pdfcraft-frame",
                                    &path,
                                )
                                .await;
                            });
                        }
                    }
                }
            });
        }
    }

    view! {
        <div class="pdfcraft-frame-container relative w-full h-full flex-1 overflow-hidden">
            // Trạng thái loading skeleton hiển thị trong lúc WASM canvas đang khởi động
            <div
                id="pdfcraft-skeleton"
                class="pdfcraft-loading-skeleton"
                class:hidden=move || studio_state.map(|s| s.is_canvas_ready.get()).unwrap_or(false)
                aria-live="polite"
            >
                <div class="pdfcraft-skeleton-content">
                    <div class="pdfcraft-skeleton-spinner" aria-hidden="true"></div>
                    <div class="pdfcraft-skeleton-text">
                        <span class="pdfcraft-skeleton-title">
                            "Đang tải PDFCraft Studio..."
                        </span>
                        <span class="pdfcraft-skeleton-subtitle">
                            "Khởi tạo WebAssembly & WebGL On-Device"
                        </span>
                    </div>
                </div>
            </div>

            // Khung nhúng iframe PDFCraft Canvas
            <iframe
                id="pdfcraft-frame"
                src="/pdfcraft/index.html?host=parent"
                class="w-full h-full border-none"
                allow="clipboard-read; clipboard-write"
                title="PDFCraft Canvas Studio"
            ></iframe>
        </div>
    }
}
