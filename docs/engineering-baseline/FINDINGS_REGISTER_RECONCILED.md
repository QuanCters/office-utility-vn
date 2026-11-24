# Sổ Bộ Ghi Nhận Phát Hiện Kiểm Toán Đã Đối Chiếu (Reconciled Findings Register)
## Office Utility VN — Engineering Baseline Certification Audit

**Tài liệu tham chiếu:** `docs/engineering-baseline/findings_register.json`  
**Phiên bản:** 2.0.0 (Đối chiếu hoàn chỉnh)  
**Tổng số phát hiện:** **18** (10 P0 / 5 P1 / 3 P2)  
**Trạng thái kiểm toán:** BASELINE NOT CERTIFIED  

---

### Danh Mục Tóm Tắt Phát Hiện (Executive Summary Table)

| ID Phát Hiện | Mức Độ | Module / Tệp Nguồn | Tiêu Đề Phát Hiện | Trạng Thái Bằng Chứng |
| :--- | :---: | :--- | :--- | :---: |
| **FINDING-P0-IPC-01** | **P0** | `src-tauri::commands` | Nhánh wildcard trả về SUCCESS giả mạo & tệp gốc | `CONFIRMED_STATIC` |
| **FINDING-P0-PDF-01** | **P0** | `office-engine::doc_suite` | Phân tích PDF bằng chuỗi UTF-8 lossy (99 vị trí) | `CONFIRMED_STATIC` |
| **FINDING-P0-PDF-02** | **P0** | `doc_suite::pdf_merge_split` | Rơi rụng đồ thị tài nguyên gián tiếp (/Resources, Font) | `CONFIRMED_STATIC` |
| **FINDING-P0-FAKE-01** | **P0** | `doc_suite::vietnamese_ocr` | Động cơ OCR trả về chuỗi văn bản hardcode tĩnh | `CONFIRMED_STATIC` |
| **FINDING-P0-FAKE-02** | **P0** | `media_studio::video_to_gif` | Chuyển đổi video sang GIF sinh dải màu gradient | `CONFIRMED_STATIC` |
| **FINDING-P0-FAKE-03** | **P0** | `doc_suite::pdf_to_long_img` | Render PDF sang ảnh dài bằng bitmap vẽ sọc xám | `CONFIRMED_STATIC` |
| **FINDING-P0-FAKE-04** | **P0** | `doc_suite::pdf_pki_sign` | Chữ ký số PKCS#7 giả lập bằng chuỗi ASCII hex | `CONFIRMED_STATIC` |
| **FINDING-P0-FAKE-05** | **P0** | `doc_suite::pdf_protect_sign` | Mã hóa PDF giả lập bằng XOR Cipher và muối cố định | `CONFIRMED_STATIC` |
| **FINDING-P0-MEDIA-01**| **P0** | `media_studio::image_batch` | Tự động sinh ảnh trắng/xám khi giải mã PNG lỗi | `FIXED_UNVERIFIED` |
| **FINDING-P0-HARNESS-01**| **P0** | `verification-kit` | Thiếu adapter nhị phân và 17 bài kiểm tra hợp đồng | `REPRODUCED_DYNAMIC` |
| **FINDING-P1-IME-01** | **P1** | `office-utility-vn-ui` | 43 trường nhập liệu Leptos nuốt dấu UniKey/EVKey | `CONFIRMED_STATIC` |
| **FINDING-P1-DB-01** | **P1** | `src-tauri::db` | Seed data SQLite dùng slug cũ gây lỗi FK nuốt lịch sử | `CONFIRMED_STATIC` |
| **FINDING-P1-SEC-01** | **P1** | `src-tauri::tauri.conf.json` | CSP bị tắt (null) và bật withGlobalTauri trên Desktop | `CONFIRMED_STATIC` |
| **FINDING-P1-IO-01** | **P1** | `src-tauri::commands` | I/O đồng bộ trên luồng Tokio & nuốt lỗi đọc đĩa | `CONFIRMED_STATIC` |
| **FINDING-P1-BUILD-01** | **P1** | `workspace::build` | 97 Clippy warnings và Formatting Check thất bại | `REPRODUCED_DYNAMIC` |
| **FINDING-P2-PERSIST-01**| **P2** | `src-tauri::safe_persistence`| ALLOWED_EXTENSIONS thiếu đuôi webp, gif, zip, ics, sql | `CONFIRMED_STATIC` |
| **FINDING-P2-TEST-01** | **P2** | `workspace::tests` | Mock Mirroring trong 379 unit tests nội bộ | `CONFIRMED_STATIC` |
| **FINDING-P2-DATA-01** | **P2** | `data_lab::vn_font_converter`| Nhận diện mã VNI/TCVN3 bằng Latin-1 byte char còn rủi ro | `FIXED_UNVERIFIED` |

