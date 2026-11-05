# Kế Hoạch Triển Khai: Tích Hợp Động Cơ & Giao Diện Đồ Họa PDFCraft Vào PDF Studio (PDFCraft Integration Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Thay thế hoàn toàn phân hệ PDF Studio cũ bằng không gian làm việc đồ họa trực quan của PDFCraft (chuẩn Adobe Acrobat) chạy qua WebAssembly Canvas bên trong cửa sổ Tauri duy nhất, đồng thời tích hợp toàn bộ các công cụ độc bản phục vụ văn phòng Việt Nam (Vietnamese OCR, Table Extractor → Excel, In sách Booklet, Ghép ảnh dài, In Grayscale, Sửa file hỏng).

**Architecture:** 
1. Đặt mã nguồn PDFCraft tại `vendor/pdfcraft/` với pipeline build độc lập `trunk build --release` cho target `pdfcraft-web`, xuất các tệp tĩnh ra `public/pdfcraft/`.
2. Thay thế `PdfStudioApp` trong Leptos bằng một khung nhìn nhúng `<iframe>` kết nối với PDFCraft qua giao thức `postMessage` (`host::Bridge`).
3. Giữ nguyên thanh Header vỏ ngoài của Leptos để điều hướng về Launcher và chuyển đổi các micro-app khác.
4. Xây dựng thanh công cụ tiện ích mở rộng Việt Nam (**VN Extension Ribbon**) bên ngoài iframe, trao đổi byte nhị phân hai chiều giữa Canvas và các engine native của `office-engine` thông qua Tauri IPC.

**Tech Stack:** Rust (Edition 2024), Leptos 0.8, Tauri v2, `egui`, `eframe` (WASM / WebGL Canvas), Trunk, `office-engine`.

**Spec:** `docs/superpowers/specs/2026-11-05-pdfcraft-integration-design.md`

## Global Constraints

- `NFR-SEC-01`: Cấm tuyệt đối kết nối mạng (Zero-Cloud). Cần gỡ bỏ hoặc tắt tính năng `craft-update` trong PDFCraft.
- `NFR-SEC-03`: Cấm `.unwrap()` và `.expect()` trong mã nghiệp vụ kết nối và điều phối IPC.
- `NFR-REL-01`: Giữ tính tương thích với `PdfStudioState` để các component phụ trợ và test suite không bị gãy đổ.
- `NFR-PERF-01`: Quá trình nạp và chuyển mảng byte tài liệu lớn qua `postMessage` bắt buộc dùng `ArrayBuffer` transferables để không bị nghẽn giao diện.

## Review Focus

- File bundle WASM của PDFCraft chưa build hoặc đường dẫn asset bị sai: Hiển thị giao diện fallback thông báo hướng dẫn build thay vì để iframe trắng trơn.
- Tài liệu dung lượng lớn (>50MB) chuyển qua lại giữa Leptos và Canvas: Truyền mảng byte dạng zero-copy qua `post_message_with_transfer`.
- Tài liệu đang có thay đổi chưa lưu (`dirty = true`): Cảnh báo người dùng khi bấm nút quay về Launcher hoặc đổi sang micro-app khác.
- Chuyển đổi qua lại giữa các micro-app: Thu hồi bộ nhớ canvas khi chuyển tab, tránh rò rỉ bộ nhớ WebGL context.
- Tên file có dấu tiếng Việt (UTF-8) khi xuất từ canvas hoặc truyền xuống `office-engine`: Giữ nguyên tính toàn vẹn ký tự không bị lỗi font (mojibake).

---

## Danh Sách Task Thực Thi

### Task 1: Vendor PDFCraft & Cấu Hình Build Pipeline Độc Lập

**Files:**
- Create: `scripts/build_pdfcraft_web.sh`
- Modify: `vendor/pdfcraft/apps/pdfcraft-web/Cargo.toml`
- Modify: `tauri.conf.json` (đảm bảo `public/pdfcraft` được nạp vào asset bundle)
- Test: `scripts/build_pdfcraft_web.sh`

**Interfaces:**
- Produces: Thư mục `public/pdfcraft/` chứa `index.html`, `pdfcraft-web.wasm`, `pdfcraft-web.js`.

- [ ] **Step 1: Viết test kịch bản kiểm tra build output**
  - Viết script kiểm tra file `public/pdfcraft/index.html` và `public/pdfcraft/*.wasm` tồn tại sau khi chạy build.
- [ ] **Step 2: Sao chép mã nguồn PDFCraft vào `vendor/pdfcraft/`**
  - Di chuyển `/home/quan/workspace/pdfcraft` vào `/home/quan/workspace/tauri-v2/office-utility-vn/vendor/pdfcraft`.
  - Vô hiệu hóa `craft-update` trong `vendor/pdfcraft/apps/pdfcraft/Cargo.toml` và các file liên quan để tuân thủ `NFR-SEC-01`.
