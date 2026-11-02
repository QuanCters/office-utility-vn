use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use serde_json::json;
use office_shared::DispatchTaskRequest;
use crate::harness::TestHarness;

macro_rules! define_vn_office_tier1_tests {
    ($slug:ident, $slug_str:expr, $happy_opts:expr, $second_opts:expr, $matrix_opts:expr) => {
        paste::paste! {
            #[test]
            fn [<test_ $slug _happy_path>]() {
                let harness = TestHarness::new();
                let invoice = harness.create_valid_xml_invoice(concat!($slug_str, "_happy.xml"));
                let req = DispatchTaskRequest {
                    task_id: format!("{}-happy", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![invoice.to_string_lossy().to_string()],
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
                let invoice = harness.create_valid_xml_invoice(concat!($slug_str, "_second.xml"));
                let req = DispatchTaskRequest {
                    task_id: format!("{}-second", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![invoice.to_string_lossy().to_string()],
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
                let inv1 = harness.create_valid_xml_invoice(concat!($slug_str, "_b1.xml"));
                let inv2 = harness.create_valid_xml_invoice(concat!($slug_str, "_b2.xml"));
                let req = DispatchTaskRequest {
                    task_id: format!("{}-batch", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![
                        inv1.to_string_lossy().to_string(),
                        inv2.to_string_lossy().to_string(),
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
                let invoice = harness.create_valid_xml_invoice(concat!($slug_str, "_opt.xml"));
                let req = DispatchTaskRequest {
                    task_id: format!("{}-opt", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![invoice.to_string_lossy().to_string()],
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
                let invoice = harness.create_valid_xml_invoice(concat!($slug_str, "_prog.xml"));
                let req = DispatchTaskRequest {
                    task_id: format!("{}-prog", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![invoice.to_string_lossy().to_string()],
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

// 42. vietnamese-number-to-words (FR-VNO-01)
define_vn_office_tier1_tests!(
    vietnamese_number_to_words,
    "vietnamese-number-to-words",
    json!({ "number": "15000000", "currency": "VND", "dialect": "North", "capitalize_first": true }),
    json!({ "number": "1000000000", "currency": "VND", "dialect": "South", "use_linh": true }),
    json!({ "number": "250.75", "currency": "USD", "dialect": "North" })
);

// 43. gross-net-salary-calculator (FR-VNO-02)
define_vn_office_tier1_tests!(
    gross_net_salary_calculator,
    "gross-net-salary-calculator",
    json!({ "mode": "gross_to_net", "salary": 25000000, "region": 1, "dependents": 1 }),
    json!({ "mode": "net_to_gross", "salary": 20000000, "region": 1, "dependents": 0 }),
    json!({ "mode": "gross_to_net", "salary": 50000000, "region": 2, "dependents": 2, "apply_bhxh_cap": true })
);

// 44. vietqr-napas247-generator (FR-VNO-03)
define_vn_office_tier1_tests!(
    vietqr_napas247_generator,
    "vietqr-napas247-generator",
    json!({ "bank_bin": "970415", "account_number": "1012345678", "amount": 500000, "description": "THANH TOAN TIEN NUOC" }),
    json!({ "bank_bin": "970422", "account_number": "0071000123456", "amount": 1250000, "description": "HOC PHI KY 1" }),
    json!({ "bank_bin": "970436", "account_number": "0123456789", "output_format": "png", "qr_size": 512 })
);

// 45. vn-einvoice-xml-reader (FR-VNO-04)
define_vn_office_tier1_tests!(
    vn_einvoice_xml_reader,
    "vn-einvoice-xml-reader",
    json!({ "validate_schema": true, "export_summary": true }),
    json!({ "extract_items_table": true, "export_format": "json" }),
    json!({ "verify_digital_signature": true, "strict_circular78": true })
);

// 46. vn-cccd-mst-validator (FR-VNO-05)
define_vn_office_tier1_tests!(
    vn_cccd_mst_validator,
    "vn-cccd-mst-validator",
    json!({ "query": "001090012345", "type": "cccd" }),
    json!({ "query": "0101234567", "type": "mst_enterprise" }),
    json!({ "query": "8001234567-001", "type": "mst_branch", "validate_checksum": true })
);

// 47. vn-address-normalizer (FR-VNO-06)
define_vn_office_tier1_tests!(
    vn_address_normalizer,
    "vn-address-normalizer",
    json!({ "raw_address": "Số 10 Tràng Tiền, Hoàn Kiếm, HN", "target_model": "3_tier" }),
    json!({ "raw_address": "123 Đinh Tiên Hoàng, P. Đa Kao, Q.1, TP HCM", "target_model": "3_tier" }),
    json!({ "raw_address": "Khu công nghiệp VSIP, Thuận An, Bình Dương", "fuzzy_threshold": 0.8 })
);

// 48. vn-working-days-calc (FR-VNO-07)
define_vn_office_tier1_tests!(
    vn_working_days_calc,
    "vn-working-days-calc",
    json!({ "start_date": "2026-01-01", "end_date": "2026-12-31", "exclude_weekends": true, "exclude_vn_holidays": true }),
    json!({ "start_date": "2026-04-25", "end_date": "2026-05-05", "include_lunar_tet": true }),
    json!({ "start_date": "2026-09-01", "end_date": "2026-09-10", "generate_ics": true })
);

// 49. vn-loan-amortization (FR-VNO-08)
define_vn_office_tier1_tests!(
    vn_loan_amortization,
    "vn-loan-amortization",
    json!({ "principal": 500000000, "annual_rate_percent": 8.5, "term_months": 60, "method": "reducing_balance" }),
    json!({ "principal": 1000000000, "annual_rate_percent": 9.0, "term_months": 120, "method": "annuity" }),
    json!({ "principal": 200000000, "preferential_rate": 6.5, "preferential_months": 12, "floating_rate": 10.5 })
);

// 50. vn-bank-bin-mapper (FR-VNO-09)
define_vn_office_tier1_tests!(
    vn_bank_bin_mapper,
    "vn-bank-bin-mapper",
    json!({ "query": "VCB", "lookup_by": "short_name" }),
    json!({ "query": "970415", "lookup_by": "bin" }),
    json!({ "query": "VietinBank", "include_citad": true, "include_swift": true })
);
