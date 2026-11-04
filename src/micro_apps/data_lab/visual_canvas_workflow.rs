use leptos::prelude::*;
use crate::micro_apps::data_lab::state::{category_for_tool, DataLabCategory, DataLabState};
use crate::micro_apps::data_lab::toolbar::data_lab_tool_icon;

/// Phân loại nhóm màu sắc hiển thị cho Node trên Canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasNodeColor {
    /// Xanh lá (Clean, Normalizer, Unlocker)
    Green,
    /// Tím (Fuzzy, Structure, Mailmerge, Base64)
    Purple,
    /// Cam (Split, Merge, Join, Diff, Set)
    Orange,
    /// Xanh dương (Input File, Output Destination)
    Blue,
}

impl CanvasNodeColor {
    pub fn class_name(self) -> &'static str {
        match self {
            CanvasNodeColor::Green => "node-theme-green",
            CanvasNodeColor::Purple => "node-theme-purple",
            CanvasNodeColor::Orange => "node-theme-orange",
            CanvasNodeColor::Blue => "node-theme-blue",
        }
    }

    pub fn stroke_color(self) -> &'static str {
        match self {
            CanvasNodeColor::Green => "var(--success)",
            CanvasNodeColor::Purple => "var(--primary-light)",
            CanvasNodeColor::Orange => "var(--warning)",
            CanvasNodeColor::Blue => "var(--info-light)",
        }
    }
}

/// Trạng thái thực thi của từng Node trên bảng vẽ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanvasNodeStatus {
    /// Sẵn sàng thực thi
    Ready,
    /// Đang trong tiến trình xử lý
    Running,
    /// Đã hoàn tất xử lý
    Completed,
    /// Gặp lỗi thực thi
    Error(String),
}

impl CanvasNodeStatus {
    pub fn label(&self) -> &'static str {
        match self {
            CanvasNodeStatus::Ready => "Sẵn sàng",
            CanvasNodeStatus::Running => "Đang chạy...",
            CanvasNodeStatus::Completed => "Hoàn thành",
            CanvasNodeStatus::Error(_) => "Lỗi",
        }
    }

    pub fn badge_class(&self) -> &'static str {
        match self {
            CanvasNodeStatus::Ready => "status-badge-ready",
            CanvasNodeStatus::Running => "status-badge-running",
            CanvasNodeStatus::Completed => "status-badge-completed",
            CanvasNodeStatus::Error(_) => "status-badge-error",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            CanvasNodeStatus::Ready => "⏱",
            CanvasNodeStatus::Running => "⚡",
            CanvasNodeStatus::Completed => "✓",
            CanvasNodeStatus::Error(_) => "✕",
        }
    }
}

/// Phân loại vai trò của Node trong đồ thị DAG luồng dữ liệu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanvasNodeKind {
    /// Nguồn dữ liệu đầu vào (Input Source)
    InputSource,
    /// Bước biến đổi dữ liệu (Transform Step)
    Transform,
    /// Thư mục xuất kết quả (Output Destination)
    OutputDestination,
}

/// Một Node hoàn chỉnh trên Workflow Canvas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasNode {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub kind: CanvasNodeKind,
    pub color: CanvasNodeColor,
    pub status: CanvasNodeStatus,
    pub summary: String,
    pub tool_slug: Option<String>,
    pub category: Option<DataLabCategory>,
    pub row_info: String,
    pub config_details: String,
}

/// Xác định màu chủ đề dựa trên nhóm danh mục hoặc slug của công cụ.
pub fn color_for_slug(slug: &str) -> CanvasNodeColor {
    match slug {
        "excel-cleaner-normalizer" | "excel-sheet-unlocker" => CanvasNodeColor::Green,
        "excel-split-by-column" | "excel-multi-merger" | "excel-visual-joiner" => {
            CanvasNodeColor::Orange
        }
        "diff-checker-text-sheets" | "list-set-operations" => CanvasNodeColor::Orange,
        "data-structure-converter"
        | "docx-template-mailmerge"
        | "base64-file-codec"
        | "universal-file-converter-hub"
        | "universal-converter" => CanvasNodeColor::Purple,
        "text-pii-anonymizer" | "sql-formatter-linter" => CanvasNodeColor::Purple,
        _ => CanvasNodeColor::Blue,
    }
}