---

## CHI TIẾT CÁC PHÁT HIỆN MỨC ĐỘ P0 (BLOCKING & SAFETY-CRITICAL)

### FINDING-P0-IPC-01
- **Phân loại:** Lỗ hổng Kiến trúc & Báo cáo Sai lệch Nghiêm trọng
- **Tệp nguồn:** `src-tauri/src/commands/mod.rs` (dòng 74-115)
- **Tiêu đề:** Nhánh wildcard trong `dispatch_tool_task` giả mạo SUCCESS và trả về tệp đầu vào
- **Nguyên nhân gốc rễ (Root Cause):** Lệnh IPC `dispatch_tool_task` chỉ có nhánh `match` cho 8 slug; 50 slug còn lại rơi vào nhánh wildcard `_ =>` phát sự kiện tiến độ giả 50% rồi 100% và trả về `status: "SUCCESS"` kèm `output_paths: req_clone.input_paths.clone()`.
- **Kịch bản sự cố (Failure Scenario):** Người dùng chọn công cụ bất kỳ ngoài 8 công cụ (ví dụ `pdf-merge-split`), ấn Thực thi. Ứng dụng báo hoàn thành thành công tức thì trong 2ms, nhưng tệp kết quả mở ra chính là tệp đầu vào nguyên bản chưa hề được xử lý.
- **Bằng chứng xác minh:** Đoạn mã `mod.rs:74-115`. Vi phạm trực tiếp bất biến **INV-01** (Honest Execution) và **INV-02** (Source Preservation).
- **Hành động khắc phục:** Sửa nhánh wildcard thành fail-closed: trả về `Err(CommandError::ValidationFailed { reason: "Công cụ tạm thời chưa khả dụng...".into() })`.
- **Bài kiểm tra hồi quy:** `src-tauri/tests/verification_ipc.rs::unknown_and_disabled_tools_never_return_success`.

---

### FINDING-P0-PDF-01
- **Phân loại:** Phá hủy Dữ liệu Nhị phân & Lỗi Cú pháp Tài liệu
- **Tệp nguồn:** `crates/office-engine/src/doc_suite/*.rs` (99 vị trí vi phạm rule `PDF001`)
- **Tiêu đề:** Phân tích cú pháp PDF bằng chuỗi UTF-8 lossy và cắt chuỗi nhị phân bằng `endobj`
- **Nguyên nhân gốc rễ:** Sử dụng `String::from_utf8_lossy` trên toàn bộ tệp nhị phân PDF và tìm chuỗi `" obj"` / `"endobj"` thô sơ.
- **Kịch bản sự cố:** Tệp PDF chứa luồng nén zlib `FlateDecode` hoặc font nhúng chứa các byte không phải UTF-8 hợp lệ bị chuyển thành `\u{FFFD}`, phá hủy vĩnh viễn cấu trúc nén. Chuỗi byte ngẫu nhiên chứa ký tự `endobj` làm cụt đối tượng.
- **Bằng chứng xác minh:** Bộ guardrails phát hiện 99 điểm `from_utf8_lossy` trên file PDF. Lệnh `qpdf --check` trên tệp xuất ra báo lỗi hỏng xref và unrecoverable stream error.
- **Hành động khắc phục:** Thay thế toàn bộ bằng thư viện nhị phân an toàn (`lopdf`) và renderer chuẩn (`pdfium-render`).
- **Bài kiểm tra hồi quy:** `crates/office-engine/tests/verification_pdf.rs::malformed_pdf_does_not_produce_success`.

---

