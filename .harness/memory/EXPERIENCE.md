# Nhật ký Kinh nghiệm & Bài học Kỹ thuật (Harness² Per-Domain Memory)

## [Domain: Global & Architecture]
- **Tauri v2 Window & Capabilities:** Do sử dụng thanh tiêu đề tùy biến (Custom Window Titlebar), trong `src-tauri/tauri.conf.json` phải đặt `"decorations": false` và `"createUpdaterArtifacts": true`. Trong `capabilities/default.json` phải cấp đủ quyền `global-shortcut:allow-register`, `core:window:allow-set-always-on-top`, `core:window:allow-set-size` và `updater:default`.
- **Chặn Phím tắt Mặc định của WebView:** Khi lắng nghe phím tắt toàn cục trong cửa sổ ứng dụng (`Ctrl + P` để ghim cửa sổ Always-on-Top, `Ctrl + W` để thu xuống System Tray, `Ctrl + O` để mở thư mục kết quả), bắt buộc gọi `ev.prevent_default()` để ngăn WebView2/WebKit mở hộp thoại In hoặc đóng cửa sổ.
- **Khóa Bất biến Khung Xương Sống (Spine Lock):** Tầng nền tảng (`crates/office-shared`, `src-tauri/src/db`, `src-tauri/src/commands`) đã được niêm phong bảo vệ qua script `scripts/verify_spine_lock.sh`. Mọi công cụ nghiệp vụ mới bắt buộc chỉ được thêm vào `crates/office-engine` dưới dạng Tool Pod độc lập tuân thủ trait `OfficeTool: Send + Sync`.

## [Domain: Concurrency & Rayon Threadpool Management]
- **Ràng buộc Sync trên Closure Song song:** Tuyệt đối không chụp (`capture`) các tham chiếu `&dyn Fn(TaskProgress)` không có trait bound `Sync` vào bên trong closure `par_iter().map(...)` của Rayon, vì trình biên dịch Rust sẽ báo lỗi `E0277`.
- **Đếm Tiến độ Đa luồng:** Sử dụng `std::sync::atomic::AtomicUsize` để ghi nhận số lượng tệp đã xử lý xong bên trong Rayon threadpool mà không cần khóa `Mutex` cồng kềnh.
- **Hủy tác vụ Hợp tác Tức thì (< 100ms):** Bên trong thân lặp `par_iter()`, bắt buộc kiểm tra `cancel_flag.load(Ordering::Relaxed)` trước mỗi bước I/O hoặc tính toán nặng, trả về `Err((path, CommandError::TaskCancelled))` để dừng ngay lập tức khi người dùng bấm nút Hủy trên giao diện.

## [Domain: doc-suite (Tài liệu & PDF)]
- **Tái cấu trúc Cây Đối tượng & Bảng XREF:** Khi ghép (Merge) hoặc tách (Split) PDF, toàn bộ các Object phải được đánh số lại ID tuần tự (1..N). Chuỗi tham chiếu `X 0 R` trong `/Kids` và `/Parent` phải được thay thế đồng bộ sang ID mới để tránh lỗi tham chiếu vòng lặp hoặc đối tượng ma.
- **Tính toán Byte Offset XREF Tuyệt đối:** Header XREF `xref\n0 N\n` bắt buộc phải có dòng đầu tiên là `0000000000 65535 f \n`, các dòng tiếp theo có đúng định dạng 10 chữ số byte offset `{:010} 00000 n \n`.
- **Xoay trang PDF Không Tốn Chi Phí:** Khi xoay trang (Rotate), chèn hoặc cập nhật trực tiếp khóa `/Rotate <deg>` vào từ điển `/Type /Page` của từng trang thay vì re-encode luồng `/Contents`, giúp tốc độ xử lý đạt hàng nghìn trang/giây.
- **Phân tách `lopdf` và `pdfium-render`:** Chỉ dùng AST manipulation cho các thao tác cây đối tượng nhẹ (ghép/tách trang, xoay trang, metadata). Với các tác vụ cần kết xuất đồ họa thực tế (`FR-DOC-10` Auto-Crop BBox, `FR-DOC-12` Visual Diff, `FR-DOC-15` PDF sang ảnh dài), bắt buộc ủy quyền cho `pdfium-render`.

