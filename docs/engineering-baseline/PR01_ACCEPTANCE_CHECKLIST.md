# Bảng Kiểm Nghiệm Thu Chốt An Toàn PR-01 (PR-01 Acceptance Checklist)
## Office Utility VN — Execution Readiness & Verification Sign-Off

**Mã tài liệu:** `CHECKLIST-PR01-EXECUTION-READINESS`  
**Phiên bản:** 3.0.0 (Bản Đồng Bộ Hoàn Hảo Theo Yêu Cầu Hội Đồng Đánh Giá)  
**Tác giả:** Kỹ sư Trưởng Rust/Tauri & Trưởng nhóm Kiểm định Độc lập  
**Mục tiêu:** Cung cấp quy trình kiểm soát từng bước (Step-by-Step Gate) để bảo đảm việc triển khai Safety PR-01 diễn ra an toàn, có bằng chứng kiểm định, tuân thủ nghiêm ngặt Khung xương sống và không gây rò rỉ hồi quy.

---

## 1. GIAI ĐOẠN 1: THIẾT KẾ & KIỂM CHỨNG TOÀN VẸN TIỀN THỰC THI (PRE-EXECUTION AUDIT)

- [x] **1.1. Xác nhận Working Tree:**
  - Chạy `git status --porcelain`.
  - Khẳng định không có thay đổi ngoài ý muốn; bảo toàn các tệp uncommitted hợp lệ (`.github/CODEOWNERS`, `image_batch.rs`, `base64_file_codec.rs`, `vietnamese_font_converter.rs`, `safe_persistence.rs`).
- [x] **1.2. Xác thực Mã Băm Khung Xương Sống Ban Đầu:**
  - Chạy `./scripts/verify_spine_lock.sh`.
  - Khẳng định cả 3 tệp khớp SHA-256:
    - `crates/office-shared/src/lib.rs` -> `26a8aed8...` (PASS)
    - `src-tauri/src/db/mod.rs` -> `38bcd07c...` (PASS)
    - `src-tauri/src/commands/mod.rs` -> `ca12b1ad5586453d6753bda21cb31213333c8f13f2d621b6d00f5bdc2228b1a0` (PASS)
- [x] **1.3. Kiểm tra Tính Toàn vẹn Dữ liệu Tự Động Bằng AST/Regex:**
  - Chạy `python3 scripts/verify_audit_integrity.py`.
  - Kết quả: `100% PASS` (58 công cụ, 8 routed tools, 18 phát hiện, 18 bằng chứng, 0 release eligible).
- [x] **1.4. Chạy Bộ Test Harness:**
  - Chạy `python3 -m unittest discover -s tests/harness`.
  - Kết quả: `Ran 47 tests ... OK` (Bao gồm 14 unit/negative tests cho script kiểm toán).
- [x] **1.5. Đệ Trình Phiếu Yêu Cầu Cấp Phép Thực Thi:**
  - Hoàn tất tài liệu `docs/engineering-baseline/PR01_EXECUTION_AUTHORIZATION_REQUEST.md` (v1.0.0).
  - Hoàn tất tài liệu `docs/engineering-baseline/PR01_FINAL_TEST_MATRIX.md` (v3.0.0).
  - Hoàn tất RFC Tu chính Khung xương sống `docs/engineering-baseline/PR01_SPINE_AMENDMENT_RFC.md` (v2.0.0).
  - Hoàn tất Đặc tả An toàn `docs/engineering-baseline/PR01_SAFETY_SPEC.md` (v3.0.0).
  - Hoàn tất Thiết kế Kiểm thử `docs/engineering-baseline/PR01_TEST_DESIGN.md` (v3.0.0).
  - Chờ chữ ký phê duyệt của Maintainers để chuyển sang Giai đoạn 2.

---

## 2. GIAI ĐOẠN 2: THỰC THI BẢN VÁ AN TOÀN TRÊN NHÁNH CÔ LẬP (ISOLATED PATCH EXECUTION)

*(Chỉ được thực hiện SAU KHI nhận được phê duyệt Stage 1 Authorization từ Maintainers)*