### FINDING-P0-PDF-02
- **Phân loại:** Rơi rụng Tài nguyên Đồ họa (Dangling References)
- **Tệp nguồn:** `crates/office-engine/src/doc_suite/pdf_merge_split.rs` (dòng 254-287, 420-450)
- **Tiêu đề:** Bỏ rơi đồ thị tài nguyên gián tiếp khi tách/ghép trang PDF
- **Nguyên nhân gốc rễ:** Chỉ sao chép từ điển `/Type /Page` mà không duyệt đệ quy transitive closure để sao chép `/Contents`, `/Resources`, `/Font`, `/XObject`.
- **Kịch bản sự cố:** Tách trang từ tài liệu có font nhúng hoặc ảnh; tệp đầu ra mở lên trong Adobe Acrobat báo lỗi *"An error exists on this page"* và chỉ hiển thị trang trắng tinh.
- **Bằng chứng xác minh:** Đoạn mã dòng 177-179 chỉ gộp `catalog`, `pages_root` và `split_page_objects`, hoàn toàn thiếu các object con phụ thuộc.
- **Hành động khắc phục:** Duyệt transitive closure của đồ thị đối tượng, sao chép và đánh số lại toàn bộ các object phụ thuộc sang tài liệu mới.
- **Bài kiểm tra hồi quy:** `crates/office-engine/tests/verification_pdf.rs::real_pdf_merge_split_preserves_resources`.

---

### FINDING-P0-FAKE-01
- **Phân loại:** Động cơ Giả mạo (Fake Engine / Mock Production Code)
- **Tệp nguồn:** `crates/office-engine/src/doc_suite/vietnamese_ocr.rs` (dòng 121-130)
- **Tiêu đề:** Động cơ OCR giả mạo trả về chuỗi văn bản tiếng Việt hardcode
- **Nguyên nhân gốc rễ:** Hàm `perform_ocr_extract` trả về chuỗi tĩnh cố định cho PDF và ảnh JPEG/PNG thay vì tích hợp mô hình OCR thực tế.
- **Kịch bản sự cố:** Người dùng nạp bất kỳ ảnh hóa đơn hay chứng từ scan nào vào đều nhận được nội dung hóa đơn tiền điện mẫu của *"Nguyễn Văn An"*.
- **Bằng chứng xác minh:** Mã nguồn chứa chuỗi hardcode: `"CỘNG HÒA XÃ HỘI CHỦ NGHĨA VIỆT NAM..."` và `"HÓA ĐƠN GIÁ TRỊ GIA TĂNG..."`. Vi phạm quy tắc `FAKE001`.
- **Hành động khắc phục:** Vô hiệu hóa công cụ ngay lập tức (trả về lỗi `ToolNotReady`) cho đến khi tích hợp model ONNX hoặc Tesseract vie.
- **Bài kiểm tra hồi quy:** `crates/office-engine/tests/verification_ocr.rs::ocr_result_depends_on_distinct_real_inputs`.

---

### FINDING-P0-FAKE-02
- **Phân loại:** Động cơ Giả mạo (Fake Engine)
- **Tệp nguồn:** `crates/office-engine/src/media_studio/video_to_gif_webp.rs` (dòng 254-293)
- **Tiêu đề:** Động cơ chuyển đổi Video sang GIF giả mạo sinh khung hình vạch màu mô phỏng
- **Nguyên nhân gốc rễ:** Hàm `extract_frames_from_video` không giải mã video mà vẽ một dải sáng di chuyển trên nền gradient vàng `(255, 215, 0)`.
- **Kịch bản sự cố:** Người dùng chuyển đổi video MP4/MOV, tệp GIF thu được là một vạch màu vàng chạy qua lại, không hề có hình ảnh nào của video gốc.
- **Bằng chứng xác minh:** Mã nguồn dòng 270-285 sinh pixel vàng nhân tạo; vi phạm quy tắc `FAKE002`.
- **Hành động khắc phục:** Trả về lỗi `ToolNotReady` cho đến khi tích hợp bộ giải mã video phần cứng hệ điều hành hoặc FFmpeg nhúng.
- **Bài kiểm tra hồi quy:** `crates/office-engine/tests/verification_media.rs::video_frames_match_timestamps_of_input`.

---

