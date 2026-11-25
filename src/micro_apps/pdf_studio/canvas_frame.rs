use crate::micro_apps::pdf_studio::state::PdfStudioState;
use leptos::prelude::*;

/// Kết quả kiểm tra có cần gửi lại tài liệu PDF vào iframe canvas hiện tại hay không (P2 #4684793560).
pub fn should_deliver_pdf_to_canvas(
    state: &PdfStudioState,
    instance_id: u64,
    in_flight: Option<(u64, &str)>,
) -> Option<String> {
    if !state.is_canvas_ready.get() {
        return None;
    }
    if state.active_canvas_instance_id.get() != Some(instance_id) {
        return None;
    }
    let path = state.active_file_path()?;
    if state.is_path_bound_to_active_canvas(&path) {
        return None;
    }
    if in_flight == Some((instance_id, path.as_str())) {
        return None;
    }
    Some(path)
}

/// Hoàn tất kết quả giao hàng tài liệu vào iframe canvas (P2 #4684793560):
/// Chỉ đánh dấu bound khi giao hàng thành công tới đúng thực thể iframe còn sẵn sàng và vẫn đang chọn `path`;
/// nếu thất bại, giữ nguyên trạng thái unbound để người dùng hoặc lượt chọn tiếp theo có thể khôi phục.
pub fn finalize_canvas_delivery_result(
    state: &PdfStudioState,
    instance_id: u64,
    path: &str,
    result: Result<(), String>,
) -> bool {
    match result {
        Ok(()) => {
            if state.active_canvas_instance_id.get_untracked() == Some(instance_id)
                && state.is_canvas_ready.get_untracked()
                && state.active_file_path().as_deref() == Some(path)
            {
                state.mark_canvas_bound(instance_id, path)
            } else {
                false
            }
        }
        Err(err) => {
            if state.active_canvas_instance_id.get_untracked() == Some(instance_id) {
                state.invalidate_canvas_binding();
            }
            state
                .status_message
                .set(format!("❌ Không thể nạp tệp vào Canvas: {err}"));
            false
        }
    }
}

/// Component `PdfCraftCanvasFrame` nhúng giao diện đồ họa trực quan của PDFCraft (WebAssembly Canvas)
/// thông qua thẻ `iframe`.
///
/// Hỗ trợ:
/// - `allow="clipboard-read; clipboard-write"` để thao tác sao chép / dán nội dung trong canvas.
/// - Chiều rộng & chiều cao 100% chiếm trọn không gian container (`w-full h-full border-none`).
/// - Hiển thị trạng thái loading skeleton trong lúc WebAssembly và WebGL Canvas đang khởi tạo.
/// - Tự động ẩn skeleton khi nhận thông điệp `pdfcraft:ready` từ bridge.
/// - Tự động nạp tệp PDF đang kích hoạt vào Canvas khi sẵn sàng.
#[component]
pub fn PdfCraftCanvasFrame(#[prop(into, optional)] state: Option<PdfStudioState>) -> impl IntoView {
    let studio_state = state.or_else(use_context::<PdfStudioState>);

    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = studio_state {
            // P2 #4684793560: Gắn vòng đời binding theo từng thực thể iframe mới được mount
            let instance_id = s.begin_canvas_instance();
            let listener_cleanup = StoredValue::new_local(
                crate::micro_apps::pdf_studio::bridge::setup_bridge_listener(
                    "pdfcraft-frame".to_string(),
                    s,
                    None,
                )
                .ok()
                .flatten(),
            );
            on_cleanup(move || {
                s.end_canvas_instance(instance_id);
                if let Some(handle) = listener_cleanup.try_update_value(|v| v.take()).flatten() {
                    handle.run();
                }
            });

            // Theo dõi tệp đang chọn và tự động nạp vào canvas khi iframe đã sẵn sàng
            // P1 #4680342554 & P2 #4684793560: Ràng buộc danh tính theo thực thể iframe `instance_id`.
            // Chỉ đánh dấu bound SAU KHI `open_file_into_canvas` giao hàng thành công;
            // nếu thất bại sẽ giữ unbound để có thể thử lại/khôi phục.
            let in_flight: RwSignal<Option<(u64, String)>> = RwSignal::new(None);
            Effect::new(move |_| {
                let flight = in_flight.get_untracked();
                let flight_ref = flight.as_ref().map(|(id, p)| (*id, p.as_str()));
                if let Some(path) = should_deliver_pdf_to_canvas(&s, instance_id, flight_ref) {
                    in_flight.set(Some((instance_id, path.clone())));
                    wasm_bindgen_futures::spawn_local(async move {
                        let res = crate::micro_apps::pdf_studio::bridge::open_file_into_canvas(
                            "pdfcraft-frame",
                            &path,
                        )
                        .await;
                        in_flight.update(|cur| {
                            if cur.as_ref() == Some(&(instance_id, path.clone())) {
                                *cur = None;
                            }
                        });
                        finalize_canvas_delivery_result(&s, instance_id, &path, res);
                    });
                }
            });
        }
    }

    view! {
        <div class="pdfcraft-frame-container relative w-full h-full flex-1 overflow-hidden" style="width: 100%; height: 100%; min-height: 0; display: flex; flex-direction: column; flex: 1;">
            // Trạng thái loading skeleton hiển thị trong lúc WASM canvas đang khởi động
            <div
                id="pdfcraft-skeleton"
                class="pdfcraft-loading-skeleton"
                class:hidden=move || studio_state.map(|s| s.is_canvas_ready.get()).unwrap_or(false)
                aria-live="polite"
            >
                <div class="pdfcraft-skeleton-content">
                    <div class="pdfcraft-skeleton-spinner" aria-hidden="true"></div>
                    <div class="pdfcraft-skeleton-text">
                        <span class="pdfcraft-skeleton-title">
                            "Đang tải PDF Studio..."
                        </span>
                        <span class="pdfcraft-skeleton-subtitle">
                            "Khởi tạo WebAssembly Canvas 100% On-Device"
                        </span>
                    </div>
                    <div class="pdfcraft-skeleton-fallback-box" style="margin-top: 16px; display: flex; flex-direction: column; align-items: center; gap: 8px;">
                        <span style="font-size: 12px; color: var(--text-muted, #94A3B8);">
                            "Nếu Canvas tải chậm hoặc không tương thích trên thiết bị này:"
                        </span>
                        <button
                            type="button"
                            class="pdfcraft-fallback-btn"
                            style="background: #3B82F6; color: #FFFFFF; border: none; border-radius: 6px; padding: 8px 16px; font-size: 13px; font-weight: 500; cursor: pointer; transition: background 0.15s ease;"
                            on:click=move |_| {
                                if let Some(s) = studio_state {
                                    s.open_tool("pdf-merge-split");
                                }
                            }
                        >
                            "⚡ Chuyển sang Giao diện Công cụ (Classic Tools)"
                        </button>
                    </div>
                </div>
            </div>

            // Khung nhúng iframe PDF Studio Canvas (Đồng bộ Theme Dark & Ngôn ngữ tiếng Việt)
            <iframe
                id="pdfcraft-frame"
                src="/pdfcraft/index.html?host=parent&theme=dark&lang=vi"
                class="w-full h-full border-none"
                style="width: 100%; height: 100%; border: none; display: block; flex: 1;"
                allow="clipboard-read; clipboard-write"
                title="PDF Studio Canvas"
            ></iframe>
        </div>
    }
}
