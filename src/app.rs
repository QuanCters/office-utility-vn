use leptos::ev::{KeyboardEvent, MouseEvent};
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::launcher::LauncherView;
use crate::micro_apps::{
    DataLabApp, DataLabState, MediaStudioApp, MediaStudioState, MicroAppId, PdfStudioApp,
    PdfStudioState, VnOfficeApp, VnOfficeState, gridcraft::GridCraftState, tool_slug_to_micro_app,
    vn_office::VnLegalRegulations,
};
use crate::platform::window_close::{WindowCloseDecision, WindowCloseManager};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolItem {
    pub slug: &'static str,
    pub name: &'static str,
    pub domain: &'static str,
    pub description: &'static str,
    pub is_pinned: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TaskHistoryUi {
    pub id: String,
    pub tool_slug: Option<String>,
    pub workflow_id: Option<String>,
    pub status: String,
    pub elapsed_ms: u64,
    pub created_at: String,
    pub error_message: Option<String>,
}

pub const ALL_TOOLS: &[ToolItem] = &[
    ToolItem {
        slug: "vietnamese-number-to-words",
        name: "Đọc Số Tiền Thành Chữ Chuẩn",
        domain: "vn-office",
        description: "Chuyển đổi số tiền thành chữ tiếng Việt chuẩn xác theo quy tắc kế toán.",
        is_pinned: true,
    },
    ToolItem {
        slug: "gross-net-salary-calculator",
        name: "Tính Lương Gross - Net & Thuế",
        domain: "vn-office",
        description: "Tính lương Gross sang Net và đối soát thuế TNCN, BHXH theo luật mới.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vietqr-napas247-generator",
        name: "Tạo Mã VietQR Napas247",
        domain: "vn-office",
        description: "Tạo mã QR thanh toán Napas247 chuẩn EMVCo kèm logo ngân hàng.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vn-einvoice-xml-reader",
        name: "Đọc Hóa đơn Điện tử XML VN",
        domain: "vn-office",
        description: "Đọc, bóc tách và đối soát dữ liệu hóa đơn điện tử VAT chuẩn Tổng cục Thuế.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vn-cccd-mst-validator",
        name: "Kiểm tra Hợp lệ CCCD & Mã số thuế",
        domain: "vn-office",
        description: "Giải mã cấu trúc CCCD 12 số và kiểm tra tính hợp lệ Mã số thuế bằng Modulo 11.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vn-address-normalizer",
        name: "Chuẩn hóa Địa giới Hành chính VN",
        domain: "vn-office",
        description: "Khớp chuỗi mờ Jaro-Winkler chuẩn hóa địa chỉ theo mô hình 2 cấp và 3 cấp.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vn-working-days-calc",
        name: "Tính Ngày Làm việc Trừ Ngày lễ VN",
        domain: "vn-office",
        description: "Tính số ngày làm việc trừ ngày nghỉ lễ theo Bộ luật Lao động và xuất tệp .ics.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vn-loan-amortization",
        name: "Bảng tính Trả nợ Vay Ngân hàng",
        domain: "vn-office",
        description: "Tính lịch trả nợ vay theo dư nợ giảm dần hoặc niên kim cố định chính xác từng đồng.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vn-bank-bin-mapper",
        name: "Tra cứu Mã Định danh Ngân hàng",
        domain: "vn-office",
        description: "Tra cứu mã BIN Napas, CITAD và SWIFT code hơn 60 ngân hàng Việt Nam.",
        is_pinned: true,
    },
    ToolItem {
        slug: "pdf-merge-split",
        name: "Ghép, Tách & Xoay trang PDF",
        domain: "doc-suite",
        description: "Ghép nhiều file PDF hoặc tách trang theo dải lựa chọn trực quan trên PDFCraft Canvas.",
        is_pinned: true,
    },
    ToolItem {
        slug: "pdf-protect-sign",
        name: "Bảo mật & Mở khóa PDF",
        domain: "doc-suite",
        description: "Áp dụng mã hóa AES-256 hoặc mở khóa mật khẩu PDF trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-to-markdown",
        name: "Chuyển đổi PDF sang Markdown",
        domain: "doc-suite",
        description: "Bóc tách luồng văn bản, font size và tái tạo bảng biểu sang cú pháp Markdown.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-compress",
        name: "Nén giảm Dung lượng PDF",
        domain: "doc-suite",
        description: "Nén lại các luồng FlateDecode và tối ưu dung lượng trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "vietnamese-ocr",
        name: "OCR Nhận dạng Ký tự Tiếng Việt",
        domain: "doc-suite",
        description: "Nhận dạng chữ tiếng Việt có dấu từ tài liệu scan hoặc ảnh bằng OS Native OCR hoặc VietOCR.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-watermark-stamp",
        name: "Đóng dấu Watermark & Con dấu đỏ",
        domain: "doc-suite",
        description: "Chèn luồng đồ họa ExtGState chứa watermark bản quyền hoặc nhúng con dấu đỏ PNG.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-page-number-header",
        name: "Đánh số trang & Bates Numbering",
        domain: "doc-suite",
        description: "Đánh số trang tự động, Bates Numbering và tùy biến căn lề header/footer.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-metadata-scrubber",
        name: "Tẩy sạch Metadata ẩn PDF",
        domain: "doc-suite",
        description: "Xóa sạch Trailer Info và luồng XML metadata XMP trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-flatten-forms",
        name: "Làm phẳng PDF (Flatten Forms)",
        domain: "doc-suite",
        description: "Làm phẳng biểu mẫu AcroForm thành văn bản tĩnh trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-crop-margin",
        name: "Cắt Viền Trắng Thừa (Auto-Crop)",
        domain: "doc-suite",
        description: "Tìm hộp giới hạn Bounding Box và cắt lề trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-booklet-imposition",
        name: "Dàn trang In Sách Gấp đôi & N-up",
        domain: "doc-suite",
        description: "Tính toán ma trận biến đổi cm và sắp xếp lại thứ tự trang theo thuật toán đóng sách lồng Saddle-Stitch.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-visual-diff",
        name: "So sánh Đối soát 2 Bản PDF",
        domain: "doc-suite",
        description: "Hỗ trợ Text Diff và Visual Pixel Diff trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "docx-template-mailmerge",
        name: "Trộn Hợp đồng Mailmerge Hàng loạt",
        domain: "doc-suite",
        description: "Giải nén luồng XML, hợp nhất thẻ <w:r> và trộn hàng loạt file Word tốc độ cao.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-bookmark-toc",
        name: "Tạo Cây Mục lục PDF (Bookmark TOC)",
        domain: "doc-suite",
        description: "Khởi tạo và liên kết cấu trúc cây từ điển Outlines trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-to-long-image",
        name: "Ghép PDF thành Ảnh Dọc Dài",
        domain: "doc-suite",
        description: "Kết xuất từng trang PDF và ghép nối tiếp theo chiều dọc thành một ảnh dài duy nhất.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-grayscale-print",
        name: "Chuyển PDF sang Đen trắng",
        domain: "doc-suite",
        description: "Chuyển đổi toàn bộ toán tử màu sang DeviceGray 8-bit tiết kiệm mực in.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-repair-corrupt",
        name: "Phục hồi File PDF Lỗi Cấu trúc",
        domain: "doc-suite",
        description: "Quét tuyến tính toàn bộ byte để tìm obj/endobj và tái tạo lại bảng xref bị hỏng.",
        is_pinned: false,
    },
    ToolItem {
        slug: "markdown-to-styled-pdf",
        name: "Biên dịch Markdown sang PDF",
        domain: "doc-suite",
        description: "Phân tích Markdown AST, kết xuất PDF khổ A4 với Unicode tiếng Việt tự động ngắt trang.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-table-extractor",
        name: "Trích xuất Bảng từ PDF ra Excel",
        domain: "doc-suite",
        description: "Phân tích tọa độ (x, y) cụm từ và đường kẻ vector để khôi phục bảng biểu ra file .xlsx.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-form-manager",
        name: "Quản lý & Điền Biểu mẫu AcroForm",
        domain: "doc-suite",
        description: "Khảo sát trường AcroForm và tự động điền form trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-redaction",
        name: "Xóa Dữ liệu Nhạy cảm PDF",
        domain: "doc-suite",
        description: "Cắt bỏ triệt để chuỗi ký tự nhạy cảm và che đen trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-archive-linearize",
        name: "Chuẩn hóa Lưu trữ PDF/A & Fast Web View",
        domain: "doc-suite",
        description: "Chuẩn hóa PDF/A và tuyến tính hóa tải nhanh trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-pki-sign",
        name: "Chữ ký số PKI & Xác thực Toàn vẹn",
        domain: "doc-suite",
        description: "Gắn chữ ký số chuẩn PKCS#7 với /ByteRange trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-page-organizer",
        name: "Sắp xếp & Đảo trang PDF Trực quan",
        domain: "doc-suite",
        description: "Trực quan hóa danh sách trang, xoay góc và đổi thứ tự trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-signature-pad",
        name: "Ký tay Điện tử & Dán Chữ ký PDF",
        domain: "doc-suite",
        description: "Ký tay tự do trên canvas và dán chữ ký số trực quan trên PDFCraft Canvas.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-to-images-pack",
        name: "Xuất Toàn bộ Trang PDF sang Gói Ảnh ZIP",
        domain: "doc-suite",
        description: "Kết xuất từng trang PDF thành ảnh PNG/JPEG chất lượng cao và đóng gói thành file ZIP tiện lợi.",
        is_pinned: false,
    },
    ToolItem {
        slug: "image-batch",
        name: "Nén & Đổi Định dạng Ảnh Hàng loạt",
        domain: "media-studio",
        description: "Tối ưu hóa ảnh WebP, AVIF, JPEG đa luồng với Rayon.",
        is_pinned: true,
    },
    ToolItem {
        slug: "photo-id-crop",
        name: "Cắt Ảnh Thẻ Chuẩn 300 DPI",
        domain: "media-studio",
        description: "Cắt ảnh thẻ theo tỷ lệ chuẩn 3x4, 4x6 và dàn trang khổ in 10x15 cm ở 300 DPI.",
        is_pinned: true,
    },
    ToolItem {
        slug: "image-bg-redact",
        name: "Làm mờ Dữ liệu Nhạy cảm Giấy tờ",
        domain: "media-studio",
        description: "Che mờ hoặc tô đen vĩnh viễn vùng thông tin nhạy cảm (CCCD, mã QR) trên giấy tờ.",
        is_pinned: true,
    },
    ToolItem {
        slug: "image-exif-inspector",
        name: "Kiểm tra & Xóa Exif/GPS Ảnh",
        domain: "media-studio",
        description: "Đọc thông số máy ảnh, tọa độ GPS và tước bỏ siêu dữ liệu bảo vệ riêng tư.",
        is_pinned: true,
    },
    ToolItem {
        slug: "watermark-batch-studio",
        name: "Đóng dấu Bản quyền Ảnh Hàng loạt",
        domain: "media-studio",
        description: "Chèn logo hoặc chữ bản quyền dạng lưới lặp lại (Tiled) hoặc ở góc cố định.",
        is_pinned: true,
    },
    ToolItem {
        slug: "screenshot-beautifier",
        name: "Làm đẹp Ảnh Chụp Màn hình",
        domain: "media-studio",
        description: "Kết xuất khung viền macOS/Windows sắc nét và hiệu ứng đổ bóng mềm trên nền Gradient.",
        is_pinned: true,
    },
    ToolItem {
        slug: "doc-scanner-perspective",
        name: "Phẳng hóa Tài liệu Chụp Nghiêng",
        domain: "media-studio",
        description: "Nắn phẳng góc nghiêng 4 điểm phối cảnh và tăng tương phản chữ đen trên nền trắng.",
        is_pinned: true,
    },
    ToolItem {
        slug: "audio-silence-trimmer",
        name: "Cắt Bỏ Khoảng lặng Ghi âm",
        domain: "media-studio",
        description: "Phân tích RMS năng lượng âm thanh và loại bỏ các đoạn im lặng kéo dài.",
        is_pinned: true,
    },
    ToolItem {
        slug: "video-to-gif-webp",
        name: "Chuyển Đổi Video Ngắn sang GIF / WebP",
        domain: "media-studio",
        description: "Chuyển đổi video clip, cắt trích xuất khung hình và lượng tử hóa bảng màu động.",
        is_pinned: true,
    },
    ToolItem {
        slug: "universal-file-converter-hub",
        name: "Trạm Chuyển đổi Tệp Vạn năng",
        domain: "sys-hub",
        description: "Tự động nhận diện chữ ký file và chuyển đổi định dạng linh hoạt.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vietnamese-font-converter",
        name: "Chuyển mã Font Tiếng Việt",
        domain: "data-lab",
        description: "Chuyển mã font TCVN3 (ABC), VNI sang Unicode tổ hợp và dựng sẵn.",
        is_pinned: false,
    },
    ToolItem {
        slug: "excel-cleaner-normalizer",
        name: "Làm sạch & Chuẩn hóa Bảng tính",
        domain: "data-lab",
        description: "Chuẩn hóa định dạng ngày tháng, viết hoa tên riêng, tách họ tên và loại bỏ dòng trùng lặp.",
        is_pinned: false,
    },
    ToolItem {
        slug: "diff-checker-text-sheets",
        name: "Đối chiếu Văn bản & Bảng tính",
        domain: "data-lab",
        description: "Thuật toán Myers Diff tính toán và tô màu trực quan từng từ, dòng bị sai lệch.",
        is_pinned: false,
    },
    ToolItem {
        slug: "data-structure-converter",
        name: "Chuyển đổi Cấu trúc Dữ liệu",
        domain: "data-lab",
        description: "Chuyển đổi hai chiều tốc độ cao giữa bảng tính với JSON, SQL INSERT INTO hoặc Markdown.",
        is_pinned: false,
    },
    ToolItem {
        slug: "excel-split-by-column",
        name: "Tách File Excel theo Cột",
        domain: "data-lab",
        description: "Phân nhóm dữ liệu theo giá trị của cột chỉ định và xuất thành các file riêng biệt.",
        is_pinned: false,
    },
    ToolItem {
        slug: "excel-multi-merger",
        name: "Gộp Nhiều File Excel / Sheet",
        domain: "data-lab",
        description: "Gộp nối tiếp dòng hoặc gộp thành nhiều Sheet trong một Workbook kèm truy vết nguồn.",
        is_pinned: false,
    },
    ToolItem {
        slug: "excel-visual-joiner",
        name: "Nối Trực quan Bảng tính (Join)",
        domain: "data-lab",
        description: "Thực hiện Hash Join (INNER, LEFT, FULL, ANTI) giữa hai bảng tính theo cột khóa.",
        is_pinned: false,
    },
    ToolItem {
        slug: "excel-sheet-unlocker",
        name: "Gỡ Khóa Bảo vệ Chỉnh sửa Sheet Excel",
        domain: "data-lab",
        description: "Loại bỏ thẻ khóa bảo vệ sheetProtection trong OpenXML để cho phép chỉnh sửa toàn diện.",
        is_pinned: false,
    },
    ToolItem {
        slug: "list-set-operations",
        name: "Phép toán Tập hợp Danh sách",
        domain: "data-lab",
        description: "Thực hiện phép toán tập hợp giao, hợp, hiệu và hiệu đối xứng cho các danh sách dòng.",
        is_pinned: false,
    },
    ToolItem {
        slug: "encoding-mojibake-fixer",
        name: "Sửa Lỗi Hiển thị Chữ Mojibake",
        domain: "data-lab",
        description: "Khôi phục chuỗi văn bản tiếng Việt bị lỗi hiển thị bảng mã UTF-8 và Windows-1252.",
        is_pinned: false,
    },
    ToolItem {
        slug: "text-pii-anonymizer",
        name: "Ẩn danh hóa Dữ liệu Nhạy cảm PII",
        domain: "data-lab",
        description: "Quét và ẩn tự động số CCCD, SĐT di động VN, MST, STK ngân hàng thành nhãn ẩn danh.",
        is_pinned: false,
    },
    ToolItem {
        slug: "sql-formatter-linter",
        name: "Định dạng & Linter Câu lệnh SQL",
        domain: "data-lab",
        description: "Kiểm tra lỗi cú pháp AST và căn chỉnh thụt lề, viết hoa từ khóa theo phương ngữ SQL.",
        is_pinned: false,
    },
    ToolItem {
        slug: "base64-file-codec",
        name: "Mã hóa / Giải mã Base64 cho File",
        domain: "data-lab",
        description: "Mã hóa/giải mã theo luồng Streaming Base64 Codec giữa tệp nhị phân và chuỗi Base64.",
        is_pinned: false,
    },
];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: Option<String>,
    pub release_date: Option<String>,
    pub should_prompt: bool,
}

