# Kế hoạch Hiện thực Tính năng Công cụ Gần đây trong Tìm Nhanh & Dashboard

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hiện thực cơ chế theo dõi công cụ sử dụng gần nhất (Most Recently Used - MRU) lưu trong `localStorage` và ưu tiên hiển thị các lựa chọn công cụ gần nhất khi người dùng mở Hộp Tìm Nhanh (Command Palette `Ctrl + K`) và trên Dashboard.

**Architecture:** Sử dụng mô hình Client-side Reactive State (Leptos Signal) kết hợp bộ lưu trữ trình duyệt `window.localStorage` để lưu vết danh sách tối đa 6 công cụ vừa chọn. Đóng gói logic biến đổi MRU thành module thuần Rust `recent_tools.rs` có bộ kiểm thử đơn vị độc lập 100%, sau đó tích hợp vào giao diện `src/app.rs` và `styles.css` mà không can thiệp vào khung xương sống (Spine Lock).

**Tech Stack:** Rust (2024 edition), Leptos 0.8 (CSR / Signals), `web-sys` (Storage), CSS3 (Flexbox/Grid/Modal).

**Spec:** [`docs/specs/05-ui-ux.md`](file:///home/quan/workspace/tauri-v2/office-utility-vn/docs/specs/05-ui-ux.md) (Mục 5.3.1), [`docs/specs/02-srs.md`](file:///home/quan/workspace/tauri-v2/office-utility-vn/docs/specs/02-srs.md) (US-01), [`docs/specs/08_quy_trinh_van_hanh.md`](file:///home/quan/workspace/tauri-v2/office-utility-vn/docs/specs/08_quy_trinh_van_hanh.md) (Quy trình `modify-pod` Zone GREEN).

## Global Constraints

- **NFR-SEC-01 (Zero Network):** Lưu trữ 100% On-Device qua `localStorage`, không truyền bất kỳ telemetry hay dữ liệu nào ra mạng ngoài.
- **NFR-SEC-03 (An toàn Không Panic):** Tuyệt đối không dùng `unwrap()` hoặc `expect()` trong logic xử lý `localStorage` và phân tích JSON; dùng `unwrap_or_default()` và `if let` an toàn khi webview chặn storage.
- **Spine Lock Integrity:** Giữ nguyên vẹn 100% SHA-256 của `crates/office-shared/src/lib.rs`, `src-tauri/src/db/mod.rs` và `src-tauri/src/commands/mod.rs`.
- **NFR-PERF-01 (Hiệu năng tức thời):** Command Palette mở trong < 50ms, chuyển đổi danh sách tìm kiếm < 5ms.
- **Tương thích IME Tiếng Việt:** Không chặn hoặc nuốt sự kiện gõ phím khi người dùng gõ tiếng Việt có dấu trong ô tìm kiếm.

## Review Focus

1. **Khởi tạo lần đầu (First Launch):** Khi `localStorage` chưa có dữ liệu (mảng rỗng), không hiển thị phần tiêu đề "Gần đây" thừa và hiển thị danh sách tất cả công cụ bình thường.
2. **Khử trùng lặp và Đẩy lên đầu (MRU Dedup & Reorder):** Khi chọn lại một công cụ đã tồn tại ở vị trí giữa hoặc cuối danh sách, công cụ đó phải được chuyển lên đầu (`index 0`), không tạo bản sao trùng lặp.
3. **Giới hạn số lượng (Capacity Truncation):** Khi danh sách đạt ngưỡng (mặc định 6 công cụ), việc thêm công cụ mới phải tự động cắt tỉa các công cụ cũ nhất.
4. **Môi trường lỗi LocalStorage (Storage Access Exception):** Khi chạy ở chế độ hạn chế (Private/Incognito) hoặc test runner không có DOM `window`, module fallback mượt mà về bộ nhớ RAM mà không crash app.
5. **Chế độ Lọc Từ Khóa (Active Search vs Empty Search):** Khi ô tìm kiếm rỗng, hiển thị 2 khối rõ rệt: "🕒 Công cụ gần đây" và "📌 Tất cả công cụ". Khi đang gõ tìm kiếm, hiển thị danh sách kết quả tìm kiếm thông thường kèm huy hiệu `"🕒 Gần đây"` cho công cụ nào nằm trong danh sách MRU.

---

### Task 1: Module Lõi MRU & Lưu trữ Cục bộ (`src/recent_tools.rs`)

**Files:**
- Create: `src/recent_tools.rs`
- Modify: `src/main.rs:1-6` (khai báo `pub mod recent_tools;`)
- Test: Nhúng trực tiếp trong `src/recent_tools.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces:
  - `pub fn update_recent_list(existing: &[String], new_slug: &str, limit: usize) -> Vec<String>`
  - `pub fn serialize_recent_tools(slugs: &[String]) -> String`
  - `pub fn deserialize_recent_tools(raw: &str) -> Vec<String>`
  - `pub fn save_recent_to_storage(slugs: &[String])`
  - `pub fn load_recent_from_storage() -> Vec<String>`

- [ ] **Step 1: Viết test thất bại (RED) kiểm tra logic MRU**

Tạo các test case kiểm tra:
1. `test_update_recent_list_insert_and_ordering`: Thêm slug mới vào đầu danh sách.
2. `test_update_recent_list_dedup_and_promote`: Chọn slug đã có thì đẩy lên đầu và giữ mảng không trùng.
3. `test_update_recent_list_capacity_limit`: Cắt tỉa khi vượt quá `limit = 6`.
4. `test_serialize_and_deserialize_roundtrip`: Chuyển đổi qua lại giữa JSON string và `Vec<String>`.
5. `test_deserialize_corrupt_data_fallback`: Khi chuỗi JSON bị hỏng, trả về `Vec::new()` mà không hoảng loạn.

- [ ] **Step 2: Chạy test để xác nhận thất bại (RED)**

Run: `CARGO_TARGET_DIR=/tmp/cargo-target cargo test --bin office-utility-vn-ui recent_tools`
Expected: FAIL (module chưa được hiện thực).

- [ ] **Step 3: Hiện thực các hàm trong `src/recent_tools.rs`**

- `update_recent_list`: Lọc bỏ `new_slug` khỏi `existing`, chèn vào vị trí 0, gọi `truncate(limit)`.
- `serialize_recent_tools` & `deserialize_recent_tools`: Sử dụng `serde_json`.
- `save_recent_to_storage` & `load_recent_from_storage`: Truy cập `web_sys::window()?.local_storage()` với key `"office_utility_recent_tools_v1"` an toàn với `unwrap_or_default()`.

- [ ] **Step 4: Chạy test để xác nhận thành công (GREEN)**

Run: `CARGO_TARGET_DIR=/tmp/cargo-target cargo test --bin office-utility-vn-ui recent_tools`
Expected: PASS 100%.

- [ ] **Step 5: Commit thay đổi**

```bash
git add src/recent_tools.rs src/main.rs
git commit -m "feat(ui): add recent tools MRU management module"
```

---

### Task 2: Tích hợp Command Palette & Dashboard Quick Chips (`src/app.rs` & `styles.css`)

**Files:**
- Modify: `src/app.rs` (khai báo signal `recent_tools`, hàm `select_tool_and_record`, cập nhật hiển thị palette và thanh Quick Access)
- Modify: `styles.css` (thêm class `.palette-section-header`, `.palette-item-badge.recent`, `.dashboard-recent-bar`, `.recent-tool-chip`)

**Interfaces:**
- Consumes: `recent_tools::{update_recent_list, save_recent_to_storage, load_recent_from_storage}` từ Task 1.

- [ ] **Step 1: Khởi tạo Reactive Signal trong `src/app.rs`**

- Thêm `let (recent_tool_slugs, set_recent_tool_slugs) = signal(recent_tools::load_recent_from_storage());`.
- Tạo hàm dùng chung `select_and_record_tool(slug: String)`:
  - Cập nhật `current_slug.set(slug.clone())`.
  - Cập nhật `recent_tool_slugs`:
    ```rust
    let updated = recent_tools::update_recent_list(&recent_tool_slugs.get(), &slug, 6);
    recent_tools::save_recent_to_storage(&updated);
    set_recent_tool_slugs.set(updated);
    ```
  - Đóng Command Palette, chuyển tab sang `WorkspaceView::Tools`, và gọi `load_presets_for_current_tool()`.

- [ ] **Step 2: Cập nhật giao diện Command Palette (Modal `Ctrl + K`)**

- Khi `search_query.trim().is_empty()`:
  - Nếu `recent_tool_slugs` không rỗng: Hiển thị khối tiêu đề `"🕒 CÔNG CỤ VỪA DÙNG GẦN ĐÂY"`, render danh sách các công cụ tương ứng (kèm badge `"Gần đây"`).
  - Tiếp theo hiển thị khối tiêu đề `"📌 TẤT CẢ CÔNG CỤ"`, render các công cụ còn lại.
- Khi `search_query.trim()` có nội dung:
  - Render kết quả lọc mờ theo từ khóa.
  - Nếu công cụ nào nằm trong `recent_tool_slugs`, gắn thêm badge `"🕒 Gần đây"`.

- [ ] **Step 3: Thêm Thanh Truy cập Nhanh (Recent Quick Chips) trên Dashboard**

- Trong `Zone 1` hoặc thanh tiêu đề phụ (Sub-header) của màn hình chính, khi có công cụ trong danh sách gần đây:
  - Hiển thị dải nút bấm nhanh (chips) chứa biểu tượng và tên của top 3-4 công cụ vừa dùng.
  - Bấm vào chip sẽ chuyển ngay lập tức sang công cụ đó mà không cần mở Command Palette.

- [ ] **Step 4: Bổ sung Style CSS trong `styles.css`**

- Định nghĩa `.palette-section-header`: font 11px, màu `#94A3B8`, in hoa, padding 8px 12px.
- Định nghĩa `.palette-item-badge.recent`: màu sắc nổi bật cyan `#38BDF8`, viền `1px solid rgba(56, 189, 248, 0.4)`.
- Định nghĩa `.dashboard-recent-bar` và `.recent-tool-chip`: thanh cuộn ngang mượt mà, hover hiệu ứng sáng nhẹ.

- [ ] **Step 5: Kiểm tra biên dịch & linting**

Run: `CARGO_TARGET_DIR=/tmp/cargo-target cargo clippy --bin office-utility-vn-ui`
Expected: 0 errors, 0 warnings.

- [ ] **Step 6: Commit thay đổi**

```bash
git add src/app.rs styles.css
git commit -m "feat(ui): integrate recent tools in command palette and dashboard"
```

---

### Task 3: Cổng Kiểm định Chất lượng Toàn diện & Xác thực Khung Xương Sống

**Files:**
- Verification: Không sửa code thêm, kiểm tra toàn bộ hệ thống.

- [ ] **Step 1: Kiểm định tính toàn vẹn Khung xương sống (Spine Lock)**

Run: `./scripts/verify_spine_lock.sh`
Expected: SHA-256 khớp chuẩn 100% cho cả 3 tệp cốt lõi, 517+ tests pass.

- [ ] **Step 2: Kiểm tra biên dịch gói WebAssembly (Trunk Build)**

Run: `make build-web` hoặc `trunk build`
Expected: Build WASM thành công, tối ưu hóa bundle CSS/HTML.

- [ ] **Step 3: Kiểm tra toàn bộ Workspace**

Run: `CARGO_TARGET_DIR=/tmp/cargo-target cargo check --workspace`
Expected: Hoàn tất với mã thoát 0.

- [ ] **Step 4: Cập nhật nhật ký tiến độ & hoàn tất**

Ghi nhận hoàn thành vào `.superpowers/sdd/` và bàn giao cho người dùng.
