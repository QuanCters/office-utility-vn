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
    /// Không gian làm việc đồ họa trực quan PDFCraft Canvas (Visual Workspace)
    VisualWorkspace,
    /// Trang chủ Hub/Portal (được giữ tương thích ngược, ánh xạ vào VisualWorkspace)
    Hub,
    /// Không gian làm việc tập trung của công cụ cụ thể
    ToolWorkspace,
    /// Trình chạy chuỗi quy trình Workflow Macro DAG
    Workflow,
}

/// Dữ liệu phiên tài liệu PDF được ủy quyền (Finding 1: PDF Document Session Identity)
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PdfDocumentSession {
    pub file_path: String,
    pub document_id: Option<String>,
    pub revision: u64,
    pub is_dirty: bool,
    pub edit_generation: u64,
}

/// Bối cảnh yêu cầu lưu bất biến (P1 #4676016520: Preserve Originating Document Context During Save As)
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SaveRequestContext {
    pub request_id: String,
    pub operation_intent: crate::platform::adapter::SaveOperationIntent,
    pub originating_document_id: Option<String>,
    pub originating_path: Option<String>,
    pub session_generation: u64,
    pub snapshot_edit_generation: u64,
    pub expected_committed_revision: u64,
    pub snapshot_fingerprint: String,
    pub authorized_dest_path: Option<String>,
}

