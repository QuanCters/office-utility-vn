# 3. Đặc tả Yêu cầu Phi chức năng (Non-Functional Requirements - NFR)

Đối với một nền tảng tiện ích tổng hợp xử lý tệp tin trực tuyến với hơn 60 công cụ chuyên sâu, chất lượng trải nghiệm và sự sống còn của hệ thống phụ thuộc trực tiếp vào tốc độ phản hồi, khả năng cô lập tài nguyên khi xử lý các tệp tin nặng, và cam kết bảo vệ dữ liệu nhạy cảm của người dùng. Dưới đây là các chỉ số kỹ thuật bắt buộc (SLAs & SLOs) cho toàn bộ hệ thống.

---

## 3.1. Yêu cầu về Hiệu năng & Tốc độ Phản hồi (Performance & Latency)

Hệ thống phải đảm bảo sự mượt mà tuyệt đối ở giao diện người dùng (UI) ngay cả khi đang xử lý các tệp tin dung lượng lớn ở tầng nền hoặc điều phối qua hàng đợi máy chủ.

| Mã NFR | Hạng mục Hiệu năng | Chỉ số Mục tiêu (Target Metric / SLA) | Giải pháp Kỹ thuật Đảm bảo |
| --- | --- | --- | --- |
| **NFR-PERF-01** | **Tốc độ tải trang & Khởi chạy ứng dụng (Core Web Vitals)** | • **LCP** (Largest Contentful Paint): Dưới **1.5 giây**.<br>• **INP** (Interaction to Next Paint): Dưới **100ms**.<br>• Kích thước gói JavaScript khởi tạo (Initial Bundle Size): Nhỏ hơn **180 KB** (đã nén Brotli/Gzip). | • Áp dụng **Route-based & Tool-based Code Splitting**: Chỉ tải mã nguồn của trang khung (App Shell) khi vào trang chủ.<br>• Các tệp nhị phân nặng (`.wasm` của `FFmpeg`, `PDF.js`, `ONNX`, `SheetJS`) chỉ được tải động (Dynamic Import) khi người dùng mở đúng công cụ đó, sau đó lưu đệm vào **IndexedDB** để lần truy cập thứ 2 đạt tốc độ khởi chạy **0ms**. |
| **NFR-PERF-02** | **Độ mượt giao diện khi xử lý tại chỗ (Non-blocking Client Execution)** | • Tốc độ khung hình giao diện chính (Main Thread UI) luôn duy trì **60 FPS**.<br>• Tuyệt đối không xuất hiện cảnh báo *"Page Unresponsive"* của trình duyệt khi đang nén ảnh hay ghép PDF. | • **100% tác vụ tính toán Client-side** (đọc/ghi PDF, giải mã ảnh HEIC, phân tích 50.000 dòng Excel, tính mã băm SHA-256) bắt buộc phải chạy bên trong **Web Workers** tách biệt hoàn toàn với luồng giao diện (DOM Thread).<br>• Sử dụng `Transferable Objects` (`ArrayBuffer`) để truyền dữ liệu giữa Main Thread và Worker với chi phí sao chép bộ nhớ bằng 0 (Zero-copy). |
| **NFR-PERF-03** | **Độ trễ API & Thông lượng (API Latency & Throughput)** | • Độ trễ phản hồi API (cấp Presigned URL, khởi tạo Job, lưu Secret): **p95 dưới 80ms**, **p99 dưới 150ms**.<br>• Độ trễ đẩy bản tin tiến độ qua SSE: Dưới **200ms** kể từ khi Worker phát sự kiện. | • Sử dụng **NestJS với `FastifyAdapter`** thay vì Express.<br>• Áp dụng kiến trúc **Zero-Proxy Binary**: API Server chỉ xử lý các gói tin JSON vài KB, tuyệt đối không nhận luồng upload/download tệp nhị phân. |
| **NFR-PERF-04** | **Thời gian xử lý tác vụ theo 5 phân lớp hàng đợi BullMQ** | • **Nhóm `fast-light`** *(Ảnh Sharp, QPDF, XML)*: Hoàn tất dưới **2 giây**.<br>• **Nhóm `office-convert`** *(Gotenberg, LibreOffice, Grayscale dưới 30 trang)*: Hoàn tất dưới **8 giây**.<br>• **Nhóm `heavy-media`** *(FFmpeg, Calibre Ebook)*: Báo % tiến độ mỗi **1 giây**.<br>• **Nhóm `ai-ocr`** *(PaddleOCR)*: Thời gian chờ hàng đợi dưới **5 giây**.<br>• **Nhóm `ai-docling`** *(Docling Sidecar & Pandoc)*: Hoàn tất tài liệu phức tạp 20 trang dưới **15 giây**. | • Gắn ổ đĩa tạm của Worker (`workDir`) lên phân vùng **RAM Disk (`tmpfs`)** trên Linux để tốc độ đọc/ghi tệp trung gian đạt **5–10 GB/s**, loại bỏ hoàn toàn nút thắt cổ chai I/O ổ cứng (Disk I/O bottleneck). |
| **NFR-PERF-05** | **Thích ứng Phần cứng Khách (Hardware-Aware Throttling)** | • Tự động sniffing phần cứng người dùng qua `navigator.hardwareConcurrency`, `navigator.deviceMemory`, và `navigator.gpu` để phân loại 3 tầng thiết bị (Tier 1/2/3).<br>• Không bao giờ để CPU client đạt 100% gây đơ máy tính văn phòng. | • **Tier 1 (Máy yếu: $\le 4$ cores / $\le 4$GB RAM):** Giới hạn tối đa 1 Web Worker, giới hạn lô ảnh $\le 5$ tệp, trần PDF $\le 30$MB; các tác vụ vượt ngưỡng tự động định tuyến lên Server.<br>• **Tier 2 (Trung bình: 4–8 cores / 4–16GB RAM):** Chạy song song 2–3 Worker, lô ảnh $\le 15$ tệp, PDF $\le 80$MB.<br>• **Tier 3 (Máy mạnh: $> 8$ cores / $> 16$GB RAM + WebGPU):** Chạy song song 4–6 Worker, lô ảnh $\le 30$ tệp, PDF $\le 200$MB. |
| **NFR-PERF-06** | **Động cơ Đóng gói ZIP Phát trực tuyến (Streaming ZIP Engine)** | • Dung lượng bộ nhớ RAM (JS Heap) khi nén hàng chục tệp kết quả thành file `.zip` luôn duy trì **dưới 150 MB** ngay cả khi kích thước gói ZIP đạt **500 MB**. | • Sử dụng thư viện **`fflate`** với kỹ thuật Stream Compression (ghi từng khối dữ liệu nhị phân tuần tự trực tiếp qua Web Streams API), triệt tiêu hoàn toàn nguy cơ tràn bộ nhớ RAM trình duyệt (Out-of-Memory Crash). |

