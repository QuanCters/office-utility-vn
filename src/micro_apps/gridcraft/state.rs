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
    /// Số thứ tự edit revision hiện thời của workspace (Finding 2: In-Flight Edit Preservation)
    pub edit_revision: RwSignal<u64>,
    /// Ánh xạ các phiên workbook theo ID hoặc tên tệp (Finding 2: Multi-Workbook Save Isolation)
    pub document_sessions: RwSignal<std::collections::HashMap<String, WorkbookSession>>,
}

/// Dữ liệu phiên làm việc của một workbook độc lập (Stage 4C-1 Workbook Session Identity)
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WorkbookSession {
    pub document_id: String,
    pub file_path: String,
    pub file_name: String,
    pub revision: u64,
    #[serde(default)]
    pub is_dirty: bool,
    #[serde(default)]
    pub edit_revision: u64,
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
            edit_revision: RwSignal::new(0),
            document_sessions: RwSignal::new(std::collections::HashMap::new()),
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

    /// Đăng ký hoặc cập nhật một phiên workbook cụ thể (Finding 2)
    pub fn register_document_session(&self, session: WorkbookSession) {
        self.document_sessions.update(|map| {
            map.insert(session.document_id.clone(), session);
        });
        self.recompute_aggregate_dirty();
    }

    /// Lấy thông tin phiên workbook theo document ID hoặc file name
    pub fn get_session_for_document(&self, doc_id_or_name: &str) -> Option<WorkbookSession> {
        self.document_sessions.with_untracked(|map| {
            if let Some(s) = map.get(doc_id_or_name) {
                Some(s.clone())
            } else {
                map.values().find(|s| s.file_name == doc_id_or_name).cloned()
            }
        })
    }

    /// Phân giải phiên workbook đích chính xác để lưu (Finding 2: Multi-Workbook Save Isolation & P1 #4679994753)
    /// Trả về `None` nếu tệp không thuộc phiên làm việc nào đã đăng ký, hoặc tên tệp bị mơ hồ (buộc Save As / chọn đích rõ ràng).
    pub fn resolve_save_session(&self, name_or_doc_id: &str) -> Option<WorkbookSession> {
        // 1. Tìm trực tiếp theo document_id (định danh bền vững duy nhất)
        let direct_match = self.document_sessions.with_untracked(|map| map.get(name_or_doc_id).cloned());
        if let Some(sess) = direct_match {
            return Some(sess);
        }

        // 2. Phân giải theo tên tệp: Chỉ chấp nhận nếu tên tệp là duy nhất tuyệt đối across all sessions
        let matches: Vec<WorkbookSession> = self.document_sessions.with_untracked(|map| {
            map.values()
                .filter(|s| s.file_name == name_or_doc_id)
                .cloned()
                .collect()
        });

        if matches.len() == 1 {
            Some(matches[0].clone())
        } else {
            // matches.len() > 1 (nhiều workbook cùng tên ở các thư mục khác nhau) hoặc 0 (không tìm thấy):
            // Fail closed (P1 #4679994753): Tuyệt đối KHÔNG sử dụng active_document_id hay active_file_name để đoán mò session!
            // Yêu cầu chỉ định document_id rõ ràng hoặc buộc người dùng chọn đích qua Save As.
            None
        }
    }

    /// Đánh dấu trạng thái dirty cho một workbook cụ thể và cập nhật trạng thái dirty tổng hợp của workspace
    pub fn set_document_dirty(&self, doc_id_or_name: &str, dirty: bool) {
        let mut sessions = self.document_sessions.get_untracked();
        let mut matched = false;
        if let Some(s) = sessions.get_mut(doc_id_or_name) {
            s.is_dirty = dirty;
            matched = true;
        } else {
            for s in sessions.values_mut() {
                if s.file_path == doc_id_or_name || s.file_name == doc_id_or_name {
                    s.is_dirty = dirty;
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            if let Some(active_id) = self.active_document_id.get_untracked() {
                if let Some(s) = sessions.get_mut(&active_id) {
                    s.is_dirty = dirty;
                }
            }
        }
        self.document_sessions.set(sessions);
        self.recompute_aggregate_dirty();
    }

    /// Tăng edit revision của tài liệu khi có thao tác chỉnh sửa từ người dùng (Finding 2)
    pub fn bump_edit_revision(&self, doc_id_or_name: &str) -> u64 {
        let mut sessions = self.document_sessions.get_untracked();
        let mut target_rev = 1;
        let mut matched = false;
        if let Some(s) = sessions.get_mut(doc_id_or_name) {
            s.edit_revision = s.edit_revision.wrapping_add(1);
            s.is_dirty = true;
            target_rev = s.edit_revision;
            matched = true;
        } else {
            for s in sessions.values_mut() {
                if s.file_path == doc_id_or_name || s.file_name == doc_id_or_name {
                    s.edit_revision = s.edit_revision.wrapping_add(1);
                    s.is_dirty = true;
                    target_rev = s.edit_revision;
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            if let Some(active_id) = self.active_document_id.get_untracked() {
                if let Some(s) = sessions.get_mut(&active_id) {
                    s.edit_revision = s.edit_revision.wrapping_add(1);
                    s.is_dirty = true;
                    target_rev = s.edit_revision;
                }
            }
        }
        self.document_sessions.set(sessions);
        self.edit_revision.update(|r| *r = r.wrapping_add(1));
        self.is_dirty.set(true);
        target_rev
    }

    /// Lấy edit revision hiện tại của một tài liệu (Finding 2)
    pub fn get_edit_revision(&self, doc_id_or_name: &str) -> u64 {
        self.document_sessions.with_untracked(|map| {
            if let Some(s) = map.get(doc_id_or_name) {
                s.edit_revision
            } else if let Some(s) = map.values().find(|s| s.file_name == doc_id_or_name || s.file_path == doc_id_or_name) {
                s.edit_revision
            } else {
                self.edit_revision.get_untracked()
            }
        })
    }

    /// Đặt lại toàn bộ trạng thái phiên làm việc khi người dùng chọn Hủy Thay đổi & Thoát (Finding 4)
    pub fn reset_state(&self) {
        self.is_dirty.set(false);
        self.active_file_name.set(None);
        self.active_file_path.set(None);
        self.active_document_id.set(None);
        self.active_revision.set(0);
        self.edit_revision.set(0);
        self.error_message.set(None);
        self.is_save_blocked_alert.set(false);
        self.save_blocked_detail.set(String::new());
        self.is_exit_confirm_open.set(false);
        self.provisional_open.set(None);
        self.pending_close_request_id.set(None);
        self.document_sessions.set(std::collections::HashMap::new());
    }

    /// Tái tính toán trạng thái dirty tổng hợp của toàn bộ workspace từ mọi workbook đang mở
    pub fn recompute_aggregate_dirty(&self) {
        let sessions = self.document_sessions.get_untracked();
        let any_dirty = sessions.values().any(|s| s.is_dirty);
        self.is_dirty.set(any_dirty);
    }

    /// Cập nhật phiên tài liệu đã mở hoặc lưu
    pub fn set_document_session(&self, doc_id: Option<String>, path: Option<String>, rev: u64) {
        if let (Some(d_id), Some(f_path)) = (&doc_id, &path) {
            let f_name = std::path::Path::new(f_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("workbook.xlsx")
                .to_string();
            let (current_dirty, current_edit_rev) = self.document_sessions.with(|map| {
                map.get(d_id).map(|s| (s.is_dirty, s.edit_revision)).unwrap_or((false, 0))
            });
            self.register_document_session(WorkbookSession {
                document_id: d_id.clone(),
                file_path: f_path.clone(),
                file_name: f_name,
                revision: rev,
                is_dirty: current_dirty,
                edit_revision: current_edit_rev,
            });
        }
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
    pub fn commit_provisional_open_with_generation(
        &self,
        open_id: &str,
        current_generation: u64,
    ) -> bool {
        if let Some(prov) = self.provisional_open.get_untracked() {
            if prov.open_id == open_id && prov.generation == current_generation {
                self.register_document_session(WorkbookSession {
                    document_id: prov.document_id.clone(),
                    file_path: prov.file_path.clone(),
                    file_name: prov.file_name.clone(),
                    revision: prov.revision,
                    is_dirty: false,
                    edit_revision: 0,
                });
                self.set_document_session(
                    Some(prov.document_id),
                    Some(prov.file_path),
                    prov.revision,
                );
                self.active_file_name.set(Some(prov.file_name));
                // BẢO MẬT BẤT BIẾN: Tái tính toán trạng thái dirty tổng hợp từ mọi workbook đang mở,
                // không xóa mù quáng is_dirty về false nếu các workbook khác còn thay đổi chưa lưu!
                self.recompute_aggregate_dirty();
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
