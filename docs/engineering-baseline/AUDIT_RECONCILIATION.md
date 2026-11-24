# Báo Cáo Đối Chiếu & Hoàn Thiện Kiểm Toán Kỹ Thuật
## Audit Integrity Reconciliation Report — Office Utility VN

**Vai trò:** Giám đốc Kiến trúc Phần mềm, Kỹ sư Kiểm định Độc lập & Kiến trúc sư An toàn Phát hành  
**Trạng thái kiểm toán:** READ-ONLY INVESTIGATION & AUDIT RECONCILIATION  
**Commit cơ sở kiểm tra:** `8c57f9655bb0841064973effd6ddeb72c0fa3599` (Git HEAD)  
**Ngày thực hiện đối chiếu:** 22/11/2026  
**Phiên bản tài liệu:** 2.0.0 (Reconciled Baseline)

---

## 1. TỔNG QUAN ĐỐI CHIẾU (EXECUTIVE SUMMARY)

Báo cáo kiểm toán ban đầu đã trả về kết luận **`BASELINE NOT CERTIFIED`** do phát hiện hàng loạt lỗ hổng kiến trúc, động cơ giả mạo (Fake Engines), vi phạm bất biến an toàn dữ liệu và thiếu hụt hạ tầng kiểm định.

Tuy nhiên, quá trình đối soát độc lập (Phase 0 Audit Integrity Reconciliation) phát hiện một số sai lệch số liệu và thiếu sót giữa các tài liệu kiểm toán trước đó:
1. **Lệch số lượng phát hiện (P0/P1/P2 Count Mismatch):** Tiêu đề tóm tắt trong `findings_register.json` ghi `8 P0 / 6 P1 / 3 P2` (tổng 17), nhưng thực tế trong mảng JSON có **9 P0 / 5 P1 / 3 P2** (tổng 17), và thiếu hẳn phát hiện `FINDING-P0-MEDIA-01` (được nhắc đến trong `REMEDIATION_PLAN.md`).
2. **Thiếu sót `FINDING-P0-MEDIA-01`:** Lỗi trả về ảnh màu trắng/xám giả mạo khi giải mã PNG thất bại (rule `PNG002`) đã bị bỏ quên trong JSON register. Khi bổ sung đầy đủ, danh mục kiểm toán có chính xác **10 P0 / 5 P1 / 3 P2 = 18 phát hiện**.
3. **Phân định rõ Định tuyến IPC vs Tính đúng đắn của Động cơ:** Trong số 58 công cụ UI, chỉ có **8 công cụ** được nối trực tiếp vào `dispatch_tool_task`. **50 công cụ** rơi vào nhánh wildcard `_ =>` giả mạo trạng thái `SUCCESS`. Tuy nhiên, trong 50 công cụ unrouted, nhiều công cụ đã có mã nguồn Rust thực tế trong `crates/office-engine` (như `image-batch`, `vietnamese-font-converter`, `base64-file-codec`), trong khi một số công cụ khác lại là Fake Engine (như `video-to-gif-webp`, `pdf-pki-sign`).
4. **Đối chiếu số lượng trường nhập liệu IME:** Con số "144 trường nhập liệu" trong kiểm toán ban đầu là tổng số thẻ `<input>` thô trong mã giao diện `src/`. Trong khi đó, con số **43 vi phạm** là số lượng chính xác các controlled input Leptos (`prop:value` đi kèm `on:input`) thiếu bộ lắng nghe `compositionstart`/`compositionend` (rule `IME001`).
5. **Chuẩn hóa 12 Quy tắc Bất biến sang 10 Bất biến Hệ thống:** 12 nguyên tắc không thể thương lượng (`verification_non_negotiable.md`) đã được cô đọng một cách có cấu trúc thành 10 bất biến kỹ thuật trong `QUALITY_BASELINE.md` (INV-01 đến INV-10), kết hợp 2 quy tắc còn lại vào hợp đồng quy trình PR và tiêu chí chặn phát hành.
6. **Truy vết mã chưa commit trong Working Tree:** Có 5 tệp đang có uncommitted changes (`image_batch.rs`, `base64_file_codec.rs`, `vietnamese_font_converter.rs`, `safe_persistence.rs`, `.github/CODEOWNERS`). Những thay đổi này là các nỗ lực vá lỗi ban đầu nhưng chưa được kiểm định độc lập (`FIXED_UNVERIFIED`).

