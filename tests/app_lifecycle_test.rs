use leptos::prelude::*;
use office_utility_vn_ui::app::AppState;
use office_utility_vn_ui::micro_apps::MicroAppId;

#[test]
fn test_has_active_work_initial_state() {
    let state = AppState::new();

    // Ban đầu cả 4 micro-apps đều không có tác vụ nào đang làm việc
    for &app_id in &MicroAppId::ALL {
        assert!(
            !state.has_active_work(app_id),
            "MicroApp {:?} should not have active work initially",
            app_id
        );
    }
}

#[test]
fn test_has_active_work_vn_office() {
    let state = AppState::new();

    assert!(!state.has_active_work(MicroAppId::VnOffice));

    // Thêm bản ghi tính toán vào calculation_history
    state.vn_office_state.record_calculation(
        "vietnamese-number-to-words",
        "Đọc số tiền",
        "1500000",
        "Một triệu năm trăm nghìn đồng",
    );

    assert!(state.has_active_work(MicroAppId::VnOffice));

    // Reset micro-app state
    state.reset_app_state(MicroAppId::VnOffice);

    assert!(!state.has_active_work(MicroAppId::VnOffice));
    assert!(state.vn_office_state.calculation_history.with(|h| h.is_empty()));
}

#[test]
fn test_has_active_work_pdf_studio() {
    let state = AppState::new();

    assert!(!state.has_active_work(MicroAppId::PdfStudio));

    // Nạp tệp PDF vào workspace
    state.pdf_studio_state.add_file("/documents/contract.pdf");

    assert!(state.has_active_work(MicroAppId::PdfStudio));

    // Reset micro-app state
    state.reset_app_state(MicroAppId::PdfStudio);

    assert!(!state.has_active_work(MicroAppId::PdfStudio));
    assert!(state.pdf_studio_state.loaded_files.with(|f| f.is_empty()));
}

#[test]
fn test_has_active_work_media_studio_queue_and_files() {
    let state = AppState::new();

    assert!(!state.has_active_work(MicroAppId::MediaStudio));

    // 1. Kích hoạt thông qua Batch Queue
    state.media_studio_state.enqueue_batch_item("avatar.png");
    assert!(state.has_active_work(MicroAppId::MediaStudio));

    // Reset micro-app state
    state.reset_app_state(MicroAppId::MediaStudio);
    assert!(!state.has_active_work(MicroAppId::MediaStudio));
    assert_eq!(state.media_studio_state.batch_items_count(), 0);

    // 2. Kích hoạt thông qua loaded_files
    state.media_studio_state.add_file("/media/banner.jpg");
    assert!(state.has_active_work(MicroAppId::MediaStudio));

    // Reset micro-app state
    state.reset_app_state(MicroAppId::MediaStudio);
    assert!(!state.has_active_work(MicroAppId::MediaStudio));
    assert!(state.media_studio_state.loaded_files.with(|f| f.is_empty()));
}

#[test]
fn test_has_active_work_data_lab() {
    let state = AppState::new();

    assert!(!state.has_active_work(MicroAppId::DataLab));

    // Nạp tệp bảng tính vào workspace
    state.data_lab_state.add_file("/spreadsheets/q3_report.xlsx");

    assert!(state.has_active_work(MicroAppId::DataLab));

    // Reset micro-app state
    state.reset_app_state(MicroAppId::DataLab);

    assert!(!state.has_active_work(MicroAppId::DataLab));
    assert!(state.data_lab_state.loaded_files.with(|f| f.is_empty()));
}

#[test]
fn test_reset_app_state_resets_selected_tool_and_signals() {
    let state = AppState::new();

    // Thay đổi công cụ được chọn trên cả 4 micro-apps
    state.vn_office_state.select_tool("gross-net-salary-calculator");
    state.pdf_studio_state.select_tool("pdf-to-markdown");
    state.media_studio_state.select_tool("photo-id-crop");
    state.data_lab_state.select_tool("excel-split-by-column");

    assert_eq!(state.vn_office_state.active_tool_slug(), "gross-net-salary-calculator");
    assert_eq!(state.pdf_studio_state.active_tool_slug(), "pdf-to-markdown");
    assert_eq!(state.media_studio_state.active_tool_slug(), "photo-id-crop");
    assert_eq!(state.data_lab_state.active_tool_slug(), "excel-split-by-column");

    // Reset từng micro-app
    state.reset_app_state(MicroAppId::VnOffice);
    assert_eq!(state.vn_office_state.active_tool_slug(), "vietnamese-number-to-words");

    state.reset_app_state(MicroAppId::PdfStudio);
    assert_eq!(state.pdf_studio_state.active_tool_slug(), "pdf-merge-split");

    state.reset_app_state(MicroAppId::MediaStudio);
    assert_eq!(state.media_studio_state.active_tool_slug(), "image-batch");

    state.reset_app_state(MicroAppId::DataLab);
    assert_eq!(state.data_lab_state.active_tool_slug(), "excel-cleaner-normalizer");
}

#[test]
fn test_hibernate_inactive_apps() {
    let state = AppState::new();

    // Giả lập trạng thái:
    // - PDF Studio: Đang có file nạp vào (active work)
    state.pdf_studio_state.add_file("/docs/contract.pdf");

    // - Media Studio: Đang có tác vụ hàng loạt (active work)
    state.media_studio_state.enqueue_batch_item("photo.jpg");

    // - VN Office: Đã đổi công cụ nhưng KHÔNG có active work (chưa tính toán gì)
    state.vn_office_state.select_tool("gross-net-salary-calculator");

    // - Data Lab: Đã đổi công cụ nhưng KHÔNG có active work (chưa nạp file)
    state.data_lab_state.select_tool("excel-split-by-column");

    // Giả định người dùng đang làm việc ở PdfStudio
    state.hibernate_inactive_apps(MicroAppId::PdfStudio);

    // 1. PdfStudio (active app): Giữ nguyên công việc
    assert!(state.has_active_work(MicroAppId::PdfStudio));
    assert_eq!(state.pdf_studio_state.loaded_files_count(), 1);

    // 2. MediaStudio (inactive app nhưng CÓ active work): Không bị xóa công việc
    assert!(state.has_active_work(MicroAppId::MediaStudio));
    assert_eq!(state.media_studio_state.batch_items_count(), 1);

    // 3. VnOffice (inactive app và KHÔNG CÓ active work): Bị hibernate / reset về mặc định
    assert_eq!(state.vn_office_state.active_tool_slug(), "vietnamese-number-to-words");

    // 4. DataLab (inactive app và KHÔNG CÓ active work): Bị hibernate / reset về mặc định
    assert_eq!(state.data_lab_state.active_tool_slug(), "excel-cleaner-normalizer");
}
