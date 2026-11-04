# Thiết kế Kỹ thuật: PDF Studio Categorized Card Grid Hub & Mini-Sidebar

- **Tác giả:** Antigravity / User Pair Programming
- **Ngày lập:** 2026-11-04
- **Trạng thái:** Bản thảo đề xuất duyệt (Draft for Approval)
- **Căn cứ yêu cầu:** Refactor UI PDF Studio theo Mô hình Hub/Portal (kiểu iLovePDF) với Mini Sidebar tự động thu gọn.

---

## 1. Bối cảnh & Mục tiêu (Intent & The Why)

### 1.1. Hiện trạng
- Hiện tại, giao diện của Micro-App **PDF Studio** sử dụng bố cục cố định 2 cột:
  - Cột trái: `PdfStudioFileListPanel` (chiếm ~280px–300px cố định) hiển thị danh sách tệp nguồn.
  - Cột phải: Thanh Tab Toolbar chia 4 nhóm nghiệp vụ và mở mặc định công cụ đầu tiên `pdf-merge-split`.
- Hạn chế của giao diện cũ:
  - Buộc người dùng rơi vào một công cụ cụ thể ngay khi vào ứng dụng, gây bối rối nếu họ có nhu cầu khác (ví dụ: cần nén PDF hoặc chuyển PDF sang Excel).
  - Không gian vùng thao tác bị thu hẹp do Sidebar tệp chiếm diện tích liên tục dù chưa có file nào được nạp.
  - Khó quét bằng mắt toàn bộ 22 tính năng phong phú của PDF Studio.

### 1.2. Mục tiêu (Goals)
1. **Kiến trúc Chọn công cụ trước (Tool-First Portal):** Khi mở PDF Studio, người dùng được chào đón bằng **Trang chủ Hub (Portal View)** dạng lưới thẻ (Categorized Card Grid) trực quan, chia theo các khối Section danh mục nghiệp vụ.
2. **Quét nhanh & Tìm kiếm thời gian thực:** Mỗi Card chứa icon đặc trưng cỡ lớn, tiêu đề tiếng Việt rõ ràng, mô tả ngắn gọn 1 dòng. Kèm thanh Search/Filter trên đỉnh để lọc nhanh 22 công cụ theo từ khóa.
3. **Cơ chế Mini Sidebar tự động thu gọn (Collapsible Shared Context):**
   - Khi click chọn một công cụ từ Hub, giao diện chuyển sang **Không gian làm việc tập trung (Focused Tool Workspace)**.
   - Thanh Sidebar tệp tự động thu hẹp thành **Mini Sidebar (~52px)** hiển thị icon badge số lượng tệp và nút thêm nhanh, giải phóng hơn 90% chiều rộng màn hình cho Dropzone và cấu hình.
   - Cung cấp nút Toggle (`◀`/`▶` hoặc hamburger) để người dùng mở rộng lại Sidebar đầy đủ khi cần quản lý tệp chi tiết.
4. **Điều hướng Breadcrumb linh hoạt:** Nút `← Tất cả công cụ (PDF Hub)` trên thanh tiêu đề cho phép quay về Portal bất kỳ lúc nào.
5. **Độ an toàn kiến trúc & Tương thích ngược:** Giữ vững tính toàn vẹn 100% của 9 unit tests hiện có trong `tests/pdf_studio_test.rs`, tuân thủ khóa Spine Lock và bộ tiêu chuẩn Design Tokens (`make verify-ui`).

### 1.3. Phi mục tiêu & Ranh giới cấm (Non-Goals & Anti-Goals)
- **Non-Goals:** Không thay đổi logic nghiệp vụ xử lý PDF trong `crates/office-engine` hoặc chữ ký lệnh IPC Tauri.
- **Anti-Goals:** Tuyệt đối không làm mất danh sách tệp đã nạp khi người dùng điều hướng qua lại giữa Hub và các công cụ khác nhau (Shared Context phải được duy trì liên tục).

---

## 2. Các phương án kiến trúc đã cân nhắc (Approaches)

### Phương án A: Quản lý Chế độ hiển thị bằng Reactive Signal trong `PdfStudioState` (**ĐÃ CHỌN**)
- **Mô tả:** Bổ sung `view_mode: RwSignal<PdfViewMode>` (`Hub`, `ToolWorkspace`, `Workflow`) và `sidebar_collapsed: RwSignal<bool>`.
- **Ưu điểm:**
  - Tự nhiên theo mô hình Leptos CSR Reactive, chuyển đổi tức thì (<16ms, 60 FPS) không cần tải lại DOM.
  - Duy trì trọn vẹn danh sách tệp `loaded_files` trong suốt phiên làm việc.
  - Tách bạch cấu trúc: Module `hub_view.rs` đảm nhiệm Portal, `layout.rs` điều phối chuyển đổi, `file_list_panel.rs` hỗ trợ cả 2 trạng thái Full và Mini.
- **Đánh đổi:** Cần viết thêm CSS chuyển động mượt cho Mini Sidebar.

