use leptos::prelude::*;
use crate::app::ToolItem;
use crate::micro_apps::{MicroAppId, ToolRegistry};

/// Phân loại 4 nhóm nghiệp vụ xử lý tài liệu PDF trong PDF Studio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PdfCategory {
    /// Ghép, tách, cắt viền, dàn trang sách, cây mục lục & đối soát PDF
    MergeSplit,
    /// Mật khẩu, nén tối ưu, tẩy metadata, làm phẳng form & phục hồi PDF lỗi
    SecurityOptimize,
    /// Đánh số trang Bates, đóng dấu watermark & chuyển PDF sang đen trắng
    FormatPage,
    /// OCR tiếng Việt, chuyển đổi Markdown, ảnh dọc dài & trích xuất bảng Excel
    ConvertOcr,
}

impl PdfCategory {
    pub const ALL: [PdfCategory; 4] = [
        PdfCategory::MergeSplit,
        PdfCategory::SecurityOptimize,
        PdfCategory::FormatPage,
        PdfCategory::ConvertOcr,
    ];

    /// Tên hiển thị tiếng Việt của nhóm danh mục.
    pub fn label(self) -> &'static str {
        match self {
            PdfCategory::MergeSplit => "Ghép/Tách",
            PdfCategory::SecurityOptimize => "Bảo mật & Tối ưu",
            PdfCategory::FormatPage => "Định dạng & Trang",
            PdfCategory::ConvertOcr => "Chuyển đổi & OCR",
        }
    }

    /// Icon đại diện trực quan cho danh mục.
    pub fn icon(self) -> &'static str {
        match self {
            PdfCategory::MergeSplit => "📑",
            PdfCategory::SecurityOptimize => "🛡️",
            PdfCategory::FormatPage => "📐",
            PdfCategory::ConvertOcr => "🔄",
        }
    }

    /// Danh sách slug công cụ thuộc nhóm nghiệp vụ.
    pub fn slugs(self) -> &'static [&'static str] {
        match self {
            PdfCategory::MergeSplit => &[
                "pdf-merge-split",
                "pdf-page-organizer",
                "pdf-crop-margin",
                "pdf-booklet-imposition",
                "pdf-bookmark-toc",
                "pdf-visual-diff",
            ],
            PdfCategory::SecurityOptimize => &[
                "pdf-protect-sign",
                "pdf-signature-pad",
                "pdf-compress",
                "pdf-metadata-scrubber",
                "pdf-flatten-forms",
                "pdf-form-manager",
                "pdf-redaction",
                "pdf-archive-linearize",
                "pdf-pki-sign",
                "pdf-repair-corrupt",
            ],
            PdfCategory::FormatPage => &[
                "pdf-page-number-header",
                "pdf-watermark-stamp",
                "pdf-grayscale-print",
            ],
            PdfCategory::ConvertOcr => &[
                "vietnamese-ocr",
                "pdf-to-markdown",
                "pdf-to-long-image",
                "pdf-to-images-pack",
                "pdf-table-extractor",
                "markdown-to-styled-pdf",
            ],
        }
    }
}

/// Ánh xạ một slug công cụ PDF sang nhóm danh mục tương ứng.
pub fn category_for_tool(slug: &str) -> Option<PdfCategory> {
    PdfCategory::ALL.into_iter().find(|&cat| cat.slugs().contains(&slug))
}

/// Chế độ hiển thị chính của PDF Studio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PdfViewMode {
    /// Trang chủ Hub/Portal hiển thị danh mục dạng lưới thẻ (iLovePDF model)
    Hub,
    /// Không gian làm việc tập trung của công cụ cụ thể
    ToolWorkspace,
    /// Trình chạy chuỗi quy trình Workflow Macro DAG
    Workflow,
}