/// Tạo danh sách các Node mẫu chuẩn mực khởi đầu theo đúng đặc tả KNIME / Alteryx DAG:
/// 1. Input Source Node: `danh_sach_nhan_vien.xlsx` • 1,200 dòng (icon 📥).
/// 2. Transform Node 1: `Làm sạch bảng tính & Tách Họ Tên` (icon 🧹, Hoàn tất).
/// 3. Transform Node 2: `Gom cụm Fuzzy Cột Tỉnh Thành` (icon ✨, Hoàn tất).
/// 4. Transform Node 3: `Tách File theo Cột Phòng Ban` (icon 🗂️, Đang chạy...).
/// 5. Output Destination Node: Thư mục xuất kết quả (icon 📁, Chờ).
pub fn create_default_canvas_nodes(active_file: Option<String>) -> Vec<CanvasNode> {
    let input_file_name = active_file.unwrap_or_else(|| "danh_sach_nhan_vien.xlsx".to_string());

    vec![
        // Node 1: Input Source Node
        CanvasNode {
            id: "node-input".to_string(),
            name: "Tệp Dữ liệu Đầu vào".to_string(),
            icon: "📥".to_string(),
            kind: CanvasNodeKind::InputSource,
            color: CanvasNodeColor::Blue,
            status: CanvasNodeStatus::Completed,
            summary: format!("{} • 1,200 dòng dữ liệu", input_file_name),
            tool_slug: None,
            category: Some(DataLabCategory::Spreadsheet),
            row_info: "1,200 dòng • 18 cột".to_string(),
            config_details: "Trang tính: Sheet1 • Bảng mã: Unicode UTF-8".to_string(),
        },
        // Node 2: Transform Node 1
        CanvasNode {
            id: "node-transform-1".to_string(),
            name: "Làm sạch bảng tính & Tách Họ Tên".to_string(),
            icon: "🧹".to_string(),
            kind: CanvasNodeKind::Transform,
            color: CanvasNodeColor::Green,
            status: CanvasNodeStatus::Completed,
            summary: "Cột: Họ và tên • Tách họ kép • Trim khoảng trắng".to_string(),
            tool_slug: Some("excel-cleaner-normalizer".to_string()),
            category: Some(DataLabCategory::Spreadsheet),
            row_info: "1,200/1,200 dòng hợp lệ (0.18s)".to_string(),
            config_details: "Tách: Họ đệm + Tên riêng • Viết hoa chữ cái đầu • Xóa dòng trùng lặp".to_string(),
        },
        // Node 3: Transform Node 2
        CanvasNode {
            id: "node-transform-2".to_string(),
            name: "Gom cụm Fuzzy Cột Tỉnh Thành".to_string(),
            icon: "✨".to_string(),
            kind: CanvasNodeKind::Transform,
            color: CanvasNodeColor::Purple,
            status: CanvasNodeStatus::Completed,
            summary: "Cột: Tỉnh / Thành phố • Ngưỡng khớp: 85%".to_string(),
            tool_slug: Some("text-pii-anonymizer".to_string()),
            category: Some(DataLabCategory::TextStructure),
            row_info: "Gom 63 cụm chuẩn hóa (0.32s)".to_string(),
            config_details: "Thuật toán: Jaro-Winkler • Tự động chuẩn hóa dấu tiếng Việt NFC".to_string(),
        },
        // Node 4: Transform Node 3
        CanvasNode {
            id: "node-transform-3".to_string(),
            name: "Tách File theo Cột Phòng Ban".to_string(),
            icon: "🗂️".to_string(),
            kind: CanvasNodeKind::Transform,
            color: CanvasNodeColor::Orange,
            status: CanvasNodeStatus::Running,
            summary: "Cột: Phòng ban • 1 file riêng/phòng".to_string(),
            tool_slug: Some("excel-split-by-column".to_string()),
            category: Some(DataLabCategory::Spreadsheet),
            row_info: "Đang phân tách: 8/14 phòng ban...".to_string(),
            config_details: "Định dạng: Excel .xlsx • Thêm tiêu đề chuẩn • Đặt tên tự động".to_string(),
        },
        // Node 5: Output Destination Node
        CanvasNode {
            id: "node-output".to_string(),
            name: "Thư mục Xuất Kết quả".to_string(),
            icon: "📁".to_string(),
            kind: CanvasNodeKind::OutputDestination,
            color: CanvasNodeColor::Blue,
            status: CanvasNodeStatus::Ready,
            summary: "/exports/data_lab_output/ • 14 files .xlsx".to_string(),
            tool_slug: None,
            category: None,
            row_info: "Chờ tiến trình tách file hoàn tất".to_string(),
            config_details: "Thư mục đích: ./exports/output_phong_ban/ • Nén ZIP tự động: Tắt".to_string(),
        },
    ]
}

