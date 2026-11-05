# Thiết Kế Kỹ Thuật: Tích Hợp Động Cơ & Giao Diện Đồ Họa PDFCraft Vào PDF Studio (Office Utility VN)

**Mã tài liệu:** `SPEC-2026-11-05-PDFCRAFT-INTEGRATION`  
**Dự án:** Office Utility VN (`tauri-v2`)  
**Phân hệ áp dụng:** Micro-App `PDF Studio` & Toàn bộ kiến trúc Workspace  
**Quy mô:** Dự án lớn (Project)  
**Trạng thái:** Đã qua Builder Check — Sẵn sàng phê duyệt (Approved Candidate)

---

## 1. Bối cảnh & Mục đích (Intent & The Why)

Hiện tại, phân hệ **PDF Studio** trong Office Utility VN sở hữu 22 công cụ xử lý tệp PDF nhưng sử dụng động cơ tự viết (`MiniPdf`) thao tác thô trên các byte block. Cách tiếp cận này bộc lộ những hạn chế kỹ thuật:
* Không đầy đủ chuẩn ISO 32000, dễ lỗi khi gặp các tệp PDF phức tạp, font nhúng đặc thù, bảng mã CMap dị biệt hoặc mã hóa nâng cao.
* Thiếu một Viewport/Canvas tương tác đồ họa thời gian thực (real-time rendering) để người dùng có thể xem trang, phóng to thu nhỏ mượt mà, chọn vùng và biên tập trực quan.

**PDFCraft** (`https://github.com/storytold/pdfcraft.git`) là một dự án mã nguồn mở thuần Rust xuất sắc, được thiết kế theo tiêu chuẩn công nghiệp:
* Tuân thủ chuẩn ISO 32000, mô hình dữ liệu Copy-on-Write (CoW), garbage collection đối tượng PDF thừa.
* Giao diện đồ họa người dùng (UI) xây dựng trên nền tảng **`egui`**, hỗ trợ tăng tốc phần cứng, các công cụ ký số PKI chuyên nghiệp (X.509, CMS/DER, Keychain/WinStore), so sánh tài liệu, che dữ liệu (two-phase redaction), quản lý form và OCR bằng Rust.
* Người dùng đặc biệt đánh giá cao giao diện biên tập trực quan và chuyên nghiệp của PDFCraft.

**Mục đích:** Fork toàn bộ mã nguồn PDFCraft vào dự án hiện tại, **thay thế hoàn toàn Hub cũ** của PDF Studio bằng không gian làm việc đồ họa của PDFCraft, đồng thời **bảo tồn và tích hợp toàn bộ các tính năng đặc thù độc bản của dự án hiện tại** (tiếng Việt, xuất Excel, in ấn chuyên dụng, liên kết dữ liệu với các micro-app khác) vào một giao diện đồng nhất chuẩn Acrobat-grade.

---

## 2. Mục tiêu, Giới hạn & Chống mục tiêu (Goals, Non-Goals, Anti-Goals)

### Mục tiêu (Goals)
1. **Fork toàn bộ mã nguồn PDFCraft** vào dự án (cấu trúc workspace đồng bộ) để toàn quyền tùy biến dài hạn.
2. **Thay thế hoàn toàn Hub 22 thẻ cũ của PDF Studio:** Khi người dùng mở PDF Studio, ứng dụng lập tức hiển thị không gian làm việc đồ họa trực quan (Visual Studio Cockpit) của PDFCraft.
3. **Tích hợp giao diện Canvas của PDFCraft vào cùng 1 cửa sổ Tauri duy nhất** (thông qua WebAssembly Canvas + iframe bridge), giữ thanh điều hướng vỏ ngoài (Header) của Leptos để chuyển đổi qua lại giữa các micro-app.
4. **Bảo tồn và kết nối các tính năng độc bản của dự án hiện tại**:
   * *Vietnamese OCR:* Nhận dạng ký tự tiếng Việt có dấu chuyên sâu (VietOCR).
   * *Table Extractor → Excel:* Nhận diện bảng biểu tọa độ và xuất bảng tính `.xlsx`.
   * *PDF → Long Image:* Ghép cuộn toàn bộ các trang PDF thành ảnh dài duy nhất.
   * *Booklet Imposition:* Xếp trang in sách đóng kim (saddle-stitch, N-up) chuẩn khổ in.
   * *Markdown → Styled PDF:* Kết xuất văn bản quy chuẩn hành chính Việt Nam khổ A4.
   * *Grayscale Print:* Chuyển đổi hệ màu DeviceGray tối ưu mực in văn phòng.
   * *Repair Corrupt PDF:* Quét tuyến tính byte stream cứu bảng xref bị hỏng.
   * *Hệ sinh thái Micro-App:* Cho phép nạp và xuất file qua lại liền mạch giữa PDF Studio và VN Office / Data Lab / Media Studio.
5. **Đảm bảo tính riêng tư & Offline tuyệt đối (Zero-Cloud):** 100% xử lý on-device, loại bỏ telemetry và các module tự động check update qua internet (`craft-update`).

