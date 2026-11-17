# Đặc Tả Kiến Trúc Tái Thiết (Rebuild) Micro-App Data Lab v2.0

Bản đặc tả **Đặc Tả Kiến Trúc Tái Thiết (Rebuild) Micro-App Data Lab v2.0** đã được khởi tạo và lưu trực tiếp vào Google Docs trên Google Drive của bạn. Dưới đây là toàn bộ nội dung kiến trúc lõi, danh mục công cụ mã nguồn mở cần tích hợp và đặc tả 6 động cơ thuần Rust cần viết lại để mang lại giá trị thực chiến cao nhất.

---

## 1. Tầm Nhìn Tái Thiết: Tại Sao Cần "Đập Đi Xây Lại"?

### 1.1. Điểm nghẽn của kiến trúc Data Lab v1.0

Mặc dù phiên bản v1.0 đã có giao diện 3 vùng rõ ràng và vượt qua 94 unit tests, kiến trúc bên dưới vẫn vận hành theo mô hình **tập hợp các tiện ích rời rạc (Isolated Utilities)**:

* **Phân mảnh bộ nhớ & Đọc/Ghi lặp lại:** Mỗi công cụ (`excel_cleaner_normalizer`, `excel_visual_joiner`, `excel_split_by_column`) tự gọi `calamine` đọc tệp thành các cấu trúc `Vec` theo dòng riêng lẻ. Khi chuyển tab hoặc chạy chuỗi DAG Workflow, dữ liệu phải cấp phát lại hoặc đọc lại từ đĩa.
* **Nghẽn cổ chai tại Tauri IPC (JSON Serialization):** Việc truyền bảng dữ liệu từ Rust Backend lên Leptos CSR Frontend qua `serde_json` truyền thống khiến các bảng từ 50.000 dòng trở lên tiêu tốn RAM gấp 3–4 lần dung lượng gốc và gây giật khung hình.
* **Thiếu Động cơ Truy vấn Trung tâm:** Người dùng không thể gõ câu lệnh SQL hoặc biểu thức lọc liên kết trực tiếp giữa nhiều tệp/sheet đang mở trong `DataLabFileListPanel`.
* **Chưa khai thác lợi thế Super App:** Data Lab và PDF Studio (PDF Craft) đang tách biệt, bỏ lỡ quy trình nghiệp vụ cốt lõi: **Bóc bảng từ PDF → Làm sạch & Đối soát tại Data Lab → Trộn mẫu & Xuất hàng loạt PDF ký số**.

### 1.2. Bốn nguyên lý kiến trúc của Data Lab v2.0

1. **Columnar In-Memory First (Chuẩn hóa bộ nhớ dạng cột với Apache Arrow):** Mọi tệp nạp vào `DataLabFileListPanel` chỉ parse một lần duy nhất thành `RecordBatch` của Apache Arrow lưu trong bộ nhớ chia sẻ của Rust. Toàn bộ công cụ truy cập vùng nhớ này theo cơ chế **Zero-Copy**.
2. **Hybrid OLAP Compute (Kết hợp Polars Lazy & DuckDB In-Process):** Sử dụng `polars-lazy` làm lõi biến đổi dữ liệu cho đồ thị DAG và `duckdb` làm động cơ truy vấn SQL liên bảng trực tiếp trong tiến trình.
3. **Zero-Serialization IPC (Truyền tải nhị phân thô qua Tauri v2):** Dùng `tauri::ipc::Response` truyền thẳng byte buffer định dạng **Arrow IPC Stream** sang WebAssembly để render lưới dữ liệu tốc độ 60 FPS.
4. **Vietnamese Back-Office Native:** Tập trung nguồn lực viết bằng Rust các thuật toán đặc thù tại Việt Nam mà phần mềm quốc tế bỏ ngỏ (Hóa đơn điện tử XML chuẩn Tổng cục Thuế, Gom cụm mờ ngữ âm tiếng Việt, Chuẩn hóa địa giới hành chính 3 cấp, Đọc số tiền thành chữ và Kiểm định mã định danh CCCD/MST).

---

## 2. Kiến Trúc Kỹ Thuật Cốt Lõi Cần Sử Dụng (Target Architecture v2.0)

