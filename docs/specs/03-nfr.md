# 3. Đặc tả Yêu cầu Phi chức năng (Non-Functional Requirements - NFR)

---

Đối với một ứng dụng tiện ích máy tính để bàn thuần Rust (**Tauri v2 + Leptos 0.8**) phục vụ hơn 55 công cụ xử lý tệp tin chuyên sâu, các chỉ số kỹ thuật phi chức năng (SLAs & SLOs) tập trung vào: **Tốc độ khởi động siêu tốc theo từng hệ điều hành**, **Mức tiêu thụ bộ nhớ RAM được kiểm soát chặt chẽ**, **Tính an toàn tuyệt đối khi xử lý tại chỗ (100% On-Device)** và **Độ mượt mà của giao diện đa luồng (60 FPS Main Thread)**.

---

## 3.1. Yêu cầu về Hiệu năng & Tốc độ Phản hồi (Performance & Latency)

Ứng dụng phải đảm bảo sự mượt mà tuyệt đối ở giao diện người dùng (UI) ngay cả khi đang thực thi các tác vụ xử lý hàng trăm tệp tin nặng nề trên các luồng CPU chạy ngầm.

| Mã NFR | Hạng mục Hiệu năng | Chỉ số Mục tiêu (Target Metric / SLA) | Giải pháp Kỹ thuật Đảm bảo Chuẩn Production |
| --- | --- | --- | --- |
| **NFR-PERF-01** | **Thời gian Khởi chạy & Kích thước Bộ nhớ (Startup & Footprint)** | • **Khởi động lạnh (Cold Start):** Dưới **450 ms** trên macOS/Linux (WebKit) và dưới **750 ms** trên Windows (bao gồm thời gian khởi tạo tiến trình `WebView2`).<br>• **Khởi động nóng từ System Tray (Warm Start):** Dưới **100 ms**.<br>• **RAM lúc nghỉ (Idle Memory):** Dưới **80 MB**.<br>• **Kích thước bộ cài cốt lõi (Core Installer Size):** Dưới **15 MB** (Windows `.exe`), dưới **25 MB** (macOS Universal `.dmg`). | • Sử dụng **Tauri v2** tận dụng WebView có sẵn của hệ điều hành, loại bỏ hoàn toàn runtime Chromium/Node.js nặng hàng trăm MB của Electron.<br>• Bật cờ tối ưu hóa kích thước nhị phân trong `Cargo.toml`: `opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`.<br>• **Kiến trúc Gói mở rộng theo yêu cầu (On-Demand Add-ons):** Tận dụng API bản địa của hệ điều hành cho giải mã HEIC/Video và OCR nhanh (0 MB). Các thư viện nhị phân nặng như `pdfium` hoặc mô hình AI VietOCR ONNX (`~12-18 MB`) được tách thành gói mở rộng tải về thư mục `{app_data_dir}/addons/` khi người dùng kích hoạt tính năng nâng cao. |
| **NFR-PERF-02** | **Độ mượt Giao diện Đa luồng (Non-blocking Main Thread UI)** | • Tốc độ khung hình giao diện (Main UI Thread) luôn duy trì ổn định **60 FPS**.<br>• Tuyệt đối không xuất hiện tình trạng treo đơ cửa sổ (*"Application Not Responding"*) khi đang nén ảnh hoặc xuất file lớn. | • **100% tác vụ tính toán nặng** (đọc/ghi PDF, nén ảnh, bóc tách bảng tính, mã hóa) bắt buộc phải được điều phối sang luồng nền (`tokio::task::spawn_blocking` hoặc `rayon::spawn`).<br>• Luồng giao diện Leptos chỉ tiếp nhận các sự kiện tiến độ nhẹ đã được điều tiết (throttled tối đa 10 sự kiện/giây) qua `tauri::ipc::Channel`. |
| **NFR-PERF-03** | **Độ trễ Giao tiếp Nội bộ (IPC Latency & Throughput)** | • Độ trễ gọi lệnh IPC (`invoke` command): **p95 dưới 2 ms**, **p99 dưới 5 ms**.<br>• Tần suất phát tín hiệu tiến độ qua Channel: Đạt ổn định **5–10 sự kiện/giây** mà không làm nghẽn kênh truyền. | • Sử dụng cơ chế truyền tải nhị phân tốc độ cao của Tauri v2 IPC.<br>• Với dữ liệu nhị phân lớn (ảnh, tệp PDF), ứng dụng truyền trực tiếp đường dẫn file (`std::path::PathBuf`) để nhân Rust đọc trực tiếp từ đĩa cứng, triệt tiêu hoàn toàn chi phí tuần tự hóa JSON hay mã hóa Base64 qua cầu nối WebView. |
| **NFR-PERF-04** | **Thông lượng Xử lý Đa lõi (Multi-core Parallel Processing)** | • Tận dụng tối đa số nhân CPU vật lý của máy tính khi xử lý các tác vụ dạng lô (Batch).<br>• Tốc độ nén 100 ảnh hoặc trộn 1.000 hợp đồng Word nhanh hơn **5–10 lần** so với xử lý đơn luồng. | • Tích hợp thư viện **`rayon`** tự động phân chia công việc (Work-stealing threadpool) với số luồng bằng $\max(1, N_{\text{cores}} - 1)$ nhằm chừa lại 1 luồng CPU cho hệ điều hành và giao diện WebView luôn phản hồi tức thì. |
| **NFR-PERF-05** | **Quản lý Bộ nhớ & Phân tầng Xử lý Tệp Lớn (Memory Guardrails)** | • **Với tác vụ Streaming I/O** (CSV, Base64, Hash, Nén lô ảnh, Giải nén ZIP): Dung lượng RAM tiêu thụ luôn duy trì **dưới 250 MB** ngay cả với tệp/lô dữ liệu $> 1\text{ GB}$.<br>• **Với tác vụ In-Memory DOM/AST** (`lopdf` sửa cây đối tượng PDF, `calamine` nạp bảng tính): Giới hạn kích thước tệp đơn lẻ khuyến nghị ở mức $\le 200\text{ MB}$ (RAM đỉnh $< 500\text{ MB}$); tự động cảnh báo hoặc chuyển sang chế độ đọc từng trang (Paged Render) nếu tệp vượt ngưỡng để tuyệt đối không xảy ra lỗi tràn bộ nhớ (Out-of-Memory Panic). | • Áp dụng kỹ thuật đọc ghi dòng dữ liệu (`BufReader`, `BufWriter`, `quick-xml` streaming reader) cho các tệp tuần tự.<br>• Kiểm tra dung lượng RAM vật lý còn trống (`sysinfo`) trước khi cấp phát bộ đệm cho các tệp PDF/Excel lớn. |

