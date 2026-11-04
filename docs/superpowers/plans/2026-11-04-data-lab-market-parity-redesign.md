# Data Lab Market Parity & Hybrid Layout Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mở rộng Data Lab với 3 tính năng nâng cao chuẩn thị trường (Data Profiling, Fuzzy Clustering, Recipe History Engine) và tái thiết kế giao diện theo 3 kiểu layout phổ biến (Data Grid + Facet Sidebar, Split-Pane Master-Detail, Visual Workflow Canvas).

**Architecture:** 
- Tầng Engine (`crates/office-engine/src/data_lab/`): Bổ sung các công cụ `DataProfilerTool`, `FuzzyClusteringTool`, và `RecipeEngine` tuân thủ trait `OfficeTool`, xử lý song song qua `rayon`, không egress mạng, bọc an toàn lỗi `CommandError`.
- Tầng UI (`src/micro_apps/data_lab/`): Tái cấu trúc layout theo ngữ cảnh tương tác:
  1. `SpreadsheetLayout`: Data Grid kết hợp Facet Sidebar & Profiling Histogram từng cột (mô hình OpenRefine).
  2. `SplitPaneLayout`: Input / Output song song với chuyển đổi trực tiếp real-time (mô hình DevToys / CyberChef).
  3. `WorkflowCanvasLayout`: Sơ đồ Node & Dây nối luồng trực quan cho Macro DAG (mô hình KNIME / Alteryx).

**Tech Stack:** Rust (1.80+), Leptos 0.8 (CSR WASM), Rayon, Serde JSON, Calamine/Umpteen (Excel reading), Tauri v2.

**Spec:** Market Analysis & Feature Parity Spec (OpenRefine, DevToys, KNIME, Localfile)

## Global Constraints
- `crates/office-shared`: KHÔNG sửa đổi (Spine Read-Only Guard).
- `src-tauri/src/db`: KHÔNG sửa đổi.
- `src-tauri/src/commands`: KHÔNG sửa đổi.
- 100% On-Device & Zero Network Egress (NFR-SEC-01).
- Non-blocking UI: mọi tác vụ tính toán nặng chạy trên luồng nền qua `rayon`/`tokio`.
- `./scripts/verify_spine_lock.sh` và `make verify-ui` phải vượt qua sau khi hoàn tất.

## Review Focus
1. Tệp bảng tính rỗng hoặc chứa dữ liệu đặc thù (chỉ có dòng tiêu đề hoặc không có dòng nào): Profiler và Fuzzy Clustering phải xử lý an toàn không panic.
2. Tiếng Việt có dấu phức tạp (Unicode tổ hợp vs dựng sẵn NFC): Fuzzy clustering phải chuẩn hóa NFC trước khi tính khoảng cách chuỗi.
3. Bảng dữ liệu lớn (> 50,000 dòng): Profiler phải xử lý song song qua `par_iter()`, giao diện Data Grid phải hỗ trợ phân trang ảo (virtualization/pagination).
4. Phản hồi thời gian thực trên Split-pane: Tránh vòng lặp kích hoạt re-render vô tận khi nhập liệu ở cột Input.
5. Cờ hủy tác vụ (`AtomicBool`): Mọi tác vụ nặng dừng trong < 100ms khi người dùng bấm Hủy.

---

## Task Structure

### Phase 1: Engine Features (Data Profiling, Fuzzy Clustering, Recipe History)

#### Task 1: `DataProfilerTool` - Phân tích Hồ sơ Dữ liệu Cột (OpenRefine Parity)
**Files:**
- Create: `crates/office-engine/src/data_lab/data_profiler.rs`
- Modify: `crates/office-engine/src/data_lab/mod.rs`
- Test: `tests/data_profiler_test.rs` hoặc unit tests trong `data_profiler.rs`

- [ ] Viết test cho thuật toán phân tích cột (tổng dòng, null count, null percentage, data type infer, min/max, top 5 value frequency histogram).
- [ ] Chạy `cargo test -p office-engine data_profiler` để xác nhận test đỏ (fail).
- [ ] Hiện thực hóa `DataProfilerTool` cài đặt trait `OfficeTool`.
- [ ] Chạy `cargo test -p office-engine data_profiler` xác nhận test xanh (pass).
- [ ] Commit thay đổi: `feat(data-lab): implement DataProfilerTool for column metrics & histograms`.

#### Task 2: `FuzzyClusteringTool` - Gom cụm Dữ liệu Mờ & Chuẩn hóa Lỗi Chính tả (OpenRefine Parity)
**Files:**
- Create: `crates/office-engine/src/data_lab/fuzzy_clustering.rs`
- Modify: `crates/office-engine/src/data_lab/mod.rs`
- Test: Unit tests trong `fuzzy_clustering.rs`

