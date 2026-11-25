# Review Rubric: Security & Sandboxing (Zero Egress & Bomb Defense)

Tài liệu tiêu chuẩn đánh giá bảo mật bắt buộc đối với mọi PR và module công cụ trong **Office Utility VN**.

## 1. Nguyên Tắc Cốt Lõi: 100% On-Device & Zero Network Egress (NFR-SEC-01)
- Ứng dụng cam kết xử lý hoàn toàn cục bộ trên thiết bị của người dùng.
- **TUYỆT ĐỐI CẤM:** Sử dụng các crate liên quan đến mạng ngoại vi trong các crate xử lý tệp:
  - `std::net`, `tokio::net`
  - `reqwest`, `hyper`, `ureq`, `surf`, `curl`
  - Bất kỳ API web fetch nào trong WASM/Leptos ra ngoài domain nội bộ `localhost` / `tauri://localhost`.
- Mọi nỗ lực thêm kết nối mạng vào engine sẽ bị gate L0 và L1 từ chối ngay lập tức.

## 2. Xác Thực Magic Bytes & Chống Tệp Độc Hại (NFR-SEC-04)
- **Không tin tưởng phần mở rộng tệp (Extension Spoofing):** Một tệp `.pdf` có thể là mã độc ELF/PE thực thi được ngụy trang.
- **Quy trình kiểm tra:**
  1. Đọc header bytes đầu tiên (tối thiểu 512 bytes).
  2. Dùng crate `infer` để xác định MIME type thực sự.
  3. Từ chối ngay lập tức nếu phát hiện chữ ký nhị phân thực thi:
     - `\x7fELF` (Linux executable)
     - `MZ` (Windows PE executable `0x4D 0x5A`)
     - Script headers shell không an toàn.
  4. Trả về mã lỗi chuẩn: `CommandError::InvalidFileSignature`.

## 3. Chống Tấn Công Khai Thác Tài Nguyên (Bomb & Decompression Protection - NFR-SEC-05)
- **Image Bomb (Decompression Bomb):**
  - Trước khi decode toàn bộ pixel buffer, bắt buộc đọc IHDR chunk (PNG) hoặc SOF chunk (JPEG) để lấy `(width, height)`.
  - Giới hạn cứng tối đa: `16,384 x 16,384` pixels (268.4 MegaPixels).
  - Sử dụng `image::Limits` hoặc kiểm tra bounds tường minh:
    ```rust
    if width > 16384 || height > 16384 {
        return Err(CommandError::ValidationFailed("Kích thước ảnh vượt quá ngưỡng an toàn (16384x16384)".into()));
    }
    ```
- **Zip Bomb / XML Bomb (Excel, Word OpenXML):**
  - Tỷ lệ nén tối đa cho phép: `1:100`.
  - Kích thước giải nén tối đa cho một tệp lưu trữ đơn lẻ: `1 GiB`.
  - Đếm tổng số bytes giải nén trong luồng lặp, ngắt ngay lập tức khi vượt ngưỡng.

## 4. An Toàn Bộ Nhớ & Dữ Liệu Nhạy Cảm (NFR-SEC-03)
- Mọi mật khẩu giải mã tài liệu (PDF, Excel, ZIP) bắt buộc phải bọc trong `zeroize::Zeroizing<String>` hoặc `zeroize::Zeroize`.
- Đảm bảo vùng nhớ chứa mật khẩu được xóa sạch (zeroed-out) ngay khi ra khỏi scope hoặc khi hoàn thành tác vụ.

## 5. Kiểm Soát Quyền IPC Tauri v2 (Tauri Capability & Scope)
- **Lưu ý đặc biệt từ tài liệu Tauri v2:** Các command đăng ký qua `tauri::generate_handler![...]` có thể được gọi từ bất kỳ webview nào nếu không có rào chắn scope.
- **Rà soát:**
  - Xác thực nguồn gốc và tham số trong từng custom command.
  - Các lệnh ghi đĩa bắt buộc thông qua `NativeSafePersistence` và kiểm tra quyền qua `NativeSaveGrantManager`.
  - Không bao giờ cho phép frontend truyền đường dẫn tùy ý không được người dùng xác nhận vào các lệnh thực thi nhị phân.
