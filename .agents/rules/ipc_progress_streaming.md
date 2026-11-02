---
description: Quy chuẩn Phát Tiến độ tauri::ipc::Channel, Không Khóa Luồng Chính và Đảm bảo 60 FPS
trigger: always_on
---

# Quy tắc Phát Tiến độ IPC & Đảm bảo 60 FPS

1. **Non-blocking UI Thread (60 FPS):**
   - Mọi tác vụ xử lý tệp, mã hóa, tính toán có thời gian thực thi dự kiến trên **50ms** bắt buộc phải ủy quyền tính toán cho luồng nền qua `tauri::async_runtime::spawn_blocking`.
   - Tuyệt đối không thực thi các thuật toán nặng trực tiếp trên luồng IPC chính (Tauri Main Thread), đảm bảo giao diện Leptos không bao giờ bị giật lag hay đơ cứng.

2. **Tần số Phát Tiến độ qua `tauri::ipc::Channel`:**
   - Trong quá trình xử lý hàng loạt, tiến độ công việc `TaskProgress` phải được phát về giao diện Leptos thông qua `tauri::ipc::Channel` theo chu kỳ từ **100ms đến 200ms**.
   - Tránh phát tiến độ quá dày đặc (< 20ms) vì sẽ gây tắc nghẽn hàng đợi thông điệp IPC của WebView.

3. **Cấu trúc DTO `TaskProgress` Bắt buộc:**
   - Mọi bản tin tiến độ bắt buộc phải chứa đầy đủ các trường:
     ```rust
     TaskProgress {
         task_id: String,
         current_step: u32,
         total_steps: u32,
         percentage: f32, // Từ 0.0 đến 100.0
         message: String,  // Mô tả ngắn gọn tiếng Việt
         processed_files: usize,
         failed_files: usize,
         total_files: usize,
         is_completed: bool,
     }
     ```
   - Khi hoàn tất hoặc gặp lỗi hủy, bản tin tiến độ cuối cùng phải đặt `percentage: 100.0` và `is_completed: true`.
