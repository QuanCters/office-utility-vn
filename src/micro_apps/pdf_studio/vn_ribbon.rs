use leptos::prelude::*;
use crate::micro_apps::pdf_studio::state::PdfStudioState;
use crate::shared_ui::open_output_folder_ipc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: wasm_bindgen::JsValue) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

/// 6 công cụ độc bản Việt Nam được bảo tồn và gắn trực tiếp trên thanh Action Ribbon
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VnRibbonTool {
    VietnameseOcr,
    TableExtractor,
    ToLongImage,
    BookletImposition,
    GrayscalePrint,
    RepairCorrupt,
}

impl VnRibbonTool {
    pub const ALL: [Self; 6] = [
        Self::VietnameseOcr,
        Self::TableExtractor,
        Self::ToLongImage,
        Self::BookletImposition,
        Self::GrayscalePrint,
        Self::RepairCorrupt,
    ];

    pub fn slug(&self) -> &'static str {
        match self {
            Self::VietnameseOcr => "vietnamese-ocr",
            Self::TableExtractor => "pdf-table-extractor",
            Self::ToLongImage => "pdf-to-long-image",
            Self::BookletImposition => "pdf-booklet-imposition",
            Self::GrayscalePrint => "pdf-grayscale-print",
            Self::RepairCorrupt => "pdf-repair-corrupt",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::VietnameseOcr => "🇻🇳",
            Self::TableExtractor => "📊",
            Self::ToLongImage => "📜",
            Self::BookletImposition => "📖",
            Self::GrayscalePrint => "🖨️",
            Self::RepairCorrupt => "🛠️",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::VietnameseOcr => "Nhận dạng OCR tiếng Việt",
            Self::TableExtractor => "Xuất bảng biểu sang Excel",
            Self::ToLongImage => "Ghép ảnh dài Infographic",
            Self::BookletImposition => "In sách đóng kim Booklet",
            Self::GrayscalePrint => "In tiết kiệm mực Grayscale",
            Self::RepairCorrupt => "Khôi phục PDF lỗi",
        }
    }

    pub fn short_desc(&self) -> &'static str {
        match self {
            Self::VietnameseOcr => "Nhận dạng ký tự tiếng Việt có dấu 100% On-Device, hỗ trợ xuất Word, Text, Markdown & Searchable PDF",
            Self::TableExtractor => "Tự động nhận diện cấu trúc lưới và trích xuất bảng biểu từ PDF sang file Excel (.xlsx) hoặc CSV",
            Self::ToLongImage => "Nối liên hoàn các trang tài liệu PDF thành ảnh dọc dài (Infographic) độ nét cao",
            Self::BookletImposition => "Dàn trang in sách gấp đôi khổ A4/A3 theo quy chuẩn đóng kim gáy (Saddle Stitch)",
            Self::GrayscalePrint => "Chuyển PDF và hình ảnh sang đen trắng Grayscale tối ưu hóa tiết kiệm mực in",
            Self::RepairCorrupt => "Khôi phục và tái tạo cấu trúc xref cho file PDF bị hỏng hoặc tải về gián đoạn",
        }
    }

    pub fn is_pdf_output(&self) -> bool {
        matches!(self, Self::BookletImposition | Self::GrayscalePrint | Self::RepairCorrupt)
    }

    /// Khởi tạo VnRibbonTool từ slug công cụ nếu thuộc 6 tiện ích Việt Nam.
    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.slug() == slug)
    }
}

