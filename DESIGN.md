---
name: Office Utility VN
description: "Hệ thống thiết kế công thái học cho ứng dụng Desktop tiện ích văn phòng 100% On-Device chuẩn hóa nghiệp vụ Việt Nam"
colors:
  primary: "#4F46E5"
  primary-hover: "#4338CA"
  primary-light: "#6366F1"
  neutral-app: "#0B1120"
  neutral-bg: "#0F172A"
  neutral-surface: "#1E293B"
  neutral-card: "#334155"
  neutral-card-hover: "#3B4D66"
  neutral-border: "#475569"
  neutral-border-subtle: "#334155"
  neutral-text: "#F8FAFC"
  neutral-text-muted: "#94A3B8"
  neutral-text-subtle: "#64748B"
  success: "#10B981"
  success-hover: "#059669"
  success-light: "#34D399"
  success-surface: "#064E3B"
  warning: "#F59E0B"
  warning-hover: "#D97706"
  warning-light: "#FBBF24"
  warning-surface: "#78350F"
  danger: "#EF4444"
  danger-hover: "#DC2626"
  danger-light: "#F87171"
  danger-surface: "#7F1D1D"
  info: "#0284C7"
  info-light: "#38BDF8"
typography:
  display:
    fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"
    fontSize: "24px"
    fontWeight: 700
    lineHeight: 1.25
    letterSpacing: "-0.02em"
  headline:
    fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"
    fontSize: "20px"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "-0.01em"
  title:
    fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"
    fontSize: "16px"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "0em"
  body:
    fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "0em"
  body-medium:
    fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"
    fontSize: "14px"
    fontWeight: 500
    lineHeight: 1.5
    letterSpacing: "0em"
  label:
    fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"
    fontSize: "12px"
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: "0.01em"
  mono:
    fontFamily: "'JetBrains Mono', 'Fira Code', ui-monospace, SFMono-Regular, monospace"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: "0.02em"
rounded:
  sm: "4px"
  md: "6px"
  lg: "8px"
  xl: "12px"
  full: "9999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "20px"
  2xl: "24px"
  3xl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.neutral-text}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"
  button-secondary:
    backgroundColor: "{colors.neutral-card}"
    textColor: "{colors.neutral-text}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  button-danger:
    backgroundColor: "{colors.danger}"
    textColor: "{colors.neutral-text}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  input-field:
    backgroundColor: "{colors.neutral-surface}"
    textColor: "{colors.neutral-text}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
---

# Design System: Office Utility VN

## Overview

**Creative North Star: "The Sovereign Workspace Cockpit"**

Office Utility VN là không gian làm việc chuyên nghiệp, khép kín và an toàn tuyệt đối cho nhân sự kế toán, hành chính và quản lý doanh nghiệp Việt Nam. Giao diện được thiết kế với phong thái điềm tĩnh, chính xác và chuẩn mực của một công cụ văn phòng cao cấp: loại bỏ hoàn toàn các yếu tố đồ họa thừa thãi, tối ưu hóa công thái học thị giác khi người dùng phải đối chiếu song song với bảng tính Excel và phần mềm kế toán trong nhiều giờ liên tục.

Hệ thống thị giác chuyển từ cảm giác "template AI công nghệ đen tối" sang phong cách phòng điều hành bản địa (Native Sovereign Cockpit): nền Slate phân tầng sắc nét, đường viền cấu trúc 1px thanh mảnh, màu nhấn Indigo Brand sang trọng và tín hiệu bảo mật xanh ngọc Emerald củng cố cam kết Zero-Cloud.

