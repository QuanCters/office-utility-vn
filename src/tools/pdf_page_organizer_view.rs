use leptos::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct PageCardItem {
    pub id: usize,
    pub source_page: usize,
    pub rotation_deg: i32,
    pub is_landscape: bool,
}

#[component]
pub fn PdfPageOrganizerView() -> impl IntoView {
    let initial_items = vec![
        PageCardItem {
            id: 1,
            source_page: 1,
            rotation_deg: 0,
            is_landscape: false,
        },
        PageCardItem {
            id: 2,
            source_page: 2,
            rotation_deg: 0,
            is_landscape: false,
        },
        PageCardItem {
            id: 3,
            source_page: 3,
            rotation_deg: 0,
            is_landscape: true,
        },
        PageCardItem {
            id: 4,
            source_page: 4,
            rotation_deg: 0,
            is_landscape: false,
        },
    ];

    let (pages, set_pages) = signal(initial_items.clone());
    let (preview_mode, set_preview_mode) = signal(false);

    let on_reverse = move |_| {
        set_pages.update(|list| {
            list.reverse();
        });
    };

    let on_rotate_all = move |_| {
        set_pages.update(|list| {
            for p in list.iter_mut() {
                p.rotation_deg = (p.rotation_deg + 90) % 360;
            }
        });
    };

    let on_reset = {
        let initial = initial_items.clone();
        move |_| {
            set_pages.set(initial.clone());
        }
    };

    let on_remove_even = move |_| {
        set_pages.update(|list| {
            list.retain(|p| p.source_page % 2 != 0);
        });
    };

    let on_remove_odd = move |_| {
        set_pages.update(|list| {
            list.retain(|p| p.source_page % 2 == 0);
        });
    };

    view! {
        <div class="tool-config-panel">
            // Thanh thao tác hàng loạt
            <div class="batch-actions-bar" style="display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 16px; align-items: center;">
                <button
                    type="button"
                    class="action-btn"
                    on:click=on_reverse
                    style="padding: 6px 12px; font-size: 13px; font-weight: 600; background: var(--bg-surface); border: 1px solid var(--border-color); border-radius: 6px; cursor: pointer;"
                >
                    "⇄ Đảo ngược thứ tự"
                </button>
                <button
                    type="button"
                    class="action-btn"
                    on:click=on_rotate_all
                    style="padding: 6px 12px; font-size: 13px; font-weight: 600; background: var(--bg-surface); border: 1px solid var(--border-color); border-radius: 6px; cursor: pointer;"
                >
                    "↻ Xoay tất cả 90°"
                </button>
                <button
                    type="button"
                    class="action-btn"
                    on:click=on_reset
                    style="padding: 6px 12px; font-size: 13px; font-weight: 600; background: var(--bg-surface); border: 1px solid var(--border-color); border-radius: 6px; cursor: pointer;"
                >
                    "↺ Khôi phục gốc"
                </button>
                <button
                    type="button"
                    class="action-btn"
                    on:click=on_remove_even
                    style="padding: 6px 12px; font-size: 13px; font-weight: 600; background: var(--bg-surface); border: 1px solid var(--border-color); border-radius: 6px; cursor: pointer;"
                >
                    "Xóa trang chẵn"
                </button>
                <button
                    type="button"
                    class="action-btn"
                    on:click=on_remove_odd
                    style="padding: 6px 12px; font-size: 13px; font-weight: 600; background: var(--bg-surface); border: 1px solid var(--border-color); border-radius: 6px; cursor: pointer;"
                >
                    "Xóa trang lẻ"
                </button>
                <div style="margin-left: auto; display: flex; align-items: center; gap: 8px;">
                    <label class="radio-item" style="cursor: pointer; font-size: 13px; font-weight: 600;">
                        <input
                            type="checkbox"
                            checked=preview_mode
                            on:change=move |ev| set_preview_mode.set(event_target_checked(&ev))
                        />
                        "👁️ Chế độ Preview"
                    </label>
                </div>
            </div>

            // Lưới hiển thị danh sách thẻ trang Smart Hybrid
            <div
                class="organizer-grid"
                style="display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 14px; margin-top: 10px;"
            >
                {move || {
                    let cur_pages = pages.get();
                    let total = cur_pages.len();
                    let is_preview = preview_mode.get();

                    if total == 0 {
                        return view! {
                            <div style="grid-column: 1 / -1; padding: 24px; text-align: center; color: var(--text-muted); background: var(--bg-surface); border: 1px dashed var(--border-color); border-radius: 8px;">
                                "Không còn trang nào được chọn. Nhấn 'Khôi phục gốc' để lấy lại toàn bộ trang."
                            </div>
                        }.into_any();
                    }

                    cur_pages.into_iter().enumerate().map(|(idx, item)| {
                        let eff_landscape = item.is_landscape ^ ((item.rotation_deg / 90) % 2 != 0);
                        let orient_icon = if eff_landscape { "📑" } else { "📄" };
                        let orient_label = if eff_landscape { "Ngang" } else { "Dọc" };

                        let on_rot_left = move |_: web_sys::MouseEvent| {
                            set_pages.update(|list| {
                                if let Some(p) = list.get_mut(idx) {
                                    p.rotation_deg = (p.rotation_deg - 90 + 360) % 360;
                                }
                            });
                        };

                        let on_rot_right = move |_: web_sys::MouseEvent| {
                            set_pages.update(|list| {
                                if let Some(p) = list.get_mut(idx) {
                                    p.rotation_deg = (p.rotation_deg + 90) % 360;
                                }
                            });
                        };

                        let on_move_left = move |_: web_sys::MouseEvent| {
                            if idx > 0 {
                                set_pages.update(|list| {
                                    list.swap(idx, idx - 1);
                                });
                            }
                        };

                        let on_move_right = move |_: web_sys::MouseEvent| {
                            if idx + 1 < total {
                                set_pages.update(|list| {
                                    list.swap(idx, idx + 1);
                                });
                            }
                        };

                        let on_delete = move |_: web_sys::MouseEvent| {
                            set_pages.update(|list| {
                                if idx < list.len() {
                                    list.remove(idx);
                                }
                            });
                        };

                        let is_first = idx == 0;
                        let is_last = idx + 1 >= total;

                        view! {
                            <div
                                class="page-organizer-card"
                                style="background: var(--bg-surface); border: 1px solid var(--border-color); border-radius: 8px; padding: 12px; display: flex; flex-direction: column; gap: 8px; box-shadow: var(--shadow-sm); transition: transform 0.15s ease;"
                            >
                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                    <span style="font-weight: 700; font-size: 13px;">
                                        {format!("Trang {}", item.source_page)}
                                    </span>
                                    <span
                                        class="badge-rotation"
                                        style="font-size: 12px; padding: 2px 6px; border-radius: 4px; background: var(--primary-bg, rgba(59, 130, 246, 0.15)); color: var(--primary); font-weight: 600;"
                                    >
                                        {format!("{}°", item.rotation_deg)}
                                    </span>
                                </div>

                                <div
                                    class="page-card-thumbnail"
                                    style="height: 110px; background: var(--bg-card, rgba(0,0,0,0.03)); border: 1px dashed var(--border-subtle, rgba(0,0,0,0.1)); border-radius: 6px; display: flex; flex-direction: column; align-items: center; justify-content: center; position: relative; overflow: hidden;"
                                >
                                    <div style="font-size: 32px; line-height: 1;">
                                        {orient_icon}
                                    </div>
                                    <div style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">
                                        {format!("Khổ {orient_label} (Vị trí: {})", idx + 1)}
                                    </div>

                                    {if is_preview {
                                        view! {
                                            <div
                                                class="preview-overlay"
                                                style="position: absolute; inset: 0; background: rgba(0,0,0,0.04); display: flex; align-items: center; justify-content: center; font-size: 12px; font-weight: 600; color: var(--text-muted);"
                                            >
                                                "👁️ Xem trước 60 FPS"
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div style="display: none;"></div> }.into_any()
                                    }}
                                </div>

                                // Thanh điều khiển trực tiếp trên thẻ
                                <div style="display: flex; gap: 4px; justify-content: space-between; margin-top: 4px;">
                                    <button
                                        type="button"
                                        title="Xoay trái 90°"
                                        on:click=on_rot_left
                                        style="flex: 1; padding: 4px 0; border: 1px solid var(--border-color); border-radius: 4px; background: var(--bg-card); cursor: pointer; font-size: 13px;"
                                    >
                                        "↺"
                                    </button>
                                    <button
                                        type="button"
                                        title="Xoay phải 90°"
                                        on:click=on_rot_right
                                        style="flex: 1; padding: 4px 0; border: 1px solid var(--border-color); border-radius: 4px; background: var(--bg-card); cursor: pointer; font-size: 13px;"
                                    >
                                        "↻"
                                    </button>
                                    <button
                                        type="button"
                                        title="Di chuyển sang trước"
                                        disabled=is_first
                                        on:click=on_move_left
                                        style="flex: 1; padding: 4px 0; border: 1px solid var(--border-color); border-radius: 4px; background: var(--bg-card); cursor: pointer; font-size: 13px;"
                                    >
                                        "←"
                                    </button>
                                    <button
                                        type="button"
                                        title="Di chuyển sang sau"
                                        disabled=is_last
                                        on:click=on_move_right
                                        style="flex: 1; padding: 4px 0; border: 1px solid var(--border-color); border-radius: 4px; background: var(--bg-card); cursor: pointer; font-size: 13px;"
                                    >
                                        "→"
                                    </button>
                                    <button
                                        type="button"
                                        title="Xóa trang này"
                                        on:click=on_delete
                                        style="flex: 1; padding: 4px 0; border: 1px solid rgba(239, 68, 68, 0.4); border-radius: 4px; background: var(--danger-bg); color: var(--danger-light); cursor: pointer; font-size: 13px;"
                                    >
                                        "🗑️"
                                    </button>
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>().into_any()
                }}
            </div>

            <div style="color: var(--text-muted); font-size: 12px; margin-top: 14px; line-height: 1.5;">
                "💡 Chế độ Smart Hybrid: Các thẻ trang phản hồi lập tức 60 FPS, không gây gián đoạn thao tác khi xử lý tài liệu PDF nhiều trang dung lượng lớn."
            </div>
        </div>
    }
}
