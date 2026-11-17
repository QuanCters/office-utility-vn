---
target: src/micro_apps/data_lab/layout.rs
total_score: 18
max_score: 40
na_heuristics: 
p0_count: 2
p1_count: 2
target_identity: "file:/home/quan/workspace/tauri-v2/office-utility-vn/src/micro_apps/data_lab/layout.rs"
target_fingerprint: "sha256:61f4a2f7d960210a8ca71b568151932d97c999a1d08726bc959bea59678c583b"
target_path: /home/quan/workspace/tauri-v2/office-utility-vn/src/micro_apps/data_lab/layout.rs
timestamp: 2026-11-06T16-03-45Z
slug: src-micro-apps-data-lab-layout-rs
---
Method: dual-agent (A: aaea8b01-3750-406f-8ec1-34a55f6c1653 · B: 8ccaa031-90e8-4a38-adcc-213db5a20ace)

# Báo Cáo Đánh Giá Toàn Diện Giao Diện Data Lab v2.0 (Design Critique Report)
**Đối tượng thẩm định:** `src/micro_apps/data_lab/layout.rs` (và toàn bộ hệ thống UI Data Lab v2.0)  
**Phương thức:** Đánh giá độc lập 2 Sub-agent (Assessment A: Design Review & Assessment B: Deterministic Browser Evidence)  
**Môi trường thử nghiệm:** Chrome DevTools / Playwright trên 5 tỷ lệ khung hình & độ phân giải máy tính thực tế

---

### Design Health Score (Thang Điểm 10 Tiêu Chuẩn Nielsen Heuristics)

| # | Tiêu Chuẩn Heuristic | Điểm (0-4) | Vấn Đề Then Chốt & Minh Chứng Đo Lường |
|---|----------------------|:----------:|-----------------------------------------|
| 1 | **Visibility of System Status** | **2/4** | Lọc dữ liệu qua quality bar hoạt động tức thì, nhưng thiếu Filter Chip/Tag xóa bộ lọc rõ ràng; status bar tĩnh luôn báo 50.000 dòng dù đang nạp bảng 25 dòng; chạy SQL trong Drawer chỉ đổi message text ở footer mà không nạp kết quả vào lưới. |
| 2 | **Match System & Real World** | **2/4** | Số tiền hiển thị dạng thô `22000000` thay vì phân cách hàng nghìn `22.000.000 đ` chuẩn kế toán VN; thẻ Top Facets bên Inspector hiển thị gán cứng Tỉnh/TP cho cả cột Lương và Ngày sinh. |
| 3 | **User Control and Freedom** | **1/4** | **P0 SHOWSTOPPER:** Khi ấn `Ctrl+B` (đóng Explorer), vùng làm việc trung tâm (`dl-main-stage`) bị co rúm thành một vạch đen rộng **4.0px** do thẻ `<span style="display: none;"></span>` làm lệch cấu trúc CSS Grid; thanh resizers hoàn toàn là UI stub không có handler chuột. |
| 4 | **Consistency and Standards** | **2/4** | Chuyển sang Mode 2 (Đối soát) hoặc Mode 3 (Chuyển mã), Right Inspector vẫn giữ nguyên danh sách thao tác bảng tính của Mode 1; tồn tại 2 cụm nút Hoàn tác/Làm lại độc lập không đồng bộ trạng thái. |
| 5 | **Error Prevention** | **2/4** | Thanh chất lượng 4 màu phân loại lỗi trực quan tốt; tuy nhiên thao tác xóa bước trong Applied Steps (✕) và nút Xóa trắng trong Text Codecs thực thi tức thì không có xác nhận. |
| 6 | **Recognition Rather Than Recall** | **2/4** | Header Trifacta có icon kiểu dữ liệu dễ nhận biết; tuy nhiên khi click lọc theo dải màu, người dùng không có nhãn hiển thị tiêu chí lọc cụ thể nếu không tự nhớ vị trí đã click. |
| 7 | **Flexibility and Efficiency of Use** | **2/4** | Có đăng ký phím tắt (`Ctrl+B`, `Ctrl+J`, `Ctrl+K`), nhưng phím tắt chính `Ctrl+B` làm sập layout và thanh vách ngăn không kéo thả được khiến Power User bị tước đoạt khả năng tối ưu không gian. |
| 8 | **Aesthetic & Minimalist Design** | **2/4** | Nhận diện màu sắc tối đạt chuẩn Sovereign Cockpit; tuy nhiên mật độ nén quá lớn; trên laptop 1366×768 khi mở Drawer, chiều cao bảng tính bị bóp nghẹt chỉ còn hiển thị **9 dòng dữ liệu** (ở 1024×768 chỉ còn **5 dòng**). |
| 9 | **Error Recovery** | **2/4** | Icon ⚠️ cảnh báo ô lỗi rất trực quan; nhưng nút Sửa Font TCVN3 chỉ thực thi hàm cắt khoảng trắng `clean_whitespace()`, đưa ra thông báo ảo mà không thực sự sửa mã byte font. |
| 10 | **Help and Documentation** | **1/4** | Thiếu hoàn toàn hướng dẫn ngữ cảnh (inline hints, tooltip giải thích thuật toán) cho các khái niệm nặng tính kỹ thuật như "Fuzzy Clustering", "DAG Pipeline", "Arrow In-Memory". |
| **Tổng** | **Health Score Toàn Diện** | **18 / 40** | **Xếp loại: YẾU (Poor - 45%)** — *Cần ưu tiên xử lý các lỗi sập layout và hoàn thiện tính năng trước khi phát hành.* |

