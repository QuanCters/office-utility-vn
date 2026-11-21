#[cfg(unix)]
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: wasm_bindgen::JsValue) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Báo cáo chi tiết các mốc an toàn của thao tác ghi đĩa bền vững.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistenceReport {
    /// Đường dẫn tệp đích thực tế.
    pub dest_path: String,
    /// Số lượng byte đã ghi thành công.
    pub bytes_written: usize,
    /// Mã băm xác thực nội dung tệp đã ghi (hex).
    pub fingerprint: String,
    /// Mốc 1: Ghi hoàn tất vào bộ đệm tầng OS (Write Completion).
    pub write_completed: bool,
    /// Mốc 2: Đồng bộ hóa nội dung và siêu dữ liệu xuống controller đĩa (fsync/sync_all).
    pub file_synced: bool,
    /// Mốc 3: Hoán đổi nguyên tử (Atomic Replacement qua rename).
    pub atomically_replaced: bool,
    /// Mốc 4: Đồng bộ hóa thư mục cha để đảm bảo tính bền vững khi mất điện (Directory Crash Durability).
    pub dir_synced: bool,
}

/// Các lỗi phát sinh trong quá trình lưu trữ đĩa bền vững.
#[derive(Debug)]
pub enum PersistenceError {
    TargetDirectoryNotFound(String),
    TempFileCreate { path: PathBuf, source: io::Error },
    WriteAll { path: PathBuf, source: io::Error },
    SyncAll { path: PathBuf, source: io::Error },
    AtomicRename { from: PathBuf, to: PathBuf, source: io::Error },
    ParentDirSync { path: PathBuf, source: io::Error },
    SecurityViolation(String),
    StaleWriteIgnored { attempted_seq: u64, current_seq: u64 },
    IpcError(String),
    Io(io::Error),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TargetDirectoryNotFound(p) => write!(f, "Thư mục đích không tồn tại: {p}"),
            Self::TempFileCreate { path, source } => {
                write!(f, "Không thể tạo tệp tạm '{}': {source}", path.display())
            }
            Self::WriteAll { path, source } => {
                write!(f, "Không thể ghi dữ liệu vào tệp tạm '{}': {source}", path.display())
            }
            Self::SyncAll { path, source } => {
                write!(f, "Không thể đồng bộ fsync tệp tạm '{}': {source}", path.display())
            }
            Self::AtomicRename { from, to, source } => {
                write!(
                    f,
                    "Không thể hoán đổi nguyên tử từ '{}' sang '{}': {source}",
                    from.display(),
                    to.display()
                )
            }
            Self::ParentDirSync { path, source } => {
                write!(f, "Không thể đồng bộ thư mục cha '{}': {source}", path.display())
            }
            Self::SecurityViolation(msg) => write!(f, "Vi phạm an toàn bảo mật đường dẫn: {msg}"),
            Self::StaleWriteIgnored { attempted_seq, current_seq } => {
                write!(
                    f,
                    "Lượt ghi lỗi thời bị từ chối: phiên ghi {attempted_seq} đã bị vượt qua bởi phiên {current_seq}"
                )
            }
            Self::IpcError(msg) => write!(f, "Lỗi giao tiếp IPC tầng lưu trữ: {msg}"),
            Self::Io(e) => write!(f, "Lỗi I/O: {e}"),
        }
    }
}

impl std::error::Error for PersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TempFileCreate { source, .. } => Some(source),
            Self::WriteAll { source, .. } => Some(source),
            Self::SyncAll { source, .. } => Some(source),
            Self::AtomicRename { source, .. } => Some(source),
            Self::ParentDirSync { source, .. } => Some(source),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

/// Trạng thái phiên ghi của một đường dẫn đích cụ thể.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DestinationState {
    pub last_issued_seq: u64,
    pub last_committed_seq: u64,
}

