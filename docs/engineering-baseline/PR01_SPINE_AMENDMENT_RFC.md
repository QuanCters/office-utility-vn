# Hồ Sơ Yêu Cầu Tu Chính Khung Xương Sống (Authorized Spine Amendment RFC)
## Office Utility VN — Safety Gate Hardening & Two-Stage Authorization Protocol

**Mã RFC:** `RFC-SPINE-2026-001-PR01`  
**Phiên bản:** 2.0.0 (Bản Hiệu đính Toàn diện & Phân định Ranh giới Khóa)  
**Trạng thái:** DRAFT / PENDING STAGE 1 AUTHORIZATION (Chờ Cấp phép Triển khai Nhánh Cô lập)  
**Tác giả đề xuất:** Kỹ sư Trưởng Rust/Tauri & Kiến trúc sư An toàn  
**Người có quyền phê duyệt:** Repository Maintainers (`@office-utility-vn/maintainers` theo `.github/CODEOWNERS`)  
**Mục tiêu sửa đổi:** Thay thế nhánh wildcard phát tán `SUCCESS` giả mạo, chặn đứng 4 công cụ PDF không an toàn và vô hiệu hóa đường dẫn workflow giả lập trong tầng Khung Xương Sống (`src-tauri/src/commands/mod.rs`) theo nguyên lý Deny-by-Default tối giản.

---

## 1. PHÂN ĐỊNH RANH GIỚI BẢO VỆ KHUNG XƯƠNG SỐNG (SCOPE BOUNDARY)

Để bảo đảm tính minh bạch tuyệt đối, RFC này phân định rạch ròi giữa tệp Khung xương sống được bảo vệ bởi khóa băm SHA-256 và các tệp mã nguồn thông thường được tạo mới/sửa đổi trong PR:

### 1.1. Tệp Khung Xương Sống Được Bảo Vệ (Protected Spine Files):
- **`crates/office-shared/src/lib.rs`:** Khóa mã băm `26a8aed8...` -> **KHÔNG THAY ĐỔI**.
- **`src-tauri/src/db/mod.rs`:** Khóa mã băm `38bcd07c...` -> **KHÔNG THAY ĐỔI**.
- **`src-tauri/src/commands/mod.rs`:** Khóa mã băm `ca12b1ad5586453d6753bda21cb31213333c8f13f2d621b6d00f5bdc2228b1a0` -> **TỆP DUY NHẤT ĐƯỢC ĐỀ XUẤT TU CHÍNH**.

### 1.2. Các Tệp Thông Thường Khác trong PR (Unprotected PR Files):
- `src-tauri/tests/verification_ipc.rs`: Tệp kiểm thử tích hợp sản xuất mới (8 integration tests).
- `crates/office-engine/src/doc_suite/*.rs`: Chèn chốt chặn an toàn cho các động cơ giả mạo (`vietnamese_ocr.rs`, `pdf_to_long_image.rs`, `pdf_pki_sign.rs`, `pdf_protect_sign.rs`).
- `crates/office-engine/src/media_studio/video_to_gif_webp.rs`: Chèn chốt chặn an toàn cho động cơ giả mạo gradient video.
- `scripts/verify_audit_integrity.py` & `tests/harness/test_audit_integrity.py`: Công cụ kiểm định tính toàn vẹn dữ liệu.
- `docs/engineering-baseline/*`: Các tài liệu kiến trúc, kiểm toán và báo cáo.

---

## 2. NGUYÊN NHÂN GỐC RỄ & CÁC PHÁT HIỆN KIỂM TOÁN ĐƯỢC XỬ LÝ

Bản tu chính này giải quyết dứt điểm các lỗ hổng an toàn dữ liệu nghiêm trọng nhất:
- **`FINDING-P0-IPC-01`:** Nhánh wildcard `_ =>` trong `dispatch_tool_task` phát sự kiện tiến độ giả 50% rồi 100% và trả về `status: "SUCCESS"` kèm tệp đầu vào cho 50 slug chưa định tuyến.
- **`FINDING-P0-FAKE-01` & `FINDING-P0-FAKE-03`:** Hai công cụ giả mạo (`vietnamese-ocr` và `pdf-to-long-image`) đang được match trực tiếp trong `dispatch_tool_task`.
- **`FINDING-P0-PDF-01` (Nhóm Routed):** Bốn công cụ PDF thao tác chuỗi UTF-8 lossy (`pdf-booklet-imposition`, `pdf-grayscale-print`, `pdf-repair-corrupt`, `pdf-table-extractor`) đang được match trực tiếp và có nguy cơ phá hủy luồng nhị phân tài liệu của người dùng.
- **Lỗ hổng Workflow Entry Point:** Hàm `execute_workflow_pipeline` trả về `SUCCESS` và input paths mà không chạy bất kỳ node nào.