### Phương án B: Slide-over Drawer / Lớp phủ Overlay (Loại bỏ)
- **Mô tả:** Giữ Hub ở nền, bật tool view dạng Drawer trượt từ phải sang.
- **Lý do loại bỏ:** Giới hạn diện tích chiều ngang, không tối ưu cho các công cụ so sánh trực quan 2 trang (`pdf-visual-diff`) hoặc biên tập Form phức tạp.

### Phương án C: Giữ Tab Toolbar cố định phía trên kết hợp Hub Grid (Loại bỏ)
- **Mô tả:** Vừa có tab ở trên đỉnh, vừa có danh mục khối dọc bên dưới.
- **Lý do loại bỏ:** Trùng lặp điều hướng, làm rối giao diện và mất đi tính tinh gọn chuẩn mực của phong cách iLovePDF.

---

## 3. Kiến trúc Chi tiết & Thành phần Giao diện

```
┌────────────────────────────────────────────────────────────────────────┐
│ PDF Studio Layout                                                      │
├────────────────────────────────────────────────────────────────────────┤
│ Chế độ 1: HUB PORTAL VIEW (Toàn màn hình)                             │
│ ┌────────────────────────────────────────────────────────────────────┐ │
│ │ Header: Tiêu đề + Tagline + Nút "← Về Launcher" + Ctrl+K          │ │
│ │ Thanh tìm kiếm: 🔍 "Tìm kiếm nhanh trong 22 công cụ PDF..."        │ │
│ ├────────────────────────────────────────────────────────────────────┤ │
│ │ 📁 SECTION 1: TỔ CHỨC & BỐ CỤC TRANG (5 Cards)                     │ │
│ │   [Ghép/Tách PDF]  [Cắt lề & Canh lề]  [Dàn trang in Booklet]...   │ │
│ │ 🔒 SECTION 2: BẢO MẬT & BIỂU MẪU (7 Cards)                          │ │
│ │   [Đặt mật khẩu]   [Xóa siêu dữ liệu]  [Bôi đen nhạy cảm]...       │ │
│ │ 🎨 SECTION 3: ĐỊNH DẠNG & ĐÓNG DẤU (3 Cards)                        │ │
│ │   [Đánh số trang]  [Đóng dấu Watermark][Thang xám in ấn]           │ │
│ │ 🔄 SECTION 4: CHUYỂN ĐỔI & TRÍCH XUẤT (5 Cards)                     │ │
│ │   [PDF sang Markdown] [Trích xuất bảng Excel] [OCR Tiếng Việt]... │ │
│ │ ⚡ SECTION 5: TỰ ĐỘNG HÓA QUY TRÌNH (1 Banner Card)                 │ │
│ │   [Chuỗi Quy trình Macro DAG - Xử lý liên hoàn]                   │ │
│ └────────────────────────────────────────────────────────────────────┘ │
├────────────────────────────────────────────────────────────────────────┤
│ Chế độ 2: FOCUSED TOOL WORKSPACE (Khi đã chọn công cụ)                │
│ ┌──────────┬─────────────────────────────────────────────────────────┐ │
│ │ MINI     │ Top Bar: ← Tất cả công cụ (Hub) | Tên công cụ | 🔒 Native│ │
│ │ SIDEBAR  ├─────────────────────────────────────────────────────────┤ │
│ │ (52px)   │ Context Banner: Đang áp dụng cho tệp X...               │ │
│ │          │                                                         │ │
│ │ [Toggle] │ [ Khu vực Dropzone Kéo-Thả Tệp Rộng Lớn / Tool View ]  │ │
│ │ [Badge 3]│                                                         │ │
│ │ [+] Thêm │ [ Panel Cấu hình Tham số Nghiệp vụ ]                    │ │
│ │ [File 1] │                                                         │ │
│ │ [File 2] │ [ TaskExecutor: 🚀 Thực thi tác vụ (Ctrl+Enter) ]       │ │
│ └──────────┴─────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘
```

### 3.1. Phân bổ 22 Công cụ vào 4 Section Grid chuẩn
1. **Section 1: Tổ chức & Bố cục trang (`MergeSplit`)**
   - `pdf-merge-split`: Ghép & Tách trang PDF (Icon: 📑)
   - `pdf-crop-margin`: Cắt lề & Cân chỉnh trang (Icon: ✂️)
   - `pdf-booklet-imposition`: Dàn trang sách / Booklet (Icon: 📖)
   - `pdf-bookmark-toc`: Tạo mục lục & Bookmarks (Icon: 🔖)
   - `pdf-visual-diff`: So sánh trực quan 2 bản PDF (Icon: 🔍)

2. **Section 2: Bảo mật & Biểu mẫu (`SecurityOptimize`)**
   - `pdf-protect-sign`: Đặt mật khẩu & Phân quyền bảo mật (Icon: 🔒)
   - `pdf-compress`: Nén & Tối ưu dung lượng PDF (Icon: 🗜️)
   - `pdf-metadata-scrubber`: Xóa sạch siêu dữ liệu nhạy cảm (Icon: 🧹)
   - `pdf-redaction`: Bôi đen che thông tin mật (Icon: ⬛)
   - `pdf-flatten-forms`: Làm phẳng trường AcroForm (Icon: 📄)
   - `pdf-form-manager`: Quản lý & Điền form PDF (Icon: 📋)
   - `pdf-archive-linearize`: Chuẩn hóa PDF/A & Fast Web View (Icon: 📦)
   - `pdf-pki-sign`: Ký số điện tử chuẩn PKI (Icon: ✍️)
   - `pdf-repair-corrupt`: Sửa chữa tệp PDF bị lỗi/hỏng (Icon: 🛠️)

