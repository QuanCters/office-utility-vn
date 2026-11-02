# 5. Đặc tả Thiết kế Giao diện & Trải nghiệm Người dùng (UI/UX Design Specification)

---

Thiết kế giao diện của **Office Utility VN** tuân thủ triết lý **"Zero-Friction, Keyboard-Centric & Native Desktop Integration"** (Không rào cản, Lấy bàn phím làm trung tâm và Tích hợp sâu vào Hệ điều hành): loại bỏ tối đa các thao tác điều hướng thừa thãi, đảm bảo tính phản hồi tức thì với tốc độ khung hình 60 FPS thông qua giao diện phản ứng **Leptos 0.8** (Rust CSR WebAssembly) kết hợp khung cửa sổ tùy biến không viền của **Tauri v2** (`decorations: false`).

---

## 5.1. Nguyên tắc Thiết kế Cốt lõi & Hệ thống Thiết kế (Design System Tokens)

### 5.1.1. 4 Nguyên tắc UX Bắt buộc

1. **Minh bạch An toàn Dữ liệu (100% On-Device Trust):** Người dùng luôn được xác nhận rằng dữ liệu tệp được xử lý hoàn toàn cục bộ trên máy tính cá nhân thông qua huy hiệu bảo mật xanh lá ngay trên thanh tiêu đề:
   * 🟢 **`[⚡ Xử lý Cục bộ - 100% On-Device]`**: Khẳng định dữ liệu không rời khỏi máy tính, hoạt động hoàn hảo khi ngắt mạng.
2. **Tối ưu hóa Thao tác Bàn phím & Chặn Phím tắt Trình duyệt (Keyboard-First & WebView Interception):**
   * Mọi tác vụ từ tìm công cụ (`Ctrl + K` hoặc phím tắt toàn hệ điều hành `Ctrl + Shift + K`), dán dữ liệu (`Ctrl + V`), thực thi (`Ctrl + Enter`), bật ghim cửa sổ (`Ctrl + P`), thu xuống khay hệ thống (`Ctrl + W`) đến mở thư mục kết quả (`Ctrl + O`) đều có thể hoàn tất mà không cần chuột.
   * **Cơ chế chặn phím tắt WebView mặc định:** Trình lắng nghe sự kiện bàn phím toàn cục trong Leptos chủ động gọi `event.prevent_default()` và `event.stop_propagation()` đối với các tổ hợp `Ctrl + P`, `Ctrl + W`, `Ctrl + O`, `Ctrl + F`, `Ctrl + R`, `F5` để ngăn WebView2/WebKit mở hộp thoại in trang web hoặc tự đóng khung hiển thị.
3. **Phản hồi Không Khóa Luồng Giao diện (Non-blocking 60 FPS Feedback):**
   * Giữ vững tốc độ khung hình 60 FPS trên Main UI Thread. Mọi tiến trình chạy ngầm qua Rayon/Tokio đều phát sự kiện tiến độ chi tiết qua kênh `tauri::ipc::Channel` và hiển thị thanh tiến trình Stepper Progress thay vì vòng xoay bất định.
4. **Hỗ trợ Chế độ Sáng/Tối & An toàn Bộ gõ Tiếng Việt (Adaptive Theme & IME-Safe):**
   * Tương thích hoàn toàn với chế độ `Light / Dark Mode` theo hệ điều hành hoặc người dùng chọn.
   * Sử dụng signal `is_ime_composing` theo dõi sự kiện `compositionstart` và `compositionend` trên mọi thẻ `<input>` và `<textarea>` để tuyệt đối không làm đứt quãng quá trình bỏ dấu tiếng Việt trên các bộ gõ UniKey, EVKey, OpenKey, macOS IME.

### 5.1.2. Hệ thống Màu sắc Ngữ nghĩa & Kiểu chữ (Color & Typography Tokens)

Giao diện sử dụng **Tailwind CSS** kết hợp bộ biến màu ngữ nghĩa chuẩn hóa:

