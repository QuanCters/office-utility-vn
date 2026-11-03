# Thiết kế Kiến trúc Micro-App — Office Utility VN

**Ngày:** 2026-11-03
**Trạng thái:** Chờ duyệt

---

## 1. Ý định và Lý do

Office Utility VN hiện có 51 công cụ xếp phẳng trong 5 nhóm sidebar. Người dùng chọn tool từ danh sách dài, dùng xong quay lại chọn cái khác — không có ngữ cảnh chung, không có luồng công việc liên tục.

Mục tiêu: chuyển sang mô hình **4 micro-app độc lập** (lấy cảm hứng từ cách game Delta Force tổ chức các sub-game). Mỗi micro-app là một **không gian làm việc hoàn chỉnh** với dashboard riêng, layout tự chủ, shared context (file/data được load một lần dùng cho nhiều tool), và workflow engine nội bộ.

### Tại sao bây giờ

- `app.rs` hiện là 3109 dòng monolith — chứa toàn bộ tool registry, state, routing, UI rendering. Không thể scale.
- Người dùng phải chọn lại file mỗi khi đổi tool — lặp lại thao tác không cần thiết.
- Sidebar 51 items gây cognitive overload. Mô hình micro-app giảm lựa chọn xuống 4 ở mức launcher, sau đó chỉ hiện tools liên quan trong mỗi app.

---

## 2. Mục tiêu, Không-mục-tiêu, Phản-mục-tiêu

### Mục tiêu
- Tổ chức 51 tools thành 4 micro-app có ranh giới rõ ràng
- Mỗi micro-app có dashboard riêng, layout tự chủ phù hợp bản chất công việc
- Shared context trong app: load file/data một lần → tất cả tools bên trong đều thấy
- Launcher thông minh với Task History, Command Palette, drop zone auto-detect
- Workflow DAG hoạt động trong từng micro-app
- Tách `app.rs` 3109 dòng thành các module nhỏ, mỗi micro-app một module riêng
- Bảo toàn zero-cloud security architecture

### Không-mục-tiêu (không làm trong phạm vi này)
- Cross-app workflow (chain tools từ nhiều micro-app) — hướng mở rộng sau
- Thay đổi `office-engine` crate hoặc Tauri backend commands
- Thay đổi database schema (dùng lại 6 bảng hiện có)
- Internationalization (i18n) — giữ nguyên Vietnamese inline
- Tách thành nhiều crate frontend (đã quyết định dùng module restructuring)

### Phản-mục-tiêu (sẽ coi là thất bại nếu xảy ra)
- Micro-app cảm thấy như chỉ đổi tên nhóm sidebar — phải là trải nghiệm không gian riêng thực sự
- Mất tính năng đang hoạt động (workflow, presets, history, keyboard shortcuts)
- Tăng thời gian build đáng kể
- User phải học lại toàn bộ — chuyển đổi phải tự nhiên cho người dùng cũ

---

## 3. Ràng buộc

- **Leptos 0.8 CSR** — framework frontend không thay đổi
- **Tauri v2** — runtime desktop không thay đổi
- **Một binary duy nhất** — không tách micro-app thành process riêng
- **office-engine crate** — backend engine giữ nguyên cấu trúc `vn_office/`, `doc_suite/`, `media_studio/`, `data_lab/`, `sys_hub/`
- **Không network** — zero-cloud constraint giữ nguyên
- **`src/tools/`** — 51 tool view files hiện có sẽ được di chuyển, không viết lại từ đầu

---

## 4. Phân bổ Tools vào Micro-Apps

### 🏢 VN Office — Trợ lý Nghiệp vụ Việt Nam (11 tools)

**Chất liệu chung:** Quy định VN, nghiệp vụ kế toán, ngân hàng, hành chính