---

## 3.2. Yêu cầu về Bảo mật & Tuân thủ Quy định Pháp lý (Security & Compliance)

Do người dùng thường xuyên xử lý các tài liệu chứa thông tin cá nhân và bí mật kinh doanh (hợp đồng, bảng lương, CCCD, báo cáo thuế), hệ thống được thiết kế theo tiêu chuẩn **Privacy-by-Design** và tuân thủ chặt chẽ **Nghị định 13/2023/NĐ-CP về Bảo vệ dữ liệu cá nhân** tại Việt Nam.

### 3.2.1. Tuân thủ Nghị định 13/2023/NĐ-CP và Quyền riêng tư Dữ liệu

* **NFR-SEC-01 (Nguyên tắc Tối thiểu hóa & Không thu thập dữ liệu - Data Minimization):**
  * Đối với các công cụ gắn nhãn **"100% On-Device (Xử lý tại chỗ)"**, toàn bộ quá trình đọc, biến đổi và xuất tệp diễn ra độc quyền trong bộ nhớ RAM của trình duyệt người dùng. Hệ thống cam kết không gửi bất kỳ byte dữ liệu nội dung nào về máy chủ (chỉ ghi nhận số liệu thống kê ẩn danh về số lượt gọi công cụ nếu người dùng đồng ý).