---

## 3.2. Yêu cầu về Bảo mật & Quyền riêng tư (Security & Privacy)

Do người dùng thường xuyên xử lý các tài liệu nhạy cảm (bảng lương, hợp đồng pháp lý, CCCD, hóa đơn VAT), ứng dụng được thiết kế theo nguyên tắc **Privacy-by-Design** và tuân thủ nghiêm ngặt **Nghị định 13/2023/NĐ-CP về Bảo vệ dữ liệu cá nhân**.

### 3.2.1. Tuân thủ Quyền riêng tư Dữ liệu Bản địa

* **NFR-SEC-01 (Mô hình 100% On-Device & Zero Data Egress):**
  * Toàn bộ quá trình đọc, chuyển đổi, bóc tách và xuất tệp diễn ra độc quyền trên phần cứng máy tính người dùng.
  * Ứng dụng cam kết **không mở kết nối mạng Internet** cho bất kỳ tác vụ xử lý tài liệu nào. Chỉ có 2 kết nối mạng hợp lệ duy nhất và đều yêu cầu xác thực chữ ký số:
    1. Kiểm tra bản cập nhật ứng dụng/quy tắc pháp lý qua `tauri-plugin-updater` (chỉ tải tệp tĩnh `latest.json` từ GitHub Releases).
    2. Tải gói mở rộng ngoại tuyến theo yêu cầu (`On-Demand Add-on Pack` như mô hình VietOCR ONNX) khi người dùng chủ động nhấn nút tải.
* **NFR-SEC-02 (Cách ly Thư mục Tạm & Tự động Thu dọn - Ephemeral Cache Isolation):**
  * Mọi tệp tin trung gian tạo ra trong quá trình xử lý (ảnh thumbnail, tệp giải nén tạm, kết quả trung gian giữa các bước trong chuỗi Workflow DAG) đều được lưu trữ trong thư mục cache chuyên biệt của ứng dụng (`{app_cache_dir}/temp_jobs/{task_uuid}`).
  * Khi tác vụ kết thúc, bị hủy, hoặc khi ứng dụng khởi động/đóng lại, hệ thống tự động xóa sạch toàn bộ thư mục tạm này.
* **NFR-SEC-03 (Xóa Sạch Bộ Nhớ Nhạy Cảm - In-Memory Scrubbing with Zeroize):**
  * Đối với các tác vụ bảo mật (nhập mật khẩu bảo vệ PDF, giải mã khóa AES), các biến chứa mật khẩu và khóa trong RAM bắt buộc được bọc bởi kiểu `Zeroizing<String>` hoặc `Zeroizing<Vec<u8>>` (từ crate `zeroize`).
  * Ngay khi biến rời khỏi phạm vi (Out of Scope / Drop), vùng nhớ RAM tương ứng sẽ lập tức bị ghi đè bằng các byte `0x00`, triệt tiêu hoàn toàn khả năng đọc trộm mật khẩu qua kỹ thuật Memory Dump.

