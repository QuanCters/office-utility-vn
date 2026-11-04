use leptos::prelude::*;
use office_utility_vn_ui::micro_apps::media_studio::state::{
    category_for_tool, media_studio_tool_icon, MediaBatchItem, MediaCategory, MediaStudioState,
};
use office_utility_vn_ui::micro_apps::{tool_slug_to_micro_app, MicroAppId, MEDIA_STUDIO_TOOLS};

const EXPECTED_MEDIA_STUDIO_SLUGS: &[&str] = &[
    "image-batch",
    "photo-id-crop",
    "image-bg-redact",
    "image-exif-inspector",
    "watermark-batch-studio",
    "screenshot-beautifier",
    "doc-scanner-perspective",
    "audio-silence-trimmer",
    "video-to-gif-webp",
];

#[test]
fn test_media_studio_state_default_initialization() {
    let state = MediaStudioState::new();

    // 1. Mặc định công cụ ban đầu là "image-batch"
    assert_eq!(state.active_tool_slug(), "image-batch");

    // 2. Chế độ workflow ban đầu phải là false
    assert!(!state.is_workflow());

    // 3. Trạng thái thực thi ban đầu phải là false
    assert!(!state.is_running());

    // 4. Danh sách tệp ban đầu rỗng
    assert_eq!(state.loaded_files_count(), 0);
    assert!(!state.has_files());
    assert_eq!(state.active_file_index.get(), None);
    assert_eq!(state.active_file_path(), None);
    assert_eq!(state.active_file_name(), None);

    // 5. Danh mục mặc định ban đầu là Image (Xử lý Ảnh)
    assert_eq!(state.active_category.get(), MediaCategory::Image);

    // 6. Hàng đợi batch rỗng và cờ mở rộng mặc định là true
    assert_eq!(state.batch_items_count(), 0);
    assert!(!state.has_active_batches());
    assert!(state.is_queue_expanded.get());
}

#[test]
fn test_media_studio_state_with_custom_initial_tool() {
    // 1. Khởi tạo với công cụ thuộc AudioVideo
    let state_audio = MediaStudioState::with_initial_tool(Some("audio-silence-trimmer".to_string()));
    assert_eq!(state_audio.active_tool_slug(), "audio-silence-trimmer");
    assert_eq!(state_audio.active_category.get(), MediaCategory::AudioVideo);
    assert!(!state_audio.is_workflow());

    let state_video = MediaStudioState::with_initial_tool(Some("video-to-gif-webp".to_string()));
    assert_eq!(state_video.active_tool_slug(), "video-to-gif-webp");
    assert_eq!(state_video.active_category.get(), MediaCategory::AudioVideo);

    // 2. Khởi tạo với công cụ thuộc Image
    let state_crop = MediaStudioState::with_initial_tool(Some("photo-id-crop".to_string()));
    assert_eq!(state_crop.active_tool_slug(), "photo-id-crop");
    assert_eq!(state_crop.active_category.get(), MediaCategory::Image);

    // 3. Khởi tạo với None hoặc chuỗi rỗng -> Fallback về "image-batch"
    let state_none = MediaStudioState::with_initial_tool(None);
    assert_eq!(state_none.active_tool_slug(), "image-batch");
    assert_eq!(state_none.active_category.get(), MediaCategory::Image);

    let state_empty = MediaStudioState::with_initial_tool(Some("   ".to_string()));
    assert_eq!(state_empty.active_tool_slug(), "image-batch");
    assert_eq!(state_empty.active_category.get(), MediaCategory::Image);
}

