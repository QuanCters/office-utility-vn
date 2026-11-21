# Hướng Dẫn Quản Lý & Tối Ưu Hóa Cargo Build Cache
## Office Utility VN — Rust / Tauri v2 / Leptos / Vendored Craft Apps

Tài liệu này cung cấp hướng dẫn kỹ thuật chi tiết dành cho kỹ sư phát triển về kiến trúc thư mục `target`, nguyên nhân tích tụ dung lượng, và quy trình bảo trì cache an toàn, không phá vỡ tính cô lập của workspace và không làm gián đoạn các nhánh phát triển.

---

### 1. Kiến Trúc Thư Mục Build & Tính Cô Lập Của Workspace

Dự án hiện có 3 thư mục `target` độc lập:

| Thư mục | Không gian quản lý | Mục đích |
| :--- | :--- | :--- |
| `office-utility-vn/target` | Main Cargo Workspace | Cung cấp mã máy cho Desktop Tauri, UI Leptos CSR, `office-engine`, `office-shared`. |
| `office-utility-vn/vendor/pdfcraft/target` | Sub-workspace PDFCraft | Biên dịch WASM canvas cho PDF Studio qua Trunk (`scripts/build_pdfcraft_web.sh`). |
| `office-utility-vn/vendor/gridcraft/target` | Sub-workspace GridCraft | Biên dịch WASM canvas cho Data Lab Spreadsheet qua Trunk (`scripts/build_gridcraft_web.sh`). |

#### Tại sao KHÔNG nên gom chung `CARGO_TARGET_DIR`?
- **Khác biệt Resolver:** `pdfcraft` và `gridcraft` sử dụng `resolver = "3"`, trong khi main workspace sử dụng resolver chuẩn của Rust 2024.
- **Khác biệt `[patch.crates-io]`:** `pdfcraft` patch các crate `hayro-*` đường dẫn cục bộ; `gridcraft` patch `ecolor`, `emath`, `epaint` qua Git rev.
- **Tránh nhiễm bẩn cache & lock contention:** Dùng chung một thư mục `target` sẽ gây xung đột lock file `.cargo-lock`, làm mất tính tái lập (reproducibility) và có thể dẫn đến việc invalidate toàn bộ cache chéo giữa các bên.
- **Nguyên tắc:** Duy trì cô lập hoàn toàn giữa Main Workspace và các Vendor sub-projects.

---

### 2. Vòng Đời Của Tài Sản Sản Xuất (Production Assets Lifecycle)

Ứng dụng Desktop Tauri chỉ nạp giao diện WebAssembly của PDFCraft và GridCraft thông qua:
- `office-utility-vn/public/pdfcraft/`
- `office-utility-vn/public/gridcraft/`

Khi chạy `trunk serve` hoặc đóng gói ứng dụng Tauri, Trunk sao chép nguyên trạng 2 thư mục tĩnh trên vào `dist/`. 
> [!IMPORTANT]
> Toàn bộ nội dung trong `vendor/pdfcraft/target` và `vendor/gridcraft/target` chỉ là **cache biên dịch trung gian**. Xóa hai thư mục target này **hoàn toàn KHÔNG làm mất** các file `.wasm`, `.js` đang phục vụ ứng dụng. Nếu sau này có thay đổi mã nguồn trong vendor, chạy lại script build web sẽ tái tạo lại cache.

---

### 3. Nguyên Nhân Gây Ra Dung Lượng ~39 GB

1. **Bộ nhớ đệm biên dịch gia tăng (`target/debug/incremental` ~17.3 GiB):**
   - Rustc lưu trữ đồ thị AST/MIR/LLVM bitcode để tăng tốc biên dịch các thay đổi nhỏ. Qua hàng trăm phiên làm việc, cache này không tự dọn dẹp các nhánh code cũ.
2. **File nhị phân kiểm thử tồn đọng (`target/debug/deps` ~12.9 GiB):**
   - Main workspace khai báo 12 integration test độc lập (`[[test]]`). Mỗi lần chạy `cargo test`, Cargo liên kết 12 file nhị phân độc lập (~50–60MB mỗi file) kèm file binary UI (~630MB).
   - Cargo tạo hash ngẫu nhiên cho mỗi phiên bản sửa đổi (ví dụ: `office_utility_vn_ui-347edf55...`) và **không bao giờ tự động xóa các hash cũ**.
