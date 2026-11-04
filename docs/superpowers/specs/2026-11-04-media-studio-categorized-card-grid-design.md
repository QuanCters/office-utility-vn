# Thiết kế Kỹ thuật: Media Studio Categorized Card Grid Hub, Mini-Sidebar & Floating Queue Widget

- **Tác giả:** Antigravity / User Pair Programming
- **Ngày lập:** 2026-11-04
- **Trạng thái:** Bản thảo đề xuất duyệt (Draft for Approval)
- **Căn cứ yêu cầu:** Refactor UI Media Studio theo Mô hình Hub/Portal (chuẩn iLoveIMG & Lightroom) với Mini Gallery Sidebar tự động thu gọn (56px), Persistent Dock Batch Queue tại Workspace và Floating Contextual Progress Widget trên Hub khi có tác vụ chạy ngầm.

---

## 1. Bối cảnh & Mục tiêu (Intent & The Why)

### 1.1. Hiện trạng
- Giao diện hiện tại của Micro-App **Media Studio** sử dụng bố cục cố định:
  - Cột trái: `MediaStudioGalleryPanel` (~280px) hiển thị danh sách tệp nguồn.
  - Trung tâm: Tool View với công cụ mặc định `image-batch`.
  - Đáy: `MediaStudioBatchQueue` chiếm dụng cố định chiều cao màn hình.
- Nhược điểm:
  - Buộc người dùng vào thẳng công cụ `image-batch` dù nhu cầu có thể là cắt ảnh thẻ, che mờ giấy tờ hay chuyển video sang GIF.
  - Chiếm dụng nhiều diện tích chiều ngang và chiều dọc ngay cả khi chưa có tệp nào được xử lý.
  - Khi quay lại màn hình chọn công cụ, nếu thanh hàng đợi bị ẩn hoàn toàn thì người dùng sẽ gặp "điểm mù UX" — không biết tiến độ xử lý ngầm của hàng loạt tệp đã nén/chuyển đổi.

### 1.2. Mục tiêu (Goals)
1. **Kiến trúc Chọn công cụ trước (Tool-First Portal):** Khi mở Media Studio, người dùng được chào đón bằng **Trang chủ Hub (Portal View)** dạng lưới thẻ (Categorized Card Grid) chia 2 phân khu chuẩn mực:
   - 📸 **Xử lý Ảnh (Image Studio)** (7 công cụ)
   - 🎬 **Audio & Video Studio** (2 công cụ)
   - ⚡ **Banner Section: Chuỗi Tự động hóa Macro DAG Đa phương tiện**
2. **Cơ chế Mini Gallery Sidebar tự động thu gọn (Collapsible Gallery):**
   - Khi chọn một công cụ từ Hub, giao diện chuyển sang **Focused Tool Workspace**.
   - Gallery bên trái tự động thu gọn thành **Mini Sidebar (56px)** với nút toggle `▶`/`◀`, icon thumbnail rút gọn, badge số lượng tệp và nút thêm nhanh `+`.
3. **Thanh Hàng Đợi Hai Trạng Thái (Dual-State Batch Queue Strategy):**
   - **Tại Workspace (Trạng thái tĩnh):** `MediaStudioBatchQueue` hoạt động như một Persistent Dock ở đáy (kiểu Spotify Mini-player), cho phép thu gọn còn 1 dòng tóm tắt hoặc mở rộng trượt lên để xem chi tiết từng file.
   - **Tại Hub Portal (Trạng thái động):** Ẩn hoàn toàn thanh Dock cố định để giữ Hub tinh gọn, nhưng kích hoạt một **Floating Contextual Widget (Mini Progress Pill / FAB)** ở góc dưới bên phải hoặc trên Header nếu đang có tiến trình xử lý ngầm. Người dùng click vào widget sẽ mở một Popover/Drawer nhỏ để xem nhanh hoặc tạm dừng/hủy tác vụ mà không cần rời Hub.
4. **Điều hướng Breadcrumb linh hoạt:** Nút `← Tất cả công cụ (Media Hub)` trên thanh tiêu đề cho phép quay về Portal bất kỳ lúc nào.
5. **Độ an toàn kiến trúc & Tương thích ngược:** Giữ vững 100% 10 unit tests hiện có trong `tests/media_studio_test.rs`, tuân thủ khóa Spine Lock và Design Tokens (`make verify-ui`).

---

## 2. Kiến trúc Giao diện & Bố cục Không gian

