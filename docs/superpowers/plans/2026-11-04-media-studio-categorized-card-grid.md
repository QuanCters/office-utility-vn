# Media Studio Categorized Card Grid Hub, Mini-Sidebar & Floating Queue Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor giao diện Media Studio từ bố cục 2 cột cố định sang mô hình Hub/Portal với Categorized Card Grid (chuẩn iLoveIMG & Lightroom) kết hợp Mini Gallery Sidebar tự động thu gọn (56px), Persistent Dock Batch Queue tại Workspace và Floating Contextual Progress Widget trên Hub khi có tác vụ chạy ngầm.

**Architecture:** Mở rộng `MediaStudioState` với `view_mode: RwSignal<MediaViewMode>` (`Hub`, `ToolWorkspace`, `Workflow`), `sidebar_collapsed: RwSignal<bool>` và `is_floating_queue_open: RwSignal<bool>`. Xây dựng component `MediaStudioHubView` hiển thị danh mục 2 phân khu lớn kèm Floating Progress Pill / Drawer khi có batch chạy ngầm. Nâng cấp `MediaStudioGalleryPanel` hỗ trợ hiển thị 2 trạng thái (Mini 56px và Full 280px) kèm nút toggle. Điều phối qua `MediaStudioLayout` với thanh breadcrumb quay lại Hub.

**Tech Stack:** Rust, Leptos 0.8 CSR WebAssembly, Tauri v2, CSS Variables (Design Tokens).

**Spec:** `docs/superpowers/specs/2026-11-04-media-studio-categorized-card-grid-design.md`

## Global Constraints

- 100% On-Device & Zero Network Egress (NFR-SEC-01).
- Spine Lock Read-Only Guard (`crates/office-shared`, `src-tauri/src/db`, `src-tauri/src/commands` đều bất biến).
- Đáy công thái học 12px và chuẩn Design Tokens (`make verify-ui`).
- Không phá vỡ bất kỳ assertion nào trong 10 unit tests hiện có tại `tests/media_studio_test.rs`.
- Tốc độ phản hồi giao diện đạt 60 FPS, chuyển view tức thì qua Leptos Signals.

## Review Focus

1. **State Persistence:** Khi người dùng nạp tệp ở công cụ A, sau đó bấm `← Tất cả công cụ (Media Hub)` về Hub rồi vào công cụ B, danh sách `loaded_files` và `batch_queue` phải được giữ nguyên 100%.
2. **Floating Contextual Widget:** Chỉ hiển thị trên Hub khi có tác vụ batch đang chạy ngầm (`is_background_processing` hoặc `has_active_batches`). Click vào widget phải bật/tắt Popover Drawer mà không làm chuyển trang.
3. **Mini Gallery Sidebar Toggle:** Nút bấm `▶` và `◀` phải chuyển đổi mượt mà giữa 56px và 280px, không làm vỡ bố cục khu vực Dropzone chính.
4. **Search Filter Case-Insensitive:** Tìm kiếm trên Hub với tiếng Việt có dấu/không dấu (vd: "ảnh thẻ", "anh the", "nén", "nen", "exif", "gif") đều phải lọc chính xác.
5. **Ergonomic Floor & Color Tokens:** Không sử dụng màu hex cứng ngoài `styles/base.css`, toàn bộ `font-size >= 12px`.

---

### Task 1: State & Navigation Modeling (TDD)

**Files:**
- Modify: `src/micro_apps/media_studio/state.rs:1-250`
- Test: `tests/media_studio_test.rs:1-120`

- [x] **Step 1.1: Viết test kiểm thử các trạng thái mới trong `tests/media_studio_test.rs`**
  Thêm unit test `test_media_studio_view_mode_and_sidebar_toggle`:
  - `state.view_mode.get() == MediaViewMode::Hub` mặc định khi khởi tạo.
  - `state.open_tool("photo-id-crop")` chuyển `view_mode` thành `MediaViewMode::ToolWorkspace`, `active_tool_slug()` thành `"photo-id-crop"`, và `sidebar_collapsed` thành `true`.
  - `state.open_hub()` chuyển `view_mode` về `MediaViewMode::Hub`.
  - `state.open_workflow()` chuyển `view_mode` về `MediaViewMode::Workflow`.
  - `state.toggle_sidebar()` đảo ngược cờ `sidebar_collapsed`.
  - `state.toggle_floating_queue()` đảo ngược cờ `is_floating_queue_open`.
- [x] **Step 1.2: Chạy test để xác nhận test thất bại (chưa có enum `MediaViewMode`)**
  `cargo test --test media_studio_test test_media_studio_view_mode_and_sidebar_toggle` -> compile error.
