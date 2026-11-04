# Kế hoạch Mở rộng Toàn diện PDF Studio (PDF Studio Advanced Expansion)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hiện thực hóa toàn bộ các tính năng nâng cao còn thiếu của PDF Studio (Tách file theo dung lượng, Searchable PDF OCR, Quản lý & Auto-fill AcroForm, PDF Redaction triệt để, Chuẩn hóa PDF/A & Linearization, và Ký số PDF PKI) theo tiêu chuẩn 100% On-Device, thuần Rust, bảo mật cao.

**Architecture:** Mở rộng phân hệ Document Suite (`crates/office-engine/src/doc_suite/`) với các pod công cụ mới và nâng cấp các pod hiện có. Tích hợp chặt chẽ vào UI Micro-App `PdfStudio` (`office-ui` và `src/micro_apps/`), đảm bảo tương thích ngược cấu trúc JSON `#[serde(default)]`, cấm tuyệt đối network crate (`NFR-SEC-01`) và cấm `unwrap()`/`expect()` trong mã nghiệp vụ (`NFR-SEC-03`).

**Tech Stack:** Rust (Edition 2024), `lopdf`, `pdf-writer`, `image`, `sha2`, `flate2`, Leptos 0.8, Tauri v2.

---

## Global Constraints
- `NFR-SEC-01`: Cấm tuyệt đối kết nối mạng trong `crates/office-engine` (`std::net`, `tokio::net`, `reqwest`, `hyper`).
- `NFR-SEC-03`: Cấm `.unwrap()` và `.expect()` trong mã nghiệp vụ của `crates/office-engine/src/` (ngoại trừ khối kiểm thử `#[cfg(test)]`).
- `NFR-REL-01`: Mọi trường struct mới thêm vào Options bắt buộc phải có `#[serde(default)]` để bảo toàn tương thích ngược cho `tool_presets` và workflow JSON cũ.
- `COMPILER-GATE`: Mỗi pod công cụ hoàn thành bắt buộc phải vượt qua `./scripts/verify_pod.sh <slug>` với kết quả exit code 0.

---

## Danh sách Task Thực thi

### Task 1: Nâng cấp `pdf-merge-split` (Tách theo dung lượng & Chèn trang trắng)
**Files:**
- Modify: `crates/office-engine/src/doc_suite/pdf_merge_split.rs`
- Modify: `crates/office-ui/src/tools/pdf_merge_split_view.rs`
- Modify: `src/tools/pdf_merge_split_view.rs`
- Test: `crates/office-engine/src/doc_suite/pdf_merge_split.rs` (bổ sung tests)

**Interfaces:**
- Consumes: `PdfMergeSplitOptions`, `MergeSplitMode`
- Produces: `max_size_mb: Option<f64>`, `insert_blank_pages: Option<Vec<usize>>`

- [ ] **Step 1: Viết failing tests**
  - Viết `test_backward_compatible_options_deserialization`
  - Viết `test_split_by_max_size_mb`
  - Viết `test_insert_blank_pages`
- [ ] **Step 2: Chạy cargo test để xác nhận test fail**
- [ ] **Step 3: Triển khai logic tách theo dung lượng và chèn trang trắng an toàn**
- [ ] **Step 4: Chạy kiểm định `./scripts/verify_pod.sh pdf-merge-split` xác nhận pass 100%**
- [ ] **Step 5: Git commit**

---

### Task 2: Nâng cấp `vietnamese-ocr` (Xuất định dạng Searchable PDF)
**Files:**
- Modify: `crates/office-engine/src/doc_suite/vietnamese_ocr.rs`
- Modify: `crates/office-ui/src/tools/vietnamese_ocr_view.rs`
- Modify: `src/tools/vietnamese_ocr_view.rs`
- Test: `crates/office-engine/src/doc_suite/vietnamese_ocr.rs`

**Interfaces:**
- Consumes: `OcrOutputFormat`, `VietnameseOcrOptions`
- Produces: `OcrOutputFormat::SearchablePdf`

- [ ] **Step 1: Viết failing tests**
  - Viết `test_ocr_output_format_searchable_pdf`
  - Viết `test_searchable_pdf_contains_text_render_mode_3`
- [ ] **Step 2: Chạy cargo test xác nhận test fail**
- [ ] **Step 3: Triển khai engine tạo Searchable PDF với invisible text layer**
- [ ] **Step 4: Chạy kiểm định `./scripts/verify_pod.sh vietnamese-ocr`**
- [ ] **Step 5: Git commit**

---

### Task 3: Tạo mới công cụ `pdf-form-manager` (Quản lý Biểu mẫu & Auto-fill AcroForm)
**Files:**
- Create: `crates/office-engine/src/doc_suite/pdf_form_manager.rs`
- Modify: `crates/office-engine/src/doc_suite/mod.rs`
- Modify: `crates/office-engine/src/lib.rs`
- Create: `crates/office-ui/src/tools/pdf_form_manager_view.rs`
- Create: `src/tools/pdf_form_manager_view.rs`
- Modify: `src/micro_apps/registry.rs`
- Modify: `src/micro_apps/pdf_studio/mod.rs`
- Modify: `src/micro_apps/pdf_studio/state.rs`

