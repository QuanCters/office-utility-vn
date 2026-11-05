# Universal Converter Matrix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hiện thực hóa Ma trận Điều phối Chuyển đổi Tệp Vạn năng (Universal Conversion Matrix) trong `crates/office-engine/src/sys_hub/universal_converter.rs`, tự động nhận dạng Magic Bytes và điều phối chéo giữa các định dạng Documents (PDF, DOCX, MD, TXT), Tables (XLSX, CSV) và Images (PNG, JPEG, BMP, WebP).

**Architecture:** Sử dụng kiến trúc In-Process Delegated Router (Tiếp cận 1). `UniversalConverterTool` đóng vai trò Hub điều phối: đọc Magic Bytes phân loại tệp nguồn, đối chiếu với `target_extension` để định tuyến tới các engine pod nội bộ (`PdfToMarkdownTool`, `MarkdownToStyledPdfTool`, `DataStructureConverterTool`, `ImageBatchTool`, `PdfToLongImageTool`, `VietnameseOcrTool`, kèm bộ bóc tách XML OpenXML cho DOCX và XLSX).

**Tech Stack:** Rust 2024, `office-engine`, `office-shared`, `infer`, `flate2`, `quick-xml`/`regex`, `serde_json`, Rayon, Tokio.

**Spec:** `docs/superpowers/specs/2026-11-02-universal-converter-matrix-design.md`

## Global Constraints

- 100% On-Device, Zero Network Egress (`NFR-SEC-01`): Cấm mọi thư viện mạng (`std::net`, `tokio::net`, `reqwest`).
- Khóa bảo vệ Khung xương sống Spine Lock (`AGENTS.md`): Tuyệt đối không chỉnh sửa `crates/office-shared` và `src-tauri/src/db`.
- Không sử dụng `.unwrap()` hoặc `.expect()` trong mã logic nghiệp vụ (`NFR-SEC-03`).
- Hỗ trợ cờ hủy tác vụ `cancel_flag: &AtomicBool` phản hồi dừng dưới 100ms (`NFR-REL-01`).
- Mọi trường mở rộng trong `UniversalConverterOptions` phải có thuộc tính `#[serde(default)]` để bảo toàn tương thích ngược (`SOP modify-pod`).
- Kiểm định bắt buộc: Vượt qua `./scripts/verify_pod.sh universal-converter` và `./scripts/verify_spine_lock.sh`.

## Review Focus

1. **ZIP Bomb / File nén độc hại:** Tệp ZIP DOCX/XLSX nén giả mạo tỷ lệ > 1:100 hoặc kích thước giải nén vượt quá 250MB phải bị chặn an toàn, trả về lỗi validation thay vì tràn RAM.
2. **Định dạng nhị phân độc quyền cũ (.doc, .xls, .ppt) & .pptx:** Xử lý từ chối bằng `CommandError::ValidationFailed` có thông báo tiếng Việt lịch sự, không gây hoảng loạn (panic) hoặc treo luồng.
3. **Tệp rỗng hoặc hỏng magic bytes:** Phát hiện sớm và trả về `CommandError::InvalidFileSignature`.
4. **Hủy tác vụ cộng tác (Cooperative Cancellation):** Kiểm tra `cancel_flag` tại mỗi chu kỳ xử lý tệp và bên trong các bước giải nén/chuyển đổi để thoát ngay lập tức khi người dùng bấm Hủy.
5. **Giữ nguyên định dạng đầu ra UTF-8 chuẩn:** Các văn bản xuất ra (Markdown, CSV, JSON) phải được mã hóa chuẩn UTF-8, giữ nguyên toàn vẹn dấu thanh tiếng Việt.

---

### Task 1: Mở rộng Phân loại Tệp (`inspect_file_category`) và Nhận diện Magic Bytes

**Files:**
- Modify: `crates/office-engine/src/sys_hub/universal_converter.rs:38-100`
- Test: `crates/office-engine/src/sys_hub/universal_converter.rs:303-421`

**Interfaces:**
- Produces: `enum FileCategory`:
  - `DocumentPdf`
  - `DocumentDocx`
  - `TableXlsx`
  - `TableCsv`
  - `DocumentMarkdown`
  - `ImageMedia(String)`
  - `TextData`
  - `Unsupported(String)`
- Produces function: `pub fn inspect_file_category(path: &Path) -> Result<FileCategory, CommandError>`

- [ ] **Step 1: Viết test failing cho nhận diện các loại tệp mở rộng**

