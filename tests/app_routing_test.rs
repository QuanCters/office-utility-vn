use leptos::prelude::*;
use office_utility_vn_ui::app::{AppState, AppView};
use office_utility_vn_ui::micro_apps::MicroAppId;

#[test]
fn test_app_state_initialization() {
    let state = AppState::new();

    // 1. Initial view must be Launcher
    assert_eq!(state.current_view.get_untracked(), AppView::Launcher);

    // 2. Initial deep link tool must be None
    assert_eq!(state.deep_link_tool.get_untracked(), None);

    // 3. All 4 micro-app states are initialized with default tools
    assert_eq!(
        state.vn_office_state.active_tool.get_untracked(),
        "vietnamese-number-to-words"
    );
    assert_eq!(
        state.pdf_studio_state.active_tool.get_untracked(),
        "pdf-merge-split"
    );
    assert_eq!(
        state.media_studio_state.active_tool.get_untracked(),
        "image-batch"
    );
    assert_eq!(
        state.data_lab_state.active_tool.get_untracked(),
        "excel-cleaner-normalizer"
    );
}

#[test]
fn test_navigation_to_each_micro_app() {
    let state = AppState::new();

    // Navigate to VN Office
    state.navigate_to(MicroAppId::VnOffice);
    assert_eq!(state.current_view.get_untracked(), AppView::MicroApp(MicroAppId::VnOffice));
    assert_eq!(state.deep_link_tool.get_untracked(), None);

    // Navigate to PDF Studio
    state.navigate_to(MicroAppId::PdfStudio);
    assert_eq!(state.current_view.get_untracked(), AppView::MicroApp(MicroAppId::PdfStudio));
    assert_eq!(state.deep_link_tool.get_untracked(), None);

    // Navigate to Media Studio
    state.navigate_to(MicroAppId::MediaStudio);
    assert_eq!(state.current_view.get_untracked(), AppView::MicroApp(MicroAppId::MediaStudio));
    assert_eq!(state.deep_link_tool.get_untracked(), None);

    // Navigate to Data Lab
    state.navigate_to(MicroAppId::DataLab);
    assert_eq!(state.current_view.get_untracked(), AppView::MicroApp(MicroAppId::DataLab));
    assert_eq!(state.deep_link_tool.get_untracked(), None);
}

#[test]
fn test_deep_linking_and_tool_activation() {
    let state = AppState::new();

    // Deep link to VN Office tool
    state.deep_link(MicroAppId::VnOffice, "gross-net-salary-calculator");
    assert_eq!(state.current_view.get_untracked(), AppView::MicroApp(MicroAppId::VnOffice));
    assert_eq!(state.deep_link_tool.get_untracked(), Some("gross-net-salary-calculator".to_string()));
    assert_eq!(state.vn_office_state.active_tool.get_untracked(), "gross-net-salary-calculator");

    // Deep link to PDF Studio tool
    state.deep_link(MicroAppId::PdfStudio, "pdf-to-markdown");
    assert_eq!(state.current_view.get_untracked(), AppView::MicroApp(MicroAppId::PdfStudio));
    assert_eq!(state.deep_link_tool.get_untracked(), Some("pdf-to-markdown".to_string()));
    assert_eq!(state.pdf_studio_state.active_tool.get_untracked(), "pdf-to-markdown");

    // Deep link to Media Studio tool
    state.deep_link(MicroAppId::MediaStudio, "photo-id-crop");
    assert_eq!(state.current_view.get_untracked(), AppView::MicroApp(MicroAppId::MediaStudio));
    assert_eq!(state.deep_link_tool.get_untracked(), Some("photo-id-crop".to_string()));
    assert_eq!(state.media_studio_state.active_tool.get_untracked(), "photo-id-crop");

    // Deep link to Data Lab tool
    state.deep_link(MicroAppId::DataLab, "excel-split-by-column");
    assert_eq!(state.current_view.get_untracked(), AppView::MicroApp(MicroAppId::DataLab));
    assert_eq!(state.deep_link_tool.get_untracked(), Some("excel-split-by-column".to_string()));
    assert_eq!(state.data_lab_state.active_tool.get_untracked(), "excel-split-by-column");

    // Deep link by slug (automatic domain resolution)
    let resolved = state.deep_link_by_slug("vietqr-napas247-generator");
    assert!(resolved);
    assert_eq!(state.current_view.get_untracked(), AppView::MicroApp(MicroAppId::VnOffice));
    assert_eq!(state.deep_link_tool.get_untracked(), Some("vietqr-napas247-generator".to_string()));
    assert_eq!(state.vn_office_state.active_tool.get_untracked(), "vietqr-napas247-generator");
}

#[test]
fn test_returning_to_launcher() {
    let state = AppState::new();

    // Go to a micro-app with deep link
    state.deep_link(MicroAppId::PdfStudio, "pdf-compress");
    assert_eq!(state.current_view.get_untracked(), AppView::MicroApp(MicroAppId::PdfStudio));
    assert_eq!(state.deep_link_tool.get_untracked(), Some("pdf-compress".to_string()));

    // Return to launcher
    state.back_to_launcher();
    assert_eq!(state.current_view.get_untracked(), AppView::Launcher);
    assert_eq!(state.deep_link_tool.get_untracked(), None);

    // Micro-app state persists even after returning to launcher
    assert_eq!(state.pdf_studio_state.active_tool.get_untracked(), "pdf-compress");
}

#[test]
fn test_deep_link_by_unknown_slug() {
    let state = AppState::new();
    let resolved = state.deep_link_by_slug("non-existent-tool-slug-xyz");
    assert!(!resolved);
    // View should remain Launcher
    assert_eq!(state.current_view.get_untracked(), AppView::Launcher);
}
