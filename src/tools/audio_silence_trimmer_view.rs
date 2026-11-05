use leptos::prelude::*;

#[component]
pub fn AudioSilenceTrimmerView(
    #[prop(into)] silence_threshold_db: RwSignal<f32>,
    #[prop(into)] min_silence_duration_ms: RwSignal<u32>,
    #[prop(into)] manual_trim_enabled: RwSignal<bool>,
    #[prop(into)] start_time_s: RwSignal<f32>,
    #[prop(into)] end_time_s: RwSignal<f32>,
    #[prop(into)] volume_boost_db: RwSignal<f32>,
    #[prop(into)] enable_limiter: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            // 1. Chuyển đổi chế độ: Cắt khoảng lặng vs Cắt thủ công
            <div class="form-group">
                <label class="form-label" style="font-weight: 600; color: #E2E8F0;">"Chế độ cắt âm thanh:"</label>
                <div style="display: flex; gap: 16px; margin-top: 6px;">
                    <label class="radio-item" style="display: flex; align-items: center; gap: 6px; cursor: pointer;">
                        <input
                            type="radio"
                            name="trim_mode"
                            checked=move || !manual_trim_enabled.get()
                            on:change=move |_| manual_trim_enabled.set(false)
                        />
                        <span>"🎙️ Tự động gọt khoảng lặng (RMS Silence)"</span>
                    </label>
                    <label class="radio-item" style="display: flex; align-items: center; gap: 6px; cursor: pointer;">
                        <input
                            type="radio"
                            name="trim_mode"
                            checked=move || manual_trim_enabled.get()
                            on:change=move |_| manual_trim_enabled.set(true)
                        />
                        <span>"✂️ Cắt theo mốc thời gian thủ công (Manual Trim)"</span>
                    </label>
                </div>
            </div>

            // 2. Nội dung tùy chọn theo chế độ đã chọn
            {move || {
                if !manual_trim_enabled.get() {
                    view! {
                        <div class="options-row" style="margin-top: 10px;">
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
                    }.into_any()
                } else {
                    view! {
                        <div class="options-row" style="margin-top: 10px;">
                            <div class="form-group" style="flex: 1;">
                                <label class="form-label">
                                    {move || format!("Bắt đầu từ: {:.1} giây", start_time_s.get())}
                                </label>
                                <input
                                    type="number"
                                    step="0.5"
                                    min="0"
                                    class="form-input"
                                    prop:value=start_time_s
                                    on:input=move |ev| {
                                        if let Ok(val) = event_target_value(&ev).parse::<f32>() {
                                            start_time_s.set(val.max(0.0));
                                        }
                                    }
                                />
                            </div>

                            <div class="form-group" style="flex: 1;">
                                <label class="form-label">
                                    {move || {
                                        let end = end_time_s.get();
                                        if end <= 0.0 {
                                            "Kết thúc: Hết tệp (0 = Full)".to_string()
                                        } else {
                                            format!("Kết thúc tại: {:.1} giây", end)
                                        }
                                    }}
                                </label>
                                <input
                                    type="number"
                                    step="0.5"
                                    min="0"
                                    class="form-input"
                                    placeholder="0 = Hết tệp"
                                    prop:value=end_time_s
                                    on:input=move |ev| {
                                        if let Ok(val) = event_target_value(&ev).parse::<f32>() {
                                            end_time_s.set(val.max(0.0));
                                        }
                                    }
                                />
                            </div>
                        </div>
                    }.into_any()
                }
            }}

            // 3. Phân khu Khuếch đại âm thanh (Volume Booster & Limiter)
            <div style="margin-top: 14px; padding-top: 12px; border-top: 1px solid rgba(51, 65, 85, 0.6);">
                <div style="font-size: 13px; font-weight: 600; color: #38BDF8; margin-bottom: 8px;">
                    "🔊 KHUẾCH ĐẠI ÂM LƯỢNG (VOLUME BOOSTER):"
                </div>

                <div class="options-row">
                    <div class="form-group" style="flex: 1;">
                        <label class="form-label">
                            {move || {
                                let boost = volume_boost_db.get();
                                if boost <= 0.0 {
                                    "Âm lượng: 0 dB (Giữ nguyên gốc)".to_string()
                                } else {
                                    format!("Tăng âm lượng: +{:.1} dB (+{:.0}% biên độ)", boost, (10.0f32.powf(boost / 20.0) - 1.0) * 100.0)
                                }
                            }}
                        </label>
                        <input
                            type="range"
                            min="0"
                            max="24"
                            step="1"
                            prop:value=move || volume_boost_db.get() as i32
                            on:input=move |ev| {
                                if let Ok(val) = event_target_value(&ev).parse::<f32>() {
                                    volume_boost_db.set(val);
                                }
                            }
                        />
                    </div>

                    <div class="form-group" style="flex: 1; display: flex; align-items: flex-end; padding-bottom: 8px;">
                        <label class="checkbox-item" style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
                            <input
                                type="checkbox"
                                checked=enable_limiter
                                on:change=move |ev| enable_limiter.set(event_target_checked(&ev))
                            />
                            <span style="font-size: 12px; color: #E2E8F0;">
                                "Bật Soft-clipping Limiter (Chống vỡ tiếng rè rè khi tăng volume)"
                            </span>
                        </label>
                    </div>
                </div>
            </div>

            // 4. Hướng dẫn nghiệp vụ
            <div style="margin-top: 12px; padding: 12px; background: rgba(30, 41, 59, 0.5); border-radius: 6px; font-size: 12px; color: var(--text-muted);">
                <div style="font-weight: 600; color: #94A3B8; margin-bottom: 6px;">"💡 THÔNG TIN TIỆN ÍCH AUDIO:"</div>
                <div style="line-height: 1.6;">
                    "Hỗ trợ giải mã tệp WAV và MP3 trực tiếp On-Device. Thuật toán RMS tính toán năng lượng sóng âm theo từng khung 50ms, kết hợp bộ giới hạn âm học Soft-clipping hyperbolic tangent (tanh) giúp tăng âm lượng đoạn ghi âm nhỏ mà không gây méo biên độ."
                </div>
            </div>
        </div>
    }
}