**Key Characteristics:**
- **Điềm tĩnh & Tập trung (Sovereign Calm):** Tông màu nền Slate đậm nhiều tầng triệt tiêu chói sáng nhưng vẫn giữ độ tương phản chữ đạt chuẩn WCAG 2.1 AA ($\ge 4.5:1$).
- **Chuẩn hóa Nghiệp vụ Kế toán (Accounting Grade Precision):** Mọi con số, đơn vị tiền tệ, mã thuế và ngày tháng đều sử dụng font số đơn cách (tabular numerals) kèm phân định dấu chấm/phẩy theo chuẩn Việt Nam.
- **Phản hồi Tức thì & Không Bẫy Tương tác (Zero-Trap Responsiveness):** Không có component "chết" hay dữ liệu mẫu bị gán cứng. Mọi nút bấm, phím tắt và vùng kéo thả đều kết nối trực tiếp với năng lực thật của hệ thống.

## Colors

Bảng màu được tổ chức chặt chẽ theo hệ trục Slate Hiện đại kết hợp các điểm nhấn chức năng ngữ nghĩa cao, loại bỏ triệt để tình trạng lệch tông ngẫu nhiên giữa các module.

### Primary (Màu Thương hiệu Chính)
- **Indigo Brand** (`#4F46E5`): Sử dụng cho các hành động chính yếu nhất của màn hình: nút "Thực thi tác vụ" (`Ctrl + Enter`), thanh tiến trình đang xử lý, trạng thái kích hoạt của tab điều hướng.
- **Indigo Deep** (`#4338CA`): Trạng thái hover và active của nút chính.
- **Indigo Focus Glow** (`#6366F1`): Đường viền focus cho các ô nhập liệu khi người dùng kích hoạt bàn phím.

### Secondary & Semantic Accents (Tín hiệu Chức năng Bản địa)
- **Emerald Guard (Thành công & Bảo mật)** (`#10B981` / nền `#064E3B`): Đại diện cho cam kết bảo mật "100% On-Device Native", thông báo hoàn thành tác vụ không lỗi, badge xác thực chữ ký số và tệp hợp lệ.
- **Amber Notice (Cảnh báo Thận trọng)** (`#F59E0B` / nền `#78350F`): Cảnh báo ghi đè tệp, cảnh báo dung lượng lớn hoặc khi người dùng hủy tiến trình giữa chừng.
- **Rose Alert (Lỗi & Hành động Phá hủy)** (`#EF4444` / nền `#7F1D1D`): Viền và hộp thông báo lỗi nghiệp vụ (sai định dạng, tệp hỏng), nút Đóng cửa sổ khi hover, nút Xóa cấu hình mẫu.
- **Sky Precision (Thông tin & Điều hướng)** (`#0284C7` / nền `#0C4A6E`): Icon thông tin tooltip, phím tắt điều hướng nhanh, chỉ báo tệp đang được xếp hàng chờ.

### Neutral (Hệ thống Nền Slate 6 Bậc)
- **Window Deep** (`#0B1120`): Thanh tiêu đề cửa sổ frameless titlebar và vùng kéo cửa sổ.
- **Canvas Main** (`#0F172A`): Nền tổng thể của toàn bộ ứng dụng, tạo chiều sâu thị giác ổn định.
- **Surface Zone** (`#1E293B`): Nền phân vùng chính (Zone 1 Header, Zone 2 Canvas, Zone 3 Action Bar, Sidebar).
- **Card & Resting** (`#334155`): Nền của ô nhập liệu, thẻ danh sách tệp, nút phụ, vùng kéo thả tệp khi nghỉ.
- **Card Hover & Active** (`#3B4D66`): Trạng thái hover của các thẻ tương tác và hàng danh sách.
- **Structural Border** (`#475569`): Đường kẻ ngăn cách cấu trúc 1px giữa các vùng làm việc.
- **Subtle Border** (`#334155`): Đường viền tinh tế của các ô input, dropdown selector.
- **Text Main** (`#F8FAFC`): Chữ trắng ngà có độ tương phản tối cao cho tiêu đề, nhãn trường và dữ liệu kết quả.
- **Text Muted** (`#94A3B8`): Chữ phụ cho mô tả, phím tắt ghi chú, metadata tệp.
- **Text Subtle** (`#64748B`): Placeholder, icon vô hiệu hóa, đường kẻ phụ.