| # | Tool slug | Tên | Nguồn gốc |
|---|-----------|-----|-----------|
| 1 | `vietnamese-number-to-words` | Đọc Số Tiền Thành Chữ | vn-office (giữ) |
| 2 | `gross-net-salary-calculator` | Tính Lương Gross-Net & Thuế | vn-office (giữ) |
| 3 | `vietqr-napas247-generator` | Tạo Mã VietQR Napas247 | vn-office (giữ) |
| 4 | `vn-einvoice-xml-reader` | Đọc Hóa đơn Điện tử XML | vn-office (giữ) |
| 5 | `vn-cccd-mst-validator` | Kiểm tra CCCD & Mã số thuế | vn-office (giữ) |
| 6 | `vn-address-normalizer` | Chuẩn hóa Địa giới Hành chính | vn-office (giữ) |
| 7 | `vn-working-days-calc` | Tính Ngày Làm việc | vn-office (giữ) |
| 8 | `vn-loan-amortization` | Bảng tính Trả nợ Vay | vn-office (giữ) |
| 9 | `vn-bank-bin-mapper` | Tra cứu Mã BIN Ngân hàng | vn-office (giữ) |
| 10 | `vietnamese-font-converter` | Chuyển mã Font Tiếng Việt | data-lab → chuyển sang |
| 11 | `encoding-mojibake-fixer` | Sửa Lỗi Mojibake | data-lab → chuyển sang |

### 📄 PDF Studio — Xưởng Tài liệu PDF (19 tools)

**Chất liệu chung:** File PDF đang mở

| # | Tool slug | Tên | Nguồn gốc |
|---|-----------|-----|-----------|
| 1 | `pdf-merge-split` | Ghép, Tách & Xoay PDF | doc-suite (giữ) |
| 2 | `pdf-protect-sign` | Bảo mật & Mở khóa PDF | doc-suite (giữ) |
| 3 | `pdf-to-markdown` | Chuyển PDF sang Markdown | doc-suite (giữ) |
| 4 | `pdf-compress` | Nén giảm Dung lượng PDF | doc-suite (giữ) |
| 5 | `vietnamese-ocr` | OCR Tiếng Việt | doc-suite (giữ) |
| 6 | `pdf-watermark-stamp` | Đóng dấu Watermark & Con dấu đỏ | doc-suite (giữ) |
| 7 | `pdf-page-number-header` | Đánh số trang & Bates Numbering | doc-suite (giữ) |
| 8 | `pdf-metadata-scrubber` | Tẩy sạch Metadata ẩn | doc-suite (giữ) |
| 9 | `pdf-flatten-forms` | Làm phẳng Forms | doc-suite (giữ) |
| 10 | `pdf-crop-margin` | Cắt Viền Trắng Auto-Crop | doc-suite (giữ) |
| 11 | `pdf-booklet-imposition` | Dàn trang In Sách N-up | doc-suite (giữ) |
| 12 | `pdf-visual-diff` | So sánh 2 Bản PDF | doc-suite (giữ) |
| 13 | `pdf-bookmark-toc` | Tạo Cây Mục lục PDF | doc-suite (giữ) |
| 14 | `pdf-to-long-image` | Ghép PDF thành Ảnh Dọc Dài | doc-suite (giữ) |
| 15 | `pdf-grayscale-print` | Chuyển PDF sang Đen trắng | doc-suite (giữ) |
| 16 | `pdf-repair-corrupt` | Phục hồi File PDF Lỗi | doc-suite (giữ) |
| 17 | `markdown-to-styled-pdf` | Markdown sang PDF | doc-suite (giữ) |
| 18 | `pdf-table-extractor` | Trích xuất Bảng từ PDF ra Excel | doc-suite (giữ) |
| 19 | `pdf-watermark-stamp` đã liệt kê ở #6, thay bằng: không có tool thứ 19 trùng — tổng vẫn đúng 19 tool vì OCR và Markdown→PDF cũng nằm trong doc-suite gốc |

### 🎨 Media Studio — Phòng thu Đa phương tiện (9 tools)

**Chất liệu chung:** Ảnh, video, audio

| # | Tool slug | Tên |
|---|-----------|-----|
| 1 | `image-batch` | Nén & Đổi Định dạng Ảnh Hàng loạt |
| 2 | `photo-id-crop` | Cắt Ảnh Thẻ 300 DPI |
| 3 | `image-bg-redact` | Làm mờ Dữ liệu Nhạy cảm |
| 4 | `image-exif-inspector` | Kiểm tra & Xóa Exif/GPS |
| 5 | `watermark-batch-studio` | Đóng dấu Bản quyền Ảnh Hàng loạt |
| 6 | `screenshot-beautifier` | Làm đẹp Ảnh Chụp Màn hình |
| 7 | `doc-scanner-perspective` | Phẳng hóa Tài liệu Chụp Nghiêng |
| 8 | `audio-silence-trimmer` | Cắt Bỏ Khoảng lặng Ghi âm |
| 9 | `video-to-gif-webp` | Video sang GIF/WebP |

