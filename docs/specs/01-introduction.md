# 1. Giới thiệu Dự án (Introduction)

---

## 1.1. Bối cảnh và Động lực

### 1.1.1. Bối cảnh thực tế

Trong môi trường làm việc văn phòng hiện đại, nhân sự thuộc mọi bộ phận—từ hành chính, nhân sự, kế toán đến truyền thông và kỹ thuật—đều phải xử lý liên tục các tác vụ chuyển đổi định dạng và tiền xử lý dữ liệu vi mô (micro-tasks): ghép và nén tài liệu PDF, đổi định dạng ảnh (HEIC, WebP sang JPG/PNG), bóc tách văn bản từ ảnh scan (OCR), trích xuất dữ liệu bảng tính Excel/CSV, chuyển mã phông chữ tiếng Việt (TCVN3, VNI sang Unicode), tạo mã thanh toán VietQR hay tính toán lương Gross/Net và thuế TNCN.

Mặc dù thị trường hiện nay có nhiều giải pháp trực tuyến (như iLovePDF, Smallpdf, TinyWow, Convertio) hoặc các phần mềm desktop đóng gói cồng kềnh, việc sử dụng các công cụ này đang bộc lộ nhiều rủi ro và bất cập nghiêm trọng:

* **Mối đe dọa an toàn thông tin và quyền riêng tư (Privacy & Data Leakage Risks):** Hầu hết các dịch vụ trực tuyến đều yêu cầu tải toàn bộ tệp tin lên máy chủ đám mây trung gian. Đối với các tài liệu chứa dữ liệu nhạy cảm của doanh nghiệp (bảng lương, báo cáo tài chính, hợp đồng kinh tế, bản scan CCCD), việc gửi file lên các máy chủ không rõ nguồn gốc vi phạm trực tiếp các tiêu chuẩn tuân thủ bảo mật nội bộ và quy định pháp lý về bảo vệ dữ liệu cá nhân (Nghị định 13/2023/NĐ-CP).
* **Sự phân mảnh công cụ và gián đoạn công việc (Tool Fragmentation & Context Switching):** Người dùng phải ghi nhớ và chuyển đổi qua lại giữa hàng chục trang web khác nhau. Mỗi công cụ sở hữu một giao diện, chính sách sử dụng, quảng cáo hiển thị phiền toái hoặc áp đặt giới hạn lượt dùng khắt khe (chỉ cho phép 1–2 tác vụ mỗi giờ trước khi bắt trả phí).
* **Hạn chế của phần mềm Desktop truyền thống (Electron Bloatware):** Các phần mềm đa năng xây dựng trên nền tảng Chromium/Electron thường ngốn hàng trăm megabyte đến hàng gigabyte bộ nhớ RAM, kích thước bộ cài đặt vượt quá 150–200 MB và thời gian khởi động chậm chạp, gây suy giảm hiệu năng trên các máy tính văn phòng cấu hình phổ thông.
* **Phụ thuộc vào kết nối Internet:** Các công cụ nền web hoàn toàn bị tê liệt khi người dùng làm việc trong môi trường ngoại tuyến (offline), mạng nội bộ cô lập (air-gapped network) hoặc khi đường truyền internet gặp sự cố.

### 1.1.2. Động lực phát triển

Dự án được khởi xướng nhằm xây dựng **Office Utility VN** — một **Ứng dụng Desktop Tiện ích Văn phòng Toàn diện Thuần Rust (All-in-One Pure Rust Desktop Utility App)** hoạt động như một "dao đa năng kỹ thuật số" (Digital Swiss Army Knife) dành riêng cho người dùng máy tính tại Việt Nam. Dự án được thúc đẩy bởi 5 động lực kỹ thuật và sản phẩm cốt lõi:

