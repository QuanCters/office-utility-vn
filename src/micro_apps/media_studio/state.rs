use leptos::prelude::*;
use crate::app::ToolItem;
use crate::micro_apps::{MicroAppId, ToolRegistry};

/// Mục công việc trong hàng đợi xử lý đa phương tiện hàng loạt (Batch Queue).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MediaBatchItem {
    /// Định danh duy nhất của tác vụ hàng loạt
    pub id: String,
    /// Tên tệp hoặc đường dẫn tệp đang xử lý
    pub file_name: String,
    /// Trạng thái tác vụ ("Pending", "Processing", "Done", "Failed")
    pub status: String,
    /// Tiến trình từ 0.0 -> 1.0 (hoặc 0 -> 100)
    pub progress: f32,
}

impl MediaBatchItem {
    pub fn new(
        id: impl Into<String>,
        file_name: impl Into<String>,
        status: impl Into<String>,
        progress: f32,
    ) -> Self {
        Self {
            id: id.into(),
            file_name: file_name.into(),
            status: status.into(),
            progress,
        }
    }
}

impl From<&str> for MediaBatchItem {
    fn from(file_name: &str) -> Self {
        Self {
            id: format!("batch-{}", file_name),
            file_name: file_name.to_string(),
            status: "Pending".to_string(),
            progress: 0.0,
        }
    }
}

impl From<String> for MediaBatchItem {
    fn from(file_name: String) -> Self {
        Self::from(file_name.as_str())
    }
}

/// Phân loại 2 nhóm nghiệp vụ đa phương tiện trong Media Studio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MediaCategory {
    /// Nén & đổi định dạng, cắt ảnh thẻ, che mờ, xóa Exif, watermark, làm đẹp screenshot, nắn scan
    Image,
    /// Cắt khoảng lặng âm thanh & chuyển video sang GIF/WebP
    AudioVideo,
}

impl MediaCategory {
    pub const ALL: [MediaCategory; 2] = [
        MediaCategory::Image,
        MediaCategory::AudioVideo,
    ];

    /// Tên hiển thị tiếng Việt của nhóm danh mục.
    pub fn label(self) -> &'static str {
        match self {
            MediaCategory::Image => "Xử lý Ảnh",
            MediaCategory::AudioVideo => "Audio & Video",
        }
    }

    /// Icon đại diện trực quan cho danh mục.
    pub fn icon(self) -> &'static str {
        match self {
            MediaCategory::Image => "📸",
            MediaCategory::AudioVideo => "🎬",
        }
    }

    /// Danh sách slug công cụ thuộc nhóm nghiệp vụ.
    pub fn slugs(self) -> &'static [&'static str] {
        match self {
            MediaCategory::Image => &[
                "image-batch",
                "photo-id-crop",
                "image-bg-redact",
                "image-exif-inspector",
                "watermark-batch-studio",
                "screenshot-beautifier",
                "doc-scanner-perspective",
            ],
            MediaCategory::AudioVideo => &[
                "audio-silence-trimmer",
                "video-to-gif-webp",
            ],
        }
    }
}

/// Ánh xạ một slug công cụ Media sang nhóm danh mục tương ứng.
pub fn category_for_tool(slug: &str) -> Option<MediaCategory> {
    MediaCategory::ALL.into_iter().find(|&cat| cat.slugs().contains(&slug))
}

/// Chế độ hiển thị chính của Media Studio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MediaViewMode {
    /// Trang chủ Hub/Portal hiển thị danh mục dạng lưới thẻ (iLoveIMG & Lightroom model)
    Hub,
    /// Không gian làm việc tập trung của công cụ cụ thể
    ToolWorkspace,
    /// Trình chạy chuỗi quy trình Workflow Macro DAG
    Workflow,
}

/// Ánh xạ tĩnh từ slug công cụ Media Studio sang icon emoji trực quan.
pub fn media_studio_tool_icon(slug: &str) -> &'static str {
    match slug {
        "image-batch" => "🗜️",
        "photo-id-crop" => "👤",
        "image-bg-redact" => "🕶️",
        "image-exif-inspector" => "🔍",
        "watermark-batch-studio" => "🏷️",
        "screenshot-beautifier" => "✨",
        "doc-scanner-perspective" => "📐",
        "audio-silence-trimmer" => "🎙️",
        "video-to-gif-webp" => "🎞️",
        _ => "🎨",
    }
}

