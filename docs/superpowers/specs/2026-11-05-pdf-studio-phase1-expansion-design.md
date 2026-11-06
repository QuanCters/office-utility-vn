# Thiết Kế Kỹ Thuật: Mở Rộng PDF Studio Phase 1 — Trải Nghiệm Trực Quan & Tiện Ích Đồ Họa

**Mã tài liệu:** `SPEC-2026-11-05-PDF-PHASE1`  
**Dự án:** Office Utility VN (`tauri-v2`)  
**Phân hệ áp dụng:** Micro-App `PDF Studio`  
**Trạng thái:** Chờ phê duyệt (Pending Review)

---

## 1. Bối cảnh & Mục đích (Intent & The Why)

PDF Studio hiện có 22 công cụ xử lý mạnh mẽ, an toàn tuyệt đối on-device (Zero-Cloud). Tuy nhiên, so sánh với các công cụ hàng đầu thị trường (iLovePDF, PDF24, Smallpdf, Adobe Acrobat), điểm nghẽn lớn nhất của người dùng hiện nay là:
1. **Thiếu trải nghiệm sắp xếp trang trực quan:** Khi cần đổi thứ tự trang hoặc bỏ bớt trang trong hợp đồng/báo cáo, người dùng phải nhớ số trang và gõ chuỗi văn bản như `"1, 3, 5-8"`, rất dễ nhầm lẫn.
2. **Nhu cầu ký tay nhanh trên chứng từ:** Nhân viên văn phòng, kế toán thường nhận được văn bản PDF cần ký nháy, ký tên xác nhận nhưng không muốn in ra giấy để ký rồi scan lại. Hiện tại người dùng chỉ có thể dán watermark chữ hoặc nạp file ảnh con dấu có sẵn.
3. **Nhu cầu chia sẻ từng trang PDF dưới dạng bộ ảnh:** Người dùng cần trích xuất toàn bộ các trang tài liệu PDF thành từng file ảnh rời (`trang_1.png`, `trang_2.png`...) đóng gói trong 1 tệp `.zip` duy nhất để gửi qua Zalo, Messenger, email hoặc nhúng vào slide thuyết trình.

Kế hoạch Phase 1 tập trung giải quyết triệt để 3 nhu cầu cấp thiết này bằng 3 công cụ Pod độc lập, chạy 100% on-device, thuần Rust, không cần kết nối mạng.

---

## 2. Mục tiêu, Giới hạn & Chống mục tiêu (Goals, Non-Goals, Anti-Goals)

### Mục tiêu (Goals)
* Bổ sung 3 công cụ Pod chuyên biệt mới vào PDF Studio:
  1. `pdf-page-organizer`: Lưới sắp xếp trang trực quan, kéo thả đổi vị trí, xoay góc (90°/180°/270°) từng trang, xóa trang thừa.
  2. `pdf-signature-pad`: Bảng vẽ chữ ký tay điện tử HTML5 Canvas (chọn màu mực xanh/đen/đỏ, nét bút), xuất PNG trong suốt và dán vào đúng trang/tọa độ tài liệu.
  3. `pdf-to-images-pack`: Kết xuất toàn bộ các trang PDF thành bộ ảnh độc lập (PNG/JPEG, 150/300 DPI) và đóng gói vào tệp `.zip` hoàn chỉnh.
* Tuân thủ kiến trúc phân tầng Micro-App: Động cơ xử lý tại `crates/office-engine/src/doc_suite/`, giao diện Leptos 0.8 tại `src/tools/` và `crates/office-ui/src/tools/`, tích hợp tự động qua `ToolRegistry`.
* 100% On-Device, zero-cloud (`NFR-SEC-01`), không sử dụng unwrap/expect trong mã nghiệp vụ (`NFR-SEC-03`).
* 100% tương thích ngược cấu trúc dữ liệu (`NFR-REL-01`) với `#[serde(default)]`.
* Kiểm định chất lượng từng pod bằng `./scripts/verify_pod.sh <slug>` đạt exit code 0.

### Giới hạn (Non-Goals)
* Không xây dựng trình soạn thảo văn bản trực tiếp trong PDF (In-place WYSIWYG text editing) vì đây là bài toán của Phase 3.
* Chưa hỗ trợ ký số phần cứng USB Token PKCS#11 (dành cho Phase 3).
* Không phụ thuộc vào các công cụ dòng lệnh bên ngoài (như ImageMagick, Ghostscript, Poppler CLI) để đảm bảo tính độc lập và phân phối portable.

