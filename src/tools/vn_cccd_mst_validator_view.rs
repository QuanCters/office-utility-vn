use leptos::prelude::*;

#[component]
pub fn VnCccdMstValidatorView(
    input_value: ReadSignal<String>,
    set_input_value: WriteSignal<String>,
    result_json: ReadSignal<String>,
    set_is_ime_composing: WriteSignal<bool>,
) -> impl IntoView {
    let is_composing = RwSignal::new(false);

    view! {
        <div class="tool-config-panel">
            <div class="form-group">
                <label class="form-label">"Nhập số CCCD (12 chữ số) hoặc Mã số thuế (10 hoặc 13 chữ số):"</label>
                <input
                    class="form-input"
                    type="text"
                    prop:value=input_value
                    on:compositionstart=move |_| {
                        is_composing.set(true);
                        set_is_ime_composing.set(true);
                    }
                    on:compositionend=move |ev| {
                        is_composing.set(false);
                        set_is_ime_composing.set(false);
                        set_input_value.set(event_target_value(&ev));
                    }
                    on:input=move |ev| {
                        if !is_composing.get_untracked() {
                            set_input_value.set(event_target_value(&ev));
                        }
                    }
                    placeholder="Ví dụ: 001205012345 (CCCD) hoặc 0100109106 (MST 10 số)"
                />
            </div>

            { move || {
                let res = result_json.get();
                if res.is_empty() {
                    return view! { <div></div> }.into_any();
                }

                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&res) {
                    let v_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    match v_type {
                        "Cccd" => {
                            let is_valid = v.get("is_valid").and_then(|b| b.as_bool()).unwrap_or(false);
                            let cccd_number = v.get("cccd_number").and_then(|s| s.as_str()).unwrap_or("");
                            let prov_name = v.get("province_name").and_then(|s| s.as_str()).unwrap_or("");
                            let prov_code = v.get("province_code").and_then(|s| s.as_str()).unwrap_or("");
                            let gender = v.get("gender").and_then(|s| s.as_str()).unwrap_or("");
                            let birth_year = v.get("birth_year").and_then(|n| n.as_u64()).unwrap_or(0);
                            let century = v.get("century").and_then(|n| n.as_u64()).unwrap_or(0);
                            let random_no = v.get("random_number").and_then(|s| s.as_str()).unwrap_or("");

                            let badge_color = if is_valid { "var(--success)" } else { "var(--danger)" };
                            let border_color = if is_valid { "rgba(16, 185, 129, 0.3)" } else { "rgba(239, 68, 68, 0.3)" };
                            let bg_color = if is_valid { "rgba(16, 185, 129, 0.08)" } else { "rgba(239, 68, 68, 0.08)" };

                            view! {
                                <div class="cccd-result-card" style=format!("margin-top: 12px; padding: 14px; background: {bg_color}; border: 1px solid {border_color}; border-radius: 8px; display: flex; flex-direction: column; gap: 10px;")>
                                    <div style="display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid rgba(148, 163, 184, 0.15); padding-bottom: 8px;">
                                        <div style=format!("font-size: 13px; font-weight: 700; color: {badge_color};")>
                                            {if is_valid { "✓ CĂN CƯỚC CÔNG DÂN HỢP LỆ (12 SỐ)" } else { "❌ MÃ TỈNH THÀNH CCCD KHÔNG XÁC ĐỊNH" }}
                                        </div>
                                        <span style="font-size: 12px; color: var(--text-muted); background: var(--bg-card); padding: 2px 8px; border-radius: 4px; white-space: nowrap; flex-shrink: 0;">
                                            "Chuẩn Bộ Công an VN"
                                        </span>
                                    </div>

                                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 12px; font-size: 13px;">
                                        <div>
                                            <span style="color: var(--text-muted); font-size: 12px; display: block;">"Số CCCD:"</span>
                                            <span style="font-family: var(--font-mono); font-size: 15px; font-weight: 700; color: var(--text-main); letter-spacing: 1px;">{cccd_number}</span>
                                        </div>
                                        <div>
                                            <span style="color: var(--text-muted); font-size: 12px; display: block;">"Nơi khai sinh (Tỉnh/Thành):"</span>
                                            <span style="font-weight: 600; color: var(--text-main);">{format!("{prov_name} (Mã: {prov_code})")}</span>
                                        </div>
                                        <div>
                                            <span style="color: var(--text-muted); font-size: 12px; display: block;">"Giới tính & Năm sinh:"</span>
                                            <span style="font-weight: 600; color: var(--text-main);">{format!("{gender} • Năm {birth_year} (Thế kỷ {century})")}</span>
                                        </div>
                                        <div>
                                            <span style="color: var(--text-muted); font-size: 12px; display: block;">"Dãy số ngẫu nhiên:"</span>
                                            <span style="font-family: var(--font-mono); font-weight: 600; color: var(--text-muted);">{random_no}</span>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        }
                        "Mst" => {
                            let is_valid = v.get("is_valid").and_then(|b| b.as_bool()).unwrap_or(false);
                            let mst = v.get("mst").and_then(|s| s.as_str()).unwrap_or("");
                            let mst_type = v.get("mst_type").and_then(|s| s.as_str()).unwrap_or("");
                            let checksum = v.get("checksum_digit").and_then(|n| n.as_u64()).unwrap_or(0);
                            let expected = v.get("expected_checksum").and_then(|n| n.as_u64()).unwrap_or(0);
                            let branch = v.get("branch_code").and_then(|s| s.as_str());

                            let badge_color = if is_valid { "var(--success)" } else { "var(--danger)" };
                            let border_color = if is_valid { "rgba(16, 185, 129, 0.3)" } else { "rgba(239, 68, 68, 0.3)" };
                            let bg_color = if is_valid { "rgba(16, 185, 129, 0.08)" } else { "rgba(239, 68, 68, 0.08)" };

                            view! {
                                <div class="cccd-result-card" style=format!("margin-top: 12px; padding: 14px; background: {bg_color}; border: 1px solid {border_color}; border-radius: 8px; display: flex; flex-direction: column; gap: 10px;")>
                                    <div style="display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid rgba(148, 163, 184, 0.15); padding-bottom: 8px;">
                                        <div style=format!("font-size: 13px; font-weight: 700; color: {badge_color};")>
                                            {if is_valid { "✓ MÃ SỐ THUẾ HỢP LỆ (MODULO 11 CHUẨN)" } else { "❌ SAI CHỮ SỐ KIỂM TRA (CHECKSUM)" }}
                                        </div>
                                        <span style="font-size: 12px; color: var(--text-muted); background: var(--bg-card); padding: 2px 8px; border-radius: 4px; white-space: nowrap; flex-shrink: 0;">
                                            "Thuật toán Modulo 11"
                                        </span>
                                    </div>

                                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 12px; font-size: 13px;">
                                        <div>
                                            <span style="color: var(--text-muted); font-size: 12px; display: block;">"Mã số thuế:"</span>
                                            <span style="font-family: var(--font-mono); font-size: 15px; font-weight: 700; color: var(--text-main); letter-spacing: 1px;">{mst}</span>
                                        </div>
                                        <div>
                                            <span style="color: var(--text-muted); font-size: 12px; display: block;">"Loại đối tượng:"</span>
                                            <span style="font-weight: 600; color: var(--text-main);">{mst_type}</span>
                                        </div>
                                        <div>
                                            <span style="color: var(--text-muted); font-size: 12px; display: block;">"Kiểm tra Checksum (D10):"</span>
                                            <span style="font-family: var(--font-mono); font-weight: 600; color: var(--text-main);">{format!("Thực tế: {checksum} / Kỳ vọng: {expected}")}</span>
                                        </div>
                                        {if let Some(b) = branch {
                                            view! {
                                                <div>
                                                    <span style="color: var(--text-muted); font-size: 12px; display: block;">"Mã chi nhánh (3 số):"</span>
                                                    <span style="font-family: var(--font-mono); font-weight: 600; color: var(--text-main);">{b}</span>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <div></div> }.into_any()
                                        }}
                                    </div>
                                </div>
                            }.into_any()
                        }
                        "Invalid" => {
                            let reason = v.get("reason").and_then(|s| s.as_str()).unwrap_or("Đầu vào không hợp lệ");
                            view! {
                                <div style="margin-top: 12px; padding: 14px; background: rgba(245, 158, 11, 0.08); border: 1px solid rgba(245, 158, 11, 0.3); border-radius: 8px;">
                                    <div style="font-size: 13px; font-weight: 700; color: var(--warning); margin-bottom: 4px;">
                                        "⚠️ ĐẦU VÀO KHÔNG ĐÚNG ĐỊNH DẠNG"
                                    </div>
                                    <div style="font-size: 13px; color: var(--text-secondary);">{reason}</div>
                                </div>
                            }.into_any()
                        }
                        _ => {
                            view! {
                                <div style="margin-top: 12px; padding: 14px; background: rgba(30, 41, 59, 0.7); border: 1px solid rgba(148, 163, 184, 0.2); border-radius: 6px; font-family: monospace; white-space: pre-wrap; font-size: 13px; color: #E2E8F0;">
                                    {res}
                                </div>
                            }.into_any()
                        }
                    }
                } else {
                    view! {
                        <div style="margin-top: 12px; padding: 14px; background: rgba(30, 41, 59, 0.7); border: 1px solid rgba(148, 163, 184, 0.2); border-radius: 6px; font-family: monospace; white-space: pre-wrap; font-size: 13px; color: #E2E8F0;">
                            {res}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