/// Component `PdfVnExtensionRibbon` hiển thị thanh Action Bar chứa 6 công cụ tiện ích độc bản Việt Nam
/// được đặt ngay phía trên không gian PDFCraft Canvas.
#[component]
pub fn PdfVnExtensionRibbon(
    state: PdfStudioState,
) -> impl IntoView {
    // Trạng thái ngăn drawer/popover cấu hình nhanh đang mở (đồng bộ với initial tool nếu có)
    let initial_ribbon = VnRibbonTool::from_slug(&state.active_tool.get_untracked());
    let active_tool = RwSignal::new(initial_ribbon);

    // Đồng bộ reactive khi deep link hoặc state.active_tool thay đổi trong môi trường browser/WASM
    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            let current_slug = state.active_tool.get();
            if let Some(tool) = VnRibbonTool::from_slug(&current_slug) {
                active_tool.set(Some(tool));
            }
        });
    }

    // Trạng thái thực thi tác vụ
    let is_executing = RwSignal::new(false);
    let progress_pct = RwSignal::new(0.0f32);
    let progress_msg = RwSignal::new(String::new());
    let result_msg = RwSignal::new(None::<String>);
    let error_msg = RwSignal::new(None::<String>);
    #[allow(unused_variables)]
    let last_output_pdf = RwSignal::new(None::<String>);

    // Options cho từng công cụ
    // 1. Vietnamese OCR
    let ocr_engine = RwSignal::new("fast_os_native".to_string());
    let ocr_format = RwSignal::new("docx".to_string());

    // 2. Table Extractor
    let table_range = RwSignal::new(String::new());
    let table_csv = RwSignal::new(false);

    // 3. To Long Image
    let image_format = RwSignal::new("png".to_string());
    let image_dpi = RwSignal::new(150u32);
    let image_divider = RwSignal::new(true);

    // 4. Booklet Imposition
    let booklet_mode = RwSignal::new("saddle_stitch".to_string());
    let booklet_paper = RwSignal::new("A4".to_string());

    // 5. Grayscale Print
    let grayscale_images = RwSignal::new(true);

    // 6. Repair Corrupt
    let repair_depth = RwSignal::new("Quét tuyến tính tái tạo xref".to_string());

    // Chuyển đổi trạng thái đóng/mở cấu hình nhanh
    let toggle_tool = move |tool: VnRibbonTool| {
        if active_tool.get() == Some(tool) {
            active_tool.set(None);
        } else {
            active_tool.set(Some(tool));
            error_msg.set(None);
            result_msg.set(None);
        }
    };

    // Hàm gọi IPC thực thi công cụ
    let run_current_tool = move || {
        let Some(tool) = active_tool.get() else { return; };
        let current_file = state.active_file_path();

        #[allow(unused_variables)]
        let Some(file_path) = current_file else {
            error_msg.set(Some("Vui lòng mở hoặc chọn một tài liệu PDF trước khi thực thi công cụ.".to_string()));
            return;
        };

        #[allow(unused_variables)]
        let opts = match tool {
            VnRibbonTool::VietnameseOcr => serde_json::json!({
                "engine": ocr_engine.get(),
                "output_format": ocr_format.get(),
            }),
            VnRibbonTool::TableExtractor => serde_json::json!({
                "page_range": if table_range.get().trim().is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::String(table_range.get().trim().to_string())
                },
                "output_csv": table_csv.get(),
            }),
            VnRibbonTool::ToLongImage => serde_json::json!({
                "output_format": image_format.get(),
                "dpi": image_dpi.get(),
                "add_divider": image_divider.get(),
            }),
            VnRibbonTool::BookletImposition => serde_json::json!({
                "mode": booklet_mode.get(),
                "paper_size": booklet_paper.get(),
            }),
            VnRibbonTool::GrayscalePrint => serde_json::json!({
                "convert_images": grayscale_images.get(),
            }),
            VnRibbonTool::RepairCorrupt => serde_json::json!({
                "scan_depth": repair_depth.get(),
            }),
        };

        is_executing.set(true);
        progress_pct.set(20.0);
        progress_msg.set(format!("Đang khởi chạy {} On-Device...", tool.title()));
        error_msg.set(None);
        result_msg.set(None);

        #[cfg(target_arch = "wasm32")]
        {
            let channel_val = crate::shared_ui::task_executor::create_tauri_channel(
                Some(progress_pct.write_only()),
                Some(progress_msg.write_only()),
            );
            let task_id_str = format!("vn-task-{}", js_sys::Date::now() as u64);
            let request_payload = serde_json::json!({
                "task_id": task_id_str,
                "tool_slug": tool.slug(),
                "input_paths": [file_path],
                "output_directory": null,
                "options": opts,
            });

            let args_obj = js_sys::Object::new();
            if let Ok(req_js) = serde_wasm_bindgen::to_value(&request_payload) {
                let _ = js_sys::Reflect::set(&args_obj, &wasm_bindgen::JsValue::from_str("request"), &req_js);
            }
            let _ = js_sys::Reflect::set(&args_obj, &wasm_bindgen::JsValue::from_str("onProgress"), &channel_val);
            let _ = js_sys::Reflect::set(&args_obj, &wasm_bindgen::JsValue::from_str("on_progress"), &channel_val);

            leptos::task::spawn_local(async move {
                let invoke_res = invoke("dispatch_tool_task", args_obj.into()).await;
                is_executing.set(false);

                match invoke_res {
                    Ok(resp) => {
                        if let Ok(res_obj) = serde_wasm_bindgen::from_value::<serde_json::Value>(resp) {
                            let status = res_obj.get("status").and_then(|v| v.as_str()).unwrap_or("SUCCESS");
                            let outputs = res_obj.get("output_paths").and_then(|v| v.as_array());
                            let first_out = outputs.and_then(|arr| arr.first()).and_then(|v| v.as_str());

                            if status == "SUCCESS" {
                                progress_pct.set(100.0);
                                progress_msg.set("Đã hoàn tất thành công!".to_string());

                                if let Some(out_path) = first_out {
                                    let out_str = out_path.to_string();
                                    if out_str.to_lowercase().ends_with(".pdf") {
                                        last_output_pdf.set(Some(out_str.clone()));
                                        state.add_file(&out_str);
                                        let _ = crate::micro_apps::pdf_studio::bridge::open_file_into_canvas(
                                            "pdfcraft-frame",
                                            &out_str,
                                        )
                                        .await;
                                        result_msg.set(Some(format!(
                                            "✓ Đã hoàn tất! Đã nạp tài liệu mới vào Canvas: {out_str}"
                                        )));
                                    } else {
                                        result_msg.set(Some(format!("✓ Đã hoàn tất! Tệp kết quả: {out_str}")));
                                    }
                                } else {
                                    result_msg.set(Some("✓ Tác vụ hoàn tất thành công.".to_string()));
                                }
                            } else {
                                let reason = res_obj
                                    .get("failed_items")
                                    .and_then(|arr| arr.as_array())
                                    .and_then(|a| a.first())
                                    .and_then(|item| item.get("reason"))
                                    .and_then(|r| r.as_str())
                                    .unwrap_or("Xử lý tác vụ thất bại");
                                error_msg.set(Some(format!("⚠️ Lỗi thực thi: {reason}")));
                            }
                        }
                    }
                    Err(err) => {
                        let err_text = err.as_string().unwrap_or_else(|| format!("{err:?}"));
                        error_msg.set(Some(format!("⚠️ Lỗi IPC Desktop: {err_text}")));
                    }
                }
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            is_executing.set(false);
            progress_pct.set(100.0);
            progress_msg.set("Đã hoàn tất!".to_string());
            result_msg.set(Some(format!("✓ [Mô phỏng Desktop] Đã thực thi xong: {}", tool.title())));
        }
    };

    view! {
        <section class="pdf-vn-extension-ribbon" aria-label="Thanh tiện ích độc bản Việt Nam">
            // Thanh Action Bar ngang nằm sát phía trên Canvas
            <div class="vn-ribbon-bar">
                <div class="vn-ribbon-brand">
                    <span class="vn-ribbon-brand-flag">"🇻🇳"</span>
                    <span class="vn-ribbon-brand-title">"Tiện Ích Việt Nam"</span>
                </div>

                <div class="vn-ribbon-tools-group">
                    {VnRibbonTool::ALL.into_iter().map(|tool| {
                        let is_active = move || active_tool.get() == Some(tool);
                        let slug = tool.slug();
                        let icon = tool.icon();
                        let title = tool.title();
                        view! {
                            <button
                                type="button"
                                class="vn-ribbon-btn"
                                class:active=is_active
                                data-tool=slug
                                title=tool.short_desc()
                                on:click=move |_| toggle_tool(tool)
                            >
                                <span class="vn-ribbon-icon">{icon}</span>
                                <span class="vn-ribbon-label">{title}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>

                <div class="vn-ribbon-file-status">
                    <span class="vn-ribbon-file-icon">"📄"</span>
                    <span class="vn-ribbon-file-name">
                        {move || state.active_file_name().unwrap_or_else(|| "Chưa chọn tệp PDF".to_string())}
                    </span>
                </div>
            </div>

            // Ngăn Drawer / Modal cấu hình nhanh khi người dùng click vào một công cụ
            {move || active_tool.get().map(|tool| {
                view! {
                    <div class="vn-ribbon-drawer">
                        <div class="vn-ribbon-drawer-header">
                            <div class="vn-ribbon-drawer-title-group">
                                <h3 class="vn-ribbon-drawer-title">
                                    <span class="vn-ribbon-drawer-icon">{tool.icon()}</span>
                                    <span>{tool.title()}</span>
                                </h3>
                                <p class="vn-ribbon-drawer-desc">{tool.short_desc()}</p>
                            </div>
                            <button
                                type="button"
                                class="vn-ribbon-close-btn"
                                title="Đóng ngăn cấu hình"
                                on:click=move |_| active_tool.set(None)
                            >
                                "✕"
                            </button>
                        </div>

                        // Tùy chọn cấu hình riêng biệt theo từng công cụ
                        <div class="vn-ribbon-drawer-body">
                            {match tool {
                                VnRibbonTool::VietnameseOcr => view! {
                                    <div class="vn-ribbon-form-group">
                                        <label class="vn-ribbon-form-label">"Động cơ OCR"</label>
                                        <select
                                            class="vn-ribbon-select"
                                            prop:value=move || ocr_engine.get()
                                            on:change=move |ev| ocr_engine.set(event_target_value(&ev))
                                        >
                                            <option value="fast_os_native">"Hệ điều hành OS (Nhanh)"</option>
                                            <option value="accurate_onnx">"ONNX Offline (Chính xác cao)"</option>
                                        </select>
                                    </div>
                                    <div class="vn-ribbon-form-group">
                                        <label class="vn-ribbon-form-label">"Định dạng xuất"</label>
                                        <select
                                            class="vn-ribbon-select"
                                            prop:value=move || ocr_format.get()
                                            on:change=move |ev| ocr_format.set(event_target_value(&ev))
                                        >
                                            <option value="docx">"Microsoft Word (.docx)"</option>
                                            <option value="txt">"Văn bản thuần (.txt)"</option>
                                            <option value="md">"Markdown (.md)"</option>
                                            <option value="searchable_pdf">"Searchable PDF (.pdf)"</option>
                                        </select>
                                    </div>
                                }.into_any(),

                                VnRibbonTool::TableExtractor => view! {
                                    <div class="vn-ribbon-form-group">
                                        <label class="vn-ribbon-form-label">"Trang trích xuất (để trống = tất cả)"</label>
                                        <input
                                            type="text"
                                            class="vn-ribbon-input"
                                            placeholder="Ví dụ: 1,2,3 hoặc để trống"
                                            prop:value=move || table_range.get()
                                            on:input=move |ev| table_range.set(event_target_value(&ev))
                                        />
                                    </div>
                                    <div class="vn-ribbon-form-group">
                                        <label class="vn-ribbon-form-label">"Định dạng xuất"</label>
                                        <select
                                            class="vn-ribbon-select"
                                            prop:value=move || if table_csv.get() { "csv" } else { "xlsx" }
                                            on:change=move |ev| table_csv.set(event_target_value(&ev) == "csv")
                                        >
                                            <option value="xlsx">"Excel Workbook (.xlsx)"</option>
                                            <option value="csv">"Comma-Separated (.csv)"</option>
                                        </select>
                                    </div>
                                }.into_any(),

                                VnRibbonTool::ToLongImage => view! {
                                    <div class="vn-ribbon-form-group">
                                        <label class="vn-ribbon-form-label">"Định dạng ảnh"</label>
                                        <select
                                            class="vn-ribbon-select"
                                            prop:value=move || image_format.get()
                                            on:change=move |ev| image_format.set(event_target_value(&ev))
                                        >
                                            <option value="png">"PNG (Không suy giảm chất lượng)"</option>
                                            <option value="jpg">"JPEG (Dung lượng nhỏ gọn)"</option>
                                            <option value="webp">"WebP (Tối ưu web)"</option>
                                        </select>
                                    </div>
                                    <div class="vn-ribbon-form-group">
                                        <label class="vn-ribbon-form-label">"Độ phân giải DPI"</label>
                                        <select
                                            class="vn-ribbon-select"
                                            prop:value=move || image_dpi.get().to_string()
                                            on:change=move |ev| image_dpi.set(event_target_value(&ev).parse().unwrap_or(150))
                                        >
                                            <option value="150">"150 DPI (Tiêu chuẩn xem màn hình)"</option>
                                            <option value="300">"300 DPI (In ấn độ nét cao)"</option>
                                        </select>
                                    </div>
                                    <div class="vn-ribbon-form-checkbox-group">
                                        <label class="vn-ribbon-checkbox-label">
                                            <input
                                                type="checkbox"
                                                prop:checked=move || image_divider.get()
                                                on:change=move |ev| image_divider.set(event_target_checked(&ev))
                                            />
                                            <span>"Kẻ vạch ngăn cách giữa các trang"</span>
                                        </label>
                                    </div>
                                }.into_any(),

                                VnRibbonTool::BookletImposition => view! {
                                    <div class="vn-ribbon-form-group">
                                        <label class="vn-ribbon-form-label">"Kiểu dàn trang in"</label>
                                        <select
                                            class="vn-ribbon-select"
                                            prop:value=move || booklet_mode.get()
                                            on:change=move |ev| booklet_mode.set(event_target_value(&ev))
                                        >
                                            <option value="saddle_stitch">"Đóng kim giữa (Saddle Stitch gập sách)"</option>
                                            <option value="two_up">"Dàn 2 trang song song (2-Up)"</option>
                                            <option value="four_up">"Dàn 4 trang trên 1 tờ (4-Up Lưới)"</option>
                                        </select>
                                    </div>
                                    <div class="vn-ribbon-form-group">
                                        <label class="vn-ribbon-form-label">"Khổ giấy in"</label>
                                        <select
                                            class="vn-ribbon-select"
                                            prop:value=move || booklet_paper.get()
                                            on:change=move |ev| booklet_paper.set(event_target_value(&ev))
                                        >
                                            <option value="A4">"Khổ A4"</option>
                                            <option value="A3">"Khổ A3 (Chuẩn in sách mini)"</option>
                                        </select>
                                    </div>
                                }.into_any(),

                                VnRibbonTool::GrayscalePrint => view! {
                                    <div class="vn-ribbon-form-checkbox-group">
                                        <label class="vn-ribbon-checkbox-label">
                                            <input
                                                type="checkbox"
                                                prop:checked=move || grayscale_images.get()
                                                on:change=move |ev| grayscale_images.set(event_target_checked(&ev))
                                            />
                                            <span>"Chuyển đổi hình ảnh nhúng sang đen trắng (Grayscale)"</span>
                                        </label>
                                    </div>
                                }.into_any(),

                                VnRibbonTool::RepairCorrupt => view! {
                                    <div class="vn-ribbon-form-group">
                                        <label class="vn-ribbon-form-label">"Chế độ quét phục hồi"</label>
                                        <input
                                            type="text"
                                            class="vn-ribbon-input"
                                            readonly=true
                                            prop:value=move || repair_depth.get()
                                        />
                                    </div>
                                }.into_any(),
                            }}
                        </div>

                        // Thanh tiến trình khi đang thực thi
                        {move || if is_executing.get() {
                            view! {
                                <div class="vn-ribbon-progress-bar">
                                    <div class="vn-ribbon-progress-info">
                                        <span class="vn-ribbon-progress-msg">{move || progress_msg.get()}</span>
                                        <span class="vn-ribbon-progress-pct">{move || format!("{:.0}%", progress_pct.get())}</span>
                                    </div>
                                    <div class="vn-ribbon-progress-track">
                                        <div
                                            class="vn-ribbon-progress-fill"
                                            style=move || format!("width: {}%", progress_pct.get())
                                        />
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <span style="display: none;"></span> }.into_any()
                        }}

                        // Thông báo kết quả hoặc lỗi
                        {move || {
                            if let Some(err) = error_msg.get() {
                                view! {
                                    <div class="vn-ribbon-error">
                                        <span>{err}</span>
                                    </div>
                                }.into_any()
                            } else if let Some(res) = result_msg.get() {
                                view! {
                                    <div class="vn-ribbon-result">
                                        <span>{res}</span>
                                        <div class="vn-ribbon-result-actions">
                                            <button
                                                type="button"
                                                class="btn-secondary btn-sm"
                                                on:click=move |_| open_output_folder_ipc()
                                            >
                                                "📂 Mở thư mục kết quả"
                                            </button>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span style="display: none;"></span> }.into_any()
                            }
                        }}

                        // Nút hành động
                        <div class="vn-ribbon-drawer-actions">
                            <button
                                type="button"
                                class="vn-ribbon-run-btn"
                                disabled=move || is_executing.get() || !state.has_files()
                                on:click=move |_| run_current_tool()
                            >
                                <span class="btn-icon">"⚡"</span>
                                <span>"Thực thi ngay On-Device"</span>
                            </button>

                            <button
                                type="button"
                                class="btn-secondary btn-sm"
                                on:click=move |_| active_tool.set(None)
                            >
                                "Đóng"
                            </button>
                        </div>
                    </div>
                }
            })}
        </section>
    }
}