### Giới hạn (Non-Goals)
* Không duy trì song song giao diện Hub 22 thẻ cũ (Hub đã được thay thế triệt để).
* Không viết lại 3 micro-app còn lại (VN Office, Media Studio, Data Lab) sang `egui`. Ba phân hệ này vẫn chạy trên nền Leptos 0.8 / HTML5 / CSS vì tính tối ưu cho việc nhập liệu, bảng biểu và văn bản tiếng Việt.
* Không can thiệp làm vỡ cấu trúc crate lõi của PDFCraft để giữ tính tương thích cao nếu sau này cần cập nhật thêm từ upstream.

### Chống mục tiêu (Anti-Goals)
* Tuyệt đối không mở thêm một cửa sổ hệ điều hành rời rạc (cửa sổ con độc lập), gây đứt gãy trải nghiệm người dùng.
* Tuyệt đối không để ứng dụng thực hiện bất kỳ kết nối mạng nào ra ngoài Internet.
* Không phá vỡ trải nghiệm điều hướng chung của Office Utility VN (người dùng luôn có nút thoát về Launcher và chuyển đổi micro-app).

---

## 3. Các hướng tiếp cận & Đánh đổi (Approaches Considered)

| Tiêu chí | Hướng 1: Nhúng `pdfcraft-web` Canvas vào Tauri qua WebAssembly / iframe Bridge **[ĐÃ CHỌN]** | Hướng 2: Native Multi-window (Tauri Launcher + PDFCraft Desktop wgpu) | Hướng 3: Port toàn bộ `crates/ui-egui` sang Leptos / HTML5 |
| :--- | :--- | :--- | :--- |
| **Trải nghiệm người dùng** | **Tối ưu:** 1 cửa sổ duy nhất, chuyển tab mượt mà, đồng nhất với các micro-app khác. | **Rời rạc:** Bấm PDF Studio sẽ nảy ra 1 cửa sổ mới riêng biệt. | **Đồng nhất:** 100% giao diện là web/HTML. |
| **Hiệu năng hiển thị** | Rất cao (WebGL Canvas 60 FPS, đáp ứng hoàn hảo tài liệu văn phòng). | Tối đa (wgpu gọi thẳng driver GPU). | Kém nhất khi render các trang PDF nặng qua DOM/HTML. |
| **Mức độ khả thi & Tiến độ** | **Rất nhanh & an toàn:** PDFCraft đã có sẵn target `pdfcraft-web` và `host::Bridge` qua `postMessage`. | Trung bình: Cần viết thêm IPC socket đồng bộ trạng thái giữa 2 tiến trình. | Cực kỳ tốn kém: Phải viết lại hàng chục ngàn dòng code UI phức tạp. |
| **Bảo trì lâu dài** | Dễ dàng nâng cấp hoặc cherry-pick tính năng mới từ PDFCraft. | Tương đối dễ nhưng phức tạp việc quản lý tiến trình nền. | Cực kỳ khó, phải tự bảo trì toàn bộ UI tự viết lại. |

**Quyết định:** Chọn **Hướng 1**.

---

## 4. Kiến trúc kỹ thuật & Luồng dữ liệu (Technical Architecture & Data Flow)

### 4.1. Cấu trúc thư mục Workspace
Mã nguồn PDFCraft được đặt tại `vendor/pdfcraft/` và duy trì pipeline build độc lập cho target WASM:
```text
office-utility-vn/
├── Cargo.toml                  # Workspace root
├── src-tauri/                  # Backend điều phối Tauri v2
├── src/                        # Frontend Leptos (VN Office, Data Lab, Media Studio)
│   ├── components/layout/      # Header điều hướng chung (vẫn hiển thị để back về Launcher)
│   └── micro_apps/
│       └── pdf_studio/         # View PDF Studio chứa khung nhúng iframe PDFCraft + Action Ribbon VN
├── crates/
│   ├── office-shared/          # Cấu trúc dữ liệu dùng chung
│   └── office-engine/          # Giữ lại các engine độc bản (Vietnamese OCR, Table Extractor, Long Image...)
└── vendor/pdfcraft/            # Fork nguyên vẹn của PDFCraft
    ├── apps/
    │   └── pdfcraft-web/       # Target WebAssembly biên dịch ra canvas bundle (dist/pdfcraft)
    └── crates/
        ├── ui-egui/            # Toàn bộ giao diện egui của PDFCraft
        ├── cos/, render/, fonts/, forms/, sign/, export/... (các engine lõi)
```

### 4.2. Khung nhìn & Điều hướng (Navigation Shell)
* **Vỏ ngoài Leptos:** Thanh Header trên cùng của ứng dụng được giữ nguyên:
  * Nút `← Về Launcher` để quay lại danh mục 4 Micro-Apps.
  * Nút chuyển đổi nhanh giữa các micro-app (`VN Office`, `Data Lab`, `Media Studio`).
  * Thanh tiêu đề tài liệu hiện hành.
* **Khu vực làm việc PDF Studio:** Chiếm toàn bộ không gian bên dưới Header:
  * Nhúng `<iframe>` trỏ vào `/pdfcraft/index.html?host=parent`.
  * Thanh điều khiển tiện ích Việt Nam (**VN Extension Ribbon**) được bố trí trang nhã ở phía trên hoặc tích hợp dưới dạng nút bấm tiện ích.