/// Trạng thái hoạt động dùng chung (Shared Context) của Micro-App PDF Studio.
///
/// Các tệp PDF được nạp vào một lần sẽ có sẵn cho toàn bộ 18 công cụ xử lý.
#[derive(Clone, Copy, Debug)]
pub struct PdfStudioState {
    /// Danh sách đường dẫn hoặc tên các tệp PDF đã nạp vào workspace dùng chung.
    pub loaded_files: RwSignal<Vec<String>>,
    /// Chỉ số của file đang được kích hoạt trong danh sách loaded_files (None nếu chưa nạp file).
    pub active_file_index: RwSignal<Option<usize>>,
    /// Slug của công cụ PDF đang được chọn xử lý.
    pub active_tool: RwSignal<String>,
    /// Cờ hiển thị chế độ Workflow Pipeline Macro DAG nội bộ.
    pub is_workflow_active: RwSignal<bool>,
    /// Trạng thái đang thực thi tác vụ.
    pub is_running: RwSignal<bool>,
    /// Thông điệp kết quả hoặc tiến trình thực thi.
    pub status_message: RwSignal<String>,
    /// Danh mục toolbar tab hiện tại đang được chọn xem.
    pub active_category: RwSignal<PdfCategory>,
    /// Chế độ xem hiện tại của PDF Studio (Hub, ToolWorkspace hoặc Workflow).
    pub view_mode: RwSignal<PdfViewMode>,
    /// Trạng thái thu gọn của Sidebar danh sách tệp (true: Mini Sidebar 56px, false: Full 280px).
    pub sidebar_collapsed: RwSignal<bool>,
}

impl Default for PdfStudioState {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfStudioState {
    /// Khởi tạo trạng thái PDF Studio với công cụ mặc định "pdf-merge-split".
    pub fn new() -> Self {
        Self::with_initial_tool(None)
    }

    /// Khởi tạo trạng thái PDF Studio với công cụ ban đầu tùy chọn.
    pub fn with_initial_tool(initial_tool: Option<String>) -> Self {
        let has_custom_tool = initial_tool.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let initial_slug = initial_tool
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "pdf-merge-split".to_string());

        let category = category_for_tool(&initial_slug).unwrap_or(PdfCategory::MergeSplit);
        let view_mode = if has_custom_tool {
            PdfViewMode::ToolWorkspace
        } else {
            PdfViewMode::Hub
        };

        Self {
            loaded_files: RwSignal::new(Vec::new()),
            active_file_index: RwSignal::new(None),
            active_tool: RwSignal::new(initial_slug),
            is_workflow_active: RwSignal::new(false),
            is_running: RwSignal::new(false),
            status_message: RwSignal::new(String::new()),
            active_category: RwSignal::new(category),
            view_mode: RwSignal::new(view_mode),
            sidebar_collapsed: RwSignal::new(true),
        }
    }

    /// Thêm một tệp PDF vào danh sách làm việc chung.
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

    /// Thêm nhiều tệp PDF vào danh sách làm việc chung.
    pub fn add_files(&self, paths: impl IntoIterator<Item = impl Into<String>>) {
        let mut added = false;
        self.loaded_files.update(|f| {
            for p in paths {
                let s = p.into();
                if !s.trim().is_empty() {
                    f.push(s);
                    added = true;
                }
            }
        });
        if added && self.active_file_index.get().is_none() {
            self.active_file_index.set(Some(0));
        }
    }

    /// Xóa một tệp khỏi danh sách làm việc theo chỉ số và điều chỉnh active_file_index an toàn.
    pub fn remove_file(&self, index: usize) {
        let initial_len = self.loaded_files.with(|f| f.len());
        if index >= initial_len {
            return;
        }

        self.loaded_files.update(|files| {
            files.remove(index);
        });

        let new_len = self.loaded_files.with(|f| f.len());
        if new_len == 0 {
            self.active_file_index.set(None);
        } else {
            self.active_file_index.update(|opt_idx| {
                if let Some(active) = *opt_idx {
                    if active == index {
                        if active >= new_len {
                            *opt_idx = Some(new_len - 1);
                        } else {
                            *opt_idx = Some(active);
                        }
                    } else if active > index {
                        *opt_idx = Some(active - 1);
                    }
                }
            });
        }
    }

    /// Xóa toàn bộ danh sách tệp PDF đã nạp.
    pub fn clear_files(&self) {
        self.loaded_files.set(Vec::new());
        self.active_file_index.set(None);
    }

    /// Lấy đường dẫn tệp PDF đang được kích hoạt (nếu có).
    pub fn active_file_path(&self) -> Option<String> {
        let idx = self.active_file_index.get()?;
        self.loaded_files.with(|files| files.get(idx).cloned())
    }

    /// Lấy tên tệp rút gọn (basename) của tệp PDF đang kích hoạt.
    pub fn active_file_name(&self) -> Option<String> {
        self.active_file_path().map(|p| {
            p.rsplit(['/', '\\'])
                .next()
                .unwrap_or(&p)
                .to_string()
        })
    }