1. **Hiện thực hóa mô hình bảo mật tuyệt đối "100% On-Device & Zero-Upload":** Toàn bộ quá trình đọc, phân tích, xử lý và xuất tệp tin diễn ra hoàn toàn cục bộ trên phần cứng máy tính người dùng (sử dụng Rust Native Engine). Dữ liệu nhạy cảm không bao giờ rời khỏi thiết bị, hoạt động hoàn hảo ở chế độ Offline và xóa bỏ 100% nguy cơ rò rỉ dữ liệu.
2. **Hiệu năng Cực hạn & Kiến trúc Mô-đun Mở rộng (Ultra-Light Core & On-Demand Signed Packs):** Bằng việc kết hợp giữa **Tauri v2** và giao diện **Leptos 0.8** (biên dịch sang WebAssembly CSR), ứng dụng chỉ tiêu thụ dưới 80 MB RAM khi chạy nền (nhẹ hơn Electron gấp 5–10 lần), thời gian khởi động lạnh dưới 450 ms (macOS/Linux) và dưới 750 ms (Windows). Bộ cài đặt cốt lõi được giữ siêu nhỏ gọn ($< 15\text{ MB}$), trong khi các động cơ nặng (như PDFium Render hoặc mô hình AI OCR chuyên sâu) được thiết kế theo cơ chế tận dụng API bản địa của hệ điều hành hoặc tải gói mở rộng ngoại tuyến có chữ ký số (`On-Demand Signed Add-ons`).
3. **Tận dụng Tối đa Năng lực Tính toán Đa lõi (Multi-threaded Native Rust Engine):** Sử dụng các thư viện Rust tối ưu cao (`rayon`, `tokio`, `image`, `lopdf`, `calamine`) để thực thi song song các tác vụ nặng (nén hàng loạt hàng trăm ảnh, ghép tài liệu PDF lớn) trên toàn bộ số nhân CPU thực mà không làm đơ hay gián đoạn luồng giao diện (duy trì ổn định 60 FPS).
4. **Tích hợp Sâu vào Hệ điều hành & Trải nghiệm "Keyboard-Centric":** Tận dụng tối đa các đặc quyền của ứng dụng desktop: Khay hệ thống (System Tray), Phím tắt toàn cục OS (`Ctrl + Shift + K`), Bộ nhận diện nội dung thông minh từ bộ nhớ tạm (`Ctrl + V`), Kéo-thả trực tiếp từ File Explorer / Finder, và Chế độ cửa sổ nổi ghim trên cùng (Always-on-Top / Mini PiP) giúp rút ngắn thời gian thao tác xuống dưới vài giây.
5. **Động cơ Quy tắc Pháp lý Việt Nam Linh hoạt (Hot-Reloadable Legal & Administrative Rulesets):** Các tham số nghiệp vụ thường xuyên thay đổi theo quy định Nhà nước (biểu thuế TNCN, mức trần BHXH, cấu trúc XML hóa đơn điện tử theo Thông tư 78 / Nghị định 123 / Nghị định 70/2025/NĐ-CP, và danh mục địa giới hành chính trước/sau sáp nhập) được tách thành bộ quy tắc dữ liệu độc lập trong SQLite/JSON, cho phép người dùng tùy chỉnh hoặc cập nhật tức thì mà không cần chờ biên dịch lại ứng dụng.

---

## 1.2. Mục tiêu Dự án

### 1.2.1. Mục tiêu tổng quát

Xây dựng một ứng dụng desktop mã nguồn mở / freeware tập trung, siêu nhẹ, tốc độ cao, an toàn tuyệt đối và tối ưu trải nghiệm cho người dùng văn phòng Việt Nam; hỗ trợ giải quyết toàn diện **55+ công cụ chuyên sâu** (mở rộng lên 60+ micro-tools) xử lý tài liệu, hình ảnh, bảng tính, chuyển đổi định dạng và nghiệp vụ văn phòng hàng ngày mà không phát sinh bất kỳ chi phí máy chủ nào.

### 1.2.2. Mục tiêu cụ thể

