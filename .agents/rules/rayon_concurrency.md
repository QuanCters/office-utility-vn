---
description: Quy chuẩn bắt buộc về Quản lý Luồng Rayon, An toàn Đa luồng Sync/Send và Hủy tác vụ Tức thì (< 100ms)
trigger: always_on
---

# Quy tắc Quản lý Luồng Rayon (Rayon Concurrency Rules)

1. **Tuyệt đối Không Chụp Tham chiếu Non-Sync trong `par_iter()`:**
   - Các tham chiếu dạng `&dyn Fn(TaskProgress)` không có trait bound `Sync` bị cấm đưa vào closure của `rayon::iter::ParallelIterator::map`.
   - Vi phạm quy tắc này sẽ gây ra lỗi biên dịch Rust `error[E0277]: dyn Fn(TaskProgress) cannot be shared between threads safely`.
   - **Giải pháp chuẩn:** Sử dụng biến đếm nguyên tử `std::sync::atomic::AtomicUsize` để ghi nhận số tệp đã xử lý bên trong closure song song. Phát tiến độ `TaskProgress` từ luồng điều phối bên ngoài hoặc sau khi thu thập kết quả.

2. **Kiểm tra Cờ Hủy Tác vụ Hợp tác (Cooperative Cancellation):**
   - Mọi vòng lặp xử lý nặng hoặc tác vụ song song trong `par_iter()` bắt buộc kiểm tra cờ hủy:
     ```rust
     if cancel_flag.load(Ordering::Relaxed) {
         return Err((file_path.clone(), CommandError::TaskCancelled));
     }
     ```
   - Thời gian từ khi kích hoạt cờ hủy đến khi toàn bộ threadpool dừng lại không được vượt quá **100ms**.

3. **Thu hồi Lỗi Tập trung và Báo cáo Trạng thái:**
   - Khi chạy đa luồng, kết quả trả về của từng phần tử phải là `Result<String, (String, CommandError)>`.
   - Nếu phát hiện bất kỳ phần tử nào trả về `CommandError::TaskCancelled`, toàn bộ phương thức `OfficeTool::execute` phải trả về `Err(CommandError::TaskCancelled)` ngay lập tức.
   - Các lỗi đọc/xử lý cục bộ phải được gom vào danh sách `failed_items: Vec<FileFailureDetail>` để xác định trạng thái `SUCCESS`, `PARTIAL_SUCCESS`, hoặc `FAILED`.