#[test]
fn test_media_studio_file_management() {
    let state = MediaStudioState::new();

    // 1. Thêm một tệp đầu tiên
    state.add_file("/home/user/pictures/profile_raw.png");
    assert_eq!(state.loaded_files_count(), 1);
    assert!(state.has_files());
    assert_eq!(state.active_file_index.get(), Some(0));
    assert_eq!(
        state.active_file_path(),
        Some("/home/user/pictures/profile_raw.png".to_string())
    );
    assert_eq!(
        state.active_file_name(),
        Some("profile_raw.png".to_string())
    );

    // 2. Thêm nhiều tệp tiếp theo
    state.add_files(vec![
        "/home/user/podcasts/episode_42.wav".to_string(),
        "promo_video.mp4".to_string(),
    ]);
    assert_eq!(state.loaded_files_count(), 3);
    // Active file index vẫn giữ nguyên ở tệp đầu tiên
    assert_eq!(state.active_file_index.get(), Some(0));

    // 3. Bỏ qua tệp có đường dẫn rỗng
    state.add_file("");
    state.add_file("   ");
    assert_eq!(state.loaded_files_count(), 3);

    // 4. Chọn tệp khác theo index
    state.select_file(1);
    assert_eq!(state.active_file_index.get(), Some(1));
    assert_eq!(state.active_file_name(), Some("episode_42.wav".to_string()));

    state.select_file(2);
    assert_eq!(state.active_file_index.get(), Some(2));
    assert_eq!(state.active_file_name(), Some("promo_video.mp4".to_string()));

    // 5. Chọn index vượt quá giới hạn -> Không thay đổi
    state.select_file(99);
    assert_eq!(state.active_file_index.get(), Some(2));
}

#[test]
fn test_media_studio_remove_file_and_adjust_index() {
    let state = MediaStudioState::new();

    state.add_files(vec![
        "photo_a.jpg".to_string(),
        "audio_b.mp3".to_string(),
        "video_c.mp4".to_string(),
    ]);
    assert_eq!(state.loaded_files_count(), 3);

    // Trường hợp 1: Đang chọn video_c (index 2), xóa audio_b (index 1)
    state.select_file(2);
    assert_eq!(state.active_file_name(), Some("video_c.mp4".to_string()));
    state.remove_file(1);
    assert_eq!(state.loaded_files_count(), 2);
    // Index của video_c giảm từ 2 xuống 1
    assert_eq!(state.active_file_index.get(), Some(1));
    assert_eq!(state.active_file_name(), Some("video_c.mp4".to_string()));

    // Trường hợp 2: Đang chọn tệp ở cuối danh sách (index 1 = video_c) và xóa chính nó
    state.remove_file(1);
    assert_eq!(state.loaded_files_count(), 1);
    // Index tự động lùi về tệp liền trước (index 0 = photo_a)
    assert_eq!(state.active_file_index.get(), Some(0));
    assert_eq!(state.active_file_name(), Some("photo_a.jpg".to_string()));

    // Trường hợp 3: Xóa tệp duy nhất còn lại
    state.remove_file(0);
    assert_eq!(state.loaded_files_count(), 0);
    assert_eq!(state.active_file_index.get(), None);
    assert_eq!(state.active_file_path(), None);
    assert_eq!(state.active_file_name(), None);

    // Trường hợp 4: Xóa ngoài biên an toàn không panic
    state.remove_file(0);
    state.remove_file(99);
    assert_eq!(state.loaded_files_count(), 0);
}

#[test]
fn test_media_studio_clear_files() {
    let state = MediaStudioState::new();
    state.add_files(vec!["img1.png", "img2.jpg", "sound.wav"]);
    assert_eq!(state.loaded_files_count(), 3);
    assert!(state.has_files());

    state.clear_files();
    assert_eq!(state.loaded_files_count(), 0);
    assert!(!state.has_files());
    assert_eq!(state.active_file_index.get(), None);
}