---

## 2. CHI TIẾT 10 NỘI DUNG ĐỐI CHIẾU BẮT BUỘC (PHASE 0 CHECKS)

### Kiểm tra 1 & 2: Tính toán lại Số lượng P0/P1/P2 & Giải mã Lệch Số liệu JSON

Trong tệp `docs/engineering-baseline/findings_register.json` phiên bản 1.0.0:
- **Phần tóm tắt (Header Summary) ghi:**
  ```json
  "summary": { "total_findings": 17, "p0_count": 8, "p1_count": 6, "p2_count": 3, "p3_count": 0 }
  ```
- **Đếm thực tế các phần tử trong mảng `findings`:**
  - P0: `FINDING-P0-IPC-01`, `FINDING-P0-PDF-01`, `FINDING-P0-PDF-02`, `FINDING-P0-FAKE-01`, `FINDING-P0-FAKE-02`, `FINDING-P0-FAKE-03`, `FINDING-P0-FAKE-04`, `FINDING-P0-FAKE-05`, `FINDING-P0-HARNESS-01` -> **9 phát hiện P0**.
  - P1: `FINDING-P1-IME-01`, `FINDING-P1-DB-01`, `FINDING-P1-SEC-01`, `FINDING-P1-IO-01`, `FINDING-P1-BUILD-01` -> **5 phát hiện P1**.
  - P2: `FINDING-P2-PERSIST-01`, `FINDING-P2-TEST-01`, `FINDING-P2-DATA-01` -> **3 phát hiện P2**.
  - Tổng số phần tử: 9 + 5 + 3 = **17 phát hiện**.

**Nguyên nhân sai lệch:** Khi soạn thảo bản nháp ban đầu, `FINDING-P0-HARNESS-01` ban đầu được phân loại là P1 (làm P1 = 6, P0 = 8). Sau đó, kiểm toán viên nâng `HARNESS-01` lên mức P0 trong mảng JSON nhưng quên cập nhật lại trường đếm tổng hợp `"summary"`, dẫn đến mâu thuẫn giữa phần tóm tắt và nội dung mảng.

---

### Kiểm tra 3: Phục hồi và Hợp nhất `FINDING-P0-MEDIA-01`

Trong `docs/engineering-baseline/REMEDIATION_PLAN.md` dòng 137, kế hoạch PR-07 xử lý:
> `- **Phát hiện xử lý:** FINDING-P0-MEDIA-01, FINDING-P2-PERSIST-01.`

Tuy nhiên, trong `findings_register.json` mảng phần tử lại không có ID này. 

**Xác minh thực tế trong mã nguồn:**
- Tệp `crates/office-engine/src/media_studio/image_batch.rs` tại commit HEAD (`8c57f9655bb0841064973effd6ddeb72c0fa3599`):
  Hàm `decode_png` khi gặp lỗi bit depth hoặc zlib decode thất bại đã trả về:
  `Ok(RawImage { width, height, pixels: vec![200; (width * height * 3) as usize] })` và `pixels: vec![255; ...]`.
- Hành vi này vi phạm quy tắc guardrail `PNG002` (P0) và bất biến INV-01/INV-06 (sinh ảnh màu xám/trắng 200/255 thay vì báo lỗi xác thực).
- **Quyết định đối chiếu:** Phục hồi `FINDING-P0-MEDIA-01` vào mảng chính thức của `findings_register.json` và `FINDINGS_REGISTER_RECONCILED.md`.
- **Số lượng sau khi hợp nhất:** **10 P0 / 5 P1 / 3 P2 = 18 phát hiện**.

---

### Kiểm tra 4 & 5: Đối chiếu 58 UI Slugs, 65 Engine Structs & Định Tuyến IPC

Quá trình quét tự động toàn bộ cây mã nguồn đã phân loại chính xác mối liên hệ giữa giao diện Leptos, tầng IPC Tauri và động cơ Rust:

| Hạng mục | Số lượng | Danh sách / Chi tiết |
| :--- | :---: | :--- |
| **Tool Slugs đăng ký trên UI** (`registry.rs`) | **58** | 58 slug người dùng nhìn thấy trong Unified 3-Zone Workspace |
| **Được nối trực tiếp trong IPC** (`commands/mod.rs`) | **8** | `vietnamese-number-to-words`, `vn-cccd-mst-validator`, `vietnamese-ocr`, `pdf-booklet-imposition`, `pdf-to-long-image`, `pdf-grayscale-print`, `pdf-repair-corrupt`, `pdf-table-extractor` |
| **Bị rơi vào Wildcard IPC (`_ =>`)** | **50** | 50 slug còn lại nhận kết quả `SUCCESS` giả mạo và trả lại file đầu vào |
| **Triển khai `OfficeTool` trong Engine** | **65** | 65 struct trong `crates/office-engine` |
| **Động cơ mồ côi (Orphaned in Engine)** | **8** | `data-profiler`, `fuzzy-clustering`, `recipe-replay`, `schema-quality-gate`, `universal-converter`, `visual-reconciler`, `vn-einvoice-auditor`, `vn-font-encoding-doc` |
| **Lệch Slug UI vs Engine** | **1** | UI dùng `universal-file-converter-hub`, Engine dùng `universal-converter` |

#### Phân định Trạng thái Định tuyến vs Tính đúng đắn của Động cơ (Crucial Distinction)

Rất nhiều kiểm toán viên nhầm lẫn giữa *chưa nối dây (unrouted)* và *chưa viết động cơ (unimplemented)*. Thực tế repository phân hóa thành 5 nhóm trạng thái riêng biệt:

1. **Nhóm A — Routed + Algorithmic Correct (Đã nối & Hoạt động đúng):**
   - Ví dụ: `vietnamese-number-to-words`, `vn-cccd-mst-validator`. Động cơ thuật toán thuần túy, tính toán chính xác, kiểm thử nội bộ đạt.
2. **Nhóm B — Routed + Fake / Defective (Đã nối nhưng Giả mạo / Lỗi nặng):**
   - Ví dụ: `vietnamese-ocr` (trả về văn bản hardcode), `pdf-to-long-image` (vẽ vạch xám tượng trưng), `pdf-repair-corrupt` (thay thế chuỗi đơn giản làm hỏng file).
3. **Nhóm C — Unrouted + Working Engine (Chưa nối IPC nhưng Động cơ Đã Hoàn Thiện):**
   - Ví dụ: `image-batch` (hỗ trợ đầy đủ PNG, JPEG, BMP), `vietnamese-font-converter` (bảng mã TCVN3, VNI đầy đủ), `base64-file-codec` (stream I/O an toàn), `excel-cleaner-normalizer` (phân tích bảng tính qua calamine), `list-set-operations`.
   - **Đánh giá:** Các công cụ này có mã nguồn Rust chất lượng tốt nhưng người dùng không thể sử dụng vì nhánh `dispatch_tool_task` bỏ quên!
4. **Nhóm D — Unrouted + Fake / Lossy Engine (Chưa nối & Động cơ Giả mạo/Lỗi nhị phân):**
   - Ví dụ: `video-to-gif-webp` (vẽ dải màu gradient), `pdf-protect-sign` (mã hóa XOR), `pdf-pki-sign` (chữ ký ASCII hex), các công cụ PDF thao tác bằng `from_utf8_lossy`.
5. **Nhóm E — Orphaned Engine Structs (Động cơ không có trên UI):**
   - 8 struct trong engine không được đưa vào `registry.rs`.

---

### Kiểm tra 6: Đối chiếu Số liệu Lỗi Gõ Tiếng Việt Leptos IME

