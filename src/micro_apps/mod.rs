use crate::app::ToolItem;
use serde::{Deserialize, Serialize};

pub mod registry;
pub use registry::{ToolRegistration, ToolRegistry};

pub mod vn_office;
pub use vn_office::{VnCalculationRecord, VnOfficeApp, VnOfficeState};

pub mod pdf_studio;
pub use pdf_studio::{PdfStudioApp, PdfStudioState};

pub mod media_studio;
pub use media_studio::{MediaStudioApp, MediaStudioState};

pub mod data_lab;
pub use data_lab::{DataLabApp, DataLabState};

/// Định danh cho 4 Micro-Apps độc lập của Office Utility VN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MicroAppId {
    VnOffice,
    PdfStudio,
    MediaStudio,
    DataLab,
}

/// Metadata mô tả tổng quan của một Micro-App.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroAppMetadata {
    pub id: MicroAppId,
    pub name: &'static str,
    pub icon: &'static str,
    pub description: &'static str,
    pub tool_count: usize,
}

impl MicroAppId {
    pub const ALL: [MicroAppId; 4] = [
        MicroAppId::VnOffice,
        MicroAppId::PdfStudio,
        MicroAppId::MediaStudio,
        MicroAppId::DataLab,
    ];

    pub fn metadata(self) -> MicroAppMetadata {
        let count = ToolRegistry::count_for_app(self);
        match self {
            MicroAppId::VnOffice => MicroAppMetadata {
                id: self,
                name: "VN Office",
                icon: "🏢",
                description: "Trợ lý nghiệp vụ kế toán, thuế, ngân hàng & văn bản quy chuẩn Việt Nam",
                tool_count: count,
            },
            MicroAppId::PdfStudio => MicroAppMetadata {
                id: self,
                name: "PDF Studio",
                icon: "📄",
                description: "Xưởng xử lý, biên tập, bảo mật và số hóa tài liệu PDF chuyên nghiệp",
                tool_count: count,
            },
            MicroAppId::MediaStudio => MicroAppMetadata {
                id: self,
                name: "Media Studio",
                icon: "🎨",
                description: "Phòng thu xử lý ảnh thẻ, làm đẹp ảnh, xóa siêu dữ liệu và tối ưu đa phương tiện",
                tool_count: count,
            },
            MicroAppId::DataLab => MicroAppMetadata {
                id: self,
                name: "Data Lab",
                icon: "📊",
                description: "Phòng thí nghiệm bảng tính Excel, đối chiếu dữ liệu và trạm chuyển đổi tệp vạn năng",
                tool_count: count,
            },
        }
    }

    pub fn name(self) -> &'static str {
        self.metadata().name
    }

    pub fn icon(self) -> &'static str {
        self.metadata().icon
    }

    pub fn description(self) -> &'static str {
        self.metadata().description
    }

    pub fn tool_count(self) -> usize {
        self.metadata().tool_count
    }

    pub fn tools(self) -> &'static [ToolItem] {
        micro_app_tools(self)
    }
}

/// Ánh xạ tĩnh từ slug công cụ sang MicroAppId tương ứng (ủy quyền cho ToolRegistry).
pub fn tool_slug_to_micro_app(slug: &str) -> Option<MicroAppId> {
    ToolRegistry::lookup_app(slug)
}

pub const VN_OFFICE_TOOLS: &[ToolItem] = &[
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
            slug: "vietnamese-font-converter",
            name: "Chuyển mã Font Tiếng Việt",
            domain: "data-lab",
            description: "Chuyển mã font TCVN3 (ABC), VNI sang Unicode tổ hợp và dựng sẵn.",
            is_pinned: false,
        },
    ToolItem {
            slug: "encoding-mojibake-fixer",
            name: "Sửa Lỗi Hiển thị Chữ Mojibake",
            domain: "data-lab",
            description: "Khôi phục chuỗi văn bản tiếng Việt bị lỗi hiển thị bảng mã UTF-8 và Windows-1252.",
            is_pinned: false,
        },
];