### 3.2.2. Phòng chống Tệp Độc hại & Giới hạn Đặc quyền

* **NFR-SEC-04 (Xác thực Chữ ký Tệp Nhị phân - Deep Magic Bytes Validation):**
  * Ứng dụng không bao giờ tin tưởng mù quáng vào phần mở rộng tên tệp (`.pdf`, `.jpg`, `.xlsx`, `.docx`).
  * Sử dụng crate **`infer`** đọc 64 byte đầu tiên (File Signature / Magic Bytes) kết hợp kiểm tra cấu trúc nội bộ (ví dụ: tệp `.docx` và `.xlsx` đều có Magic Bytes `PK\x03\x04` của ZIP nên hệ thống sẽ kiểm tra tiếp sự tồn tại của `word/document.xml` hoặc `xl/workbook.xml` bên trong). Nếu chữ ký không khớp, ứng dụng lập tức trả về lỗi `InvalidFileSignature` và từ chối xử lý.
* **NFR-SEC-05 (Chống Tấn công Tệp Nén & Bom Điểm ảnh - Decompression & Image Bomb Defense):**
  * **Kiểm soát tỷ lệ giải nén (Zip Bomb Defense):** Khi đọc các tệp đóng gói ZIP/OpenXML (`.zip`, `.docx`, `.xlsx`), luồng giải nén sử dụng bộ đếm `Read::take()` giới hạn tỷ lệ giải nén không vượt quá **1:100** và tổng dung lượng bung ra không vượt quá **1 GB**.
  * **Giới hạn kích thước ảnh (Image Bomb Defense):** Cấu hình `image::Limits` giới hạn kích thước khung hình tối đa không vượt quá $16.384 \times 16.384\text{ pixels}$ và bộ đệm cấp phát tối đa $512\text{ MB}$ cho mỗi bức ảnh.
* **NFR-SEC-06 (Phân quyền Tối thiểu theo Chuẩn Tauri v2 Capabilities):**
  * Tận dụng hệ thống phân quyền **Tauri v2 Capabilities** (`src-tauri/capabilities/default.json`): Chỉ cấp đúng các quyền cụ thể cho quản lý cửa sổ tùy biến (`core:window:allow-*`), hộp thoại tệp (`dialog:allow-open`, `dialog:allow-save`), bảng nhớ tạm (`clipboard-manager`), phím tắt toàn cục (`global-shortcut`) và trình cập nhật (`updater:default`).
  * Không bật quyền truy cập `fs` (filesystem) toàn cục cho tầng WebView JavaScript/WASM; mọi thao tác đọc/ghi tệp đều đi qua các lệnh Rust `#[tauri::command]` đã được kiểm duyệt đường dẫn.

---

## 3.3. Yêu cầu về Độ tin cậy & Khả năng Chịu lỗi (Reliability & Fault Tolerance)

* **NFR-REL-01 (An toàn Bộ nhớ & Không Đổ vỡ - Memory Safety & Panic Isolation):**
  * Ở tầng WebAssembly (Frontend Leptos), cài đặt `console_error_panic_hook` để bắt và hiển thị thông báo lỗi giao diện thân thiện.
  * Ở tầng Native (Tauri Core), mọi lệnh IPC command đều trả về kiểu `Result<T, CommandError>`. Các tác vụ chạy trong `rayon` threadpool được bọc bởi `std::panic::catch_unwind` tại ranh giới xử lý từng tệp tin để nếu một tệp có cấu trúc dị biệt gây panic trong thư viện bên thứ ba, ứng dụng vẫn không bị sập (Crash) mà chỉ ghi nhận lỗi cho riêng tệp đó.
* **NFR-REL-02 (Khả năng Hủy Tác vụ Tức thì - Cooperative Cancellation):**
  * Mọi tác vụ chạy ngầm kéo dài đều gắn kèm một cờ hiệu hủy tác vụ nguyên tử (`Arc<AtomicBool>` lưu trong `TaskRegistry` của Tauri State).
  * Vòng lặp xử lý trang PDF, dòng Excel hoặc lô ảnh trong `rayon` kiểm tra cờ `is_cancelled.load(Ordering::Relaxed)` trước mỗi đơn vị công việc. Khi người dùng nhấn nút *"Hủy"* (`Esc`), luồng xử lý nền lập tức dừng lại trong vòng **dưới 100 ms**, giải phóng bộ nhớ và xóa sạch các tệp xuất dở dang.
* **NFR-REL-03 (Phục hồi Lỗi Tệp Hỏng trong Xử lý Lô - Batch Partial Success):**
  * Khi xử lý một lô 100 tệp mà có 2 tệp bị hỏng cấu trúc nhị phân, hệ thống cô lập 2 tệp lỗi, hoàn tất 98 tệp hợp lệ còn lại và trả về báo cáo chi tiết danh sách tệp thành công cùng nguyên nhân lỗi của từng tệp thất bại.

