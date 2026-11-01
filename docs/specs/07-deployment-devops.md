# 7. Kiến trúc Triển khai Hạ tầng, Cấu hình Docker Compose & Vận hành Giám sát (Deployment Architecture, DevOps & Observability)

Kiến trúc triển khai hạ tầng của nền tảng được xây dựng để hiện thực hóa ba cam kết cốt lõi trong tài liệu: **Chi phí vận hành tối thiểu (Hybrid Edge + Stateless Workers)**, **Hiệu năng I/O cực đại bằng ổ đĩa RAM (`tmpfs`)**, và **Cô lập an toàn tuyệt đối (Sandboxed Zero-Trace Execution)**.

---

## 7.1. Sơ đồ Tô-pô Triển khai Hạ tầng (Infrastructure Topology Diagram)

Hệ thống phân tách hoàn toàn luồng phân phối giao diện tĩnh / mô-đun WebAssembly ở tầng Biên (Edge CDN) và luồng xử lý tác vụ nặng ở cụm máy chủ Backend thông qua mạng nội bộ cô lập (Isolated Docker Networks):

```mermaid
flowchart TB
    %% Tầng Người dùng
    Client(["💻 Trình duyệt Người dùng<br/>(Web Workers + WASM + IndexedDB Cache)"])
    Mobile(["📱 Điện thoại Di động<br/>(Quick Drop PWA / QR Scanner)"])

    %% Tầng Edge & Lưu trữ Đám mây
    subgraph CloudflareEdge ["Tầng Phân phối Biên & Lưu trữ Tạm (Cloudflare Edge)"]
        CDN["⚡ Cloudflare Pages / CDN<br/>(App Shell < 180KB + Lazy WASM Bundles)"]
        R2[("🪣 Cloudflare R2 / MinIO<br/>(AES-256 • Bucket Lifecycle TTL = 30m)")]
    end

    %% Cụm Máy chủ Ứng dụng & Hàng đợi
    subgraph VPSCluster ["Cụm Máy chủ Xử lý Trung tâm (Docker Host / Linux Server)"]
        direction TB

        subgraph PublicNet ["🌐 Mạng Giao tiếp Ngoài (frontend-egress-net)"]
            Gateway["🛡️ Reverse Proxy / API Gateway<br/>(TLS 1.3 • Rate Limiting • CORS)"]
            API["🚀 nestjs-api (FastifyAdapter)<br/>(REST API • SSE Stream • Socket.io Signaling)"]
        end

        subgraph InternalNet ["🔒 Mạng Nội bộ Cô lập - Chặn Internet Ngoài (isolated-worker-net • internal: true)"]
            Redis[("🧠 Redis 8 (AOF)<br/>(BullMQ 5 Queues • Rate Limit • Ephemeral Secrets)")]
            Postgres[("🐘 PostgreSQL 18<br/>(Relational + JSONB + Partial Index)")]
            
            WorkerFast["⚙️ worker-fast-light<br/>(Sharp • QPDF • SheetJS)<br/>[RAM Disk /tmp: 2GB]"]
            WorkerOffice["📄 worker-office-ai<br/>(Ghostscript • FFmpeg • PaddleOCR • Pandoc)<br/>[RAM Disk /tmp: 4GB]"]
            Gotenberg["🖨️ Gotenberg Sidecar<br/>(LibreOffice Headless + Bộ Font VN)<br/>[RAM Disk /tmp: 2GB]"]
            Docling["🤖 Docling AI Sidecar<br/>(IBM Docling • PyTorch • OCR)<br/>[RAM Disk /tmp: 3GB]"]
        end
    end

    %% Luồng kết nối
    Client <-->|"1. Tải UI & WASM (HTTPS)"| CDN
    Client <-->|"2. PUT/GET Tệp Trực tiếp (Presigned URL - Zero Proxy)"| R2
    Client <-->|"3. JSON API (<80ms) & Luồng SSE Tiến độ"| Gateway
    Mobile <-->|"4. WebRTC P2P DataChannel"| Client
    Mobile <-->|"Signaling Room"| Gateway

    Gateway --> API
    API <--> Redis
    API <--> Postgres
    API -.->|"Ký & Cấp link Presigned / Lệnh Purge"| R2

    Redis <-->|"BullMQ Dispatch & Progress Events (5 Queues)"| WorkerFast
    Redis <-->|"BullMQ Dispatch & Progress Events (5 Queues)"| WorkerOffice
    WorkerOffice <-->|"HTTP Nội bộ (Port 3000)"| Gotenberg
    WorkerOffice <-->|"HTTP Nội bộ + X-Internal-Token (Port 5001)"| Docling

    WorkerFast <-->|"Tải Input / Đẩy Output qua VPC Endpoint"| R2
    WorkerOffice <-->|"Tải Input / Đẩy Output qua VPC Endpoint"| R2

```