### FINDING-P0-FAKE-03
- **Phân loại:** Động cơ Giả mạo (Fake Engine)
- **Tệp nguồn:** `crates/office-engine/src/doc_suite/pdf_to_long_image.rs` (dòng 73-100)
- **Tiêu đề:** Động cơ kết xuất PDF sang ảnh dài giả lập bằng bitmap tượng trưng
- **Nguyên nhân gốc rễ:** Hàm `render_pdf_page_to_bitmap` vẽ banner và vạch xám tượng trưng nếu text chứa toán tử `re`/`Tj` thay vì rasterize thực tế.
- **Kịch bản sự cố:** Chuyển PDF sang ảnh dài chỉ thu được ảnh trắng với vài sọc xám tượng trưng.
- **Bằng chứng xác minh:** Mã nguồn dòng 73-100 gán thủ công `rgb[idx] = 50;`. Vi phạm quy tắc `FAKE003`.
- **Hành động khắc phục:** Tích hợp thư viện PDFium để rasterize trang thực tế trước khi ghép ảnh.
- **Bài kiểm tra hồi quy:** `crates/office-engine/tests/verification_pdf.rs::rendered_pdf_pixels_represent_input_content`.

---

### FINDING-P0-FAKE-04
- **Phân loại:** Vi phạm Chuẩn Mật mã & Chữ ký Điện tử Giả
- **Tệp nguồn:** `crates/office-engine/src/doc_suite/pdf_pki_sign.rs` (dòng 186-202)
- **Tiêu đề:** Chữ ký số PKCS#7 giả mạo sử dụng chuỗi văn bản ASCII hex
- **Nguyên nhân gốc rễ:** Ghi chuỗi `'PKCS7-SIG:CN=...'` đệm 0 vào từ điển chữ ký thay vì cấu trúc ASN.1 DER CMS tiêu chuẩn.
- **Kịch bản sự cố:** Tài liệu sau khi ký mở trong Adobe Acrobat bị báo lỗi *"Signature contains errors"* hoặc *"Unrecognized signature format"*.
- **Bằng chứng xác minh:** Mã nguồn dòng 186-193; vi phạm quy tắc `PDF003`.
- **Hành động khắc phục:** Vô hiệu hóa tính năng cho đến khi tích hợp thư viện ký số CMS chuẩn (như `cms` / `x509`).
- **Bài kiểm tra hồi quy:** `crates/office-engine/tests/verification_signing.rs::signed_pdf_has_cryptographically_valid_cms_and_byterange`.

---

### FINDING-P0-FAKE-05
- **Phân loại:** Vi phạm Chuẩn Mật mã & Giả mạo Bảo mật
- **Tệp nguồn:** `crates/office-engine/src/doc_suite/pdf_protect_sign.rs` (dòng 149-165, 196-202)
- **Tiêu đề:** Mã hóa PDF giả mạo bằng XOR Cipher và muối cố định dưới danh nghĩa AES-256
- **Nguyên nhân gốc rễ:** Dùng hàm `cipher_xor` với khóa băm lặp lại và muối hardcode `'OFFICE_SEC_SALT_VN_2026'` thay vì Standard Security Handler AES-256.
- **Kịch bản sự cố:** Đặt mật khẩu cho PDF bằng ứng dụng, sau đó mở lại bằng Adobe Acrobat: Acrobat báo lỗi tệp hỏng không thể mở khóa.
- **Bằng chứng xác minh:** Mã nguồn hàm `cipher_xor` dòng 159-165 và `render_encrypted_pdf` dòng 197.
- **Hành động khắc phục:** Vô hiệu hóa công cụ cho đến khi triển khai Standard Security Handler Rev 5/6 AES-256 chuẩn ISO 32000-1.
- **Bài kiểm tra hồi quy:** `crates/office-engine/tests/verification_secrets.rs::real_pdf_aes_encryption_roundtrip`.

---

