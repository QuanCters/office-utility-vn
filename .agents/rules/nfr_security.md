---
description: Ràng buộc Bảo mật On-Device, Đa luồng 60 FPS và Chống Trôi dạt API
trigger: always_on
---

# Quy tắc Bắt buộc (Hard Guardrails)

1. **100% On-Device & Zero Network Egress (NFR-SEC-01):** Tuyệt đối cấm sử dụng `std::net`, `tokio::net`, `reqwest`, `hyper` trong bất kỳ công cụ xử lý tệp nào.
2. **Xác thực Magic Bytes & Chống Bomb (NFR-SEC-04, NFR-SEC-05):**
   - Bắt buộc xác thực chữ ký tệp bằng crate `infer` trước khi nạp vào bộ xử lý.
   - Áp dụng `image::Limits` tối đa 16384x16384 pixels và giới hạn tỷ lệ bung nén ZIP/OpenXML ở mức 1:100 (tối đa 1 GB).
3. **An toàn Bộ nhớ & Không Khóa UI (NFR-SEC-03, NFR-PERF-02, NFR-REL-01):**
   - Bọc mọi biến mật khẩu (PDF, Excel) bằng `zeroize::Zeroizing`.
   - Cấm dùng `.unwrap()` hoặc `.expect()` trong luồng nghiệp vụ; bắt buộc trả về `Result<T, CommandError>`.
   - Mọi tác vụ trên 50ms phải chạy trên `rayon::spawn` hoặc `tokio::task::spawn_blocking`, phát tiến độ `TaskProgress` qua `tauri::ipc::Channel` mỗi 200ms và kiểm tra cờ hủy tác vụ để dừng trong dưới 100ms.
4. **Chuẩn Phiên bản (Anti-Drift):**
   - Giao diện Leptos 0.8: Sử dụng `signal()`, xử lý sự kiện `compositionstart` và `compositionend` để không nuốt dấu tiếng Việt (UniKey/EVKey).