### Đặc điểm Kỹ thuật của Tô-pô Triển khai

1. Tầng Edge CDN (Chịu tải 70–80% lưu lượng với chi phí Server = 0đ):

* Toàn bộ khung ứng dụng (App Shell dưới **180 KB**) và các gói nhị phân `.wasm` (`pdf-lib`, `PDF.js`, `FFmpeg.wasm`, `ONNX`) được phân phối qua mạng lưới CDN biên.

* Ngay cả khi cụm máy chủ Backend đang bảo trì, người dùng vẫn truy cập và sử dụng bình thường hơn 45 công cụ Client-side trong tổng số hơn 60 công cụ (`NFR-REL-02`).

2. Luồng Truyền tải Nhị phân Zero-Proxy (`Client` $\leftrightarrow$ `Cloudflare R2`):

* Băng thông tải tệp lên và tải tệp về đi trực tiếp giữa trình duyệt và Cloudflare R2 thông qua `presignedPutUrl` và `presignedDownloadUrl`.

* Container `nestjs-api` chỉ xử lý các bản tin điều phối JSON nhẹ vài KB, giúp duy trì độ trễ API **p95 < 80ms** ngay cả trên cấu hình máy chủ khiêm tốn (`NFR-PERF-03`).

3. Cách ly Mạng Nội bộ Chống SSRF (`isolated-worker-net`):

* Các container xử lý tệp (`worker-fast-light`, `worker-office-ai`, `gotenberg`) được đặt trong mạng nội bộ có cờ `internal: true` (hoặc chặn toàn bộ Outbound Internet trừ dải IP của R2/MinIO).

* Nếu kẻ tấn công cài mã độc SSRF (Server-Side Request Forgery) vào tài liệu Word, SVG hay PDF để ép máy chủ gọi ra mạng ngoài, kết nối sẽ bị từ chối ngay ở tầng nhân mạng Linux (`NFR-SEC-06`).

---

## 7.2. Đặc tả Cấu hình `docker-compose.prod.yml` Chuẩn Sản xuất

Tệp cấu hình dưới đây hiện thực hóa đầy đủ các yêu cầu phi chức năng ở Mục 3: gắn ổ đĩa **RAM Disk (`tmpfs`)** cho `/tmp` để đạt tốc độ đọc/ghi **5–10 GB/s** (`NFR-PERF-04`), chạy dưới quyền **`non-root user`**, khóa hệ thống tệp gốc ở chế độ **chỉ đọc (`read_only: true`)** (`NFR-SEC-06`), và giới hạn cứng tài nguyên CPU/RAM để chống lỗi **OOM Killer** (`NFR-SCA-02`).