Thêm test `test_inspect_file_category_extended` kiểm tra nhận diện PDF, PNG, BMP, JPEG, DOCX (ZIP chứa `word/`), XLSX (ZIP chứa `xl/`), CSV và MD.

- [ ] **Step 2: Chạy test để xác nhận test thất bại**

Run: `cargo test -p office-engine test_inspect_file_category_extended`
Expected: FAIL do chưa có các biến thể enum mới `DocumentDocx`, `TableXlsx`, `TableCsv`, `DocumentMarkdown`.

- [ ] **Step 3: Cập nhật `FileCategory` và hoàn thiện logic `inspect_file_category`**

Hiện thực kiểm tra:
1. `b"%PDF-"` -> `DocumentPdf`.
2. `b"PK\x03\x04"` -> Đọc danh sách entry trong ZIP header:
   - Nếu có `word/` -> `DocumentDocx`
   - Nếu có `xl/` -> `TableXlsx`
3. Ảnh qua `infer::get` hoặc magic byte: PNG, JPEG (`\xFF\xD8\xFF`), BMP (`BM`), WebP (`RIFF...WEBP`) -> `ImageMedia`.
4. Text hợp lệ UTF-8:
   - Phân biệt CSV (chứa dấu phẩy/chấm phẩy/tab và có dòng tiêu đề), Markdown (bắt đầu bằng `# ` hoặc chứa `## `, `| --- |`), hoặc Text thuần (`TextData`).

- [ ] **Step 4: Chạy test để xác nhận test pass**

Run: `cargo test -p office-engine test_inspect_file_category_extended`
Expected: PASS.

- [ ] **Step 5: Chạy linter kiểm tra**

Run: `cargo check -p office-engine`
Expected: PASS.

---

### Task 2: Hiện thực Bộ trích xuất OpenXML Bảng tính và Tài liệu (`Docx` & `Xlsx`)

**Files:**
- Modify: `crates/office-engine/src/sys_hub/universal_converter.rs`
- Test: `crates/office-engine/src/sys_hub/universal_converter.rs`

**Interfaces:**
- Produces:
  - `pub fn extract_text_from_docx_bytes(bytes: &[u8], cancel_flag: &AtomicBool) -> Result<String, CommandError>`
  - `pub fn extract_csv_from_xlsx_bytes(bytes: &[u8], cancel_flag: &AtomicBool) -> Result<String, CommandError>`

- [ ] **Step 1: Viết test failing cho bộ bóc tách OpenXML DOCX và XLSX**

Thêm test `test_docx_and_xlsx_extractors` tạo file ZIP giả lập chứa `word/document.xml` và `xl/worksheets/sheet1.xml` + `xl/sharedStrings.xml`, gọi 2 hàm trích xuất và so sánh kết quả nội dung văn bản / CSV.

- [ ] **Step 2: Chạy test để xác nhận test thất bại**

Run: `cargo test -p office-engine test_docx_and_xlsx_extractors`
Expected: FAIL vì 2 hàm chưa được định nghĩa.

- [ ] **Step 3: Hiện thực `extract_text_from_docx_bytes` và `extract_csv_from_xlsx_bytes`**

Sử dụng bộ giải nén ZIP `flate2::read::DeflateDecoder` (đã có sẵn mẫu trong `excel_sheet_unlocker.rs` và `docx_template_mailmerge.rs`):
- `extract_text_from_docx_bytes`: Tìm entry `word/document.xml`, giải nén với giới hạn kích thước tối đa 50MB (chống Zip Bomb), bóc tách nội dung giữa các thẻ `<w:t>...</w:t>` và các đoạn `<w:p>` tạo thành các dòng văn bản Markdown.
- `extract_csv_from_xlsx_bytes`: Tìm `xl/sharedStrings.xml` để xây dựng bảng chuỗi dùng chung, tìm `xl/worksheets/sheet1.xml` để trích xuất các ô `<c r="..." t="s"><v>...</v></c>`, định dạng thành chuỗi CSV chuẩn phân cách bằng dấu phẩy.

- [ ] **Step 4: Chạy test để xác nhận test pass**

Run: `cargo test -p office-engine test_docx_and_xlsx_extractors`
Expected: PASS.

---

### Task 3: Xây dựng Ma trận Điều phối Chuyển đổi Chéo (Conversion Router Matrix)

**Files:**
- Modify: `crates/office-engine/src/sys_hub/universal_converter.rs`
- Test: `crates/office-engine/src/sys_hub/universal_converter.rs`

