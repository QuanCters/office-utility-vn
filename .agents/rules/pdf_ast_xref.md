---
description: Quy chuẩn Thao tác Cây Đối tượng PDF (AST), Đánh số ID và Tái tạo Bảng XREF Chuẩn
trigger: always_on
---

# Quy tắc Thao tác Cây Đối tượng PDF & Tái tạo XREF

1. **Chuẩn hóa Mã Định danh Đối tượng (Object ID Re-indexing):**
   - Khi tách (Split) hoặc gộp (Merge) tệp PDF, tuyệt đối không giữ nguyên ID gốc để tránh xung đột định danh.
   - Toàn bộ các Object phải được đánh số lại tuần tự `1..N`.
   - Mọi chuỗi tham chiếu dạng `X 0 R` trong nội dung các Object (đặc biệt trong từ điển `/Pages`, `/Kids`, `/Parent`, `/Resources`) bắt buộc phải được thay thế đồng bộ sang ID mới `new_X 0 R`.

2. **Cấu trúc Bảng XREF và Trailer Tuyệt đối:**
   - Dòng đầu tiên của bảng XREF luôn là mục số 0 tự do:
     ```text
     xref
     0 <Tổng số object + 1>
     0000000000 65535 f \n
     ```
   - Mỗi đối tượng tiếp theo phải có đúng 10 chữ số biểu diễn byte offset tính từ đầu tệp:
     ```text
     0000000058 00000 n \n
     ```
   - Trailer bắt buộc chứa `/Size` chính xác bằng tổng số mục trong XREF và `/Root` trỏ tới ID của `/Type /Catalog`:
     ```text
     trailer
     << /Size <N+1> /Root <catalog_id> 0 R >>
     startxref
     <byte_offset_of_xref>
     %%EOF
     ```

3. **Tối ưu Hóa Thao tác Thuộc tính Trang (Page Rotation):**
   - Khi thực hiện xoay trang (Rotate 90, 180, 270), chèn hoặc sửa đổi trực tiếp khóa `/Rotate <deg>` vào từ điển `/Type /Page`.
   - Tuyệt đối không re-encode hoặc giải nén lại luồng nội dung `/Contents` nếu không có yêu cầu sửa đổi đồ họa điểm ảnh hoặc văn bản, đảm bảo tốc độ xử lý siêu tốc (> 1000 trang/giây).