    /// Chọn một tệp cụ thể trong danh sách theo chỉ số index.
    pub fn select_file(&self, index: usize) {
        let len = self.loaded_files.with(|f| f.len());
        if index < len {
            self.active_file_index.set(Some(index));
        }
    }

    /// Chọn một công cụ PDF, tự động đồng bộ danh mục toolbar và thoát chế độ workflow.
    pub fn select_tool(&self, slug: &str) {
        self.active_tool.set(slug.to_string());
        self.is_workflow_active.set(false);
        self.view_mode.set(PdfViewMode::ToolWorkspace);
        if let Some(cat) = category_for_tool(slug) {
            self.active_category.set(cat);
        }
    }

    /// Chuyển tab danh mục toolbar. Nếu công cụ hiện tại không thuộc tab mới, chọn công cụ đầu tiên của tab.
    pub fn select_category(&self, cat: PdfCategory) {
        self.active_category.set(cat);
        self.is_workflow_active.set(false);
        self.view_mode.set(PdfViewMode::ToolWorkspace);
        let cur_tool = self.active_tool.get();
        if !cat.slugs().contains(&cur_tool.as_str())
            && let Some(&first_slug) = cat.slugs().first()
        {
            self.active_tool.set(first_slug.to_string());
        }
    }

    /// Mở màn hình thiết kế chuỗi Workflow Macro DAG.
    pub fn open_workflow(&self) {
        self.is_workflow_active.set(true);
        self.view_mode.set(PdfViewMode::Workflow);
    }

    /// Đóng màn hình workflow trở về công cụ hiện tại.
    pub fn close_workflow(&self) {
        self.is_workflow_active.set(false);
        self.view_mode.set(PdfViewMode::ToolWorkspace);
    }

    /// Mở trang chủ Hub/Portal (lưới thẻ phân loại theo danh mục).
    pub fn open_hub(&self) {
        self.view_mode.set(PdfViewMode::Hub);
        self.is_workflow_active.set(false);
    }

    /// Mở không gian làm việc tập trung của công cụ, tự động thu gọn sidebar về mini.
    pub fn open_tool(&self, slug: &str) {
        self.select_tool(slug);
        self.view_mode.set(PdfViewMode::ToolWorkspace);
        self.sidebar_collapsed.set(true);
    }

    /// Đảo ngược cờ thu gọn sidebar (mini sidebar <-> expanded sidebar).
    pub fn toggle_sidebar(&self) {
        self.sidebar_collapsed.update(|c| *c = !*c);
    }

    /// Kiểm tra xem có đang ở trang Hub Portal không.
    pub fn is_hub(&self) -> bool {
        self.view_mode.get_untracked() == PdfViewMode::Hub
    }

    /// Kiểm tra xem có đang ở không gian công cụ tập trung không.
    pub fn is_tool_workspace(&self) -> bool {
        self.view_mode.get_untracked() == PdfViewMode::ToolWorkspace
    }

    /// Lấy slug của công cụ đang kích hoạt.
    pub fn active_tool_slug(&self) -> String {
        self.active_tool.get()
    }

    /// Kiểm tra xem chế độ workflow có đang hiển thị hay không.
    pub fn is_workflow(&self) -> bool {
        self.is_workflow_active.get_untracked() || self.view_mode.get_untracked() == PdfViewMode::Workflow
    }

    /// Kiểm tra xem tác vụ có đang chạy hay không.
    pub fn is_running(&self) -> bool {
        self.is_running.get_untracked()
    }

    /// Số lượng tệp PDF hiện đang có trong workspace.
    pub fn loaded_files_count(&self) -> usize {
        self.loaded_files.with_untracked(|f| f.len())
    }

    /// Kiểm tra xem workspace đã có tệp PDF nào được nạp chưa.
    pub fn has_files(&self) -> bool {
        self.loaded_files.with_untracked(|f| !f.is_empty())
    }

    /// Danh sách công cụ thuộc phạm vi PDF Studio (ủy quyền từ ToolRegistry).
    pub fn tools(&self) -> &'static [ToolItem] {
        ToolRegistry::micro_app_tools(MicroAppId::PdfStudio)
    }

    /// Danh sách công cụ khả dụng (alias cho tools()).
    pub fn available_tools(&self) -> &'static [ToolItem] {
        self.tools()
    }

    /// Tìm kiếm thông tin công cụ PDF theo slug.
    pub fn find_tool(&self, slug: &str) -> Option<&'static ToolItem> {
        self.tools().iter().find(|t| t.slug == slug)
    }
}