/// Trạng thái hoạt động dùng chung (Shared Context) của Micro-App Media Studio.
///
/// Quản lý danh sách các tệp media đã nạp, công cụ đang chọn, chế độ Workflow Macro DAG,
/// và Hàng đợi xử lý hàng loạt (Batch Processing Queue).
#[derive(Clone, Copy, Debug)]
pub struct MediaStudioState {
    /// Danh sách đường dẫn hoặc tên các tệp media đã nạp vào workspace dùng chung.
    pub loaded_files: RwSignal<Vec<String>>,
    /// Chỉ số của file đang được kích hoạt trong loaded_files (None nếu chưa có file).
    pub active_file_index: RwSignal<Option<usize>>,
    /// Slug của công cụ Media đang được kích hoạt.
    pub active_tool: RwSignal<String>,
    /// Hàng đợi xử lý đa phương tiện hàng loạt.
    pub batch_queue: RwSignal<Vec<MediaBatchItem>>,
    /// Danh mục toolbar tab hiện tại đang được chọn xem.
    pub active_category: RwSignal<MediaCategory>,
    /// Cờ hiển thị chế độ Workflow Pipeline Macro DAG.
    pub is_workflow_active: RwSignal<bool>,
    /// Trạng thái đang thực thi tác vụ.
    pub is_running: RwSignal<bool>,
    /// Thông điệp kết quả hoặc tiến trình thực thi.
    pub status_message: RwSignal<String>,
    /// Trạng thái mở rộng/thu gọn của bảng Batch Queue ở dưới đáy.
    pub is_queue_expanded: RwSignal<bool>,
    /// Chế độ xem hiện tại của Media Studio (Hub, ToolWorkspace hoặc Workflow).
    pub view_mode: RwSignal<MediaViewMode>,
    /// Trạng thái thu gọn của Sidebar danh sách tệp (true: Mini Sidebar 56px, false: Full 280px).
    pub sidebar_collapsed: RwSignal<bool>,
    /// Trạng thái mở Popover/Drawer của Floating Contextual Widget trên Hub.
    pub is_floating_queue_open: RwSignal<bool>,

    // Signals lưu cấu hình của từng công cụ
    pub img_batch_target_format: RwSignal<String>,
    pub img_batch_quality: RwSignal<u32>,
    pub img_batch_max_width: RwSignal<String>,
    pub img_batch_strip_exif: RwSignal<bool>,

    pub photo_id_size: RwSignal<String>,
    pub photo_id_sheet_layout: RwSignal<String>,

    pub redact_type: RwSignal<String>,
    pub redact_auto_detect: RwSignal<bool>,

    pub exif_strip_all: RwSignal<bool>,

    pub watermark_type: RwSignal<String>,
    pub watermark_opacity: RwSignal<f32>,
    pub watermark_text: RwSignal<String>,

    pub beautifier_theme: RwSignal<String>,
    pub beautifier_background: RwSignal<String>,
    pub beautifier_shadow_blur: RwSignal<f32>,

    pub scanner_auto_enhance: RwSignal<bool>,

    pub audio_silence_threshold_db: RwSignal<f32>,
    pub audio_min_silence_duration_ms: RwSignal<u32>,

    pub video_fps: RwSignal<u32>,
    pub video_max_width: RwSignal<Option<u32>>,
    pub video_output_format: RwSignal<String>,
}

impl Default for MediaStudioState {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaStudioState {
    /// Khởi tạo trạng thái Media Studio với công cụ mặc định "image-batch".
    pub fn new() -> Self {
        Self::with_initial_tool(None)
    }

    /// Khởi tạo trạng thái Media Studio với công cụ ban đầu tùy chọn.
    pub fn with_initial_tool(initial_tool: Option<String>) -> Self {
        let has_custom_tool = initial_tool.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let initial_slug = initial_tool
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "image-batch".to_string());

        let category = category_for_tool(&initial_slug).unwrap_or(MediaCategory::Image);
        let view_mode = if has_custom_tool {
            MediaViewMode::ToolWorkspace
        } else {
            MediaViewMode::Hub
        };

