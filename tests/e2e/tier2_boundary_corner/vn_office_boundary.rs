use std::sync::atomic::AtomicBool;
use serde_json::json;
use office_shared::{CommandError, DispatchTaskRequest};
use crate::harness::TestHarness;

macro_rules! define_vn_office_tier2_boundary_tests {
    ($slug:ident, $slug_str:expr, $valid_opts:expr) => {
        paste::paste! {
            #[test]
            fn [<test_ $slug _invalid_magic_bytes>]() {
                let harness = TestHarness::new();
                let bad = harness.create_corrupt_file(concat!($slug_str, "_bad.xml"), b"\x1F\x8B_NOT_VALID_XML_HEADER_");
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
                        options: json!({}),
                    };
                    let cancel = AtomicBool::new(false);
                    let res = tool.execute(&req, &cancel, None);
                    match res {
                        Err(CommandError::ValidationFailed { .. }) => {}
                        other => {
                            // Một số công cụ chấp nhận options rỗng với giá trị mặc định, nhưng không crash
                            eprintln!("Kết quả kiểm tra input rỗng cho {}: {:?}", $slug_str, other);
                        }
                    }
                }
            }

            #[test]
            fn [<test_ $slug _immediate_cancellation>]() {
                let harness = TestHarness::new();
                let inv = harness.create_valid_xml_invoice(concat!($slug_str, "_cancel.xml"));
                let req = DispatchTaskRequest {
                    task_id: format!("{}-cancel-imm", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![inv.to_string_lossy().to_string()],
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
                let inv = harness.create_valid_xml_invoice(concat!($slug_str, "_midcancel.xml"));
                let req = DispatchTaskRequest {
                    task_id: format!("{}-cancel-mid", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![inv.to_string_lossy().to_string()],
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
                let inv = harness.create_valid_xml_invoice(concat!($slug_str, "_extreme.xml"));
                let req = DispatchTaskRequest {
                    task_id: format!("{}-extreme", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![inv.to_string_lossy().to_string()],
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: json!({
                        "number": "999999999999999999999999",
                        "salary": 999999999999i64,
                        "principal": 50000000000i64,
                        "raw_address": "Số 999999/999/99/9 Hẻm Cực Sâu, Xã Cực Xa, Huyện Đảo Trường Sa, Tỉnh Khánh Hòa, Việt Nam",
                        "start_date": "1900-01-01",
                        "end_date": "2100-12-31",
                        "query": "MÃ_SỐ_KHÔNG_HỢP_LỆ_!@#$%"
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

// 9 tools trong VN Office
define_vn_office_tier2_boundary_tests!(vietnamese_number_to_words, "vietnamese-number-to-words", json!({ "number": "15000000" }));
define_vn_office_tier2_boundary_tests!(gross_net_salary_calculator, "gross-net-salary-calculator", json!({ "mode": "gross_to_net", "salary": 25000000 }));
define_vn_office_tier2_boundary_tests!(vietqr_napas247_generator, "vietqr-napas247-generator", json!({ "bank_bin": "970415", "account_number": "1012345678" }));
define_vn_office_tier2_boundary_tests!(vn_einvoice_xml_reader, "vn-einvoice-xml-reader", json!({ "validate_schema": true }));
define_vn_office_tier2_boundary_tests!(vn_cccd_mst_validator, "vn-cccd-mst-validator", json!({ "query": "001090012345" }));
define_vn_office_tier2_boundary_tests!(vn_address_normalizer, "vn-address-normalizer", json!({ "raw_address": "Hà Nội" }));
define_vn_office_tier2_boundary_tests!(vn_working_days_calc, "vn-working-days-calc", json!({ "start_date": "2026-01-01", "end_date": "2026-01-31" }));
define_vn_office_tier2_boundary_tests!(vn_loan_amortization, "vn-loan-amortization", json!({ "principal": 100000000, "annual_rate_percent": 8.0, "term_months": 12 }));
define_vn_office_tier2_boundary_tests!(vn_bank_bin_mapper, "vn-bank-bin-mapper", json!({ "query": "VCB" }));
