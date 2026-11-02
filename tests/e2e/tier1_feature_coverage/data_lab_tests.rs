use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use serde_json::json;
use office_shared::DispatchTaskRequest;
use crate::harness::TestHarness;

macro_rules! define_data_lab_tier1_tests {
    ($slug:ident, $slug_str:expr, $happy_opts:expr, $second_opts:expr, $matrix_opts:expr) => {
        paste::paste! {
            #[test]
            fn [<test_ $slug _happy_path>]() {
                let harness = TestHarness::new();
                let input_paths = match $slug_str {
                    "diff-checker-text-sheets" => {
                        let f1 = harness.create_valid_text_file(concat!($slug_str, "_h1.txt"), "Dòng 1: ABC\nDòng 2: XYZ\n");
                        let f2 = harness.create_valid_text_file(concat!($slug_str, "_h2.txt"), "Dòng 1: ABC\nDòng 2: 123\n");
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "list-set-operations" => {
                        let f1 = harness.create_valid_text_file(concat!($slug_str, "_h1.txt"), "Mục 1\nMục 2\nMục 3\n");
                        let f2 = harness.create_valid_text_file(concat!($slug_str, "_h2.txt"), "Mục 2\nMục 3\nMục 4\n");
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "excel-visual-joiner" => {
                        let f1 = harness.create_valid_text_file(
                            concat!($slug_str, "_h1.csv"),
                            "MaNV,EmployeeID,MST,Ten\nNV01,E01,0101234567,An\nNV02,E02,8001234567,Binh\n",
                        );
                        let f2 = harness.create_valid_text_file(
                            concat!($slug_str, "_h2.csv"),
                            "MaNV,ID,MST,PhongBan\nNV01,E01,0101234567,IT\nNV02,E02,8001234567,HR\n",
                        );
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "excel-sheet-unlocker" => {
                        vec![harness.create_valid_xlsx(concat!($slug_str, "_happy.xlsx")).to_string_lossy().to_string()]
                    }
                    "excel-split-by-column" => {
                        let f = harness.create_valid_text_file(
                            concat!($slug_str, "_happy.csv"),
                            "MaNV,Ten,MaPhongBan,KhuVuc\nNV1,An,IT,MienBac\nNV2,Binh,HR,MienNam\n",
                        );
                        vec![f.to_string_lossy().to_string()]
                    }
                    _ => {
                        let f = harness.create_valid_text_file(
                            concat!($slug_str, "_happy.txt"),
                            "Họ và tên,Mã số thuế,Địa chỉ\nNguyễn Văn An,0101234567,Hà Nội\nTrần Thị Bình,8001234567,TP.HCM\n",
                        );
                        vec![f.to_string_lossy().to_string()]
                    }
                };
                let req = DispatchTaskRequest {
                    task_id: format!("{}-happy", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths,
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
                let input_paths = match $slug_str {
                    "base64-file-codec" => {
                        let f = harness.create_valid_text_file(
                            concat!($slug_str, "_second.txt"),
                            "RMOybmcgMTogVGjhu60gbmdoaeG7h20gdMOtbmggbsSDbmc=",
                        );
                        vec![f.to_string_lossy().to_string()]
                    }
                    "diff-checker-text-sheets" => {
                        let f1 = harness.create_valid_text_file(concat!($slug_str, "_s1.txt"), "Dòng 1: Hello\nDòng 2: World\n");
                        let f2 = harness.create_valid_text_file(concat!($slug_str, "_s2.txt"), "Dòng 1: Hello\nDòng 2: Rust\n");
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "list-set-operations" => {
                        let f1 = harness.create_valid_text_file(concat!($slug_str, "_s1.txt"), "Tập hợp A: 1\n2\n3\n");
                        let f2 = harness.create_valid_text_file(concat!($slug_str, "_s2.txt"), "Tập hợp B: 2\n3\n4\n");
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "excel-visual-joiner" => {
                        let f1 = harness.create_valid_text_file(
                            concat!($slug_str, "_s1.csv"),
                            "MaNV,EmployeeID,MST,Ten\nNV01,E01,0101234567,An\nNV02,E02,8001234567,Binh\n",
                        );
                        let f2 = harness.create_valid_text_file(
                            concat!($slug_str, "_s2.csv"),
                            "MaNV,ID,MST,PhongBan\nNV01,E01,0101234567,IT\nNV02,E02,8001234567,HR\n",
                        );
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "excel-sheet-unlocker" => {
                        vec![harness.create_valid_xlsx(concat!($slug_str, "_second.xlsx")).to_string_lossy().to_string()]
                    }
                    "excel-split-by-column" => {
                        let f = harness.create_valid_text_file(
                            concat!($slug_str, "_second.csv"),
                            "MaNV,Ten,MaPhongBan,KhuVuc\nNV1,An,IT,MienBac\nNV2,Binh,HR,MienNam\n",
                        );
                        vec![f.to_string_lossy().to_string()]
                    }
                    _ => {
                        let f = harness.create_valid_text_file(
                            concat!($slug_str, "_second.txt"),
                            "Họ và tên,Mã số thuế,Địa chỉ\nNguyễn Văn An,0101234567,Hà Nội\nTrần Thị Bình,8001234567,TP.HCM\n",
                        );
                        vec![f.to_string_lossy().to_string()]
                    }
                };
                let req = DispatchTaskRequest {
                    task_id: format!("{}-second", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths,
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
                let input_paths = match $slug_str {
                    "diff-checker-text-sheets" => {
                        let f1 = harness.create_valid_text_file(concat!($slug_str, "_b1.txt"), "A1,B1\n1,2\n");
                        let f2 = harness.create_valid_text_file(concat!($slug_str, "_b2.txt"), "A1,B1\n1,3\n");
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "list-set-operations" => {
                        let f1 = harness.create_valid_text_file(concat!($slug_str, "_b1.txt"), "a\nb\nc\n");
                        let f2 = harness.create_valid_text_file(concat!($slug_str, "_b2.txt"), "b\nc\nd\n");
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "excel-visual-joiner" => {
                        let f1 = harness.create_valid_text_file(
                            concat!($slug_str, "_b1.csv"),
                            "MaNV,EmployeeID,MST,Ten\nNV01,E01,0101234567,An\n",
                        );
                        let f2 = harness.create_valid_text_file(
                            concat!($slug_str, "_b2.csv"),
                            "MaNV,ID,MST,PhongBan\nNV01,E01,0101234567,IT\n",
                        );
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "excel-sheet-unlocker" => {
                        let x1 = harness.create_valid_xlsx(concat!($slug_str, "_b1.xlsx"));
                        let x2 = harness.create_valid_xlsx(concat!($slug_str, "_b2.xlsx"));
                        vec![x1.to_string_lossy().to_string(), x2.to_string_lossy().to_string()]
                    }
                    "excel-split-by-column" => {
                        let f1 = harness.create_valid_text_file(
                            concat!($slug_str, "_b1.csv"),
                            "MaNV,Ten,MaPhongBan,KhuVuc\nNV1,An,IT,MienBac\n",
                        );
                        let f2 = harness.create_valid_text_file(
                            concat!($slug_str, "_b2.csv"),
                            "MaNV,Ten,MaPhongBan,KhuVuc\nNV2,Binh,HR,MienNam\n",
                        );
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    _ => {
                        let f1 = harness.create_valid_text_file(
                            concat!($slug_str, "_b1.txt"),
                            "Họ và tên,Mã số thuế,Địa chỉ\nNguyễn Văn An,0101234567,Hà Nội\n",
                        );
                        let f2 = harness.create_valid_text_file(
                            concat!($slug_str, "_b2.txt"),
                            "Họ và tên,Mã số thuế,Địa chỉ\nTrần Thị Bình,8001234567,TP.HCM\n",
                        );
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                };
                let req = DispatchTaskRequest {
                    task_id: format!("{}-batch", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths,
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
                let input_paths = match $slug_str {
                    "diff-checker-text-sheets" => {
                        let f1 = harness.create_valid_text_file(concat!($slug_str, "_o1.txt"), "A\nB\n");
                        let f2 = harness.create_valid_text_file(concat!($slug_str, "_o2.txt"), "A\nC\n");
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "list-set-operations" => {
                        let f1 = harness.create_valid_text_file(concat!($slug_str, "_o1.txt"), "X\nY\n");
                        let f2 = harness.create_valid_text_file(concat!($slug_str, "_o2.txt"), "Y\nZ\n");
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "excel-visual-joiner" => {
                        let f1 = harness.create_valid_text_file(
                            concat!($slug_str, "_o1.csv"),
                            "MaNV,EmployeeID,MST,Ten\nNV01,E01,0101234567,An\n",
                        );
                        let f2 = harness.create_valid_text_file(
                            concat!($slug_str, "_o2.csv"),
                            "MaNV,ID,MST,PhongBan\nNV01,E01,0101234567,IT\n",
                        );
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "excel-sheet-unlocker" => {
                        vec![harness.create_valid_xlsx(concat!($slug_str, "_opt.xlsx")).to_string_lossy().to_string()]
                    }
                    "excel-split-by-column" => {
                        let f = harness.create_valid_text_file(
                            concat!($slug_str, "_opt.csv"),
                            "MaNV,Ten,MaPhongBan,KhuVuc\nNV1,An,IT,MienBac\nNV2,Binh,HR,MienNam\n",
                        );
                        vec![f.to_string_lossy().to_string()]
                    }
                    _ => {
                        let f = harness.create_valid_text_file(
                            concat!($slug_str, "_opt.txt"),
                            "Họ và tên,Mã số thuế,Địa chỉ\nNguyễn Văn An,0101234567,Hà Nội\n",
                        );
                        vec![f.to_string_lossy().to_string()]
                    }
                };
                let req = DispatchTaskRequest {
                    task_id: format!("{}-opt", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths,
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
                let input_paths = match $slug_str {
                    "diff-checker-text-sheets" => {
                        let f1 = harness.create_valid_text_file(concat!($slug_str, "_p1.txt"), "P1\n");
                        let f2 = harness.create_valid_text_file(concat!($slug_str, "_p2.txt"), "P2\n");
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "list-set-operations" => {
                        let f1 = harness.create_valid_text_file(concat!($slug_str, "_p1.txt"), "P1\n");
                        let f2 = harness.create_valid_text_file(concat!($slug_str, "_p2.txt"), "P2\n");
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "excel-visual-joiner" => {
                        let f1 = harness.create_valid_text_file(
                            concat!($slug_str, "_p1.csv"),
                            "MaNV,EmployeeID,MST,Ten\nNV01,E01,0101234567,An\n",
                        );
                        let f2 = harness.create_valid_text_file(
                            concat!($slug_str, "_p2.csv"),
                            "MaNV,ID,MST,PhongBan\nNV01,E01,0101234567,IT\n",
                        );
                        vec![f1.to_string_lossy().to_string(), f2.to_string_lossy().to_string()]
                    }
                    "excel-sheet-unlocker" => {
                        vec![harness.create_valid_xlsx(concat!($slug_str, "_prog.xlsx")).to_string_lossy().to_string()]
                    }
                    "excel-split-by-column" => {
                        let f = harness.create_valid_text_file(
                            concat!($slug_str, "_prog.csv"),
                            "MaNV,Ten,MaPhongBan,KhuVuc\nNV1,An,IT,MienBac\n",
                        );
                        vec![f.to_string_lossy().to_string()]
                    }
                    _ => {
                        let f = harness.create_valid_text_file(
                            concat!($slug_str, "_prog.txt"),
                            "Họ và tên,Mã số thuế,Địa chỉ\nNguyễn Văn An,0101234567,Hà Nội\n",
                        );
                        vec![f.to_string_lossy().to_string()]
                    }
                };
                let req = DispatchTaskRequest {
                    task_id: format!("{}-prog", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths,
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

// 29. vietnamese-font-converter (FR-DAT-01)
define_data_lab_tier1_tests!(
    vietnamese_font_converter,
    "vietnamese-font-converter",
    json!({ "source_encoding": "TCVN3", "target_encoding": "Unicode", "text": "Céng hoµ x· héi chñ nghÜa ViÖt Nam" }),
    json!({ "source_encoding": "VNI", "target_encoding": "Unicode", "text": "Coäng hoøa xaõ hoäi chuû nghóa Vieät Nam" }),
    json!({ "source_encoding": "Unicode-NFD", "target_encoding": "Unicode-NFC" })
);

// 30. excel-cleaner-normalizer (FR-DAT-02)
define_data_lab_tier1_tests!(
    excel_cleaner_normalizer,
    "excel-cleaner-normalizer",
    json!({ "trim_whitespace": true, "normalize_dates": true, "date_format": "DD/MM/YYYY" }),
    json!({ "capitalize_names": true, "split_full_name": true }),
    json!({ "deduplicate_rows": true, "fill_empty_cells": "N/A" })
);

// 31. diff-checker-text-sheets (FR-DAT-03)
define_data_lab_tier1_tests!(
    diff_checker_text_sheets,
    "diff-checker-text-sheets",
    json!({ "mode": "line", "algorithm": "myers", "ignore_case": false }),
    json!({ "mode": "word", "algorithm": "patience", "ignore_whitespace": true }),
    json!({ "mode": "semantic", "color_mode": "unified" })
);

// 32. data-structure-converter (FR-DAT-04)
define_data_lab_tier1_tests!(
    data_structure_converter,
    "data-structure-converter",
    json!({ "source_format": "csv", "target_format": "json_array" }),
    json!({ "source_format": "json", "target_format": "sql_insert", "table_name": "employees" }),
    json!({ "source_format": "csv", "target_format": "markdown_table" })
);

// 33. excel-split-by-column (FR-DAT-05)
define_data_lab_tier1_tests!(
    excel_split_by_column,
    "excel-split-by-column",
    json!({ "split_column": "MaPhongBan", "preserve_header": true }),
    json!({ "split_column_index": 2, "output_prefix": "dept_" }),
    json!({ "split_column": "KhuVuc", "sanitize_sheet_names": true })
);

// 34. excel-multi-merger (FR-DAT-06)
define_data_lab_tier1_tests!(
    excel_multi_merger,
    "excel-multi-merger",
    json!({ "merge_mode": "append_rows", "match_header_names": true, "add_source_column": true }),
    json!({ "merge_mode": "combine_sheets", "rename_duplicates": true }),
    json!({ "merge_mode": "append_rows", "skip_empty_sheets": true })
);

// 35. excel-visual-joiner (FR-DAT-07)
define_data_lab_tier1_tests!(
    excel_visual_joiner,
    "excel-visual-joiner",
    json!({ "join_type": "inner", "left_key": "MaNV", "right_key": "MaNV" }),
    json!({ "join_type": "left_outer", "left_key": "EmployeeID", "right_key": "ID" }),
    json!({ "join_type": "anti_join", "left_key": "MST", "right_key": "MST" })
);

// 36. excel-sheet-unlocker (FR-DAT-08)
define_data_lab_tier1_tests!(
    excel_sheet_unlocker,
    "excel-sheet-unlocker",
    json!({ "strip_protection": true, "unlock_all_sheets": true }),
    json!({ "target_sheets": ["Sheet1", "BaoCao"] }),
    json!({ "remove_workbook_password": true })
);

// 37. list-set-operations (FR-DAT-09)
define_data_lab_tier1_tests!(
    list_set_operations,
    "list-set-operations",
    json!({ "operation": "union", "case_sensitive": false, "trim_lines": true }),
    json!({ "operation": "intersection" }),
    json!({ "operation": "difference", "a_minus_b": true })
);

// 38. encoding-mojibake-fixer (FR-DAT-10)
define_data_lab_tier1_tests!(
    encoding_mojibake_fixer,
    "encoding-mojibake-fixer",
    json!({ "expected_language": "vi", "auto_detect_scramble": true }),
    json!({ "source_guess": "windows-1252", "target": "utf-8" }),
    json!({ "source_guess": "iso-8859-1", "deep_heuristic": true })
);

// 39. text-pii-anonymizer (FR-DAT-11)
define_data_lab_tier1_tests!(
    text_pii_anonymizer,
    "text-pii-anonymizer",
    json!({ "mask_cccd": true, "mask_phone": true, "mask_email": true }),
    json!({ "replacement_strategy": "hash_sha256", "mask_tax_id": true }),
    json!({ "replacement_strategy": "star_redact", "preserve_last_4_digits": true })
);

// 40. sql-formatter-linter (FR-DAT-12)
define_data_lab_tier1_tests!(
    sql_formatter_linter,
    "sql-formatter-linter",
    json!({ "dialect": "postgresql", "uppercase_keywords": true, "indent_size": 4 }),
    json!({ "dialect": "sqlite", "uppercase_keywords": false, "align_expressions": true }),
    json!({ "dialect": "mysql", "comma_first": false, "lines_between_queries": 2 })
);

// 41. base64-file-codec (FR-DAT-13)
define_data_lab_tier1_tests!(
    base64_file_codec,
    "base64-file-codec",
    json!({ "mode": "encode", "emit_data_uri": true, "mime_type": "text/plain" }),
    json!({ "mode": "decode", "safe_padding": true }),
    json!({ "mode": "encode", "chunk_lines": 76, "url_safe": false })
);
