use leptos::prelude::*;
use crate::micro_apps::pdf_studio::file_list_panel::PdfStudioFileListPanel;
use crate::micro_apps::pdf_studio::state::{category_for_tool, PdfStudioState};
use crate::micro_apps::pdf_studio::toolbar::{pdf_studio_tool_icon, PdfStudioToolbar};
use crate::micro_apps::PDF_STUDIO_TOOLS;
use crate::shared_ui::{BackButton, WorkflowRunner};
use crate::tools::{
    MarkdownToStyledPdfView, PdfBookletImpositionView, PdfBookmarkTocView, PdfCompressView,
    PdfCropMarginView, PdfFlattenFormsView, PdfGrayscalePrintView, PdfMergeSplitView,
    PdfMetadataScrubberView, PdfPageNumberHeaderView, PdfProtectSignView, PdfRepairCorruptView,
    PdfTableExtractorView, PdfToLongImageView, PdfToMarkdownView, PdfVisualDiffView,
    PdfWatermarkStampView, VietnameseOcrView,
};

/// Bố cục tổng thể của Micro-App PDF Studio (Header + Tab Toolbar + File List Panel + Workspace).
#[component]
pub fn PdfStudioLayout(
    state: PdfStudioState,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    // Cung cấp context cho toàn bộ cây con nếu các view cần truy cập PdfStudioState
    provide_context(state);

    // Callbacks xử lý thực thi chuỗi quy trình Workflow Macro DAG
    let on_run_workflow = Callback::new(move |dag: crate::shared_ui::workflow_runner::WorkflowDag| {
        state.is_running.set(true);
        state.status_message.set(format!(
            "Đang thực thi chuỗi '{}' gồm {} bước...",
            dag.title,
            dag.steps.len()
        ));
    });

    let on_cancel_workflow = Callback::new(move |_| {
        state.is_running.set(false);
        state.status_message.set("Đã dừng thực thi chuỗi quy trình.".to_string());
    });

    view! {
        <div class="pdf-studio-app-container">
            // Header trên cùng với nút BackButton chuẩn mực và tiêu đề
            <header class="pdf-studio-header">
                <div class="pdf-studio-header-left">
                    <BackButton on_back=on_back label="← Về Launcher" />
                    <div class="pdf-studio-header-title-group">
                        <h1 class="pdf-studio-header-title">
                            "📄 PDF Studio — Xưởng Tài liệu PDF"
                        </h1>
                        <span class="pdf-studio-header-tagline">
                            "Ghép tách • Nén • Bảo mật • Đóng dấu • OCR • Chuyển đổi PDF 100% On-Device"
                        </span>
                    </div>
                </div>

                <div class="pdf-studio-header-right">
                    <span class="pdf-studio-header-badge">
                        "18 Công cụ PDF"
                    </span>
                    <span class="pdf-studio-privacy-badge">
                        "🔒 100% On-Device"
                    </span>
                </div>
            </header>

            // Thanh công cụ Tab Toolbar phân loại theo 4 nhóm nghiệp vụ + 1 tab Workflow
            <PdfStudioToolbar state=state />

            // Khu vực làm việc chính (Bố cục 2 cột: Left Panel quản lý file + Center Workspace)
            <div class="pdf-studio-workspace-body">
                // Cột 1: File List Panel quản lý các file PDF nạp vào (Shared Context)
                <PdfStudioFileListPanel state=state />

                // Cột 2: Workspace hiển thị công cụ đang chọn hoặc Workflow Runner
                <main class="pdf-studio-center-workspace">
                    {move || {
                        if state.is_workflow() {
                            // Chế độ chuỗi Workflow Macro DAG
                            view! {
                                <div class="pdf-studio-workflow-wrapper">
                                    <WorkflowRunner
                                        available_tools=PDF_STUDIO_TOOLS
                                        title="⚡ Chuỗi Quy trình Xử lý PDF Studio"
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
                            let tool_name = current_tool.map(|t| t.name).unwrap_or("Công cụ PDF");
                            let tool_desc = current_tool.map(|t| t.description).unwrap_or("");
                            let icon = pdf_studio_tool_icon(&active_slug);
                            let category_label = category_for_tool(&active_slug)
                                .map(|c| c.label())
                                .unwrap_or("Tài liệu PDF");

                            view! {
                                <div class="pdf-studio-tool-workspace">
                                    // Header của thẻ công cụ
                                    <div class="pdf-studio-tool-card-header">
                                        <div class="pdf-studio-tool-title-row">
                                            <span class="pdf-studio-tool-icon" aria-hidden="true">{icon}</span>
                                            <div>
                                                <h2 class="pdf-studio-tool-name">{tool_name}</h2>
                                                <p class="pdf-studio-tool-description">{tool_desc}</p>
                                            </div>
                                        </div>
                                        <div class="pdf-studio-tool-meta">
                                            <span class="pdf-studio-cat-badge">{category_label}</span>
                                            <span class="pdf-studio-slug-badge">{active_slug.clone()}</span>
                                            <span class="pdf-studio-privacy-chip">"🔒 On-Device"</span>
                                        </div>
                                    </div>

                                    // Context Banner hiển thị tệp PDF đang mở dùng chung
                                    <div class="pdf-studio-context-banner">
                                        {move || {
                                            if let Some(active_name) = state.active_file_name() {
                                                let total = state.loaded_files_count();
                                                let current_idx = state.active_file_index.get().map(|i| i + 1).unwrap_or(0);
                                                view! {
                                                    <div class="context-banner-active">
                                                        <span class="context-icon" aria-hidden="true">"📁"</span>
                                                        <div class="context-text-group">
                                                            <span class="context-title">
                                                                "Đang áp dụng cho tệp: "
                                                                <strong>{active_name}</strong>
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
                                                            "Mẹo: Thêm tệp PDF ở danh sách bên trái để sẵn sàng xử lý và áp dụng tự động cho công cụ này."
                                                        </span>
                                                    </div>
                                                }.into_any()
                                            }
                                        }}
                                    </div>

                                    // Khối hiển thị View của từng công cụ
                                    <div class="pdf-studio-tool-view-body">
                                        {match active_slug.as_str() {
                                            "pdf-merge-split" => view! {
                                                <PdfMergeSplitView />
                                            }.into_any(),

                                            "pdf-crop-margin" => view! {
                                                <PdfCropMarginView />
                                            }.into_any(),

                                            "pdf-booklet-imposition" => view! {
                                                <PdfBookletImpositionView />
                                            }.into_any(),

                                            "pdf-bookmark-toc" => view! {
                                                <PdfBookmarkTocView />
                                            }.into_any(),

                                            "pdf-visual-diff" => view! {
                                                <PdfVisualDiffView />
                                            }.into_any(),

                                            "pdf-protect-sign" => view! {
                                                <PdfProtectSignView />
                                            }.into_any(),

                                            "pdf-compress" => view! {
                                                <PdfCompressView />
                                            }.into_any(),

                                            "pdf-metadata-scrubber" => view! {
                                                <PdfMetadataScrubberView />
                                            }.into_any(),

                                            "pdf-flatten-forms" => view! {
                                                <PdfFlattenFormsView />
                                            }.into_any(),

                                            "pdf-repair-corrupt" => view! {
                                                <PdfRepairCorruptView />
                                            }.into_any(),

                                            "pdf-page-number-header" => view! {
                                                <PdfPageNumberHeaderView />
                                            }.into_any(),

                                            "pdf-watermark-stamp" => view! {
                                                <PdfWatermarkStampView />
                                            }.into_any(),

                                            "pdf-grayscale-print" => view! {
                                                <PdfGrayscalePrintView />
                                            }.into_any(),

                                            "vietnamese-ocr" => view! {
                                                <VietnameseOcrView />
                                            }.into_any(),

                                            "pdf-to-markdown" => view! {
                                                <PdfToMarkdownView />
                                            }.into_any(),

                                            "pdf-to-long-image" => view! {
                                                <PdfToLongImageView />
                                            }.into_any(),

                                            "pdf-table-extractor" => view! {
                                                <PdfTableExtractorView />
                                            }.into_any(),

                                            "markdown-to-styled-pdf" => view! {
                                                <MarkdownToStyledPdfView />
                                            }.into_any(),

                                            _ => view! {
                                                <div class="pdf-studio-not-found">
                                                    "Công cụ PDF không tồn tại."
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