* **NFR-SEC-02 (Quyền được xóa & Cơ chế Hủy dữ liệu Không thể khôi phục - Zero-Trace Retention):**
  * Đối với các tác vụ bắt buộc xử lý trên máy chủ (Server-side Queue), hệ thống đóng vai trò là *Bên xử lý dữ liệu tạm thời* theo yêu cầu trực tiếp của người dùng:
    1. **Trong quá trình xử lý:** Tệp được tải về thư mục cô lập `/tmp/jobs/{uuid}` trên RAM Disk (`tmpfs`). Ngay khi tiến trình kết thúc (dù thành công hay thất bại), khối `finally` sẽ xóa trắng thư mục này khỏi bộ nhớ RAM.
    2. **Sau khi xử lý:** Tệp gốc và tệp kết quả trên Object Storage (Cloudflare R2 / MinIO) tự động bị xóa vĩnh viễn sau đúng **30 phút** thông qua cơ chế 3 tầng: BullMQ Delayed Job, tiến trình quét dọn Cron 2 phút một lần (`cleanup.service.ts`), và chính sách vòng đời bucket (S3 Lifecycle TTL 30 phút).
    3. **Quyền chủ động của chủ thể dữ liệu:** Giao diện kết quả luôn hiển thị nút **"Xóa ngay khỏi máy chủ" (Purge Now)**. Khi người dùng nhấn nút này, hệ thống thực thi lệnh `DeleteObjectsCommand` lên Object Storage và xóa bản ghi trong PostgreSQL trong vòng **dưới 1 giây**.

* **NFR-SEC-03 (Mã hóa Dữ liệu Toàn trình - Encryption in Transit & at Rest):**
  * Mọi kết nối HTTP, SSE, WebSocket và Presigned URL bắt buộc sử dụng giao thức **TLS 1.3 (HTTPS)**.
  * Các tệp lưu tạm trên Object Storage được mã hóa tự động ở tầng lưu trữ bằng chuẩn **AES-256 (Server-Side Encryption)**. Đường dẫn tải tệp (`presignedDownloadUrl`) được ký bằng thuật toán **AWS Signature v4**, chỉ có hiệu lực trong **15 phút** và gắn kèm header `Content-Disposition: attachment` để ngăn chặn việc nhúng link phát tán công khai.

* **NFR-SEC-07 (Bảo mật Không Tri thức cho Liên kết Mật - Zero-Knowledge One-Time Secret Link):**
  * Đối với công cụ chia sẻ văn bản mật (`/secret-link`):
    1. **Mã hóa hoàn toàn tại Client:** Trình duyệt sử dụng **Web Crypto API** với thuật toán chuẩn **AES-GCM-256** và vector khởi tạo **IV 96-bit** ngẫu nhiên.
    2. **Khóa giải mã không bao giờ rời khỏi trình duyệt:** Khóa giải mã được đính kèm vào URL fragment (`#key=...`). Do URL fragment theo chuẩn RFC 3986 không bao giờ được gửi trong HTTP request header, máy chủ hoàn toàn không biết khóa giải mã (Zero-Knowledge).
    3. **Tiêu hủy nguyên tử sau 1 lần đọc:** Máy chủ chỉ lưu `ciphertext` và `iv` vào Redis với thời gian sống tối đa 30 phút (1800 giây). Khi người nhận mở link, backend thực thi lệnh Redis nguyên tử **`GETDEL`**; dữ liệu bị xóa khỏi RAM máy chủ ngay lập tức tại thời điểm trả về ciphertext. Lần truy cập thứ 2 sẽ nhận mã lỗi `404 Not Found`.

### 3.2.2. Phòng chống Tấn công bằng Tệp độc hại (Malicious File Defense)

