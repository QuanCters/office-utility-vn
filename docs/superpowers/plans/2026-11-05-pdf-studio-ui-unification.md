# Kế Hoạch Triển Khai: Chuẩn Hóa & Hợp Nhất Giao Diện PDF Studio (PDF Studio UI Unification Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Loại bỏ hoàn toàn cảm giác "app trong app" của PDF Studio bằng cách gộp các tầng khung ngang chồng chéo, dùng chung bộ Design Tokens (màu sắc, nền, viền, bo góc, spacing), thay thế emoji bằng line icon đồng bộ, chuẩn hóa typography và hoàn thiện bảng thuật ngữ tiếng Việt nhất quán.

**Architecture:** 
1. **Lớp Khung & Điều Hướng (Host + Canvas):** Giữ Titlebar và Header tinh gọn của host; gộp `tab_strip` và `mode_bar` bên trong Canvas thành một thanh toolbar duy nhất (~40px), loại bỏ các nút Home, theme switch, settings và menu trùng lặp.
2. **Lớp Token Alias (`.pdfcraft-root` & `theme.rs`):** Ánh xạ trực tiếp các CSS variables của host (`--bg-main`, `--bg-surface`, `--bg-card`, `--border-subtle`, `--primary-light`) vào struct `theme::Tokens` trong egui.
3. **Màu sắc & Trạng thái Active:** Icon đơn sắc đổi màu accent khi hover/active; trạng thái active thống nhất (gạch chân accent 2px cho tabs, nền accent nhạt + thanh viền trái 3px cho sidebar).
4. **Chuẩn hóa Icon & Component:** Thay thế toàn bộ emoji (`📄`, `🔒`, `🔄`, `📌`) bằng SVG Line Icons; loại bỏ nhãn "Sử dụng" lặp lại trên cards; biến "Mở tệp" thành primary action / dropzone; empty states đồng bộ.
5. **Ngôn ngữ & Thuật ngữ:** Đồng bộ 100% thuật ngữ Anh - Việt trong `locales/vi/` và chuẩn hóa tên thương hiệu sang "PDF Studio".

**Tech Stack:** Leptos 0.8 (CSR WebAssembly), Tauri v2, `egui` / `eframe` (WASM Canvas), Trunk, CSS Design Tokens.

**Spec / Requirements:** Chi tiết yêu cầu người dùng đề ra theo thứ tự ưu tiên (1) Gộp khung, (2) Design tokens, (3) Màu sắc, (4) Icon, (5) Typography, (6) Component, (7) Ngôn ngữ.

## Global Constraints

- `NFR-SEC-01`: 100% On-Device & Zero Network Egress. Không tạo bất kỳ kết nối mạng hay tải font/icon từ CDN ngoài.
- `SPINE-LOCK`: Không sửa đổi các crate thuộc Spine Layer (`crates/office-shared/src/lib.rs`, `src-tauri/src/db/mod.rs`, `src-tauri/src/commands/mod.rs`). Phải vượt qua `./scripts/verify_spine_lock.sh`.
- `NFR-COM-02`: Đảm bảo an toàn bộ gõ tiếng Việt (IME-Safe) và typography không bị cắt dấu (`line-height >= 1.4`).
- `BUILD-PIPELINE`: Mọi thay đổi trong `vendor/pdfcraft/` phải được biên dịch lại thành công qua `scripts/build_pdfcraft_web.sh` xuất ra `public/pdfcraft/`.

## Review Focus

1. **Chiều cao dọc hữu ích (Vertical Real Estate):** Giảm từ 4 thanh ngang (Titlebar + Module Header + Tab Strip + Mode Bar = ~160px) xuống chỉ còn 2 thanh tối giản (Titlebar + Combined Header Toolbar = ~78px), giải phóng thêm hơn 80px chiều cao cho không gian đọc tài liệu.
2. **Trải nghiệm thị giác liền mạch (Visual Cohesion):** Nền Canvas WebGL (`#0F172A`) và nền Host WebView (`#0F172A`) phải đồng nhất tuyệt đối không có khe hở màu khác lạ.
3. **Tránh xung đột phím tắt:** Host giữ độc quyền `Ctrl+K` (Command Palette) và phím tắt toàn cục; canvas chuyển tiếp sự kiện tìm kiếm về Command Palette chung.
4. **Cắt dấu tiếng Việt:** Font 'Inter' và text label trong canvas phải có padding dọc và line-height đủ lớn để các ký tự có dấu phức tạp như `ệ`, `ở`, `ữ`, `ẩ` hiển thị trọn vẹn.
5. **Tính phản hồi (Responsive Grid):** Danh sách card công cụ trên màn hình lớn phân bố đều thay vì dồn cục sang bên trái.

---

## Danh Sách Task Thực Thi

### Task 1: Tinh Giản Header Của Host & Chuẩn Hóa Icon SVG

**Files:**
- Modify: `src/micro_apps/pdf_studio/layout.rs`
- Modify: `styles/pdf_studio.css`
- Test: `tests/pdf_studio_test.rs`

