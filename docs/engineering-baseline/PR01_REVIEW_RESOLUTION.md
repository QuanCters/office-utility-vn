# Báo Cáo Giải Trình & Khắc Phục Ý Kiến Đánh Giá (PR-01 Review Resolution)
## Office Utility VN — Final Hardening & Safety Architecture Resolution

**Mã tài liệu:** `RESOLUTION-PR01-FINAL-REVIEW`  
**Phiên bản:** 1.0.0  
**Tác giả:** Kỹ sư Trưởng Rust/Tauri & Kiến trúc sư An toàn Hệ thống  
**Ngày hoàn thành:** 23/11/2026  
**Trạng thái:** RESOLVED — SẴN SÀNG TRÌNH DUYỆT (READY FOR STAGE 2 AUTHORIZATION)

---

## 1. TỔNG QUAN GIẢI TRÌNH (EXECUTIVE SUMMARY)

Hội đồng Thẩm định An toàn đã chấp thuận về mặt nguyên tắc định hướng của Safety PR-01, nhưng đưa ra 7 yêu cầu hiệu chỉnh kỹ thuật bắt buộc trước khi được phép chạm vào mã nguồn Khung xương sống (Spine Layer).

Tài liệu này giải trình chi tiết từng nội dung tiếp thu, thiết kế khắc phục và tiêu chí nghiệm thu khách quan tương ứng.

---

## 2. MA TRẬN TIẾP THU & GIẢI QUYẾT 7 NỘI DUNG ĐÁNH GIÁ

