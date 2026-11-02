# Thiết Kế Kỹ Thuật: Mô Hình Hub-and-Spoke Cho Trạm Chuyển Đổi Tệp Vạn Năng

> **Mã thiết kế:** `SPEC-HUB-SPOKE-01`  
> **Các công cụ liên quan:** `universal-converter` (`FR-HUB-01`), `markdown-to-styled-pdf` (`FR-DOC-18`), `pdf-table-extractor` (`FR-DOC-19`)  
> **Quy chuẩn:** Kịch bản 2 `modify-pod` (Tài liệu `docs/specs/08_quy_trinh_van_hanh.md`)  
> **Trạng thái:** Chờ phê duyệt (Awaiting User Approval)

---

## 1. Mục Đích & Bối Cảnh (Intent & Why)

Sau khi rà soát và thảo luận thiết kế, người dùng đã lựa chọn **Hướng 1: Mô hình Hub-and-Spoke**.
- **Vấn đề đặt ra:** Trạm Chuyển đổi Tệp Vạn năng (`universal-converter`) đóng vai trò trung tâm (Hub) giúp người dùng ném tệp bất kỳ vào và đổi đuôi nhanh mà không cần cấu hình. Tuy nhiên, các tác vụ như **"Biên dịch Markdown sang PDF"** và **"Trích xuất Bảng từ PDF ra Excel"** đòi hỏi các thông số nghiệp vụ chuyên sâu (Theme chuẩn công vụ VN, khổ giấy A4, dải trang sao kê ngân hàng).
- **Giải pháp:** 
  1. Ở tầng Engine: Bổ sung định tuyến `PDF -> XLSX/CSV` vào `UniversalConverterTool` (ủy quyền cho `PdfTableExtractorTool` với preset mặc định), song song với nhánh `MD -> PDF` đã có sẵn.
  2. Ở tầng Giao diện (UI): **Giữ nguyên 2 công cụ chuyên biệt độc lập trên Sidebar và Dashboard**, cho phép người dùng vừa có trải nghiệm chuyển đổi 1-click tiện lợi, vừa giữ trọn khả năng tinh chỉnh tham số chuyên sâu khi cần.

---

## 2. Mục Tiêu, Phi Mục Tiêu & Chống Mục Tiêu

### Mục tiêu (Goals):
1. **Engine Delegation (PDF -> Excel/CSV):** Tích hợp `PdfTableExtractorTool` vào `UniversalConverterTool` trong `crates/office-engine/src/sys_hub/universal_converter.rs`.
2. **Tự động hóa nhánh định tuyến:** Khi tệp đầu vào có chữ ký `%PDF-` (`FileCategory::DocumentPdf`):
   - Đích đến là `xlsx` hoặc `excel`: Chuyển tiếp sang `pdf-table-extractor` với tùy chọn `{ "output_csv": false, "page_range": null }`.
   - Đích đến là `csv`: Chuyển tiếp sang `pdf-table-extractor` với tùy chọn `{ "output_csv": true, "page_range": null }`.
3. **Bảo toàn giao diện độc lập (Spokes):** Giữ nguyên `markdown-to-styled-pdf` và `pdf-table-extractor` trong danh mục `ALL_TOOLS`, Sidebar và Command Palette.
4. **Kiểm thử tự động (TDD):** Bổ sung test case ma trận kiểm tra `PDF -> XLSX` và `PDF -> CSV` trong `universal_converter.rs`.

### Phi mục tiêu (Non-goals):
- Không xóa bỏ hoặc ẩn 2 công cụ chuyên biệt khỏi thanh Sidebar.
- Không thêm các trường tùy biến phức tạp (như dropdown chọn theme hoặc ô nhập dải trang) vào `UniversalConverterOptions`. Trạm vạn năng giữ nguyên giao diện tối giản với duy nhất trường `target_extension`.

### Chống mục tiêu (Anti-goals):
- Tuyệt đối KHÔNG vi phạm Spine Lock (`crates/office-shared` và `src-tauri/src/db`).
- Tuyệt đối KHÔNG gọi tiến trình nhị phân ngoài (không gọi LibreOffice, Poppler, Pandoc).
- Tuyệt đối KHÔNG dùng `.unwrap()` hoặc `.expect()` trong luồng xử lý.

---

## 3. Ràng Buộc Kỹ Thuật (Constraints)

- **`NFR-SEC-01`**: 100% On-Device, Zero Network Egress.
- **`NFR-SEC-03`**: An toàn bộ nhớ, không panic.
- **`NFR-PERF-01`**: Hỗ trợ cờ hủy tác vụ `cancel_flag: &AtomicBool` phản hồi dừng dưới 100ms.
- **Spine Read-Only Lock**: Giữ nguyên vẹn mã nguồn các crate đã khóa.

---

## 4. Các Hướng Tiếp Cận Đã Đánh Giá (Approaches)

1. **Hướng 1 (Đã chọn): Mô hình Hub-and-Spoke**
   - *Engine:* `universal-converter` làm Hub tự động điều phối `MD -> PDF` (đã có) và `PDF -> XLSX/CSV` (bổ sung mới).
   - *UI:* Giữ các công cụ chuyên biệt làm Spokes trên Sidebar.
   - *Đánh giá:* Cân bằng hoàn hảo giữa tính tiện lợi "1-click" và độ sâu nghiệp vụ.
2. **Hướng 2: Gộp hoàn toàn (Loại bỏ công cụ lẻ)**
   - *Đánh giá:* Làm mất tính trực quan của 54+ công cụ nghiệp vụ, giao diện trạm vạn năng bị cồng kềnh.
3. **Hướng 3: Phân định ranh giới tuyệt đối (Không hỗ trợ trong Hub)**
   - *Đánh giá:* Trạm vạn năng bị thiếu hụt tính năng cơ bản khi người dùng muốn đổi PDF sang bảng tính.

---

## 5. Chi Tiết Hiện Thực Kỹ Thuật (Implementation Details)

### 5.1. Cập nhật `UniversalConverterTool`
Trong `crates/office-engine/src/sys_hub/universal_converter.rs`:
- Khai báo thêm trường `pdf_table_tool: PdfTableExtractorTool` trong struct `UniversalConverterTool`.
- Khởi tạo trong `UniversalConverterTool::default()`.
- Trong phương thức `execute`, bổ sung nhánh xử lý match:
  ```rust
  (FileCategory::DocumentPdf, "xlsx" | "excel") => {
      let mut req = single_req.clone();
      req.tool_slug = "pdf-table-extractor".to_string();
      req.options = serde_json::json!({
          "output_csv": false,
          "page_range": None::<String>,
      });
      self.pdf_table_tool.execute(&req, cancel_flag, None)
  }

  (FileCategory::DocumentPdf, "csv") => {
      let mut req = single_req.clone();
      req.tool_slug = "pdf-table-extractor".to_string();
      req.options = serde_json::json!({
          "output_csv": true,
          "page_range": None::<String>,
      });
      self.pdf_table_tool.execute(&req, cancel_flag, None)
  }
  ```

### 5.2. Kiểm thử TDD
Bổ sung test unit `test_matrix_pdf_to_xlsx_and_csv_dispatch` trong module `tests` của `universal_converter.rs`.

---

## 6. Nhiệm Vụ Tiếp Theo (Next Steps)
Sau khi nhận được sự đồng ý của bạn với tài liệu thiết kế này, tôi sẽ kích hoạt kỹ năng `writing-plans` để lập kế hoạch triển khai chi tiết và tiến hành code theo chuẩn TDD.
