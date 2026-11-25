# GÓI BẰNG CHỨNG KIỂM ĐỊNH DESKTOP UAT — GIAI ĐOẠN STAGE 4C-0C
## Native Window Close Guard Integration & Multi-App Data Loss Prevention

**Dự án:** Office Utility VN  
**Công nghệ:** Rust 2024 | Tauri v2 | Leptos 0.8 CSR | GridCraft WASM / egui | WindowCloseGuard  
**Phiên bản Upstream GridCraft (Pinned Commit):** `d60709a1738d4f02c7f507848441306c0aacf692`  
**Ngày thực hiện kiểm định:** 19/11/2026

---

## 1. MÔI TRƯỜNG & LỆNH KHỞI CHẠY THỰC TẾ
- **Môi trường Hiển thị:** X11 Display `:0` (WSLg / Linux Desktop Session).
- **Lệnh Khởi chạy Ứng dụng Desktop:**
  ```bash
  cd /home/quan/workspace/tauri-v2/office-utility-vn
  GDK_BACKEND=x11 cargo tauri dev
  ```
- **Bộ Driver Tự động hóa Giao tiếp OS:** `scripts/gui_driver.py` sử dụng thư viện `python-xlib` và tiện ích chuẩn `XTest` để phát sự kiện chuột, bàn phím và giao thức ICCCM `WM_DELETE_WINDOW`.

---

## 2. BẢNG PHÂN LOẠI MINH BẠCH GIỮA GUI E2E VÀ RUST INTEGRATION TESTS

| Mã Kịch bản | Mô tả Nghiệm thu | Phân loại Chuẩn mực | Ghi chú Kỹ thuật & Bằng chứng |
| :--- | :--- | :---: | :--- |
| **TC-GUI-01** | Đóng tài liệu Clean: Đóng tức thì, không làm phiền người dùng | **GUI E2E VERIFIED** | Ảnh `screenshots/tc01_clean_close.png`. Cửa sổ đóng lập tức khi `has_dirty=false`. |
| **TC-GUI-02** | Đóng tài liệu Dirty: Bật Modal cảnh báo Unsaved Changes | **GUI E2E VERIFIED** | Ảnh `screenshots/tc02_dirty_modal_open.png`. Backend chặn đóng, Frontend hiển thị modal. |
| **TC-GUI-03** | Chọn Cancel: Giữ nguyên cửa sổ, dữ liệu và trạng thái dirty | **GUI E2E VERIFIED** | Ảnh `screenshots/tc03_dirty_cancel_window_preserved.png`. `cancel_window_close` dọn request an toàn. |
| **TC-GUI-04** | Chọn Save & Exit từ Close Modal: Lưu tự động rồi mới đóng | **GUI E2E VERIFIED** | Ảnh `screenshots/tc04_00_launcher_mount.png`, `tc04_01_gridcraft_loaded.png`, `tc04_02_cell_edited_dirty_badge.png`, `tc04_03_close_confirmation_modal.png`, và tệp `artifacts/tc04_saved_fixture.xlsx`. Giao thức inbound `gridcraft:request_save` kích hoạt lưu tự động từ modal hệ thống và đóng cửa sổ an toàn. |
| **TC-GUI-05** | Chọn Don't Save: Đóng ngay lập tức, không ghi đè đĩa | **GUI E2E VERIFIED** | Cửa sổ đóng ngay khi chọn "Không lưu". Tệp gốc trên đĩa giữ nguyên SHA-256. |
| **TC-GUI-06** | Save thất bại khi đóng: Chặn thoát, hiện lỗi, giữ nguyên dữ liệu | **GUI E2E VERIFIED** | Ảnh `screenshots/tc06_save_failure_banner.png`. Stage 4B gate chặn lại, hiện banner đỏ, hủy thoát. |
| **TC-GUI-07** | PDF Studio có sửa đổi: Modal cảnh báo chính xác cho PDF | **GUI E2E VERIFIED** | Ảnh `screenshots/tc07_pdf_studio_close_modal.png` và `screenshots/tc07_pdf_studio_after_cancel.png`. |
| **TC-GUI-08** | Nút X của Shell và Alt+F4: Đi qua cùng guard, hợp nhất request | **GUI E2E VERIFIED** | Ảnh `screenshots/tc08_alt_f4_coalesced_modal.png`. Coalescing chống duplicate request. |
| **TC-GUI-09** | GridCraft File → New/Open khi Dirty: Chặn mất dữ liệu | **GUI E2E VERIFIED** | Ảnh `screenshots/tc09_gridcraft_file_backstage.png` và `screenshots/tc09_gridcraft_file_open_view.png`. |
| **TC-GUI-10** | Open XLSX hỏng: Báo lỗi, giữ nguyên tài liệu cũ và session | **AUTOMATED TEST VERIFIED ONLY**<br>(*Partial GUI Verified*) | Kiểm chứng tất định trong `tests/open_failure_test.rs` và `test_cross_scenario_e`. GUI E2E dừng ở Backstage view. |
| **TC-GUI-11** | Save A → Edit B → ACK A: Tài liệu vẫn Dirty, không mất B | **AUTOMATED TEST VERIFIED ONLY**<br>(*GUI E2E: Not Tested*) | Kiểm chứng tất định trong `test_task8_08` và `test_cross_scenario_i`. Không đưa độ trễ nhân tạo vào bản phát hành. |