### Quy chuẩn Loại bỏ Màu Lệch tông (Color Drift Elimination)
1. **Loại bỏ 11+ mã màu nền tùy tiện:** Xóa bỏ hoàn toàn các mã màu nền phát sinh lẻ tẻ trong các file CSS cũ (`#0d1527`, `#111827`, `#141E2E`, `#141E33`, `#162032`, `#182234`, `#223046`, `#243048`, `#243248`, `#293548`). Tất cả phải quy về 4 cấp: `#0B1120` (Titlebar) $\rightarrow$ `#0F172A` (Main) $\rightarrow$ `#1E293B` (Surface) $\rightarrow$ `#334155` (Card/Input).
2. **Loại bỏ màu Tím/Hồng rác:** Thay thế các mã màu tím `#9333EA`, `#C084FC`, `#A855F7`, `#8B5CF6`, `#7C3AED` trên nút Ghim cửa sổ bằng màu Indigo thương hiệu `#4F46E5` hoặc Sky `#0284C7`.
3. **Thống nhất màu Xanh lá:** Bãi bỏ sự pha trộn lộn xộn giữa họ Tailwind Green (`#22C55E`, `#4ADE80`) và Emerald (`#059669`, `#10B981`, `#34D399`). Chuẩn hóa độc tôn về hệ Emerald (`#10B981` cho điểm nhấn, `#34D399` cho chữ sáng, `#064E3B` cho nền badge).

### Named Rules
**The Slate Sanctum Rule.** Nền ứng dụng chỉ sử dụng duy nhất bảng màu Slate trung tính. Không pha sắc độ xanh lam sáng hay xám tro vào nền để tránh sai lệch cảm nhận màu sắc chứng từ của người dùng.

**The 10% Accent Restraint Rule.** Màu nhấn chính Indigo (`#4F46E5`) không chiếm quá 10% diện tích màn hình. Màu sắc rực rỡ chỉ xuất hiện để dẫn dắt hành động cốt lõi, không dùng để trang trí tràn lan.

**The Status Truth Rule.** Không bao giờ ngụy trang lỗi thành thành công. Khung thông báo lỗi bắt buộc dùng viền Rose `#EF4444` và nền `#7F1D1D`, tuyệt đối không bao giờ hiển thị thông điệp lỗi trong khung xanh lá `#10B981`.

## Typography

**Body & Interface Font:** `'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", sans-serif`  
**Accounting & Technical Font:** `'JetBrains Mono', 'Fira Code', ui-monospace, SFMono-Regular, monospace`

Kiểu chữ mang tinh thần kỹ thuật chính xác, hỗ trợ hiển thị dấu thanh điệu tiếng Việt hoàn mỹ ở mọi kích thước, không xảy ra tình trạng chân chữ bị cắt hoặc lỗi nhảy ký tự IME.

### Hierarchy
- **Display** (Bold 700, `24px`, line-height `1.25`, tracking `-0.02em`): Tiêu đề trang trọng của modal khởi tạo, màn hình chào mừng hoặc số liệu thống kê lớn.
- **Headline** (Semi-bold 600, `20px`, line-height `1.3`, tracking `-0.01em`): Tên công cụ chuyên sâu tại Vùng 1 (`Zone 1 Tool Title`).
- **Title** (Semi-bold 600, `16px`, line-height `1.4`, tracking `0em`): Tiêu đề các nhóm tùy chọn, tên khối tệp kéo thả, nhãn trạng thái kết quả.
- **Body** (Regular 400 & Medium 500, `14px`, line-height `1.5`, tracking `0em`): Văn bản đọc chính, giá trị ô nhập liệu, nhãn tùy chọn radio/checkbox, mô tả công cụ.
- **Label & Badge** (Medium 500 & Semi-bold 600, `12px`, line-height `1.4`, tracking `0.01em`): Huy hiệu bảo mật "100% On-Device", nhãn phím tắt (`Ctrl + Enter`), metadata kích thước tệp, tooltip hướng dẫn.
- **Tabular Mono** (Regular 400 & Medium 500, `13px`, line-height `1.4`, tracking `0.02em`): Số tiền kế toán bằng số, mã căn cước công dân, mã bưu chính, đường dẫn tệp, checksum SHA-256.