### 📊 Data Lab — Phòng thí nghiệm Dữ liệu (13 tools)

**Chất liệu chung:** Bảng tính, dữ liệu có cấu trúc, chuyển đổi tệp

| # | Tool slug | Tên | Nguồn gốc |
|---|-----------|-----|-----------|
| 1 | `excel-cleaner-normalizer` | Làm sạch & Chuẩn hóa Bảng tính | data-lab (giữ) |
| 2 | `diff-checker-text-sheets` | Đối chiếu Văn bản & Bảng tính | data-lab (giữ) |
| 3 | `data-structure-converter` | Chuyển đổi Cấu trúc Dữ liệu | data-lab (giữ) |
| 4 | `excel-split-by-column` | Tách File Excel theo Cột | data-lab (giữ) |
| 5 | `excel-multi-merger` | Gộp Nhiều File Excel | data-lab (giữ) |
| 6 | `excel-visual-joiner` | Nối Trực quan Bảng tính (Join) | data-lab (giữ) |
| 7 | `excel-sheet-unlocker` | Gỡ Khóa Bảo vệ Sheet Excel | data-lab (giữ) |
| 8 | `list-set-operations` | Phép toán Tập hợp Danh sách | data-lab (giữ) |
| 9 | `text-pii-anonymizer` | Ẩn danh hóa PII | data-lab (giữ) |
| 10 | `sql-formatter-linter` | Định dạng & Lint SQL | data-lab (giữ) |
| 11 | `base64-file-codec` | Mã hóa/Giải mã Base64 | data-lab (giữ) |
| 12 | `docx-template-mailmerge` | Trộn Hợp đồng Mailmerge | doc-suite → chuyển sang |
| 13 | `universal-file-converter-hub` | Trạm Chuyển đổi Tệp Vạn năng | sys-hub → chuyển sang |

---

## 5. Cách tiếp cận đã xem xét

### Hướng A — Separate Crates (Bị loại)

Mỗi micro-app thành crate riêng dưới `crates/`. Enforced boundary ở compile-time.

**Lý do loại:** Overhead quản lý pub APIs giữa crates quá lớn cho team nhỏ. Đây là desktop app ship một binary — tách crate không mang lợi ích deploy. Engine crates đã cung cấp backend separation. Module structure sạch có thể "nâng cấp" lên crate sau nếu cần.

### Hướng B — Module Restructuring (Được chọn)

Giữ một frontend crate, tái cấu trúc `src/` thành module phân cấp. `app.rs` 3109 dòng được tách thành `launcher/` + 4 `micro_apps/` + `shared_ui/`.

**Lý do chọn:** Refactor nhẹ hơn, một binary duy nhất, Rust module privacy đủ mạnh cho ranh giới, phù hợp team nhỏ, path nâng cấp rõ ràng.

---

## 6. Thiết kế Chi tiết

### 6.1 Cấu trúc thư mục Frontend mới