```text
+-----------------------------------------------------------------------------------+
| TẦNG 4: GIAO DIỆN LEPTOS 0.8 CSR (WASM FRONTEND)                                  |
|  • Arrow Virtual Grid (60 FPS, 1M+ rows)   • SQL & Expression Scratchpad Bar      |
|  • Reactive DAG Canvas (với Diff Badges)   • Smart Clipboard Detector Banner      |
+-----------------------------------------+-----------------------------------------+
                                          |
                 Tauri v2 Raw Binary IPC  |  Arrow IPC Stream (Zero-JSON)
                 (tauri::ipc::Response)   |  + Channel<TaskProgress> (200ms)
                                          v
+-----------------------------------------------------------------------------------+
| TẦNG 3: ĐIỀU PHỐI TÁC VỤ & DAG ENGINE (RUST ORCHESTRATION LAYER)                  |
|  • Recipe & DAG Compiler (Biên dịch JSON Recipe sang chuỗi Polars Lazy / Arrow)   |
|  • Cooperative Cancellation (CancellationToken < 50ms) + Undo/Redo Time-Travel    |
+-----------------------------------------+-----------------------------------------+
                                          |
                              Zero-Copy   |  Arc<Vec<RecordBatch>>
                                          v
+-----------------------------------------------------------------------------------+
| TẦNG 2: ĐỘNG CƠ TÍNH TOÁN & CÔNG CỤ CHUYÊN BIỆT (COMPUTE & NATIVE RUST TOOLS)     |
|  [A. Core Compute Engine]                  [B. Custom Native Rust Domain Tools]   |
|  • Polars LazyFrame (Transform/Join/Agg)   • Vietnamese Fuzzy & Phonetic Engine   |
|  • DuckDB Bundled (Cross-file SQL Query)   • E-Invoice XML Parser & Reconciler    |
|  • Arrow Compute Kernels (SIMD Vectorized) • Font/Mojibake Auto-Doctor & Mailmerge|
+-----------------------------------------+-----------------------------------------+
                                          |
+-----------------------------------------v-----------------------------------------+
| TẦNG 1: TẦNG NẠP/XUẤT ĐA ĐỊNH DẠNG & LIÊN THÔNG SUPER APP (I/O & BRIDGE LAYER)    |
|  • Readers: calamine, arrow-csv, parquet, quick-xml, infer (Magic Bytes)          |
|  • Writers: rust_xlsxwriter (Parallel), arrow-ipc, docx-xml-stream                |
|  • Super App Bridge: PDF Studio Table Extractor <-> Data Lab <-> PDF Batch Signer |
+-----------------------------------------------------------------------------------+
```

### Chi tiết vận hành của 3 cơ chế trọng tâm:

* **Kho dữ liệu cột dùng chung (`SharedWorkspaceState`):**
Nhờ cấu trúc bất biến (immutable) của mảng cột Apache Arrow `ArrayRef`, khi một công cụ chuẩn hóa 1 cột trên bảng 500.000 dòng, chỉ duy nhất cột đó được cấp phát `ArrayRef` mới; 29 cột còn lại giữ nguyên con trỏ `Arc` cũ. Nhờ vậy, tính năng **Time-Travel Undo/Redo** trong `recipe_history.rs` chuyển đổi trạng thái tức thì (0ms) mà không nhân bản bộ nhớ.
* **Giao thức `Viewport Window Streaming` qua Tauri v2 Raw IPC:**
Leptos 0.8 Frontend chỉ yêu cầu khung nhìn hiện tại (ví dụ: từ dòng 1.500, lấy 80 dòng). Rust Backend cắt lát `RecordBatch::slice(1500, 80)` trong 0.1ms, đóng gói vào `arrow_ipc::writer::StreamWriter` và trả thẳng mảng byte qua `tauri::ipc::Response`. WASM Frontend đọc trực tiếp buffer Arrow mà không tốn chi phí `JSON.parse()`.
* **Đường ống liên thông `PDF Studio ↔ Data Lab`:**
Bảng biểu trích xuất từ PDF Studio được đẩy thẳng vào `SharedWorkspaceState` của Data Lab dưới dạng `RecordBatch` để tự động xóa dòng tiêu đề lặp và chuẩn hóa số liệu; chiều ngược lại cho phép đẩy kết quả trộn mẫu hợp đồng từ Data Lab sang PDF Studio để đóng dấu và ký số hàng loạt.

