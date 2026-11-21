pub mod canvas_frame;
pub mod command_bar;
pub mod drawer;
pub mod explorer;
pub mod file_list_panel;
pub mod inspector;
pub mod layout;
pub mod reconcile_workbench;
pub mod split_pane_view;
pub mod spreadsheet_grid_view;
pub mod state;
pub mod toolbar;
pub mod visual_canvas_workflow;

pub use canvas_frame::GridCraftCanvasFrame;
pub use command_bar::{DataLabCommandBar, DataLabWorkspaceMode};
pub use drawer::{DataLabDrawer, DrawerTab};
pub use explorer::DataLabExplorer;
pub use file_list_panel::DataLabFileListPanel;
pub use inspector::{DataLabInspector, RecipeStepDisplay};
pub use layout::DataLabLayout;
pub use reconcile_workbench::{ReconcileDiscrepancyTab, VisualReconcileWorkbench};
pub use split_pane_view::{ConverterMode, SplitPaneConverterView};
pub use spreadsheet_grid_view::{ColumnType, SpreadsheetGridFacetView};
pub use state::{category_for_tool, DataLabCategory, DataLabState};
pub use toolbar::{data_lab_tool_icon, DataLabToolbar};
pub use visual_canvas_workflow::VisualCanvasWorkflowView;

use leptos::prelude::*;

/// Micro-App 4: Data Lab — Phòng thí nghiệm Bảng tính Excel, Đối chiếu Dữ liệu & Trạm Chuyển đổi Tệp Vạn năng.
///
/// Quản lý 13 công cụ dữ liệu:
/// 1. `excel-cleaner-normalizer` (Làm sạch & Chuẩn hóa Bảng tính)
/// 2. `diff-checker-text-sheets` (Đối chiếu Văn bản & Bảng tính)
/// 3. `data-structure-converter` (Chuyển đổi Cấu trúc Dữ liệu)
/// 4. `excel-split-by-column` (Tách File Excel theo Cột)
/// 5. `excel-multi-merger` (Gộp Nhiều File Excel / Sheet)
/// 6. `excel-sheet-unlocker` (Gỡ Khóa Bảo vệ Chỉnh sửa Sheet Excel)
/// 7. `excel-visual-joiner` (Nối Trực quan Bảng tính - Join)
/// 8. `list-set-operations` (Phép toán Tập hợp Danh sách)
/// 9. `text-pii-anonymizer` (Ẩn danh hóa Dữ liệu Nhạy cảm PII)
/// 10. `sql-formatter-linter` (Định dạng & Linter Câu lệnh SQL)
/// 11. `base64-file-codec` (Mã hóa / Giải mã Base64 cho File)
/// 12. `docx-template-mailmerge` (Trộn Hợp đồng Mailmerge Hàng loạt)
/// 13. `universal-file-converter-hub` (Trạm Chuyển đổi Tệp Vạn năng)
#[component]
pub fn DataLabApp(
    #[prop(into, optional)] state: Option<DataLabState>,
    #[prop(into, optional)] initial_tool: Option<String>,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    let app_state = state.unwrap_or_else(|| DataLabState::with_initial_tool(initial_tool.clone()));
    if let Some(ref slug) = initial_tool {
        app_state.select_tool(slug);
    }

    view! {
        <layout::DataLabLayout state=app_state on_back=on_back />
    }
}
