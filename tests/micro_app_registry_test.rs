use office_utility_vn_ui::app::ALL_TOOLS;
use office_utility_vn_ui::micro_apps::{
    micro_app_tools, tool_slug_to_micro_app, MicroAppId, ToolRegistration, ToolRegistry,
};

#[test]
fn test_all_58_tools_registered_in_registry_entries() {
    assert_eq!(
        ToolRegistry::ENTRIES.len(),
        58,
        "ToolRegistry::ENTRIES phải chứa đúng 58 công cụ"
    );

    // Mọi công cụ trong ALL_TOOLS phải xuất hiện trong ToolRegistry::ENTRIES với cùng slug
    for tool in ALL_TOOLS.iter() {
        let entry = ToolRegistry::ENTRIES
            .iter()
            .find(|e| e.tool.slug == tool.slug);
        assert!(
            entry.is_some(),
            "Công cụ '{}' thiếu trong ToolRegistry::ENTRIES",
            tool.slug
        );
        let entry = entry.unwrap();
        assert_eq!(entry.tool.name, tool.name);
        assert_eq!(entry.tool.domain, tool.domain);
    }
}

#[test]
fn test_lookup_app_for_all_58_slugs() {
    for tool in ALL_TOOLS.iter() {
        let app = ToolRegistry::lookup_app(tool.slug);
        assert!(
            app.is_some(),
            "Slug '{}' phải tra cứu được MicroAppId từ ToolRegistry::lookup_app",
            tool.slug
        );
    }

    // Tra cứu slug không tồn tại trả về None
    assert_eq!(ToolRegistry::lookup_app("non-existent-tool-xyz"), None);
    assert_eq!(ToolRegistry::lookup_app(""), None);

    // Alias universal-converter cũng ánh xạ đúng về DataLab
    assert_eq!(
        ToolRegistry::lookup_app("universal-converter"),
        Some(MicroAppId::DataLab)
    );
}

#[test]
fn test_count_for_app_matches_exact_distribution() {
    assert_eq!(
        ToolRegistry::count_for_app(MicroAppId::VnOffice),
        11,
        "VN Office phải có chính xác 11 công cụ"
    );
    assert_eq!(
        ToolRegistry::count_for_app(MicroAppId::PdfStudio),
        25,
        "PDF Studio phải có chính xác 25 công cụ"
    );
    assert_eq!(
        ToolRegistry::count_for_app(MicroAppId::MediaStudio),
        9,
        "Media Studio phải có chính xác 9 công cụ"
    );
    assert_eq!(
        ToolRegistry::count_for_app(MicroAppId::DataLab),
        13,
        "Data Lab phải có chính xác 13 công cụ"
    );

    let total: usize = MicroAppId::ALL
        .iter()
        .map(|&app| ToolRegistry::count_for_app(app))
        .sum();
    assert_eq!(total, 58, "Tổng số công cụ phân bổ cho 4 Micro-Apps phải là 58");
}

#[test]
fn test_tools_for_app_and_all_tools() {
    let all = ToolRegistry::all_tools();
    assert_eq!(all.len(), 58, "ToolRegistry::all_tools() phải trả về 58 công cụ");

    let vn_tools = ToolRegistry::tools_for_app(MicroAppId::VnOffice);
    assert_eq!(vn_tools.len(), 11);
    for t in &vn_tools {
        assert_eq!(ToolRegistry::lookup_app(t.slug), Some(MicroAppId::VnOffice));
    }

    let pdf_tools = ToolRegistry::tools_for_app(MicroAppId::PdfStudio);
    assert_eq!(pdf_tools.len(), 25);
    for t in &pdf_tools {
        assert_eq!(ToolRegistry::lookup_app(t.slug), Some(MicroAppId::PdfStudio));
    }

    let media_tools = ToolRegistry::tools_for_app(MicroAppId::MediaStudio);
    assert_eq!(media_tools.len(), 9);
    for t in &media_tools {
        assert_eq!(ToolRegistry::lookup_app(t.slug), Some(MicroAppId::MediaStudio));
    }

    let data_tools = ToolRegistry::tools_for_app(MicroAppId::DataLab);
    assert_eq!(data_tools.len(), 13);
    for t in &data_tools {
        assert_eq!(ToolRegistry::lookup_app(t.slug), Some(MicroAppId::DataLab));
    }
}

#[test]
fn test_tools_by_tag() {
    // Tìm kiếm các công cụ theo tag
    let vn_tagged = ToolRegistry::tools_by_tag("vn-office");
    assert!(!vn_tagged.is_empty(), "Phải có công cụ mang tag 'vn-office'");

    let pdf_tagged = ToolRegistry::tools_by_tag("pdf");
    assert!(!pdf_tagged.is_empty(), "Phải có công cụ mang tag 'pdf'");

    let non_existent = ToolRegistry::tools_by_tag("tag-khong-ton-tai");
    assert!(non_existent.is_empty(), "Tag không tồn tại phải trả về rỗng");
}

#[test]
fn test_secondary_apps_feature() {
    // Kiểm tra cấu trúc ToolRegistration và logic secondary_apps
    let mock_registration = ToolRegistration {
        tool: office_utility_vn_ui::app::ToolItem {
            slug: "mock-tool",
            name: "Mock Tool",
            domain: "vn-office",
            description: "Mô tả kiểm thử",
            is_pinned: false,
        },
        primary_app: MicroAppId::VnOffice,
        secondary_apps: &[MicroAppId::DataLab],
        tags: &["mock", "test"],
    };

    assert_eq!(mock_registration.primary_app, MicroAppId::VnOffice);
    assert!(mock_registration.secondary_apps.contains(&MicroAppId::DataLab));
    assert!(mock_registration.tags.contains(&"mock"));
}

#[test]
fn test_legacy_api_delegation_compatibility() {
    // 1. tool_slug_to_micro_app khớp 100% với ToolRegistry::lookup_app
    for tool in ALL_TOOLS.iter() {
        assert_eq!(
            tool_slug_to_micro_app(tool.slug),
            ToolRegistry::lookup_app(tool.slug),
            "tool_slug_to_micro_app({}) phải khớp với ToolRegistry::lookup_app",
            tool.slug
        );
    }

    // 2. micro_app_tools(app) khớp số lượng và danh sách với ToolRegistry::tools_for_app(app)
    for &app in &MicroAppId::ALL {
        let legacy_tools = micro_app_tools(app);
        let registry_tools = ToolRegistry::tools_for_app(app);
        assert_eq!(legacy_tools.len(), registry_tools.len());
        for (legacy, registry) in legacy_tools.iter().zip(registry_tools.iter()) {
            assert_eq!(legacy.slug, registry.slug);
            assert_eq!(legacy.name, registry.name);
        }
    }

    // 3. MicroAppId::metadata().tool_count phải khớp với ToolRegistry::count_for_app
    for &app in &MicroAppId::ALL {
        assert_eq!(
            app.metadata().tool_count,
            ToolRegistry::count_for_app(app),
            "MicroAppId::metadata().tool_count cho {:?} phải khớp ToolRegistry::count_for_app",
            app
        );
        assert_eq!(app.tool_count(), ToolRegistry::count_for_app(app));
    }
}