```
┌────────────────────────────────────────────────────────────────────────┐
│ Media Studio Layout                                                    │
├────────────────────────────────────────────────────────────────────────┤
│ Chế độ 1: HUB PORTAL VIEW (Trang chủ)                                  │
│ ┌────────────────────────────────────────────────────────────────────┐ │
│ │ Header: ← Về Launcher | 🎨 Media Studio | 9 Công cụ | 🔒 On-Device │ │
│ │ Hero + Search Bar: 🔍 "Tìm kiếm nhanh trong 9 công cụ Media..."    │ │
│ ├────────────────────────────────────────────────────────────────────┤ │
│ │ 📸 SECTION 1: XỬ LÝ & TỐI ƯU ẢNH (7 Cards)                         │ │
│ │   [Nén ảnh hàng loạt] [Cắt ảnh thẻ 3x4/4x6] [Làm mờ giấy tờ]       │ │
│ │   [Xóa Exif/GPS]      [Đóng dấu Watermark]  [Làm đẹp Screenshot]   │ │
│ │   [Nắn phẳng văn bản chụp nghiêng]                                 │ │
│ ├────────────────────────────────────────────────────────────────────┤ │
│ │ 🎬 SECTION 2: AUDIO & VIDEO (2 Cards)                               │ │
│ │   [Cắt khoảng lặng ghi âm]  [Chuyển Video sang GIF/WebP]           │ │
│ ├────────────────────────────────────────────────────────────────────┤ │
│ │ ⚡ SECTION 3: TỰ ĐỘNG HÓA QUY TRÌNH (1 Banner Card)                 │ │
│ │   [Chuỗi Macro DAG - Xử lý liên hoàn Đa phương tiện]               │ │
│ └────────────────────────────────────────────────────────────────────┘ │
│ [Floating Progress Pill FAB] (Chỉ nổi khi có batch đang chạy ngầm)  │ │
│   └─► Click mở Floating Popover Drawer can thiệp hàng đợi tức thì      │
├────────────────────────────────────────────────────────────────────────┤
│ Chế độ 2: FOCUSED TOOL WORKSPACE (Không gian thao tác tập trung)       │
│ ┌──────────┬─────────────────────────────────────────────────────────┐ │
│ │ MINI     │ Top Bar: ← Tất cả công cụ (Hub) | Tên công cụ | 🔒 Native│ │
│ │ GALLERY  ├─────────────────────────────────────────────────────────┤ │
│ │ (56px)   │ Context Banner: Đang áp dụng cho tệp X...               │ │
│ │          │                                                         │ │
│ │ [Toggle] │ [ Vùng Thao tác / Dropzone Kéo-Thả Ảnh / Canvas ]       │ │
│ │ [Badge 5]│                                                         │ │
│ │ [+] Thêm │ [ Panel Cấu hình Tham số Nghiệp vụ ]                    │ │
│ │ [Img 1]  │                                                         │ │
│ │ [Img 2]  ├─────────────────────────────────────────────────────────┤ │
│ │ [Img 3]  │ Persistent Dock Batch Queue: [▶ 3/5 hoàn tất] [Expand ▲] │ │
│ └──────────┴─────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Phân bổ Chi tiết 9 Công cụ Media Studio

### 3.1. Phân khu 1: Xử lý Ảnh (`MediaCategory::Image` - 7 công cụ)
1. `image-batch`: Nén & đổi định dạng ảnh hàng loạt (Icon: 🗜️)
2. `photo-id-crop`: Cắt ảnh thẻ 3x4, 4x6 chuẩn 300 DPI (Icon: 👤)
3. `image-bg-redact`: Làm mờ dữ liệu nhạy cảm giấy tờ CCCD/MST (Icon: 🕶️)
4. `image-exif-inspector`: Kiểm tra & xóa sạch siêu dữ liệu Exif/GPS (Icon: 🔍)
5. `watermark-batch-studio`: Đóng dấu bản quyền ảnh hàng loạt (Icon: 🏷️)
6. `screenshot-beautifier`: Làm đẹp ảnh chụp màn hình chuẩn mockup (Icon: ✨)
7. `doc-scanner-perspective`: Phẳng hóa tài liệu chụp nghiêng 4 góc (Icon: 📐)

### 3.2. Phân khu 2: Audio & Video (`MediaCategory::AudioVideo` - 2 công cụ)
8. `audio-silence-trimmer`: Cắt bỏ khoảng lặng file ghi âm podcast (Icon: 🎙️)
9. `video-to-gif-webp`: Chuyển đổi video ngắn sang GIF/WebP hoạt họa (Icon: 🎞️)

### 3.3. Phân khu 3: Tự động hóa Pipeline Macro DAG
- Banner Card: Kéo ghép liên hoàn các bước xử lý ảnh/video on-device.

---

## 4. Mô hình Trạng thái (State Modeling)

Trong `src/micro_apps/media_studio/state.rs`:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MediaViewMode {
    Hub,
    ToolWorkspace,
    Workflow,
}
```

