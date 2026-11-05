use leptos::prelude::*;
use crate::launcher::CommandPalette;
use crate::micro_apps::media_studio::batch_queue::MediaStudioBatchQueue;
use crate::micro_apps::media_studio::gallery_panel::MediaStudioGalleryPanel;
use crate::micro_apps::media_studio::hub_view::MediaStudioHubView;
use crate::micro_apps::media_studio::state::{
    category_for_tool, media_studio_tool_icon, MediaStudioState, MediaViewMode,
};
use crate::micro_apps::{micro_app_tools, MicroAppId};
use crate::shared_ui::{BackButton, TaskExecutor, WorkflowRunner, cancel_task_ipc, execute_workflow_ipc};
use crate::tools::{
    AudioSilenceTrimmerView, DocScannerPerspectiveView, ImageBatchView, ImageBgRedactView,
    ImageExifInspectorView, PhotoIdCropView, ScreenshotBeautifierView, VideoToGifWebpView,
    WatermarkBatchStudioView,
};

/// Bố cục tổng thể của Micro-App Media Studio theo mô hình Hub/Portal.
///
/// Hỗ trợ 3 chế độ xem chính:
/// 1. `MediaViewMode::Hub`: Trang chủ Portal dạng lưới thẻ (Categorized Card Grid, iLoveIMG model)
///    với Floating Contextual Widget ở góc dưới phải khi có tiến trình nền.
/// 2. `MediaViewMode::ToolWorkspace`: Không gian làm việc tập trung của từng công cụ với Mini Gallery Sidebar (56px)
///    và Persistent Dock Batch Queue ở đáy.
/// 3. `MediaViewMode::Workflow`: Trình thiết kế chuỗi tự động hóa Macro DAG.
#[component]
pub fn MediaStudioLayout(
    state: MediaStudioState,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    // Cung cấp context cho toàn bộ cây con nếu các view cần truy cập MediaStudioState
    provide_context(state);

    let palette_open = RwSignal::new(false);

    // Callbacks xử lý thực thi chuỗi quy trình Workflow Macro DAG qua IPC
    let on_run_workflow = Callback::new(move |dag: crate::shared_ui::workflow_runner::WorkflowDag| {
        execute_workflow_ipc(dag, state.loaded_files.get(), state.is_running, state.status_message);
    });

    let on_cancel_workflow = Callback::new(move |_| {
        cancel_task_ipc("");
        state.is_running.set(false);
        state.status_message.set("Đã dừng thực thi chuỗi quy trình đa phương tiện.".to_string());
    });

    // Options builder cho từng công cụ Media Studio
    let options_builder = Callback::new(move |_| {
        let slug = state.active_tool.get();
        match slug.as_str() {
            "image-batch" => serde_json::json!({
                "target_format": state.img_batch_target_format.get(),
                "quality": state.img_batch_quality.get(),
                "max_width": state.img_batch_max_width.get(),
                "strip_exif": state.img_batch_strip_exif.get(),
            }),
            "photo-id-crop" => serde_json::json!({
                "id_size": state.photo_id_size.get(),
                "sheet_layout": state.photo_id_sheet_layout.get(),
            }),
            "image-bg-redact" => serde_json::json!({
                "redact_type": state.redact_type.get(),
                "auto_detect": state.redact_auto_detect.get(),
            }),
            "image-exif-inspector" => serde_json::json!({
                "strip_all": state.exif_strip_all.get(),
            }),
            "watermark-batch-studio" => serde_json::json!({
                "watermark_type": state.watermark_type.get(),
                "opacity": state.watermark_opacity.get(),
                "text": state.watermark_text.get(),
            }),
            "screenshot-beautifier" => serde_json::json!({
                "theme": state.beautifier_theme.get(),
                "background": state.beautifier_background.get(),
                "shadow_blur": state.beautifier_shadow_blur.get(),
            }),
            "doc-scanner-perspective" => serde_json::json!({
                "auto_enhance": state.scanner_auto_enhance.get(),
            }),
            "audio-silence-trimmer" => serde_json::json!({
                "silence_threshold_db": state.audio_silence_threshold_db.get(),
                "min_silence_duration_ms": state.audio_min_silence_duration_ms.get(),
                "manual_trim_enabled": state.audio_manual_trim_enabled.get(),
                "start_time_ms": (state.audio_start_time_s.get() * 1000.0) as u32,
                "end_time_ms": if state.audio_end_time_s.get() > 0.0 { Some((state.audio_end_time_s.get() * 1000.0) as u32) } else { None },
                "volume_boost_db": if state.audio_volume_boost_db.get() > 0.0 { Some(state.audio_volume_boost_db.get()) } else { None },
                "enable_limiter": state.audio_enable_limiter.get(),
            }),
            "video-to-gif-webp" => serde_json::json!({
                "fps": state.video_fps.get(),
                "max_width": state.video_max_width.get(),
                "output_format": state.video_output_format.get(),
            }),
            _ => serde_json::json!({}),
        }
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
            {move || match state.view_mode.get() {
                MediaViewMode::Hub => view! {
                    // =========================================================
                    // CHẾ ĐỘ 1: HUB PORTAL VIEW (Toàn màn hình)
                    // =========================================================
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

                    <main class="media-studio-hub-main">
                        <MediaStudioHubView state=state />
                    </main>
                }.into_any(),

                MediaViewMode::Workflow => view! {
                    // =========================================================
                    // CHẾ ĐỘ 2: WORKFLOW MACRO DAG (Toàn màn hình)
                    // =========================================================
                    <header class="media-studio-header">
                        <div class="media-studio-header-left">
                            <button
                                type="button"
                                class="media-breadcrumb-btn"
                                title="Quay lại Trang chủ Hub công cụ Media"
                                on:click=move |_| state.open_hub()
                            >
                                <span class="breadcrumb-arrow" aria-hidden="true">"←"</span>
                                <span>"Tất cả công cụ (Media Hub)"</span>
                            </button>
                            <div class="media-studio-header-title-group">
                                <h1 class="media-studio-header-title">
                                    "⚡ Chuỗi Quy Trình Tự Động Hóa Media"
                                </h1>
                                <span class="media-studio-header-tagline">
                                    "Kết nối liên hoàn các công cụ xử lý đồ họa, âm thanh và video on-device"
                                </span>
                            </div>
                        </div>

                        <div class="media-studio-header-right">
                            <BackButton on_back=on_back label="Về Launcher" />
                            <span class="media-studio-privacy-badge">
                                "🔒 100% On-Device"
                            </span>
                        </div>
                    </header>

                    <div class="media-studio-workspace-body">
                        <MediaStudioGalleryPanel state=state />
                        <main class="media-studio-center-workspace">
                            <div class="media-studio-workflow-wrapper">
                                <WorkflowRunner
                                    available_tools=micro_app_tools(MicroAppId::MediaStudio)
                                    title="⚡ Chuỗi Quy trình Xử lý Media Studio"
                                    is_running=Signal::derive(move || state.is_running.get())
                                    status_message=Signal::derive(move || state.status_message.get())
                                    on_run=on_run_workflow
                                    on_cancel=on_cancel_workflow
                                />
                            </div>
                        </main>
                    </div>
                }.into_any(),

                MediaViewMode::ToolWorkspace => {
                    let active_slug = state.active_tool.get();
                    let current_tool = state.find_tool(&active_slug);
                    let tool_name = current_tool.map(|t| t.name).unwrap_or("Công cụ Media");
                    let tool_desc = current_tool.map(|t| t.description).unwrap_or("");
                    let icon = media_studio_tool_icon(&active_slug);
                    let category_label = category_for_tool(&active_slug)
                        .map(|c| c.label())
                        .unwrap_or("Đa phương tiện");

                    view! {
                        // =========================================================
                        // CHẾ ĐỘ 3: FOCUSED TOOL WORKSPACE (Mini Sidebar + Tool View + Persistent Dock)
                        // =========================================================
                        <header class="media-studio-header media-studio-focused-header">
                            <div class="media-studio-header-left">
                                <button
                                    type="button"
                                    class="media-breadcrumb-btn"
                                    title="Quay lại Trang chủ Hub công cụ Media"
                                    on:click=move |_| state.open_hub()
                                >
                                    <span class="breadcrumb-arrow" aria-hidden="true">"←"</span>
                                    <span>"Tất cả công cụ (Media Hub)"</span>
                                </button>

                                <div class="media-focused-tool-indicator">
                                    <span class="focused-tool-icon" aria-hidden="true">{icon}</span>
                                    <div class="focused-tool-text">
                                        <h2 class="media-focused-tool-title">{tool_name}</h2>
                                        <span class="media-focused-tool-desc">{tool_desc}</span>
                                    </div>
                                </div>
                            </div>

                            <div class="media-studio-header-right">
                                <span class="media-studio-cat-badge">{category_label}</span>
                                <span class="media-studio-slug-badge">{active_slug.clone()}</span>
                                <span class="media-studio-privacy-badge">"🔒 100% On-Device"</span>
                                <BackButton on_back=on_back label="Về Launcher" />
                            </div>
                        </header>

                        <div class="media-studio-workspace-body">
                            // Cột 1: Gallery Panel (Hỗ trợ Mini Sidebar 56px hoặc Expanded 280px)
                            <MediaStudioGalleryPanel state=state />

                            // Cột 2: Workspace hiển thị View của công cụ đang chọn
                            <main class="media-studio-center-workspace">
                                <div class="media-studio-tool-workspace">
                                    // Context Banner hiển thị tệp Media nguồn chung đang áp dụng
                                    <div class="media-studio-context-banner">
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
                                                                {format!("(Tệp {}/{} trong thư viện media)", current_idx, total)}
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
                                                    manual_trim_enabled=state.audio_manual_trim_enabled
                                                    start_time_s=state.audio_start_time_s
                                                    end_time_s=state.audio_end_time_s
                                                    volume_boost_db=state.audio_volume_boost_db
                                                    enable_limiter=state.audio_enable_limiter
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

                                        // Thanh thực thi tác vụ Media Studio On-Device
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
                                            options_builder=options_builder
                                            external_is_running=state.is_running
                                        />
                                    </div>
                                </div>
                            </main>
                        </div>

                        // Khu vực đáy: Persistent Dock Batch Processing Queue (ở ToolWorkspace)
                        <MediaStudioBatchQueue state=state />
                    }.into_any()
                }
            }}

            // Scoped Command Palette (Ctrl+K) trong Media Studio
            <CommandPalette
                open=palette_open
                tools=state.tools()
                on_deep_link=Callback::new(move |(_, slug): (_, String)| state.open_tool(&slug))
                on_back=on_back
            />
        </div>
    }
}
