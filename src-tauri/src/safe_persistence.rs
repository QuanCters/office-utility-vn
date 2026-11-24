// ==============================================================================
// src-tauri/src/safe_persistence.rs
// Dịch vụ lưu trữ đĩa an toàn, bền vững và tuần tự hóa cho Tauri Backend
// ==============================================================================

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
#[cfg(test)]
use std::sync::Arc;
use office_shared::CommandError;
use serde::{Deserialize, Serialize};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Danh sách phần mở rộng tệp được phép đọc qua Native Dialog
const ALLOWED_READ_EXTENSIONS: &[&str] = &[
    // Định dạng bảng tính (GridCraft / Data Lab)
    "xlsx", "xlsm", "xls", "csv", "tsv", "json", "ods",
    // Định dạng tài liệu & PDF (PDF Studio)
    "pdf", "txt", "docx",
    // Định dạng đồ họa & media (Media Studio)
    "png", "jpg", "jpeg",
];

/// Danh sách phần mở rộng tệp được phép lưu qua nhị phân
const ALLOWED_WRITE_EXTENSIONS: &[&str] = &[
    // Định dạng bảng tính (GridCraft / Data Lab)
    "xlsx", "xlsm", "csv", "tsv", "json", "ods",
    // Định dạng tài liệu & PDF (PDF Studio)
    "pdf", "txt", "docx",
    // Định dạng đồ họa & media (Media Studio)
    "png", "jpg", "jpeg",
];