| Nhóm Token | Biến CSS / Tailwind | Mã màu (Light / Dark) | Mục đích Sử dụng Ngữ nghĩa |
| --- | --- | --- | --- |
| **Primary Brand** | `--primary` (`indigo-600`) | `#4F46E5` / `#6366F1` | Nút hành động chính (CTA), bước đang kích hoạt, viền vùng Dropzone khi kéo tệp vào. |
| **On-Device Trust** | `--emerald` (`emerald-600`) | `#059669` / `#10B981` | Nhãn bảo mật *"100% On-Device"*, cổng kết nối DAG hợp lệ, thông báo xử lý thành công. |
| **Native Execution** | `--sky` (`sky-600`) | `#0284C7` / `#38BDF8` | Thanh tiến độ xử lý đa luồng, thông báo trạng thái luồng nền Rayon, huy hiệu công cụ hệ thống. |
| **Danger / Alert** | `--rose` (`rose-600`) | `#E11D48` / `#F43F5E` | Nút *"Hủy tác vụ"*, cảnh báo lỗi chữ ký tệp Magic Bytes, cổng kết nối DAG không tương thích MIME. |
| **Warning** | `--amber` (`amber-500`) | `#D97706` / `#F59E0B` | Cảnh báo tệp có dung lượng vượt ngưỡng DOM khuyến nghị, thông báo kết quả `PARTIAL_SUCCESS`. |
| **Workflow / Macro** | `--purple` (`purple-600`) | `#9333EA` / `#A855F7` | Nút *"Chạy Pipeline"*, đường nối liên kết trong Workflow DAG Builder, chế độ ghim Always-on-Top. |
| **Typography** | `--font-sans` & `--font-mono` | **Inter** & **JetBrains Mono** | `Inter` hiển thị tiếng Việt hoàn hảo; `JetBrains Mono` dùng cho bảng tính, đường dẫn tệp, mã băm SHA-256. |

---

## 5.2. Bố cục Chuẩn của Cửa sổ Ứng dụng (Unified Desktop Workspace)

Ứng dụng sử dụng một khung cửa sổ tùy biến không viền hệ điều hành (**Custom Frameless Window Titlebar** với thuộc tính `data-tauri-drag-region`) kết hợp bố cục 3 vùng tiêu chuẩn (**Unified 3-Zone Workspace Layout**) giúp toàn bộ hơn 55 công cụ duy trì sự đồng nhất 100%.

### 5.2.1. Sơ đồ Khung Bố cục Cửa sổ Desktop (Wireframe Layout)

```text
+-----------------------------------------------------------------------------------------------------------------------+
| [Logo] Office Utility VN - v0.1.0   [ 🔍 Tìm công cụ... (Ctrl + K) ]     (data-tauri-drag-region)  [📌 Ghim] [🗕] [🗖] [✕] |
+-----------------------------------------------------------------------------------------------------------------------+
| 📌 Ghim nhanh: [Ghép PDF] [Nén Ảnh] [Universal Hub] [VietQR] [Đọc Số Tiền] [⚡ Pipeline DAG]            [⚙️ Cài đặt] |
+-----------------------------------------------------------------------------------------------------------------------+
|                                                                                                                       |
|  [ VÙNG 1: TIÊU ĐỀ CÔNG CỤ, HUY HIỆU & QUẢN LÝ PRESET ]                                                               |
|  TIÊU ĐỀ: Nén & Đổi Định dạng Ảnh Hàng loạt                                                                           |
|  Huy hiệu: 🟢 100% On-Device Native (Rayon) | 🎛️ Mẫu Preset: [ Nén ảnh Web 85% ▼ ] [💾 Lưu mẫu] | ⏱️ Lịch sử         |
|                                                                                                                       |
|-----------------------------------------------------------------------------------------------------------------------|
|                                                                                                                       |
|  [ VÙNG 2: VÙNG KÉO THẢ TỆP & CẤU HÌNH THAM SỐ ]                                                                      |
|                                                                                                                       |
|  +-----------------------------------------------------------------------------------------------------------------+  |
|  |                              📥 KÉO THẢ TỆP TỪ EXPLORER/FINDER VÀO ĐÂY HOẶC NHẤN ĐỂ DUYỆT                      |  |
|  |                                  (Hỗ trợ dán ảnh trực tiếp từ Clipboard qua Ctrl + V)                           |  |
|  +-----------------------------------------------------------------------------------------------------------------+  |
|                                                                                                                       |
|  Cấu hình:                                                                                                            |
|  [ Định dạng đích: (•) WebP  ( ) AVIF ( ) JPG ]   [ Mức chất lượng: [==== 85% ====] ]    [ Xóa sạch EXIF/GPS: [x] ]   |
|  [ Thư mục lưu: [ Thư mục cùng cấp với tệp gốc (hậu tố _compressed)              ] [📂 Đổi thư mục...] ]               |
|                                                                                                                       |
|-----------------------------------------------------------------------------------------------------------------------|
|                                                                                                                       |
|  [ VÙNG 3: THANH TIẾN TRÌNH & NÚT ĐIỀU KHIỂN ]                                                                        |
|                                                                                                                       |
|  Tiến độ: [======================================== 75% =======================================] 75/100 tệp (0.8s)    |
|                                                                                                                       |
|  [ ⚡ THỰC THI NGAY (Ctrl + Enter) ]      [ 🛑 HỦY (Esc) ]      [ 📂 MỞ THƯ MỤC KẾT QUẢ (Ctrl + O) ]                   |
|                                                                                                                       |
+-----------------------------------------------------------------------------------------------------------------------+
```

