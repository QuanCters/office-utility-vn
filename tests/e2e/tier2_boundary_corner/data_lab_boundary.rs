use std::sync::atomic::AtomicBool;
use serde_json::json;
use office_shared::{CommandError, DispatchTaskRequest};
use crate::harness::TestHarness;

macro_rules! define_data_lab_tier2_boundary_tests {
    ($slug:ident, $slug_str:expr, $valid_opts:expr) => {
        paste::paste! {
            #[test]
            fn [<test_ $slug _invalid_magic_bytes>]() {
                let harness = TestHarness::new();
                let bad = harness.create_corrupt_file(concat!($slug_str, "_bad.dat"), b"\x7fELF\x02\x01\x01\x00_INVALID_BINARY_EXECUTABLE_");
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
                let f = harness.create_valid_text_file(concat!($slug_str, "_cancel.txt"), "Dữ liệu mẫu kiểm tra hủy tác vụ");
                let req = DispatchTaskRequest {
                    task_id: format!("{}-cancel-imm", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![f.to_string_lossy().to_string()],
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
                let f = harness.create_valid_text_file(concat!($slug_str, "_midcancel.txt"), "Dữ liệu dòng 1\nDữ liệu dòng 2\nDữ liệu dòng 3");
                let req = DispatchTaskRequest {
                    task_id: format!("{}-cancel-mid", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![f.to_string_lossy().to_string()],
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
                let extreme_payload = "a".repeat(10000) + " Tiếng Việt có dấu: Ắ Ằ Ẳ Ẵ Ặ Ẹ Ẻ Ẽ Ê Ề Ể Ễ Ệ Ớ Ờ Ở Ỡ Ợ Ụ Ủ Ứ Ừ Ử Ữ Ự Ỳ Ỷ Ỹ ";
                let f = harness.create_valid_text_file(concat!($slug_str, "_extreme.txt"), &extreme_payload);
                let req = DispatchTaskRequest {
                    task_id: format!("{}-extreme", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![f.to_string_lossy().to_string()],
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: json!({
                        "text": extreme_payload,
                        "source_encoding": "TCVN3",
                        "target_encoding": "Unicode",
                        "trim_whitespace": true
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

// 13 tools trong Data Lab
define_data_lab_tier2_boundary_tests!(vietnamese_font_converter, "vietnamese-font-converter", json!({ "source_encoding": "TCVN3", "target_encoding": "Unicode" }));
define_data_lab_tier2_boundary_tests!(excel_cleaner_normalizer, "excel-cleaner-normalizer", json!({ "trim_whitespace": true }));
define_data_lab_tier2_boundary_tests!(diff_checker_text_sheets, "diff-checker-text-sheets", json!({ "mode": "line" }));
define_data_lab_tier2_boundary_tests!(data_structure_converter, "data-structure-converter", json!({ "source_format": "csv", "target_format": "json_array" }));
define_data_lab_tier2_boundary_tests!(excel_split_by_column, "excel-split-by-column", json!({ "split_column": "ID" }));
define_data_lab_tier2_boundary_tests!(excel_multi_merger, "excel-multi-merger", json!({ "merge_mode": "append_rows" }));
define_data_lab_tier2_boundary_tests!(excel_visual_joiner, "excel-visual-joiner", json!({ "join_type": "inner" }));
define_data_lab_tier2_boundary_tests!(excel_sheet_unlocker, "excel-sheet-unlocker", json!({ "strip_protection": true }));
define_data_lab_tier2_boundary_tests!(list_set_operations, "list-set-operations", json!({ "operation": "union" }));
define_data_lab_tier2_boundary_tests!(encoding_mojibake_fixer, "encoding-mojibake-fixer", json!({ "expected_language": "vi" }));
define_data_lab_tier2_boundary_tests!(text_pii_anonymizer, "text-pii-anonymizer", json!({ "mask_cccd": true }));
define_data_lab_tier2_boundary_tests!(sql_formatter_linter, "sql-formatter-linter", json!({ "dialect": "postgresql" }));
define_data_lab_tier2_boundary_tests!(base64_file_codec, "base64-file-codec", json!({ "mode": "encode" }));
