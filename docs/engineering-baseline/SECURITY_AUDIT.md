# BÁO CÁO KIỂM TOÁN BẢO MẬT CHUYÊN SÂU (SECURITY AUDIT)
**Dự án:** Office Utility VN (Desktop Application)  
**Phiên bản kiểm toán:** Commit SHA `8c57f9655bb0841064973effd6ddeb72c0fa3599`  
**Vai trò thẩm định:** Principal Security Reviewer & Independent Code Auditor  

---

## 1. PHÂN TÍCH BỀ MẶT TẤN CÔNG & QUẢN TRỊ QUYỀN HẠN TAURI V2

### 1.1. Cấu hình Content Security Policy (CSP) & Global Tauri Scope
- **Vị trí vi phạm:** `src-tauri/tauri.conf.json`, dòng 21–23 và dòng 13:
  ```json
  "withGlobalTauri": true,
  "security": {
    "csp": null
  }
  ```
- **Phân tích rủi ro (Rủi ro Cao - P1):**
  1. `"csp": null`: Việc vô hiệu hóa hoàn toàn chính sách bảo vệ tài nguyên nội dung (CSP) trong WebView khiến ứng dụng dễ bị tổn thương nếu có bất kỳ lỗ hổng Cross-Site Scripting (XSS) nào xảy ra (ví dụ: khi render nội dung Markdown, HTML hoặc tên tệp tin độc hại). Kẻ tấn công có thể chèn script độc hại tải mã từ bên ngoài hoặc đánh cắp dữ liệu DOM.
  2. `"withGlobalTauri": true`: Tùy chọn này tiêm toàn bộ đối tượng `window.__TAURI__` vào môi trường JavaScript toàn cục. Nếu kẻ tấn công chèn được script vào WebView, chúng có thể trực tiếp gọi các lệnh IPC nhạy cảm (`dispatch_tool_task`, `execute_workflow_pipeline`, `open_output_folder`, v.v.).
- **Khuyến nghị khắc phục:**
  - Thiết lập CSP chặt chẽ trong `tauri.conf.json`:
    ```json
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:;"
    }
    ```
  - Đặt `"withGlobalTauri": false` và sử dụng ES module bindings đóng gói cục bộ.

### 1.2. Phân quyền Tauri Capabilities & Plugins
- **Vị trí kiểm tra:** `src-tauri/capabilities/default.json`:
  ```json
  {
    "windows": ["main"],
    "permissions": [
      "core:default",
      "core:window:deny-destroy",
      "opener:default"
    ]
  }
  ```
- **Đánh giá:**
  - Nhóm quyền cơ bản được giới hạn tốt (`core:default`, `opener:default`), không cấp quyền mở rộng nguy hiểm như `fs:allow-all` hay `shell:allow-execute`.
  - Tuy nhiên, lệnh `open_output_folder` sử dụng `tauri_plugin_opener` với đường dẫn lấy từ `app.path().download_dir()`. Cần đảm bảo đường dẫn này luôn được backend kiểm tra tính hợp lệ trước khi mở ứng dụng mặc định của hệ điều hành.

---

## 2. KIỂM TOÁN AN TOÀN HỆ THỐNG TỆP & PERSISTENCE

### 2.1. Đánh giá Hệ thống Safe Persistence & Save Grants (Stage 4B/4C)
- **Vị trí kiểm tra:** `src-tauri/src/safe_persistence.rs` và `src-tauri/src/commands/mod.rs` (dòng 320–498).
- **Kết quả tích cực:**
  - Hai lệnh cũ nguy hiểm `read_file_binary` và `save_file_binary` đã được **khóa hoàn toàn vĩnh viễn (Fail-Closed)**:
    ```rust
    #[tauri::command]
    pub async fn read_file_binary(_file_path: String) -> Result<Vec<u8>, CommandError> {
        Err(CommandError::ValidationFailed { reason: "...đã bị vô hiệu hóa vĩnh viễn..." })
    }
    ```
  - Mọi thao tác đọc/lưu tệp nhị phân cho tài liệu giao diện (GridCraft, PDF Studio) bắt buộc phải thông qua `native_dialog_open_file` và `secure_save_document` kết hợp với `NativeSaveGrantManager`.
  - Quy trình ghi đĩa 10 bước (`native_atomic_durable_write`) thực hiện kiểm tra `validate_and_authorize_path`, kiểm tra null bytes, chống Path Traversal (`Component::ParentDir`), chặn các thư mục hệ thống nhạy cảm (`/etc`, `/usr`, `/bin`), tạo tệp tạm với RAII guard và hoán đổi nguyên tử kèm fsync.
