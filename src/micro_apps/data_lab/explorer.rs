use leptos::prelude::*;
use crate::micro_apps::data_lab::state::DataLabState;

/// Panel Explorer cây thư mục dữ liệu và mẫu công thức lưu sẵn (Shared Datasets & Saved Recipes).
#[component]
pub fn DataLabExplorer(
    state: DataLabState,
    #[prop(into)] on_load_dataset: Callback<String>,
    #[prop(into)] on_apply_recipe_template: Callback<String>,
) -> impl IntoView {
    // Trạng thái thu gọn/mở rộng cây thư mục dataset mẫu
    let saoke_expanded = RwSignal::new(true);

    view! {
        <aside class="data-lab-explorer-panel">
            // SECTION 1: Dữ liệu đang mở (Shared Datasets)
            <div class="explorer-section">
                <div class="explorer-section-header">
                    <div class="explorer-title-wrap">
                        <span class="explorer-section-icon">"📁"</span>
                        <h2 class="explorer-section-heading">
                            "DỮ LIỆU ĐANG MỞ"
                        </h2>
                    </div>
                    <button
                        type="button"
                        class="explorer-btn-add-file"
                        title="Thêm tệp bảng tính Excel/CSV/JSON mới vào workspace"
                        on:click={
                            let state_ref = state;
                            move |_| {
                                // Thêm file demo hoặc kích hoạt tải file
                                state_ref.add_file("SaoKe_T10.xlsx");
                                on_load_dataset.run("SaoKe_T10.xlsx".to_string());
                            }
                        }
                    >
                        "+ Thêm tệp"
                    </button>
                </div>

                <div class="explorer-tree-view">
                    // Tree item 1: SaoKe_T10.xlsx (Expandable)
                    <div class="explorer-tree-group">
                        <div
                            class="explorer-tree-item tree-parent-item"
                            on:click=move |_| saoke_expanded.update(|v| *v = !*v)
                        >
                            <span class="tree-toggle-arrow">
                                {move || if saoke_expanded.get() { "▾" } else { "▸" }}
                            </span>
                            <span class="tree-item-icon">"📄"</span>
                            <span class="tree-item-title">"SaoKe_T10.xlsx"</span>
                        </div>

                        {move || {
                            if saoke_expanded.get() {
                                view! {
                                    <div class="explorer-tree-children">
                                        <div
                                            class="explorer-tree-subitem"
                                            on:click=move |_| {
                                                state.set_sheet("Sheet1");
                                                on_load_dataset.run("SaoKe_T10.xlsx::Sheet1".to_string());
                                            }
                                        >
                                            <span class="tree-subitem-icon">"📑"</span>
                                            <span class="tree-subitem-label">"Sheet1"</span>
                                            <span class="tree-subitem-meta">"(50,000 dòng)"</span>
                                        </div>
                                        <div
                                            class="explorer-tree-subitem"
                                            on:click=move |_| {
                                                state.set_sheet("Sheet2");
                                                on_load_dataset.run("SaoKe_T10.xlsx::Sheet2".to_string());
                                            }
                                        >
                                            <span class="tree-subitem-icon">"📑"</span>
                                            <span class="tree-subitem-label">"Sheet2"</span>
                                            <span class="tree-subitem-meta">"(1,200 dòng)"</span>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                ().into_any()
                            }
                        }}
                    </div>

                    // Tree item 2: 500_HD_XML/ (500 hóa đơn TT78)
                    <div
                        class="explorer-tree-item"
                        on:click=move |_| on_load_dataset.run("500_HD_XML".to_string())
                    >
                        <span class="tree-leaf-spacer"></span>
                        <span class="tree-item-icon">"🧾"</span>
                        <span class="tree-item-title">"500_HD_XML/"</span>
                        <span class="tree-item-meta">"(500 hóa đơn TT78)"</span>
                    </div>

                    // Tree item 3: Từ PDF Studio (Bảng trích xuất)
                    <div
                        class="explorer-tree-item"
                        on:click=move |_| on_load_dataset.run("PDF_Studio_Extracted_Table".to_string())
                    >
                        <span class="tree-leaf-spacer"></span>
                        <span class="tree-item-icon">"📑"</span>
                        <span class="tree-item-title">"Từ PDF Studio"</span>
                        <span class="tree-item-meta">"(Bảng trích xuất)"</span>
                    </div>

                    // Danh sách file thực tế trong state.loaded_files
                    {move || {
                        let files = state.loaded_files.get();
                        let active_idx = state.active_file_index.get();
                        if files.is_empty() {
                            view! { <div class="explorer-empty-files-hint">"Chưa có tệp nạp thêm"</div> }.into_any()
                        } else {
                            view! {
                                <div class="explorer-loaded-files-group">
                                    <div class="explorer-loaded-files-subheading">
                                        "Tệp đã nạp vào workspace:"
                                    </div>
                                    {files.into_iter().enumerate().map(|(idx, path)| {
                                        let file_name = path.rsplit(['/', '\\']).next().unwrap_or(&path).to_string();
                                        let is_active = active_idx == Some(idx);
                                        let path_clone = path.clone();
                                        view! {
                                            <div
                                                class="explorer-tree-item explorer-custom-file"
                                                class:active=is_active
                                                on:click=move |_| {
                                                    state.select_file(idx);
                                                    on_load_dataset.run(path_clone.clone());
                                                }
                                            >
                                                <span class="tree-leaf-spacer"></span>
                                                <span class="tree-item-icon">"📊"</span>
                                                <span class="tree-item-title" title=path.clone()>{file_name}</span>
                                                <button
                                                    type="button"
                                                    class="tree-item-remove-btn"
                                                    title="Xóa tệp khỏi danh sách"
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        state.remove_file(idx);
                                                    }
                                                >
                                                    "✕"
                                                </button>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            // SECTION 2: Mẫu & Công thức (Saved Recipes & Batch Templates)
            <div class="explorer-section explorer-recipes-section">
                <div class="explorer-section-header">
                    <div class="explorer-title-wrap">
                        <span class="explorer-section-icon">"📌"</span>
                        <h2 class="explorer-section-heading">
                            "MẪU & CÔNG THỨC"
                        </h2>
                    </div>
                </div>

                <div class="explorer-recipes-list">
                    <button
                        type="button"
                        class="recipe-template-card"
                        on:click=move |_| on_apply_recipe_template.run("reconcile_tt78".to_string())
                    >
                        <div class="recipe-card-title">
                            "📌 Đối soát TT78 (Thuế & Hóa đơn)"
                        </div>
                        <div class="recipe-card-desc">
                            "So khớp MST, số hóa đơn, tổng tiền giữa sổ cái và XML"
                        </div>
                    </button>

                    <button
                        type="button"
                        class="recipe-template-card"
                        on:click=move |_| on_apply_recipe_template.run("clean_hr_names_cccd".to_string())
                    >
                        <div class="recipe-card-title">
                            "📌 Làm sạch Nhân sự (Tách tên + CCCD)"
                        </div>
                        <div class="recipe-card-desc">
                            "Tách Họ - Đệm - Tên và kiểm định CCCD 12 số hợp chuẩn"
                        </div>
                    </button>

                    <button
                        type="button"
                        class="recipe-template-card"
                        on:click=move |_| on_apply_recipe_template.run("reconcile_bank_statement".to_string())
                    >
                        <div class="recipe-card-title">
                            "📌 Đối chiếu Sao kê Ngân hàng"
                        </div>
                        <div class="recipe-card-desc">
                            "Khớp dòng giao dịch, số dư đầu kỳ/cuối kỳ và tiền lệch"
                        </div>
                    </button>

                    <button
                        type="button"
                        class="recipe-template-card"
                        on:click=move |_| on_apply_recipe_template.run("fix_all_tcvn3_fonts".to_string())
                    >
                        <div class="recipe-card-title">
                            "📌 Khôi phục Font TCVN3 Toàn bộ"
                        </div>
                        <div class="recipe-card-desc">
                            "Chuyển mã TCVN3/VNI toàn bộ sheet sang Unicode dựng sẵn"
                        </div>
                    </button>

                    <button
                        type="button"
                        class="recipe-template-card"
                        on:click=move |_| on_apply_recipe_template.run("split_admin_address_3_levels".to_string())
                    >
                        <div class="recipe-card-title">
                            "📌 Bóc tách Địa giới Hành chính 3 cấp"
                        </div>
                        <div class="recipe-card-desc">
                            "Tự động tách Tỉnh/TP, Quận/Huyện, Xã/Phường từ địa chỉ thô"
                        </div>
                    </button>
                </div>
            </div>
        </aside>
    }
}
