# Kế hoạch Hiện thực Hóa & Phân loại Micro-Tools Media Studio theo `smart_dispatch.py`

> **Quy trình:** Tuân thủ `08_quy_trinh_van_hanh.md` (Day-2 SOP) và các ranh giới tự trị trong `smart_dispatch.py`.
> **Mục tiêu:** Phân loại 3 nhóm ưu tiên theo 15 loại tác vụ chuẩn hóa, và hiện thực hóa trực tiếp các tính năng thuộc Ưu tiên 1 và Ưu tiên 2.

**Goal:** Hiện thực hóa các tính năng cốt lõi được săn đón trong Media Studio (giải mã HEIC cho `image-batch`, liên kết 1-click OCR cho `doc-scanner-perspective`, cắt thủ công & Volume Booster cho `audio-silence-trimmer`) và phân loại toàn bộ ma trận theo `smart_dispatch.py`.

**Architecture:** Mở rộng module `image_batch` và `audio_silence_trimmer` trong `office-engine` theo nguyên tắc tương thích ngược 100% (`#[serde(default)]`), bổ sung giao diện tương tác trực quan trong Leptos UI, duy trì Zero Network (`NFR-SEC-01`) và No Unwrap/Expect (`NFR-SEC-03`).

**Tech Stack:** Rust 2024, Leptos 0.8, rayon, image 0.25, puremp3, ISOBMFF HEIC parser.

---

## 1. Bảng Phân loại Tác vụ Chuẩn hóa (`smart_dispatch.py` Routing Matrix)

| STT | Tính năng | Nhóm Ưu tiên | `task_type` | `target_slug` | Mã FR | Vùng Tự trị | Protected Paths | Kiểm định Bắt buộc |
|:---:|:---|:---:|:---:|:---:|:---:|:---:|:---|:---|
| 1 | **Giải mã ảnh HEIC/HEIF (iPhone)** | Ưu tiên 1 | `modify` | `image-batch` | `FR-MED-01` | **GREEN** | `crates/office-shared`, `src-tauri/src/db` | `verify_pod.sh image-batch` |
| 2 | **1-Click OCR từ Nắn phẳng tài liệu** | Ưu tiên 1 | `modify` | `doc-scanner-perspective` | `FR-MED-07` | **GREEN** | `crates/office-shared`, `src-tauri/src/db` | `cargo check --tests` |
| 3 | **Cắt thời gian (Manual Trim) & Volume Booster** | Ưu tiên 2 | `modify` | `audio-silence-trimmer` | `FR-MED-08` | **GREEN** | `crates/office-shared`, `src-tauri/src/db` | `verify_pod.sh audio-silence-trimmer` |
| 4 | **Background Remover & AI Upscaler** | Ưu tiên 3 | `addon-pack` | `image-ai-enhancer` | `FR-MED-02/03` | **RED** | Khóa Spine, phân phối gói On-Device | Chữ ký số Ed25519 + ONNX offline test |
| 5 | **Video Compressor & MP4 to MP3** | Ưu tiên 3 | `add-tools` / `os-compat` | `video-audio-demux` | `FR-MED-09` | **YELLOW** | Khóa Spine | OS Media Framework / Container Demux |

---

## 2. Chi tiết Kế hoạch Hiện thực (Task Breakdown)

### Task 1: Tích hợp HEIC Decoder vào `image-batch` (Ưu tiên 1)
- **Tệp sửa đổi:**
  - `crates/office-engine/src/media_studio/image_batch.rs`
  - `src/tools/image_batch_view.rs`
- **Hành động kỹ thuật:**
  1. Viết hàm nhận diện ISOBMFF magic bytes (`ftypheic`, `ftypmif1`, `ftypmsf1`, `ftypheix`, `ftyphevc`).
  2. Xây dựng parser ISOBMFF trích xuất metadata `ispe` (kích thước width, height) và dữ liệu hình ảnh/thumbnail embedded JPEG/RGB.
  3. Cập nhật `load_and_validate_image` để tự động giải mã HEIC và xuất sang WebP, JPG, PNG, AVIF.
  4. Cập nhật UI `image_batch_view.rs` hiển thị hỗ trợ định dạng HEIC.
  5. Viết Unit Test kiểm thử giải mã HEIC và kiểm tra tương thích ngược `ImageBatchOptions`.

### Task 2: Nút bấm 1-Click "Trích xuất văn bản (OCR)" trên `doc-scanner-perspective` (Ưu tiên 1)
- **Tệp sửa đổi:**
  - `src/tools/doc_scanner_perspective_view.rs`
- **Hành động kỹ thuật:**
  1. Thêm nút hành động nhanh *"🔍 Trích xuất văn bản (Chuyển sang Vietnamese OCR)"* trong giao diện `DocScannerPerspectiveView`.
  2. Tích hợp `use_context::<AppState>()` và gọi `app_state.deep_link_by_slug("vietnamese-ocr")` để điều hướng mượt mà 1-click.
  3. Bổ sung thông tin hướng dẫn luồng liên kết tài liệu scan $\rightarrow$ OCR.

### Task 3: Cắt theo Thời gian (Manual Trim) & Volume Booster với Soft-clipping (Ưu tiên 2)
- **Tệp sửa đổi:**
  - `crates/office-engine/src/media_studio/audio_silence_trimmer.rs`
  - `src/tools/audio_silence_trimmer_view.rs`
- **Hành động kỹ thuật:**
  1. Mở rộng `AudioTrimOptions` với các trường `#[serde(default)]`:
     - `manual_trim_enabled: bool`
     - `start_time_s: Option<f32>`
     - `end_time_s: Option<f32>`
     - `volume_boost_db: Option<f32>`
     - `enable_limiter: bool`
  2. Hiện thực logic `trim_by_time_range(&PcmAudio, start_s: f32, end_s: Option<f32>) -> PcmAudio`.
  3. Hiện thực thuật toán `boost_volume_with_limiter(&mut PcmAudio, boost_db: f32, enable_limiter: bool)` sử dụng Soft-clipping hyperbolic tangent ($\tanh$) chống vỡ tiếng.
  4. Tích hợp kết hợp Manual Trim và Silence Trim trong `execute()`.
  5. Cập nhật giao diện `AudioSilenceTrimmerView` với các thanh trượt điều khiển: Thời gian Bắt đầu, Kết thúc, Tăng âm lượng (0 - 24 dB) và Công tắc Limiter chống vỡ.
  6. Viết Unit Tests kiểm thử Manual Trim, Volume Booster, Limiter, và Backward Compatibility Deserialization.

### Task 4: Kiểm tra 4 Cổng Kiểm định Chất lượng
- Chạy `./scripts/verify_pod.sh image-batch`
- Chạy `./scripts/verify_pod.sh audio-silence-trimmer`
- Chạy `cargo check --tests` và `cargo test --test media_studio_test`
- Xác minh không vi phạm NFR-SEC-01 (Zero Network) và NFR-SEC-03 (No Unwrap/Expect trong engine code).