---

## 3.4. Yêu cầu về Khả năng Mở rộng & Kiến trúc Mã nguồn (Scalability & Maintainability)

* **NFR-SCA-01 (Kiến trúc Hướng Trait Chuẩn hóa - Typed Modular Tool Trait):**
  * Toàn bộ các công cụ trong ứng dụng tuân thủ nguyên tắc mở/đóng (Open-Closed Principle - OCP) thông qua một Trait Rust chung có định nghĩa cổng MIME-type rõ ràng để phục vụ cả chạy đơn lẻ lẫn ghép chuỗi Workflow DAG:
    ```rust
    use async_trait::async_trait;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    pub struct ToolMetadata {
        pub slug: &'static str,
        pub name: &'static str,
        pub input_mimes: &'static [&'static str],
        pub output_mimes: &'static [&'static str],
    }

    #[async_trait]
    pub trait OfficeTool: Send + Sync {
        type Options: serde::de::DeserializeOwned + Send + Sync;
        type Output: serde::Serialize + Send;

        fn metadata(&self) -> ToolMetadata;

        async fn execute(
            &self,
            input_paths: Vec<std::path::PathBuf>,
            output_dir: std::path::PathBuf,
            options: Self::Options,
            cancel_flag: Arc<AtomicBool>,
            progress: tauri::ipc::Channel<TaskProgress>,
        ) -> Result<Self::Output, CommandError>;
    }
    ```
* **NFR-SCA-02 (Cấu trúc Workspace Cargo Phân tách Rõ ràng):**
  * Dự án được tổ chức theo mô hình **Cargo Workspace** kết hợp crate dùng chung (`shared-types`) để đồng bộ kiểu dữ liệu 100% giữa Frontend WASM và Backend Native:
    * `office-utility-vn-shared` (`shared/`): Chứa toàn bộ structs dùng chung (`WorkflowDag`, `TaskProgress`, `CommandError`, `ToolPreset`) không phụ thuộc vào OS hay DOM.
    * `office-utility-vn-ui` (Thư mục gốc): Quản lý toàn bộ giao diện Leptos 0.8, Tailwind CSS, WebAssembly CSR.
    * `office_utility_vn_lib` (`src-tauri/`): Quản lý toàn bộ nhân xử lý native, Tauri commands, SQLite WAL và tương tác OS.
* **NFR-SCA-03 (Chi phí Hạ tầng Máy chủ Bằng 0 - Zero Server Cost):**
  * Do ứng dụng chạy 100% trên thiết bị người dùng, chi phí vận hành máy chủ xử lý hàng tháng là **0 VNĐ**.

---

## 3.5. Yêu cầu về Tính Tương thích & Trải nghiệm Desktop Nâng cao (Compatibility & Desktop UX)

* **NFR-COM-01 (Tương thích Đa nền tảng Hệ điều hành):**
  * **Windows:** Tương thích Windows 10 (1809 trở lên) và Windows 11 (`x86_64`).
  * **macOS:** Tương thích macOS 12 (Monterey) trở lên; hỗ trợ Universal Binary (chạy gốc mượt mà trên cả Apple Silicon `aarch64` M1/M2/M3/M4 và chip Intel `x86_64`).
  * **Linux:** Tương thích các bản phân phối phổ biến (Ubuntu 22.04+, Debian 12+, Fedora, Arch) có thư viện `webkit2gtk-4.1`.
* **NFR-COM-02 (An toàn Bộ gõ Tiếng Việt - IME Safe):**
  * Tương thích hoàn hảo với các bộ gõ tiếng Việt phổ biến (**UniKey, EVKey, OpenKey, bộ gõ mặc định macOS/ibus-bamboo**) trong tất cả các ô nhập liệu của Leptos.
  * Sử dụng cờ trạng thái `is_composing` lắng nghe sự kiện `compositionstart` và `compositionend` để trì hoãn việc kích hoạt bộ lọc tìm kiếm (Command Palette) cho đến khi ký tự dấu tiếng Việt được ghép hoàn chỉnh, tuyệt đối không bị nuốt chữ hoặc mất dấu khi gõ nhanh.
* **NFR-COM-03 (Chặn Phím tắt Mặc định của WebView - Native Shortcut Interception):**
  * Giao diện Leptos chủ động chặn (`event.prevent_default()`) các phím tắt mặc định của trình duyệt WebView như `Ctrl + P` (mở hộp thoại in trang web), `Ctrl + W` (đóng tab WebView), `Ctrl + F`, `F5`, `Ctrl + R` để gán trọn vẹn cho các hành động bản địa của ứng dụng Desktop.