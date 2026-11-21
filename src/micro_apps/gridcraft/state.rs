use leptos::prelude::*;

/// Trạng thái phiên làm việc của GridCraft Workspace (Stage 4B-2B State)
#[derive(Clone, Copy, Debug)]
pub struct GridCraftState {
    /// Cờ đánh dấu iframe WebAssembly đã khởi tạo thành công và gửi thông điệp ready
    pub is_canvas_ready: RwSignal<bool>,
    /// Cờ đánh dấu tài liệu có thay đổi chưa lưu (dirty)
    pub is_dirty: RwSignal<bool>,
    /// Tên tệp hiện đang mở
    pub active_file_name: RwSignal<Option<String>>,
    /// Đường dẫn tệp đích thực tế trên đĩa (nếu đã lưu/mở)
    pub active_file_path: RwSignal<Option<String>>,
    /// ID phiên tài liệu được ủy quyền an toàn (Native Document Session ID)
    pub active_document_id: RwSignal<Option<String>>,
    /// Số thứ tự revision hiện hành của tài liệu đã được cam kết ghi đĩa
    pub active_revision: RwSignal<u64>,
    /// Thông báo lỗi nếu có
    pub error_message: RwSignal<Option<String>>,
    /// Cảnh báo an ninh khi người dùng cố gắng thực hiện lệnh lưu tệp (bị chặn an toàn ở Stage 4B-2B)
    pub is_save_blocked_alert: RwSignal<bool>,
    /// Thông điệp chi tiết về sự kiện chặn lưu
    pub save_blocked_detail: RwSignal<String>,
    /// Cờ cho phép lưu trong môi trường kiểm thử nội bộ (Dev-only Save Permitted)
    pub is_dev_save_permitted: RwSignal<bool>,
    /// Hiển thị modal hỏi xác nhận khi rời khỏi workspace có thay đổi chưa lưu
    pub is_exit_confirm_open: RwSignal<bool>,
    /// Phiên mở tệp tạm thời đang chờ iframe xác nhận giải mã thành công (Stage 4C-0B Task 3)
    pub provisional_open: RwSignal<Option<ProvisionalOpen>>,
    /// ID yêu cầu đóng cửa sổ đang chờ lưu tệp hoàn tất (Stage 4C-0C Task 3)
    pub pending_close_request_id: RwSignal<Option<String>>,
}

/// Dữ liệu của một phiên mở tệp tạm thời đang chờ GridCraft giải mã
#[derive(Clone, Debug, PartialEq)]
pub struct ProvisionalOpen {
    pub open_id: String,
    pub document_id: String,
    pub file_path: String,
    pub file_name: String,
    pub revision: u64,
    pub generation: u64,
}

impl Default for GridCraftState {
    fn default() -> Self {
        Self::new()
    }
}

impl GridCraftState {
    pub fn new() -> Self {
        Self {
            is_canvas_ready: RwSignal::new(false),
            is_dirty: RwSignal::new(false),
            active_file_name: RwSignal::new(None),
            active_file_path: RwSignal::new(None),
            active_document_id: RwSignal::new(None),
            active_revision: RwSignal::new(0),
            error_message: RwSignal::new(None),
            is_save_blocked_alert: RwSignal::new(false),
            save_blocked_detail: RwSignal::new(String::new()),
            is_dev_save_permitted: RwSignal::new(false),
            is_exit_confirm_open: RwSignal::new(false),
            provisional_open: RwSignal::new(None),
            pending_close_request_id: RwSignal::new(None),
        }
    }

    /// Xử lý yêu cầu lưu tệp từ GridCraft iframe (Stage 4C-1 Product Cutover: Cho phép lưu chính thức qua Native Persistence)
    pub fn handle_save_requested(
        &self,
        _name: &str,
        _request_id: Option<&str>,
        _bytes: &[u8],
    ) -> Result<(), String> {
        Ok(())
    }

    /// Cập nhật phiên tài liệu đã mở hoặc lưu
    pub fn set_document_session(
        &self,
        doc_id: Option<String>,
        path: Option<String>,
        rev: u64,
    ) {
        self.active_document_id.set(doc_id);
        self.active_file_path.set(path);
        self.active_revision.set(rev);
    }

    /// Đánh dấu đã nạp tài liệu thành công
    pub fn set_active_file(&self, name: impl Into<String>) {
        self.active_file_name.set(Some(name.into()));
        self.is_dirty.set(false);
    }

    /// Bắt đầu một giao dịch mở tệp tạm thời (Stage 4C-0B Task 3 / Stage 4C-1 Transactional Open)
    pub fn begin_provisional_open(
        &self,
        open_id: String,
        document_id: String,
        file_path: String,
        file_name: String,
        revision: u64,
        generation: u64,
    ) {
        self.provisional_open.set(Some(ProvisionalOpen {
            open_id,
            document_id,
            file_path,
            file_name,
            revision,
            generation,
        }));
    }

    /// Hoàn tất cam kết giao dịch mở tệp khi iframe xác nhận parse thành công (Có kiểm tra generation)
    pub fn commit_provisional_open_with_generation(&self, open_id: &str, current_generation: u64) -> bool {
        if let Some(prov) = self.provisional_open.get_untracked() {
            if prov.open_id == open_id && prov.generation == current_generation {
                self.set_document_session(Some(prov.document_id), Some(prov.file_path), prov.revision);
                self.active_file_name.set(Some(prov.file_name));
                self.is_dirty.set(false);
                self.provisional_open.set(None);
                return true;
            }
        }
        false
    }

    /// Hoàn tất cam kết giao dịch mở tệp khi iframe xác nhận parse thành công
    pub fn commit_provisional_open(&self, open_id: &str) -> bool {
        if let Some(prov) = self.provisional_open.get_untracked() {
            return self.commit_provisional_open_with_generation(open_id, prov.generation);
        }
        false
    }

    /// Hủy bỏ giao dịch mở tệp khi iframe báo lỗi hoặc người dùng cancel
    pub fn abort_provisional_open(&self, open_id: Option<&str>) {
        if let Some(id) = open_id {
            if let Some(prov) = self.provisional_open.get_untracked() {
                if prov.open_id == id {
                    self.provisional_open.set(None);
                }
            }
        } else {
            self.provisional_open.set(None);
        }
    }
}