### Chống mục tiêu (Anti-Goals)
* Tuyệt đối không gửi dữ liệu ra ngoài Internet (Cấm `std::net`, `reqwest`, `hyper`...).
* Không làm đơ giao diện người dùng khi xử lý tệp PDF nhiều trang (phải có thanh tiến trình và cho phép hủy tác vụ cooperative cancellation).
* Không tạo ra file nén zip hoặc file PDF bị lỗi cấu trúc font hay sai lệch tỷ lệ hiển thị.

---

## 3. Các hướng tiếp cận & Đánh đổi (Approaches Considered)

| Tiêu chí | Hướng 1: Nhồi thêm vào các Pod cũ (`pdf-merge-split`, `pdf-watermark-stamp`, `pdf-to-long-image`) | Hướng 2: Xây dựng 3 Pod độc lập chuyên biệt **[ĐÃ CHỌN]** |
| :--- | :--- | :--- |
| **Độ rõ ràng về UI** | Thấp (nhiều tab con lồng nhau làm rối giao diện) | Rất cao (thẻ riêng trên Hub Grid, 1-click mở đúng tính năng) |
| **Tách biệt trách nhiệm** | Kém (1 file engine gánh 5-6 nhiệm vụ) | Chuẩn mực kiến trúc Micro-App của Office Utility VN |
| **Khả năng song song hóa subagents** | Kém (dễ merge conflict vì sửa chung file) | Hoàn hảo (3 subagent chạy độc lập trên 3 file pod) |
| **Bảo trì & Kiểm định** | Khó viết unit tests cô lập | Dễ dàng chạy `./scripts/verify_pod.sh <slug>` riêng lẻ |

**Quyết định:** Chọn **Hướng tiếp cận 2 (3 Pod độc lập)** theo đúng thống nhất với người dùng.

---

## 4. Chi tiết Kỹ thuật 3 Pod Mới

### 4.1. Pod 1: `pdf-page-organizer` (Sắp xếp & Đảo trang Trực quan)

* **Slug:** `pdf-page-organizer`
* **Tên hiển thị:** "Sắp xếp & Đảo trang PDF Trực quan"
* **Phân nhóm:** `PdfCategory::MergeSplit` (Nhóm Ghép/Tách)
* **Mô hình hiển thị Smart Hybrid (Kết hợp Thẻ Tượng trưng & Thumbnail Tiến trình):**
  - **Mặc định (Interactive Symbolic Cards):**
    - Hiển thị tức thì lưới thẻ trang với hiệu năng 60 FPS (không cần chờ render nặng).
    - Mỗi thẻ thể hiện rõ: Số thứ tự trang, huy hiệu góc xoay hiện tại ($0^\circ, 90^\circ, 180^\circ, 270^\circ$), biểu tượng hướng khổ giấy (Dọc Portrait `📄` / Ngang Landscape `📑` tính toán từ tỷ lệ MediaBox đọc từ backend).
    - Thanh điều khiển trực tiếp trên thẻ: Nút xoay trái 90° (`↺`), xoay phải 90° (`↻`), di chuyển trước/sau (`←`, `→`), xóa trang (`🗑️`).
  - **Cơ chế Thumbnail On-Demand & Background Progressive Load:**
    - Nút bật/tắt chế độ xem trước "👁️ Chế độ Preview": Khi bật hoặc khi người dùng hover/click vào trang, hệ thống kích hoạt hiển thị ảnh xem trước thu nhỏ.
    - Luồng nền (Rust background task) tự động render thumbnail chất lượng thấp (low-res rasterization) và nạp dần lên UI khi rảnh rỗi (idle), cam kết không bao giờ làm gián đoạn (block) thao tác kéo thả và sắp xếp chính của người dùng.
  - **Thanh thao tác hàng loạt:** "Đảo ngược thứ tự", "Xoay tất cả 90°", "Khôi phục gốc", "Xóa các trang chẵn/lẻ".
