use leptos::prelude::*;

#[component]
pub fn DocScannerPerspectiveView(
    #[prop(into)] auto_enhance: RwSignal<bool>,
) -> impl IntoView {
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

            <div style="margin-top: 12px; padding: 12px; background: rgba(30, 41, 59, 0.5); border-radius: 6px; font-size: 12px; color: var(--text-muted);">
                <div style="font-weight: 600; color: #94A3B8; margin-bottom: 6px;">"📐 HƯỚNG DẪN NẮN PHẲNG GÓC NGHIÊNG:"</div>
                <div style="line-height: 1.6;">
                    "Thuật toán biến đổi phối cảnh 4 điểm ma trận đồng nhất (Homography 3x3) tự động nhận diện mép giấy và nắn góc chụp nghiêng thành trang tài liệu vuông vức chuẩn scan."
                </div>
            </div>
        </div>
    }
}