* **NFR-SEC-04 (Xác thực Chữ ký Tệp - Magic Bytes Validation):**
  * Hệ thống không bao giờ tin tưởng vào phần mở rộng tên tệp (`.pdf`, `.jpg`, `.xlsx`) hay header `Content-Type` do trình duyệt gửi lên.
  * Trước khi Worker đưa tệp vào nhân xử lý, hệ thống bắt buộc đọc **16–64 byte đầu tiên (File Header / Magic Bytes)** thông qua hàm kiểm tra nhị phân để đối chiếu chữ ký thực tế (PDF phải bắt đầu bằng `%PDF-`, PNG phải là `89 50 4E 47`, ZIP/DOCX/XLSX phải là `50 4B 03 04`). Nếu phát hiện sai lệch, lập tức trả về mã lỗi `415 MAGIC_BYTES_MISMATCH`, hủy Job và xóa file trên Storage.

* **NFR-SEC-05 (Chống tấn công Zip Bomb, PDF Bomb & Pixel Flood):**
  * **Kiểm soát tỷ lệ giải nén:** Khi xử lý các tệp nén (`.zip`, `.docx`, `.xlsx`), nếu tỷ lệ giải nén vượt quá ngưỡng **1:100** hoặc tổng dung lượng sau giải nén vượt quá **500MB**, Worker lập tức ngắt tiến trình (`SIGKILL`) để chống tràn bộ nhớ.
  * **Giới hạn kích thước điểm ảnh (Image Bomb):** Cấu hình `Sharp({ limitInputPixels: 268435456 })` từ chối các tệp ảnh có kích thước cạnh vượt quá **16.384 px** (ngăn chặn tệp ảnh nặng 100KB nhưng khi bung ra chiếm hàng chục GB RAM).

* **NFR-SEC-06 (Cô lập Môi trường Thực thi - Process Sandboxing & Chống Command Injection):**
  * Các công cụ gọi tiến trình hệ thống (`LibreOffice`, `Ghostscript`, `QPDF`, `FFmpeg`, `Pandoc`) tuyệt đối không bao giờ được dùng `child_process.exec` hay `spawn(..., { shell: true })` nhằm **triệt tiêu 100% lỗ hổng Command Injection**. Bắt buộc sử dụng `execFile` hoặc `spawn` với mảng tham số tách rời và `{ shell: false }`.
  * Container của Worker chạy dưới quyền người dùng không có đặc quyền (`non-root user`, UID `1001:1001`), hệ thống tệp gốc ở chế độ chỉ đọc (`read-only root filesystem`, chỉ cho phép ghi vào `/tmp`), và đặt trong mạng nội bộ cô lập (`isolated-worker-net`, `internal: true`) để vô hiệu hóa hoàn toàn các cuộc tấn công **SSRF (Server-Side Request Forgery)**.

* **NFR-SEC-08 (An toàn Cô lập Vi dịch vụ Docling Sidecar):**
  * Vi dịch vụ Docling Sidecar (`services/docling`) chạy bằng container Python 3.11 độc lập:
    1. Xác thực bắt buộc bằng mã thông báo nội bộ bí mật qua header `X-Internal-Token`.
    2. Chạy dưới quyền `user: "1000:1000"`, loại bỏ toàn bộ Linux capabilities (`cap_drop: ALL`), bật `no-new-privileges: true`.
    3. Kiểm tra bảo mật đường dẫn `validate_job_path` nghiêm ngặt: Tuyệt đối không cho phép truy cập tệp ngoài phạm vi thư mục được cấp phát trong `WORK_DIR=/tmp/jobs` (chống lỗ hổng Path Traversal / LFI).

---

## 3.3. Yêu cầu về Độ tin cậy & Khả năng Chịu lỗi (Reliability & Fault Tolerance)

* **NFR-REL-01 (Độ sẵn sàng của hệ thống - System Availability):**
  * Cam kết thời gian hoạt động (Uptime) của hệ thống cốt lõi và cụm công cụ Client-side đạt **99.9%** (tương đương thời gian gián đoạn tối đa không quá 43 phút/tháng).