/// Trình theo dõi và tuần tự hóa ghi đĩa theo đích đến (Destination-Scoped Save Tracker).
/// Bảo đảm các yêu cầu lưu vào cùng một tệp đích được cấp số thứ tự đơn điệu tăng dần.
/// Bất kỳ yêu cầu lưu nào hoàn tất trễ hơn một lượt lưu mới hơn sẽ bị từ chối trước bước rename,
/// loại trừ hoàn toàn nguy cơ rollback âm thầm trên hệ thống tệp.
#[derive(Debug, Default)]
pub struct DestinationSaveTracker {
    inner: Mutex<HashMap<PathBuf, DestinationState>>,
}

/// Vé lưu đĩa đại diện cho một phiên ghi đã được đăng ký thứ tự.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicket {
    pub dest_path: PathBuf,
    pub seq: u64,
}

/// Chuẩn hóa đường dẫn đích để các đường dẫn tương đối và tuyệt đối cùng trỏ về một khóa duy nhất.
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

impl DestinationSaveTracker {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Đăng ký cấp vé lưu đĩa đơn điệu tăng dần cho đường dẫn đích (đã chuẩn hóa).
    pub fn reserve(&self, dest_path: &Path) -> WriteTicket {
        let normalized = canonicalize_destination_path(dest_path);
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let state = map.entry(normalized.clone()).or_default();
        state.last_issued_seq = state.last_issued_seq.wrapping_add(1);
        let seq = state.last_issued_seq;
        WriteTicket {
            dest_path: normalized,
            seq,
        }
    }

    /// Kiểm tra xem vé lưu này có còn quyền commit hay không.
    /// Nếu số thứ tự seq <= last_committed_seq, lượt ghi này đã bị một lượt lưu mới hơn
    /// cam kết trước -> trả về lỗi StaleWriteIgnored.
    pub fn can_commit(&self, ticket: &WriteTicket) -> Result<(), PersistenceError> {
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = map.get(&ticket.dest_path) {
            if ticket.seq <= state.last_committed_seq {
                return Err(PersistenceError::StaleWriteIgnored {
                    attempted_seq: ticket.seq,
                    current_seq: state.last_committed_seq,
                });
            }
        }
        Ok(())
    }

    /// Đánh dấu vé lưu đã commit thành công xuống đĩa.
    pub fn record_committed(&self, ticket: &WriteTicket) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let state = map.entry(ticket.dest_path.clone()).or_default();
        if ticket.seq > state.last_committed_seq {
            state.last_committed_seq = ticket.seq;
        }
    }

    /// Điều phối commit nguyên tử dưới khóa Mutex theo đường dẫn đích (Stage 2C Closure):
    /// Thực hiện chuỗi ba thao tác:
    /// 1. Kiểm tra tính hợp lệ của thứ tự commit (ticket.seq > last_committed_seq).
    /// 2. Thực thi hàm commit (hoán đổi tệp std::fs::rename).
    /// 3. Cập nhật last_committed_seq = ticket.seq.
    /// Chuỗi thao tác này diễn ra nguyên tử dưới cùng một lần giữ lock, loại trừ 100%
    /// khe hở chạy đua (TOCTOU race) giữa các luồng ghi đồng thời vào cùng một đích đến.
    pub fn commit_atomic<F, R>(&self, ticket: &WriteTicket, commit_fn: F) -> Result<R, PersistenceError>
    where
        F: FnOnce() -> Result<R, PersistenceError>,
    {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let state = map.entry(ticket.dest_path.clone()).or_default();
        if ticket.seq <= state.last_committed_seq {
            return Err(PersistenceError::StaleWriteIgnored {
                attempted_seq: ticket.seq,
                current_seq: state.last_committed_seq,
            });
        }
        let res = commit_fn()?;
        state.last_committed_seq = ticket.seq;
        Ok(res)
    }

    /// Lấy trạng thái hiện tại của một đích đến để kiểm tra và kiểm thử.
    pub fn get_state(&self, dest_path: &Path) -> Option<DestinationState> {
        let normalized = canonicalize_destination_path(dest_path);
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.get(&normalized).cloned()
    }
}