- **Báo cáo sơ bộ:** Ghi nhận "144 suspect inputs".
- **Kiểm tra đối chiếu thực tế:**
  - Lệnh `grep -rn "<input" src/ | wc -l` trả về **155 thẻ `<input>`**. Trong đó có các input kiểu `type="checkbox"`, `type="radio"`, `type="file"`, hoặc các input không bị kiểm soát giá trị hai chiều (`uncontrolled`).
  - Khi chạy bộ kiểm định bảo vệ `scripts/verify_guardrails.py` với quy tắc `IME001`:
    Bộ phân tích tìm kiếm các thẻ `<input>` hoặc `<textarea>` có **đồng thời cả hai thuộc tính**:
    1. Ràng buộc dữ liệu hai chiều: `prop:value`
    2. Lắng nghe cập nhật: `on:input`
    3. **KHÔNG** bắt sự kiện `on:compositionstart` và `on:compositionend`.
  - Kết quả: Có chính xác **43 vị trí vi phạm `IME001`** nằm rải rác trên 43 tệp view trong `src/tools/` và `src/components/`.
- **Kết luận:** Con số 144/155 là tổng số thẻ nhập liệu; con số **43** là số lượng vị trí thực tế gây lỗi nuốt dấu UniKey/EVKey cần khắc phục.

---

### Kiểm tra 7: Đối chiếu 12 Quy tắc Non-Negotiables với 10 Bất biến Hệ thống

Bảng ánh xạ trực tiếp từ `.agents/rules/verification_non_negotiable.md` sang `QUALITY_BASELINE.md`:

| Quy tắc Gốc (12 Non-Negotiables) | Bất biến Hệ thống (10 Invariants) | Phân loại & Cơ chế Thực thi |
| :--- | :--- | :--- |
| **Quy tắc 1:** Không báo SUCCESS khi chưa chạy code thật, không trả input path làm output. | **INV-01** (Honest Execution) + **INV-02** (Source Preservation) | Cấm wildcard `_ =>`; test hợp đồng IPC `unknown_and_disabled_tools_never_return_success`. |
| **Quy tắc 2:** Cấm mock success, OCR hardcode, video gradient, PDF giả lập, fake PKCS#7. | **INV-01** (Honest Execution) + **INV-08** (Security & Cryptography) | Guardrails `FAKE001`, `FAKE002`, `FAKE003`, `PDF003`. |
| **Quy tắc 3:** Bảo vệ file gốc. Không ghi đè mặc định, không nuốt lỗi I/O. | **INV-02** (Source Preservation) + **INV-06** (Error Integrity) | Guardrails `FS001`, `IO001`, `IO002`; loại bỏ `let _ = fs::rename`. |
| **Quy tắc 4:** Thao tác PDF phải an toàn nhị phân, bảo toàn đồ thị đối tượng transitive closure. | **INV-03** (Valid Output) + **INV-04** (Binary Safety) | Guardrails `PDF001`, `PDF002`; xác thực đầu ra bằng `qpdf --check`. |
| **Quy tắc 5:** Giới hạn tài nguyên bộ nhớ/CPU/pixel (16384x16384, 1GB ZIP/zlib bomb). | **INV-05** (Resource Bounds) | Bắt buộc `take(LIMIT + 1)` trong zlib và kiểm tra `checked_mul`. |
| **Quy tắc 6:** Leptos IME an toàn, không ngắt quãng composition. | **INV-07** (Vietnamese IME Correctness) | Guardrail `IME001`; chuyển đổi sang `ImeSafeInput`. |
| **Quy tắc 7:** Bí mật không được log/lưu lịch sử, phải dùng zeroize. | **INV-08** (Security & Cryptography) | Tích hợp crate `zeroize` cho mật khẩu PDF/Excel. |
| **Quy tắc 8:** Kỳ vọng kiểm thử phải từ fixture độc lập, không lấy từ chính Engine. | **INV-09** (Independent Oracle) | Fixtures trong `fixtures/`; test bằng công cụ CLI độc lập. |
| **Quy tắc 9:** Không bỏ qua/sửa golden để pass test trái phép. | **INV-10** (Fail-Closed Verification) | Bắt buộc review qua `CODEOWNERS` cho `/policy/` và `/tests/`. |
| **Quy tắc 10:** Bằng chứng PR phải có red-green regression test. | **Tiêu chí PR Gate (docs/PR_TEMPLATE.md)** | Kiểm tra bắt buộc trong GitHub Actions PR workflow. |
| **Quy tắc 11:** Thiếu adapter, fixtures hoặc tests là RELEASE BLOCKER. | **INV-10** (Fail-Closed Verification) | `verify_source_test_inventory.py` và `verify_behavior.py`. |
| **Quy tắc 12:** Giữ nguyên Spine Lock; thay đổi phải có tu chính được ký duyệt. | **Kiến trúc Quản trị (Spine Amendment RFC)** | `./scripts/verify_spine_lock.sh` kiểm tra mã băm SHA-256. |