- [ ] **Step 1: Cập nhật `layout.rs` bỏ emoji và pill trùng lặp**
  - Bỏ nhãn/pill `"Biên tập trực quan"` (vốn trông như nút bấm gây hiểu lầm).
  - Thay thế emoji `"📄 PDF Studio"` thành `PDF Studio` kèm icon SVG `FileText` (stroke 1.75).
  - Tinh giản badge `"🔒 100% On-Device"`: bỏ emoji `🔒`, thay bằng icon SVG `ShieldCheck` viền mảnh tinh tế.
- [ ] **Step 2: Cập nhật CSS của Header module**
  - Giảm chiều cao `pdf-studio-compact-header` xuống `38px`.
  - Tinh chỉnh `pdf-studio-privacy-badge` màu nhã nhặn, hòa hợp với thanh tiêu đề.
- [ ] **Step 3: Kiểm tra và chạy test**
  - Chạy `cargo test -p office-utility-vn-ui --test pdf_studio_test` đảm bảo view render chuẩn xác.

---

### Task 2: Gộp `tab_strip` Và `mode_bar` Trong PDFCraft Thành Một Toolbar Duy Nhất

**Files:**
- Modify: `vendor/pdfcraft/crates/ui-egui/src/chrome.rs`
- Modify: `vendor/pdfcraft/crates/ui-egui/src/app.rs`
- Test: `vendor/pdfcraft/crates/ui-egui/tests/it/ui.rs`

- [ ] **Step 1: Tạo hàm `combined_toolbar` trong `chrome.rs`**
  - Tích hợp tài liệu đang mở (Document Tabs), nút mở tệp, và các tab điều hướng Mode (`Chỉnh sửa`, `Chuyển đổi`, `Ký điện tử`) vào một hàng ngang duy nhất có chiều cao `40px`.
  - Loại bỏ nút `Home` trùng lặp (vì host đã có `← Launcher`).
  - Loại bỏ nút toggle theme và phím tắt trùng lặp (vì host đã quản lý toàn cục).
  - Loại bỏ menu bar trùng lặp `File / Edit / Sign` khi đã có sẵn mode tabs.
  - Loại bỏ tab "Tất cả công cụ" thừa thãi khi sidebar đã hiển thị danh mục công cụ.
  - Giữ các nút hành động thiết yếu ở góc phải: In tài liệu, Lưu tệp, Thuộc tính tài liệu.
- [ ] **Step 2: Cập nhật `app.rs` chỉ gọi một panel toolbar duy nhất**
  - Thay thế hai lệnh `chrome::tab_strip(self, ui);` và `chrome::mode_bar(self, ui);` bằng lệnh gọi toolbar hợp nhất.
- [ ] **Step 3: Kiểm tra biên dịch nội bộ `ui-egui`**
  - Chạy `cargo check -p pdfcraft-ui-egui`.

---

### Task 3: Ánh Xạ Lớp Token Alias & Đồng Bộ Bảng Màu Chuẩn Host

**Files:**
- Modify: `styles/pdf_studio.css`
- Modify: `vendor/pdfcraft/apps/pdfcraft-web/index.html`
- Modify: `vendor/pdfcraft/crates/ui-egui/src/theme.rs`
- Modify: `vendor/pdfcraft/crates/ui-egui/src/panels/tools.rs`
- Modify: `vendor/pdfcraft/crates/ui-egui/src/panels/all_tools_ui.rs`

- [ ] **Step 1: Khai báo lớp CSS Token Alias `.pdfcraft-root`**
  - Trong `styles/pdf_studio.css` và `index.html`, khai báo:
    ```css
    .pdfcraft-root {
      --pc-bg: var(--bg-main);        /* #0F172A */
      --pc-surface: var(--bg-surface); /* #1E293B */
      --pc-card: var(--bg-surface);    /* #1E293B */
      --pc-border: var(--border-subtle);/* #334155 */
      --pc-text: var(--text-main);      /* #F8FAFC */
      --pc-text-dim: var(--text-subtle);/* #94A3B8 */
      --pc-accent: var(--primary-light);/* #6366F1 */
      --pc-radius: var(--radius-md);    /* 6px */
    }
    ```
- [ ] **Step 2: Đồng bộ giá trị màu trong `Tokens::for_kind` (`theme.rs`)**
  - Đặt `titlebar`, `chrome`, `pasteboard` trùng với `#0F172A` / `#0B1120`.
  - Đặt `panel`, `card` trùng với `#1E293B`.
  - Đặt `border`, `divider` trùng với `#334155`.
  - Đặt `accent` thành `#6366F1`, `accent_text` thành `#818CF8`, `accent_soft` thành `rgba(99, 102, 241, 0.16)`.
- [ ] **Step 3: Chuyển icon sidebar sang đơn sắc & phân nhóm theo palette**
  - Trong `panels/tools.rs`, thay thế việc dùng 8 mã màu rời rạc bằng màu đơn sắc `t.icon` (chuyển sang `t.accent` khi hover/active).
  - Phân nhóm lớn thành 3-4 màu chuẩn host: Indigo (Chung), Emerald (Xuất bản), Sky (Biên tập), Amber (Bảo mật).
