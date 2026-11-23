# Phiếu Yêu Cầu Cấp Phép Thực Thi PR-01 (PR-01 Execution Authorization Request)
## Chuyển Tiếp Từ Giai Đoạn 1 Thiết Kế Sang Giai Đoạn 2 Triển Khai Mã Nguồn
### Office Utility VN — Safety Gate Hardening & Two-Stage Authorization Protocol

**Mã tài liệu:** `AUTH-REQ-2026-001-PR01`  
**Phiên bản:** 1.0.0  
**Ngày đệ trình:** 23/11/2026  
**Người đệ trình:** Kỹ sư Trưởng Rust/Tauri & Trưởng nhóm Kiểm định Độc lập  
**Người nhận (Cấp thẩm quyền):** Hội đồng Quản trị Kho mã nguồn (Repository Maintainers — `@office-utility-vn/maintainers`)  
**Tham chiếu hồ sơ:**
- `docs/engineering-baseline/PR01_SPINE_AMENDMENT_RFC.md` (v2.0.0)
- `docs/engineering-baseline/PR01_SAFETY_SPEC.md` (v3.0.0)
- `docs/engineering-baseline/PR01_TEST_DESIGN.md` (v3.0.0)
- `docs/engineering-baseline/PR01_FINAL_TEST_MATRIX.md` (v3.0.0)
- `docs/engineering-baseline/PR01_REVIEW_RESOLUTION.md` (v1.0.0)
- `docs/engineering-baseline/PR01_ACCEPTANCE_CHECKLIST.md` (v3.0.0)

---

## 1. TÓM TẮT ĐỆ TRÌNH (EXECUTIVE SUMMARY)

Kính gửi Hội đồng Quản trị Kho mã nguồn,

Trưởng nhóm Kiểm định Độc lập và Kỹ sư An toàn trân trọng đệ trình yêu cầu **Cấp phép Chuyển tiếp từ Giai đoạn 1 (Thiết kế & Kiểm chứng Sẵn sàng) sang Giai đoạn 2 (Triển khai Mã nguồn Thực tế)** cho gói giải pháp an toàn **PR-01 (Safety Gate Hardening)**.

Trong Giai đoạn 1, toàn bộ các yêu cầu hiệu đính kỹ thuật từ Hội đồng Đánh giá đã được giải quyết triệt để:
1. **Đơn giản hóa Deny-by-Default:** Loại bỏ hoàn toàn ý tưởng đưa 58 slug vào `commands/mod.rs`. Thiết lập **Allowlist tối thiểu chỉ gồm 2 công cụ đã kiểm chứng** (`ENABLED_TOOL_ALLOWLIST`). Không dùng `unreachable!()` trên các luồng thực thi bên ngoài.
2. **Kiểm định Hành vi Thực tế cho Công cụ Enabled:** Đã thiết kế các bài kiểm thử hợp đồng hành vi độc lập cho `vietnamese-number-to-words` và `vn-cccd-mst-validator`, đối soát với oracle độc lập và bắt lỗi chuỗi đầu vào không hợp lệ. Khẳng định: Chưa công cụ nào được coi là "Release Eligible" (đủ điều kiện phát hành sản phẩm thương mại) cho đến khi hoàn tất kiểm toán giao diện IME.
3. **Kiểm định Tuyến Sản Xuất Thực Tế (Không Dùng Test Mocks):** Xác lập điểm kết nối sản xuất chung `dispatch_tool_execution` và `execute_workflow_execution` được gọi trực tiếp bởi IPC command, Workflow pipeline và bộ kiểm thử tích hợp `src-tauri/tests/verification_ipc.rs`.
4. **Kiểm Chứng Toàn Vẹn Tự Động Bằng AST/Regex:** Công cụ `scripts/verify_audit_integrity.py` đã được nâng cấp để quét trực tiếp cây AST của `src/micro_apps/registry.rs` và `src-tauri/src/commands/mod.rs`. Bộ test harness gồm 14 unit tests độc lập đã kiểm chứng mọi chế độ bắt lỗi tiêu cực (negative tests).
5. **Thiết kế 8 Bài Kiểm Thử Tích Hợp & 4 Ca Đột Biến Biên Dịch Được:** Đã xây dựng hoàn chỉnh mã nguồn Rust cho 8 bài kiểm thử tích hợp (bao gồm `execute_workflow_pipeline_fails_closed`) và 4 ca kiểm chứng đột biến chứng minh khả năng bắt lỗi RED/GREEN.