---

### Kiểm tra 8: Rà soát Mã chưa Commit trong Working Tree

Tại thời điểm kiểm toán, lệnh `git status` ghi nhận 5 tệp bị sửa đổi chưa commit:

1. **`crates/office-engine/src/media_studio/image_batch.rs`:**
   - Đã xóa fallback `pixels: vec![200; ...]` và `vec![255; ...]` trong `decode_png`.
   - Đã thêm giới hạn an toàn 1 GB (`MAX_DECOMPRESSED_IDAT_BYTES`) phòng chống Decompression Bomb.
   - Thêm 2 unit tests kiểm tra PNG hỏng và bit-depth không hỗ trợ.
   - *Tình trạng:* `FIXED_UNVERIFIED` trong code chính, nhưng dòng 1017 trong unit test `test_image_batch_real_jpeg_encode_decode` vẫn chứa `vec![200; 16*16*3]` làm guardrail `PNG002` vẫn kích hoạt 1 cảnh báo.
2. **`crates/office-engine/src/data_lab/base64_file_codec.rs`:**
   - Đã bổ sung hàm `safe_move_file` với cơ chế rename -> copy + remove.
   - Thay thế 2 điểm gọi `let _ = fs::rename` tại dòng 348 và 422.
   - *Tình trạng:* `FIXED_UNVERIFIED` cho code nghiệp vụ, nhưng các dòng 509, 510, 545 trong phần unit test dọn dẹp vẫn còn `let _ = fs::remove_file`.
3. **`crates/office-engine/src/data_lab/vietnamese_font_converter.rs`:**
   - Thay đổi hàm `detect_encoding` từ `String::from_utf8_lossy` sang duyệt mảng byte Latin-1 `bytes.iter().map(|&b| b as char).collect()`.
   - Bổ sung nhận diện marker VNI `0xF8` (`aø`).
   - *Tình trạng:* `FIXED_UNVERIFIED`.
4. **`src-tauri/src/safe_persistence.rs`:**
   - Loại bỏ lần acquire mutex thừa và lệnh `.unwrap()` nguy hiểm trên `sessions.get(&doc_id)`.
   - *Tình trạng:* `FIXED_UNVERIFIED`.
5. **`.github/CODEOWNERS`:**
   - Bổ sung quy tắc sở hữu mã nguồn cho `/policy/`, `/scripts/verify_*.py`, `/tests/verification/`.
   - *Tình trạng:* `FIXED_UNVERIFIED`.

---

### Kiểm tra 9: Tính Hợp lệ của Tham chiếu Finding ID trong Remediation PRs

- Toàn bộ 8 PR trong kế hoạch cũ đã được rà soát. 
- Phát hiện `FINDING-P0-MEDIA-01` tham chiếu ở PR-07 nay đã được chính thức bổ sung vào `findings_register.json`.
- Phát hiện `FINDING-P2-DATA-01` bị bỏ quên trong lộ trình cũ đã được tích hợp vào PR-08 trong Lộ trình V2.
- Không còn bất kỳ tham chiếu mồ côi nào giữa kế hoạch khắc phục và danh mục phát hiện.

---

### Kiểm tra 10: Xác minh Git Commit SHA & Môi trường Kiểm định

- **Git HEAD SHA:** `8c57f9655bb0841064973effd6ddeb72c0fa3599`
- **Tình trạng Working Tree:** Dirty (5 tệp sửa đổi, các thư mục kiểm toán untracked).
- **Trình biên dịch Rust:** `rustc 1.99.0 (b940084d7 2026-09-28)`
- **Cargo:** `cargo 1.99.0 (5f94df478 2026-08-27)`
- **Python:** `Python 3.12.3`
- **Hệ điều hành:** Linux 6.6.137+ x86_64

---

## 3. BẢNG TỔNG HỢP DANH MỤC PHÁT HIỆN SAU ĐỐI CHIẾU