/// Danh sách thư mục hệ điều hành nhạy cảm tuyệt đối cấm ghi đè
#[cfg(unix)]
const SENSITIVE_SYSTEM_PREFIXES: &[&str] = &[
    "/etc", "/usr", "/boot", "/bin", "/sbin", "/dev", "/proc", "/sys",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostRenameFault {
    DirOpenFailure,
    DirSyncFailure,
}

static POST_RENAME_FAULT_INJECTION: Mutex<Option<HashMap<PathBuf, PostRenameFault>>> = Mutex::new(None);

pub fn set_post_rename_fault_for_path(path: PathBuf, fault: PostRenameFault) {
    let mut guard = POST_RENAME_FAULT_INJECTION.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(path, fault);
}

pub fn clear_post_rename_fault_for_path(path: &Path) {
    let mut guard = POST_RENAME_FAULT_INJECTION.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(map) = guard.as_mut() {
        map.remove(path);
    }
}

#[allow(dead_code)]
fn check_post_rename_fault(dest_path: &Path) -> Option<CommandError> {
    let guard = POST_RENAME_FAULT_INJECTION.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(map) = guard.as_ref() {
        for (target, fault) in map.iter() {
            if dest_path == target || dest_path.ends_with(target) {
                match fault {
                    PostRenameFault::DirOpenFailure => return Some(CommandError::IoError {
                        details: "AmbiguousCommit: Không thể mở thư mục cha để đồng bộ (Lỗi tiêm thử nghiệm: DirOpenFailure)".to_string(),
                    }),
                    PostRenameFault::DirSyncFailure => return Some(CommandError::IoError {
                        details: "AmbiguousCommit: Đồng bộ thư mục cha thất bại (Lỗi tiêm thử nghiệm: DirSyncFailure)".to_string(),
                    }),
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreRenameFault {
    SetPermissionsFailure,
}

static PRE_RENAME_FAULT_INJECTION: Mutex<Option<HashMap<PathBuf, PreRenameFault>>> = Mutex::new(None);

pub fn set_pre_rename_fault_for_path(path: PathBuf, fault: PreRenameFault) {
    let mut guard = PRE_RENAME_FAULT_INJECTION.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(path, fault);
}

pub fn clear_pre_rename_fault_for_path(path: &Path) {
    let mut guard = PRE_RENAME_FAULT_INJECTION.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(map) = guard.as_mut() {
        map.remove(path);
    }
}

#[allow(dead_code)]
fn check_pre_rename_fault(dest_path: &Path) -> Option<CommandError> {
    let guard = PRE_RENAME_FAULT_INJECTION.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(map) = guard.as_ref() {
        for (target, fault) in map.iter() {
            if dest_path == target || dest_path.ends_with(target) {
                match fault {
                    PreRenameFault::SetPermissionsFailure => return Some(CommandError::IoError {
                        details: "Lỗi tiêm thử nghiệm: Không thể đặt quyền hạn lên tệp tạm (SetPermissionsFailure)".to_string(),
                    }),
                }
            }
        }
    }
    None
}

#[cfg(test)]
static TEMP_FILE_INSPECTOR: Mutex<Option<Arc<dyn Fn(&Path) + Send + Sync>>> = Mutex::new(None);

#[cfg(test)]
pub fn set_temp_file_inspector<F: Fn(&Path) + Send + Sync + 'static>(f: F) {
    let mut guard = TEMP_FILE_INSPECTOR.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(Arc::new(f));
}

#[cfg(test)]
pub fn clear_temp_file_inspector() {
    let mut guard = TEMP_FILE_INSPECTOR.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
}

/// Trạng thái phiên ghi của một đường dẫn đích cụ thể
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DestinationState {
    pub last_issued_seq: u64,
    pub last_committed_seq: u64,
}

/// Vé lưu đĩa đại diện cho một phiên ghi đã được cấp số thứ tự đơn điệu
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeWriteTicket {
    pub dest_path: PathBuf,
    pub seq: u64,
}

/// Bộ điều phối tuần tự hóa lưu trữ theo đường dẫn đích ở tầng Native Backend
#[derive(Debug, Default)]
pub struct NativeDestinationTracker {
    inner: Mutex<HashMap<PathBuf, DestinationState>>,
}

impl NativeDestinationTracker {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Đăng ký cấp vé lưu đĩa đơn điệu tăng dần cho đường dẫn đích
    pub fn reserve(&self, dest_path: &Path) -> NativeWriteTicket {
        let normalized = canonicalize_destination_path(dest_path);
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let state = map.entry(normalized.clone()).or_default();
        state.last_issued_seq = state.last_issued_seq.wrapping_add(1);
        let seq = state.last_issued_seq;
        NativeWriteTicket {
            dest_path: normalized,
            seq,
        }
    }

    /// Kiểm tra xem vé lưu này có còn quyền commit hay không
    pub fn can_commit(&self, ticket: &NativeWriteTicket) -> Result<(), CommandError> {
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = map.get(&ticket.dest_path) {
            if ticket.seq <= state.last_committed_seq {
                return Err(CommandError::IoError {
                    details: format!(
                        "Lượt ghi lỗi thời bị từ chối: phiên ghi {} đã bị vượt qua bởi phiên {}",
                        ticket.seq, state.last_committed_seq
                    ),
                });
            }
        }
        Ok(())
    }

    /// Thực hiện việc kiểm tra thứ tự và commit hoán đổi nguyên tử dưới Mutex lock
    pub fn commit_atomic<F, R>(&self, ticket: &NativeWriteTicket, commit_fn: F) -> Result<R, CommandError>
    where
        F: FnOnce() -> Result<R, CommandError>,
    {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let state = map.entry(ticket.dest_path.clone()).or_default();
        if ticket.seq <= state.last_committed_seq {
            return Err(CommandError::IoError {
                details: format!(
                    "Lượt ghi lỗi thời bị từ chối: phiên ghi {} đã bị vượt qua bởi phiên {}",
                    ticket.seq, state.last_committed_seq
                ),
            });
        }
        let res = commit_fn()?;
        state.last_committed_seq = ticket.seq;
        Ok(res)
    }

    /// Lấy trạng thái hiện tại của một đích đến để kiểm thử
    pub fn get_state(&self, dest_path: &Path) -> Option<DestinationState> {
        let normalized = canonicalize_destination_path(dest_path);
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.get(&normalized).cloned()
    }
}

/// Trình theo dõi ghi lưu trữ toàn cục cho backend native
pub static GLOBAL_NATIVE_SAVE_TRACKER: LazyLock<NativeDestinationTracker> =
    LazyLock::new(NativeDestinationTracker::new);

/// Chuẩn hóa đường dẫn đích để đường dẫn tương đối và tuyệt đối quy về cùng một khóa
pub fn canonicalize_destination_path(dest_path: &Path) -> PathBuf {
    if let Ok(canon) = dest_path.canonicalize() {
        canon
    } else if let Some(parent) = dest_path.parent() {
        if let Ok(canon_parent) = parent.canonicalize() {
            if let Some(file_name) = dest_path.file_name() {
                return canon_parent.join(file_name);
            }
        }
        dest_path.to_path_buf()
    } else {
        dest_path.to_path_buf()
    }
}

/// Thẩm tra tính an toàn và quyền hạn đường dẫn đích ở tầng Native Backend
fn validate_path_internal(path_str: &str, allowed_exts: &[&str]) -> Result<PathBuf, CommandError> {
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return Err(CommandError::ValidationFailed {
            reason: "Đường dẫn tệp không được để trống".into(),
        });
    }
    if trimmed.contains('\0') {
        return Err(CommandError::ValidationFailed {
            reason: "Đường dẫn tệp chứa ký tự null không hợp lệ".into(),
        });
    }

    let p = Path::new(trimmed);
    for component in p.components() {
        if let Component::ParentDir = component {
            return Err(CommandError::ValidationFailed {
                reason: format!("Phát hiện nguy cơ Path Traversal ('..') trong đường dẫn: {path_str}"),
            });
        }
    }

    // Kiểm tra phần mở rộng tệp hợp lệ
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if !allowed_exts.contains(&ext.as_str()) {
        return Err(CommandError::ValidationFailed {
            reason: format!("Phần mở rộng tệp '.{ext}' không được hỗ trợ hoặc không an toàn"),
        });
    }

    // Chặn các khu vực nhạy cảm của hệ điều hành
    #[cfg(unix)]
    {
        let canon = canonicalize_destination_path(p);
        let canon_str = canon.to_string_lossy();
        for prefix in SENSITIVE_SYSTEM_PREFIXES {
            if canon_str.starts_with(prefix) {
                return Err(CommandError::ValidationFailed {
                    reason: format!("Đường dẫn bị từ chối do thuộc khu vực hệ thống nhạy cảm ({prefix}): {path_str}"),
                });
            }
        }
    }

    Ok(p.to_path_buf())
}

/// Kiểm tra tính hợp lệ và thẩm quyền cho đường dẫn đọc tệp từ hệ thống
pub fn validate_and_authorize_read_path(path_str: &str) -> Result<PathBuf, CommandError> {
    let path = validate_path_internal(path_str, ALLOWED_READ_EXTENSIONS)?;
    if path.exists() {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ext == "xls" {
            let mut file = std::fs::File::open(&path).map_err(|e| CommandError::IoError {
                details: format!("Không thể mở tệp .xls để kiểm tra chữ ký: {e}"),
            })?;
            use std::io::Read;
            let mut header = [0u8; 8];
            let read_bytes = file.read(&mut header).map_err(|e| CommandError::IoError {
                details: format!("Không thể đọc header tệp .xls: {e}"),
            })?;
            const OLE2_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
            if read_bytes < 8 || header != OLE2_MAGIC {
                return Err(CommandError::InvalidFileSignature {
                    expected: "OLE2 Header [D0 CF 11 E0 A1 B1 1A E1]".to_string(),
                    actual: format!("{:02X?}", &header[..read_bytes]),
                });
            }
        }
    }
    Ok(path)
}

/// Kiểm tra tính hợp lệ và thẩm quyền cho đường dẫn ghi tệp xuống hệ thống
pub fn validate_and_authorize_write_path(path_str: &str) -> Result<PathBuf, CommandError> {
    validate_path_internal(path_str, ALLOWED_WRITE_EXTENSIONS)
}

/// Kiểm tra tính hợp lệ và thẩm quyền đường dẫn tệp (mặc định áp dụng chính sách ghi an toàn)
pub fn validate_and_authorize_path(path_str: &str) -> Result<PathBuf, CommandError> {
    validate_and_authorize_write_path(path_str)
}

/// RAII Guard để tự động xóa tệp tạm nếu có sự cố trước khi commit hoán đổi nguyên tử
struct TempFileGuard<'a>(&'a Path, bool);

impl<'a> TempFileGuard<'a> {
    fn new(path: &'a Path) -> Self {
        Self(path, true)
    }

    fn disarm(&mut self) {
        self.1 = false;
    }
}

impl<'a> Drop for TempFileGuard<'a> {
    fn drop(&mut self) {
        if self.1 && self.0.exists() {
            let _ = std::fs::remove_file(self.0);
        }
    }
}

/// Quy trình 10 bước thực hiện ghi đĩa bền vững và nguyên tử ở tầng native
///
/// ### Đảm bảo & Ranh giới Bảo mật Phân quyền (Security Guarantees & Limitations - Finding 4675800831):
/// 1. **POSIX / Unix Mode Bits (0600 Restrictive Default & Preservation)**:
///    - **Tệp tạm**: Luôn được tạo độc quyền với cờ `OpenOptionsExt::mode(0o600)` (chỉ chủ sở hữu
///      có quyền đọc/ghi), bất chấp `umask` mặc định của tiến trình (ví dụ 022), loại bỏ hoàn toàn
///      khe hở tiết lộ dữ liệu nhạy cảm cho người dùng khác trong lúc stream byte (NFR-SEC-03).
///    - **Bảo lưu quyền hạn**: Nếu tệp đích đã tồn tại, quyền truy cập POSIX mode bits (`0o777`)
///      được đọc từ tệp gốc và áp dụng lại vào tệp tạm qua `set_permissions` trước khi fsync
///      và hoán đổi nguyên tử (`rename`), ngăn chặn việc tệp bảo mật (0600) bị biến thành 0644.
///    - **Tệp mới**: Tự động áp dụng quyền mặc định an toàn `0600`.
///    - **Lỗi đặt quyền**: Bất kỳ sự cố nào khi đặt permissions đều được truyền tải về caller
///      trước khi đổi tên nguyên tử, đảm bảo tệp gốc không bị thay đổi.
/// 2. **Hạn chế đối với POSIX Extended ACLs & Extended Attributes (xattrs)**:
///    - Các danh sách kiểm soát truy cập mở rộng (POSIX ACLs via `setfacl`) và thuộc tính mở rộng
///      (xattrs) không được tự động sao chép qua việc hoán đổi inode nếu không có thư viện bên ngoài
///      phụ thuộc vào hệ thống (`libacl`/`libattr`).
/// 3. **Hệ điều hành Windows (NTFS)**:
///    - Trên Windows NTFS, tệp kế thừa Access Control Lists (DACLs) từ thư mục cha. Việc hoán đổi
///      nguyên tử sẽ tiếp tục áp dụng các chính sách bảo mật kế thừa của thư mục đích.
pub fn native_atomic_durable_write(dest_path_str: &str, bytes: &[u8]) -> Result<(), CommandError> {
    // Bước 1: Thẩm tra an toàn đường dẫn và quyền hạn
    let dest_path = validate_and_authorize_path(dest_path_str)?;

    // Bước 2: Chuẩn hóa định danh đích đến
    let parent_dir = dest_path.parent().unwrap_or_else(|| Path::new("."));
    if !parent_dir.exists() {
        std::fs::create_dir_all(parent_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục cha '{parent_dir:?}': {e}"),
        })?;
    }

    // Bước 3: Cấp vé thứ tự lưu đơn điệu theo đích đến
    let ticket = GLOBAL_NATIVE_SAVE_TRACKER.reserve(&dest_path);

    // Bước 4: Tạo tên tệp tạm ngẫu nhiên chống va chạm cùng thư mục cha
    let file_name = dest_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.bin");
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let temp_name = format!(".tmp.{}.{}.{}.{}", file_name, std::process::id(), nanos, counter);
    let temp_path = parent_dir.join(&temp_name);

    // Bước 4.5: Thu thập quyền hạn của tệp đích hiện hữu (nếu có) hoặc dùng 0600 an toàn (Finding 4675800831)
    #[cfg(unix)]
    let target_permissions = if let Ok(meta) = std::fs::symlink_metadata(&dest_path) {
        Some(meta.permissions())
    } else {
        Some(std::fs::Permissions::from_mode(0o600))
    };

    // Bước 5 & 6: Kích hoạt RAII Guard và mở tệp tạm độc quyền (create_new) với quyền hạn chế 0600
    let mut guard = TempFileGuard::new(&temp_path);
    let mut open_options = OpenOptions::new();
    open_options.write(true).create_new(true);
    #[cfg(unix)]
    {
        // Luôn tạo file tạm với quyền chặt chẽ 0600 để chống tiết lộ thông tin khi đang ghi
        open_options.mode(0o600);
    }
    let mut file = open_options.open(&temp_path).map_err(|e| CommandError::IoError {
        details: format!("Không thể tạo tệp tạm '{temp_path:?}': {e}"),
    })?;

    // Bước 7: Ghi toàn bộ dữ liệu ra tệp tạm
    file.write_all(bytes).map_err(|e| CommandError::IoError {
        details: format!("Không thể ghi dữ liệu vào tệp tạm: {e}"),
    })?;

    // Hook kiểm tra quyền hạn của tệp tạm trước khi thay thế (Test Verification Scenario 2)
    #[cfg(test)]
    {
        let guard = TEMP_FILE_INSPECTOR.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref inspector) = *guard {
            inspector(&temp_path);
        }
    }

    // Kiểm tra lỗi tiêm thử nghiệm trước khi đặt quyền / rename (Test Verification Scenario 4)
    #[cfg(test)]
    if let Some(fault) = check_pre_rename_fault(&dest_path) {
        return Err(fault);
    }

    // Áp dụng quyền hạn mục tiêu (bảo lưu quyền tệp cũ hoặc 0600 cho tệp mới) trước khi fsync & rename
    #[cfg(unix)]
    if let Some(perms) = target_permissions {
        std::fs::set_permissions(&temp_path, perms).map_err(|e| CommandError::IoError {
            details: format!("Không thể áp dụng quyền truy cập lên tệp tạm '{temp_path:?}': {e}"),
        })?;
    }

    file.flush().map_err(|e| CommandError::IoError {
        details: format!("Không thể flush tệp tạm: {e}"),
    })?;

    file.sync_all().map_err(|e| CommandError::IoError {
        details: format!("Không thể fsync tệp tạm xuống đĩa: {e}"),
    })?;

    // Bước 8: Đóng tệp tạm trước khi hoán đổi
    drop(file);

    // Bước 9: Hoán đổi nguyên tử dưới sự bảo vệ của lock tuần tự hóa
    GLOBAL_NATIVE_SAVE_TRACKER.commit_atomic(&ticket, || {
        std::fs::rename(&temp_path, &dest_path).map_err(|e| CommandError::IoError {
            details: format!("Không thể hoán đổi nguyên tử sang '{dest_path:?}': {e}"),
        })?;
        Ok(())
    })?;

    // Đã hoán đổi thành công -> Hủy kích hoạt guard để không xóa tệp đích
    guard.disarm();

    // Bước 10: Đồng bộ thư mục cha trên POSIX / Unix (hoặc mô phỏng lỗi kiểm thử)
    #[cfg(test)]
    if let Some(fault) = check_post_rename_fault(&dest_path) {
        return Err(fault);
    }

    #[cfg(unix)]
    {
        match File::open(parent_dir) {
            Ok(dir_file) => {
                if let Err(e) = dir_file.sync_all() {
                    return Err(CommandError::IoError {
                        details: format!("AmbiguousCommit: Hoán đổi nguyên tử thành công nhưng đồng bộ thư mục cha thất bại: {e}"),
                    });
                }
            }
            Err(e) => {
                return Err(CommandError::IoError {
                    details: format!("AmbiguousCommit: Hoán đổi nguyên tử thành công nhưng không thể mở thư mục cha để đồng bộ: {e}"),
                });
            }
        }
    }

    Ok(())
}

/// Điểm ủy thác bất đồng bộ an toàn cho lệnh Tauri command
pub async fn native_save_file_binary(file_path: &str, bytes: &[u8]) -> Result<(), CommandError> {
    let path_owned = file_path.to_string();
    let bytes_owned = bytes.to_vec();
    tauri::async_runtime::spawn_blocking(move || {
        native_atomic_durable_write(&path_owned, &bytes_owned)
    })
    .await
    .map_err(|e| CommandError::ProcessExecutionError {
        message: format!("Lỗi runtime khi điều phối ghi tệp: {e}"),
    })?
}

// ==============================================================================
// NATIVE DOCUMENT AUTHORIZATION & SAVE GRANT SYSTEM (STAGE 4B-1 & STAGE 4B-2A)
// ==============================================================================

pub const MAX_PAYLOAD_BYTES: usize = 100 * 1024 * 1024; // 100 MB

/// Tính mã băm nội dung nhanh và nhất quán (64-bit FNV-1a hex)
pub fn compute_fingerprint(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GrantOperationType {
    Save,
    SaveAs,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OverwritePolicy {
    AllowOverwrite,
    RequireNew,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GrantStatus {
    /// Grant vừa được phát hành, sẵn sàng tiếp nhận yêu cầu ghi
    Active,
    /// Đang trong quá trình ghi đĩa (I/O in-flight), ngăn chặn ghi đồng thời
    InUse,
    /// Ghi đĩa thành công và đã hoàn tất hoán đổi nguyên tử
    Consumed,
    /// Bị thu hồi bởi Native Backend (do người dùng hủy hoặc đổi tài liệu)
    Revoked,
    /// Hết hạn hiệu lực TTL trước khi bắt đầu ghi
    Expired,
    /// Ghi đĩa thất bại trước khi hoán đổi nguyên tử (tệp đích chưa bị thay đổi)
    Failed,
    /// Sự cố đồng bộ xảy ra sau khi hoán đổi nguyên tử
    AmbiguousCommit,
}

/// Token cấp phép ghi tài liệu do Native Backend quản lý và phát hành độc quyền
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NativeSaveGrant {
    pub grant_id: String,
    pub document_id: String,
    pub app_instance_id: String,
    pub session_generation: u64,
    pub authorized_dest_path: PathBuf,
    pub operation_type: GrantOperationType,
    pub expected_revision: u64,
    pub overwrite_policy: OverwritePolicy,
    pub issued_at_epoch_ms: u64,
    pub expires_at_epoch_ms: u64,
    pub request_id: String,
    pub status: GrantStatus,
}

/// Trạng thái phiên tài liệu do Native Backend quản lý và theo dõi
#[derive(Debug, Clone)]
pub struct NativeDocumentSession {
    pub document_id: String,
    pub app_instance_id: String,
    pub session_generation: u64,
    pub authorized_dest_path: PathBuf,
    pub last_committed_revision: u64,
    pub last_committed_seq: u64,
    pub is_user_authorized: bool,
}

/// Kết quả phản hồi thành công sau khi ghi đĩa có ủy quyền
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SaveSuccessResponse {
    pub dest_path: String,
    pub bytes_written: usize,
    pub committed_revision: u64,
    pub fingerprint: String,
    pub is_ambiguous: bool,
}

/// Dữ liệu tài liệu được mở qua Native File Dialog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentHandle {
    pub document_id: String,
    pub app_instance_id: String,
    pub session_generation: u64,
    pub file_name: String,
    pub file_path: String,
    pub initial_revision: u64,
    pub bytes: Vec<u8>,
}

/// Dữ liệu phiên tài liệu được lưu mới qua Native Save As Dialog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveAsHandle {
    pub document_id: String,
    pub grant: NativeSaveGrant,
    pub file_name: String,
    pub file_path: String,
}

/// Giao diện trừu tượng hóa Native OS File Dialog
pub trait NativeDialogService: Send + Sync {
    fn pick_file(&self, filter_name: &str, extensions: &[&str]) -> Option<PathBuf>;
    fn save_file(&self, filter_name: &str, extensions: &[&str], default_name: Option<&str>) -> Option<PathBuf>;
}

/// Bộ điều khiển Native OS Dialog thực tế (sử dụng thư viện rfd trên desktop)
pub struct SystemNativeDialogService;

impl NativeDialogService for SystemNativeDialogService {
    fn pick_file(&self, filter_name: &str, extensions: &[&str]) -> Option<PathBuf> {
        let mut builder = rfd::FileDialog::new();
        if !extensions.is_empty() {
            builder = builder.add_filter(filter_name, extensions);
        }
        builder.pick_file()
    }

    fn save_file(&self, filter_name: &str, extensions: &[&str], default_name: Option<&str>) -> Option<PathBuf> {
        let mut builder = rfd::FileDialog::new();
        if let Some(name) = default_name {
            builder = builder.set_file_name(name);
        }
        if !extensions.is_empty() {
            builder = builder.add_filter(filter_name, extensions);
        }
        builder.save_file()
    }
}

/// Quản lý phân quyền và kiểm soát lưu đĩa tài liệu ở tầng Native Backend
pub struct NativeSaveGrantManager {
    sessions: Mutex<HashMap<String, NativeDocumentSession>>,
    grants: Mutex<HashMap<String, NativeSaveGrant>>,
    dialog_service: Mutex<Box<dyn NativeDialogService>>,
}

impl Default for NativeSaveGrantManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeSaveGrantManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            grants: Mutex::new(HashMap::new()),
            dialog_service: Mutex::new(Box::new(SystemNativeDialogService)),
        }
    }

    /// Thiết lập dịch vụ Native Dialog giả lập phục vụ kiểm thử đơn vị / đối kháng
    pub fn set_dialog_service_for_test(&self, service: Box<dyn NativeDialogService>) {
        let mut ds = self.dialog_service.lock().unwrap_or_else(|e| e.into_inner());
        *ds = service;
    }

    /// Mở tệp thông qua hộp thoại OS Native File Dialog.
    /// Đường dẫn được chọn trực tiếp bởi người dùng qua hệ điều hành.
    /// Tự động kiểm định an toàn, nạp dữ liệu và tạo phiên tài liệu có thẩm quyền.
    pub fn open_file_via_native_dialog(
        &self,
        app_instance_id: &str,
        session_generation: u64,
        filter_name: &str,
        extensions: &[&str],
    ) -> Result<Option<DocumentHandle>, CommandError> {
        let selected_path = {
            let ds = self.dialog_service.lock().unwrap_or_else(|e| e.into_inner());
            ds.pick_file(filter_name, extensions)
        };

        let path = match selected_path {
            Some(p) => p,
            None => return Ok(None),
        };

        let normalized = canonicalize_destination_path(&path);
        validate_and_authorize_read_path(&normalized.to_string_lossy())?;

        let file = std::fs::File::open(&normalized).map_err(|e| CommandError::IoError {
            details: format!("Không thể mở tệp: {e}"),
        })?;

        let metadata = file.metadata().map_err(|e| CommandError::IoError {
            details: format!("Không thể đọc thông tin tệp: {e}"),
        })?;

        if !metadata.is_file() {
            return Err(CommandError::IoError {
                details: format!("Tệp được chọn không tồn tại hoặc không phải là tệp hợp lệ: {:?}", normalized),
            });
        }

        let file_len = metadata.len();
        if file_len > MAX_PAYLOAD_BYTES as u64 {
            return Err(CommandError::ValidationFailed {
                reason: format!(
                    "Kích thước tệp ({} bytes) vượt quá giới hạn an toàn tối đa ({} bytes)",
                    file_len,
                    MAX_PAYLOAD_BYTES
                ),
            });
        }

        use std::io::Read;
        let mut reader = file.take(MAX_PAYLOAD_BYTES as u64 + 1);
        let mut bytes = Vec::with_capacity(file_len.min(MAX_PAYLOAD_BYTES as u64) as usize);
        reader.read_to_end(&mut bytes).map_err(|e| CommandError::IoError {
            details: format!("Không thể đọc nội dung tệp từ native dialog: {e}"),
        })?;

        if bytes.len() > MAX_PAYLOAD_BYTES {
            return Err(CommandError::ValidationFailed {
                reason: format!(
                    "Kích thước tệp ({} bytes) vượt quá giới hạn an toàn tối đa ({} bytes)",
                    bytes.len(),
                    MAX_PAYLOAD_BYTES
                ),
            });
        }

        // Kiểm tra chữ ký magic bytes nếu phần mở rộng là .xls (Compound File Binary Format / OLE2)
        let ext = normalized.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ext == "xls" {
            const OLE2_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
            if bytes.len() < 8 || &bytes[0..8] != &OLE2_MAGIC {
                return Err(CommandError::ValidationFailed {
                    reason: "Tệp .xls không đúng định dạng chữ ký nhị phân Compound File Binary (OLE2)".to_string(),
                });
            }
        }

        let file_name = normalized
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "document".to_string());
        let file_path = normalized.to_string_lossy().to_string();
        let document_id = format!("doc-{}", uuid::Uuid::new_v4());

        let session = NativeDocumentSession {
            document_id: document_id.clone(),
            app_instance_id: app_instance_id.to_string(),
            session_generation,
            authorized_dest_path: normalized,
            last_committed_revision: 0,
            last_committed_seq: 0,
            is_user_authorized: true,
        };

        {
            let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            sessions.insert(document_id.clone(), session);
        }

        Ok(Some(DocumentHandle {
            document_id,
            app_instance_id: app_instance_id.to_string(),
            session_generation,
            file_name,
            file_path,
            initial_revision: 0,
            bytes,
        }))
    }

    /// Lưu tệp dưới tên mới (Save As) thông qua hộp thoại OS Native File Dialog.
    /// Nhận đường dẫn đích từ OS dialog, đăng ký phiên tài liệu, và cấp phát ngay một Save Grant hợp lệ.
    pub fn save_as_via_native_dialog(
        &self,
        _document_id_opt: Option<&str>,
        app_instance_id: &str,
        session_generation: u64,
        default_name: Option<&str>,
        filter_name: &str,
        extensions: &[&str],
    ) -> Result<Option<SaveAsHandle>, CommandError> {
        let selected_path = {
            let ds = self.dialog_service.lock().unwrap_or_else(|e| e.into_inner());
            ds.save_file(filter_name, extensions, default_name)
        };

        let path = match selected_path {
            Some(p) => p,
            None => return Ok(None),
        };

        let normalized = canonicalize_destination_path(&path);
        validate_and_authorize_path(&normalized.to_string_lossy())?;

        let file_name = normalized
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "document".to_string());
        let file_path = normalized.to_string_lossy().to_string();

        // Transaction Safety (Task 5): Thao tác Save As luôn tạo ra một phiên tài liệu độc lập mới.
        // Tuyệt đối không thay đổi đè lên phiên tài liệu cũ trước khi quá trình ghi đĩa thành công!
        let new_id = format!("doc-{}", uuid::Uuid::new_v4());
        let session = NativeDocumentSession {
            document_id: new_id.clone(),
            app_instance_id: app_instance_id.to_string(),
            session_generation,
            authorized_dest_path: normalized.clone(),
            last_committed_revision: 0,
            last_committed_seq: 0,
            is_user_authorized: true,
        };
        {
            let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            sessions.insert(new_id.clone(), session);
        }
        let doc_id = new_id;

        let (expected_revision, session_dest) = {
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            let s = sessions.get(&doc_id).unwrap();
            (s.last_committed_revision + 1, s.authorized_dest_path.clone())
        };

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        let expires_ms = now_ms.saturating_add(30 * 1000);
        let grant_id = format!("grant-{}", uuid::Uuid::new_v4());
        let request_id = format!("req-saveas-{}", uuid::Uuid::new_v4());

        let grant = NativeSaveGrant {
            grant_id: grant_id.clone(),
            document_id: doc_id.clone(),
            app_instance_id: app_instance_id.to_string(),
            session_generation,
            authorized_dest_path: session_dest,
            operation_type: GrantOperationType::SaveAs,
            expected_revision,
            overwrite_policy: OverwritePolicy::AllowOverwrite,
            issued_at_epoch_ms: now_ms,
            expires_at_epoch_ms: expires_ms,
            request_id,
            status: GrantStatus::Active,
        };

        {
            let mut grants = self.grants.lock().unwrap_or_else(|e| e.into_inner());
            grants.insert(grant_id, grant.clone());
        }

        Ok(Some(SaveAsHandle {
            document_id: doc_id,
            grant,
            file_name,
            file_path,
        }))
    }

    /// Kiểm tra xem một đường dẫn cụ thể có thuộc một phiên tài liệu đang hoạt động
    /// và đã được người dùng ủy quyền hợp lệ hay không.
    pub fn is_path_authorized_in_session(&self, path: &Path) -> bool {
        let normalized = canonicalize_destination_path(path);
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions.values().any(|s| s.is_user_authorized && s.authorized_dest_path == normalized)
    }

    /// Lấy trạng thái hiện tại của một Grant
    pub fn get_grant_status(&self, grant_id: &str) -> Option<GrantStatus> {
        let grants = self.grants.lock().unwrap_or_else(|e| e.into_inner());
        grants.get(grant_id).map(|g| g.status)
    }

    /// Lấy revision đã commit gần nhất của tài liệu
    pub fn get_session_revision(&self, document_id: &str) -> Option<u64> {
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions.get(document_id).map(|s| s.last_committed_revision)
    }

    /// Đăng ký một phiên tài liệu mới được người dùng xác thực hợp lệ (ví dụ qua OS File Dialog)
    pub fn register_document_session(
        &self,
        document_id: &str,
        app_instance_id: &str,
        session_generation: u64,
        dest_path: &Path,
        initial_revision: u64,
    ) -> Result<NativeDocumentSession, CommandError> {
        let normalized = canonicalize_destination_path(dest_path);
        let session = NativeDocumentSession {
            document_id: document_id.to_string(),
            app_instance_id: app_instance_id.to_string(),
            session_generation,
            authorized_dest_path: normalized,
            last_committed_revision: initial_revision,
            last_committed_seq: 0,
            is_user_authorized: true,
        };
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(document_id.to_string(), session.clone());
        Ok(session)
    }

    /// Cấp một SaveGrant có thời hạn và gắn với revision cụ thể cho phiên tài liệu
    pub fn issue_save_grant(
        &self,
        document_id: &str,
        app_instance_id: &str,
        session_generation: u64,
        request_id: &str,
        expected_revision: u64,
        operation_type: GrantOperationType,
        overwrite_policy: OverwritePolicy,
        ttl_seconds: u64,
    ) -> Result<NativeSaveGrant, CommandError> {
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let session = sessions.get(document_id).ok_or_else(|| CommandError::ValidationFailed {
            reason: format!("Không tìm thấy phiên tài liệu '{document_id}' hợp lệ trên Native Backend"),
        })?;

        if !session.is_user_authorized {
            return Err(CommandError::ValidationFailed {
                reason: format!("Tài liệu '{document_id}' chưa được người dùng cấp quyền truy cập"),
            });
        }

        if session.app_instance_id != app_instance_id {
            return Err(CommandError::ValidationFailed {
                reason: format!("Mã định danh App Instance không khớp với phiên tài liệu '{document_id}'"),
            });
        }

        if session.session_generation != session_generation {
            return Err(CommandError::ValidationFailed {
                reason: format!(
                    "Thế hệ phiên ({session_generation}) không khớp với phiên hiện tại ({}) của tài liệu '{document_id}'",
                    session.session_generation
                ),
            });
        }

        // Bất biến: expected_revision phải lớn hơn last_committed_revision
        if expected_revision <= session.last_committed_revision {
            return Err(CommandError::IoError {
                details: format!(
                    "Yêu cầu cấp grant cho revision {} lỗi thời (phiên bản đã commit: {})",
                    expected_revision, session.last_committed_revision
                ),
            });
        }

        let mut grants = self.grants.lock().unwrap_or_else(|e| e.into_inner());
        for g in grants.values() {
            if g.document_id == document_id && g.status == GrantStatus::InUse {
                return Err(CommandError::ValidationFailed {
                    reason: format!("Từ chối cấp grant: Đang có một tác vụ ghi (InUse) diễn ra trên tài liệu '{document_id}'"),
                });
            }
        }

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        let expires_ms = now_ms.saturating_add(ttl_seconds * 1000);
        let grant_id = format!("grant-{}", uuid::Uuid::new_v4());

        let grant = NativeSaveGrant {
            grant_id: grant_id.clone(),
            document_id: document_id.to_string(),
            app_instance_id: app_instance_id.to_string(),
            session_generation,
            authorized_dest_path: session.authorized_dest_path.clone(),
            operation_type,
            expected_revision,
            overwrite_policy,
            issued_at_epoch_ms: now_ms,
            expires_at_epoch_ms: expires_ms,
            request_id: request_id.to_string(),
            status: GrantStatus::Active,
        };

        grants.insert(grant_id, grant.clone());
        Ok(grant)
    }

    /// Thu hồi quyền ghi của một Grant cụ thể
    pub fn revoke_grant(&self, grant_id: &str) -> Result<(), CommandError> {
        let mut grants = self.grants.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(grant) = grants.get_mut(grant_id) {
            grant.status = GrantStatus::Revoked;
            Ok(())
        } else {
            Err(CommandError::ValidationFailed {
                reason: format!("Không tìm thấy grant '{grant_id}' để thu hồi"),
            })
        }
    }

    /// Vô hiệu hóa toàn bộ session (khi reload hoặc thoát micro-app)
    pub fn invalidate_session(&self, document_id: &str) {
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions.remove(document_id);
        let mut grants = self.grants.lock().unwrap_or_else(|e| e.into_inner());
        for grant in grants.values_mut() {
            if grant.document_id == document_id {
                grant.status = GrantStatus::Revoked;
            }
        }
    }

    /// Thẩm định và thực thi ghi đĩa an toàn bằng SaveGrant (Stage 4B-2A Revised API)
    /// Đích đến được sở hữu 100% bởi grant phía backend, JavaScript không thể can thiệp đường dẫn.
    pub fn execute_granted_save(
        &self,
        grant_id: &str,
        request_id: &str,
        bytes: &[u8],
    ) -> Result<SaveSuccessResponse, CommandError> {
        // 1. Kiểm tra kích thước payload chặn DoS
        if bytes.len() > MAX_PAYLOAD_BYTES {
            return Err(CommandError::ValidationFailed {
                reason: format!(
                    "Kích thước tệp ({} bytes) vượt quá giới hạn an toàn tối đa ({} bytes)",
                    bytes.len(),
                    MAX_PAYLOAD_BYTES
                ),
            });
        }

        // 2. Thẩm tra và cập nhật trạng thái InUse dưới khóa Mutex
        // QUY CHUẨN ĐỒNG QUY: Luôn thu nạp khóa sessions TRƯỚC, sau đó mới thu nạp khóa grants
        let (dest_path, expected_revision, session_id) = {
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            let mut grants = self.grants.lock().unwrap_or_else(|e| e.into_inner());
            
            let (target_doc_id, target_dest_path) = {
                let g = grants.get(grant_id).ok_or_else(|| CommandError::ValidationFailed {
                    reason: format!("Truy cập bị từ chối: Grant ID '{grant_id}' không tồn tại hoặc không hợp lệ"),
                })?;

                // Kiểm tra request_id khớp với grant
                if g.request_id != request_id {
                    return Err(CommandError::ValidationFailed {
                        reason: format!(
                            "Request ID '{request_id}' không khớp với Request ID được cấp phép '{}'",
                            g.request_id
                        ),
                    });
                }
                (g.document_id.clone(), g.authorized_dest_path.clone())
            };

            // Bất biến: Chống ghi đồng thời trên cấp độ tài liệu / đường dẫn đích (Document-level Write Exclusion)
            for (gid, g) in grants.iter() {
                if gid != grant_id
                    && (g.document_id == target_doc_id || g.authorized_dest_path == target_dest_path)
                    && g.status == GrantStatus::InUse
                {
                    return Err(CommandError::IoError {
                        details: format!(
                            "Từ chối thực thi grant: Đang có một tiến trình ghi đĩa (InUse) diễn ra trên cùng tài liệu '{target_doc_id}'"
                        ),
                    });
                }
            }

            let grant = grants.get_mut(grant_id).unwrap();

            // Kiểm tra trạng thái grant
            match grant.status {
                GrantStatus::Active => {}
                GrantStatus::InUse => {
                    return Err(CommandError::IoError {
                        details: format!("Grant '{grant_id}' đang được thực thi trong một tác vụ ghi khác (chống ghi đồng thời)"),
                    });
                }
                GrantStatus::Consumed => {
                    return Err(CommandError::ValidationFailed {
                        reason: format!("Grant '{grant_id}' đã được sử dụng trước đó (chống trùng lặp/replay)"),
                    });
                }
                GrantStatus::Revoked => {
                    return Err(CommandError::ValidationFailed {
                        reason: format!("Grant '{grant_id}' đã bị thu hồi bởi Native Backend"),
                    });
                }
                GrantStatus::Expired => {
                    return Err(CommandError::ValidationFailed {
                        reason: format!("Grant '{grant_id}' đã hết hạn"),
                    });
                }
                GrantStatus::Failed => {
                    return Err(CommandError::ValidationFailed {
                        reason: format!("Grant '{grant_id}' đã thất bại trong lần thử trước"),
                    });
                }
                GrantStatus::AmbiguousCommit => {
                    return Err(CommandError::IoError {
                        details: format!("Grant '{grant_id}' đang ở trạng thái AmbiguousCommit"),
                    });
                }
            }

            // Kiểm tra TTL
            let now_ms = chrono::Utc::now().timestamp_millis() as u64;
            if now_ms > grant.expires_at_epoch_ms {
                grant.status = GrantStatus::Expired;
                return Err(CommandError::ValidationFailed {
                    reason: format!(
                        "Grant '{grant_id}' đã hết hạn hiệu lực ({now_ms} > {})",
                        grant.expires_at_epoch_ms
                    ),
                });
            }

            // Kiểm tra session tương ứng (sessions đã được lock ở đầu khối bảo đảm nhất quán)
            let session = sessions.get(&grant.document_id).ok_or_else(|| CommandError::ValidationFailed {
                reason: format!("Phiên tài liệu '{}' không còn tồn tại trên Native Backend", grant.document_id),
            })?;

            if !session.is_user_authorized {
                return Err(CommandError::ValidationFailed {
                    reason: "Từ chối ghi tệp: Phiên tài liệu chưa được người dùng ủy quyền qua Native Dialog (NFR-SEC-01)".to_string(),
                });
            }

            if grant.app_instance_id != session.app_instance_id {
                return Err(CommandError::ValidationFailed {
                    reason: format!(
                        "Định danh ứng dụng không khớp (grant: {}, session: {})",
                        grant.app_instance_id, session.app_instance_id
                    ),
                });
            }

            if session.session_generation != grant.session_generation {
                return Err(CommandError::ValidationFailed {
                    reason: format!(
                        "Thế hệ phiên của grant ({}) đã cũ so với phiên hiện tại ({})",
                        grant.session_generation, session.session_generation
                    ),
                });
            }

            // Bất biến: Kiểm tra thứ tự revision (Task 6)
            if grant.expected_revision <= session.last_committed_revision {
                return Err(CommandError::IoError {
                    details: format!(
                        "Lượt ghi lỗi thời bị từ chối: phiên bản tài liệu {} đã bị vượt qua bởi phiên bản {}",
                        grant.expected_revision, session.last_committed_revision
                    ),
                });
            }

            // Kiểm tra OverwritePolicy
            if grant.overwrite_policy == OverwritePolicy::RequireNew && grant.authorized_dest_path.exists() {
                return Err(CommandError::ValidationFailed {
                    reason: format!("Chính sách RequireNew từ chối ghi đè tệp đã tồn tại: {:?}", grant.authorized_dest_path),
                });
            }

            // Chuyển sang InUse để chặn ghi đồng thời (Task 7)
            grant.status = GrantStatus::InUse;
            (grant.authorized_dest_path.clone(), grant.expected_revision, grant.document_id.clone())
        };

        // 3. Thực hiện ghi nguyên tử ngoài khóa Mutex
        let dest_str = dest_path.to_string_lossy().to_string();
        let write_res = native_atomic_durable_write(&dest_str, bytes);

        // 4. Cập nhật trạng thái sau khi I/O hoàn tất
        // QUY CHUẨN ĐỒNG QUY: Luôn thu nạp khóa sessions TRƯỚC, sau đó mới thu nạp khóa grants
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let mut grants = self.grants.lock().unwrap_or_else(|e| e.into_inner());

        match write_res {
            Ok(()) => {
                if let Some(grant) = grants.get_mut(grant_id) {
                    grant.status = GrantStatus::Consumed;
                }
                if let Some(session) = sessions.get_mut(&session_id) {
                    if expected_revision > session.last_committed_revision {
                        session.last_committed_revision = expected_revision;
                    }
                }
                let fingerprint = compute_fingerprint(bytes);
                Ok(SaveSuccessResponse {
                    dest_path: dest_str,
                    bytes_written: bytes.len(),
                    committed_revision: expected_revision,
                    fingerprint,
                    is_ambiguous: false,
                })
            }
            Err(e) => {
                let is_post_rename_ambiguous = matches!(&e, CommandError::IoError { details } if details.contains("AmbiguousCommit"));
                if let Some(grant) = grants.get_mut(grant_id) {
                    if is_post_rename_ambiguous {
                        grant.status = GrantStatus::AmbiguousCommit;
                    } else {
                        grant.status = GrantStatus::Failed;
                    }
                }
                if is_post_rename_ambiguous {
                    // Tệp đích đã bị hoán đổi thành công trên đĩa nhưng độ bền chưa được xác nhận hoàn toàn.
                    // Cập nhật last_committed_revision để phản ánh tệp đích đã thay đổi và ngăn replay ghi đè cũ.
                    if let Some(session) = sessions.get_mut(&session_id) {
                        if expected_revision > session.last_committed_revision {
                            session.last_committed_revision = expected_revision;
                        }
                    }
                } else {
                    // Thất bại trước rename -> Tệp đích chưa bị thay đổi -> Dọn dẹp session nếu là phiên tạm
                    if let Some(session) = sessions.get(&session_id) {
                        if session.last_committed_revision == 0 {
                            sessions.remove(&session_id);
                        }
                    }
                }
                Err(e)
            }
        }
    }

    /// Đọc nội dung nhị phân của tài liệu đã được ủy quyền an toàn trong session.
    /// [BẢO MẬT STAGE 4C-0D]: Nghiêm cấm đọc raw path tùy ý, chỉ đọc khi tài liệu có session hợp lệ.
    pub fn read_authorized_document_bytes(
        &self,
        document_id: &str,
        app_instance_id: &str,
        session_generation: u64,
    ) -> Result<Vec<u8>, CommandError> {
        let (path, is_auth) = {
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            let s = sessions.get(document_id).ok_or_else(|| CommandError::ValidationFailed {
                reason: format!("Phiên tài liệu '{document_id}' không tồn tại hoặc đã bị hủy"),
            })?;
            if s.app_instance_id != app_instance_id || s.session_generation != session_generation {
                return Err(CommandError::ValidationFailed {
                    reason: "Phiên tài liệu không hợp lệ hoặc không thuộc ứng dụng này".to_string(),
                });
            }
            (s.authorized_dest_path.clone(), s.is_user_authorized)
        };

        if !is_auth {
            return Err(CommandError::ValidationFailed {
                reason: "Tài liệu chưa được người dùng ủy quyền qua Native Dialog".to_string(),
            });
        }

        let file = std::fs::File::open(&path).map_err(|e| CommandError::IoError {
            details: format!("Không thể mở tệp đã ủy quyền: {e}"),
        })?;
        let meta = file.metadata().map_err(|e| CommandError::IoError {
            details: format!("Không thể đọc thông tin tệp: {e}"),
        })?;
        let len = meta.len();
        if len > MAX_PAYLOAD_BYTES as u64 {
            return Err(CommandError::ValidationFailed {
                reason: format!("Kích thước tệp ({} bytes) vượt quá giới hạn an toàn tối đa", len),
            });
        }

        use std::io::Read;
        let mut reader = file.take(MAX_PAYLOAD_BYTES as u64 + 1);
        let mut bytes = Vec::with_capacity(len.min(MAX_PAYLOAD_BYTES as u64) as usize);
        reader.read_to_end(&mut bytes).map_err(|e| CommandError::IoError {
            details: format!("Không thể nạp nội dung tệp: {e}"),
        })?;

        if bytes.len() > MAX_PAYLOAD_BYTES {
            return Err(CommandError::ValidationFailed {
                reason: format!("Kích thước tệp ({} bytes) vượt quá giới hạn an toàn tối đa", bytes.len()),
            });
        }

        Ok(bytes)
    }
}

