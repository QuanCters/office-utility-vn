# Thiết Kế Kiến Trúc Mở Rộng & Scale Cho Hệ Thống Micro-Apps (Option A)

- **Ngày ban hành:** 2026-11-04
- **Trạng thái:** DRAFT (Chờ phê duyệt)
- **Tác giả:** Antigravity Architect
- **Quy trình áp dụng:** `08_quy_trinh_van_hanh.md` (Kịch bản `spine-upgrade` / `refactor` — VÙNG VÀNG `YELLOW`)

---

## 1. Bối cảnh & Mục tiêu (Context & Goals)

### 1.1. Bối cảnh
Hệ thống Office Utility VN đã hoàn tất chuyển đổi sang 4 Micro-Apps độc lập (VN Office, PDF Studio, Media Studio, Data Lab) cùng Launcher trung tâm. Tuy nhiên, theo đánh giá 5 quyết định kỹ thuật:
1. Các ánh xạ công cụ và danh sách công cụ đang dùng `match` thủ công và mảng tĩnh rải rác trong `src/micro_apps/mod.rs`.
2. Khi mở rộng sang công cụ thứ 52+, lập trình viên phải sửa đổi đồng thời 4 vị trí (enum, mảng công cụ, match arm phân loại, và `tool_count` cố định).
3. Toàn bộ form input và signal tệp đang nằm thường trực trong bộ nhớ RAM mà chưa có cơ chế kiểm soát vòng đời (Lifecycle/Hibernation).
4. `TaskExecutor` nhận `Callback<(), serde_json::Value>` vô danh, thiếu cơ chế xác thực ràng buộc tham số (Payload Contract Validation) trước khi gửi qua Tauri IPC.

### 1.2. Mục tiêu (Goals)
- **G1 (Declarative Tool Registry):** Xây dựng một nguồn chân lý duy nhất (Single Source of Truth) dạng Declarative Registry. Việc thêm công cụ mới chỉ cần khai báo một lần trong bảng đăng ký; các hàm `tool_slug_to_micro_app()`, `micro_app_tools()`, và `tool_count()` đều được tự động suy luận.
- **G2 (Lifecycle & Memory Control):** Bổ sung cơ chế quản lý vòng đời bộ nhớ vào `AppState` (`hibernate_inactive_states`, `clear_unfocused_buffers`), bảo toàn khả năng tương thích 100% với các test hiện tại.
- **G3 (Type-Safe Task Execution Contract):** Thiết lập trait `ToolPayloadContract` với cơ chế `validate()` tự động trên Frontend trước khi `invoke` lệnh IPC sang Tauri, bắt sớm lỗi tham số.
- **G4 (Zero Backend / Zero DB Regression):** Giữ nguyên 100% backend (SHA-256 Spine Lock) và toàn bộ 568 bài test hiện có phải tiếp tục PASS.

### 1.3. Phi mục tiêu (Non-Goals)
- Không can thiệp sửa đổi backend native trong `crates/office-shared`, `src-tauri/src/db`, hay `src-tauri/src/commands`.
- Không thay đổi giao diện UI bên ngoài đối với người dùng cuối.

---

## 2. Thiết Kế Kiến Trúc Chi Tiết (Detailed Architectural Design)

### 2.1. Module Declarative Tool Registry (`src/micro_apps/registry.rs`)

Thay vì bảo trì song song các mảng tĩnh và hàm `match` thủ công, tạo cấu trúc `ToolRegistry`:

```rust
use crate::app::ToolItem;
use crate::micro_apps::MicroAppId;

pub struct ToolRegistration {
    pub tool: ToolItem,
    pub primary_app: MicroAppId,
    pub secondary_apps: &'static [MicroAppId], // Cho phép một công cụ hiển thị ở nhiều app
    pub tags: &'static [&'static str],
}

pub struct ToolRegistry;

impl ToolRegistry {
    /// Bảng đăng ký khai báo toàn bộ công cụ của hệ sinh thái
    pub const ENTRIES: &'static [ToolRegistration] = &[
        // VN Office (11 công cụ)
        ToolRegistration {
            tool: ToolItem {
                slug: "vietnamese-number-to-words",
                name: "Đọc Số Tiền Thành Chữ Chuẩn",
                domain: "vn-office",
                description: "Chuyển đổi số tiền thành chữ tiếng Việt chuẩn xác theo quy tắc kế toán.",
                is_pinned: true,
            },
            primary_app: MicroAppId::VnOffice,
            secondary_apps: &[],
            tags: &["ke-toan", "tien-te", "phap-ly"],
        },
        // ... các công cụ khác
    ];

    /// Tra cứu Micro-App chính từ slug
    pub fn lookup_app(slug: &str) -> Option<MicroAppId> {
        Self::ENTRIES.iter().find(|e| e.tool.slug == slug).map(|e| e.primary_app)
    }

    /// Lấy danh sách công cụ thuộc về một Micro-App cụ thể
    pub fn tools_for_app(app: MicroAppId) -> Vec<ToolItem> {
        Self::ENTRIES.iter()
            .filter(|e| e.primary_app == app || e.secondary_apps.contains(&app))
            .map(|e| e.tool.clone())
            .collect()
    }

    /// Đếm số lượng công cụ tự động (không sợ lệch số hardcoded)
    pub fn count_for_app(app: MicroAppId) -> usize {
        Self::ENTRIES.iter()
            .filter(|e| e.primary_app == app || e.secondary_apps.contains(&app))
            .count()
    }
}
```