#[test]
fn test_media_studio_9_tools_exact_list() {
    // 1. Số lượng công cụ chính xác 9
    assert_eq!(MEDIA_STUDIO_TOOLS.len(), 9);
    assert_eq!(EXPECTED_MEDIA_STUDIO_SLUGS.len(), 9);

    for slug in EXPECTED_MEDIA_STUDIO_SLUGS {
        // 2. Mọi slug phải tồn tại trong MEDIA_STUDIO_TOOLS
        let found = MEDIA_STUDIO_TOOLS.iter().find(|t| t.slug == *slug);
        assert!(found.is_some(), "Công cụ '{}' thiếu trong MEDIA_STUDIO_TOOLS", slug);

        // 3. Mọi slug phải được ánh xạ đúng về MicroAppId::MediaStudio
        assert_eq!(
            tool_slug_to_micro_app(slug),
            Some(MicroAppId::MediaStudio),
            "Slug '{}' không định tuyến đúng về MediaStudio",
            slug
        );

        // 4. Mọi slug phải thuộc đúng 1 nhóm MediaCategory
        let cat = category_for_tool(slug);
        assert!(cat.is_some(), "Slug '{}' không được gán danh mục MediaCategory nào", slug);

        // 5. Mọi slug phải có icon emoji trực quan
        let icon = media_studio_tool_icon(slug);
        assert!(!icon.is_empty(), "Slug '{}' không có icon", slug);
    }
}

#[test]
fn test_media_studio_category_grouping() {
    // 1. Kiểm tra 2 danh mục của Media Studio
    let categories = MediaCategory::ALL;
    assert_eq!(categories.len(), 2);

    let mut total_slugs = 0;
    for cat in categories {
        assert!(!cat.label().is_empty());
        assert!(!cat.icon().is_empty());
        let slugs = cat.slugs();
        assert!(!slugs.is_empty());
        total_slugs += slugs.len();

        for slug in slugs {
            assert_eq!(category_for_tool(slug), Some(cat));
        }
    }

    // 2. Tổng số công cụ trên 2 danh mục phải đúng bằng 9
    assert_eq!(total_slugs, 9);

    // 3. Kiểm tra cụ thể từng nhóm danh mục theo thiết kế
    assert_eq!(
        MediaCategory::Image.slugs(),
        &[
            "image-batch",
            "photo-id-crop",
            "image-bg-redact",
            "image-exif-inspector",
            "watermark-batch-studio",
            "screenshot-beautifier",
            "doc-scanner-perspective",
        ]
    );

    assert_eq!(
        MediaCategory::AudioVideo.slugs(),
        &[
            "audio-silence-trimmer",
            "video-to-gif-webp",
        ]
    );
}

#[test]
fn test_media_studio_tool_and_category_switching() {
    let state = MediaStudioState::new();

    // 1. Chọn công cụ thuộc nhóm Audio & Video
    state.select_tool("video-to-gif-webp");
    assert_eq!(state.active_tool_slug(), "video-to-gif-webp");
    assert_eq!(state.active_category.get(), MediaCategory::AudioVideo);
    assert!(!state.is_workflow());

    // 2. Chuyển sang danh mục Image (Xử lý Ảnh)
    state.select_category(MediaCategory::Image);
    assert_eq!(state.active_category.get(), MediaCategory::Image);
    // Tự động chọn công cụ đầu tiên của tab mới
    assert_eq!(state.active_tool_slug(), "image-batch");

    // 3. Chọn công cụ khác cùng trong nhóm Image
    state.select_tool("watermark-batch-studio");
    assert_eq!(state.active_tool_slug(), "watermark-batch-studio");
    // Nhấp lại vào tab Image không đổi công cụ đã chọn
    state.select_category(MediaCategory::Image);
    assert_eq!(state.active_tool_slug(), "watermark-batch-studio");
}

#[test]
fn test_media_studio_workflow_toggle() {
    let state = MediaStudioState::new();

    // 1. Mở chế độ Workflow
    state.open_workflow();
    assert!(state.is_workflow());

    // 2. Chọn một công cụ bất kỳ -> Tự động đóng chế độ Workflow
    state.select_tool("photo-id-crop");
    assert!(!state.is_workflow());
    assert_eq!(state.active_tool_slug(), "photo-id-crop");

    // 3. Mở lại Workflow và chọn danh mục -> Tự động đóng Workflow
    state.open_workflow();
    assert!(state.is_workflow());
    state.select_category(MediaCategory::AudioVideo);
    assert!(!state.is_workflow());

    // 4. Đóng thủ công
    state.open_workflow();
    assert!(state.is_workflow());
    state.close_workflow();
    assert!(!state.is_workflow());
}

