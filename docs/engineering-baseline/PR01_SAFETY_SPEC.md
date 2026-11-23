# Đặc Tả Kỹ Thuật Gói Thay Đổi An Toàn Số 01 (Corrected Safety PR-01 Spec)
## Office Utility VN — Minimal Allowlist Deny-by-Default & Production Gate Hardening

**Mã tài liệu:** `SPEC-PR01-SAFETY-GATE-HARDENING-V3`  
**Phiên bản:** 3.0.0 (Bản Hiệu đính Hoàn chỉnh — Tối giản Hóa Kiến trúc Deny-by-Default)  
**Tác giả:** Kỹ sư Trưởng Rust/Tauri, Kiến trúc sư An toàn & Trưởng nhóm Kiểm định Độc lập  
**Trạng thái:** HOÀN THIỆN THIẾT KẾ (CHỜ PHÊ DUYỆT GIAI ĐOẠN 1 TỪ MAINTAINER)  
**Mục tiêu tối thượng:** Chặn đứng 100% các đường dẫn sinh kết quả `SUCCESS` giả mạo, phong tỏa mọi công cụ PDF không an toàn và Fake Engines trực tiếp, thiết lập chính sách Deny-by-Default tối giản không trùng lặp và bảo vệ tệp gốc của người dùng.

---

## 1. NGUYÊN TẮC THIẾT KẾ TỐI GIẢN (SIMPLIFIED DENY-BY-DEFAULT)

Theo chỉ đạo từ Hội đồng Đánh giá Kỹ thuật:
1. **Tuyệt đối không đưa danh mục 58 slug vào `commands/mod.rs`:**
   Việc đưa toàn bộ 58 slug vào tầng Spine vừa tạo ra một bảng đăng ký trùng lặp với `src/micro_apps/registry.rs`, vừa làm phình to mã nguồn và vi phạm nguyên lý Zero AI Slop.
2. **Áp dụng Minimal Allowlist:**
   Tầng backend chỉ duy trì một danh sách trắng tối thiểu gồm các công cụ đã hoàn thiện thuật toán và được kiểm chứng hợp đồng hành vi (`ENABLED_TOOL_ALLOWLIST`).
   Toàn bộ mọi slug khác—bất kể là slug lạ, unrouted, fake engine hay unsafe PDF—đều tự động rơi vào nhánh `unauthorized_slug => Err(CommandError::ValidationFailed)`.
3. **Không dùng `unreachable!()`:**
   Mọi luồng thực thi sản xuất đều có nhánh xử lý lỗi rõ ràng, không bao giờ gây panic tiến trình.

---

## 2. NỘI DUNG SỬA ĐỔI MÃ NGUỒN SẢN XUẤT CỤ THỂ

### 2.1. Tệp Khung Xương Sống: `src-tauri/src/commands/mod.rs`

#### A. Khai Báo Allowlist Tối Thiểu:
```rust
/// Danh sách trắng tối thiểu các công cụ đã được kiểm định hành vi và kích hoạt (Minimal Allowlist)
pub const ENABLED_TOOL_ALLOWLIST: &[&str] = &[
    "vietnamese-number-to-words",
    "vn-cccd-mst-validator",
];

/// Kiểm tra công cụ có nằm trong danh sách được phép thực thi hay không
pub fn is_tool_enabled(slug: &str) -> bool {
    ENABLED_TOOL_ALLOWLIST.contains(&slug)
}
```

#### B. Hàm Điều Phối Cốt Lõi Sản Xuất (`dispatch_tool_execution`):
```rust
/// Hàm điều phối cốt lõi sản xuất (Core Production Dispatcher)
/// Dùng chung cho cả IPC dispatch_tool_task và Integration Tests
pub fn dispatch_tool_execution(
    request: &DispatchTaskRequest,
    cancel_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    progress_callback: Option<&dyn Fn(TaskProgress)>,
) -> Result<DispatchTaskResponse, CommandError> {
    let slug = request.tool_slug.as_str();

    // Deny-by-Default: Chỉ thực thi nếu nằm trong Allowlist tối thiểu
    match slug {
        "vietnamese-number-to-words" => {
            let tool = VietnameseNumberToWordsTool::default();
            tool.execute(request, cancel_flag, progress_callback)
        }
        "vn-cccd-mst-validator" => {
            let tool = VnCccdMstValidatorTool::default();
            tool.execute(request, cancel_flag, progress_callback)
        }
        unauthorized_slug => {
            Err(CommandError::ValidationFailed {
                reason: format!(
                    "Công cụ '{}' chưa khả dụng hoặc đang bị khóa bảo vệ an toàn nhị phân.",
                    unauthorized_slug
                ),
            })
        }
    }
}
```

