# Universal Converter Hub-and-Spoke Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tích hợp `PdfTableExtractorTool` vào `UniversalConverterTool` để trạm chuyển đổi tệp vạn năng tự động điều phối khi người dùng ném tệp PDF vào và chọn chuyển đổi sang bảng tính Excel (`xlsx`) hoặc `csv`, đồng thời bảo toàn 2 công cụ chuyên biệt độc lập trên giao diện theo mô hình Hub-and-Spoke.

**Architecture:** Sử dụng mô hình In-Process Delegated Router trong `UniversalConverterTool` (`crates/office-engine/src/sys_hub/universal_converter.rs`). Khi nhận diện tệp đầu vào là PDF qua magic bytes `%PDF-`, router kiểm tra `target_extension` để ủy quyền thực thi cho `PdfTableExtractorTool` với cấu hình mặc định (phân phối tệp kết quả và trả về `DispatchTaskResponse`).

**Tech Stack:** Rust (2024 edition), `office-engine` crate, `pdf-table-extractor`, `office-shared`.

**Spec:** `docs/superpowers/specs/2026-11-02-universal-converter-hub-and-spoke-design.md`

## Global Constraints

- **NFR-SEC-01 (Zero Network):** 100% On-Device, không egress mạng.
- **NFR-SEC-03 (Zero Panic):** Không sử dụng `unwrap()` hoặc `expect()` trong mã nghiệp vụ.
- **NFR-PERF-01 (Cooperative Cancellation):** Kiểm tra `cancel_flag.load(Ordering::Relaxed)` trước mỗi thao tác I/O hoặc tính toán nặng, phản hồi dừng < 100ms.
- **Spine Read-Only Lock:** Tuyệt đối không thay đổi các file trong `crates/office-shared` và `src-tauri/src/db`.

## Review Focus

1. **PDF không có bảng tính:** Khi tệp PDF không chứa cấu trúc bảng rõ ràng, `PdfTableExtractorTool` trích xuất layout văn bản/bảng rỗng và xuất tệp CSV/XLSX hợp lệ không bị crash.
2. **Hủy tác vụ tức thời:** Khi `cancel_flag` được kích hoạt giữa chừng, `UniversalConverterTool` truyền cờ hủy tới `PdfTableExtractorTool` và trả về `CommandError::TaskCancelled` < 100ms.
3. **Đa dạng phần mở rộng đích:** Hỗ trợ cả `.xlsx`, `xlsx`, `excel`, `.csv`, `csv` (chuẩn hóa `trim().trim_start_matches('.').to_lowercase()`).
4. **Đường dẫn thư mục đầu ra:** Tôn trọng `output_directory` trong `DispatchTaskRequest` hoặc tự động ghi vào cùng thư mục tệp nguồn nếu `output_directory` là `None`.
5. **Giữ nguyên các công cụ độc lập trên UI:** Không thay đổi danh sách công cụ trong `ALL_TOOLS`, `Sidebar` hay `Command Palette`.

---

### Task 1: Tích hợp `PdfTableExtractorTool` vào `UniversalConverterTool`

**Files:**
- Modify: `crates/office-engine/src/sys_hub/universal_converter.rs`
- Test: nhúng trong `crates/office-engine/src/sys_hub/universal_converter.rs` (`mod tests`)

**Interfaces:**
- Consumes: `crate::doc_suite::PdfTableExtractorTool`
- Produces: `UniversalConverterTool` hỗ trợ thêm 2 đường chuyển đổi:
  - `FileCategory::DocumentPdf -> "xlsx" | "excel"`
  - `FileCategory::DocumentPdf -> "csv"`

- [x] **Step 1: Viết test thất bại (RED) kiểm tra dispatch PDF -> XLSX và PDF -> CSV**

Thêm test case `test_matrix_pdf_to_xlsx_and_csv_dispatch` vào cuối `mod tests` trong `crates/office-engine/src/sys_hub/universal_converter.rs`:

```rust
    #[test]
    fn test_matrix_pdf_to_xlsx_and_csv_dispatch() {
        let tool = UniversalConverterTool::default();
        let tmp = std::env::temp_dir().join("test_univ_pdf_table");
        let _ = fs::create_dir_all(&tmp);

        // Tạo PDF mẫu hợp lệ tối thiểu
        let sample_pdf = tmp.join("sample.pdf");
        let minimal_pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << >> >>\nendobj\n4 0 obj\n<< /Length 44 >>\nstream\nBT /F1 12 Tf 100 700 Td (Ma Hang | Don Gia) Tj ET\nendstream\nendobj\nxref\n0 5\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n0000000115 00000 n \n0000000214 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n308\n%%EOF\n";
        fs::write(&sample_pdf, minimal_pdf).expect("write sample pdf");

        // 1. PDF -> XLSX
        let req_xlsx = DispatchTaskRequest {
            task_id: "task-pdf-xlsx".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![sample_pdf.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_extension": "xlsx" }),
        };
        let cancel = AtomicBool::new(false);
        let res_xlsx = tool.execute(&req_xlsx, &cancel, None).expect("convert pdf to xlsx");
        assert_eq!(res_xlsx.status, "SUCCESS");
        assert_eq!(res_xlsx.output_paths.len(), 1);
        assert!(res_xlsx.output_paths[0].ends_with(".xlsx"));

        // 2. PDF -> CSV
        let req_csv = DispatchTaskRequest {
            task_id: "task-pdf-csv".to_string(),
            tool_slug: "universal-converter".to_string(),
            input_paths: vec![sample_pdf.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "target_extension": "csv" }),
        };
        let res_csv = tool.execute(&req_csv, &cancel, None).expect("convert pdf to csv");
        assert_eq!(res_csv.status, "SUCCESS");
        assert_eq!(res_csv.output_paths.len(), 1);
        assert!(res_csv.output_paths[0].ends_with(".csv"));
    }
```

- [x] **Step 2: Chạy test để xác nhận thất bại (RED)**

Run: `cargo test -p office-engine test_matrix_pdf_to_xlsx_and_csv_dispatch`
Expected: FAIL với lỗi `ValidationFailed` vì `PDF -> xlsx` chưa được phân nhánh xử lý.

- [x] **Step 3: Hiện thực mã nguồn trong `crates/office-engine/src/sys_hub/universal_converter.rs`**

1. Import `PdfTableExtractorTool`:
   ```rust
   use crate::doc_suite::{
       MarkdownToPdfTool, PdfMergeSplitTool, PdfTableExtractorTool, PdfToLongImageTool,
       PdfToMarkdownTool, VietnameseOcrTool,
   };
   ```
2. Thêm trường `pdf_table_tool: PdfTableExtractorTool` vào struct `UniversalConverterTool`.
3. Khởi tạo `pdf_table_tool: PdfTableExtractorTool` trong `UniversalConverterTool::default()`.
4. Bổ sung 2 nhánh match trong `execute`:
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
       let mut res = self.pdf_table_tool.execute(&req, cancel_flag, None)?;
       res.output_paths.retain(|p| p.ends_with(".csv"));
       Ok(res)
   }
   ```

- [x] **Step 4: Chạy lại test để xác nhận thành công (GREEN)**

Run: `cargo test -p office-engine test_matrix_pdf_to_xlsx_and_csv_dispatch`
Expected: PASS (ok. 1 passed; 0 failed).

- [x] **Step 5: Chạy toàn bộ test suite và xác thực Spine Lock**

Run: `./scripts/verify_spine_lock.sh`
Expected: PASS 100% (517 unit tests passed, spine lock hợp lệ).

- [x] **Step 6: Commit thay đổi**

```bash
git add crates/office-engine/src/sys_hub/universal_converter.rs docs/superpowers/
git commit -m "feat(hub): integrate pdf table extraction to xlsx and csv in universal converter"
```
