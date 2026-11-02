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
pub fn resolve_tool(slug: &str) -> Option<Box<dyn OfficeTool>> {
    match slug {
        // Subsystem 1: Document Suite (19 tools)
        "pdf-merge-split" => Some(Box::new(office_engine::doc_suite::PdfMergeSplitTool::default())),
        "pdf-protect-sign" => Some(Box::new(office_engine::doc_suite::PdfProtectSignTool::default())),
        "pdf-to-markdown" => Some(Box::new(office_engine::doc_suite::PdfToMarkdownTool::default())),
        "pdf-compress" => Some(Box::new(office_engine::doc_suite::PdfCompressTool::default())),
        "vietnamese-ocr" => Some(Box::new(office_engine::doc_suite::VietnameseOcrTool::default())),
        "pdf-watermark-stamp" => Some(Box::new(office_engine::doc_suite::PdfWatermarkStampTool::default())),
        "pdf-page-number-header" => Some(Box::new(office_engine::doc_suite::PdfPageNumberHeaderTool::default())),
        "pdf-metadata-scrubber" => Some(Box::new(office_engine::doc_suite::PdfMetadataScrubberTool::default())),
        "pdf-flatten-forms" => Some(Box::new(office_engine::doc_suite::PdfFlattenFormsTool::default())),
        "pdf-crop-margin" => Some(Box::new(office_engine::doc_suite::PdfCropMarginTool::default())),
        "pdf-booklet-imposition" => Some(Box::new(office_engine::doc_suite::PdfBookletImpositionTool::default())),
        "pdf-visual-diff" => Some(Box::new(office_engine::doc_suite::PdfVisualDiffTool::default())),
        "docx-template-mailmerge" => Some(Box::new(office_engine::doc_suite::DocxTemplateMailmergeTool::default())),
        "pdf-bookmark-toc" => Some(Box::new(office_engine::doc_suite::PdfBookmarkTocTool::default())),
        "pdf-to-long-image" => Some(Box::new(office_engine::doc_suite::PdfToLongImageTool::default())),
        "pdf-grayscale-print" => Some(Box::new(office_engine::doc_suite::PdfGrayscalePrintTool::default())),
        "pdf-repair-corrupt" => Some(Box::new(office_engine::doc_suite::PdfRepairCorruptTool::default())),
        "markdown-to-styled-pdf" => Some(Box::new(office_engine::doc_suite::MarkdownToPdfTool::default())),
        "pdf-table-extractor" => Some(Box::new(office_engine::doc_suite::PdfTableExtractorTool::default())),

        // Subsystem 2: Media Studio (9 tools)
        "image-batch" => Some(Box::new(office_engine::media_studio::ImageBatchTool::default())),
        "photo-id-crop" => Some(Box::new(office_engine::media_studio::PhotoIdCropTool::default())),
        "image-bg-redact" => Some(Box::new(office_engine::media_studio::ImageBgRedactTool::default())),
        "image-exif-inspector" => Some(Box::new(office_engine::media_studio::ImageExifInspectorTool::default())),
        "watermark-batch-studio" => Some(Box::new(office_engine::media_studio::WatermarkBatchTool::default())),
        "screenshot-beautifier" => Some(Box::new(office_engine::media_studio::ScreenshotBeautifierTool::default())),
        "doc-scanner-perspective" => Some(Box::new(office_engine::media_studio::DocScannerPerspectiveTool::default())),
        "audio-silence-trimmer" => Some(Box::new(office_engine::media_studio::AudioSilenceTrimmerTool::default())),
        "video-to-gif-webp" => Some(Box::new(office_engine::media_studio::VideoToGifWebpTool::default())),

        // Subsystem 3: Data Lab (13 tools)
        "vietnamese-font-converter" => Some(Box::new(office_engine::data_lab::VietnameseFontConverterTool::default())),
        "excel-cleaner-normalizer" => Some(Box::new(office_engine::data_lab::ExcelCleanerNormalizerTool::default())),
        "diff-checker-text-sheets" => Some(Box::new(office_engine::data_lab::DiffCheckerTextSheetsTool::default())),
        "data-structure-converter" => Some(Box::new(office_engine::data_lab::DataStructureConverterTool::default())),
        "excel-split-by-column" => Some(Box::new(office_engine::data_lab::ExcelSplitByColumnTool::default())),
        "excel-multi-merger" => Some(Box::new(office_engine::data_lab::ExcelMultiMergerTool::default())),
        "excel-visual-joiner" => Some(Box::new(office_engine::data_lab::ExcelVisualJoinerTool::default())),
        "excel-sheet-unlocker" => Some(Box::new(office_engine::data_lab::ExcelSheetUnlockerTool::default())),
        "list-set-operations" => Some(Box::new(office_engine::data_lab::ListSetOperationsTool::default())),
        "encoding-mojibake-fixer" => Some(Box::new(office_engine::data_lab::EncodingMojibakeFixerTool::default())),
        "text-pii-anonymizer" => Some(Box::new(office_engine::data_lab::TextPiiAnonymizerTool::default())),
        "sql-formatter-linter" => Some(Box::new(office_engine::data_lab::SqlFormatterLinterTool::default())),
        "base64-file-codec" => Some(Box::new(office_engine::data_lab::Base64FileCodecTool::default())),

        // Subsystem 4: Vietnam Office (9 tools)
        "vietnamese-number-to-words" => Some(Box::new(office_engine::vn_office::VietnameseNumberToWordsTool::default())),
        "gross-net-salary-calculator" => Some(Box::new(office_engine::vn_office::GrossNetSalaryCalculatorTool::default())),
        "vietqr-napas247-generator" => Some(Box::new(office_engine::vn_office::VietQrNapas247GeneratorTool::default())),
        "vn-einvoice-xml-reader" => Some(Box::new(office_engine::vn_office::VnEinvoiceXmlReaderTool::default())),
        "vn-cccd-mst-validator" => Some(Box::new(office_engine::vn_office::VnCccdMstValidatorTool::default())),
        "vn-address-normalizer" => Some(Box::new(office_engine::vn_office::VnAddressNormalizerTool::default())),
        "vn-working-days-calc" => Some(Box::new(office_engine::vn_office::VnWorkingDaysCalcTool::default())),
        "vn-loan-amortization" => Some(Box::new(office_engine::vn_office::VnLoanAmortizationTool::default())),
        "vn-bank-bin-mapper" => Some(Box::new(office_engine::vn_office::VnBankBinMapperTool::default())),

        // System Hub (Spine Foundation)
        "universal-converter" => Some(Box::new(office_engine::sys_hub::UniversalConverterTool::default())),

        _ => None,
    }
}