**Interfaces:**
- Produces: `PdfFormManagerTool`, `PdfFormManagerOptions`, `FormAction` (`Inspect`, `ExportData`, `AutoFill`)

- [ ] **Step 1: Viết TDD failing tests**
  - Viết `test_pdf_form_manager_inspect_fields`
  - Viết `test_pdf_form_manager_auto_fill`
  - Viết `test_pdf_form_manager_invalid_signature`
- [ ] **Step 2: Chạy cargo test xác nhận test fail**
- [ ] **Step 3: Triển khai `PdfFormManagerTool` đọc mảng `/Fields`, cập nhật `/V` và export JSON/CSV**
- [ ] **Step 4: Đăng ký tool vào `doc_suite`, UI view và Registry**
- [ ] **Step 5: Chạy kiểm định `./scripts/verify_pod.sh pdf-form-manager`**
- [ ] **Step 6: Git commit**

---

### Task 4: Tạo mới công cụ `pdf-redaction` (Xóa bỏ triệt để dữ liệu nhạy cảm)
**Files:**
- Create: `crates/office-engine/src/doc_suite/pdf_redaction.rs`
- Modify: `crates/office-engine/src/doc_suite/mod.rs`
- Modify: `crates/office-engine/src/lib.rs`
- Create: `crates/office-ui/src/tools/pdf_redaction_view.rs`
- Create: `src/tools/pdf_redaction_view.rs`
- Modify: `src/micro_apps/registry.rs`
- Modify: `src/micro_apps/pdf_studio/state.rs`

**Interfaces:**
- Produces: `PdfRedactionTool`, `PdfRedactionOptions`, `RedactTarget`

- [ ] **Step 1: Viết TDD failing tests**
  - Viết `test_pdf_redaction_removes_sensitive_stream_bytes`
  - Viết `test_pdf_redaction_draws_blackout_box`
- [ ] **Step 2: Chạy cargo test xác nhận test fail**
- [ ] **Step 3: Triển khai `PdfRedactionTool` bóc tách content stream, xóa lệnh text và chèn hộp che màu đen tĩnh**
- [ ] **Step 4: Đăng ký tool vào Registry và UI**
- [ ] **Step 5: Chạy kiểm định `./scripts/verify_pod.sh pdf-redaction`**
- [ ] **Step 6: Git commit**

---

### Task 5: Tạo mới công cụ `pdf-archive-linearize` (Chuẩn hóa PDF/A & Linearization Fast Web View)
**Files:**
- Create: `crates/office-engine/src/doc_suite/pdf_archive_linearize.rs`
- Modify: `crates/office-engine/src/doc_suite/mod.rs`
- Modify: `crates/office-engine/src/lib.rs`
- Create: `crates/office-ui/src/tools/pdf_archive_linearize_view.rs`
- Create: `src/tools/pdf_archive_linearize_view.rs`
- Modify: `src/micro_apps/registry.rs`
- Modify: `src/micro_apps/pdf_studio/state.rs`

**Interfaces:**
- Produces: `PdfArchiveLinearizeTool`, `PdfArchiveLinearizeOptions`, `ArchiveMode`

- [ ] **Step 1: Viết TDD failing tests**
  - Viết `test_pdf_archive_injects_pdfa_xmp`
  - Viết `test_pdf_linearization_dictionary_created`
- [ ] **Step 2: Chạy cargo test xác nhận test fail**
- [ ] **Step 3: Triển khai chuyển đổi PDF/A-1b metadata và cấu trúc Fast Web View Linearization**
- [ ] **Step 4: Đăng ký tool vào Registry và UI**
- [ ] **Step 5: Chạy kiểm định `./scripts/verify_pod.sh pdf-archive-linearize`**
- [ ] **Step 6: Git commit**

---

### Task 6: Tạo mới công cụ `pdf-pki-sign` (Ký số điện tử & Xác thực toàn vẹn PKI)
**Files:**
- Create: `crates/office-engine/src/doc_suite/pdf_pki_sign.rs`
- Modify: `crates/office-engine/src/doc_suite/mod.rs`
- Modify: `crates/office-engine/src/lib.rs`
- Create: `crates/office-ui/src/tools/pdf_pki_sign_view.rs`
- Create: `src/tools/pdf_pki_sign_view.rs`
- Modify: `src/micro_apps/registry.rs`
- Modify: `src/micro_apps/pdf_studio/state.rs`

**Interfaces:**
- Produces: `PdfPkiSignTool`, `PdfPkiSignOptions`, `SignAction` (`Sign`, `Verify`)

- [ ] **Step 1: Viết TDD failing tests**
  - Viết `test_pdf_pki_sign_and_verify`
  - Viết `test_pdf_pki_tamper_detection`
- [ ] **Step 2: Chạy cargo test xác nhận test fail**
- [ ] **Step 3: Triển khai cấu trúc `/ByteRange`, `/Contents` và xác thực SHA-256 PKCS#7**
- [ ] **Step 4: Đăng ký tool vào Registry và UI**
- [ ] **Step 5: Chạy kiểm định `./scripts/verify_pod.sh pdf-pki-sign`**
- [ ] **Step 6: Git commit**
