# Kế hoạch Chuyển đổi Toàn bộ Specs sang Kiến trúc Desktop Thuần Rust (Tauri v2 + Leptos)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (hoặc superpowers:subagent-driven-development) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Chuyển đổi và chuẩn hóa toàn bộ 7 tài liệu đặc tả kỹ thuật trong `docs/specs/` từ mô hình kiến trúc Web/Cloud đa máy chủ (NestJS, PostgreSQL, Redis BullMQ, S3/R2, Python Docling sidecar) sang kiến trúc **ứng dụng Desktop thuần Rust** 100% On-Device, Offline-First dựa trên nền tảng **Tauri v2 + Leptos 0.8 CSR (WebAssembly) + Tokio/Rayon**.

**Architecture:** 
- **Desktop Shell & Native Core:** Tauri v2 (Rust 2024 edition) quản lý vòng đời cửa sổ, System Tray, Global Shortcuts, OS Notifications, Native Dialogs và phân phối tác vụ song song qua `rayon` / `tokio`.
- **Frontend UI:** Leptos 0.8 (Rust CSR compiled to WASM qua Trunk), giao diện phản ứng nhanh (fine-grained reactivity), Tailwind CSS, giao tiếp với Core qua Tauri IPC (`invoke`, `Channel`, `Event`).
- **Xử lý Tác vụ Chuyên sâu (100% Rust Crate):** Thay thế toàn bộ cụm worker đám mây bằng các thư viện Rust thuần tối ưu cao (`lopdf`, `pdf-writer`, `image`, `imageproc`, `calamine`, `rust_xlsxwriter`, `csv`, `ort` nhúng ONNX runtime offline cho OCR/bóc tách dữ liệu).
- **Lưu trữ Cục bộ (Local Persistence):** Thay thế PostgreSQL/Redis bằng **SQLite** nhúng (`rusqlite` / `sqlx-sqlite`) và thư mục AppData cục bộ của hệ điều hành.
- **Đóng gói & CI/CD:** GitHub Actions release matrix biên dịch tự động ra `.exe`, `.msi` (Windows), `.dmg`, `.app` (macOS Universal Binary), `.deb`, `AppImage` (Linux) kết hợp cơ chế tự cập nhật `tauri-plugin-updater`.

**Tech Stack:**
- Rust 2024, Tauri v2, Leptos 0.8 (CSR WASM), Trunk
- Rayon (data parallelism), Tokio (async I/O)
- SQLite (local embedded DB), Serde / Bincode
- Pure Rust processing crates: `lopdf`, `image`, `calamine`, `rust_xlsxwriter`, `ort` (ONNX Runtime)
- Testing & CI/CD: `cargo test`, `wasm-bindgen-test`, `tauri::test`, GitHub Actions matrix