```yaml
version: "3.9"

services:
  # ============================================================================
  # 1. API SERVER (NestJS + FastifyAdapter - Zero-Proxy Binary)
  # ============================================================================
  nestjs-api:
    image: registry.office-utility.vn/nestjs-api:latest
    container_name: office-utility-api
    restart: unless-stopped
    user: "1001:1001" # Chạy dưới quyền non-root user (NFR-SEC-06)
    read_only: true   # Hệ thống tệp gốc chỉ đọc
    environment:
      - NODE_ENV=production
      - PORT=3000
      - DATABASE_URL=postgresql://${DB_USER}:${DB_PASSWORD}@postgres:5432/${DB_NAME}
      - REDIS_URL=redis://:${REDIS_PASSWORD}@redis:6379
      - R2_ENDPOINT=${R2_ENDPOINT}
      - R2_ACCESS_KEY_ID=${R2_ACCESS_KEY_ID}
      - R2_SECRET_ACCESS_KEY=${R2_SECRET_ACCESS_KEY}
      - R2_BUCKET_NAME=office-utility-zero-trace
    ports:
      - "127.0.0.1:3000:3000"
    networks:
      - frontend-egress-net
      - isolated-worker-net
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    deploy:
      resources:
        limits:
          cpus: "1.50"
          memory: 1024M
        reservations:
          cpus: "0.50"
          memory: 512M
    healthcheck:
      test: ["CMD", "wget", "--no-verbose", "--tries=1", "--spider", "http://localhost:3000/api/v1/health"]
      interval: 15s
      timeout: 5s
      retries: 3

  # ============================================================================
  # 2. WORKER NHÓM NHANH (Sharp, QPDF, CSV - SLA hoàn tất < 2 giây)
  # ============================================================================
  worker-fast-light:
    image: registry.office-utility.vn/nestjs-worker:latest
    restart: unless-stopped
    user: "1001:1001"
    read_only: true
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    environment:
      - NODE_ENV=production
      - WORKER_QUEUES=fast-light
      - WORKER_CONCURRENCY=8
      - JOB_HARD_TIMEOUT_MS=60000 # Hard Timeout 60s (NFR-REL-03)
      - WORK_DIR=/tmp/jobs
      - DATABASE_URL=postgresql://${DB_USER}:${DB_PASSWORD}@postgres:5432/${DB_NAME}
      - REDIS_URL=redis://:${REDIS_PASSWORD}@redis:6379
      - R2_ENDPOINT=${R2_ENDPOINT}
      - R2_ACCESS_KEY_ID=${R2_ACCESS_KEY_ID}
      - R2_SECRET_ACCESS_KEY=${R2_SECRET_ACCESS_KEY}
    # Gắn phân vùng RAM Disk (tmpfs) tốc độ 5-10 GB/s, tự xóa sạch trên RAM (NFR-PERF-04 & NFR-SEC-02)
    tmpfs:
      - /tmp:size=2G,mode=1770,noexec,nosuid,nodev
    networks:
      - isolated-worker-net
    deploy:
      replicas: 2 # Mở rộng ngang Stateless (NFR-SCA-01)
      resources:
        limits:
          cpus: "2.00"
          memory: 2560M # Ngăn chặn OOM Killer khi gặp Image/Zip Bomb (NFR-SCA-02)

  # ============================================================================
  # 3. WORKER NHÓM NẶNG (Office Convert, Ghostscript, FFmpeg, AI OCR, Docling)
  # ============================================================================
  worker-heavy-office:
    image: registry.office-utility.vn/nestjs-worker-heavy:latest
    restart: unless-stopped
    user: "1001:1001"
    read_only: true
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    environment:
      - NODE_ENV=production
      - WORKER_QUEUES=office-convert,heavy-media,ai-ocr,ai-docling
      - WORKER_CONCURRENCY=2
      - JOB_HARD_TIMEOUT_MS=180000 # Hard Timeout 180s cho OCR/Video/Docling (NFR-REL-03)
      - GOTENBERG_URL=http://gotenberg:3000
      - DOCLING_URL=http://docling:5001
      - DOCLING_SECRET_TOKEN=${DOCLING_SECRET_TOKEN}
      - WORK_DIR=/tmp/jobs
      - DATABASE_URL=postgresql://${DB_USER}:${DB_PASSWORD}@postgres:5432/${DB_NAME}
      - REDIS_URL=redis://:${REDIS_PASSWORD}@redis:6379
      - R2_ENDPOINT=${R2_ENDPOINT}
    tmpfs:
      - /tmp:size=4G,mode=1770,noexec,nosuid,nodev
    networks:
      - isolated-worker-net
    deploy:
      replicas: 2
      resources:
        limits:
          cpus: "4.00"
          memory: 4096M

  # ============================================================================
  # 4. GOTENBERG SIDECAR (LibreOffice Headless + Bộ Font Tiếng Việt Chuẩn)
  # ============================================================================
  gotenberg:
    image: gotenberg/gotenberg:8
    container_name: office-utility-gotenberg
    restart: unless-stopped
    user: "1001:1001"
    read_only: true
    command:
      - "gotenberg"
      - "--libreoffice-disable-routes=false"
      - "--chromium-disable-javascript=true" # Chặn thực thi JS độc hại trong tài liệu
      - "--chromium-deny-list=^https?://.*"  # Chặn 100% kết nối SSRF ra ngoài (NFR-SEC-06)
    volumes:
      # Nạp bộ phông chữ tiếng Việt (Times New Roman, Arial, Roboto, TCVN3, VNI) theo FR-DOC-03
      - ./docker/fonts/vietnamese:/usr/local/share/fonts/vietnamese:ro
    tmpfs:
      - /tmp:size=2G,mode=1770,noexec,nosuid
    networks:
      - isolated-worker-net
    deploy:
      resources:
        limits:
          cpus: "2.00"
          memory: 2048M

  # ============================================================================
  # 5. DOCLING AI SIDECAR (IBM Docling + PyTorch + Pandoc DOCX/Markdown)
  # ============================================================================
  docling:
    image: registry.office-utility.vn/docling-sidecar:latest
    container_name: office-utility-docling
    restart: unless-stopped
    user: "1000:1000"
    read_only: true
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    environment:
      - PORT=5001
      - WORK_DIR=/tmp/jobs
      - TMPDIR=/tmp
      - DOCLING_SECRET_TOKEN=${DOCLING_SECRET_TOKEN}
      - HF_HUB_OFFLINE=1
      - TORCH_NUM_THREADS=4
      - LOG_LEVEL=WARNING
    volumes:
      - /tmp/jobs:/tmp/jobs
    tmpfs:
      - /tmp:size=512M,uid=1000,gid=1000,mode=1777
    networks:
      - isolated-worker-net
    deploy:
      resources:
        limits:
          cpus: "2.00"
          memory: 3072M
    healthcheck:
      test: ["CMD", "python3", "/app/scripts/healthcheck.py"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 45s

  # ============================================================================
  # 6. POSTGRESQL 18 (Relational + JSONB + Partial Indexing)
  # ============================================================================
  postgres:
    image: postgres:18-alpine
    container_name: office-utility-postgres
    restart: unless-stopped
    environment:
      - POSTGRES_USER=${DB_USER}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME}
    volumes:
      - pg_data:/var/lib/postgresql/data
    networks:
      - isolated-worker-net
    deploy:
      resources:
        limits:
          cpus: "2.00"
          memory: 2048M
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${DB_USER} -d ${DB_NAME}"]
      interval: 10s
      timeout: 5s
      retries: 5

  # ============================================================================
  # 6. REDIS 8.10 (BullMQ Broker, SSE Pub/Sub & Sliding Window Rate Limiter)
  # ============================================================================
  redis:
    image: redis:8.10
    container_name: office-utility-redis
    restart: unless-stopped
    command: >
      redis-server
      --requirepass ${REDIS_PASSWORD}
      --maxmemory 1024mb
      --maxmemory-policy noeviction
      --appendonly yes
    volumes:
      - redis_data:/data
    networks:
      - isolated-worker-net
    deploy:
      resources:
        limits:
          cpus: "1.00"
          memory: 1200M
    healthcheck:
      test: ["CMD", "redis-cli", "-a", "${REDIS_PASSWORD}", "ping"]
      interval: 10s
      timeout: 3s
      retries: 5

networks:
  frontend-egress-net:
    driver: bridge
  isolated-worker-net:
    driver: bridge
    internal: true # Chặn kết nối Internet trực tiếp từ cụm Database/Gotenberg (NFR-SEC-06)

volumes:
  pg_data:
  redis_data:

```

