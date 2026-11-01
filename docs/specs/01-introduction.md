# 1. Giới thiệu Dự án (Introduction)

---

## 1.1. Bối cảnh và Động lực

### 1.1.1. Bối cảnh thực tế

Trong môi trường văn phòng hiện đại, nhân sự thuộc mọi bộ phận—từ hành chính, kế toán, nhân sự đến truyền thông và kỹ thuật—đều phải xử lý liên tục các tác vụ chuyển đổi định dạng và tiền xử lý dữ liệu vi mô (micro-tasks): ghép và nén tài liệu PDF, đổi định dạng ảnh (HEIC, WebP sang JPG), bóc tách văn bản từ ảnh scan (OCR), chuyển đổi PDF sang Word/Markdown giữ nguyên cấu trúc bảng biểu, chuẩn hóa bảng tính Excel/CSV, chuyển mã phông chữ tiếng Việt, tạo mã thanh toán QR hay chia sẻ thông tin mật nội bộ.

Mặc dù thị trường hiện nay có nhiều giải pháp trực tuyến riêng lẻ (như iLovePDF, Smallpdf, TinyWow, Convertio), việc sử dụng các công cụ này trong công việc hàng ngày đang bộc lộ nhiều rào cản và rủi ro lớn:

* **Sự phân mảnh công cụ (Tool Fragmentation):** Người dùng phải ghi nhớ, quản lý và chuyển đổi qua lại giữa hàng chục trang web khác nhau cho các nhu cầu cơ bản. Mỗi nền tảng sở hữu một giao diện, luồng thao tác và chính sách sử dụng riêng, gây gián đoạn mạch làm việc (context switching) và làm giảm năng suất lao động.
* **Mối đe dọa an toàn thông tin và quyền riêng tư (Privacy & Data Leakage Risks):** Hầu hết các dịch vụ chuyển đổi hiện nay đều yêu cầu tải toàn bộ tệp tin lên máy chủ trung gian. Đối với các tài liệu chứa dữ liệu nhạy cảm của doanh nghiệp (bảng lương nhân sự, báo cáo tài chính, hợp đồng pháp lý, bản scan CCCD), việc gửi file lên các máy chủ không rõ nguồn gốc hoặc lưu trữ không kiểm soát vi phạm trực tiếp các tiêu chuẩn tuân thủ bảo mật nội bộ và các quy định bảo vệ dữ liệu cá nhân (như Nghị định 13/2023/NĐ-CP).
* **Trải nghiệm người dùng suy giảm (Poor User Experience):** Các nền tảng miễn phí có xu hướng lạm dụng quảng cáo hiển thị dạng pop-up, cài cắm các nút tải xuống giả mạo mã độc, áp đặt giới hạn lượt dùng khắt khe (chỉ cho phép 1–2 tác vụ mỗi giờ trước khi khóa tính năng) hoặc ép buộc người dùng đăng ký tài khoản phiền toái.
* **Lãng phí tài nguyên hạ tầng:** Phần lớn các thao tác đơn giản (như tách/ghép PDF, xoay ảnh, chuyển đổi dữ liệu bảng tính, xử lý chuỗi văn bản) hoàn toàn có thể thực thi trực tiếp trên năng lực tính toán của máy tính người dùng. Việc đẩy toàn bộ các tác vụ này về máy chủ backend gây lãng phí băng thông mạng, tốn kém chi phí máy chủ và gia tăng độ trễ mạng không cần thiết.

### 1.1.2. Động lực phát triển

Dự án được khởi xướng nhằm xây dựng một **Nền tảng Tiện ích Văn phòng Toàn diện (All-in-One Web Utility Platform - OfficeUtility.vn)** hoạt động như một "dao đa năng kỹ thuật số" (Digital Swiss Army Knife). Dự án được thúc đẩy bởi 4 động lực kỹ thuật và sản phẩm cốt lõi:

1. **Hiện thực hóa mô hình bảo mật "On-Device First" (Zero-Upload):** Tận dụng sức mạnh của các công nghệ web hiện đại như WebAssembly (WASM), Web Workers và WebGPU để xử lý tệp tin trực tiếp trên bộ nhớ RAM của trình duyệt. Dữ liệu nhạy cảm không bao giờ rời khỏi thiết bị của người dùng đối với các tác vụ cơ bản, xóa bỏ hoàn toàn nguy cơ rò rỉ thông tin.
2. **Tối ưu hóa quy trình làm việc với trải nghiệm "Clipboard-First" & "Keyboard-Centric":** Xóa bỏ các thao tác duyệt file thủ công rườm rà; thay vào đó, hệ thống chủ động nhận diện dữ liệu từ clipboard (`Ctrl + V`), cung cấp công cụ tìm kiếm nhanh (`Ctrl + K`), liên kết cấu hình nén chia sẻ nhanh (`Shareable Preset URLs`), và cơ chế Picture-in-Picture (PiP) để rút ngắn thời gian thao tác xuống dưới vài giây.
3. **Tiết kiệm chi phí vận hành thông qua kiến trúc Hybrid thông minh:** Phân tách rõ ràng giữa năng lực tính toán tại Client (xử lý miễn phí, 0 đồng chi phí server với hơn 70% công cụ) và cụm Worker bất đồng bộ tại Server (chỉ dành cho các tác vụ chuyển đổi nặng như LibreOffice headless, FFmpeg, Docling AI Document Extraction hay OCR tiếng Việt), tạo ra một hệ thống có tính bền vững cao về mặt hạ tầng và kinh tế.
4. **Bảo mật tuyệt đối với Zero-Knowledge End-to-End Encryption:** Cung cấp tính năng liên kết bí mật tự hủy một lần (One-Time Secret Link) với cơ chế mã hóa giải mã hoàn toàn tại Client (Web Crypto AES-GCM-256), máy chủ chỉ lưu ciphertext tạm thời trên Redis và tự hủy ngay sau khi đọc hoặc sau 30 phút.

---

## 1.2. Mục tiêu Dự án

### 1.2.1. Mục tiêu tổng quát

Xây dựng một ứng dụng web tiện ích văn phòng tập trung, hiệu năng cao, bảo mật và thân thiện với người dùng; hỗ trợ giải quyết toàn diện hơn **60 tác vụ** xử lý tài liệu, đa phương tiện, chuyển đổi định dạng và dữ liệu hàng ngày với tốc độ tối ưu và chi phí vận hành máy chủ thấp nhất.

### 1.2.2. Mục tiêu cụ thể

* **Về tính năng & nghiệp vụ:**
  * Cung cấp hệ sinh thái hơn **60 công cụ chuyên sâu** phân chia trên các phân hệ: *Xử lý Tài liệu & PDF*, *Xử lý Hình ảnh & Media*, *Làm sạch & Chuyển đổi Dữ liệu*, *Tiện ích Văn phòng Bản địa hóa Việt Nam*, *Chẩn đoán Phần cứng & Thiết bị*, *Trạm Chuyển đổi Vạn năng (Universal Converter Hub)* và *Chia sẻ Bí mật Tự hủy (Zero-Knowledge Secret Links)*.
  * Tích hợp **Trạm Chuyển đổi Vạn năng (Universal Document & Data Converter Hub)** cho phép chuyển đổi qua lại giữa hàng chục cặp định dạng (PDF, Word, Excel, Markdown, Pandoc, Text, Image) với bộ phát hiện tệp nén thông minh (ZIP sub-type sniffer) và tự động điều hướng sang công cụ chuyên sâu (Spoke Redirects).
  * Phát triển tính năng **Workflow Pipeline** cho phép người dùng ghép nối nhiều công cụ rời rạc thành một chuỗi tự động hóa liên tục (Macro DAG) chỉ với 1 lần tải tệp đầu vào, trực quan hóa trên giao diện kéo-thả `@dnd-kit`.
  * Tích hợp cơ chế tương tác nhanh: Omnibox Palette (`Ctrl + K`), Bộ nhận diện nội dung thông minh từ bộ nhớ tạm (`Ctrl + V`), Bộ định tuyến thả tệp ngữ cảnh (Smart Dropzone Router), Trình chia sẻ cấu hình nén qua URL (`?preset=...`), và Chế độ cửa sổ nổi Picture-in-Picture (PiP).