### 5.2.2. Chi tiết 3 Vùng Chức năng

1. **Vùng 1 (Tool Header, Trust Badge & Preset Selector):**
   * Hiển thị biểu tượng và tên công cụ bằng tiếng Việt.
   * Huy hiệu cam kết bảo mật: `[⚡ 100% On-Device Native]`.
   * **Bộ chọn Cấu hình Mẫu (Preset Dropdown):** Cho phép chọn nhanh các Preset đã lưu trong bảng `tool_presets` (ví dụ: *Nén ảnh Báo cáo 75%*, *Nén ảnh Web 85%*) hoặc nhấn `[💾 Lưu mẫu]` để tạo Preset mới.
   * Nút ghim công cụ lên thanh công cụ nhanh và nút bật chế độ cửa sổ nổi (**Always-on-Top Pin**).
2. **Vùng 2 (Interactive Dropzone & Configuration Panel):**
   * Vùng kéo-thả tệp tương tác trực tiếp với sự kiện kéo thả tệp bản địa của hệ điều hành (`tauri://drag-drop`).
   * Ngay khi tệp được thả vào, gọi lệnh `inspect_file_metadata` để kiểm tra chữ ký Magic Bytes và hiển thị danh sách tệp kèm dung lượng, định dạng thực tế và cảnh báo nếu tệp không hợp lệ.
   * Bảng tham số tùy biến tinh gọn theo từng công cụ; đối với các công cụ pháp lý Việt Nam (như Tính lương Gross/Net, Thuế TNCN), có thêm nút `[⚙️ Tùy chỉnh biểu thuế / mức trần]` liên kết trực tiếp với bảng `legal_rulesets`.
3. **Vùng 3 (Action Bar & Progress Stepper):**
   * Thanh tiến trình thời gian thực nhận dữ liệu từ `tauri::ipc::Channel<TaskProgress>`, hiển thị số lượng tệp đã xử lý, số tệp lỗi (nếu có) và thời gian thực thi.
   * Nút bấm hành động chính (CTA) với phím tắt gợi ý `Ctrl + Enter`.
   * Nút mở trực tiếp thư mục kết quả trong Windows Explorer hoặc macOS Finder (`show_item_in_folder`).

---

## 5.3. Luồng Tương tác "Clipboard-First" (`Ctrl + V`) & Command Palette (`Ctrl + K`)

### 5.3.1. Hộp Tìm kiếm Toàn năng (Command Palette)

* **Kích hoạt:** Nhấn tổ hợp phím `Ctrl + K` (hoặc `Cmd + K` trên macOS) khi ứng dụng đang mở, hoặc nhấn phím tắt toàn cầu **`Ctrl + Shift + K`** khi đang ở bất kỳ phần mềm nào trong hệ điều hành.
* **Tốc độ:** Hộp thoại tìm kiếm xuất hiện trong vòng **dưới 50 ms**.
* **Tính năng:**
  * Tìm kiếm mờ (Fuzzy Search) hỗ trợ gõ tiếng Việt có dấu hoặc không dấu (ví dụ: gõ `ghep pdf`, `vietqr`, `doc tien` đều gợi ý chính xác công cụ tương ứng).
  * Hiển thị nhóm công cụ theo chuyên mục, các Preset đã lưu và các công cụ vừa sử dụng gần đây.

