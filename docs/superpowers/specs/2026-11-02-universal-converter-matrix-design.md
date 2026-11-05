# Thiết Kế Kỹ Thuật: Ma Trận Điều Phối Chuyển Đổi Tệp Vạn Năng (Universal Converter Router Matrix)

> **Mã công cụ:** `FR-HUB-01` | **Slug:** `universal-converter`  
> **Quy chuẩn:** Kịch bản 2 `modify-pod` (Tài liệu `docs/specs/08_quy_trinh_van_hanh.md`)  
> **Mô hình kiến trúc:** Tiếp cận 1 — In-Process Delegated Router (Điều phối viên Nội bộ)

---

## 1. Mục Đích & Bối Cảnh (Intent & Why)

Nâng cấp công cụ **Trạm Chuyển đổi Tệp Vạn năng** (`UniversalConverterTool` trong `crates/office-engine/src/sys_hub/universal_converter.rs`) từ cơ chế phân nhánh tối giản hiện tại (chỉ hỗ trợ PDF split, chuyển mã font Text, và nén ảnh đơn giản) thành một **Bộ điều phối ma trận chuyển đổi đa năng** (Universal Conversion Matrix). Người dùng chỉ cần kéo thả tệp bất kỳ và chọn định dạng đích mong muốn; hệ thống tự nhận diện Magic Bytes và chuyển hướng đến module chuyên trách thuần Rust tương ứng một cách tự động, an toàn và tức thì (< 2 giây).

---

## 2. Mục Tiêu, Phi Mục Tiêu & Chống Mục Tiêu

### Mục tiêu (Goals):
- **Nhận diện tự động mở rộng:** Hỗ trợ nhận diện chính xác Magic Bytes của:
  - Tài liệu: PDF (`%PDF-`), DOCX (`PK\x03\x04` chứa `word/document.xml`).
  - Bảng tính: XLSX (`PK\x03\x04` chứa `xl/`), CSV/TSV (Văn bản bảng phân cách).
  - Hình ảnh: PNG (`\x89PNG`), JPEG (`\xFF\xD8\xFF`), BMP (`BM`), WebP (`RIFF...WEBP`).
  - Văn bản thuần / Đánh dấu: Markdown (`.md`, `.mdx`), TXT (UTF-8 hợp lệ).
- **Điều phối các cặp chuyển đổi khả thi (Conversion Matrix):**
  - **Documents:**
    - `PDF -> Markdown` (gọi `PdfToMarkdownTool`)
    - `PDF -> Image PNG` (gọi `PdfToLongImageTool`)
    - `PDF -> Text/OCR` (gọi `VietnameseOcrTool`)
    - `Markdown/Text -> PDF` (gọi `MarkdownToStyledPdfTool`)
    - `DOCX -> Markdown/Text` (giải nén ZIP `word/document.xml` bóc tách văn bản)
  - **Tables:**
    - `CSV -> JSON / SQL / Markdown` (gọi `DataStructureConverterTool`)
    - `XLSX -> CSV / JSON` (giải nén ZIP bóc tách `xl/sharedStrings.xml` và `xl/worksheets/sheet1.xml`)
  - **Images:**
    - `PNG <-> JPEG <-> BMP <-> WebP` (gọi `ImageBatchTool`)
- **Tương thích ngược 100%:** Giữ nguyên chữ ký trait `OfficeTool` và các trường `UniversalConverterOptions` cũ (`#[serde(default)]`).

### Phi mục tiêu (Non-goals):
- Không hỗ trợ chuyển đổi tệp nhị phân độc quyền cũ của Microsoft Office 97-2003 (`.doc`, `.xls`, `.ppt`) do không thể giải mã an toàn trong môi trường On-Device thuần Rust gọn nhẹ.
- Không hỗ trợ render đồ họa động phức tạp của tệp trình chiếu PowerPoint (`.pptx`). Với các tệp này, hệ thống sẽ trả về lỗi `CommandError::ValidationFailed` có thông điệp hướng dẫn thân thiện.

### Chống mục tiêu (Anti-goals):
- Tuyệt đối KHÔNG gọi tiến trình ngoài hệ điều hành (không gọi LibreOffice, Pandoc, ffmpeg CLI).
- Tuyệt đối KHÔNG sử dụng network crate (`std::net`, `tokio::net`, `reqwest`).
- Tuyệt đối KHÔNG dùng `.unwrap()` hoặc `.expect()` trong luồng logic xử lý nghiệp vụ.