* **Về tính năng & nghiệp vụ:**
  * Cung cấp hệ sinh thái **55+ công cụ cốt lõi** phân chia trên 6 phân hệ: *Xử lý Tài liệu & PDF (19 công cụ)*, *Xử lý Hình ảnh & Media (9 công cụ)*, *Làm sạch & Chuyển đổi Dữ liệu (13 công cụ)*, *Tiện ích Văn phòng Bản địa hóa Việt Nam (9 công cụ)*, *Chẩn đoán Phần cứng & Thiết bị (3 công cụ)* và *Trạm Chuyển đổi Vạn năng & Tiện ích Tiếp thị (5 công cụ)*.
  * Tích hợp **Trạm Chuyển đổi Vạn năng (Universal Converter Hub)** tự động phát hiện định dạng tệp đầu vào qua Magic Bytes và điều hướng đến công cụ xử lý tương ứng.
  * Phát triển tính năng **Workflow Pipeline** cục bộ: Cho phép người dùng ghép nối nhiều công cụ độc lập thành chuỗi tự động hóa liên tục (Macro DAG) có kiểm tra tương thích kiểu dữ liệu (MIME-Type Port Validation) chỉ với một thao tác kéo-thả.
  * Tối ưu hóa tương tác máy tính để bàn: Command Palette toàn cục (`Ctrl + Shift + K`), Tự động phân tích dữ liệu Clipboard (`Ctrl + V`), Kéo-thả tệp trực tiếp từ Desktop/Explorer, và Cửa sổ tiện ích thu nhỏ ghim trên màn hình.

* **Về kiến trúc & công nghệ (Thuần Rust & Tích hợp OS Native):**
  * **Desktop Shell & Native Core:** Xây dựng trên nền tảng **Tauri v2** (Rust 2024 edition) đảm nhận quản lý vòng đời ứng dụng, menu, khay hệ thống, phân quyền tệp tin và giao tiếp IPC siêu tốc.
  * **Giao diện Người dùng (UI):** Xây dựng bằng **Leptos 0.8** (Rust CSR WebAssembly), phản ứng trạng thái tức thì (fine-grained reactivity), không phụ thuộc vào Node.js runtime khi chạy ứng dụng.
  * **Động cơ Xử lý:** Thay thế hoàn toàn các dịch vụ đám mây bằng các thư viện Rust chuyên dụng kết hợp Hardware API của hệ điều hành:
    * PDF AST & Đồ họa: `lopdf`, `pdf-writer`, kết hợp `pdfium-render` (hỗ trợ nạp động cho các tác vụ kết xuất bitmap/visual diff).
    * Hình ảnh & Media: `image`, `imageproc`, `turbojpeg`, `ravif`, kết hợp bộ giải mã phần cứng OS Native (Windows WIC/Media Foundation, macOS ImageIO/AVFoundation, Linux `libheif-rs`) để xử lý mượt mà ảnh HEIC và trích xuất khung hình Video.
    * Bảng tính, Word & Dữ liệu: `calamine`, `rust_xlsxwriter`, `csv`, `serde_json`, `zip` + `quick-xml` (xử lý cấu trúc OpenXML `.docx`/`.xlsx` ở mức luồng XML).
    * Trí tuệ Nhân tạo & OCR Ngoại tuyến: Cơ chế **Dual-Engine OCR** — mặc định sử dụng API OCR bản địa của hệ điều hành (`Windows.Media.Ocr` trên Windows, `Vision Framework` trên macOS) với kích thước 0 MB, đồng thời hỗ trợ tải gói **On-Demand VietOCR ONNX Pack** (`tract` / `ort`) có xác thực chữ ký số cho nhu cầu nhận diện tài liệu scan phức tạp.
  * **Lưu trữ Cục bộ (Local Persistence):** Sử dụng cơ sở dữ liệu nhúng **SQLite 3** (`rusqlite`) ở chế độ WAL lưu trữ cấu hình, nhiều mẫu Preset cho từng công cụ, bảng quy tắc pháp lý Việt Nam, lịch sử tác vụ và mẫu workflow ngay trong thư mục `AppData`.

* **Về bảo mật & quyền riêng tư (Privacy-by-Design):**
  * **100% On-Device:** Không gửi dữ liệu tệp hay nội dung người dùng qua mạng Internet. Không yêu cầu đăng ký tài khoản đám mây.
  * **Zero-Trace Sandbox:** Mọi tệp tạm thời sinh ra trong quá trình xử lý được lưu trữ trong thư mục cache ứng dụng và tự động quét dọn khi đóng tiến trình.
  * **Memory Zeroing:** Tự động xóa sạch bộ nhớ RAM chứa dữ liệu nhạy cảm (như mật khẩu giải mã tệp) bằng thư viện `zeroize`.

---

## 1.3. Phạm vi Dự án & Lộ trình Phát hành (Project Scope & Roadmap)