### 5.3.2. Bộ Nhận diện Nội dung Bộ nhớ tạm (Clipboard Monitor)

* Khi người dùng nhấn `Ctrl + V` tại màn hình chính hoặc thanh Command Palette:
  * Nếu Clipboard chứa **Hình ảnh (Bitmap):** Tự động gợi ý nạp ảnh vào *Làm đẹp ảnh chụp màn hình*, *Nén & Đổi định dạng ảnh* hoặc *OCR Tiếng Việt*.
  * Nếu Clipboard chứa **Bảng tính sao chép từ Excel (Tab-separated TSV):** Tự động gợi ý *Làm sạch & Chuẩn hóa Bảng tính* hoặc *Chuyển đổi Cấu trúc Dữ liệu (Excel sang JSON/Markdown)*.
  * Nếu Clipboard chứa **Dãy số tiền hoặc số CCCD/MST:** Tự động hiển thị ngay kết quả *Đọc số tiền thành chữ* hoặc *Kiểm tra CCCD/MST* ngay trong hộp thoại xem trước.
  * Nếu Clipboard chứa **Văn bản dính lỗi font TCVN3/VNI:** Tự động điều hướng đến công cụ *Chuyển mã Font Tiếng Việt*.

---

## 5.4. Giao diện Kéo-Thả Chuỗi Tự động hóa Có Kiểm tra Kiểu MIME (Typed Workflow Pipeline Builder UI)

Tính năng **Workflow Pipeline** cho phép người dùng ghép nối nhiều công cụ độc lập thành chuỗi tự động hóa (Macro DAG) với cơ chế **kiểm tra tương thích cổng MIME-type trực quan**:

```text
+-----------------------------------------------------------------------------------------------------------------------+
|  [Quay lại]   ⚡ THIẾT KẾ CHUỖI TỰ ĐỘNG HÓA: "Chuẩn hóa Ảnh Bài viết Web"                 [▶ Chạy Chuỗi] [💾 Lưu]    |
+-----------------------------------------------------------------------------------------------------------------------+
|                                                                                                                       |
|   +-----------------------+          +------------------------+          +------------------------+                   |
|   | 1. Đổi định dạng ảnh  |          | 2. Đóng dấu Watermark  |          | 3. Xóa sạch EXIF/GPS   |                   |
|   | [In: image/*]         | =======> | [In: image/webp,png]   | =======> | [In: image/webp,jpg]   |                   |
|   | [Out: image/webp]  🟢 | (Hợp lệ) | [Out: image/webp]   🟢 | (Hợp lệ) | [Out: image/webp]   🟢 |                   |
|   +-----------------------+          +------------------------+          +------------------------+                   |
|                                                                                                                       |
|   + Thêm bước công cụ mới vào chuỗi... (Hệ thống tự động lọc các công cụ nhận đầu vào là [image/webp])                |
|                                                                                                                       |
+-----------------------------------------------------------------------------------------------------------------------+
```

* **Kiểm tra Tương thích Cổng Kết nối (MIME-Type Port Snapping):**
  * Mỗi Node công cụ hiển thị rõ nhãn định dạng đầu vào (`[In: ...]`) và đầu ra (`[Out: ...]`).
  * Khi người dùng kéo dây nối từ Node A sang Node B, nếu tập hợp `NodeA.output_mimes` giao với `NodeB.input_mimes` khác rỗng, đường nối hiển thị **màu xanh ngọc (`emerald-600`)**. Nếu không tương thích (ví dụ nối đầu ra `.mp3` vào đầu vào `.xlsx`), đường nối chuyển sang **màu đỏ (`rose-600`)** và tự động từ chối kết nối.
* **Kiểm tra Chu trình (Cycle Detection):** Tự động ngăn chặn việc tạo vòng lặp vô hạn trước khi lưu vào bảng `workflows` trong SQLite.

---