/// Dữ liệu một yêu cầu lưu PDF đang được thực hiện ở tầng native (Finding 4675800826 & 4676016520)
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PendingPdfSave {
    pub request_id: String,
    pub context: SaveRequestContext,
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
    /// Cờ trạng thái sẵn sàng của PDFCraft Canvas WebAssembly / WebGL iframe.
    pub is_canvas_ready: RwSignal<bool>,
    /// Cờ đánh dấu tài liệu đang có thay đổi chưa lưu (dirty state từ PDFCraft).
    pub has_unsaved_changes: RwSignal<bool>,
    /// ID phiên tài liệu được ủy quyền an toàn (Native Document Session ID).
    pub active_document_id: RwSignal<Option<String>>,
    /// Số thứ tự revision hiện hành của tài liệu đã được cam kết ghi đĩa.
    pub active_revision: RwSignal<u64>,
    /// Thế hệ chỉnh sửa của tài liệu (tăng dần khi có edit mới để chống mất dirty lúc đang lưu)
    pub edit_generation: RwSignal<u64>,
    /// Bộ đệm lưu trữ nội dung nhị phân của các tệp PDF đã mở trong phiên
    pub cached_documents: RwSignal<std::collections::HashMap<String, Vec<u8>>>,
    /// Ánh xạ các phiên tài liệu theo đường dẫn tệp (Finding 1: PDF Document Session Identity)
    pub document_sessions: RwSignal<std::collections::HashMap<String, PdfDocumentSession>>,
    /// Danh sách các tác vụ lưu tệp đang pending ở tầng native (Finding 4675800826)
    pub pending_saves: RwSignal<std::collections::HashMap<String, PendingPdfSave>>,
    /// Danh mục lọc hiện tại trên trang chủ PDF Studio Hub (None = Tất cả).
    pub hub_active_category: RwSignal<Option<PdfCategory>>,
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
        let initial_slug = initial_tool
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "pdf-merge-split".to_string());

        let category = category_for_tool(&initial_slug).unwrap_or(PdfCategory::MergeSplit);

        Self {
            loaded_files: RwSignal::new(Vec::new()),
            active_file_index: RwSignal::new(None),
            active_tool: RwSignal::new(initial_slug),
            is_workflow_active: RwSignal::new(false),
            is_running: RwSignal::new(false),
            status_message: RwSignal::new(String::new()),
            active_category: RwSignal::new(category),
            view_mode: RwSignal::new(PdfViewMode::Hub),
            sidebar_collapsed: RwSignal::new(true),
            is_canvas_ready: RwSignal::new(false),
            has_unsaved_changes: RwSignal::new(false),
            active_document_id: RwSignal::new(None),
            active_revision: RwSignal::new(0),
            edit_generation: RwSignal::new(0),
            cached_documents: RwSignal::new(std::collections::HashMap::new()),
            document_sessions: RwSignal::new(std::collections::HashMap::new()),
            pending_saves: RwSignal::new(std::collections::HashMap::new()),
            hub_active_category: RwSignal::new(None),
        }
    }

    /// Đăng ký một tác vụ lưu PDF pending trước khi gọi native async persistence (Finding 4675800826 & 4676016520)
    pub fn register_pending_save(
        &self,
        request_id: String,
        context: SaveRequestContext,
    ) {
        let mut map = self.pending_saves.get_untracked();
        map.insert(
            request_id.clone(),
            PendingPdfSave {
                request_id,
                context,
            },
        );
        self.pending_saves.set(map);
        self.is_running.set(true);
    }

    /// Xóa tác vụ lưu pending sau khi nhận kết quả terminal (thành công, thất bại, hủy)
    pub fn clear_pending_save(&self, request_id: &str) -> Option<PendingPdfSave> {
        let mut map = self.pending_saves.get_untracked();
        let removed = map.remove(request_id);
        self.pending_saves.set(map.clone());
        if map.is_empty() {
            self.is_running.set(false);
        }
        removed
    }

    /// Kiểm tra xem có bất kỳ tác vụ lưu PDF nào đang pending hay không
    pub fn has_pending_saves(&self) -> bool {
        self.pending_saves.with_untracked(|m| !m.is_empty())
    }

    /// Kiểm tra xem có tác vụ lưu pending cho một file cụ thể hay không (khớp originating_path hoặc authorized_dest_path)
    pub fn has_pending_saves_for_file(&self, path: &str) -> bool {
        self.pending_saves.with_untracked(|m| {
            m.values().any(|s| {
                s.context.originating_path.as_deref() == Some(path)
                    || s.context.authorized_dest_path.as_deref() == Some(path)
            })
        })
    }

    /// Kiểm tra xem có bất kỳ tài liệu PDF nào (kể cả inactive) chưa lưu hoặc đang pending lưu không
    pub fn has_unsaved_documents_or_pending_saves(&self) -> bool {
        if self.has_pending_saves() {
            return true;
        }
        if self.has_unsaved_changes.get_untracked() {
            return true;
        }
        self.document_sessions.with_untracked(|map| map.values().any(|s| s.is_dirty))
    }

    /// Đăng ký hoặc cập nhật phiên tài liệu cho một đường dẫn tệp cụ thể (Finding 1)
    pub fn register_document_session(&self, path: &str, doc_id: Option<String>, rev: u64) {
        let mut map = self.document_sessions.get_untracked();
        let existing = map.get(path);
        let is_dirty = existing.map(|s| s.is_dirty).unwrap_or(false);
        let edit_gen = existing.map(|s| s.edit_generation).unwrap_or(0);
        map.insert(
            path.to_string(),
            PdfDocumentSession {
                file_path: path.to_string(),
                document_id: doc_id,
                revision: rev,
                is_dirty,
                edit_generation: edit_gen,
            },
        );
        self.document_sessions.set(map);
    }

    /// Lấy phiên tài liệu theo đường dẫn tệp
    pub fn get_document_session(&self, path: &str) -> Option<PdfDocumentSession> {
        self.document_sessions.with_untracked(|map| map.get(path).cloned())
    }

    /// Lưu bộ đệm nội dung nhị phân của tài liệu đã được ủy quyền
    pub fn cache_document_bytes(&self, key: &str, bytes: Vec<u8>) {
        let mut map = self.cached_documents.get_untracked();
        map.insert(key.to_string(), bytes);
        self.cached_documents.set(map);
    }

    /// Lấy bộ đệm nội dung nhị phân của tài liệu
    pub fn get_cached_document_bytes(&self, key: &str) -> Option<Vec<u8>> {
        self.cached_documents.with(|map| map.get(key).cloned())
    }

    /// Thêm một tệp PDF vào danh sách làm việc chung.
    /// Tự động cập nhật chỉ số tệp active sang tệp mới thêm.
    pub fn add_file(&self, path: impl Into<String>) {
        let path_str = path.into();
        if path_str.trim().is_empty() {
            return;
        }
        if let Some(current_path) = self.active_file_path() {
            let current_dirty = self.has_unsaved_changes.get_untracked();
            let current_gen = self.edit_generation.get_untracked();
            let mut map = self.document_sessions.get_untracked();
            if let Some(sess) = map.get_mut(&current_path) {
                sess.is_dirty = current_dirty;
                sess.edit_generation = current_gen;
            }
            self.document_sessions.set(map);
        }

        let mut idx = 0;
        self.loaded_files.update(|f| {
            if let Some(pos) = f.iter().position(|p| p == &path_str) {
                idx = pos;
            } else {
                f.push(path_str.clone());
                idx = f.len() - 1;
            }
        });
        self.active_file_index.set(Some(idx));
        if let Some(sess) = self.get_document_session(&path_str) {
            self.active_document_id.set(sess.document_id);
            self.active_revision.set(sess.revision);
            self.has_unsaved_changes.set(sess.is_dirty);
            self.edit_generation.set(sess.edit_generation);
        } else {
            // P1 #4676016514: Tuyệt đối không kế thừa session/doc_id của tệp active trước đó
            self.active_document_id.set(None);
            self.active_revision.set(0);
            self.has_unsaved_changes.set(false);
            self.edit_generation.set(0);
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
            self.select_file(0);
        }
    }

    /// Xóa một tệp khỏi danh sách làm việc theo chỉ số và điều chỉnh active_file_index an toàn.
    pub fn remove_file(&self, index: usize) {
        let initial_len = self.loaded_files.with(|f| f.len());
        if index >= initial_len {
            return;
        }

        let mut removed_path = String::new();
        self.loaded_files.update(|files| {
            removed_path = files.remove(index);
        });

        // Dọn dẹp session của tệp bị xóa nếu không còn pending save
        if !self.has_pending_saves_for_file(&removed_path) {
            self.document_sessions.update(|map| {
                map.remove(&removed_path);
            });
        }

        let new_len = self.loaded_files.with(|f| f.len());
        if new_len == 0 {
            self.active_file_index.set(None);
            self.active_document_id.set(None);
            self.active_revision.set(0);
            self.has_unsaved_changes.set(false);
            self.edit_generation.set(0);
        } else {
            let current_active = self.active_file_index.get_untracked();
            let was_active_removed = current_active == Some(index);
            let mut next_idx = 0;

            self.active_file_index.update(|opt_idx| {
                if let Some(active) = *opt_idx {
                    if active == index {
                        if active >= new_len {
                            *opt_idx = Some(new_len - 1);
                            next_idx = new_len - 1;
                        } else {
                            *opt_idx = Some(active);
                            next_idx = active;
                        }
                    } else if active > index {
                        *opt_idx = Some(active - 1);
                        next_idx = active - 1;
                    } else {
                        next_idx = active;
                    }
                }
            });

            // P1 #4676016514 Fix: CHỈ đồng bộ hóa phiên tài liệu nếu chính tệp đang active bị xóa!
            // Nếu xóa tệp khác, TUYỆT ĐỐI KHÔNG chạm vào has_unsaved_changes hoặc edit_generation của tệp đang active.
            if was_active_removed {
                let opt_new_path = self.loaded_files.with_untracked(|f| f.get(next_idx).cloned());
                if let Some(new_path) = opt_new_path {
                    if let Some(sess) = self.get_document_session(&new_path) {
                        self.active_document_id.set(sess.document_id);
                        self.active_revision.set(sess.revision);
                        self.has_unsaved_changes.set(sess.is_dirty);
                        self.edit_generation.set(sess.edit_generation);
                    } else {
                        self.active_document_id.set(None);
                        self.active_revision.set(0);
                        self.has_unsaved_changes.set(false);
                        self.edit_generation.set(0);
                    }
                }
            }
        }
    }

    /// Xóa toàn bộ danh sách tệp PDF đã nạp.
    pub fn clear_files(&self) {
        self.loaded_files.set(Vec::new());
        self.active_file_index.set(None);
        self.active_document_id.set(None);
        self.active_revision.set(0);
        self.has_unsaved_changes.set(false);
        self.edit_generation.set(0);
        self.document_sessions.set(std::collections::HashMap::new());
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
    /// Khôi phục chính xác ID phiên tài liệu, revision, dirty state và edit generation (Finding 1 & Blocker 1).
    pub fn select_file(&self, index: usize) {
        let opt_path = self.loaded_files.with_untracked(|f| f.get(index).cloned());
        if let Some(path) = opt_path {
            if let Some(current_path) = self.active_file_path() {
                let current_dirty = self.has_unsaved_changes.get_untracked();
                let current_gen = self.edit_generation.get_untracked();
                let mut map = self.document_sessions.get_untracked();
                if let Some(sess) = map.get_mut(&current_path) {
                    sess.is_dirty = current_dirty;
                    sess.edit_generation = current_gen;
                }
                self.document_sessions.set(map);
            }

            self.active_file_index.set(Some(index));
            if let Some(sess) = self.get_document_session(&path) {
                self.active_document_id.set(sess.document_id);
                self.active_revision.set(sess.revision);
                self.has_unsaved_changes.set(sess.is_dirty);
                self.edit_generation.set(sess.edit_generation);
            } else {
                self.active_document_id.set(None);
                self.active_revision.set(0);
                self.has_unsaved_changes.set(false);
                self.edit_generation.set(0);
            }
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

    /// Mở không gian làm việc đồ họa trực quan PDFCraft Canvas (Visual Workspace).
    pub fn open_visual_workspace(&self) {
        self.view_mode.set(PdfViewMode::VisualWorkspace);
        self.is_workflow_active.set(false);
    }

    /// Mở trang chủ Hub/Portal của PDF Studio.
    pub fn open_hub(&self) {
        self.view_mode.set(PdfViewMode::Hub);
        self.is_workflow_active.set(false);
    }

    /// Chọn danh mục hiển thị trên trang chủ Hub.
    pub fn set_hub_category(&self, cat: Option<PdfCategory>) {
        self.hub_active_category.set(cat);
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

    /// Kiểm tra xem có đang ở không gian làm việc đồ họa trực quan PDFCraft Canvas hay không.
    pub fn is_visual_workspace(&self) -> bool {
        self.view_mode.get_untracked() == PdfViewMode::VisualWorkspace
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

    /// Kiểm tra xem PDFCraft Canvas iframe đã sẵn sàng nhận lệnh hay chưa.
    pub fn is_canvas_ready(&self) -> bool {
        self.is_canvas_ready.get_untracked()
    }

    /// Kiểm tra xem có thay đổi nào chưa được lưu trong canvas hay không.
    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes.get_untracked()
    }

    /// Cập nhật cờ sẵn sàng của canvas.
    pub fn set_canvas_ready(&self, ready: bool) {
        self.is_canvas_ready.set(ready);
    }

    /// Cập nhật cờ thay đổi chưa lưu (dirty) và tăng thế hệ chỉnh sửa (edit_generation)
    pub fn set_unsaved_changes(&self, dirty: bool) {
        self.has_unsaved_changes.set(dirty);
        if dirty {
            self.edit_generation.update(|g| *g += 1);
        }
    }

    /// Lấy document ID được ủy quyền hiện tại.
    pub fn active_document_id(&self) -> Option<String> {
        self.active_document_id.get()
    }

    /// Lấy revision hiện tại của tài liệu.
    pub fn active_revision(&self) -> u64 {
        self.active_revision.get()
    }

    /// Cập nhật thông tin phiên tài liệu sau khi nạp hoặc lưu (Finding 1: PDF Document Session Identity).
    pub fn set_active_document_session(&self, doc_id: Option<String>, rev: u64) {
        self.active_document_id.set(doc_id.clone());
        self.active_revision.set(rev);
        if let Some(active_path) = self.active_file_path() {
            self.register_document_session(&active_path, doc_id, rev);
        }
    }

    /// Xử lý cập nhật trạng thái phản ứng khi nhận thông điệp từ PDFCraft Bridge.
    pub fn handle_bridge_message(&self, message: &crate::micro_apps::pdf_studio::bridge::IncomingBridgeMessage) {
        match message {
            crate::micro_apps::pdf_studio::bridge::IncomingBridgeMessage::Ready { version } => {
                self.is_canvas_ready.set(true);
                self.status_message.set(format!("PDFCraft Canvas sẵn sàng (v{})", version));
            }
            crate::micro_apps::pdf_studio::bridge::IncomingBridgeMessage::Dirty { dirty } => {
                if !*dirty {
                    // Nếu iframe báo sạch (dirty = false) nhưng đang có pending save cho file này,
                    // KHÔNG ĐƯỢC chấp nhận thông báo sạch sớm từ iframe để xóa dirty!
                    // Dirty chỉ được xóa khi native persistence hoàn tất thành công bền vững.
                    let active_has_pending = self
                        .active_file_path()
                        .map(|p| self.has_pending_saves_for_file(&p))
                        .unwrap_or_else(|| self.has_pending_saves());
                    if active_has_pending {
                        return;
                    }
                }
                self.set_unsaved_changes(*dirty);
            }
            crate::micro_apps::pdf_studio::bridge::IncomingBridgeMessage::Save { name, .. } => {
                // TUYỆT ĐỐI KHÔNG xóa has_unsaved_changes ở đây! (Gate 1 - Stage 4B-2B)
                // has_unsaved_changes chỉ được chuyển sang false khi tác vụ lưu nhị phân thực sự thành công
                self.status_message.set(format!("Đang xử lý yêu cầu lưu tệp {}...", name));
            }
            crate::micro_apps::pdf_studio::bridge::IncomingBridgeMessage::Failed { error } => {
                self.is_canvas_ready.set(false);
                self.status_message.set(format!("Lỗi PDFCraft Canvas: {}", error));
            }
        }
    }
}
