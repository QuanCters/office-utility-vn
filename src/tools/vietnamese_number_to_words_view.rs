use leptos::prelude::*;

#[component]
pub fn VietnameseNumberToWordsView(
    input_value: ReadSignal<String>,
    set_input_value: WriteSignal<String>,
    dialect: ReadSignal<String>,
    set_dialect: WriteSignal<String>,
    use_dong_chan: ReadSignal<bool>,
    set_use_dong_chan: WriteSignal<bool>,
    output_result: ReadSignal<String>,
    set_is_ime_composing: WriteSignal<bool>,
) -> impl IntoView {
    let is_playing = RwSignal::new(false);

    on_cleanup(move || {
        crate::shared_ui::stop_audio();
    });

    let toggle_speak = move |_| {
        if is_playing.get_untracked() {
            crate::shared_ui::stop_audio();
            is_playing.set(false);
        } else {
            let text = output_result.get_untracked();
            if text.trim().is_empty() {
                return;
            }
            let is_south = dialect.get_untracked() == "South";
            let tokens = crate::shared_ui::words_to_audio_tokens(&text, is_south);
            if tokens.is_empty() {
                return;
            }
            is_playing.set(true);
            let on_ended = Callback::new(move |_| {
                is_playing.set(false);
            });
            crate::shared_ui::play_audio_tokens(tokens, on_ended);
        }
    };

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Nhập số tiền hoặc số bất kỳ cần đọc:"</label>
                <input
                    class="form-input"
                    type="text"
                    prop:value=input_value
                    on:compositionstart=move |_| set_is_ime_composing.set(true)
                    on:compositionend=move |_| set_is_ime_composing.set(false)
                    on:input=move |ev| {
                        set_input_value.set(event_target_value(&ev));
                    }
                    placeholder="Ví dụ: 1250000 hoặc 1.250.000"
                />
            </div>

            <div class="options-row">
                <div class="form-group">
                    <label class="form-label">"Phương ngữ:"</label>
                    <div style="display: flex; gap: 12px; margin-top: 4px;">
                        <label class="radio-item">
                            <input
                                type="radio"
                                name="dialect"
                                checked=move || dialect.get() == "North"
                                on:change=move |_| set_dialect.set("North".to_string())
                            />
                            "Miền Bắc (nghìn, lẻ)"
                        </label>
                        <label class="radio-item">
                            <input
                                type="radio"
                                name="dialect"
                                checked=move || dialect.get() == "South"
                                on:change=move |_| set_dialect.set("South".to_string())
                            />
                            "Miền Nam (ngàn, linh)"
                        </label>
                    </div>
                </div>

                <div class="form-group">
                    <label class="form-label">"Quy chuẩn kế toán:"</label>
                    <div style="display: flex; gap: 12px; margin-top: 4px;">
                        <label class="radio-item">
                            <input
                                type="checkbox"
                                checked=use_dong_chan
                                on:change=move |ev| set_use_dong_chan.set(event_target_checked(&ev))
                            />
                            "Thêm hậu tố 'đồng chẵn'"
                        </label>
                    </div>
                </div>
            </div>

            { move || {
                let res = output_result.get();
                if !res.is_empty() {
                    view! {
                        <div style="margin-top: 12px; padding: 14px; background: rgba(5, 150, 105, 0.1); border: 1px solid rgba(5, 150, 105, 0.3); border-radius: 6px;">
                            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px;">
                                <div style="font-size: 12px; color: var(--success); font-weight: 600;">"KẾT QUẢ ĐỌC SỐ CHUẨN:"</div>
                                <button
                                    type="button"
                                    class=move || if is_playing.get() { "vn-audio-btn playing" } else { "vn-audio-btn" }
                                    on:click=toggle_speak
                                >
                                    {move || if is_playing.get() { "⏹ Dừng đọc" } else { "🔊 Nghe đọc số" }}
                                </button>
                            </div>
                            <div style="font-size: 16px; font-weight: 600; color: var(--text-main);">{res}</div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