- [ ] **2.1. Tạo Nhánh Cô Lập:**
  ```bash
  git checkout -b feature/pr01-safety-gate-hardening
  ```
- [ ] **2.2. Áp dụng Bản vá Chính Sách Sẵn Sàng (Minimal Allowlist Deny-by-Default):**
  - Khởi tạo hằng số `pub const ENABLED_TOOL_ALLOWLIST: &[&str] = &["vietnamese-number-to-words", "vn-cccd-mst-validator"];` trong `src-tauri/src/commands/mod.rs`.
  - Hiện thực hóa hàm `is_tool_enabled(slug: &str) -> bool`.
  - Hiện thực hóa điểm kết nối sản xuất chung `dispatch_tool_execution(req, cancel_flag, progress_callback)`.
  - Chuyển `dispatch_tool_task` ủy quyền sang `dispatch_tool_execution` qua `spawn_blocking`.
- [ ] **2.3. Khóa 4 Công cụ PDF Nguy hiểm Được Nối Trực Tiếp:**
  - `pdf-booklet-imposition`, `pdf-grayscale-print`, `pdf-repair-corrupt`, `pdf-table-extractor` rơi vào nhánh từ chối `unauthorized_slug => Err(CommandError::ValidationFailed)`.
- [ ] **2.4. Khóa 2 Động cơ Giả Mạo Được Nối Trực Tiếp:**
  - `vietnamese-ocr`, `pdf-to-long-image` rơi vào nhánh từ chối `unauthorized_slug => Err(CommandError::ValidationFailed)`.
- [ ] **2.5. Khóa Nhánh Wildcard và Workflow Pipeline:**
  - Xóa bỏ việc phát tiến độ giả 50%/100% trong `_ =>`.
  - Khóa hàm `execute_workflow_execution` và `execute_workflow_pipeline` trả về `Err(CommandError::ValidationFailed)`.
- [ ] **2.6. Thêm Chốt Chặn trong Động cơ Giả Mạo:**
  - Cập nhật hàm `execute` trong `vietnamese_ocr.rs`, `video_to_gif_webp.rs`, `pdf_to_long_image.rs`, `pdf_pki_sign.rs`, `pdf_protect_sign.rs` trả về `Err(CommandError::ValidationFailed)`.

---

## 3. GIAI ĐOẠN 3: KIỂM CHỨNG BỘ TEST HỒI QUY SẢN XUẤT (PRODUCTION REGRESSION SUITE)

- [ ] **3.1. Thêm Tệp Test Tích Hợp Sản Xuất:**
  - Tạo tệp `src-tauri/tests/verification_ipc.rs`.
- [ ] **3.2. Chạy Toàn Bộ 8 Bài Kiểm Thử Tích Hợp & 2 Bí Danh Hợp Đồng:**
  ```bash
  cargo test --test verification_ipc -p office_utility_vn_lib -- --nocapture
  ```
  - [ ] **TEST-01:** `test_01_unknown_tool_fails_closed_never_reports_success` -> **PASS**
  - [ ] **TEST-02:** `test_02_disabled_fake_engines_reject_execution` -> **PASS**
  - [ ] **TEST-03:** `test_03_unsafe_routed_pdf_tools_are_blocked` -> **PASS**
  - [ ] **TEST-04:** `test_04_source_preservation_sha256_unmodified` -> **PASS**
  - [ ] **TEST-05:** `test_05_honest_progress_never_emits_completion_on_failure` -> **PASS**
  - [ ] **TEST-06:** `test_06_verified_tool_compatibility_and_behavioral_contract` -> **PASS**
  - [ ] **TEST-07:** `test_07_registry_completeness_all_58_slugs` -> **PASS**
  - [ ] **TEST-08:** `test_08_execute_workflow_pipeline_fails_closed` -> **PASS**
  - [ ] **ALIAS-01:** `unknown_and_disabled_tools_never_return_success` -> **PASS**
  - [ ] **ALIAS-02:** `every_advertised_tool_has_registry_entry` -> **PASS**

---

## 4. GIAI ĐOẠN 4: THỰC THI KIỂM CHỨNG ĐỘT BIẾN (MUTATION TESTING VERIFICATION)

