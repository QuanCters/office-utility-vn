use serde::{Deserialize, Serialize};

/// Các trạng thái vòng đời chuẩn tắc của tài liệu phía Host (Host Document Lifecycle State).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentState {
    /// Chưa có tài liệu nào được nạp.
    Unloaded,
    /// Tài liệu sạch: dữ liệu trong editor khớp 100% với đĩa bền vững.
    Clean,
    /// Tài liệu có thay đổi chưa được lưu bền vững.
    Dirty,
    /// Đang trong tiến trình ghi đĩa (I/O in-flight).
    Saving {
        request_id: String,
        document_id: String,
        snapshot_fingerprint: String,
        dest_path: String,
        started_at_ms: u64,
        dirty_during_save: bool,
        snapshot_edit_revision: u64,
    },
    /// Ghi đĩa thất bại (Permission Denied, Disk Full, IO error).
    SaveFailed {
        error: String,
        dest_path: String,
        dirty: bool,
    },
    /// Phát hiện xung đột dữ liệu ngoài luồng hoặc bất đồng bộ.
    Conflict { reason: String },
    /// Đang chờ xác nhận của người dùng trước khi đóng/chuyển app vì có thay đổi chưa lưu.
    ClosingPending { target_navigation: String },
}

/// Quyết định của người dùng khi được hỏi về tài liệu chưa lưu (Unsaved Changes Dialog).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserClosingDecision {
    /// Lưu tài liệu trước khi đóng/chuyển tiếp.
    Save,
    /// Hủy bỏ toàn bộ thay đổi chưa lưu và tiếp tục đóng/chuyển tiếp.
    Discard,
    /// Hủy thao tác đóng/chuyển tiếp, ở lại ứng dụng hiện tại.
    Cancel,
}

/// Kết quả phản hồi của một hành động điều phối.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinatorAction {
    /// Cho phép tiếp tục điều hướng/đóng cửa sổ.
    ProceedNavigation(String),
    /// Ngăn chặn điều hướng, yêu cầu hiển thị hộp thoại xác nhận lưu.
    PromptUnsavedChanges {
        dest_path: String,
        target_navigation: String,
    },
    /// Bắt đầu ghi đĩa bền vững với payload bytes.
    PerformDurableWrite {
        request_id: String,
        dest_path: String,
        bytes: Vec<u8>,
    },
    /// Hiển thị thông báo lỗi lưu tệp cho người dùng.
    ShowSaveError { error: String, dest_path: String },
    /// Không cần hành động bổ sung.
    None,
}

/// Cấu hình và bộ điều phối vòng đời tài liệu Host (HostDocumentCoordinator).
/// Đóng vai trò cơ quan thẩm quyền duy nhất xác định tài liệu có an toàn hay không.
#[derive(Debug, Clone)]
pub struct HostDocumentCoordinator {
    /// Định danh thế hệ iframe hiện tại. Tăng lên mỗi khi iframe reload để vô hiệu hóa response cũ.
    session_generation: u64,
    /// Trạng thái tài liệu hiện tại.
    state: DocumentState,
    /// Định danh tài liệu đang mở.
    active_document_id: Option<String>,
    /// Đường dẫn tệp đích trên đĩa.
    dest_path: Option<String>,
    /// Mã băm SHA-256 của nội dung tệp lưu thành công gần nhất trên đĩa.
    last_persisted_fingerprint: Option<String>,
    /// Số thứ tự edit revision của tài liệu (Finding 2: In-Flight Edit Preservation)
    edit_revision: u64,
}

impl HostDocumentCoordinator {
    /// Khởi tạo một phiên coordinator mới ở trạng thái Unloaded.
    pub fn new() -> Self {
        Self {
            session_generation: 1,
            state: DocumentState::Unloaded,
            active_document_id: None,
            dest_path: None,
            last_persisted_fingerprint: None,
            edit_revision: 0,
        }
    }

    /// Lấy thế hệ iframe session hiện tại.
    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    /// Lấy trạng thái hiện tại.
    pub fn state(&self) -> &DocumentState {
        &self.state
    }

    /// Kiểm tra xem tài liệu hiện tại có thay đổi chưa được ghi đĩa bền vững hay không.
    pub fn is_dirty(&self) -> bool {
        match &self.state {
            DocumentState::Clean | DocumentState::Unloaded => false,
            DocumentState::Dirty => true,
            DocumentState::Saving { .. } => true,
            DocumentState::SaveFailed { dirty, .. } => *dirty,
            DocumentState::Conflict { .. } => true,
            DocumentState::ClosingPending { .. } => true,
        }
    }

    /// Trả về đường dẫn tài liệu hiện tại (nếu có)
    pub fn dest_path(&self) -> Option<&str> {
        self.dest_path.as_deref()
    }