### FINDING-P0-MEDIA-01
- **Phân loại:** Sinh Dữ liệu Giả & Nuốt Lỗi Giải mã Ảnh
- **Tệp nguồn:** `crates/office-engine/src/media_studio/image_batch.rs` (dòng 161-180, 1017)
- **Tiêu đề:** Tự động sinh ảnh trắng/xám giả mạo khi giải mã ảnh PNG thất bại
- **Nguyên nhân gốc rễ:** Hàm `decode_png` trả về `Ok(RawImage { pixels: vec![200; ...] })` hoặc `vec![255; ...]` khi `bit_depth` không hỗ trợ hoặc zlib decode thất bại.
- **Kịch bản sự cố:** Người dùng nạp vào ảnh PNG hỏng hoặc ảnh PNG 16-bit, ứng dụng báo thành công nhưng xuất ra một tệp ảnh xám hoặc trắng vô nghĩa.
- **Bằng chứng xác minh:** Mã nguồn commit HEAD dòng 161-180; quy tắc guardrail `PNG002`. Trong working tree, code chính đã được sửa nhưng test line 1017 vẫn kích hoạt guardrail regex.
- **Hành động khắc phục:** Trả về `Err(CommandError::ValidationFailed)` rõ ràng; dọn dẹp fixture trong unit test để không kích hoạt `PNG002`.
- **Bài kiểm tra hồi quy:** `crates/office-engine/src/media_studio/image_batch.rs::test_image_batch_broken_png_idat_fails_validation`.

---

### FINDING-P0-HARNESS-01
- **Phân loại:** Thiếu Hụt Hạ tầng Kiểm định Phát hành (Release Blocker)
- **Tệp nguồn:** `tests/verification/source_test_inventory.json` (dòng 1-91)
- **Tiêu đề:** Thiếu hoàn toàn 17 bài kiểm thử nguồn bắt buộc và chưa có Verification Adapter
- **Nguyên nhân gốc rễ:** Các tệp kiểm thử tích hợp thực tế theo hợp đồng chưa được hiện thực hóa trong repository; thiếu binary `office_verify_adapter`.
- **Kịch bản sự cố:** Cổng kiểm định chất lượng phát hành (`verify_source_test_inventory.py` và `verify_behavior.py`) thất bại 100%.
- **Bằng chứng xác minh:** `python3 scripts/verify_source_test_inventory.py --repo .` thoát mã 1, báo thiếu 17 tệp test.
- **Hành động khắc phục:** Xây dựng binary adapter `office_verify_adapter` và hiện thực hóa đầy đủ 17 bài kiểm tra hợp đồng.
- **Bài kiểm tra hồi quy:** `python3 scripts/verify_source_test_inventory.py --repo . && python3 scripts/verify_behavior.py --repo .`.

---

## CHI TIẾT CÁC PHÁT HIỆN MỨC ĐỘ P1 (MAJOR DEFECTS)

### FINDING-P1-IME-01
- **Phân loại:** Lỗi Trải nghiệm & Nhập liệu Tiếng Việt Leptos
- **Tệp nguồn:** `src/tools/*_view.rs` (43 tệp view)
- **Tiêu đề:** 43 trường nhập liệu Leptos gây lỗi nuốt ký tự bộ gõ tiếng Việt UniKey/EVKey
- **Nguyên nhân gốc rễ:** Gán `prop:value` và `on:input` đồng thời mà không bắt sự kiện `compositionstart` và `compositionend`.
- **Kịch bản sự cố:** Người dùng gõ tiếng Việt có dấu trong các ô tìm kiếm/nhập liệu, con trỏ nhảy loạn hoặc ký tự dấu bị nuốt (ví dụ gõ "tiếng" biến thành "tieengs").
- **Bằng chứng xác minh:** `scripts/verify_guardrails.py` phát hiện chính xác 43 vi phạm quy tắc `IME001`.
- **Hành động khắc phục:** Thay thế các thẻ input trực tiếp bằng component `ImeSafeInput` hoặc xử lý cờ composition.
- **Bài kiểm tra hồi quy:** `tests/e2e/verification_ime.spec.ts::vietnamese_ime_composition_is_not_interrupted`.

---

