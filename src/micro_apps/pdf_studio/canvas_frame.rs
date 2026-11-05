use leptos::prelude::*;

/// Component `PdfCraftCanvasFrame` nhúng giao diện đồ họa trực quan của PDFCraft (WebAssembly Canvas)
/// thông qua thẻ `iframe`.
///
/// Hỗ trợ:
/// - `allow="clipboard-read; clipboard-write"` để thao tác sao chép / dán nội dung trong canvas.
/// - Chiều rộng & chiều cao 100% chiếm trọn không gian container (`w-full h-full border-none`).
/// - Hiển thị trạng thái loading skeleton trong lúc WebAssembly và WebGL Canvas đang khởi tạo.
#[component]
pub fn PdfCraftCanvasFrame() -> impl IntoView {
    view! {
        <div class="pdfcraft-frame-container relative w-full h-full flex-1 overflow-hidden">
            // Trạng thái loading skeleton hiển thị trong lúc WASM canvas đang khởi động
            <div
                id="pdfcraft-skeleton"
                class="pdfcraft-loading-skeleton"
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