3. **Section 3: Định dạng & Đóng dấu (`FormatStamp`)**
   - `pdf-page-number-header`: Đánh số trang, Header & Footer (Icon: 🔢)
   - `pdf-watermark-stamp`: Đóng dấu chìm Watermark bản quyền (Icon: 💧)
   - `pdf-grayscale-print`: Chuyển đổi sang thang xám tiết kiệm mực (Icon: 🖨️)

4. **Section 4: Chuyển đổi & Trích xuất (`ConvertOcr`)**
   - `pdf-to-markdown`: Chuyển đổi tài liệu PDF sang Markdown (Icon: 📝)
   - `pdf-to-long-image`: Xuất toàn bộ PDF thành ảnh dài cuộn (Icon: 🖼️)
   - `pdf-table-extractor`: Trích xuất bảng biểu PDF sang Excel/CSV (Icon: 📊)
   - `vietnamese-ocr`: Nhận diện chữ OCR tiếng Việt độ chính xác cao (Icon: 👁️)
   - `markdown-to-styled-pdf`: Biên dịch Markdown thành PDF định kiểu chuẩn (Icon: 🎨)

5. **Section 5: Tự động hóa Macro DAG**
   - Banner Card: Tự động hóa quy trình liên hoàn nhiều bước (Icon: ⚡)

### 3.2. Cấu trúc Trạng thái (State Modeling)
Trong `src/micro_apps/pdf_studio/state.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfViewMode {
    Hub,
    ToolWorkspace,
    Workflow,
}
```
- Bổ sung `view_mode: RwSignal<PdfViewMode>` (mặc định là `PdfViewMode::Hub`).
- Bổ sung `sidebar_collapsed: RwSignal<bool>` (mặc định `true` khi ở `ToolWorkspace`).
- Phương thức điều hướng:
  - `state.open_hub()`: Đặt `view_mode = PdfViewMode::Hub`.
  - `state.open_tool(slug)`: Đặt `active_tool = slug`, `view_mode = PdfViewMode::ToolWorkspace`, `sidebar_collapsed = true`.
  - `state.open_workflow()`: Đặt `view_mode = PdfViewMode::Workflow`.
  - `state.toggle_sidebar()`: Đảo ngược trạng thái `sidebar_collapsed`.

### 3.3. Thiết kế Mini-Sidebar
- Chiều rộng thu gọn: `56px`. Chiều rộng mở rộng: `280px`.
- Chế độ Mini:
  - Nút Toggle mở rộng ở trên cùng (`▶`).
  - Badge tròn hiển thị tổng số tệp đã nạp (ví dụ: `3`).
  - Nút thêm tệp nhanh `+`.
  - Danh sách icon đại diện cho các tệp đã nạp; hover hiện tooltip tên tệp; click để đổi tệp active.
- Chế độ Expanded:
  - Nút Toggle thu gọn (`◀`).
  - Toàn bộ danh sách tệp đầy đủ với tên file, kích thước, nút xóa, nút nạp thêm tệp.

---

## 4. Kiểm thử & Tiêu chuẩn Chất lượng

1. **Unit Tests (`tests/pdf_studio_test.rs`):**
   - Giữ nguyên 100% các assertion hiện tại (không sửa đổi test gây vi phạm harness).
   - Bổ sung test kiểm thử trạng thái `view_mode` (`Hub` -> `open_tool` -> `ToolWorkspace` -> `open_hub`).
   - Bổ sung test kiểm thử `sidebar_collapsed` và `toggle_sidebar`.
2. **Kiểm tra UI Tokens & Đáy công thái học:**
   - Chạy `make verify-ui` đảm bảo 100% tuân thủ CSS variables trong `styles/base.css` và `font-size >= 12px`.
3. **Spine Lock Verification:**
   - Chạy `./scripts/verify_spine_lock.sh` đảm bảo khung xương sống (shared, db, commands) không bị suy chuyển.
4. **Trực quan trên trình duyệt:**
   - Khởi chạy dev web và chụp ảnh đối chứng Hub View và Mini Sidebar qua Playwright.

---

## 5. Kế hoạch Triển khai (Next Steps)
Sau khi tài liệu thiết kế này được duyệt, quy trình sẽ tiếp tục:
1. Tạo kịch bản kế hoạch chi tiết `docs/superpowers/plans/2026-11-04-pdf-studio-categorized-card-grid.md`.
2. Hiện thực từng bước TDD: Cập nhật State -> Thêm `hub_view.rs` -> Nâng cấp `file_list_panel.rs` (Mini Sidebar) -> Cập nhật `layout.rs` và `pdf_studio.css`.
3. Chạy 4 cổng kiểm định chất lượng và commit theo chuẩn Git Hook.