- [x] **Step 1.3: Hiện thực hóa enum `MediaViewMode` và signals trong `src/micro_apps/media_studio/state.rs`**
  - Khai báo:
    ```rust
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    pub enum MediaViewMode {
        Hub,
        ToolWorkspace,
        Workflow,
    }
    ```
  - Bổ sung vào struct `MediaStudioState`:
    - `pub view_mode: RwSignal<MediaViewMode>`
    - `pub sidebar_collapsed: RwSignal<bool>`
    - `pub is_floating_queue_open: RwSignal<bool>`
  - Bổ sung các helper methods:
    - `open_hub(&self)`
    - `open_tool(&self, slug: &str)`
    - `open_workflow(&self)`
    - `toggle_sidebar(&self)`
    - `toggle_floating_queue(&self)`
    - `is_hub(&self) -> bool`
    - `is_tool_workspace(&self) -> bool`
    - `is_background_processing(&self) -> bool`
    - `batch_progress_ratio(&self) -> f32`
- [x] **Step 1.4: Chạy lại toàn bộ test suite của `media_studio_test`**
  `cargo test --test media_studio_test` -> Tất cả các test cũ và mới đều PASS (11/11).

---

### Task 2: Xây dựng Component `MediaStudioHubView` & Floating Contextual Widget

**Files:**
- Create: `src/micro_apps/media_studio/hub_view.rs`
- Modify: `src/micro_apps/media_studio/mod.rs`
- Test: `tests/media_studio_test.rs`

- [x] **Step 2.1: Viết component `MediaStudioHubView` trong `src/micro_apps/media_studio/hub_view.rs`**
  - Header Hero của Hub:
    - Tiêu đề "Phòng thu Đa phương tiện On-Device".
    - Ô tìm kiếm thời gian thực: `search_query: RwSignal<String>` với placeholder `"Tìm kiếm nhanh trong 9 công cụ Media... (Ctrl+K)"`.
  - Logic lọc danh sách công cụ theo từ khóa (khớp tên tiếng Việt, slug, mô tả hoặc từ đồng nghĩa: "ảnh thẻ", "nén", "exif", "watermark", "screenshot", "scan", "audio", "gif").
  - 2 Section phân nhóm theo `MediaCategory`:
    - **Xử lý & Tối ưu Ảnh** (7 cards)
    - **Audio & Video Studio** (2 cards)
  - 1 Section Banner: **Chuỗi Tự động hóa Macro DAG Đa phương tiện** (khi click gọi `state.open_workflow()`).
  - Card Item:
    - Icon đặc trưng 40px với badge màu theo nhóm danh mục.
    - Tiêu đề công cụ in đậm.
    - Mô tả 1 dòng ngắn gọn.
    - Khi click gọi `state.open_tool(tool.slug)`.
  - Floating Contextual Progress Widget:
    - Hiển thị pill ở góc phải khi `state.is_background_processing()` hoặc `state.has_active_batches()`.
    - Click vào pill bật/tắt Floating Popover Drawer hiển thị tiến độ chi tiết từng tệp trong batch queue.
- [x] **Step 2.2: Export `MediaStudioHubView`, `filter_media_studio_tools` trong `src/micro_apps/media_studio/mod.rs`**
- [x] **Step 2.3: Viết unit test lọc công cụ trên Hub và kiểm thử Floating Widget trong `tests/media_studio_test.rs`**
- [x] **Step 2.4: Chạy kiểm thử biên dịch**
  `cargo check` và `cargo test --test media_studio_test`.

---

### Task 3: Nâng cấp `MediaStudioGalleryPanel` thành Collapsible Mini-Sidebar

**Files:**
- Modify: `src/micro_apps/media_studio/gallery_panel.rs:1-199`
- Test: `tests/media_studio_test.rs`

- [x] **Step 3.1: Thêm nhánh hiển thị Mini Gallery Sidebar khi `state.sidebar_collapsed.get() == true`**
  - Nút Toggle `▶` ở trên cùng với `title="Mở rộng thư viện tệp"`.
  - Badge tròn đếm số tệp đã nạp (`state.loaded_files_count()`).
  - Nút thêm tệp nhanh `+`.
  - Danh sách thumbnail/icon các file đã nạp (rút gọn), click để chọn active file.
- [x] **Step 3.2: Thêm nút Toggle `◀` trên thanh Header của Expanded Gallery**
  - Cho phép người dùng thu gọn lại Mini Sidebar bất cứ khi nào.
- [x] **Step 3.3: Kiểm thử tương tác logic chuyển đổi trạng thái**
  `cargo test --test media_studio_test`.

---

### Task 4: Tích hợp `MediaStudioLayout` & Breadcrumb Navigation

**Files:**
- Modify: `src/micro_apps/media_studio/layout.rs:1-336`
- Test: `tests/media_studio_test.rs`