* **NFR-REL-02 (Cơ chế Suy giảm Dịch vụ Mềm dẻo - Graceful Degradation & Circuit Breaker):**
  * Nếu cụm máy chủ Backend hoặc Redis gặp sự cố bảo trì, **toàn bộ hơn 45 công cụ chạy bằng WebAssembly ở phía Client vẫn phải hoạt động bình thường 100%** (nhờ trang tĩnh được phân phối qua CDN/Edge).
  * Mỗi công cụ trong `Tool Registry` có trạng thái độc lập (`ACTIVE`, `DEGRADED`, `MAINTENANCE`). Nếu container `gotenberg` hoặc `docling` bị quá tải, hệ thống chỉ tạm khóa riêng công cụ tương ứng và hiển thị thông báo thân thiện, các công cụ khác vẫn vận hành bình thường.

* **NFR-REL-03 (Chống treo tiến trình & Tự phục hồi - Timeout, Retry & Semaphore Concurrency):**
  * **Hard Timeout:** Mọi Job chạy trên Worker đều có thời gian giới hạn cứng (**60 giây** cho công cụ nhanh, **180 giây** cho Docling/OCR/Video). Quá thời gian trên, Worker tự động hủy tiến trình con (`AbortController` + `SIGKILL`) để giải phóng tài nguyên.
  * **Kiểm soát Tải AI Docling:** Vi dịch vụ Docling Sidecar áp dụng biến đếm cờ hiệu `asyncio.Semaphore(1)` để đảm bảo mỗi container chỉ xử lý duy nhất 1 tác vụ mô hình AI tại một thời điểm, các yêu cầu tiếp theo được xếp hàng chờ nhằm tránh xung đột VRAM/CPU.
  * **Cơ chế Thử lại Thông minh (Exponential Backoff Retry):** Với các lỗi mạng tạm thời (kết nối S3/R2 chập chờn), BullMQ tự động thử lại tối đa **3 lần** ($2\text{s} \rightarrow 4\text{s} \rightarrow 8\text{s}$). Với các lỗi tệp hỏng (Corrupted File), đánh dấu `FAILED` ngay lập tức để tiết kiệm tài nguyên.

* **NFR-REL-04 (Tính lũy đẳng - Idempotency & Deduplication):**
  * Trình duyệt tính mã băm **SHA-256** của tệp gốc kết hợp với `toolSlug` và `options` để tạo `idempotency_key`. Nếu người dùng bấm liên tiếp nút chuyển đổi cho cùng 1 tệp trong vòng 5 phút, hệ thống trả về ngay kết quả của Job đang chạy hoặc vừa hoàn tất.

---

## 3.4. Yêu cầu về Khả năng Mở rộng & Bảo trì (Scalability & Maintainability)

Hệ thống được thiết kế để chịu được lưu lượng tăng đột biến vào khung giờ cao điểm văn phòng (8h30 – 11h30 và 13h30 – 17h00 từ Thứ Hai đến Thứ Sáu).

* **NFR-SCA-01 (Mở rộng Ngang Độc lập - Stateless Horizontal Scaling qua 5 Hàng đợi BullMQ):**
  * Cả `nestjs-api`, `nestjs-worker` và `docling-sidecar` đều được thiết kế **100% Stateless**.
  * Hệ thống phân luồng tải thành **5 hàng đợi chuyên biệt**:
    1. `fast-light`: Xử lý ảnh Sharp, phục hồi QPDF, định dạng CSV/XML.
    2. `office-convert`: Xử lý LibreOffice/Gotenberg, chuyển đổi thang xám Ghostscript.
    3. `heavy-media`: Xử lý FFmpeg video/audio, Calibre ebook.
    4. `ai-ocr`: Xử lý nhận dạng chữ tiếng Việt PaddleOCR.
    5. `ai-docling`: Xử lý bóc tách tài liệu cấu trúc sâu IBM Docling + Pandoc DOCX/Markdown.
  * Khi `Queue Depth` của bất kỳ hàng đợi nào vượt quá **50 Jobs**, hệ thống tự động scale thêm worker container tương ứng kết nối vào Redis mà không ảnh hưởng tới API Server.