### FINDING-P1-DB-01
- **Phân loại:** Toàn vẹn Dữ liệu & Mất Dữ liệu Lịch sử Tác vụ
- **Tệp nguồn:** `src-tauri/src/db/mod.rs` (dòng 131-148)
- **Tiêu đề:** Slug công cụ trong SQLite Seed Data sai lệch gây lỗi khóa ngoại và nuốt mất lịch sử
- **Nguyên nhân gốc rễ:** Seed data chỉ có 16 slug cũ; bảng `task_history` có ràng buộc khóa ngoại `REFERENCES tool_settings(slug)`. Khi chạy công cụ có slug mới, lệnh INSERT bị lỗi và bị nuốt bởi `let _ =`.
- **Kịch bản sự cố:** Người dùng chạy các công cụ thành công nhưng bảng lịch sử tác vụ Task History hoàn toàn không ghi nhận dữ liệu.
- **Bằng chứng xác minh:** Đối chiếu danh mục 16 slug trong `default_tools` tại `db/mod.rs:132-147` với 58 slug tại `registry.rs`.
- **Hành động khắc phục:** Đồng bộ toàn bộ 58 slug chuẩn vào bảng `tool_settings`; loại bỏ `let _ =` tại `add_task_history`.
- **Bài kiểm tra hồi quy:** `src-tauri/tests/verification_db.rs::task_history_persists_for_all_valid_tool_slugs`.

---

### FINDING-P1-SEC-01
- **Phân loại:** An ninh Môi trường Desktop & Bề mặt Tấn công
- **Tệp nguồn:** `src-tauri/tauri.conf.json` (dòng 13, 21-23)
- **Tiêu đề:** Content Security Policy bị tắt (null) và bật withGlobalTauri trong môi trường Desktop
- **Nguyên nhân gốc rễ:** Cấu hình `tauri.conf.json` đặt `csp: null` và `withGlobalTauri: true`.
- **Kịch bản sự cố:** Nếu xảy ra XSS qua việc nạp file HTML/SVG độc hại, kẻ tấn công có quyền truy cập toàn bộ API native của Tauri qua `window.__TAURI__`.
- **Bằng chứng xác minh:** Kiểm tra tệp `src-tauri/tauri.conf.json` dòng 13 và 22.
- **Hành động khắc phục:** Bật CSP nghiêm ngặt (`default-src 'self'; img-src 'self' data: asset:;`) và tắt `withGlobalTauri`.
- **Bài kiểm tra hồi quy:** Kiểm tra cấu hình tĩnh qua `scripts/verify_guardrails.py`.

---

### FINDING-P1-IO-01
- **Phân loại:** Hiệu năng & Nghẽn Luồng Async
- **Tệp nguồn:** `src-tauri/src/commands/mod.rs` (dòng 275-294)
- **Tiêu đề:** Thao tác I/O đồng bộ chạy trực tiếp trên luồng async Tokio và nuốt lỗi đọc file
- **Nguyên nhân gốc rễ:** `inspect_file_metadata` gọi `std::fs::metadata` và `File::open` đồng bộ trên luồng Tokio, kết hợp `.unwrap_or(0)`.
- **Kịch bản sự cố:** Duyệt file trên ổ đĩa mạng chậm làm nghẽn worker threads của Tokio; lỗi đọc đĩa bị biến thành file rỗng 0 byte.
- **Bằng chứng xác minh:** Dòng 283-293 trong `src-tauri/src/commands/mod.rs`; vi phạm rule `IO001` và `IO002`.
- **Hành động khắc phục:** Chuyển I/O sang `spawn_blocking` hoặc `tokio::fs`; trả về lỗi rõ ràng thay vì `unwrap_or(0)`.
- **Bài kiểm tra hồi quy:** `src-tauri/tests/verification_commands.rs::slow_file_metadata_does_not_starve_async_executor`.

---