*Tính tương thích ngược (Backward Compatibility):*
Các hàm API công khai hiện có tại `src/micro_apps/mod.rs` (`tool_slug_to_micro_app`, `micro_app_tools`, `MicroAppId::tool_count`) sẽ ủy quyền (delegate) trực tiếp sang `ToolRegistry`, đảm bảo không làm gãy bất kỳ mã nguồn hay bài test nào.

---

### 2.2. Quản lý Vòng đời & Bộ nhớ State (`src/app.rs`)

Để chuẩn bị cho hệ thống mở rộng lên hàng chục Micro-Apps và hàng trăm công cụ:

```rust
impl AppState {
    /// Đánh dấu Micro-App đang hoạt động và dọn dẹp các vùng nhớ đệm tạm của các app không dùng nếu cần
    pub fn hibernate_inactive_apps(&self, active_app: MicroAppId) {
        // Ví dụ: Giải phóng danh sách tệp đệm lớn ở các app đang ẩn để tiết kiệm RAM
        match active_app {
            MicroAppId::VnOffice => {
                // Giữ nguyên input form nhưng có thể dọn bộ nhớ đệm preview media/pdf nếu cần
            }
            MicroAppId::PdfStudio => {}
            MicroAppId::MediaStudio => {}
            MicroAppId::DataLab => {}
        }
    }

    /// Kiểm tra trạng thái đã được người dùng thao tác hay chưa
    pub fn has_active_work(&self, app_id: MicroAppId) -> bool {
        match app_id {
            MicroAppId::VnOffice => !self.vn_office_state.calculation_history.get_untracked().is_empty(),
            MicroAppId::PdfStudio => !self.pdf_studio_state.files.get_untracked().is_empty(),
            MicroAppId::MediaStudio => !self.media_studio_state.batch_queue.get_untracked().is_empty(),
            MicroAppId::DataLab => !self.data_lab_state.files.get_untracked().is_empty(),
        }
    }
}
```

---

### 2.3. Type-Safe Task Execution Contract (`src/shared_ui/task_executor.rs`)

Thêm trait định nghĩa giao ước tham số thực thi:

```rust
pub trait ToolPayloadContract {
    /// Kiểm tra tính hợp lệ của tham số trước khi gửi sang Tauri IPC
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
    /// Đóng gói thành JSON Value tương thích với backend
    fn to_payload(&self) -> serde_json::Value;
}

/// Triển khai mặc định cho trường hợp không có tham số mở rộng (Empty Payload)
#[derive(Debug, Clone, Default)]
pub struct EmptyToolPayload;

impl ToolPayloadContract for EmptyToolPayload {
    fn to_payload(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}

/// Triển khai cho Generic JSON Payload kèm validator tùy chọn
#[derive(Debug, Clone)]
pub struct GenericToolPayload {
    pub data: serde_json::Value,
    pub validator: Option<fn(&serde_json::Value) -> Result<(), String>>,
}

impl ToolPayloadContract for GenericToolPayload {
    fn validate(&self) -> Result<(), String> {
        if let Some(v) = self.validator {
            v(&self.data)
        } else {
            Ok(())
        }
    }
    fn to_payload(&self) -> serde_json::Value {
        self.data.clone()
    }
}
```

Nâng cấp `TaskExecutor` để hỗ trợ cả 2 cách:
1. `options_builder: Option<Callback<(), serde_json::Value>>` (giữ nguyên tương thích cũ).
2. `payload_contract: Option<Box<dyn ToolPayloadContract>>` hoặc callback trả về `Result<serde_json::Value, String>` để hiển thị lỗi validation ngay lập tức trên UI mà không cần gọi IPC.

---

## 3. Kế hoạch Kiểm Thử & Nghiệm Thu (Verification Gates)

1. **Gate 1 - Spine Lock:** `./scripts/verify_spine_lock.sh` phải trả về `100% khớp chuẩn`.
2. **Gate 2 - Unit & Integration Tests:** Chạy `cargo test --workspace`, 100% toàn bộ 568 tests phải PASS.
3. **Gate 3 - Thêm Test mới cho Extensibility:**
   - Test thêm công cụ động qua `ToolRegistry`.
   - Test `ToolRegistry::count_for_app` khớp chính xác số công cụ.
   - Test `ToolPayloadContract::validate()` bắt lỗi trước khi gọi IPC.
   - Test `AppState::has_active_work()` và `hibernate_inactive_apps()`.
4. **Gate 4 - WASM Compilation:** `cargo check --target wasm32-unknown-unknown` PASS.
