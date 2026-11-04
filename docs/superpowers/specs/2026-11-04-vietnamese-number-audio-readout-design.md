# Thiết Kế Kỹ Thuật: Phát Âm Thanh Đọc Số Tiền Chuẩn (Vietnamese Number Audio Readout)

**Mã tài liệu:** `SPEC-2026-11-04-VNO-AUDIO`  
**Công cụ áp dụng:** `vietnamese-number-to-words` (Micro App: **VN Office**)  
**Trạng thái:** Chờ phê duyệt (Pending Review)

---

## 1. Bối cảnh & Mục đích (Intent & The Why)
Trong nghiệp vụ kế toán, thủ quỹ, giao dịch viên ngân hàng và văn phòng hành chính, việc kiểm tra chéo (eye-free cross-check) số tiền trên chứng từ giấy (hợp đồng, hóa đơn, ủy nhiệm chi, phiếu chi) với hệ thống là thao tác diễn ra hàng ngày. 
Việc bổ sung khả năng phát âm thanh (Audio Readout) đọc to chính xác số tiền mà người dùng đã nhập giúp:
1. **Kiểm tra chéo không cần rời mắt:** Người dùng vừa nhìn vào hóa đơn giấy vừa nghe máy đọc to rõ ràng từng từ để phát hiện sai lệch ngay lập tức.
2. **Nâng cao tính tiếp cận (Accessibility):** Hỗ trợ người có thị lực yếu hoặc làm việc lâu với màn hình máy tính giảm mỏi mắt.

---

## 2. Mục tiêu, Giới hạn & Chống mục tiêu (Goals, Non-Goals, Anti-Goals)

### Mục tiêu (Goals)
* Phát âm thanh đọc chuẩn xác con số mà người dùng đã nhập và định dạng thành chữ tiếng Việt trong công cụ `vietnamese-number-to-words`.
* Áp dụng kỹ thuật **Ghép mẫu âm thanh chuyên dụng (Audio Stitching / Concatenative Audio)** từ ~35 mẫu âm studio AI chuẩn tiếng Việt.
* Hỗ trợ chuẩn xác cả 2 phương ngữ Bắc và Nam (`nghìn` vs `ngàn`, `lẻ` vs `linh`, `tư` vs `bốn`).
* Đọc đúng hậu tố kế toán tài chính (`đồng chẵn`, `đô la Mỹ và xu`, số âm `âm ...`).
* Nút điều khiển công thái học `🔊 Nghe đọc số (Space)` / `⏹ Dừng đọc` ngay trên khung kết quả.
* 100% On-Device, hoạt động hoàn toàn offline, 0ms latency mạng.

### Giới hạn (Non-Goals)
* Không xây dựng bộ đọc Text-to-Speech (TTS) tổng quát cho toàn bộ văn bản tùy ý (chỉ tập trung tối ưu đọc số tiền tài chính).
* Không nhúng các mô hình deep learning TTS nặng hàng chục MB (như Piper ONNX, VITS) gây hao tốn CPU/RAM máy tính.

### Chống mục tiêu (Anti-Goals)
* Tuyệt đối không gọi Cloud API bên ngoài (NFR-SEC-01 Zero Network Egress).
* Không để xảy ra hiện tượng méo tiếng, khựng (lag), hoặc khoảng lặng quá dài giữa các từ khi phát.

---

## 3. Các hướng tiếp cận & Lựa chọn (Approaches Considered)

| Tiêu chí | Hướng 1: Web Speech API (`speechSynthesis`) | Hướng 2: On-device Deep Learning (Piper TTS) | Hướng 3: Ghép mẫu âm chuyên dụng (Audio Stitching) **[ĐÃ CHỌN]** |
| :--- | :--- | :--- | :--- |
| **Dung lượng thêm** | 0 MB | 40 MB – 60 MB | ~350 KB – 500 KB |
| **Tính nhất quán OS** | Kém (phụ thuộc OS có cài voice `vi-VN` hay không; Linux thường thiếu) | Cao | Tuyệt đối 100% (chạy giống nhau trên Windows, Linux, macOS) |
| **Độ tự nhiên giọng đọc số** | Trung bình (giọng máy đọc văn bản chung) | Khá | Cao (ngắt nhịp dứt khoát như tổng đài ngân hàng / ATM) |
| **Mức độ phức tạp & CPU** | Rất thấp | Cao (tốn CPU render âm thanh) | Rất thấp (chỉ lập lịch phát buffer) |
| **Hỗ trợ Bắc / Nam** | Rất khó điều khiển | Cần 2 model riêng biệt | Rất dễ dàng (2 soundpack âm thanh chuẩn) |