        Self {
            loaded_files: RwSignal::new(Vec::new()),
            active_file_index: RwSignal::new(None),
            active_tool: RwSignal::new(initial_slug),
            batch_queue: RwSignal::new(Vec::new()),
            active_category: RwSignal::new(category),
            is_workflow_active: RwSignal::new(false),
            is_running: RwSignal::new(false),
            status_message: RwSignal::new(String::new()),
            is_queue_expanded: RwSignal::new(true),
            view_mode: RwSignal::new(view_mode),
            sidebar_collapsed: RwSignal::new(true),
            is_floating_queue_open: RwSignal::new(false),

            // Tool defaults
            img_batch_target_format: RwSignal::new("webp".to_string()),
            img_batch_quality: RwSignal::new(80),
            img_batch_max_width: RwSignal::new("1920".to_string()),
            img_batch_strip_exif: RwSignal::new(true),

            photo_id_size: RwSignal::new("3x4".to_string()),
            photo_id_sheet_layout: RwSignal::new("single".to_string()),

            redact_type: RwSignal::new("pixelate".to_string()),
            redact_auto_detect: RwSignal::new(true),

            exif_strip_all: RwSignal::new(true),

            watermark_type: RwSignal::new("tiled".to_string()),
            watermark_opacity: RwSignal::new(0.3),
            watermark_text: RwSignal::new("BẢN QUYỀN".to_string()),

            beautifier_theme: RwSignal::new("macos_dark".to_string()),
            beautifier_background: RwSignal::new("sunset".to_string()),
            beautifier_shadow_blur: RwSignal::new(24.0),

            scanner_auto_enhance: RwSignal::new(true),

            audio_silence_threshold_db: RwSignal::new(-40.0),
            audio_min_silence_duration_ms: RwSignal::new(500),

            video_fps: RwSignal::new(15),
            video_max_width: RwSignal::new(Some(480)),
            video_output_format: RwSignal::new("gif".to_string()),
        }
    }

    /// Thêm một tệp vào danh sách làm việc chung.
    /// Nếu chưa có tệp nào đang được chọn, tự động đặt tệp đầu tiên làm active.
    pub fn add_file(&self, path: impl Into<String>) {
        let path_str = path.into();
        if path_str.trim().is_empty() {
            return;
        }
        self.loaded_files.update(|f| f.push(path_str));
        if self.active_file_index.get().is_none() {
            self.active_file_index.set(Some(0));
        }
    }

    /// Thêm danh sách nhiều tệp cùng lúc vào workspace.
    pub fn add_files<I, S>(&self, paths: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut added_any = false;
        self.loaded_files.update(|list| {
            for p in paths {
                let s = p.into();
                if !s.trim().is_empty() {
                    list.push(s);
                    added_any = true;
                }
            }
        });
        if added_any && self.active_file_index.get().is_none() {
            self.active_file_index.set(Some(0));
        }
    }

    /// Xóa một tệp khỏi danh sách theo chỉ số vị trí, đồng thời hiệu chỉnh lại active_file_index.
    pub fn remove_file(&self, index: usize) {
        let mut removed = false;
        self.loaded_files.update(|files| {
            if index < files.len() {
                files.remove(index);
                removed = true;
            }
        });

        if !removed {
            return;
        }

        let total = self.loaded_files_count();
        if total == 0 {
            self.active_file_index.set(None);
            return;
        }

        if let Some(cur) = self.active_file_index.get() {
            if cur == index {
                let next_idx = if cur >= total { total - 1 } else { cur };
                self.active_file_index.set(Some(next_idx));
            } else if cur > index {
                self.active_file_index.set(Some(cur - 1));
            }
        }
    }

    /// Xóa toàn bộ tệp trong danh sách đã nạp.
    pub fn clear_files(&self) {
        self.loaded_files.set(Vec::new());
        self.active_file_index.set(None);
    }

    /// Chọn một tệp cụ thể trong danh sách theo chỉ số.
    pub fn select_file(&self, index: usize) {
        let len = self.loaded_files_count();
        if index < len {
            self.active_file_index.set(Some(index));
        }
    }

