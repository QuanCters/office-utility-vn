use leptos::prelude::*;
use crate::app::ToolItem;
use crate::launcher::remove_vietnamese_diacritics;
use crate::micro_apps::pdf_studio::state::{category_for_tool, PdfCategory, PdfStudioState};
use crate::micro_apps::pdf_studio::toolbar::pdf_studio_tool_icon;
use crate::shared_ui::BackButton;

/// Danh sách slug các công cụ PDF được ghim phổ biến nhất.
pub const PINNED_PDF_SLUGS: &[&str] = &[
    "pdf-merge-split",
    "pdf-compress",
    "vietnamese-ocr",
    "pdf-table-extractor",
    "pdf-protect-sign",
    "pdf-page-organizer",
];

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

/// Component hiển thị Trang chủ Hub/Portal chính của PDF Studio.
///
/// Tích hợp đầy đủ:
/// - Hero Header & Primary Open PDF Action (Dropzone lớn, File Picker, Quick Action Chips)
/// - Sidebar Navigation (Bộ lọc danh mục, Công cụ ghim, Tài liệu gần đây)
/// - Pinned Tools Showcase (6 công cụ phổ biến nhất)
/// - Recent Documents List (Danh sách tệp đang làm việc, chọn active, xóa)
/// - Real-Time Search Bar (Tìm kiếm tiếng Việt không dấu & từ đồng nghĩa)
/// - Categorized Tool Cards Grid (25 công cụ PDF on-device)
/// - Workflow Automation Banner (Macro DAG Pipeline)
#[component]
pub fn PdfStudioHubView(
    state: PdfStudioState,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    let search_query = RwSignal::new(String::new());
    let is_dragging = RwSignal::new(false);
    let view_filter = RwSignal::new("all".to_string()); // "all", "pinned", "recent", or category slug

    // Kích hoạt hộp thoại chọn file của hệ điều hành
    let on_open_picker = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if crate::micro_apps::pdf_studio::bridge::is_tauri_runtime() {
                let s = state;
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = crate::micro_apps::pdf_studio::bridge::open_file_into_pdf_studio("pdfcraft-frame", s).await;
                });
            } else {
                let _ = js_sys::eval("document.getElementById('pdf-hub-native-picker')?.click()");
            }
        }
    };

    // Lắng nghe sự kiện người dùng chọn tệp từ thẻ input
    let on_file_input_change = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Ok(js_val) = js_sys::eval("Array.from(document.getElementById('pdf-hub-native-picker')?.files || []).map(f => f.path || f.name)") {
                if let Ok(names) = serde_wasm_bindgen::from_value::<Vec<String>>(js_val) {
                    if !names.is_empty() {
                        state.add_files(names);
                        let _ = js_sys::eval("var el = document.getElementById('pdf-hub-native-picker'); if (el) el.value = '';");
                    }
                }
            }
        }
    };

    // Lắng nghe phím tắt Esc để xóa tìm kiếm
    let on_search_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            search_query.set(String::new());
        }
    };

    let all_tools = state.tools();

    view! {
        <div class="pdf-studio-hub-container" role="region" aria-label="Trang chủ PDF Studio Hub">
            // Thẻ input ẩn chọn tệp PDF
            <input
                type="file"
                id="pdf-hub-native-picker"
                accept=".pdf,application/pdf"
                multiple=true
                style="display: none;"
                on:change=on_file_input_change
            />

            // =========================================================
            // HEADER TRANG CHỦ PDF STUDIO
            // =========================================================
            <header class="pdf-studio-header pdf-hub-top-header">
                <div class="pdf-studio-header-left">
                    <BackButton on_back=on_back label="← Về Launcher" />
                    <div class="pdf-studio-header-title-group">
                        <h1 class="pdf-studio-header-title">
                            <span class="pdf-hub-title-icon" aria-hidden="true">"📄"</span>
                            <span>"PDF Studio"</span>
                        </h1>
                        <span class="pdf-studio-header-tagline">
                            "Xưởng Xử Lý, Biên Tập & Số Hóa Tài Liệu PDF Chuyên Nghiệp"
                        </span>
                    </div>
                </div>

                <div class="pdf-studio-header-right">
                    <button
                        type="button"
                        class="pdf-hub-canvas-btn"
                        on:click=move |_| state.open_visual_workspace()
                        title="Mở trình biên tập đồ họa trực quan WebAssembly Canvas"
                    >
                        <span>"🎨 Không gian Canvas"</span>
                    </button>

                    <button
                        type="button"
                        class="pdf-hub-workflow-btn-header"
                        on:click=move |_| state.open_workflow()
                        title="Mở trình kết nối chuỗi quy trình tự động hóa Macro DAG"
                    >
                        <span>"⚡ Macro DAG"</span>
                    </button>

                    <span class="pdf-studio-privacy-badge">
                        <svg class="privacy-badge-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path>
                        </svg>
                        <span>"100% On-Device"</span>
                    </span>
                </div>
            </header>

            // =========================================================
            // BỐ CỤC 2 CỘT: SIDEBAR ĐIỀU HƯỚNG + MAIN PORTAL
            // =========================================================
            <div class="pdf-hub-body-layout">
                // -----------------------------------------------------
                // CỘT TRÁI: SIDEBAR ĐIỀU HƯỚNG (Categories, Pinned, Recent)
                // -----------------------------------------------------
                <aside class="pdf-hub-sidebar" aria-label="Điều hướng danh mục PDF Studio">
                    <div class="pdf-hub-sidebar-section">
                        <span class="pdf-hub-sidebar-heading">"ĐIỀU HƯỚNG NHANH"</span>

                        <button
                            type="button"
                            class="pdf-hub-nav-item"
                            class:active=move || view_filter.get() == "all"
                            on:click=move |_| {
                                view_filter.set("all".to_string());
                                state.set_hub_category(None);
                            }
                        >
                            <span class="nav-icon" aria-hidden="true">"📚"</span>
                            <span class="nav-label">"Tất cả công cụ"</span>
                            <span class="nav-count">"25"</span>
                        </button>

                        <button
                            type="button"
                            class="pdf-hub-nav-item"
                            class:active=move || view_filter.get() == "pinned"
                            on:click=move |_| {
                                view_filter.set("pinned".to_string());
                                state.set_hub_category(None);
                            }
                        >
                            <span class="nav-icon" aria-hidden="true">"📌"</span>
                            <span class="nav-label">"Công cụ đã ghim"</span>
                            <span class="nav-count">"6"</span>
                        </button>

                        <button
                            type="button"
                            class="pdf-hub-nav-item"
                            class:active=move || view_filter.get() == "recent"
                            on:click=move |_| {
                                view_filter.set("recent".to_string());
                            }
                        >
                            <span class="nav-icon" aria-hidden="true">"🕒"</span>
                            <span class="nav-label">"Tài liệu gần đây"</span>
                            <span class="nav-count">
                                {move || format!("{}", state.loaded_files_count())}
                            </span>
                        </button>
                    </div>

                    <div class="pdf-hub-sidebar-divider"></div>

                    <div class="pdf-hub-sidebar-section">
                        <span class="pdf-hub-sidebar-heading">"DANH MỤC NGHIỆP VỤ"</span>

                        {PdfCategory::ALL.into_iter().map(|cat| {
                            let label = cat.label();
                            let icon = cat.icon();
                            let count = cat.slugs().len();
                            let slug_id = format!("{:?}", cat);
                            let slug_id_active = slug_id.clone();
                            view! {
                                <button
                                    type="button"
                                    class="pdf-hub-nav-item"
                                    class:active=move || view_filter.get() == slug_id_active
                                    on:click=move |_| {
                                        view_filter.set(slug_id.clone());
                                        state.set_hub_category(Some(cat));
                                    }
                                >
                                    <span class="nav-icon" aria-hidden="true">{icon}</span>
                                    <span class="nav-label">{label}</span>
                                    <span class="nav-count">{format!("{}", count)}</span>
                                </button>
                            }
                        }).collect::<Vec<_>>()}
                    </div>

                    <div class="pdf-hub-sidebar-divider"></div>

                    <div class="pdf-hub-sidebar-section">
                        <span class="pdf-hub-sidebar-heading">"TỰ ĐỘNG HÓA"</span>

                        <button
                            type="button"
                            class="pdf-hub-nav-item"
                            on:click=move |_| state.open_workflow()
                        >
                            <span class="nav-icon" aria-hidden="true">"⚡"</span>
                            <span class="nav-label">"Chuỗi Macro DAG"</span>
                            <span class="nav-arrow" aria-hidden="true">"→"</span>
                        </button>
                    </div>

                    // Hộp thông tin bảo mật On-Device
                    <div class="pdf-hub-sidebar-info-box">
                        <div class="info-box-header">
                            <span class="info-box-icon" aria-hidden="true">"🛡️"</span>
                            <span class="info-box-title">"Zero-Cloud Security"</span>
                        </div>
                        <p class="info-box-text">
                            "Mọi tệp PDF được xử lý hoàn toàn ngoại tuyến qua Rust WebAssembly & Rayon. Không truyền dữ liệu qua Internet."
                        </p>
                    </div>
                </aside>

                // -----------------------------------------------------
                // CỘT PHẢI: KHÔNG GIAN NỘI DUNG CHÍNH (Scrollable Portal)
                // -----------------------------------------------------
                <main class="pdf-hub-main-content">
                    // =================================================
                    // 1. PRIMARY "OPEN PDF" ACTION HERO DROPZONE
                    // =================================================
                    <section
                        class="pdf-hub-open-zone"
                        class:dragging=move || is_dragging.get()
                        aria-label="Khu vực mở tệp PDF"
                        on:dragover=move |ev| {
                            ev.prevent_default();
                            is_dragging.set(true);
                        }
                        on:dragleave=move |ev| {
                            ev.prevent_default();
                            is_dragging.set(false);
                        }
                        on:drop=move |ev: leptos::ev::DragEvent| {
                            ev.prevent_default();
                            is_dragging.set(false);
                            #[cfg(target_arch = "wasm32")]
                            {
                                let event: &web_sys::Event = ev.as_ref();
                                if let Ok(dt) = js_sys::Reflect::get(event, &wasm_bindgen::JsValue::from_str("dataTransfer")) {
                                    if let Ok(files) = js_sys::Reflect::get(&dt, &wasm_bindgen::JsValue::from_str("files")) {
                                        let files_arr = js_sys::Array::from(&files);
                                        let mut names = Vec::new();
                                        for i in 0..files_arr.length() {
                                            let file_obj = files_arr.get(i);
                                            let name = js_sys::Reflect::get(&file_obj, &wasm_bindgen::JsValue::from_str("name"))
                                                .ok()
                                                .and_then(|v| v.as_string());
                                            if let Some(n) = name {
                                                names.push(n);
                                            }
                                        }
                                        if !names.is_empty() {
                                            state.add_files(names);
                                        }
                                    }
                                }
                            }
                        }
                    >
                        <div class="pdf-open-zone-inner">
                            <div class="pdf-open-zone-icon-wrap" aria-hidden="true">
                                <span class="pdf-open-zone-icon">"📄"</span>
                            </div>

                            <div class="pdf-open-zone-text">
                                <h2 class="pdf-open-zone-title">
                                    "Mở tài liệu PDF để bắt đầu"
                                </h2>
                                <p class="pdf-open-zone-subtitle">
                                    "Kéo & thả tệp PDF vào đây hoặc duyệt từ máy tính. Tệp sẽ sẵn sàng cho toàn bộ 25 công cụ xử lý on-device."
                                </p>
                            </div>

                            <div class="pdf-open-zone-actions">
                                <button
                                    type="button"
                                    class="pdf-open-primary-btn"
                                    on:click=on_open_picker
                                    title="Chọn tệp PDF từ máy tính để nạp vào không gian làm việc"
                                >
                                    <span class="btn-icon" aria-hidden="true">"📂"</span>
                                    <span>"Chọn tệp PDF từ máy tính"</span>
                                </button>

                                <button
                                    type="button"
                                    class="pdf-open-canvas-btn"
                                    on:click=move |_| state.open_visual_workspace()
                                    title="Chuyển sang chế độ vẽ và biên tập trực quan Canvas"
                                >
                                    <span class="btn-icon" aria-hidden="true">"🎨"</span>
                                    <span>"Trình Biên Tập Trực Quan (Canvas)"</span>
                                </button>
                            </div>

                            // Quick Tool Chips
                            <div class="pdf-open-zone-quick-chips">
                                <span class="quick-chips-label">"Hoặc chọn tác vụ nhanh:"</span>
                                <button type="button" class="quick-chip-btn" on:click=move |_| state.open_tool("pdf-merge-split")>
                                    "📑 Ghép / Tách trang"
                                </button>
                                <button type="button" class="quick-chip-btn" on:click=move |_| state.open_tool("pdf-compress")>
                                    "🗜️ Nén PDF"
                                </button>
                                <button type="button" class="quick-chip-btn" on:click=move |_| state.open_tool("vietnamese-ocr")>
                                    "🇻🇳 OCR Tiếng Việt"
                                </button>
                                <button type="button" class="quick-chip-btn" on:click=move |_| state.open_tool("pdf-table-extractor")>
                                    "📊 Xuất Excel"
                                </button>
                                <button type="button" class="quick-chip-btn" on:click=move |_| state.open_tool("pdf-protect-sign")>
                                    "🛡️ Bảo mật & Khóa"
                                </button>
                            </div>
                        </div>
                    </section>

                    // =================================================
                    // 2. RECENT DOCUMENTS / LOADED FILES SHELF
                    // =================================================
                    <section class="pdf-hub-recent-shelf" aria-label="Tài liệu gần đây trong phiên làm việc">
                        <div class="recent-shelf-header">
                            <div class="recent-shelf-title-wrap">
                                <span class="recent-shelf-icon" aria-hidden="true">"🕒"</span>
                                <h3 class="recent-shelf-title">"Tài liệu trong không gian làm việc"</h3>
                                <span class="recent-shelf-count">
                                    {move || format!("{} tệp", state.loaded_files_count())}
                                </span>
                            </div>

                            {move || {
                                if state.has_files() {
                                    view! {
                                        <button
                                            type="button"
                                            class="recent-clear-btn"
                                            title="Xóa danh sách tệp làm việc tạm thời"
                                            on:click=move |_| state.clear_files()
                                        >
                                            "✕ Xóa danh sách"
                                        </button>
                                    }.into_any()
                                } else {
                                    view! { <span style="display: none;"></span> }.into_any()
                                }
                            }}
                        </div>

                        {move || {
                            let files = state.loaded_files.get();
                            let active_idx = state.active_file_index.get();

                            if files.is_empty() {
                                view! {
                                    <div class="recent-shelf-empty">
                                        <span class="empty-icon" aria-hidden="true">"💡"</span>
                                        <div class="empty-text">
                                            <span class="empty-title">"Chưa có tài liệu nào được nạp"</span>
                                            <span class="empty-desc">
                                                "Kéo thả tệp PDF vào khung phía trên hoặc nhấn \"Chọn tệp PDF\" để bắt đầu."
                                            </span>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="recent-files-grid">
                                        {files.into_iter().enumerate().map(|(idx, path)| {
                                            let is_active = active_idx == Some(idx);
                                            let display_name = path.rsplit(['/', '\\']).next().unwrap_or(&path).to_string();
                                            let card_title = format!("Chọn tệp {}", display_name);
                                            view! {
                                                <div
                                                    class="recent-file-card"
                                                    class:active=is_active
                                                    role="button"
                                                    tabindex="0"
                                                    title=card_title
                                                    on:click=move |_| state.select_file(idx)
                                                >
                                                    <div class="file-card-icon-col">
                                                        <span class="file-icon" aria-hidden="true">"📄"</span>
                                                        <span class="file-idx">{format!("#{}", idx + 1)}</span>
                                                    </div>
                                                    <div class="file-card-details">
                                                        <span class="file-name">{display_name}</span>
                                                        <span class="file-path">{path.clone()}</span>
                                                        {if is_active {
                                                            view! {
                                                                <span class="file-active-badge">
                                                                    "✓ Đang chọn làm tệp chính"
                                                                </span>
                                                            }.into_any()
                                                        } else {
                                                            view! { <span style="display: none;"></span> }.into_any()
                                                        }}
                                                    </div>
                                                    <div class="file-card-actions">
                                                        <button
                                                            type="button"
                                                            class="file-remove-btn"
                                                            title="Xóa tệp khỏi danh sách"
                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                ev.stop_propagation();
                                                                state.remove_file(idx);
                                                            }
                                                        >
                                                            "✕"
                                                        </button>
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any()
                            }
                        }}
                    </section>

                    // =================================================
                    // 3. PINNED TOOLS SHOWCASE SECTION
                    // =================================================
                    {move || {
                        let filter = view_filter.get();
                        let q = search_query.get();
                        // Chỉ hiển thị section Pinned khi đang ở chế độ xem all hoặc pinned và không có search lọc đặc biệt
                        if (filter == "all" || filter == "pinned") && q.trim().is_empty() {
                            view! {
                                <section class="pdf-hub-pinned-section" aria-label="Công cụ phổ biến được ghim">
                                    <div class="pinned-section-header">
                                        <div class="pinned-title-wrap">
                                            <span class="pinned-icon" aria-hidden="true">"📌"</span>
                                            <h3 class="pinned-section-title">
                                                "CÔNG CỤ PHỔ BIẾN ĐƯỢC GHIM"
                                            </h3>
                                        </div>
                                        <span class="pinned-section-badge">"6 công cụ thiết yếu"</span>
                                    </div>

                                    <div class="pinned-tools-grid">
                                        {PINNED_PDF_SLUGS.iter().filter_map(|&slug| {
                                            all_tools.iter().find(|t| t.slug == slug)
                                        }).map(|tool| {
                                            let slug = tool.slug;
                                            let icon = pdf_studio_tool_icon(slug);
                                            let cat_label = category_for_tool(slug).map(|c| c.label()).unwrap_or("PDF");
                                            view! {
                                                <div
                                                    class="pinned-tool-card"
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
                                                    <div class="pinned-card-header">
                                                        <span class="pinned-card-icon" aria-hidden="true">{icon}</span>
                                                        <span class="pinned-card-badge">{cat_label}</span>
                                                    </div>
                                                    <h4 class="pinned-card-title">{tool.name}</h4>
                                                    <p class="pinned-card-desc">{tool.description}</p>
                                                    <div class="pinned-card-footer">
                                                        <span class="pinned-card-slug">{tool.slug}</span>
                                                        <span class="pinned-card-action">"Mở công cụ →"</span>
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </section>
                            }.into_any()
                        } else {
                            view! { <span style="display: none;"></span> }.into_any()
                        }
                    }}

                    // =================================================
                    // 4. SEARCH BAR SECTION
                    // =================================================
                    <section class="pdf-hub-search-section" aria-label="Tìm kiếm công cụ PDF">
                        <div class="pdf-hub-search-bar">
                            <span class="pdf-hub-search-icon" aria-hidden="true">"🔍"</span>
                            <input
                                type="text"
                                class="pdf-hub-search-input"
                                placeholder="Tìm kiếm nhanh trong 25 công cụ PDF... (hoặc nhấn Ctrl+K)"
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
                    </section>

                    // =================================================
                    // 5. CATEGORIZED TOOL CARDS GRID
                    // =================================================
                    <div class="pdf-hub-sections-container">
                        {move || {
                            let q = search_query.get();
                            let filter = view_filter.get();

                            // Lọc danh mục hiển thị dựa trên sidebar filter
                            let categories_to_render: Vec<PdfCategory> = match filter.as_str() {
                                "pinned" => vec![], // Đã hiển thị ở phần pinned phía trên
                                "recent" => vec![], // Đang tập trung xem recent files
                                _ => {
                                    if let Some(cat) = state.hub_active_category.get() {
                                        vec![cat]
                                    } else {
                                        PdfCategory::ALL.to_vec()
                                    }
                                }
                            };

                            let mut total_matches = 0;

                            let rendered_categories = categories_to_render.into_iter().filter_map(|cat| {
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
                                                    <h3 class="pdf-hub-section-title" id=format!("cat-heading-{}", cat.label())>
                                                        {category_full_title(cat)}
                                                    </h3>
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
                                                        <h4 class="pdf-hub-card-title">{tool.name}</h4>
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

                            // Kiểm tra từ khóa workflow
                            let q_lower = q.to_lowercase();
                            let q_unaccented = remove_vietnamese_diacritics(&q_lower);
                            let matches_workflow = (q.is_empty() || "workflow chuoi quy trinh tu dong hoa macro dag pipeline".contains(&q_unaccented))
                                && filter != "recent";

                            if rendered_categories.is_empty() && !matches_workflow && filter != "recent" && filter != "pinned" {
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
                                            on:click=move |_| {
                                                search_query.set(String::new());
                                                state.set_hub_category(None);
                                                view_filter.set("all".to_string());
                                            }
                                        >
                                            "Xem toàn bộ 25 công cụ"
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
                </main>
            </div>
        </div>
    }
}
