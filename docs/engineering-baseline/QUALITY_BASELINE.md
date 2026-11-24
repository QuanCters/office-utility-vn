# ĐẶC TẢ CHUẨN MỰC CHẤT LƯỢNG KỸ THUẬT (QUALITY BASELINE)
**Dự án:** Office Utility VN (Desktop Application)  
**Phiên bản kiểm toán:** Commit SHA `8c57f9655bb0841064973effd6ddeb72c0fa3599`  
**Ngày xác lập:** 22/11/2026  
**Thẩm quyền:** Principal Software Architect & Verification Lead  

---

## 1. DỮ LIỆU ĐO KIỂM THỰC TẾ TẠI THỜI ĐIỂM KIỂM TOÁN (AUDITED EVIDENCE)

| Tiêu chuẩn / Lệnh kiểm định | Lệnh thực thi chính xác | Kết quả đo kiểm | Ý nghĩa & Đánh giá kỹ thuật |
| :--- | :--- | :---: | :--- |
| **Git Working Tree** | `git status --short` | 5 Modified, 15 Untracked | Có uncommitted changes liên quan đến bản vá lỗi unwrap và zlib limit. Không bị ghi đè. |
| **Git Commit SHA** | `git rev-parse HEAD` | `8c57f9655bb0841064973effd6ddeb72c0fa3599` | Điểm neo kỹ thuật cố định làm cơ sở đối soát toàn bộ báo cáo này. |
| **Rust Toolchain** | `rustc --version; cargo --version` | `rustc 1.99.0`, `cargo 1.99.0` | Rust 2024 Edition. |
| **Định dạng mã nguồn** | `cargo fmt --all -- --check` | ❌ **FAILED** | Phát hiện sai lệch định dạng trên diện rộng tại `benches/`, `crates/office-engine/`, `src/`, `tests/`. |
| **Kiểm tra tĩnh Rust** | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | ❌ **FAILED** | 97 lỗi cảnh báo (64 lỗi trong `office-engine`, 33 lỗi trong `office-utility-vn-ui`). |
| **Bộ kiểm thử nội bộ** | `cargo test --workspace --all-targets` | 🟢 **PASSED** | 588 tests pass (379 tests trong `office-engine`, 209 tests trong `office-utility-vn-ui`, 0 failed). Tuy nhiên đây là kết quả của Test Mock Mirroring. |
| **Tự kiểm thử Harness** | `python3 -m unittest discover -s tests/harness -v` | 🟢 **PASSED** | 34/34 tests vượt qua trong 0.48s; chứng minh các script kiểm tra guardrails hoạt động chính xác. |
| **Guardrails (PR Mode)** | `python3 scripts/verify_guardrails.py --repo . --mode incremental` | 🟢 **PASSED** | Thoát với exit code 0 vì toàn bộ 201 vi phạm đã được cấp ngoại lệ tạm thời trong `policy/approved_legacy_findings.json`. |
| **Guardrails (Strict)** | `python3 scripts/verify_guardrails.py --repo . --mode enforce` | ❌ **FAILED** | Bắt được **201 lỗi vi phạm** (99 PDF001, 29 PDF002, 43 IME001, 16 FS001, 3 PDF003, 2 FAKE001, 1 FAKE002, 2 FAKE003, 2 VNI001, 1 PNG002, 1 IO001, 1 IO002, 1 IPC001). |
| **Cổng Kiểm thử Nguồn** | `python3 scripts/verify_source_test_inventory.py --repo .` | ❌ **FAILED** | Thất bại do thiếu toàn bộ 17/17 tệp kiểm thử hợp đồng nguồn bắt buộc. |
| **Cổng Kiểm tra Hành vi** | `python3 scripts/verify_behavior.py --repo .` | ❌ **FAILED** | Thất bại do các ca kiểm thử ở trạng thái `TODO` và thiếu binary adapter `office_verify_adapter`. |
| **Khóa Khung Xương Sống** | `./scripts/verify_spine_lock.sh` | 🟢 **PASSED** | 3 tệp thuộc tầng Phase 0 khớp đúng SHA-256 chuẩn. (Tuy nhiên đang niêm phong lỗi P0 wildcard). |

---

## 2. CÁC HỢP ĐỒNG CHẤT LƯỢNG BẮT BUỘC (NON-NEGOTIABLE INVARIANTS)

Bất kỳ mã nguồn nào muốn được đưa vào Baseline được chứng nhận bắt buộc phải thỏa mãn 10 hợp đồng bất biến:

1. **INV-01 (Honest Execution - Thực thi trung thực):**
   - Nghiêm cấm mọi hình thức trả về `SUCCESS` mà không thực sự gọi động cơ xử lý.
   - Công cụ chưa hoàn thiện hoặc bị vô hiệu hóa bắt buộc phải trả về lỗi `CommandError::ValidationFailed` rõ ràng.
2. **INV-02 (Source Preservation - Bảo vệ tệp gốc):**
   - Tuyệt đối không ghi đè lên tệp nguồn đầu vào nếu không có sự ủy quyền rõ ràng từ người dùng.
   - Nghiêm cấm trả về đường dẫn tệp đầu vào làm đường dẫn tệp đầu ra khi chưa xử lý.
3. **INV-03 (Valid Output - Tính hợp lệ đầu ra):**
   - Mọi tệp PDF, ảnh, âm thanh, bảng tính đầu ra bắt buộc phải vượt qua các công cụ kiểm định độc lập của bên thứ ba (`qpdf --check`, `pdfinfo`, `pdftotext`, `pdfsig -nocert`, `pdftoppm`).
4. **INV-04 (Binary Safety - An toàn nhị phân):**
   - Cấm sử dụng `String::from_utf8_lossy` trên dữ liệu PDF nhị phân. Cấm tìm đối tượng bằng chuỗi `" obj"` / `"endobj"`.
   - Phải sao chép trọn vẹn đồ thị đối tượng phụ thuộc (Transitive Resource Closure) khi thao tác trang.
5. **INV-05 (Resource Bounds - Kiểm soát tài nguyên):**
   - Mọi thao tác giải nén (Zlib, Zip) phải có ngân sách bộ nhớ tối đa có kiểm tra vượt ngưỡng (`take(LIMIT + 1)`).
   - Kiểm tra kích thước pixel và bộ đệm trước khi cấp phát bộ nhớ bằng phép tính có kiểm tra tràn số `checked_mul`.
6. **INV-06 (Error Integrity - Toàn vẹn xử lý lỗi):**
   - Nghiêm cấm nuốt lỗi hệ thống tệp bằng `let _ = fs::rename` hoặc `.unwrap_or(0)`.
   - Lỗi đọc/ghi phải được chuyển đổi thành `CommandError::IoError` có ngữ cảnh cụ thể.
7. **INV-07 (Vietnamese IME Correctness - An toàn bộ gõ tiếng Việt):**
   - Các trường nhập liệu Leptos không được ngắt quãng quá trình gõ dấu của UniKey/EVKey.
   - Bắt buộc xử lý sự kiện `compositionstart` và `compositionend` để bảo toàn con trỏ và dấu tiếng Việt.
8. **INV-08 (Security & Cryptography - Bảo mật và Mật mã học):**
   - Không được giả lập chữ ký số bằng chuỗi text thô. Chữ ký số phải tuân thủ chuẩn PKCS#7 / CMS DER ASN.1.
   - Không được dùng phép XOR đối xứng thay cho mã hóa AES.
   - Mật khẩu phải được xóa sạch khỏi bộ nhớ Heap bằng `zeroize`.
9. **INV-09 (Independent Oracle - Thẩm định độc lập):**
   - Dữ liệu kỳ vọng trong test suite bắt buộc phải lấy từ các tệp fixture thực tế có nguồn gốc độc lập (Word, Acrobat, Scan), không được lấy từ chính output của Engine đang test.
10. **INV-10 (Fail-Closed Verification - Kiểm định đóng khi thiếu sót):**
    - Thiếu fixture, thiếu adapter hoặc thiếu test case là nguyên nhân gây THẤT BẠI cổng phát hành, không được phép coi là bỏ qua (SKIP).

---

## 3. CHỈ SỐ MỤC TIÊU ĐỂ ĐẠT GREEN BASELINE

Một commit trong tương lai chỉ được cấp chứng nhận **TRUSTED BASELINE** khi và chỉ khi:
1. `cargo fmt --all -- --check` -> PASS (0 diffs).
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` -> PASS (0 warnings).
3. `python3 scripts/verify_guardrails.py --repo . --mode enforce` -> PASS (0 violations).
4. `python3 scripts/verify_source_test_inventory.py --repo .` -> PASS (17/17 tests exist & pass).
5. `python3 scripts/verify_behavior.py --repo .` -> PASS (12/12 behavior cases verified by independent tools).
6. `./scripts/verify_spine_lock.sh` -> PASS (với hash mới sau khi tu chính).
7. Tất cả các tính năng quảng cáo trên giao diện đều được nối dây thực tế hoặc hiển thị nhãn "Tạm thời chưa khả dụng".