    /// Kiểm tra danh sách tệp có rỗng hay không.
    pub fn has_files(&self) -> bool {
        !self.loaded_files.with_untracked(|f| f.is_empty())
    }

    /// Số lượng tệp đã nạp vào workspace.
    pub fn loaded_files_count(&self) -> usize {
        self.loaded_files.with_untracked(|f| f.len())
    }

    /// Đường dẫn tệp đang được chọn (None nếu chưa có tệp).
    pub fn active_file_path(&self) -> Option<String> {
        let files = self.loaded_files.get();
        let idx = self.active_file_index.get()?;
        files.get(idx).cloned()
    }

    /// Tên rút gọn (basename) của tệp đang được chọn.
    pub fn active_file_name(&self) -> Option<String> {
        self.active_file_path().map(|p| {
            p.split('/')
                .next_back()
                .or_else(|| p.split('\\').next_back())
                .unwrap_or(&p)
                .to_string()
        })
    }

    /// Slug công cụ đang kích hoạt.
    pub fn active_tool_slug(&self) -> String {
        self.active_tool.get()
    }

    /// Chọn công cụ cụ thể, tự động cập nhật category tương ứng và chuyển sang chế độ ToolWorkspace.
    pub fn select_tool(&self, slug: impl Into<String>) {
        let s = slug.into();
        if let Some(cat) = category_for_tool(&s) {
            self.active_category.set(cat);
        }
        self.active_tool.set(s);
        self.is_workflow_active.set(false);
        self.view_mode.set(MediaViewMode::ToolWorkspace);
    }

    /// Chuyển tab danh mục, tự động chọn công cụ đầu tiên thuộc danh mục đó nếu công cụ hiện tại không thuộc nhóm.
    pub fn select_category(&self, category: MediaCategory) {
        self.active_category.set(category);
        self.is_workflow_active.set(false);
        self.view_mode.set(MediaViewMode::ToolWorkspace);

        let cur_tool = self.active_tool.get();
        if category_for_tool(&cur_tool) != Some(category)
            && let Some(&first_slug) = category.slugs().first()
        {
            self.active_tool.set(first_slug.to_string());
        }
    }

    /// Kích hoạt chế độ Workflow Pipeline Macro DAG.
    pub fn open_workflow(&self) {
        self.is_workflow_active.set(true);
        self.view_mode.set(MediaViewMode::Workflow);
    }

    /// Tắt chế độ Workflow Pipeline Macro DAG quay về ToolWorkspace.
    pub fn close_workflow(&self) {
        self.is_workflow_active.set(false);
        self.view_mode.set(MediaViewMode::ToolWorkspace);
    }

    /// Mở trang chủ Hub/Portal (lưới thẻ phân loại theo danh mục).
    pub fn open_hub(&self) {
        self.view_mode.set(MediaViewMode::Hub);
        self.is_workflow_active.set(false);
    }

    /// Mở không gian làm việc tập trung của công cụ, tự động thu gọn sidebar về mini.
    pub fn open_tool(&self, slug: &str) {
        self.select_tool(slug);
        self.view_mode.set(MediaViewMode::ToolWorkspace);
        self.sidebar_collapsed.set(true);
    }

    /// Đảo ngược cờ thu gọn sidebar (mini sidebar <-> expanded sidebar).
    pub fn toggle_sidebar(&self) {
        self.sidebar_collapsed.update(|c| *c = !*c);
    }

    /// Đảo ngược trạng thái mở Drawer của Floating Contextual Widget trên Hub.
    pub fn toggle_floating_queue(&self) {
        self.is_floating_queue_open.update(|open| *open = !*open);
    }

    /// Đóng Drawer của Floating Contextual Widget.
    pub fn close_floating_queue(&self) {
        self.is_floating_queue_open.set(false);
    }

    /// Kiểm tra xem có đang ở trang Hub Portal không.
    pub fn is_hub(&self) -> bool {
        self.view_mode.get_untracked() == MediaViewMode::Hub
    }

    /// Kiểm tra xem có đang ở không gian công cụ tập trung không.
    pub fn is_tool_workspace(&self) -> bool {
        self.view_mode.get_untracked() == MediaViewMode::ToolWorkspace
    }

