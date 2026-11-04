use leptos::prelude::*;
use crate::app::ToolItem;
use crate::launcher::remove_vietnamese_diacritics;
use crate::micro_apps::media_studio::state::{media_studio_tool_icon, MediaCategory, MediaStudioState};

/// Từ đồng nghĩa hoặc từ khóa bổ trợ cho từng slug công cụ Media Studio.
pub fn media_tool_synonyms(slug: &str) -> &'static str {
    match slug {
        "image-batch" => "nén ảnh nen anh đổi đuôi doi duoi chuyển định dạng chuyen dinh dang convert format resize giảm kích thước giam dung luong webp png jpg",
        "photo-id-crop" => "cắt ảnh thẻ cat anh the 3x4 4x6 300dpi avatar chân dung chan dung visa passport hộ chiếu ho chieu căn cước can cuoc",
        "image-bg-redact" => "che mờ che mo bôi đen boi den cccd cmnd căn cước can cuoc mã số thuế ma so thue pii nhạy cảm nhay cam làm mờ lam mo che giấu che giau pixelate blur",
        "image-exif-inspector" => "xóa exif xoa exif vị trí vi tri gps camera thiết bị thiet bi ngày giờ ngay gio siêu dữ liệu sieu du lieu privacy bí mật bi mat",
        "watermark-batch-studio" => "đóng dấu dong dau watermark chìm chim bản quyền ban quyen logo mộc đỏ moc do text stamp hàng loạt hang loat",
        "screenshot-beautifier" => "làm đẹp ảnh lam dep anh chụp màn hình chup man hinh mockup khung macos window shadow bo góc bóng đổ background nền gradient",
        "doc-scanner-perspective" => "nắn phẳng nan phang scan quét tài liệu quet tai lieu chụp nghiêng chup nghieng 4 góc ke góc perspective căn góc trang sách",
        "audio-silence-trimmer" => "cắt khoảng lặng cat khoang lang âm thanh am thanh audio podcast ghi âm ghi am lọc tạp âm loc tap am remove silence wav mp3",
        "video-to-gif-webp" => "chuyển video chuyen video sang gif webp hoạt họa hoat hoa animation cắt clip mp4 sang gif tạo meme",
        _ => "",
    }
}

/// Kiểm tra một công cụ Media có khớp với chuỗi tìm kiếm hay không.
/// Hỗ trợ không phân biệt hoa thường và tự động bóc tách dấu tiếng Việt (diacritics-insensitive).
pub fn matches_media_tool(tool: &ToolItem, query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return true;
    }
    let q_lower = q.to_lowercase();
    let q_unaccented = remove_vietnamese_diacritics(&q_lower);

    let name_lower = tool.name.to_lowercase();
    let desc_lower = tool.description.to_lowercase();
    let slug_lower = tool.slug.to_lowercase();

    let name_unaccented = remove_vietnamese_diacritics(tool.name);
    let desc_unaccented = remove_vietnamese_diacritics(tool.description);

    let synonyms = media_tool_synonyms(tool.slug);
    let synonyms_unaccented = remove_vietnamese_diacritics(synonyms);

    name_lower.contains(&q_lower)
        || desc_lower.contains(&q_lower)
        || slug_lower.contains(&q_lower)
        || name_unaccented.contains(&q_unaccented)
        || desc_unaccented.contains(&q_unaccented)
        || slug_lower.contains(&q_unaccented)
        || synonyms.contains(&q_lower)
        || synonyms_unaccented.contains(&q_unaccented)
}

/// Lọc danh sách công cụ Media Studio theo từ khóa tìm kiếm.
pub fn filter_media_studio_tools<'a>(tools: &'a [ToolItem], query: &str) -> Vec<&'a ToolItem> {
    tools.iter().filter(|t| matches_media_tool(t, query)).collect()
}

/// Tiêu đề đầy đủ của từng phân khu Media Studio.
pub fn category_full_title(cat: MediaCategory) -> &'static str {
    match cat {
        MediaCategory::Image => "Xử lý & Tối ưu Ảnh Chuyên Nghiệp",
        MediaCategory::AudioVideo => "Audio & Video Studio",
    }
}

/// Mô tả tóm tắt tính năng của từng danh mục.
pub fn category_description(cat: MediaCategory) -> &'static str {
    match cat {
        MediaCategory::Image => "Nén ảnh hàng loạt, cắt ảnh thẻ 3x4/4x6 300 DPI, che mờ giấy tờ, xóa Exif/GPS & đóng dấu bản quyền",
        MediaCategory::AudioVideo => "Cắt bỏ khoảng lặng ghi âm tự động & chuyển đổi video clip sang GIF/WebP hoạt họa nhẹ nhàng",
    }
}