* **Về kiến trúc & hiệu năng hệ thống:**
  * Đạt tỷ lệ xử lý tại chỗ (Client-side WASM execution) từ **70% đến 80%** trên tổng số công cụ, đảm bảo không phát sinh lưu lượng mạng đối với các tác vụ nhẹ.
  * Tối ưu hóa thời gian phản hồi: Các tác vụ Client-side phản hồi trong vòng dưới 500ms; các tác vụ Server-side (thông qua BullMQ 5 phân lớp hàng đợi và Cloudflare R2/MinIO) được điều phối và bắt đầu xử lý trong vòng dưới 2 giây.
  * Triển khai cụm vi dịch vụ chuyên trách **Docling Sidecar Microservice** (`services/docling`) viết bằng Python/FastAPI kết hợp PyTorch và Pandoc, xử lý OCR đa ngữ (Việt - Anh), bóc tách cấu trúc bảng biểu chính xác cao và tái tạo tài liệu DOCX/Markdown hoàn chỉnh.
  * Tích hợp cơ chế **Nhận diện Năng lực Phần cứng (Hardware-Aware Throttling)**: Phân tầng thiết bị người dùng thành 3 cấp (Tier 1/2/3) dựa trên số nhân CPU, dung lượng RAM và WebGPU để tự động điều chỉnh mức độ xử lý song song, kích thước lô xử lý và chuyển hướng thực thi linh hoạt.
  * Động cơ nén tệp phát trực tuyến phía Client (**Streaming ZIP Engine với `fflate`**): Cho phép nén và đóng gói hàng trăm tệp tin đầu ra mà không gây tràn bộ nhớ RAM trình duyệt (< 150MB heap).
  * Áp dụng kiến trúc Zero-Proxy: Trình duyệt giao tiếp trực tiếp với Object Storage qua Presigned URL, giải phóng 95% áp lực băng thông và RAM trên API Server trung tâm.
  * Đảm bảo thời gian tải trang ban đầu (First Contentful Paint) đạt dưới 1.5 giây thông qua kỹ thuật Dynamic Code Splitting và lazy loading module WASM. Hỗ trợ PWA với huy hiệu cảnh báo trạng thái mạng Offline.

* **Về bảo mật & quyền riêng tư (Zero-Trace Privacy & Zero-Knowledge):**
  * Đối với các tác vụ xử lý trên Server: Thiết lập vòng đời dữ liệu tạm thời nghiêm ngặt, tự động xóa vĩnh viễn toàn bộ tệp tải lên và tệp kết quả sau đúng **30 phút** (thông qua cơ chế BullMQ Delayed Job, 2-phút cron cleanup kết hợp S3 Bucket Lifecycle Policy). Cung cấp nút *"Xóa ngay khỏi máy chủ"* để người dùng chủ động purge tệp tức thì.
  * Cách ly môi trường thực thi của Worker bằng các thư mục tạm (RAM Disk / `tmpfs`), đảm bảo tiến trình xử lý độc lập và xóa sạch tàn dư dữ liệu trong khối `finally`.
  * Cơ chế Zero-Knowledge One-Time Secret: Mã hóa đối xứng AES-GCM ngay tại trình duyệt trước khi đẩy ciphertext lên Redis; khóa giải mã chỉ nằm trên URL fragment (`#key=...`) và không bao giờ gửi đến máy chủ. Tự động xóa vĩnh viễn sau 1 lần truy cập thông qua lệnh Redis nguyên tử `getdel`.

---

## 1.3. Phạm vi Dự án (Project Scope)

Để đảm bảo dự án hoàn thành đúng tiến độ và tập trung nguồn lực vào các giá trị cốt lõi, phạm vi nghiên cứu và triển khai được xác định rõ ràng như sau:

### 1.3.1. Các hạng mục trong phạm vi (In-Scope)

