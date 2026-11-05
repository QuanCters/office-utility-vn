use leptos::prelude::*;

#[component]
pub fn PdfSignaturePadView() -> impl IntoView {
    let (ink_color, set_ink_color) = signal("#003399".to_string());
    let (line_width, set_line_width) = signal(2.5f64);
    let (target_page, set_target_page) = signal("-1".to_string());
    let (placement, set_placement) = signal("bottom_right".to_string());
    let (custom_x, set_custom_x) = signal(36.0f64);
    let (custom_y, set_custom_y) = signal(36.0f64);
    let (_has_drawn, set_has_drawn) = signal(false);

    // Khởi tạo Canvas và lắng nghe sự kiện vẽ tay chuột / cảm ứng
    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = js_sys::eval(r#"
                (function() {
                    const canvas = document.getElementById('signature-canvas');
                    if (!canvas || canvas.__sig_init) return;
                    canvas.__sig_init = true;

                    const ctx = canvas.getContext('2d');
                    if (!ctx) return;
                    ctx.strokeStyle = '#003399';
                    ctx.lineWidth = 2.5;
                    ctx.lineCap = 'round';
                    ctx.lineJoin = 'round';

                    let drawing = false;
                    let lastX = 0;
                    let lastY = 0;

                    function getPos(e) {
                        const rect = canvas.getBoundingClientRect();
                        const clientX = e.touches ? e.touches[0].clientX : e.clientX;
                        const clientY = e.touches ? e.touches[0].clientY : e.clientY;
                        return [
                            (clientX - rect.left) * (canvas.width / rect.width),
                            (clientY - rect.top) * (canvas.height / rect.height)
                        ];
                    }

                    function startDraw(e) {
                        drawing = true;
                        [lastX, lastY] = getPos(e);
                    }

                    function draw(e) {
                        if (!drawing) return;
                        if (e.cancelable) e.preventDefault();
                        const [x, y] = getPos(e);
                        ctx.beginPath();
                        ctx.moveTo(lastX, lastY);
                        ctx.lineTo(x, y);
                        ctx.stroke();
                        [lastX, lastY] = [x, y];
                        window.__signatureDataUrl = canvas.toDataURL('image/png');
                    }

                    function stopDraw() {
                        drawing = false;
                    }

                    canvas.addEventListener('mousedown', startDraw);
                    canvas.addEventListener('mousemove', draw);
                    canvas.addEventListener('mouseup', stopDraw);
                    canvas.addEventListener('mouseleave', stopDraw);

                    canvas.addEventListener('touchstart', startDraw, { passive: false });
                    canvas.addEventListener('touchmove', draw, { passive: false });
                    canvas.addEventListener('touchend', stopDraw);
                })();
            "#);
        }
    });

    let update_pen_style = move |_color: &str, _width: f64| {
        #[cfg(target_arch = "wasm32")]
        {
            let code = format!(
                r#"
                (function() {{
                    const canvas = document.getElementById('signature-canvas');
                    if (!canvas) return;
                    const ctx = canvas.getContext('2d');
                    if (!ctx) return;
                    ctx.strokeStyle = '{}';
                    ctx.lineWidth = {};
                }})();
                "#,
                _color, _width
            );
            let _ = js_sys::eval(&code);
        }
    };

    let on_select_color = move |color: &'static str| {
        set_ink_color.set(color.to_string());
        update_pen_style(color, line_width.get());
    };

    let on_select_width = move |w: f64| {
        set_line_width.set(w);
        update_pen_style(&ink_color.get(), w);
    };

    let on_clear = move |_| {
        set_has_drawn.set(false);
        #[cfg(target_arch = "wasm32")]
        {
            let _ = js_sys::eval(r#"
                (function() {
                    const canvas = document.getElementById('signature-canvas');
                    if (!canvas) return;
                    const ctx = canvas.getContext('2d');
                    if (!ctx) return;
                    ctx.clearRect(0, 0, canvas.width, canvas.height);
                    window.__signatureDataUrl = null;
                })();
            "#);
        }
    };

    view! {
        <div class="tool-config-panel">
            // Khu vực bảng vẽ chữ ký điện tử
            <div class="form-group" style="margin-bottom: 16px;">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
                    <label class="form-label" style="font-weight: 600; font-size: 14px; margin: 0;">
                        "✍️ Khung ký tay điện tử (Vẽ bằng Chuột hoặc Bút cảm ứng):"
                    </label>
                    <button
                        type="button"
                        class="action-btn"
                        on:click=on_clear
                        style="padding: 4px 10px; font-size: 12px; font-weight: 600; background: rgba(239, 68, 68, 0.1); color: #EF4444; border: 1px solid rgba(239, 68, 68, 0.3); border-radius: 6px; cursor: pointer;"
                    >
                        "↺ Xóa làm lại"
                    </button>
                </div>

                <div style="width: 100%; display: flex; justify-content: center; background: rgba(15, 23, 42, 0.2); padding: 12px; border-radius: 8px;">
                    <canvas
                        id="signature-canvas"
                        width="600"
                        height="200"
                        style="background: #FFFFFF; border: 2px dashed #94A3B8; border-radius: 6px; cursor: crosshair; touch-action: none; width: 100%; max-width: 600px; height: 200px; box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);"
                    />
                </div>
            </div>

            // Thanh chọn màu mực & Độ dày nét bút
            <div class="options-row" style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-bottom: 16px;">
                <div class="form-group">
                    <label class="form-label">"Màu mực chữ ký:"</label>
                    <div style="display: flex; gap: 8px; margin-top: 4px;">
                        <button
                            type="button"
                            class="ink-btn"
                            style=move || format!(
                                "flex: 1; padding: 6px 10px; font-size: 12px; font-weight: 600; border-radius: 6px; cursor: pointer; display: flex; align-items: center; justify-content: center; gap: 6px; background: {}; color: {}; border: 2px solid {};",
                                if ink_color.get() == "#003399" { "rgba(0, 51, 153, 0.15)" } else { "var(--bg-surface, #1E293B)" },
                                if ink_color.get() == "#003399" { "#38BDF8" } else { "var(--text-muted, #94A3B8)" },
                                if ink_color.get() == "#003399" { "#003399" } else { "var(--border-color, #334155)" }
                            )
                            on:click=move |_| on_select_color("#003399")
                        >
                            <span style="display: inline-block; width: 12px; height: 12px; border-radius: 50%; background: #003399;"></span>
                            "Xanh (#003399)"
                        </button>

                        <button
                            type="button"
                            class="ink-btn"
                            style=move || format!(
                                "flex: 1; padding: 6px 10px; font-size: 12px; font-weight: 600; border-radius: 6px; cursor: pointer; display: flex; align-items: center; justify-content: center; gap: 6px; background: {}; color: {}; border: 2px solid {};",
                                if ink_color.get() == "#111111" { "rgba(17, 17, 17, 0.15)" } else { "var(--bg-surface, #1E293B)" },
                                if ink_color.get() == "#111111" { "#F8FAFC" } else { "var(--text-muted, #94A3B8)" },
                                if ink_color.get() == "#111111" { "#475569" } else { "var(--border-color, #334155)" }
                            )
                            on:click=move |_| on_select_color("#111111")
                        >
                            <span style="display: inline-block; width: 12px; height: 12px; border-radius: 50%; background: #111111; border: 1px solid #64748B;"></span>
                            "Đen (#111111)"
                        </button>

                        <button
                            type="button"
                            class="ink-btn"
                            style=move || format!(
                                "flex: 1; padding: 6px 10px; font-size: 12px; font-weight: 600; border-radius: 6px; cursor: pointer; display: flex; align-items: center; justify-content: center; gap: 6px; background: {}; color: {}; border: 2px solid {};",
                                if ink_color.get() == "#CC0000" { "rgba(204, 0, 0, 0.15)" } else { "var(--bg-surface, #1E293B)" },
                                if ink_color.get() == "#CC0000" { "#F87171" } else { "var(--text-muted, #94A3B8)" },
                                if ink_color.get() == "#CC0000" { "#CC0000" } else { "var(--border-color, #334155)" }
                            )
                            on:click=move |_| on_select_color("#CC0000")
                        >
                            <span style="display: inline-block; width: 12px; height: 12px; border-radius: 50%; background: #CC0000;"></span>
                            "Đỏ (#CC0000)"
                        </button>
                    </div>
                </div>

                <div class="form-group">
                    <label class="form-label">"Độ dày nét bút:"</label>
                    <div style="display: flex; gap: 8px; margin-top: 4px;">
                        <button
                            type="button"
                            class="pen-btn"
                            style=move || format!(
                                "flex: 1; padding: 6px 10px; font-size: 12px; font-weight: 600; border-radius: 6px; cursor: pointer; background: {}; color: {}; border: 1px solid {};",
                                if (line_width.get() - 1.5).abs() < 0.1 { "rgba(56, 189, 248, 0.15)" } else { "var(--bg-surface, #1E293B)" },
                                if (line_width.get() - 1.5).abs() < 0.1 { "#38BDF8" } else { "var(--text-muted, #94A3B8)" },
                                if (line_width.get() - 1.5).abs() < 0.1 { "#38BDF8" } else { "var(--border-color, #334155)" }
                            )
                            on:click=move |_| on_select_width(1.5)
                        >
                            "Mảnh (1.5px)"
                        </button>

                        <button
                            type="button"
                            class="pen-btn"
                            style=move || format!(
                                "flex: 1; padding: 6px 10px; font-size: 12px; font-weight: 600; border-radius: 6px; cursor: pointer; background: {}; color: {}; border: 1px solid {};",
                                if (line_width.get() - 2.5).abs() < 0.1 { "rgba(56, 189, 248, 0.15)" } else { "var(--bg-surface, #1E293B)" },
                                if (line_width.get() - 2.5).abs() < 0.1 { "#38BDF8" } else { "var(--text-muted, #94A3B8)" },
                                if (line_width.get() - 2.5).abs() < 0.1 { "#38BDF8" } else { "var(--border-color, #334155)" }
                            )
                            on:click=move |_| on_select_width(2.5)
                        >
                            "Vừa (2.5px)"
                        </button>

                        <button
                            type="button"
                            class="pen-btn"
                            style=move || format!(
                                "flex: 1; padding: 6px 10px; font-size: 12px; font-weight: 600; border-radius: 6px; cursor: pointer; background: {}; color: {}; border: 1px solid {};",
                                if (line_width.get() - 4.0).abs() < 0.1 { "rgba(56, 189, 248, 0.15)" } else { "var(--bg-surface, #1E293B)" },
                                if (line_width.get() - 4.0).abs() < 0.1 { "#38BDF8" } else { "var(--text-muted, #94A3B8)" },
                                if (line_width.get() - 4.0).abs() < 0.1 { "#38BDF8" } else { "var(--border-color, #334155)" }
                            )
                            on:click=move |_| on_select_width(4.0)
                        >
                            "Đậm (4.0px)"
                        </button>
                    </div>
                </div>
            </div>

            // Lựa chọn Trang mục tiêu & Preset vị trí dán chữ ký
            <div class="options-row" style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
                <div class="form-group">
                    <label class="form-label">"Trang áp dụng chữ ký:"</label>
                    <select
                        class="preset-select"
                        prop:value=target_page
                        on:change=move |ev| set_target_page.set(event_target_value(&ev))
                    >
                        <option value="-1">"Trang cuối cùng (Khuyến nghị cho hợp đồng)"</option>
                        <option value="1">"Trang đầu tiên"</option>
                        <option value="0">"Tất cả các trang (Ký nháy toàn bộ hồ sơ)"</option>
                    </select>
                </div>

                <div class="form-group">
                    <label class="form-label">"Vị trí dán chữ ký (Placement Preset):"</label>
                    <select
                        class="preset-select"
                        prop:value=placement
                        on:change=move |ev| set_placement.set(event_target_value(&ev))
                    >
                        <option value="bottom_right">"Góc dưới bên phải (Bottom Right - Tiêu chuẩn)"</option>
                        <option value="bottom_left">"Góc dưới bên trái (Bottom Left)"</option>
                        <option value="center">"Chính giữa trang (Center)"</option>
                        <option value="top_right">"Góc trên bên phải (Top Right)"</option>
                        <option value="custom">"Tọa độ tùy chỉnh (Custom X/Y)"</option>
                    </select>
                </div>
            </div>

            // Tùy chỉnh tọa độ khi chọn Custom
            {move || {
                if placement.get() == "custom" {
                    view! {
                        <div class="custom-coords-row" style="display: flex; gap: 16px; margin-top: 12px; padding: 12px; background: rgba(30, 41, 59, 0.4); border-radius: 6px;">
                            <div class="form-group" style="flex: 1; margin: 0;">
                                <label class="form-label">"Tọa độ X (point từ mép trái):"</label>
                                <input
                                    type="number"
                                    class="form-input"
                                    prop:value=move || custom_x.get().to_string()
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                            set_custom_x.set(v);
                                        }
                                    }
                                />
                            </div>
                            <div class="form-group" style="flex: 1; margin: 0;">
                                <label class="form-label">"Tọa độ Y (point từ mép dưới):"</label>
                                <input
                                    type="number"
                                    class="form-input"
                                    prop:value=move || custom_y.get().to_string()
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                            set_custom_y.set(v);
                                        }
                                    }
                                />
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}
        </div>
    }
}