> **Lưu ý về Cờ Bảo mật `tmpfs` (`noexec,nosuid,nodev`):**
> Phân vùng `/tmp` trên RAM Disk được gắn cờ `noexec` ở cấp nhân hệ điều hành Linux. Ngay cả trong trường hợp xấu nhất khi một tệp thực thi nhị phân lọt vào thư mục `/tmp/jobs/{uuid}`, hệ điều hành sẽ từ chối cấp quyền thực thi (`Permission Denied`) cho bất kỳ tệp nào nằm trong phân vùng này.

---

## 7.3. Cơ chế Thực thi Cam kết "Zero-Trace" Đa tầng (3-Layer Zero-Trace Defense)

Để đảm bảo không một tệp nhạy cảm nào của người dùng tồn tại quá **30 phút** ngay cả khi một tiến trình gặp sự cố (`NFR-SEC-02`), hệ thống vận hành **3 lớp dọn dẹp độc lập**:

| Lớp Bảo vệ | Thời điểm Kích hoạt | Cơ chế Kỹ thuật Thực thi | Mục tiêu Đảm bảo |
| --- | --- | --- | --- |
| **Lớp 1: Xóa Tức thì theo Yêu cầu (`User-Triggered Purge`)** | Ngay khi người dùng nhấn nút *"Xóa ngay khỏi máy chủ"* trên UI. | Gọi `DELETE /api/v1/jobs/:jobId/purge` $\rightarrow$ Thực thi lệnh `DeleteObjectsCommand` xóa đồng thời tệp gốc và tệp kết quả trên R2/MinIO, cập nhật `status = 'DELETED'` và ghi dấu thời gian `deleted_at` trong vòng **< 1 giây**. | Trao quyền kiểm soát chủ động tuyệt đối cho chủ thể dữ liệu theo Nghị định 13/2023/NĐ-CP. |
| **Lớp 2: Tiến trình Quét Dọn Định kỳ (`BullMQ Delayed Job & Cron`)** | Đúng **30 phút** sau khi tệp được tạo (`expires_at <= NOW()`). | • Mỗi khi tạo bản ghi `file_artifacts`, hệ thống đẩy kèm 1 Delayed Job (`delay: 1800000ms`) vào BullMQ.<br>• Đồng thời, một Cron Worker chạy mỗi **2 phút/lần** truy vấn vào **Partial Index** `idx_artifacts_pending_cleanup` (tốc độ `< 1ms`) để gom xóa hàng loạt (`DeleteObjects` tối đa 1.000 keys/lần) các tệp quá hạn và ghi nhận `deleted_at`. | Đảm bảo đồng bộ trạng thái kiểm toán (Audit Log) trong PostgreSQL và xóa tệp đúng hạn 30 phút. |
| **Lớp 3: Chính sách Tự hủy Tầng Lưu trữ (`R2 / S3 Bucket Lifecycle Policy`)** | Cấp độ hạ tầng Cloudflare R2 / MinIO (Chốt chặn cuối cùng). | Thiết lập quy tắc `Object Lifecycle Rule` trực tiếp trên Bucket cho tiền tố `temp/`: Tự động xóa cứng mọi đối tượng ở tầng phần cứng lưu trữ sau khi hết hạn ngay cả khi toàn bộ máy chủ Backend bị sập. | Cam kết **100% Zero-Trace** không phụ thuộc vào trạng thái của ứng dụng. |

