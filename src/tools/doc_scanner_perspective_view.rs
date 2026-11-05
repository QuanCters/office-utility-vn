use leptos::prelude::*;
use crate::app::AppState;

#[component]
pub fn DocScannerPerspectiveView(
    #[prop(into)] auto_enhance: RwSignal<bool>,
) -> impl IntoView {
    let app_state = use_context::<AppState>();
    let on_goto_ocr = move |_| {
        if let Some(app) = app_state {
            app.deep_link_by_slug("vietnamese-ocr");
        }
    };

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="checkbox-item" style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
                    <input
                        type="checkbox"
                        checked=auto_enhance
                        on:change=move |ev| auto_enhance.set(event_target_checked(&ev))
                    />
                    <span>"Tự động cân bằng trắng giấy và tăng độ tương phản nét chữ đen (High Contrast Scan)"</span>
                </label>
            </div>

            <div style="margin-top: 14px; padding: 12px; background: rgba(14, 165, 233, 0.08); border: 1px solid rgba(56, 189, 248, 0.25); border-radius: 6px; display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px;">
                <div style="max-width: 380px;">
                    <div style="font-size: 13px; font-weight: 600; color: #38BDF8; margin-bottom: 2px;">"⚡ Liên kết 1-Click sang OCR Văn bản"</div>
                    <div style="font-size: 12px; color: var(--text-muted, #94A3B8); line-height: 1.4;">"Chuyển ngay tệp tài liệu đã làm phẳng sang công cụ OCR nhận dạng chữ tiếng Việt có dấu."</div>
                </div>
                <button
                    type="button"
                    class="media-studio-btn-primary"
                    style="padding: 8px 14px; font-size: 12px; font-weight: 600; background: #0284C7; color: white; border: none; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 6px; box-shadow: 0 2px 4px rgba(0,0,0,0.2);"
                    on:click=on_goto_ocr
                >
                    <span>"🔍 Mở OCR Nhận dạng Văn bản"</span>
                </button>
            </div>

            <div style="margin-top: 12px; padding: 12px; background: rgba(30, 41, 59, 0.5); border-radius: 6px; font-size: 12px; color: var(--text-muted);">
                <div style="font-weight: 600; color: #94A3B8; margin-bottom: 6px;">"📐 HƯỚNG DẪN NẮN PHẲNG GÓC NGHIÊNG:"</div>
                <div style="line-height: 1.6;">
                    "Thuật toán biến đổi phối cảnh 4 điểm ma trận đồng nhất (Homography 3x3) tự động nhận diện mép giấy và nắn góc chụp nghiêng thành trang tài liệu vuông vức chuẩn scan."
                </div>
            </div>
        </div>
    }
}