**Trạng thái hiện tại:** Tầng Khung Xương Sống (`crates/office-shared`, `src-tauri/src/db`, `src-tauri/src/commands`) **VẪN NGUYÊN VẸN 100% VÀ ĐANG ĐƯỢC NIÊM PHONG BỞI KHÓA CHỈ ĐỌC (`verify_spine_lock.sh`)**. Chưa có bất kỳ dòng code sản xuất nào trong Khung Xương Sống bị can thiệp khi chưa có văn bản phê duyệt này.

---

## 2. BẰNG CHỨNG XÁC MINH GIAI ĐOẠN 1 (STAGE 1 VERIFICATION EVIDENCE)

Toàn bộ các lệnh kiểm thử và kiểm tra tính toàn vẹn đã được thực thi và vượt qua 100%:

```text
1. Kiểm tra Tính toàn vẹn Dữ liệu Kiểm toán (Audit Integrity):
   Command: python3 scripts/verify_audit_integrity.py
   Result:  ✓ 58 tools registered in UI registry
            ✓ 8 tools currently routed in commands/mod.rs
            ✓ 18 findings in findings_register.json (10 P0, 5 P1, 3 P2)
            ✓ 18 evidence entries in AUDIT_EVIDENCE_MANIFEST.json
            ✓ 0 tools release eligible
            Status: 100% SYNCHRONIZED & PASS

2. Bộ Kiểm thử Tiêu cực & Đơn vị Cho Harness (Test Harness Suite):
   Command: python3 -m unittest discover -s tests/harness
   Result:  Ran 47 tests in 0.563s
            Status: OK (100% PASS)

3. Xác thực Niêm phong Khung Xương Sống (Spine Lock):
   Command: ./scripts/verify_spine_lock.sh
   Result:  ✓ crates/office-shared [MATCH: 26a8aed8...]
            ✓ src-tauri/src/db      [MATCH: 38bcd07c...]
            ✓ src-tauri/src/commands [MATCH: ca12b1ad5586453d6753bda21cb31213333c8f13f2d621b6d00f5bdc2228b1a0]
            Status: ALL UNIT TESTS PASSED - SPINE UNTOUCHED
```

---

## 3. PHẠM VI THỰC THI GIAI ĐOẠN 2 (STAGE 2 EXECUTION SCOPE)

Khi nhận được sự phê duyệt từ Maintainers, nhóm thực thi sẽ triển khai theo đúng quy trình cô lập:

1. **Khởi tạo nhánh cô lập:** `git checkout -b feature/pr01-safety-gate-hardening`.
2. **Tu chính tệp Khung Xương Sống duy nhất:** `src-tauri/src/commands/mod.rs`:
   - Định nghĩa `const ENABLED_TOOL_ALLOWLIST: &[&str] = &["vietnamese-number-to-words", "vn-cccd-mst-validator"];`.
   - Chuyển `dispatch_tool_task` ủy quyền sang `dispatch_tool_execution`.
   - Áp dụng Deny-by-Default: mọi slug nằm ngoài allowlist đều trả về `Err(CommandError::ValidationFailed)`.
   - Vô hiệu hóa `execute_workflow_pipeline`, trả về lỗi Fail-Closed rõ ràng.
