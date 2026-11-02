use leptos::prelude::*;

#[component]
pub fn ScreenshotBeautifierView(
    #[prop(into)] theme: RwSignal<String>,
    #[prop(into)] background: RwSignal<String>,
    #[prop(into)] shadow_blur: RwSignal<f32>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row">
                <div class="form-group" style="flex: 1;">
                    <label class="form-label">"Khung viền cửa sổ:"</label>
                    <select
                        class="form-select"
                        prop:value=theme
                        on:change=move |ev| theme.set(event_target_value(&ev))
                    >
                        <option value="macos_dark">"macOS Dark Mode"</option>
                        <option value="macos_light">"macOS Light Mode"</option>
                        <option value="windows_11">"Windows 11 Mica"</option>
                        <option value="none">"Không khung"</option>
                    </select>
                </div>

                <div class="form-group" style="flex: 1;">
                    <label class="form-label">"Nền Gradient nghệ thuật:"</label>
                    <select
                        class="form-select"
                        prop:value=background
                        on:change=move |ev| background.set(event_target_value(&ev))
                    >
                        <option value="sunset">"Tím hoàng hôn (Sunset)"</option>
                        <option value="ocean">"Xanh đại dương (Ocean)"</option>
                        <option value="solid">"Đơn sắc tối (Solid)"</option>
                    </select>
                </div>
            </div>

            <div class="form-group">
                <label class="form-label">
                    {move || format!("Độ mờ đổ bóng (Drop Shadow): {:.0}px", shadow_blur.get())}
                </label>
                <input
                    type="range"
                    min="4"
                    max="40"
                    prop:value=move || shadow_blur.get() as u32
                    on:input=move |ev| {
                        if let Ok(val) = event_target_value(&ev).parse::<f32>() {
                            shadow_blur.set(val);
                        }
                    }
                />
            </div>
        </div>
    }
}
