use leptos::prelude::*;
use crate::launcher::CommandPalette;
use crate::micro_apps::vn_office::sidebar::{vn_office_tool_icon, VnOfficeSidebar};
use crate::micro_apps::vn_office::state::{BankReference, VnOfficeState, POPULAR_VN_BANKS};
use crate::micro_apps::VN_OFFICE_TOOLS;
use crate::shared_ui::{BackButton, TaskExecutor, WorkflowRunner, cancel_task_ipc, execute_workflow_ipc};
use crate::tools::{
    EncodingMojibakeFixerView, GrossNetSalaryCalculatorView, VietnameseFontConverterView,
    VietnameseNumberToWordsView, VietQrNapas247GeneratorView, VnAddressNormalizerView,
    VnBankBinMapperView, VnCccdMstValidatorView, VnEinvoiceXmlReaderView,
    VnLoanAmortizationView, VnWorkingDaysCalcView,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["navigator", "clipboard"])]
    fn writeText(text: &str) -> js_sys::Promise;
}

/// Hàm tiện ích sao chép văn bản vào Clipboard an toàn (hỗ trợ cả WASM và Non-WASM target).
fn copy_to_clipboard(text: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = writeText(text);
    }
    let _ = text;
}

/// Panel tra cứu quy định pháp lý và danh sách ngân hàng mẫu bên phải.
#[component]
pub fn VnOfficeLegalPanel(
    state: VnOfficeState,
) -> impl IntoView {
    let (bank_search, set_bank_search) = signal(String::new());
    let (copied_msg, set_copied_msg) = signal(Option::<String>::None);

    let regs = state.regulations;

    let filtered_banks = move || {
        let q = bank_search.get().trim().to_lowercase();
        if q.is_empty() {
            POPULAR_VN_BANKS.iter().take(12).copied().collect::<Vec<BankReference>>()
        } else {
            POPULAR_VN_BANKS
                .iter()
                .filter(|b| {
                    b.bin.contains(&q)
                        || b.short_name.to_lowercase().contains(&q)
                        || b.full_name.to_lowercase().contains(&q)
                        || b.swift.to_lowercase().contains(&q)
                })
                .take(15)
                .copied()
                .collect::<Vec<BankReference>>()
        }
    };

    let on_copy_bin = move |bin: &'static str, name: &'static str| {
        copy_to_clipboard(bin);
        set_copied_msg.set(Some(format!("✓ Đã chép mã BIN {} ({})", bin, name)));
    };

    view! {
        <aside class="vn-office-right-panel" aria-label="Tra cứu quy định pháp lý">
            <div class="legal-panel-header">
                <div class="legal-panel-title">
                    <span aria-hidden="true">"📚"</span>
                    <h3>"Tra cứu Pháp lý & Ngân hàng"</h3>
                </div>
                <p class="legal-panel-subtitle">
                    "Quy chuẩn kế toán, thuế TNCN & ngân hàng Việt Nam 2026"
                </p>
            </div>

            // Thông báo sao chép
            {move || {
                if let Some(msg) = copied_msg.get() {
                    view! {
                        <div class="legal-copy-banner">
                            <span>{msg}</span>
                            <button
                                type="button"
                                class="legal-copy-banner-close"
                                on:click=move |_| set_copied_msg.set(None)
                            >
                                "✕"
                            </button>
                        </div>
                    }.into_any()
                } else {
                    view! { <span style="display: none;"></span> }.into_any()
                }
            }}

            <div class="legal-panel-content">
                // Khối 1: Thuế TNCN & Giảm trừ gia cảnh
                <section class="legal-section-card">
                    <div class="legal-card-header">
                        <span class="legal-card-icon" aria-hidden="true">"🧾"</span>
                        <h4>"Giảm trừ gia cảnh TNCN"</h4>
                    </div>
                    <div class="legal-stat-list">
                        <div class="legal-stat-item">
                            <span class="legal-stat-label">"Bản thân người nộp thuế:"</span>
                            <span class="legal-stat-value emerald">
                                {move || format!("{:.0} đ/tháng", regs.get().personal_deduction)}
                            </span>
                            <span class="legal-stat-sub">"(132.000.000 đ/năm)"</span>
                        </div>
                        <div class="legal-stat-item">
                            <span class="legal-stat-label">"Mỗi người phụ thuộc:"</span>
                            <span class="legal-stat-value emerald">
                                {move || format!("{:.0} đ/tháng", regs.get().dependent_deduction)}
                            </span>
                            <span class="legal-stat-sub">"(52.800.000 đ/năm/người)"</span>
                        </div>
                    </div>
                    <div class="legal-citation">
                        "Căn cứ: Nghị quyết 954/2020/UBTVQH14 & Luật Thuế TNCN."
                    </div>
                </section>

                // Khối 2: Tỷ lệ trích nộp BHXH, BHYT, BHTN
                <section class="legal-section-card">
                    <div class="legal-card-header">
                        <span class="legal-card-icon" aria-hidden="true">"🛡️"</span>
                        <h4>"Tỷ lệ Trích nộp Bảo hiểm"</h4>
                    </div>
                    <div class="legal-table-wrapper">
                        <table class="legal-mini-table">
                            <thead>
                                <tr>
                                    <th>"Khoản trích nộp"</th>
                                    <th>"NLĐ"</th>
                                    <th>"NSDLĐ"</th>
                                    <th>"Tổng"</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr>
                                    <td>"BHXH (Hưu trí - Tử tuất)"</td>
                                    <td class="bold">"8.0%"</td>
                                    <td>"17.5%"</td>
                                    <td>"25.5%"</td>
                                </tr>
                                <tr>
                                    <td>"BHYT (Y tế)"</td>
                                    <td class="bold">"1.5%"</td>
                                    <td>"3.0%"</td>
                                    <td>"4.5%"</td>
                                </tr>
                                <tr>
                                    <td>"BHTN (Thất nghiệp)"</td>
                                    <td class="bold">"1.0%"</td>
                                    <td>"1.0%"</td>
                                    <td>"2.0%"</td>
                                </tr>
                                <tr class="legal-table-total">
                                    <td>"Tổng cộng"</td>
                                    <td class="highlight-emerald">"10.5%"</td>
                                    <td>"21.5%"</td>
                                    <td class="bold">"32.0%"</td>
                                </tr>
                            </tbody>
                        </table>
                    </div>
                    <div class="legal-stat-list" style="margin-top: 8px;">
                        <div class="legal-stat-item">
                            <span class="legal-stat-label">"Lương cơ sở mới nhất:"</span>
                            <span class="legal-stat-value">"2.340.000 đ"</span>
                        </div>
                        <div class="legal-stat-item">
                            <span class="legal-stat-label">"Trần đóng BHXH (20x):"</span>
                            <span class="legal-stat-value">"46.800.000 đ"</span>
                        </div>
                    </div>
                    <div class="legal-citation">
                        "Căn cứ: Nghị định 73/2024/NĐ-CP & Luật BHXH mới nhất."
                    </div>
                </section>

                // Khối 3: Biểu thuế lũy tiến từng phần
                <section class="legal-section-card">
                    <div class="legal-card-header">
                        <span class="legal-card-icon" aria-hidden="true">"📈"</span>
                        <h4>"Biểu thuế TNCN Lũy tiến"</h4>
                    </div>
                    <div class="legal-tax-tiers">
                        <div class="tax-tier-row"><span>"Bậc 1 (≤ 5 tr):"</span><span class="tier-rate">"5%"</span></div>
                        <div class="tax-tier-row"><span>"Bậc 2 (5 - 10 tr):"</span><span class="tier-rate">"10%"</span></div>
                        <div class="tax-tier-row"><span>"Bậc 3 (10 - 18 tr):"</span><span class="tier-rate">"15%"</span></div>
                        <div class="tax-tier-row"><span>"Bậc 4 (18 - 32 tr):"</span><span class="tier-rate">"20%"</span></div>
                        <div class="tax-tier-row"><span>"Bậc 5 (32 - 52 tr):"</span><span class="tier-rate">"25%"</span></div>
                        <div class="tax-tier-row"><span>"Bậc 6 (52 - 80 tr):"</span><span class="tier-rate">"30%"</span></div>
                        <div class="tax-tier-row"><span>"Bậc 7 (> 80 tr):"</span><span class="tier-rate">"35%"</span></div>
                    </div>
                </section>

                // Khối 4: Danh sách ngân hàng mẫu & Mã BIN Napas 247
                <section class="legal-section-card">
                    <div class="legal-card-header">
                        <span class="legal-card-icon" aria-hidden="true">"🏛️"</span>
                        <h4>"Tra cứu Mã BIN Ngân hàng (Napas 247)"</h4>
                    </div>
                    <div class="legal-bank-search">
                        <input
                            type="text"
                            class="form-input"
                            placeholder="Nhập tên bank hoặc mã BIN..."
                            prop:value=move || bank_search.get()
                            on:input=move |ev| set_bank_search.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="legal-bank-list">
                        {move || {
                            let list = filtered_banks();
                            if list.is_empty() {
                                view! {
                                    <div class="legal-bank-empty">
                                        "Không tìm thấy ngân hàng."
                                    </div>
                                }.into_any()
                            } else {
                                list.into_iter().map(|b| {
                                    let bin = b.bin;
                                    let short_name = b.short_name;
                                    let full_name = b.full_name;
                                    let swift = b.swift;

                                    view! {
                                        <div class="legal-bank-item">
                                            <div class="legal-bank-info">
                                                <div class="legal-bank-top">
                                                    <span class="legal-bank-code">{short_name}</span>
                                                    <span class="legal-bank-bin">{bin}</span>
                                                </div>
                                                <div class="legal-bank-name">{full_name}</div>
                                                <div class="legal-bank-swift">{format!("SWIFT: {}", swift)}</div>
                                            </div>
                                            <button
                                                type="button"
                                                class="legal-bank-copy-btn"
                                                title="Sao chép mã BIN"
                                                on:click=move |_| on_copy_bin(bin, short_name)
                                            >
                                                "📋 Sao chép"
                                            </button>
                                        </div>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            }
                        }}
                    </div>
                </section>
            </div>
        </aside>
    }
}

/// Workspace chính hiển thị nội dung công cụ nghiệp vụ hoặc Workflow DAG.
#[component]
pub fn VnOfficeLayout(
    state: VnOfficeState,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    // Tín hiệu từ VnToolInputs (in-memory state persistence trên VnOfficeState)
    // 1. vietnamese-number-to-words
    let (num_input, set_num_input) = state.tool_inputs.num_input;
    let (num_dialect, set_num_dialect) = state.tool_inputs.num_dialect;
    let (num_dong_chan, set_num_dong_chan) = state.tool_inputs.num_dong_chan;
    let (num_result, set_num_result) = state.tool_inputs.num_result;
    let (_is_ime_composing, set_is_ime_composing) = state.tool_inputs.is_ime_composing;

    // 2. gross-net-salary-calculator
    let (salary_input, set_salary_input) = state.tool_inputs.salary_input;
    let (salary_region, set_salary_region) = state.tool_inputs.salary_region;
    let (salary_dependents, set_salary_dependents) = state.tool_inputs.salary_dependents;
    let (salary_result, set_salary_result) = state.tool_inputs.salary_result;

    // 3. vietqr-napas247-generator
    let (qr_bank_bin, set_qr_bank_bin) = state.tool_inputs.qr_bank_bin;
    let (qr_account, set_qr_account) = state.tool_inputs.qr_account;
    let (qr_amount, set_qr_amount) = state.tool_inputs.qr_amount;
    let (qr_message, set_qr_message) = state.tool_inputs.qr_message;
    let (qr_payload, set_qr_payload) = state.tool_inputs.qr_payload;

    // 4. vn-einvoice-xml-reader
    let (xml_content, set_xml_content) = state.tool_inputs.xml_content;
    let (xml_verify, set_xml_verify) = state.tool_inputs.xml_verify;
    let (xml_result, set_xml_result) = state.tool_inputs.xml_result;

    // 5. vn-cccd-mst-validator
    let (cccd_input, set_cccd_input) = state.tool_inputs.cccd_input;
    let (cccd_result, set_cccd_result) = state.tool_inputs.cccd_result;

    // 6. vn-address-normalizer
    let (addr_input, set_addr_input) = state.tool_inputs.addr_input;
    let (addr_model, set_addr_model) = state.tool_inputs.addr_model;
    let (addr_result, set_addr_result) = state.tool_inputs.addr_result;

    // 7. vn-working-days-calc
    let (wd_start, set_wd_start) = state.tool_inputs.wd_start;
    let (wd_end, set_wd_end) = state.tool_inputs.wd_end;
    let (wd_exclude_sat, set_wd_exclude_sat) = state.tool_inputs.wd_exclude_sat;
    let (wd_result, set_wd_result) = state.tool_inputs.wd_result;

    // 8. vn-loan-amortization
    let (loan_principal, set_loan_principal) = state.tool_inputs.loan_principal;
    let (loan_rate, set_loan_rate) = state.tool_inputs.loan_rate;
    let (loan_term, set_loan_term) = state.tool_inputs.loan_term;
    let (loan_method, set_loan_method) = state.tool_inputs.loan_method;
    let (loan_result, set_loan_result) = state.tool_inputs.loan_result;

    // 9. vn-bank-bin-mapper
    let (bank_query, set_bank_query) = state.tool_inputs.bank_query;
    let (bank_result, set_bank_result) = state.tool_inputs.bank_result;

    // 10. vietnamese-font-converter
    let (font_source, set_font_source) = state.tool_inputs.font_source;
    let (font_target, set_font_target) = state.tool_inputs.font_target;

    // 11. encoding-mojibake-fixer
    let (mojibake_auto, set_mojibake_auto) = state.tool_inputs.mojibake_auto;

    let palette_open = RwSignal::new(false);

    // Xây dựng options JSON tương ứng cho từng công cụ nghiệp vụ
    let options_builder = Callback::new(move |_| {
        let slug = state.active_tool_slug();
        match slug.as_str() {
            "vietnamese-number-to-words" => {
                serde_json::json!({
                    "number": num_input.get(),
                    "dialect": num_dialect.get(),
                    "currency": "VND",
                    "use_dong_chan": num_dong_chan.get(),
                    "capitalize_first": true,
                    "negative_prefix": "âm"
                })
            }
            "gross-net-salary-calculator" => {
                serde_json::json!({
                    "salary_gross": salary_input.get(),
                    "dependents": salary_dependents.get(),
                    "region": salary_region.get()
                })
            }
            "vietqr-napas247-generator" => {
                serde_json::json!({
                    "bank": qr_bank_bin.get(),
                    "account": qr_account.get(),
                    "amount": qr_amount.get(),
                    "note": qr_message.get()
                })
            }
            "vn-einvoice-xml-reader" => {
                serde_json::json!({
                    "xml_content": xml_content.get(),
                    "verify_signature": xml_verify.get()
                })
            }
            "vn-cccd-mst-validator" => {
                serde_json::json!({
                    "cccd": cccd_input.get()
                })
            }
            "vn-address-normalizer" => {
                serde_json::json!({
                    "address": addr_input.get(),
                    "model": addr_model.get()
                })
            }
            "vn-working-days-calc" => {
                serde_json::json!({
                    "start_date": wd_start.get(),
                    "end_date": wd_end.get(),
                    "exclude_saturday": wd_exclude_sat.get()
                })
            }
            "vn-loan-amortization" => {
                serde_json::json!({
                    "principal": loan_principal.get(),
                    "annual_rate": loan_rate.get(),
                    "months": loan_term.get(),
                    "method": loan_method.get()
                })
            }
            "vn-bank-bin-mapper" => {
                serde_json::json!({
                    "query": bank_query.get()
                })
            }
            "vietnamese-font-converter" => {
                serde_json::json!({
                    "source_encoding": font_source.get(),
                    "target_encoding": font_target.get()
                })
            }
            "encoding-mojibake-fixer" => {
                serde_json::json!({
                    "auto_detect": mojibake_auto.get()
                })
            }
            _ => serde_json::json!({}),
        }
    });

    // Callback khi tác vụ thực thi thành công: ghi nhận kết quả và lưu lịch sử
    let on_task_success = Callback::new(move |res: serde_json::Value| {
        let slug = state.active_tool_slug();
        let output_str = res.get("output_paths")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("Tác vụ xử lý hoàn tất thành công!");

        match slug.as_str() {
            "vietnamese-number-to-words" => {
                set_num_result.set(output_str.to_string());
                state.record_calculation(
                    &slug,
                    "Đọc Số Tiền Thành Chữ Chuẩn",
                    &format!("{} ({})", num_input.get(), num_dialect.get()),
                    output_str,
                );
            }
            "gross-net-salary-calculator" => {
                set_salary_result.set(output_str.to_string());
                state.record_calculation(
                    &slug,
                    "Tính Lương Gross - Net & Thuế",
                    &format!("Lương {} đ, Vùng {}, Phụ thuộc {}", salary_input.get(), salary_region.get(), salary_dependents.get()),
                    output_str,
                );
            }
            "vietqr-napas247-generator" => {
                set_qr_payload.set(output_str.to_string());
                state.record_calculation(
                    &slug,
                    "Tạo Mã VietQR Napas 247",
                    &format!("Bank {}, STK {}, Tiền {}", qr_bank_bin.get(), qr_account.get(), qr_amount.get()),
                    output_str,
                );
            }
            "vn-einvoice-xml-reader" => {
                set_xml_result.set(output_str.to_string());
            }
            "vn-cccd-mst-validator" => {
                set_cccd_result.set(output_str.to_string());
                state.record_calculation(
                    &slug,
                    "Kiểm tra CCCD & MST",
                    &cccd_input.get(),
                    output_str,
                );
            }
            "vn-address-normalizer" => {
                set_addr_result.set(output_str.to_string());
                state.record_calculation(
                    &slug,
                    "Chuẩn hóa Địa chỉ",
                    &addr_input.get(),
                    output_str,
                );
            }
            "vn-working-days-calc" => {
                set_wd_result.set(output_str.to_string());
                state.record_calculation(
                    &slug,
                    "Tính Ngày Làm Việc",
                    &format!("Từ {} đến {}", wd_start.get(), wd_end.get()),
                    output_str,
                );
            }
            "vn-loan-amortization" => {
                set_loan_result.set(output_str.to_string());
                state.record_calculation(
                    &slug,
                    "Bảng Tính Trả Nợ Vay",
                    &format!("Gốc {}, Lãi {}%, {} tháng", loan_principal.get(), loan_rate.get(), loan_term.get()),
                    output_str,
                );
            }
            "vn-bank-bin-mapper" => {
                set_bank_result.set(output_str.to_string());
            }
            _ => {}
        }
    });

    // Xử lý thực thi chạy quy trình Workflow DAG qua Tauri IPC execute_workflow_pipeline
    let on_run_workflow = Callback::new(move |dag: crate::shared_ui::WorkflowDag| {
        execute_workflow_ipc(dag, vec![], state.is_running, state.status_message);
    });

    let on_cancel_workflow = Callback::new(move |_| {
        cancel_task_ipc("");
        state.is_running.set(false);
        state.status_message.set("Đã dừng thực thi chuỗi quy trình.".to_string());
    });

    view! {
        <div class="vn-office-app-container">
            // Header trên cùng với BackButton và tiêu đề chuẩn
            <header class="vn-office-header">
                <div class="vn-office-header-left">
                    <BackButton on_back=on_back label="← Về Launcher" />
                    <div class="vn-office-header-title-group">
                        <h1 class="vn-office-header-title">
                            "🏢 VN Office — Trợ lý Nghiệp vụ Việt Nam"
                        </h1>
                        <span class="vn-office-header-tagline">
                            "Kế toán • Thuế • Ngân hàng • Chuyển mã Font • Pháp lý On-Device"
                        </span>
                    </div>
                </div>

                <div class="vn-office-header-right">
                    <span class="vn-office-header-badge">
                        "11 Công cụ nghiệp vụ"
                    </span>
                    <button
                        type="button"
                        class="vn-office-toggle-legal-btn"
                        class:active=move || state.is_legal_panel_open.get()
                        title="Bật/Tắt panel tra cứu nhanh quy định pháp lý"
                        on:click=move |_| state.is_legal_panel_open.update(|v| *v = !*v)
                    >
                        <span aria-hidden="true">"📚"</span>
                        <span>"Pháp lý & Ngân hàng"</span>
                    </button>
                </div>
            </header>

            // Khu vực làm việc chính (3 cột: Sidebar + Center Workspace + Right Legal Panel)
            <div class="vn-office-workspace-body">
                // Cột 1: Internal Sidebar
                <VnOfficeSidebar state=state />

                // Cột 2: Center Workspace (Nội dung công cụ hoặc Workflow Runner)
                <main class="vn-office-center-workspace">
                    {move || {
                        if state.is_workflow_active.get() {
                            // Chế độ Workflow DAG
                            view! {
                                <div class="vn-office-workflow-wrapper">
                                    <WorkflowRunner
                                        available_tools=VN_OFFICE_TOOLS
                                        title="⚡ Chuỗi Quy trình Nghiệp vụ VN Office"
                                        is_running=Signal::derive(move || state.is_running.get())
                                        status_message=Signal::derive(move || state.status_message.get())
                                        on_run=on_run_workflow
                                        on_cancel=on_cancel_workflow
                                    />
                                </div>
                            }.into_any()
                        } else {
                            // Chế độ công cụ đơn lẻ
                            let active_slug = state.active_tool.get();
                            let current_tool = state.find_tool(&active_slug);
                            let tool_name = current_tool.map(|t| t.name).unwrap_or("Công cụ Nghiệp vụ");
                            let tool_desc = current_tool.map(|t| t.description).unwrap_or("");
                            let icon = vn_office_tool_icon(&active_slug);

                            view! {
                                <div class="vn-office-tool-workspace">
                                    // Header công cụ
                                    <div class="vn-office-tool-card-header">
                                        <div class="vn-office-tool-title-row">
                                            <span class="vn-office-tool-icon" aria-hidden="true">{icon}</span>
                                            <div>
                                                <h2 class="vn-office-tool-name">{tool_name}</h2>
                                                <p class="vn-office-tool-description">{tool_desc}</p>
                                            </div>
                                        </div>
                                        <div class="vn-office-tool-meta">
                                            <span class="vn-office-slug-badge">{active_slug.clone()}</span>
                                            <span class="vn-office-privacy-badge">"🔒 100% On-Device"</span>
                                        </div>
                                    </div>

                                    // Khối hiển thị View của từng công cụ
                                    <div class="vn-office-tool-view-body">
                                        {match active_slug.as_str() {
                                            "vietnamese-number-to-words" => view! {
                                                <VietnameseNumberToWordsView
                                                    input_value=num_input
                                                    set_input_value=set_num_input
                                                    dialect=num_dialect
                                                    set_dialect=set_num_dialect
                                                    use_dong_chan=num_dong_chan
                                                    set_use_dong_chan=set_num_dong_chan
                                                    output_result=num_result
                                                    set_is_ime_composing=set_is_ime_composing
                                                />
                                            }.into_any(),

                                            "gross-net-salary-calculator" => view! {
                                                <GrossNetSalaryCalculatorView
                                                    salary_input=salary_input
                                                    set_salary_input=set_salary_input
                                                    region=salary_region
                                                    set_region=set_salary_region
                                                    dependents=salary_dependents
                                                    set_dependents=set_salary_dependents
                                                    result_json=salary_result
                                                    set_is_ime_composing=set_is_ime_composing
                                                />
                                            }.into_any(),

                                            "vietqr-napas247-generator" => view! {
                                                <VietQrNapas247GeneratorView
                                                    bank_bin=qr_bank_bin
                                                    set_bank_bin=set_qr_bank_bin
                                                    account_number=qr_account
                                                    set_account_number=set_qr_account
                                                    amount=qr_amount
                                                    set_amount=set_qr_amount
                                                    message=qr_message
                                                    set_message=set_qr_message
                                                    qr_payload=qr_payload
                                                    set_is_ime_composing=set_is_ime_composing
                                                />
                                            }.into_any(),

                                            "vn-einvoice-xml-reader" => view! {
                                                <VnEinvoiceXmlReaderView
                                                    xml_content=xml_content
                                                    set_xml_content=set_xml_content
                                                    verify_signature=xml_verify
                                                    set_verify_signature=set_xml_verify
                                                    invoice_json=xml_result
                                                    set_is_ime_composing=set_is_ime_composing
                                                />
                                            }.into_any(),

                                            "vn-cccd-mst-validator" => view! {
                                                <VnCccdMstValidatorView
                                                    input_value=cccd_input
                                                    set_input_value=set_cccd_input
                                                    result_json=cccd_result
                                                    set_is_ime_composing=set_is_ime_composing
                                                />
                                            }.into_any(),

                                            "vn-address-normalizer" => view! {
                                                <VnAddressNormalizerView
                                                    address_input=addr_input
                                                    set_address_input=set_addr_input
                                                    target_model=addr_model
                                                    set_target_model=set_addr_model
                                                    result_json=addr_result
                                                    set_is_ime_composing=set_is_ime_composing
                                                />
                                            }.into_any(),

                                            "vn-working-days-calc" => view! {
                                                <VnWorkingDaysCalcView
                                                    start_date=wd_start
                                                    set_start_date=set_wd_start
                                                    end_date=wd_end
                                                    set_end_date=set_wd_end
                                                    exclude_saturday=wd_exclude_sat
                                                    set_exclude_saturday=set_wd_exclude_sat
                                                    result_json=wd_result
                                                />
                                            }.into_any(),

                                            "vn-loan-amortization" => view! {
                                                <VnLoanAmortizationView
                                                    principal=loan_principal
                                                    set_principal=set_loan_principal
                                                    annual_rate=loan_rate
                                                    set_annual_rate=set_loan_rate
                                                    term_months=loan_term
                                                    set_term_months=set_loan_term
                                                    method=loan_method
                                                    set_method=set_loan_method
                                                    result_json=loan_result
                                                    _set_is_ime_composing=set_is_ime_composing
                                                />
                                            }.into_any(),

                                            "vn-bank-bin-mapper" => view! {
                                                <VnBankBinMapperView
                                                    query=bank_query
                                                    set_query=set_bank_query
                                                    result_json=bank_result
                                                    set_is_ime_composing=set_is_ime_composing
                                                />
                                            }.into_any(),

                                            "vietnamese-font-converter" => view! {
                                                <VietnameseFontConverterView
                                                    source_encoding=font_source
                                                    set_source_encoding=set_font_source
                                                    target_encoding=font_target
                                                    set_target_encoding=set_font_target
                                                />
                                            }.into_any(),

                                            "encoding-mojibake-fixer" => view! {
                                                <EncodingMojibakeFixerView
                                                    auto_detect=mojibake_auto
                                                    set_auto_detect=set_mojibake_auto
                                                />
                                            }.into_any(),

                                            _ => view! {
                                                <div class="vn-office-not-found">
                                                    "Công cụ chưa được hỗ trợ."
                                                </div>
                                            }.into_any(),
                                        }}
                                    </div>

                                    // Thanh thực thi tác vụ On-Device
                                    <TaskExecutor
                                        tool_slug=Signal::derive(move || state.active_tool_slug())
                                        options_builder=options_builder
                                        external_is_running=state.is_running
                                        on_success=on_task_success
                                    />
                                </div>
                            }.into_any()
                        }
                    }}
                </main>

                // Cột 3: Right Panel (Tra cứu nhanh quy định pháp lý)
                {move || {
                    if state.is_legal_panel_open.get() {
                        view! {
                            <VnOfficeLegalPanel state=state />
                        }.into_any()
                    } else {
                        view! { <span style="display: none;"></span> }.into_any()
                    }
                }}
            </div>

            // Scoped Command Palette (Ctrl+K) trong VN Office
            <CommandPalette
                open=palette_open
                tools=state.tools()
                on_deep_link=Callback::new(move |(_, slug): (_, String)| state.select_tool(&slug))
                on_back=on_back
            />
        </div>
    }
}