### 1.3.1. Các hạng mục trong phạm vi (In-Scope)

| Nhóm phạm vi | Chi tiết hạng mục thực hiện |
| --- | --- |
| **Phân hệ chức năng** | • **Document Suite (19 công cụ):** Ghép/tách/nén PDF, xoay trang, mã hóa & mở khóa mật khẩu PDF, đóng dấu bản quyền Watermark, đánh số trang Bates, tẩy sạch metadata ẩn, làm phẳng PDF forms, cắt viền trắng auto-crop, dàn trang in sách booklet, so sánh trực quan visual diff, trộn hợp đồng Word Mailmerge chuẩn hóa XML runs, tạo mục lục bookmark TOC, ghép PDF thành ảnh dọc dài, chuyển PDF đen trắng tiết kiệm mực in, phục hồi PDF lỗi XREF, biên dịch Markdown sang PDF, trích xuất bảng biểu ra Excel, chuyển PDF sang Markdown, OCR tiếng Việt ngoại tuyến.<br>• **Media Studio (9 công cụ):** Nén & đổi định dạng ảnh hàng loạt song song (JPEG, PNG, WebP, AVIF, HEIC $\leftrightarrow$ JPG/PNG), cắt ảnh thẻ chuẩn 300 DPI ($3\times 4$, $4\times 6$), làm mờ thông tin nhạy cảm trên giấy tờ scan, kiểm tra và xóa sạch thẻ EXIF/GPS, đóng dấu bản quyền logo hàng loạt, làm đẹp ảnh chụp màn hình, phẳng hóa tài liệu chụp nghiêng, cắt khoảng lặng file ghi âm, trích xuất khung hình từ video ngắn sang ảnh GIF/WebP.<br>• **Data & Text Lab (13 công cụ):** Chuyển mã phông chữ tiếng Việt (TCVN3, VNI $\leftrightarrow$ Unicode), chuẩn hóa bảng tính Excel/CSV (sửa lỗi ngày tháng, tách họ và tên lót), so sánh đối soát văn bản và bảng tính (Diff Checker), chuyển đổi dữ liệu đa năng (Excel $\leftrightarrow$ JSON/SQL/Markdown), tách file Excel theo cột, gộp nhiều file Excel vào một workbook, nối bảng trực quan (Visual Joiner), gỡ khóa bảo vệ chỉnh sửa sheet Excel (`<sheetProtection>`), phép toán tập hợp danh sách, sửa lỗi font mojibake, ẩn danh hóa dữ liệu nhạy cảm PII, định dạng & linter SQL, mã hóa/giải mã Base64 file.<br>• **Vietnam Office Micro-Tools (9 công cụ):** Đọc số tiền thành chữ chuẩn kế toán (VND/USD), tính lương Gross $\leftrightarrow$ Net và thuế TNCN với bảng tham số pháp lý tùy chỉnh được, tạo mã VietQR chuyển khoản ngân hàng Napas247, đọc hóa đơn điện tử XML e-Invoice VN (TT78/NĐ123/NĐ70), kiểm tra tính hợp lệ số CCCD 12 số & Mã số thuế MST, chuẩn hóa danh mục địa giới hành chính Việt Nam (hỗ trợ cả mô hình 3 cấp cũ và 2 cấp mới sau sáp nhập), tính ngày làm việc trừ ngày nghỉ lễ theo Luật Lao động VN xuất file lịch ICS, bảng tính trả nợ vay ngân hàng, tra cứu mã định danh ngân hàng Bank BIN Mapper.<br>• **Universal Converter, Hardware & Utilities (8 công cụ):** Trạm chuyển đổi tệp vạn năng tự động nhận diện chữ ký Magic Bytes, kiểm tra bàn phím & chuột, kiểm tra màn hình/webcam/micro, xem thông tin phần cứng hệ thống, bộ sinh mã UTM campaign, tiện ích tạo font chữ mạng xã hội Unicode, quay số ngẫu nhiên minh bạch kèm mã băm SHA-256, tính điểm hòa vốn bán hàng TMĐT. |
| **Tính năng hệ thống Desktop** | • Khung ứng dụng Tauri v2 với thanh tiêu đề tùy biến (`decorations: false`) tối ưu hóa tài nguyên phần cứng.<br>• Command Palette toàn cục (`Ctrl/Cmd + Shift + K`) và Trình lắng nghe Clipboard (`Ctrl + V`).<br>• Tích hợp Khay hệ thống (System Tray Icon) hỗ trợ truy cập nhanh các micro-tool.<br>• Chế độ cửa sổ tiện ích thu nhỏ ghim trên cùng màn hình (Always-on-Top Floating Pin).<br>• Bộ chọn tệp OS nguyên bản (Native File Dialogs) và Trình kéo-thả tệp trực tiếp từ Desktop.<br>• Thông báo đẩy hệ thống (OS Native Notifications) khi hoàn tất xử lý tác vụ nền.<br>• Trình xây dựng chuỗi tự động hóa cục bộ (Local Workflow Pipeline DAG) có kiểm tra tương thích cổng MIME-type. |
| **Lưu trữ & Dữ liệu Cục bộ** | • Cơ sở dữ liệu nhúng **SQLite 3** (`rusqlite` ở chế độ WAL) lưu trữ cấu hình người dùng, danh sách Preset theo từng công cụ, bảng quy tắc pháp lý VN, mẫu workflow và lịch sử tác vụ cục bộ.<br>• Cơ chế quản lý gói mở rộng ngoại tuyến có chữ ký số (`On-Demand Signed Add-ons`) cho PDFium và mô hình AI OCR. |
| **Đóng gói & Phát hành** | • **Windows:** Bộ cài đặt `.exe` (NSIS Installer) và gói doanh nghiệp `.msi` (`x86_64`).<br>• **macOS:** Gói cài đặt `.dmg` và `.app` Universal Binary (hỗ trợ cả Apple Silicon `aarch64` và Intel `x86_64`).<br>• **Linux:** Định dạng đóng gói `.deb` và `.AppImage` (Ubuntu/Debian, Fedora, Arch).<br>• Tự động cập nhật ứng dụng và gói quy tắc bảo mật qua `tauri-plugin-updater` sử dụng chữ ký điện tử Minisign. |