**Interfaces:**
- Consumes:
  - `crate::doc_suite::PdfToMarkdownTool`
  - `crate::doc_suite::MarkdownToStyledPdfTool`
  - `crate::doc_suite::PdfToLongImageTool`
  - `crate::doc_suite::VietnameseOcrTool`
  - `crate::doc_suite::PdfMergeSplitTool`
  - `crate::data_lab::DataStructureConverterTool`
  - `crate::data_lab::VietnameseFontConverterTool`
  - `crate::media_studio::ImageBatchTool`
- Produces: Logic xử lý đầy đủ trong `UniversalConverterTool::execute`.

- [ ] **Step 1: Viết test failing cho các cặp chuyển đổi chéo chính**

Thêm các test:
- `test_matrix_pdf_to_markdown`: Chuyển đổi PDF sang Markdown.
- `test_matrix_markdown_to_pdf`: Chuyển đổi Markdown sang PDF.
- `test_matrix_csv_to_json`: Chuyển đổi CSV sang JSON.
- `test_matrix_image_cross_conversion`: Chuyển đổi ảnh giữa PNG và BMP.
- `test_matrix_unsupported_format_graceful_error`: Nộp file lạ, kỳ vọng nhận lỗi `CommandError::ValidationFailed` có thông báo hướng dẫn rõ ràng.

- [ ] **Step 2: Chạy test để xác nhận test thất bại**

Run: `cargo test -p office-engine test_matrix_`
Expected: FAIL do các luồng điều phối mới chưa được kết nối.

- [ ] **Step 3: Hiện thực ma trận điều phối trong `UniversalConverterTool::execute`**

Khai báo các instance tool cần thiết trong `UniversalConverterTool`:
- Dựa trên `(category, target_ext)`:
  - `(DocumentPdf, "md" | "markdown")` -> gọi `PdfToMarkdownTool`.
  - `(DocumentPdf, "png" | "jpg" | "jpeg")` -> gọi `PdfToLongImageTool`.
  - `(DocumentPdf, "txt")` -> gọi `VietnameseOcrTool` / bóc tách text.
  - `(DocumentMarkdown | TextData, "pdf")` -> gọi `MarkdownToStyledPdfTool`.
  - `(DocumentDocx, "md" | "txt")` -> gọi `extract_text_from_docx_bytes`.
  - `(TableCsv | TextData, "json" | "sql")` -> gọi `DataStructureConverterTool`.
  - `(TableXlsx, "csv" | "json")` -> gọi `extract_csv_from_xlsx_bytes` (và `DataStructureConverterTool` nếu đích là JSON).
  - `(ImageMedia(_), target_ext)` -> gọi `ImageBatchTool`.
  - Nếu tệp nguồn là `doc`, `xls`, `ppt`, `pptx` hoặc binary lạ không thuộc ma trận: trả về `CommandError::ValidationFailed` với giải thích định dạng chưa được hỗ trợ trên môi trường thuần Rust on-device.

- [ ] **Step 4: Chạy toàn bộ test trong `universal_converter.rs`**

Run: `cargo test -p office-engine universal_converter`
Expected: PASS (tất cả các test mới và test cũ đều vượt qua 100%).

---

### Task 4: Kiểm tra Tuân thủ Quy chuẩn SOP `modify-pod` & Cổng Chất lượng

**Files:**
- Verify: `crates/office-engine/src/sys_hub/universal_converter.rs`
- Verify scripts: `scripts/verify_pod.sh`, `scripts/verify_spine_lock.sh`

- [ ] **Step 1: Chạy kiểm định Pod qua script chuẩn**

Run: `./scripts/verify_pod.sh universal-converter`
Expected:
- Cổng 1 (Cargo Check): PASS.
- Cổng 2 (Zero Network & No Unwrap/Expect): PASS (không phát hiện `std::net`, `reqwest`, `unwrap`, `expect`).
- Cổng 3 (Unit Tests): PASS 100%.

- [ ] **Step 2: Chạy kiểm định Khóa Khung xương sống (Spine Guard)**

Run: `./scripts/verify_spine_lock.sh`
Expected: SHA-256 của `crates/office-shared`, `src-tauri/src/db`, `src-tauri/src/commands` khớp chuẩn 100%.

- [ ] **Step 3: Kiểm tra biên dịch workspace**

Run: `cargo check --workspace`
Expected: PASS không có lỗi biên dịch.
