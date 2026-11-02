use leptos::prelude::*;

#[component]
pub fn AudioSilenceTrimmerView(
    #[prop(into)] silence_threshold_db: RwSignal<f32>,
    #[prop(into)] min_silence_duration_ms: RwSignal<u32>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row">
                <div class="form-group" style="flex: 1;">
                    <label class="form-label">
                        {move || format!("Ngưỡng âm lượng im lặng: {:.0} dB", silence_threshold_db.get())}
                    </label>
                    <input
                        type="range"
                        min="-60"
                        max="-20"
                        prop:value=move || silence_threshold_db.get() as i32
                        on:input=move |ev| {
                            if let Ok(val) = event_target_value(&ev).parse::<f32>() {
                                silence_threshold_db.set(val);
                            }
                        }
                    />
                </div>

                <div class="form-group" style="flex: 1;">
                    <label class="form-label">
                        {move || format!("Thời lượng khoảng lặng tối thiểu: {} ms", min_silence_duration_ms.get())}
                    </label>
                    <input
                        type="range"
                        min="200"
                        max="2000"
                        step="50"
                        prop:value=min_silence_duration_ms
                        on:input=move |ev| {
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                min_silence_duration_ms.set(val);
                            }
                        }
                    />
                </div>
            </div>

            <div style="margin-top: 12px; padding: 12px; background: rgba(30, 41, 59, 0.5); border-radius: 6px; font-size: 12px; color: var(--text-muted);">
                <div style="font-weight: 600; color: #94A3B8; margin-bottom: 6px;">"🎙️ CẮT BỎ KHOẢNG LẶNG CUỘC HỌP & GHI ÂM:"</div>
                <div style="line-height: 1.6;">
                    "Giải mã âm thanh và phân tích năng lượng hiệu dụng RMS (Root Mean Square) theo cửa sổ trượt 50ms, tự động gọt bỏ các khoảng ngừng nghỉ kéo dài giúp rút ngắn thời lượng ghi âm từ 30% đến 60%."
                </div>
            </div>
        </div>
    }
}