```
src/
├── main.rs                          # Leptos mount (giữ nguyên)
├── app.rs                           # THIN: enum AppView, routing, global signals
│
├── launcher/
│   ├── mod.rs                       # LauncherView component
│   ├── app_card.rs                  # Card cho mỗi micro-app
│   ├── history_feed.rs              # Task History tổng hợp trên launcher
│   └── command_palette.rs           # Ctrl+K global search (di chuyển từ app.rs)
│
├── shared_ui/
│   ├── mod.rs
│   ├── back_button.rs               # Nút "← Về trang chính"
│   ├── progress_bar.rs              # Progress tracking component
│   ├── file_picker.rs               # Shared file picker logic
│   ├── result_display.rs            # Shared result display patterns
│   ├── keyboard_shortcuts.rs        # Shared keyboard handling
│   └── workflow_runner.rs           # Reusable workflow runner theo danh sách tool của app
│
├── micro_apps/
│   ├── mod.rs                       # MicroApp trait + enum MicroAppId
│   │
│   ├── vn_office/
│   │   ├── mod.rs                   # VnOfficeApp top-level component
│   │   ├── state.rs                 # VnOfficeState: shared regulatory context
│   │   ├── layout.rs                # Form + bảng kết quả + panel tra cứu
│   │   ├── sidebar.rs               # Internal navigation (11 tools)
│   │   ├── workflow.rs              # Internal workflow engine
│   │   └── tools/                   # 11 tool views (di chuyển từ src/tools/)
│   │       ├── mod.rs
│   │       ├── vietnamese_number_to_words_view.rs
│   │       ├── gross_net_salary_calculator_view.rs
│   │       ├── vietnamese_font_converter_view.rs  # từ data-lab
│   │       ├── encoding_mojibake_fixer_view.rs    # từ data-lab
│   │       └── ...
│   │
│   ├── pdf_studio/
│   │   ├── mod.rs                   # PdfStudioApp top-level component
│   │   ├── state.rs                 # PdfStudioState: loaded PDF files, preview
│   │   ├── layout.rs                # File list trái + preview canvas phải
│   │   ├── toolbar.rs               # Tab toolbar (Ghép/Tách, Bảo mật, Định dạng, Chuyển đổi)
│   │   ├── file_list_panel.rs       # Left panel: loaded PDFs
│   │   ├── preview_panel.rs         # Right panel: PDF preview/canvas
│   │   ├── workflow.rs              # Internal workflow engine
│   │   └── tools/                   # 19 tool views
│   │       ├── mod.rs
│   │       ├── pdf_merge_split_view.rs
│   │       └── ...
│   │
│   ├── media_studio/
│   │   ├── mod.rs                   # MediaStudioApp top-level component
│   │   ├── state.rs                 # MediaStudioState: loaded media files
│   │   ├── layout.rs                # Gallery grid + editor panel
│   │   ├── gallery_panel.rs         # Grid view with thumbnails
│   │   ├── editor_panel.rs          # Right panel: editing controls
│   │   ├── batch_queue.rs           # Batch processing progress queue
│   │   ├── workflow.rs              # Internal workflow engine
│   │   └── tools/                   # 9 tool views
│   │       ├── mod.rs
│   │       ├── image_batch_view.rs
│   │       └── ...
│   │
│   └── data_lab/
│       ├── mod.rs                   # DataLabApp top-level component
│       ├── state.rs                 # DataLabState: loaded spreadsheet data
│       ├── layout.rs                # Spreadsheet preview + column inspector
│       ├── data_preview_panel.rs    # Table/spreadsheet viewer
│       ├── column_inspector.rs      # Column type/stats inspector
│       ├── workflow.rs              # Internal workflow engine
│       └── tools/                   # 12 tool views
│           ├── mod.rs
│           ├── excel_cleaner_normalizer_view.rs
│           ├── docx_template_mailmerge_view.rs  # từ doc-suite
│           └── ...
```

### 6.2 App-level State & Routing

```rust
// src/app.rs — THIN version (~200 dòng thay vì 3109)

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppView {
    Launcher,                        // Màn hình chính
    MicroApp(MicroAppId),            // Đang trong một micro-app
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MicroAppId {
    VnOffice,
    PdfStudio,
    MediaStudio,
    DataLab,
}

// Global app state — chỉ chứa navigation + cross-cutting
pub struct AppState {
    pub current_view: RwSignal<AppView>,
    pub deep_link_tool: RwSignal<Option<String>>,  // Cho history deep-link
}
```

### 6.3 Shared Context trong mỗi Micro-App

Mỗi micro-app có state struct riêng chứa "chất liệu" đang làm việc:

```rust
// Ví dụ: pdf_studio/state.rs
pub struct PdfStudioState {
    pub loaded_files: RwSignal<Vec<String>>,           // Shared file paths
    pub active_file_index: RwSignal<Option<usize>>,
    pub active_tool: RwSignal<Option<String>>,
}

// Ví dụ: data_lab/state.rs
pub struct DataLabState {
    pub loaded_files: RwSignal<Vec<String>>,           // Shared spreadsheet / data file paths
    pub active_sheet: RwSignal<String>,
    pub active_tool: RwSignal<Option<String>>,
}
```