#### C. Lệnh IPC `dispatch_tool_task`:
```rust
pub async fn dispatch_tool_task(
    _app_handle: AppHandle,
    state: State<'_, AppState>,
    request: DispatchTaskRequest,
    on_progress: tauri::ipc::Channel<TaskProgress>,
) -> Result<DispatchTaskResponse, CommandError> {
    let task_id = request.task_id.clone();
    let cancel_flag = state.register_task(&task_id);

    let req_clone = request.clone();
    let cancel_clone = std::sync::Arc::clone(&cancel_flag);

    let progress_sender = on_progress.clone();
    let progress_callback = Box::new(move |p: TaskProgress| {
        let _ = progress_sender.send(p);
    });

    let result = tauri::async_runtime::spawn_blocking(move || {
        dispatch_tool_execution(&req_clone, &cancel_clone, Some(&*progress_callback))
    })
    .await
    .map_err(|e| CommandError::ProcessExecutionError {
        message: format!("Lỗi runtime luồng nền: {e}"),
    })?;

    result
}
```

#### D. Lệnh IPC `execute_workflow_pipeline` (Khóa Fail-Closed):
```rust
/// Hàm thực thi workflow cốt lõi (Core Workflow Execution)
pub fn execute_workflow_execution(
    workflow_id: &str,
    _input_paths: Vec<String>,
    _output_directory: Option<String>,
    _on_progress: Option<&dyn Fn(TaskProgress)>,
) -> Result<DispatchTaskResponse, CommandError> {
    // CHỐT AN TOÀN PR-01: Chặn đứng việc sinh SUCCESS giả mạo cho workflow
    Err(CommandError::ValidationFailed {
        reason: format!(
            "Quy trình tự động hóa Workflow '{workflow_id}' tạm thời bị khóa cho đến khi hoàn tất kiểm định an toàn từng node."
        ),
    })
}

pub async fn execute_workflow_pipeline(
    _app_handle: AppHandle,
    _state: State<'_, AppState>,
    workflow_id: String,
    input_paths: Vec<String>,
    output_directory: Option<String>,
    on_progress: tauri::ipc::Channel<TaskProgress>,
) -> Result<DispatchTaskResponse, CommandError> {
    let progress_sender = on_progress.clone();
    let progress_callback = Box::new(move |p: TaskProgress| {
        let _ = progress_sender.send(p);
    });

    execute_workflow_execution(&workflow_id, input_paths, output_directory, Some(&*progress_callback))
}
```

---

### 2.2. Các Động Cơ Giả Mạo trong `crates/office-engine`: Chèn Chốt Chặn
Tại các tệp `vietnamese_ocr.rs`, `video_to_gif_webp.rs`, `pdf_to_long_image.rs`, `pdf_pki_sign.rs`, `pdf_protect_sign.rs`, hàm `execute` được cập nhật chốt chặn ngay từ dòng đầu tiên:
```rust
return Err(CommandError::ValidationFailed {
    reason: "Động cơ này đã bị tạm khóa bảo vệ an toàn nhị phân.".to_string(),
});
```

---

## 3. THIẾT KẾ ĐỘI NGŨ TEST VÀ TIÊU CHÍ NGHIỆM THU

Chi tiết toàn bộ 8 bài test tích hợp và 4 ca đột biến được quy định tại [PR01_TEST_DESIGN.md](file:///home/quan/workspace/tauri-v2/office-utility-vn/docs/engineering-baseline/PR01_TEST_DESIGN.md).

---

## 4. QUY TRÌNH QUẢN TRỊ KHÓA XƯƠNG SỐNG

Toàn bộ quy trình tu chính tuân thủ hồ sơ [PR01_SPINE_AMENDMENT_RFC.md](file:///home/quan/workspace/tauri-v2/office-utility-vn/docs/engineering-baseline/PR01_SPINE_AMENDMENT_RFC.md) với quy trình phê duyệt 2 giai đoạn.