/// Component chính `VisualCanvasWorkflowView`: Bảng vẽ luồng dữ liệu trực quan (KNIME / Alteryx / Easy Data Transform).
#[component]
pub fn VisualCanvasWorkflowView(
    state: DataLabState,
) -> impl IntoView {
    // Khởi tạo danh sách các node ban đầu
    let initial_nodes = create_default_canvas_nodes(state.active_file_name());
    let nodes = RwSignal::new(initial_nodes);

    // Trạng thái mở Drawer danh mục công cụ (Node Palette)
    let is_palette_open = RwSignal::new(false);

    // Trạng thái mở Modal cấu hình node
    let active_config_node = RwSignal::new(None::<CanvasNode>);

    // Tiến trình thực thi toàn bộ luồng
    let is_pipeline_running = RwSignal::new(true); // ban đầu node 3 đang chạy như mô tả
    let overall_message = RwSignal::new("⚡ Đang thực thi: Bước 3/5 (Tách File theo Cột Phòng Ban)...".to_string());

    // Đồng bộ tên file đầu vào khi state.active_file_name() thay đổi
    Effect::new(move |_| {
        if let Some(file_name) = state.active_file_name() {
            nodes.update(|n| {
                if let Some(first_node) = n.first_mut() {
                    if first_node.kind == CanvasNodeKind::InputSource {
                        first_node.summary = format!("{} • 1,200 dòng dữ liệu", file_name);
                    }
                }
            });
        }
    });

    // Thao tác: Chạy toàn bộ luồng
    let run_pipeline = move || {
        is_pipeline_running.set(true);
        state.is_running.set(true);
        overall_message.set("⚡ Đang khởi động chuỗi pipeline xử lý dữ liệu...".to_string());

        // Đặt trạng thái các node: Node 1 & 2 Completed, Node 3 Running, Node 4 Ready
        nodes.update(|n| {
            for (idx, node) in n.iter_mut().enumerate() {
                if idx < 3 {
                    node.status = CanvasNodeStatus::Completed;
                } else if idx == 3 {
                    node.status = CanvasNodeStatus::Running;
                } else {
                    node.status = CanvasNodeStatus::Ready;
                }
            }
        });

        // Giả lập hoàn tất bước 3 sau khi chạy
        overall_message.set("⚡ Đang thực thi: Bước 3/5 (Tách File theo Cột Phòng Ban)...".to_string());
    };

    // Thao tác: Dừng luồng
    let stop_pipeline = move || {
        is_pipeline_running.set(false);
        state.is_running.set(false);
        overall_message.set("⏹ Đã tạm dừng thực thi chuỗi quy trình dữ liệu.".to_string());
        nodes.update(|n| {
            for node in n.iter_mut() {
                if node.status == CanvasNodeStatus::Running {
                    node.status = CanvasNodeStatus::Ready;
                }
            }
        });
    };

    // Thao tác: Hoàn tất toàn bộ luồng (Testing helper)
    let complete_pipeline = move || {
        is_pipeline_running.set(false);
        state.is_running.set(false);
        overall_message.set("✓ Toàn bộ chuỗi quy trình đã hoàn thành xuất sắc (0.85s)!".to_string());
        nodes.update(|n| {
            for node in n.iter_mut() {
                node.status = CanvasNodeStatus::Completed;
            }
        });
    };

    // Thao tác: Đặt lại trạng thái mặc định
    let reset_pipeline = move || {
        let default_list = create_default_canvas_nodes(state.active_file_name());
        nodes.set(default_list);
        is_pipeline_running.set(true);
        state.is_running.set(true);
        overall_message.set("⚡ Đang thực thi: Bước 3/5 (Tách File theo Cột Phòng Ban)...".to_string());
    };

    // Thao tác: Xóa một node khỏi canvas
    let remove_node = move |node_id: String| {
        nodes.update(|n| {
            if let Some(pos) = n.iter().position(|node| node.id == node_id) {
                // Không xóa InputSource hoặc OutputDestination để bảo toàn DAG pipeline
                if n[pos].kind == CanvasNodeKind::Transform {
                    n.remove(pos);
                }
            }
        });
    };

    // Thao tác: Thêm một tool mới từ Palette vào trước Node Output
    let add_tool_node = move |slug: &'static str| {
        let tool = state.find_tool(slug);
        let name = tool.map(|t| t.name).unwrap_or(slug).to_string();
        let icon = data_lab_tool_icon(slug).to_string();
        let cat = category_for_tool(slug);
        let color = color_for_slug(slug);

        let new_id = format!("node-transform-{}", js_sys_time());
        let new_node = CanvasNode {
            id: new_id,
            name: name.clone(),
            icon,
            kind: CanvasNodeKind::Transform,
            color,
            status: CanvasNodeStatus::Ready,
            summary: format!("Cấu hình mặc định cho {}", name),
            tool_slug: Some(slug.to_string()),
            category: cat,
            row_info: "Sẵn sàng nhận dữ liệu từ bước trước".to_string(),
            config_details: format!("Công cụ: {} • Xử lý 100% on-device", name),
        };

        nodes.update(|n| {
            // Chèn vào vị trí áp chót (trước OutputDestination)
            let insert_pos = if n.is_empty() {
                0
            } else if n.last().map(|last| last.kind == CanvasNodeKind::OutputDestination).unwrap_or(false) {
                n.len() - 1
            } else {
                n.len()
            };
            n.insert(insert_pos, new_node);
        });

        is_palette_open.set(false);
    };

    view! {
        <div class="visual-canvas-view">
            // Header: Điều khiển luồng, trạng thái tổng thể và các nút hành động chính
            <header class="canvas-toolbar">
                <div class="canvas-toolbar-left">
                    <div class="canvas-title-group">
                        <span class="canvas-title-icon" aria-hidden="true">"⚡"</span>
                        <div>
                            <h2 class="canvas-title">"Visual Data Pipeline Canvas"</h2>
                            <span class="canvas-subtitle">"Mô hình DAG dòng chảy dữ liệu (KNIME / Alteryx / Easy Data Transform)"</span>
                        </div>
                    </div>
                </div>

                <div class="canvas-toolbar-center">
                    <div class="canvas-overall-status">
                        <span class="status-pulse-dot" class:running=move || is_pipeline_running.get()></span>
                        <span class="status-text">{move || overall_message.get()}</span>
                    </div>
                </div>

                <div class="canvas-toolbar-right">
                    <button
                        type="button"
                        class="canvas-action-btn canvas-btn-palette"
                        title="Mở bảng chọn 13 công cụ Data Lab để thêm bước mới"
                        on:click=move |_| is_palette_open.update(|open| *open = !*open)
                    >
                        <span class="btn-icon" aria-hidden="true">"+"</span>
                        <span class="btn-text">"Thêm Node Công cụ"</span>
                    </button>

                    {move || {
                        if is_pipeline_running.get() {
                            view! {
                                <button
                                    type="button"
                                    class="canvas-action-btn canvas-btn-stop"
                                    title="Tạm dừng thực thi chuỗi quy trình"
                                    on:click=move |_| stop_pipeline()
                                >
                                    <span class="btn-icon" aria-hidden="true">"⏹"</span>
                                    <span class="btn-text">"Dừng"</span>
                                </button>
                            }.into_any()
                        } else {
                            view! {
                                <button
                                    type="button"
                                    class="canvas-action-btn canvas-btn-run"
                                    title="Chạy toàn bộ luồng quy trình dữ liệu (Ctrl+Enter)"
                                    on:click=move |_| run_pipeline()
                                >
                                    <span class="btn-icon" aria-hidden="true">"▶"</span>
                                    <span class="btn-text">"Chạy toàn bộ luồng"</span>
                                </button>
                            }.into_any()
                        }
                    }}

                    <button
                        type="button"
                        class="canvas-action-btn canvas-btn-secondary"
                        title="Đánh dấu hoàn tất toàn bộ"
                        on:click=move |_| complete_pipeline()
                    >
                        <span class="btn-icon" aria-hidden="true">"✓"</span>
                        <span class="btn-text">"Hoàn tất"</span>
                    </button>

                    <button
                        type="button"
                        class="canvas-action-btn canvas-btn-reset"
                        title="Đặt lại chuỗi mẫu ban đầu"
                        on:click=move |_| reset_pipeline()
                    >
                        <span class="btn-icon" aria-hidden="true">"↺"</span>
                        <span class="btn-text">"Đặt lại"</span>
                    </button>
                </div>
            </header>

            // Vùng bảng vẽ trực quan (Canvas Viewport với dot-pattern grid)
            <div class="canvas-viewport">
                // Khối chứa các node và đường nối dạng chuỗi liên kết ngang
                <div class="canvas-nodes-container">
                    {move || {
                        let current_nodes = nodes.get();
                        let total = current_nodes.len();

                        current_nodes.into_iter().enumerate().map(|(idx, node)| {
                            let is_last = idx + 1 >= total;
                            let node_status = node.status.clone();
                            let node_id_for_remove = node.id.clone();
                            let node_for_config = node.clone();
                            let is_node_running = node_status == CanvasNodeStatus::Running;

                            // Trạng thái đường dây nối sau node này (nếu không phải node cuối)
                            let wire_class = if is_node_running {
                                "wire-running"
                            } else if node_status == CanvasNodeStatus::Completed {
                                "wire-completed"
                            } else {
                                "wire-idle"
                            };

                            view! {
                                <div class="canvas-node-wrapper" data-node-id=node.id.clone()>
                                    // Thẻ biểu diễn Node
                                    <div
                                        class=format!("canvas-node-card {} {}", node.color.class_name(), if is_node_running { "is-running" } else { "" })
                                    >
                                        // Cổng Input Port (bên trái)
                                        {if node.kind != CanvasNodeKind::InputSource {
                                            view! {
                                                <div class="canvas-port canvas-port-in" title="Cổng Dữ liệu Đầu vào (Data In)">
                                                    <span class="port-socket"></span>
                                                    <span class="port-label">"In"</span>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <span class="canvas-port-placeholder"></span> }.into_any()
                                        }}

                                        // Cổng Output Port (bên phải)
                                        {if node.kind != CanvasNodeKind::OutputDestination {
                                            view! {
                                                <div class="canvas-port canvas-port-out" title="Cổng Dữ liệu Đầu ra (Data Out)">
                                                    <span class="port-label">"Out"</span>
                                                    <span class="port-socket"></span>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <span class="canvas-port-placeholder"></span> }.into_any()
                                        }}

                                        // Header của Node
                                        <div class="canvas-node-header">
                                            <div class="header-left">
                                                <span class="node-icon" aria-hidden="true">{node.icon.clone()}</span>
                                                <span class="node-category-tag">
                                                    {match node.kind {
                                                        CanvasNodeKind::InputSource => "Nguồn Tệp".to_string(),
                                                        CanvasNodeKind::Transform => node.category.map(|c| c.label().to_string()).unwrap_or_else(|| "Biến đổi".to_string()),
                                                        CanvasNodeKind::OutputDestination => "Đích Xuất".to_string(),
                                                    }}
                                                </span>
                                            </div>

                                            <div class="header-actions">
                                                <button
                                                    type="button"
                                                    class="node-tool-btn node-btn-settings"
                                                    title="Mở cấu hình thông số Node"
                                                    on:click={
                                                        let n = node_for_config.clone();
                                                        move |_| active_config_node.set(Some(n.clone()))
                                                    }
                                                >
                                                    "⚙️"
                                                </button>

                                                {if node.kind == CanvasNodeKind::Transform {
                                                    let nid = node_id_for_remove.clone();
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="node-tool-btn node-btn-delete"
                                                            title="Xóa Node khỏi luồng quy trình"
                                                            on:click=move |_| remove_node(nid.clone())
                                                        >
                                                            "✖"
                                                        </button>
                                                    }.into_any()
                                                } else {
                                                    view! { <span class="node-tool-btn-placeholder"></span> }.into_any()
                                                }}
                                            </div>
                                        </div>

                                        // Body của Node: Tên và mô tả thông số tóm tắt
                                        <div class="canvas-node-body">
                                            <h3 class="node-name">{node.name.clone()}</h3>
                                            <p class="node-summary">{node.summary.clone()}</p>
                                            <div class="node-stats-pill">
                                                <span class="stats-icon" aria-hidden="true">"📊"</span>
                                                <span class="stats-text">{node.row_info.clone()}</span>
                                            </div>
                                        </div>

                                        // Footer của Node: Badge trạng thái và thứ tự bước
                                        <div class="canvas-node-footer">
                                            <span class=format!("canvas-status-badge {}", node.status.badge_class())>
                                                <span class="status-icon" aria-hidden="true">{node.status.icon()}</span>
                                                <span class="status-label">{node.status.label()}</span>
                                            </span>

                                            <span class="node-step-index">
                                                {format!("Bước {}/{}", idx + 1, total)}
                                            </span>
                                        </div>
                                    </div>

                                    // Đường dây nối SVG (Connecting Wire) sang Node tiếp theo
                                    {if !is_last {
                                        view! {
                                            <div class="canvas-wire-container" aria-hidden="true">
                                                <svg class="canvas-wire-svg" viewBox="0 0 80 50" preserveAspectRatio="none">
                                                    <defs>
                                                        <linearGradient id=format!("wire-grad-{}", idx) x1="0%" y1="0%" x2="100%" y2="0%">
                                                            <stop offset="0%" stop-color=node.color.stroke_color() stop-opacity="0.9" />
                                                            <stop offset="100%" stop-color="var(--border-color)" stop-opacity="0.8" />
                                                        </linearGradient>
                                                    </defs>

                                                    // Đường cong Bezier nối giữa Out của node trước và In của node sau
                                                    <path
                                                        d="M 0 25 C 35 25, 45 25, 80 25"
                                                        class=format!("canvas-wire-path {}", wire_class)
                                                    />

                                                    // Chấm hạt sáng chuyển động (Flow particle) khi luồng đang chạy
                                                    {if is_node_running {
                                                        view! {
                                                            <circle r="3.5" class="wire-flowing-particle">
                                                                <animateMotion
                                                                    path="M 0 25 C 35 25, 45 25, 80 25"
                                                                    dur="1.2s"
                                                                    repeatCount="indefinite"
                                                                />
                                                            </circle>
                                                        }.into_any()
                                                    } else {
                                                        view! { <g></g> }.into_any()
                                                    }}
                                                </svg>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <span class="canvas-wire-none"></span> }.into_any()
                                    }}
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    }}
                </div>
            </div>

            // Drawer / Palette 13 công cụ Data Lab để thêm Node mới
            {move || {
                if is_palette_open.get() {
                    view! {
                        <div class="canvas-palette-drawer">
                            <div class="palette-drawer-header">
                                <div class="palette-title-group">
                                    <span class="palette-icon" aria-hidden="true">"🧰"</span>
                                    <div>
                                        <h3 class="palette-title">"Bảng Chọn 13 Công cụ Data Lab"</h3>
                                        <p class="palette-desc">"Bấm vào một công cụ để chèn một Node biến đổi mới vào chuỗi pipeline DAG"</p>
                                    </div>
                                </div>
                                <button
                                    type="button"
                                    class="palette-close-btn"
                                    title="Đóng bảng chọn"
                                    on:click=move |_| is_palette_open.set(false)
                                >
                                    "✕ Đóng"
                                </button>
                            </div>

                            <div class="palette-categories-scroll">
                                {DataLabCategory::ALL.into_iter().map(|cat| {
                                    view! {
                                        <div class="palette-category-block">
                                            <div class="palette-cat-header">
                                                <span class="cat-icon" aria-hidden="true">{cat.icon()}</span>
                                                <h4 class="cat-label">{cat.label()}</h4>
                                                <span class="cat-count">{format!("({} công cụ)", cat.slugs().len())}</span>
                                            </div>

                                            <div class="palette-tools-grid">
                                                {cat.slugs().iter().map(|&slug| {
                                                    let tool = state.find_tool(slug);
                                                    let name = tool.map(|t| t.name).unwrap_or(slug);
                                                    let desc = tool.map(|t| t.description).unwrap_or("");
                                                    let icon = data_lab_tool_icon(slug);

                                                    view! {
                                                        <div class="palette-tool-card">
                                                            <div class="tool-card-top">
                                                                <span class="tool-card-icon" aria-hidden="true">{icon}</span>
                                                                <div class="tool-card-title-group">
                                                                    <h5 class="tool-card-name">{name}</h5>
                                                                    <span class="tool-card-slug">{slug}</span>
                                                                </div>
                                                            </div>
                                                            <p class="tool-card-desc">{desc}</p>
                                                            <button
                                                                type="button"
                                                                class="tool-card-add-btn"
                                                                title=format!("Thêm công cụ {} vào Canvas", name)
                                                                on:click=move |_| add_tool_node(slug)
                                                            >
                                                                <span class="btn-icon" aria-hidden="true">"+"</span>
                                                                <span>"Thêm vào Canvas"</span>
                                                            </button>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span class="palette-closed"></span> }.into_any()
                }
            }}

            // Modal cấu hình Node (Node Configuration Inspector)
            {move || {
                if let Some(cfg_node) = active_config_node.get() {
                    let node_id = cfg_node.id.clone();
                    let node_name = cfg_node.name.clone();
                    let node_summary = cfg_node.summary.clone();
                    let node_details = cfg_node.config_details.clone();

                    let edited_summary = RwSignal::new(node_summary);

                    let save_config = move || {
                        let new_summary = edited_summary.get();
                        nodes.update(|n| {
                            if let Some(node) = n.iter_mut().find(|item| item.id == node_id) {
                                node.summary = new_summary;
                            }
                        });
                        active_config_node.set(None);
                    };

                    view! {
                        <div class="canvas-modal-backdrop" on:click=move |_| active_config_node.set(None)>
                            <div class="canvas-modal-dialog" on:click=move |ev| ev.stop_propagation()>
                                <div class="canvas-modal-header">
                                    <div class="modal-title-row">
                                        <span class="modal-icon" aria-hidden="true">{cfg_node.icon.clone()}</span>
                                        <div>
                                            <h3 class="modal-title">{format!("Cấu hình Node: {}", node_name)}</h3>
                                            <span class="modal-id">{format!("Định danh: {}", cfg_node.id)}</span>
                                        </div>
                                    </div>
                                    <button
                                        type="button"
                                        class="modal-close-btn"
                                        title="Đóng cấu hình"
                                        on:click=move |_| active_config_node.set(None)
                                    >
                                        "✕"
                                    </button>
                                </div>

                                <div class="canvas-modal-body">
                                    <div class="config-field-group">
                                        <label class="config-label">"Tóm tắt tham số xử lý:"</label>
                                        <input
                                            type="text"
                                            class="config-input-text"
                                            prop:value=move || edited_summary.get()
                                            on:input=move |ev| edited_summary.set(event_target_value(&ev))
                                        />
                                    </div>

                                    <div class="config-field-group">
                                        <label class="config-label">"Chi tiết cấu hình thuật toán:"</label>
                                        <div class="config-readonly-box">
                                            {node_details}
                                        </div>
                                    </div>

                                    <div class="config-hint-box">
                                        <span class="hint-icon" aria-hidden="true">"🔒"</span>
                                        <span class="hint-text">
                                            "Các thay đổi cấu hình được áp dụng trực tiếp cho bước tính toán tiếp theo mà không cần tải lại tệp. Toàn bộ xử lý diễn ra 100% On-Device."
                                        </span>
                                    </div>
                                </div>

                                <div class="canvas-modal-footer">
                                    <button
                                        type="button"
                                        class="modal-btn-cancel"
                                        on:click=move |_| active_config_node.set(None)
                                    >
                                        "Hủy"
                                    </button>
                                    <button
                                        type="button"
                                        class="modal-btn-save"
                                        on:click=move |_| save_config()
                                    >
                                        "Lưu Cấu hình"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span class="modal-closed"></span> }.into_any()
                }
            }}
        </div>
    }
}

/// Hàm phụ trợ sinh timestamp cho id node duy nhất
fn js_sys_time() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(1)
    }
}