3. **Vendor targets (~5.2 GiB):**
   - PDFCraft target (~3.54 GiB) và GridCraft target (~1.56 GiB) được giữ nguyên sau các lần build WASM ban đầu.

---

### 4. Quy Tắc Dọn Dẹp An Toàn (Safe vs Unsafe Practices)

#### ❌ CÁC HÀNH VI BỊ CẤM (UNSAFE):
- **CẤM:** `find target/debug/deps -type f -executable -delete`
  - *Lý do:* Phá vỡ tính đồng bộ giữa thư mục `deps` và bảng kiểm định `.fingerprint`. Xóa file thực thi đang cần sẽ khiến Cargo báo lỗi liên kết hoặc crash khi chạy test.
- **CẤM:** Tự động chạy `rm -rf target` trong các lệnh build/dev thông thường.
  - *Lý do:* Làm mất sạch cache của hơn 300 thư viện bên ngoài (WebKit, GTK, Tokio, Leptos), khiến lần build tiếp theo bị nghẽn (cold build mất 2-5 phút).

#### ✅ CÁC QUY TRÌNH ĐƯỢC HỖ TRỢ (SUPPORTED):

1. **Kiểm tra hiện trạng cache (Read-Only):**
   ```bash
   make cache-report
   # Hoặc trực tiếp: ./scripts/inspect_build_cache.sh
   ```

2. **Chạy thử nghiệm giả lập (Dry-run):**
   ```bash
   make clean-cache-dry-run
   ```

3. **Thu hồi dung lượng an toàn (Giữ nguyên thư viện bên thứ ba ~8.0 GiB):**
   Nếu đĩa cứng gần đầy nhưng bạn muốn giữ tốc độ build nhanh, chỉ dọn dẹp các crate nội bộ:
   ```bash
   cargo clean --profile dev -p office-utility-vn-ui -p office-engine -p office-shared -p office-utility-vn
   ```
   *Số liệu thực tế kiểm nghiệm (Phase A - 17/11/2026):*
   - Kích thước trước dọn dẹp: `target/` đạt **33.52 GiB** (35.993.013.486 B).
   - Đã xóa: 16.660 files, giải phóng **27.27 GB (25.40 GiB)** apparent size và **27.29 GB** dung lượng đĩa vật lý.
   - Kích thước ngay sau dọn dẹp: `target/` giảm xuống còn **8.12 GiB** (8.719.363.626 B).
   - Bảo tồn 100%: Toàn bộ 3rd-party dependencies, `target/release`, `vendor/*/target`, và assets trong `public/`.
   - Sau khi chạy lại trọn bộ 376 unit test & 12 test suites, `target/` chỉ đạt **11.98 GiB** (tiết kiệm ròng **~23.13 GB** so với lượng rác tích tụ trước đó).

4. **Thu hồi vendor cache không dùng đến (Thu hồi ~5.2 GiB):**
   Sau khi đã có file tĩnh trong `public/`, có thể xóa cache của vendor bằng lệnh Cargo chính quy:
   ```bash
   (cd vendor/pdfcraft && cargo clean)
   (cd vendor/gridcraft && cargo clean)
   ```

5. **Dọn dẹp triệt để (Cold Clean):**
   Chỉ sử dụng khi cần giải phóng đĩa tối đa hoặc sửa lỗi cache hỏng:
   ```bash
   make clean
   ```

---

### 5. Khuyến Nghị Cấu Hình Profile (Tùy Chọn - Cần Đánh Đổi)

Trong [Cargo.toml](../Cargo.toml):
- **Đối với Test Profile (`[profile.test]`):**
  Có thể cân nhắc thêm `incremental = false` riêng cho profile `test` nếu thường xuyên chạy trọn bộ kiểm thử trên CI/Local, nhằm ngăn chặn việc sinh ra 12 đồ thị incremental riêng biệt cho 12 test binaries.
- **Đối với Dev Profile (`[profile.dev]`):**
  **NÊN GIỮ NGUYÊN** `debug = 1` và `incremental = true` để giữ tốc độ phản hồi nhanh khi kỹ sư sửa code giao diện và chạy `trunk serve` / `cargo tauri dev`.
