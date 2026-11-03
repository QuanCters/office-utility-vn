# Micro-App Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Chuyển đổi toàn bộ kiến trúc frontend Leptos của Office Utility VN từ danh sách phẳng 51 công cụ sang 4 Micro-Apps chuyên biệt (VN Office, PDF Studio, Media Studio, Data Lab) với Launcher trung tâm và shared context.

**Architecture:** Tái cấu trúc module phân cấp trong crate frontend (Module Restructuring). Tách `src/app.rs` monolith thành Shell mỏng (~200 dòng) phối hợp trạng thái in-memory với `src/launcher/`, `src/shared_ui/`, và 4 micro-app độc lập trong `src/micro_apps/`. Giữ nguyên 100% backend IPC commands và database SQLite (Zero DB Migration, khóa cứng Spine).

**Tech Stack:** Leptos 0.8 (Rust WASM CSR), Tauri v2, Trunk bundler, WebSys/Wasm-bindgen.

**Spec:** [docs/superpowers/specs/2026-11-03-micro-app-architecture-design.md](file:///home/quan/workspace/tauri-v2/office-utility-vn/docs/superpowers/specs/2026-11-03-micro-app-architecture-design.md)

## Global Constraints

- **Zero Backend Changes:** `crates/office-shared`, `src-tauri/src/db`, và `src-tauri/src/commands` không được phép sửa đổi; phải luôn vượt qua `./scripts/verify_spine_lock.sh`.
- **Zero DB Migration:** Giữ nguyên schema SQLite hiện tại. `app_id` được suy luận bằng hàm mapping tĩnh ở frontend.
- **In-Memory State:** Trạng thái của 4 micro-app được lưu trữ in-memory ở App root để tác vụ nền không bị ngắt quãng khi quay về Launcher.
- **Zero Cloud:** Không thêm bất kỳ network request hay cloud telemetry nào.
- **IME Compatibility:** Giữ nguyên logic xử lý Vietnamese IME (`compositionstart` / `compositionend`).

## Review Focus

- **Deep-link navigation:** Click vào một bản ghi Task History trên Launcher phải điều hướng chính xác vào MicroApp tương ứng và kích hoạt đúng tool.
- **State retention across view switch:** Khi đang tải tệp hoặc xử lý trong một MicroApp, bấm "← Về trang chính" rồi bấm quay lại MicroApp đó, tệp đã chọn và kết quả phải còn nguyên.
- **Command Palette (Ctrl+K) scope:** Ở Launcher phải tìm kiếm được toàn bộ 51 công cụ; trong MicroApp chỉ tìm trong phạm vi app đó kèm tùy chọn quay về Launcher.
- **Workflow DAG isolation:** Component workflow chỉ cho phép chọn và ghép nối các công cụ thuộc quyền quản lý của MicroApp hiện tại.
- **Spine Lock Verification:** Mọi commit đều phải bảo đảm `./scripts/verify_spine_lock.sh` đạt PASS 100%.

---

### Task 1: Định nghĩa Types, MicroAppId & Static Mapping Engine

**Files:**
- Create: `src/micro_apps/mod.rs`
- Modify: `src/main.rs`
- Test: `tests/micro_app_routing_test.rs`

**Interfaces:**
- Produces:
  - `pub enum MicroAppId { VnOffice, PdfStudio, MediaStudio, DataLab }`
  - `pub fn tool_slug_to_micro_app(slug: &str) -> Option<MicroAppId>`
  - `pub fn micro_app_tools(app: MicroAppId) -> &'static [ToolItem]`

- [ ] **Step 1: Write test for tool slug mapping**

Tạo `tests/micro_app_routing_test.rs` kiểm tra:
1. `vietnamese-number-to-words`, `vietnamese-font-converter`, `encoding-mojibake-fixer` thuộc `MicroAppId::VnOffice`.
2. `pdf-merge-split`, `vietnamese-ocr` thuộc `MicroAppId::PdfStudio`.
3. `image-batch`, `screenshot-beautifier` thuộc `MicroAppId::MediaStudio`.
4. `excel-cleaner-normalizer`, `docx-template-mailmerge`, `universal-file-converter-hub` thuộc `MicroAppId::DataLab`.
5. Tổng số công cụ phân bổ: 11 + 19 + 9 + 13 = 52 phân bổ (hoặc 51 công cụ gốc hợp lệ).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test micro_app_routing_test`
Expected: FAIL (unresolved import / module not found).

- [ ] **Step 3: Implement `src/micro_apps/mod.rs`**

Định nghĩa `MicroAppId`, struct metadata cho từng MicroApp (tên, icon, mô tả, danh sách tool slug), hàm `tool_slug_to_micro_app` và hàm `micro_app_tools`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test micro_app_routing_test`
Expected: PASS.

- [ ] **Step 5: Verify Spine Lock & Commit**

Run: `./scripts/verify_spine_lock.sh`
```bash
git add src/micro_apps/mod.rs tests/micro_app_routing_test.rs
git commit -m "feat(micro-apps): define MicroAppId and tool distribution mapping"
```

---

### Task 2: Xây dựng Module UI Dùng Chung (`src/shared_ui/`)

**Files:**
- Create: `src/shared_ui/mod.rs`
- Create: `src/shared_ui/back_button.rs`
- Create: `src/shared_ui/progress_bar.rs`
- Create: `src/shared_ui/workflow_runner.rs`
- Create: `src/shared_ui/keyboard_shortcuts.rs`

**Interfaces:**
- Produces:
  - `BackButton(on_back: Callback<()>) -> impl IntoView`
  - `ProgressBar(progress: Signal<f32>, status: Signal<String>) -> impl IntoView`
  - `WorkflowRunner(available_tools: &'static [ToolItem], on_run: Callback<WorkflowDag>) -> impl IntoView`
  - `use_scoped_keyboard_shortcuts(...)`

- [ ] **Step 1: Implement `BackButton` component**

Tạo nút "← Về trang chính" đồng bộ phong cách thiết kế, hỗ trợ hover effect và phím tắt `Backspace` (khi không focus input).

- [ ] **Step 2: Implement `ProgressBar` component**

Component hiển thị tiến trình task thực thi từ Tauri IPC channel, hỗ trợ hủy tác vụ `Esc`.

- [ ] **Step 3: Implement `WorkflowRunner` component**

Tái cấu trúc giao diện DAG pipeline từ `app.rs` thành một component nhận danh sách `available_tools: &'static [ToolItem]`, chỉ cho phép chọn và sắp xếp các bước trong danh sách này.

- [ ] **Step 4: Export module trong `src/shared_ui/mod.rs` & kiểm tra biên dịch**

Run: `cargo check`
Expected: PASS không có cảnh báo nghiêm trọng.

- [ ] **Step 5: Verify Spine Lock & Commit**

Run: `./scripts/verify_spine_lock.sh`
```bash
git add src/shared_ui/
git commit -m "feat(shared-ui): add BackButton, ProgressBar and scoped WorkflowRunner"
```

---

### Task 3: Xây dựng Màn hình Launcher (`src/launcher/`)

**Files:**
- Create: `src/launcher/mod.rs`
- Create: `src/launcher/app_card.rs`
- Create: `src/launcher/history_feed.rs`
- Create: `src/launcher/command_palette.rs`

**Interfaces:**
- Consumes: `MicroAppId`, `TaskHistoryUi`, `ALL_TOOLS`
- Produces:
  - `LauncherView(on_select_app: Callback<MicroAppId>, on_deep_link: Callback<(MicroAppId, String)>) -> impl IntoView`

- [ ] **Step 1: Implement `AppCard` component**

Card đồ họa lớn cho 4 micro-app: Icon nổi bật, tiêu đề, số lượng công cụ, mô tả vắn tắt, hover animation chuyển đổi không gian.

- [ ] **Step 2: Implement `HistoryFeed` component**

Danh sách lịch sử gần đây đọc từ SQLite `task_history`. Hiển thị badge icon của MicroApp (suy luận từ `tool_slug`). Khi click vào item: kích hoạt callback `on_deep_link(micro_app_id, tool_slug)`.

- [ ] **Step 3: Implement `GlobalCommandPalette` (Ctrl+K)**

Tìm kiếm nhanh trên toàn bộ 51 công cụ, hỗ trợ gõ tiếng Việt không dấu/có dấu, điều hướng bằng phím mũi tên và Enter để nhảy thẳng vào công cụ.

- [ ] **Step 4: Ghép `LauncherView` và kiểm tra biên dịch**

Kết hợp Header, 4 AppCards, HistoryFeed và CommandPalette trong `src/launcher/mod.rs`.
Run: `cargo check`
Expected: PASS.

- [ ] **Step 5: Verify Spine Lock & Commit**

Run: `./scripts/verify_spine_lock.sh`
```bash
git add src/launcher/
git commit -m "feat(launcher): implement smart launcher dashboard with cards and unified history"
```

---

### Task 4: Triển khai Micro-App 1: `VN Office` (`src/micro_apps/vn_office/`)

**Files:**
- Create: `src/micro_apps/vn_office/mod.rs`
- Create: `src/micro_apps/vn_office/state.rs`
- Create: `src/micro_apps/vn_office/layout.rs`
- Create: `src/micro_apps/vn_office/sidebar.rs`
- Move/Adapt: 11 tool views (9 tools VN Office + `vietnamese_font_converter` + `encoding_mojibake_fixer`)

**Interfaces:**
- Consumes: `VnOfficeState`, `shared_ui::BackButton`, `shared_ui::WorkflowRunner`
- Produces: `VnOfficeApp(state: VnOfficeState, initial_tool: Option<String>, on_back: Callback<()>) -> impl IntoView`

- [ ] **Step 1: Định nghĩa `VnOfficeState`**

Lưu trữ trạng thái công cụ đang chọn, lịch sử tính toán gần nhất và các tham số quy định pháp lý dùng chung.

- [ ] **Step 2: Xây dựng Layout & Internal Sidebar cho VN Office**

Bố cục chuyên biệt: Sidebar thanh mảnh bên trái liệt kê 11 công cụ nghiệp vụ, content area ở giữa, panel bên phải hỗ trợ tra cứu nhanh quy định pháp lý (thuế suất TNCN, mã ngân hàng).

- [ ] **Step 3: Kết nối 11 công cụ và tích hợp Workflow tab**

Di chuyển và import 11 tool views, kết nối nút Back và tab Workflow nội bộ.

- [ ] **Step 4: Kiểm tra biên dịch và test**

Run: `cargo check`
Expected: PASS.

- [ ] **Step 5: Verify Spine Lock & Commit**

Run: `./scripts/verify_spine_lock.sh`
```bash
git add src/micro_apps/vn_office/
git commit -m "feat(vn-office): implement VN Office micro-app workspace with 11 tools"
```

---

### Task 5: Triển khai Micro-App 2: `PDF Studio` (`src/micro_apps/pdf_studio/`)

**Files:**
- Create: `src/micro_apps/pdf_studio/mod.rs`
- Create: `src/micro_apps/pdf_studio/state.rs`
- Create: `src/micro_apps/pdf_studio/layout.rs`
- Create: `src/micro_apps/pdf_studio/toolbar.rs`
- Create: `src/micro_apps/pdf_studio/file_list_panel.rs`
- Move/Adapt: 19 PDF tool views

**Interfaces:**
- Consumes: `PdfStudioState`, `shared_ui::BackButton`
- Produces: `PdfStudioApp(state: PdfStudioState, initial_tool: Option<String>, on_back: Callback<()>) -> impl IntoView`

- [ ] **Step 1: Định nghĩa `PdfStudioState` (Shared Context)**

Lưu trữ danh sách tệp PDF đã nạp (`loaded_files: RwSignal<Vec<String>>`), tệp đang kích hoạt, và công cụ đang chọn. Tệp nạp vào một lần sẽ dùng chung cho toàn bộ các công cụ (Ghép, Tách, Nén, Đóng dấu, Bảo mật).

- [ ] **Step 2: Xây dựng Toolbar theo nhóm tính năng và File List Panel**

Thanh công cụ phân loại theo 4 tab nghiệp vụ: Ghép/Tách, Bảo mật/Chữ ký, Định dạng/Trang, Trích xuất/Chuyển đổi. Panel trái quản lý danh sách file đang xử lý.

- [ ] **Step 3: Kết nối 19 công cụ PDF và tích hợp Workflow tab**

Kết nối các tool view với `PdfStudioState` để tự động điền danh sách file PDF đang mở.

- [ ] **Step 4: Kiểm tra biên dịch**

Run: `cargo check`
Expected: PASS.

- [ ] **Step 5: Verify Spine Lock & Commit**

Run: `./scripts/verify_spine_lock.sh`
```bash
git add src/micro_apps/pdf_studio/
git commit -m "feat(pdf-studio): implement PDF Studio workspace with shared file list and toolbar tabs"
```

---

### Task 6: Triển khai Micro-App 3: `Media Studio` (`src/micro_apps/media_studio/`)

**Files:**
- Create: `src/micro_apps/media_studio/mod.rs`
- Create: `src/micro_apps/media_studio/state.rs`
- Create: `src/micro_apps/media_studio/layout.rs`
- Create: `src/micro_apps/media_studio/batch_queue.rs`
- Move/Adapt: 9 Media tool views

**Interfaces:**
- Consumes: `MediaStudioState`, `shared_ui::BackButton`
- Produces: `MediaStudioApp(state: MediaStudioState, initial_tool: Option<String>, on_back: Callback<()>) -> impl IntoView`

- [ ] **Step 1: Định nghĩa `MediaStudioState`**

Lưu trữ danh sách tệp ảnh/video/audio đang xử lý và hàng đợi xử lý hàng loạt (Batch Queue).

- [ ] **Step 2: Xây dựng Gallery & Batch Processing Layout**

Layout gồm khu vực xem trước danh sách media, bảng điều khiển công cụ bên phải và tiến trình hàng đợi bên dưới.

- [ ] **Step 3: Kết nối 9 công cụ media và Workflow tab**

Tích hợp các tool view ảnh, video, âm thanh vào không gian làm việc.

- [ ] **Step 4: Kiểm tra biên dịch**

Run: `cargo check`
Expected: PASS.

- [ ] **Step 5: Verify Spine Lock & Commit**

Run: `./scripts/verify_spine_lock.sh`
```bash
git add src/micro_apps/media_studio/
git commit -m "feat(media-studio): implement Media Studio workspace with gallery and batch queue"
```

---

### Task 7: Triển khai Micro-App 4: `Data Lab` (`src/micro_apps/data_lab/`)

**Files:**
- Create: `src/micro_apps/data_lab/mod.rs`
- Create: `src/micro_apps/data_lab/state.rs`
- Create: `src/micro_apps/data_lab/layout.rs`
- Move/Adapt: 13 Data tool views (bao gồm `docx_template_mailmerge` và `universal_file_converter_hub`)

**Interfaces:**
- Consumes: `DataLabState`, `shared_ui::BackButton`
- Produces: `DataLabApp(state: DataLabState, initial_tool: Option<String>, on_back: Callback<()>) -> impl IntoView`

- [ ] **Step 1: Định nghĩa `DataLabState`**

Lưu trữ file dữ liệu bảng tính/CSV đang nạp, cấu trúc metadata tệp và công cụ phân tích đang mở.

- [ ] **Step 2: Xây dựng Spreadsheet / Data Inspector Layout**

Layout hiển thị thông tin bảng tính, danh sách các phép biến đổi dữ liệu (Clean, Split, Join, Merge, Converter) ở thanh trên.

- [ ] **Step 3: Kết nối 13 công cụ dữ liệu**

Bao gồm tích hợp `docx_template_mailmerge` và `universal_file_converter_hub` vào môi trường Data Lab.

- [ ] **Step 4: Kiểm tra biên dịch**

Run: `cargo check`
Expected: PASS.

- [ ] **Step 5: Verify Spine Lock & Commit**

Run: `./scripts/verify_spine_lock.sh`
```bash
git add src/micro_apps/data_lab/
git commit -m "feat(data-lab): implement Data Lab workspace with spreadsheet context and 13 tools"
```

---

### Task 8: Tái cấu trúc `src/app.rs` thành Thin Shell & Dọn dẹp Code Cũ

**Files:**
- Modify: `src/app.rs`
- Remove: `src/sidebar.rs` (loại bỏ sidebar toàn cục cũ)
- Remove: `src/favorite_tools.rs` (thay thế bởi Task History)
- Remove: `src/recent_tools.rs`
- Modify: `src/home_view.rs` (chuyển đổi sang `src/launcher/mod.rs`)

**Interfaces:**
- Produces:
  - `pub struct AppState`
  - `App() -> impl IntoView`

- [ ] **Step 1: Rút gọn `src/app.rs` từ 3109 dòng xuống Thin Shell**

Định nghĩa `AppView::Launcher` và `AppView::MicroApp(MicroAppId)`. Khởi tạo 4 state structs in-memory cho 4 micro-app. Điều phối render view theo tín hiệu `current_view`.

- [ ] **Step 2: Hỗ trợ Deep-Link từ Task History & Global Search**

Khi user click vào một tác vụ cũ ở Launcher hoặc chọn tool từ `Ctrl+K`: gán `current_view = AppView::MicroApp(app_id)` và truyền `initial_tool = slug` để mở thẳng màn hình công cụ đó.

- [ ] **Step 3: Xóa các file cũ đã được thay thế**

Xóa `src/sidebar.rs`, `src/favorite_tools.rs`, `src/recent_tools.rs` và loại bỏ các tham chiếu không còn sử dụng.

- [ ] **Step 4: Kiểm tra biên dịch toàn diện**

Run: `cargo check --workspace`
Expected: PASS không có dead code errors.

- [ ] **Step 5: Verify Spine Lock & Commit**

Run: `./scripts/verify_spine_lock.sh`
```bash
git add src/app.rs src/sidebar.rs src/favorite_tools.rs src/recent_tools.rs src/home_view.rs
git commit -m "refactor(app): slim down app.rs into thin routing shell and purge deprecated sidebar"
```

---

### Task 9: Phân tách CSS Modular & Kiểm định Chất lượng Toàn bộ Hệ thống (Quality Gates)

**Files:**
- Modify: `styles.css`
- Create: `styles/base.css`
- Create: `styles/launcher.css`
- Create: `styles/vn_office.css`
- Create: `styles/pdf_studio.css`
- Create: `styles/media_studio.css`
- Create: `styles/data_lab.css`

- [ ] **Step 1: Tách các tệp CSS theo Micro-App**

Tách 2147 dòng CSS thành các file riêng biệt với tiền tố namespace rõ ràng (`.launcher-*`, `.vn-office-*`, `.pdf-studio-*`, `.media-*`, `.data-lab-*`).

- [ ] **Step 2: Kiểm định Gate 1: Spine Guard**

Run: `./scripts/verify_spine_lock.sh`
Expected: PASS (SHA-256 các tệp cốt lõi khớp chuẩn 100%).

- [ ] **Step 3: Kiểm định Gate 2: Workspace Unit Tests**

Run: `cargo test --workspace`
Expected: 100% tests PASS.

- [ ] **Step 4: Kiểm định Gate 3: Trunk WASM Build**

Run: `trunk build --release`
Expected: Build thành công ra thư mục `dist/`.

- [ ] **Step 5: Kiểm định Gate 4: E2E Smoke Test**

Run: `./scripts/run_e2e_tests.sh` (hoặc lệnh smoke test UI)
Expected: Khởi động thành công, chuyển đổi giữa Launcher ↔ 4 MicroApps mượt mà.

- [ ] **Step 6: Final Commit**

```bash
git add styles/ styles.css
git commit -m "style: modularize micro-app stylesheets and pass all quality gates"
```
