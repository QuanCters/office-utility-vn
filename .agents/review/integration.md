# Review Rubric: Integration & Concurrency Integrity

Tài liệu chuẩn mực đánh giá tích hợp giữa Leptos 0.8 CSR, Tauri v2 IPC và Rayon Concurrency Engine.

## 1. Phân Tách Luồng & Đảm Bảo 60 FPS (NFR-PERF-02)
- **Tauri Main Thread / Webview Thread:**
  - Phải luôn phản hồi tức thì với sự kiện chuột, bàn phím và IME tiếng Việt (UniKey / EVKey).
  - Bất kỳ phép toán, phân tích tệp, mã hóa hoặc I/O nào dự kiến chạy trên **50ms** bắt buộc phải chuyển sang luồng nền qua `tauri::async_runtime::spawn_blocking` hoặc threadpool của `rayon`.
  - Không được thực thi đồng bộ các vòng lặp tính toán nặng trong thân hàm command.

## 2. Quản Lý Luồng Rayon & Ràng Buộc Hợp Tác (Rayon Concurrency Rules)
- **Cấm chụp Non-Sync Closure:**
  - Tuyệt đối không chụp tham chiếu `&dyn Fn(TaskProgress)` không có `Sync` vào `par_iter().map(...)`. Trình biên dịch Rust sẽ báo lỗi `E0277`.
  - Sử dụng các kiểu dữ liệu nguyên tử (`AtomicUsize`, `AtomicBool`) để tổng hợp trạng thái tiến độ từ nhiều luồng.
- **Hủy Tác Vụ Hợp Tác Tức Thì (< 100ms - NFR-REL-01):**
  - Mọi vòng lặp song song phải kiểm tra cờ hủy:
    ```rust
    if cancel_flag.load(Ordering::Relaxed) {
        return Err((file_path, CommandError::TaskCancelled));
    }
    ```
  - Khi cờ hủy được bật, toàn bộ threadpool phải dừng nạp công việc mới và thoát trong dưới 100ms.

## 3. Phát Tiến Độ IPC & Điều Tiết (Throttle - 100ms đến 200ms)
- Tần số phát `TaskProgress` qua `tauri::ipc::Channel` phải được điều tiết trong khoảng **100ms - 200ms**.
- Tránh phát quá nhanh (< 20ms) vì sẽ làm nghẽn Event Loop của WebView2/WebKit.
- Bản tin kết thúc bắt buộc có `percentage: 100.0` và `is_completed: true`.

## 4. Kiểm Thử Lỗi & Khôi Phục (Fault Injection & Rollback)
- Mọi hàm IPC thao tác dữ liệu phải có kịch bản xử lý khi:
  1. Đĩa đích bị đầy (`DiskFull`).
  2. File đang bị khóa bởi tiến trình khác (`PermissionDenied`).
  3. Quá trình xử lý bị hủy ngang (`TaskCancelled`).
  4. Trả về danh sách `FileFailureDetail` đầy đủ để người dùng biết chính xác tệp nào thành công, tệp nào thất bại.
