---
name: office-tool-pod
description: Khuôn mẫu chuẩn bắt buộc khi hiện thực hóa bất kỳ công cụ xử lý tệp hoặc dữ liệu nào trong crates/office-engine theo kiến trúc OfficeTool trait, đảm bảo 100% NFR bảo mật, đa luồng và TDD.
---

# Quy chuẩn Hiện thực Hóa Công cụ trong `crates/office-engine`

Mọi công cụ thuộc 5 phân hệ (`DOCUMENT`, `MEDIA`, `DATA`, `VN_OFFICE`, `SYSTEM/HUB`) bắt buộc phải tuân thủ cấu trúc mã nguồn và kiểm thử dưới đây.

## 1. Cấu trúc DTO & Kiểu Lỗi Chuẩn (`crates/office-shared`)

Không tự ý tạo kiểu lỗi rời rạc. Mọi lỗi từ tầng Engine phải ánh xạ sạch sang `CommandError` để giao diện Leptos hiển thị tiếng Việt trực quan:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub task_id: String,
    pub current_step: u32,
    pub total_steps: u32,
    pub percentage: f32,
    pub message: String,
    pub processed_files: usize,
    pub total_files: usize,
    pub is_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandError {
    FileNotFound { path: String },
    InvalidFileSignature { expected: String, actual: String },
    TaskCancelled,
    ProcessExecutionError { message: String },
    DatabaseError { details: String },
    IoError { details: String },
    ValidationFailed { reason: String },
}