## 5.5. Tích hợp Khay Hệ thống & Cửa sổ Nổi Tiện ích (System Tray & Always-on-Top Pin)

### 5.5.1. Menu Ngữ cảnh Khay Hệ thống (System Tray)
* **Nhấp chuột trái vào biểu tượng Tray:** Ẩn/Hiện cửa sổ chính tức thì ($< 100\text{ ms}$).
* **Nhấp chuột phải vào biểu tượng Tray hiển thị Menu ngữ cảnh:**
  * 📋 *Xử lý nhanh nội dung từ Clipboard...*
  * 💰 *Đọc nhanh số tiền thành chữ (Mini Pin)*
  * 🏦 *Tạo nhanh mã VietQR (Mini Pin)*
  * 🧮 *Tính nhanh Lương Gross / Net (Mini Pin)*
  * ─────────────
  * 🗔 *Hiện cửa sổ chính*
  * ✕ *Thoát ứng dụng hoàn toàn*

### 5.5.2. Chế độ Cửa sổ Nổi Tiện ích (Always-on-Top Floating Mini-Tools)
* Khi người dùng nhấn biểu tượng **`[📌 Ghim]`** (`Ctrl + P`) trên thanh tiêu đề của các công cụ tra cứu vi mô:
  * Ứng dụng gọi Tauri Window API thu gọn kích thước cửa sổ về chế độ Compact ($380 \times 520\text{ px}$) và bật `window.set_always_on_top(true)`.
  * Thanh công cụ nhanh được ẩn tạm thời để tối ưu diện tích hiển thị, giúp kế toán viên hoặc nhân sự vừa mở phần mềm MISA / Excel vừa đối chiếu số liệu song song.

---

## 5.6. Bảng Ma trận Phím tắt Toàn cục (Global Keyboard Shortcuts Matrix)

| Tổ hợp Phím tắt | Ngữ cảnh Kích hoạt | Hành động Thực thi & Xử lý Sự kiện |
| --- | --- | --- |
| `Ctrl + Shift + K` (hoặc `Cmd + Shift + K`) | **Toàn hệ điều hành (Global OS)** | Đưa cửa sổ ứng dụng lên trên cùng, lấy tiêu điểm (focus) và mở ngay thanh tìm kiếm Command Palette. |
| `Ctrl + K` (hoặc `Cmd + K`) | **Trong cửa sổ ứng dụng** | `preventDefault()` + Mở hộp tìm kiếm nhanh công cụ và Preset. |
| `Ctrl + V` (hoặc `Cmd + V`) | **Ngoài ô nhập văn bản** | Tự động phân tích nội dung Clipboard và nạp dữ liệu vào công cụ tương ứng. |
| `Ctrl + Enter` (hoặc `Cmd + Enter`) | **Tại màn hình công cụ** | Kích hoạt thực thi tác vụ xử lý ngay lập tức (`dispatch_tool_task`). |
| `Esc` | **Khi đang mở modal / chạy tác vụ** | Đóng hộp thoại tìm kiếm hoặc gửi tín hiệu `cancel_task` hủy tác vụ đang chạy. |
| `Ctrl + O` (hoặc `Cmd + O`) | **Sau khi tác vụ hoàn thành** | `preventDefault()` + Mở thư mục chứa tệp kết quả trong Explorer / Finder. |
| `Ctrl + P` (hoặc `Cmd + P`) | **Tại màn hình công cụ** | `preventDefault()` (chặn hộp thoại Print của WebView) + Bật/Tắt chế độ ghim cửa sổ nổi Always-on-Top. |
| `Ctrl + S` (hoặc `Cmd + S`) | **Tại màn hình công cụ** | `preventDefault()` + Mở hộp thoại lưu cấu hình hiện tại thành Preset mới. |
| `Ctrl + W` (hoặc `Cmd + W`) | **Trong cửa sổ ứng dụng** | `preventDefault()` + Ẩn cửa sổ ứng dụng xuống khay hệ thống (System Tray). |
| `Ctrl + Q` (hoặc `Cmd + Q`) | **Trong cửa sổ ứng dụng** | Dọn dẹp thư mục cache tạm thời và thoát ứng dụng hoàn toàn. |