## [Domain: media-studio (Xử lý Đồ họa & Ảnh)]
- **Phòng chống Image Bomb (NFR-SEC-05):** Mọi công cụ nạp ảnh bắt buộc phải đọc header IHDR/SOF và từ chối xử lý ngay lập tức nếu kích thước vượt quá giới hạn an toàn 16384x16384 pixels, trả về `CommandError::ValidationFailed`.
- **Tối ưu Hóa PNG Không Cần External Crate:** Sử dụng trực tiếp `flate2::read::ZlibDecoder` và `flate2::write::ZlibEncoder` cho các chunk `IDAT`. Triển khai thuật toán CRC-32 chuẩn IEEE 802.3 cục bộ để tạo các chunk PNG hợp lệ mà không phụ thuộc vào thư viện bên ngoài.
- **Loại bỏ Metadata (Strip EXIF):** Bỏ qua các chunk `eXIf`, `tEXt`, `iTXt` khi ghi tệp đích giúp giảm dung lượng tệp từ 15% - 40% mà không làm suy giảm chất lượng điểm ảnh.

## [Domain: data-lab & vn-office (Bảng tính & Font Tiếng Việt)]
- **Giải mã Byte Font Kế thừa (Legacy Encodings):** Tuyệt đối không dùng `String::from_utf8_lossy` trực tiếp trên chuỗi byte TCVN3 hoặc VNI-Windows vì các byte dấu tiếng Việt đặc thù (0xA7, 0xA8, 0xB5..0xFE) sẽ bị nuốt thành ký tự thay thế `\u{FFFD}`. Bắt buộc chuyển đổi qua ánh xạ byte 8-bit tương đương Windows-1252 trước khi ánh xạ sang Unicode dựng sẵn NFC.
- **Tự động Nhận diện Bảng mã:** Quét các byte dấu đặc thù của TCVN3 hoặc các tổ hợp nguyên âm 2-byte của VNI (`aø`, `aù`, `eù`, `oø`, `uø`, `ñ`) để tự động nhận dạng chính xác bảng mã nguồn khi người dùng chọn chế độ `auto`.
- **Xử lý Số Cực Lớn & Chuẩn Kế toán VN:** Đọc số thành chữ phân tách các lớp hàng tỷ lặp lại (`tỷ`, `nghìn tỷ`, `triệu tỷ`, `tỷ tỷ`), hỗ trợ phương ngữ Bắc ("nghìn", "lẻ", "mốt") và phương ngữ Nam ("ngàn", "linh", "tư"), kèm hậu tố kế toán `"đồng chẵn"`.

## [Domain: sys-hub (Trạm Điều phối Vạn năng)]
- **Nhận diện Magic Bytes Đa lớp:** Sử dụng crate `infer` kết hợp kiểm tra ký hiệu byte đặc thù (%PDF-, BM, P6) và kiểm tra byte null (`0x00`) để phân loại chính xác giữa PDF, Ảnh, Văn bản UTF-8 và Binary lạ.
- **Từ chối Thực thi Tệp Độc hại:** Chặn ngay lập tức các tệp thực thi nhị phân chứa header `\x7fELF` hoặc `MZ` với `CommandError::InvalidFileSignature`.
- **Typed Dispatcher:** Điều phối tác vụ đến đúng struct công cụ nội bộ (`PdfMergeSplitTool`, `ImageBatchTool`, `VietnameseFontConverterTool`) qua trait `OfficeTool`, gom nhóm kết quả và trả về báo cáo tổng hợp `DispatchTaskResponse`.
