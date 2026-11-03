use office_utility_vn_ui::app::ALL_TOOLS;
use office_utility_vn_ui::micro_apps::{
    micro_app_tools, tool_slug_to_micro_app, MicroAppId,
};

#[test]
fn test_vn_office_routing() {
    assert_eq!(
        tool_slug_to_micro_app("vietnamese-number-to-words"),
        Some(MicroAppId::VnOffice)
    );
    assert_eq!(
        tool_slug_to_micro_app("vietnamese-font-converter"),
        Some(MicroAppId::VnOffice)
    );
    assert_eq!(
        tool_slug_to_micro_app("encoding-mojibake-fixer"),
        Some(MicroAppId::VnOffice)
    );
}

#[test]
fn test_pdf_studio_routing() {
    assert_eq!(
        tool_slug_to_micro_app("pdf-merge-split"),
        Some(MicroAppId::PdfStudio)
    );
    assert_eq!(
        tool_slug_to_micro_app("vietnamese-ocr"),
        Some(MicroAppId::PdfStudio)
    );
}

#[test]
fn test_media_studio_routing() {
    assert_eq!(
        tool_slug_to_micro_app("image-batch"),
        Some(MicroAppId::MediaStudio)
    );
    assert_eq!(
        tool_slug_to_micro_app("screenshot-beautifier"),
        Some(MicroAppId::MediaStudio)
    );
}

#[test]
fn test_data_lab_routing() {
    assert_eq!(
        tool_slug_to_micro_app("excel-cleaner-normalizer"),
        Some(MicroAppId::DataLab)
    );
    assert_eq!(
        tool_slug_to_micro_app("docx-template-mailmerge"),
        Some(MicroAppId::DataLab)
    );
    assert_eq!(
        tool_slug_to_micro_app("universal-file-converter-hub"),
        Some(MicroAppId::DataLab)
    );
}

#[test]
fn test_all_51_tools_mapped_and_distributed() {
    assert_eq!(ALL_TOOLS.len(), 51, "Tổng số công cụ trong ALL_TOOLS phải là 51");

    let mut vn_office_count = 0;
    let mut pdf_studio_count = 0;
    let mut media_studio_count = 0;
    let mut data_lab_count = 0;

    for tool in ALL_TOOLS.iter() {
        let app = tool_slug_to_micro_app(tool.slug)
            .unwrap_or_else(|| panic!("Công cụ '{}' không được map vào MicroAppId nào!", tool.slug));

        match app {
            MicroAppId::VnOffice => vn_office_count += 1,
            MicroAppId::PdfStudio => pdf_studio_count += 1,
            MicroAppId::MediaStudio => media_studio_count += 1,
            MicroAppId::DataLab => data_lab_count += 1,
        }
    }

    assert_eq!(vn_office_count, 11, "VnOffice phải có chính xác 11 công cụ");
    assert_eq!(pdf_studio_count, 18, "PdfStudio phải có chính xác 18 công cụ");
    assert_eq!(media_studio_count, 9, "MediaStudio phải có chính xác 9 công cụ");
    assert_eq!(data_lab_count, 13, "DataLab phải có chính xác 13 công cụ");
    assert_eq!(
        vn_office_count + pdf_studio_count + media_studio_count + data_lab_count,
        51,
        "Tổng số công cụ phân bổ phải khớp 51"
    );
}

#[test]
fn test_micro_app_tools_consistency() {
    let vn_tools = micro_app_tools(MicroAppId::VnOffice);
    assert_eq!(vn_tools.len(), 11);
    for tool in vn_tools {
        assert_eq!(tool_slug_to_micro_app(tool.slug), Some(MicroAppId::VnOffice));
    }

    let pdf_tools = micro_app_tools(MicroAppId::PdfStudio);
    assert_eq!(pdf_tools.len(), 18);
    for tool in pdf_tools {
        assert_eq!(tool_slug_to_micro_app(tool.slug), Some(MicroAppId::PdfStudio));
    }

    let media_tools = micro_app_tools(MicroAppId::MediaStudio);
    assert_eq!(media_tools.len(), 9);
    for tool in media_tools {
        assert_eq!(tool_slug_to_micro_app(tool.slug), Some(MicroAppId::MediaStudio));
    }

    let data_tools = micro_app_tools(MicroAppId::DataLab);
    assert_eq!(data_tools.len(), 13);
    for tool in data_tools {
        assert_eq!(tool_slug_to_micro_app(tool.slug), Some(MicroAppId::DataLab));
    }
}

#[test]
fn test_unknown_slug_returns_none() {
    assert_eq!(tool_slug_to_micro_app("unknown-tool-slug"), None);
    assert_eq!(tool_slug_to_micro_app(""), None);
}
