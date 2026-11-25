# Guideline: Finding Validation & Review Reporting

Tài liệu chuẩn mực hướng dẫn agent và reviewer cách xác thực, phân loại và báo cáo các phát hiện (findings) để đảm bảo độ tin cậy tuyệt đối, tránh báo động giả (false positives).

## 1. Cấu Trúc Một Finding Chuẩn (Finding Schema)
Mọi phát hiện khi review hoặc audit phải tuân thủ schema tại `harness/review/finding.schema.json`:
- `id`: Mã định danh duy nhất (ví dụ: `SEC-001`, `CORR-014`).
- `title`: Tóm tắt ngắn gọn lỗi.
- `severity`: Phân loại nghiêm trọng: `CRITICAL`, `HIGH`, `MEDIUM`, `LOW`, hoặc `INFO`.
- `category`: `CORRECTNESS`, `SECURITY`, `INTEGRATION`, `PERFORMANCE`, hoặc `CODE_STYLE`.
- `file`: Đường dẫn tệp tuyệt đối hoặc tương đối tính từ workspace root.
- `line_range`: Khoảng dòng bị ảnh hưởng `[start_line, end_line]`.
- `evidence`: Bằng chứng cụ thể trích dẫn từ code hoặc log lỗi kiểm thử.
- `recommendation`: Giải pháp khắc phục cụ thể, khả thi và rõ ràng.

## 2. Quy Tắc Phân Cấp Mức Độ Nghiêm Trọng (Severity Grading)
1. **CRITICAL:**
   - Vi phạm Zero Network Egress (kết nối mạng ngoài ý muốn).
   - Nguy cơ mất dữ liệu người dùng (ghi đè file không nguyên tử, mất dữ liệu chưa lưu khi tắt ứng dụng).
   - Bỏ qua kiểm tra quyền IPC hoặc Directory Traversal.
   - Sửa đổi các file thuộc Spine Lock hoặc Harness Policies mà không có quyền.
2. **HIGH:**
   - Thiếu kiểm tra kích thước ảnh/zip gây nguy cơ Decompression Bomb.
   - Panics trong luồng nghiệp vụ (`.unwrap()`, `.expect()`).
   - Chặn main thread quá 50ms mà không đưa vào luồng nền.
   - Vi phạm cơ chế hủy tác vụ hợp tác (< 100ms).
3. **MEDIUM:**
   - Thiếu throttle phát tiến độ IPC (< 20ms).
   - Xử lý lỗi sơ sài, thông báo lỗi bằng tiếng Anh thay vì tiếng Việt thân thiện với người dùng văn phòng.
   - Thiếu test coverage cho các trường hợp biên (edge cases).
4. **LOW / INFO:**
   - Cải tiến nhỏ về định dạng code, chú thích hoặc refactoring cục bộ không ảnh hưởng logic.

## 3. Quy Trình Xác Thực Để Tránh False Positives
1. **Kiểm tra sự tồn tại:** Đảm bảo file và số dòng được trích dẫn thực sự tồn tại trong commit hiện tại.
2. **Kiểm tra ngữ cảnh:** Xem xét hàm bao bọc và các lớp bảo vệ bên ngoài. Ví dụ: một hàm gọi `.unwrap()` nhưng đã được bọc trong một kiểm tra bounds tuyệt đối hoặc trong ngữ cảnh compile-time / test fixture không được đánh giá là `CRITICAL`.
3. **Khử trùng lặp (Deduplication):** Nếu nhiều finding bắt nguồn từ cùng một nguyên nhân gốc rễ, gộp chúng vào một finding duy nhất với danh sách vị trí liên quan.
4. **Chạy script kiểm định:** Bắt buộc chạy `python3 harness/review/validate-findings.py <findings_file.json>` trước khi đính kèm báo cáo vào PR.