### 1.3.2. Các hạng mục ngoài phạm vi (Out-of-Scope)

* **Không phát triển ứng dụng nền Web hoặc Mobile:** Dự án tập trung 100% vào trải nghiệm người dùng máy tính để bàn (Windows, macOS, Linux). Không triển khai phiên bản web lưu trữ trên máy chủ công cộng hay ứng dụng di động iOS/Android.
* **Không yêu cầu Máy chủ Trung tâm & Điện toán Đám mây:** Hệ thống không vận hành cụm máy chủ backend (Node.js, Python, BullMQ, Redis) và không lưu trữ dữ liệu của người dùng trên đám mây (S3/R2). Mọi chi phí hạ tầng máy chủ cho người dùng cuối là **0 đồng**.
* **Không thay thế bộ phần mềm văn phòng chuyên sâu:** Ứng dụng không hướng tới việc thay thế Microsoft Word, Excel hoặc các trình đọc/soạn thảo tài liệu toàn diện, mà đóng vai trò là trạm tiện ích xử lý nhanh trước và sau khi làm việc với tài liệu.
* **Không bẻ khóa mật khẩu mã hóa mật mã học (Brute-force Cracking):** Công cụ mở khóa PDF/Excel chỉ hỗ trợ gỡ mật khẩu khi người dùng nhập đúng mật khẩu mở file hoặc gỡ thẻ khóa bảo vệ trang tính XML (`<sheetProtection>`), tuyệt đối không cung cấp tính năng dò mật khẩu bất hợp pháp.

### 1.3.3. Lộ trình Triển khai Sản phẩm (Production Phasing Roadmap)

Để đảm bảo chất lượng kỹ thuật và độ ổn định ở cấp độ Production, dự án được phân chia thành 3 giai đoạn phát hành:

| Giai đoạn | Mục tiêu Trọng tâm | Danh mục Hạng mục & Công cụ Phát hành |
| --- | --- | --- |
| **Phase 1: Core Foundation & MVP**<br>*(Tháng 1 – Tháng 2)* | Hoàn thiện khung kiến trúc Tauri v2 + Leptos 0.8, SQLite WAL, Custom Titlebar, Command Palette (`Ctrl+K`), Clipboard Monitor (`Ctrl+V`), System Tray, Preset Manager và **20 công cụ Must-have** tần suất sử dụng cao nhất. | • **Document:** `FR-DOC-01` (Ghép/Tách PDF), `FR-DOC-02` (Bảo mật PDF), `FR-DOC-04` (Nén PDF), `FR-DOC-06` (Watermark PDF), `FR-DOC-08` (Tẩy Metadata).<br>• **Media:** `FR-MED-01` (Nén & Đổi định dạng ảnh hàng loạt), `FR-MED-04` (Kiểm tra/Xóa EXIF), `FR-MED-05` (Đóng dấu ảnh hàng loạt).<br>• **Data:** `FR-DAT-01` (Chuyển mã Font TCVN3/VNI), `FR-DAT-02` (Làm sạch Excel), `FR-DAT-04` (Chuyển đổi Excel $\leftrightarrow$ JSON/SQL/MD), `FR-DAT-08` (Gỡ khóa Sheet Excel), `FR-DAT-13` (Base64 Codec).<br>• **VN Office:** `FR-VNO-01` (Đọc số tiền), `FR-VNO-02` (Tính lương Gross/Net), `FR-VNO-03` (Tạo VietQR Napas247), `FR-VNO-04` (Đọc hóa đơn XML), `FR-VNO-05` (Kiểm tra CCCD/MST), `FR-VNO-07` (Tính ngày làm việc VN), `FR-VNO-09` (Tra cứu Bank BIN). |
| **Phase 2: Advanced Suite & Workflow DAG**<br>*(Tháng 3 – Tháng 4)* | Ra mắt **Workflow Pipeline Builder (Macro DAG)** có kiểm tra kiểu MIME, **Universal Converter Hub**, tích hợp gói `pdfium-render` và mở rộng **22 công cụ** xử lý nâng cao. | • **Core Hub & DAG:** `FR-HUB-01` (Trạm chuyển đổi vạn năng), Trình thiết kế Workflow DAG, Chế độ cửa sổ nổi Always-on-Top Pin.<br>• **Document:** `FR-DOC-03`, `FR-DOC-07`, `FR-DOC-09`, `FR-DOC-10`, `FR-DOC-11`, `FR-DOC-12`, `FR-DOC-13` (Mailmerge XML normalization), `FR-DOC-14`, `FR-DOC-15`, `FR-DOC-16`, `FR-DOC-17`, `FR-DOC-18`, `FR-DOC-19`.<br>• **Data:** `FR-DAT-03` (Diff Checker), `FR-DAT-05` (Tách Excel), `FR-DAT-06` (Gộp Excel), `FR-DAT-07` (Visual Joiner), `FR-DAT-09`, `FR-DAT-10`, `FR-DAT-11`, `FR-DAT-12`.<br>• **VN Office:** `FR-VNO-06` (Chuẩn hóa địa giới hành chính 2 cấp/3 cấp), `FR-VNO-08` (Lịch trả nợ vay). |
| **Phase 3: Offline AI, Media Pro & Diagnostics**<br>*(Tháng 5 – Tháng 6)* | Tích hợp **Dual-Engine Offline OCR**, nhóm công cụ xử lý hình ảnh/âm thanh/video nâng cao, chẩn đoán phần cứng và tiện ích TMĐT (**13 công cụ còn lại**). | • **AI & Document:** `FR-DOC-05` (OCR Tiếng Việt ngoại tuyến qua OS Native API + On-Demand VietOCR ONNX Pack).<br>• **Media Pro:** `FR-MED-02` (Ảnh thẻ 300 DPI), `FR-MED-03` (Che mờ CCCD), `FR-MED-06` (Beautify Screenshot), `FR-MED-07` (Phẳng hóa tài liệu nghiêng), `FR-MED-08` (Cắt khoảng lặng âm thanh), `FR-MED-09` (Video sang GIF/WebP).<br>• **Hardware & Marketing:** `FR-SYS-01`, `FR-SYS-02`, `FR-SYS-03`, `FR-UTL-01`, `FR-UTL-02`, `FR-UTL-03`, `FR-UTL-04`. |