---

## 7.4. Chiến lược Tự động Mở rộng & Phục hồi Sự cố (Auto-Scaling & Circuit Breaker)

### 7.4.1. Quy tắc Mở rộng Ngang theo Độ sâu Hàng đợi (Queue-Depth Scaling Rules)

Đáp ứng yêu cầu `NFR-SCA-01` (lưu lượng tăng vọt vào giờ hành chính 8h30–11h30 và 13h30–17h00), bộ giám sát hàng đợi đọc chỉ số từ Redis BullMQ mỗi **15 giây** để điều phối số lượng bản sao (`replicas`) của Worker:

1. **Điều kiện Mở rộng (Scale-Out Trigger):**
* Khi tổng số Job đang ở trạng thái `waiting` (`Queue Depth`) của hàng đợi `fast-light` hoặc `office-convert` vượt ngưỡng **`> 50 Jobs`** trong 30 giây liên tiếp (`NFR-SCA-01`), hoặc thời gian chờ trung bình (`Estimated Wait Time`) vượt quá **5 giây** (`NFR-PERF-04`).

* Hệ thống tự động khởi chạy thêm các node `nestjs-worker` mới kết nối thẳng vào `REDIS_URL` và `R2_ENDPOINT` mà không cần cấu hình lại Load Balancer.

2. **Điều kiện Thu hẹp An toàn (Graceful Scale-In):**
* Sau 18h00 tối hoặc cuối tuần khi `Queue Depth == 0` trong 15 phút, hệ thống gửi tín hiệu `SIGTERM` cho các Worker dư thừa.

* Worker nhận tín hiệu `SIGTERM` sẽ ngừng nhận Job mới (`worker.close()`), chờ hoàn tất nốt Job đang xử lý dở (tối đa 60–180s), dọn sạch thư mục `/tmp/jobs/{uuid}` trên RAM Disk rồi mới tắt container.

### 7.4.2. Cơ chế Tự ngắt Bảo vệ & Suy giảm Mềm dẻo (Per-Tool Circuit Breaker)

Đáp ứng yêu cầu `NFR-REL-02`:

* Nếu một công cụ phụ thuộc vào tiến trình bên ngoài gặp sự cố (ví dụ: dịch vụ `Gotenberg` dùng cho công cụ `office-to-pdf` có tỷ lệ lỗi vượt quá **20%** trong 2 phút hoặc thời gian phản hồi vượt ngưỡng Timeout **60 giây**):

1. Hệ thống tự động chuyển cột `status` của riêng công cụ `office-to-pdf` trong bảng `tool_definitions` từ `'ACTIVE'` sang `'DEGRADED'` hoặc `'MAINTENANCE'`.

