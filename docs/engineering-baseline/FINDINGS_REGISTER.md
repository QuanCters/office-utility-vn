# SỔ ĐĂNG KÝ PHÁT HIỆN KIỂM TOÁN (FINDINGS REGISTER)
**Dự án:** Office Utility VN (Desktop Application)  
**Phiên bản kiểm toán:** Commit SHA `8c57f9655bb0841064973effd6ddeb72c0fa3599`  
**Ngày kiểm toán:** 22/11/2026  
**Định dạng máy đọc được:** [findings_register.json](file:///home/quan/workspace/tauri-v2/office-utility-vn/docs/engineering-baseline/findings_register.json)  

---

## 1. BẢNG TỔNG HỢP THEO MỨC ĐỘ NGHIÊM TRỌNG

| Mức độ | Định nghĩa | Số lượng | Trạng thái phát hành |
| :---: | :--- | :---: | :---: |
| 🔴 **P0** | Lỗ hổng hỏng dữ liệu, giả mạo nghiệp vụ, fake crypto, bypass kiểm định, chặn release gate | **8** | **BLOCKER (Chặn 100%)** |
| 🟠 **P1** | Lỗi trải nghiệm bộ gõ tiếng Việt, blocking async, CSP tắt, lỗi khóa ngoại DB, build clippy | **6** | **BLOCKER (Chặn release)** |
| 🟡 **P2** | Lệch danh mục đuôi file whitelist, Test mock mirroring, chấm điểm nhận diện encoding | **3** | Cần xử lý theo lộ trình |
| ⚪ **P3** | Cải tiến tài liệu, tối ưu hóa kích thước gói cài đặt | **0** | Đã gom vào các nhóm trên |
| **TỔNG CỘNG** | | **17** | **BASELINE NOT CERTIFIED** |

---

## 2. DANH MỤC CHI TIẾT CÁC PHÁT HIỆN (DETAILED FINDINGS)

### FINDING-P0-IPC-01
- **Severity:** 🔴 P0 (Critical Blocker)
- **Affected Module:** `src-tauri::commands`
- **File Path & Lines:** [`src-tauri/src/commands/mod.rs#L74-L115`](file:///home/quan/workspace/tauri-v2/office-utility-vn/src-tauri/src/commands/mod.rs#L74-L115)
- **Root Cause:** Lệnh IPC cốt lõi `dispatch_tool_task` chỉ khai báo 8 nhánh `match` cho 8 slug; 50 công cụ còn lại rơi vào nhánh wildcard `_ =>` tự động phát tiến độ giả 50%, 100% trong < 1ms, trả về `status: "SUCCESS"` và dội ngược lại `output_paths: req_clone.input_paths.clone()`.
- **Demonstrable Failure Scenario:** Người dùng mở bất kỳ công cụ nào trong 50 công cụ chưa định tuyến (ví dụ: `image-batch`, `pdf-merge-split`, `excel-cleaner-normalizer`), tải tệp lên và bấm "Thực hiện". Giao diện lập tức hiện thông báo xanh "Hoàn tất thành công", nhưng không có bất kỳ byte dữ liệu nào được chuyển đổi, và đường dẫn tệp kết quả chính là tệp đầu vào ban đầu.
- **Reproduction Evidence:** Mã nguồn `src-tauri/src/commands/mod.rs` dòng 74–115; chạy kiểm tra đối chiếu danh mục phát hiện 50/58 slug trên UI bị bỏ rơi vào wildcard; vi phạm quy tắc `IPC001` trong `policy/guardrails.json`.
- **Real-world Impact:** Vi phạm trực tiếp hai bất biến cốt lõi INV-01 (Honest Execution) và INV-02 (Source Preservation). Lừa dối người dùng rằng dữ liệu đã được xử lý trong khi chưa hề đụng tới, tiềm ẩn nguy cơ phá vỡ chuỗi quy trình văn phòng.
- **Proposed Minimal Fix:** Xóa bỏ việc trả về `Ok(SUCCESS)` trong nhánh `_ =>`. Thay bằng việc trả về `Err(CommandError::ValidationFailed { reason: format!("Công cụ '{}' chưa được kích hoạt hoặc đang trong quá trình nâng cấp", req_clone.tool_slug) })`. Lần lượt bổ sung các nhánh match gọi sang struct Engine thực tế cho các công cụ đã hoàn thiện.
- **Required Regression Test:** `src-tauri/tests/verification_ipc.rs::unknown_and_disabled_tools_never_return_success`
- **Verification Commands:** `cargo test -p office-utility-vn --test verification_ipc; python3 scripts/verify_guardrails.py --repo . --mode enforce`
- **Residual Risk:** Do `src-tauri/src/commands/mod.rs` bị khóa cứng bởi `scripts/verify_spine_lock.sh`, việc sửa đổi tệp này bắt buộc phải đi kèm một quy trình Spine Lock Amendment có chữ ký của maintainer.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P0-PDF-01
- **Severity:** 🔴 P0 (Critical Blocker)
- **Affected Module:** `office-engine::doc_suite`
- **File Path & Lines:** 25 tệp trong [`crates/office-engine/src/doc_suite/*.rs`](file:///home/quan/workspace/tauri-v2/office-utility-vn/crates/office-engine/src/doc_suite/) (ví dụ: `pdf_crop_margin.rs#L69-L145`, `pdf_merge_split.rs#L80-L160`, `pdf_redaction.rs#L81-L125`, `pdf_archive_linearize.rs#L66-L105`)
- **Root Cause:** Toàn bộ 25 công cụ xử lý PDF sử dụng `String::from_utf8_lossy(bytes)` trên dữ liệu nhị phân của tệp PDF và tự tìm kiếm chuỗi ASCII `" obj"` và `"endobj"`.
- **Demonstrable Failure Scenario:** Tệp PDF chuẩn tạo từ Word/Acrobat chứa các luồng nén zlib FlateDecode, font nhúng và ảnh nhúng. Khi đi qua `String::from_utf8_lossy`, các byte nhị phân không hợp lệ bị biến thành cụm 3 byte `\u{FFFD}`, làm hỏng hoàn toàn luồng nén. Nếu luồng nén tình cờ chứa chuỗi byte ASCII `endobj`, parser sẽ cắt cụt đối tượng ngay giữa chừng.
- **Reproduction Evidence:** Ghi nhận 99 lỗi `PDF001` và 29 lỗi `PDF002` qua `scripts/verify_guardrails.py`; công cụ độc lập `qpdf --check` báo lỗi hỏng xref và hỏng stream.
- **Real-world Impact:** Phá hủy dữ liệu tệp PDF của người dùng. Tệp xuất ra không thể mở được trên Adobe Acrobat, Chrome PDF Viewer hoặc Apple Preview.
- **Proposed Minimal Fix:** Thay thế toàn bộ 25 bộ parser chuỗi tự chế bằng việc tích hợp crate phân tích nhị phân cây đối tượng AST chuyên dụng (`lopdf`) và động cơ render đồ họa (`pdfium-render`).
- **Required Regression Test:** `crates/office-engine/tests/verification_pdf.rs::malformed_pdf_does_not_produce_success`
- **Verification Commands:** `cargo test -p office-engine --test verification_pdf; qpdf --check <output.pdf>`
- **Residual Risk:** Cần quản lý giới hạn bộ nhớ khi nạp tài liệu PDF lớn (> 100 MB) vào bộ nhớ RAM.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P0-PDF-02
- **Severity:** 🔴 P0 (Critical Blocker)
- **Affected Module:** `office-engine::doc_suite::pdf_merge_split`
- **File Path & Lines:** [`crates/office-engine/src/doc_suite/pdf_merge_split.rs#L254-L287`](file:///home/quan/workspace/tauri-v2/office-utility-vn/crates/office-engine/src/doc_suite/pdf_merge_split.rs#L254-L287) và `#L420-L450`
- **Root Cause:** Khi tách hoặc ghép trang, mã nguồn chỉ copy từ điển `/Type /Page` và trỏ lại `/Parent 2 0 R`, nhưng hoàn toàn không duyệt đệ quy (Transitive Dependency Closure) để copy các đối tượng tài nguyên được trang đó tham chiếu (`/Contents`, `/Resources`, font chữ, XObject hình ảnh).
- **Demonstrable Failure Scenario:** Người dùng tách trang từ một tài liệu scan có hình ảnh hoặc hợp đồng có font nhúng. Tệp PDF mới tạo ra chứa các con trỏ `5 0 R`, `6 0 R` nhưng trong tệp mới các đối tượng này không tồn tại. Mở file bằng Acrobat sẽ báo lỗi "An error exists on this page" và hiển thị trang giấy trắng.
- **Reproduction Evidence:** Mã nguồn dòng 177–179: `let mut all_objects = vec![catalog, pages_root]; all_objects.extend(split_page_objects);` hoàn toàn không có đối tượng nội dung hay tài nguyên.
- **Real-world Impact:** Mất toàn bộ nội dung văn bản và hình ảnh trong các trang PDF được tách/ghép; tệp xuất ra hoàn toàn vô dụng.
- **Proposed Minimal Fix:** Xây dựng thuật toán duyệt đệ quy toàn bộ cây tham chiếu gián tiếp của từng trang được chọn, đánh số lại Object ID tuần tự (1..N) và cập nhật đồng bộ các tham chiếu `X 0 R`.
- **Required Regression Test:** `crates/office-engine/tests/verification_pdf.rs::real_pdf_merge_split_preserves_resources`
- **Verification Commands:** `cargo test -p office-engine --test verification_pdf; pdftotext -layout <output.pdf> -`
- **Residual Risk:** Cần tránh đệ quy vô hạn khi gặp các cấu trúc tham chiếu vòng tròn trong từ điển PDF.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P0-FAKE-01
- **Severity:** 🔴 P0 (Critical Blocker)
- **Affected Module:** `office-engine::doc_suite::vietnamese_ocr`
- **File Path & Lines:** [`crates/office-engine/src/doc_suite/vietnamese_ocr.rs#L121-L130`](file:///home/quan/workspace/tauri-v2/office-utility-vn/crates/office-engine/src/doc_suite/vietnamese_ocr.rs#L121-L130)
- **Root Cause:** Hàm `perform_ocr_extract` trả về chuỗi văn bản hardcode tĩnh cố định cho mọi tệp đầu vào.
- **Demonstrable Failure Scenario:** Bất kỳ tệp ảnh chụp hoặc tài liệu scan nào đưa vào công cụ đều cho ra đúng một kết quả văn bản là hóa đơn tiền điện của "Nguyễn Văn An" hoặc quốc hiệu tiêu ngữ.
- **Reproduction Evidence:** Mã nguồn dòng 122 và 129 chứa chuỗi hardcode: `"HÓA ĐƠN GIÁ TRỊ GIA TĂNG (TIỀN ĐIỆN / TIỀN NƯỚC)\nCông ty Điện lực Việt Nam - Chi nhánh Hà Nội..."`.
- **Real-world Impact:** Lừa dối người dùng; hoàn toàn không có tính năng OCR thực tế; vi phạm nghiêm trọng INV-01 và INV-02.
- **Proposed Minimal Fix:** Chặn truy cập công cụ trên UI và trả về `CommandError::ValidationFailed { reason: "Tính năng OCR đang chờ tích hợp mô hình On-Device ONNX/Tesseract".into() }`.
- **Required Regression Test:** `crates/office-engine/tests/verification_ocr.rs::ocr_result_depends_on_distinct_real_inputs`
- **Verification Commands:** `cargo test -p office-engine --test verification_ocr`
- **Residual Risk:** Kích thước binary gói cài đặt sẽ tăng khi đóng gói mô hình OCR ngoại tuyến.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P0-FAKE-02
- **Severity:** 🔴 P0 (Critical Blocker)
- **Affected Module:** `office-engine::media_studio::video_to_gif_webp`
- **File Path & Lines:** [`crates/office-engine/src/media_studio/video_to_gif_webp.rs#L254-L293`](file:///home/quan/workspace/tauri-v2/office-utility-vn/crates/office-engine/src/media_studio/video_to_gif_webp.rs#L254-L293)
- **Root Cause:** Hàm `extract_frames_from_video` không hề giải mã video MP4/MOV, mà sinh ra một mảng ảnh gradient với vạch sáng màu vàng `(255, 215, 0)` di chuyển qua lại.
- **Demonstrable Failure Scenario:** Người dùng đưa tệp video clip của mình vào để tạo ảnh GIF động. Kết quả nhận được là một ảnh GIF hoạt họa hình vạch vàng chạy ngang màn hình, hoàn toàn không có hình ảnh từ video gốc.
- **Reproduction Evidence:** Mã nguồn dòng 270–285 tính toán `let bar_x = (progress_ratio * base_w as f32) as i32; let dist = (x as i32 - bar_x).abs();` để tô màu vàng; vi phạm rule `FAKE002`.
- **Real-world Impact:** Lừa dối người dùng; tạo dữ liệu rác; tính năng hoàn toàn giả tạo.
- **Proposed Minimal Fix:** Tạm thời vô hiệu hóa công cụ trên giao diện cho đến khi tích hợp bộ giải mã video phần cứng OS Native hoặc FFmpeg nhúng.
- **Required Regression Test:** `crates/office-engine/tests/verification_media.rs::video_frames_match_timestamps_of_input`
- **Verification Commands:** `cargo test -p office-engine --test verification_media`
- **Residual Risk:** Cần thẩm tra giấy phép bản quyền của codec giải mã video (LGPL/GPL).
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P0-FAKE-03
- **Severity:** 🔴 P0 (Critical Blocker)
- **Affected Module:** `office-engine::doc_suite::pdf_to_long_image`
- **File Path & Lines:** [`crates/office-engine/src/doc_suite/pdf_to_long_image.rs#L73-L100`](file:///home/quan/workspace/tauri-v2/office-utility-vn/crates/office-engine/src/doc_suite/pdf_to_long_image.rs#L73-L100)
- **Root Cause:** Hàm `render_pdf_page_to_bitmap` không kết xuất đồ họa thực tế, mà chỉ vẽ khung trắng với dải màu đầu trang và vạch xám tượng trưng nếu text chứa toán tử `"re"` hoặc `"Tj"`.
- **Demonstrable Failure Scenario:** Chuyển đổi PDF sang ảnh dài chỉ thu được một bức ảnh trắng sọc xám, không đọc được bất kỳ nội dung nào của tài liệu.
- **Reproduction Evidence:** Mã nguồn dòng 73–100; vi phạm rule `FAKE003`.
- **Real-world Impact:** Phá hủy tính năng xem/ghép ảnh PDF; không thể sử dụng trong sản xuất.
- **Proposed Minimal Fix:** Ủy quyền kết xuất cho thư viện render chuẩn (PDFium) để tạo bitmap thật từ vector PDF.
- **Required Regression Test:** `crates/office-engine/tests/verification_pdf.rs::rendered_pdf_pixels_represent_input_content`
- **Verification Commands:** `cargo test -p office-engine --test verification_pdf`
- **Residual Risk:** Cần kiểm soát dung lượng bộ nhớ khi ghép ảnh dài hàng chục trang.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P0-FAKE-04
- **Severity:** 🔴 P0 (Critical Blocker)
- **Affected Module:** `office-engine::doc_suite::pdf_pki_sign`
- **File Path & Lines:** [`crates/office-engine/src/doc_suite/pdf_pki_sign.rs#L186-L202`](file:///home/quan/workspace/tauri-v2/office-utility-vn/crates/office-engine/src/doc_suite/pdf_pki_sign.rs#L186-L202)
- **Root Cause:** Nhúng chuỗi ASCII text thô `PKCS7-SIG:CN=...` đệm số 0 vào trường `/Contents` thay vì sinh cấu trúc chữ ký ASN.1 DER CMS tiêu chuẩn.
- **Demonstrable Failure Scenario:** Tệp PDF sau khi ký khi mở trong Adobe Acrobat hoặc kiểm tra bằng lệnh `pdfsig` bị từ chối và báo lỗi chữ ký không hợp lệ / không thể nhận dạng.
- **Reproduction Evidence:** Mã nguồn dòng 186–193; vi phạm rule `PDF003`.
- **Real-world Impact:** Tài liệu pháp lý không có giá trị chữ ký số; vi phạm các chuẩn chữ ký điện tử PAdES.
- **Proposed Minimal Fix:** Khóa công cụ cho đến khi tích hợp crate ký số mật mã học CMS (RFC 5652).
- **Required Regression Test:** `crates/office-engine/tests/verification_signing.rs::signed_pdf_has_cryptographically_valid_cms_and_byterange`
- **Verification Commands:** `cargo test -p office-engine --test verification_signing; pdfsig -nocert <signed.pdf>`
- **Residual Risk:** Cần chứng thư số X.509 hợp lệ để kiểm thử.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P0-FAKE-05
- **Severity:** 🔴 P0 (Critical Blocker)
- **Affected Module:** `office-engine::doc_suite::pdf_protect_sign`
- **File Path & Lines:** [`crates/office-engine/src/doc_suite/pdf_protect_sign.rs#L149-L165`](file:///home/quan/workspace/tauri-v2/office-utility-vn/crates/office-engine/src/doc_suite/pdf_protect_sign.rs#L149-L165) và `#L196-L202`
- **Root Cause:** Dùng thuật toán XOR đối xứng đơn giản (`cipher_xor`) với chuỗi muối cố định `b"OFFICE_SEC_SALT_VN_2026"` nhưng tuyên bố là mã hóa chuẩn AES-128 / AES-256.
- **Demonstrable Failure Scenario:** Người dùng đặt mật khẩu cho tệp PDF bằng ứng dụng, sau đó gửi cho đối tác mở bằng Adobe Acrobat. Acrobat báo lỗi tệp hỏng và không thể nhập mật khẩu để mở khóa.
- **Reproduction Evidence:** Mã nguồn dòng 159–165 và trailer tự tạo dòng 197.
- **Real-world Impact:** Tuyên bố sai lệch nghiêm trọng về bảo mật; tạo ra tài liệu không tương thích với thế giới bên ngoài; rủi ro lộ dữ liệu nhạy cảm.
- **Proposed Minimal Fix:** Vô hiệu hóa tính năng đặt mật khẩu cho đến khi hiện thực hóa đầy đủ Standard Security Handler Revision 6 (AES-256) chuẩn ISO 32000-1.
- **Required Regression Test:** `crates/office-engine/tests/verification_secrets.rs::real_pdf_aes_encryption_roundtrip`
- **Verification Commands:** `cargo test -p office-engine --test verification_secrets`
- **Residual Risk:** Thuật toán mã hóa PDF chuẩn yêu cầu xử lý từ điển `/O`, `/U`, `/UE`, `/OE` và `/Perms` phức tạp.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P0-HARNESS-01
- **Severity:** 🔴 P0 (Critical Blocker)
- **Affected Module:** `verification-kit`
- **File Path & Lines:** [`tests/verification/source_test_inventory.json#L1-L91`](file:///home/quan/workspace/tauri-v2/office-utility-vn/tests/verification/source_test_inventory.json#L1-L91)
- **Root Cause:** 17 tệp kiểm thử nguồn theo hợp đồng bắt buộc chưa hề được tạo trong `src-tauri/tests/` và `crates/office-engine/tests/`; adapter nhị phân `office_verify_adapter` chưa được xây dựng.
- **Demonstrable Failure Scenario:** Khi chạy cổng kiểm định release `verify_source_test_inventory.py` và `verify_behavior.py`, toàn bộ quy trình lập tức thoát với mã lỗi 1.
- **Reproduction Evidence:** Lệnh `python3 scripts/verify_source_test_inventory.py --repo .` báo `FAIL` 17 tests.
- **Real-world Impact:** Kho mã nguồn không có bất kỳ bằng chứng kiểm định độc lập nào được xác nhận; không thể chứng nhận baseline.
- **Proposed Minimal Fix:** Xây dựng binary adapter `office_verify_adapter` và hiện thực hóa đầy đủ 17 bài kiểm tra nguồn.
- **Required Regression Test:** `python3 scripts/verify_source_test_inventory.py --repo .`
- **Verification Commands:** `python3 scripts/verify_source_test_inventory.py --repo .; python3 scripts/verify_behavior.py --repo .`
- **Residual Risk:** Cần bộ fixture nhị phân thực tế độc lập trong thư mục `fixtures/`.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P1-IME-01
- **Severity:** 🟠 P1 (High Quality Defect)
- **Affected Module:** `office-utility-vn-ui::tools`
- **File Path & Lines:** 43 tệp view trong [`src/tools/*_view.rs`](file:///home/quan/workspace/tauri-v2/office-utility-vn/src/tools/)
- **Root Cause:** Liên kết hai chiều `prop:value` và `on:input` trên thẻ input mà không xử lý cờ composition.
- **Demonstrable Failure Scenario:** Người dùng gõ tiếng Việt bằng UniKey hoặc EVKey trong các ô nhập liệu, Leptos ép cập nhật DOM giữa phiên gõ làm ngắt nhịp bộ gõ, nhảy con trỏ về cuối ô hoặc nuốt mất ký tự dấu tiếng Việt.
- **Reproduction Evidence:** `scripts/verify_guardrails.py` phát hiện 43 vi phạm rule `IME001`.
- **Real-world Impact:** Trải nghiệm người dùng văn phòng Việt Nam bị suy giảm nghiêm trọng; gây ức chế khi nhập văn bản.
- **Proposed Minimal Fix:** Thay thế bằng component `ImeSafeInput` có bắt sự kiện `compositionstart` và `compositionend`.
- **Required Regression Test:** `tests/e2e/verification_ime.spec.ts::vietnamese_ime_composition_is_not_interrupted`
- **Verification Commands:** `python3 scripts/verify_guardrails.py --repo . --mode enforce`
- **Residual Risk:** Cần xác thực trên cả WebKitGTK và WebView2.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P1-DB-01
- **Severity:** 🟠 P1 (High Reliability Defect)
- **Affected Module:** `src-tauri::db`
- **File Path & Lines:** [`src-tauri/src/db/mod.rs#L131-L148`](file:///home/quan/workspace/tauri-v2/office-utility-vn/src-tauri/src/db/mod.rs#L131-L148)
- **Root Cause:** Danh mục seed data trong `default_tools` sử dụng slug cũ không khớp với slug chuẩn của UI Registry và Engine; bảng `task_history` có khóa ngoại `REFERENCES tool_settings(slug)` gây lỗi foreign key và bị nuốt bởi `let _ =`.
- **Demonstrable Failure Scenario:** Người dùng thực hiện các tác vụ thành công nhưng bảng lịch sử tác vụ Task History hoàn toàn không lưu được bản ghi nào.
- **Reproduction Evidence:** Đối chiếu danh mục slug trong `db/mod.rs:132-147` với `registry.rs:35-738`.
- **Real-world Impact:** Mất hoàn toàn tính năng lưu trữ lịch sử tác vụ; không thể truy vết kiểm toán.
- **Proposed Minimal Fix:** Cập nhật seed data với đầy đủ 58 slug chuẩn; loại bỏ `let _ =` và log lỗi nếu thêm lịch sử thất bại.
- **Required Regression Test:** `src-tauri/tests/verification_db.rs::task_history_persists_for_all_valid_tool_slugs`
- **Verification Commands:** `cargo test -p office-utility-vn --test verification_db`
- **Residual Risk:** Cần migration script cho các CSDL SQLite cũ trên máy người dùng.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P1-SEC-01
- **Severity:** 🟠 P1 (High Security Risk)
- **Affected Module:** `src-tauri::config`
- **File Path & Lines:** [`src-tauri/tauri.conf.json#L13`](file:///home/quan/workspace/tauri-v2/office-utility-vn/src-tauri/tauri.conf.json#L13) và [`#L21-L23`](file:///home/quan/workspace/tauri-v2/office-utility-vn/src-tauri/tauri.conf.json#L21-L23)
- **Root Cause:** Cấu hình `csp: null` và `withGlobalTauri: true` trong cấu hình ứng dụng desktop.
- **Demonstrable Failure Scenario:** Kẻ tấn công khai thác lỗ hổng XSS thông qua việc nạp file HTML/Markdown độc hại có thể truy cập trực tiếp toàn bộ lệnh native của Tauri qua `window.__TAURI__`.
- **Reproduction Evidence:** Nội dung tệp `tauri.conf.json`.
- **Real-world Impact:** Mở rộng bề mặt tấn công; vi phạm nguyên tắc phòng thủ chiều sâu.
- **Proposed Minimal Fix:** Thiết lập CSP nghiêm ngặt và chuyển `withGlobalTauri: false`.
- **Required Regression Test:** Static configuration check trong CI.
- **Verification Commands:** `python3 scripts/verify_guardrails.py --repo . --mode enforce`
- **Residual Risk:** Cần đảm bảo frontend không bị vỡ do chính sách CSP mới.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P1-IO-01
- **Severity:** 🟠 P1 (High Performance & Concurrency Risk)
- **Affected Module:** `src-tauri::commands`
- **File Path & Lines:** [`src-tauri/src/commands/mod.rs#L275-L294`](file:///home/quan/workspace/tauri-v2/office-utility-vn/src-tauri/src/commands/mod.rs#L275-L294)
- **Root Cause:** Hàm bất đồng bộ `inspect_file_metadata` thực thi `std::fs::metadata`, `File::open` và `file.read` đồng bộ trên luồng Tokio; kết hợp `.unwrap_or(0)` nuốt lỗi I/O.
- **Demonstrable Failure Scenario:** Khi duyệt tệp trên ổ đĩa mạng SMB/NFS hoặc đĩa quang chậm, toàn bộ worker threads của Tokio bị chặn, giao diện người dùng bị đơ cứng.
- **Reproduction Evidence:** Dòng 283–293 trong `src-tauri/src/commands/mod.rs`; vi phạm rule `IO001` và `IO002`.
- **Real-world Impact:** Vi phạm NFR-PERF-02 (giữ 60 FPS không khóa UI); nuốt lỗi đọc đĩa thành tệp rỗng 0 byte.
- **Proposed Minimal Fix:** Chuyển toàn bộ thao tác I/O sang `tauri::async_runtime::spawn_blocking` hoặc `tokio::fs`; trả về lỗi `CommandError::IoError` rõ ràng.
- **Required Regression Test:** `src-tauri/tests/verification_commands.rs::slow_file_metadata_does_not_starve_async_executor`
- **Verification Commands:** `cargo test -p office-utility-vn --test verification_commands`
- **Residual Risk:** Cần Spine Lock Amendment do sửa đổi `src-tauri/src/commands/mod.rs`.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P1-BUILD-01
- **Severity:** 🟠 P1 (High Quality Defect)
- **Affected Module:** `workspace::build`
- **File Path & Lines:** Toàn bộ workspace (`crates/office-engine`, `src/`, `benches/`)
- **Root Cause:** 97 lỗi cảnh báo Clippy (`collapsible_if`, `unnecessary_map_or`, `manual_unwrap_or_default`) trên Rust 1.99 và hàng trăm dòng mã chưa định dạng chuẩn `cargo fmt`.
- **Demonstrable Failure Scenario:** Lệnh `cargo clippy --workspace --all-targets --all-features -- -D warnings` và `cargo fmt --all -- --check` thất bại ngay lập tức, chặn toàn bộ quy trình CI.
- **Reproduction Evidence:** Chạy lệnh `cargo fmt --all -- --check` (thất bại); `cargo clippy` báo 64 lỗi trong `office-engine` và 33 lỗi trong `office-utility-vn-ui`.
- **Real-world Impact:** Kho mã nguồn không đạt chuẩn chất lượng biên dịch sạch (Clean Build); không thể vượt qua cổng PR CI.
- **Proposed Minimal Fix:** Chạy `cargo fmt --all` và khắc phục triệt để các cảnh báo clippy bằng cú pháp gập `if let ... && ...` và `is_none_or`.
- **Required Regression Test:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
- **Verification Commands:** `cargo fmt --all -- --check; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- **Residual Risk:** Không có.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P2-PERSIST-01
- **Severity:** 🟡 P2 (Medium Gap)
- **Affected Module:** `src-tauri::safe_persistence`
- **File Path & Lines:** [`src-tauri/src/safe_persistence.rs#L18-L25`](file:///home/quan/workspace/tauri-v2/office-utility-vn/src-tauri/src/safe_persistence.rs#L18-L25)
- **Root Cause:** Danh mục `ALLOWED_EXTENSIONS` thiếu các đuôi tệp: `webp`, `gif`, `zip`, `ics`, `sql`, `md`.
- **Demonstrable Failure Scenario:** Khi tích hợp các công cụ xuất ảnh webp, gif hoặc file lịch ics vào hệ thống Native Save Grant, lệnh lưu sẽ bị từ chối với lỗi phần mở rộng không hợp lệ.
- **Reproduction Evidence:** Mảng `ALLOWED_EXTENSIONS` tại `safe_persistence.rs:18-25`.
- **Real-world Impact:** Cản trở việc mở rộng các tính năng xuất dữ liệu an toàn.
- **Proposed Minimal Fix:** Thêm đầy đủ các đuôi tệp văn phòng hợp lệ vào mảng whitelist.
- **Required Regression Test:** `src-tauri/tests/verification_persistence.rs::all_app_output_extensions_are_authorized`
- **Verification Commands:** `cargo test -p office-utility-vn --test verification_persistence`
- **Residual Risk:** Cần đảm bảo không bao giờ thêm các đuôi thực thi nguy hiểm (`.exe`, `.sh`, `.bat`).
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P2-TEST-01
- **Severity:** 🟡 P2 (Medium Quality Smell)
- **Affected Module:** `workspace::tests`
- **File Path & Lines:** [`crates/office-engine/src/**/*.rs`](file:///home/quan/workspace/tauri-v2/office-utility-vn/crates/office-engine/src/) và [`tests/**/*.rs`](file:///home/quan/workspace/tauri-v2/office-utility-vn/tests/)
- **Root Cause:** 379 unit tests vượt qua 100% nhờ tự sinh fixture ASCII đồ chơi khớp với parser lỗi (Test Mock Mirroring).
- **Demonstrable Failure Scenario:** Tỷ lệ test pass 100% đánh lừa đội ngũ phát triển rằng hệ thống hoạt động hoàn hảo, trong khi môi trường thực tế gặp lỗi sập nghiêm trọng.
- **Reproduction Evidence:** Hàm `create_test_wide_margin_pdf` trong `pdf_crop_margin.rs`.
- **Real-world Impact:** Triệt tiêu giá trị phòng thủ hồi quy của hệ thống kiểm thử tự động.
- **Proposed Minimal Fix:** Bổ sung các fixture nhị phân thực tế độc lập có nguồn gốc Word/Acrobat/Scan vào thư mục `fixtures/`.
- **Required Regression Test:** Toàn bộ test suite chạy trên real fixtures.
- **Verification Commands:** `cargo test --workspace`
- **Residual Risk:** Cần quản lý dung lượng repo khi thêm tệp fixture.
- **Confidence Level:** CONFIRMED (100%)

---

### FINDING-P2-DATA-01
- **Severity:** 🟡 P2 (Medium Risk)
- **Affected Module:** `office-engine::data_lab::vietnamese_font_converter`
- **File Path & Lines:** [`crates/office-engine/src/data_lab/vietnamese_font_converter.rs#L175-L185`](file:///home/quan/workspace/tauri-v2/office-utility-vn/crates/office-engine/src/data_lab/vietnamese_font_converter.rs#L175-L185)
- **Root Cause:** Tự động nhận diện bảng mã VNI/TCVN3 dựa trên ánh xạ ký tự Latin-1 (`bytes.iter().map(|&b| b as char)`) để tìm marker đơn lẻ mà chưa có mô hình chấm điểm phân phối n-gram byte.
- **Demonstrable Failure Scenario:** Tệp văn bản chứa chuỗi ký tự đặc biệt hoặc mã nhị phân có thể bị nhận dạng nhầm bảng mã, dẫn tới việc chuyển đổi sai làm hỏng văn bản.
- **Reproduction Evidence:** Mã nguồn dòng 179; cảnh báo `VNI001` trong `guardrails.json`.
- **Real-world Impact:** Nguy cơ sai lệch ký tự khi chuyển mã tài liệu tiếng Việt cổ.
- **Proposed Minimal Fix:** Bổ sung thuật toán chấm điểm tần suất n-gram tiếng Việt và trả về trạng thái `Ambiguous` yêu cầu người dùng xác nhận khi độ tin cậy < 80%.
- **Required Regression Test:** `crates/office-engine/tests/verification_encoding.rs::vni_and_tcvn3_goldens_preserve_vietnamese_diacritics`
- **Verification Commands:** `cargo test -p office-engine --test verification_encoding`
- **Residual Risk:** Cần tập mẫu ngữ liệu tiếng Việt lớn để chuẩn hóa bảng tần suất.
- **Confidence Level:** CONFIRMED (100%)