    /// Kiểm tra xem có tiến trình xử lý ngầm trong hàng đợi không.
    pub fn is_background_processing(&self) -> bool {
        self.is_running.get_untracked() || self.batch_queue.with_untracked(|q| {
            q.iter().any(|it| it.status == "Processing")
        })
    }

    /// Tính tỷ lệ tiến trình hoàn thành trung bình của Batch Queue (0.0 đến 1.0).
    pub fn batch_progress_ratio(&self) -> f32 {
        self.batch_queue.with_untracked(|q| {
            if q.is_empty() {
                return 0.0;
            }
            let sum: f32 = q.iter().map(|it| {
                if it.status == "Done" || it.status == "Hoàn thành" {
                    1.0
                } else if it.status == "Processing" {
                    it.progress.clamp(0.0, 1.0)
                } else {
                    0.0
                }
            }).sum();
            (sum / q.len() as f32).clamp(0.0, 1.0)
        })
    }

    /// Kiểm tra xem hiện có đang ở chế độ Workflow hay không.
    pub fn is_workflow(&self) -> bool {
        self.is_workflow_active.get_untracked() || self.view_mode.get_untracked() == MediaViewMode::Workflow
    }

    /// Kiểm tra xem có đang thực thi tác vụ hay không.
    pub fn is_running(&self) -> bool {
        self.is_running.get_untracked()
    }

    /// Tìm thông tin chi tiết của một công cụ theo slug từ ToolRegistry.
    pub fn find_tool(&self, slug: &str) -> Option<&'static ToolItem> {
        self.tools().iter().find(|t| t.slug == slug)
    }

    // --- Batch Queue helper methods ---

    /// Đưa một tác vụ vào hàng đợi xử lý hàng loạt.
    pub fn enqueue_batch_item(&self, item: impl Into<MediaBatchItem>) {
        let batch_item = item.into();
        self.batch_queue.update(|q| q.push(batch_item));
    }

    /// Cập nhật tiến trình và trạng thái của một mục trong hàng đợi theo id.
    pub fn update_batch_progress(&self, id: &str, status: &str, progress: f32) {
        self.batch_queue.update(|queue| {
            if let Some(item) = queue.iter_mut().find(|it| it.id == id) {
                item.status = status.to_string();
                item.progress = progress;
            }
        });
    }

    /// Xóa toàn bộ các tác vụ đã hoàn thành ("Done" hoặc "Hoàn thành") khỏi hàng đợi.
    pub fn clear_completed_batch(&self) {
        self.batch_queue.update(|queue| {
            queue.retain(|it| it.status != "Done" && it.status != "Hoàn thành");
        });
    }

    /// Xóa một mục cụ thể khỏi hàng đợi theo id.
    pub fn remove_batch_item(&self, id: &str) {
        self.batch_queue.update(|queue| {
            queue.retain(|it| it.id != id);
        });
    }

    /// Xóa sạch toàn bộ hàng đợi xử lý hàng loạt.
    pub fn clear_batch_queue(&self) {
        self.batch_queue.set(Vec::new());
    }

    /// Số lượng mục hiện tại trong hàng đợi xử lý hàng loạt.
    pub fn batch_items_count(&self) -> usize {
        self.batch_queue.with_untracked(|q| q.len())
    }

    /// Kiểm tra xem hàng đợi có tác vụ nào đang chờ hoặc đang chạy không.
    pub fn has_active_batches(&self) -> bool {
        self.batch_queue.with_untracked(|q| {
            q.iter().any(|it| it.status == "Pending" || it.status == "Processing")
        })
    }

    /// Bật/tắt mở rộng bảng Batch Queue.
    pub fn toggle_queue_expanded(&self) {
        self.is_queue_expanded.update(|exp| *exp = !*exp);
    }

    /// Danh sách công cụ thuộc phạm vi Media Studio (ủy quyền từ ToolRegistry).
    pub fn tools(&self) -> &'static [ToolItem] {
        ToolRegistry::micro_app_tools(MicroAppId::MediaStudio)
    }

    /// Danh sách công cụ khả dụng (alias cho tools()).
    pub fn available_tools(&self) -> &'static [ToolItem] {
        self.tools()
    }
}
