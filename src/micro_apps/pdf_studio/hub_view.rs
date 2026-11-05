use leptos::prelude::*;
use crate::app::ToolItem;
use crate::launcher::remove_vietnamese_diacritics;
use crate::micro_apps::pdf_studio::state::{PdfCategory, PdfStudioState};
use crate::micro_apps::pdf_studio::toolbar::pdf_studio_tool_icon;

/// Danh sách các từ đồng nghĩa hoặc từ khóa bổ trợ cho từng slug công cụ PDF.
pub fn tool_synonyms(slug: &str) -> &'static str {
    match slug {
        "pdf-merge-split" => "gop gộp noi nối join hợp nhất hop nhat chia tách cắt cat",
        "pdf-compress" => "giảm dung lượng giam dung luong tối ưu toi uu optimize shrink nén nen",
        "pdf-protect-sign" => "khóa password lock mật mã mat ma bảo vệ bao ve mã hóa ma hoa",
        "pdf-table-extractor" => "bảng bang biểu bieu excel spreadsheet csv sheet trích xuất trich xuat",
        "vietnamese-ocr" => "nhận diện nhan dien chữ chu đọc doc quét scan hình ảnh hinh anh",
        "pdf-watermark-stamp" => "đóng dấu dong dau bản quyền ban quyen logo chìm chim mộc đỏ moc do",
        "pdf-page-number-header" => "đánh số trang danh so bates header footer tiêu đề tieu de",
        "pdf-redaction" => "bôi đen boi den che giấu che giau xóa nhạy cảm xoa mat",
        "pdf-metadata-scrubber" => "tẩy sạch tay sach xóa tác giả xoa tac gia exif metadata",
        "pdf-visual-diff" => "so sánh so sanh đối soát doi soat xem khác biệt khac biet",
        "pdf-booklet-imposition" => "in sách in sach gấp đôi gap doi n-up brochure sổ tay",
        "pdf-crop-margin" => "cắt lề cat le viền trắng vien trang auto crop xén xen",
        "pdf-bookmark-toc" => "mục lục muc luc cây mục lục cay muc luc toc bookmark",
        "pdf-to-markdown" => "chuyển sang md markdown text văn bản van ban",
        "pdf-to-long-image" => "ảnh dài anh dai cuộn dọc cuon doc ghép ảnh ghep anh png jpg",
        "markdown-to-styled-pdf" => "soạn thảo soan thao tài liệu tai lieu in ấn in an css style",
        "pdf-page-organizer" => "sắp xếp sap xep đảo trang dao trang xoay reorder rotate organize",
        "pdf-signature-pad" => "ký tay ky tay chữ ký chu ky signature stamp vẽ ve bút but",
        "pdf-to-images-pack" => "xuất ảnh xuat anh gói ảnh goi anh zip extract images pack png jpeg",
        _ => "",
    }
}

/// Kiểm tra một công cụ PDF có khớp với chuỗi tìm kiếm hay không.
/// Hỗ trợ không phân biệt hoa thường và tự động bóc tách dấu tiếng Việt (diacritics-insensitive).
pub fn matches_pdf_tool(tool: &ToolItem, query: &str) -> bool {
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

    let synonyms = tool_synonyms(tool.slug);
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

/// Lọc danh sách công cụ PDF theo từ khóa tìm kiếm.
pub fn filter_pdf_studio_tools<'a>(tools: &'a [ToolItem], query: &str) -> Vec<&'a ToolItem> {
    tools.iter().filter(|t| matches_pdf_tool(t, query)).collect()
}

/// Tiêu đề chi tiết đầy đủ của danh mục PDF.
pub fn category_full_title(cat: PdfCategory) -> &'static str {
    match cat {
        PdfCategory::MergeSplit => "Tổ chức & Bố cục trang",
        PdfCategory::SecurityOptimize => "Bảo mật & Biểu mẫu",
        PdfCategory::FormatPage => "Định dạng & Đóng dấu",
        PdfCategory::ConvertOcr => "Chuyển đổi & OCR",
    }
}