### Named Rules
**The 12px Ergonomic Floor Rule.** Không có bất kỳ thành phần giao diện hay nhãn văn bản chức năng nào được phép có cỡ chữ dưới `12px`. Loại bỏ triệt để các cỡ chữ siêu nhỏ `10px` và `11px` vốn gây mỏi mắt cho nhân sự kế toán/văn phòng.

**The Vietnamese Accent Preservation Rule.** Cấm áp dụng `text-transform: uppercase` trên các chuỗi tiếng Việt có độ dài trên 2 từ. Chữ in hoa làm các dấu mũ (â, ê, ô), dấu móc (ơ, ư) và dấu thanh (sắc, huyền, hỏi, ngã, nặng) bị dính chùm hoặc mất cân đối thị giác.

**The Tabular Figures Doctrine.** Mọi trường dữ liệu số tài chính, số tiền kế toán hoặc thông số kỹ thuật bắt buộc áp dụng thuộc tính `font-variant-numeric: tabular-nums` và tự động phân tách hàng nghìn bằng dấu chấm (`1.250.000 ₫`) ngay khi người dùng gõ phím.

## Layout

Cấu trúc không gian làm việc tuân thủ triệt để mô hình **Unified 3-Zone Workspace** kết hợp Sidebar điều hướng phân hệ thông minh, giải quyết triệt để vấn đề "60+ công cụ bị giấu sau Ctrl+K" được nêu trong critique.

### Spatial Model: Unified 3-Zone Workspace
1. **Zone 1: Header, Security Badge & Presets (Cố định 54px):**
   - Góc trái: Tên công cụ đang hoạt động (`16px Semi-bold`) kèm icon phân hệ đặc trưng.
   - Ở giữa: Huy hiệu bảo mật *"🟢 100% On-Device Native (Rayon)"* tạo dựng niềm tin tuyệt đối.
   - Góc phải: Bộ chọn cấu hình mẫu (`Preset Selector`) và nút Ghim cửa sổ Always-on-Top.
2. **Zone 2: Smart Morphing Canvas (Tự động co giãn theo viewport):**
   - Đối với công cụ Tài liệu/Tệp: Vùng kéo thả tệp thông minh, hiển thị danh sách tệp trực quan (có icon định dạng PDF/Word/Excel, dung lượng, nút xóa từng tệp) kèm các thẻ tham số tương thích thực tế.
   - Đối với công cụ Kế toán/Văn bản/Số: Bố cục 2 cột song song (Cột trái: Ô nhập liệu & Tùy chọn phương ngữ; Cột phải: Kết quả đọc chữ/chuyển đổi có thể bôi đen và sao chép 1-click).
3. **Zone 3: Progress & Action Bar (Cố định 60px):**
   - Bên trái: Thanh tiến trình mượt mà (0% $\rightarrow$ 100%) hiển thị đúng thông điệp tiến trình từ luồng nền.
   - Ở giữa: Gợi ý phím tắt nhanh (`Ctrl + Enter` thực thi, `Esc` hủy, `Ctrl + O` mở thư mục).
   - Bên phải: Cụm nút hành động chính (Nút Thực thi Indigo nổi bật, nút Hủy tác vụ khi đang chạy).