- [ ] **Step 3: Tạo script `scripts/build_pdfcraft_web.sh`**
  - Chạy `trunk build --release --dist ../../../public/pdfcraft` bên trong thư mục `vendor/pdfcraft/apps/pdfcraft-web`.
- [ ] **Step 4: Chạy thử script build và xác nhận tạo thành công file WASM**
  - Run: `bash scripts/build_pdfcraft_web.sh`
  - Expected: Tạo ra thư mục `public/pdfcraft` đầy đủ asset tĩnh.
- [ ] **Step 5: Commit mã nguồn vendor và build script**
  - `git add scripts/build_pdfcraft_web.sh vendor/pdfcraft`
  - `git commit -m "feat(pdf-studio): vendor pdfcraft and setup standalone wasm build pipeline"`

---

### Task 2: Nhúng Iframe PDFCraft Canvas Vào `PdfStudioApp` (Leptos)

**Files:**
- Modify: `src/micro_apps/pdf_studio/mod.rs`
- Create: `src/micro_apps/pdf_studio/canvas_frame.rs`
- Modify: `src/micro_apps/pdf_studio/layout.rs`
- Test: `tests/pdf_studio_test.rs`

**Interfaces:**
- Produces: Component `PdfCraftCanvasFrame` nhúng thẻ `<iframe id="pdfcraft-frame" src="/pdfcraft/index.html?host=parent">`.
- Preserves: Navigation Header (`← Về Launcher`, chuyển đổi Micro-App).

- [ ] **Step 1: Viết test kiểm tra khởi tạo `PdfCraftCanvasFrame` trong `tests/pdf_studio_test.rs`**
  - Test kiểm tra component `PdfCraftCanvasFrame` trả về cấu trúc HTML chứa iframe với `src="/pdfcraft/index.html?host=parent"`.
- [ ] **Step 2: Run test xác nhận fail**
  - Run: `cargo test --test pdf_studio_test test_pdfcraft_canvas_frame_renders`
  - Expected: FAIL do chưa có component `PdfCraftCanvasFrame`.
- [ ] **Step 3: Triển khai component `PdfCraftCanvasFrame` trong `canvas_frame.rs`**
  - Tạo khung `iframe` với thuộc tính `allow="clipboard-read; clipboard-write"`, chiều cao và chiều rộng chiếm trọn container `100%`.
  - Thêm trạng thái loading skeleton hiển thị trong lúc WASM canvas đang khởi động.
- [ ] **Step 4: Tích hợp vào `PdfStudioApp` và giữ Header điều hướng**
  - Cập nhật `mod.rs` và `layout.rs`: Thay thế Hub view cũ bằng `PdfCraftCanvasFrame`.
- [ ] **Step 5: Run test xác nhận pass và commit**
  - Run: `cargo test --test pdf_studio_test test_pdfcraft_canvas_frame_renders`
  - Expected: PASS
  - `git commit -m "feat(pdf-studio): embed pdfcraft canvas iframe inside leptos workspace"`

---

### Task 3: Xây Dựng Cầu Nối Giao Tiếp Hai Chiều (`host::Bridge`)

**Files:**
- Create: `src/micro_apps/pdf_studio/bridge.rs`
- Modify: `src/micro_apps/pdf_studio/state.rs`
- Test: `tests/pdf_studio_test.rs`

**Interfaces:**
- Consumes: Giao thức `pdfcraft:ready`, `pdfcraft:open`, `pdfcraft:save`, `pdfcraft:dirty` từ PDFCraft.
- Produces: `PdfCraftBridge::open_document(name: &str, bytes: &[u8])`, `PdfCraftBridge::on_ready(cb)`, `PdfCraftBridge::on_save(cb)`.

- [ ] **Step 1: Viết test kiểm tra serialize / deserialize tin nhắn bridge**
  - Viết unit tests kiểm tra cấu trúc payload JSON/JsValue của các thông điệp `pdfcraft:ready`, `pdfcraft:open`, `pdfcraft:save`, `pdfcraft:dirty`.
- [ ] **Step 2: Run test xác nhận fail**
  - Run: `cargo test --test pdf_studio_test test_bridge_protocol`
  - Expected: FAIL do chưa có module `bridge.rs`.
- [ ] **Step 3: Triển khai `bridge.rs` bằng `web_sys` và `wasm_bindgen`**
  - Thiết lập listener lắng nghe sự kiện `message` từ `window`.
  - Kiểm tra origin và source window khớp với iframe con.
  - Cập nhật `PdfStudioState`: Đặt cờ `is_canvas_ready = true` khi nhận `pdfcraft:ready`; cập nhật `is_dirty` khi nhận `pdfcraft:dirty`.
  - Hàm `send_open_file(iframe_id, name, bytes)` sử dụng `post_message_with_transfer`.
- [ ] **Step 4: Đấu nối nạp file từ đĩa qua Tauri Command**
  - Khi người dùng kéo thả file hoặc chọn mở file từ máy tính, đọc byte và gọi `send_open_file` để đưa thẳng vào PDFCraft Canvas.