---

### Design Specificity Verdict (Phán Quyết Tính Độc Bản Thiết Kế)

* **Đánh giá Chuyên gia Thiết kế (LLM Assessment):**
  Data Lab v2.0 sở hữu một DNA sản phẩm rất khác biệt và xuất sắc ở chiều sâu nghiệp vụ bản địa Việt Nam: sự kết hợp giữa bộ lọc Trifacta 3 tầng với các bài toán đặc thù như kiểm định CCCD Modulo 11, khôi phục font TCVN3/VNI, bóc tách địa giới hành chính 3 cấp và đặc biệt là module **Visual Reconcile** đối soát hóa đơn điện tử TT78 với Sổ cái nội bộ.
  Tuy nhiên, vỏ bọc "IDE 5 vùng" đang gặp sự đứt gãy giữa ý niệm thiết kế và hiện thực kỹ thuật: xuất hiện nhiều bẫy "Deceptive UI" (thanh kéo vách ngăn có con trỏ resize nhưng không kéo được; nút sửa font TCVN3 chỉ gọi hàm trim; dữ liệu phân bổ Top Facets bị fix cứng tĩnh).

* **Kiểm định Thực nghiệm & Đo lường Đoán định (Deterministic Evidence):**
  - **Quét CLI Detector:** Phát hiện **36 cảnh báo quy chuẩn thiết kế (Advisories)** trong `styles/data_lab.css` (25 mã màu RGBA cứng ngoài bảng token, 7 cỡ chữ lệch thang, 4 thông số border-radius lệch thang chuẩn).
  - **Lệch hình học 20px thường trực:** Trên 100% độ phân giải, khung làm việc `.data-lab-v2-workbench` đều bị tràn ngang 20px (`scrollWidth` = 1940px trên viewport 1920px; 1386px trên 1366px; 1460px trên 1440px) do kích thước con của Explorer (280px) và Inspector (320px) vượt kích thước cột CSS Grid (240px và 300px).
  - **Tràn vỡ thanh điều khiển Top Command Bar:** Tại độ phân giải 1024×768, Command Bar cần tối thiểu 1265px, gây tràn ngang **+241px**, làm toàn bộ cụm nút toggle (`⚡ Luồng/SQL`, `🛠️ Ctrl+J`, badge `100% On-Device`) bị đẩy văng ra ngoài khung hình.

---

### Overall Impression (Ấn Tượng Tổng Thể)

Data Lab v2.0 có một bước nhảy vọt ngoạn mục về tầm nhìn kiến trúc so với v1.0 — bố cục 5 vùng mang lại dáng dấp của một Data Workbench đẳng cấp quốc tế. Tuy nhiên, bản dựng hiện tại giống như một bản concept tương tác cao cấp (High-fidelity Mockup) hơn là một phần mềm sản xuất hoàn chỉnh: các tương tác cốt lõi (co giãn panel, ẩn Explorer, chạy SQL, chuyển đổi ngữ cảnh Inspector theo Mode) đều đang bị nghẽn hoặc vỡ layout.

---

### What's Working (Điểm Mạnh Nổi Bật)

