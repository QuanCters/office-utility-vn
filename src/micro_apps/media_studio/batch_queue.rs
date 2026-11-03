use leptos::prelude::*;
use crate::micro_apps::media_studio::state::MediaStudioState;

/// Biểu tượng trực quan dựa trên đuôi định dạng file của tác vụ hàng loạt.
fn media_item_icon(file_name: &str) -> &'static str {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".mp4") || lower.ends_with(".mov") || lower.ends_with(".avi") || lower.ends_with(".mkv") {
        "🎞️"
    } else if lower.ends_with(".mp3") || lower.ends_with(".wav") || lower.ends_with(".m4a") || lower.ends_with(".flac") || lower.ends_with(".aac") {
        "🎙️"
    } else {
        "🖼️"
    }
}

/// Component hiển thị Hàng đợi Xử lý Đa phương tiện Hàng loạt (Batch Processing Queue).
///
/// Hỗ trợ xem danh sách các tác vụ hàng loạt, thanh tiến trình từng tệp, badge trạng thái
/// (Pending, Processing, Done, Failed) và các nút thao tác (Hủy tác vụ, Xóa mục đã hoàn thành, Xóa toàn bộ).
#[component]
pub fn MediaStudioBatchQueue(
    state: MediaStudioState,
) -> impl IntoView {
    let is_expanded = Signal::derive(move || state.is_queue_expanded.get());
    let queue_items = Signal::derive(move || state.batch_queue.get());
    let total_count = Signal::derive(move || state.batch_items_count());

    let active_count = Signal::derive(move || {
        state.batch_queue.with(|q| {
            q.iter()
                .filter(|it| it.status == "Pending" || it.status == "Processing")
                .count()
        })
    });

    let completed_count = Signal::derive(move || {
        state.batch_queue.with(|q| {
            q.iter()
                .filter(|it| it.status == "Done" || it.status == "Hoàn thành")
                .count()
        })
    });

    view! {
        <section class="media-batch-queue-container" aria-label="Hàng đợi xử lý hàng loạt Media Studio">
            // Header bar của hàng đợi
            <div class="media-batch-queue-header">
                <div class="media-batch-header-left">
                    <button
                        type="button"
                        class="media-batch-toggle-btn"
                        title=move || if is_expanded.get() { "Thu gọn hàng đợi" } else { "Mở rộng hàng đợi" }
                        on:click=move |_| state.toggle_queue_expanded()
                    >
                        <span class="media-batch-toggle-icon" aria-hidden="true">
                            {move || if is_expanded.get() { "▼" } else { "▲" }}
                        </span>
                        <span class="media-batch-title">"⚡ Hàng đợi Xử lý Hàng loạt (Batch Queue)"</span>
                    </button>

                    <div class="media-batch-badges-group">
                        <span class="media-batch-count-badge">
                            {move || format!("{} tác vụ", total_count.get())}
                        </span>

                        {move || {
                            let act = active_count.get();
                            if act > 0 {
                                view! {
                                    <span class="media-batch-active-badge">
                                        {format!("⚙️ {} đang chạy", act)}
                                    </span>
                                }.into_any()
                            } else {
                                view! { <span style="display: none;"></span> }.into_any()
                            }
                        }}

                        {move || {
                            let comp = completed_count.get();
                            if comp > 0 {
                                view! {
                                    <span class="media-batch-done-badge">
                                        {format!("✓ {} xong", comp)}
                                    </span>
                                }.into_any()
                            } else {
                                view! { <span style="display: none;"></span> }.into_any()
                            }
                        }}
                    </div>
                </div>

                <div class="media-batch-header-right">
                    {move || {
                        if completed_count.get() > 0 {
                            view! {
                                <button
                                    type="button"
                                    class="media-batch-action-btn btn-clear-completed"
                                    title="Xóa các tác vụ đã hoàn thành khỏi danh sách"
                                    on:click=move |_| state.clear_completed_batch()
                                >
                                    "🧹 Dọn tác vụ xong"
                                </button>
                            }.into_any()
                        } else {
                            view! { <span style="display: none;"></span> }.into_any()
                        }
                    }}

                    {move || {
                        if total_count.get() > 0 {
                            view! {
                                <button
                                    type="button"
                                    class="media-batch-action-btn btn-clear-all"
                                    title="Xóa toàn bộ hàng đợi xử lý"
                                    on:click=move |_| state.clear_batch_queue()
                                >
                                    "✕ Xóa danh sách"
                                </button>
                            }.into_any()
                        } else {
                            view! { <span style="display: none;"></span> }.into_any()
                        }
                    }}
                </div>
            </div>

            // Thân danh sách hàng đợi (chỉ hiển thị khi expanded)
            {move || {
                if !is_expanded.get() {
                    return view! { <div style="display: none;"></div> }.into_any();
                }

                let items = queue_items.get();
                if items.is_empty() {
                    view! {
                        <div class="media-batch-empty-state">
                            <span class="media-batch-empty-icon" aria-hidden="true">"🗂️"</span>
                            <div class="media-batch-empty-text-group">
                                <span class="media-batch-empty-title">
                                    "Hàng đợi xử lý hàng loạt đang trống"
                                </span>
                                <span class="media-batch-empty-subtitle">
                                    "Nạp tệp media từ bảng bên trái hoặc bấm 'Đưa vào Batch Queue' để tự động xử lý hàng loạt trên thiết bị."
                                </span>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="media-batch-items-list" role="list">
                            {items.into_iter().enumerate().map(|(idx, item)| {
                                let item_id = item.id.clone();
                                let item_id_cancel = item.id.clone();
                                let item_id_remove = item.id.clone();
                                let icon = media_item_icon(&item.file_name);
                                let percent_val = if item.progress <= 1.0 {
                                    item.progress * 100.0
                                } else {
                                    item.progress
                                }.clamp(0.0, 100.0);

                                let width_style = format!("width: {:.1}%", percent_val);
                                let percent_label = format!("{:.0}%", percent_val);
                                let status_str = item.status.clone();
                                let is_active_task = status_str == "Pending" || status_str == "Processing";

                                view! {
                                    <div class="media-batch-item" role="listitem">
                                        <div class="media-batch-item-index">{format!("#{}", idx + 1)}</div>

                                        <div class="media-batch-item-icon" aria-hidden="true">
                                            {icon}
                                        </div>

                                        <div class="media-batch-item-info">
                                            <div class="media-batch-item-title-row">
                                                <span class="media-batch-item-filename" title=item.file_name.clone()>
                                                    {item.file_name.clone()}
                                                </span>
                                                <span class="media-batch-item-id">
                                                    {format!("({})", item_id)}
                                                </span>
                                            </div>

                                            <div class="media-batch-progress-wrapper">
                                                <div
                                                    class="media-batch-progress-track"
                                                    role="progressbar"
                                                    aria-valuenow=percent_val as u32
                                                    aria-valuemin="0"
                                                    aria-valuemax="100"
                                                >
                                                    <div
                                                        class=format!(
                                                            "media-batch-progress-bar {}",
                                                            if status_str == "Done" || status_str == "Hoàn thành" {
                                                                "progress-bar-done"
                                                            } else if status_str == "Failed" || status_str == "Thất bại" {
                                                                "progress-bar-failed"
                                                            } else {
                                                                "progress-bar-active"
                                                            }
                                                        )
                                                        style=width_style
                                                    ></div>
                                                </div>
                                                <span class="media-batch-percent-text">{percent_label}</span>
                                            </div>
                                        </div>

                                        <div class="media-batch-item-status-col">
                                            {match status_str.as_str() {
                                                "Pending" | "Chờ xử lý" => view! {
                                                    <span class="batch-badge batch-badge-pending">"⏳ Chờ xử lý"</span>
                                                }.into_any(),
                                                "Processing" | "Đang xử lý" => view! {
                                                    <span class="batch-badge batch-badge-processing">"⚙️ Đang xử lý"</span>
                                                }.into_any(),
                                                "Done" | "Hoàn thành" => view! {
                                                    <span class="batch-badge batch-badge-done">"✓ Hoàn thành"</span>
                                                }.into_any(),
                                                "Failed" | "Thất bại" => view! {
                                                    <span class="batch-badge batch-badge-failed">"✕ Thất bại"</span>
                                                }.into_any(),
                                                _ => view! {
                                                    <span class="batch-badge batch-badge-other">{status_str}</span>
                                                }.into_any(),
                                            }}
                                        </div>

                                        <div class="media-batch-item-actions-col">
                                            {if is_active_task {
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="media-batch-item-btn btn-cancel-item"
                                                        title="Hủy tác vụ này"
                                                        on:click=move |_| state.update_batch_progress(&item_id_cancel, "Failed", 0.0)
                                                    >
                                                        "✕ Hủy"
                                                    </button>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="media-batch-item-btn btn-remove-item"
                                                        title="Xóa mục này khỏi danh sách"
                                                        on:click=move |_| state.remove_batch_item(&item_id_remove)
                                                    >
                                                        "✕ Xóa"
                                                    </button>
                                                }.into_any()
                                            }}
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            }}
        </section>
    }
}