| Nhóm phạm vi | Chi tiết hạng mục thực hiện |
| --- | --- |
| **Phân hệ chức năng** | • **Document Suite (20+ công cụ):** Ghép/tách/nén PDF, xoay trang, bảo mật & chữ ký số PDF, chuyển đổi Office sang PDF (`Gotenberg`), chuyển đổi PDF sang Word DOCX & Markdown chuyên sâu (`Docling Sidecar`), đóng dấu watermark & con dấu đỏ, đánh số trang Bates, tẩy sạch metadata ẩn, làm phẳng PDF forms, cắt viền trắng thừa auto-crop, dàn trang in sách booklet, so sánh trực quan visual diff, trộn mẫu hợp đồng mailmerge, tạo cây mục lục bookmark TOC, ghép PDF thành ảnh dài, chuyển PDF đen trắng tiết kiệm mực, phục hồi file PDF hỏng, biên dịch Markdown + Mermaid sang PDF, trích xuất bảng biểu ra Excel, chuyển đổi sách điện tử EPUB/MOBI/AZW3.<br>• **Media Studio (14+ công cụ):** Nén ảnh hàng loạt, đổi định dạng (HEIC/WebP/AVIF $\leftrightarrow$ JPG/PNG), cắt ảnh thẻ chuẩn 300 DPI (3x4, 4x6), xóa nền AI & làm mờ thông tin nhạy cảm CCCD, kiểm tra & xóa EXIF/GPS, đóng dấu bản quyền hàng loạt, cắt khoảng lặng ghi âm, làm đẹp ảnh chụp màn hình, lọc tiếng ồn cuộc họp AI Denoise, ghép phụ đề cứng video, chuyển video sang GIF/WebP động, phẳng hóa tài liệu chụp nghiêng CamScanner, quay màn hình thuyết minh trực tiếp, làm nét ảnh chữ mờ WebGPU.<br>• **Data & Text Lab (13+ công cụ):** Chuyển mã phông chữ tiếng Việt (TCVN3/VNI $\leftrightarrow$ Unicode), chuẩn hóa bảng tính Excel/CSV (lỗi định dạng ngày tháng, tách họ tên), so sánh đối soát văn bản & bảng tính (Diff Checker), chuyển đổi cấu trúc dữ liệu đa năng (Excel $\leftrightarrow$ JSON/SQL/MD), tách file Excel theo cột, gộp nhiều file Excel/sheet, nối trực quan bảng tính, gỡ mật khẩu bảo vệ sheet Excel, phép toán tập hợp danh sách, sửa lỗi font mojibake, ẩn danh hóa dữ liệu nhạy cảm PII, định dạng SQL linter, mã hóa/giải mã Base64 file.<br>• **Vietnam Office Micro-Tools (9+ công cụ):** Đọc số tiền thành chữ chuẩn kế toán (VND/USD), tính lương Gross $\leftrightarrow$ Net & thuế TNCN, tạo mã VietQR chuyển khoản Napas247 & chia bill nhóm, đọc hóa đơn điện tử XML e-Invoice VN, kiểm tra hợp lệ CCCD 12 số & Mã số thuế MST, chuẩn hóa địa chỉ hành chính 3 cấp VN, tính ngày làm việc trừ ngày lễ xuất file ICS, bảng tính trả nợ vay ngân hàng amortization, tra cứu mã định danh ngân hàng Bank BIN Mapper.<br>• **Universal Converter & Privacy:** Trạm chuyển đổi tệp vạn năng (Universal File Converter Hub) tự động nhận diện ma trận định dạng và gói ZIP, tính năng chia sẻ liên kết bí mật tự hủy Zero-Knowledge One-Time Secret Link.<br>• **Daily Utilities & Hardware:** Kiểm tra bàn phím/chuột/màn hình, kiểm tra webcam/micro/loa, bộ công cụ UTM campaign builder, định dạng font chữ mạng xã hội unicode, quay số ngẫu nhiên mật mã raffle, tính điểm hòa vốn sàn TMĐT, kiểm tra vùng an toàn mạng xã hội, tìm giờ họp đa múi giờ quốc tế. |
| **Tính năng hệ thống & UX** | • Trình điều phối Command Palette (`Ctrl + K`) và Trình bắt sự kiện Clipboard (`Ctrl + V`).<br>• Trình định tuyến kéo-thả tệp thông minh theo ngữ cảnh (Smart Dropzone Router).<br>• Trình xây dựng chuỗi công cụ tự động (Workflow Pipeline Builder với `@dnd-kit`).<br>• Cơ chế truyền tệp nhanh P2P giữa Điện thoại và Máy tính (Quick Drop) qua mã QR sử dụng WebRTC DataChannel và WebSocket signaling.<br>• Chia sẻ và khôi phục tức thì cấu hình công cụ qua URL nén (`Shareable Preset URLs` với `lz-string`).<br>• Chế độ cửa sổ thu nhỏ Picture-in-Picture (PiP) và hộp thoại nổi kéo-thả đa nhiệm.<br>• Báo cáo tiến độ xử lý file thời gian thực đa tầng (Stepper Progress UX) thông qua Server-Sent Events (SSE).<br>• Huy hiệu trạng thái mạng PWA (Online/Offline Resilience Indicator). |
| **Hạ tầng & Dữ liệu** | • Hệ thống lưu trữ tệp tạm thời không lưu vết (Cloudflare R2 / MinIO) với cơ chế tự động dọn dẹp sau 30 phút (Delayed Queue, Cron 2 phút, S3 Bucket Lifecycle).<br>• Cơ sở dữ liệu PostgreSQL 18 + TypeORM 0.3+ với cấu trúc lai Relational + JSONB có GIN Index.<br>• Hệ thống hàng đợi Redis 8.10 + BullMQ phân lớp 5 hàng đợi (`fast-light`, `office-convert`, `heavy-media`, `ai-ocr`, `ai-docling`).<br>• Vi dịch vụ Docling Sidecar Microservice (`services/docling`) với PyTorch/Pandoc/FastAPI chạy trên mạng nội bộ cô lập.<br>• Cơ chế xác thực tài khoản cơ bản (OAuth2 Google / Email) dùng để lưu trữ các mẫu Workflow cá nhân và xem lịch sử tác vụ.<br>• Chế độ Khách (Guest Mode) cho phép sử dụng toàn bộ công cụ mà không bắt buộc phải đăng nhập. |
| **Môi trường triển khai** | • Ứng dụng web tương thích hoàn toàn trên các trình duyệt máy tính hiện đại (Chrome, Edge, Firefox, Safari, Brave) có hỗ trợ WebAssembly và Web Workers.<br>• Thiết kế Web tương thích (Responsive Web / PWA) hỗ trợ thiết bị di động quét mã QR nhận diện và truyền tệp qua Quick Drop. |

