# Kế Hoạch Triển Khai Mở Rộng PDF Studio Phase 1 (PDF Studio Phase 1 Expansion Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bổ sung 3 công cụ chuyên biệt mới cho PDF Studio gồm Sắp xếp trang trực quan Smart Hybrid (`pdf-page-organizer`), Ký tay điện tử Canvas (`pdf-signature-pad`), và Xuất gói ảnh ZIP rời (`pdf-to-images-pack`) theo chuẩn 100% On-Device, thuần Rust, bảo mật tuyệt đối.

**Architecture:** Mở rộng phân hệ Document Suite (`crates/office-engine/src/doc_suite/`) với 3 pod công cụ mới, xây dựng view Leptos 0.8 độc lập (`src/tools/` và `crates/office-ui/src/tools/`), đăng ký vào `ToolRegistry` thuộc `MicroAppId::PdfStudio` (nâng tổng số công cụ PDF Studio lên 25), tuân thủ nghiêm ngặt mô hình Smart Hybrid và các ràng buộc bảo mật.

**Tech Stack:** Rust (Edition 2024), Leptos 0.8, Tauri v2, `image`, `flate2`, `crc32fast`, `serde`, `serde_json`.

**Spec:** `docs/superpowers/specs/2026-11-05-pdf-studio-phase1-expansion-design.md`

## Global Constraints

- `NFR-SEC-01`: Cấm tuyệt đối kết nối mạng trong `crates/office-engine` (`std::net`, `tokio::net`, `reqwest`, `hyper`).
- `NFR-SEC-03`: Cấm `.unwrap()` và `.expect()` trong mã nghiệp vụ của `crates/office-engine/src/` (ngoại trừ khối kiểm thử `#[cfg(test)]`).
- `NFR-REL-01`: Mọi trường struct mới thêm vào Options bắt buộc phải có `#[serde(default)]` để bảo toàn tương thích ngược cho `tool_presets` và workflow JSON cũ.
- `COMPILER-GATE`: Mỗi pod công cụ hoàn thành bắt buộc phải vượt qua `./scripts/verify_pod.sh <slug>` với kết quả exit code 0.

## Review Focus

- File PDF có cây `/Pages` lồng nhau nhiều cấp (Nested Pages tree): Trích xuất chính xác danh sách lá `/Kids` mà không làm mất trang.
- File PDF có `MediaBox` dạng tọa độ âm hoặc không bắt đầu từ (0, 0): Tính toán chính xác tọa độ chữ ký dán vào không bị tràn ra ngoài trang in.
- File PDF được yêu cầu ký có trang bị xoay `/Rotate 90` hoặc `270`: Điều chỉnh ma trận biến đổi cm của chữ ký không bị xoay ngược hướng.
- File PDF chứa stream nén hỏng hoặc bị cắt cụt giữa chừng khi xuất ảnh ZIP: Báo lỗi `CommandError` an toàn mà không crash ứng dụng.
- Tác vụ xử lý tài liệu lớn khi người dùng bấm Hủy (Cancel): Dừng ngay lập tức sau $<100\text{ ms}$ thông qua `is_cancelled.load(Ordering::Relaxed)`.

---

## Danh sách Task Thực thi

### Task 1: Xây dựng Pod Công cụ `pdf-page-organizer` (Sắp xếp & Đảo trang Smart Hybrid)

**Files:**
- Create: `crates/office-engine/src/doc_suite/pdf_page_organizer.rs`
- Modify: `crates/office-engine/src/doc_suite/mod.rs`
- Modify: `crates/office-engine/src/lib.rs`
- Create: `crates/office-ui/src/tools/pdf_page_organizer_view.rs`
- Create: `src/tools/pdf_page_organizer_view.rs`
- Test: `crates/office-engine/src/doc_suite/pdf_page_organizer.rs` (bổ sung 5 unit tests chuẩn)

**Interfaces:**
- Produces: `PdfPageOrganizerTool`, `PdfPageOrganizerOptions`, `PageAction { source_page: usize, rotation_deg: i32 }`, `PageInfo { page_index: usize, width_pt: f64, height_pt: f64, rotation_deg: i32, is_landscape: bool }`, hàm `get_pdf_page_info(path: &Path) -> Result<Vec<PageInfo>, CommandError>`