---

## 3. Bản Đồ Công Cụ Trên Thị Trường: Những Gì Nên "Mang Về" Dev Tiếp?

Thay vì tự viết lại các phép toán cơ bản, bạn nên kết hợp trực tiếp các crate mã nguồn mở mạnh nhất trong hệ sinh thái Rust và học hỏi mô hình từ các sản phẩm đầu ngành:

| Công cụ / Dự án | Phân loại | Thứ nên "Mang về" Data Lab v2.0 | Cách thức tích hợp vào `crates/data_lab` |
| --- | --- | --- | --- |
| **Polars (`polars-lazy`)** | Rust Crate (MIT / Apache-2.0) | Động cơ xử lý DataFrame đa luồng, tối ưu hóa truy vấn (Query Optimizer), Join, GroupBy, Pivot, Unpivot. | Nhúng làm lõi thực thi cho đồ thị `VisualCanvasWorkflowView` (DAG) và các phép biến đổi bảng lớn. |
| **DuckDB (`duckdb` bundled)** | Rust/C++ Crate (MIT) | Động cơ OLAP SQL in-process, hỗ trợ truy vấn SQL trực tiếp ngang qua nhiều bảng trong RAM hoặc file trên đĩa. | Tích hợp vào thanh **SQL Scratchpad** trên `SpreadsheetGridFacetView` để người dùng chạy SQL nối nhiều file Excel/CSV. |
| **`leptos-arrow-grid`** | Rust WASM Crate (MIT) | Component lưới dữ liệu thiết kế cho Leptos 0.8 và Apache Arrow `RecordBatch`, hỗ trợ virtual scroll, lọc cột và chọn vùng kiểu Excel. | Mang mã nguồn về làm nền tảng cho `SpreadsheetGridFacetView`, mở rộng thêm thanh Mini-Profiling Bar và Facet Drawer. |
| **`qsv` (Quicksilver)** | Rust Open Source CLI (MIT) | Thuật toán suy luận JSON Schema (`schema`), kiểm định dữ liệu (`validate`), thống kê 73 chỉ số (`moarstats`) và nhận diện 19 chuẩn ngày tháng (`datefmt`). | Chuyển ngữ (port) logic từ các module `schema.rs`, `stats.rs`, `datefmt.rs` của `qsv` sang chạy trực tiếp trên mảng cột Arrow. |
| **OpenRefine** | Java Open Source (BSD-3) | Mô hình tương tác Facet tần suất, cấu trúc **Recipe History JSON** và quy trình Gom cụm mờ (Clustering Modal). | Kế thừa toàn bộ luồng UX Facet/Clustering; viết lại thuật toán gom cụm sang Rust và bổ sung ngữ âm tiếng Việt. |
| **DevToys** | C# Open Source (MIT) | Cơ chế **Smart Detection** tự động phân tích nội dung Clipboard hoặc file kéo thả để gợi ý đúng công cụ. | Viết module `smart_detector.rs` gắn ở Header: tự động phát hiện chuỗi JSON, SQL, Base64, XML Hóa đơn hoặc văn bản lỗi font trong Clipboard. |
| **CyberChef** | JS Open Source (Apache-2.0) | Tư duy đường ống biến đổi dữ liệu tức thì (Recipe Pipeline) với khả năng bật/tắt (breakpoint) từng bước để soi dữ liệu trung gian. | Áp dụng vào `SplitPaneConverterView` và `recipe_history.rs`, cho phép bấm vào từng bước lịch sử để xem trạng thái bảng tại bước đó. |
| **FINOS Perspective** | C++/WASM Open Source (Apache-2.0) | Khả năng tạo bảng chéo (Cross-tab / Pivot Table) và gom nhóm tức thì trên giao diện lưới. | Bổ sung thanh cấu hình nhanh `Group By / Pivot / Aggregate` ngay phía trên lưới dữ liệu. |

---

## 4. Sáu Công Cụ Cần Viết Lại Thuần Rust (Native Rust) Mang Lại Giá Trị Cao Nhất

Để tạo ra sự khác biệt rõ rệt so với Excel hay các công cụ quốc tế, bạn nên thiết kế mới trait `ColumnarTransformTool` (nhận/trả `Arc` của `RecordBatch`) và viết lại 6 công cụ trọng tâm sau bằng Native Rust:

### 4.1. Động cơ Bóc tách & Đối soát Hóa đơn Điện tử XML (`vn_einvoice_auditor.rs`)

* **Giá trị thực tế:** Kế toán và kiểm toán viên tại Việt Nam hàng tháng phải kiểm tra hàng trăm đến hàng nghìn tệp `.xml` hóa đơn điện tử (chuẩn Thông tư 78/2021/TT-BTC) và đối chiếu thủ công với bảng kê Excel.
* **Thiết kế bằng Rust:**
  * Sử dụng `quick-xml` kết hợp `rayon` đọc song song hàng nghìn tệp `.xml` (hoặc giải nén trực tiếp từ `.zip` trong RAM) chỉ trong **dưới 200ms**.
  * Bóc tách đầy đủ các trường chuẩn TT78 (`KHHDon`, `SHDon`, `NLap`, `NBan/MST`, `NMua/MST`, danh sách hàng hóa `HHDVu`, `TgTCThue`, `TgTThue`, `TgTTTBSo`, chữ ký số) thành 2 bảng `RecordBatch`: **Bảng Tổng hợp Hóa đơn** và **Bảng Chi tiết Hàng hóa**.
  * Thực thi **Đối soát 3 chiều (Three-Way Reconciliation)** bằng `hashbrown` với bảng kê Excel nội bộ để tự động tách ra: (1) Hóa đơn có XML nhưng thiếu trên Excel, (2) Hóa đơn có trên Excel nhưng thiếu XML gốc, (3) Hóa đơn khớp số nhưng lệch tiền thuế/tổng tiền.

### 4.2. Động cơ Gom cụm mờ Tiếng Việt & Chuẩn hóa Địa giới Hành chính (`vn_fuzzy_clustering.rs`)

* **Giá trị thực tế:** Dữ liệu khách hàng, nhân sự và danh mục vật tư tại Việt Nam thường xuyên bị trùng lặp do lỗi chính tả vùng miền, gõ thiếu dấu, viết tắt hoặc thay đổi tên phường/xã sau sáp nhập.
* **Thiết kế bằng Rust:**
  * **Vietnamese Key Fingerprint:** Chuẩn hóa Unicode NFC → Chuyển chữ thường → Bóc dấu → Mở rộng từ viết tắt (`cty` → `cong ty`, `tnhh`, `mtv`) → Sắp xếp âm tiết (token sort).
  * **Vietnamese Phonetic Fingerprint (Mã hóa Ngữ âm Tiếng Việt):** Ánh xạ các phụ âm đầu/cuối và thanh điệu dễ nhầm lẫn theo vùng miền về cùng một mã đại diện (`ch/tr` → `C`, `s/x` → `S`, `d/gi/r` → `Z`, `c/k/q` → `K`, `n/ng` → `N`, `t/c` → `T`, gộp thanh `hỏi/ngã`). Nhờ đó tự động phát hiện và gom nhóm các biến thể như *"Nguyễn Thị Trúc"* và *"Nguyễn Thị Chúc"* hay *"Sửa chữa"* và *"Xữa chửa"*.
  * **Chuẩn hóa Địa giới Hành chính 3 Cấp:** Dùng bộ máy `aho-corasick` nạp từ điển Tỉnh/Thành – Quận/Huyện – Phường/Xã (kèm bảng ánh xạ các đơn vị hành chính cũ sang mới sau sáp nhập) để tách một cột địa chỉ tự do thành 4 cột chuẩn hóa.

### 4.3. Bác sĩ Bảng mã & Khôi phục Lỗi Chữ Tự động (`vn_font_encoding_doc.rs`)

* **Giá trị thực tế:** Hợp nhất `vietnamese_font_converter.rs` và `encoding_mojibake_fixer.rs` thành một động cơ tự động hoàn toàn. Thực tế một bảng Excel tổng hợp từ nhiều nguồn có thể vừa chứa ô mã TCVN3, vừa chứa ô VNI-Windows, vừa chứa ô lỗi UTF-8 Mojibake.
* **Thiết kế bằng Rust:**
  * Xây dựng **Bộ chấm điểm Âm tiết Tiếng Việt (`Vietnamese Syllable Scorer`)** dựa trên bảng băm tần suất các vần tiếng Việt hợp lệ.
  * Tự động quét từng ô (Cell-level Auto-Detection): thử giải mã qua các bảng mã (TCVN3, VNI, Vietware, CP1258, Windows-1252 Mojibake) và chọn phương án có điểm âm tiết tiếng Việt cao nhất mà không cần người dùng đoán bảng mã nguồn.
  * Khi sửa trực tiếp trên tệp `.xlsx` và `.docx`, tự động ánh xạ lại cả khai báo font trong `styles.xml` (ví dụ chuyển `.VnTime` hoặc `VNI-Times` về `Times New Roman`) để mở tệp đọc được ngay.