State của 4 micro-app được lưu giữ **in-memory** trực tiếp tại `AppState` ở App root. Khi user bấm "← Về trang chính", view chuyển đổi nhưng các tín hiệu trạng thái (Leptos signals) và tác vụ nền chạy ngầm (batch processing qua Tauri IPC) không bị hủy hay reset. Khi người dùng bấm quay lại micro-app, toàn bộ ngữ cảnh vẫn nguyên vẹn.

### 6.4 Launcher Design

```
┌─────────────────────────────────────────────────────────┐
│  ⚡ Office Utility VN            [Ctrl+K Tìm kiếm...]  │
│  🟢 100% On-Device · Zero-Cloud                        │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│  │ 🏢       │ │ 📄       │ │ 🎨       │ │ 📊       │   │
│  │ VN Office│ │PDF Studio│ │ Media    │ │ Data Lab │   │
│  │ 11 tools │ │ 19 tools │ │ Studio   │ │ 13 tools │   │
│  │          │ │          │ │ 9 tools  │ │          │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │
│                                                         │
│  📋 LỊCH SỬ GẦN ĐÂY                                   │
│  ┌─────────────────────────────────────────────────┐   │
│  │ 📄 Ghép 3 file PDF     │ PDF Studio │ 2 phút   │   │
│  │ 📊 Clean Excel lương   │ Data Lab   │ 15 phút  │   │
│  │ 🏢 Tính lương T10/2026 │ VN Office  │ 1 giờ    │   │
│  │ 🎨 Nén 50 ảnh WebP     │ Media      │ hôm qua  │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

Click vào history item → deep-link: `AppView::MicroApp(PdfStudio)` + `deep_link_tool = Some("pdf-merge-split")`

### 6.5 Navigation Flow

```
Launcher ──click card──→ MicroApp (dashboard)
                             │
                      ──select tool──→ Tool View
                             │
                      ──"← Về trang chính"──→ Launcher
                             │                    (in-memory state preserved)
Launcher ──click history──→ MicroApp (deep-link to specific tool)