- [x] **Step 1: Viết failing unit tests trong `pdf_page_organizer.rs`**
  - Viết `test_pdf_page_organizer_happy_path`: Reorder trang `[3, 1, 2]` và xoay trang 1 thêm `90°`.
  - Viết `test_pdf_page_organizer_invalid_magic_bytes`: Kiểm tra từ chối file không phải `%PDF-`.
  - Viết `test_pdf_page_organizer_cooperative_cancellation`: Kiểm tra cờ hủy dừng tiến trình.
  - Viết `test_pdf_page_organizer_security_guardrails`: Kiểm tra trang nguồn vượt quá tổng số trang.
  - Viết `test_pdf_page_organizer_edge_case`: Kiểm tra PDF 1 trang hoặc giữ nguyên thứ tự.
- [x] **Step 2: Chạy cargo test xác nhận test fail**
  - Run: `cargo test -p office-engine pdf_page_organizer`
  - Expected: FAIL do chưa có struct và tool implementation
- [x] **Step 3: Triển khai động cơ `PdfPageOrganizerTool`**
  - Đọc cây `/Pages`, giải quyết `/Kids` đệ quy để thu thập danh sách Object ID của từng trang.
  - Tái tạo cây `/Pages` mới chỉ chứa các trang có trong `options.pages` theo đúng thứ tự.
  - Cập nhật từ điển `/Rotate` trên từng trang (cộng dồn mod 360).
  - Cập nhật trường `/Count` và xây dựng lại bảng Xref.
- [x] **Step 4: Triển khai giao diện `PdfPageOrganizerView` (Leptos 0.8)**
  - Hiển thị danh sách thẻ trang Smart Hybrid (số trang, biểu tượng `📄` dọc / `📑` ngang, badge góc xoay).
  - Cung cấp nút xoay trái 90° (`↺`), xoay phải 90° (`↻`), di chuyển trước/sau (`←`, `→`), xóa trang (`🗑️`).
  - Hỗ trợ nút thao tác nhanh: "Đảo ngược thứ tự", "Xoay tất cả 90°", "Khôi phục gốc".
  - Nút bật/tắt "👁️ Chế độ Preview" để hiển thị xem trước theo yêu cầu.
- [x] **Step 5: Chạy kiểm định `./scripts/verify_pod.sh pdf-page-organizer`**
  - Run: `./scripts/verify_pod.sh pdf-page-organizer`
  - Expected: Exit code 0, 100% tests pass, 0 network, 0 unwrap/expect.
- [x] **Step 6: Commit**
  - Commit message: `feat(pdf-page-organizer): implement smart hybrid visual page organizer tool`

---

### Task 2: Xây dựng Pod Công cụ `pdf-signature-pad` (Ký tay Điện tử & Dán Chữ ký PDF)

**Files:**
- Create: `crates/office-engine/src/doc_suite/pdf_signature_pad.rs`
- Modify: `crates/office-engine/src/doc_suite/mod.rs`
- Modify: `crates/office-engine/src/lib.rs`
- Create: `crates/office-ui/src/tools/pdf_signature_pad_view.rs`
- Create: `src/tools/pdf_signature_pad_view.rs`
- Test: `crates/office-engine/src/doc_suite/pdf_signature_pad.rs` (bổ sung 5 unit tests)

**Interfaces:**
- Produces: `PdfSignaturePadTool`, `PdfSignaturePadOptions`, `SignaturePlacement` (`BottomRight`, `BottomLeft`, `Center`, `TopRight`, `Custom { x_pt: f64, y_pt: f64 }`)