- **Lỗ hổng & Hạn chế Tồn đọng (P2):**
  - **Danh mục phần mở rộng `ALLOWED_EXTENSIONS` bị thiếu:** Hiện chỉ cho phép `xlsx`, `xlsm`, `csv`, `tsv`, `json`, `ods`, `pdf`, `txt`, `docx`, `png`, `jpg`, `jpeg`. Thiếu hoàn toàn các định dạng đầu ra của một số công cụ như `.webp`, `.gif`, `.zip`, `.ics`, `.sql`, `.md`. Nếu các công cụ này được chuyển sang dùng `safe_persistence`, chúng sẽ bị từ chối với lỗi `ValidationFailed`.

---

## 3. MÃ HÓA, CHỮ KÝ SỐ & BẢO VỆ MẬT KHẨU (CRYPTOGRAPHY & SECRETS)

### 3.1. Giả Mạo Mã Hóa PDF Chuẩn (Fake AES Encryption - P0)
- **Vị trí vi phạm:** `crates/office-engine/src/doc_suite/pdf_protect_sign.rs`, dòng 149–165 và 196–204:
  ```rust
  fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
      let mut hasher = Sha256::new();
      hasher.update(password.as_bytes());
      hasher.update(salt);
      let hash = hasher.finalize();
      let mut key = [0u8; 32];
      key.copy_from_slice(&hash);
      key
  }

  fn cipher_xor(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
      let mut result = Vec::with_capacity(data.len());
      for (i, &b) in data.iter().enumerate() {
          result.push(b ^ key[i % 32]);
      }
      result
  }
  ```
- **Phân tích rủi ro (Nghiêm trọng - P0):**
  1. **Tuyên bố sai lệch (Deceptive Security Claim):** Công cụ quảng cáo triển khai chuẩn bảo mật PDF ISO 32000-1 (Standard Security Handler với AES-128 / AES-256). Nhưng trong thực tế, mã nguồn sử dụng thuật toán **XOR đối xứng đơn giản** với một chuỗi muối cố định hardcode `b"OFFICE_SEC_SALT_VN_2026"`.
  2. **Dữ liệu không thể đọc được:** Tệp đầu ra không tương thích với bất kỳ trình đọc PDF tiêu chuẩn nào (Adobe Acrobat, Foxit, PDF.js). Khi người dùng mở tệp bằng Acrobat, tài liệu sẽ báo lỗi hỏng từ điển `/Encrypt` hoặc lỗi cú pháp giải mã.
  3. **Nguy cơ rò rỉ dữ liệu:** Mã hóa XOR với khóa tuần hoàn 32 byte hoàn toàn không có giá trị bảo mật trước các kỹ thuật phân tích tần suất hoặc tấn công Known-Plaintext Attack.

### 3.2. Giả Mạo Chữ Ký Số PKCS#7 / CMS (Fake Digital Signature - P0)
- **Vị trí vi phạm:** `crates/office-engine/src/doc_suite/pdf_pki_sign.rs`, dòng 186–202:
  ```rust
  let sig_payload = format!(
      "PKCS7-SIG:CN={}:HASH={}:SIGNER={}",
      options.cert_cn, doc_hash_hex, options.signer_name
  );
  let mut sig_hex = String::new();
  for b in sig_payload.as_bytes() {
      sig_hex.push_str(&format!("{:02x}", b));
  }
  while sig_hex.len() < sig_placeholder_len {
      sig_hex.push('0');
  }
  ```
- **Phân tích rủi ro (Nghiêm trọng - P0):**
  - Công cụ ghi chuỗi ASCII hex `PKCS7-SIG:...` vào trường `/Contents` của từ điển `/Sig` thay vì cấu trúc ASN.1 DER-encoded Cryptographic Message Syntax (CMS) hợp lệ theo RFC 5652 / ISO 32000-2 (PAdES).
  - Bất kỳ công cụ xác thực chữ ký độc lập nào (như `pdfsig` hoặc Adobe Acrobat) đều từ chối tài liệu và báo chữ ký giả mạo / không hợp lệ.