---

## 3. Ràng Buộc Kỹ Thuật (Constraints)

- **`NFR-SEC-01`**: 100% On-Device, Zero Network Egress.
- **`NFR-SEC-03`**: Zero Unwrap / Expect trong mã nghiệp vụ.
- **`NFR-PERF-01`**: Hỗ trợ cờ hủy tác vụ `cancel_flag: &AtomicBool` phản hồi dừng dưới 100ms.
- **Spine Read-Only Lock**: Không chỉnh sửa các tệp được bảo vệ trong `crates/office-shared` và `src-tauri/src/db`.
- **4 Cổng kiểm định**: Vượt qua `./scripts/verify_pod.sh universal-converter`.

---

## 4. Các Hướng Tiếp Cận Đã Đánh Giá (Approaches)

1. **Tiếp cận 1 (Đã chọn): In-Process Delegated Router**
   - Tái sử dụng các Pod đã được hiện thực và kiểm định trong `crates/office-engine` (`PdfToMarkdownTool`, `MarkdownToStyledPdfTool`, `DataStructureConverterTool`, `ImageBatchTool`, `PdfToLongImageTool`, `VietnameseOcrTool`).
   - Tối ưu hóa phân tích ZIP trong bộ nhớ bằng `flate2` cho DOCX và XLSX để trích xuất văn bản/dữ liệu dạng text mà không cần phụ thuộc thư viện bên ngoài cồng kềnh.
2. **Tiếp cận 2: Monolithic In-Place** (Bị loại bỏ vì trùng lặp mã nguồn và khó bảo trì).
3. **Tiếp cận 3: External CLI Pipeline** (Bị loại bỏ vì vi phạm Zero Dependency / Pure Rust).

---

## 5. Chi Tiết Kiến Trúc Bộ Điều Phối (Implementation Details)

```
[Tệp Đầu Vào]
      │
      ▼
inspect_file_category(path) ── Magic Bytes & Header Check
      │
      ├─► PDF ─────────┬─► to .md  ──► PdfToMarkdownTool
      │                ├─► to .png ──► PdfToLongImageTool
      │                └─► to .txt ──► VietnameseOcrTool
      │
      ├─► DOCX (ZIP) ──┴─► to .md / .txt ──► DocxTextExtractor (word/document.xml)
      │
      ├─► MD / TXT ────┬─► to .pdf ──► MarkdownToStyledPdfTool
      │                └─► to .txt / .md ──► VietnameseFontConverterTool
      │
      ├─► CSV ─────────┬─► to .json / .sql / .md ──► DataStructureConverterTool
      │
      ├─► XLSX (ZIP) ──┴─► to .csv / .json ──► XlsxTableExtractor (xl/sharedStrings.xml)
      │
      ├─► IMAGE ───────┬─► to .png / .jpg / .bmp / .webp ──► ImageBatchTool
      │
      └─► KHÁC (doc, xls, ppt, pptx, binary lạ)
                       └─► CommandError::ValidationFailed (Thông báo rõ ràng, lịch sự)
```

---

## 6. Kế Hoạch Kiểm Thử (Test Plan)

1. `test_universal_converter_pdf_to_markdown`: Kiểm thử chuyển đổi PDF sang Markdown.
2. `test_universal_converter_markdown_to_pdf`: Kiểm thử chuyển đổi Markdown sang PDF.
3. `test_universal_converter_csv_to_json_and_sql`: Kiểm thử chuyển đổi CSV sang JSON và SQL.
4. `test_universal_converter_docx_to_text`: Kiểm thử đọc DOCX mẫu và xuất Text/Markdown.
5. `test_universal_converter_xlsx_to_csv`: Kiểm thử đọc XLSX mẫu và xuất CSV.
6. `test_universal_converter_image_cross_conversion`: Kiểm thử chuyển đổi ảnh PNG sang BMP.
7. `test_universal_converter_unsupported_formats_graceful_error`: Kiểm tra xử lý từ chối định dạng không hỗ trợ (OLE/Binary lạ) không gây panic hay crash.
8. `test_universal_converter_cancellation`: Kiểm thử hủy tác vụ qua `cancel_flag`.
