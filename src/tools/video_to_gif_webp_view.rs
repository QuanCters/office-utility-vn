use leptos::prelude::*;

#[component]
pub fn VideoToGifWebpView(
    #[prop(into)] fps: RwSignal<u32>,
    #[prop(into)] max_width: RwSignal<Option<u32>>,
    #[prop(into)] output_format: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row">
                <div class="form-group" style="flex: 1;">
                    <label class="form-label">
                        {move || format!("Tốc độ khung hình (FPS): {} fps", fps.get())}
                    </label>
                    <input
                        type="range"
                        min="10"
                        max="30"
                        step="1"
                        prop:value=fps
                        on:input=move |ev| {
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                fps.set(val);
                            }
                        }
                    />
                </div>

                <div class="form-group" style="flex: 1;">
                    <label class="form-label">"Độ phân giải tối đa (Chiều rộng):"</label>
                    <select
                        class="form-select"
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            if val == "0" {
                                max_width.set(None);
                            } else if let Ok(w) = val.parse::<u32>() {
                                max_width.set(Some(w));
                            }
                        }
                    >
                        <option value="0">"Giữ nguyên kích thước gốc"</option>
                        <option value="480">"480 px (Nhẹ, tối ưu chat/web)"</option>
                        <option value="640">"640 px (Chuẩn SD)"</option>
                        <option value="720">"720 px (HD 720p)"</option>
                    </select>
                </div>

                <div class="form-group" style="flex: 1;">
                    <label class="form-label">"Định dạng xuất:"</label>
                    <select
                        class="form-select"
                        prop:value=output_format
                        on:change=move |ev| output_format.set(event_target_value(&ev))
                    >
                        <option value="gif">"GIF Động (GIF89a 256 màu LZW)"</option>
                        <option value="webp">"WebP Động (Animated WebP)"</option>
                    </select>
                </div>
            </div>

            <div style="margin-top: 12px; padding: 12px; background: rgba(30, 41, 59, 0.5); border-radius: 6px; font-size: 12px; color: var(--text-muted);">
                <div style="font-weight: 600; color: #94A3B8; margin-bottom: 6px;">"🎞️ CHUYỂN ĐỔI VIDEO CLIP SANG GIF / WEBP:"</div>
                <div style="line-height: 1.6;">
                    "Chuyển đổi video ngắn (MP4, MOV) thành ảnh động GIF hoặc WebP chất lượng cao, lượng tử hóa bảng màu tối ưu 256 màu và nén LZW chuẩn GIF89a mượt mà, hoàn toàn xử lý ngoại tuyến không chèn watermark."
                </div>
            </div>
        </div>
    }
}
