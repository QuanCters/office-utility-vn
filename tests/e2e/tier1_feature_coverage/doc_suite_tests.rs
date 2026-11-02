use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use serde_json::json;
use office_shared::DispatchTaskRequest;
use crate::harness::TestHarness;

// Macro tiện ích sinh 5 ca kiểm thử bao phủ tính năng (Tier 1) cho mỗi công cụ tài liệu
macro_rules! define_doc_suite_tier1_tests {
    ($slug:ident, $slug_str:expr, $happy_opts:expr, $second_opts:expr, $matrix_opts:expr) => {
        paste::paste! {
            #[test]
            fn [<test_ $slug _happy_path>]() {
                let harness = TestHarness::new();
                let pdf = harness.create_valid_pdf(concat!($slug_str, "_happy.pdf"), 3);
                let req = DispatchTaskRequest {
                    task_id: format!("{}-happy", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![pdf.to_string_lossy().to_string()],
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $happy_opts,
                };
                let cancel = AtomicBool::new(false);
                if let Some(tool) = harness.get_tool($slug_str) {
                    let res = tool.execute(&req, &cancel, None).expect("Tier 1 Happy path must succeed");
                    assert!(res.status == "SUCCESS" || res.status == "PARTIAL_SUCCESS");
                }
            }

            #[test]
            fn [<test_ $slug _secondary_mode>]() {
                let harness = TestHarness::new();
                let pdf = harness.create_valid_pdf(concat!($slug_str, "_second.pdf"), 2);
                let req = DispatchTaskRequest {
                    task_id: format!("{}-second", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![pdf.to_string_lossy().to_string()],
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $second_opts,
                };
                let cancel = AtomicBool::new(false);
                if let Some(tool) = harness.get_tool($slug_str) {
                    let res = tool.execute(&req, &cancel, None).expect("Tier 1 Secondary mode must succeed");
                    assert!(res.status == "SUCCESS" || res.status == "PARTIAL_SUCCESS");
                }
            }

            #[test]
            fn [<test_ $slug _batch_processing>]() {
                let harness = TestHarness::new();
                let p1 = harness.create_valid_pdf(concat!($slug_str, "_b1.pdf"), 1);
                let p2 = harness.create_valid_pdf(concat!($slug_str, "_b2.pdf"), 2);
                let req = DispatchTaskRequest {
                    task_id: format!("{}-batch", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![
                        p1.to_string_lossy().to_string(),
                        p2.to_string_lossy().to_string(),
                    ],
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $happy_opts,
                };
                let cancel = AtomicBool::new(false);
                if let Some(tool) = harness.get_tool($slug_str) {
                    let res = tool.execute(&req, &cancel, None).expect("Tier 1 Batch must succeed");
                    assert!(res.status == "SUCCESS" || res.status == "PARTIAL_SUCCESS");
                }
            }

            #[test]
            fn [<test_ $slug _options_matrix>]() {
                let harness = TestHarness::new();
                let pdf = harness.create_valid_pdf(concat!($slug_str, "_opt.pdf"), 2);
                let req = DispatchTaskRequest {
                    task_id: format!("{}-opt", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![pdf.to_string_lossy().to_string()],
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $matrix_opts,
                };
                let cancel = AtomicBool::new(false);
                if let Some(tool) = harness.get_tool($slug_str) {
                    let res = tool.execute(&req, &cancel, None).expect("Tier 1 Options matrix must succeed");
                    assert!(res.status == "SUCCESS" || res.status == "PARTIAL_SUCCESS");
                }
            }

            #[test]
            fn [<test_ $slug _progress_callback>]() {
                let harness = TestHarness::new();
                let pdf = harness.create_valid_pdf(concat!($slug_str, "_prog.pdf"), 2);
                let req = DispatchTaskRequest {
                    task_id: format!("{}-prog", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![pdf.to_string_lossy().to_string()],
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $happy_opts,
                };
                let cancel = AtomicBool::new(false);
                let progress_called = Arc::new(AtomicBool::new(false));
                let p_clone = Arc::clone(&progress_called);
                let cb = move |p: office_shared::TaskProgress| {
                    p_clone.store(true, Ordering::Relaxed);
                    assert!(p.percentage >= 0.0 && p.percentage <= 100.0);
                };

                if let Some(tool) = harness.get_tool($slug_str) {
                    let _ = tool.execute(&req, &cancel, Some(&cb));
                    assert!(progress_called.load(Ordering::Relaxed), "Progress callback must be invoked");
                }
            }
        }
    };
}

// 1. pdf-merge-split (FR-DOC-01)
define_doc_suite_tier1_tests!(
    pdf_merge_split,
    "pdf-merge-split",
    json!({ "mode": "merge" }),
    json!({ "mode": "split", "page_ranges": "1-2" }),
    json!({ "mode": "rotate", "rotation_deg": 90 })
);

// 2. pdf-protect-sign (FR-DOC-02)
define_doc_suite_tier1_tests!(
    pdf_protect_sign,
    "pdf-protect-sign",
    json!({ "password": "UserPass123", "algorithm": "AES-256", "allow_printing": true }),
    json!({ "action": "unlock", "password": "UserPass123" }),
    json!({ "password": "AuditPass456!", "allow_copy": false, "allow_printing": false })
);

// 3. pdf-to-markdown (FR-DOC-03)
define_doc_suite_tier1_tests!(
    pdf_to_markdown,
    "pdf-to-markdown",
    json!({ "extract_images": false, "detect_tables": true }),
    json!({ "extract_images": true, "image_format": "png" }),
    json!({ "detect_headings": true, "format_vietnamese": true })
);

// 4. pdf-compress (FR-DOC-04)
define_doc_suite_tier1_tests!(
    pdf_compress,
    "pdf-compress",
    json!({ "compression_level": "medium", "dpi": 150 }),
    json!({ "compression_level": "maximum", "dpi": 72 }),
    json!({ "compression_level": "minimal", "dpi": 300, "downsample_images": true })
);

// 5. vietnamese-ocr (FR-DOC-05)
define_doc_suite_tier1_tests!(
    vietnamese_ocr,
    "vietnamese-ocr",
    json!({ "mode": "fast", "language": "vie" }),
    json!({ "mode": "accurate", "export_format": "txt" }),
    json!({ "mode": "fast", "export_format": "docx", "preserve_layout": true })
);

// 6. pdf-watermark-stamp (FR-DOC-06)
define_doc_suite_tier1_tests!(
    pdf_watermark_stamp,
    "pdf-watermark-stamp",
    json!({ "watermark_text": "BẢN QUYỀN NỘI BỘ", "opacity": 0.3, "rotation": 45 }),
    json!({ "stamp_type": "confidential", "opacity": 0.5, "position": "center" }),
    json!({ "watermark_text": "TUYỆT MẬT", "font_size": 36, "color_hex": "#FF0000" })
);

// 7. pdf-page-number-header (FR-DOC-07)
define_doc_suite_tier1_tests!(
    pdf_page_number_header,
    "pdf-page-number-header",
    json!({ "format": "Trang {page}/{total}", "position": "bottom-right", "start_page": 1 }),
    json!({ "format": "Bates {bates:06}", "position": "bottom-center", "bates_start": 1 }),
    json!({ "header_text": "CÔNG TY TIỆN ÍCH VIỆT NAM", "font_size": 10, "skip_first_page": true })
);

// 8. pdf-metadata-scrubber (FR-DOC-08)
define_doc_suite_tier1_tests!(
    pdf_metadata_scrubber,
    "pdf-metadata-scrubber",
    json!({ "strip_author": true, "strip_producer": true, "strip_xmp": true }),
    json!({ "strip_all": true }),
    json!({ "scrub_history": true, "preserve_creation_date": false })
);

// 9. pdf-flatten-forms (FR-DOC-09)
define_doc_suite_tier1_tests!(
    pdf_flatten_forms,
    "pdf-flatten-forms",
    json!({ "flatten_annotations": true, "flatten_acroforms": true }),
    json!({ "flatten_signatures": false }),
    json!({ "rasterize_unsupported_fields": true, "render_dpi": 200 })
);

// 10. pdf-crop-margin (FR-DOC-10)
define_doc_suite_tier1_tests!(
    pdf_crop_margin,
    "pdf-crop-margin",
    json!({ "auto_detect_whitespace": true, "margin_padding_pt": 10 }),
    json!({ "custom_crop_box": [50, 50, 500, 750] }),
    json!({ "crop_even_odd_differently": false, "threshold_luminance": 250 })
);

// 11. pdf-booklet-imposition (FR-DOC-11)
define_doc_suite_tier1_tests!(
    pdf_booklet_imposition,
    "pdf-booklet-imposition",
    json!({ "layout": "saddle-stitch", "sheet_size": "A4", "two_sided": true }),
    json!({ "layout": "2-up", "sheet_size": "A3", "two_sided": false }),
    json!({ "layout": "4-up", "creep_adjustment_mm": 1.5, "binding_edge": "left" })
);

// 12. pdf-visual-diff (FR-DOC-12)
define_doc_suite_tier1_tests!(
    pdf_visual_diff,
    "pdf-visual-diff",
    json!({ "mode": "pixel-diff", "tolerance": 0.05, "highlight_color": "#FF0000" }),
    json!({ "mode": "text-diff", "ignore_whitespace": true }),
    json!({ "mode": "side-by-side", "render_dpi": 150 })
);

// 13. docx-template-mailmerge (FR-DOC-13)
define_doc_suite_tier1_tests!(
    docx_template_mailmerge,
    "docx-template-mailmerge",
    json!({ "data_source": "inline", "output_pattern": "hop_dong_{stt}.docx" }),
    json!({ "data_source": "xlsx", "sheet_name": "Sheet1" }),
    json!({ "normalize_xml_runs": true, "batch_limit": 500 })
);

// 14. pdf-bookmark-toc (FR-DOC-14)
define_doc_suite_tier1_tests!(
    pdf_bookmark_toc,
    "pdf-bookmark-toc",
    json!({ "auto_generate_from_headings": true, "max_depth": 3 }),
    json!({ "bookmarks": [{ "title": "Chương 1", "page": 1 }, { "title": "Chương 2", "page": 3 }] }),
    json!({ "include_page_numbers": true, "bookmark_font_style": "normal" })
);

// 15. pdf-to-long-image (FR-DOC-15)
define_doc_suite_tier1_tests!(
    pdf_to_long_image,
    "pdf-to-long-image",
    json!({ "format": "png", "dpi": 150, "page_separator_line": true }),
    json!({ "format": "jpg", "quality": 85, "max_width": 1200 }),
    json!({ "format": "webp", "separator_color": "#CCCCCC", "padding_px": 20 })
);

// 16. pdf-grayscale-print (FR-DOC-16)
define_doc_suite_tier1_tests!(
    pdf_grayscale_print,
    "pdf-grayscale-print",
    json!({ "convert_images": true, "convert_vector": true, "convert_text": true }),
    json!({ "gamma_correction": 1.2, "dithering": false }),
    json!({ "black_threshold": 10, "pure_monochrome": false })
);

// 17. pdf-repair-corrupt (FR-DOC-17)
define_doc_suite_tier1_tests!(
    pdf_repair_corrupt,
    "pdf-repair-corrupt",
    json!({ "rebuild_xref": true, "salvage_pages": true }),
    json!({ "linear_scan": true, "strict_mode": false }),
    json!({ "reconstruct_catalog": true, "ignore_trailer_offset": true })
);

// 18. markdown-to-styled-pdf (FR-DOC-18)
define_doc_suite_tier1_tests!(
    markdown_to_styled_pdf,
    "markdown-to-styled-pdf",
    json!({ "theme": "business", "page_size": "A4", "embed_vietnamese_font": true }),
    json!({ "theme": "academic", "margin_mm": 25, "syntax_highlight": true }),
    json!({ "header_template": "Báo cáo nội bộ", "footer_template": "Trang {page}", "page_breaks_on_h1": true })
);

// 19. pdf-table-extractor (FR-DOC-19)
define_doc_suite_tier1_tests!(
    pdf_table_extractor,
    "pdf-table-extractor",
    json!({ "output_format": "xlsx", "detect_merged_cells": true }),
    json!({ "output_format": "csv", "delimiter": "," }),
    json!({ "strategy": "lattice", "extract_empty_rows": false })
);