#[derive(Serialize, Deserialize)]
#[allow(dead_code)]
struct CheckUpdateArgs {
    manual: bool,
}

#[derive(Serialize, Deserialize)]
#[allow(dead_code)]
struct SetUpdateActionArgs {
    action: String,
    target_version: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[allow(dead_code)]
struct AlwaysOnTopArgs {
    enabled: bool,
}

/// Trạng thái định tuyến view toàn cục của ứng dụng.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppView {
    Launcher,
    MicroApp(MicroAppId),
    #[cfg(debug_assertions)]
    GridCraftDev,
}

/// Trạng thái toàn cục kết nối các Micro-Apps độc lập với Launcher trung tâm.
#[derive(Clone, Copy, Debug)]
pub struct AppState {
    pub current_view: RwSignal<AppView>,
    pub deep_link_tool: RwSignal<Option<String>>,
    pub vn_office_state: VnOfficeState,
    pub pdf_studio_state: PdfStudioState,
    pub media_studio_state: MediaStudioState,
    pub data_lab_state: DataLabState,
    pub gridcraft_state: GridCraftState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_view: RwSignal::new(AppView::Launcher),
            deep_link_tool: RwSignal::new(None),
            vn_office_state: VnOfficeState::default(),
            pdf_studio_state: PdfStudioState::default(),
            media_studio_state: MediaStudioState::default(),
            data_lab_state: DataLabState::default(),
            gridcraft_state: GridCraftState::new(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Kiểm tra xem ứng dụng có bất kỳ tài liệu nào chưa lưu hoặc tác vụ đang chạy dở dang hay không.
    pub fn has_unsaved_work_or_running_task(&self) -> bool {
        // 1. GridCraft dirty state OR in-flight pending native write (P1 #4690659358):
        // User may undo edits to reach a "clean" state while a native save is still in-flight.
        // is_dirty becomes false but the write is not done — closing would interrupt persistence.
        if self.gridcraft_state.is_dirty.get_untracked() || self.gridcraft_state.has_pending_saves()
        {
            return true;
        }
        // 2. PDF Studio unsaved changes, inactive dirty documents, hoặc đang có pending saves (Finding 4675800826)
        if self
            .pdf_studio_state
            .has_unsaved_documents_or_pending_saves()
            || self.pdf_studio_state.is_running.get_untracked()
        {
            return true;
        }
        // 3. Data Lab có tệp mở hoặc đang chạy
        if self.data_lab_state.has_files() || self.data_lab_state.is_running.get_untracked() {
            return true;
        }
        // 4. VnOffice hoặc MediaStudio đang chạy tác vụ
        if self.vn_office_state.is_running.get_untracked()
            || self.media_studio_state.is_running.get_untracked()
            || self.media_studio_state.batch_items_count() > 0
        {
            return true;
        }
        false
    }

    /// Tóm tắt các tác vụ / dữ liệu chưa lưu thành chuỗi văn bản thân thiện với người dùng
    pub fn unsaved_work_summary(&self) -> String {
        let mut parts = Vec::new();
        let gc_dirty = self.gridcraft_state.is_dirty.get_untracked();
        let gc_pending = self.gridcraft_state.has_pending_saves();
        if gc_dirty || gc_pending {
            let name = self
                .gridcraft_state
                .active_file_name
                .get_untracked()
                .unwrap_or_else(|| "Bảng tính GridCraft".to_string());
            if gc_pending && !gc_dirty {
                // In-flight write but workbook reverted to clean state (P1 #4690659358)
                parts.push(format!("Bảng tính ({name}) (đang lưu)"));
            } else {
                parts.push(format!("Bảng tính ({name})"));
            }
        }
        if self
            .pdf_studio_state
            .has_unsaved_documents_or_pending_saves()
        {
            if self.pdf_studio_state.has_pending_saves() {
                parts.push("Tài liệu PDF Studio (đang lưu)".to_string());
            } else {
                parts.push("Tài liệu PDF Studio".to_string());
            }
        }
        if self.pdf_studio_state.is_running.get_untracked() {
            parts.push("Tác vụ xử lý PDF".to_string());
        }
        if self.data_lab_state.has_files() {
            parts.push("Tệp dữ liệu Data Lab".to_string());
        }
        if self.data_lab_state.is_running.get_untracked() {
            parts.push("Tác vụ phân tích Data Lab".to_string());
        }
        if self.vn_office_state.is_running.get_untracked() {
            parts.push("Tác vụ tính toán Văn phòng VN".to_string());
        }
        if self.media_studio_state.is_running.get_untracked()
            || self.media_studio_state.batch_items_count() > 0
        {
            parts.push("Tác vụ xử lý Media Studio".to_string());
        }
        if parts.is_empty() {
            "Dữ liệu chưa lưu".to_string()
        } else {
            parts.join(", ")
        }
    }

    /// Trả về AppView hiện tại (untracked).
    pub fn current_view(&self) -> AppView {
        self.current_view.get_untracked()
    }

    /// Trả về slug công cụ deep-link hiện tại (untracked).
    pub fn deep_link_tool(&self) -> Option<String> {
        self.deep_link_tool.get_untracked()
    }

    /// Điều hướng sang một Micro-App cụ thể.
    pub fn navigate_to(&self, app_id: MicroAppId) {
        self.deep_link_tool.set(None);
        if app_id == MicroAppId::PdfStudio {
            self.pdf_studio_state.open_visual_workspace();
        }
        self.current_view.set(AppView::MicroApp(app_id));
    }

    /// Kích hoạt không gian làm việc thử nghiệm GridCraft (Stage 4A Development-Only Gate).
    /// Hoàn toàn bị loại bỏ trong bản dựng Release.
    #[cfg(debug_assertions)]
    pub fn open_gridcraft_dev(&self) {
        self.deep_link_tool.set(None);
        self.navigate_to(MicroAppId::DataLab);
    }

    /// Kích hoạt deep-link trực tiếp đến một công cụ bên trong một Micro-App.
    pub fn deep_link(&self, app_id: MicroAppId, tool_slug: impl Into<String>) {
        let slug = tool_slug.into();
        match app_id {
            MicroAppId::VnOffice => self.vn_office_state.select_tool(&slug),
            MicroAppId::PdfStudio => {
                self.pdf_studio_state.select_tool(&slug);
                self.pdf_studio_state.open_visual_workspace();
            }
            MicroAppId::MediaStudio => self.media_studio_state.select_tool(&slug),
            MicroAppId::DataLab => self.data_lab_state.select_tool(&slug),
        }
        self.deep_link_tool.set(Some(slug));
        self.current_view.set(AppView::MicroApp(app_id));
    }

    /// Kích hoạt deep-link dựa trên slug công cụ bằng cách tự động suy luận MicroAppId.
    pub fn deep_link_by_slug(&self, slug: &str) -> bool {
        if let Some(app_id) = tool_slug_to_micro_app(slug) {
            self.deep_link(app_id, slug);
            true
        } else {
            false
        }
    }

    /// Quay trở về màn hình Launcher trung tâm.
    pub fn back_to_launcher(&self) {
        self.deep_link_tool.set(None);
        self.current_view.set(AppView::Launcher);
    }

    /// Kiểm tra xem Micro-App cụ thể có tác vụ / dữ liệu đang làm việc hay không.
    pub fn has_active_work(&self, app_id: MicroAppId) -> bool {
        match app_id {
            MicroAppId::VnOffice => {
                self.vn_office_state.is_running()
                    || self
                        .vn_office_state
                        .calculation_history
                        .with_untracked(|h| !h.is_empty())
            }
            MicroAppId::PdfStudio => {
                self.pdf_studio_state.is_running() || self.pdf_studio_state.has_files()
            }
            MicroAppId::MediaStudio => {
                self.media_studio_state.is_running()
                    || self.media_studio_state.batch_items_count() > 0
                    || self.media_studio_state.has_files()
            }
            MicroAppId::DataLab => {
                self.gridcraft_state.is_dirty.get_untracked()
                    || self.data_lab_state.is_running()
                    || self.data_lab_state.has_files()
            }
        }
    }

    /// Khôi phục trạng thái bộ nhớ của một Micro-App về mặc định.
    pub fn reset_app_state(&self, app_id: MicroAppId) {
        match app_id {
            MicroAppId::VnOffice => {
                self.vn_office_state
                    .select_tool("vietnamese-number-to-words");
                self.vn_office_state.clear_calculation_history();
                self.vn_office_state
                    .regulations
                    .set(VnLegalRegulations::default());
                self.vn_office_state.sidebar_filter.set(String::new());
                self.vn_office_state.is_legal_panel_open.set(true);
                self.vn_office_state.is_running.set(false);
                self.vn_office_state.status_message.set(String::new());
            }
            MicroAppId::PdfStudio => {
                self.pdf_studio_state.clear_files();
                self.pdf_studio_state.select_tool("pdf-merge-split");
                self.pdf_studio_state.is_running.set(false);
                self.pdf_studio_state.status_message.set(String::new());
            }
            MicroAppId::MediaStudio => {
                self.media_studio_state.clear_files();
                self.media_studio_state.clear_batch_queue();
                self.media_studio_state.select_tool("image-batch");
                self.media_studio_state.is_running.set(false);
                self.media_studio_state.status_message.set(String::new());
                self.media_studio_state.is_queue_expanded.set(false);
            }
            MicroAppId::DataLab => {
                self.data_lab_state.clear_files();
                self.data_lab_state.select_tool("excel-cleaner-normalizer");
                self.data_lab_state.set_sheet(String::new());
                self.data_lab_state.available_sheets.set(Vec::new());
                self.data_lab_state.is_running.set(false);
                self.data_lab_state.status_message.set(String::new());
            }
        }
    }

    /// Đánh dấu Micro-App đang hoạt động và dọn dẹp các vùng nhớ đệm tạm của các app không dùng nếu không có tác vụ dở dang.
    pub fn hibernate_inactive_apps(&self, active_app: MicroAppId) {
        for app_id in MicroAppId::ALL {
            if app_id != active_app && !self.has_active_work(app_id) {
                self.reset_app_state(app_id);
            }
        }
    }
}

/// Thin Shell điều phối tổng thể của Office Utility VN:
/// - Thanh tiêu đề cửa sổ máy tính (Titlebar) với nút Always-on-Top, Thu nhỏ, Phóng to, Đóng
/// - Khởi tạo `AppState` quản lý in-memory state cho 4 micro-apps
/// - Điều phối render linh hoạt giữa Launcher trung tâm và 4 Micro-Apps
/// - Quản lý vòng đời cập nhật ứng dụng (Update Dialog & Toast)
#[component]
pub fn App() -> impl IntoView {
    let app_state = AppState::new();
    provide_context(app_state);

    let close_manager = WindowCloseManager::new();
    provide_context(close_manager);

    #[cfg(target_arch = "wasm32")]
    {
        let unlisten_opt =
            crate::platform::window_close::register_window_close_listener(close_manager, app_state);
        if let Some(unlisten) = unlisten_opt {
            on_cleanup(move || {
                unlisten.run();
            });
        }
    }

    // Trạng thái cập nhật ứng dụng (Updater Lifecycle)
    let (update_modal_open, set_update_modal_open) = signal(false);
    #[allow(unused_variables)]
    let (available_update, set_available_update) = signal(None::<UpdateInfo>);
    let (is_checking_update, set_is_checking_update) = signal(false);
    let (is_installing_update, set_is_installing_update) = signal(false);
    let (update_toast, set_update_toast) = signal(None::<String>);
    let (is_always_on_top, set_is_always_on_top) = signal(false);

    let check_for_updates = move |manual: bool| {
        set_is_checking_update.set(true);
        #[cfg(target_arch = "wasm32")]
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&CheckUpdateArgs { manual }).unwrap();
            let resp = invoke("check_app_update", args).await;
            if let Ok(info) = serde_wasm_bindgen::from_value::<UpdateInfo>(resp) {
                if info.has_update {
                    set_available_update.set(Some(info.clone()));
                    if info.should_prompt || manual {
                        set_update_modal_open.set(true);
                    }
                } else if manual {
                    set_update_toast.set(Some(format!(
                        "Bạn đang dùng phiên bản v{} mới nhất!",
                        info.current_version
                    )));
                }
            } else if manual {
                set_update_toast.set(Some(
                    "Không thể kết nối máy chủ cập nhật lúc này.".to_string(),
                ));
            }
            set_is_checking_update.set(false);
        });
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = manual;
            set_is_checking_update.set(false);
        }
    };

    #[cfg(target_arch = "wasm32")]
    {
        let check_fn = check_for_updates;
        spawn_local(async move {
            let promise = js_sys::Promise::new(&mut |resolve, _| {
                if let Some(win) = web_sys::window() {
                    let _ =
                        win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 3000);
                }
            });
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            check_fn(false);
        });
    }

    let handle_update_action = move |action: &'static str| {
        #[cfg(target_arch = "wasm32")]
        let current_target = available_update.get().map(|u| u.latest_version);
        if action == "NOW" {
            set_is_installing_update.set(true);
            #[cfg(target_arch = "wasm32")]
            spawn_local(async move {
                let empty_args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
                let _ = invoke("install_app_update", empty_args).await;
                set_is_installing_update.set(false);
                set_update_modal_open.set(false);
            });
            #[cfg(not(target_arch = "wasm32"))]
            {
                set_is_installing_update.set(false);
                set_update_modal_open.set(false);
            }
        } else {
            #[cfg(target_arch = "wasm32")]
            {
                let act_str = action.to_string();
                spawn_local(async move {
                    let args = serde_wasm_bindgen::to_value(&SetUpdateActionArgs {
                        action: act_str,
                        target_version: current_target,
                    })
                    .unwrap();
                    let _ = invoke("set_update_action", args).await;
                });
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = action;
            }
            set_update_modal_open.set(false);
        }
    };

    let minimize_win = move || {
        #[cfg(target_arch = "wasm32")]
        spawn_local(async move {
            let empty_args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
            let _ = invoke("minimize_window", empty_args).await;
        });
    };
    let toggle_maximize_win = move || {
        #[cfg(target_arch = "wasm32")]
        spawn_local(async move {
            let empty_args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
            let _ = invoke("toggle_maximize_window", empty_args).await;
        });
    };
    let close_win = move || {
        #[cfg(target_arch = "wasm32")]
        spawn_local(async move {
            let empty_args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
            let _ = invoke("close_window", empty_args).await;
        });
        #[cfg(not(target_arch = "wasm32"))]
        {
            let has_dirty = app_state.has_unsaved_work_or_running_task();
            let summary = app_state.unsaved_work_summary();
            close_manager.handle_native_close_requested(
                "shell-close-req".into(),
                has_dirty,
                &summary,
            );
        }
    };
    let open_output_dir = move || {
        #[cfg(target_arch = "wasm32")]
        spawn_local(async move {
            let empty_args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
            let _ = invoke("open_output_folder", empty_args).await;
        });
    };

    let toggle_pin = move || {
        let new_state = !is_always_on_top.get();
        set_is_always_on_top.set(new_state);
        #[cfg(target_arch = "wasm32")]
        spawn_local(async move {
            let args =
                serde_wasm_bindgen::to_value(&AlwaysOnTopArgs { enabled: new_state }).unwrap();
            let _ = invoke("toggle_always_on_top", args).await;
        });
    };

    let on_global_keydown = move |ev: KeyboardEvent| {
        let key = ev.key();
        let ctrl_or_meta = ev.ctrl_key() || ev.meta_key();

        if ctrl_or_meta && (key == "p" || key == "P") {
            ev.prevent_default();
            toggle_pin();
        } else if ctrl_or_meta && (key == "o" || key == "O") {
            ev.prevent_default();
            open_output_dir();
        } else if ctrl_or_meta && (key == "w" || key == "W") {
            ev.prevent_default();
            close_win();
        } else if key == "Escape" && update_modal_open.get() {
            set_update_modal_open.set(false);
        }
    };

    view! {
        <div class="app-window" on:keydown=on_global_keydown tabindex="0">
            <header class="titlebar" data-tauri-drag-region="true">
                <div class="titlebar-left">
                    <span style="font-size: 16px;">"⚡"</span>
                    <span style="font-weight: 600;">"Office Utility VN"</span>
                    <span style="font-size: 12px; color: var(--text-muted); font-weight: 400;">"v0.2.2"</span>
                    <button
                        class="titlebar-update-btn"
                        prop:disabled=move || is_checking_update.get()
                        on:click=move |_| check_for_updates(true)
                        title="Kiểm tra bản cập nhật mới nhất"
                    >
                        { move || if is_checking_update.get() { "🔄 Đang kiểm tra..." } else { "🔄 Kiểm tra cập nhật" } }
                    </button>
                </div>

                <div class="titlebar-center" data-tauri-drag-region="true"></div>

                <div class="titlebar-right">
                    // [Stage 4C-1 Product Cutover]: Ẩn hoàn toàn nút dev điều hướng trùng lặp,
                    // GridCraft nay đã là ứng dụng bảng tính mặc định trên Launcher.
                    <button
                        class=move || if is_always_on_top.get() { "pin-btn pinned" } else { "pin-btn" }
                        title=move || if is_always_on_top.get() { "Bỏ ghim cửa sổ (Ctrl+P)" } else { "Ghim cửa sổ trên cùng (Ctrl+P)" }
                        on:click=move |_| toggle_pin()
                    >
                        "📌"
                    </button>
                    <button class="window-control-btn" title="Thu nhỏ (Minimize)" on:click=move |_| minimize_win()>"🗕"</button>
                    <button class="window-control-btn" title="Phóng to (Maximize)" on:click=move |_| toggle_maximize_win()>"🗖"</button>
                    <button class="window-control-btn close" title="Đóng (Close)" on:click=move |_| close_win()>"✕"</button>
                </div>
            </header>

            <main class="app-main-content" style="flex: 1; overflow: hidden; display: flex; flex-direction: column;">
                { move || match app_state.current_view.get() {
                    AppView::Launcher => view! {
                        <LauncherView
                            on_select_app=Callback::new(move |app_id| app_state.navigate_to(app_id))
                            on_deep_link=Callback::new(move |(app_id, slug)| app_state.deep_link(app_id, slug))
                        />
                    }.into_any(),
                    AppView::MicroApp(MicroAppId::VnOffice) => {
                        let deep = app_state.deep_link_tool.get_untracked();
                        if let Some(slug) = deep {
                            view! {
                                <VnOfficeApp
                                    state=app_state.vn_office_state
                                    initial_tool=slug
                                    on_back=move || app_state.back_to_launcher()
                                />
                            }.into_any()
                        } else {
                            view! {
                                <VnOfficeApp
                                    state=app_state.vn_office_state
                                    on_back=move || app_state.back_to_launcher()
                                />
                            }.into_any()
                        }
                    },
                    AppView::MicroApp(MicroAppId::PdfStudio) => {
                        let deep = app_state.deep_link_tool.get_untracked();
                        if let Some(slug) = deep {
                            view! {
                                <PdfStudioApp
                                    state=app_state.pdf_studio_state
                                    initial_tool=slug
                                    on_back=move || app_state.back_to_launcher()
                                />
                            }.into_any()
                        } else {
                            view! {
                                <PdfStudioApp
                                    state=app_state.pdf_studio_state
                                    on_back=move || app_state.back_to_launcher()
                                />
                            }.into_any()
                        }
                    },
                    AppView::MicroApp(MicroAppId::MediaStudio) => {
                        let deep = app_state.deep_link_tool.get_untracked();
                        if let Some(slug) = deep {
                            view! {
                                <MediaStudioApp
                                    state=app_state.media_studio_state
                                    initial_tool=slug
                                    on_back=move || app_state.back_to_launcher()
                                />
                            }.into_any()
                        } else {
                            view! {
                                <MediaStudioApp
                                    state=app_state.media_studio_state
                                    on_back=move || app_state.back_to_launcher()
                                />
                            }.into_any()
                        }
                    },
                    AppView::MicroApp(MicroAppId::DataLab) => {
                        // [Stage 4C-1 Product Cutover]: GridCraft trở thành ứng dụng bảng tính mặc định.
                        // Cơ chế Rollback có kiểm soát: nếu biến môi trường hoặc localStorage bật, cho phép trở về Legacy Data Lab.
                        let rollback_to_legacy = {
                            #[cfg(target_arch = "wasm32")]
                            {
                                web_sys::window()
                                    .and_then(|w| w.local_storage().ok().flatten())
                                    .and_then(|s| s.get_item("OFFICE_UTILITY_ROLLBACK_LEGACY_DATALAB").ok().flatten())
                                    .map(|v| v == "true" || v == "1")
                                    .unwrap_or(false)
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                std::env::var("OFFICE_UTILITY_ROLLBACK_LEGACY_DATALAB").is_ok()
                            }
                        };

                        if rollback_to_legacy {
                            let deep = app_state.deep_link_tool.get_untracked();
                            if let Some(slug) = deep {
                                view! {
                                    <DataLabApp
                                        state=app_state.data_lab_state
                                        initial_tool=slug
                                        on_back=move || app_state.back_to_launcher()
                                    />
                                }.into_any()
                            } else {
                                view! {
                                    <DataLabApp
                                        state=app_state.data_lab_state
                                        on_back=move || app_state.back_to_launcher()
                                    />
                                }.into_any()
                            }
                        } else {
                            view! {
                                <crate::micro_apps::gridcraft::GridCraftApp
                                    state=app_state.gridcraft_state
                                    on_back=move || app_state.back_to_launcher()
                                />
                            }.into_any()
                        }
                    },
                    #[cfg(debug_assertions)]
                    AppView::GridCraftDev => {
                        view! {
                            <crate::micro_apps::gridcraft::GridCraftApp
                                state=app_state.gridcraft_state
                                on_back=move || app_state.back_to_launcher()
                            />
                        }.into_any()
                    },
                }}
            </main>

            // Update Dialog Modal
            { move || if update_modal_open.get() {
                if let Some(info) = available_update.get() {
                    let notes = info.release_notes.unwrap_or_else(|| "Bản cập nhật tối ưu hóa hiệu năng, sửa lỗi và cải tiến trải nghiệm người dùng.".to_string());
                    let latest_ver = info.latest_version.clone();
                    view! {
                        <div class="modal-overlay" on:click=move |_| set_update_modal_open.set(false)>
                            <div class="update-dialog" on:click=move |ev: MouseEvent| ev.stop_propagation()>
                                <div class="update-dialog-header">
                                    <div class="update-dialog-title">
                                        <span>"🚀"</span>
                                        <span>"Đã có Bản Cập nhật Mới!"</span>
                                    </div>
                                    <button class="window-control-btn close" on:click=move |_| set_update_modal_open.set(false)>"✕"</button>
                                </div>

                                <div class="update-dialog-body">
                                    <div class="update-version-badge">
                                        {format!("Phiên bản mới: v{}", latest_ver)}
                                    </div>
                                    <p style="margin: 0; font-size: 13px; color: #E2E8F0;">
                                        "Phiên bản mới của Office Utility VN đã sẵn sàng. Dưới đây là tóm tắt những thay đổi nổi bật:"
                                    </p>
                                    <div class="update-notes-box">
                                        {notes}
                                    </div>
                                </div>

                                <div class="update-dialog-footer">
                                    <button
                                        class="btn-skip-version"
                                        title="Không bao giờ hỏi lại về phiên bản này"
                                        on:click=move |_| handle_update_action("SKIP")
                                    >
                                        "Bỏ qua bản này"
                                    </button>

                                    <div style="display: flex; gap: 8px;">
                                        <button
                                            class="btn-update-later"
                                            title="Nhắc tôi lại sau 24 giờ"
                                            on:click=move |_| handle_update_action("LATER")
                                        >
                                            "⏰ Nhắc lại sau"
                                        </button>
                                        <button
                                            class="btn-update-now"
                                            prop:disabled=move || is_installing_update.get()
                                            on:click=move |_| handle_update_action("NOW")
                                        >
                                            { move || if is_installing_update.get() { "⏳ Đang cài đặt..." } else { "🚀 Cập nhật ngay" } }
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            } else {
                view! { <div></div> }.into_any()
            }}

            // Update Toast
            { move || {
                if let Some(msg) = update_toast.get() {
                    view! {
                        <div class="clipboard-toast" on:click=move |_| set_update_toast.set(None)>
                            <span>"🔄"</span>
                            <span>{msg}</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            // Window Close Confirmation Dialog Modal (Stage 4C-0C)
            {move || {
                if close_manager.is_dialog_open.get() {
                    let summary = close_manager.dirty_summary.get();
                    let is_saving = close_manager.is_saving.get();
                    let err_opt = close_manager.error_message.get();

                    view! {
                        <div
                            class="modal-backdrop"
                            style="position: fixed; inset: 0; background: rgba(0, 0, 0, 0.75); display: flex; align-items: center; justify-content: center; z-index: 100; -webkit-app-region: no-drag;"
                        >
                            <div
                                class="modal-dialog"
                                style="background: #18181B; border: 1px solid #3F3F46; border-radius: 8px; padding: 24px; max-width: 480px; width: 100%; color: #FAFAFA; box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5); font-family: sans-serif; -webkit-app-region: no-drag;"
                            >
                                <div style="font-size: 16px; font-weight: 600; margin-bottom: 8px; display: flex; align-items: center; gap: 8px;">
                                    <span>"⚠️"</span>
                                    <span>"Thay đổi chưa được lưu"</span>
                                </div>
                                <p style="font-size: 13px; color: #A1A1AA; line-height: 1.5; margin-bottom: 12px;">
                                    "Bạn có thay đổi chưa được lưu: "
                                    <strong style="color: #FCD34D;">{summary}</strong>
                                    ". Nếu đóng cửa sổ bây giờ, các thay đổi chưa lưu sẽ bị mất vĩnh viễn."
                                </p>
                                {if let Some(err) = err_opt {
                                    view! {
                                        <div style="font-size: 12px; color: #FCA5A5; background: rgba(239, 68, 68, 0.2); border: 1px solid #DC2626; border-radius: 4px; padding: 6px 10px; margin-bottom: 16px;">
                                            {err}
                                        </div>
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }}
                                <div style="display: flex; justify-content: flex-end; gap: 10px;">
                                    <button
                                        type="button"
                                        style="background: #27272A; color: #FAFAFA; border: 1px solid #3F3F46; border-radius: 6px; padding: 6px 14px; font-size: 13px; cursor: pointer; white-space: nowrap; -webkit-app-region: no-drag;"
                                        on:click=move |_| close_manager.on_user_decision(WindowCloseDecision::Cancel, app_state)
                                        disabled=is_saving
                                    >
                                        "Hủy (Cancel)"
                                    </button>
                                    <button
                                        type="button"
                                        style="background: #DC2626; color: #FFFFFF; border: none; border-radius: 6px; padding: 6px 14px; font-size: 13px; font-weight: 500; cursor: pointer; white-space: nowrap; -webkit-app-region: no-drag;"
                                        on:click=move |_| close_manager.on_user_decision(WindowCloseDecision::Discard, app_state)
                                        disabled=is_saving
                                    >
                                        "Không lưu (Don't Save)"
                                    </button>
                                    <button
                                        type="button"
                                        style="background: #059669; color: #FFFFFF; border: none; border-radius: 6px; padding: 6px 14px; font-size: 13px; font-weight: 500; cursor: pointer; white-space: nowrap; -webkit-app-region: no-drag;"
                                        on:click=move |_| close_manager.on_user_decision(WindowCloseDecision::Save, app_state)
                                        disabled=is_saving
                                    >
                                        {if is_saving { "Đang lưu..." } else { "Lưu & Thoát (Save)" }}
                                    </button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }
            }}
        </div>
    }
}