1. **Ý tưởng Trifacta 3-Tier Column Header đột phá:** Cấu trúc 3 tầng (Kiểu dữ liệu ➔ Thanh chất lượng 4 màu ➔ Sparkline/Tần suất cụm) là tính năng vượt trội, trực quan hóa sức khỏe dữ liệu ngay lập tức.
2. **Module Visual Reconcile giải quyết đúng điểm đau kế toán VN:** Giao diện 3-Pane nối trường khóa và phân loại 4 tab chênh lệch tiền cực kỳ sát sườn với nghiệp vụ đối soát hóa đơn điện tử và sao kê ngân hàng.
3. **Thẩm mỹ Sovereign Cockpit chuyên nghiệp:** Gam màu nền Slate tối, viền 1px tinh tế, font đơn cách hiển thị mã số và dữ liệu tạo cảm giác công cụ phân tích dữ liệu tin cậy.

---

### Priority Issues (Danh Mục Vấn Đề Ưu Tiên P0 - P3)

#### [P0] Vỡ nát Layout khi Đóng Explorer (`Ctrl+B`) — Sập xuống 4.0px
- **Vấn đề:** Bấm `Ctrl+B` làm vùng lưới trung tâm `.dl-main-stage` biến mất, chỉ còn một khe đen 4.0px.
- **Tác hại:** Làm tê liệt hoàn toàn chức năng xem bảng khi người dùng muốn mở rộng không gian.
- **Giải pháp:** Trong `layout.rs#L175`, không dùng `<span>display: none</span>`. Giữ nguyên thẻ `<DataLabExplorer>` và gắn class `.is-collapsed` với `width: 0; min-width: 0; overflow: hidden;` kết hợp cập nhật CSS Grid `grid-template-columns: 0px 0px 1fr 4px var(--right-w)`.
- **Lệnh đề xuất:** `$impeccable layout`

#### [P0] Tràn ngang Command Bar (+241px) ở Viewport ≤ 1200px
- **Vấn đề:** Trên màn hình 1024×768 hoặc 1366×768 (khi bật Windows DPI 125%), thanh lệnh tràn ngang 241px, nuốt chửng các nút toggle Luồng/SQL và Inspector.
- **Tác hại:** Người dùng màn hình nhỏ bị cô lập, không thể bật/tắt các công cụ.
- **Giải pháp:** Bổ sung media query responsive: co ngắn input `Ctrl+K` từ 380px xuống 180px; thu gọn Smart Clipboard Pill thành icon badge; ẩn chữ trên các nút toggle chỉ giữ lại biểu tượng.
- **Lệnh đề xuất:** `$impeccable adapt`

#### [P1] Inspector Bên Phải Mù Ngữ Cảnh & Dữ Liệu Phân Bổ Bị Gán Cứng
- **Vấn đề:** Chọn bất kỳ cột nào (Lương, Ngày sinh) thì Inspector vẫn hiển thị "Hà Nội 42%, TP.HCM 35%". Chuyển sang Mode Đối soát hay Chuyển mã, Inspector vẫn giữ nguyên công cụ bảng tính.
- **Tác hại:** Gây mất niềm tin nghiêm trọng vào tính xác thực của dữ liệu phân tích.
- **Giải pháp:** Truyền `top_values` động tính từ dữ liệu thực tế vào `DataLabInspector`. Điều kiện hóa nội dung Inspector theo `current_mode`.
- **Lệnh đề xuất:** `$impeccable harden`

#### [P1] Bóp Nghẹt Chiều Cao Bảng Tính ở Màn Hình Phổ Thông 1366×768
- **Vấn đề:** Khi mở Drawer (cao 156px - 229px) trên màn hình 768px chiều cao, bảng tính chỉ còn hiển thị được 9 dòng (ở 1024x768 chỉ còn 5 dòng).
- **Tác hại:** Hủy hoại trải nghiệm duyệt dữ liệu của hơn 60% kế toán viên sử dụng laptop 14 inch.
- **Giải pháp:** Hạ chiều cao mặc định của Drawer xuống 120px - 130px; tự động cuộn gọn khi người dùng tương tác với bảng tính; cho phép kéo chỉnh chiều cao Drawer.
- **Lệnh đề xuất:** `$impeccable adapt`

