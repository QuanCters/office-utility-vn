use leptos::prelude::*;

/// 4 Tab phân loại trạng thái đối soát dữ liệu (Live Discrepancy Tabs)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ReconcileDiscrepancyTab {
    /// Khớp hoàn toàn cả mã định danh lẫn số tiền
    Matched,
    /// Khớp mã số thuế và số hóa đơn nhưng lệch số tiền
    #[default]
    MoneyDiff,
    /// Chỉ có ở Bảng A (Sổ cái nội bộ) - Thiếu file XML hóa đơn
    OnlySourceA,
    /// Chỉ có ở Bảng B (Hóa đơn Thuế) - Chưa ghi nhận trong Sổ cái
    OnlySourceB,
}

/// Bàn làm việc đối soát trực quan & so sánh 2 nguồn dữ liệu (Visual Reconcile Workbench)
///
/// Hỗ trợ map key trực quan giữa Sổ cái nội bộ và XML hóa đơn, dung sai tiền tệ
/// và lưới đối chiếu 4 tab thời gian thực.
#[component]
pub fn VisualReconcileWorkbench(
    #[prop(into, optional)] on_export_excel: Option<Callback<()>>,
    #[prop(into, optional)] on_run_reconcile: Option<Callback<()>>,
) -> impl IntoView {
    // Tab đang chọn trong bảng kết quả đối soát
    let active_tab = RwSignal::new(ReconcileDiscrepancyTab::MoneyDiff);
    // Dung sai tiền tệ (Tolerance slider: ± 1,000 VNĐ)
    let tolerance_vnd = RwSignal::new(1000i64);
    // Key mapping giữa 2 nguồn
    let key1_a = RwSignal::new("ma_so_thue".to_string());
    let key1_b = RwSignal::new("MST".to_string());
    let key2_a = RwSignal::new("so_hd".to_string());
    let key2_b = RwSignal::new("SoHoaDon".to_string());
    let key3_a = RwSignal::new("tong_tien".to_string());
    let key3_b = RwSignal::new("TongTien".to_string());

    let is_reconciling = RwSignal::new(false);
    let run_count = RwSignal::new(0);

    let handle_run_reconcile = move |_| {
        is_reconciling.set(true);
        run_count.update(|c| *c += 1);
        if let Some(cb) = on_run_reconcile {
            cb.run(());
        }
        is_reconciling.set(false);
    };

    let handle_export = move |_| {
        if let Some(cb) = on_export_excel {
            cb.run(());
        }
    };

    view! {
        <div class="visual-reconcile-workbench">
            // UPPER: Split Source Mapper
            <div class="reconcile-upper-mapper">
                <div class="mapper-header">
                    <div class="mapper-title-group">
                        <span class="mapper-icon">"☍"</span>
                        <h2 class="mapper-heading">"CẤU HÌNH ĐỐI SOÁT HAI NGUỒN DỮ LIỆU"</h2>
                    </div>
                    <span class="mapper-speed-badge">"⚡ Tốc độ đối soát <100ms in-memory"</span>
                </div>

                <div class="mapper-sources-grid">
                    // Nguồn A: Sổ cái nội bộ Excel
                    <div class="source-card source-a-card">
                        <div class="source-card-badge">"BẢNG A (Nội bộ)"</div>
                        <div class="source-card-title">"Sổ cái nội bộ Excel"</div>
                        <div class="source-card-sub">"500 dòng nạp từ SaoKe_T10.xlsx"</div>
                    </div>

                    // Cột nối Mapping Selectors
                    <div class="mapper-keys-connector">
                        <div class="mapper-key-pair">
                            <span class="key-label-a">{move || key1_a.get()}</span>
                            <span class="key-link-icon">"⟷"</span>
                            <span class="key-label-b">{move || key1_b.get()}</span>
                        </div>
                        <div class="mapper-key-pair">
                            <span class="key-label-a">{move || key2_a.get()}</span>
                            <span class="key-link-icon">"⟷"</span>
                            <span class="key-label-b">{move || key2_b.get()}</span>
                        </div>
                        <div class="mapper-key-pair">
                            <span class="key-label-a">{move || key3_a.get()}</span>
                            <span class="key-link-icon">"⟷"</span>
                            <span class="key-label-b">{move || key3_b.get()}</span>
                        </div>
                    </div>

                    // Nguồn B: 500 file XML Hóa đơn Thuế
                    <div class="source-card source-b-card">
                        <div class="source-card-badge">"BẢNG B (Thuế TT78)"</div>
                        <div class="source-card-title">"500 file XML Hóa đơn Thuế"</div>
                        <div class="source-card-sub">"Thư mục 500_HD_XML/ (Cơ quan Thuế)"</div>
                    </div>
                </div>

                // Toolbar Dung sai tiền tệ & Nút Chạy đối soát tức thì
                <div class="mapper-control-bar">
                    <div class="tolerance-control-group">
                        <label class="tolerance-label" for="reconcile-tolerance-slider">
                            <span>"Dung sai làm tròn tiền tệ:"</span>
                            <strong class="tolerance-value">
                                {move || format!("± {} VNĐ", tolerance_vnd.get())}
                            </strong>
                        </label>
                        <input
                            id="reconcile-tolerance-slider"
                            type="range"
                            min="0"
                            max="50000"
                            step="500"
                            class="tolerance-slider"
                            prop:value=move || tolerance_vnd.get()
                            on:input=move |ev| {
                                if let Ok(val) = event_target_value(&ev).parse::<i64>() {
                                    tolerance_vnd.set(val);
                                }
                            }
                        />
                    </div>

                    <button
                        type="button"
                        class="reconcile-run-btn"
                        on:click=handle_run_reconcile
                    >
                        <span class="btn-icon">"⚡"</span>
                        <span class="btn-text">"Chạy Đối Soát Tức Thì (< 100ms)"</span>
                    </button>
                </div>
            </div>

            // LOWER: 4-Tab Live Discrepancy Grid
            <div class="reconcile-lower-grid">
                <div class="discrepancy-header-bar">
                    <div class="discrepancy-tabs-list">
                        <button
                            type="button"
                            class="discrepancy-tab-btn"
                            class:active=move || active_tab.get() == ReconcileDiscrepancyTab::Matched
                            on:click=move |_| active_tab.set(ReconcileDiscrepancyTab::Matched)
                        >
                            "✅ Khớp hoàn toàn (482)"
                        </button>
                        <button
                            type="button"
                            class="discrepancy-tab-btn highlight-warn"
                            class:active=move || active_tab.get() == ReconcileDiscrepancyTab::MoneyDiff
                            on:click=move |_| active_tab.set(ReconcileDiscrepancyTab::MoneyDiff)
                        >
                            "⚠️ Khớp mã nhưng Lệch tiền (11)"
                        </button>
                        <button
                            type="button"
                            class="discrepancy-tab-btn"
                            class:active=move || active_tab.get() == ReconcileDiscrepancyTab::OnlySourceA
                            on:click=move |_| active_tab.set(ReconcileDiscrepancyTab::OnlySourceA)
                        >
                            "⬅️ Chỉ có ở Bảng A - Thiếu XML (5)"
                        </button>
                        <button
                            type="button"
                            class="discrepancy-tab-btn"
                            class:active=move || active_tab.get() == ReconcileDiscrepancyTab::OnlySourceB
                            on:click=move |_| active_tab.set(ReconcileDiscrepancyTab::OnlySourceB)
                        >
                            "➡️ Chỉ có ở Bảng B - Chưa kê khai (2)"
                        </button>
                    </div>

                    <button
                        type="button"
                        class="discrepancy-export-btn"
                        title="Xuất biên bản đối soát và danh sách chênh lệch ra Excel (.xlsx)"
                        on:click=handle_export
                    >
                        <span class="btn-icon">"📥"</span>
                        <span class="btn-text">"Xuất Biên Bản Đối Soát Excel"</span>
                    </button>
                </div>

                // Bảng hiển thị so sánh 2 bên
                <div class="discrepancy-table-container">
                    <table class="discrepancy-table">
                        <thead>
                            <tr>
                                <th class="col-stt">"STT"</th>
                                <th class="col-src-a">"Mã Số Thuế (A)"</th>
                                <th class="col-src-a">"Số HĐ (A)"</th>
                                <th class="col-src-a">"Tổng Tiền A (VNĐ)"</th>
                                <th class="col-divider">"⟷"</th>
                                <th class="col-src-b">"MST XML (B)"</th>
                                <th class="col-src-b">"Số HĐ XML (B)"</th>
                                <th class="col-src-b">"Tổng Tiền B (VNĐ)"</th>
                                <th class="col-diff">"Chênh Lệch Tiền"</th>
                                <th class="col-status">"Trạng Thái"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || match active_tab.get() {
                                ReconcileDiscrepancyTab::MoneyDiff => {
                                    view! {
                                        <>
                                            <tr class="discrepancy-row row-diff-highlight">
                                                <td class="col-stt">"1"</td>
                                                <td class="col-src-a">"0101234567"</td>
                                                <td class="col-src-a">"0001245"</td>
                                                <td class="col-src-a">"15,400,000"</td>
                                                <td class="col-divider">"≠"</td>
                                                <td class="col-src-b">"0101234567"</td>
                                                <td class="col-src-b">"0001245"</td>
                                                <td class="col-src-b">"15,200,000"</td>
                                                <td class="col-diff diff-red">"+200,000 đ"</td>
                                                <td class="col-status"><span class="status-pill status-warn">"Lệch tiền"</span></td>
                                            </tr>
                                            <tr class="discrepancy-row row-diff-highlight">
                                                <td class="col-stt">"2"</td>
                                                <td class="col-src-a">"0314567890"</td>
                                                <td class="col-src-a">"0008912"</td>
                                                <td class="col-src-a">"8,750,000"</td>
                                                <td class="col-divider">"≠"</td>
                                                <td class="col-src-b">"0314567890"</td>
                                                <td class="col-src-b">"0008912"</td>
                                                <td class="col-src-b">"8,800,000"</td>
                                                <td class="col-diff diff-red">"-50,000 đ"</td>
                                                <td class="col-status"><span class="status-pill status-warn">"Lệch tiền"</span></td>
                                            </tr>
                                            <tr class="discrepancy-row row-diff-highlight">
                                                <td class="col-stt">"3"</td>
                                                <td class="col-src-a">"0109988776"</td>
                                                <td class="col-src-a">"0000311"</td>
                                                <td class="col-src-a">"42,000,000"</td>
                                                <td class="col-divider">"≠"</td>
                                                <td class="col-src-b">"0109988776"</td>
                                                <td class="col-src-b">"0000311"</td>
                                                <td class="col-src-b">"41,500,000"</td>
                                                <td class="col-diff diff-red">"+500,000 đ"</td>
                                                <td class="col-status"><span class="status-pill status-warn">"Lệch tiền"</span></td>
                                            </tr>
                                        </>
                                    }.into_any()
                                }
                                ReconcileDiscrepancyTab::Matched => {
                                    view! {
                                        <>
                                            <tr class="discrepancy-row row-matched">
                                                <td class="col-stt">"1"</td>
                                                <td class="col-src-a">"0108889999"</td>
                                                <td class="col-src-a">"0005541"</td>
                                                <td class="col-src-a">"12,000,000"</td>
                                                <td class="col-divider">"="</td>
                                                <td class="col-src-b">"0108889999"</td>
                                                <td class="col-src-b">"0005541"</td>
                                                <td class="col-src-b">"12,000,000"</td>
                                                <td class="col-diff diff-green">"0 đ"</td>
                                                <td class="col-status"><span class="status-pill status-ok">"Khớp chuẩn"</span></td>
                                            </tr>
                                            <tr class="discrepancy-row row-matched">
                                                <td class="col-stt">"2"</td>
                                                <td class="col-src-a">"0307771122"</td>
                                                <td class="col-src-a">"0002190"</td>
                                                <td class="col-src-a">"3,450,000"</td>
                                                <td class="col-divider">"="</td>
                                                <td class="col-src-b">"0307771122"</td>
                                                <td class="col-src-b">"0002190"</td>
                                                <td class="col-src-b">"3,450,000"</td>
                                                <td class="col-diff diff-green">"0 đ"</td>
                                                <td class="col-status"><span class="status-pill status-ok">"Khớp chuẩn"</span></td>
                                            </tr>
                                        </>
                                    }.into_any()
                                }
                                ReconcileDiscrepancyTab::OnlySourceA => {
                                    view! {
                                        <tr class="discrepancy-row row-missing">
                                            <td class="col-stt">"1"</td>
                                            <td class="col-src-a">"0107776655"</td>
                                            <td class="col-src-a">"0009981"</td>
                                            <td class="col-src-a">"5,600,000"</td>
                                            <td class="col-divider">"∅"</td>
                                            <td class="col-src-b text-muted">"(Không có)"</td>
                                            <td class="col-src-b text-muted">"(Chưa có)"</td>
                                            <td class="col-src-b text-muted">"-"</td>
                                            <td class="col-diff diff-red">"Thiếu XML"</td>
                                            <td class="col-status"><span class="status-pill status-danger">"Chưa có XML"</span></td>
                                        </tr>
                                    }.into_any()
                                }
                                ReconcileDiscrepancyTab::OnlySourceB => {
                                    view! {
                                        <tr class="discrepancy-row row-missing">
                                            <td class="col-stt">"1"</td>
                                            <td class="col-src-a text-muted">"(Chưa vào sổ)"</td>
                                            <td class="col-src-a text-muted">"-"</td>
                                            <td class="col-src-a text-muted">"-"</td>
                                            <td class="col-divider">"∅"</td>
                                            <td class="col-src-b">"0312233445"</td>
                                            <td class="col-src-b">"0001199"</td>
                                            <td class="col-src-b">"18,300,000"</td>
                                            <td class="col-diff diff-red">"Chưa kê khai"</td>
                                            <td class="col-status"><span class="status-pill status-danger">"Chưa vào Sổ Cái"</span></td>
                                        </tr>
                                    }.into_any()
                                }
                            }}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    }
}