pub const PDF_STUDIO_TOOLS: &[ToolItem] = &[
    ToolItem {
            slug: "pdf-merge-split",
            name: "Ghép, Tách & Xoay trang PDF",
            domain: "doc-suite",
            description: "Ghép nhiều file PDF hoặc tách trang theo dải lựa chọn cực nhanh.",
            is_pinned: true,
        },
    ToolItem {
            slug: "pdf-protect-sign",
            name: "Bảo mật & Mở khóa PDF",
            domain: "doc-suite",
            description: "Áp dụng mã hóa AES-256 hoặc mở khóa mật khẩu PDF an toàn on-device.",
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
            description: "Nén lại các luồng FlateDecode và tái lấy mẫu ảnh nhúng giảm dung lượng file.",
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
            description: "Xóa sạch Trailer Info và luồng XML metadata XMP bảo mật danh tính tác giả.",
            is_pinned: false,
        },
    ToolItem {
            slug: "pdf-flatten-forms",
            name: "Làm phẳng PDF (Flatten Forms)",
            domain: "doc-suite",
            description: "Làm phẳng biểu mẫu AcroForm, hợp nhất dữ liệu nhập liệu thành văn bản tĩnh.",
            is_pinned: false,
        },
    ToolItem {
            slug: "pdf-crop-margin",
            name: "Cắt Viền Trắng Thừa (Auto-Crop)",
            domain: "doc-suite",
            description: "Tìm hộp giới hạn Bounding Box nội dung và cập nhật lại CropBox cắt lề chính xác.",
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
            description: "Hỗ trợ Text Diff và Visual Pixel Diff so sánh sự sai lệch giữa hai tài liệu.",
            is_pinned: false,
        },
    ToolItem {
            slug: "pdf-bookmark-toc",
            name: "Tạo Cây Mục lục PDF (Bookmark TOC)",
            domain: "doc-suite",
            description: "Khởi tạo và liên kết cấu trúc cây từ điển Outlines đa cấp trong PDF Catalog.",
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
            description: "Khảo sát trường AcroForm, xuất dữ liệu JSON/CSV và tự động điền form hàng loạt.",
            is_pinned: false,
        },
    ToolItem {
            slug: "pdf-redaction",
            name: "Xóa Dữ liệu Nhạy cảm PDF",
            domain: "doc-suite",
            description: "Cắt bỏ triệt để chuỗi ký tự nhạy cảm khỏi stream PDF và phủ hộp đen bảo mật.",
            is_pinned: false,
        },
    ToolItem {
            slug: "pdf-archive-linearize",
            name: "Chuẩn hóa Lưu trữ PDF/A & Fast Web View",
            domain: "doc-suite",
            description: "Chuyển đổi luồng siêu dữ liệu sang chuẩn quốc tế PDF/A-1b và tuyến tính hóa tải nhanh trên web.",
            is_pinned: false,
        },
    ToolItem {
            slug: "pdf-pki-sign",
            name: "Chữ ký số PKI & Xác thực Toàn vẹn",
            domain: "doc-suite",
            description: "Gắn chữ ký số chuẩn PKCS#7 với /ByteRange, kiểm tra toàn vẹn và phát hiện tài liệu bị giả mạo.",
            is_pinned: false,
        },
];

pub const MEDIA_STUDIO_TOOLS: &[ToolItem] = &[
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
];

pub const DATA_LAB_TOOLS: &[ToolItem] = &[
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
    ToolItem {
            slug: "docx-template-mailmerge",
            name: "Trộn Hợp đồng Mailmerge Hàng loạt",
            domain: "doc-suite",
            description: "Giải nén luồng XML, hợp nhất thẻ <w:r> và trộn hàng loạt file Word tốc độ cao.",
            is_pinned: false,
        },
    ToolItem {
            slug: "universal-file-converter-hub",
            name: "Trạm Chuyển đổi Tệp Vạn năng",
            domain: "sys-hub",
            description: "Tự động nhận diện chữ ký file và chuyển đổi định dạng linh hoạt.",
            is_pinned: true,
        },
];

/// Trả về danh sách tĩnh các công cụ thuộc một MicroAppId (ủy quyền cho ToolRegistry).
pub fn micro_app_tools(app: MicroAppId) -> &'static [ToolItem] {
    ToolRegistry::micro_app_tools(app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ALL_TOOLS;

    #[test]
    fn test_vn_office_routing() {
        assert_eq!(
            tool_slug_to_micro_app("vietnamese-number-to-words"),
            Some(MicroAppId::VnOffice)
        );
        assert_eq!(
            tool_slug_to_micro_app("vietnamese-font-converter"),
            Some(MicroAppId::VnOffice)
        );
        assert_eq!(
            tool_slug_to_micro_app("encoding-mojibake-fixer"),
            Some(MicroAppId::VnOffice)
        );
    }

    #[test]
    fn test_pdf_studio_routing() {
        assert_eq!(
            tool_slug_to_micro_app("pdf-merge-split"),
            Some(MicroAppId::PdfStudio)
        );
        assert_eq!(
            tool_slug_to_micro_app("vietnamese-ocr"),
            Some(MicroAppId::PdfStudio)
        );
    }

    #[test]
    fn test_media_studio_routing() {
        assert_eq!(
            tool_slug_to_micro_app("image-batch"),
            Some(MicroAppId::MediaStudio)
        );
        assert_eq!(
            tool_slug_to_micro_app("screenshot-beautifier"),
            Some(MicroAppId::MediaStudio)
        );
    }

    #[test]
    fn test_data_lab_routing() {
        assert_eq!(
            tool_slug_to_micro_app("excel-cleaner-normalizer"),
            Some(MicroAppId::DataLab)
        );
        assert_eq!(
            tool_slug_to_micro_app("docx-template-mailmerge"),
            Some(MicroAppId::DataLab)
        );
        assert_eq!(
            tool_slug_to_micro_app("universal-file-converter-hub"),
            Some(MicroAppId::DataLab)
        );
    }

    #[test]
    fn test_all_55_tools_mapped() {
        assert_eq!(ALL_TOOLS.len(), 55);
        for tool in ALL_TOOLS.iter() {
            assert!(
                tool_slug_to_micro_app(tool.slug).is_some(),
                "Công cụ {} phải được map vào MicroAppId",
                tool.slug
            );
        }
    }

    #[test]
    fn test_tools_count_per_app() {
        assert_eq!(micro_app_tools(MicroAppId::VnOffice).len(), 11);
        assert_eq!(micro_app_tools(MicroAppId::PdfStudio).len(), 22);
        assert_eq!(micro_app_tools(MicroAppId::MediaStudio).len(), 9);
        assert_eq!(micro_app_tools(MicroAppId::DataLab).len(), 13);
    }
}