### Spacing Scale Chuẩn hóa (4pt/8pt Rhythm)
Bãi bỏ hoàn toàn các khoảng cách tùy tiện (2px, 3px, 6px, 7px, 10px, 14px). Toàn bộ hệ thống áp dụng thang 7 bước chuẩn:
- `xs` (`4px`): Khoảng cách giữa icon và chữ nhỏ, offset viền.
- `sm` (`8px`): Khoảng cách giữa các chip, padding dọc của nút bấm nhỏ và badge.
- `md` (`12px`): Padding trong của input field, khoảng cách giữa các phần tử form thông thường.
- `lg` (`16px`): Khoảng cách giữa các khối chức năng trong Vùng 2, padding chuẩn của card.
- `xl` (`20px`): Khoảng cách phân cách giữa các cụm tùy chọn lớn.
- `2xl` (`24px`): Padding biên xung quanh các Vùng làm việc chính.
- `3xl` (`32px`): Khoảng cách thoáng cho các màn hình trống (Empty States) hoặc hộp thoại xác nhận.

## Elevation & Depth

Ứng dụng desktop văn phòng đòi hỏi tính thực dụng và độ phản hồi nhanh, do đó triết lý thiết kế chủ đạo là **Phẳng & Phân tầng Sắc độ (Flat & Tonal Layering)** thay vì lạm dụng bóng đổ mờ ảo (Heavy Blurs).

### Tonal Hierarchy
- Chiều sâu được tạo ra bằng sự xếp lớp màu nền từ tối nhất đến sáng nhất:
  - Cấp 0 (Sâu nhất): Titlebar `#0B1120`
  - Cấp 1 (Nền tổng): Workspace Canvas `#0F172A`
  - Cấp 2 (Vùng chức năng): Zone Container `#1E293B`
  - Cấp 3 (Vật thể nổi): Input Fields, File Cards, Tool Cards `#334155`
  - Cấp 4 (Phần tử bay / Overlay): Command Palette, Tooltips, Context Menus `#1E293B` viền `#475569` với bóng đổ nổi nhẹ (`box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5)`).

### Named Rules
**The No-Nested-Cards Rule.** Nghiêm cấm lồng một thẻ có viền và nền đậm bên trong một thẻ khác có cùng tính chất (loại bỏ lỗi "hộp Dropzone lồng trong hộp Zone 2" được chỉ ra trong critique). Vùng thả tệp phải trở thành một phần tự nhiên của Canvas với viền nét đứt tinh tế thay vì đóng khung nặng nề.

**The 1px Precision Border Rule.** Mọi đường viền phân tách cấu trúc đều có độ dày đúng `1px` với màu `#334155` (tinh tế) hoặc `#475569` (rõ ràng). Không dùng viền dày $> 1px$ gây cảm giác thô kệch.

## Shapes

Hệ thống hình khối hướng đến cảm giác chắc chắn, chuyên nghiệp và thanh thoát của công cụ quản trị hiện đại.

- **Micro / Tag Radius** (`4px`): Dành cho các phím tắt kbd (`Ctrl`, `Enter`), badge phân loại tệp và checkbox.
- **Control Radius** (`6px`): Dành cho toàn bộ nút bấm (buttons), ô nhập văn bản (inputs), dropdown select và switch toggle.
- **Container Radius** (`8px`): Dành cho các phân vùng Vùng 1/2/3, thẻ danh mục công cụ, thẻ tệp đã nạp và khung kết quả.
- **Overlay Radius** (`12px`): Dành cho hộp thoại xác nhận, modal cập nhật phần mềm và Command Palette (`Ctrl + K`).
- **Pill Radius** (`9999px`): Chỉ dùng cho huy hiệu bảo mật *"🟢 100% On-Device Native"* và các chip đếm số lượng tệp đã xử lý.

## Components

### 1. Frameless Window Titlebar
- Chiều cao cố định `38px`, nền `#0B1120`, viền dưới `1px solid #1E293B`.
- Vùng kéo cửa sổ (`data-tauri-drag-region`) bao trùm phần giữa.
- Cụm nút điều khiển cửa sổ góc phải: 🗕 Thu nhỏ, 🗖 Phóng to, ✕ Đóng (khi hover chuyển sang nền đỏ `#EF4444`).
- Nút "Ghim cửa sổ" (Always-on-Top): khi kích hoạt chuyển sang nền Indigo `#4F46E5` kèm biểu tượng ghim rõ nét.

