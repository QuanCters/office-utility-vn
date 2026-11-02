use std::sync::atomic::AtomicBool;
use serde_json::json;
use office_shared::{CommandError, DispatchTaskRequest};
use crate::harness::TestHarness;

macro_rules! define_doc_suite_tier2_boundary_tests {
    ($slug:ident, $slug_str:expr, $valid_opts:expr) => {
        paste::paste! {
            #[test]
            fn [<test_ $slug _invalid_magic_bytes>]() {
                let harness = TestHarness::new();
                let bad = harness.create_corrupt_file(concat!($slug_str, "_bad.pdf"), b"NOT_A_PDF_HEADER_DEADBEEF");
                harness.assert_invalid_signature($slug_str, &bad, $valid_opts);
            }

            #[test]
            fn [<test_ $slug _empty_inputs>]() {
                let harness = TestHarness::new();
                if let Some(tool) = harness.get_tool($slug_str) {
                    let req = DispatchTaskRequest {
                        task_id: format!("{}-empty", $slug_str),
                        tool_slug: $slug_str.to_string(),
                        input_paths: vec![],
                        output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                        options: $valid_opts,
                    };
                    let cancel = AtomicBool::new(false);
                    let res = tool.execute(&req, &cancel, None);
                    match res {
                        Err(CommandError::ValidationFailed { .. }) => {}
                        other => {
                            panic!("Kỳ vọng ValidationFailed khi input_paths trống, nhưng nhận: {:?}", other);
                        }
                    }
                }
            }

            #[test]
            fn [<test_ $slug _immediate_cancellation>]() {
                let harness = TestHarness::new();
                let pdf = harness.create_valid_pdf(concat!($slug_str, "_cancel.pdf"), 2);
                let req = DispatchTaskRequest {
                    task_id: format!("{}-cancel-imm", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![pdf.to_string_lossy().to_string()],
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $valid_opts,
                };
                if let Some(tool) = harness.get_tool($slug_str) {
                    harness.assert_cancellation_under_100ms(|cancel| {
                        tool.execute(&req, cancel, None)
                    });
                }
            }

            #[test]
            fn [<test_ $slug _midflight_cancellation>]() {
                let harness = TestHarness::new();
                let pdf = harness.create_valid_pdf(concat!($slug_str, "_midcancel.pdf"), 5);
                let req = DispatchTaskRequest {
                    task_id: format!("{}-cancel-mid", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![pdf.to_string_lossy().to_string()],
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $valid_opts,
                };
                if let Some(tool) = harness.get_tool($slug_str) {
                    let cancel = AtomicBool::new(false);
                    let start = std::time::Instant::now();
                    let cb = |_: office_shared::TaskProgress| {
                        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                    };
                    let res = tool.execute(&req, &cancel, Some(&cb));
                    let elapsed = start.elapsed();
                    assert!(elapsed.as_millis() < 500, "Cooperative cancellation must react rapidly");
                    if let Err(CommandError::TaskCancelled) = res {
                        // Hủy thành công
                    }
                }
            }

            #[test]
            fn [<test_ $slug _extreme_boundary>]() {
                let harness = TestHarness::new();
                let pdf = harness.create_valid_pdf(concat!($slug_str, "_extreme.pdf"), 1);
                let req = DispatchTaskRequest {
                    task_id: format!("{}-extreme", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![pdf.to_string_lossy().to_string()],
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: json!({
                        "page_ranges": "1-999999",
                        "watermark_text": "CỘNG HÒA XÃ HỘI CHỦ NGHĨA VIỆT NAM - ĐỘC LẬP TỰ DO HẠNH PHÚC 🇻🇳 1234567890 !@#$%^&*()",
                        "password": "🔐Mật_Khẩu_Cực_Dài_Có_Ký_Tự_Đặc_Biệt_1234567890!@#$%",
                        "rotation_deg": 360,
                        "dpi": 1200
                    }),
                };
                let cancel = AtomicBool::new(false);
                if let Some(tool) = harness.get_tool($slug_str) {
                    let _ = tool.execute(&req, &cancel, None);
                }
            }
        }
    };
}

// 19 tools trong Document Suite
define_doc_suite_tier2_boundary_tests!(pdf_merge_split, "pdf-merge-split", json!({ "mode": "merge" }));
define_doc_suite_tier2_boundary_tests!(pdf_protect_sign, "pdf-protect-sign", json!({ "password": "Pass" }));
define_doc_suite_tier2_boundary_tests!(pdf_to_markdown, "pdf-to-markdown", json!({ "extract_images": false }));
define_doc_suite_tier2_boundary_tests!(pdf_compress, "pdf-compress", json!({ "compression_level": "medium" }));
define_doc_suite_tier2_boundary_tests!(vietnamese_ocr, "vietnamese-ocr", json!({ "mode": "fast" }));
define_doc_suite_tier2_boundary_tests!(pdf_watermark_stamp, "pdf-watermark-stamp", json!({ "watermark_text": "TEST" }));
define_doc_suite_tier2_boundary_tests!(pdf_page_number_header, "pdf-page-number-header", json!({ "format": "Trang {page}" }));
define_doc_suite_tier2_boundary_tests!(pdf_metadata_scrubber, "pdf-metadata-scrubber", json!({ "strip_all": true }));
define_doc_suite_tier2_boundary_tests!(pdf_flatten_forms, "pdf-flatten-forms", json!({ "flatten_annotations": true }));
define_doc_suite_tier2_boundary_tests!(pdf_crop_margin, "pdf-crop-margin", json!({ "auto_detect_whitespace": true }));
define_doc_suite_tier2_boundary_tests!(pdf_booklet_imposition, "pdf-booklet-imposition", json!({ "layout": "saddle-stitch" }));
define_doc_suite_tier2_boundary_tests!(pdf_visual_diff, "pdf-visual-diff", json!({ "mode": "text-diff" }));
define_doc_suite_tier2_boundary_tests!(docx_template_mailmerge, "docx-template-mailmerge", json!({ "data_source": "inline" }));
define_doc_suite_tier2_boundary_tests!(pdf_bookmark_toc, "pdf-bookmark-toc", json!({ "auto_generate_from_headings": true }));
define_doc_suite_tier2_boundary_tests!(pdf_to_long_image, "pdf-to-long-image", json!({ "format": "png" }));
define_doc_suite_tier2_boundary_tests!(pdf_grayscale_print, "pdf-grayscale-print", json!({ "convert_images": true }));
define_doc_suite_tier2_boundary_tests!(pdf_repair_corrupt, "pdf-repair-corrupt", json!({ "rebuild_xref": true }));
define_doc_suite_tier2_boundary_tests!(markdown_to_styled_pdf, "markdown-to-styled-pdf", json!({ "theme": "business" }));
define_doc_suite_tier2_boundary_tests!(pdf_table_extractor, "pdf-table-extractor", json!({ "output_format": "xlsx" }));
