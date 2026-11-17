use leptos::prelude::*;

/// 2 Tab làm việc trong ngăn kéo Data Lab Drawer
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum DrawerTab {
    /// Chuỗi quy trình tự động hóa DAG
    #[default]
    WorkflowDag,
    /// Bảng soạn thảo câu lệnh DuckDB SQL tốc độ cao
    SqlScratchpad,
}

/// Ngăn kéo đáy trượt tương tác (Bottom Drawer) cho Data Lab v2.0
///
/// Tích hợp Chuỗi luồng xử lý tự động DAG (Workflow) và Bảng lệnh DuckDB SQL Scratchpad (<5ms).
#[component]
pub fn DataLabDrawer(
    drawer_open: RwSignal<bool>,
    active_tab: RwSignal<DrawerTab>,
    #[prop(into)] on_run_sql: Callback<String>,
    #[prop(into)] on_run_workflow: Callback<()>,
) -> impl IntoView {
    // Câu lệnh SQL hiện tại trong scratchpad
    let sql_input = RwSignal::new("SELECT * FROM data LIMIT 100;".to_string());
    // Trạng thái gõ Tiếng Việt IME
    let is_composing = RwSignal::new(false);
    // Node DAG đang được highlight
    let selected_node_idx = RwSignal::new(Option::<usize>::None);

    view! {
        <section
            class="data-lab-drawer"
            class:open=move || drawer_open.get()
        >
            // Header drawer với Tab switcher và nút đóng
            <div class="drawer-header">
                <div class="drawer-tab-group">
                    <button
                        type="button"
                        class="drawer-tab-btn"
                        class:active=move || active_tab.get() == DrawerTab::WorkflowDag
                        on:click=move |_| active_tab.set(DrawerTab::WorkflowDag)
                    >
                        <span class="tab-icon">"⚡"</span>
                        <span class="tab-label">"Luồng DAG"</span>
                    </button>
                    <button
                        type="button"
                        class="drawer-tab-btn"
                        class:active=move || active_tab.get() == DrawerTab::SqlScratchpad
                        on:click=move |_| active_tab.set(DrawerTab::SqlScratchpad)
                    >
                        <span class="tab-icon">"💻"</span>
                        <span class="tab-label">"SQL Scratchpad"</span>
                    </button>
                </div>

                <div class="drawer-header-actions">
                    <span class="drawer-engine-badge">
                        "⚡ In-Memory Engine DuckDB"
                    </span>
                    <button
                        type="button"
                        class="drawer-close-btn"
                        title="Đóng ngăn kéo"
                        on:click=move |_| drawer_open.set(false)
                    >
                        "✕"
                    </button>
                </div>
            </div>

            // Thân drawer tùy theo tab đang chọn
            <div class="drawer-body">
                {move || match active_tab.get() {
                    DrawerTab::WorkflowDag => {
                        view! {
                            <div class="drawer-workflow-tab">
                                <div class="dag-pipeline-desc">
                                    <span class="desc-icon">"💡"</span>
                                    <span>
                                        "Chuỗi xử lý không phá hủy dữ liệu (Non-destructive Pipeline). Tự động theo dõi các bước biến đổi dữ liệu."
                                    </span>
                                </div>

                                // Pipeline node tương tác
                                <div class="dag-pipeline-container">
                                    <div
                                        class="dag-node"
                                        class:selected=move || selected_node_idx.get() == Some(0)
                                        on:click=move |_| selected_node_idx.set(Some(0))
                                    >
                                        <div class="dag-node-icon">"📄"</div>
                                        <div class="dag-node-info">
                                            <span class="dag-node-name">"SaoKe_T10.xlsx"</span>
                                            <span class="dag-node-sub">"Nguồn gốc (50K dòng)"</span>
                                        </div>
                                    </div>

                                    <div class="dag-arrow">"➔"</div>

                                    <div
                                        class="dag-node"
                                        class:selected=move || selected_node_idx.get() == Some(1)
                                        on:click=move |_| selected_node_idx.set(Some(1))
                                    >
                                        <div class="dag-node-icon">"🔤"</div>
                                        <div class="dag-node-info">
                                            <span class="dag-node-name">"Sửa Font TCVN3"</span>
                                            <span class="dag-node-sub">"Unicode Dựng sẵn"</span>
                                        </div>
                                    </div>

                                    <div class="dag-arrow">"➔"</div>

                                    <div
                                        class="dag-node"
                                        class:selected=move || selected_node_idx.get() == Some(2)
                                        on:click=move |_| selected_node_idx.set(Some(2))
                                    >
                                        <div class="dag-node-icon">"🪪"</div>
                                        <div class="dag-node-info">
                                            <span class="dag-node-name">"Kiểm CCCD/MST"</span>
                                            <span class="dag-node-sub">"Checksum Modulo 11"</span>
                                        </div>
                                    </div>

                                    <div class="dag-arrow">"➔"</div>

                                    <div
                                        class="dag-node"
                                        class:selected=move || selected_node_idx.get() == Some(3)
                                        on:click=move |_| selected_node_idx.set(Some(3))
                                    >
                                        <div class="dag-node-icon">"📊"</div>
                                        <div class="dag-node-info">
                                            <span class="dag-node-name">"Xuất Excel"</span>
                                            <span class="dag-node-sub">"Kèm Sheet Báo Cáo"</span>
                                        </div>
                                    </div>
                                </div>

                                <div class="dag-action-row">
                                    <button
                                        type="button"
                                        class="dag-run-btn"
                                        on:click=move |_| on_run_workflow.run(())
                                    >
                                        <span class="btn-icon">"▶"</span>
                                        <span class="btn-text">"Chạy chuỗi quy trình"</span>
                                    </button>
                                </div>
                            </div>
                        }.into_any()
                    }
                    DrawerTab::SqlScratchpad => {
                        view! {
                            <div class="drawer-sql-tab">
                                // Hàng gợi ý câu lệnh mẫu
                                <div class="sql-snippets-row">
                                    <span class="snippets-label">"Mẫu gợi ý:"</span>
                                    <button
                                        type="button"
                                        class="sql-snippet-chip"
                                        on:click=move |_| {
                                            sql_input.set("SELECT * FROM data WHERE length(trim(mst)) NOT IN (10, 13);".to_string());
                                        }
                                    >
                                        "Lọc sai MST"
                                    </button>
                                    <button
                                        type="button"
                                        class="sql-snippet-chip"
                                        on:click=move |_| {
                                            sql_input.set("SELECT * FROM data ORDER BY so_tien DESC LIMIT 10;".to_string());
                                        }
                                    >
                                        "Top 10 Giao dịch"
                                    </button>
                                    <button
                                        type="button"
                                        class="sql-snippet-chip"
                                        on:click=move |_| {
                                            sql_input.set("SELECT * FROM data WHERE so_tien > 15000000;".to_string());
                                        }
                                    >
                                        "Số tiền > 15M"
                                    </button>
                                    <button
                                        type="button"
                                        class="sql-snippet-chip"
                                        on:click=move |_| {
                                            sql_input.set("SELECT * FROM data;".to_string());
                                        }
                                    >
                                        "Xóa bộ lọc"
                                    </button>
                                </div>

                                // Textarea nhập câu lệnh SQL với an toàn IME tiếng Việt
                                <div class="sql-editor-container">
                                    <textarea
                                        class="sql-textarea"
                                        rows="3"
                                        placeholder="Nhập truy vấn SQL DuckDB (ví dụ: SELECT * FROM data WHERE so_tien > 1000000)..."
                                        prop:value=move || sql_input.get()
                                        on:compositionstart=move |_| is_composing.set(true)
                                        on:compositionend=move |ev| {
                                            is_composing.set(false);
                                            let val = event_target_value(&ev);
                                            sql_input.set(val);
                                        }
                                        on:input=move |ev| {
                                            if !is_composing.get() {
                                                let val = event_target_value(&ev);
                                                sql_input.set(val);
                                            }
                                        }
                                    ></textarea>

                                    <button
                                        type="button"
                                        class="sql-execute-btn"
                                        on:click=move |_| on_run_sql.run(sql_input.get())
                                    >
                                        <span class="btn-icon">"⚡"</span>
                                        <span class="btn-text">"Thực thi SQL (<5ms DuckDB)"</span>
                                    </button>
                                </div>
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </section>
    }
}