### 4.4. Cổng Kiểm định Chất lượng Dữ liệu & Định danh (`schema_quality_gate.rs`)

* **Giá trị thực tế:** Đóng vai trò "Chốt chặn kiểm định" trước khi doanh nghiệp import bảng lương, danh mục khách hàng hoặc bút toán vào ERP/phần mềm kế toán.
* **Thiết kế bằng Rust:**
  * **Kiểm định cấu trúc & Checksum định danh Việt Nam:** Kiểm tra mã tỉnh (3 số đầu), mã giới tính/thế kỷ và năm sinh trong CCCD 12 số; kiểm tra chữ số kiểm tra (check digit thứ 10) của Mã số thuế doanh nghiệp/cá nhân; kiểm tra thuật toán Luhn cho số thẻ ngân hàng.
  * **Chuẩn hóa 19 định dạng ngày tháng (Port từ `qsv datefmt`):** Tự động xử lý các cột ngày tháng bị trộn lẫn giữa chuẩn `MM/DD/YYYY`, `DD/MM/YYYY`, số serial Excel và chuỗi văn bản tiếng Việt.
  * **Cảnh báo dị biệt tài chính:** Tự động gắn cờ các dòng nghi ngờ thanh toán đúp (trùng người nhận + số tiền + ngày gần nhau) hoặc số tiền vượt ngưỡng phân phối chuẩn (Outlier IQR/Z-Score).

### 4.5. Động cơ Đối soát Công nợ & Sao kê Đa khóa (`visual_reconciler.rs`)

* **Giá trị thực tế:** Nâng cấp `excel_visual_joiner.rs` từ phép Join đơn thuần thành công cụ đối soát chuyên nghiệp cho kế toán và vận hành TMĐT.
* **Thiết kế bằng Rust:**
  * **Substring / Pattern Key Join:** Dùng `aho-corasick` và Regex để tìm Mã đơn hàng / Mã khách hàng nằm lẫn bên trong chuỗi "Nội dung chuyển khoản" dài của sao kê ngân hàng và nối với bảng đơn hàng nội bộ.
  * **Tolerance Matching (Đối soát có dung sai):** Cho phép khớp các giao dịch có độ lệch số tiền trong ngưỡng cấu hình (ví dụ lệch dưới 1.000đ do làm tròn phí) hoặc lệch ngày giao dịch từ 1 đến 2 ngày làm việc.
  * **Báo cáo 4 Sheet tự động:** Dùng `rust_xlsxwriter` xuất ngay tệp đối soát gồm 4 Sheet: `1_Khop_Hoan_Toan`, `2_Khop_Lech_So_Tien` (tô màu cột chênh lệch), `3_Chi_Co_O_Sao_Ke`, `4_Chi_Co_O_So_Noi_Bo`.

### 4.6. Động cơ Xuất bản Hợp đồng Đa luồng & Đọc số thành chữ (`docx_batch_publisher.rs`)

* **Giá trị thực tế:** Nâng cấp `docx_template_mailmerge.rs` thành dây chuyền tạo hợp đồng, báo giá, quyết định nhân sự tốc độ cao liên thông với PDF Studio.
* **Thiết kế bằng Rust:**
  * **Hợp nhất thẻ XML Run thế hệ 2 + Bảng lặp động (1-N):** Ngoài việc nối các thẻ `w:r` bị Word cắt vụn trong biến đơn, hỗ trợ cú pháp lặp dòng trong bảng Word để trộn được cả phụ lục hợp đồng hoặc hóa đơn có danh sách hàng hóa thay đổi theo từng khách hàng.
  * **Bộ lọc Đọc số tiền thành chữ tích hợp sẵn:** Hỗ trợ viết trực tiếp `{{so_tien | vnd_words}}` hoặc `{{so_tien | usd_words}}` ngay trong mẫu Word, tự động chuyển số thành câu chữ chuẩn quy định kế toán Việt Nam.
  * **Liên thông PDF Studio:** Đẩy thẳng luồng đầu ra sang PDF Studio để xuất hàng loạt tệp PDF đặt tên động theo cột dữ liệu.

