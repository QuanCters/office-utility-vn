# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Nhân viên văn phòng, kế toán, nhân sự, hành chính, và quản lý doanh nghiệp (đặc biệt là khối SME và doanh nghiệp có yêu cầu bảo mật cao tại Việt Nam) thường xuyên phải xử lý các tệp dữ liệu nhạy cảm: bảng lương nhân viên, hợp đồng lao động, bản scan căn cước công dân (CCCD), file hóa đơn điện tử XML, sao kê ngân hàng và báo cáo tài chính nội bộ.

## Product Purpose

Cung cấp bộ công cụ tiện ích văn phòng toàn diện, đa năng (>60 công cụ chuyên sâu) chạy hoàn toàn trên máy tính cá nhân (100% On-Device). Giúp tự động hóa các thao tác lặp đi lặp lại trên văn bản, bảng tính, tệp PDF, hình ảnh chứng từ và dữ liệu hành chính một cách tức thì mà không cần cài đặt phần mềm cồng kềnh hay chuyển dữ liệu lên đám mây.

## Positioning

"100% On-Device Native — An toàn Tuyệt đối, Không Đám mây, Chuẩn hóa Nghiệp vụ Việt Nam". 
Khác biệt hoàn toàn với các dịch vụ trực tuyến chuyển đổi file (Smallpdf, iLovePDF...) hay các công cụ văn phòng dùng chung của nước ngoài: Office Utility VN cam kết Zero-Cloud (không có bất kỳ byte dữ liệu nào rời khỏi máy người dùng, hoạt động air-gapped hoàn hảo) và được tối ưu hóa sâu cho các định dạng và quy định pháp lý hành chính đặc thù của Việt Nam (biểu thuế TNCN, BHXH, VietQR Napas247, đọc số tiền kế toán bằng chữ, chuẩn hóa địa danh hành chính 2 cấp/3 cấp, đối soát XML hóa đơn VAT của Tổng cục Thuế).

## Operating Context

- Môi trường máy tính văn phòng đa nền tảng (Windows 10/11, macOS, Linux).
- Thường xuyên làm việc song song với bộ phần mềm văn phòng (Microsoft Office Excel/Word, LibreOffice), phần mềm kế toán (MISA, FAST), trình duyệt web khai thuế/BHXH điện tử và các ứng dụng ngân hàng số.
- Thao tác kéo-thả tệp liên tục từ File Explorer/Finder, sao chép/dán dữ liệu từ Clipboard (`Ctrl + V`), sử dụng phím tắt (`Ctrl + Enter`, `Ctrl + K`, `Ctrl + P`).

## Capabilities and Constraints

- **Năng lực cốt lõi**:
  - Hơn 60 công cụ phân theo 5 phân hệ: Document Suite (PDF/DOCX), Media Studio (Image/Video/Audio), Data Lab (Excel/CSV/Text), VN Office (Nghiệp vụ hành chính VN), System Hub (Macro Workflow DAG).
  - Khung giao diện 3 vùng chuẩn hóa: Vùng 1 (Tiêu đề, Badge bảo mật, Cấu hình mẫu), Vùng 2 (Kéo thả tệp & Tham số), Vùng 3 (Thanh tiến trình & Nút thực thi).
  - Micro-App 4: Data Lab (Bảng tính & Dữ liệu) vận hành trên kiến trúc bất biến Spine Lock: Zone 1 (Header, DataLabToolbar 2 tầng, DataLabFileListPanel cố định 290px), Zone 2 (Context Banner & 4 chế độ: SpreadsheetGridFacetView, SplitPaneConverterView, VisualCanvasWorkflowView, Batch Settings Form), Zone 3 (TaskExecutor luồng nền).
  - Tự động hóa chuỗi quy trình (Macro Workflow DAG Pipeline).
- **Ràng buộc bất biến**:
  - `NFR-SEC-01`: Cấm tuyệt đối kết nối mạng trong động cơ xử lý dữ liệu (`office-engine`).
  - `NFR-PERF-01`: Khởi động ứng dụng tức thì (Cold Start $< 300\text{ ms}$).
  - `NFR-REL-01`: Không bao giờ làm hỏng dữ liệu cục bộ; hỗ trợ hủy tác vụ nhanh ($< 100\text{ ms}$) mà không crash.
  - Tương thích 100% với các bộ gõ tiếng Việt phổ biến (UniKey, EVKey, OpenKey, ibus-bamboo) không gây lỗi nhảy ký tự IME.

## Brand Commitments

- **Tên sản phẩm**: Office Utility VN.
- **Phong cách thương hiệu**: Chuyên nghiệp, đáng tin cậy, chuẩn mực công sở hiện đại, tốc độ và an tâm.
- **Bảng màu nhận diện**: Tông Slate/Dark cao cấp (`#0F172A`, `#1E293B`, `#334155`), màu nhấn chính Indigo Brand (`#4F46E5` / `#6366F1`), màu xanh lục bảo vệ an toàn dữ liệu (`#22C55E` / `#4ADE80`).
- **Ngôn ngữ**: 100% tiếng Việt tự nhiên, chuẩn hóa thuật ngữ kế toán, thuế và tin học văn phòng Việt Nam.

## Evidence on Hand

- Mã nguồn thực tế và 517 unit tests đã được kiểm chứng an toàn 100%.
- Bộ tài liệu đặc tả kiến trúc từ `01-introduction.md` đến `08_quy_trinh_van_hanh.md`.
- Các mẫu tệp thử nghiệm thực tế trong `crates/office-engine/tests/fixtures/` (PDF, Excel, hóa đơn XML, CCCD, VietQR Napas).

## Product Principles

1. **Bảo mật Tối thượng (Zero-Cloud Privacy)**: Dữ liệu thuộc về người dùng, chỉ xử lý tại chỗ, tuyệt đối không gửi lên bất kỳ máy chủ nào.
2. **Nghiệp vụ Bản địa Chuẩn xác (Vietnamese Native First)**: Tính toán thuế, ngày làm việc, đọc số tiền, mã QR, địa danh phải chuẩn xác theo quy định pháp luật và văn bản hiện hành của Việt Nam.
3. **Phản hồi Tức thì & Tinh giản (Snappy & Focused)**: Mọi thao tác phải phản hồi dưới 16ms (60 FPS), không có bước thừa, quy trình kéo-thả trực quan "1-Click là xong".
4. **An toàn Tài nguyên (Resource-Safe & Non-Destructive)**: Không làm tràn RAM (giữ $< 250\text{ MB}$), không ghi đè mất dữ liệu gốc của người dùng.

## Accessibility & Inclusion

- Tương phản màu sắc tuân thủ chuẩn WCAG 2.1 AA (tỷ lệ tương phản chữ $\ge 4.5:1$).
- Điều khiển đầy đủ thông qua bàn phím (Tab navigation, phím tắt toàn cục `Ctrl + K`, `Ctrl + Enter`, `Escape`).
- Thông báo trạng thái rõ ràng qua thanh tiến trình và thông điệp tiếng Việt dễ hiểu.

