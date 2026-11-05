# Micro-App Architecture Extensibility & Scaling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Triển khai kiến trúc mở rộng và scale cho hệ sinh thái Micro-Apps của Office Utility VN bao gồm Declarative Tool Registry tập trung, Quản lý Vòng đời & Bộ nhớ AppState, và Type-Safe Task Execution Contract.

**Architecture:** Tách biệt cấu hình metadata công cụ thành một Declarative Registry độc lập (`src/micro_apps/registry.rs`) đóng vai trò Single Source of Truth; bổ sung các phương thức quản lý vòng đời và bộ nhớ đệm tại `src/app.rs`; chuẩn hóa trait `ToolPayloadContract` và cơ chế xác thực validation trước khi dispatch Tauri IPC trong `src/shared_ui/task_executor.rs`.

**Tech Stack:** Rust 2021, Leptos 0.8 (WASM CSR), Tauri v2, Serde/Serde_json.

**Spec:** [docs/superpowers/specs/2026-11-04-micro-app-extensibility-scale-design.md](file:///home/quan/workspace/tauri-v2/office-utility-vn/docs/superpowers/specs/2026-11-04-micro-app-extensibility-scale-design.md)

## Global Constraints

- **Zero Backend Changes:** Tuyệt đối không chỉnh sửa `crates/office-shared`, `src-tauri/src/db`, hay `src-tauri/src/commands`. `./scripts/verify_spine_lock.sh` phải luôn PASS 100% với SHA-256 nguyên vẹn.
- **Zero DB Migration:** Không thêm bảng/cột mới hay thay đổi lược đồ SQLite.
- **100% Backward Compatibility:** Giữ nguyên các hàm public và struct fields hiện có (`tool_slug_to_micro_app`, `micro_app_tools`, `MicroAppId::tool_count`, `AppState::vn_office_state`, v.v.) để bảo đảm 568 tests hiện tại tiếp tục pass mà không sửa 1 dòng code test cũ nào.
- **Zero Cloud:** Không thêm bất kỳ network request hay cloud telemetry nào.
- **WASM CSR Safe:** Mã nguồn UI và shared_ui phải biên dịch sạch trên target `wasm32-unknown-unknown`.

## Review Focus

- **Count & Slug Sync:** Khi thêm một công cụ mới vào Registry, `MicroAppId::tool_count()`, `tools()`, và `lookup_app()` phải tự động phản ánh chính xác mà không cần chỉnh sửa số đếm thủ công ở bất kỳ file nào khác.
- **Multi-App Tool Visibility:** Một công cụ được gắn `secondary_apps` phải xuất hiện hợp lệ trong danh sách công cụ của cả app chính và app phụ.
- **State Active Work Detection:** `has_active_work()` phải nhận diện chính xác khi người dùng có lịch sử tính toán, hàng đợi batch ảnh, hoặc tệp đang nạp trong từng micro-app.
- **Payload Validation Interception:** `TaskExecutor` với contract validation không hợp lệ phải chặn việc gọi Tauri IPC `invoke("dispatch_tool_task", ...)` và hiển thị thông báo lỗi rõ ràng.
- **Spine Lock Integrity:** Chạy `./scripts/verify_spine_lock.sh` sau mỗi task để bảo đảm không có mã backend nào bị vô tình chỉnh sửa.

---

### Task 1: Declarative Tool Registry Module

**Files:**
- Create: `office-utility-vn/src/micro_apps/registry.rs`
- Modify: `office-utility-vn/src/micro_apps/mod.rs`
- Test: `office-utility-vn/tests/micro_app_registry_test.rs`

- [x] **Step 1: Viết test thất bại cho Tool Registry**
  Tạo `office-utility-vn/tests/micro_app_registry_test.rs` kiểm tra:
  - Toàn bộ 51 công cụ được đăng ký đầy đủ trong `ToolRegistry::ENTRIES`.
  - `ToolRegistry::lookup_app(slug)` trả về đúng `MicroAppId` cho cả 51 slugs.
  - `ToolRegistry::count_for_app(app)` khớp chính xác: VN Office (11), PDF Studio (18), Media Studio (9), Data Lab (13).
  - Tra cứu slug không tồn tại trả về `None`.
  - Hỗ trợ công cụ phụ thuộc nhiều app qua `secondary_apps`.
  - Hàm API cũ `tool_slug_to_micro_app()` và `micro_app_tools()` trả về kết quả tương thích 100% với registry.

- [x] **Step 2: Chạy test để xác nhận test thất bại**
  ```bash
  cd office-utility-vn && cargo test --test micro_app_registry_test
  ```

- [x] **Step 3: Hiện thực `src/micro_apps/registry.rs`**
  - Định nghĩa struct `ToolRegistration`:
    ```rust
    use crate::app::ToolItem;
    use crate::micro_apps::MicroAppId;

    #[derive(Debug, Clone)]
    pub struct ToolRegistration {
        pub tool: ToolItem,
        pub primary_app: MicroAppId,
        pub secondary_apps: &'static [MicroAppId],
        pub tags: &'static [&'static str],
    }
    ```
  - Định nghĩa `ToolRegistry` chứa mảng tĩnh `ENTRIES: &'static [ToolRegistration]` hợp nhất toàn bộ 51 công cụ hiện tại.
  - Hiện thực các phương thức:
    - `pub fn lookup_app(slug: &str) -> Option<MicroAppId>`
    - `pub fn tools_for_app(app: MicroAppId) -> Vec<ToolItem>`
    - `pub fn count_for_app(app: MicroAppId) -> usize`
    - `pub fn all_tools() -> Vec<ToolItem>`
    - `pub fn tools_by_tag(tag: &str) -> Vec<ToolItem>`

- [x] **Step 4: Tái cấu trúc `src/micro_apps/mod.rs` để ủy quyền sang `ToolRegistry`**
  - Khai báo `pub mod registry; pub use registry::{ToolRegistration, ToolRegistry};`
  - Cập nhật `MicroAppId::metadata()` sử dụng `ToolRegistry::count_for_app(self)` thay vì hardcoded số.
  - Cập nhật `tool_slug_to_micro_app(slug: &str)` ủy quyền cho `ToolRegistry::lookup_app(slug)`.
  - Cập nhật `micro_app_tools(app: MicroAppId)` trả về slice tĩnh hoặc tham chiếu tương thích từ `ToolRegistry`.

- [x] **Step 5: Chạy lại bài test và kiểm tra toàn bộ test suite**
  ```bash
  cd office-utility-vn && cargo test --test micro_app_registry_test && cargo test --test micro_app_routing_test
  ```

- [x] **Step 6: Xác nhận Spine Lock & Commit**
  ```bash
  ./scripts/verify_spine_lock.sh
  git add src/micro_apps/ tests/micro_app_registry_test.rs
  git commit -m "refactor(micro_apps): introduce declarative ToolRegistry as single source of truth"
  ```

---

### Task 2: Quản lý Vòng đời & Bộ nhớ State trong `AppState`

**Files:**
- Modify: `office-utility-vn/src/app.rs:438-510`
- Test: `office-utility-vn/tests/app_lifecycle_test.rs`

- [ ] **Step 1: Viết test thất bại cho tính năng quản lý vòng đời AppState**
  Tạo `office-utility-vn/tests/app_lifecycle_test.rs` kiểm tra:
  - `has_active_work(MicroAppId)` ban đầu trả về `false` cho tất cả micro-apps.
  - Khi thêm lịch sử vào `vn_office_state.calculation_history`, `has_active_work(MicroAppId::VnOffice)` trả về `true`.
  - Khi thêm file vào `pdf_studio_state.files`, `has_active_work(MicroAppId::PdfStudio)` trả về `true`.
  - Khi thêm batch item vào `media_studio_state.batch_queue`, `has_active_work(MicroAppId::MediaStudio)` trả về `true`.
  - Khi thêm file vào `data_lab_state.files`, `has_active_work(MicroAppId::DataLab)` trả về `true`.
  - Phương thức `reset_app_state(MicroAppId)` đặt lại trạng thái của app về mặc định.
  - Phương thức `hibernate_inactive_apps(active_app)` dọn dẹp các cache không dùng của các app đang ẩn.

- [ ] **Step 2: Chạy test để xác nhận test thất bại**
  ```bash
  cd office-utility-vn && cargo test --test app_lifecycle_test
  ```

- [ ] **Step 3: Hiện thực các phương thức vòng đời trong `AppState` tại `src/app.rs`**
  - Bổ sung `pub fn has_active_work(&self, app_id: MicroAppId) -> bool`:
    - VN Office: kiểm tra `calculation_history` không rỗng.
    - PDF Studio: kiểm tra `files` không rỗng.
    - Media Studio: kiểm tra `batch_queue` không rỗng hoặc `files` không rỗng.
    - Data Lab: kiểm tra `files` không rỗng.
  - Bổ sung `pub fn reset_app_state(&self, app_id: MicroAppId)`:
    - Khởi tạo lại state của app được chọn về giá trị default.
  - Bổ sung `pub fn hibernate_inactive_apps(&self, active_app: MicroAppId)`:
    - Duyệt qua các app không phải `active_app`. Nếu app không có `has_active_work()`, dọn dẹp các tài nguyên đệm nhẹ.

- [ ] **Step 4: Chạy lại bài test và kiểm tra toàn bộ suite liên quan đến routing**
  ```bash
  cd office-utility-vn && cargo test --test app_lifecycle_test && cargo test --test app_routing_test
  ```

- [ ] **Step 5: Xác nhận Spine Lock & Commit**
  ```bash
  ./scripts/verify_spine_lock.sh
  git add src/app.rs tests/app_lifecycle_test.rs
  git commit -m "feat(app): add state lifecycle and memory management methods to AppState"
  ```

---

### Task 3: Type-Safe Task Execution Contract trong `shared_ui`

**Files:**
- Modify: `office-utility-vn/src/shared_ui/task_executor.rs`
- Test: `office-utility-vn/tests/task_executor_contract_test.rs`

- [x] **Step 1: Viết test thất bại cho Task Execution Contract & Validation**
  Tạo `office-utility-vn/tests/task_executor_contract_test.rs` kiểm tra:
  - Trait `ToolPayloadContract` với các struct `EmptyToolPayload` và `GenericToolPayload`.
  - Hàm `validate()` trả về `Err(reason)` khi dữ liệu không hợp lệ.
  - Hàm `validate()` trả về `Ok(())` khi dữ liệu hợp lệ.
  - Hàm `to_payload()` sinh ra `serde_json::Value` chính xác.
  - Kiểm tra hàm trợ giúp `validate_payload_before_dispatch` chặn thực thi khi validation thất bại.

- [x] **Step 2: Chạy test để xác nhận test thất bại**
  ```bash
  cd office-utility-vn && cargo test --test task_executor_contract_test
  ```

- [x] **Step 3: Hiện thực `ToolPayloadContract` và tích hợp vào `src/shared_ui/task_executor.rs`**
  - Định nghĩa trait:
    ```rust
    pub trait ToolPayloadContract {
        fn validate(&self) -> Result<(), String> { Ok(()) }
        fn to_payload(&self) -> serde_json::Value;
    }
    ```
  - Triển khai `EmptyToolPayload`:
    ```rust
    #[derive(Debug, Clone, Default)]
    pub struct EmptyToolPayload;
    impl ToolPayloadContract for EmptyToolPayload {
        fn to_payload(&self) -> serde_json::Value { serde_json::json!({}) }
    }
    ```
  - Triển khai `GenericToolPayload`:
    ```rust
    #[derive(Debug, Clone)]
    pub struct GenericToolPayload {
        pub data: serde_json::Value,
        pub validator: Option<fn(&serde_json::Value) -> Result<(), String>>,
    }
    ```
  - Bổ sung prop tùy chọn vào component `TaskExecutor`:
    `validator: Option<Callback<serde_json::Value, Result<(), String>>>`.
  - Trong logic click chạy tác vụ của `TaskExecutor`: nếu có validator và validation trả về `Err(msg)`, cập nhật ngay `status_message.set(format!("⚠️ Lỗi tham số: {msg}"))` và không gọi `invoke("dispatch_tool_task", ...)`.

- [x] **Step 4: Chạy lại bài test Task Executor**
  ```bash
  cd office-utility-vn && cargo test --test task_executor_contract_test && cargo test --test shared_ui_test
  ```

- [x] **Step 5: Xác nhận Spine Lock & Commit**
  ```bash
  ./scripts/verify_spine_lock.sh
  git add src/shared_ui/task_executor.rs tests/task_executor_contract_test.rs
  git commit -m "feat(shared_ui): implement ToolPayloadContract and pre-dispatch validation in TaskExecutor"
  ```

---

### Task 4: Toàn Diện Quality Gates & Đóng Gói (Whole-System Verification)

**Files:**
- Toàn bộ workspace

- [ ] **Step 1: Kiểm tra biên dịch WASM (Frontend Leptos)**
  ```bash
  cd office-utility-vn && cargo check --target wasm32-unknown-unknown
  ```

- [ ] **Step 2: Chạy toàn bộ test suite (Toàn bộ 568+ tests)**
  ```bash
  cd office-utility-vn && cargo test --workspace
  ```

- [ ] **Step 3: Xác minh Khung xương sống (Spine Guard Lock)**
  ```bash
  cd office-utility-vn && ./scripts/verify_spine_lock.sh
  ```

- [ ] **Step 4: Kiểm tra Trunk build release**
  ```bash
  cd office-utility-vn && trunk build --release
  ```

- [ ] **Step 5: Commit tổng kết hoàn thiện**
  ```bash
  git commit --allow-empty -m "chore: complete micro-app extensibility and scaling refactor (Option A)"
  ```
