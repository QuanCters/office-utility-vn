# BÁO CÁO KIỂM TOÁN TÍNH ĐÚNG ĐẮN NGHIỆP VỤ (CORRECTNESS AUDIT)
**Dự án:** Office Utility VN (Desktop Application)  
**Phiên bản kiểm toán:** Commit SHA `8c57f9655bb0841064973effd6ddeb72c0fa3599`  
**Vai trò thẩm định:** Principal Code Auditor & Correctness Lead  

---

## 1. TOÀN VẸN DỮ LIỆU & AN TOÀN NHỊ PHÂN (DATA INTEGRITY)

### 1.1. Phá Hủy Cấu Trúc Nhị Phân PDF Bằng Parser Chuỗi UTF-8 (P0)
- **Tập tin vi phạm:** Toàn bộ 25 công cụ trong `crates/office-engine/src/doc_suite/*.rs` (ghi nhận 99 vi phạm rule `PDF001` và 29 vi phạm rule `PDF002`).
- **Căn nguyên kỹ thuật (Root Cause):**
  - Sử dụng hàm `String::from_utf8_lossy(bytes)` để đọc toàn bộ tệp PDF nhị phân.
  - Tệp PDF hiện đại chứa các luồng nén FlateDecode (zlib), luồng nhị phân chứa font nhúng (TrueType/OpenType/CFF), ảnh nhúng (JPEG/PNG/JPX) và bảng tham chiếu chéo dạng luồng (Cross-Reference Streams `/Type /XRef`).
  - Khi dữ liệu nhị phân này được ép kiểu sang UTF-8 lossy, mọi byte không hợp lệ đều bị thay thế bằng 3 byte ký tự thay thế `\u{FFFD}` (`0xEF 0xBF 0xBD`). Thao tác này làm hỏng không thể phục hồi luồng nén và các con trỏ offset nhị phân.
  - Sử dụng `.find(" obj")` và `.find("endobj")`: Bất kỳ chuỗi byte ngẫu nhiên nào trong luồng nén của ảnh hoặc font tình cờ trùng với 6 byte ASCII `65 6E 64 6F 62 6A` (`endobj`) đều bị parser coi là kết thúc đối tượng, làm cắt cụt đối tượng (Truncated Streams) và tạo ra lỗi cú pháp PDF nghiêm trọng.

### 1.2. Mất Liên Kết Cây Đối Tượng (Dangling References & Missing Resource Graph - P0)
- **Vị trí vi phạm:** `crates/office-engine/src/doc_suite/pdf_merge_split.rs`, dòng 254–287 và 420–450.
- **Phân tích hành vi:**
  - Trong PDF, đối tượng `/Type /Page` không lưu nội dung trực tiếp mà chỉ chứa tham chiếu gián tiếp (`/Contents 5 0 R`, `/Resources << /Font << /F1 6 0 R >> /XObject << /Im1 7 0 R >> >>`).
  - Khi tách trang (`Split`) hoặc ghép trang (`Merge`), mã nguồn hiện tại chỉ sao chép từ điển `/Type /Page` và gán lại `/Parent 2 0 R`, nhưng **hoàn toàn không duyệt đệ quy (Transitive Graph Closure)** để sao chép các đối tượng số 5, 6, 7 vào tệp mới.
  - Tệp PDF kết quả chứa các tham chiếu gián tiếp trỏ tới các ID không hề tồn tại trong bảng XREF mới. Khi mở tệp bằng Adobe Acrobat hoặc Apple Preview, ứng dụng sẽ báo lỗi thiếu đối tượng hoặc hiển thị trang giấy trắng tinh.

### 1.3. Lệch Tuyệt Đối Bảng XREF và Trailer Không Hợp Lệ (P0)
- **Vị trí vi phạm:** `crates/office-engine/src/doc_suite/pdf_merge_split.rs`, `pdf_crop_margin.rs`, `pdf_compress.rs`.
- **Phân tích hành vi:**
  - PDF Spec ISO 32000-1 quy định bảng XREF phải ghi chính xác byte offset 10 chữ số tính từ byte 0 của tệp (`{:010} 00000 n \n`).
  - Do quá trình tuần tự hóa sử dụng chuỗi UTF-8 làm biến dạng số lượng byte thực tế, các giá trị offset được tính toán sai lệch hoàn toàn so với vị trí đối tượng trên đĩa.
  - Các công cụ kiểm định độc lập (`qpdf --check`, `pdfinfo`) lập tức báo lỗi: `qpdf: invalid xref offset` và từ chối xử lý tài liệu.