| Mục Đánh Giá | Vấn Đề Nhận Diện Ban Đầu | Giải Pháp Khắc Phục Cụ Thể | Tiêu Chí Nghiệm Thu Khách Quan | Tài Liệu Cập Nhật |
| :--- | :--- | :--- | :--- | :--- |
| **1. Tối Giản Hóa Deny-by-Default** | Dự thảo cũ đưa toàn bộ 58 slug vào `commands/mod.rs` (tạo ra bảng đăng ký trùng lặp thứ 2) và dùng `unreachable!()`. | Loại bỏ hoàn toàn danh mục 58 slug trong Spine. Chỉ dùng một **Allowlist tối thiểu** gồm 2 công cụ (`vietnamese-number-to-words`, `vn-cccd-mst-validator`). Toàn bộ slug còn lại rơi vào nhánh fail-closed `unauthorized_slug => Err(ValidationFailed)`. Không dùng `unreachable!()`. | `commands/mod.rs` không chứa danh mục 58 slug; chỉ có 2 slug allowlist. | `PR01_SAFETY_SPEC.md`, `PR01_SPINE_AMENDMENT_RFC.md` |
| **2. Kiểm Chứng Động Cơ Enabled** | Kích hoạt 2 công cụ mà chưa có kiểm thử hành vi độc lập chứng minh kết quả nghiệp vụ thực tế. | Thiết kế bài kiểm tra hợp đồng hành vi chi tiết (`test_06`), đối soát kết quả đọc số và giải mã CCCD với dữ liệu kỳ vọng độc lập; kiểm tra xử lý dữ liệu đầu vào không hợp lệ. | Test 6 khẳng định đúng chữ, đúng định dạng JSON CCCD/MST và bắt lỗi đầu vào sai. | `PR01_TEST_DESIGN.md`, `PR01_FINAL_TEST_MATRIX.md` |
| **3. Kiểm Thử Trực Tiếp Tuyến Sản Xuất** | Test trước đây chỉ kiểm tra hàm phụ `evaluate_tool_readiness()`. | Thiết kế điểm kiểm thử production chung cho cả IPC `dispatch_tool_task`, `execute_workflow_pipeline`, đo lường SHA-256 tệp gốc, kiểm tra không tạo task history ảo và không tạo tệp thừa trên ổ đĩa. | Bài test gọi trực tiếp `dispatch_tool_execution` và kiểm tra SHA-256 tệp thật trên đĩa. | `PR01_TEST_DESIGN.md` |
| **4. Đối Soát Tính Toàn Vẹn Tự Động** | `verify_audit_integrity.py` chỉ so sánh số tổng hardcode. | Bổ sung hàm phân tích AST/regex trích xuất trực tiếp 58 slug từ `src/micro_apps/registry.rs` và 8 slug từ `src-tauri/src/commands/mod.rs`. Bất kỳ slug mới nào xuất hiện hoặc bị thiếu đều làm fail gate. | Bổ sung 14 test harness (bao gồm Negative Test 12 bắt lệch source registration). | `scripts/verify_audit_integrity.py`, `tests/harness/test_audit_integrity.py` |
| **5. Chuẩn Hóa Số Lượng Bài Test Hồi Quy** | Tài liệu ghi 8 integration tests nhưng mã nguồn chỉ có 7 hàm (đếm nhầm mutation test). | Hiện thực hóa đủ 8 hàm Rust integration test độc lập, bổ sung `test_08_execute_workflow_pipeline_fails_closed`. Đồng bộ tên test 100% qua tất cả tài liệu. | Đúng 8 hàm test trong `src-tauri/tests/verification_ipc.rs`. | `PR01_TEST_DESIGN.md`, `PR01_FINAL_TEST_MATRIX.md` |
| **6. Chuẩn Hóa Kiểm Chứng Đột Biến (Mutation Testing)** | Đột biến chỉ hướng dẫn chung chung, chưa đủ 4 ca đột biến biên dịch được trong production code. | Thiết kế 4 ca đột biến cụ thể biên dịch thành công trong worktree cô lập: 1) Wildcard fake success; 2) Bỏ qua kiểm tra unsafe PDF; 3) Fake workflow success; 4) Bắn tiến độ 100% khi fail. Chứng minh RED trước và GREEN sau. | Mỗi ca đột biến đều biên dịch được và làm đúng bài test tương ứng thất bại. | `PR01_TEST_DESIGN.md` |
| **7. Hiệu Đính Hồ Sơ Spine RFC** | RFC thiếu phân định giữa Spine file và PR file; hướng dẫn rollback quay lại mã cũ có lỗ hổng; thiếu cơ chế phê duyệt 2 giai đoạn. | Phân định rõ: Spine file duy nhất là `commands/mod.rs`. Thiết lập cơ chế Kill-Switch an toàn thay vì rollback về mã lỗi. Thiết lập quy trình 2 giai đoạn: Giai đoạn 1 (Duyệt triển khai nhánh cô lập), Giai đoạn 2 (Nghiệm thu và Niêm phong lại khóa). | RFC có chữ ký phê duyệt 2 giai đoạn và cơ chế Kill-Switch fail-closed. | `PR01_SPINE_AMENDMENT_RFC.md`, `PR01_EXECUTION_AUTHORIZATION_REQUEST.md` |

---

## 3. THIẾT KẾ KIẾN TRÚC SINGLE-SOURCE-OF-TRUTH CHO TƯƠNG LAI

Để tránh việc lặp lại định nghĩa công cụ ở cả Leptos frontend và Tauri backend trong tương lai:
1. **Giai đoạn hiện tại (PR-01):** Áp dụng Allowlist tối thiểu (2 công cụ), mọi thứ còn lại tự động fail-closed. Tuyệt đối không sinh ra bảng 58 slug thứ hai.
2. **Lộ trình tương lai (PR-02/PR-03):** 
   - Xây dựng file schema `policy/tool_manifest.json` hoặc khai báo macro dùng chung trong `crates/office-shared`.
   - Frontend Leptos đọc metadata từ manifest để render UI.
   - Backend Tauri đọc manifest để tự động sinh bảng routing IPC.
   - Bảo đảm duy nhất 1 Single Source of Truth cho toàn bộ dự án.

---

## 4. KẾT LUẬN

Mọi thiếu sót kỹ thuật trong đợt đánh giá đã được giải quyết triệt để trên bình diện thiết kế, tài liệu và công cụ tự động hóa. Đội ngũ kiểm định chính thức đệ trình Hồ sơ Yêu cầu Cấp phép Thực thi (Execution Authorization Request) để chuyển sang Giai đoạn 2.
