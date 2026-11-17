use leptos::prelude::*;
use crate::micro_apps::data_lab::command_bar::DataLabWorkspaceMode;
use crate::micro_apps::data_lab::spreadsheet_grid_view::ColumnType;

/// Mô tả một bước biến đổi trong quy trình xử lý dữ liệu (Power Query style)
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecipeStepDisplay {
    pub id: usize,
    pub title: String,
    pub diff_badge: String,
    pub is_enabled: bool,
}

impl RecipeStepDisplay {
    pub fn new(id: usize, title: impl Into<String>, diff_badge: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            diff_badge: diff_badge.into(),
            is_enabled: true,
        }
    }

    pub fn demo_steps() -> Vec<RecipeStepDisplay> {
        vec![
            RecipeStepDisplay::new(1, "1. Nạp SaoKe_T10.xlsx", "50,000 dòng"),
            RecipeStepDisplay::new(2, "2. Sửa Font TCVN3", "✨ 142 ô"),
            RecipeStepDisplay::new(3, "3. Gom cụm mờ", "-38 trùng"),
            RecipeStepDisplay::new(4, "4. Tách Họ Tên", "+2 cột"),
        ]
    }
}

/// Panel Thanh tra cột & Lịch sử bước xử lý (Right Inspector: Contextual Tool & Applied Steps)
#[component]
pub fn DataLabInspector(
    selected_col_name: Signal<String>,
    selected_col_type: Signal<ColumnType>,
    applied_steps: RwSignal<Vec<RecipeStepDisplay>>,
    #[prop(into)] on_quick_action: Callback<String>,
    #[prop(into)] on_toggle_step: Callback<usize>,
    #[prop(into)] on_remove_step: Callback<usize>,
    #[prop(into)] on_apply_fuzzy_merge: Callback<()>,
    #[prop(optional)] workspace_mode: Option<Signal<DataLabWorkspaceMode>>,
) -> impl IntoView {
    // Trạng thái checkbox gom cụm mờ (Fuzzy Clustering)
    let cluster1_checked = RwSignal::new(true);
    let cluster2_checked = RwSignal::new(true);

    // Tính toán phân bổ dữ liệu động theo kiểu cột đang chọn
    let get_facets_for_col = move || {
        let col_name = selected_col_name.get();
        let col_type = selected_col_type.get();

        match col_type {
            ColumnType::Number => vec![
                ("> 30.000.000 đ".to_string(), 28),
                ("20.000.000 - 30.000.000 đ".to_string(), 44),
                ("< 20.000.000 đ".to_string(), 24),
                ("(Trống / Null)".to_string(), 4),
            ],
            ColumnType::Date => vec![
                ("Năm 1990 - 1996".to_string(), 52),
                ("Năm 1980 - 1989".to_string(), 32),
                ("Trước năm 1980".to_string(), 12),
                ("(Trống / Null)".to_string(), 4),
            ],
            ColumnType::CccdMst => vec![
                ("Chuẩn 12 số CCCD".to_string(), 84),
                ("MST 10 số Doanh nghiệp".to_string(), 12),
                ("Lỗi Checksum Modulo 11".to_string(), 4),
            ],
            ColumnType::Address => vec![
                ("Hà Nội".to_string(), 44),
                ("TP. Hồ Chí Minh".to_string(), 36),
                ("Đà Nẵng / Miền Trung".to_string(), 16),
                ("(Khác / Trống)".to_string(), 4),
            ],
            ColumnType::Text => {
                if col_name.contains("Phòng") || col_name.contains("ban") {
                    vec![
                        ("Kỹ thuật / R&D".to_string(), 36),
                        ("Kinh doanh / Marketing".to_string(), 32),
                        ("Kế toán / Tài chính".to_string(), 20),
                        ("Ban Giám đốc / Khác".to_string(), 12),
                    ]
                } else if col_name.contains("Họ") || col_name.contains("tên") {
                    vec![
                        ("Họ Nguyễn".to_string(), 38),
                        ("Họ Trần".to_string(), 24),
                        ("Họ Lê".to_string(), 18),
                        ("Họ khác".to_string(), 20),
                    ]
                } else {
                    vec![
                        ("Hà Nội".to_string(), 42),
                        ("TP. Hồ Chí Minh".to_string(), 35),
                        ("Đà Nẵng".to_string(), 15),
                        ("(Khác / Trống)".to_string(), 8),
                    ]
                }
            }
            ColumnType::Other => vec![
                ("Đạt chuẩn dữ liệu".to_string(), 92),
                ("Nghi ngờ sai lệch".to_string(), 8),
            ],
        }
    };

    view! {
        <aside class="data-lab-inspector-panel">
            // Contextual Mode Banner
            {move || {
                if let Some(mode_sig) = workspace_mode {
                    match mode_sig.get() {
                        DataLabWorkspaceMode::VisualReconcile => view! {
                            <div class="inspector-mode-banner reconcile-banner">
                                <span class="mode-banner-badge">"⚖️ ĐỐI SOÁT"</span>
                                <span class="mode-banner-desc">"So khớp 2 chiều ERP & Sao kê ngân hàng"</span>
                            </div>
                        }.into_any(),
                        DataLabWorkspaceMode::TextCodecs => view! {
                            <div class="inspector-mode-banner codecs-banner">
                                <span class="mode-banner-badge">"🔤 BẢNG MÃ"</span>
                                <span class="mode-banner-desc">"Tự động nhận diện TCVN3 / VNI / Unicode"</span>
                            </div>
                        }.into_any(),
                        DataLabWorkspaceMode::TableClean => view! { <span style="display: none;"></span> }.into_any(),
                    }
                } else {
                    view! { <span style="display: none;"></span> }.into_any()
                }
            }}

            // UPPER SECTION: Contextual Tool & Facet Inspector
            <div class="inspector-section inspector-upper-section">
                <div class="inspector-section-header">
                    <span class="inspector-col-badge">
                        {move || selected_col_type.get().icon()}
                    </span>
                    <h2 class="inspector-col-title" title=move || selected_col_name.get()>
                        {move || {
                            let name = selected_col_name.get();
                            if name.trim().is_empty() {
                                "Chọn cột để xem công cụ".to_string()
                            } else {
                                format!("Cột: {}", name)
                            }
                        }}
                    </h2>
                </div>

                // Tự động hiển thị công cụ ngữ cảnh theo selected_col_type
                <div class="inspector-contextual-tools">
                    {move || match selected_col_type.get() {
                        ColumnType::Text => {
                            view! {
                                <div class="contextual-tool-block">
                                    <div class="tool-block-label">"Tác vụ Văn bản nhanh:"</div>
                                    <div class="tool-btn-grid">
                                        <button
                                            type="button"
                                            class="inspector-action-btn"
                                            on:click=move |_| on_quick_action.run("split_vietnamese_name".to_string())
                                        >
                                            "Tách Họ - Đệm - Tên"
                                        </button>
                                        <button
                                            type="button"
                                            class="inspector-action-btn"
                                            on:click=move |_| on_quick_action.run("normalize_casing".to_string())
                                        >
                                            "Chuẩn hóa Hoa/Thường"
                                        </button>
                                        <button
                                            type="button"
                                            class="inspector-action-btn"
                                            on:click=move |_| on_quick_action.run("fix_tcvn3_font".to_string())
                                        >
                                            "Sửa Lỗi Font TCVN3"
                                        </button>
                                        <button
                                            type="button"
                                            class="inspector-action-btn highlight"
                                            on:click=move |_| on_quick_action.run("fuzzy_clustering".to_string())
                                        >
                                            "Gom Cụm Mờ (Fuzzy)"
                                        </button>
                                    </div>

                                    // Inline Fuzzy Clustering list với checkbox
                                    <div class="fuzzy-clusters-container">
                                        <div class="fuzzy-clusters-title">
                                            "Phát hiện trùng lặp viết sai chính tả:"
                                        </div>
                                        <label class="fuzzy-cluster-item">
                                            <input
                                                type="checkbox"
                                                prop:checked=move || cluster1_checked.get()
                                                on:change=move |ev| cluster1_checked.set(event_target_checked(&ev))
                                            />
                                            <span class="cluster-text">
                                                <span class="cluster-main">"Trần Thị Trúc"</span>
                                                <span class="cluster-sep">"↔"</span>
                                                <span class="cluster-alt">"Chần Thị Chúc"</span>
                                            </span>
                                        </label>
                                        <label class="fuzzy-cluster-item">
                                            <input
                                                type="checkbox"
                                                prop:checked=move || cluster2_checked.get()
                                                on:change=move |ev| cluster2_checked.set(event_target_checked(&ev))
                                            />
                                            <span class="cluster-text">
                                                <span class="cluster-main">"Nguyễn Văn An"</span>
                                                <span class="cluster-sep">"↔"</span>
                                                <span class="cluster-alt">"Nguyễn  Văn  An"</span>
                                            </span>
                                        </label>
                                        <button
                                            type="button"
                                            class="fuzzy-apply-btn"
                                            on:click=move |_| on_apply_fuzzy_merge.run(())
                                        >
                                            "Áp dụng gom cụm"
                                        </button>
                                    </div>
                                </div>
                            }.into_any()
                        }
                        ColumnType::CccdMst => {
                            view! {
                                <div class="contextual-tool-block">
                                    <div class="tool-block-label">"Kiểm định Định danh Việt Nam:"</div>
                                    <div class="tool-btn-grid">
                                        <button
                                            type="button"
                                            class="inspector-action-btn"
                                            on:click=move |_| on_quick_action.run("validate_cccd_12".to_string())
                                        >
                                            "Kiểm định CCCD 12 số (Mã tỉnh/Năm sinh)"
                                        </button>
                                        <button
                                            type="button"
                                            class="inspector-action-btn"
                                            on:click=move |_| on_quick_action.run("validate_mst_tax".to_string())
                                        >
                                            "Kiểm định MST 10/13 số (Modulo 11)"
                                        </button>
                                    </div>
                                </div>
                            }.into_any()
                        }
                        ColumnType::Number => {
                            view! {
                                <div class="contextual-tool-block">
                                    <div class="tool-block-label">"Tác vụ Số & Tiền tệ:"</div>
                                    <div class="tool-btn-grid">
                                        <button
                                            type="button"
                                            class="inspector-action-btn"
                                            on:click=move |_| on_quick_action.run("number_to_vnd_words".to_string())
                                        >
                                            "Đọc số thành chữ VND"
                                        </button>
                                        <button
                                            type="button"
                                            class="inspector-action-btn"
                                            on:click=move |_| on_quick_action.run("detect_outliers".to_string())
                                        >
                                            "Lọc dị biệt (Outliers Boxplot)"
                                        </button>
                                    </div>
                                </div>
                            }.into_any()
                        }
                        ColumnType::Address => {
                            view! {
                                <div class="contextual-tool-block">
                                    <div class="tool-block-label">"Địa giới Hành chính:"</div>
                                    <div class="tool-btn-grid">
                                        <button
                                            type="button"
                                            class="inspector-action-btn"
                                            on:click=move |_| on_quick_action.run("split_vietnam_address_3_levels".to_string())
                                        >
                                            "Tách Tỉnh - Huyện - Xã (3 cấp)"
                                        </button>
                                    </div>
                                </div>
                            }.into_any()
                        }
                        ColumnType::Date => {
                            view! {
                                <div class="contextual-tool-block">
                                    <div class="tool-block-label">"Tác vụ Ngày tháng:"</div>
                                    <div class="tool-btn-grid">
                                        <button
                                            type="button"
                                            class="inspector-action-btn"
                                            on:click=move |_| on_quick_action.run("normalize_date_format".to_string())
                                        >
                                            "Chuẩn hóa DD/MM/YYYY ISO-8601"
                                        </button>
                                    </div>
                                </div>
                            }.into_any()
                        }
                        ColumnType::Other => {
                            view! {
                                <div class="contextual-tool-block">
                                    <div class="tool-block-label">"Công cụ Chung:"</div>
                                    <div class="tool-btn-grid">
                                        <button
                                            type="button"
                                            class="inspector-action-btn"
                                            on:click=move |_| on_quick_action.run("auto_detect_type".to_string())
                                        >
                                            "Tự động nhận diện kiểu dữ liệu"
                                        </button>
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }}
                </div>

                // Thống kê Facet tần suất các giá trị hàng đầu
                <div class="inspector-facet-stats">
                    <div class="facet-header">
                        <span class="facet-title">"Phân bổ giá trị hàng đầu (Top Facets)"</span>
                    </div>
                    <div class="facet-bars-list">
                        {move || {
                            get_facets_for_col().into_iter().map(|(name, pct)| {
                                view! {
                                    <div class="facet-bar-row">
                                        <div class="facet-info">
                                            <span class="facet-name">{name}</span>
                                            <span class="facet-pct">{format!("{}%", pct)}</span>
                                        </div>
                                        <div class="facet-track">
                                            <div class="facet-fill" style=format!("width: {}%;", pct)></div>
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </div>
                </div>
            </div>

            // LOWER SECTION: Applied Steps (Power Query style)
            <div class="inspector-section inspector-lower-section">
                <div class="inspector-section-header">
                    <span class="applied-steps-icon">"📜"</span>
                    <h2 class="inspector-col-title">"CÁC BƯỚC ĐÃ ÁP DỤNG"</h2>
                </div>

                <div class="applied-steps-list">
                    {move || {
                        let steps = applied_steps.get();
                        let display_steps = if steps.is_empty() {
                            RecipeStepDisplay::demo_steps()
                        } else {
                            steps
                        };

                        display_steps.into_iter().enumerate().map(|(idx, step)| {
                            let is_enabled = step.is_enabled;
                            let diff_badge = step.diff_badge.clone();
                            let title = step.title.clone();
                            view! {
                                <div
                                    class="applied-step-card"
                                    class:disabled=!is_enabled
                                >
                                    <button
                                        type="button"
                                        class="step-toggle-eye-btn"
                                        class:active=is_enabled
                                        title="Bật/Tắt xem trước bước này (Time-travel)"
                                        on:click=move |_| on_toggle_step.run(idx)
                                    >
                                        {if is_enabled { "👁️" } else { "👁️‍🗨️" }}
                                    </button>

                                    <span class="step-card-title">{title}</span>

                                    <span class="step-diff-badge">{diff_badge}</span>

                                    <button
                                        type="button"
                                        class="step-remove-btn"
                                        title="Xóa bước này khỏi chuỗi quy trình"
                                        on:click=move |_| on_remove_step.run(idx)
                                    >
                                        "✕"
                                    </button>
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    }}
                </div>
            </div>
        </aside>
    }
}