### 1.4. Âm Thầm Suy Thoái Chất Lượng Dữ Liệu (Silent Degradation - P0)
- **Vị trí vi phạm:** `crates/office-engine/src/media_studio/image_batch.rs`.
- **Hiện trạng:** Khi giải mã tệp ảnh bị lỗi cú pháp hoặc hỏng stream, bộ giải mã trả về mảng pixel toàn màu trắng hoặc xám:
  ```rust
  if decoder.read_to_end(&mut decompressed).is_err() {
      return Ok(RawImage {
          width,
          height,
          pixels: vec![255; (width * height * 3) as usize],
      });
  }
  ```
- **Hậu quả thực tế:** Người dùng chuyển đổi hàng loạt ảnh quan trọng nhưng nhận lại các tệp ảnh toàn màu trắng mà không hề có bất kỳ cảnh báo hay lỗi nào, dẫn tới mất mát dữ liệu nghiêm trọng mà người dùng không hề hay biết (vi phạm INV-05 và INV-06).

---

## 2. TÍNH ĐÚNG ĐẮN CỦA LOGIC NGHIỆP VỤ (BUSINESS LOGIC INTEGRITY)

### 2.1. Động Cơ Giả Mạo Masquerading As Native (P0)
Bốn công cụ nghiệp vụ quan trọng trong hệ thống hiện tại hoàn toàn là **mô phỏng đồ chơi (Fake/Mock Engine)**:

| Công cụ | Tệp tin vi phạm | Hành vi thực tế trong mã nguồn | Kỳ vọng thực tế theo hợp đồng |
| :--- | :--- | :--- | :--- |
| `vietnamese-ocr` | `doc_suite/vietnamese_ocr.rs:121-130` | Trả về chuỗi văn bản tiếng Việt hardcode tĩnh cố định ("CỘNG HÒA XÃ HỘI...", "HÓA ĐƠN...") | Nhận dạng chữ từ ảnh/PDF bằng OS Native OCR hoặc mô hình ONNX thực |
| `video-to-gif-webp` | `media_studio/video_to_gif_webp.rs:254-293` | Bỏ qua tệp video đầu vào; tạo mảng khung hình gradient với một vạch vàng chuyển động | Giải mã luồng video MP4/MOV thực tế qua FFmpeg / MediaFoundation |
| `pdf-to-long-image` | `doc_suite/pdf_to_long_image.rs:73-100` | Vẽ canvas trắng với banner đầu trang và các vạch xám tượng trưng nếu text chứa "re"/"Tj" | Kết xuất đồ họa thực tế (Rasterize) từng trang PDF thành điểm ảnh |
| `pdf-pki-sign` | `doc_suite/pdf_pki_sign.rs:186-202` | Chèn chuỗi text ASCII `PKCS7-SIG:...` | Ký số mã hóa CMS/PAdES chuẩn với ASN.1 DER structure |

### 2.2. Nhánh Điều Phối Wildcard Báo Hoàn Tất Giả (Wildcard Fake Success - P0)
- **Vị trí vi phạm:** `src-tauri/src/commands/mod.rs`, dòng 74–115.
- **Hiện trạng:**
  - Lệnh IPC `dispatch_tool_task` chỉ match trực tiếp 8 slug.
  - 50 công cụ còn lại rơi vào nhánh `_ =>`.
  - Nhánh này phát tiến độ 50% rồi 100% trong vòng dưới 1ms, trả về `status: "SUCCESS"` và trả về `output_paths: req_clone.input_paths.clone()`.
- **Hậu quả:** 50 công cụ khi được người dùng bấm "Thực hiện" trên giao diện Leptos đều báo "Hoàn tất thành công", nhưng không hề có tác vụ nào được thực thi và tệp xuất ra chính là tệp đầu vào (vi phạm INV-01 và INV-02).

---

## 3. ĐỒNG THỜI & ĐỘ TIN CẬY (CONCURRENCY & RELIABILITY)

