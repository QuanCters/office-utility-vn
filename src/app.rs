use leptos::ev::{KeyboardEvent, MouseEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["navigator", "clipboard"])]
    fn writeText(text: &str) -> js_sys::Promise;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolItem {
    pub slug: &'static str,
    pub name: &'static str,
    pub domain: &'static str,
    pub description: &'static str,
    pub is_pinned: bool,
}

const ALL_TOOLS: &[ToolItem] = &[
    ToolItem {
        slug: "vietnamese-number-to-words",
        name: "Đọc Số Tiền Thành Chữ Chuẩn",
        domain: "vn-office",
        description: "Chuyển đổi số tiền thành chữ tiếng Việt chuẩn xác theo quy tắc kế toán.",
        is_pinned: true,
    },
    ToolItem {
        slug: "gross-net-salary-calculator",
        name: "Tính Lương Gross - Net & Thuế",
        domain: "vn-office",
        description: "Tính lương Gross sang Net và đối soát thuế TNCN, BHXH theo luật mới.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vietqr-napas247-generator",
        name: "Tạo Mã VietQR Napas247",
        domain: "vn-office",
        description: "Tạo mã QR thanh toán Napas247 chuẩn EMVCo kèm logo ngân hàng.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vn-einvoice-xml-reader",
        name: "Đọc Hóa đơn Điện tử XML VN",
        domain: "vn-office",
        description: "Đọc, bóc tách và đối soát dữ liệu hóa đơn điện tử VAT chuẩn Tổng cục Thuế.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vn-cccd-mst-validator",
        name: "Kiểm tra Hợp lệ CCCD & Mã số thuế",
        domain: "vn-office",
        description: "Giải mã cấu trúc CCCD 12 số và kiểm tra tính hợp lệ Mã số thuế bằng Modulo 11.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vn-address-normalizer",
        name: "Chuẩn hóa Địa giới Hành chính VN",
        domain: "vn-office",
        description: "Khớp chuỗi mờ Jaro-Winkler chuẩn hóa địa chỉ theo mô hình 2 cấp và 3 cấp.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vn-working-days-calc",
        name: "Tính Ngày Làm việc Trừ Ngày lễ VN",
        domain: "vn-office",
        description: "Tính số ngày làm việc trừ ngày nghỉ lễ theo Bộ luật Lao động và xuất tệp .ics.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vn-loan-amortization",
        name: "Bảng tính Trả nợ Vay Ngân hàng",
        domain: "vn-office",
        description: "Tính lịch trả nợ vay theo dư nợ giảm dần hoặc niên kim cố định chính xác từng đồng.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vn-bank-bin-mapper",
        name: "Tra cứu Mã Định danh Ngân hàng",
        domain: "vn-office",
        description: "Tra cứu mã BIN Napas, CITAD và SWIFT code hơn 60 ngân hàng Việt Nam.",
        is_pinned: true,
    },
    ToolItem {
        slug: "pdf-merge-split",
        name: "Ghép, Tách & Xoay trang PDF",
        domain: "doc-suite",
        description: "Ghép nhiều file PDF hoặc tách trang theo dải lựa chọn cực nhanh.",
        is_pinned: true,
    },
    ToolItem {
        slug: "pdf-protect-sign",
        name: "Bảo mật & Mở khóa PDF",
        domain: "doc-suite",
        description: "Áp dụng mã hóa AES-256 hoặc mở khóa mật khẩu PDF an toàn on-device.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-to-markdown",
        name: "Chuyển đổi PDF sang Markdown",
        domain: "doc-suite",
        description: "Bóc tách luồng văn bản, font size và tái tạo bảng biểu sang cú pháp Markdown.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-compress",
        name: "Nén giảm Dung lượng PDF",
        domain: "doc-suite",
        description: "Nén lại các luồng FlateDecode và tái lấy mẫu ảnh nhúng giảm dung lượng file.",
        is_pinned: false,
    },
    ToolItem {
        slug: "vietnamese-ocr",
        name: "OCR Nhận dạng Ký tự Tiếng Việt",
        domain: "doc-suite",
        description: "Nhận dạng chữ tiếng Việt có dấu từ tài liệu scan hoặc ảnh bằng OS Native OCR hoặc VietOCR.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-watermark-stamp",
        name: "Đóng dấu Watermark & Con dấu đỏ",
        domain: "doc-suite",
        description: "Chèn luồng đồ họa ExtGState chứa watermark bản quyền hoặc nhúng con dấu đỏ PNG.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-page-number-header",
        name: "Đánh số trang & Bates Numbering",
        domain: "doc-suite",
        description: "Đánh số trang tự động, Bates Numbering và tùy biến căn lề header/footer.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-metadata-scrubber",
        name: "Tẩy sạch Metadata ẩn PDF",
        domain: "doc-suite",
        description: "Xóa sạch Trailer Info và luồng XML metadata XMP bảo mật danh tính tác giả.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-flatten-forms",
        name: "Làm phẳng PDF (Flatten Forms)",
        domain: "doc-suite",
        description: "Làm phẳng biểu mẫu AcroForm, hợp nhất dữ liệu nhập liệu thành văn bản tĩnh.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-crop-margin",
        name: "Cắt Viền Trắng Thừa (Auto-Crop)",
        domain: "doc-suite",
        description: "Tìm hộp giới hạn Bounding Box nội dung và cập nhật lại CropBox cắt lề chính xác.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-booklet-imposition",
        name: "Dàn trang In Sách Gấp đôi & N-up",
        domain: "doc-suite",
        description: "Tính toán ma trận biến đổi cm và sắp xếp lại thứ tự trang theo thuật toán đóng sách lồng Saddle-Stitch khổ A4/A5 hoặc ghép 2-up, 4-up trên một mặt giấy.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-visual-diff",
        name: "So sánh Đối soát 2 Bản PDF",
        domain: "doc-suite",
        description: "Hỗ trợ 2 chế độ: Text Diff (so sánh câu chữ) và Visual Pixel Diff (so sánh hình ảnh và tô đỏ vùng sai khác).",
        is_pinned: false,
    },
    ToolItem {
        slug: "docx-template-mailmerge",
        name: "Trộn Hợp đồng Mailmerge Hàng loạt",
        domain: "doc-suite",
        description: "Giải nén luồng XML, hợp nhất thẻ <w:r> bị cắt vụn và trộn hàng loạt file Word tốc độ cao.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-bookmark-toc",
        name: "Tạo Cây Mục lục PDF (Bookmark TOC)",
        domain: "doc-suite",
        description: "Khởi tạo và liên kết cấu trúc cây từ điển /Outlines đa cấp trong PDF Catalog trỏ trực tiếp đến Page Object ID.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-to-long-image",
        name: "Ghép PDF thành Ảnh Dọc Dài",
        domain: "doc-suite",
        description: "Kết xuất từng trang PDF và ghép nối tiếp theo chiều dọc thành một ảnh dài duy nhất kèm đường phân cách trang.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-grayscale-print",
        name: "Chuyển PDF sang Đen trắng",
        domain: "doc-suite",
        description: "Chuyển đổi toàn bộ toán tử màu trong luồng nội dung và hình ảnh nhúng sang DeviceGray 8-bit tiết kiệm mực in.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-repair-corrupt",
        name: "Phục hồi File PDF Lỗi Cấu trúc",
        domain: "doc-suite",
        description: "Quét tuyến tính toàn bộ mảng byte để tìm obj / endobj và tái tạo lại bảng xref cùng trailer bị hỏng.",
        is_pinned: false,
    },
    ToolItem {
        slug: "markdown-to-styled-pdf",
        name: "Biên dịch Markdown sang PDF",
        domain: "doc-suite",
        description: "Phân tích Markdown AST, kết xuất PDF chuẩn khổ in A4 với Unicode tiếng Việt, tự động ngắt trang và kẻ bảng.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-table-extractor",
        name: "Trích xuất Bảng từ PDF ra Excel",
        domain: "doc-suite",
        description: "Phân tích tọa độ (x, y) cụm từ và đường kẻ vector để khôi phục bảng biểu ra file .xlsx và .csv.",
        is_pinned: false,
    },









    ToolItem {
        slug: "image-batch",
        name: "Nén & Đổi Định dạng Ảnh Hàng loạt",
        domain: "media-studio",
        description: "Tối ưu hóa ảnh WebP, AVIF, JPEG đa luồng với Rayon.",
        is_pinned: true,
    },
    ToolItem {
        slug: "photo-id-crop",
        name: "Cắt Ảnh Thẻ Chuẩn 300 DPI",
        domain: "media-studio",
        description: "Cắt ảnh thẻ theo tỷ lệ chuẩn và dàn trang khổ in 10x15 cm ở 300 DPI.",
        is_pinned: true,
    },
    ToolItem {
        slug: "image-bg-redact",
        name: "Làm mờ Dữ liệu Nhạy cảm Giấy tờ",
        domain: "media-studio",
        description: "Che mờ hoặc tô đen vĩnh viễn vùng thông tin nhạy cảm (CCCD, mã QR) trên giấy tờ.",
        is_pinned: true,
    },
    ToolItem {
        slug: "image-exif-inspector",
        name: "Kiểm tra & Xóa Exif/GPS Ảnh",
        domain: "media-studio",
        description: "Đọc thông số máy ảnh, tọa độ GPS và tước bỏ siêu dữ liệu bảo vệ riêng tư.",
        is_pinned: true,
    },
    ToolItem {
        slug: "watermark-batch-studio",
        name: "Đóng dấu Bản quyền Ảnh Hàng loạt",
        domain: "media-studio",
        description: "Chèn logo hoặc chữ bản quyền dạng lưới lặp lại (Tiled) hoặc ở góc cố định.",
        is_pinned: true,
    },
    ToolItem {
        slug: "screenshot-beautifier",
        name: "Làm đẹp Ảnh Chụp Màn hình",
        domain: "media-studio",
        description: "Kết xuất khung viền macOS/Windows sắc nét và hiệu ứng đổ bóng mềm trên nền Gradient.",
        is_pinned: true,
    },
    ToolItem {
        slug: "doc-scanner-perspective",
        name: "Phẳng hóa Tài liệu Chụp Nghiêng",
        domain: "media-studio",
        description: "Nắn phẳng góc nghiêng 4 điểm phối cảnh và tăng tương phản chữ đen trên nền trắng.",
        is_pinned: true,
    },
    ToolItem {
        slug: "audio-silence-trimmer",
        name: "Cắt Bỏ Khoảng lặng Ghi âm",
        domain: "media-studio",
        description: "Phân tích RMS năng lượng âm thanh và loại bỏ các đoạn im lặng kéo dài.",
        is_pinned: true,
    },
    ToolItem {
        slug: "video-to-gif-webp",
        name: "Chuyển Đổi Video Ngắn sang GIF / WebP",
        domain: "media-studio",
        description: "Chuyển đổi video clip, cắt trích xuất khung hình và lượng tử hóa bảng màu GIF động mượt mà.",
        is_pinned: true,
    },
    ToolItem {
        slug: "universal-file-converter-hub",
        name: "Trạm Chuyển đổi Tệp Vạn năng",
        domain: "sys-hub",
        description: "Tự động nhận diện chữ ký file và chuyển đổi định dạng linh hoạt.",
        is_pinned: true,
    },
    ToolItem {
        slug: "vietnamese-font-converter",
        name: "Chuyển mã Font Tiếng Việt",
        domain: "data-lab",
        description: "Chuyển mã font TCVN3 (ABC), VNI sang Unicode tổ hợp và dựng sẵn.",
        is_pinned: false,
    },
    ToolItem {
        slug: "excel-cleaner-normalizer",
        name: "Làm sạch & Chuẩn hóa Bảng tính",
        domain: "data-lab",
        description: "Chuẩn hóa định dạng ngày tháng, viết hoa tên riêng, tách họ tên và loại bỏ dòng trùng lặp.",
        is_pinned: false,
    },
    ToolItem {
        slug: "diff-checker-text-sheets",
        name: "Đối chiếu Văn bản & Bảng tính",
        domain: "data-lab",
        description: "Thuật toán Myers Diff tính toán và tô màu trực quan từng từ, dòng, ô bảng tính bị sai lệch.",
        is_pinned: false,
    },
    ToolItem {
        slug: "data-structure-converter",
        name: "Chuyển đổi Cấu trúc Dữ liệu",
        domain: "data-lab",
        description: "Chuyển đổi hai chiều tốc độ cao giữa bảng tính với JSON, SQL INSERT INTO hoặc Markdown.",
        is_pinned: false,
    },
    ToolItem {
        slug: "excel-split-by-column",
        name: "Tách File Excel theo Cột",
        domain: "data-lab",
        description: "Phân nhóm dữ liệu theo giá trị của cột chỉ định và xuất thành các file riêng biệt.",
        is_pinned: false,
    },
    ToolItem {
        slug: "excel-multi-merger",
        name: "Gộp Nhiều File Excel / Sheet",
        domain: "data-lab",
        description: "Gộp nối tiếp dòng hoặc gộp thành nhiều Sheet trong một Workbook kèm truy vết nguồn.",
        is_pinned: false,
    },
    ToolItem {
        slug: "excel-visual-joiner",
        name: "Nối Trực quan Bảng tính (Join)",
        domain: "data-lab",
        description: "Thực hiện Hash Join (INNER, LEFT, FULL, ANTI) giữa hai bảng tính theo cột khóa.",
        is_pinned: false,
    },
    ToolItem {
        slug: "excel-sheet-unlocker",
        name: "Gỡ Khóa Bảo vệ Chỉnh sửa Sheet Excel",
        domain: "data-lab",
        description: "Loại bỏ thẻ khóa bảo vệ sheetProtection trong OpenXML để cho phép chỉnh sửa toàn diện.",
        is_pinned: false,
    },
    ToolItem {
        slug: "list-set-operations",
        name: "Phép toán Tập hợp Danh sách",
        domain: "data-lab",
        description: "Thực hiện phép toán tập hợp giao, hợp, hiệu và hiệu đối xứng cho các danh sách dòng.",
        is_pinned: false,
    },
    ToolItem {
        slug: "encoding-mojibake-fixer",
        name: "Sửa Lỗi Hiển thị Chữ Mojibake",
        domain: "data-lab",
        description: "Khôi phục chuỗi văn bản tiếng Việt bị lỗi hiển thị bảng mã UTF-8 và Windows-1252 / ISO-8859-1.",
        is_pinned: false,
    },
    ToolItem {
        slug: "text-pii-anonymizer",
        name: "Ẩn danh hóa Dữ liệu Nhạy cảm PII",
        domain: "data-lab",
        description: "Quét và ẩn tự động số CCCD, SĐT di động VN, MST, STK ngân hàng thành nhãn ẩn danh.",
        is_pinned: false,
    },
    ToolItem {
        slug: "sql-formatter-linter",
        name: "Định dạng & Linter Câu lệnh SQL",
        domain: "data-lab",
        description: "Kiểm tra lỗi cú pháp AST bằng linter và căn chỉnh thụt lề, viết hoa từ khóa theo các phương ngữ SQL.",
        is_pinned: false,
    },
    ToolItem {
        slug: "base64-file-codec",
        name: "Mã hóa / Giải mã Base64 cho File",
        domain: "data-lab",
        description: "Mã hóa/giải mã theo luồng (Streaming Base64 Codec) giữa tệp nhị phân và chuỗi Base64 / Data URI mà không làm tăng đột biến bộ nhớ RAM.",
        is_pinned: false,
    },
];

