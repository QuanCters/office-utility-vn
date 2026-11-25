# Review Rubric: Correctness & Invariant Integrity

Tài liệu chuẩn mực hướng dẫn agent và reviewer kiểm tra tính đúng đắn logic, quản lý trạng thái và tính toàn vẹn của các bất biến hệ thống trong **Office Utility VN**.

## 1. Mục tiêu Kiểm tra
Đảm bảo mọi thay đổi code không làm suy thoái logic nghiệp vụ, không gây rò rỉ trạng thái, data racing, hoặc vi phạm các bất biến nền tảng của ứng dụng Desktop Office.

## 2. Các Bất Biến Cốt Lõi (Core Invariants)
Mỗi thay đổi đối với document coordinator, persistence hoặc UI state bắt buộc phải bảo toàn 8 bất biến sau:

| Thành phần | Bất biến bắt buộc tuân thủ | Hậu quả nếu vi phạm |
| :--- | :--- | :--- |
| **HostDocumentCoordinator** | **Không bao giờ ghi đè state cũ lên state mới (No Stale Writes).** Khi tải lại hoặc nhận sự kiện từ nhiều tab/cửa sổ, ID phiên bản hoặc timestamp phải được kiểm tra trước khi ghi nhận thay đổi vào bộ nhớ. | Mất dữ liệu người dùng vừa chỉnh sửa, rollback ngầm không kiểm soát. |
| **NativeSaveGrantManager** | **Chỉ thao tác đúng file/path được cấp quyền (Path Boundary Enforcement).** Không được tùy ý duyệt hoặc ghi file ngoài đường dẫn được người dùng chọn qua dialog hoặc cấp quyền tường minh. | Lỗ hổng Directory Traversal, ghi đè file hệ thống ngoài ý muốn. |
| **NativeSafePersistence** | **Ghi lỗi không làm hỏng tài liệu gốc (Atomic Persistence).** Bắt buộc sử dụng cơ chế ghi tệp tạm (`.tmp`), đồng bộ đĩa (`sync_all`), và đổi tên nguyên tử (`atomic rename`). Nếu thao tác thất bại giữa chừng, tài liệu gốc phải còn nguyên vẹn 100%. | File bị cắt cụt (truncated 0 bytes), mất trắng dữ liệu khi mất điện hoặc đầy đĩa. |
| **WindowCloseGuard** | **Không bao giờ thoát làm mất thay đổi chưa lưu (Dirty State Interception).** Khi người dùng hoặc OS gửi lệnh đóng cửa sổ (`CloseRequested`), phải chặn lại nếu `is_dirty == true`, hiển thị hộp thoại xác nhận Lưu/Bỏ qua/Hủy. | Mất dữ liệu chưa lưu khi vô tình đóng cửa sổ. |
| **Tauri IPC** | **Không vượt qua kiểm tra quyền và validation (Strict Input Sanitization).** Mọi tham số đầu vào từ Leptos qua invoke handler phải được serialize thành DTO chuẩn, kiểm tra validation bounds trước khi chuyển sang Engine. | Lỗi panics, logic injection, crash tiến trình Tauri backend. |
| **PDF/WebAssembly** | **Không làm hỏng byte đầu ra hoặc tài liệu nguồn (Byte Offset & XREF Precision).** Thao tác PDF phải đánh số lại Object ID tuần tự, cập nhật đúng XREF byte offset 10 chữ số, không sửa đổi file nguồn gốc. | PDF hỏng cấu trúc (corrupted), không mở được bằng Acrobat/Foxit. |
| **GridCraft Vendor** | **Không thay đổi patch/pin ngoài phạm vi được duyệt.** Mã nguồn bên thứ ba trong `vendor/` phải được khóa hash hoặc commit pin rõ ràng. | Drift phiên bản, xung đột license hoặc đưa code chưa kiểm định vào production. |
| **Legacy Data Lab** | **Không phá đường rollback đã được giữ lại.** Các tính năng mới phải cung cấp fallback hoặc adapter tương thích ngược, không phá hủy khả năng khôi phục phiên bản trước. | Gián đoạn quy trình xử lý của người dùng đang phụ thuộc vào định dạng cũ. |

## 3. Checklist Phê Duyệt Correctness
- [ ] Hàm nghiệp vụ có xử lý đầy đủ các nhánh lỗi (`Result<T, CommandError>`) không? Tuyệt đối cấm `.unwrap()` hoặc `.expect()`.
- [ ] Các tác vụ tuần tự có đảm bảo tính nguyên tử (Atomicity) không?
- [ ] Vòng đời dữ liệu Leptos CSR: Signals và Resources có bị memory leak hoặc stale closure không?
- [ ] Quản lý đường dẫn Windows/Linux/macOS: Có sử dụng `PathBuf` chuẩn hóa (`canonicalize`) và xử lý ký tự phân cách đường dẫn an toàn không?