**Spec Reference:** Các tài liệu hiện tại tại [docs/specs](file:///home/quan/workspace/tauri-v2/office-utility-vn/docs/specs).

---

## Global Constraints

- **Ngôn ngữ duy nhất:** 100% Rust cho cả Backend (Tauri Core) và Frontend (Leptos WASM CSR). Không sử dụng Node.js, Python, hay Go server trong kiến trúc vận hành.
- **Bảo mật & Quyền riêng tư:** 100% On-Device, Zero-Upload. Mọi dữ liệu tài liệu, hình ảnh, văn bản chỉ được xử lý trong RAM/Disk cục bộ của máy tính người dùng. Không phát sinh kết nối mạng ngoại trừ khi kiểm tra bản cập nhật mới (Tauri Updater).
- **Hiệu năng & Tài nguyên:** Tận dụng triệt để đa nhân CPU qua `rayon`, không gây nghẽn UI Main Thread; dung lượng bộ nhớ RAM lúc nghỉ (idle) dưới 80MB; khởi động lạnh (cold start) dưới 500ms.
- **Tương thích Đa nền tảng:** Windows 10/11 (x86_64), macOS 12+ (Universal Binary: Apple Silicon & Intel), Linux (WebKitGTK 4.1).

---

## Review Focus

1. **Loại bỏ triệt để dấu vết Web/Server:** Đảm bảo không còn bất kỳ thuật ngữ hay cấu trúc nào liên quan đến NestJS, BullMQ, Redis, PostgreSQL, Fastify, Docker Compose, S3/R2 Presigned URLs, SSE, WebSocket signaling server trong toàn bộ 7 file specs.
2. **Tính khả thi của Crate Thuần Rust:** Các công cụ nặng (PDF merge/split, OCR tiếng Việt, xử lý Excel/Word, nén ảnh) phải chỉ định rõ crate Rust cụ thể thay thế cho các công cụ command-line/python trước đây.
3. **Mô hình Giao tiếp IPC Chính xác:** Thay thế toàn bộ REST API endpoints bằng Tauri Commands (`#[tauri::command]`), DTO Rust `struct`, và luồng báo tiến độ thời gian thực bằng `tauri::ipc::Channel` hoặc Tauri Event bus.
4. **Cơ chế Lưu trữ & Quản lý Dữ liệu:** Thay thế bảng PostgreSQL sang SQLite schema cục bộ, lưu trữ cấu hình trong `app_data_dir()`, quản lý tệp tạm trong `app_cache_dir()`.
5. **Chiến lược Kiểm thử & CI/CD Phù hợp:** Đổi từ Vitest/Jest/k6 sang `cargo test`, `wasm-bindgen-test`, `criterion` benchmark, và GitHub Actions release matrix.

---

## Danh sách Task Thực thi

### Task 1: Tinh chỉnh Tài liệu 01 - Giới thiệu Dự án (`01-introduction.md`)
**Files:**
- Modify: `docs/specs/01-introduction.md`

**Nội dung chuyển đổi:**
- Đổi định vị sản phẩm: Từ "Nền tảng Web Tiện ích Văn phòng" sang "Ứng dụng Desktop Đa nền tảng Thuần Rust (Tauri v2 + Leptos WASM)".
- Cập nhật 4 động lực cốt lõi:
  1. *Native On-Device & Zero-Upload:* Xử lý 100% trên phần cứng máy khách bằng Rust Native, loại bỏ hoàn toàn rủi ro rò rỉ dữ liệu lên máy chủ đám mây.
  2. *Tích hợp Hệ điều hành Sâu (OS-Integrated & Keyboard-Centric):* Tận dụng System Tray, phím tắt toàn cục OS, tích hợp Clipboard hệ thống, Drag & Drop trực tiếp từ File Explorer / Finder.
  3. *Hiệu năng Cực hạn & Siêu nhẹ (Zero Server Cost, Low Footprint):* Bộ nhớ RAM tiêu thụ ít hơn 80MB (nhẹ hơn Electron nhiều lần), khởi động dưới 500ms, không tốn bất kỳ chi phí duy trì máy chủ backend nào.
  4. *Bảo mật & Quyền riêng tư Tuyệt đối:* Không lưu vết, tuân thủ Nghị định 13/2023/NĐ-CP do dữ liệu không bao giờ rời khỏi máy.
- Điều chỉnh In-Scope / Out-of-Scope: Bỏ hạ tầng máy chủ đám mây, PostgreSQL, Redis, BullMQ; thêm đóng gói native installer (.exe, .dmg, .deb), auto-updater, offline AI OCR nhúng cục bộ.

- [x] **Step 1:** Soạn thảo lại toàn bộ nội dung `01-introduction.md` chuẩn hóa theo kiến trúc Desktop thuần Rust.
- [x] **Step 2:** Kiểm tra tính nhất quán thuật ngữ (Tauri v2, Leptos, Rust 2024, Native, On-Device).

---

### Task 2: Tinh chỉnh Tài liệu 02 - Đặc tả Yêu cầu Hệ thống (`02-srs.md`)
**Files:**
- Modify: `docs/specs/02-srs.md`

**Nội dung chuyển đổi:**
- Tái cấu trúc Tác nhân (Actors):
  - Người dùng Desktop cục bộ (Local Desktop User - không còn phân chia Guest vs Authenticated theo tài khoản cloud, tất cả tính năng đều khả dụng không giới hạn).
  - Hệ điều hành & Tập tin Cục bộ (Local OS & Native Filesystem).
  - Luồng Xử lý Nền (Native Worker Threadpool - Rayon & Tokio Background Tasks).
- Cập nhật Use Case Diagram & Mô tả Chức năng:
  - Bỏ các Use Case: Đăng nhập OAuth2, Cấp phát Presigned URL, Điều phối hàng đợi máy chủ BullMQ, Đẩy tệp P2P qua máy chủ trung gian.
  - Thay thế bằng: Quản lý cửa sổ (Pin-on-top PiP, System Tray), Giám sát Clipboard OS (`Ctrl + V`), Kéo-thả tệp trực tiếp từ Desktop/Explorer, Lưu trữ Workflow cục bộ (Local DAG Pipeline), Xuất tệp trực tiếp ra thư mục đích trên máy.
- Điều chỉnh ma trận 60+ công cụ: Mô tả rõ cách từng công cụ chạy trực tiếp bằng Crate Rust cục bộ.

- [x] **Step 1:** Soạn thảo lại `02-srs.md` với sơ đồ Mermaid use case desktop mới và chi tiết tính năng bản địa.
- [x] **Step 2:** Kiểm tra ma trận chức năng, đảm bảo mọi công cụ đều được định nghĩa thực thi cục bộ.

---

### Task 3: Tinh chỉnh Tài liệu 03 - Đặc tả Yêu cầu Phi chức năng (`03-nfr.md`)
**Files:**
- Modify: `docs/specs/03-nfr.md`

**Nội dung chuyển đổi:**
- Chuyển đổi SLA Hiệu năng:
  - Bỏ Web Vitals (LCP, INP) và API latency Fastify.
  - Thay bằng: Thời gian khởi động ứng dụng (Cold start < 500ms, Warm start < 150ms), Dung lượng nhị phân (.exe < 15MB, .dmg < 25MB), Tiêu thụ RAM (Lúc nghỉ < 80MB, lúc xử lý đỉnh < 300MB đối với tác vụ thông thường).
  - Tốc độ xử lý đa luồng CPU qua Rayon: Tận dụng 100% số luồng vật lý máy tính khi xử lý lô lớn (batch) mà không làm lag Main UI Thread (luôn giữ 60 FPS).
- An toàn & Bảo mật Desktop:
  - Cơ chế cô lập thư mục tạm trong `app_cache_dir()` và tự động dọn dẹp khi hoàn tất hoặc khi thoát ứng dụng.
  - Xóa sạch dữ liệu nhạy cảm trong bộ nhớ (`zeroize` memory scrubbing) sau khi hoàn tất mã hóa / giải mã.
  - Sandbox quyền truy cập tệp qua Tauri v2 Scopes (chỉ đọc/ghi các thư mục mà người dùng chỉ định qua dialog hoặc kéo thả).

- [x] **Step 1:** Cập nhật `03-nfr.md` với các tiêu chuẩn NFR đo lường trên Desktop.
- [x] **Step 2:** Rà soát các chỉ số SLA phần cứng và kiểm tra an ninh bộ nhớ.

---

### Task 4: Tinh chỉnh Tài liệu 04 - Thiết kế Dữ liệu & Đặc tả IPC Commands (`04-database-api.md`)
**Files:**
- Modify: `docs/specs/04-database-api.md`

**Nội dung chuyển đổi:**
- Tái thiết kế Cơ sở Dữ liệu Cục bộ (Embedded SQLite):
  - Bỏ PostgreSQL 18, JSONB GIN Index, TypeORM, Redis 8.
  - Thay bằng **SQLite 3** nhúng (`rusqlite` hoặc `sqlx` sqlite mode) lưu tại `{app_data_dir}/office_utility.db`.
  - Thiết kế ERD mới cho SQLite:
    - `tool_presets` (Cấu hình công cụ tùy biến của người dùng).
    - `workflows` (Đồ thị chuỗi tác vụ tự động hóa DAG lưu dưới dạng JSON text).
    - `task_history` (Lịch sử tác vụ thực thi cục bộ, đường dẫn tệp vào/ra, trạng thái, thời gian thực thi).
    - `user_preferences` (Cài đặt giao diện, phím tắt, đường dẫn lưu mặc định, ngôn ngữ).
- Thay thế toàn bộ REST API bằng **Tauri IPC Command Specification**:
  - `#[tauri::command] async fn dispatch_task(...)`
  - `#[tauri::command] async fn cancel_task(...)`
  - `#[tauri::command] async fn inspect_file_metadata(...)`
  - `#[tauri::command] async fn get_system_hardware_info(...)`
  - `#[tauri::command] async fn save_workflow(...)`
  - Khai báo chi tiết các DTO Rust (Request/Response structs với `serde::Serialize`, `serde::Deserialize`).
  - Giao chế độ streaming tiến độ: Dùng `tauri::ipc::Channel<TaskProgress>` truyền trực tiếp từ Rust Native Thread về Leptos WASM UI.

- [x] **Step 1:** Soạn thảo lại `04-database-api.md` với lược đồ SQLite và chi tiết các Tauri Commands & IPC Data Types.
- [x] **Step 2:** Rà soát tính chặt chẽ của các Rust Structs và kiểu dữ liệu.

---

### Task 5: Tinh chỉnh Tài liệu 05 - Thiết kế Giao diện & Trải nghiệm Người dùng (`05-ui-ux.md`)
**Files:**
- Modify: `docs/specs/05-ui-ux.md`

**Nội dung chuyển đổi:**
- Công nghệ Giao diện: Leptos 0.8 reactive components (Signals, Memos, Actions) kết hợp Tailwind CSS.
- Tinh chỉnh Wireframe & Trải nghiệm Desktop:
  - Khung cửa sổ Native: Custom title bar tích hợp điều khiển đóng/thu nhỏ/ghim (Always on Top cho các tiện ích nhỏ như VietQR, Đọc số thành chữ, Chuyển mã font).
  - Tích hợp System Tray: Biểu tượng khay hệ thống, menu chuột phải (Mở nhanh, Dán từ clipboard, Thoát).
  - Command Palette toàn cục (`Ctrl/Cmd + K` hoặc phím tắt toàn hệ thống OS `Ctrl + Shift + O`).
  - Lắng nghe Clipboard tự động: Phát hiện ảnh chụp màn hình, danh sách dữ liệu bảng tính để hiển thị thanh gợi ý công cụ tức thì.
  - Bộ chọn tệp OS (Native File Dialog): Hỗ trợ chọn tệp đơn, chọn nhiều tệp và chọn thư mục xuất kết quả.

- [x] **Step 1:** Cập nhật `05-ui-ux.md` với các quy chuẩn thiết kế giao diện Desktop Leptos + Tailwind.
- [x] **Step 2:** Kiểm tra tính khả thi của các tương tác Desktop (Tray, Custom Titlebar, Native Drag & Drop).

---

### Task 6: Tinh chỉnh Tài liệu 06 - Chiến lược Kiểm thử & Đảm bảo Chất lượng (`06-testing-qa.md`)
**Files:**
- Modify: `docs/specs/06-testing-qa.md`

**Nội dung chuyển đổi:**
- Tái cấu trúc Kim tự tháp Kiểm thử chuẩn Rust:
  - **Tầng 1: Unit Testing (`cargo test`):** Kiểm thử các module Rust lõi xử lý nhị phân (thuật toán đọc số tiền VN, tính thuế TNCN, tạo chuỗi VietQR Napas, parser TCVN3/VNI, logic gộp/tách PDF với `lopdf`, mã hóa AES cục bộ).
  - **Tầng 2: WASM & Leptos Component Testing (`wasm-bindgen-test`):** Kiểm thử các component giao diện Leptos, phản ứng Signals, tính toán hiển thị.
  - **Tầng 3: Tauri IPC Integration Testing (`tauri::test`):** Kiểm thử các lệnh `#[tauri::command]`, state quản lý cơ sở dữ liệu SQLite, kênh stream tiến độ `Channel`.
  - **Tầng 4: Desktop E2E Testing (`tauri-driver` / WebDriver):** Kiểm thử tự động trên giao diện desktop thực tế (WebView2 trên Windows, WebKit trên macOS/Linux).
  - **Tầng 5: Benchmark & Stress Testing (`criterion`):** Đo đạc thời gian xử lý và mức sử dụng bộ nhớ của các thuật toán xử lý tệp nặng khi chạy song song qua Rayon.
- Loại bỏ toàn bộ k6 stress test hàng đợi BullMQ, Docker testcontainers, Playwright web browser test.

- [x] **Step 1:** Soạn thảo lại `06-testing-qa.md` theo phương pháp QA và công cụ kiểm thử Rust chuyên biệt.
- [x] **Step 2:** Kiểm tra ma trận test cases và lệnh chạy kiểm thử `cargo test`.

---

### Task 7: Tinh chỉnh Tài liệu 07 - Đóng gói, CI/CD & Cập nhật Ứng dụng (`07-deployment-devops.md`)
**Files:**
- Modify: `docs/specs/07-deployment-devops.md`

**Nội dung chuyển đổi:**
- Thay thế hoàn toàn Docker Compose, VPS Cluster, Reverse Proxy, Nginx, Cloudflare CDN/Pages.
- Cấu trúc Triển khai Mới:
  - **Kiến trúc Biên dịch & Đóng gói:** Tauri CLI (`cargo tauri build`) kết hợp Trunk đóng gói bundle WebAssembly CSR.
  - **Định dạng Gói Cài đặt:**
    - Windows: `.exe` (NSIS Installer) và `.msi` (WiX Toolset).
    - macOS: `.dmg` và `.app` (Universal Binary hỗ trợ đồng thời Apple Silicon `aarch64` và Intel `x86_64`).
    - Linux: `.deb` và `.AppImage`.
  - **Quy trình CI/CD Ma trận GitHub Actions:**
    - Chi tiết workflow `.github/workflows/release.yml` đa nền tảng (`windows-latest`, `macos-latest`, `ubuntu-latest`).
    - Tự động cache cargo dependencies, cài đặt công cụ cần thiết, build và đính kèm artifacts vào GitHub Release.
  - **Cơ chế Tự Động Cập Nhật (Auto-Updater):**
    - Sử dụng `tauri-plugin-updater` chính thức của Tauri v2.
    - Ký mã bảo mật bằng cặp khóa mật mã Minisign / Ed25519.
    - Điểm cuối kiểm tra phiên bản: GitHub Releases endpoint hoặc tĩnh trên kho lưu trữ.
  - **Khai báo Quyền hạn & Bảo mật (Tauri Permissions & Capabilities):**
    - Thiết lập file `src-tauri/capabilities/default.json` theo nguyên tắc tối thiểu hóa đặc quyền (Least Privilege).

- [x] **Step 1:** Soạn thảo lại `07-deployment-devops.md` với đầy đủ kiến trúc đóng gói đa nền tảng, GitHub Actions và auto-updater.
- [x] **Step 2:** Kiểm tra cấu hình và hướng dẫn đóng gói cho Windows, macOS, Linux.

---

### Task 8: Tự đánh giá và Rà soát Đồng bộ Tổng thể (Self-Review & Verification)
**Files:**
- Review: Toàn bộ 7 file trong `docs/specs/`
- Review: `README.md` (nếu cần bổ sung tham chiếu)

- [x] **Step 1:** Rà soát chéo giữa 7 tài liệu để đảm bảo tính nhất quán 100% về thuật ngữ, kiểu dữ liệu, các crate Rust được sử dụng và cấu trúc hệ thống.
- [x] **Step 2:** Đảm bảo không còn bất kỳ liên kết hỏng hay tham chiếu lỗi thời đến kiến trúc Web/Cloud cũ.
- [x] **Step 3:** Báo cáo tổng kết hoàn tất công việc cho người dùng.
