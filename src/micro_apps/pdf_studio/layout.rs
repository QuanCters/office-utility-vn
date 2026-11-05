use leptos::prelude::*;
use crate::launcher::CommandPalette;
use crate::micro_apps::pdf_studio::canvas_frame::PdfCraftCanvasFrame;
use crate::micro_apps::pdf_studio::file_list_panel::PdfStudioFileListPanel;
use crate::micro_apps::pdf_studio::state::{category_for_tool, PdfStudioState, PdfViewMode};
use crate::micro_apps::pdf_studio::toolbar::pdf_studio_tool_icon;
use crate::micro_apps::pdf_studio::vn_ribbon::PdfVnExtensionRibbon;
use crate::micro_apps::{micro_app_tools, MicroAppId};
use crate::shared_ui::{BackButton, TaskExecutor, WorkflowRunner, cancel_task_ipc, execute_workflow_ipc};
use crate::tools::{
    MarkdownToStyledPdfView, PdfArchiveLinearizeView, PdfBookletImpositionView, PdfBookmarkTocView, PdfCompressView,
    PdfCropMarginView, PdfFlattenFormsView, PdfFormManagerView, PdfGrayscalePrintView, PdfMergeSplitView,
    PdfMetadataScrubberView, PdfPageNumberHeaderView, PdfPageOrganizerView, PdfPkiSignView, PdfProtectSignView,
    PdfRedactionView, PdfRepairCorruptView, PdfSignaturePadView, PdfTableExtractorView, PdfToImagesPackView,
    PdfToLongImageView, PdfToMarkdownView, PdfVisualDiffView, PdfWatermarkStampView, VietnameseOcrView,
};