3. **Thêm tệp kiểm thử tích hợp sản xuất:** `src-tauri/tests/verification_ipc.rs` chứa 8 bài kiểm thử tích hợp và 2 bí danh hợp đồng.
4. **Chèn chốt chặn an toàn trong các động cơ giả mạo:** `vietnamese_ocr.rs`, `video_to_gif_webp.rs`, `pdf_to_long_image.rs`, `pdf_pki_sign.rs`, `pdf_protect_sign.rs`.
5. **Thực thi kiểm chứng 4 ca đột biến (Mutation Testing)** để ghi nhận bằng chứng RED/GREEN thực tế.
6. **Thực hiện Quy trình Tái Niêm Phong Khung Xương Sống (Spine Re-sealing):** Tính toán SHA-256 mới của `src-tauri/src/commands/mod.rs`, cập nhật `scripts/verify_spine_lock.sh`, và chạy kiểm tra xác thực.

---

## 4. ĐÁNH GIÁ RỦI RO & PHƯƠNG ÁN THU HỒI KHẨN CẤP (ROLLBACK & KILL-SWITCH)

- **Rủi ro vận hành:** 50 công cụ unrouted và 6 công cụ routed không an toàn/giả mạo sẽ bị từ chối rõ ràng trên giao diện UI thay vì trả về `SUCCESS` giả mạo. Đây là hành vi đúng đắn theo thiết kế bảo vệ an toàn dữ liệu người dùng.
- **Cơ chế Kill-Switch tức thì:**
  Nếu phát hiện bất kỳ lỗi hồi quy ngoài ý muốn, hệ thống hỗ trợ biến môi trường khẩn cấp:
  ```bash
  export OFFICE_UTILITY_EMERGENCY_FAIL_CLOSED=1
  ```
  giúp khóa cứng toàn bộ IPC dispatcher mà không cần biên dịch lại mã nguồn.
- **Kế hoạch thu hồi sạch sẽ (Safe Rollback):**
  Nhánh làm việc là nhánh riêng biệt `feature/pr01-safety-gate-hardening`. Nếu có sự cố, việc hủy bỏ nhánh không làm ảnh hưởng đến nhánh chính (`main`).

---

## 5. BIÊN BẢN KÝ DUYỆT CẤP PHÉP (MAINTAINER SIGN-OFF)

Hội đồng Quản trị Kho mã nguồn vui lòng xác nhận các mục sau để cấp quyền thực thi Stage 2:

- [ ] **Điều kiện 1:** Đã thẩm duyệt và chấp thuận thiết kế Allowlist tối thiểu trong `PR01_SAFETY_SPEC.md` v3.0.0.
- [ ] **Điều kiện 2:** Đã xác nhận tính đầy đủ của 8 bài kiểm thử tích hợp và 4 ca đột biến trong `PR01_TEST_DESIGN.md` và `PR01_FINAL_TEST_MATRIX.md`.
- [ ] **Điều kiện 3:** Đã thẩm tra báo cáo toàn vẹn tự động `scripts/verify_audit_integrity.py` đạt 100% PASS.
- [ ] **Điều kiện 4:** Cho phép sửa đổi có kiểm soát tệp Khung Xương Sống `src-tauri/src/commands/mod.rs` trên nhánh `feature/pr01-safety-gate-hardening`.
- [ ] **Điều kiện 5:** Cho phép tái tính toán mã băm SHA-256 và cập nhật `scripts/verify_spine_lock.sh` sau khi bộ test tích hợp và test đột biến đạt 100% GREEN.

---

**Quyết định của Maintainers:**  
[ ] **CHẤP THUẬN (APPROVED) — Cho phép bắt đầu Giai đoạn 2**  
[ ] **YÊU CẦU ĐIỀU CHỈNH THÊM (CHANGES REQUESTED)**  

*Chữ ký điện tử của Maintainer:* __________________________________  
*Ngày ký duyệt:* ______________________