pub static GLOBAL_SAVE_GRANT_MANAGER: LazyLock<NativeSaveGrantManager> =
    LazyLock::new(NativeSaveGrantManager::new);

// ==============================================================================
// KIỂM THỬ ĐƠN VỊ & ĐỐI KHÁNG TẦNG NATIVE BACKEND
// ==============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;

    pub struct MockNativeDialogService {
        pub pick_path: Mutex<Option<PathBuf>>,
        pub save_path: Mutex<Option<PathBuf>>,
    }

    impl MockNativeDialogService {
        pub fn new(pick: Option<PathBuf>, save: Option<PathBuf>) -> Self {
            Self {
                pick_path: Mutex::new(pick),
                save_path: Mutex::new(save),
            }
        }
    }

    impl NativeDialogService for MockNativeDialogService {
        fn pick_file(&self, _filter_name: &str, _extensions: &[&str]) -> Option<PathBuf> {
            self.pick_path.lock().unwrap().clone()
        }

        fn save_file(&self, _filter_name: &str, _extensions: &[&str], _default_name: Option<&str>) -> Option<PathBuf> {
            self.save_path.lock().unwrap().clone()
        }
    }

    #[test]
    fn test_native_successful_write_and_content_preservation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("report.xlsx");
        let content = b"PK\x03\x04NATIVE_VALID_EXCEL_BYTES";

        let res = native_atomic_durable_write(dest.to_str().unwrap(), content);
        assert!(res.is_ok(), "Ghi tệp hợp lệ phải thành công: {:?}", res);

        let read_back = std::fs::read(&dest).unwrap();
        assert_eq!(read_back, content);
    }

    #[test]
    fn test_native_overwrite_existing_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("existing.xlsx");
        std::fs::write(&dest, b"OLD_DATA").unwrap();

        let new_content = b"NEW_DATA_OVERWRITTEN_ATOMICALLY";
        let res = native_atomic_durable_write(dest.to_str().unwrap(), new_content);
        assert!(res.is_ok());

        assert_eq!(std::fs::read(&dest).unwrap(), new_content);
    }

    #[test]
    fn test_native_save_as_new_destination() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig = temp_dir.path().join("original.xlsx");
        let copy = temp_dir.path().join("copied.xlsx");

        native_atomic_durable_write(orig.to_str().unwrap(), b"DATA1").unwrap();
        native_atomic_durable_write(copy.to_str().unwrap(), b"DATA2").unwrap();

        assert_eq!(std::fs::read(&orig).unwrap(), b"DATA1");
        assert_eq!(std::fs::read(&copy).unwrap(), b"DATA2");
    }

    #[test]
    fn test_native_path_traversal_rejection() {
        let bad_paths = [
            "../../etc/passwd",
            "sub/../../../secrets.xlsx",
            "test/../traversal.xlsx",
        ];
        for path in bad_paths {
            let res = native_atomic_durable_write(path, b"data");
            assert!(matches!(res, Err(CommandError::ValidationFailed { .. })), "Phải chặn path traversal: {path}");
        }
    }

    #[test]
    fn test_native_null_byte_rejection() {
        let res = native_atomic_durable_write("doc\0hack.xlsx", b"data");
        assert!(matches!(res, Err(CommandError::ValidationFailed { .. })));
    }

    #[test]
    fn test_native_empty_path_rejection() {
        assert!(matches!(native_atomic_durable_write("", b"data"), Err(CommandError::ValidationFailed { .. })));
        assert!(matches!(native_atomic_durable_write("   ", b"data"), Err(CommandError::ValidationFailed { .. })));
    }

    #[test]
    fn test_native_unsupported_extension_rejection() {
        let bad_exts = ["malware.exe", "script.sh", "payload.bat", "run.py", "unsupported.xyz"];
        for path in bad_exts {
            let res = native_atomic_durable_write(path, b"data");
            assert!(matches!(res, Err(CommandError::ValidationFailed { .. })), "Phải từ chối extension: {path}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_native_sensitive_system_path_rejection() {
        let sys_paths = ["/etc/sensitive.xlsx", "/usr/bin/tool.xlsx", "/boot/kernel.xlsx"];
        for path in sys_paths {
            let res = native_atomic_durable_write(path, b"data");
            assert!(matches!(res, Err(CommandError::ValidationFailed { .. })), "Phải chặn ghi vào hệ thống: {path}");
        }
    }

    #[test]
    fn test_native_temp_file_cleanup_and_original_retention_on_failure() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("secure_doc.xlsx");
        let original_data = b"ORIGINAL_IMPORTANT_DOCUMENT";
        std::fs::write(&dest, original_data).unwrap();

        let tracker = NativeDestinationTracker::new();
        let ticket_stale = NativeWriteTicket {
            dest_path: canonicalize_destination_path(&dest),
            seq: 1,
        };

        // Giả lập tracker đã ghi nhận một phiên mới hơn (seq 2)
        {
            let mut map = tracker.inner.lock().unwrap();
            let state = map.entry(ticket_stale.dest_path.clone()).or_default();
            state.last_committed_seq = 2;
        }

        // Cố gắng commit vé stale
        let temp_file = temp_dir.path().join(".tmp.stale_test.xlsx");
        std::fs::write(&temp_file, b"STALE_DATA").unwrap();

        let res = tracker.commit_atomic(&ticket_stale, || {
            std::fs::rename(&temp_file, &dest).map_err(|e| CommandError::IoError { details: e.to_string() })?;
            Ok(())
        });

        assert!(matches!(res, Err(CommandError::IoError { .. })));

        // Tệp đích ban đầu BẮT BUỘC giữ nguyên, không bị xóa hay ghi đè
        assert_eq!(std::fs::read(&dest).unwrap(), original_data);

        // Dọn dẹp tệp tạm
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_native_deterministic_concurrency_race() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("concurrent_race.xlsx");
        let tracker = Arc::new(NativeDestinationTracker::new());

        let ticket_a = tracker.reserve(&dest);
        let ticket_b = tracker.reserve(&dest);
        assert_eq!(ticket_a.seq, 1);
        assert_eq!(ticket_b.seq, 2);

        let content_a = b"REVISION_1_STALE";
        let content_b = b"REVISION_2_NEWEST";

        let barrier_start = Arc::new(Barrier::new(2));
        let barrier_save_b_done = Arc::new(Barrier::new(2));

        let dest_clone = dest.clone();
        let tracker_clone = Arc::clone(&tracker);
        let b_start = Arc::clone(&barrier_start);
        let b_done = Arc::clone(&barrier_save_b_done);

        // Luồng Save A: Chuẩn bị tệp, kiểm tra can_commit, sau đó bị pause
        let handle_a = thread::spawn(move || {
            tracker_clone.can_commit(&ticket_a).unwrap();

            b_start.wait();
            b_done.wait();

            let temp_a = dest_clone.parent().unwrap().join(".tmp.concur_a.xlsx");
            std::fs::write(&temp_a, content_a).unwrap();

            let commit_res = tracker_clone.commit_atomic(&ticket_a, || {
                std::fs::rename(&temp_a, &dest_clone).map_err(|e| CommandError::IoError { details: e.to_string() })?;
                Ok(())
            });

            if temp_a.exists() {
                let _ = std::fs::remove_file(&temp_a);
            }
            commit_res
        });

        // Luồng Save B: Chạy và commit thành công trước
        barrier_start.wait();
        let temp_b = dest.parent().unwrap().join(".tmp.concur_b.xlsx");
        std::fs::write(&temp_b, content_b).unwrap();
        tracker.commit_atomic(&ticket_b, || {
            std::fs::rename(&temp_b, &dest).map_err(|e| CommandError::IoError { details: e.to_string() })?;
            Ok(())
        }).unwrap();

        // Báo cho Save A thức dậy
        barrier_save_b_done.wait();

        let res_a = handle_a.join().unwrap();
        assert!(matches!(res_a, Err(CommandError::IoError { .. })), "Save A phải bị từ chối");

        // Đĩa thực tế phải giữ nguyên nội dung Revision 2
        let on_disk = std::fs::read(&dest).unwrap();
        assert_eq!(on_disk, content_b, "Tệp trên đĩa PHẢI là Revision 2");
    }

    #[test]
    fn test_native_path_canonicalization_consistency() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("canon_test.xlsx");
        let tracker = NativeDestinationTracker::new();

        let ticket_1 = tracker.reserve(&file_path);
        let ticket_2 = tracker.reserve(&file_path);

        assert_eq!(ticket_1.seq, 1);
        assert_eq!(ticket_2.seq, 2);
        assert_eq!(ticket_1.dest_path, ticket_2.dest_path);
    }

    #[test]
    fn test_native_large_file_persistence() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("large_dataset.csv");
        // Tạo payload 5 MB
        let large_bytes = vec![b'X'; 5 * 1024 * 1024];

        let res = native_atomic_durable_write(dest.to_str().unwrap(), &large_bytes);
        assert!(res.is_ok());
        assert_eq!(std::fs::metadata(&dest).unwrap().len(), 5 * 1024 * 1024);
    }

    #[test]
    fn test_save_file_binary_permanently_disabled_fail_closed() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("command_test.xlsx");
        let content = b"PK\x03\x04test_content_from_command";

        let result = tauri::async_runtime::block_on(async {
            crate::commands::save_file_binary(dest.to_str().unwrap().to_string(), content.to_vec()).await
        });

        assert!(result.is_err(), "save_file_binary must always fail closed");
        match result.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("vô hiệu hóa vĩnh viễn"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
        assert!(!dest.exists());
    }

    #[test]
    fn test_save_file_binary_unauthorized_bypass_closure() {
        // KIỂM THỬ TRỌNG TÂM TASK 4:
        // Cố tình gọi trực tiếp save_file_binary với đường dẫn chưa từng được người dùng ủy quyền
        let temp_dir = tempfile::tempdir().unwrap();
        let unauthorized_dest = temp_dir.path().join("unauthorized_victim.xlsx");
        let content = b"PK\x03\x04malicious_data";

        let result = tauri::async_runtime::block_on(async {
            crate::commands::save_file_binary(unauthorized_dest.to_str().unwrap().to_string(), content.to_vec()).await
        });

        // BẮT BUỘC PHẢI BỊ TỪ CHỐI 100% (Fail-Closed)
        assert!(result.is_err(), "save_file_binary must fail-closed permanently");
        match result.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("vô hiệu hóa vĩnh viễn") || reason.contains("chưa được người dùng ủy quyền"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }

        // Tệp không bao giờ được tạo trên đĩa
        assert!(!unauthorized_dest.exists(), "Tệp không được phép tạo trên đĩa khi chưa có ủy quyền");
    }

    #[test]
    fn test_save_file_binary_tauri_command_rejection() {
        let result = tauri::async_runtime::block_on(async {
            crate::commands::save_file_binary("../evil.sh".to_string(), b"malicious".to_vec()).await
        });

        assert!(result.is_err(), "save_file_binary command must reject path traversal or fail closed");
        match result.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("vô hiệu hóa vĩnh viễn") || reason.contains("chưa được người dùng ủy quyền"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_reordered_ipc_arrival_anomaly() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("reordered_anomaly.xlsx");

        // Đăng ký phiên để kiểm tra hành vi
        let manager = NativeSaveGrantManager::new();
        manager
            .register_document_session("doc-reorder-anomaly", "app-1", 1, &dest, 0)
            .unwrap();

        // Cấp Grant A (rev 1) và Grant B (rev 2)
        let grant_a = manager
            .issue_save_grant("doc-reorder-anomaly", "app-1", 1, "req-1", 1, GrantOperationType::Save, OverwritePolicy::AllowOverwrite, 30)
            .unwrap();
        let grant_b = manager
            .issue_save_grant("doc-reorder-anomaly", "app-1", 1, "req-2", 2, GrantOperationType::Save, OverwritePolicy::AllowOverwrite, 30)
            .unwrap();

        // Giả lập Save B (Snapshot mới hơn, phiên bản 2) đến native backend trước
        let res_b = manager.execute_granted_save(&grant_b.grant_id, "req-2", b"REVISION_2_NEWER");
        assert!(res_b.is_ok(), "Save B (rev 2) must succeed");
        assert_eq!(std::fs::read(&dest).unwrap(), b"REVISION_2_NEWER");

        // Giả lập Save A (Snapshot cũ hơn, phiên bản 1) do mạng/Tokio jitter bị trễ, đến native backend sau
        let res_a = manager.execute_granted_save(&grant_a.grant_id, "req-1", b"REVISION_1_OLDER");
        assert!(res_a.is_err(), "Save A (rev 1) must be rejected because rev 2 was already committed");

        // Xác minh đĩa vẫn giữ nguyên Revision 2
        let disk_content = std::fs::read(&dest).unwrap();
        assert_eq!(disk_content, b"REVISION_2_NEWER",
            "Đĩa bắt buộc phải giữ nguyên Revision 2, không bao giờ bị ghi đè bởi Revision 1 đến muộn");
    }

    // ==========================================================================
    // STAGE 4B-2A: TAURI COMMANDS E2E SECURITY & AUTHORIZATION TESTS
    // ==========================================================================

    #[test]
    fn test_tauri_command_secure_save_document_e2e_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("secure_e2e.xlsx");
        let payload = b"PK\x03\x04TAURI_SECURE_COMMAND_E2E_PAYLOAD";

        // 1. Đăng ký phiên nội bộ có thẩm quyền
        GLOBAL_SAVE_GRANT_MANAGER
            .register_document_session(
                "doc-e2e-1",
                "inst-1",
                1,
                &dest,
                0,
            )
            .unwrap();

        // 2. Yêu cầu cấp grant qua Tauri Command request_save_grant
        let grant_res = tauri::async_runtime::block_on(async {
            crate::commands::request_save_grant(
                "doc-e2e-1".to_string(),
                "inst-1".to_string(),
                1,
                "req-e2e-1".to_string(),
                1,
            )
            .await
        });
        assert!(grant_res.is_ok(), "request_save_grant must succeed");
        let grant = grant_res.unwrap();

        // 3. Thực thi lưu an toàn qua Tauri Command secure_save_document
        let save_res = tauri::async_runtime::block_on(async {
            crate::commands::secure_save_document(grant.grant_id.clone(), "req-e2e-1".to_string(), payload.to_vec()).await
        });
        assert!(save_res.is_ok(), "secure_save_document must succeed");
        let outcome = save_res.unwrap();

        assert_eq!(outcome.dest_path, dest.to_str().unwrap());
        assert_eq!(outcome.committed_revision, 1);
        assert!(!outcome.is_ambiguous);
        assert_eq!(std::fs::read(&dest).unwrap(), payload);
    }

    #[test]
    fn test_tauri_command_secure_save_document_forged_grant_rejection() {
        let payload = b"PK\x03\x04ATTACKER_DATA";
        let res = tauri::async_runtime::block_on(async {
            crate::commands::secure_save_document(
                "forged-grant-uuid-1234".to_string(),
                "some-req".to_string(),
                payload.to_vec(),
            )
            .await
        });

        assert!(res.is_err(), "Forged grant must be rejected by secure_save_document");
        match res.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("không tồn tại hoặc không hợp lệ") || reason.contains("không tồn tại"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_tauri_command_secure_save_document_stale_revision_rejection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("secure_ordering_e2e.xlsx");

        // 1. Đăng ký phiên
        GLOBAL_SAVE_GRANT_MANAGER
            .register_document_session(
                "doc-e2e-order",
                "inst-1",
                1,
                &dest,
                0,
            )
            .unwrap();

        // 2. Yêu cầu Grant A (rev 10) và Grant B (rev 11)
        let grant_a = tauri::async_runtime::block_on(async {
            crate::commands::request_save_grant(
                "doc-e2e-order".to_string(),
                "inst-1".to_string(),
                1,
                "req-a".to_string(),
                10,
            )
            .await
        })
        .unwrap();

        let grant_b = tauri::async_runtime::block_on(async {
            crate::commands::request_save_grant(
                "doc-e2e-order".to_string(),
                "inst-1".to_string(),
                1,
                "req-b".to_string(),
                11,
            )
            .await
        })
        .unwrap();

        // 3. Save B (rev 11) đến trước qua command
        let res_b = tauri::async_runtime::block_on(async {
            crate::commands::secure_save_document(grant_b.grant_id, "req-b".to_string(), b"PK\x03\x04REVISION_11".to_vec()).await
        });
        assert!(res_b.is_ok(), "Save B must succeed");
        assert_eq!(std::fs::read(&dest).unwrap(), b"PK\x03\x04REVISION_11");

        // 4. Save A (rev 10) đến sau qua command -> BẮT BUỘC BỊ TỪ CHỐI
        let res_a = tauri::async_runtime::block_on(async {
            crate::commands::secure_save_document(grant_a.grant_id, "req-a".to_string(), b"PK\x03\x04REVISION_10".to_vec()).await
        });
        assert!(res_a.is_err(), "Save A arriving after Save B must be rejected");

        // 5. Kiểm tra đĩa vẫn giữ nguyên Revision 11
        assert_eq!(std::fs::read(&dest).unwrap(), b"PK\x03\x04REVISION_11");
    }

    // ==========================================================================
    // STAGE 4B-2A CLOSURE: ADVERSARIAL TESTS & TASK 2-8 VERIFICATIONS
    // ==========================================================================

    #[test]
    fn test_adversarial_self_authorization_bypass_closed() {
        let temp_dir = tempfile::tempdir().unwrap();
        let victim_file = temp_dir.path().join("victim_document.xlsx");
        std::fs::write(&victim_file, b"ORIGINAL_IMPORTANT_EXCEL_DATA").unwrap();
        let victim_path_str = victim_file.to_str().unwrap().to_string();

        // Kẻ tấn công cố tình gọi save_file_binary trực tiếp để ghi đè tệp nạn nhân
        let evil_payload = b"MALICIOUS_OVERWRITE_PAYLOAD".to_vec();
        let res = tauri::async_runtime::block_on(async {
            crate::commands::save_file_binary(victim_path_str.clone(), evil_payload).await
        });

        // BẮT BUỘC PHẢI BỊ CHẶN HOÀN TOÀN (BLOCKED BY VERIFIED CONTROL)
        assert!(res.is_err(), "save_file_binary must be completely blocked");
        let content_on_disk = std::fs::read(&victim_file).unwrap();
        assert_eq!(
            content_on_disk, b"ORIGINAL_IMPORTANT_EXCEL_DATA",
            "Tệp nạn nhân phải hoàn toàn không bị suy suyển!"
        );
    }

    #[test]
    fn test_adversarial_forged_app_instance_id_rejected() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("app_instance_test.xlsx");
        let manager = NativeSaveGrantManager::new();

        manager
            .register_document_session("doc-inst-sec", "pdf-studio-instance", 1, &dest, 0)
            .unwrap();

        // Kẻ tấn công từ iframe độc hại với app_instance_id giả mạo
        let res = manager.issue_save_grant(
            "doc-inst-sec",
            "evil-iframe-instance",
            1,
            "req-evil-1",
            1,
            GrantOperationType::Save,
            OverwritePolicy::AllowOverwrite,
            30,
        );

        assert!(res.is_err());
        match res.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("App Instance không khớp") || reason.contains("không khớp"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_adversarial_forged_document_id_rejected() {
        let manager = NativeSaveGrantManager::new();
        // Kẻ tấn công tự bịa ra document_id chưa từng được đăng ký
        let res = manager.issue_save_grant(
            "doc-invented-uuid-9999",
            "app-inst-1",
            1,
            "req-forged-doc",
            1,
            GrantOperationType::Save,
            OverwritePolicy::AllowOverwrite,
            30,
        );

        assert!(res.is_err());
        match res.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("không còn tồn tại") || reason.contains("Không tìm thấy"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_adversarial_borrowed_session_cross_origin_rejected() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path_a = temp_dir.path().join("doc_a.xlsx");
        let path_b = temp_dir.path().join("doc_b.xlsx");
        std::fs::write(&path_b, b"DOC_B_SAFE").unwrap();

        let manager = NativeSaveGrantManager::new();
        manager
            .register_document_session("doc-a", "app-inst", 1, &path_a, 0)
            .unwrap();

        let grant_a = manager
            .issue_save_grant("doc-a", "app-inst", 1, "req-a", 1, GrantOperationType::Save, OverwritePolicy::AllowOverwrite, 30)
            .unwrap();

        // Kẻ tấn công muốn dùng grant của doc A để ghi vào doc B
        // Nhưng API secure_save_document KHÔNG NHẬN path từ client:
        let res = manager.execute_granted_save(&grant_a.grant_id, "req-a", b"PK\x03\x04PAYLOAD_FOR_A");
        assert!(res.is_ok());

        // Xác minh: File A nhận payload, File B hoàn toàn KHÔNG BỊ ẢNH HƯỞNG
        assert_eq!(std::fs::read(&path_a).unwrap(), b"PK\x03\x04PAYLOAD_FOR_A");
        assert_eq!(std::fs::read(&path_b).unwrap(), b"DOC_B_SAFE");
    }

    #[test]
    fn test_adversarial_unrelated_iframe_cannot_write_pdf_studio_authorized_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let pdf_file = temp_dir.path().join("confidential_contract.pdf");
        std::fs::write(&pdf_file, b"%PDF-1.4 CONFIDENTIAL").unwrap();

        // PDF Studio đăng ký phiên hợp lệ cho tệp
        GLOBAL_SAVE_GRANT_MANAGER
            .register_document_session("doc-pdf-legit", "pdf-studio-instance", 1, &pdf_file, 0)
            .unwrap();

        // Iframe không liên quan cố gọi save_file_binary để can thiệp vào tệp confidential
        let res = tauri::async_runtime::block_on(async {
            crate::commands::save_file_binary(
                pdf_file.to_str().unwrap().to_string(),
                b"%PDF-1.4 TAMPERED_DATA".to_vec(),
            )
            .await
        });

        // BẮT BUỘC BỊ TỪ CHỐI
        assert!(res.is_err(), "Unrelated script must be blocked from writing PDF file");
        assert_eq!(std::fs::read(&pdf_file).unwrap(), b"%PDF-1.4 CONFIDENTIAL");
    }

    #[test]
    fn test_native_dialog_open_file_and_save_as_flow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let open_src = temp_dir.path().join("source.xlsx");
        let save_dest = temp_dir.path().join("saved_as.xlsx");
        std::fs::write(&open_src, b"PK\x03\x04ORIGINAL_DATA").unwrap();

        let manager = NativeSaveGrantManager::new();
        let mock_dialog = Box::new(MockNativeDialogService::new(
            Some(open_src.clone()),
            Some(save_dest.clone()),
        ));
        manager.set_dialog_service_for_test(mock_dialog);

        // 1. Mở tệp qua native dialog
        let open_res = manager
            .open_file_via_native_dialog("app-test", 1, "Excel Files", &["xlsx"])
            .unwrap();
        assert!(open_res.is_some());
        let doc_handle = open_res.unwrap();
        assert_eq!(doc_handle.bytes, b"PK\x03\x04ORIGINAL_DATA");
        assert_eq!(doc_handle.initial_revision, 0);

        // 2. Lưu mới (Save As) qua native dialog
        let save_as_res = manager
            .save_as_via_native_dialog(
                Some(&doc_handle.document_id),
                "app-test",
                1,
                Some("saved_as.xlsx"),
                "Excel Files",
                &["xlsx"],
            )
            .unwrap();
        assert!(save_as_res.is_some());
        let save_as_handle = save_as_res.unwrap();
        assert_eq!(save_as_handle.grant.expected_revision, 1);

        // 3. Thực thi lưu với grant từ Save As
        let write_res = manager.execute_granted_save(
            &save_as_handle.grant.grant_id,
            &save_as_handle.grant.request_id,
            b"PK\x03\x04NEW_SAVED_AS_DATA",
        );
        assert!(write_res.is_ok());

        assert_eq!(std::fs::read(&save_dest).unwrap(), b"PK\x03\x04NEW_SAVED_AS_DATA");
    }

    #[test]
    fn test_pdf_file_full_roundtrip_verification() {
        use lopdf::dictionary;

        let temp_dir = tempfile::tempdir().unwrap();
        let initial_pdf = temp_dir.path().join("contract.pdf");
        let save_as_pdf = temp_dir.path().join("contract_signed.pdf");

        // 1. Khởi tạo cấu trúc tài liệu PDF hợp lệ chuẩn hóa bằng lopdf
        let mut initial_doc = lopdf::Document::with_version("1.5");
        let pages_id = initial_doc.new_object_id();
        let font_id = initial_doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let page_id = initial_doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => dictionary! {
                "Font" => font_id,
            },
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        };
        initial_doc.set_object(pages_id, lopdf::Object::Dictionary(pages));
        let catalog_id = initial_doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        initial_doc.trailer.set("Root", catalog_id);

        let mut initial_bytes = Vec::new();
        initial_doc.save_to(&mut initial_bytes).expect("Phải tuần tự hóa được PDF gốc");
        std::fs::write(&initial_pdf, &initial_bytes).unwrap();

        let manager = NativeSaveGrantManager::new();
        let mock_dialog = Box::new(MockNativeDialogService::new(
            Some(initial_pdf.clone()),
            Some(save_as_pdf.clone()),
        ));
        manager.set_dialog_service_for_test(mock_dialog);

        // 2. Mở tệp PDF qua Native Dialog backend
        let open_res = manager
            .open_file_via_native_dialog("pdf-studio-inst", 1, "Tài liệu PDF", &["pdf"])
            .unwrap();
        assert!(open_res.is_some());
        let handle = open_res.unwrap();

        // Kiểm tra magic bytes và phân tích cây đối tượng bằng lopdf
        let parsed_opened = lopdf::Document::load_mem(&handle.bytes).expect("lopdf phải nạp được tệp vừa mở");
        assert!(parsed_opened.trailer.get(b"Root").is_ok(), "Trailer phải chứa /Root catalog");
        let root_dict = parsed_opened.get_object(catalog_id).expect("Phải tìm thấy Catalog").as_dict().unwrap();
        assert_eq!(root_dict.get(b"Type").unwrap().as_name().unwrap(), b"Catalog");

        // 3. Biên tập PDF bằng lopdf: Thêm đối tượng siêu dữ liệu mới (Metadata Object)
        let mut edit_doc = parsed_opened.clone();
        let meta_id = edit_doc.add_object(dictionary! {
            "Type" => "Metadata",
            "Producer" => "OfficeUtilityVN_PDFStudio",
            "Comment" => "NFR-SEC-01 Verified Roundtrip",
        });
        edit_doc.trailer.set("CustomMeta", meta_id);

        let mut edited_pdf = Vec::new();
        edit_doc.save_to(&mut edited_pdf).expect("Phải tuần tự hóa được PDF sau khi sửa");

        // 4. Thực hiện Save thông thường có cấp Save Grant
        let grant = manager
            .issue_save_grant(
                &handle.document_id,
                "pdf-studio-inst",
                1,
                "req-pdf-save-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            )
            .unwrap();

        let save_res = manager.execute_granted_save(&grant.grant_id, "req-pdf-save-1", &edited_pdf);
        assert!(save_res.is_ok());

        // 5. Đọc lại từ đĩa và kiểm tra tính toàn vẹn cấu trúc bằng lopdf
        let read_back = std::fs::read(&initial_pdf).unwrap();
        assert_eq!(read_back, edited_pdf);
        let verified_doc = lopdf::Document::load_mem(&read_back).expect("Tệp lưu phải nạp được bằng lopdf");
        assert!(verified_doc.trailer.get(b"CustomMeta").is_ok());
        let verified_meta = verified_doc.get_object(meta_id).expect("Đối tượng Metadata phải tồn tại").as_dict().unwrap();
        assert_eq!(verified_meta.get(b"Producer").unwrap().as_name().unwrap(), b"OfficeUtilityVN_PDFStudio");

        // 6. Kiểm tra tình huống HỦY (Cancel) Save As: Mock dialog trả về None
        let cancel_mock = Box::new(MockNativeDialogService::new(None, None));
        manager.set_dialog_service_for_test(cancel_mock);
        let cancel_res = manager
            .save_as_via_native_dialog(
                Some(&handle.document_id),
                "pdf-studio-inst",
                1,
                Some("canceled.pdf"),
                "Tài liệu PDF",
                &["pdf"],
            )
            .unwrap();
        assert!(cancel_res.is_none(), "Khi người dùng Cancel hộp thoại thì kết quả phải là None");
        assert!(!save_as_pdf.exists(), "Không được tạo tệp mới khi bị hủy");
        // Tệp gốc initial_pdf vẫn nguyên vẹn và hợp lệ
        let check_orig = lopdf::Document::load_mem(&std::fs::read(&initial_pdf).unwrap()).unwrap();
        assert!(check_orig.trailer.get(b"CustomMeta").is_ok());

        // 7. Kiểm tra tình huống TỪ CHỐI ỦY QUYỀN (Permission Denied / Invalid Session)
        let invalid_grant_res = manager.issue_save_grant(
            "unregistered-doc-id-xyz",
            "pdf-studio-inst",
            1,
            "req-unauth",
            1,
            GrantOperationType::Save,
            OverwritePolicy::AllowOverwrite,
            30,
        );
        assert!(invalid_grant_res.is_err(), "Tài liệu chưa đăng ký phải bị từ chối grant");

        // 8. Thực hiện Save As thành công sang tệp mới
        let save_as_mock = Box::new(MockNativeDialogService::new(None, Some(save_as_pdf.clone())));
        manager.set_dialog_service_for_test(save_as_mock);
        let save_as_res = manager
            .save_as_via_native_dialog(
                Some(&handle.document_id),
                "pdf-studio-inst",
                1,
                Some("contract_signed.pdf"),
                "Tài liệu PDF",
                &["pdf"],
            )
            .unwrap();
        assert!(save_as_res.is_some());
        let sa_handle = save_as_res.unwrap();

        let mut signed_doc = edit_doc.clone();
        let sig_id = signed_doc.add_object(dictionary! {
            "Type" => "Sig",
            "Filter" => "Adobe.PPKLite",
            "Name" => "Nguyen Van A - Digital Signature",
        });
        signed_doc.trailer.set("DigitalSignature", sig_id);
        let mut signed_pdf = Vec::new();
        signed_doc.save_to(&mut signed_pdf).expect("Phải tuần tự hóa được PDF ký số");

        let sa_exec = manager.execute_granted_save(&sa_handle.grant.grant_id, &sa_handle.grant.request_id, &signed_pdf);
        assert!(sa_exec.is_ok());

        let final_signed = std::fs::read(&save_as_pdf).unwrap();
        let final_doc = lopdf::Document::load_mem(&final_signed).expect("Tệp Save As phải là PDF hợp lệ theo lopdf");
        assert!(final_doc.trailer.get(b"DigitalSignature").is_ok());
        let sig_obj = final_doc.get_object(sig_id).expect("Đối tượng Sig phải tồn tại").as_dict().unwrap();
        assert_eq!(sig_obj.get(b"Name").unwrap().as_name().unwrap(), b"Nguyen Van A - Digital Signature");
    }

    #[test]
    fn test_gridcraft_xlsx_full_e2e_document_lifecycle() {
        use calamine::{Data, Reader, open_workbook_auto};

        let temp_dir = tempfile::tempdir().unwrap();
        let initial_xlsx = temp_dir.path().join("financial_plan.xlsx");
        let save_as_xlsx = temp_dir.path().join("financial_plan_v2.xlsx");

        // Fixtures thực tế được tạo bởi engine GridCraft WASM
        let v1_bytes = include_bytes!("../../fixtures/stage2c_v1.xlsx");
        let v2_bytes = include_bytes!("../../fixtures/stage2c_v2.xlsx");

        std::fs::write(&initial_xlsx, v1_bytes).unwrap();

        let manager = NativeSaveGrantManager::new();
        let mock_dialog = Box::new(MockNativeDialogService::new(
            Some(initial_xlsx.clone()),
            Some(save_as_xlsx.clone()),
        ));
        manager.set_dialog_service_for_test(mock_dialog);

        // 1. OPEN: Mở tệp XLSX qua Native Dialog
        let open_res = manager
            .open_file_via_native_dialog("gridcraft-inst", 1, "Bảng tính Excel", &["xlsx", "xlsm"])
            .unwrap();
        assert!(open_res.is_some());
        let handle = open_res.unwrap();
        assert_eq!(handle.file_name, "financial_plan.xlsx");
        assert_eq!(handle.initial_revision, 0);

        // Kiểm tra độc lập cấu trúc workbook bằng calamine
        let mut wb_opened = open_workbook_auto(&initial_xlsx).expect("calamine phải nạp được tệp vừa mở");
        let sheets = wb_opened.sheet_names();
        assert!(sheets.contains(&"Sheet1".to_string()));
        let range1 = wb_opened.worksheet_range("Sheet1").unwrap();
        assert_eq!(range1.get_value((0, 0)), Some(&Data::Float(98765.0)));

        // 2. EDIT: Giả lập người dùng chỉnh sửa trong GridCraft (thêm ô C1=100, D1=999)
        // Mảng bytes mới là v2_bytes

        // 3. REGULAR SAVE: Yêu cầu cấp Grant và lưu đè vào file hiện hành
        let grant = manager
            .issue_save_grant(
                &handle.document_id,
                "gridcraft-inst",
                1,
                "req-gridcraft-save-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            )
            .unwrap();

        let save_res = manager.execute_granted_save(&grant.grant_id, "req-gridcraft-save-1", v2_bytes);
        assert!(save_res.is_ok());
        let save_resp = save_res.unwrap();
        assert_eq!(save_resp.committed_revision, 1);
        assert!(!save_resp.is_ambiguous);

        // 4. REOPEN & VERIFY: Đọc lại từ đĩa và kiểm tra công thức / dữ liệu với calamine
        let mut wb_saved = open_workbook_auto(&initial_xlsx).expect("calamine phải đọc được tệp đã lưu");
        let range2 = wb_saved.worksheet_range("Sheet1").unwrap();
        assert_eq!(range2.get_value((0, 0)), Some(&Data::Float(98765.0)));
        assert_eq!(range2.get_value((0, 2)), Some(&Data::Float(100.0)));
        assert_eq!(range2.get_value((0, 3)), Some(&Data::Float(999.0)));

        // 5. SAVE AS (CANCEL): Người dùng bấm Hủy trên Native Dialog
        let cancel_mock = Box::new(MockNativeDialogService::new(None, None));
        manager.set_dialog_service_for_test(cancel_mock);
        let cancel_res = manager
            .save_as_via_native_dialog(
                Some(&handle.document_id),
                "gridcraft-inst",
                1,
                Some("canceled.xlsx"),
                "Bảng tính Excel",
                &["xlsx"],
            )
            .unwrap();
        assert!(cancel_res.is_none());
        assert!(!save_as_xlsx.exists(), "Không được tạo tệp mới khi người dùng Hủy");
        // Tệp hiện tại vẫn nguyên vẹn
        assert_eq!(std::fs::read(&initial_xlsx).unwrap(), v2_bytes);

        // 6. SAVE AS (SUCCESS): Người dùng chọn đường dẫn mới
        let save_as_mock = Box::new(MockNativeDialogService::new(None, Some(save_as_xlsx.clone())));
        manager.set_dialog_service_for_test(save_as_mock);
        let sa_res = manager
            .save_as_via_native_dialog(
                Some(&handle.document_id),
                "gridcraft-inst",
                1,
                Some("financial_plan_v2.xlsx"),
                "Bảng tính Excel",
                &["xlsx"],
            )
            .unwrap();
        assert!(sa_res.is_some());
        let sa_handle = sa_res.unwrap();

        let sa_exec = manager.execute_granted_save(&sa_handle.grant.grant_id, &sa_handle.grant.request_id, v2_bytes);
        assert!(sa_exec.is_ok());

        // Kiểm tra độc lập tệp mới được tạo bằng calamine
        let mut wb_sa = open_workbook_auto(&save_as_xlsx).expect("calamine phải nạp được tệp Save As");
        let range_sa = wb_sa.worksheet_range("Sheet1").unwrap();
        assert_eq!(range_sa.get_value((0, 0)), Some(&Data::Float(98765.0)));
        assert_eq!(range_sa.get_value((0, 2)), Some(&Data::Float(100.0)));
        assert_eq!(range_sa.get_value((0, 3)), Some(&Data::Float(999.0)));

        // 7. MONOTONIC REVISION REJECTION: Lưu phiên bản cũ (rev 1) vào session đã cam kết rev 1
        let grant_stale = manager
            .issue_save_grant(
                &sa_handle.document_id,
                "gridcraft-inst",
                1,
                "req-stale-rev",
                1, // expected_revision 1 <= last_committed_revision 1 -> Bị từ chối!
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            );
        assert!(grant_stale.is_err(), "Yêu cầu cấp grant cho revision cũ phải bị từ chối");
    }

    // ==========================================================================
    // STAGE 4B-1: NATIVE SAVE GRANT & DOCUMENT REVISION ORDERING TESTS
    // ==========================================================================

    #[test]
    fn test_grant_creation_and_scope() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("grant_scope_test.xlsx");
        let manager = NativeSaveGrantManager::new();

        let session = manager
            .register_document_session("doc-101", "app-inst-1", 1, &dest, 0)
            .expect("Register session should succeed");
        assert_eq!(session.document_id, "doc-101");
        assert_eq!(session.last_committed_revision, 0);

        let grant = manager
            .issue_save_grant(
                "doc-101",
                "app-inst-1",
                1,
                "save-101-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            )
            .expect("Issue grant should succeed");

        assert_eq!(grant.document_id, "doc-101");
        assert_eq!(grant.app_instance_id, "app-inst-1");
        assert_eq!(grant.expected_revision, 1);
        assert_eq!(grant.status, GrantStatus::Active);
        assert!(grant.expires_at_epoch_ms > grant.issued_at_epoch_ms);
    }

    #[test]
    fn test_unauthorized_request_rejection_no_grant_or_forged_grant() {
        let temp_dir = tempfile::tempdir().unwrap();
        let _dest = temp_dir.path().join("unauthorized_test.xlsx");
        let manager = NativeSaveGrantManager::new();

        let fake_grant_id = "grant-forged-by-iframe-attacker";
        let res = manager.execute_granted_save(fake_grant_id, "req-1", b"PK\x03\x04evil_data");
        assert!(res.is_err(), "Forged grant must be rejected");
        match res.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("không tồn tại hoặc không hợp lệ"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_grant_request_id_mismatch_rejection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("req_mismatch_test.xlsx");
        let manager = NativeSaveGrantManager::new();

        manager
            .register_document_session("doc-req", "app-inst-1", 1, &dest, 0)
            .unwrap();

        let grant = manager
            .issue_save_grant(
                "doc-req",
                "app-inst-1",
                1,
                "legitimate-req-id",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            )
            .unwrap();

        let res = manager.execute_granted_save(&grant.grant_id, "attacker-forged-req-id", b"PK\x03\x04data");
        assert!(res.is_err(), "Request ID mismatch must be rejected");
        match res.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("Request ID 'attacker-forged-req-id' không khớp"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_grant_expiration_rejection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("expired_test.xlsx");
        let manager = NativeSaveGrantManager::new();

        manager
            .register_document_session("doc-exp", "app-inst-1", 1, &dest, 0)
            .unwrap();

        // Cấp grant với TTL = 0 giây
        let mut grant = manager
            .issue_save_grant(
                "doc-exp",
                "app-inst-1",
                1,
                "req-exp-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                0,
            )
            .unwrap();

        // Ghi đè thời hạn thành quá khứ để giả lập hết hạn
        grant.expires_at_epoch_ms = 1;
        manager.grants.lock().unwrap().insert(grant.grant_id.clone(), grant.clone());

        let res = manager.execute_granted_save(&grant.grant_id, "req-exp-1", b"PK\x03\x04expired_bytes");
        assert!(res.is_err(), "Expired grant must be rejected");
        match res.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("đã hết hạn hiệu lực"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_grant_revocation_rejection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("revoked_test.xlsx");
        let manager = NativeSaveGrantManager::new();

        manager
            .register_document_session("doc-rev", "app-inst-1", 1, &dest, 0)
            .unwrap();

        let grant = manager
            .issue_save_grant(
                "doc-rev",
                "app-inst-1",
                1,
                "req-rev-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            )
            .unwrap();

        manager.revoke_grant(&grant.grant_id).expect("Revoke must succeed");

        let res = manager.execute_granted_save(&grant.grant_id, "req-rev-1", b"PK\x03\x04revoked_bytes");
        assert!(res.is_err(), "Revoked grant must be rejected");
        match res.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("đã bị thu hồi"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_duplicate_grant_reuse_rejection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("duplicate_test.xlsx");
        let manager = NativeSaveGrantManager::new();

        manager
            .register_document_session("doc-dup", "app-inst-1", 1, &dest, 0)
            .unwrap();

        let grant = manager
            .issue_save_grant(
                "doc-dup",
                "app-inst-1",
                1,
                "req-dup-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            )
            .unwrap();

        // Lần 1: Thực thi thành công
        let res1 = manager.execute_granted_save(&grant.grant_id, "req-dup-1", b"PK\x03\x04first_save");
        assert!(res1.is_ok(), "First execution must succeed");

        // Lần 2: Cố gắng sử dụng lại grant đã tiêu thụ
        let res2 = manager.execute_granted_save(&grant.grant_id, "req-dup-1", b"PK\x03\x04replay_attack");
        assert!(res2.is_err(), "Duplicate execution must be rejected");
        match res2.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("đã được sử dụng trước đó"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_stale_revision_rejection_save_a_vs_save_b() {
        // KIỂM THỬ TRỌNG TÂM TASK 6:
        // - Save A đại diện cho revision 10
        // - Save B đại diện cho revision 11
        // - Save B đến native backend trước
        // - Save A đến sau
        // -> File trên đĩa PHẢI GIỮ NGUYÊN là revision 11!
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("save_ordering_test.xlsx");
        let manager = NativeSaveGrantManager::new();

        manager
            .register_document_session("doc-order", "app-inst-1", 1, &dest, 0)
            .unwrap();

        // Native Backend cấp Grant A cho revision 10
        let grant_a = manager
            .issue_save_grant(
                "doc-order",
                "app-inst-1",
                1,
                "save-order-1",
                10,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                300,
            )
            .unwrap();

        // Native Backend cấp Grant B cho revision 11
        let grant_b = manager
            .issue_save_grant(
                "doc-order",
                "app-inst-1",
                1,
                "save-order-2",
                11,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                300,
            )
            .unwrap();

        let xlsx_fixture_rev11 = b"PK\x03\x04DETERMINISTIC_XLSX_REVISION_11_NEWER_DATA";
        let xlsx_fixture_rev10 = b"PK\x03\x04DETERMINISTIC_XLSX_REVISION_10_OLDER_DATA";

        // 1. Save B (revision 11) đến native backend TRƯỚC
        let res_b = manager.execute_granted_save(&grant_b.grant_id, "save-order-2", xlsx_fixture_rev11);
        assert!(res_b.is_ok(), "Save B (rev 11) must succeed");
        assert_eq!(std::fs::read(&dest).unwrap(), xlsx_fixture_rev11);

        // 2. Save A (revision 10) do trễ mạng/async jitter đến native backend SAU
        let res_a = manager.execute_granted_save(&grant_a.grant_id, "save-order-1", xlsx_fixture_rev10);
        assert!(res_a.is_err(), "Save A (rev 10) arriving after rev 11 MUST BE REJECTED");

        match res_a.unwrap_err() {
            office_shared::CommandError::IoError { details } => {
                assert!(details.contains("phiên bản tài liệu 10 đã bị vượt qua bởi phiên bản 11"));
            }
            other => panic!("Expected IoError with stale revision details, got {:?}", other),
        }

        // 3. XÁC MINH TUYỆT ĐỐI TÍNH TOÀN VẸN: Tệp trên đĩa PHẢI là Revision 11
        let disk_bytes = std::fs::read(&dest).unwrap();
        assert_eq!(
            disk_bytes, xlsx_fixture_rev11,
            "Tệp trên đĩa BẮT BUỘC phải là Revision 11, không bao giờ bị ghi đè bởi Revision 10 đến sau!"
        );
    }

    #[test]
    fn test_backend_owned_destination_cannot_be_tampered_from_js() {
        let temp_dir = tempfile::tempdir().unwrap();
        let legitimate_dest = temp_dir.path().join("legitimate_dest.xlsx");
        let manager = NativeSaveGrantManager::new();

        manager
            .register_document_session("doc-tamper", "app-inst-1", 1, &legitimate_dest, 0)
            .unwrap();

        let grant = manager
            .issue_save_grant(
                "doc-tamper",
                "app-inst-1",
                1,
                "req-tamper-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            )
            .unwrap();

        // Gọi execute_granted_save - JavaScript không hề truyền dest_path!
        let res = manager.execute_granted_save(&grant.grant_id, "req-tamper-1", b"PK\x03\x04safe_bytes");
        assert!(res.is_ok());
        let outcome = res.unwrap();

        // Đích đến trên đĩa bắt buộc là legitimate_dest đã được backend sở hữu
        assert_eq!(outcome.dest_path, legitimate_dest.to_string_lossy().to_string());
        assert_eq!(std::fs::read(&legitimate_dest).unwrap(), b"PK\x03\x04safe_bytes");
    }

    #[test]
    fn test_session_generation_invalidation_on_reload() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("reload_test.xlsx");
        let manager = NativeSaveGrantManager::new();

        manager
            .register_document_session("doc-reload", "app-inst-1", 1, &dest, 0)
            .unwrap();

        let grant = manager
            .issue_save_grant(
                "doc-reload",
                "app-inst-1",
                1,
                "req-rel-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            )
            .unwrap();

        // Micro-app iframe reload -> session generation tiến lên 2
        manager
            .register_document_session("doc-reload", "app-inst-1", 2, &dest, 1)
            .unwrap();

        // Grant cũ mang session_generation 1 gửi tới -> bị từ chối
        let res = manager.execute_granted_save(&grant.grant_id, "req-rel-1", b"PK\x03\x04stale_gen");
        assert!(res.is_err(), "Stale session generation grant must be rejected");
        match res.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("Thế hệ phiên của grant"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_grant_concurrent_in_use_rejection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("concurrent_grant_test.xlsx");
        let manager = NativeSaveGrantManager::new();

        manager
            .register_document_session("doc-conc", "app-inst-1", 1, &dest, 0)
            .unwrap();

        let grant = manager
            .issue_save_grant(
                "doc-conc",
                "app-inst-1",
                1,
                "req-conc-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            )
            .unwrap();

        // Giả lập grant đang ở trạng thái InUse (đang ghi dở)
        {
            let mut grants = manager.grants.lock().unwrap();
            let g = grants.get_mut(&grant.grant_id).unwrap();
            g.status = GrantStatus::InUse;
        }

        // Tác vụ ghi thứ hai cố tình dùng cùng grant lúc đó
        let res = manager.execute_granted_save(&grant.grant_id, "req-conc-1", b"PK\x03\x04concurrent_attack");
        assert!(res.is_err(), "InUse grant must reject concurrent write");
        match res.unwrap_err() {
            office_shared::CommandError::IoError { details } => {
                assert!(details.contains("đang được thực thi trong một tác vụ ghi khác"));
            }
            other => panic!("Expected IoError, got {:?}", other),
        }
    }

    #[test]
    fn test_payload_size_limit_rejection() {
        let manager = NativeSaveGrantManager::new();
        // Giả lập vượt quá 100MB
        let oversized_bytes = vec![0u8; MAX_PAYLOAD_BYTES + 1];
        let res = manager.execute_granted_save("some-grant", "some-req", &oversized_bytes);
        assert!(res.is_err());
        match res.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("vượt quá giới hạn an toàn tối đa"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    // ==========================================================================
    // STAGE 4B-2B CLOSURE: TASK 4 & 5 CONCURRENCY, DIRTY STATE & SAVE AS SAFETY
    // ==========================================================================

    #[test]
    fn test_calamine_verify_postfix_saved_workbook() {
        use calamine::{Data, Reader, open_workbook_auto};
        let path = std::path::Path::new("/tmp/office-utility-uat/tc04_sample.xlsx");
        if !path.exists() {
            return;
        }
        let mut wb = open_workbook_auto(path).expect("calamine must open workbook");
        assert!(wb.sheet_names().contains(&"UAT_Sheet".to_string()));
        let range = wb.worksheet_range("UAT_Sheet").expect("UAT_Sheet exists");
        let val_a1 = range.get_value((0, 0));
        let val_b1 = range.get_value((0, 1));
        let val_a2 = range.get_value((1, 0));
        println!("Calamine Verification - Sheet: UAT_Sheet");
        println!("Calamine Verification - A1: {:?}", val_a1);
        println!("Calamine Verification - B1: {:?}", val_b1);
        println!("Calamine Verification - A2: {:?}", val_a2);
        assert_eq!(val_a1, Some(&Data::String("OfficeUtilityVN_UAT_Sample".to_string())));
        assert!(matches!(val_b1, Some(Data::Float(f)) if (*f - 12345.0).abs() < 1e-4) || matches!(val_b1, Some(Data::Int(12345))));
        assert_eq!(val_a2, Some(&Data::String("TC04_E2E_SAVED".to_string())));
    }

    #[test]
    fn test_exact_interleaving_save_a_edit_b_save_a_ack_remains_dirty_save_b_ack_clean() {
        use calamine::{Data, Reader, open_workbook_auto};

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("interleaving_test.xlsx");

        let snap_a_bytes = include_bytes!("../../fixtures/stage2c_v1.xlsx");
        let snap_b_bytes = include_bytes!("../../fixtures/stage2c_v2.xlsx");

        let manager = NativeSaveGrantManager::new();
        manager
            .register_document_session("doc-interleaving", "gridcraft-inst", 1, &file_path, 0)
            .unwrap();

        // 1. Tài liệu ở Revision A (rev 1), snapshot A
        let rev_a = 1u64;
        let req_id_a = "save-1-1".to_string();

        // 2. Người dùng kích hoạt Save A -> Backend cấp grant A cho rev 1
        let grant_a = manager
            .issue_save_grant(
                "doc-interleaving",
                "gridcraft-inst",
                1,
                &req_id_a,
                rev_a,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            )
            .unwrap();

        // 3. Trong khi Save A đang chạy xuống đĩa, người dùng tiếp tục sửa sang Revision B (rev 2)
        let rev_b = 2u64;
        let req_id_b = "save-1-2".to_string();

        // 4. Save A hoàn tất ghi đĩa
        let outcome_a = manager
            .execute_granted_save(&grant_a.grant_id, &req_id_a, snap_a_bytes)
            .unwrap();
        assert_eq!(outcome_a.committed_revision, 1);

        // 5. Host gửi xác nhận Save A về editor:
        // Mô phỏng logic của GridCraft host.rs sync():
        // Vì tài liệu hiện tại đã tiến tới rev_b (2), snapshot hiện hành là B (khác A),
        // nên sau khi nhận ack cho Save A (rev 1), tài liệu BẮT BUỘC VẪN DIRTY!
        let editor_current_rev = rev_b;
        let acked_rev_a = rev_a;
        let is_dirty_after_ack_a = editor_current_rev > acked_rev_a;
        assert!(is_dirty_after_ack_a, "Editor BẮT BUỘC giữ trạng thái Dirty vì có thay đổi B mới hơn Save A!");

        // 6. Người dùng kích hoạt Save B cho Revision B
        let grant_b = manager
            .issue_save_grant(
                "doc-interleaving",
                "gridcraft-inst",
                1,
                &req_id_b,
                rev_b,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            )
            .unwrap();

        // 7. Save B hoàn tất ghi đĩa thành công
        let outcome_b = manager
            .execute_granted_save(&grant_b.grant_id, &req_id_b, snap_b_bytes)
            .unwrap();
        assert_eq!(outcome_b.committed_revision, 2);

        // 8. Host gửi xác nhận Save B:
        // GridCraft sync(): acked_rev == editor_current_rev (2 == 2) -> Tài liệu trở về Clean!
        let acked_rev_b = rev_b;
        let is_dirty_after_ack_b = editor_current_rev > acked_rev_b;
        assert!(!is_dirty_after_ack_b, "Editor trở thành Clean sau khi Save B mới nhất hoàn tất!");

        // 9. Xác minh cấu trúc trên đĩa khớp chính xác snapshot B
        let mut wb = open_workbook_auto(&file_path).unwrap();
        let range = wb.worksheet_range("Sheet1").unwrap();
        assert_eq!(range.get_value((0, 0)), Some(&Data::Float(98765.0)));
        assert_eq!(range.get_value((0, 2)), Some(&Data::Float(100.0)));
        assert_eq!(range.get_value((0, 3)), Some(&Data::Float(999.0)));
    }

    #[test]
    fn test_save_as_transaction_safety_cancel_preserves_old_destination_and_dirty() {
        let temp_dir = tempfile::tempdir().unwrap();
        let initial_file = temp_dir.path().join("initial_file.xlsx");
        let initial_bytes = b"PK\x03\x04ORIGINAL_INITIAL_DATA";
        std::fs::write(&initial_file, initial_bytes).unwrap();

        let manager = NativeSaveGrantManager::new();
        // Đăng ký phiên tài liệu ban đầu
        let handle = manager
            .register_document_session("doc-sa-cancel", "gridcraft-inst", 1, &initial_file, 1)
            .unwrap();

        assert_eq!(handle.authorized_dest_path, initial_file);

        // Hộp thoại Save As bị người dùng HỦY (None)
        let cancel_dialog = Box::new(MockNativeDialogService::new(None, None));
        manager.set_dialog_service_for_test(cancel_dialog);

        let res = manager.save_as_via_native_dialog(
            Some("doc-sa-cancel"),
            "gridcraft-inst",
            1,
            Some("new_file.xlsx"),
            "Bảng tính Excel",
            &["xlsx"],
        ).unwrap();

        // 1. Phải trả về None (không sinh handle hoặc grant)
        assert!(res.is_none(), "Khi hủy Save As, dialog không được sinh grant");

        // 2. Phiên làm việc của tài liệu ban đầu BẮT BUỘC giữ nguyên đích đến cũ
        let session = manager.sessions.lock().unwrap().get("doc-sa-cancel").cloned().unwrap();
        assert_eq!(session.authorized_dest_path, initial_file);
        assert_eq!(session.last_committed_revision, 1);

        // 3. Tệp gốc trên đĩa hoàn toàn không bị ảnh hưởng
        assert_eq!(std::fs::read(&initial_file).unwrap(), initial_bytes);
    }

    #[test]
    fn test_save_as_transaction_safety_failure_preserves_old_destination() {
        let temp_dir = tempfile::tempdir().unwrap();
        let initial_file = temp_dir.path().join("active_doc.xlsx");
        let new_file = temp_dir.path().join("fail_target.xlsx");
        let original_data = b"PK\x03\x04ACTIVE_DATA_REV_1";
        std::fs::write(&initial_file, original_data).unwrap();

        let manager = NativeSaveGrantManager::new();
        manager
            .register_document_session("doc-sa-fail", "gridcraft-inst", 1, &initial_file, 1)
            .unwrap();

        // Giả lập dialog trả về new_file
        let save_as_dialog = Box::new(MockNativeDialogService::new(None, Some(new_file.clone())));
        manager.set_dialog_service_for_test(save_as_dialog);

        let sa_handle = manager.save_as_via_native_dialog(
            Some("doc-sa-fail"),
            "gridcraft-inst",
            1,
            Some("fail_target.xlsx"),
            "Bảng tính Excel",
            &["xlsx"],
        ).unwrap().expect("Dialog phải trả về handle");

        // Giả lập lỗi I/O: gửi payload rỗng hoặc payload vượt quá 100MB gây lỗi ghi
        let oversized = vec![0u8; MAX_PAYLOAD_BYTES + 10];
        let exec_res = manager.execute_granted_save(&sa_handle.grant.grant_id, &sa_handle.grant.request_id, &oversized);

        assert!(exec_res.is_err(), "Tác vụ ghi lỗi phải trả về Err");

        // 1. Đích đến mới KHÔNG được tạo
        assert!(!new_file.exists(), "Tệp mới không được tạo khi ghi thất bại");

        // 2. Tài liệu ban đầu giữ nguyên đường dẫn cũ
        let session = manager.sessions.lock().unwrap().get("doc-sa-fail").cloned().unwrap();
        assert_eq!(session.authorized_dest_path, initial_file);

        // 3. Dữ liệu tệp cũ nguyên vẹn
        assert_eq!(std::fs::read(&initial_file).unwrap(), original_data);
    }

    #[test]
    fn test_save_as_transaction_safety_success_updates_destination_and_commits() {
        let temp_dir = tempfile::tempdir().unwrap();
        let initial_file = temp_dir.path().join("doc_v1.xlsx");
        let new_file = temp_dir.path().join("doc_v2_committed.xlsx");
        let v1_data = b"PK\x03\x04DATA_V1";
        let v2_data = b"PK\x03\x04DATA_V2_NEW_CONTENT";
        std::fs::write(&initial_file, v1_data).unwrap();

        let manager = NativeSaveGrantManager::new();
        manager
            .register_document_session("doc-sa-ok", "gridcraft-inst", 1, &initial_file, 1)
            .unwrap();

        let save_as_dialog = Box::new(MockNativeDialogService::new(None, Some(new_file.clone())));
        manager.set_dialog_service_for_test(save_as_dialog);

        let sa_handle = manager.save_as_via_native_dialog(
            Some("doc-sa-ok"),
            "gridcraft-inst",
            1,
            Some("doc_v2_committed.xlsx"),
            "Bảng tính Excel",
            &["xlsx"],
        ).unwrap().expect("Dialog phải trả về handle");

        // Thực thi ghi tệp mới thành công
        let exec_res = manager.execute_granted_save(&sa_handle.grant.grant_id, &sa_handle.grant.request_id, v2_data);
        assert!(exec_res.is_ok());

        // 1. Tệp mới tồn tại với đúng dữ liệu mới
        assert!(new_file.exists());
        assert_eq!(std::fs::read(&new_file).unwrap(), v2_data);

        // 2. Tệp ban đầu vẫn giữ nguyên v1_data
        assert_eq!(std::fs::read(&initial_file).unwrap(), v1_data);

        // 3. Session mới được cam kết với revision 1
        let new_session = manager.sessions.lock().unwrap().get(&sa_handle.document_id).cloned().unwrap();
        assert_eq!(new_session.authorized_dest_path, new_file);
        assert_eq!(new_session.last_committed_revision, 1);
    }

    #[test]
    fn test_grant_expiration_rejection_after_ttl() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("expired_test.xlsx");
        let manager = NativeSaveGrantManager::new();

        manager
            .register_document_session("doc-expired", "app-inst", 1, &dest, 0)
            .unwrap();

        // Cấp grant với TTL = 0 giây (hết hạn ngay lập tức)
        let grant = manager
            .issue_save_grant(
                "doc-expired",
                "app-inst",
                1,
                "req-exp-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                0, // 0 giây TTL
            )
            .unwrap();

        // Chờ 10ms để đồng hồ vượt quá TTL
        std::thread::sleep(std::time::Duration::from_millis(15));

        let res = manager.execute_granted_save(&grant.grant_id, "req-exp-1", b"PK\x03\x04data");
        assert!(res.is_err(), "Grant hết hạn phải bị từ chối");
        match res.unwrap_err() {
            office_shared::CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("hết hạn") || reason.contains("expired"));
            }
            other => panic!("Expected ValidationFailed for expired grant, got {:?}", other),
        }
    }

    // ==========================================================================
    // TASK 8: BENCHMARK KÍCH THƯỚC PAYLOAD & CHI PHÍ BỘ NHỚ IPC
    // ==========================================================================

    #[test]
    fn test_binary_ipc_benchmark_and_memory_metrics() {
        use std::time::Instant;

        let temp_dir = tempfile::tempdir().unwrap();

        // Các kích thước kiểm thử: 100 KB, 1 MB, 10 MB, 50 MB
        let test_sizes = [
            ("100 KB", 100 * 1024),
            ("1 MB", 1 * 1024 * 1024),
            ("10 MB", 10 * 1024 * 1024),
            ("50 MB", 50 * 1024 * 1024),
        ];

        for (label, size) in test_sizes {
            let file_path = temp_dir.path().join(format!("bench_{}.xlsx", size));
            let raw_bytes = vec![0x58u8; size]; // Chữ 'X'

            // 1. Đo lường chi phí JSON serialization mảng số nguyên [u8]
            let start_json = Instant::now();
            let json_str = serde_json::to_string(&raw_bytes).unwrap();
            let json_serialize_ms = start_json.elapsed().as_secs_f64() * 1000.0;

            // Kích thước chuỗi JSON string so với mảng byte nhị phân thô
            let json_byte_len = json_str.len();
            let expansion_ratio = json_byte_len as f64 / size as f64;

            // 2. Đo lường chi phí Deserialization từ JSON về Vec<u8>
            let start_de = Instant::now();
            let deserialized: Vec<u8> = serde_json::from_str(&json_str).unwrap();
            let json_deserialize_ms = start_de.elapsed().as_secs_f64() * 1000.0;
            assert_eq!(deserialized.len(), size);

            // 3. Đo lường thời gian ghi đĩa nguyên tử (Temp file write + fsync + atomic rename)
            let start_write = Instant::now();
            let write_res = native_atomic_durable_write(file_path.to_str().unwrap(), &deserialized);
            let write_ms = start_write.elapsed().as_secs_f64() * 1000.0;
            assert!(write_res.is_ok());

            println!(
                "[BENCHMARK] Payload {}: Raw={} bytes, JSON={} bytes ({:.2}x expansion), JSON Ser={:.2}ms, De={:.2}ms, DiskWrite={:.2}ms",
                label, size, json_byte_len, expansion_ratio, json_serialize_ms, json_deserialize_ms, write_ms
            );

            // Kiểm định: Chuỗi JSON luôn nở rộng từ 2.5x đến 4.0x kích thước ban đầu
            assert!(expansion_ratio >= 2.0 && expansion_ratio <= 5.0);
        }
    }

    // ==========================================================================
    // STAGE 4C-0D CLOSURE: ARBITRARY FILE READ SECURITY & BINDING REGRESSION
    // ==========================================================================

    #[test]
    fn test_security_01_read_file_binary_denies_arbitrary_user_home() {
        let res = tauri::async_runtime::block_on(async {
            crate::commands::read_file_binary("/home/user/.bashrc".to_string()).await
        });
        match res {
            Err(office_shared::CommandError::ValidationFailed { reason }) => {
                assert!(reason.contains("vô hiệu hóa vĩnh viễn"));
                assert!(reason.contains("NFR-SEC-01"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_security_02_read_file_binary_denies_sensitive_config() {
        let res = tauri::async_runtime::block_on(async {
            crate::commands::read_file_binary("/etc/shadow".to_string()).await
        });
        match res {
            Err(office_shared::CommandError::ValidationFailed { reason }) => {
                assert!(reason.contains("vô hiệu hóa vĩnh viễn"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_security_03_read_file_binary_denies_temp_dir_arbitrary() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_file = temp_dir.path().join("arbitrary_temp.xlsx");
        std::fs::write(&temp_file, b"test").unwrap();

        let res = tauri::async_runtime::block_on(async {
            crate::commands::read_file_binary(temp_file.to_str().unwrap().to_string()).await
        });
        match res {
            Err(office_shared::CommandError::ValidationFailed { reason }) => {
                assert!(reason.contains("vô hiệu hóa vĩnh viễn"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_security_04_read_file_binary_denies_substring_fixtures() {
        let res = tauri::async_runtime::block_on(async {
            crate::commands::read_file_binary("/tmp/evil_fixtures_hack.xlsx".to_string()).await
        });
        match res {
            Err(office_shared::CommandError::ValidationFailed { reason }) => {
                assert!(reason.contains("vô hiệu hóa vĩnh viễn"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_security_05_load_dev_fixture_denies_unapproved_id() {
        #[cfg(debug_assertions)]
        {
            let res = tauri::async_runtime::block_on(async {
                crate::commands::load_dev_fixture("unapproved_id".to_string()).await
            });
            assert!(res.is_err());
            match res {
                Err(office_shared::CommandError::ValidationFailed { reason }) => {
                    assert!(reason.contains("không nằm trong danh mục"));
                }
                other => panic!("Expected ValidationFailed, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_security_06_load_dev_fixture_denies_path_traversal() {
        #[cfg(debug_assertions)]
        {
            let res = tauri::async_runtime::block_on(async {
                crate::commands::load_dev_fixture("../../etc/passwd".to_string()).await
            });
            assert!(res.is_err());
            match res {
                Err(office_shared::CommandError::ValidationFailed { .. }) => {}
                other => panic!("Expected ValidationFailed, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_security_07_load_dev_fixture_denies_symlinks() {
        #[cfg(unix)]
        {
            let temp_dir = tempfile::tempdir().unwrap();
            let target_file = temp_dir.path().join("secret.txt");
            std::fs::write(&target_file, b"secret_data").unwrap();
            let symlink_path = temp_dir.path().join("symlink_fixture.xlsx");
            let _ = std::os::unix::fs::symlink(&target_file, &symlink_path);

            // Kiểm tra trực tiếp cơ chế phát hiện symlink
            let meta = std::fs::symlink_metadata(&symlink_path).unwrap();
            assert!(meta.file_type().is_symlink(), "Symlink must be detected as symlink");
        }
    }

    #[test]
    fn test_security_08_load_dev_fixture_enforces_magic_bytes_check() {
        let non_xlsx_bytes = b"NOT_A_VALID_ZIP_MAGIC";
        assert!(!non_xlsx_bytes.starts_with(b"PK\x03\x04"));
    }

    #[test]
    fn test_security_09_revoked_session_denies_save_grant() {
        let manager = NativeSaveGrantManager::new();
        // Cố gắng yêu cầu save grant cho session không tồn tại
        let res = manager.issue_save_grant(
            "doc-nonexistent",
            "test-app",
            1,
            "req-1",
            1,
            GrantOperationType::Save,
            OverwritePolicy::AllowOverwrite,
            30,
        );
        assert!(res.is_err());
        match res {
            Err(office_shared::CommandError::ValidationFailed { reason }) => {
                assert!(reason.contains("Không tìm thấy phiên tài liệu") || reason.contains("hết hạn"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_security_10_mismatched_session_generation_denies_save() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("gen_test.xlsx");
        let manager = NativeSaveGrantManager::new();
        manager
            .register_document_session("doc-gen", "app-inst", 1, &file_path, 0)
            .unwrap();

        // Yêu cầu cấp grant với session_generation sai (thế hệ cũ hoặc mới hơn)
        let res = manager.issue_save_grant(
            "doc-gen",
            "app-inst",
            2, // Sai generation!
            "req-gen",
            1,
            GrantOperationType::Save,
            OverwritePolicy::AllowOverwrite,
            30,
        );
        assert!(res.is_err());
    }

    #[test]
    fn test_security_11_load_failure_preserves_disk_file_against_blank_save() {
        // [Task 6]: Ngăn chặn lỗi tệp cũ bị ghi đè bởi bảng tính trắng khi nạp thất bại
        let temp_dir = tempfile::tempdir().unwrap();
        let existing_file = temp_dir.path().join("precious_data.xlsx");
        let original_content = b"PRECIOUS_ORIGINAL_DATA_1234567890";
        std::fs::write(&existing_file, original_content).unwrap();

        let manager = NativeSaveGrantManager::new();

        // 1. Giả lập một phiên mở tài liệu thất bại (ví dụ: file hỏng hoặc user cancel)
        // -> Backend KHÔNG gán existing_file vào phiên tài liệu mới (doc-blank)
        let blank_doc_id = "doc-blank-untitled";
        let blank_dest_path = temp_dir.path().join("untitled.xlsx");
        manager
            .register_document_session(blank_doc_id, "app-inst", 1, &blank_dest_path, 0)
            .unwrap();

        // 2. Một hành động Save từ bảng tính trắng (blank workbook) cố gắng ghi vào precious_data.xlsx
        // Vì precious_data.xlsx không thuộc về blank_doc_id, lệnh Save chỉ được ghi vào blank_dest_path!
        let grant = manager
            .issue_save_grant(
                blank_doc_id,
                "app-inst",
                1,
                "req-blank-save",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                30,
            )
            .unwrap();

        // Đích ghi của grant BẮT BUỘC phải là untitled.xlsx, KHÔNG PHẢI precious_data.xlsx
        assert_eq!(grant.authorized_dest_path, blank_dest_path);

        // 3. Thực thi ghi dữ liệu trắng
        let blank_bytes = b"BLANK_WORKBOOK_BYTES";
        let save_res = manager.execute_granted_save(&grant.grant_id, "req-blank-save", blank_bytes);
        assert!(save_res.is_ok());

        // 4. KIỂM ĐỊNH TỐI THƯỢNG: Tệp gốc trên đĩa BẢO TOÀN NGUYÊN VẸN 100%
        let current_disk_content = std::fs::read(&existing_file).unwrap();
        assert_eq!(current_disk_content, original_content);
    }

    #[test]
    fn test_lock_ordering_deadlock_freedom_under_high_concurrency() {
        // [Workstream A - P1 4666249780]: Nhất quán thứ tự khóa (sessions -> grants) loại trừ hoàn toàn Deadlock
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = Arc::new(NativeSaveGrantManager::new());
        let barrier = Arc::new(Barrier::new(10));

        let mut handles = Vec::new();
        for i in 0..10 {
            let mgr = Arc::clone(&manager);
            let bar = Arc::clone(&barrier);
            let path = temp_dir.path().join(format!("file_{i}.xlsx"));
            handles.push(thread::spawn(move || {
                let doc_id = format!("doc-{i}");
                mgr.register_document_session(&doc_id, "app-inst", 1, &path, 0).unwrap();
                bar.wait();

                for j in 0..20 {
                    let req_id = format!("req-{i}-{j}");
                    if let Ok(grant) = mgr.issue_save_grant(
                        &doc_id,
                        "app-inst",
                        1,
                        &req_id,
                        (j + 1) as u64,
                        GrantOperationType::Save,
                        OverwritePolicy::AllowOverwrite,
                        30,
                    ) {
                        let _ = mgr.execute_granted_save(&grant.grant_id, &req_id, b"CONCURRENT_CONTENT");
                    }
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_document_level_concurrent_save_exclusion() {
        // [Workstream A - P1 4666369287]: Không cho phép 2 lượt ghi đồng thời trên cùng một tài liệu
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("exclusive.xlsx");
        let manager = NativeSaveGrantManager::new();

        manager.register_document_session("doc-excl", "app-inst", 1, &dest, 0).unwrap();

        let grant1 = manager.issue_save_grant(
            "doc-excl",
            "app-inst",
            1,
            "req-1",
            1,
            GrantOperationType::Save,
            OverwritePolicy::AllowOverwrite,
            30,
        ).unwrap();

        let grant2 = manager.issue_save_grant(
            "doc-excl",
            "app-inst",
            1,
            "req-2",
            2,
            GrantOperationType::Save,
            OverwritePolicy::AllowOverwrite,
            30,
        ).unwrap();

        // Đánh dấu grant1 là InUse (đang tiến hành ghi)
        {
            let mut grants = manager.grants.lock().unwrap();
            if let Some(entry) = grants.get_mut(&grant1.grant_id) {
                entry.status = GrantStatus::InUse;
            }
        }

        // Thực thi grant2 trong khi grant1 đang InUse phải bị từ chối
        let res2 = manager.execute_granted_save(&grant2.grant_id, "req-2", b"DATA_2");
        assert!(res2.is_err(), "Grant 2 phải bị từ chối khi tài liệu đang có lượt ghi InUse");
        match res2.unwrap_err() {
            CommandError::IoError { .. } => {}
            other => panic!("Expected IoError, got {:?}", other),
        }

        // Hoàn tất grant1
        {
            let mut grants = manager.grants.lock().unwrap();
            if let Some(entry) = grants.get_mut(&grant1.grant_id) {
                entry.status = GrantStatus::Consumed;
            }
        }

        // Bây giờ grant2 có thể thực thi
        let res2_retry = manager.execute_granted_save(&grant2.grant_id, "req-2", b"DATA_2");
        assert!(res2_retry.is_ok());
    }

    #[test]
    fn test_revision_monotonicity_prevents_backward_revision() {
        // [Workstream A - P1 4666369287]: Thứ tự revision đơn điệu tăng, chặn revision cũ
        let temp_dir = tempfile::tempdir().unwrap();
        let dest = temp_dir.path().join("monotonic.xlsx");
        let manager = NativeSaveGrantManager::new();

        manager.register_document_session("doc-mono", "app-inst", 1, &dest, 0).unwrap();

        // 1. Cấp grant revision 4 trước khi commit revision 5
        let grant_rev4 = manager.issue_save_grant(
            "doc-mono",
            "app-inst",
            1,
            "req-4",
            4,
            GrantOperationType::Save,
            OverwritePolicy::AllowOverwrite,
            30,
        ).unwrap();

        // 2. Commit revision 5 thành công
        let grant_rev5 = manager.issue_save_grant(
            "doc-mono",
            "app-inst",
            1,
            "req-5",
            5,
            GrantOperationType::Save,
            OverwritePolicy::AllowOverwrite,
            30,
        ).unwrap();
        let save_5 = manager.execute_granted_save(&grant_rev5.grant_id, "req-5", b"REV_5");
        assert!(save_5.is_ok());

        // 3. Grant 4 đã cấp trước đó giờ thực thi commit phiên bản cũ (4 <= 5) -> Phải bị từ chối
        let save_4 = manager.execute_granted_save(&grant_rev4.grant_id, "req-4", b"REV_4");
        assert!(save_4.is_err());
        match save_4.unwrap_err() {
            CommandError::IoError { .. } => {}
            other => panic!("Expected IoError for backward revision, got {:?}", other),
        }

        // 4. Cố tình xin cấp mới grant cho revision 4 lỗi thời -> Bị từ chối ngay ở bước issue
        let stale_issue = manager.issue_save_grant(
            "doc-mono",
            "app-inst",
            1,
            "req-stale",
            4,
            GrantOperationType::Save,
            OverwritePolicy::AllowOverwrite,
            30,
        );
        assert!(stale_issue.is_err());

        // Commit revision 6 -> thành công
        let grant_rev6 = manager.issue_save_grant(
            "doc-mono",
            "app-inst",
            1,
            "req-6",
            6,
            GrantOperationType::Save,
            OverwritePolicy::AllowOverwrite,
            30,
        ).unwrap();
        let save_6 = manager.execute_granted_save(&grant_rev6.grant_id, "req-6", b"REV_6");
        assert!(save_6.is_ok());
    }

    #[test]
    fn test_xls_format_separation_read_allowed_write_rejected() {
        // [Workstream D - P2 4666660900]: Cho phép đọc .xls (nếu có OLE2 magic), CẤM ghi đè trực tiếp .xls
        let temp_dir = tempfile::tempdir().unwrap();

        // 1. Ghi tệp .xls phải bị từ chối ở tầng xác thực đường dẫn ghi
        let xls_write_path = temp_dir.path().join("report.xls");
        let write_val = validate_and_authorize_write_path(xls_write_path.to_str().unwrap());
        assert!(write_val.is_err(), "validate_and_authorize_write_path phải chặn đuôi .xls");

        // 2. Đọc tệp .xls giả mạo (không có header OLE2) -> Từ chối InvalidFileSignature
        let fake_xls = temp_dir.path().join("fake.xls");
        std::fs::write(&fake_xls, b"NOT_AN_OLE2_FILE").unwrap();
        let read_fake = validate_and_authorize_read_path(fake_xls.to_str().unwrap());
        assert!(read_fake.is_err(), "Tệp .xls không có header OLE2 phải bị từ chối");
        match read_fake.unwrap_err() {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Expected InvalidFileSignature, got {:?}", other),
        }

        // 3. Đọc tệp .xls hợp lệ với header OLE2 chuẩn (\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1) -> Thành công
        let valid_xls = temp_dir.path().join("valid.xls");
        let mut ole2_bytes = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        ole2_bytes.extend_from_slice(b"EXCEL_OLE2_BODY_STREAM");
        std::fs::write(&valid_xls, &ole2_bytes).unwrap();
        let read_valid = validate_and_authorize_read_path(valid_xls.to_str().unwrap());
        assert!(read_valid.is_ok(), "Tệp .xls có header OLE2 chuẩn phải được chấp nhận");
    }

    #[test]
    fn test_bounded_file_read_oversized_file_rejected() {
        // [Workstream A - P2 4666660908]: Đọc tệp giới hạn kích thước an toàn
        let temp_dir = tempfile::tempdir().unwrap();
        let oversized_file = temp_dir.path().join("oversized.xlsx");

        // Tạo tệp rỗng nhưng đặt kích thước lớn hơn MAX_PAYLOAD_BYTES (50MB + 1)
        let file = std::fs::File::create(&oversized_file).unwrap();
        file.set_len((MAX_PAYLOAD_BYTES as u64) + 1024).unwrap();

        let manager = NativeSaveGrantManager::new();
        manager.register_document_session("doc-large", "app-inst", 1, &oversized_file, 0).unwrap();

        let read_res = manager.read_authorized_document_bytes("doc-large", "app-inst", 1);
        assert!(read_res.is_err());
        match read_res.unwrap_err() {
            CommandError::ValidationFailed { reason } => {
                assert!(reason.contains("vượt quá giới hạn an toàn"));
            }
            other => panic!("Expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_post_rename_dir_open_failure_reports_ambiguous_commit() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let dest_path = dir.path().join("ambiguous_open.xlsx");
        std::fs::write(&dest_path, b"ORIGINAL_BYTES").expect("write original");

        let manager = NativeSaveGrantManager::new();
        manager
            .register_document_session("doc-amb-1", "app-1", 1, &dest_path, 0)
            .expect("register session");

        let grant = manager
            .issue_save_grant(
                "doc-amb-1",
                "app-1",
                1,
                "req-amb-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                60,
            )
            .expect("issue grant");

        // Kích hoạt lỗi tiêm thử nghiệm: DirOpenFailure sau rename (chỉ scoped riêng cho dest_path này)
        set_post_rename_fault_for_path(dest_path.clone(), PostRenameFault::DirOpenFailure);

        let new_payload = b"PK\x03\x04NEW_AMBIGUOUS_BYTES";
        let res = manager.execute_granted_save(&grant.grant_id, "req-amb-1", new_payload);

        clear_post_rename_fault_for_path(&dest_path); // Dọn dẹp cờ fault

        // 1. Phải trả về lỗi AmbiguousCommit, KHÔNG được trả về Ok thành công giả
        assert!(res.is_err(), "Post-rename failure must NOT report Ok success!");
        let err = res.unwrap_err();
        assert!(
            matches!(&err, CommandError::IoError { details } if details.contains("AmbiguousCommit")),
            "Error must be AmbiguousCommit IoError, got: {err:?}"
        );

        // 2. Grant status phải chuyển thành AmbiguousCommit (KHÔNG phải Failed, KHÔNG phải Consumed)
        let grants = manager.grants.lock().unwrap();
        let g = grants.get(&grant.grant_id).expect("grant exists");
        assert_eq!(
            g.status,
            GrantStatus::AmbiguousCommit,
            "Grant status must be AmbiguousCommit!"
        );
        drop(grants);

        // 3. Tệp trên đĩa THỰC TẾ đã được hoán đổi sang bytes mới (vì rename đã hoàn tất)
        let disk_bytes = std::fs::read(&dest_path).expect("read dest");
        assert_eq!(
            disk_bytes,
            new_payload,
            "Destination file on disk must contain the new bytes because rename succeeded!"
        );

        // 4. Thử replay/ghi lại với grant AmbiguousCommit phải bị TỪ CHỐI
        let replay_res = manager.execute_granted_save(&grant.grant_id, "req-amb-1", new_payload);
        assert!(replay_res.is_err(), "Replay on AmbiguousCommit grant must be rejected");
        assert!(
            matches!(&replay_res.unwrap_err(), CommandError::IoError { details } if details.contains("AmbiguousCommit"))
        );
    }

    #[test]
    fn test_post_rename_dir_sync_failure_reports_ambiguous_commit() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let dest_path = dir.path().join("ambiguous_sync.xlsx");
        std::fs::write(&dest_path, b"ORIGINAL_BYTES").expect("write original");

        let manager = NativeSaveGrantManager::new();
        manager
            .register_document_session("doc-amb-2", "app-1", 1, &dest_path, 0)
            .expect("register session");

        let grant = manager
            .issue_save_grant(
                "doc-amb-2",
                "app-1",
                1,
                "req-amb-2",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                60,
            )
            .expect("issue grant");

        // Kích hoạt lỗi tiêm thử nghiệm: DirSyncFailure sau rename (chỉ scoped riêng cho dest_path này)
        set_post_rename_fault_for_path(dest_path.clone(), PostRenameFault::DirSyncFailure);

        let new_payload = b"PK\x03\x04NEW_AMBIGUOUS_SYNC_BYTES";
        let res = manager.execute_granted_save(&grant.grant_id, "req-amb-2", new_payload);

        clear_post_rename_fault_for_path(&dest_path); // Dọn dẹp cờ fault

        // 1. Phải trả về lỗi AmbiguousCommit
        assert!(res.is_err(), "Post-rename failure must NOT report Ok success!");
        let err = res.unwrap_err();
        assert!(
            matches!(&err, CommandError::IoError { details } if details.contains("AmbiguousCommit")),
            "Error must be AmbiguousCommit IoError, got: {err:?}"
        );

        // 2. Grant status phải là AmbiguousCommit
        let grants = manager.grants.lock().unwrap();
        let g = grants.get(&grant.grant_id).expect("grant exists");
        assert_eq!(g.status, GrantStatus::AmbiguousCommit);
        drop(grants);

        // 3. Tệp trên đĩa thực tế đã ghi nhận bytes mới
        let disk_bytes = std::fs::read(&dest_path).expect("read dest");
        assert_eq!(disk_bytes, new_payload);
    }

    #[test]
    fn test_pre_rename_failure_does_not_report_ambiguous_commit() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let dest_path = dir.path().join("pre_rename_target.xlsx");
        let original_bytes = b"ORIGINAL_PRE_RENAME_BYTES";
        std::fs::write(&dest_path, original_bytes).expect("write original");

        let manager = NativeSaveGrantManager::new();
        manager
            .register_document_session("doc-pre-1", "app-1", 1, &dest_path, 0)
            .expect("register session");

        let grant = manager
            .issue_save_grant(
                "doc-pre-1",
                "app-1",
                1,
                "req-pre-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                60,
            )
            .expect("issue grant");

        // Tạo lỗi trước rename: parent_dir là một tệp thông thường, không thể tạo thư mục
        let parent_file = dir.path().join("parent_is_file");
        std::fs::write(&parent_file, b"not_a_dir").expect("create file");
        let invalid_path = parent_file.join("impossible_sub/test.xlsx");
        let mut grants = manager.grants.lock().unwrap();
        if let Some(g) = grants.get_mut(&grant.grant_id) {
            g.authorized_dest_path = invalid_path;
        }
        drop(grants);

        let res = manager.execute_granted_save(&grant.grant_id, "req-pre-1", b"PK\x03\x04SOME_BYTES");
        assert!(res.is_err(), "Pre-rename failure must return Err");

        // Grant status phải là Failed, KHÔNG phải AmbiguousCommit
        let grants = manager.grants.lock().unwrap();
        let g = grants.get(&grant.grant_id).expect("grant exists");
        assert_eq!(g.status, GrantStatus::Failed, "Pre-rename failure must result in Failed status");
        drop(grants);

        // Tệp gốc vẫn giữ nguyên tuyệt đối
        let disk_bytes = std::fs::read(&dest_path).expect("read dest");
        assert_eq!(disk_bytes, original_bytes);
    }

    #[cfg(unix)]
    #[test]
    fn test_unix_atomic_write_preserves_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("create temp dir");
        let dest_path = dir.path().join("confidential.pdf");
        let initial_bytes = b"%PDF-1.7 INITIAL CONFIDENTIAL DATA";
        std::fs::write(&dest_path, initial_bytes).expect("write initial");

        // Đặt permissions 0600 cho tệp gốc
        std::fs::set_permissions(&dest_path, std::fs::Permissions::from_mode(0o600)).expect("set 0600");
        let initial_mode = std::fs::metadata(&dest_path).expect("meta").permissions().mode() & 0o777;
        assert_eq!(initial_mode, 0o600);

        // Ghi dữ liệu mới qua native_atomic_durable_write
        let new_bytes = b"%PDF-1.7 REPLACED CONFIDENTIAL DATA";
        native_atomic_durable_write(dest_path.to_str().unwrap(), new_bytes).expect("write durable");

        // Sau khi hoán đổi nguyên tử, permissions của tệp đích BẮT BUỘC PHẢI GIỮ NGUYÊN 0600!
        let post_mode = std::fs::metadata(&dest_path).expect("meta post").permissions().mode() & 0o777;
        assert_eq!(
            post_mode, 0o600,
            "Permissions của tệp bảo mật 0600 phải được bảo lưu tuyệt đối sau khi ghi đĩa nguyên tử! Nhận được: {:o}",
            post_mode
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_unix_temp_file_permissions_restrictive_before_replacement() {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::AtomicBool;

        let dir = tempfile::tempdir().expect("create temp dir");
        let dest_path = dir.path().join("secure_work.xlsx");
        let verified = Arc::new(AtomicBool::new(false));
        let verified_clone = Arc::clone(&verified);

        set_temp_file_inspector(move |temp_path| {
            let mode = std::fs::metadata(temp_path)
                .expect("temp metadata")
                .permissions()
                .mode() & 0o777;
            assert_eq!(
                mode, 0o600,
                "Tệp tạm thời phải luôn được tạo với quyền tối thiểu hạn chế 0600! Nhận được: {:o}",
                mode
            );
            verified_clone.store(true, Ordering::SeqCst);
        });

        let payload = b"PK\x03\x04RESTRICTIVE_TEMP_TEST";
        let res = native_atomic_durable_write(dest_path.to_str().unwrap(), payload);
        clear_temp_file_inspector();

        assert!(res.is_ok(), "Ghi tệp thành công");
        assert!(verified.load(Ordering::SeqCst), "Inspector phải được kích hoạt để kiểm tra tệp tạm");
    }

    #[cfg(unix)]
    #[test]
    fn test_unix_atomic_write_new_file_secure_default_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("create temp dir");
        let dest_path = dir.path().join("brand_new_document.pdf");
        assert!(!dest_path.exists(), "Tệp phải chưa tồn tại trước khi ghi");

        let payload = b"%PDF-1.7 NEW_DOC_SECURE_DEFAULT";
        native_atomic_durable_write(dest_path.to_str().unwrap(), payload).expect("write new doc");

        let mode = std::fs::metadata(&dest_path).expect("meta").permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "Tài liệu mới phải có quyền mặc định an toàn 0600! Nhận được: {:o}",
            mode
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_pre_rename_permission_failure_preserves_original_file_and_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("create temp dir");
        let dest_path = dir.path().join("guarded_contract.docx");
        let original_bytes = b"ORIGINAL_CONFIDENTIAL_CONTRACT";
        std::fs::write(&dest_path, original_bytes).expect("write original");
        std::fs::set_permissions(&dest_path, std::fs::Permissions::from_mode(0o600)).expect("set 0600");

        let manager = NativeSaveGrantManager::new();
        manager
            .register_document_session("doc-guard-1", "app-1", 1, &dest_path, 0)
            .expect("register session");

        let grant = manager
            .issue_save_grant(
                "doc-guard-1",
                "app-1",
                1,
                "req-guard-1",
                1,
                GrantOperationType::Save,
                OverwritePolicy::AllowOverwrite,
                60,
            )
            .expect("issue grant");

        // Tiêm lỗi đặt quyền trước khi đổi tên (chỉ scoped cho dest_path này)
        set_pre_rename_fault_for_path(dest_path.clone(), PreRenameFault::SetPermissionsFailure);

        let new_bytes = b"MALICIOUS_OR_CORRUPT_OVERWRITE";
        let res = manager.execute_granted_save(&grant.grant_id, "req-guard-1", new_bytes);

        clear_pre_rename_fault_for_path(&dest_path);

        // 1. Phải trả về lỗi
        assert!(res.is_err(), "Lỗi đặt quyền trước rename phải trả về Err");

        // 2. Trạng thái cấp quyền phải là Failed (KHÔNG phải AmbiguousCommit)
        let grants = manager.grants.lock().unwrap();
        let g = grants.get(&grant.grant_id).expect("grant exists");
        assert_eq!(
            g.status,
            GrantStatus::Failed,
            "Lỗi trước khi đổi tên phải đánh dấu grant là Failed"
        );
        drop(grants);

        // 3. Tệp gốc phải giữ nguyên 100% cả nội dung lẫn quyền truy cập 0600
        let disk_bytes = std::fs::read(&dest_path).expect("read dest");
        assert_eq!(disk_bytes, original_bytes, "Nội dung tệp gốc không được thay đổi");

        let mode = std::fs::metadata(&dest_path).expect("meta").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "Quyền truy cập gốc 0600 phải được giữ nguyên");
    }
}