### FINDING-P1-BUILD-01
- **Phân loại:** Vệ sinh Mã nguồn & Cổng Chất lượng Biên dịch
- **Tệp nguồn:** `crates/office-engine`, `src/`, `benches/`
- **Tiêu đề:** 97 Clippy Warnings và Formatting Check thất bại trên toàn workspace
- **Nguyên nhân gốc rễ:** 97 cảnh báo clippy (`collapsible_if`, `unnecessary_map_or`, `manual_unwrap_or_default`) trên Rust 1.99 và mã nguồn chưa chạy `cargo fmt`.
- **Kịch bản sự cố:** Mọi lệnh kiểm tra chất lượng CI cơ bản (`cargo clippy -- -D warnings` và `cargo fmt -- --check`) đều thoát với mã lỗi.
- **Bằng chứng xác minh:** `cargo fmt --all -- --check` (exit code 1) và `cargo clippy --workspace --all-targets --all-features -- -D warnings` (exit code 101).
- **Hành động khắc phục:** Chạy `cargo fmt --all` và chuẩn hóa các biểu thức theo khuyến nghị của Clippy.
- **Bài kiểm tra hồi quy:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings`.

---

## CHI TIẾT CÁC PHÁT HIỆN MỨC ĐỘ P2 (MODERATE DEFECTS)

### FINDING-P2-PERSIST-01
- **Phân loại:** Giới hạn Định dạng Tệp Native Persistence
- **Tệp nguồn:** `src-tauri/src/safe_persistence.rs` (dòng 18-25)
- **Tiêu đề:** Danh mục đuôi tệp ALLOWED_EXTENSIONS trong Safe Persistence thiếu nhiều định dạng xuất
- **Nguyên nhân gốc rễ:** Thiếu các đuôi `.webp`, `.gif`, `.zip`, `.ics`, `.sql`, `.md` trong mảng `ALLOWED_EXTENSIONS`.
- **Kịch bản sự cố:** Khi người dùng lưu ảnh webp, gif hoặc file ics qua cơ chế native persistence, thao tác lưu sẽ bị từ chối với lỗi cấp quyền.
- **Bằng chứng xác minh:** Mảng `ALLOWED_EXTENSIONS` tại `safe_persistence.rs:18-25`.
- **Hành động khắc phục:** Mở rộng danh mục `ALLOWED_EXTENSIONS` bao gồm đầy đủ các định dạng hợp lệ của ứng dụng.
- **Bài kiểm tra hồi quy:** `src-tauri/tests/verification_persistence.rs::all_app_output_extensions_are_authorized`.

---

### FINDING-P2-TEST-01
- **Phân loại:** Thiếu Khách quan trong Kiểm thử (Test Mock Mirroring)
- **Tệp nguồn:** `crates/office-engine/src/**/*.rs` (các module `tests`)
- **Tiêu đề:** Hội chứng Test Mock Mirroring tạo cảm giác an toàn giả tạo cho 379 unit tests
- **Nguyên nhân gốc rễ:** Unit test tự sinh fixture đồ chơi khớp với giải thuật nội bộ của chính mình.
- **Kịch bản sự cố:** Test pass 100% nhưng gặp tệp thực tế ngoài đời thì ứng dụng hỏng hoàn toàn.
- **Bằng chứng xác minh:** Hàm `create_test_wide_margin_pdf` trong `pdf_crop_margin.rs`.
- **Hành động khắc phục:** Thay thế toàn bộ mock fixture bằng fixture nhị phân thật độc lập trong thư mục `fixtures/`.
- **Bài kiểm tra hồi quy:** Toàn bộ test suite chạy trên real fixtures.

---

### FINDING-P2-DATA-01
- **Phân loại:** Độ chính xác Xử lý Ngôn ngữ Tiếng Việt Kế thừa
- **Tệp nguồn:** `crates/office-engine/src/data_lab/vietnamese_font_converter.rs` (dòng 175-185)
- **Tiêu đề:** Nhận diện bảng mã VNI/TCVN3 dựa trên ánh xạ ký tự Latin-1 còn rủi ro với chuỗi pha trộn
- **Nguyên nhân gốc rễ:** Dùng `b as char` Latin-1 để tìm marker `aø`/`aù` mà chưa có thuật toán chấm điểm tần suất phân phối byte n-gram tiếng Việt.
- **Kịch bản sự cố:** Tệp văn bản chứa ký tự đặc biệt hoặc mã nhị phân ngẫu nhiên có thể bị nhận diện nhầm thành VNI-Windows.
- **Bằng chứng xác minh:** Mã nguồn dòng 179; vi phạm quy tắc `VNI001`.
- **Hành động khắc phục:** Xây dựng thuật toán tính điểm xác suất n-gram và trả về trạng thái `AMBIGUOUS` khi độ tin cậy thấp.
- **Bài kiểm tra hồi quy:** `crates/office-engine/tests/verification_encoding.rs::vni_and_tcvn3_goldens_preserve_vietnamese_diacritics`.