/// Bố cục tổng thể của Micro-App PDF Studio theo mô hình Hub/Portal.
///
/// Hỗ trợ 3 chế độ xem chính:
/// 1. `PdfViewMode::Hub`: Trang chủ Portal dạng lưới thẻ (Categorized Card Grid, iLovePDF model).
/// 2. `PdfViewMode::ToolWorkspace`: Không gian làm việc tập trung của từng công cụ với Mini Sidebar (56px).
/// 3. `PdfViewMode::Workflow`: Trình thiết kế chuỗi tự động hóa Macro DAG.
#[component]
pub fn PdfStudioLayout(
    state: PdfStudioState,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    // Cung cấp context cho toàn bộ cây con nếu các view con cần truy cập PdfStudioState
    provide_context(state);

    let palette_open = RwSignal::new(false);

    // Callbacks xử lý thực thi chuỗi quy trình Workflow Macro DAG qua IPC
    let on_run_workflow = Callback::new(move |dag: crate::shared_ui::workflow_runner::WorkflowDag| {
        execute_workflow_ipc(dag, state.loaded_files.get(), state.is_running, state.status_message);
    });

    let on_cancel_workflow = Callback::new(move |_| {
        cancel_task_ipc("");
        state.is_running.set(false);
        state.status_message.set("Đã dừng thực thi chuỗi quy trình.".to_string());
    });

    view! {
        <div class="pdf-studio-app-container">
            {move || match state.view_mode.get() {
                PdfViewMode::VisualWorkspace | PdfViewMode::Hub => view! {
                    // =========================================================
                    // CHẾ ĐỘ 1: PDFCRAFT CANVAS WORKSPACE (Toàn màn hình)
                    // =========================================================
                    <header class="pdf-studio-header">
                        <div class="pdf-studio-header-left">
                            <BackButton on_back=on_back label="← Về Launcher" />
                            <div class="pdf-studio-header-title-group">
                                <h1 class="pdf-studio-header-title">
                                    "📄 PDF Studio — Xưởng Đồ Họa & Tài Liệu PDF"
                                </h1>
                                <span class="pdf-studio-header-tagline">
                                    "Biên tập trực quan PDFCraft • Ghép tách • Nén • Bảo mật • OCR 100% On-Device"
                                </span>
                            </div>
                        </div>

                        <div class="pdf-studio-header-right">
                            <span class="pdf-studio-header-badge">
                                "PDFCraft Studio"
                            </span>
                            <span class="pdf-studio-privacy-badge">
                                "🔒 100% On-Device"
                            </span>
                        </div>
                    </header>

                    <main class="pdf-studio-canvas-main">
                        <PdfVnExtensionRibbon state=state />
                        <PdfCraftCanvasFrame state=state />
                    </main>
                }.into_any(),

                PdfViewMode::Workflow => view! {
                    // =========================================================
                    // CHẾ ĐỘ 2: WORKFLOW MACRO DAG (Toàn màn hình)
                    // =========================================================
                    <header class="pdf-studio-header">
                        <div class="pdf-studio-header-left">
                            <button
                                type="button"
                                class="pdf-breadcrumb-btn"
                                title="Quay lại Trang chủ Hub công cụ PDF"
                                on:click=move |_| state.open_hub()
                            >
                                <span class="breadcrumb-arrow" aria-hidden="true">"←"</span>
                                <span>"Tất cả công cụ (PDF Hub)"</span>
                            </button>
                            <div class="pdf-studio-header-title-group">
                                <h1 class="pdf-studio-header-title">
                                    "⚡ Chuỗi Quy Trình Tự Động Hóa PDF"
                                </h1>
                                <span class="pdf-studio-header-tagline">
                                    "Kết nối liên hoàn các công cụ PDF thành quy trình tự động on-device"
                                </span>
                            </div>
                        </div>

                        <div class="pdf-studio-header-right">
                            <BackButton on_back=on_back label="Về Launcher" />
                            <span class="pdf-studio-privacy-badge">
                                "🔒 100% On-Device"
                            </span>
                        </div>
                    </header>

                    <div class="pdf-studio-workspace-body">
                        <PdfStudioFileListPanel state=state />
                        <main class="pdf-studio-center-workspace">
                            <div class="pdf-studio-workflow-wrapper">
                                <WorkflowRunner
                                    available_tools=micro_app_tools(MicroAppId::PdfStudio)
                                    title="⚡ Chuỗi Quy trình Xử lý PDF Studio"
                                    is_running=Signal::derive(move || state.is_running.get())
                                    status_message=Signal::derive(move || state.status_message.get())
                                    on_run=on_run_workflow
                                    on_cancel=on_cancel_workflow
                                />
                            </div>
                        </main>
                    </div>
                }.into_any(),

                PdfViewMode::ToolWorkspace => {
                    let active_slug = state.active_tool.get();
                    let current_tool = state.find_tool(&active_slug);
                    let tool_name = current_tool.map(|t| t.name).unwrap_or("Công cụ PDF");
                    let tool_desc = current_tool.map(|t| t.description).unwrap_or("");
                    let icon = pdf_studio_tool_icon(&active_slug);
                    let category_label = category_for_tool(&active_slug)
                        .map(|c| c.label())
                        .unwrap_or("Tài liệu PDF");

                    view! {
                        // =========================================================
                        // CHẾ ĐỘ 3: FOCUSED TOOL WORKSPACE (Mini Sidebar + Tool View)
                        // =========================================================
                        <header class="pdf-studio-header pdf-studio-focused-header">
                            <div class="pdf-studio-header-left">
                                <button
                                    type="button"
                                    class="pdf-breadcrumb-btn"
                                    title="Quay lại Trang chủ Hub công cụ PDF"
                                    on:click=move |_| state.open_hub()
                                >
                                    <span class="breadcrumb-arrow" aria-hidden="true">"←"</span>
                                    <span>"Tất cả công cụ (PDF Hub)"</span>
                                </button>

                                <div class="pdf-focused-tool-indicator">
                                    <span class="focused-tool-icon" aria-hidden="true">{icon}</span>
                                    <div class="focused-tool-text">
                                        <h2 class="pdf-focused-tool-title">{tool_name}</h2>
                                        <span class="pdf-focused-tool-desc">{tool_desc}</span>
                                    </div>
                                </div>
                            </div>

                            <div class="pdf-studio-header-right">
                                <span class="pdf-studio-cat-badge">{category_label}</span>
                                <span class="pdf-studio-slug-badge">{active_slug.clone()}</span>
                                <span class="pdf-studio-privacy-badge">"🔒 100% On-Device"</span>
                                <BackButton on_back=on_back label="Về Launcher" />
                            </div>
                        </header>

                        <div class="pdf-studio-workspace-body">
                            <PdfStudioFileListPanel state=state />

                            <main class="pdf-studio-center-workspace">
                                <div class="pdf-studio-tool-workspace">
                                    // Context Banner hiển thị tệp PDF đang mở dùng chung
                                    <div class="pdf-studio-context-banner">
                                        {move || {
                                            if let Some(active_name) = state.active_file_name() {
                                                let total = state.loaded_files_count();
                                                let current_idx = state.active_file_index.get().map(|i| i + 1).unwrap_or(0);
                                                let bound_path = state.active_file_path().unwrap_or_default();
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
                                                            <div class="context-bound-path">
                                                                <span>"🔗 Đường dẫn liên kết:"</span>
                                                                <code class="context-bound-code">
                                                                    {bound_path}
                                                                </code>
                                                            </div>
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <div class="context-banner-hint">
                                                        <span class="context-icon" aria-hidden="true">"💡"</span>
                                                        <span class="context-hint-text">
                                                            "Mẹo: Thêm tệp PDF ở thanh bên trái để sẵn sàng xử lý và áp dụng tự động cho công cụ này."
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

                                            "pdf-page-organizer" => view! {
                                                <PdfPageOrganizerView />
                                            }.into_any(),

                                            "pdf-signature-pad" => view! {
                                                <PdfSignaturePadView />
                                            }.into_any(),

                                            "pdf-to-images-pack" => view! {
                                                <PdfToImagesPackView />
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

                                            "pdf-form-manager" => view! {
                                                <PdfFormManagerView />
                                            }.into_any(),

                                            "pdf-redaction" => view! {
                                                <PdfRedactionView />
                                            }.into_any(),

                                            "pdf-archive-linearize" => view! {
                                                <PdfArchiveLinearizeView />
                                            }.into_any(),

                                            "pdf-pki-sign" => view! {
                                                <PdfPkiSignView />
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

                                    // Thanh thực thi tác vụ PDF On-Device
                                    <TaskExecutor
                                        tool_slug=Signal::derive(move || state.active_tool.get())
                                        input_files=Signal::derive(move || {
                                            let mut files = state.loaded_files.get();
                                            if let Some(pos) = state.active_file_path().and_then(|p| files.iter().position(|f| f == &p)) {
                                                let active = files.remove(pos);
                                                files.insert(0, active);
                                            }
                                            files
                                        })
                                        external_is_running=state.is_running
                                    />
                                </div>
                            </main>
                        </div>
                    }.into_any()
                }
            }}

            // Scoped Command Palette (Ctrl+K) trong PDF Studio
            <CommandPalette
                open=palette_open
                tools=state.tools()
                on_deep_link=Callback::new(move |(_, slug): (_, String)| state.open_tool(&slug))
                on_back=on_back
            />
        </div>
    }
}