### 2. Trust Badge (Bảo chứng Bảo mật)
- Nền xanh ngọc trong mờ (`rgba(16, 185, 129, 0.12)`), viền `1px solid rgba(16, 185, 129, 0.3)`.
- Chữ xanh ngọc sáng `#34D399`, font size `12px Medium`.
- Chấm tròn xanh lục có hiệu ứng phát sáng nhẹ, tạo cảm giác an tâm tuyệt đối khi xử lý hồ sơ nhạy cảm.

### 3. Smart Dropzone (Vùng Kéo Thả Thông Minh)
- Trạng thái nghỉ: Nền `#1E293B`, viền nét đứt `1.5px dashed #475569`, góc bo `8px`, khoảng đệm `24px`.
- Trạng thái kéo tệp qua (`drag-over`): Nền `#334155`, viền đổi sang màu Indigo `#6366F1` sáng rực.
- Tương tác: Hỗ trợ click chuột mở hộp thoại chọn tệp hệ thống Native File Picker thật (thay vì chèn file mẫu giả lập); hỗ trợ phím Tab/Enter.
- Phân loại icon tệp rõ ràng: Tệp PDF màu đỏ `#F87171`, Excel màu xanh lá `#34D399`, Word màu xanh dương `#60A5FA`.

### 4. Accounting Text & Number Inputs
- Nền `#1E293B`, viền `1px solid #334155`, chữ `#F8FAFC`, padding `8px 12px`, font size `14px`.
- Trạng thái focus: Viền chuyển sang `#6366F1`, bóng focus tỏa sáng nhẹ `0 0 0 3px rgba(99, 102, 241, 0.2)`.
- Input số tiền: Tự động format dấu chấm hàng nghìn và hiển thị hậu tố đơn vị tiền tệ cố định `₫` ở góc phải. Hỗ trợ bắt sự kiện composition gõ dấu tiếng Việt (UniKey/EVKey) an toàn.

### 5. Result Container (Khung Kết Quả Tách Bạch)
- **Khi tác vụ thành công:** Nền `#1E293B`, viền `1px solid #10B981`. Tiêu đề kết quả màu xanh ngọc `#34D399`. Cho phép bôi đen văn bản (`user-select: text !important`), có nút "Sao chép kết quả" (`Ctrl + C`) và nút "Mở thư mục chứa tệp" (`Ctrl + O`).
- **Khi tác vụ gặp lỗi:** Nền `#1E293B`, viền `1px solid #EF4444`. Tiêu đề thông báo "Lỗi xử lý" màu đỏ `#F87171` kèm mô tả chi tiết nguyên nhân bằng tiếng Việt dễ hiểu và nút "Thử lại ngay". Tuyệt đối không render thông báo lỗi trong khung viền xanh thành công.

### 6. Action & Progress Bar (Zone 3)
- Nút Thực thi chính: Nền Indigo `#4F46E5`, chữ trắng `#FFF`, font `14px Semi-bold`, padding `10px 20px`, bo góc `6px`.
- Nút Hủy bỏ (khi đang chạy): Nền đỏ `#EF4444`, hover `#DC2626`.
- Thanh tiến trình: Rãnh trượt nền `#334155` bo tròn `9999px`, chiều cao `6px`. Vệt tiến trình phủ dải màu Indigo sang Emerald chuyển động êm ái, phản ánh đúng dữ liệu từ IPC Channel luồng nền.

## Do's and Don'ts