**Quyết định:** Chọn **Hướng 3 (Ghép mẫu âm chuyên dụng)** theo đúng thống nhất trong phiên thảo luận.

---

## 4. Chi tiết Kiến trúc Hiện thực (Architecture & Implementation)

### 4.1. Bộ mẫu âm thanh Soundpack (`public/audio/vn_numbers/`)
Bao gồm ~35 file audio nén MP3/WebM (16kHz mono, chuẩn hóa âm lượng chuẩn EBU R128 / LUFS -16dB):
* **Chữ số (10 files):** `khong.mp3`, `mot.mp3`, `hai.mp3`, `ba.mp3`, `bon.mp3`, `nam.mp3`, `sau.mp3`, `bay.mp3`, `tam.mp3`, `chin.mp3`.
* **Biến âm ngữ pháp (3 files):** `mot_sau.mp3` (dành cho `mốt`), `tu.mp3` (dành cho `tư`), `lam.mp3` (dành cho `lăm`).
* **Hàng số (7 files):** `muoi.mp3` (hàng chục `10`), `muoi_chuc.mp3` (hàng chục `mươi`), `tram.mp3`, `nghin.mp3`, `ngan.mp3` (Nam), `trieu.mp3`, `ty.mp3`.
* **Từ nối & Tiền tệ (7 files):** `le.mp3`, `linh.mp3` (Nam), `am.mp3`, `va.mp3`, `dong.mp3`, `chan.mp3`, `xu.mp3`.

### 4.2. Bộ phân tách âm (Audio Tokenizer)
Hàm ánh xạ chuỗi kết quả văn bản hoặc danh sách từ ngữ pháp từ `format_vietnamese_number` thành chuỗi key âm thanh:
* Ví dụ: `105.000` (Bắc) -> `["mot", "tram", "le", "nam", "nghin", "dong"]`
* Ví dụ: `105.000` (Nam) -> `["mot", "tram", "linh", "nam", "ngan", "dong"]`
* Ví dụ: `1.250.000` (Bắc, chẵn) -> `["mot", "trieu", "hai", "tram", "nam", "muoi_chuc", "nghin", "dong", "chan"]`

### 4.3. Trình phát Web Audio Scheduler (`AudioContext`)
* Nạp sẵn các `AudioBuffer` vào bộ nhớ RAM khi người dùng mở công cụ (kích thước nạp chưa tới 1MB RAM).
* Sử dụng Web Audio API để lập lịch phát nối tiếp chính xác đến mili-giây:
  * Khoảng nghỉ giữa 2 từ đơn: **40ms**.
  * Khoảng nghỉ tại các mốc phân tách hàng nghìn, triệu, tỷ: **120ms**.
* Quản lý trạng thái phát:
  * `is_playing: RwSignal<bool>`: chuyển đổi trạng thái nút bấm `🔊 Nghe đọc số` <-> `⏹ Dừng`.
  * Hủy tác vụ phát tức thì khi người dùng bấm Dừng hoặc nhấn phím `Esc` / `Space`.

### 4.4. Giao diện Người dùng (UI View)
Trong [`src/tools/vietnamese_number_to_words_view.rs`](file:///home/quan/workspace/tauri-v2/office-utility-vn/src/tools/vietnamese_number_to_words_view.rs):
* Thêm nút điều khiển âm thanh công thái học cạnh kết quả:
  ```html
  <button class="btn-audio-speak" on:click=toggle_speak>
      {if is_playing { "⏹ Dừng đọc" } else { "🔊 Nghe đọc số" }}
  </button>
  ```
* Bổ sung phím tắt: Nhấn phím `Space` khi không nhập liệu hoặc `Ctrl + Shift + S` để bật/tắt phát âm thanh nhanh chóng.
* Tuân thủ 100% Design Tokens và Ergonomic Floor (`make verify-ui`).

---

## 5. Những điểm ủy quyền cho Kỹ sư Hiện thực (Left to Builder)
1. **Lựa chọn định dạng mẫu âm thanh:** Đóng gói bằng định dạng MP3 hay WebM/Opus đảm bảo tương thích tốt nhất trên cả WebKitGTK (Linux) và WebView2 (Windows).
2. **Khoảng ngắt âm (Audio Gap Tuning):** Tinh chỉnh từ 30ms đến 60ms để âm điệu tự nhiên, nghe rõ ràng và dễ chịu nhất.