### 1.3.2. Các hạng mục ngoài phạm vi (Out-of-Scope)

* **Không thay thế bộ công cụ văn phòng trực tuyến:** Hệ thống không xây dựng các trình soạn thảo tài liệu cộng tác thời gian thực (như Google Docs, Google Sheets hay Office 365). Dự án chỉ đóng vai trò là trạm xử lý, chuyển đổi và tiền xử lý định dạng tệp.
* **Không lưu trữ đám mây vĩnh viễn:** Hệ thống không cung cấp không gian lưu trữ tài liệu lâu dài như Google Drive hay Dropbox. Mọi tệp tin xử lý trên server đều là dữ liệu tạm và bị hủy bỏ hoàn toàn sau 30 phút.
* **Không phát triển ứng dụng di động bản địa (Native Mobile App):** Dự án không phát triển ứng dụng riêng cho iOS hoặc Android trong giai đoạn này; giao diện trên thiết bị di động chỉ được hỗ trợ thông qua thiết kế Web tương thích (Responsive Web / PWA) phục vụ mục đích quét QR và gửi tệp qua Quick Drop.
* **Không huấn luyện mô hình AI tùy biến (Custom AI Model Training):** Không đầu tư hạ tầng GPU chuyên sâu để train hoặc fine-tune các mô hình AI ngôn ngữ lớn (LLM); các tác vụ AI (như Docling, PaddleOCR, RMBG) chỉ sử dụng các mô hình mã nguồn mở tối ưu sẵn thông qua Python Sidecar, ONNX Runtime hoặc thư viện nhúng.
