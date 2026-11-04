# Kế Hoạch Hiện Thực Hóa: Phát Âm Thanh Đọc Số Tiền Chuẩn (Audio Readout)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bổ sung tính năng phát âm thanh (Audio Readout) đọc to chính xác số tiền người dùng đã nhập theo phương pháp ghép nối mẫu âm thanh studio AI (Audio Stitching) 100% on-device trong công cụ `vietnamese-number-to-words`.

**Architecture:** Sử dụng bộ 35 mẫu âm thanh AI phòng thu định dạng MP3 đặt trong `public/audio/vn_numbers/`, được ánh xạ bởi bộ Audio Tokenizer và lập lịch phát nối tiếp chính xác bằng Web Audio API (`AudioContext`) trên giao diện Leptos CSR. 

**Tech Stack:** Rust 2024, Leptos 0.8 CSR, Web Audio API (`web-sys::AudioContext`, `AudioBuffer`), Python (`edge-tts` cho bước tạo soundpack một lần), Tauri v2.

**Spec:** [`docs/superpowers/specs/2026-11-04-vietnamese-number-audio-readout-design.md`](file:///home/quan/workspace/tauri-v2/office-utility-vn/docs/superpowers/specs/2026-11-04-vietnamese-number-audio-readout-design.md)

---

## Global Constraints
* **NFR-SEC-01 (100% On-Device & Zero Network Egress):** Ứng dụng Desktop khi chạy không được thực hiện bất kỳ network request nào ra ngoài; toàn bộ file audio phải có sẵn cục bộ.
* **Spine Read-Only Guard:** Không sửa đổi cấu trúc DTO trong `crates/office-shared`, SQLite trong `src-tauri/src/db`, chữ ký IPC trong `src-tauri/src/commands`.
* **Ergonomic Floor & Design Tokens:** Mọi CSS và font-size trên giao diện Leptos phải `>= 12px` và sử dụng 100% CSS variables trong `styles/base.css`.
* **Non-blocking Main Thread:** Quá trình tải buffer và giải mã âm thanh không được làm đơ giao diện Leptos 60 FPS.

## Review Focus
1. Biến âm số `1` (`mười một` vs `hai mươi mốt`) phải phát đúng mẫu âm thanh `mot` vs `mot_sau`.
2. Biến âm số `5` (`lẻ năm` vs `mười lăm` vs `hai mươi lăm`) phải phát đúng `nam` vs `lam`.
3. Phân biệt phương ngữ Bắc / Nam (`nghìn` vs `ngàn`, `lẻ` vs `linh`, `tư` vs `bốn`).
4. Hậu tố kế toán (`đồng`, `đồng chẵn`, `đô la Mỹ và xu`, `âm ...`).
5. Hủy phát tức thì (< 50ms) khi người dùng bấm nút Dừng hoặc gõ số mới.

---

### Task 1: Tạo Bộ Mẫu Âm Thanh Phòng Thu (Studio Soundpack)

**Files:**
- Create: `scripts/generate_audio_soundpack.py`
- Create: `public/audio/vn_numbers/*.mp3` (35 files)
- Test: `tests/audio_soundpack_test.rs`

- [ ] **Step 1.1: Viết script `scripts/generate_audio_soundpack.py` để xuất 35 file MP3 chuẩn**
  - Sử dụng `edge-tts` với voice `vi-VN-HoaiMyNeural` (chuẩn miền Bắc) và `vi-VN-NamMinhNeural` (cho các từ đặc thù miền Nam `ngàn`, `linh`, `bốn`).
  - Danh sách từ: `khong`, `mot`, `hai`, `ba`, `bon`, `nam`, `sau`, `bay`, `tam`, `chin`, `mot_sau`, `tu`, `lam`, `muoi`, `muoi_chuc`, `tram`, `nghin`, `ngan`, `trieu`, `ty`, `le`, `linh`, `am`, `va`, `dong`, `chan`, `xu`, `do_la_my`, `euro`, `cents`.
  - Chuẩn hóa âm lượng đầu ra.

- [ ] **Step 1.2: Chạy script tạo các file audio vào `public/audio/vn_numbers/`**
  - Chạy `python3 scripts/generate_audio_soundpack.py`.
  - Kiểm tra 35 files MP3 đã xuất hiện đầy đủ trong thư mục `public/audio/vn_numbers/`.

- [ ] **Step 1.3: Viết test kiểm tra sự tồn tại và tính hợp lệ của soundpack**
  - Viết test `tests/audio_soundpack_test.rs` kiểm tra 35 file có dung lượng > 100 bytes và header MP3 chuẩn (`ID3` hoặc `0xFF 0xFB/0xF3`).
  - Chạy `cargo test --test audio_soundpack_test` và đảm bảo pass 100%.

- [ ] **Step 1.4: Commit Task 1**
  - `git add scripts/generate_audio_soundpack.py public/audio/ tests/audio_soundpack_test.rs`
  - `git commit -m "feat(audio): generate and verify 35-token vietnamese studio soundpack"`

---

### Task 2: Hiện Thực Audio Tokenizer trong `office-engine`

**Files:**
- Modify: `crates/office-engine/src/vn_office/number_to_words.rs`
- Test: `crates/office-engine/src/vn_office/number_to_words.rs` (bổ sung unit test)

- [ ] **Step 2.1: Viết failing test cho `words_to_audio_tokens`**
  - Bổ sung `test_words_to_audio_tokens_mapping` kiểm tra chuyển đổi các chuỗi số sang danh sách audio token IDs:
    - `"Một triệu hai trăm năm mươi nghìn đồng chẵn"` -> `["mot", "trieu", "hai", "tram", "nam", "muoi_chuc", "nghin", "dong", "chan"]`
    - `"Một trăm lẻ một đồng"` -> `["mot", "tram", "le", "mot", "dong"]`
    - `"Hai mươi mốt"` -> `["hai", "muoi_chuc", "mot_sau"]`
    - `"Hai mươi lăm"` -> `["hai", "muoi_chuc", "lam"]`
    - `"Hai mươi tư"` (Bắc) -> `["hai", "muoi_chuc", "tu"]`
    - `"Hai mươi bốn"` (Nam) -> `["hai", "muoi_chuc", "bon"]`
    - `"Âm năm mươi nghìn đồng"` -> `["am", "nam", "muoi_chuc", "nghin", "dong"]`
  - Chạy `cargo test -p office-engine test_words_to_audio_tokens` để xác nhận test fail.

- [ ] **Step 2.2: Hiện thực hàm `words_to_audio_tokens` trong `number_to_words.rs`**
  - Triển khai hàm `pub fn words_to_audio_tokens(words_str: &str, dialect: Dialect) -> Vec<&'static str>`.
  - Phân tích từ và ngữ cảnh (ví dụ `mốt` sau hàng chục -> `mot_sau`, `mười` vs `mươi` -> `muoi` vs `muoi_chuc`).

- [ ] **Step 2.3: Chạy test và xác nhận pass**
  - Chạy `cargo test -p office-engine vn_cccd_mst_validator` và `cargo test -p office-engine test_words_to_audio_tokens`.
  - Chạy `./scripts/verify_pod.sh vietnamese-number-to-words`.

- [ ] **Step 2.4: Commit Task 2**
  - `git commit -am "feat(vn-office): implement words_to_audio_tokens mapping engine with TDD"`

---

### Task 3: Xây Dựng Trình Phát Web Audio Scheduler trong Leptos

**Files:**
- Create: `src/shared_ui/vn_number_audio_player.rs`
- Modify: `src/shared_ui/mod.rs`
- Modify: `Cargo.toml` (nếu cần bật thêm feature của `web-sys`)

- [ ] **Step 3.1: Kiểm tra cấu hình `web-sys` trong `Cargo.toml`**
  - Đảm bảo `web-sys` kích hoạt các feature: `"AudioContext"`, `"AudioBuffer"`, `"AudioBufferSourceNode"`, `"AudioDestinationNode"`.

- [ ] **Step 3.2: Hiện thực `VnNumberAudioPlayer`**
  - Quản lý Web Audio API `AudioContext`.
  - Cơ chế cache `HashMap<String, AudioBuffer>` từ đường dẫn `/audio/vn_numbers/{token}.mp3`.
  - Hàm `play_tokens(tokens: Vec<String>, on_ended: Callback<()>)`:
    - Giải mã và lập lịch phát liên tiếp với gap 40ms giữa các từ, 120ms ở các mốc `trieu`, `ty`, `nghin`/`ngan`.
  - Hàm `stop()`: ngắt ngay lập tức toàn bộ node đang phát.

- [ ] **Step 3.3: Viết test cho Audio Player state logic**
  - Viết test trong `tests/shared_ui_test.rs` kiểm tra quản lý trạng thái phát và ngắt lệnh.

- [ ] **Step 3.4: Commit Task 3**
  - `git add src/shared_ui/vn_number_audio_player.rs src/shared_ui/mod.rs`
  - `git commit -m "feat(ui): implement zero-gap web audio scheduler for vietnamese number readout"`

---

### Task 4: Tích Hợp Nút Điều Khiển Âm Thanh Vào Giao Diện Công Cụ

**Files:**
- Modify: `src/tools/vietnamese_number_to_words_view.rs`
- Modify: `crates/office-ui/src/tools/vietnamese_number_to_words_view.rs`
- Modify: `styles/vn_office.css`

- [ ] **Step 4.1: Bổ sung nút `🔊 Nghe đọc số` / `⏹ Dừng đọc`**
  - Trong `VietnameseNumberToWordsView`, đặt nút phát âm thanh công thái học cạnh khung kết quả `"KẾT QUẢ ĐỌC SỐ CHUẨN"`.
  - Thêm hiệu ứng sóng âm nhấp nháy nhẹ (soundwave animation) khi `is_playing == true`.
  - Lắng nghe phím tắt `Ctrl + Shift + S` hoặc `Space` khi không focus vào input để bật/tắt đọc.

- [ ] **Step 4.2: Đồng bộ trong `crates/office-ui`**
  - Cập nhật phiên bản tương ứng trong `crates/office-ui/src/tools/vietnamese_number_to_words_view.rs`.

- [ ] **Step 4.3: Xác thực Design Tokens & Ergonomic Floor**
  - Chạy `make verify-ui` để đảm bảo 100% tokens hợp lệ, font-size >= 12px.

- [ ] **Step 4.4: Commit Task 4**
  - `git commit -am "feat(vn-office): integrate audio speak button and soundwave indicator into UI"`

---

### Task 5: Kiểm Định Toàn Diện Hệ Thống (E2E & Verification)

- [ ] **Step 5.1: Chạy Spine Read-Only Guard**
  - `./scripts/verify_spine_lock.sh`

- [ ] **Step 5.2: Chạy UI Design Tokens Guard**
  - `make verify-ui`

- [ ] **Step 5.3: Chạy Pod Verification**
  - `./scripts/verify_pod.sh vietnamese-number-to-words`

- [ ] **Step 5.4: Kiểm thử trực tiếp trên Chrome DevTools**
  - Mở `http://localhost:1420`, nhập số `1250000`, chuyển đổi và nhấn nút `🔊 Nghe đọc số`.
  - Xác nhận âm thanh phát ra không bị lỗi console và trạng thái nút chuyển thành `⏹ Dừng đọc`.
  - Chụp ảnh màn hình lưu vào `screenshots/audio_readout_success.png`.
