use leptos::prelude::*;
use crate::app::{ToolItem, ALL_TOOLS};

#[component]
pub fn HomeView<FSelect>(
    recent_slugs: ReadSignal<Vec<String>>,
    favorite_slugs: ReadSignal<Vec<String>>,
    on_select_tool: FSelect,
) -> impl IntoView
where
    FSelect: Fn(String) + Send + Sync + Copy + 'static,
{
    let find_tool = |slug: &str| -> Option<ToolItem> {
        ALL_TOOLS.iter().find(|t| t.slug == slug).cloned()
    };

    let resume_tools = move || {
        let recents = recent_slugs.get();
        if !recents.is_empty() {
            recents
                .into_iter()
                .take(4)
                .filter_map(|s| find_tool(&s))
                .collect::<Vec<ToolItem>>()
        } else {
            let favs = favorite_slugs.get();
            if !favs.is_empty() {
                favs
                    .into_iter()
                    .take(4)
                    .filter_map(|s| find_tool(&s))
                    .collect::<Vec<ToolItem>>()
            } else {
                ALL_TOOLS.iter().take(4).cloned().collect::<Vec<ToolItem>>()
            }
        }
    };

    view! {
        <div class="home-view-container">
            <div class="home-hero-banner">
                <div class="home-hero-badge">
                    <span class="pulse-dot">"🟢"</span>
                    <span>"100% On-Device Native · Zero-Cloud Security"</span>
                </div>
                <h1 class="home-hero-title">
                    "Trung Tâm Tiện Ích Văn Phòng & Nghiệp Vụ VN"
                </h1>
                <p class="home-hero-subtitle">
                    "Hơn 54 công cụ chuyên sâu chạy trực tiếp trên máy tính cá nhân. Bảo vệ trọn vẹn dữ liệu bảng lương, hóa đơn VAT, chứng từ CCCD và báo cáo nội bộ mà không có bất kỳ byte dữ liệu nào rời khỏi thiết bị."
                </p>
            </div>

            // Section 1: Tiếp tục công việc (Resume Cards)
            <div class="home-section">
                <div class="home-section-header">
                    <span class="home-section-title">"⚡ TIẾP TỤC CÔNG VIỆC NHANH"</span>
                    <span class="home-section-sub">"Công cụ bạn vừa sử dụng gần đây"</span>
                </div>
                <div class="home-cards-grid">
                    { move || {
                        let tools = resume_tools();
                        tools.into_iter().map(|tool| {
                            let slug = tool.slug.to_string();
                            view! {
                                <div class="home-tool-card" on:click=move |_| on_select_tool(slug.clone())>
                                    <div class="home-card-top">
                                        <span class="home-card-icon">"🛠️"</span>
                                        <span class="home-card-domain-badge">{tool.domain}</span>
                                    </div>
                                    <div class="home-card-name">{tool.name}</div>
                                    <div class="home-card-desc">{tool.description}</div>
                                    <div class="home-card-action">
                                        <span>"Mở ngay"</span>
                                        <span>"→"</span>
                                    </div>
                                </div>
                            }
                        }).collect_view()
                    }}
                </div>
            </div>

            // Section 2: Khám phá các phân hệ nghiệp vụ
            <div class="home-section">
                <div class="home-section-header">
                    <span class="home-section-title">"📂 CÁC PHÂN HỆ NĂNG LỰC CHÍNH"</span>
                    <span class="home-section-sub">"Được chuẩn hóa sâu cho nghiệp vụ kế toán, nhân sự & hành chính"</span>
                </div>
                <div class="home-suite-grid">
                    <div
                        class="home-suite-card"
                        on:click=move |_| on_select_tool("vietnamese-number-to-words".to_string())
                    >
                        <div class="home-suite-icon">"🇻🇳"</div>
                        <div class="home-suite-content">
                            <div class="home-suite-title">"Nghiệp vụ Hành chính VN"</div>
                            <div class="home-suite-desc">
                                "Đọc số tiền thành chữ, tính lương Gross - Net, VietQR Napas247, đối soát XML hóa đơn điện tử Tổng cục Thuế, kiểm tra CCCD 12 số & MST Modulo 11."
                            </div>
                            <div class="home-suite-count">"9 công cụ chuyên sâu"</div>
                        </div>
                    </div>

                    <div
                        class="home-suite-card"
                        on:click=move |_| on_select_tool("pdf-merge-split".to_string())
                    >
                        <div class="home-suite-icon">"📄"</div>
                        <div class="home-suite-content">
                            <div class="home-suite-title">"Document Suite (PDF / DOCX)"</div>
                            <div class="home-suite-desc">
                                "Ghép, tách, nén dung lượng, OCR tiếng Việt có dấu, đóng dấu watermark bản quyền, làm phẳng AcroForm và trộn hợp đồng Mailmerge hàng loạt."
                            </div>
                            <div class="home-suite-count">"19 công cụ chuyên sâu"</div>
                        </div>
                    </div>

                    <div
                        class="home-suite-card"
                        on:click=move |_| on_select_tool("excel-cleaner-normalizer".to_string())
                    >
                        <div class="home-suite-icon">"📊"</div>
                        <div class="home-suite-content">
                            <div class="home-suite-title">"Data Lab (Excel / Bảng tính)"</div>
                            <div class="home-suite-desc">
                                "Làm sạch bảng tính, Myers Diff đối soát văn bản, nối bảng Hash Join, gỡ khóa bảo vệ Sheet và chuyển đổi font TCVN3 / VNI sang Unicode chuẩn."
                            </div>
                            <div class="home-suite-count">"13 công cụ chuyên sâu"</div>
                        </div>
                    </div>

                    <div
                        class="home-suite-card"
                        on:click=move |_| on_select_tool("image-batch".to_string())
                    >
                        <div class="home-suite-icon">"🎨"</div>
                        <div class="home-suite-content">
                            <div class="home-suite-title">"Media Studio (Ảnh / Video / Âm thanh)"</div>
                            <div class="home-suite-desc">
                                "Nén ảnh WebP/AVIF đa luồng Rayon, cắt ảnh thẻ 300 DPI, che mờ thông tin nhạy cảm giấy tờ, phẳng hóa tài liệu chụp nghiêng và xóa siêu dữ liệu Exif/GPS."
                            </div>
                            <div class="home-suite-count">"9 công cụ chuyên sâu"</div>
                        </div>
                    </div>
                </div>
            </div>

            // Section 3: Bảng hướng dẫn phím tắt năng suất cao
            <div class="home-section shortcuts-panel">
                <div class="home-section-header">
                    <span class="home-section-title">"⌨️ PHÍM TẮT TOÀN CỤC NĂNG SUẤT CAO"</span>
                </div>
                <div class="home-shortcuts-row">
                    <div class="shortcut-pill">
                        <span class="kbd">"Ctrl"</span>
                        <span>"+"</span>
                        <span class="kbd">"K"</span>
                        <span class="shortcut-desc">"Tìm nhanh công cụ / preset"</span>
                    </div>
                    <div class="shortcut-pill">
                        <span class="kbd">"Ctrl"</span>
                        <span>"+"</span>
                        <span class="kbd">"Enter"</span>
                        <span class="shortcut-desc">"Thực thi tác vụ ngay"</span>
                    </div>
                    <div class="shortcut-pill">
                        <span class="kbd">"Ctrl"</span>
                        <span>"+"</span>
                        <span class="kbd">"P"</span>
                        <span class="shortcut-desc">"Ghim nổi Always-on-Top"</span>
                    </div>
                    <div class="shortcut-pill">
                        <span class="kbd">"Ctrl"</span>
                        <span>"+"</span>
                        <span class="kbd">"O"</span>
                        <span class="shortcut-desc">"Mở thư mục xuất kết quả"</span>
                    </div>
                    <div class="shortcut-pill">
                        <span class="kbd">"Esc"</span>
                        <span class="shortcut-desc">"Hủy tác vụ / Đóng bảng"</span>
                    </div>
                </div>
            </div>
        </div>
    }
}