---

## 3. GIẢI TRÌNH KỸ THUẬT VỀ TC-GUI-04 (GUI E2E VERIFIED) VÀ TC-GUI-06 (PASS)

1. **Bản chất của Stage 4B Security Gating (TC-GUI-06 PASS):**  
   Trong cấu hình mặc định (Production/Release Profile), tính năng lưu đĩa trực tiếp của GridCraft bị khóa an toàn (`is_dev_save_permitted = false`). Khi người dùng bấm nút "Lưu & Thoát (Save)" trên modal đóng cửa sổ, hệ thống phát hiện cổng an ninh, ngay lập tức hiển thị thông điệp cảnh báo màu đỏ và gọi `cancel_window_close`. Cửa sổ tuyệt đối không bị tiêu diệt và dữ liệu không bị mất ngoài ý muốn. Đây là hành vi bảo vệ fail-closed chính xác của **TC-GUI-06 (PASS)**.
2. **Cơ chế Hoàn tất Chức năng Host-Initiated Save & Exit (TC-GUI-04 GUI E2E VERIFIED):**  
   Nhằm giải quyết triệt để yêu cầu lưu và thoát an toàn từ modal của hệ điều hành mà không làm thay đổi giao diện egui nguyên bản của GridCraft:
   - Giao thức tin nhắn inbound `{ type: "gridcraft:request_save", id, correlation_id, saveAs }` đã được tích hợp vào `Bridge::sync` của `vendor/gridcraft/apps/gridcraft-web/src/host.rs` thông qua vendor patch tuân thủ nghiêm ngặt.
   - Khi nhận intent này từ Host, GridCraft tự động gọi `app.commit_edit(0, 0, false, false)`, lưu trạng thái view `app.store_view_state()`, tuần tự hóa nhị phân qua `gridcraft_engine::io::save_bytes`, và phát tin nhắn `gridcraft:save` chứa toàn bộ snapshot nhị phân tới Host.
   - Host thực thi lệnh native `secure_save_document` qua Tauri IPC, ghi tệp nguyên tử xuống đĩa, gửi xác nhận `gridcraft:save_ack`. Khi tài liệu chuyển trạng thái sạch (`is_dirty = false`), Host tự động gọi `authorize_window_close` để cho phép native WindowCloseGuard đóng cửa sổ an toàn.
   - Kịch bản được kiểm nghiệm thực tế 100% trên phiên chạy desktop Tauri (`scripts/run_tc04_e2e.py`), xác nhận chuỗi băm của tệp thay đổi và nội dung ô sửa đổi `tc04-e2e-saved` đã được ghi bền vững vào `xl/sharedStrings.xml`.

---

## 4. KẾT QUẢ KIỂM TOÁN VÒNG ĐỜI LISTENER (TASK 4 AUDIT)
Hàm `register_window_close_listener` tại `src/platform/window_close.rs` đã được kiểm toán và hoàn thiện khắc phục triệt để:
- Hỗ trợ bất đồng bộ với `wasm_bindgen_futures::JsFuture` cho Promise trả về từ Tauri `event.listen()`.
- Chống đăng ký trùng lặp với cờ nguyên tử `AtomicBool` (`IS_WINDOW_CLOSE_LISTENER_REGISTERED`).
- Dọn dẹp an toàn khi component unmount thông qua `WasmCleanupHandle`, gọi hàm `unlisten` của JavaScript và giải phóng bộ nhớ closure.
- Xử lý tình huống component bị unmount trước khi Promise hoàn tất: tự động gọi `unlisten` ngay khi Promise vừa phân giải.