/// Trình theo dõi ghi lưu trữ toàn cục mặc định.
pub static GLOBAL_SAVE_TRACKER: LazyLock<DestinationSaveTracker> = LazyLock::new(DestinationSaveTracker::new);

/// Kiểm tra tính an toàn của đường dẫn lưu tệp (Path Traversal & Sanitization Guard).
pub fn validate_destination_path(path_str: &str) -> Result<PathBuf, PersistenceError> {
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return Err(PersistenceError::SecurityViolation("Đường dẫn tệp không được để trống".into()));
    }
    if trimmed.contains('\0') {
        return Err(PersistenceError::SecurityViolation("Đường dẫn tệp chứa ký tự null không hợp lệ".into()));
    }
    let p = Path::new(trimmed);
    for component in p.components() {
        if let Component::ParentDir = component {
            return Err(PersistenceError::SecurityViolation(
                format!("Phát hiện nguy cơ Path Traversal ('..') trong đường dẫn: {path_str}")
            ));
        }
    }
    // Xác thực phần mở rộng tệp được phép
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if !matches!(ext.as_str(), "xlsx" | "xlsm" | "csv" | "tsv" | "json" | "ods" | "pdf") {
        return Err(PersistenceError::SecurityViolation(
            format!("Phần mở rộng tệp '.{ext}' không được hỗ trợ hoặc không an toàn")
        ));
    }
    Ok(p.to_path_buf())
}

/// Tính mã băm nội dung nhanh và nhất quán (64-bit FNV-1a hex).
pub fn compute_fingerprint(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Thực hiện quy trình ghi đĩa an toàn, bền vững và nguyên tử:
/// 1. Kiểm tra an toàn bảo mật đường dẫn đích (Path Traversal Guard).
/// 2. Cấp vé thứ tự lưu đơn điệu (Destination-Scoped Monotonic Sequence).
/// 3. Tạo tệp tạm nằm CÙNG thư mục cha với tệp đích.
/// 4. Ghi toàn bộ dữ liệu (Write Completion).
/// 5. Gọi fsync/sync_all để xả bộ đệm phần cứng (File Synchronization).
/// 6. Xác nhận vé lưu không bị lỗi thời (Stale Check) trước khi hoán đổi.
/// 7. Đóng tệp tạm và thực thi std::fs::rename() nguyên tử thay thế tệp đích.
/// 8. Trên Unix/Linux, mở thư mục cha và gọi fsync thư mục (Directory Crash Durability).
/// 9. Tự động dọn dẹp tệp tạm nếu có lỗi xảy ra ở bất kỳ bước nào.
pub fn atomic_durable_write(dest_path: &Path, bytes: &[u8]) -> Result<PersistenceReport, PersistenceError> {
    let ticket = GLOBAL_SAVE_TRACKER.reserve(dest_path);
    atomic_durable_write_with_ticket(dest_path, bytes, &ticket, &GLOBAL_SAVE_TRACKER)
}

/// Thực hiện ghi đĩa nguyên tử có kiểm soát vé thứ tự và bộ theo dõi được chỉ định.
pub fn atomic_durable_write_with_ticket(
    dest_path: &Path,
    bytes: &[u8],
    ticket: &WriteTicket,
    tracker: &DestinationSaveTracker,
) -> Result<PersistenceReport, PersistenceError> {
    // Bước 0: Thẩm tra an toàn đường dẫn
    validate_destination_path(&dest_path.to_string_lossy())?;

    let parent_dir = dest_path.parent().unwrap_or_else(|| Path::new("."));
    if !parent_dir.exists() {
        return Err(PersistenceError::TargetDirectoryNotFound(
            parent_dir.to_string_lossy().to_string(),
        ));
    }

    let file_name = dest_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.xlsx");

    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_name = format!(".tmp.{}.{}.{}", file_name, std::process::id(), counter);
    let temp_path = parent_dir.join(&temp_name);

    // RAII Guard để tự động xóa tệp tạm nếu có lỗi xảy ra
    struct TempGuard<'a>(&'a Path, bool);
    impl<'a> Drop for TempGuard<'a> {
        fn drop(&mut self) {
            if self.1 && self.0.exists() {
                let _ = std::fs::remove_file(self.0);
            }
        }
    }

    let mut guard = TempGuard(&temp_path, true);

    // Bước 1 & 2: Mở tệp tạm và ghi toàn bộ dữ liệu
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|e| PersistenceError::TempFileCreate {
            path: temp_path.clone(),
            source: e,
        })?;

    file.write_all(bytes).map_err(|e| PersistenceError::WriteAll {
        path: temp_path.clone(),
        source: e,
    })?;

    file.flush().map_err(|e| PersistenceError::WriteAll {
        path: temp_path.clone(),
        source: e,
    })?;

    // Bước 3: Đồng bộ đĩa (File Synchronization)
    file.sync_all().map_err(|e| PersistenceError::SyncAll {
        path: temp_path.clone(),
        source: e,
    })?;

    drop(file);

    // Bước 4 & 5 (Bất biến Item 2 - Stage 2C Closure): Cam kết nguyên tử có khóa điều phối (Atomic Commit)
    // Việc kiểm tra tính hợp lệ của thứ tự commit, thực hiện rename và ghi nhận trạng thái đã commit
    // diễn ra nguyên tử dưới cùng một lock của DestinationSaveTracker, loại bỏ hoàn toàn TOCTOU race.
    tracker.commit_atomic(ticket, || {
        std::fs::rename(&temp_path, dest_path).map_err(|e| PersistenceError::AtomicRename {
            from: temp_path.clone(),
            to: dest_path.to_path_buf(),
            source: e,
        })?;
        Ok(())
    })?;

    // Đã hoán đổi thành công -> Không cần guard dọn tệp tạm nữa
    guard.1 = false;

    // Bước 6: Đồng bộ thư mục cha trên POSIX
    #[allow(unused_mut)]
    let mut dir_synced = false;
    #[cfg(unix)]
    {
        if let Ok(dir_file) = File::open(parent_dir) {
            if dir_file.sync_all().is_ok() {
                dir_synced = true;
            }
        }
    }

    let fingerprint = compute_fingerprint(bytes);

    Ok(PersistenceReport {
        dest_path: dest_path.to_string_lossy().to_string(),
        bytes_written: bytes.len(),
        fingerprint,
        write_completed: true,
        file_synced: true,
        atomically_replaced: true,
        dir_synced,
    })
}