Bổ sung vào `MediaStudioState`:
- `pub view_mode: RwSignal<MediaViewMode>` (mặc định `MediaViewMode::Hub` khi vào ứng dụng, `ToolWorkspace` nếu có `initial_tool`).
- `pub sidebar_collapsed: RwSignal<bool>` (mặc định `true` trong workspace).
- `pub is_floating_queue_open: RwSignal<bool>` (quản lý trạng thái mở popover của Floating Contextual Widget trên Hub).

Các helper methods:
- `open_hub(&self)`: Đặt `view_mode = MediaViewMode::Hub`.
- `open_tool(&self, slug: &str)`: Đặt `active_tool = slug`, `view_mode = MediaViewMode::ToolWorkspace`, `sidebar_collapsed = true`.
- `open_workflow(&self)`: Đặt `view_mode = MediaViewMode::Workflow`.
- `toggle_sidebar(&self)`: Đảo ngược `sidebar_collapsed`.
- `toggle_floating_queue(&self)`: Đảo ngược `is_floating_queue_open`.
- `is_hub(&self) -> bool`
- `is_tool_workspace(&self) -> bool`
- `is_background_processing(&self) -> bool`: Kiểm tra xem có tác vụ batch nào đang ở trạng thái `"Processing"` hoặc `is_running == true`.
- `batch_progress_ratio(&self) -> f32`: Tính tỷ lệ hoàn thành trung bình của toàn bộ hàng đợi (0.0 đến 1.0).

---

## 5. Thiết kế Floating Contextual Widget trên Hub

- **Điều kiện hiển thị:** Chỉ xuất hiện trên Hub khi `state.is_background_processing()` hoặc `state.has_active_batches()`.
- **Cấu tạo:**
  - Vị trí: Cố định góc phải dưới màn hình (`bottom: 24px; right: 24px; z-index: 50;`).
  - Hình dạng: **Progress Pill** mềm mại với viền phát sáng gradient nhẹ theo trạng thái (Xanh lá khi chạy tốt, Cam khi chờ, Đỏ khi có file lỗi).
  - Nội dung trên Pill:
    - Icon động quay nhẹ hoặc pulse indicator `⚡`.
    - Văn bản ngắn: `Đang xử lý: 14/50 tệp (28%)`.
    - Thanh mini progress bar 3px tích hợp ngay trên mép dưới của pill.
  - Tương tác: Click vào pill sẽ mở **Floating Drawer Popover** trượt lên:
    - Danh sách các tệp trong hàng đợi và thanh phần trăm từng file.
    - Nút `Tạm dừng`, `Tiếp tục`, `Hủy hàng đợi`.
    - Nút `Đến Workspace` để nhảy thẳng vào công cụ đang chạy.

---

## 6. Kế hoạch Kiểm định (Verification Plan)

1. **Unit Tests:** Bổ sung các test trong `tests/media_studio_test.rs`:
   - `test_media_studio_view_mode_and_sidebar_toggle`: Khởi tạo mặc định Hub, mở tool chuyển sang ToolWorkspace + thu gọn sidebar, toggle sidebar, quay về Hub.
   - `test_media_studio_hub_filter`: Lọc công cụ theo từ khóa có dấu/không dấu ("ảnh thẻ", "anh the", "nén", "exif", "gif", v.v.).
   - `test_media_studio_floating_widget_signals`: Kiểm tra cờ `is_background_processing` và tính toán `batch_progress_ratio`.
2. **Spine Lock Check:** Chạy `./scripts/verify_spine_lock.sh` bảo vệ 3 tầng khung xương sống.
3. **UI Tokens & Ergonomic Floor:** Chạy `make verify-ui` đảm bảo 100% tokens từ `base.css` và font-size >= 12px.
4. **Browser E2E (Playwright):** Chụp 2 ảnh đối chứng:
   - `screenshots/media_studio_hub_view.png`
   - `screenshots/media_studio_focused_workspace.png`