#[test]
fn test_media_studio_batch_queue_operations() {
    let state = MediaStudioState::new();
    assert_eq!(state.batch_items_count(), 0);
    assert!(!state.has_active_batches());

    // 1. Thêm item tường minh
    state.enqueue_batch_item(MediaBatchItem::new(
        "task-1",
        "portrait_3x4.jpg",
        "Pending",
        0.0,
    ));
    assert_eq!(state.batch_items_count(), 1);
    assert!(state.has_active_batches());

    // 2. Thêm item qua chuỗi tên tệp (impl Into<MediaBatchItem>)
    state.enqueue_batch_item("meeting_recording.mp3");
    assert_eq!(state.batch_items_count(), 2);

    // 3. Cập nhật tiến trình task-1 sang Processing 50%
    state.update_batch_progress("task-1", "Processing", 0.5);
    state.batch_queue.with(|q| {
        let it = q.iter().find(|x| x.id == "task-1").unwrap();
        assert_eq!(it.status, "Processing");
        assert_eq!(it.progress, 0.5);
    });

    // 4. Cập nhật task-1 sang Done 100%
    state.update_batch_progress("task-1", "Done", 1.0);
    state.batch_queue.with(|q| {
        let it = q.iter().find(|x| x.id == "task-1").unwrap();
        assert_eq!(it.status, "Done");
        assert_eq!(it.progress, 1.0);
    });

    // 5. Dọn sạch tác vụ đã hoàn thành
    state.clear_completed_batch();
    assert_eq!(state.batch_items_count(), 1);
    state.batch_queue.with(|q| {
        assert_eq!(q[0].file_name, "meeting_recording.mp3");
    });

    // 6. Xóa từng item theo id
    state.remove_batch_item("batch-meeting_recording.mp3");
    assert_eq!(state.batch_items_count(), 0);
    assert!(!state.has_active_batches());

    // 7. Test clear_batch_queue
    state.enqueue_batch_item("file_a.png");
    state.enqueue_batch_item("file_b.png");
    assert_eq!(state.batch_items_count(), 2);
    state.clear_batch_queue();
    assert_eq!(state.batch_items_count(), 0);

    // 8. Test toggle expand/collapse
    assert!(state.is_queue_expanded.get());
    state.toggle_queue_expanded();
    assert!(!state.is_queue_expanded.get());
    state.toggle_queue_expanded();
    assert!(state.is_queue_expanded.get());
}

#[test]
fn test_media_studio_view_mode_and_sidebar_toggle() {
    use office_utility_vn_ui::micro_apps::media_studio::state::MediaViewMode;

    let state = MediaStudioState::new();

    // 1. Mặc định khi khởi tạo là chế độ Hub (Portal View)
    assert_eq!(state.view_mode.get(), MediaViewMode::Hub);
    assert!(state.is_hub());
    assert!(!state.is_tool_workspace());

    // 2. Mở một công cụ cụ thể -> Chuyển sang ToolWorkspace và thu gọn Sidebar (mini)
    state.open_tool("photo-id-crop");
    assert_eq!(state.view_mode.get(), MediaViewMode::ToolWorkspace);
    assert_eq!(state.active_tool_slug(), "photo-id-crop");
    assert!(state.is_tool_workspace());
    assert!(!state.is_hub());
    assert!(state.sidebar_collapsed.get(), "Sidebar phải tự động thu gọn về dạng Mini khi vào công cụ");

    // 3. Toggle sidebar mở rộng và thu gọn
    state.toggle_sidebar();
    assert!(!state.sidebar_collapsed.get(), "Sau khi toggle, sidebar phải mở rộng");
    state.toggle_sidebar();
    assert!(state.sidebar_collapsed.get(), "Sau khi toggle lần 2, sidebar phải thu gọn");

    // 4. Quay lại Hub
    state.open_hub();
    assert_eq!(state.view_mode.get(), MediaViewMode::Hub);
    assert!(state.is_hub());
    assert!(!state.is_tool_workspace());

    // 5. Mở Workflow
    state.open_workflow();
    assert_eq!(state.view_mode.get(), MediaViewMode::Workflow);
    assert!(state.is_workflow());

    // 6. Đóng Workflow
    state.close_workflow();
    assert!(!state.is_workflow());
    assert!(state.is_tool_workspace());

    // 7. Toggle Floating Queue Drawer
    assert!(!state.is_floating_queue_open.get());
    state.toggle_floating_queue();
    assert!(state.is_floating_queue_open.get());
    state.toggle_floating_queue();
    assert!(!state.is_floating_queue_open.get());
}