---

## 3. THIẾT KẾ TU CHÍNH TỐI GIẢN (MINIMAL ALLOWLIST DENY-BY-DEFAULT)

Theo khuyến nghị của Hội đồng Đánh giá, **tuyệt đối không đưa danh mục 58 slug vào `commands/mod.rs`** (tránh tạo ra bảng đăng ký trùng lặp thứ 2 và tránh AI Slop).

Thay vào đó, áp dụng một **Allowlist tối thiểu duy nhất**:
```rust
/// Danh sách trắng tối thiểu các công cụ đã được kiểm định hành vi và kích hoạt (Minimal Verified Allowlist)
pub const ENABLED_TOOL_ALLOWLIST: &[&str] = &[
    "vietnamese-number-to-words",
    "vn-cccd-mst-validator",
];

pub fn is_tool_enabled(slug: &str) -> bool {
    ENABLED_TOOL_ALLOWLIST.contains(&slug)
}
```

Mọi slug khác (dù là unknown, unrouted, fake, hay unsafe PDF) đều tự động rơi vào nhánh fail-closed:
```rust
pub fn dispatch_tool_execution(
    request: &DispatchTaskRequest,
    cancel_flag: &Arc<AtomicBool>,
    progress_callback: Option<&dyn Fn(TaskProgress)>,
) -> Result<DispatchTaskResponse, CommandError> {
    let slug = request.tool_slug.as_str();
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
**Cam kết:** Không sử dụng `unreachable!()` trên bất kỳ luồng thực thi sản xuất nào.

---

## 4. TÁC ĐỘNG HÀNH VI ĐỐI VỚI NGƯỜI DÙNG (USER-FACING BEHAVIORAL IMPACT)

1. **Khi chạy 50 công cụ unrouted hoặc 4 công cụ PDF không an toàn:**
   - *Hành vi cũ:* Ứng dụng báo "Hoàn tất thành công" sau 2ms; tệp kết quả thực chất là tệp gốc chưa hề được xử lý, gây mất dữ liệu hoặc hỏng quy trình kế toán.
   - *Hành vi mới:* Ứng dụng hiển thị thông báo lỗi tiếng Việt rõ ràng: `"Công cụ '...' chưa khả dụng hoặc đang bị khóa bảo vệ an toàn nhị phân."`
   - *Đánh giá tác động:* Đây là sự thay đổi tích cực, trung thực và bảo vệ người dùng tuyệt đối.
2. **Khi chạy chuỗi Workflow Pipeline:**
   - *Hành vi cũ:* Báo hoàn thành chuỗi 100% nhưng không hề chạy node công cụ nào.
   - *Hành vi mới:* Trả về lỗi `"Quy trình tự động hóa Workflow '...' tạm thời bị khóa cho đến khi hoàn tất kiểm định an toàn từng node."`

---

## 5. QUY TRÌNH PHÊ DUYỆT HAI GIAI ĐOẠN (TWO-STAGE AUTHORIZATION PROTOCOL)

Để tuân thủ nghiêm ngặt nguyên tắc quản trị Spine Lock, quy trình phê duyệt được chia thành 2 giai đoạn bắt buộc:

### Giai đoạn 1: Cấp Phép Triển Khai Nhánh Cô Lập (Authorization to Implement in Isolated Branch)
- Maintainer ký duyệt cho phép checkout nhánh `feature/pr01-safety-gate-hardening`.
- Kỹ sư được phép thêm các bài test hồi quy thất bại (**RED**) và áp dụng bản vá tối thiểu để đưa test về đạt chuẩn (**GREEN**).
- **Quy tắc:** Trong suốt Giai đoạn 1, **tuyệt đối không sửa đổi mã băm trong `scripts/verify_spine_lock.sh`**.

### Giai đoạn 2: Nghiệm Thu Bản Vá & Tái Niêm Phong Khóa (Final Verification & Spine Resealing)
- Kỹ sư trình báo cáo bằng chứng RED/GREEN, kết quả 8 integration tests và 4 mutation tests.
- Maintainer thực hiện kiểm toán độc lập trên diff của `src-tauri/src/commands/mod.rs`.
- Sau khi được Maintainer ký duyệt Giai đoạn 2:
  1. Tính mã băm SHA-256 mới của `src-tauri/src/commands/mod.rs`.
  2. Cập nhật biến `EXPECTED_CMD` trong `scripts/verify_spine_lock.sh`.
  3. Chạy `./scripts/verify_spine_lock.sh` xác nhận `PASS 100%`.

---

## 6. CHIẾN LƯỢC HOÀN NGUYÊN AN TOÀN & KILL-SWITCH (SAFE ROLLBACK STRATEGY)

> [!CAUTION]
> **Loại bỏ Rollback Nguy hiểm:** Tuyệt đối không hoàn nguyên bằng cách `git checkout origin/main` vì nhánh `main` hiện tại đang chứa lỗ hổng phát tán `SUCCESS` giả mạo!

Hệ thống thiết lập **Chiến lược Hoàn nguyên An toàn 2 Lớp (Two-Tier Safe Fallback)**:
1. **Lớp 1 — Emergency Kill-Switch (Không cần Build lại Binary):**
   Hệ thống hỗ trợ biến môi trường `OFFICE_UTILITY_EMERGENCY_FAIL_CLOSED=1`. Khi cờ này được bật, toàn bộ các lệnh IPC `dispatch_tool_task` tự động từ chối 100% yêu cầu mà không gọi bất kỳ engine nào.
2. **Lớp 2 — Hoàn nguyên Atomic Git Revert:**
   Nếu bản vá PR-01 cần thu hồi, Maintainer revert commit của PR-01 về một commit đã được khóa fail-closed trước đó, bảo đảm ứng dụng không bao giờ quay lại trạng thái phát tán thành công ảo.

---

## 7. MÃ BĂM KHÓA XƯƠNG SỐNG (CRYPTOGRAPHIC SPINE HASH AUDIT)

| Tệp Khóa Bảo Vệ | Trạng Thái | Mã Băm SHA-256 Hiện Tại (Trusted) | Trạng Thái Cập Nhật |
| :--- | :---: | :---: | :---: |
| `crates/office-shared/src/lib.rs` | Khóa | `26a8aed81fceb200951f4581f80635b42f3344186d3cb0a7c84d1016629e32c6` | Giữ nguyên vẹn |
| `src-tauri/src/db/mod.rs` | Khóa | `38bcd07c7358503b46664e407ed6d84e641661d82a95a428299d15f61df0c5d1` | Giữ nguyên vẹn |
| `src-tauri/src/commands/mod.rs` | **Tu chính** | `ca12b1ad5586453d6753bda21cb31213333c8f13f2d621b6d00f5bdc2228b1a0` | **CHƯA ĐƯỢC PHÉP THAY ĐỔI** (Chờ Duyệt Giai đoạn 2) |

---

## 8. BIÊN BẢN PHÊ DUYỆT (AUTHORIZATION SIGN-OFF)

### Giai đoạn 1: Cấp Phép Triển Khai Nhánh Cô Lập (Branch Implementation)
- [ ] Maintainer 1 (`@maintainer-security`): ............................................ Ngày: ....../....../2026
- [ ] Maintainer 2 (`@maintainer-core`): ................................................ Ngày: ....../....../2026

### Giai đoạn 2: Phê Duyệt Nghiệm Thu & Tái Niêm Phong Khóa (Spine Resealing)
- [ ] Bằng chứng 8 Integration Tests đạt 100% (GREEN).
- [ ] Bằng chứng 4 Mutation Tests bắt lỗi chính xác (RED).
- [ ] Bằng chứng SHA-256 tệp gốc không đổi.
- [ ] Maintainer 1 Ký duyệt Niêm phong lại Khóa: ..................................... Ngày: ....../....../2026
- [ ] Maintainer 2 Ký duyệt Niêm phong lại Khóa: ..................................... Ngày: ....../....../2026