### 3.1. Chạy Blocking I/O Đồng Bộ Trên Luồng Async Tokio (P1)
- **Vị trí vi phạm:** `src-tauri/src/commands/mod.rs`, dòng 275–294:
  ```rust
  #[tauri::command]
  pub async fn inspect_file_metadata(file_path: String) -> Result<FileMetadataResponse, CommandError> {
      let path = Path::new(&file_path);
      ...
      let metadata = std::fs::metadata(path)...?;
      let mut file = std::fs::File::open(path)...?;
      let bytes_read = file.read(&mut header).unwrap_or(0);
      ...
  }
  ```
- **Hậu quả:** Thao tác mở tệp và đọc đĩa đồng bộ (`std::fs::metadata`, `std::fs::File::open`, `file.read`) được gọi trực tiếp trong hàm bất đồng bộ `async fn` mà không được bọc qua `spawn_blocking` hoặc `tokio::fs`. Khi người dùng duyệt các tệp nằm trên ổ cứng mạng (SMB/NFS) hoặc ổ đĩa chậm, toàn bộ luồng worker của Tokio Runtime sẽ bị chặn cứng, làm đơ các lệnh IPC khác của ứng dụng.
- Ngoài ra, `.unwrap_or(0)` âm thầm nuốt lỗi đọc đĩa và biến lỗi I/O thành tệp rỗng 0 byte (vi phạm rule `IO001`).

### 3.2. Sai Lệch Slugs Database & Nuốt Lỗi Lịch Sử (P1)
- **Vị trí vi phạm:** `src-tauri/src/db/mod.rs:131-148` và `src-tauri/src/commands/mod.rs:146-149`.
- **Hiện trạng:**
  - Bảng `task_history` có ràng buộc khóa ngoại: `tool_slug TEXT REFERENCES tool_settings(slug) ON DELETE CASCADE`.
  - Bảng `tool_settings` chỉ được seed 16 slug cũ không khớp với slug thực tế (ví dụ: `spreadsheet-cleanse-normalize` thay vì `excel-cleaner-normalizer`).
  - Khi lệnh `dispatch_tool_task` hoàn tất, nó gọi:
    ```rust
    if let Ok(conn) = state.db.lock() {
        let _ = db::add_task_history(&conn, &history_record);
        let _ = db::record_tool_usage(&conn, &request.tool_slug, &history_record.options_used_json);
    }
    ```
  - Do vi phạm khóa ngoại, lệnh thêm lịch sử thất bại và trả về `Err(DatabaseError)`. Nhưng do sử dụng cú pháp `let _ =`, lỗi này bị nuốt hoàn toàn, dẫn tới việc bảng lịch sử tác vụ `task_history` luôn luôn trống rỗng đối với hầu hết các công cụ!

---

## 4. GIAO DIỆN & TRẢI NGHIỆM GÕ TIẾNG VIỆT (UI / UX CORRECTNESS)

### 4.1. Nuốt Ký Tự Bộ Gõ Tiếng Việt Trong 43 Trường Nhập Liệu Leptos (P1)
- **Vị trí vi phạm:** 43 tệp view trong `src/tools/*_view.rs` (ghi nhận bởi rule `IME001` trong `scripts/verify_guardrails.py`).
- **Căn nguyên kỹ thuật:**
  - Trong Leptos 0.8 CSR, khi liên kết thẻ `<input>` hoặc `<textarea>` bằng cú pháp hai chiều:
    ```rust
    <input
        type="text"
        prop:value=move || input_signal.get()
        on:input=move |ev| input_signal.set(event_target_value(&ev))
    />
    ```
  - Khi người dùng gõ tiếng Việt bằng UniKey hoặc EVKey (chế độ Telex/VNI), bộ gõ sẽ gửi các sự kiện gõ phím liên tiếp để ghép dấu (ví dụ: `a` + `a` -> `â`).
  - Việc liên kết trực tiếp `prop:value` khiến Leptos ép giá trị DOM cập nhật lại ngay giữa phiên gõ (`composition session`), làm ngắt quãng bộ gõ, nhảy con trỏ văn bản về cuối hoặc làm mất hoàn toàn ký tự dấu (ví dụ gõ "việt" biến thành "vieetj").
- **Giải pháp chuẩn:**
  - Bổ sung cờ lắng nghe `on:compositionstart` và `on:compositionend` để tạm hoãn việc đồng bộ ngược tín hiệu vào `prop:value` khi bộ gõ đang trong phiên xử lý dấu, hoặc chuyển sang dùng component `ImeSafeInput` đã được kiểm chứng.