* **NFR-SCA-02 (Kiểm soát Chống quá tải & Phân bổ Công bằng - Rate Limiting & Fair Queuing):**
  * Áp dụng thuật toán **Token Bucket / Sliding Window** trên Redis ở tầng API Gateway để ngăn chặn bot gửi tệp ồ ạt (15 req/giờ cho Guest, 60 req/giờ cho User).
  * Giới hạn tài nguyên ở tầng Docker (`deploy.resources.limits` về CPU và RAM cho từng container: Docling tối đa 2.0 CPUs / 3GB RAM, Gotenberg 2.0 CPUs / 2GB RAM) ngăn chặn sập máy chủ vật lý do OOM.

* **NFR-SCA-03 (Tính Mô-đun hóa Cực hạn - Zero-Touch Auto-Discovery & OCP):**
  * Tuân thủ triệt để nguyên tắc **Open-Closed Principle (OCP)**: Việc bổ sung thêm một công cụ mới vào hệ thống hoàn toàn độc lập theo cơ chế tự động quét:
    * Khai báo metadata: `packages/shared/src/tools/<slug>.meta.ts`
    * Giao diện & Worker Client: `apps/web/src/tools/<slug>/`
    * Bộ xử lý Backend: `apps/api/src/tools/<slug>/<slug>.processor.ts`
  * Không bao giờ phải sửa đổi mã nguồn cốt lõi của tầng Hàng đợi, Quản lý Vòng đời Tệp hay Cơ chế Dọn dẹp Zero-Trace.

---

## 3.5. Yêu cầu về Tính Tương thích & Trải nghiệm Người dùng Nâng cao (Compatibility & Modern UX)

* **NFR-COM-01 (Tương thích Trình duyệt & WebAssembly SIMD):**
  * Hoạt động ổn định trên 2 phiên bản mới nhất của **Google Chrome, Microsoft Edge, Apple Safari, Mozilla Firefox và Cốc Cốc** trên Windows (10/11), macOS và Linux.
  * Tự động kiểm tra hỗ trợ `WASM SIMD` và `WebGPU` để tăng tốc tính toán AI trên máy có phần cứng phù hợp, chuyển đổi mềm mại về CPU nếu không có GPU rời.

* **NFR-COM-02 (Bản địa hóa Tiếng Việt & An toàn Gõ phím IME):**
  * Hỗ trợ đầy đủ **Light Mode** và **Dark Mode** tự động theo hệ điều hành hoặc người dùng chọn.
  * Tương thích tuyệt đối với các bộ gõ tiếng Việt phổ biến (**UniKey, EVKey, OpenKey, bộ gõ macOS**) nhờ xử lý chặt chẽ sự kiện `compositionstart` / `compositionend`, không bao giờ bị nuốt chữ hay nhân đôi ký tự trong thanh tìm kiếm `Ctrl + K`.

* **NFR-COM-03 (Khả năng Phục hồi Ngoại tuyến PWA & Theo dõi Trạng thái Mạng):**
  * Ứng dụng tích hợp trình giám sát trạng thái mạng tự động (`navigator.onLine`, sự kiện `online` / `offline`).
  * Khi người dùng bị mất kết nối Internet, hệ thống hiển thị huy hiệu màu cam thân thiện: *"Đang ngoại tuyến · Các công cụ 100% On-Device vẫn hoạt động bình thường"*, đồng thời vô hiệu hóa các công cụ Server-side để tránh gửi lệnh lỗi.

* **NFR-COM-04 (Đa nhiệm với Document Picture-in-Picture & Cửa sổ Nổi):**
  * Hỗ trợ chuẩn **Document Picture-in-Picture API** (`window.documentPictureInPicture.requestWindow`) cho phép mở các công cụ tiện ích (đọc số tiền, tính thuế, chuyển đổi font) thành một cửa sổ nổi độc lập luôn nằm trên cùng của hệ điều hành.
  * Với các trình duyệt chưa hỗ trợ API này (Firefox, Safari), hệ thống tự động kích hoạt chế độ **Floating Fallback Modal** (hộp thoại nổi có thể kéo-thả và thu nhỏ vào góc màn hình) đảm bảo trải nghiệm đa nhiệm đồng nhất.