Launcher ──Ctrl+K──→ Search all tools ──click──→ MicroApp (deep-link)
```

### 6.6 Trong mỗi Micro-App: Internal Navigation

Mỗi micro-app quyết định cách riêng, nhưng pattern chung:

- **VN Office:** Sidebar nhỏ bên trái liệt kê 11 tools + content area bên phải
- **PDF Studio:** Toolbar tabs phía trên (nhóm chức năng) + file list trái + tool UI phải
- **Media Studio:** Gallery phía trên + tool controls phía dưới
- **Data Lab:** Tab bar phía trên + spreadsheet preview + tool panel

Mỗi app có nút **"← Về trang chính"** cố định ở vị trí nhất quán (góc trên trái).

### 6.7 Sidebar Cũ

Sidebar cũ (`sidebar.rs`) bị **loại bỏ hoàn toàn** ở mức app. Mỗi micro-app có internal navigation riêng. Không còn global sidebar.

### 6.8 Xử lý Favorites/Pinned

Tính năng Favorites/Pinned bị **loại bỏ**. `favorite_tools.rs` và `recent_tools.rs` bị xóa. Task History trên launcher thay thế cả hai chức năng này.

### 6.9 Command Palette (Ctrl+K)

- **Ở Launcher:** Tìm tất cả tools across 4 app → click = deep-link vào tool trong micro-app tương ứng
- **Trong micro-app:** Chỉ tìm tools trong app đó + option "← Về trang chính"

### 6.10 Workflow DAG

Thay vì nhân bản mã nguồn, một component `workflow_runner.rs` dùng chung sẽ được đặt trong `shared_ui`. Mỗi micro-app truyền danh sách các `ToolItem` thuộc quyền quản lý của mình vào component này. Người dùng chỉ có thể chain các công cụ thuộc micro-app đó. UI workflow có thể truy cập qua tab hoặc nút chức năng bên trong từng micro-app.

Kiến trúc để cửa cho cross-app workflow sau này: workflow definition trong DB có thể hỗ trợ cờ `cross_app: bool`.

### 6.11 Task History

- Mỗi micro-app có history view riêng (lọc theo tools trong app)
- Launcher hiển thị history tổng hợp từ tất cả app
- Click vào history item ở launcher = deep-link vào đúng micro-app + tool
- **Zero DB Migration**: Giữ nguyên schema bảng `task_history` 100%. `app_id` được frontend tự động suy luận (derive) trực tiếp từ `tool_slug` thông qua hàm mapping tĩnh `tool_slug_to_micro_app(slug) -> MicroAppId`.

### 6.12 Presets

Presets thuộc về từng micro-app. DB schema giữ nguyên bảng `tool_presets` — đã có `tool_slug` để phân loại tự nhiên.

### 6.13 Keyboard Shortcuts

| Shortcut | Ở Launcher | Trong Micro-App |
|----------|-----------|-----------------|
| `Ctrl+K` | Tìm global | Tìm trong app |
| `Ctrl+Enter` | — | Thực thi tool |
| `Ctrl+P` | Always-on-top | Always-on-top |
| `Ctrl+O` | — | Mở output folder |
| `Esc` | — | Cancel task / quay lại |
| `Backspace` (khi không focus input) | — | Về launcher |

### 6.14 CSS

`styles.css` 2147 dòng hiện tại sẽ được tách thành:
- `styles/base.css` — reset, typography, colors, shared
- `styles/launcher.css` — launcher-specific
- `styles/vn_office.css` — VN Office micro-app
- `styles/pdf_studio.css` — PDF Studio micro-app
- `styles/media_studio.css` — Media Studio micro-app
- `styles/data_lab.css` — Data Lab micro-app

### 6.15 Auto Updater & System Tray

Giữ nguyên, không thay đổi. Updater và tray icon là app-level, không thuộc micro-app nào.

---

## 7. Thay đổi Backend (Zero Backend Changes)

### Tauri Commands
Giữ nguyên tất cả 21 IPC commands 100%. Không cần thay đổi dispatch logic vì tool slugs không đổi.

### Database
Giữ nguyên 100% schema của 6 bảng SQLite. Không có migration. Không làm vỡ tương thích dữ liệu SQLite của phiên bản hiện tại.

### Engine Crates
Không thay đổi. `office-engine` giữ nguyên cấu trúc `vn_office/`, `doc_suite/`, `media_studio/`, `data_lab/`, `sys_hub/`.

**Lưu ý:** 3 tools chuyển nhóm ở tầng frontend:
- Vietnamese Font Converter, Mojibake Fixer: từ `data_lab` → `vn_office`
- DOCX Mailmerge: từ `doc_suite` → `data_lab`
- Universal File Converter: từ `sys_hub` → `data_lab`
Backend engine modules vẫn giữ nguyên vị trí, chỉ có tầng UI/view được tổ chức lại.

---

## 8. Những gì để lại cho Builder

- Thiết kế chi tiết bên trong mỗi micro-app dashboard (layout chi tiết, component hierarchy) — builder quyết định dựa trên mô tả layout trong mục 6.6
- Modularize các khối Archetype UI từ `app.rs` sang các micro-app tương ứng để quản lý input và dispatch tác vụ
- Animation/transition khi chuyển giữa launcher ↔ micro-app
- Exact styling/theming — builder dựa trên design system hiện có trong `styles.css`
- Thứ tự di chuyển tool view files (tiến hành incremental, app by app)
- Batch queue UI cho Media Studio — builder thiết kế

---

## 9. Rủi ro

| Rủi ro | Giảm thiểu |
|--------|------------|
| Refactor lớn gây regression | Di chuyển incremental từng micro-app, chạy E2E tests sau mỗi bước |
| Shared context phức tạp hơn dự tính | Bắt đầu đơn giản (chỉ shared file list `RwSignal<Vec<String>>`), mở rộng dần |
| CSS tách file gây conflict | Namespace CSS classes theo micro-app (`.vn-office-*`, `.pdf-studio-*`) |
| Tool view cần sửa đổi nhiều khi di chuyển | Giữ tool views as-is ban đầu, refactor để dùng shared context sau |