/// Component hiển thị Trang chủ Hub/Portal của Media Studio theo mô hình Categorized Card Grid
/// kết hợp Floating Contextual Progress Widget khi có tác vụ chạy ngầm.
#[component]
pub fn MediaStudioHubView(
    state: MediaStudioState,
) -> impl IntoView {
    let search_query = RwSignal::new(String::new());

    // Lắng nghe phím Esc để xóa ô tìm kiếm
    let on_search_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            search_query.set(String::new());
        }
    };

    let all_tools = state.tools();

    view! {
        <div class="media-studio-hub-view" role="region" aria-label="Trang chủ Media Studio Hub">
            // Hero Header & Search Section
            <div class="media-hub-hero">
                <div class="media-hub-hero-content">
                    <span class="media-hub-hero-badge">"100% On-Device • Không Tải Lên Đám Mây"</span>
                    <h1 class="media-hub-hero-title">"Phòng Thu Đa Phương Tiện On-Device"</h1>
                    <p class="media-hub-hero-subtitle">
                        "Chọn công cụ bạn cần để bắt đầu. Mọi thao tác biên tập ảnh, video, âm thanh đều chạy trực tiếp trên máy tính của bạn."
                    </p>

                    // Ô tìm kiếm thời gian thực
                    <div class="media-hub-search-bar">
                        <span class="media-hub-search-icon" aria-hidden="true">"🔍"</span>
                        <input
                            type="text"
                            class="media-hub-search-input"
                            placeholder="Tìm kiếm nhanh trong 9 công cụ Media... (hoặc nhấn Ctrl+K)"
                            prop:value=move || search_query.get()
                            on:input=move |ev| search_query.set(event_target_value(&ev))
                            on:keydown=on_search_keydown
                        />
                        {move || {
                            if !search_query.get().is_empty() {
                                view! {
                                    <button
                                        type="button"
                                        class="media-hub-search-clear-btn"
                                        title="Xóa từ khóa tìm kiếm (Esc)"
                                        on:click=move |_| search_query.set(String::new())
                                    >
                                        "✕"
                                    </button>
                                }.into_any()
                            } else {
                                view! { <span style="display: none;"></span> }.into_any()
                            }
                        }}
                    </div>
                </div>
            </div>

            // Danh sách các Section theo từng nhóm nghiệp vụ
            <div class="media-hub-sections-container">
                {move || {
                    let q = search_query.get();

                    let rendered_categories = MediaCategory::ALL.into_iter().filter_map(|cat| {
                        // Lọc các công cụ thuộc danh mục hiện tại và khớp với từ khóa
                        let matching_tools: Vec<&'static ToolItem> = cat.slugs()
                            .iter()
                            .filter_map(|&slug| all_tools.iter().find(|t| t.slug == slug))
                            .filter(|t| matches_media_tool(t, &q))
                            .collect();

                        if matching_tools.is_empty() {
                            return None;
                        }

                        let cat_count = matching_tools.len();

                        Some(view! {
                            <section class="media-hub-section" aria-labelledby=format!("media-cat-heading-{}", cat.label())>
                                <div class="media-hub-section-header">
                                    <div class="media-hub-section-title-wrap">
                                        <span class="media-hub-section-icon" aria-hidden="true">{cat.icon()}</span>
                                        <div>
                                            <h2 class="media-hub-section-title" id=format!("media-cat-heading-{}", cat.label())>
                                                {category_full_title(cat)}
                                            </h2>
                                            <p class="media-hub-section-desc">{category_description(cat)}</p>
                                        </div>
                                    </div>
                                    <span class="media-hub-section-count">
                                        {format!("{} công cụ", cat_count)}
                                    </span>
                                </div>

                                <div class="media-hub-grid">
                                    {matching_tools.into_iter().map(|tool| {
                                        let slug = tool.slug;
                                        let icon = media_studio_tool_icon(slug);
                                        view! {
                                            <div
                                                class="media-hub-card"
                                                role="button"
                                                tabindex="0"
                                                title=format!("Mở công cụ {}", tool.name)
                                                on:click=move |_| state.open_tool(slug)
                                                on:keydown=move |ev| {
                                                    if ev.key() == "Enter" || ev.key() == " " {
                                                        ev.prevent_default();
                                                        state.open_tool(slug);
                                                    }
                                                }
                                            >
                                                <div class="media-hub-card-header">
                                                    <span class="media-hub-card-icon" aria-hidden="true">{icon}</span>
                                                    <span class="media-hub-card-badge">{cat.label()}</span>
                                                </div>
                                                <h3 class="media-hub-card-title">{tool.name}</h3>
                                                <p class="media-hub-card-desc">{tool.description}</p>
                                                <div class="media-hub-card-footer">
                                                    <span class="media-hub-card-slug">{tool.slug}</span>
                                                    <span class="media-hub-card-arrow" aria-hidden="true">"→"</span>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </section>
                        })
                    }).collect::<Vec<_>>();

                    // Kiểm tra xem từ khóa có khớp với workflow automation không
                    let q_lower = q.to_lowercase();
                    let q_unaccented = remove_vietnamese_diacritics(&q_lower);
                    let matches_workflow = q.is_empty()
                        || "workflow chuoi quy trinh tu dong hoa macro dag pipeline da phuong tien media".contains(&q_unaccented);

                    if rendered_categories.is_empty() && !matches_workflow {
                        view! {
                            <div class="media-hub-empty-search">
                                <span class="media-hub-empty-icon" aria-hidden="true">"🔍"</span>
                                <h3 class="media-hub-empty-title">"Không tìm thấy công cụ Media phù hợp"</h3>
                                <p class="media-hub-empty-desc">
                                    {format!("Không có kết quả nào cho từ khóa \"{}\". Hãy thử từ khóa khác như: ảnh thẻ, nén, che mờ, exif, gif...", q)}
                                </p>
                                <button
                                    type="button"
                                    class="media-hub-clear-search-btn"
                                    on:click=move |_| search_query.set(String::new())
                                >
                                    "Xem toàn bộ 9 công cụ"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="media-hub-sections-list">
                                {rendered_categories}

                                // Banner Card Section: Chuỗi Tự động hóa Macro DAG
                                {if matches_workflow {
                                    view! {
                                        <section class="media-hub-section media-hub-workflow-section" aria-label="Tự động hóa chuỗi quy trình">
                                            <div
                                                class="media-hub-workflow-banner"
                                                role="button"
                                                tabindex="0"
                                                title="Mở trình kết nối chuỗi quy trình Macro DAG cho đa phương tiện"
                                                on:click=move |_| state.open_workflow()
                                                on:keydown=move |ev| {
                                                    if ev.key() == "Enter" || ev.key() == " " {
                                                        ev.prevent_default();
                                                        state.open_workflow();
                                                    }
                                                }
                                            >
                                                <div class="media-hub-workflow-left">
                                                    <div class="media-hub-workflow-icon-wrap" aria-hidden="true">
                                                        <span class="media-hub-workflow-icon">"⚡"</span>
                                                    </div>
                                                    <div class="media-hub-workflow-info">
                                                        <div class="media-hub-workflow-tag-row">
                                                            <span class="media-hub-workflow-badge">"TỰ ĐỘNG HÓA PIPELINE"</span>
                                                            <span class="media-hub-workflow-highlight">"Macro DAG Engine"</span>
                                                        </div>
                                                        <h3 class="media-hub-workflow-title">"Chuỗi Quy Trình Tự Động Hóa Đa Phương Tiện"</h3>
                                                        <p class="media-hub-workflow-desc">
                                                            "Ghép nối liên hoàn nhiều bước xử lý (vd: Cắt ảnh thẻ → Nén WebP → Xóa Exif → Đóng dấu Watermark) thành một quy trình tự động hóa chạy 100% on-device."
                                                        </p>
                                                    </div>
                                                </div>
                                                <div class="media-hub-workflow-action">
                                                    <span class="media-hub-workflow-btn">
                                                        <span>"Mở Trình Tạo Chuỗi"</span>
                                                        <span class="workflow-btn-arrow" aria-hidden="true">"→"</span>
                                                    </span>
                                                </div>
                                            </div>
                                        </section>
                                    }.into_any()
                                } else {
                                    view! { <span style="display: none;"></span> }.into_any()
                                }}
                            </div>
                        }.into_any()
                    }
                }}
            </div>

            // =================================================================
            // Floating Contextual Widget (Mini Progress Pill / FAB) trên Hub
            // Chỉ hiển thị khi có tác vụ batch hoặc tiến trình ngầm đang chạy
            // =================================================================
            {move || {
                let show_floating = state.is_background_processing() || state.has_active_batches();
                if show_floating {
                    let total = state.batch_items_count();
                    let pct = (state.batch_progress_ratio() * 100.0) as u32;
                    let is_open = state.is_floating_queue_open.get();

                    view! {
                        <div class="media-floating-widget-container">
                            // Mini Progress Pill FAB
                            <button
                                type="button"
                                class="media-floating-pill-btn"
                                class:active=is_open
                                title="Xem tiến độ và can thiệp hàng đợi xử lý hàng loạt"
                                on:click=move |_| state.toggle_floating_queue()
                            >
                                <span class="pill-pulse-dot" aria-hidden="true"></span>
                                <span class="pill-icon" aria-hidden="true">"⚡"</span>
                                <span class="pill-label">
                                    {format!("Hàng đợi: {} tệp ({}%)", total, pct)}
                                </span>
                                <span class="pill-arrow" aria-hidden="true">
                                    {if is_open { "▼" } else { "▲" }}
                                </span>
                                <div
                                    class="pill-progress-track"
                                    style=format!("width: {}%;", pct)
                                ></div>
                            </button>

                            // Floating Popover Drawer trượt ra khi được kích hoạt
                            {if is_open {
                                let queue = state.batch_queue.get();
                                view! {
                                    <div class="media-floating-drawer" role="dialog" aria-label="Chi tiết hàng đợi đa phương tiện">
                                        <div class="floating-drawer-header">
                                            <div class="drawer-header-title-group">
                                                <span class="drawer-header-icon" aria-hidden="true">"⚡"</span>
                                                <h4 class="drawer-header-title">"Tiến Độ Hàng Đợi Media"</h4>
                                            </div>
                                            <button
                                                type="button"
                                                class="drawer-close-btn"
                                                title="Đóng bảng tiến độ"
                                                on:click=move |_| state.close_floating_queue()
                                            >
                                                "✕"
                                            </button>
                                        </div>

                                        <div class="floating-drawer-body">
                                            {if queue.is_empty() {
                                                view! {
                                                  <div class="drawer-empty-state">
                                                      <span>"Hàng đợi hiện đang trống."</span>
                                                  </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                  <div class="drawer-item-list" role="list">
                                                      {queue.into_iter().map(|item| {
                                                          let item_pct = (item.progress * 100.0) as u32;
                                                          let fname = item.file_name.clone();
                                                          let fname_title = item.file_name;
                                                          let status_str = item.status.clone();
                                                          let status_class = match item.status.as_str() {
                                                              "Processing" => "status-processing",
                                                              "Done" | "Hoàn thành" => "status-done",
                                                              "Failed" | "Lỗi" => "status-failed",
                                                              _ => "status-pending",
                                                          };
                                                          view! {
                                                              <div class="drawer-batch-item" role="listitem">
                                                                  <div class="drawer-item-info">
                                                                      <span class="drawer-item-name" title=fname_title>
                                                                          {fname}
                                                                      </span>
                                                                      <span class=format!("drawer-item-badge {}", status_class)>
                                                                          {status_str}
                                                                      </span>
                                                                  </div>
                                                                  <div class="drawer-item-progress-track">
                                                                      <div
                                                                          class="drawer-item-progress-fill"
                                                                          style=format!("width: {}%;", item_pct)
                                                                      ></div>
                                                                  </div>
                                                              </div>
                                                          }
                                                      }).collect::<Vec<_>>()}
                                                  </div>
                                                }.into_any()
                                            }}
                                        </div>

                                        <div class="floating-drawer-footer">
                                            <button
                                                type="button"
                                                class="drawer-action-btn drawer-clear-btn"
                                                title="Dọn sạch các tác vụ đã xử lý xong"
                                                on:click=move |_| state.clear_completed_batch()
                                            >
                                                "Dọn mục đã xong"
                                            </button>
                                            <button
                                                type="button"
                                                class="drawer-action-btn drawer-goto-btn"
                                                title="Chuyển vào Workspace để xem chi tiết đầy đủ"
                                                on:click=move |_| {
                                                    state.close_floating_queue();
                                                    let cur_slug = state.active_tool.get();
                                                    state.open_tool(&cur_slug);
                                                }
                                            >
                                                "Mở Workspace →"
                                            </button>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span style="display: none;"></span> }.into_any()
                            }}
                        </div>
                    }.into_any()
                } else {
                    view! { <span style="display: none;"></span> }.into_any()
                }
            }}
        </div>
    }
}