2. Giao diện người dùng ngay lập tức hiển thị nhãn cảnh báo bảo trì ngắn hạn cho công cụ đó (`HTTP 503 TOOL_UNDER_MAINTENANCE`), trong khi **toàn bộ hơn 59 công cụ còn lại trên hệ thống vẫn hoạt động bình thường 100%**.

---

## 7.5. Hệ thống Giám sát, Đo lường & Cảnh báo (Observability & Alerting Stack)

Để vận hành hệ thống tuân thủ nguyên tắc **Privacy-by-Design (Không lưu nhật ký chứa dữ liệu người dùng)** nhưng vẫn nắm bắt tức thì sức khỏe hạ tầng, hệ thống triển khai bộ công cụ **Prometheus + Grafana + Pino Structured Logger**.

### 7.5.1. Bảng Chỉ số Giám sát Cốt lõi (Key Prometheus Metrics & Alert Rules)

| Tên Metric (Prometheus) | Loại | Ý nghĩa Kỹ thuật | Ngưỡng Cảnh báo (Alerting Threshold) |
| --- | --- | --- | --- |
| `http_request_duration_ms` | `Histogram` | Độ trễ phản hồi của các endpoint API (`/presigned-upload`, `/dispatch`, `/secrets`). | Cảnh báo **Warning** khi `p95 > 80ms` hoặc **Critical** khi `p99 > 150ms` trong 5 phút (`NFR-PERF-03`). |
| `bullmq_queue_waiting_total` | `Gauge` | Số lượng Job đang xếp hàng chờ xử lý theo 5 phân lớp (`fast-light`, `office-convert`, `heavy-media`, `ai-ocr`, `ai-docling`). | Kích hoạt Auto-Scale hoặc báo động khi `waiting > 50` (`NFR-SCA-01`). |
| `bullmq_job_processing_duration_ms` | `Histogram` | Thời gian thực thi thực tế của Worker (`jobs.processing_time_ms`). | Cảnh báo khi `p95` của `fast-light > 2.000ms` hoặc `office-convert > 8.000ms` (`NFR-PERF-04`). |
| `docling_task_duration_seconds` | `Histogram` | Thời gian bóc tách tài liệu và tổng hợp Word/Markdown của vi dịch vụ Docling Sidecar. | Cảnh báo khi `p95 > 15.000ms` cho tài liệu dưới 20 trang. |
| `docling_active_tasks_count` | `Gauge` | Số lượng tác vụ mô hình AI đang chiếm giữ Semaphore trong Docling container (giới hạn 1). | Báo động khi xếp hàng chờ Docling vượt quá **10 tác vụ**. |
| `zero_trace_overdue_artifacts_count` | `Gauge` | Số lượng bản ghi trong `file_artifacts` có `status != 'DELETED'` nhưng đã quá hạn `expires_at` quá 5 phút. | **Cảnh báo Đỏ (Critical Alert)** nếu giá trị **`> 0`** (Đảm bảo cam kết xóa tệp 30 phút không bao giờ bị trễ). |
| `security_rejected_files_total` | `Counter` | Tổng số tệp bị chặn bởi bộ lọc **Magic Bytes (`415`)**, **Zip Bomb** hoặc **Pixel Flood**. | Cảnh báo tấn công rà quét nếu tăng đột biến **`> 30 lần/phút`** từ cùng một dải IP / Fingerprint. |
| `worker_tmpfs_usage_bytes` | `Gauge` | Mức dung lượng đang chiếm dụng trên ổ đĩa RAM Disk (`/tmp`) của từng Worker. | Cảnh báo khi đạt **`> 80%`** dung lượng `tmpfs` (phát hiện sớm trường hợp rò rỉ tệp tạm chưa xóa trong khối `finally`). |

### 7.5.2. Quy tắc Ghi Nhật ký Bảo mật Không Lưu Vết (Zero-PII Structured Logging)

Toàn bộ nhật ký hệ thống (Application Logs) được xuất theo định dạng JSON qua thư viện `Pino` và tuân thủ quy tắc **Zero-PII Redaction**:

* **Được phép ghi:** `jobId`, `toolSlug`, `queueName`, `status`, `processingTimeMs`, `sizeBytes`, `mimeType`, `errorCode`.

* **Cấm tuyệt đối ghi vào Log:** Nội dung văn bản trích xuất từ OCR, dữ liệu các ô bảng tính Excel, mật khẩu mở khóa PDF của người dùng, đường dẫn đầy đủ có chứa chữ ký `X-Amz-Signature` của Presigned URL, hoặc họ tên/số tài khoản ngân hàng trong công cụ tạo VietQR và tính lương.