    /// Trả về định danh tài liệu đang hoạt động trong coordinator (nếu có)
    pub fn active_document_id(&self) -> Option<&str> {
        self.active_document_id.as_deref()
    }

    /// Đặt lại trạng thái về Clean khi toàn bộ tài liệu đã được đóng/hủy tường minh và không có lưu đang chạy
    pub fn reset_if_dirty_on_close(&mut self) {
        if matches!(
            self.state,
            DocumentState::Dirty | DocumentState::SaveFailed { .. }
        ) {
            self.state = DocumentState::Clean;
        }
    }

    /// Được gọi khi iframe reload hoặc khởi động lại: tăng generation, hủy bỏ request pending.
    pub fn on_iframe_reload(&mut self) {
        self.session_generation = self.session_generation.wrapping_add(1);
        if let DocumentState::Saving { dest_path, .. } = &self.state {
            self.state = DocumentState::SaveFailed {
                error: "Iframe reloaded while save was in-flight".to_string(),
                dest_path: dest_path.clone(),
                dirty: true,
            };
        }
    }

    /// Được gọi khi tài liệu mới được nạp thành công vào editor.
    pub fn on_document_opened(
        &mut self,
        doc_id: impl Into<String>,
        path: impl Into<String>,
        initial_fingerprint: impl Into<String>,
    ) {
        self.active_document_id = Some(doc_id.into());
        let p = path.into();
        self.dest_path = Some(p);
        let fp = initial_fingerprint.into();
        self.last_persisted_fingerprint = Some(fp);
        self.edit_revision = 0;
        self.state = DocumentState::Clean;
    }

    /// Lấy edit revision hiện hành của tài liệu (Finding 2)
    pub fn edit_revision(&self) -> u64 {
        self.edit_revision
    }

    /// Tăng edit revision và ghi nhận thay đổi người dùng
    pub fn bump_edit_revision(&mut self) -> u64 {
        self.on_user_edit();
        self.edit_revision
    }

    /// Được gọi khi editor thông báo có chỉnh sửa mới từ người dùng.
    pub fn on_user_edit(&mut self) {
        self.edit_revision = self.edit_revision.wrapping_add(1);
        match &mut self.state {
            DocumentState::Clean => {
                self.state = DocumentState::Dirty;
            }
            DocumentState::Saving {
                dirty_during_save, ..
            } => {
                // Đánh dấu có edit xen ngang khi đang lưu
                *dirty_during_save = true;
            }
            DocumentState::SaveFailed { dirty, .. } => {
                *dirty = true;
            }
            _ => {}
        }
    }

    /// Được gọi khi nhận được yêu cầu lưu tệp kèm mảng bytes từ iframe.
    pub fn on_save_requested(
        &mut self,
        generation: u64,
        doc_id: &str,
        request_id: &str,
        bytes: Vec<u8>,
        now_ms: u64,
    ) -> CoordinatorAction {
        // Bất biến 6: Thế hệ iframe không khớp -> loại bỏ
        if generation != self.session_generation {
            return CoordinatorAction::None;
        }

        // Bất biến 5: Khác document ID -> loại bỏ
        if self.active_document_id.as_deref() != Some(doc_id) {
            return CoordinatorAction::None;
        }

        let path = match &self.dest_path {
            Some(p) => p.clone(),
            None => "untitled.xlsx".to_string(),
        };

        // Băm nội dung snapshot xuất ra bằng thuật toán FNV-1a nhất quán
        let fingerprint = compute_content_fingerprint(&bytes);

        // Bất biến 7: Nếu đang lưu dở một request khác, giữ cờ dirty_during_save
        let dirty_flag = match &self.state {
            DocumentState::Saving { .. } => true,
            _ => false,
        };

        self.state = DocumentState::Saving {
            request_id: request_id.to_string(),
            document_id: doc_id.to_string(),
            snapshot_fingerprint: fingerprint,
            dest_path: path.clone(),
            started_at_ms: now_ms,
            dirty_during_save: dirty_flag,
            snapshot_edit_revision: self.edit_revision,
        };

        CoordinatorAction::PerformDurableWrite {
            request_id: request_id.to_string(),
            dest_path: path,
            bytes,
        }
    }