### 4.3. Cầu nối giao tiếp hai chiều (`host::Bridge`) & Luồng dữ liệu
1. **Nạp tài liệu:**
   * Khi người dùng kéo thả file hoặc chọn mở file từ Launcher / VN Office: Leptos gọi Tauri command đọc byte từ ổ đĩa và gửi sang iframe:
     ```javascript
     iframe.contentWindow.postMessage({ type: 'pdfcraft:open', name: fileName, bytes: arrayBuffer }, origin);
     ```
   * PDFCraft Canvas lập tức render tài liệu ở 60 FPS.
2. **Kích hoạt công cụ độc bản (Ví dụ: Trích xuất bảng sang Excel hoặc OCR Tiếng Việt):**
   * Người dùng nhấn vào nút chức năng trên thanh tiện ích Việt Nam.
   * Leptos phát lệnh yêu cầu xuất dữ liệu: `pdfcraft:request_bytes`.
   * PDFCraft trả về mảng byte mới nhất của tài liệu.
   * Leptos chuyển tiếp buffer này xuống backend Rust `office-engine` qua lệnh Tauri IPC.
   * Động cơ backend thực thi tác vụ (chạy Rayon threadpool).
   * **Kết quả:**
     * Nếu là file phụ trợ (`.xlsx`, `.png`): Xuất hiện hộp thoại lưu file hoặc xem trước.
     * Nếu là file PDF đã biến đổi (như in sách Booklet, Grayscale, Sửa file hỏng): Tự động nạp ngược trở lại Canvas thông qua `pdfcraft:open` để người dùng kiểm tra ngay lập tức.
3. **Lưu tài liệu:**
   * Khi người dùng nhấn Save/Save As trong PDFCraft, sự kiện `pdfcraft:save` được gửi lên Leptos/Tauri để ghi file an toàn xuống đĩa cứng.

---

## 5. Các quyết định kỹ thuật do Builder xác lập (Builder Calls)

Để tối ưu hóa tiến độ và đảm bảo an toàn kiến trúc, các quyết định sau đã được thống nhất:
1. **Thay thế hoàn toàn Hub cũ:** Không duy trì song song lưới 22 thẻ cũ. Deep links từ Launcher trỏ vào các tác vụ PDF cơ bản sẽ mở thẳng PDFCraft; deep links trỏ vào các tác vụ độc bản (như OCR, Trích bảng) sẽ mở PDFCraft kèm theo kích hoạt sẵn công cụ tương ứng.
2. **Bảo tồn Navigation Shell:** Luôn hiển thị thanh Header của Leptos phía trên iframe để người dùng không bị "mắc kẹt" trong PDFCraft và có thể quay về Launcher bất cứ lúc nào.
3. **Cô lập build pipeline của PDFCraft:** Build `pdfcraft-web` bằng `trunk` độc lập thành thư mục tĩnh `dist/pdfcraft` đặt trong assets của Tauri, tránh xung đột cấu hình Cargo giữa hai dự án.
4. **Vô hiệu hóa mạng (Zero-Cloud):** Tắt crate `craft-update` trong PDFCraft để đảm bảo tuyệt đối không có request kiểm tra phiên bản ra ngoài internet.
5. **Giữ nguyên engine độc bản giai đoạn đầu:** Các tool độc bản (`vietnamese_ocr`, `pdf_table_extractor`, `pdf_to_long_image`, `pdf_booklet_imposition`) được giữ nguyên logic xử lý ổn định đã có kiểm thử, sau đó sẽ dần tái cấu trúc sang dùng `pdfcraft-cos` ở các bản cập nhật sau.

---

## 6. Lộ trình triển khai (Implementation Phases)

* **Phase 1: Fork & Thiết lập Workspace**
  * Đặt mã nguồn PDFCraft vào `vendor/pdfcraft/`.
  * Cấu hình Trunk build target `pdfcraft-web` ra thư mục `dist/pdfcraft`.
  * Cấu hình Tauri nạp tài nguyên tĩnh từ `dist/pdfcraft`.
* **Phase 2: Xây dựng Cầu nối Iframe & Thay thế Hub**
  * Tái cấu trúc component `PdfStudioApp` trong Leptos: Thay Hub cũ bằng khung nhúng iframe kết nối `host::Bridge`.
  * Đấu nối sự kiện nạp file, lưu file, thông báo dirty/unsaved.
* **Phase 3: Tích hợp Thanh Tiện Ích Độc Bản (Vietnamese Action Ribbon)**
  * Thiết kế thanh công cụ tiện ích mở rộng cho các tính năng: OCR tiếng Việt, Xuất Excel, Ghép ảnh dài, In sách Booklet, In Grayscale, Sửa file hỏng.
  * Đấu nối luồng byte transfer giữa iframe và `office-engine`.
* **Phase 4: Kiểm thử E2E & Tối ưu hóa kích thước**
  * Cập nhật các kịch bản kiểm thử E2E.
  * Chạy `wasm-opt -Oz` tối ưu dung lượng tệp binary WASM.
