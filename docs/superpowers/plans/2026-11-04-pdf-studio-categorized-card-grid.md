# PDF Studio Categorized Card Grid Hub & Mini-Sidebar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor giao diện PDF Studio từ bố cục 2 cột cố định sang mô hình Hub/Portal với Categorized Card Grid (chuẩn iLovePDF) kết hợp Mini Sidebar tự động thu gọn (56px) trong không gian thao tác tập trung.

**Architecture:** Mở rộng `PdfStudioState` với `view_mode: RwSignal<PdfViewMode>` (`Hub`, `ToolWorkspace`, `Workflow`) và `sidebar_collapsed: RwSignal<bool>`. Xây dựng component `PdfStudioHubView` hiển thị danh mục khối dọc với thanh tìm kiếm thời gian thực. Nâng cấp `PdfStudioFileListPanel` hỗ trợ hiển thị 2 trạng thái (Mini 56px và Full 280px) kèm nút toggle. Điều phối qua `PdfStudioLayout` với thanh breadcrumb quay lại Hub.

**Tech Stack:** Rust, Leptos 0.8 CSR WebAssembly, Tauri v2, CSS Variables (Design Tokens).

**Spec:** `docs/superpowers/specs/2026-11-04-pdf-studio-categorized-card-grid-design.md`

## Global Constraints

- 100% On-Device & Zero Network Egress (NFR-SEC-01).
- Spine Lock Read-Only Guard (`crates/office-shared`, `src-tauri/src/db`, `src-tauri/src/commands` đều bất biến).
- Đáy công thái học 12px và chuẩn Design Tokens (`make verify-ui`).
- Không phá vỡ bất kỳ assertion nào trong 9 unit tests hiện có tại `tests/pdf_studio_test.rs`.
- Tốc độ phản hồi giao diện đạt 60 FPS, chuyển view tức thì qua Leptos Signals.

## Review Focus

1. **State Persistence:** Khi người dùng nạp tệp ở công cụ A, sau đó bấm `← Tất cả công cụ` về Hub rồi vào công cụ B, danh sách `loaded_files` phải được giữ nguyên 100%.
2. **Search Filter Case-Insensitive:** Tìm kiếm trên Hub với tiếng Việt có dấu/không dấu (vd: "gộp", "gop", "nén", "nen") hoặc slug (vd: "ocr") đều phải lọc chính xác.
3. **Mini-Sidebar Toggle:** Nút bấm `▶` và `◀` phải chuyển đổi mượt mà giữa 56px và 280px, không làm vỡ bố cục khu vực Dropzone chính.
4. **Command Palette Integration:** Nhấn `Ctrl+K` và chọn bất kỳ công cụ nào phải đưa thẳng người dùng vào `ToolWorkspace` của công cụ đó với sidebar thu gọn.
5. **Ergonomic Floor & Color Tokens:** Không sử dụng màu hex cứng ngoài `styles/base.css`, toàn bộ `font-size >= 12px`.

---

### Task 1: State & Navigation Modeling (TDD)

**Files:**
- Modify: `src/micro_apps/pdf_studio/state.rs:1-80`
- Test: `tests/pdf_studio_test.rs:1-100`

- [x] **Step 1.1: Viết test kiểm thử các trạng thái mới trong `tests/pdf_studio_test.rs`**
- [x] **Step 1.2: Chạy test để xác nhận test thất bại (chưa có enum `PdfViewMode`)**
- [x] **Step 1.3: Hiện thực hóa enum `PdfViewMode` và signals trong `src/micro_apps/pdf_studio/state.rs`**
- [x] **Step 1.4: Chạy lại toàn bộ test suite của `pdf_studio_test`**

---

### Task 2: Xây dựng Component `PdfStudioHubView` (Categorized Card Grid)

**Files:**
- Create: `src/micro_apps/pdf_studio/hub_view.rs`
- Modify: `src/micro_apps/pdf_studio/mod.rs`
- Test: `tests/pdf_studio_test.rs`

- [x] **Step 2.1: Viết component `PdfStudioHubView` trong `src/micro_apps/pdf_studio/hub_view.rs`**
- [x] **Step 2.2: Export `PdfStudioHubView` trong `src/micro_apps/pdf_studio/mod.rs`**
- [x] **Step 2.3: Viết unit test lọc công cụ trên Hub trong `tests/pdf_studio_test.rs`**
- [x] **Step 2.4: Chạy kiểm thử biên dịch**

---

### Task 3: Nâng cấp `PdfStudioFileListPanel` thành Collapsible Mini-Sidebar

**Files:**
- Modify: `src/micro_apps/pdf_studio/file_list_panel.rs:1-199`
- Test: `tests/pdf_studio_test.rs`

- [x] **Step 3.1: Thêm nhánh hiển thị Mini Sidebar khi `state.sidebar_collapsed.get() == true`**
- [x] **Step 3.2: Thêm nút Toggle `◀` trên thanh Header của Expanded Sidebar**
- [x] **Step 3.3: Kiểm thử tương tác logic chuyển đổi trạng thái**

---

### Task 4: Tích hợp `PdfStudioLayout` & Breadcrumb Navigation

**Files:**
- Modify: `src/micro_apps/pdf_studio/layout.rs:1-286`
- Test: `tests/pdf_studio_test.rs`

- [x] **Step 4.1: Cập nhật `PdfStudioLayout` phân nhánh theo `state.view_mode.get()`**
- [x] **Step 4.2: Tích hợp Command Palette (`Ctrl+K`)**
- [x] **Step 4.3: Chạy kiểm thử biên dịch**

---

### Task 5: Hoàn thiện CSS Styling trong `styles/pdf_studio.css` & Chuẩn Tokens

**Files:**
- Modify: `styles/pdf_studio.css`
- Verify: `make verify-ui`

- [x] **Step 5.1: Định kiểu cho Hub View & Search Bar**
- [x] **Step 5.2: Định kiểu cho Mini-Sidebar (56px) và Transition mở rộng**
- [x] **Step 5.3: Chạy `make verify-ui`**

---

### Task 6: Kiểm định Chất lượng Toàn diện & Trực quan Trình duyệt

**Files:**
- Test: Playwright Browser Test
- Artifacts: Chụp ảnh đối chứng `screenshots/pdf_studio_hub_view.png` & `screenshots/pdf_studio_focused_workspace.png`

- [x] **Step 6.1: Chạy toàn bộ bộ kiểm thử đơn vị**
- [x] **Step 6.2: Chạy kiểm tra Spine Lock**
- [x] **Step 6.3: Kiểm tra trực quan trên trình duyệt qua Playwright**
- [x] **Step 6.4: Git Commit theo chuẩn Conventional Commits**