    /// Được gọi khi backend báo cáo việc ghi đĩa hoàn tất bền vững.
    pub fn on_durable_write_completed(&mut self, request_id: &str, written_fingerprint: &str) {
        if let DocumentState::Saving {
            request_id: cur_req,
            snapshot_fingerprint,
            dirty_during_save,
            snapshot_edit_revision,
            ..
        } = &self.state
        {
            // Bất biến 4: Chỉ acknowledge đúng request_id đang chờ
            if cur_req == request_id && snapshot_fingerprint == written_fingerprint {
                self.last_persisted_fingerprint = Some(written_fingerprint.to_string());
                if *dirty_during_save || self.edit_revision > *snapshot_edit_revision {
                    // Bất biến 3: Có chỉnh sửa xen ngang lúc lưu (hoặc revision live > snapshot) -> vẫn giữ dirty!
                    self.state = DocumentState::Dirty;
                } else {
                    // Bất biến 1: Chỉ chuyển Clean khi đĩa đã ghi thành công
                    self.state = DocumentState::Clean;
                }
            }
        }
    }

    /// Được gọi khi backend báo cáo việc ghi đĩa thất bại (I/O error, Permission Denied...).
    pub fn on_durable_write_failed(
        &mut self,
        request_id: &str,
        error: impl Into<String>,
    ) -> CoordinatorAction {
        if let DocumentState::Saving {
            request_id: cur_req,
            dest_path,
            ..
        } = &self.state
        {
            if cur_req == request_id {
                let err_msg = error.into();
                let path = dest_path.clone();
                // Bất biến 2: Lưu hỏng tuyệt đối không chuyển clean, ghi nhận SaveFailed
                self.state = DocumentState::SaveFailed {
                    error: err_msg.clone(),
                    dest_path: path.clone(),
                    dirty: true,
                };
                return CoordinatorAction::ShowSaveError {
                    error: err_msg,
                    dest_path: path,
                };
            }
        }
        CoordinatorAction::None
    }

    /// Được gọi khi một yêu cầu lưu bị hủy bỏ (ví dụ: người dùng hủy tiến trình lưu).
    pub fn on_save_cancelled(&mut self, request_id: &str) {
        if let DocumentState::Saving {
            request_id: cur_req,
            ..
        } = &self.state
        {
            if cur_req == request_id {
                // Trả về trạng thái Dirty để bảo vệ dữ liệu chưa lưu
                self.state = DocumentState::Dirty;
            }
        }
    }

    /// Được gọi khi thao tác ghi đĩa bị quá thời gian chờ (Timeout).
    pub fn on_save_timeout(&mut self, request_id: &str) -> CoordinatorAction {
        if let DocumentState::Saving {
            request_id: cur_req,
            dest_path,
            ..
        } = &self.state
        {
            if cur_req == request_id {
                let path = dest_path.clone();
                let err = format!("Quá thời gian chờ phản hồi ghi đĩa cho request {request_id}");
                self.state = DocumentState::SaveFailed {
                    error: err.clone(),
                    dest_path: path.clone(),
                    dirty: true,
                };
                return CoordinatorAction::ShowSaveError {
                    error: err,
                    dest_path: path,
                };
            }
        }
        CoordinatorAction::None
    }

    /// Được gọi khi người dùng muốn điều hướng rời khỏi micro-app hoặc đóng cửa sổ.
    pub fn on_navigation_requested(&mut self, target: impl Into<String>) -> CoordinatorAction {
        let tgt = target.into();
        if self.is_dirty() {
            let path = self
                .dest_path
                .clone()
                .unwrap_or_else(|| "Tài liệu chưa đặt tên".to_string());
            self.state = DocumentState::ClosingPending {
                target_navigation: tgt.clone(),
            };
            CoordinatorAction::PromptUnsavedChanges {
                dest_path: path,
                target_navigation: tgt,
            }
        } else {
            CoordinatorAction::ProceedNavigation(tgt)
        }
    }

    /// Xử lý quyết định của người dùng từ hộp thoại xác nhận Unsaved Changes.
    pub fn on_closing_dialog_decision(
        &mut self,
        decision: UserClosingDecision,
    ) -> CoordinatorAction {
        if let DocumentState::ClosingPending { target_navigation } = &self.state {
            let nav = target_navigation.clone();
            match decision {
                UserClosingDecision::Cancel => {
                    // Trở về trạng thái Dirty, không chuyển trang
                    self.state = DocumentState::Dirty;
                    CoordinatorAction::None
                }
                UserClosingDecision::Discard => {
                    // Bất biến 9: Người dùng xác nhận bỏ thay đổi tường minh
                    self.state = DocumentState::Clean;
                    CoordinatorAction::ProceedNavigation(nav)
                }
                UserClosingDecision::Save => {
                    // Giữ nguyên ClosingPending và chờ Save thực hiện
                    CoordinatorAction::None
                }
            }
        } else {
            CoordinatorAction::None
        }
    }
}

/// Tính mã băm nội dung nhanh và nhất quán (64-bit FNV-1a hex).
pub fn compute_content_fingerprint(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// Hàm băm nội dung đơn giản để tính fingerprint độc lập (FNV-1a 64-bit).
pub fn md5_or_simple_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