#[test]
fn test_media_studio_hub_filter() {
    use office_utility_vn_ui::micro_apps::media_studio::hub_view::{filter_media_studio_tools, matches_media_tool};

    let state = MediaStudioState::new();
    let tools = state.tools();
    assert_eq!(tools.len(), 9);

    // 1. Chuỗi tìm kiếm rỗng -> Khớp toàn bộ 9 công cụ
    let all_filtered = filter_media_studio_tools(tools, "");
    assert_eq!(all_filtered.len(), 9);

    let all_spaces = filter_media_studio_tools(tools, "   ");
    assert_eq!(all_spaces.len(), 9);

    // 2. Tìm kiếm có dấu tiếng Việt "ảnh thẻ" -> Tìm ra photo-id-crop
    let the_results = filter_media_studio_tools(tools, "ảnh thẻ");
    assert!(the_results.iter().any(|t| t.slug == "photo-id-crop"));

    // 3. Tìm kiếm không dấu tiếng Việt "anh the" -> Vẫn tìm ra photo-id-crop
    let unaccented_the = filter_media_studio_tools(tools, "anh the");
    assert!(unaccented_the.iter().any(|t| t.slug == "photo-id-crop"));

    // 4. Tìm kiếm theo slug "exif" -> Tìm ra image-exif-inspector
    let exif_results = filter_media_studio_tools(tools, "exif");
    assert!(exif_results.iter().any(|t| t.slug == "image-exif-inspector"));

    // 5. Tìm kiếm theo từ khóa "nén"
    let nen_results = filter_media_studio_tools(tools, "nén");
    assert!(nen_results.iter().any(|t| t.slug == "image-batch"));

    // 6. Tìm kiếm video sang gif
    let gif_results = filter_media_studio_tools(tools, "gif");
    assert!(gif_results.iter().any(|t| t.slug == "video-to-gif-webp"));

    // 7. Kiểm tra trực tiếp hàm matches_media_tool
    let crop_tool = state.find_tool("photo-id-crop").expect("photo-id-crop must exist");
    assert!(matches_media_tool(crop_tool, "3x4"));
    assert!(matches_media_tool(crop_tool, "passport"));

    // 8. Từ khóa không tồn tại -> Trả về danh sách rỗng
    let not_found = filter_media_studio_tools(tools, "xyz_not_found_token_999");
    assert!(not_found.is_empty());
}

#[test]
fn test_media_studio_floating_widget_signals() {
    let state = MediaStudioState::new();

    // 1. Ban đầu không có tác vụ chạy ngầm và tỷ lệ tiến trình là 0.0
    assert!(!state.is_background_processing());
    assert_eq!(state.batch_progress_ratio(), 0.0);

    // 2. Thêm item ở trạng thái Pending
    state.enqueue_batch_item(MediaBatchItem::new("t1", "img1.png", "Pending", 0.0));
    assert!(!state.is_background_processing()); // Pending chưa phải là Processing
    assert!(state.has_active_batches());
    assert_eq!(state.batch_progress_ratio(), 0.0);

    // 3. Cập nhật sang Processing 60%
    state.update_batch_progress("t1", "Processing", 0.6);
    assert!(state.is_background_processing());
    assert_eq!(state.batch_progress_ratio(), 0.6);

    // 4. Thêm item thứ 2 và hoàn thành 100%
    state.enqueue_batch_item(MediaBatchItem::new("t2", "img2.png", "Done", 1.0));
    // Tỷ lệ trung bình của 2 item: (0.6 + 1.0) / 2 = 0.8
    assert!((state.batch_progress_ratio() - 0.8).abs() < 1e-4);
}