- [ ] **Step 5: Run test xác nhận pass và commit**
  - Run: `cargo test --test pdf_studio_test test_bridge_protocol`
  - Expected: PASS
  - `git commit -m "feat(pdf-studio): implement bidirectional host bridge for pdfcraft canvas"`

---

### Task 4: Xây Dựng Thanh Tiện Ích Độc Bản Việt Nam (VN Extension Ribbon) & Đấu Nối IPC

**Files:**
- Create: `src/micro_apps/pdf_studio/vn_ribbon.rs`
- Modify: `src/micro_apps/pdf_studio/layout.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Test: `tests/pdf_studio_test.rs`

**Interfaces:**
- Produces: Component `PdfVnExtensionRibbon` chứa các nút:
  * 🇻🇳 Nhận dạng OCR tiếng Việt (`vietnamese-ocr`)
  * 📊 Xuất bảng biểu sang Excel (`pdf-table-extractor`)
  * 📜 Ghép ảnh dài Infographic (`pdf-to-long-image`)
  * 📖 In sách đóng kim Booklet (`pdf-booklet-imposition`)
  * 🖨️ In tiết kiệm mực Grayscale (`pdf-grayscale-print`)
  * 🛠️ Khôi phục PDF lỗi (`pdf-repair-corrupt`)

- [ ] **Step 1: Viết test kiểm tra hiển thị thanh VN Extension Ribbon**
  - Kiểm tra đầy đủ 6 nút bấm công cụ độc bản với nhãn tiếng Việt và icon tương ứng.
- [ ] **Step 2: Run test xác nhận fail**
  - Run: `cargo test --test pdf_studio_test test_vn_extension_ribbon_renders`
  - Expected: FAIL do chưa có `vn_ribbon.rs`.
- [ ] **Step 3: Triển khai giao diện `PdfVnExtensionRibbon` trong `vn_ribbon.rs`**
  - Xây dựng thanh Action bar đặt ngay trên vùng iframe với phong cách giao diện tối giản (Slate & Indigo).
  - Thêm modal/popover cấu hình nhanh cho từng công cụ (chọn độ phân giải ảnh, chọn kiểu đóng kim, tùy chọn OCR).
- [ ] **Step 4: Đấu nối IPC `dispatch_tool_task` xuống `office-engine`**
  - Khi bấm nút: Lấy dữ liệu file hiện hành, gọi Tauri IPC `dispatch_tool_task` tương ứng.
  - Hiển thị thanh tiến trình xử lý song song.
  - Nếu kết quả là file PDF (Booklet, Grayscale, Repair): Gửi lệnh `pdfcraft:open` nạp ngược lại Canvas để xem trước ngay.
  - Nếu kết quả là file dữ liệu (`.xlsx`, `.png`): Mở hộp thoại lưu file an toàn.
- [ ] **Step 5: Run test xác nhận pass và commit**
  - Run: `cargo test --test pdf_studio_test test_vn_extension_ribbon_renders`
  - Expected: PASS
  - `git commit -m "feat(pdf-studio): add vietnamese extension ribbon and wire native ipc tasks"`

---

### Task 5: Cập Nhật Điều Hướng Launcher, Dọn Dẹp Mã Thừa & Kiểm Thử E2E Toàn Diện

**Files:**
- Modify: `src/micro_apps/registry.rs`
- Modify: `src/launcher/mod.rs`
- Modify: `tests/pdf_studio_test.rs`
- Modify: `tests/launcher_test.rs`
- Test: Toàn bộ test suite liên quan đến PDF Studio

**Interfaces:**
- Chuyển hướng người dùng từ Launcher vào thẳng `PdfStudio` Canvas trực quan.

- [ ] **Step 1: Viết test kiểm tra chuyển hướng từ Launcher vào PDF Studio**
  - Đảm bảo khi chọn PDF Studio từ Launcher, trạng thái điều hướng chuyển chính xác vào `MicroAppId::PdfStudio`.
- [ ] **Step 2: Cập nhật Registry và Launcher**
  - Điều chỉnh `registry.rs` để các công cụ chuẩn của PDF (ghép, tách, nén, mã hóa...) được đánh dấu xử lý trực quan trên Canvas của PDFCraft.
  - Chuyển hướng các deep links từ Command Palette vào PDF Studio.
- [ ] **Step 3: Chạy toàn bộ test suite PDF Studio**
  - Run: `cargo test --test pdf_studio_test`
  - Expected: Tất cả các unit tests kiểm thử trạng thái và giao diện mới đều đạt PASS (exit code 0).
- [ ] **Step 4: Chạy kiểm thử launcher tổng thể**
  - Run: `cargo test --test launcher_test`
  - Expected: PASS (exit code 0).
- [ ] **Step 5: Commit hoàn thiện kế hoạch**
  - `git commit -m "chore(pdf-studio): finalize pdfcraft visual workspace transition and clean up deprecated hub views"`