Thực thi kiểm chứng độ nhạy lỗi RED/GREEN trên 4 ca đột biến biên dịch được:

- [ ] **4.1. Đột Biến 1: Phục hồi Fake SUCCESS cho slug lạ (MUT-01):**
  - Sửa `dispatch_tool_execution`: `unauthorized_slug => Ok(SUCCESS...)`.
  - Chạy `cargo test --test verification_ipc test_01 -p office_utility_vn_lib`.
  - Khẳng định: **FAILED (RED)** với `assertion failed: result.is_err()`.
  - Khôi phục mã an toàn, khẳng định: **PASSED (GREEN)**.
- [ ] **4.2. Đột Biến 2: Mở khóa trái phép công cụ PDF không an toàn (MUT-02):**
  - Thêm `"pdf-booklet-imposition"` vào `ENABLED_TOOL_ALLOWLIST`.
  - Chạy `cargo test --test verification_ipc test_03 -p office_utility_vn_lib`.
  - Khẳng định: **FAILED (RED)** với `assertion failed: result.is_err()`.
  - Khôi phục mã an toàn, khẳng định: **PASSED (GREEN)**.
- [ ] **4.3. Đột Biến 3: Phục hồi Workflow Pipeline Fake SUCCESS (MUT-03):**
  - Sửa `execute_workflow_execution`: trả về `Ok(SUCCESS...)`.
  - Chạy `cargo test --test verification_ipc test_08 -p office_utility_vn_lib`.
  - Khẳng định: **FAILED (RED)** với `assertion failed: result.is_err()`.
  - Khôi phục mã an toàn, khẳng định: **PASSED (GREEN)**.
- [ ] **4.4. Đột Biến 4: Bắn sự kiện 100% hoàn tất giả mạo khi từ chối tác vụ (MUT-04):**
  - Phát sự kiện `is_completed: true` trước khi trả về `Err(...)`.
  - Chạy `cargo test --test verification_ipc test_05 -p office_utility_vn_lib`.
  - Khẳng định: **FAILED (RED)** với `VI PHẠM INV-01: Phát sự kiện is_completed=true`.
  - Khôi phục mã an toàn, khẳng định: **PASSED (GREEN)**.

---

## 5. GIAI ĐOẠN 5: QUY TRÌNH TÁI NIÊM PHONG KHÓA XƯƠNG SỐNG (SPINE RE-SEALING)

- [ ] **5.1. Xác nhận Phê duyệt Biên Bản Nghiệm Thu:**
  - Xác nhận toàn bộ 8 bài test tích hợp và 4 bài test đột biến đạt 100% GREEN.
- [ ] **5.2. Tính Toán Mã Băm SHA-256 Mới Cho Khung Xương Sống:**
  ```bash
  sha256sum src-tauri/src/commands/mod.rs
  ```
- [ ] **5.3. Cập Nhật `scripts/verify_spine_lock.sh`:**
  - Cập nhật biến `EXPECTED_CMD` với chuỗi hash mới.
- [ ] **5.4. Chạy Kiểm Thử Toàn Diện Khóa Bảo Vệ:**
  ```bash
  ./scripts/verify_spine_lock.sh
  ```
  - Khẳng định thông báo: `🎉 [SPINE GUARD] XÁC NHẬN TOÀN BỘ KHUNG XƯƠNG SỐNG AN TOÀN 100%!`.

---

## 6. GIAI ĐOẠN 6: NGHIỆM THU & BÁO CÁO CUỐI CÙNG (FINAL REVIEW & SIGN-OFF)

- [ ] Không còn bất kỳ công cụ unrouted nào trả về `status: "SUCCESS"` ảo.
- [ ] 4 công cụ PDF không an toàn và 7 công cụ giả mạo bị khóa hoàn toàn.
- [ ] Không có tệp gốc nào bị thay đổi nội dung khi tác vụ bị từ chối.
- [ ] Toàn bộ 47 tests harness và 8 tests IPC safety + 2 alias tests đều xanh.
- [ ] Lập biên bản hoàn thành PR-01 và bàn giao sang PR-02 (Verification Adapter).