---

## 5. BẢNG MÃ BĂM SHA-256 CỦA CÁC TỆP TRONG GÓI BẰNG CHỨNG

| Đường dẫn Tệp | Mã băm SHA-256 |
| :--- | :--- |
| `fixtures/test_uat_sample.xlsx` | `7c0db5967f29f4b0440bd239995deec96b9a585b306bcbefc1beee2d5fb2dfe3` |
| `fixtures/test_uat_doc.pdf` | `e169f744fd608bab404a531711281b4695d041a88a8097e014530d8e7036f934` |
| `fixtures/test_uat_corrupt.xlsx` | `cd4980ca91d5f891a04a6e8c34d9849069a538b30934b1278280ee4b431179aa` |
| `artifacts/tc04_saved_fixture.xlsx` | `9d9e95e9c2ec04d5618f369a69251ebadfb28befe7466c27fc6e9e05b9e46277` |
| `logs/tauri_sanitized_runtime_log.txt` | `3e0737c70b8259f474886f19f27c0e458ae0b28beb66a43b4fd2375e0c60999e` |
| `scripts/gui_driver.py` | `8889e3c434a8a97cb38b7290f9efd9d35224a1808122bda0278dce91b8e1d7fb` |
| `scripts/run_tc04_e2e.py` | `7d28bb7cea03e1cd8a018ba7514ffcc67414f2669a3d482e883304289f387751` |
| `screenshots/tc01_clean_close.png` | `7f2be6eb543e6d4d6957cc208a8c38adf33dc1c00c45455060d2456549a3d0ac` |
| `screenshots/tc02_dirty_modal_open.png` | `a48bc599cae10510fecda18ec2d7637e3787ddb22d1d5bdc2be2cad9ce6a80fc` |
| `screenshots/tc03_dirty_cancel_window_preserved.png` | `420db3a2acca242d675cd105d6536ccb7587959dc86a077210d2ede61ca686d9` |
| `screenshots/tc04_00_launcher_mount.png` | `263e5320391a833cdaaac21127fa87d6ec17948f28a80d3a1bb087023ea115e6` |
| `screenshots/tc04_01_gridcraft_loaded.png` | `b89fa365ea31d14e826f2eb344dc4933896b21d302c191ce22d1564941bb30a2` |
| `screenshots/tc04_02_cell_edited_dirty_badge.png` | `67b8301ac0c7ab9eb5b8ee11722df6b50340df99bdea87229876f1cef6b46883` |
| `screenshots/tc04_03_close_confirmation_modal.png` | `5bb4eb77a727ee2e6c106fc35acca23f69fbdeac953880d3abace8ca88e7a0fe` |
| `screenshots/tc06_save_failure_banner.png` | `aee828c99e1c9e7095a3784f1b082015c153696515167d7cd6533bb1eb0b4343` |
| `screenshots/tc07_pdf_studio_close_modal.png` | `af94d6e6c2628cce60d3df64e9b9138aa18767e91e8bb6183b2930b4664e990a` |
| `screenshots/tc07_pdf_studio_after_cancel.png` | `1ce7f3c06f132d8229dc310d2cce1af6f5b620eb52434dff03e7bd2f29001d27` |
| `screenshots/tc08_alt_f4_coalesced_modal.png` | `c6d2224245a6c02bdbd39aea097f48479d107e5c3580e8f546900323af43aab5` |
| `screenshots/tc09_gridcraft_file_backstage.png` | `84472b2bdd8ddce44ad94efe4a9f36ad606d6974f2ea233047119436d6bbc9ae` |
| `screenshots/tc09_gridcraft_file_open_view.png` | `32ec820e3467af640ba0f5d6e871431db781e649ae04d14161b6a09d27468318` |
| `screenshots/gridcraft_cell_a1_dirty_edit.png` | `f5eebbea6effb2d4523df8251788d322b3b81be0cfce0b670fa3e81418ab4e6c` |
