//! Bộ kiểm thử tích hợp đầu cuối E2E (End-to-End Test Suite)
//!
//! Phân hệ: Office Utility VN (50 tools qua 4 subsystems)
//! Phương pháp: Opaque-box, requirement-driven (02-srs.md & 03-nfr.md)
//! Cấu trúc: 4-Tier Test Architecture (250 + 250 + 10 + 5 = 515 test cases)

pub mod harness;
pub mod tier1_feature_coverage;
pub mod tier2_boundary_corner;
pub mod tier3_pairwise_combinations;
pub mod tier4_real_world_scenarios;
pub mod desktop_webdriver;

#[test]
fn test_e2e_suite_inventory_verification() {
    // Kiểm tra danh mục 50 công cụ trong ToolRegistry
    assert_eq!(
        harness::registry::ALL_50_TOOL_SLUGS.len(),
        50,
        "Bộ kiểm thử phải bao phủ toàn bộ 50 công cụ thuộc 4 phân hệ"
    );
}