#### [P2] Thanh Phân Vùng Kéo Giãn Bị Liệt (Dead Resizers)
- **Vấn đề:** `.dl-resizer-left` và `.dl-resizer-right` có con trỏ `col-resize` nhưng không có mã bắt sự kiện chuột.
- **Tác hại:** Vi phạm nguyên lý phản hồi tương tác (False Affordance).
- **Giải pháp:** Gắn sự kiện `pointerdown` / `pointermove` để cập nhật biến CSS `--left-w` và `--right-w` theo tọa độ chuột.
- **Lệnh đề xuất:** `$impeccable polish`

#### [P2] Định Dạng Số Tiền Kế Toán Chưa Có Phân Cách Hàng Nghìn
- **Vấn đề:** Lương hiển thị trần trụi `22000000` thay vì `22.000.000 đ`.
- **Tác hại:** Bắt kế toán viên phải căng mắt đếm từng chữ số 0, dễ gây sai lệch tài chính.
- **Giải pháp:** Áp dụng định dạng số tiền chuẩn VN với dấu chấm phân cách và font `tabular-nums`.
- **Lệnh đề xuất:** `$impeccable clarify`

---

### Persona Red Flags (Báo Động Theo Chân Dung Người Dùng)

* **Alex (Power User / Kỹ sư dữ liệu):**
  - Nhấn `Ctrl+B` để mở rộng bảng làm việc ➔ Màn hình biến mất thành một vệt đen 4px.
  - Cố gắng kéo thanh chia vách ngăn để nới rộng lưới dữ liệu ➔ Chuột không phản hồi.
  - Mở `SQL Scratchpad`, bấm Thực thi ➔ Bảng dữ liệu không cập nhật kết quả.

* **Jordan (First-Timer / Nhân viên văn phòng mới):**
  - Choáng ngợp trước 5 vùng chia cắt đầy thuật ngữ trừu tượng ("Trifacta", "DAG Pipeline", "Arrow In-Memory").
  - Vô tình click vào vệt màu đỏ trên đầu cột ➔ Bảng từ 25 dòng tụt xuống còn 1 dòng duy nhất mà không thấy nút Bỏ lọc ở đâu, hoảng sợ tưởng phần mềm làm hỏng file Excel.

* **Chị Hương (Kế toán trưởng / Doanh nghiệp SME VN):**
  - Nhìn vào cột lương phải ghé sát mắt đếm số 0 vì thiếu dấu chấm phân cách hàng nghìn (`18500000`, `25000000`).
  - Mở ứng dụng trên laptop Dell 14 inch (1366×768), bật Drawer ➔ Bảng tính chỉ còn nhìn thấy đúng 9 dòng, không thể thao tác đối chiếu.

---

### Minor Observations (Quan Sát Nhỏ)

1. **Trùng lặp nút Hoàn tác/Làm lại:** Toolbar của Super-Grid có một cặp nút (disabled), trong khi dưới Status Bar lại có một cặp nút khác bấm vào chỉ in thông báo text. Cần thống nhất về một nguồn trạng thái duy nhất.
2. **Lỗi copy/paste thuộc tính title:** Thanh phân cách bên phải (`.dl-resizer-right`) có thuộc tính `title` ghi nhầm là *"Kéo để điều chỉnh độ rộng Explorer"* thay vì *"Inspector"*.
3. **Cảnh báo Reactive tracking trong Console:** Lỗi gọi `columns.with(...)` ngoài tracking context tại `spreadsheet_grid_view.rs:603:34` cần chuyển sang `columns.with_untracked(...)`.

---

### Questions to Consider (Câu Hỏi Gợi Mở)

1. *Liệu người dùng có thực sự cần nhìn thấy cả 5 vùng làm việc cùng một lúc, hay chúng ta nên tự động ẩn Explorer và Drawer khi bắt đầu thao tác làm sạch bảng tính (Focus Mode)?*
2. *Thay vì hiển thị thanh Drawer SQL cố định chiếm 156px chiều cao bên dưới, tại sao chúng ta không biến nó thành một chế độ xem toàn màn hình (Full SQL IDE Mode) hoặc một pop-up drawer thông minh?*
3. *Làm thế nào để các nút thao tác nhanh bên Inspector (Sửa font TCVN3, Đọc số thành chữ, Gom cụm mờ) lập tức liên kết trực tiếp với các động cơ Rust backend đã có trong `crates/office-engine` thay vì chỉ hiển thị thông báo mô phỏng?*
