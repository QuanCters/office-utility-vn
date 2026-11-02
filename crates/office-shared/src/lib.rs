use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskProgress {
    pub task_id: String,
    pub current_step: u32,
    pub total_steps: u32,
    pub percentage: f32,
    pub message: String,
    pub processed_files: usize,
    pub failed_files: usize,
    pub total_files: usize,
    pub is_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, thiserror::Error)]
pub enum CommandError {
    #[error("Không tìm thấy tệp: {path}")]
    FileNotFound { path: String },

    #[error("Định dạng tệp không hợp lệ. Kỳ vọng: {expected}, Thực tế: {actual}")]
    InvalidFileSignature { expected: String, actual: String },

    #[error("Tệp ({size_mb} MB) vượt ngưỡng xử lý DOM an toàn ({max_recommended_mb} MB).")]
    FileTooLargeForInMemoryDom { size_mb: u64, max_recommended_mb: u64 },

    #[error("Đã chặn tệp có dấu hiệu gây tràn tài nguyên: {reason}")]
    BombDefenseTriggered { reason: String },

    #[error("Định dạng đầu ra của bước '{source_node}' không tương thích với đầu vào của bước '{target_node}'.")]
    IncompatibleWorkflowEdge { source_node: String, target_node: String },

    #[error("Chuỗi tự động hóa chứa vòng lặp vô hạn (Cycle Detected).")]
    WorkflowCycleDetected,

    #[error("Tính năng này cần gói mở rộng ngoại tuyến '{addon_id}' ({size_mb:.1} MB).")]
    AddonRequired { addon_id: String, size_mb: f32 },

    #[error("Tác vụ đã bị người dùng hủy.")]
    TaskCancelled,

    #[error("Lỗi thực thi: {message}")]
    ProcessExecutionError { message: String },

    #[error("Lỗi cơ sở dữ liệu: {details}")]
    DatabaseError { details: String },

    #[error("Lỗi đọc ghi tệp: {details}")]
    IoError { details: String },

    #[error("Dữ liệu không hợp lệ: {reason}")]
    ValidationFailed { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchTaskRequest {
    pub task_id: String,
    pub tool_slug: String,
    pub input_paths: Vec<String>,
    pub output_directory: Option<String>,
    pub options: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileFailureDetail {
    pub input_path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DispatchTaskResponse {
    pub task_id: String,
    pub status: String, // "SUCCESS" | "PARTIAL_SUCCESS" | "FAILED" | "CANCELLED"
    pub output_paths: Vec<String>,
    pub failed_items: Vec<FileFailureDetail>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowDag {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowNode {
    pub id: String,
    pub tool_slug: String,
    pub label: String,
    pub options: serde_json::Value,
    pub input_mimes: Vec<String>,
    pub output_mimes: Vec<String>,
    pub position_x: f64,
    pub position_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowEdge {
    pub id: String,
    pub source_node_id: String,
    pub target_node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolSettingRecord {
    pub slug: String,
    pub name: String,
    pub category: String,
    pub last_options_json: String,
    pub is_pinned: bool,
    pub usage_count: u32,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolPreset {
    pub id: String,
    pub tool_slug: String,
    pub preset_name: String,
    pub options_json: String,
    pub is_default: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskHistoryRecord {
    pub id: String,
    pub parent_task_id: Option<String>,
    pub tool_slug: Option<String>,
    pub workflow_id: Option<String>,
    pub input_files_json: String,
    pub output_files_json: String,
    pub options_used_json: String,
    pub status: String,
    pub elapsed_ms: u64,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppPreferenceRecord {
    pub key: String,
    pub value_json: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileMetadataResponse {
    pub file_path: String,
    pub mime_type: String,
    pub extension: String,
    pub file_size_bytes: u64,
    pub is_supported: bool,
    pub magic_bytes_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AddonStatusResponse {
    pub addon_id: String,
    pub is_installed: bool,
    pub version: Option<String>,
    pub local_path: Option<String>,
    pub size_mb: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UpdaterPreference {
    pub skipped_version: Option<String>,
    pub remind_later_until: Option<String>,
    pub last_checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateCheckResponse {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: Option<String>,
    pub release_date: Option<String>,
    pub should_prompt: bool,
}