| ID Phát Hiện | Mức Độ | Module / Tệp | Tiêu Đề Tóm Tắt | Trạng Thái Bằng Chứng |
| :--- | :---: | :--- | :--- | :---: |
| **FINDING-P0-IPC-01** | **P0** | `src-tauri::commands` | Nhánh wildcard trả về SUCCESS giả mạo & file gốc | `CONFIRMED_STATIC` |
| **FINDING-P0-PDF-01** | **P0** | `office-engine::doc_suite` | Phân tích PDF bằng UTF-8 lossy và cắt chuỗi thô (99 vị trí) | `CONFIRMED_STATIC` |
| **FINDING-P0-PDF-02** | **P0** | `doc_suite::pdf_merge_split` | Rơi rụng đồ thị tài nguyên gián tiếp (/Resources, Font) | `CONFIRMED_STATIC` |
| **FINDING-P0-FAKE-01** | **P0** | `doc_suite::vietnamese_ocr` | Động cơ OCR trả về chuỗi văn bản hardcode tĩnh | `CONFIRMED_STATIC` |
| **FINDING-P0-FAKE-02** | **P0** | `media_studio::video_to_gif` | Chuyển đổi video sang GIF sinh dải màu gradient | `CONFIRMED_STATIC` |
| **FINDING-P0-FAKE-03** | **P0** | `doc_suite::pdf_to_long_img` | Render PDF sang ảnh dài bằng bitmap vẽ sọc xám | `CONFIRMED_STATIC` |
| **FINDING-P0-FAKE-04** | **P0** | `doc_suite::pdf_pki_sign` | Chữ ký số PKCS#7 giả lập bằng chuỗi text thô | `CONFIRMED_STATIC` |
| **FINDING-P0-FAKE-05** | **P0** | `doc_suite::pdf_protect_sign` | Mã hóa PDF giả lập bằng XOR Cipher và muối cố định | `CONFIRMED_STATIC` |
| **FINDING-P0-MEDIA-01** | **P0** | `media_studio::image_batch` | Fallback ảnh trắng/xám 200/255 khi giải mã PNG lỗi | `FIXED_UNVERIFIED` |
| **FINDING-P0-HARNESS-01**| **P0** | `verification-kit` | Thiếu adapter nhị phân và 17 bài kiểm tra hợp đồng | `REPRODUCED_DYNAMIC` |
| **FINDING-P1-IME-01** | **P1** | `office-utility-vn-ui` | 43 trường nhập liệu Leptos nuốt dấu UniKey/EVKey | `CONFIRMED_STATIC` |
| **FINDING-P1-DB-01** | **P1** | `src-tauri::db` | Seed data SQLite dùng slug cũ gây lỗi FK nuốt lịch sử | `CONFIRMED_STATIC` |
| **FINDING-P1-SEC-01** | **P1** | `src-tauri::tauri.conf.json` | CSP bị tắt (null) và bật withGlobalTauri trong desktop | `CONFIRMED_STATIC` |
| **FINDING-P1-IO-01** | **P1** | `src-tauri::commands` | I/O đồng bộ trên luồng async Tokio & unwrap_or(0) | `CONFIRMED_STATIC` |
| **FINDING-P1-BUILD-01** | **P1** | `workspace::build` | 97 Clippy warnings và Formatting Check thất bại | `REPRODUCED_DYNAMIC` |
| **FINDING-P2-PERSIST-01**| **P2** | `src-tauri::safe_persistence`| ALLOWED_EXTENSIONS thiếu đuôi webp, gif, zip, ics, sql | `CONFIRMED_STATIC` |
| **FINDING-P2-TEST-01** | **P2** | `workspace::tests` | Mock Mirroring trong 379 unit tests nội bộ | `CONFIRMED_STATIC` |
| **FINDING-P2-DATA-01** | **P2** | `data_lab::vn_font_converter`| Nhận diện VNI/TCVN3 bằng Latin-1 byte char còn rủi ro | `FIXED_UNVERIFIED` |

**Tổng cộng sau đối chiếu:** **10 P0, 5 P1, 3 P2 (Tổng số: 18 phát hiện)**.