### Do:
- **Do** giữ cỡ chữ tối thiểu của mọi nhãn văn bản chức năng ở mức $\ge 12px$.
- **Do** định dạng mọi số tiền kế toán với dấu phân cách hàng nghìn (`1.250.000 ₫`) và kích hoạt font số `tabular-nums`.
- **Do** cho phép bôi đen chọn chữ (`user-select: text`) và cung cấp nút "Sao chép" ở tất cả các khung kết quả đầu ra.
- **Do** mở hộp thoại chọn tệp hệ thống Native File Picker khi người dùng click vào Dropzone.
- **Do** phân tách rạch ròi trạng thái Thành công (viền Emerald) và trạng thái Lỗi (viền Rose) trong khung kết quả.
- **Do** tuân thủ thang đo khoảng cách nhịp 4pt/8pt (`4px`, `8px`, `12px`, `16px`, `20px`, `24px`, `32px`).
- **Do** đảm bảo tỷ lệ tương phản chữ đạt chuẩn WCAG 2.1 AA ($\ge 4.5:1$).

### Don't:
- **Don't** sử dụng cỡ chữ siêu nhỏ `10px` hoặc `11px` trên giao diện.
- **Don't** dùng `text-transform: uppercase` trên các chuỗi tiếng Việt dài hơn 2 từ.
- **Don't** tự ý thêm các mã màu nền ngẫu nhiên ngoài 4 sắc độ Slate chuẩn (`#0B1120`, `#0F172A`, `#1E293B`, `#334155`).
- **Don't** hiển thị thông báo lỗi bên trong khung kết quả xanh lá ngọc lục bảo.
- **Don't** chèn cứng tệp mẫu giả lập vào Dropzone khi người dùng tương tác thực tế.
- **Don't** lồng hộp Dropzone có viền đậm bên trong Vùng 2 (tránh lỗi nested-cards).
- **Don't** chiếm dụng các phím tắt hệ thống quen thuộc như `Ctrl + P` (Print) cho các tính năng không liên quan.

---

## Micro-App Specific Design Protocols: Micro-App 4 — Data Lab

Khóa bất biến ngữ cảnh thiết kế cho Micro-App 4: Data Lab (Tauri v2 + Leptos 0.8 CSR WebAssembly):

### 1. Kiến trúc Bất biến (Spine Lock & Unified 3-Zone Workspace)
- **Kiểm chứng tự động:** Bắt buộc duy trì tính tương thích và pass `./scripts/verify_spine_lock.sh`.
- **Zone 1 (Header, Toolbar & Shared File Context):**
  - **Header:** Chứa `BackButton` ("← Về Launcher"), Tiêu đề ("📊 Data Lab — Phòng thí nghiệm Dữ liệu"), Huy hiệu `13 Công cụ Dữ liệu` và `🔒 100% On-Device`.
  - **DataLabToolbar (2 tầng):** Tầng 1 gồm 3 Category Tabs (`📊 Bảng tính & Excel`, `🔄 Chuyển đổi & Trộn tệp`, `📝 Văn bản & Cấu trúc`) + Tab `⚡ Chuỗi Workflow (Macro DAG)`. Tầng 2 gồm hàng Tool Chips cuộn ngang.
  - **DataLabFileListPanel (Bên trái):** Chiều rộng **cố định chính xác 290px** (`width: 290px; flex-shrink: 0;`), quản lý danh sách tệp dữ liệu dùng chung (Shared Context) cho toàn bộ 13 công cụ, tích hợp chọn Sheet cho tệp bảng tính.
- **Zone 2 (Center Workspace):**
  - Chứa **Context Banner** hiển thị tệp và sheet đang kích hoạt (hoặc Hint Card khi danh sách tệp trống).
  - Tích hợp **View Mode Switcher** và 4 chế độ hiển thị lõi:
    1. `SpreadsheetGridFacetView` (Mô hình OpenRefine: Lưới dữ liệu tương tác, Mini-profiling bar, Facet phân phối tần suất, Quick actions làm sạch, Time-travel Undo/Redo & Recipe Engine, Modal Gom cụm Fuzzy).
    2. `SplitPaneConverterView` (Mô hình DevToys / CyberChef: Bố cục 2 khung Input ⇄ Output, chuyển đổi tức thì độ trễ thấp cho Data Structure, Base64 Codec, SQL Formatter/Linter, Diff Checker).
    3. `VisualCanvasWorkflowView` (Mô hình KNIME / Alteryx DAG: Đồ thị nút luồng dữ liệu, SVG Bezier curves, Node status, Node Palette Drawer, Node Settings Modal).
    4. `Batch Settings Form` (Biểu mẫu cấu hình tham số công cụ hàng loạt).