- [ ] **Step 4: Chuẩn hóa chỉ báo Active (Active Indicator)**
  - Cho Tabs: Vẽ đường gạch chân accent dày 2px ở đáy tab đang chọn (`t.accent`).
  - Cho Sidebar: Vẽ nền accent mờ (`t.accent_soft`) kèm thanh viền accent đứng 3px sát cạnh trái.
  - Bỏ nền xanh đặc của nút Home cũ.

---

### Task 4: Nâng Cấp Typography & Đảm Bảo Không Bị Cắt Dấu Tiếng Việt

**Files:**
- Modify: `vendor/pdfcraft/crates/ui-egui/src/theme.rs`
- Modify: `vendor/pdfcraft/crates/ui-egui/src/home.rs`

- [ ] **Step 1: Chuẩn hóa thang font size trong `theme.rs`**
  - Chốt thang cỡ chữ:
    - 11.0: Nhãn phụ, badge, caption.
    - 12.5: Mô tả card và chú thích (nâng từ 9-11px lên >= 12px theo yêu cầu).
    - 14.0: Thân bài, nút bấm, nhãn trường.
    - 16.0 - 18.0: Tiêu đề mục.
    - 22.0 - 24.0: Tiêu đề lớn trang chủ.
- [ ] **Step 2: Cài đặt line-height >= 1.4 cho các đoạn văn bản**
  - Đảm bảo các `LayoutJob` trong `home.rs` và `panels/` có khoảng giãn dòng tối thiểu 1.4x để các dấu thanh tiếng Việt (ngã, hỏi, nặng, sắc) không bị mép hộp cắt ngang.

---

### Task 5: Hoàn Thiện Component Giao Diện Trang Chủ

**Files:**
- Modify: `vendor/pdfcraft/crates/ui-egui/src/home.rs`

- [ ] **Step 1: Loại bỏ nhãn "Sử dụng" lặp lại trên card**
  - Trong `tool_card`, bỏ việc vẽ dòng chữ `tl!("Use now")`.
  - Tăng cường hiệu ứng hover (đổi viền sáng `t.accent`, đổ bóng nhẹ `card_lift`) và focus khi tương tác.
- [ ] **Step 2: Biến "Mở tệp" thành Primary Action Button**
  - Tạo kiểu dáng nổi bật cho khối "Mở tệp" (nền accent Indigo hoặc khung viền nét đứt dropzone chuyên nghiệp).
- [ ] **Step 3: Đồng bộ Empty States cho "Công cụ đã ghim" và "Gần đây"**
  - Thay thế dòng chữ đơn điệu bằng container rỗng có icon minh họa nhẹ, hướng dẫn kéo thả tệp trực quan.
- [ ] **Step 4: Tối ưu Responsive Grid**
  - Điều chỉnh logic căn lề và độ rộng card để danh sách thẻ phân bố cân đối trên màn hình rộng.

---

### Task 6: Bảng Thuật Ngữ & Nhất Quán Ngôn Ngữ Anh - Việt

**Files:**
- Modify: `vendor/pdfcraft/i18n-locales/locales/vi/common.tsv`
- Modify: `vendor/pdfcraft/i18n-locales/locales/vi/pdfcraft.tsv`

- [ ] **Step 1: Thêm và chuẩn hóa các thuật ngữ còn sót**
  - "Launcher" -> "Màn hình chính"
  - "On-Device" -> "100% On-Device (Nội bộ trên máy)"
  - "Field list" -> "Danh sách trường"
  - "Form" -> "Biểu mẫu"
  - "Detect form fields" -> "Nhận diện trường biểu mẫu"
  - "Clear form" -> "Xóa dữ liệu biểu mẫu"
  - "Prepare a form" -> "Tạo biểu mẫu"
- [ ] **Step 2: Rà soát và thay thế hoàn toàn "PdfCraft" -> "PDF Studio"**
  - Đảm bảo trong toàn bộ chuỗi giao diện hiển thị tên "PDF Studio" nhất quán.

---

### Task 7: Build Bundle WASM, Chạy Test Suite & Nghiệm Thu UI

**Files:**
- Output: `public/pdfcraft/`
- Script: `scripts/build_pdfcraft_web.sh`

- [ ] **Step 1: Biên dịch lại bundle WASM của PDFCraft**
  - Chạy `bash scripts/build_pdfcraft_web.sh`.
  - Xác nhận tạo thành công `public/pdfcraft/index.html` và `public/pdfcraft/pdfcraft-web-*.wasm`.
- [ ] **Step 2: Chạy kiểm định toàn bộ test suite**
  - Chạy `cargo test --workspace`.
  - Chạy `./scripts/verify_spine_lock.sh`.
- [ ] **Step 3: Xác thực giao diện và nghiệm thu**
  - Xác nhận cả 4 tầng khung đã được gộp gọn gàng, màu sắc và icon đồng bộ 100%.
