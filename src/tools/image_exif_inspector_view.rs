use leptos::prelude::*;

#[component]
pub fn ImageExifInspectorView(
    #[prop(into)] strip_all: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="checkbox-item" style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
                    <input
                        type="checkbox"
                        checked=strip_all
                        on:change=move |ev| strip_all.set(event_target_checked(&ev))
                    />
                    <span>"Tước bỏ hoàn toàn metadata APP1/EXIF/XMP (Lossless Exif Stripping)"</span>
                </label>
            </div>

            <div style="margin-top: 12px; padding: 12px; background: rgba(30, 41, 59, 0.5); border-radius: 6px; font-size: 12px; color: var(--text-muted);">
                <div style="font-weight: 600; color: #94A3B8; margin-bottom: 6px;">"📋 THÔNG TIN EXIF / GPS HỖ TRỢ TRÍCH XUẤT:"</div>
                <ul style="margin: 0; padding-left: 18px; line-height: 1.6;">
                    <li>"Hãng và Model máy ảnh / điện thoại (Sony, Canon, Nikon, Apple iPhone...)"</li>
                    <li>"Tọa độ địa lý GPS (Vĩ độ Latitude, Kinh độ Longitude) và độ cao"</li>
                    <li>"Ngày giờ chụp gốc và phần mềm chỉnh sửa"</li>
                    <li>"Kích thước điểm ảnh thực tế và tỷ lệ khung hình"</li>
                </ul>
            </div>
        </div>
    }
}