- **Zone 3 (Chân trang - TaskExecutor):**
  - Cố định ở đáy (`TaskExecutor`), cung cấp nút Play/Run, nút Cancel phản hồi nhanh `< 100ms`, thanh tiến độ IPC phát định kỳ `200ms` qua `Channel<TaskProgress>`.

### 2. Hệ thống Design Tokens Bắt buộc (DESIGN.md & styles/data_lab.css)
- **Nền Tonal Layering:**
  - Nền ứng dụng chính: `--bg-main` (`#0B0F17`)
  - Nền bề mặt Header, Toolbar, File Panel: `--bg-surface` (`#111827`)
  - Nền thẻ tệp, card, facet item, canvas node: `--bg-card` (`#1E293B`)
- **Đường viền:**
  - Đường viền phân cách cấu trúc: `--border-subtle` (`rgba(255, 255, 255, 0.08)`)
- **Văn bản:**
  - Chữ chính, tiêu đề, số liệu: `--text-main` (`#F8FAFC`)
  - Nhãn phụ, mô tả, chú thích: `--text-muted` (`#94A3B8`)
- **Màu ngữ nghĩa chức năng:**
  - Màu nhấn chính / Active: `--brand-primary` (`#0284C7`)
  - Màu thành công / Bảo mật On-device: `--success` (`#10B981`)
  - Màu cảnh báo / Chế độ Workflow DAG: `--warning` (`#F59E0B`)
  - Màu nguy hiểm / Xóa tệp: `--danger` (`#EF4444`)
- **Bảng màu Pill định dạng tệp:**
  - `XLSX` / `XLS`: Xanh lá (`format-pill-excel`)
  - `CSV` / `TSV`: Xanh lục (`format-pill-csv`)
  - `JSON`: Cam (`format-pill-json`)
  - `SQL`: Xanh dương (`format-pill-sql`)
  - `DOCX`: Xanh biển đậm (`format-pill-docx`)
  - `TXT`: Xám (`format-pill-txt`)
- **Bảng màu Chủ đề Node DAG (Visual Canvas):**
  - `node-theme-blue`: Nguồn dữ liệu vào / Thư mục xuất ra (Input/Output).
  - `node-theme-green`: Nhóm làm sạch & gỡ khóa (Clean/Unlock).
  - `node-theme-orange`: Nhóm phân tách, gộp, nối & đối chiếu (Split/Merge/Join/Diff).
  - `node-theme-purple`: Nhóm chuyển đổi cấu trúc, PII, Base64 & Mailmerge (Convert/PII/Base64/Mailmerge).

### 3. Anti-Goals (Những điều Cấm kỵ Tuyệt đối)
1. **Cấm Hardcode Màu sắc:** Tuyệt đối không dùng mã màu HEX/RGB trực tiếp ngoài hệ thống biến CSS tokens đã định nghĩa.
2. **Cấm Phá vỡ Kích thước Layout:** Tuyệt đối không làm lệch chiều rộng cố định `290px` của `DataLabFileListPanel` hoặc gây tràn khung cuộn ngang của Interactive Data Grid.
3. **Cấm Vi phạm An toàn Gõ Tiếng Việt (IME-Safe):** Nghiêm cấm kích hoạt sự kiện lọc dữ liệu hoặc dispatch tác vụ khi bộ gõ tiếng Việt (UniKey / EVKey) đang trong trạng thái gõ dấu (`is_composing = true`).