### 3.3. Quản lý Mật Khẩu Trong Bộ Nhớ Heap (Memory Hygiene & Zeroize - P1)
- **Tình trạng:**
  - `doc_suite/pdf_protect_sign.rs` đã bọc mật khẩu trong `Zeroizing<String>` và gọi `up.zeroize()` sau khi xử lý.
  - Tuy nhiên, trong `data_lab/excel_sheet_unlocker.rs` và các cấu hình JSON DTO `DispatchTaskRequest::options`, mật khẩu vẫn được nạp qua `serde_json::Value` (dưới dạng chuỗi `String` chuẩn của Rust) trước khi parse vào struct tùy chọn. Chuỗi này tồn lưu tự do trên Heap và không được xóa sạch bằng `zeroize` khi JSON Value bị drop, vi phạm NFR-SEC-03.

---

## 4. PHÒNG CHỐNG TRÀN TÀI NGUYÊN & DECOMPRESSION BOMB

### 4.1. Giải Mã PNG & Giới Hạn Bộ Nhớ (NFR-SEC-05)
- **Hiện trạng:**
  - `crates/office-engine/src/media_studio/image_batch.rs` đã có bản vá uncommitted áp dụng `take(MAX_DECOMPRESSED_IDAT_BYTES + 1)` với ngưỡng 1 GB.
  - Tuy nhiên, 1 GB cho một luồng IDAT ảnh trên máy tính văn phòng thông thường là ngưỡng quá cao, có thể gây áp lực lớn lên RAM khi chạy song song 8–16 luồng Rayon.
  - Cần hạ ngưỡng an toàn xuống mức đo kiểm thực tế (ví dụ: tối đa 256 MB cho một ảnh đơn lẻ) và kiểm tra kích thước `width * height * 4` bằng phép tính có kiểm tra tràn số `checked_mul` trước khi phân bổ mảng pixel.

### 4.2. Bung Nén ZIP / OpenXML
- Trong `crates/office-engine/src/data_lab/excel_sheet_unlocker.rs`, logic duyệt các entry ZIP cần đảm bảo tỷ lệ bung nén (Compression Ratio) không vượt quá 1:100 và tổng dung lượng sau khi giải nén không vượt quá 1 GB để phòng chống Zip Bomb.

---

## 5. BẢNG TỔNG HỢP RỦI RO BẢO MẬT & ĐỀ XUẤT XỬ LÝ

| Mã Rủi Ro | Phân Loại | Thành Phần Ảnh Hưởng | Mô Tả Ngắn Gọn | Mức Độ | Trạng Thái Xử Lý |
| :--- | :--- | :--- | :--- | :---: | :--- |
| `SEC-01` | Fake Crypto | `pdf_protect_sign.rs` | Dùng XOR cipher và muối cố định thay vì AES-256 PDF chuẩn | 🔴 P0 | Chặn phát hành; vô hiệu hóa công cụ |
| `SEC-02` | Fake Signature | `pdf_pki_sign.rs` | Chữ ký số tạo bằng chuỗi text ASCII thay vì CMS ASN.1 DER | 🔴 P0 | Chặn phát hành; vô hiệu hóa công cụ |
| `SEC-03` | CSP Disabled | `tauri.conf.json` | `csp: null` và `withGlobalTauri: true` mở rộng bề mặt tấn công | 🟠 P1 | Thêm chính sách CSP nghiêm ngặt |
| `SEC-04` | Secret Lifetime | `excel_sheet_unlocker.rs` | Mật khẩu tồn lưu trong `serde_json::Value` không được zeroize | 🟠 P1 | Bọc `Zeroizing` ở mọi điểm nạp |
| `SEC-05` | Decompress Limit | `image_batch.rs` | Ngưỡng giải nén 1GB quá cao đối với môi trường đa luồng Rayon | 🟡 P2 | Hạ ngưỡng xuống 256MB + checked_mul |
| `SEC-06` | Whitelist Mismatch | `safe_persistence.rs` | Thiếu phần mở rộng `webp`, `gif`, `zip`, `ics`, `sql`, `md` | 🟡 P2 | Bổ sung đầy đủ định dạng hỗ trợ |
