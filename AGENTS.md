# Kiến trúc & Quy chuẩn Dự án Office Utility VN

Bạn là Kỹ sư Hệ thống Rust chịu trách nhiệm hiện thực hóa hơn 60 công cụ của ứng dụng Desktop **Office Utility VN** (Tauri v2 + Leptos 0.8 CSR WebAssembly + SQLite 3 WAL mode).

## 0. KHÓA BẢO VỆ KHUNG XƯƠNG SỐNG (SPINE READ-ONLY GUARD) - BẮT BUỘC TUÂN THỦ
> [!CAUTION]
> **TẦNG KHUNG XƯƠNG SỐNG (PHASE 0 / SPINE LAYER) ĐÃ HOÀN THIỆN VÀ KHÓA CHỈ ĐỌC:**
> 1. `crates/office-shared`: ĐÃ KHÓA (`.spine-lock`). CẤM sửa đổi cấu trúc DTO hoặc enum `CommandError`.
> 2. `src-tauri/src/db`: ĐÃ KHÓA (`.spine-lock`). CẤM thay đổi DDL SQLite hoặc cấu trúc 4 bảng chuẩn.
> 3. `src-tauri/src/commands`: ĐÃ KHÓA. CẤM thay đổi chữ ký 10 lệnh IPC cốt lõi.
> 4. **QUYỀN HẠN CỦA POD AGENT:**
>    - Các Pod Agent chỉ được phép thêm module công cụ mới vào `crates/office-engine/src/<domain>/` thông qua việc hiện thực hóa trait `OfficeTool`.
>    - Viết Leptos UI Vùng 2 tương ứng tại `src/components/` hoặc `crates/office-ui/`.
>    - Chạy `./scripts/verify_spine_lock.sh` để xác thực kiến trúc không bị vi phạm trước khi bàn giao.

## 1. Cấu trúc Workspace (Spine-and-Pod)
- `crates/office-shared`: Chứa DTO dùng chung (`TaskProgress`, `CommandError`, `WorkflowDag`, `DispatchTaskRequest`, `ToolSettingRecord`, `ToolPreset`, `TaskHistoryRecord`, v.v.). **[READ-ONLY]**
- `src-tauri/src/db`: Quản lý SQLite 3 chế độ WAL với 4 bảng chuẩn (`tool_settings`, `workflows`, `task_history`, `app_preferences`) + `tool_presets`. **[READ-ONLY]**
- `src-tauri/src/commands`: 10 lệnh IPC Command tiêu chuẩn kết nối Leptos và Rust Engine. **[READ-ONLY]**
- `crates/office-engine`: Chứa trait `OfficeTool` và các module công cụ thuần Rust. **[VÙNG PHÁT TRIỂN POD AGENT]**
- `src/`: Giao diện Leptos 0.8 CSR theo bố cục Unified 3-Zone Workspace, Command Palette, Clipboard Monitor, Always-on-Top.

## 2. Hướng dẫn Sử dụng Công cụ (Tool Guidance)
- Khi phát triển hoặc sửa lỗi một công cụ đơn lẻ, chỉ chạy `CARGO_TARGET_DIR=/tmp/cargo-target cargo check -p office-engine` và `CARGO_TARGET_DIR=/tmp/cargo-target cargo test -p office-engine <tên_công_cụ>` để nhận phản hồi trong dưới 2 giây.
- Trước khi kết thúc nhiệm vụ, bắt buộc chạy `./scripts/verify_spine_lock.sh` và `./scripts/verify_pod.sh <tên_công_cụ>`.

## 3. Thư viện Bài học Tích lũy từ Harness² (Accumulated Experience)
@[experience](.harness/memory/EXPERIENCE.md)
