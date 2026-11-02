use office_engine::OfficeTool;

/// Danh sách toàn bộ 50 slugs thuộc 4 phân hệ chính (02-srs.md)
pub const ALL_50_TOOL_SLUGS: &[&str] = &[
    // Subsystem 1: Document Suite (19 tools)
    "pdf-merge-split",
    "pdf-protect-sign",
    "pdf-to-markdown",
    "pdf-compress",
    "vietnamese-ocr",
    "pdf-watermark-stamp",
    "pdf-page-number-header",
    "pdf-metadata-scrubber",
    "pdf-flatten-forms",
    "pdf-crop-margin",
    "pdf-booklet-imposition",
    "pdf-visual-diff",
    "docx-template-mailmerge",
    "pdf-bookmark-toc",
    "pdf-to-long-image",
    "pdf-grayscale-print",
    "pdf-repair-corrupt",
    "markdown-to-styled-pdf",
    "pdf-table-extractor",
    // Subsystem 2: Media Studio (9 tools)
    "image-batch",
    "photo-id-crop",
    "image-bg-redact",
    "image-exif-inspector",
    "watermark-batch-studio",
    "screenshot-beautifier",
    "doc-scanner-perspective",
    "audio-silence-trimmer",
    "video-to-gif-webp",
    // Subsystem 3: Data Lab (13 tools)
    "vietnamese-font-converter",
    "excel-cleaner-normalizer",
    "diff-checker-text-sheets",
    "data-structure-converter",
    "excel-split-by-column",
    "excel-multi-merger",
    "excel-visual-joiner",
    "excel-sheet-unlocker",
    "list-set-operations",
    "encoding-mojibake-fixer",
    "text-pii-anonymizer",
    "sql-formatter-linter",
    "base64-file-codec",
    // Subsystem 4: Vietnam Office (9 tools)
    "vietnamese-number-to-words",
    "gross-net-salary-calculator",
    "vietqr-napas247-generator",
    "vn-einvoice-xml-reader",
    "vn-cccd-mst-validator",
    "vn-address-normalizer",
    "vn-working-days-calc",
    "vn-loan-amortization",
    "vn-bank-bin-mapper",
];

/// Ánh xạ động từ tool slug sang instance của `OfficeTool`.
///
/// Các công cụ đã tích hợp vào `crates/office-engine` sẽ trả về `Some(Box<dyn OfficeTool>)`.
/// Các công cụ đang được sản xuất trên các nhánh worktree song song (sẽ merge ở M6)
/// trả về `None` khi chưa có trong build hiện tại.
pub fn resolve_tool(slug: &str) -> Option<Box<dyn OfficeTool>> {
    match slug {
        "pdf-merge-split" => Some(Box::new(office_engine::doc_suite::PdfMergeSplitTool::default())),
        "image-batch" => Some(Box::new(office_engine::media_studio::ImageBatchTool::default())),
        "vietnamese-font-converter" => Some(Box::new(office_engine::data_lab::VietnameseFontConverterTool::default())),
        "vietnamese-number-to-words" => Some(Box::new(office_engine::vn_office::VietnameseNumberToWordsTool::default())),
        "universal-converter" => Some(Box::new(office_engine::sys_hub::UniversalConverterTool::default())),
        _ => None,
    }
}
