use leptos::prelude::*;
use crate::micro_apps::media_studio::batch_queue::MediaStudioBatchQueue;
use crate::micro_apps::media_studio::gallery_panel::MediaStudioGalleryPanel;
use crate::micro_apps::media_studio::state::{category_for_tool, media_studio_tool_icon, MediaStudioState};
use crate::micro_apps::media_studio::toolbar::MediaStudioToolbar;
use crate::micro_apps::MEDIA_STUDIO_TOOLS;
use crate::shared_ui::{BackButton, WorkflowRunner};
use crate::tools::{
    AudioSilenceTrimmerView, DocScannerPerspectiveView, ImageBatchView, ImageBgRedactView,
    ImageExifInspectorView, PhotoIdCropView, ScreenshotBeautifierView, VideoToGifWebpView,
    WatermarkBatchStudioView,
};

/// Bố cục tổng thể của Micro-App Media Studio (Header + Toolbar + Gallery Panel + Main Workspace + Batch Queue).
#[component]
pub fn MediaStudioLayout(
    state: MediaStudioState,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    // Cung cấp context cho toàn bộ cây con nếu các view cần truy cập MediaStudioState
    provide_context(state);

    // Callbacks xử lý thực thi chuỗi quy trình Workflow Macro DAG
    let on_run_workflow = Callback::new(move |dag: crate::shared_ui::workflow_runner::WorkflowDag| {
        state.is_running.set(true);
        state.status_message.set(format!(
            "Đang thực thi chuỗi đa phương tiện '{}' gồm {} bước...",
            dag.title,
            dag.steps.len()
        ));
    });

    let on_cancel_workflow = Callback::new(move |_| {
        state.is_running.set(false);
        state.status_message.set("Đã dừng thực thi chuỗi quy trình đa phương tiện.".to_string());
    });

    // Callback xử lý khi nhấn nút Đưa vào Batch Queue
    let on_enqueue_current = move |_| {
        if let Some(active_name) = state.active_file_name() {
            state.enqueue_batch_item(active_name);
        } else {
            let active_slug = state.active_tool.get();
            state.enqueue_batch_item(format!("task_{}.media", active_slug));
        }
    };

    view! {
        <div class="media-studio-app-container">
            // Header trên cùng với nút BackButton chuẩn mực và tiêu đề
            <header class="media-studio-header">
                <div class="media-studio-header-left">
                    <BackButton on_back=on_back label="← Về Launcher" />
                    <div class="media-studio-header-title-group">
                        <h1 class="media-studio-header-title">
                            "🎨 Media Studio — Phòng thu Đa phương tiện"
                        </h1>
                        <span class="media-studio-header-tagline">
                            "Ảnh thẻ • Tối ưu WebP/AVIF • Che mờ dữ liệu • Xóa Exif • Bản quyền • Nắn scan • Audio & Video 100% On-Device"
                        </span>
                    </div>
                </div>

                <div class="media-studio-header-right">
                    <span class="media-studio-header-badge">
                        "9 Công cụ Media"
                    </span>
                    <span class="media-studio-privacy-badge">
                        "🔒 100% On-Device"
                    </span>
                </div>
            </header>

            // Thanh công cụ Tab Toolbar phân loại theo 2 nhóm nghiệp vụ + 1 tab Workflow
            <MediaStudioToolbar state=state />

            // Khu vực làm việc chính (Bố cục 2 cột: Left Panel thư viện media + Center Workspace)
            <div class="media-studio-workspace-body">
                // Cột 1: Gallery Panel quản lý các file media nạp vào (Shared Context)
                <MediaStudioGalleryPanel state=state />

                // Cột 2: Workspace hiển thị công cụ đang chọn hoặc Workflow Runner
                <main class="media-studio-center-workspace">
                    {move || {
                        if state.is_workflow() {
                            // Chế độ chuỗi Workflow Macro DAG
                            view! {
                                <div class="media-studio-workflow-wrapper">
                                    <WorkflowRunner
                                        available_tools=MEDIA_STUDIO_TOOLS
                                        title="⚡ Chuỗi Quy trình Xử lý Media Studio"
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
                            let tool_name = current_tool.map(|t| t.name).unwrap_or("Công cụ Media");
                            let tool_desc = current_tool.map(|t| t.description).unwrap_or("");
                            let icon = media_studio_tool_icon(&active_slug);
                            let category_label = category_for_tool(&active_slug)
                                .map(|c| c.label())
                                .unwrap_or("Đa phương tiện");

                            view! {
                                <div class="media-studio-tool-workspace">
                                    // Header của thẻ công cụ
                                    <div class="media-studio-tool-card-header">
                                        <div class="media-studio-tool-title-row">
                                            <span class="media-studio-tool-icon" aria-hidden="true">{icon}</span>
                                            <div>
                                                <h2 class="media-studio-tool-name">{tool_name}</h2>
                                                <p class="media-studio-tool-description">{tool_desc}</p>
                                            </div>
                                        </div>
                                        <div class="media-studio-tool-meta">
                                            <span class="media-studio-cat-badge">{category_label}</span>
                                            <span class="media-studio-slug-badge">{active_slug.clone()}</span>
                                            <span class="media-studio-privacy-chip">"🔒 On-Device"</span>
                                        </div>
                                    </div>

                                    // Context Banner hiển thị tệp Media nguồn chung đang áp dụng
                                    <div class="media-studio-context-banner">
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
                                                                {format!("(Tệp {}/{} trong thư viện media)", current_idx, total)}
                                                            </span>
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <div class="context-banner-hint">
                                                        <span class="context-icon" aria-hidden="true">"💡"</span>
                                                        <span class="context-hint-text">
                                                            "Mẹo: Thêm tệp ảnh, video hoặc âm thanh ở thư viện bên trái để sẵn sàng xử lý và áp dụng tự động cho công cụ này."
                                                        </span>
                                                    </div>
                                                }.into_any()
                                            }
                                        }}
                                    </div>

                                    // Khối hiển thị View của từng công cụ
                                    <div class="media-studio-tool-view-body">
                                        {match active_slug.as_str() {
                                            "image-batch" => view! {
                                                <ImageBatchView
                                                    target_format=state.img_batch_target_format
                                                    quality=state.img_batch_quality
                                                    max_width=state.img_batch_max_width
                                                    strip_exif=state.img_batch_strip_exif
                                                />
                                            }.into_any(),

                                            "photo-id-crop" => view! {
                                                <PhotoIdCropView
                                                    id_size=state.photo_id_size
                                                    sheet_layout=state.photo_id_sheet_layout
                                                />
                                            }.into_any(),

                                            "image-bg-redact" => view! {
                                                <ImageBgRedactView
                                                    redact_type=state.redact_type
                                                    auto_detect=state.redact_auto_detect
                                                />
                                            }.into_any(),

                                            "image-exif-inspector" => view! {
                                                <ImageExifInspectorView
                                                    strip_all=state.exif_strip_all
                                                />
                                            }.into_any(),

                                            "watermark-batch-studio" => view! {
                                                <WatermarkBatchStudioView
                                                    watermark_type=state.watermark_type
                                                    opacity=state.watermark_opacity
                                                    text=state.watermark_text
                                                />
                                            }.into_any(),

                                            "screenshot-beautifier" => view! {
                                                <ScreenshotBeautifierView
                                                    theme=state.beautifier_theme
                                                    background=state.beautifier_background
                                                    shadow_blur=state.beautifier_shadow_blur
                                                />
                                            }.into_any(),

                                            "doc-scanner-perspective" => view! {
                                                <DocScannerPerspectiveView
                                                    auto_enhance=state.scanner_auto_enhance
                                                />
                                            }.into_any(),

                                            "audio-silence-trimmer" => view! {
                                                <AudioSilenceTrimmerView
                                                    silence_threshold_db=state.audio_silence_threshold_db
                                                    min_silence_duration_ms=state.audio_min_silence_duration_ms
                                                />
                                            }.into_any(),

                                            "video-to-gif-webp" => view! {
                                                <VideoToGifWebpView
                                                    fps=state.video_fps
                                                    max_width=state.video_max_width
                                                    output_format=state.video_output_format
                                                />
                                            }.into_any(),

                                            _ => view! {
                                                <div class="media-studio-not-found">
                                                    "Công cụ Media Studio không tồn tại."
                                                </div>
                                            }.into_any(),
                                        }}

                                        // Hàng nút thao tác nhanh bên dưới view cấu hình
                                        <div class="media-studio-action-row">
                                            <button
                                                type="button"
                                                class="media-studio-btn-primary"
                                                on:click=on_enqueue_current
                                                title="Đưa tác vụ này vào hàng đợi xử lý hàng loạt"
                                            >
                                                <span>"⚡ Đưa vào Batch Queue"</span>
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }}
                </main>
            </div>

            // Khu vực đáy: Collapsible Batch Processing Queue
            <MediaStudioBatchQueue state=state />
        </div>
    }
}
