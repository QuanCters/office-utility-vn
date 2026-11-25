# Harness Test Fixtures

Thư mục lưu trữ các tài liệu kiểm thử mẫu dành riêng cho Verification Harness:
1. `valid_pdf.pdf`: File PDF chuẩn hợp lệ với XREF và trailer đầy đủ.
2. `corrupted_xref.pdf`: File PDF bị sai lệch byte offset XREF để kiểm tra cơ chế phục hồi và báo lỗi.
3. `fake_image.png`: File chứa header ELF nhị phân ngụy trang đuôi `.png` để kiểm tra rào cản Magic Bytes (NFR-SEC-04).
4. `sample_grid.csv`: File dữ liệu CSV bảng tính dùng cho kiểm thử GridCraft / Data Lab.