/// Mô tả tóm tắt tính năng của từng danh mục.
pub fn category_description(cat: PdfCategory) -> &'static str {
    match cat {
        PdfCategory::MergeSplit => "Ghép, tách, cắt lề, dàn trang in sách, mục lục & đối soát trực quan",
        PdfCategory::SecurityOptimize => "Mật khẩu, nén tối ưu, tẩy sạch siêu dữ liệu, ký số PKI & phục hồi lỗi",
        PdfCategory::FormatPage => "Đánh số trang Bates, đóng dấu watermark bản quyền & chuyển đen trắng",
        PdfCategory::ConvertOcr => "Nhận dạng chữ tiếng Việt, chuyển đổi Markdown, ảnh dài & trích xuất bảng Excel",
    }
}

/// Component hiển thị Trang chủ Hub/Portal của PDF Studio theo mô hình Categorized Card Grid.
///
/// Người dùng chọn công cụ nghiệp vụ trước rồi mới nạp tệp vào không gian xử lý tập trung.
#[component]
pub fn PdfStudioHubView(
    state: PdfStudioState,
) -> impl IntoView {
    let search_query = RwSignal::new(String::new());

    // Lắng nghe phím tắt Esc để xóa tìm kiếm
    let on_search_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            search_query.set(String::new());
        }
    };

    let all_tools = state.tools();

    view! {
        <div class="pdf-studio-hub-view" role="region" aria-label="Trang chủ PDF Studio Hub">
            // Hero Header & Search Section
            <div class="pdf-hub-hero">
                <div class="pdf-hub-hero-content">
                    <span class="pdf-hub-hero-badge">"100% On-Device • Không Giới Hạn Dung Lượng"</span>
                    <h1 class="pdf-hub-hero-title">"Xưởng Tài Liệu PDF Đa Năng"</h1>
                    <p class="pdf-hub-hero-subtitle">
                        "Chọn công cụ bạn cần để bắt đầu. Mọi thao tác xử lý an toàn ngay trên máy tính của bạn."
                    </p>

                    // Ô tìm kiếm thời gian thực
                    <div class="pdf-hub-search-bar">
                        <span class="pdf-hub-search-icon" aria-hidden="true">"🔍"</span>
                        <input
                            type="text"
                            class="pdf-hub-search-input"
                            placeholder="Tìm kiếm nhanh trong 22 công cụ PDF... (hoặc nhấn Ctrl+K)"
                            prop:value=move || search_query.get()
                            on:input=move |ev| search_query.set(event_target_value(&ev))
                            on:keydown=on_search_keydown
                        />
                        {move || {
                            if !search_query.get().is_empty() {
                                view! {
                                    <button
                                        type="button"
                                        class="pdf-hub-search-clear-btn"
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
            <div class="pdf-hub-sections-container">
                {move || {
                    let q = search_query.get();
                    let mut total_matches = 0;

                    let rendered_categories = PdfCategory::ALL.into_iter().filter_map(|cat| {
                        // Lọc các công cụ thuộc danh mục hiện tại và khớp với từ khóa
                        let matching_tools: Vec<&'static ToolItem> = cat.slugs()
                            .iter()
                            .filter_map(|&slug| all_tools.iter().find(|t| t.slug == slug))
                            .filter(|t| matches_pdf_tool(t, &q))
                            .collect();

                        if matching_tools.is_empty() {
                            return None;
                        }

                        total_matches += matching_tools.len();
                        let cat_slugs_count = matching_tools.len();

                        Some(view! {
                            <section class="pdf-hub-section" aria-labelledby=format!("cat-heading-{}", cat.label())>
                                <div class="pdf-hub-section-header">
                                    <div class="pdf-hub-section-title-wrap">
                                        <span class="pdf-hub-section-icon" aria-hidden="true">{cat.icon()}</span>
                                        <div>
                                            <h2 class="pdf-hub-section-title" id=format!("cat-heading-{}", cat.label())>
                                                {category_full_title(cat)}
                                            </h2>
                                            <p class="pdf-hub-section-desc">{category_description(cat)}</p>
                                        </div>
                                    </div>
                                    <span class="pdf-hub-section-count">
                                        {format!("{} công cụ", cat_slugs_count)}
                                    </span>
                                </div>

                                <div class="pdf-hub-grid">
                                    {matching_tools.into_iter().map(|tool| {
                                        let slug = tool.slug;
                                        let icon = pdf_studio_tool_icon(slug);
                                        view! {
                                            <div
                                                class="pdf-hub-card"
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
                                                <div class="pdf-hub-card-header">
                                                    <span class="pdf-hub-card-icon" aria-hidden="true">{icon}</span>
                                                    <span class="pdf-hub-card-badge">{cat.label()}</span>
                                                </div>
                                                <h3 class="pdf-hub-card-title">{tool.name}</h3>
                                                <p class="pdf-hub-card-desc">{tool.description}</p>
                                                <div class="pdf-hub-card-footer">
                                                    <span class="pdf-hub-card-slug">{tool.slug}</span>
                                                    <span class="pdf-hub-card-arrow" aria-hidden="true">"→"</span>
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
                        || "workflow chuoi quy trinh tu dong hoa macro dag pipeline".contains(&q_unaccented);

                    if rendered_categories.is_empty() && !matches_workflow {
                        view! {
                            <div class="pdf-hub-empty-search">
                                <span class="pdf-hub-empty-icon" aria-hidden="true">"🔍"</span>
                                <h3 class="pdf-hub-empty-title">"Không tìm thấy công cụ PDF phù hợp"</h3>
                                <p class="pdf-hub-empty-desc">
                                    {format!("Không có kết quả nào cho từ khóa \"{}\". Hãy thử từ khóa khác như: gộp, nén, ocr, mật khẩu...", q)}
                                </p>
                                <button
                                    type="button"
                                    class="pdf-hub-clear-search-btn"
                                    on:click=move |_| search_query.set(String::new())
                                >
                                    "Xem toàn bộ 22 công cụ"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="pdf-hub-sections-list">
                                {rendered_categories}

                                // Banner Card Section: Chuỗi Tự động hóa Macro DAG
                                {if matches_workflow {
                                    view! {
                                        <section class="pdf-hub-section pdf-hub-workflow-section" aria-label="Tự động hóa chuỗi quy trình">
                                            <div
                                                class="pdf-hub-workflow-banner"
                                                role="button"
                                                tabindex="0"
                                                title="Mở trình kết nối chuỗi quy trình Macro DAG cho tài liệu PDF"
                                                on:click=move |_| state.open_workflow()
                                                on:keydown=move |ev| {
                                                    if ev.key() == "Enter" || ev.key() == " " {
                                                        ev.prevent_default();
                                                        state.open_workflow();
                                                    }
                                                }
                                            >
                                                <div class="pdf-hub-workflow-left">
                                                    <div class="pdf-hub-workflow-icon-wrap" aria-hidden="true">
                                                        <span class="pdf-hub-workflow-icon">"⚡"</span>
                                                    </div>
                                                    <div class="pdf-hub-workflow-info">
                                                        <div class="pdf-hub-workflow-tag-row">
                                                            <span class="pdf-hub-workflow-badge">"TỰ ĐỘNG HÓA PIPELINE"</span>
                                                            <span class="pdf-hub-workflow-highlight">"Macro DAG Engine"</span>
                                                        </div>
                                                        <h3 class="pdf-hub-workflow-title">"Chuỗi Quy Trình Tự Động Hóa PDF"</h3>
                                                        <p class="pdf-hub-workflow-desc">
                                                            "Ghép nối liên hoàn nhiều công cụ PDF (vd: Tách trang → Nén dung lượng → Đóng dấu Watermark → Ký số) thành một quy trình chạy tự động 100% on-device."
                                                        </p>
                                                    </div>
                                                </div>
                                                <div class="pdf-hub-workflow-action">
                                                    <span class="pdf-hub-workflow-btn">
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
        </div>
    }
}