* **Tùy chọn cấu hình (`PdfPageOrganizerOptions`):**
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PageAction {
      pub source_page: usize, // 1-based index
      #[serde(default)]
      pub rotation_deg: i32,  // 0, 90, 180, 270
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PdfPageOrganizerOptions {
      /// Danh sách các trang được giữ lại theo thứ tự mới kèm góc xoay
      pub pages: Vec<PageAction>,
  }
  ```
* **Động cơ xử lý (`PdfPageOrganizerTool`):**
  - Đọc file PDF gốc, phân tích cây `/Pages` và danh sách `/Kids`.
  - Cung cấp hàm đọc nhanh thông tin trang `get_pdf_page_info(path)` trả về `Vec<PageInfo>` (số trang, tỷ lệ MediaBox, góc xoay gốc).
  - Tái tạo cây `/Pages` mới chỉ chứa các trang có trong `pages` theo đúng thứ tự mong muốn.
  - Cập nhật từ điển của từng trang: bổ sung hoặc cộng dồn giá trị `/Rotate` tương ứng.
  - Cập nhật trường `/Count` tổng số trang mới và tái tạo bảng Xref chuẩn xác.

---

### 4.2. Pod 2: `pdf-signature-pad` (Ký tay Điện tử & Dán Chữ ký PDF)

* **Slug:** `pdf-signature-pad`
* **Tên hiển thị:** "Ký tay Điện tử & Chèn Chữ ký PDF"
* **Phân nhóm:** `PdfCategory::SecurityOptimize`
* **Giao diện người dùng (`PdfSignaturePadView`):**
  - Khung vẽ tương tác HTML5 `<canvas>` với viền nét đứt sang trọng, hỗ trợ vẽ bằng chuột, touchpad hoặc bút cảm ứng.
  - Bảng chọn màu mực chuẩn công sở: Xanh dương đậm (`#003399` - màu mực bút ký phổ biến), Đen (`#111111`), Đỏ (`#CC0000`).
  - Thanh chọn nét bút: Mảnh (1.5px), Vừa (2.5px), Đậm (4.0px).
  - Nút "Xóa chữ ký vẽ lại" (`↺ Xóa làm lại`).
  - Tùy chọn vị trí chèn:
    - Trang áp dụng: Trang cuối (mặc định), Trang đầu, Tất cả các trang, hoặc Trang tùy chọn.
    - Vị trí preset: Góc dưới phải (chuẩn ký nháy/ký chính), Góc dưới trái, Giữa trang, hoặc nhập tọa độ tùy chỉnh (X, Y).
    - Kích thước chữ ký: Nhỏ (120x60 pt), Vừa (160x80 pt), Lớn (200x100 pt).
* **Tùy chọn cấu hình (`PdfSignaturePadOptions`):**
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  #[serde(rename_all = "snake_case")]
  pub enum SignaturePlacement {
      BottomRight,
      BottomLeft,
      Center,
      TopRight,
      Custom { x_pt: f64, y_pt: f64 },
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PdfSignaturePadOptions {
      /// Ảnh chữ ký định dạng PNG base64 có nền trong suốt
      pub signature_png_base64: String,
      /// Trang cần chèn (1-based, 0 = tất cả các trang, -1 = trang cuối)
      #[serde(default = "default_target_page")]
      pub target_page: i32,
      /// Tùy chọn vị trí đặt chữ ký (tự động tính tọa độ theo MediaBox thực tế của trang)
      #[serde(default)]
      pub placement: SignaturePlacement,
      /// Chiều rộng hiển thị (points)
      #[serde(default = "default_sig_width")]
      pub width_pt: f64,
      /// Chiều cao hiển thị (points)
      #[serde(default = "default_sig_height")]
      pub height_pt: f64,
  }
  ```
* **Động cơ xử lý (`PdfSignaturePadTool`):**
  - Đọc `MediaBox`/`CropBox` của trang mục tiêu trong PDF gốc để tính toán tọa độ (X, Y) chính xác theo preset (cách lề 36 pt), tự động thích ứng với cả khổ A4, US Letter hoặc khổ ngang Landscape.
  - Giải mã chuỗi base64 thành dữ liệu ảnh PNG (RGB + Alpha mask).
  - Nhúng ảnh vào PDF dưới dạng XObject `/Subtype /Image` kèm `/SMask` cho độ trong suốt.
  - Chèn lệnh đồ họa vào luồng `/Contents` của trang mục tiêu:
    `q <width> 0 0 <height> <x> <y> cm /SigImage Do Q`.

---

### 4.3. Pod 3: `pdf-to-images-pack` (Xuất Toàn bộ Trang PDF sang Gói Ảnh ZIP)

* **Slug:** `pdf-to-images-pack`
* **Tên hiển thị:** "Xuất Toàn bộ Trang PDF sang Gói Ảnh ZIP"
* **Phân nhóm:** `PdfCategory::ConvertOcr`
* **Giao diện người dùng (`PdfToImagesPackView`):**
  - Tùy chọn định dạng ảnh: PNG (sắc nét, không suy hao), JPEG (nhẹ, tối ưu dung lượng).
  - Tùy chọn độ phân giải: 150 DPI (tiêu chuẩn màn hình / email), 300 DPI (chuẩn in ấn chất lượng cao).
  - Thanh trượt chất lượng nén JPEG (60% - 100%, mặc định 85%).
  - Tiền tố tên file trong ZIP: ví dụ `trang_` -> `trang_001.png`, `trang_002.png`.
* **Tùy chọn cấu hình (`PdfToImagesPackOptions`):**
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PdfToImagesPackOptions {
      #[serde(default = "default_image_format")]
      pub format: String, // "png" | "jpeg"
      #[serde(default = "default_dpi")]
      pub dpi: u32,       // 150 | 300
      #[serde(default = "default_quality")]
      pub quality: u8,    // 1..=100
      #[serde(default)]
      pub prefix: Option<String>,
  }
  ```
* **Động cơ xử lý (`PdfToImagesPackTool`):**
  - Kế thừa và mở rộng cơ chế rasterizer thuần Rust từ `pdf_to_long_image.rs` (kết xuất trang bitmap sạch sẽ và trích xuất các luồng ảnh nhúng chất lượng cao), giữ nguyên tiêu chuẩn không phụ thuộc công cụ CLI ngoài.
  - Mã hóa từng trang thành buffer PNG hoặc JPEG tương ứng bằng thư viện `image`.
  - Sử dụng module nén ZIP thuần Rust (kết hợp `flate2` + `crc32fast` đã có sẵn trong dự án) để tạo tệp `.zip` đạt chuẩn PKZIP.
  - Ghi tệp kết quả ra đĩa và trả về đường dẫn file `.zip` hoàn tất.

---

## 5. Ràng buộc Kỹ thuật & Tiêu chuẩn An toàn

1. **`NFR-SEC-01` (Zero Network)**: Không được thêm bất kỳ crate mạng nào (`reqwest`, `hyper`, `tokio::net`).
2. **`NFR-SEC-03` (No Panics)**: Cấm tuyệt đối `.unwrap()` và `.expect()` trong mã nghiệp vụ `office-engine`. Xử lý lỗi trả về `Result<DispatchTaskResponse, CommandError>`.
3. **`NFR-REL-01` (Backward Compatibility)**: Mọi trường struct thêm mới đều phải có `#[serde(default)]`.
4. **Hỗ trợ Hủy Tác vụ (Cooperative Cancellation)**: Kiểm tra `is_cancelled.load(Ordering::Relaxed)` trong các vòng lặp xử lý từng trang để hủy kịp thời $<100\text{ ms}$.

---

## 6. Kế hoạch Kiểm thử & Xác nhận

Mỗi pod mới phải có tối thiểu 5 unit tests theo đúng chuẩn văn hóa kiểm thử của dự án:
1. `test_<slug>_happy_path`: Kiểm tra luồng xử lý thành công với dữ liệu hợp lệ.
2. `test_<slug>_invalid_magic_bytes`: Kiểm tra từ chối file không phải PDF hợp lệ (`PDF-ERR-001`).
3. `test_<slug>_cooperative_cancellation`: Kiểm tra dừng tác vụ ngay lập tức khi cờ hủy được bật.
4. `test_<slug>_security_guardrails`: Kiểm tra bảo vệ chống tấn công tràn dữ liệu / input độc hại.
5. `test_<slug>_edge_case`: Kiểm tra trường hợp biên (PDF 1 trang, danh sách rỗng, chữ ký trống).

Đạt 100% kiểm định với `./scripts/verify_pod.sh <slug>` trước khi tích hợp vào nhánh chính.