/// Cầu nối lưu trữ bền vững cấp cao:
/// - Trên WebAssembly (Leptos Frontend): Kiểm tra an toàn đường dẫn và chuyển giao mảng bytes qua Tauri IPC `save_file_binary`.
/// - Trên Native Host: Thực thi `atomic_durable_write` trực tiếp.
#[cfg(target_arch = "wasm32")]
pub async fn execute_durable_save(
    dest_path: &str,
    bytes: &[u8],
    _request_id: &str,
    _generation: u64,
) -> Result<PersistenceReport, PersistenceError> {
    validate_destination_path(dest_path)?;

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "filePath": dest_path,
        "bytes": bytes,
    }))
    .map_err(|e| PersistenceError::IpcError(format!("Không thể chuyển đổi tham số IPC: {e:?}")))?;

    invoke("save_file_binary", args)
        .await
        .map_err(|e| PersistenceError::IpcError(format!("Lỗi khi gọi Tauri save_file_binary: {e:?}")))?;

    let fingerprint = compute_fingerprint(bytes);
    Ok(PersistenceReport {
        dest_path: dest_path.to_string(),
        bytes_written: bytes.len(),
        fingerprint,
        write_completed: true,
        file_synced: true,
        atomically_replaced: true,
        dir_synced: false,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn execute_durable_save(
    dest_path: &str,
    bytes: &[u8],
    _request_id: &str,
    _generation: u64,
) -> Result<PersistenceReport, PersistenceError> {
    let path = Path::new(dest_path);
    atomic_durable_write(path, bytes)
}