#[derive(Serialize, Deserialize)]
struct DispatchArgs {
    request: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct AlwaysOnTopArgs {
    enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: Option<String>,
    pub release_date: Option<String>,
    pub should_prompt: bool,
}

#[derive(Serialize, Deserialize)]
struct CheckUpdateArgs {
    manual: bool,
}

#[derive(Serialize, Deserialize)]
struct SetUpdateActionArgs {
    action: String,
    target_version: Option<String>,
}

#[component]
pub fn App() -> impl IntoView {
    // Trạng thái cập nhật ứng dụng (Updater Lifecycle)
    let (update_modal_open, set_update_modal_open) = signal(false);
    let (available_update, set_available_update) = signal(None::<UpdateInfo>);
    let (is_checking_update, set_is_checking_update) = signal(false);
    let (is_installing_update, set_is_installing_update) = signal(false);
    let (update_toast, set_update_toast) = signal(None::<String>);

    // Hàm gọi kiểm tra cập nhật (thủ công hoặc tự động ngầm)
    let check_for_updates = move |manual: bool| {
        set_is_checking_update.set(true);
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&CheckUpdateArgs { manual }).unwrap();
            let resp = invoke("check_app_update", args).await;
            if let Ok(info) = serde_wasm_bindgen::from_value::<UpdateInfo>(resp) {
                if info.has_update {
                    set_available_update.set(Some(info.clone()));
                    if info.should_prompt || manual {
                        set_update_modal_open.set(true);
                    }
                } else if manual {
                    set_update_toast.set(Some(format!(
                        "Bạn đang dùng phiên bản v{} mới nhất!",
                        info.current_version
                    )));
                }
            } else if manual {
                set_update_toast.set(Some("Không thể kết nối máy chủ cập nhật lúc này.".to_string()));
            }
            set_is_checking_update.set(false);
        });
    };

    // Tự động kiểm tra sau 3s khởi động (Non-blocking Cold Start SLA < 300ms)
    {
        let check_fn = check_for_updates;
        spawn_local(async move {
            let promise = js_sys::Promise::new(&mut |resolve, _| {
                if let Some(win) = web_sys::window() {
                    let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 3000);
                }
            });
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            check_fn(false);
        });
    }

    // Xử lý lựa chọn: NOW, LATER, SKIP
    let handle_update_action = move |action: &'static str| {
        let current_target = available_update.get().map(|u| u.latest_version);
        if action == "NOW" {
            set_is_installing_update.set(true);
            spawn_local(async move {
                let empty_args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
                let _ = invoke("install_app_update", empty_args).await;
                set_is_installing_update.set(false);
                set_update_modal_open.set(false);
            });
        } else {
            let act_str = action.to_string();
            spawn_local(async move {
                let args = serde_wasm_bindgen::to_value(&SetUpdateActionArgs {
                    action: act_str,
                    target_version: current_target,
                })
                .unwrap();
                let _ = invoke("set_update_action", args).await;
            });
            set_update_modal_open.set(false);
        }
    };

    // Trạng thái công cụ hiện tại
    let (current_slug, set_current_slug) = signal("vietnamese-number-to-words".to_string());
    let (input_value, set_input_value) = signal("1250000".to_string());
    let (dialect, set_dialect) = signal("North".to_string());
    let (use_dong_chan, set_use_dong_chan) = signal(true);
    let (output_result, set_output_result) = signal(String::new());

    // Trạng thái thanh tiến trình
    let (progress_percent, set_progress_percent) = signal(0.0f32);
    let (progress_message, set_progress_message) = signal(String::new());
    let (is_running, set_is_running) = signal(false);

    // Trạng thái giao diện cửa sổ
    let (is_always_on_top, set_is_always_on_top) = signal(false);
    let (palette_open, set_palette_open) = signal(false);
    let (search_query, set_search_query) = signal(String::new());
    let (clipboard_toast, set_clipboard_toast) = signal(None::<String>);
    let (selected_files, set_selected_files) = signal(Vec::<String>::new());

    // Window controls & system operations
    let minimize_win = move || {
        spawn_local(async move {
            let empty_args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
            let _ = invoke("minimize_window", empty_args).await;
        });
    };
    let toggle_maximize_win = move || {
        spawn_local(async move {
            let empty_args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
            let _ = invoke("toggle_maximize_window", empty_args).await;
        });
    };
    let close_win = move || {
        spawn_local(async move {
            let empty_args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
            let _ = invoke("close_window", empty_args).await;
        });
    };
    let open_output_dir = move || {
        spawn_local(async move {
            let empty_args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
            let _ = invoke("open_output_folder", empty_args).await;
        });
    };
    let copy_result_to_clipboard = move || {
        let res = output_result.get();
        if !res.is_empty() {
            let _ = writeText(&res);
            set_clipboard_toast.set(Some("Đã sao chép kết quả vào bộ nhớ tạm!".to_string()));
            spawn_local(async move {
                let promise = js_sys::Promise::new(&mut |resolve, _| {
                    if let Some(win) = web_sys::window() {
                        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 2500);
                    }
                });
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                set_clipboard_toast.set(None);
            });
        }
    };

    // IME-Safe flag cho bộ gõ tiếng Việt (UniKey, EVKey)
    let (_is_ime_composing, set_is_ime_composing) = signal(false);

    // Lấy thông tin công cụ đang chọn
    let current_tool = move || {
        let slug = current_slug.get();
        ALL_TOOLS
            .iter()
            .find(|t| t.slug == slug)
            .cloned()
            .unwrap_or(ALL_TOOLS[0].clone())
    };

    // Toggle Always-on-Top
    let toggle_pin = move || {
        let new_state = !is_always_on_top.get();
        set_is_always_on_top.set(new_state);
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&AlwaysOnTopArgs { enabled: new_state }).unwrap();
            let _ = invoke("toggle_always_on_top", args).await;
        });
    };

    // Thực thi tác vụ (Ctrl + Enter)
    let run_task = move || {
        if is_running.get() {
            return;
        }

        set_is_running.set(true);
        set_progress_percent.set(10.0);
        set_progress_message.set("Đang chuẩn bị tác vụ...".to_string());

        let slug = current_slug.get();
        let num_str = input_value.get();
        let cur_dialect = dialect.get();
        let dong_chan = use_dong_chan.get();

        spawn_local(async move {
            set_progress_percent.set(50.0);
            set_progress_message.set("Đang tính toán nội bộ 100% On-Device...".to_string());

            let request_payload = serde_json::json!({
                "task_id": format!("task-{}", js_sys::Date::now() as u64),
                "tool_slug": slug,
                "input_paths": [],
                "output_directory": null,
                "options": {
                    "number": num_str,
                    "dialect": cur_dialect,
                    "currency": "VND",
                    "use_dong_chan": dong_chan,
                    "capitalize_first": true,
                    "negative_prefix": "âm"
                }
            });

            let args = serde_wasm_bindgen::to_value(&DispatchArgs {
                request: request_payload,
            })
            .unwrap();

            let resp = invoke("dispatch_tool_task", args).await;
            if let Ok(res_obj) = serde_wasm_bindgen::from_value::<serde_json::Value>(resp) {
                if let Some(first) = res_obj
                    .get("output_paths")
                    .and_then(|v| v.as_array())
                    .and_then(|outputs| outputs.first())
                    .and_then(|v| v.as_str())
                {
                    set_output_result.set(first.to_string());
                }
                set_progress_percent.set(100.0);
                set_progress_message.set("Đã hoàn thành xuất sắc".to_string());
            } else {
                set_output_result.set("Lỗi thực thi tác vụ".to_string());
                set_progress_message.set("Có lỗi phát sinh".to_string());
            }

            set_is_running.set(false);
        });
    };

    // Hủy tác vụ (Esc)
    let cancel_current_task = move || {
        if is_running.get() {
            set_is_running.set(false);
            set_progress_percent.set(0.0);
            set_progress_message.set("Đã dừng tác vụ".to_string());
        }
    };

    // Global keyboard shortcuts interception (Ctrl+P, Ctrl+W, Ctrl+O, Ctrl+K, Ctrl+V)
    let on_global_keydown = move |ev: KeyboardEvent| {
        let key = ev.key();
        let ctrl_or_meta = ev.ctrl_key() || ev.meta_key();

        if ctrl_or_meta && (key == "k" || key == "K") {
            ev.prevent_default();
            ev.stop_propagation();
            set_palette_open.update(|v| *v = !*v);
        } else if ctrl_or_meta && (key == "p" || key == "P") {
            ev.prevent_default();
            toggle_pin();
        } else if ctrl_or_meta && (key == "o" || key == "O") {
            ev.prevent_default();
            open_output_dir();
        } else if ctrl_or_meta && (key == "w" || key == "W") {
            ev.prevent_default();
            close_win();
        } else if key == "Escape" {
            if palette_open.get() {
                set_palette_open.set(false);
            } else if is_running.get() {
                cancel_current_task();
            }
        } else if ctrl_or_meta && key == "Enter" {
            ev.prevent_default();
            run_task();
        }
    };

    // Lọc công cụ trong Command Palette
    let filtered_tools = move || {
        let q = search_query.get().trim().to_lowercase();
        if q.is_empty() {
            ALL_TOOLS.to_vec()
        } else {
            ALL_TOOLS
                .iter()
                .filter(|t| {
                    t.name.to_lowercase().contains(&q)
                        || t.slug.to_lowercase().contains(&q)
                        || t.domain.to_lowercase().contains(&q)
                })
                .cloned()
                .collect()
        }
    };

    view! {
        <div class="app-window" on:keydown=on_global_keydown tabindex="0">
            <header class="titlebar" data-tauri-drag-region="true">
                <div class="titlebar-left">
                    <span style="font-size: 16px;">"⚡"</span>
                    <span>"Office Utility VN"</span>
                    <span style="font-size: 11px; color: var(--text-muted); font-weight: 400;">"v0.1.0"</span>
                    <button
                        class="titlebar-update-btn"
                        prop:disabled=move || is_checking_update.get()
                        on:click=move |_| check_for_updates(true)
                        title="Kiểm tra bản cập nhật mới nhất"
                    >
                        { move || if is_checking_update.get() { "🔄 Đang kiểm tra..." } else { "🔄 Kiểm tra cập nhật" } }
                    </button>
                </div>

                <div class="titlebar-center">
                    <button class="quick-search-btn" on:click=move |_| set_palette_open.set(true)>
                        <span>"🔍 Tìm nhanh công cụ hoặc mẫu Preset... (Ctrl + K)"</span>
                    </button>
                </div>

                <div class="titlebar-right">
                    <button
                        class=move || if is_always_on_top.get() { "pin-btn pinned" } else { "pin-btn" }
                        on:click=move |_| toggle_pin()
                        title="Ghim cửa sổ nổi Always-on-Top (Ctrl + P)"
                    >
                        { move || if is_always_on_top.get() { "📌 Đã ghim" } else { "📌 Ghim" } }
                    </button>
                    <button class="window-control-btn" title="Thu nhỏ (Minimize)" on:click=move |_| minimize_win()>"🗕"</button>
                    <button class="window-control-btn" title="Phóng to (Maximize)" on:click=move |_| toggle_maximize_win()>"🗖"</button>
                    <button class="window-control-btn close" title="Đóng (Close)" on:click=move |_| close_win()>"✕"</button>
                </div>
            </header>

            <main class="workspace">
                <section class="zone-1">
                    <div class="tool-info">
                        <span style="font-size: 20px;">"🛠️"</span>
                        <div>
                            <div class="tool-name">{ move || current_tool().name }</div>
                            <div style="font-size: 11px; color: var(--text-muted);">
                                { move || current_tool().description }
                            </div>
                        </div>
                        <span class="trust-badge">
                            "🟢 100% On-Device Native (Rayon)"
                        </span>
                    </div>

                    <div class="preset-selector">
                        <label style="font-size: 12px; color: var(--text-muted);">"🎛️ Mẫu cấu hình:"</label>
                        <select class="preset-select">
                            <option>"Mặc định tiêu chuẩn"</option>
                            <option>"Kế toán thuế (Có chẵn)"</option>
                            <option>"Báo cáo tài chính năm"</option>
                        </select>
                        <button class="save-preset-btn">"💾 Lưu mẫu"</button>
                    </div>
                </section>

                <section class="zone-2">
                    <div
                        class="dropzone"
                        on:click=move |_| {
                            set_selected_files.update(|f| f.push("hóa_đơn_tháng_11.xlsx".to_string()));
                        }
                    >
                        <div style="font-size: 28px; margin-bottom: 6px;">"📥"</div>
                        <div class="dropzone-title">"KÉO THẢ TỆP TỪ EXPLORER / FINDER VÀO ĐÂY HOẶC NHẤN ĐỂ CHỌN"</div>
                        <div class="dropzone-sub">
                            "(Hỗ trợ dán dữ liệu hoặc tệp trực tiếp từ Clipboard qua phím tắt Ctrl + V)"
                        </div>
                    </div>

                    { move || {
                        let fs = selected_files.get();
                        if !fs.is_empty() {
                            view! {
                                <div style="font-size: 12px; color: #10B981; display: flex; align-items: center; gap: 6px;">
                                    <span>"✓ Đã nạp:"</span>
                                    <b>{ fs.join(", ") }</b>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div></div> }.into_any()
                        }
                    }}

                    <div class="tool-config-panel">
                        <div class="form-group">
                            <label class="form-label">"Nhập số tiền hoặc số bất kỳ cần đọc:"</label>
                            <input
                                class="form-input"
                                type="text"
                                prop:value=input_value
                                on:compositionstart=move |_| set_is_ime_composing.set(true)
                                on:compositionend=move |_| set_is_ime_composing.set(false)
                                on:input=move |ev| {
                                    set_input_value.set(event_target_value(&ev));
                                }
                                placeholder="Ví dụ: 1250000 hoặc 1.250.000"
                            />
                        </div>

                        <div class="options-row">
                            <div class="form-group">
                                <label class="form-label">"Phương ngữ:"</label>
                                <div style="display: flex; gap: 12px; margin-top: 4px;">
                                    <label class="radio-item">
                                        <input
                                            type="radio"
                                            name="dialect"
                                            checked=move || dialect.get() == "North"
                                            on:change=move |_| set_dialect.set("North".to_string())
                                        />
                                        "Miền Bắc (nghìn, lẻ)"
                                    </label>
                                    <label class="radio-item">
                                        <input
                                            type="radio"
                                            name="dialect"
                                            checked=move || dialect.get() == "South"
                                            on:change=move |_| set_dialect.set("South".to_string())
                                        />
                                        "Miền Nam (ngàn, linh)"
                                    </label>
                                </div>
                            </div>

                            <div class="form-group">
                                <label class="form-label">"Quy chuẩn kế toán:"</label>
                                <div style="display: flex; gap: 12px; margin-top: 4px;">
                                    <label class="radio-item">
                                        <input
                                            type="checkbox"
                                            checked=use_dong_chan
                                            on:change=move |ev| set_use_dong_chan.set(event_target_checked(&ev))
                                        />
                                        "Thêm hậu tố 'đồng chẵn'"
                                    </label>
                                </div>
                            </div>
                        </div>

                        { move || {
                            let res = output_result.get();
                            if !res.is_empty() {
                                let res_val = res.clone();
                                view! {
                                    <div class="result-box">
                                        <div class="result-header">
                                            <span class="result-label">"KẾT QUẢ ĐỌC SỐ CHUẨN:"</span>
                                            <button
                                                class="copy-btn"
                                                title="Sao chép kết quả vào bộ nhớ tạm"
                                                on:click=move |_| copy_result_to_clipboard()
                                            >
                                                "📋 Sao chép"
                                            </button>
                                        </div>
                                        <div class="result-content selectable-text">{res_val}</div>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }}
                    </div>
                </section>

                <section class="zone-3">
                    <div class="progress-track">
                        <div
                            class="progress-bar"
                            style=move || format!("width: {}%", progress_percent.get())
                        ></div>
                    </div>

                    <div class="progress-meta">
                        <span>{ move || progress_message.get() }</span>
                        <span>{ move || format!("{:.0}%", progress_percent.get()) }</span>
                    </div>

                    <div class="action-bar">
                        <button
                            class="btn-primary"
                            prop:disabled=move || is_running.get()
                            on:click=move |_| run_task()
                        >
                            <span>"⚡ THỰC THI NGAY (Ctrl + Enter)"</span>
                        </button>

                        <button
                            class="btn-danger"
                            prop:disabled=move || !is_running.get()
                            on:click=move |_| cancel_current_task()
                        >
                            <span>"🛑 HỦY (Esc)"</span>
                        </button>

                        <button
                            class="btn-secondary"
                            title="Mở thư mục lưu tệp kết quả (Ctrl + O)"
                            on:click=move |_| open_output_dir()
                        >
                            <span>"📂 MỞ THƯ MỤC KẾT QUẢ (Ctrl + O)"</span>
                        </button>
                    </div>
                </section>
            </main>

            { move || if palette_open.get() {
                view! {
                    <div class="modal-overlay" on:click=move |_| set_palette_open.set(false)>
                        <div class="command-palette" on:click=move |ev: MouseEvent| ev.stop_propagation()>
                            <div class="palette-input-box">
                                <span style="font-size: 18px;">"🔍"</span>
                                <input
                                    class="palette-input"
                                    type="text"
                                    placeholder="Tìm kiếm công cụ theo tên, chức năng hoặc miền nghiệp vụ..."
                                    autofocus=true
                                    prop:value=search_query
                                    on:input=move |ev| set_search_query.set(event_target_value(&ev))
                                />
                                <span style="font-size: 11px; color: var(--text-muted); background: #334155; padding: 2px 6px; border-radius: 4px;">"Esc để đóng"</span>
                            </div>

                            <div class="palette-list">
                                {
                                    filtered_tools().into_iter().map(|tool| {
                                        let slug_copy = tool.slug.to_string();
                                        view! {
                                            <div
                                                class="palette-item"
                                                on:click=move |_| {
                                                    set_current_slug.set(slug_copy.clone());
                                                    set_palette_open.set(false);
                                                }
                                            >
                                                <div>
                                                    <div style="font-weight: 600;">{tool.name}</div>
                                                    <div style="font-size: 11px; opacity: 0.8;">{tool.description}</div>
                                                </div>
                                                <span class="palette-item-badge">{tool.domain}</span>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()
                                }
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}

            { move || {
                if let Some(msg) = clipboard_toast.get() {
                    view! {
                        <div class="clipboard-toast">
                            <span>"📋"</span>
                            <span>{msg}</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            { move || if update_modal_open.get() {
                if let Some(info) = available_update.get() {
                    let notes = info.release_notes.unwrap_or_else(|| "Bản cập nhật tối ưu hóa hiệu năng, sửa lỗi và cải tiến trải nghiệm người dùng.".to_string());
                    let latest_ver = info.latest_version.clone();
                    view! {
                        <div class="modal-overlay" on:click=move |_| set_update_modal_open.set(false)>
                            <div class="update-dialog" on:click=move |ev: MouseEvent| ev.stop_propagation()>
                                <div class="update-dialog-header">
                                    <div class="update-dialog-title">
                                        <span>"🚀"</span>
                                        <span>"Đã có Bản Cập nhật Mới!"</span>
                                    </div>
                                    <button class="window-control-btn close" on:click=move |_| set_update_modal_open.set(false)>"✕"</button>
                                </div>

                                <div class="update-dialog-body">
                                    <div class="update-version-badge">
                                        {format!("Phiên bản mới: v{}", latest_ver)}
                                    </div>
                                    <p style="margin: 0; font-size: 13px; color: #E2E8F0;">
                                        "Phiên bản mới của Office Utility VN đã sẵn sàng. Dưới đây là tóm tắt những thay đổi nổi bật:"
                                    </p>
                                    <div class="update-notes-box">
                                        {notes}
                                    </div>
                                </div>

                                <div class="update-dialog-footer">
                                    <button
                                        class="btn-skip-version"
                                        title="Không bao giờ hỏi lại về phiên bản này"
                                        on:click=move |_| handle_update_action("SKIP")
                                    >
                                        "Bỏ qua bản này"
                                    </button>

                                    <div style="display: flex; gap: 8px;">
                                        <button
                                            class="btn-update-later"
                                            title="Nhắc tôi lại sau 24 giờ"
                                            on:click=move |_| handle_update_action("LATER")
                                        >
                                            "⏰ Nhắc lại sau"
                                        </button>
                                        <button
                                            class="btn-update-now"
                                            prop:disabled=move || is_installing_update.get()
                                            on:click=move |_| handle_update_action("NOW")
                                        >
                                            { move || if is_installing_update.get() { "⏳ Đang cài đặt..." } else { "🚀 Cập nhật ngay" } }
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            } else {
                view! { <div></div> }.into_any()
            }}

            { move || {
                if let Some(msg) = update_toast.get() {
                    view! {
                        <div class="clipboard-toast" on:click=move |_| set_update_toast.set(None)>
                            <span>"🔄"</span>
                            <span>{msg}</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