- [x] **Step 1: Viết failing unit tests trong `pdf_signature_pad.rs`**
  - Viết `test_pdf_signature_pad_happy_path`: Chèn ảnh chữ ký PNG base64 vào trang cuối vị trí `BottomRight`.
  - Viết `test_pdf_signature_pad_invalid_magic_bytes`: Kiểm tra từ chối file đầu vào không hợp lệ.
  - Viết `test_pdf_signature_pad_cooperative_cancellation`: Dừng khi cờ hủy kích hoạt.
  - Viết `test_pdf_signature_pad_security_guardrails`: Kiểm tra chữ ký base64 rỗng hoặc sai cú pháp PNG.
  - Viết `test_pdf_signature_pad_edge_case`: Chèn vào tất cả các trang (`target_page = 0`) và kiểm tra tính toán MediaBox khổ Landscape.
- [x] **Step 2: Chạy cargo test xác nhận test fail**
  - Run: `cargo test -p office-engine pdf_signature_pad`
  - Expected: FAIL do chưa có module và tool
- [x] **Step 3: Triển khai động cơ `PdfSignaturePadTool`**
  - Giải mã base64 thành byte ảnh PNG, xác minh magic bytes PNG `[0x89, 0x50, 0x4E, 0x47]`.
  - Đọc `MediaBox` trang mục tiêu trong PDF gốc, tính toán tọa độ (X, Y) chính xác theo preset `SignaturePlacement` (margin 36 pt).
  - Tạo XObject Image với luồng RGB và SMask trong suốt, nhúng vào `/Resources /XObject`.
  - Bổ sung lệnh kết xuất `q <w> 0 0 <h> <x> <y> cm /SigImage Do Q` vào luồng `/Contents`.
- [x] **Step 4: Triển khai giao diện `PdfSignaturePadView` (Leptos 0.8)**
  - Tích hợp HTML5 `<canvas id="signature-canvas">` hỗ trợ vẽ bằng chuột/bút cảm ứng (`mousedown`, `mousemove`, `mouseup`).
  - Thanh chọn màu mực: Xanh (`#003399`), Đen (`#111111`), Đỏ (`#CC0000`).
  - Thanh chọn độ dày nét bút: Mảnh (1.5px), Vừa (2.5px), Đậm (4.0px).
  - Nút "↺ Xóa làm lại", lựa chọn trang áp dụng (Trang cuối, Trang đầu, Tất cả), lựa chọn Preset vị trí.
- [x] **Step 5: Chạy kiểm định `./scripts/verify_pod.sh pdf-signature-pad`**
  - Run: `./scripts/verify_pod.sh pdf-signature-pad`
  - Expected: Exit code 0, 100% tests pass, 0 network, 0 unwrap/expect.
- [x] **Step 6: Commit**
  - Commit message: `feat(pdf-signature-pad): implement handwritten signature canvas and stamp engine`

---

### Task 3: Xây dựng Pod Công cụ `pdf-to-images-pack` (Xuất Toàn bộ Trang PDF sang Gói Ảnh ZIP)

**Files:**
- Create: `crates/office-engine/src/doc_suite/pdf_to_images_pack.rs`
- Modify: `crates/office-engine/src/doc_suite/mod.rs`
- Modify: `crates/office-engine/src/lib.rs`
- Create: `crates/office-ui/src/tools/pdf_to_images_pack_view.rs`
- Create: `src/tools/pdf_to_images_pack_view.rs`
- Test: `crates/office-engine/src/doc_suite/pdf_to_images_pack.rs` (bổ sung 5 unit tests)

**Interfaces:**
- Produces: `PdfToImagesPackTool`, `PdfToImagesPackOptions { format: String, dpi: u32, quality: u8, prefix: Option<String> }`

- [x] **Step 1: Viết failing unit tests trong `pdf_to_images_pack.rs`**
  - Viết `test_pdf_to_images_pack_happy_path`: Xuất PDF 3 trang thành file `.zip` chứa `trang_001.png`, `trang_002.png`, `trang_003.png`.
  - Viết `test_pdf_to_images_pack_invalid_magic_bytes`: Kiểm tra file không hợp lệ.
  - Viết `test_pdf_to_images_pack_cooperative_cancellation`: Kiểm tra dừng khi hủy.
  - Viết `test_pdf_to_images_pack_security_guardrails`: Kiểm tra DPI không hợp lệ hoặc chất lượng > 100.
  - Viết `test_pdf_to_images_pack_edge_case`: Xuất định dạng JPEG với chất lượng tùy biến.