- [x] **Step 4.1: Cập nhật `MediaStudioLayout` phân nhánh theo `state.view_mode.get()`**
  - Nhánh `MediaViewMode::Hub`: Render `MediaStudioHubView` toàn màn hình.
  - Nhánh `MediaViewMode::Workflow`: Render `WorkflowRunner` kèm thanh Top Bar điều hướng quay lại `← Tất cả công cụ (Media Hub)`.
  - Nhánh `MediaViewMode::ToolWorkspace`: Render Focused Workspace:
    - Top Bar: Nút Breadcrumb `← Tất cả công cụ (Media Hub)`, Tiêu đề công cụ, Tagline, Slug chip, Privacy badge.
    - Thân trang: `MediaStudioGalleryPanel` (ở chế độ mini collapsible) + Tool View Body + Persistent Dock Batch Queue (`MediaStudioBatchQueue`) + `TaskExecutor`.
- [x] **Step 4.2: Tích hợp Command Palette (`Ctrl+K`)**
  Khi người dùng chọn công cụ từ Command Palette, gọi `state.open_tool(&slug)` để đưa thẳng vào Tool Workspace tương ứng.
- [x] **Step 4.3: Chạy kiểm thử biên dịch**
  `cargo check --workspace` và `cargo test --workspace`.

---

### Task 5: Hoàn thiện CSS Styling trong `styles/media_studio.css` & Chuẩn Tokens

**Files:**
- Modify: `styles/media_studio.css`
- Verify: `make verify-ui`

- [x] **Step 5.1: Định kiểu cho Hub View & Search Bar**
  - Lưới thẻ tự co giãn: `display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 16px;`.
  - Card style: viền bo `var(--radius-lg)`, nền `var(--bg-card)`, hiệu ứng hover nâng thẻ `transform: translateY(-2px);` và viền sáng `var(--primary)`.
- [x] **Step 5.2: Định kiểu cho Mini-Sidebar (56px) và Transition mở rộng**
  - `.media-studio-gallery-panel.collapsed`: `width: 56px; min-width: 56px;`.
  - `.media-studio-gallery-panel.expanded`: `width: 280px; min-width: 280px;`.
  - `transition: width 0.2s cubic-bezier(0.4, 0, 0.2, 1);`.
- [x] **Step 5.3: Định kiểu cho Floating Contextual Widget & Popover Drawer**
  - `.media-floating-widget-container`: `position: fixed; bottom: 24px; right: 24px; z-index: 1000;`.
  - `.media-floating-drawer`: `position: absolute; bottom: calc(100% + 12px); right: 0; width: 360px; max-height: 420px;`.
- [x] **Step 5.4: Chạy `make verify-ui`**
  Đảm bảo 100% tuân thủ CSS variables, không mã hex cứng ngoài `base.css`, không font-size < 12px.

---

### Task 6: Kiểm định Chất lượng Toàn diện & Trực quan Trình duyệt

**Files:**
- Test: Playwright Browser Test
- Artifacts: Chụp ảnh đối chứng `screenshots/media_studio_hub_view.png`, `screenshots/media_studio_focused_workspace.png` & `screenshots/media_studio_floating_drawer.png`

- [x] **Step 6.1: Chạy toàn bộ bộ kiểm thử đơn vị**
  `cargo test --workspace` -> 100% PASS.
- [x] **Step 6.2: Chạy kiểm tra Spine Lock**
  `./scripts/verify_spine_lock.sh` -> PASS 100%.
- [x] **Step 6.3: Kiểm tra trực quan trên trình duyệt qua Playwright**
  - Khởi động dev server (`http://localhost:1420`).
  - Mở `http://localhost:1420`.
  - Bấm vào Media Studio -> Kiểm tra Hub View hiển thị đầy đủ 2 Sections ảnh & audio/video, Card đẹp mắt, tìm kiếm hoạt động.
  - Click vào thẻ "Cắt ảnh thẻ Chuẩn 300 DPI" -> Chuyển sang Focused Workspace, gallery thu gọn dạng mini (56px), bấm `▶` mở rộng (280px) và `◀` thu gọn.
  - Nạp tác vụ vào Batch Queue -> Bấm `← Tất cả công cụ (Media Hub)` -> Kiểm tra quay lại Hub View tức thì.
  - Kiểm tra Floating Contextual Progress Pill FAB xuất hiện ở góc phải dưới (`bottom: 24px; right: 24px`), click để mở Popover Drawer với danh sách batch item và các nút thao tác.
  - Lưu ảnh chụp màn hình vào `screenshots/media_studio_hub_view.png`, `screenshots/media_studio_focused_workspace.png`, `screenshots/media_studio_floating_drawer.png`.
- [x] **Step 6.4: Git Commit theo chuẩn Conventional Commits**
  `git commit -m "refactor(media-studio): implement categorized card grid hub, collapsible mini sidebar and floating queue widget"`