---

## 5. Cấu Trúc Thư Mục Mới & Lộ Trình Chuyển Đổi

Khi tái cấu trúc `crates/data_lab`, chia rõ thành 3 phân lớp (`engine`, `io`, `tools`) và triển khai cuốn chiếu qua 4 giai đoạn để giữ vững hệ thống kiểm thử hiện có:

```text
crates/data_lab/src/
├── lib.rs
├── engine/                      # LÕI XỬ LÝ DỮ LIỆU THỐNG NHẤT
│   ├── arrow_store.rs           # SharedDataset (Arc<RecordBatch>) & Zero-Copy Undo/Redo
│   ├── ipc_streamer.rs          # Đóng gói Arrow IPC Stream cho Tauri v2 Raw Response
│   ├── sql_scratchpad.rs        # Tích hợp DuckDB / Polars SQL truy vấn chéo giữa các file
│   ├── dag_executor.rs          # Thực thi đồ thị VisualCanvasWorkflowView & tính Data Diff Badge
│   └── smart_detector.rs        # Nhận diện thông minh Clipboard & Magic Bytes
├── io/                          # TẦNG ĐỌC/GHI TỐC ĐỘ CAO & CẦU NỐI SUPER APP
│   ├── spreadsheet_reader.rs    # Đọc Excel/CSV/Parquet -> Arrow RecordBatch
│   ├── spreadsheet_writer.rs    # Ghi song song Arrow -> XLSX (rust_xlsxwriter + rayon)
│   └── pdf_studio_bridge.rs     # Cầu nối nhận bảng từ PDF Studio & đẩy tài liệu sang PDF Studio
└── tools/                       # 6+ ĐỘNG CƠ NGHIỆP VỤ THUẦN RUST
    ├── traits.rs                # ColumnarTransformTool & StreamingBatchTool
    ├── vn_einvoice_auditor.rs   # Bóc tách hàng loạt XML Hóa đơn TT78 & Đối soát bảng kê
    ├── vn_fuzzy_clustering.rs   # Gom cụm mờ tiếng Việt (Phonetic + N-Gram) & Chuẩn hóa địa chỉ
    ├── vn_font_encoding_doc.rs  # Tự động phát hiện & sửa lỗi Font TCVN3/VNI + Mojibake UTF-8
    ├── schema_quality_gate.rs   # Kiểm định CCCD/MST/Luhn, chuẩn hóa 19 định dạng ngày, Outlier
    ├── visual_reconciler.rs     # Đối soát công nợ/sao kê ngân hàng (Fuzzy ID + Dung sai số tiền)
    └── docx_batch_publisher.rs  # Mailmerge DOCX hỗ trợ bảng lặp 1-N, đọc số thành chữ & xuất PDF
```

1. **Giai đoạn 1 (Tuần 1–2):** Dựng lõi `arrow_store.rs`, `ipc_streamer.rs` và tích hợp `leptos-arrow-grid` vào `SpreadsheetGridFacetView` để đạt 60 FPS với 500.000+ dòng.
2. **Giai đoạn 2 (Tuần 3–4):** Mở thanh `sql_scratchpad.rs` trên lưới và viết mới bộ đôi công cụ tiếng Việt (`vn_font_encoding_doc.rs` + `vn_fuzzy_clustering.rs`).
3. **Giai đoạn 3 (Tuần 5–6):** Xây dựng bộ công cụ nghiệp vụ sát thủ (`vn_einvoice_auditor.rs`, `visual_reconciler.rs`, `docx_batch_publisher.rs`) và cầu nối `pdf_studio_bridge.rs`.
4. **Giai đoạn 4 (Tuần 7–8):** Nâng cấp `dag_executor.rs` hiển thị **Data Diff Badge** trên từng mắt xích, bổ sung `smart_detector.rs` trên Header và mở rộng bộ test lên 180+ kịch bản (kèm benchmark 1 triệu dòng).