- [x] **Step 2: Chạy cargo test xác nhận test fail**
  - Run: `cargo test -p office-engine pdf_to_images_pack`
  - Expected: FAIL
- [x] **Step 3: Triển khai động cơ `PdfToImagesPackTool`**
  - Quét danh sách trang PDF, kết xuất từng trang thành ảnh bitmap RGB thông qua rasterizer thuần Rust và trích xuất ảnh nhúng.
  - Mã hóa buffer thành định dạng PNG hoặc JPEG bằng thư viện `image`.
  - Sử dụng module đóng gói PKZIP thuần Rust (dùng `flate2::write::DeflateEncoder` và `crc32fast::Hasher`) để nén danh sách file ảnh vào 1 tệp `.zip`.
  - Ghi file `.zip` ra thư mục đầu ra và trả về đường dẫn hoàn tất.
- [x] **Step 4: Triển khai giao diện `PdfToImagesPackView` (Leptos 0.8)**
  - Chọn định dạng ảnh: PNG (không suy hao), JPEG (nén nhẹ).
  - Chọn độ phân giải: 150 DPI (tiêu chuẩn màn hình), 300 DPI (chuẩn in ấn).
  - Thanh trượt chất lượng JPEG (60% - 100%).
  - Tiền tố tên tệp trong ZIP: ví dụ `trang_`.
- [x] **Step 5: Chạy kiểm định `./scripts/verify_pod.sh pdf-to-images-pack`**
  - Run: `./scripts/verify_pod.sh pdf-to-images-pack`
  - Expected: Exit code 0, 100% tests pass, 0 network, 0 unwrap/expect.
- [x] **Step 6: Commit**
  - Commit message: `feat(pdf-to-images-pack): implement multi-page image export and zip archive generator`

---

### Task 4: Đăng ký Hệ thống, Routing Micro-App và Kiểm thử Tích hợp Toàn diện

**Files:**
- Modify: `src/tools/mod.rs`
- Modify: `crates/office-ui/src/tools/mod.rs`
- Modify: `src/micro_apps/pdf_studio/state.rs`
- Modify: `src/micro_apps/registry.rs`
- Modify: `src/micro_apps/pdf_studio/layout.rs`
- Modify: `src/micro_apps/pdf_studio/toolbar.rs`
- Modify: `tests/pdf_studio_test.rs`

- [x] **Step 1: Xuất các Tool Views mới trong `src/tools/mod.rs` và `crates/office-ui/src/tools/mod.rs`**
  - Export `PdfPageOrganizerView`, `PdfSignaturePadView`, `PdfToImagesPackView`.
- [x] **Step 2: Đăng ký 3 công cụ vào `ToolRegistry` và `PdfCategory`**
  - `pdf-page-organizer` -> `PdfCategory::MergeSplit`
  - `pdf-signature-pad` -> `PdfCategory::SecurityOptimize`
  - `pdf-to-images-pack` -> `PdfCategory::ConvertOcr`
  - Thêm icon đại diện vào `toolbar.rs`: `📑`, `✍️`, `🗜️`.
- [x] **Step 3: Cập nhật `PdfStudioLayout` routing**
  - Thêm nhánh match cho 3 view mới trong `pdf-studio-tool-view-body`.
- [x] **Step 4: Cập nhật `tests/pdf_studio_test.rs`**
  - Bổ sung 3 slug mới vào `EXPECTED_PDF_STUDIO_SLUGS` (tổng 25 công cụ).
  - Cập nhật số lượng công cụ trong danh mục và test lọc tìm kiếm Hub (25 tools).
- [x] **Step 5: Chạy toàn bộ kiểm thử hệ thống**
  - Run: `cargo test -p office-engine`
  - Run: `cargo test --test pdf_studio_test`
  - Run: `cargo check --target wasm32-unknown-unknown`
  - Expected: Toàn bộ pass 100%, 0 lỗi cảnh báo compiler.
- [x] **Step 6: Commit**
  - Commit message: `feat(pdf-studio): register 3 phase 1 tools in registry, routing and update suite tests`
