use leptos::prelude::*;
use crate::micro_apps::data_lab::file_list_panel::DataLabFileListPanel;
use crate::micro_apps::data_lab::state::{category_for_tool, DataLabState};
use crate::micro_apps::data_lab::toolbar::{data_lab_tool_icon, DataLabToolbar};
use crate::micro_apps::DATA_LAB_TOOLS;
use crate::shared_ui::{BackButton, WorkflowRunner};
use crate::tools::{
    Base64FileCodecView, DataStructureConverterView, DiffCheckerTextSheetsView,
    DocxTemplateMailmergeView, ExcelCleanerNormalizerView, ExcelMultiMergerView,
    ExcelSheetUnlockerView, ExcelSplitByColumnView, ExcelVisualJoinerView,
    ListSetOperationsView, SqlFormatterLinterView, TextPiiAnonymizerView,
    UniversalFileConverterHubView,
};

/// Bố cục tổng thể của Micro-App Data Lab (Header + Tab Toolbar + File List Panel + Workspace).
#[component]
pub fn DataLabLayout(
    state: DataLabState,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    // Cung cấp context cho toàn bộ cây con nếu các view cần truy cập DataLabState
    provide_context(state);

    // Callbacks xử lý thực thi chuỗi quy trình Workflow Macro DAG
    let on_run_workflow = Callback::new(move |dag: crate::shared_ui::workflow_runner::WorkflowDag| {
        state.is_running.set(true);
        state.status_message.set(format!(
            "Đang thực thi chuỗi dữ liệu '{}' gồm {} bước...",
            dag.title,
            dag.steps.len()
        ));
    });

    let on_cancel_workflow = Callback::new(move |_| {
        state.is_running.set(false);
        state.status_message.set("Đã dừng thực thi chuỗi quy trình dữ liệu.".to_string());
    });

    view! {
        <div class="data-lab-app-container">
            // Header trên cùng với nút BackButton chuẩn mực và tiêu đề
            <header class="data-lab-header">
                <div class="data-lab-header-left">
                    <BackButton on_back=on_back label="← Về Launcher" />
                    <div class="data-lab-header-title-group">
                        <h1 class="data-lab-header-title">
                            "📊 Data Lab — Phòng thí nghiệm Dữ liệu"
                        </h1>
                        <span class="data-lab-header-tagline">
                            "Bảng tính Excel • Tách & Gộp • Đối soát Diff • Ẩn danh PII • Chuyển đổi & Trộn tệp 100% On-Device"
                        </span>
                    </div>
                </div>

                <div class="data-lab-header-right">
                    <span class="data-lab-header-badge">
                        "13 Công cụ Dữ liệu"
                    </span>
                    <span class="data-lab-privacy-badge">
                        "🔒 100% On-Device"
                    </span>
                </div>
            </header>

            // Thanh công cụ Tab Toolbar phân loại theo 3 nhóm nghiệp vụ + 1 tab Workflow
            <DataLabToolbar state=state />

            // Khu vực làm việc chính (Bố cục 2 cột: Left Panel quản lý file dữ liệu + Center Workspace)
            <div class="data-lab-workspace-body">
                // Cột 1: File List Panel quản lý các file bảng tính / dữ liệu (Shared Context)
                <DataLabFileListPanel state=state />

                // Cột 2: Workspace hiển thị công cụ đang chọn hoặc Workflow Runner
                <main class="data-lab-center-workspace">
                    {move || {
                        if state.is_workflow() {
                            // Chế độ chuỗi Workflow Macro DAG
                            view! {
                                <div class="data-lab-workflow-wrapper">
                                    <WorkflowRunner
                                        available_tools=DATA_LAB_TOOLS
                                        title="⚡ Chuỗi Quy trình Xử lý Data Lab"
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
                            let tool_name = current_tool.map(|t| t.name).unwrap_or("Công cụ Dữ liệu");
                            let tool_desc = current_tool.map(|t| t.description).unwrap_or("");
                            let icon = data_lab_tool_icon(&active_slug);
                            let category_label = category_for_tool(&active_slug)
                                .map(|c| c.label())
                                .unwrap_or("Dữ liệu & Bảng tính");

                            view! {
                                <div class="data-lab-tool-workspace">
                                    // Header của thẻ công cụ
                                    <div class="data-lab-tool-card-header">
                                        <div class="data-lab-tool-title-row">
                                            <span class="data-lab-tool-icon" aria-hidden="true">{icon}</span>
                                            <div>
                                                <h2 class="data-lab-tool-name">{tool_name}</h2>
                                                <p class="data-lab-tool-description">{tool_desc}</p>
                                            </div>
                                        </div>
                                        <div class="data-lab-tool-meta">
                                            <span class="data-lab-cat-badge">{category_label}</span>
                                            <span class="data-lab-slug-badge">{active_slug.clone()}</span>
                                            <span class="data-lab-privacy-chip">"🔒 On-Device"</span>
                                        </div>
                                    </div>

                                    // Context Banner hiển thị tệp dữ liệu đang mở dùng chung
                                    <div class="data-lab-context-banner">
                                        {move || {
                                            if let Some(active_name) = state.active_file_name() {
                                                let total = state.loaded_files_count();
                                                let current_idx = state.active_file_index.get().map(|i| i + 1).unwrap_or(0);
                                                let ext = state.active_file_extension().unwrap_or_default();
                                                let is_table = matches!(ext.as_str(), "xlsx" | "xls" | "csv" | "tsv");
                                                let sheet_info = if is_table {
                                                    format!(" • Trang tính: {}", state.active_sheet.get())
                                                } else {
                                                    String::new()
                                                };

                                                view! {
                                                    <div class="context-banner-active">
                                                        <span class="context-icon" aria-hidden="true">"📁"</span>
                                                        <div class="context-text-group">
                                                            <span class="context-title">
                                                                "Đang áp dụng cho tệp dữ liệu: "
                                                                <strong>{active_name}</strong>
                                                                <span class="context-sheet-tag">{sheet_info}</span>
                                                            </span>
                                                            <span class="context-subtitle">
                                                                {format!("(Tệp {}/{} trong danh sách dùng chung)", current_idx, total)}
                                                            </span>
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <div class="context-banner-hint">
                                                        <span class="context-icon" aria-hidden="true">"💡"</span>
                                                        <span class="context-hint-text">
                                                            "Mẹo: Nạp tệp bảng tính (.xlsx, .csv) hoặc dữ liệu ở danh sách bên trái để phân tích và áp dụng trực tiếp cho công cụ này."
                                                        </span>
                                                    </div>
                                                }.into_any()
                                            }
                                        }}
                                    </div>

                                    // Khối hiển thị View của từng công cụ
                                    <div class="data-lab-tool-view-body">
                                        {match active_slug.as_str() {
                                            "excel-cleaner-normalizer" => view! {
                                                <ExcelCleanerNormalizerView />
                                            }.into_any(),

                                            "excel-split-by-column" => view! {
                                                <ExcelSplitByColumnView />
                                            }.into_any(),

                                            "excel-multi-merger" => view! {
                                                <ExcelMultiMergerView />
                                            }.into_any(),

                                            "excel-visual-joiner" => view! {
                                                <ExcelVisualJoinerView />
                                            }.into_any(),

                                            "excel-sheet-unlocker" => view! {
                                                <ExcelSheetUnlockerView />
                                            }.into_any(),

                                            "data-structure-converter" => view! {
                                                <DataStructureConverterView />
                                            }.into_any(),

                                            "docx-template-mailmerge" => view! {
                                                <DocxTemplateMailmergeView />
                                            }.into_any(),

                                            "universal-file-converter-hub" | "universal-converter" => view! {
                                                <UniversalFileConverterHubView />
                                            }.into_any(),

                                            "base64-file-codec" => view! {
                                                <Base64FileCodecView />
                                            }.into_any(),

                                            "diff-checker-text-sheets" => view! {
                                                <DiffCheckerTextSheetsView />
                                            }.into_any(),

                                            "list-set-operations" => view! {
                                                <ListSetOperationsView />
                                            }.into_any(),

                                            "text-pii-anonymizer" => view! {
                                                <TextPiiAnonymizerView />
                                            }.into_any(),

                                            "sql-formatter-linter" => view! {
                                                <SqlFormatterLinterView />
                                            }.into_any(),

                                            _ => view! {
                                                <div class="data-lab-not-found">
                                                    "Công cụ Data Lab không tồn tại."
                                                </div>
                                            }.into_any(),
                                        }}
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }}
                </main>
            </div>
        </div>
    }
}