- [ ] Viết test thuật toán Levenshtein distance kết hợp fingerprint chuỗi tiếng Việt (vd gom cụm "Hà Nội", "Ha Noi", "Hanoi").
- [ ] Chạy `cargo test -p office-engine fuzzy_clustering` xác nhận test đỏ.
- [ ] Hiện thực hóa `FuzzyClusteringTool` cài đặt trait `OfficeTool`: tìm các nhóm tương đồng, cho phép áp dụng bảng thay thế (cluster merge).
- [ ] Chạy test xác nhận xanh.
- [ ] Commit thay đổi: `feat(data-lab): implement FuzzyClusteringTool for Vietnamese typo deduplication`.

#### Task 3: `RecipeHistoryEngine` - Time-travel & Replay Biến đổi (Undo/Redo)
**Files:**
- Create: `crates/office-engine/src/data_lab/recipe_history.rs`
- Modify: `crates/office-engine/src/data_lab/mod.rs`
- Test: Unit tests trong `recipe_history.rs`

- [ ] Viết test ghi nhận các bước biến đổi dữ liệu thành JSON recipe và replay trên tập dữ liệu khác.
- [ ] Chạy test xác nhận đỏ.
- [ ] Hiện thực hóa cấu trúc `DataRecipe`, `RecipeStep`, `RecipeHistoryEngine`.
- [ ] Chạy test xác nhận xanh.
- [ ] Commit thay đổi: `feat(data-lab): implement RecipeHistoryEngine for time-travel & workflow replay`.

---

### Phase 2: UI Hybrid Layout Redesign (OpenRefine, DevToys, KNIME Patterns)

#### Task 4: Layout 1 - `SpreadsheetGridFacetView` (Mô hình OpenRefine)
**Files:**
- Create: `src/micro_apps/data_lab/spreadsheet_grid_view.rs`
- Modify: `src/micro_apps/data_lab/mod.rs`
- Modify: `styles/data_lab.css`

- [ ] Thiết kế component hiển thị dữ liệu bảng có phân trang mượt mà.
- [ ] Thêm header bar hiển thị thanh Profiling Histogram (tỉ lệ Null/Valid/Invalid) cho từng cột.
- [ ] Tích hợp Facet Sidebar bên cạnh: lọc theo giá trị, thống kê tần suất, nút gọi nhanh "Gom cụm Fuzzy" và "Làm sạch ô".
- [ ] Tích hợp thanh công cụ Undo/Redo & Export Recipe JSON.

#### Task 5: Layout 2 - `SplitPaneConverterView` (Mô hình DevToys / CyberChef)
**Files:**
- Create: `src/micro_apps/data_lab/split_pane_view.rs`
- Modify: `src/micro_apps/data_lab/mod.rs`
- Modify: `styles/data_lab.css`

- [ ] Thiết kế component Split-pane 2 cột (Left: Input với format selector / Right: Output với Copy & Download buttons).
- [ ] Tích hợp thanh điều khiển trung tâm: Nút Swap (⇄), Real-time live conversion toggle, Clear button.
- [ ] Gắn kết với các công cụ Text & Cấu trúc (`data-structure-converter`, `base64-file-codec`, `sql-formatter-linter`, `diff-checker-text-sheets`).

#### Task 6: Layout 3 - `VisualCanvasWorkflowView` (Mô hình KNIME / Alteryx)
**Files:**
- Create: `src/micro_apps/data_lab/visual_canvas_workflow.rs`
- Modify: `src/micro_apps/data_lab/mod.rs`
- Modify: `styles/data_lab.css`

- [ ] Thiết kế giao diện Visual Canvas hiển thị các node xử lý của chuỗi Macro DAG.
- [ ] Hiển thị trực quan luồng dữ liệu (Input File Node -> Transform Nodes -> Output File Node).
- [ ] Hỗ trợ cấu hình từng node và nút Chạy / Dừng liên hoàn.

#### Task 7: Tích hợp Toàn diện vào `DataLabLayout`, `DataLabToolbar` & `DataLabState`
**Files:**
- Modify: `src/micro_apps/data_lab/layout.rs`
- Modify: `src/micro_apps/data_lab/toolbar.rs`
- Modify: `src/micro_apps/data_lab/state.rs`
- Modify: `src/micro_apps/mod.rs`

- [ ] Cập nhật `DataLabState` để hỗ trợ chế độ xem (Spreadsheet Grid, Split-pane, Visual Workflow) và Recipe History.
- [ ] Nối toàn bộ view mới vào `DataLabLayout`.
- [ ] Chạy `./scripts/verify_spine_lock.sh` và `cargo test -p office-engine` và `make verify-ui`.
- [ ] Commit hoàn tất: `feat(data-lab): complete market parity features and hybrid layout redesign`.
