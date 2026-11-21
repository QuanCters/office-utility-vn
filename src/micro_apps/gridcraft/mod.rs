pub mod state;
pub mod workspace;

pub use state::GridCraftState;
pub use workspace::{GridCraftWorkspace, GRIDCRAFT_IFRAME_ID, GRIDCRAFT_IFRAME_SRC};

use leptos::prelude::*;

/// Cổng kiểm tra tính khả dụng của GridCraft trong môi trường ứng dụng.
/// [Stage 4C-1 Product Cutover]: GridCraft trở thành ứng dụng bảng tính mặc định toàn hệ thống.
pub const fn is_gridcraft_workspace_enabled() -> bool {
    true
}

/// Giữ bí danh tương thích cho môi trường dev và kiểm thử tự động
pub const fn is_gridcraft_dev_workspace_enabled() -> bool {
    true
}

/// Component chính `GridCraftApp`:
/// Ứng dụng Bảng tính GridCraft hoàn chỉnh chiếm trọn không gian bên dưới Desktop Shell.
/// Có thanh điều hướng tiêu đề tối giản (Quay lại Launcher, Tiêu đề Tệp, Đánh dấu Dirty)
/// và hộp thoại bảo vệ thay đổi chưa lưu (Unsaved-change Protection).
#[component]
pub fn GridCraftApp(
    #[prop(into, optional)] state: Option<GridCraftState>,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    let app_state = state.unwrap_or_default();

    // Xử lý khi người dùng bấm nút Quay lại
    let handle_back_click = move |_| {
        if app_state.is_dirty.get() {
            // Hiển thị dialog xác nhận nếu có thay đổi chưa lưu
            app_state.is_exit_confirm_open.set(true);
        } else {
            on_back.run(());
        }
    };

    view! {
        <div class="gridcraft-app-container w-full h-full flex flex-col overflow-hidden" style="width: 100%; height: 100%; min-height: 0; display: flex; flex-direction: column;">
            // 1. THANH ĐIỀU HƯỚNG TỐI GIẢN CỦA DESKTOP SHELL
            <header
                class="gridcraft-top-bar"
                style="background: #09090B; border-bottom: 1px solid #27272A; padding: 6px 16px; display: flex; align-items: center; justify-content: space-between; z-index: 10;"
            >
                <div style="display: flex; align-items: center; gap: 12px;">
                    <button
                        type="button"
                        class="btn-back-launcher"
                        style="background: #27272A; color: #FAFAFA; border: 1px solid #3F3F46; border-radius: 4px; padding: 4px 10px; font-size: 12px; font-weight: 500; cursor: pointer; display: flex; align-items: center; gap: 6px;"
                        on:click=handle_back_click
                        title="Quay lại Màn hình Chính"
                    >
                        <span>"←"</span>
                        <span>"Trang chủ"</span>
                    </button>

                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span style="font-size: 15px;">"📊"</span>
                        <span style="font-size: 13px; font-weight: 600; color: #FFFFFF;">
                            {move || app_state.active_file_name.get().unwrap_or_else(|| "GridCraft Spreadsheet".to_string())}
                        </span>
                        {move || {
                            if app_state.is_dirty.get() {
                                view! {
                                    <span style="font-size: 12px; background: rgba(245, 158, 11, 0.2); color: #FCD34D; border: 1px solid #F59E0B; border-radius: 4px; padding: 1px 6px; font-weight: 500;">
                                        "● Chưa lưu"
                                    </span>
                                }.into_any()
                            } else {
                                view! {
                                    <span style="font-size: 12px; color: #71717A;">
                                        "✓ Đã lưu"
                                    </span>
                                }.into_any()
                            }
                        }}
                    </div>
                </div>

                <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-size: 12px; background: rgba(16, 185, 129, 0.15); color: #34D399; border: 1px solid #059669; border-radius: 4px; padding: 2px 8px; font-weight: 500;">
                        "GridCraft v0.19.1 Gốc"
                    </span>
                    <span style="font-size: 12px; background: rgba(239, 68, 68, 0.2); color: #FCA5A5; border: 1px solid #DC2626; border-radius: 4px; padding: 2px 8px; font-weight: 500;">
                        "Stage 4B-2B Dev Workspace"
                    </span>
                </div>
            </header>

            // 2. KHÔNG GIAN LÀM VIỆC CHÍNH (FULL-WORKSPACE)
            <main class="gridcraft-main-area w-full h-full flex-1 relative overflow-hidden" style="width: 100%; height: 100%; flex: 1; display: flex; min-height: 0; position: relative;">
                <GridCraftWorkspace state=app_state />
            </main>

            // 3. HỘP THOẠI XÁC NHẬN KHI RỜI ĐI CÓ THAY ĐỔI CHƯA LƯU (Unsaved-change Confirmation Dialog)
            {move || {
                if app_state.is_exit_confirm_open.get() {
                    view! {
                        <div
                            class="modal-backdrop"
                            style="position: fixed; inset: 0; background: rgba(0, 0, 0, 0.75); display: flex; align-items: center; justify-content: center; z-index: 50;"
                        >
                            <div
                                class="modal-dialog"
                                style="background: #18181B; border: 1px solid #3F3F46; border-radius: 8px; padding: 24px; max-width: 440px; width: 100%; color: #FAFAFA; box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);"
                            >
                                <div style="font-size: 16px; font-weight: 600; margin-bottom: 8px; display: flex; align-items: center; gap: 8px;">
                                    <span>"⚠️"</span>
                                    <span>"Thay đổi Chưa được Lưu"</span>
                                </div>
                                <p style="font-size: 13px; color: #A1A1AA; line-height: 1.5; margin-bottom: 20px;">
                                    "Bảng tính của bạn có những chỉnh sửa chưa được lưu. Trong Stage 4A, tính năng lưu tệp Production đang bị khóa an toàn. Nếu thoát bây giờ, các thay đổi chưa lưu sẽ bị hủy."
                                </p>
                                <div style="display: flex; justify-content: flex-end; gap: 10px;">
                                    <button
                                        type="button"
                                        style="background: #27272A; color: #FAFAFA; border: 1px solid #3F3F46; border-radius: 6px; padding: 6px 14px; font-size: 13px; cursor: pointer;"
                                        on:click=move |_| app_state.is_exit_confirm_open.set(false)
                                    >
                                        "Ở lại Bảng tính"
                                    </button>
                                    <button
                                        type="button"
                                        style="background: #DC2626; color: #FFFFFF; border: none; border-radius: 6px; padding: 6px 14px; font-size: 13px; font-weight: 500; cursor: pointer;"
                                        on:click=move |_| {
                                            app_state.is_exit_confirm_open.set(false);
                                            on_back.run(());
                                        }
                                    >
                                        "Hủy Thay đổi & Thoát"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }
            }}
        </div>
    }
}
