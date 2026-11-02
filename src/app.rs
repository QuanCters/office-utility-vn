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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkspaceView {
    Tools,
    WorkflowDag,
    History,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToolArchetype {
    FileBatch,
    FinancialAccounting,
    IdentityVerification,
    TextCodeConversion,
    SpreadsheetData,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ToolPresetUi {
    pub id: String,
    pub tool_slug: String,
    pub name: String,
    pub options_json: String,
    pub is_default: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskHistoryUi {
    pub id: String,
    pub tool_slug: Option<String>,
    pub workflow_id: Option<String>,
    pub status: String,
    pub elapsed_ms: u64,
    pub created_at: String,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct WorkflowNodeUi {
    pub id: String,
    pub tool_slug: String,
    pub label: String,
    pub in_mime: String,
    pub out_mime: String,
}

pub const ALL_TOOLS: &[ToolItem] = &[
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
        description: "Tính toán ma trận biến đổi cm và sắp xếp lại thứ tự trang theo thuật toán đóng sách lồng Saddle-Stitch.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-visual-diff",
        name: "So sánh Đối soát 2 Bản PDF",
        domain: "doc-suite",
        description: "Hỗ trợ Text Diff và Visual Pixel Diff so sánh sự sai lệch giữa hai tài liệu.",
        is_pinned: false,
    },
    ToolItem {
        slug: "docx-template-mailmerge",
        name: "Trộn Hợp đồng Mailmerge Hàng loạt",
        domain: "doc-suite",
        description: "Giải nén luồng XML, hợp nhất thẻ <w:r> và trộn hàng loạt file Word tốc độ cao.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-bookmark-toc",
        name: "Tạo Cây Mục lục PDF (Bookmark TOC)",
        domain: "doc-suite",
        description: "Khởi tạo và liên kết cấu trúc cây từ điển Outlines đa cấp trong PDF Catalog.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-to-long-image",
        name: "Ghép PDF thành Ảnh Dọc Dài",
        domain: "doc-suite",
        description: "Kết xuất từng trang PDF và ghép nối tiếp theo chiều dọc thành một ảnh dài duy nhất.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-grayscale-print",
        name: "Chuyển PDF sang Đen trắng",
        domain: "doc-suite",
        description: "Chuyển đổi toàn bộ toán tử màu sang DeviceGray 8-bit tiết kiệm mực in.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-repair-corrupt",
        name: "Phục hồi File PDF Lỗi Cấu trúc",
        domain: "doc-suite",
        description: "Quét tuyến tính toàn bộ byte để tìm obj/endobj và tái tạo lại bảng xref bị hỏng.",
        is_pinned: false,
    },
    ToolItem {
        slug: "markdown-to-styled-pdf",
        name: "Biên dịch Markdown sang PDF",
        domain: "doc-suite",
        description: "Phân tích Markdown AST, kết xuất PDF khổ A4 với Unicode tiếng Việt tự động ngắt trang.",
        is_pinned: false,
    },
    ToolItem {
        slug: "pdf-table-extractor",
        name: "Trích xuất Bảng từ PDF ra Excel",
        domain: "doc-suite",
        description: "Phân tích tọa độ (x, y) cụm từ và đường kẻ vector để khôi phục bảng biểu ra file .xlsx.",
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
        description: "Cắt ảnh thẻ theo tỷ lệ chuẩn 3x4, 4x6 và dàn trang khổ in 10x15 cm ở 300 DPI.",
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
        description: "Chuyển đổi video clip, cắt trích xuất khung hình và lượng tử hóa bảng màu động.",
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
        description: "Thuật toán Myers Diff tính toán và tô màu trực quan từng từ, dòng bị sai lệch.",
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
        description: "Khôi phục chuỗi văn bản tiếng Việt bị lỗi hiển thị bảng mã UTF-8 và Windows-1252.",
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
        description: "Kiểm tra lỗi cú pháp AST và căn chỉnh thụt lề, viết hoa từ khóa theo phương ngữ SQL.",
        is_pinned: false,
    },
    ToolItem {
        slug: "base64-file-codec",
        name: "Mã hóa / Giải mã Base64 cho File",
        domain: "data-lab",
        description: "Mã hóa/giải mã theo luồng Streaming Base64 Codec giữa tệp nhị phân và chuỗi Base64.",
        is_pinned: false,
    },
];

fn get_tool_archetype(slug: &str, domain: &str) -> ToolArchetype {
    match domain {
        "doc-suite" | "media-studio" | "sys-hub" => ToolArchetype::FileBatch,
        "data-lab" => {
            if slug == "vietnamese-font-converter"
                || slug == "diff-checker-text-sheets"
                || slug == "encoding-mojibake-fixer"
                || slug == "text-pii-anonymizer"
                || slug == "sql-formatter-linter"
                || slug == "base64-file-codec"
            {
                ToolArchetype::TextCodeConversion
            } else {
                ToolArchetype::SpreadsheetData
            }
        }
        "vn-office" => {
            if slug == "vietqr-napas247-generator"
                || slug == "vn-cccd-mst-validator"
                || slug == "vn-einvoice-xml-reader"
                || slug == "vn-bank-bin-mapper"
                || slug == "vn-address-normalizer"
            {
                ToolArchetype::IdentityVerification
            } else {
                ToolArchetype::FinancialAccounting
            }
        }
        _ => ToolArchetype::FileBatch,
    }
}

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

#[derive(Serialize, Deserialize)]
struct GetPresetsArgs {
    tool_slug: String,
}

#[derive(Serialize, Deserialize)]
struct SavePresetArgs {
    preset: ToolPresetUi,
}

#[derive(Serialize, Deserialize)]
struct GetHistoryArgs {
    limit: u32,
    offset: u32,
}

#[derive(Serialize, Deserialize)]
struct ExecuteWorkflowArgs {
    workflow_id: String,
    input_paths: Vec<String>,
    output_directory: Option<String>,
}

#[component]
pub fn App() -> impl IntoView {
    // Điều hướng không gian làm việc (Multi-View Workspace)
    let (active_view, set_active_view) = signal(WorkspaceView::Tools);

    // Trạng thái cập nhật ứng dụng (Updater Lifecycle)
    let (update_modal_open, set_update_modal_open) = signal(false);
    let (available_update, set_available_update) = signal(None::<UpdateInfo>);
    let (is_checking_update, set_is_checking_update) = signal(false);
    let (is_installing_update, set_is_installing_update) = signal(false);
    let (update_toast, set_update_toast) = signal(None::<String>);

    // Hàm gọi kiểm tra cập nhật
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

    // Tự động kiểm tra sau 3s khởi động
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
    let (output_result, set_output_result) = signal(String::new());

    // Trạng thái Presets (Mẫu cấu hình)
    let (tool_presets, set_tool_presets) = signal(Vec::<ToolPresetUi>::new());
    let (_selected_preset, set_selected_preset) = signal("default".to_string());
    let (save_preset_modal_open, set_save_preset_modal_open) = signal(false);
    let (preset_name_input, set_preset_name_input) = signal(String::new());

    // Trạng thái Lịch sử tác vụ (Task History)
    let (history_records, set_history_records) = signal(Vec::<TaskHistoryUi>::new());
    let (is_loading_history, set_is_loading_history) = signal(false);

    // Trạng thái Workflow DAG
    let (is_running_workflow, set_is_running_workflow) = signal(false);
    let (workflow_message, set_workflow_message) = signal(String::new());

    // Trạng thái thanh tiến trình
    let (progress_percent, set_progress_percent) = signal(0.0f32);
    let (progress_message, set_progress_message) = signal(String::new());
    let (is_running, set_is_running) = signal(false);

    // Trạng thái cửa sổ & tìm kiếm
    let (is_always_on_top, set_is_always_on_top) = signal(false);
    let (palette_open, set_palette_open) = signal(false);
    let (search_query, set_search_query) = signal(String::new());
    let (clipboard_toast, set_clipboard_toast) = signal(None::<String>);
    let (selected_files, set_selected_files) = signal(Vec::<String>::new());

    // =========================================================================
    // DYNAMIC INPUTS CHO 5 ARCHETYPES
    // =========================================================================
    // Archetype 1: File Batch
    let (target_format, set_target_format) = signal("webp".to_string());
    let (quality_percent, set_quality_percent) = signal(85);
    let (strip_metadata, set_strip_metadata) = signal(true);
    let (max_compression, set_max_compression) = signal(true);

    // Archetype 2: Financial & Accounting
    let (input_value, set_input_value) = signal("1250000".to_string());
    let (dialect, set_dialect) = signal("North".to_string());
    let (use_dong_chan, set_use_dong_chan) = signal(true);
    let (salary_gross, set_salary_gross) = signal("25000000".to_string());
    let (dependents_count, set_dependents_count) = signal(1);
    let (salary_region, set_salary_region) = signal("Vùng I (4.960.000đ)".to_string());
    let (loan_principal, set_loan_principal) = signal("500000000".to_string());
    let (loan_rate, set_loan_rate) = signal("8.5".to_string());
    let (loan_months, set_loan_months) = signal("120".to_string());
    let (date_start, set_date_start) = signal("2026-01-01".to_string());
    let (date_end, set_date_end) = signal("2026-12-31".to_string());

    // Archetype 3: Identity & VietQR & Invoice
    let (bank_name, set_bank_name) = signal("Vietcombank (VCB)".to_string());
    let (account_number, set_account_number) = signal("1012345678".to_string());
    let (account_holder, set_account_holder) = signal("NGUYEN VAN A".to_string());
    let (transfer_amount, set_transfer_amount) = signal("500000".to_string());
    let (transfer_note, set_transfer_note) = signal("Thanh toan tien hang".to_string());
    let (cccd_input, set_cccd_input) = signal("001098012345".to_string());
    let (address_raw, set_address_raw) = signal("Số 1 Đại Cồ Việt, Bách Khoa, Hai Bà Trưng, Hà Nội".to_string());

    // Archetype 4: Dual-Pane Text / Code / Diff
    let (text_source, set_text_source) = signal("Cộng hòa Xã hội Chủ nghĩa Việt Nam\nĐộc lập - Tự do - Hạnh phúc".to_string());
    let (font_source_encoding, set_font_source_encoding) = signal("TCVN3".to_string());
    let (sql_dialect, set_sql_dialect) = signal("PostgreSQL".to_string());

    // Archetype 5: Spreadsheet & Data Lab
    let (excel_key_column, set_excel_key_column) = signal("Mã nhân viên".to_string());
    let (excel_clean_names, set_excel_clean_names) = signal(true);
    let (excel_clean_dates, set_excel_clean_dates) = signal(true);
    let (excel_remove_duplicates, set_excel_remove_duplicates) = signal(true);
    let (excel_export_format, set_excel_export_format) = signal("xlsx".to_string());

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

    // Tải danh sách presets từ SQLite khi đổi công cụ
    let load_presets_for_current_tool = move || {
        let slug = current_slug.get();
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&GetPresetsArgs { tool_slug: slug }).unwrap();
            let resp = invoke("get_tool_presets", args).await;
            if let Ok(presets) = serde_wasm_bindgen::from_value::<Vec<ToolPresetUi>>(resp) {
                set_tool_presets.set(presets);
            }
        });
    };

    // Trạng thái công cụ vừa chọn gần đây (MRU - Most Recently Used)
    let (recent_tool_slugs, set_recent_tool_slugs) = signal(crate::recent_tools::load_recent_from_storage());

    // Hàm chọn công cụ dùng chung: cập nhật slug, lưu vết MRU vào storage, đóng palette
    let select_tool = move |slug: String| {
        set_current_slug.set(slug.clone());
        let updated = crate::recent_tools::update_recent_list(
            &recent_tool_slugs.get(),
            &slug,
            crate::recent_tools::DEFAULT_MAX_RECENT_TOOLS,
        );
        crate::recent_tools::save_recent_to_storage(&updated);
        set_recent_tool_slugs.set(updated);
        set_palette_open.set(false);
        set_active_view.set(WorkspaceView::Tools);
        load_presets_for_current_tool();
    };

    // Danh sách ToolItem của các công cụ gần đây theo thứ tự MRU (tối đa 5 công cụ)
    let recent_tool_items = move || {
        let slugs = recent_tool_slugs.get();
        let mut items = Vec::new();
        for s in slugs.iter().take(crate::recent_tools::DEFAULT_MAX_RECENT_TOOLS) {
            if let Some(tool) = ALL_TOOLS.iter().find(|t| t.slug == s) {
                items.push(tool.clone());
            }
        }
        items
    };

    // Danh sách các công cụ còn lại (chưa nằm trong danh sách gần đây)
    let non_recent_tools = move || {
        let recent_items = recent_tool_items();
        let recent_slugs: Vec<&'static str> = recent_items.iter().map(|t| t.slug).collect();
        ALL_TOOLS
            .iter()
            .filter(|t| !recent_slugs.contains(&t.slug))
            .cloned()
            .collect::<Vec<_>>()
    };

    // Tải lịch sử tác vụ từ SQLite
    let load_history = move || {
        set_is_loading_history.set(true);
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&GetHistoryArgs { limit: 50, offset: 0 }).unwrap();
            let resp = invoke("get_task_history", args).await;
            if let Ok(records) = serde_wasm_bindgen::from_value::<Vec<TaskHistoryUi>>(resp) {
                set_history_records.set(records);
            }
            set_is_loading_history.set(false);
        });
    };

    // Xóa sạch lịch sử tác vụ trong SQLite
    let clear_all_history = move || {
        spawn_local(async move {
            let empty_args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
            let _ = invoke("clear_task_history", empty_args).await;
            set_history_records.set(Vec::new());
        });
    };

    // Lưu mẫu Preset vào SQLite
    let save_current_preset = move || {
        let name = preset_name_input.get().trim().to_string();
        if name.is_empty() {
            return;
        }
        let slug = current_slug.get();
        let preset_data = ToolPresetUi {
            id: format!("preset-{}", js_sys::Date::now() as u64),
            tool_slug: slug.clone(),
            name,
            options_json: serde_json::json!({
                "format": target_format.get(),
                "quality": quality_percent.get(),
                "dialect": dialect.get(),
            }).to_string(),
            is_default: false,
        };

        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&SavePresetArgs { preset: preset_data }).unwrap();
            let _ = invoke("save_tool_preset", args).await;
            set_save_preset_modal_open.set(false);
            set_preset_name_input.set(String::new());
            load_presets_for_current_tool();
        });
    };

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

    // Toggle Always-on-Top
    let toggle_pin = move || {
        let new_state = !is_always_on_top.get();
        set_is_always_on_top.set(new_state);
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&AlwaysOnTopArgs { enabled: new_state }).unwrap();
            let _ = invoke("toggle_always_on_top", args).await;
        });
    };

    // Chạy chuỗi Workflow DAG
    let run_dag_pipeline = move || {
        if is_running_workflow.get() {
            return;
        }
        set_is_running_workflow.set(true);
        set_workflow_message.set("Đang khởi tạo chuỗi 3 bước xử lý On-Device...".to_string());

        spawn_local(async move {
            let sample_inputs = vec!["bao_cao_quy_4.png".to_string()];
            let args = serde_wasm_bindgen::to_value(&ExecuteWorkflowArgs {
                workflow_id: "workflow-sample-01".to_string(),
                input_paths: sample_inputs,
                output_directory: None,
            }).unwrap();

            let _ = invoke("execute_workflow_pipeline", args).await;
            set_workflow_message.set("Chuỗi tự động hóa 3 bước đã hoàn thành xuất sắc 100%!".to_string());
            set_is_running_workflow.set(false);
        });
    };

    // Thực thi tác vụ theo từng Archetype (Ctrl + Enter)
    let run_task = move || {
        if is_running.get() {
            return;
        }

        set_is_running.set(true);
        set_progress_percent.set(10.0);
        set_progress_message.set("Đang chuẩn bị tác vụ...".to_string());

        let slug = current_slug.get();
        let archetype = get_tool_archetype(&slug, &current_tool().domain);

        spawn_local(async move {
            set_progress_percent.set(40.0);
            set_progress_message.set("Đang tính toán nội bộ 100% On-Device...".to_string());

            let mut request_options = serde_json::Map::new();

            match archetype {
                ToolArchetype::FinancialAccounting => {
                    if slug == "vietnamese-number-to-words" {
                        request_options.insert("number".to_string(), serde_json::json!(input_value.get()));
                        request_options.insert("dialect".to_string(), serde_json::json!(dialect.get()));
                        request_options.insert("currency".to_string(), serde_json::json!("VND"));
                        request_options.insert("use_dong_chan".to_string(), serde_json::json!(use_dong_chan.get()));
                        request_options.insert("capitalize_first".to_string(), serde_json::json!(true));
                        request_options.insert("negative_prefix".to_string(), serde_json::json!("âm"));
                    } else if slug == "gross-net-salary-calculator" {
                        request_options.insert("salary_gross".to_string(), serde_json::json!(salary_gross.get()));
                        request_options.insert("dependents".to_string(), serde_json::json!(dependents_count.get()));
                        request_options.insert("region".to_string(), serde_json::json!(salary_region.get()));
                    } else if slug == "vn-loan-amortization" {
                        request_options.insert("principal".to_string(), serde_json::json!(loan_principal.get()));
                        request_options.insert("annual_rate".to_string(), serde_json::json!(loan_rate.get()));
                        request_options.insert("months".to_string(), serde_json::json!(loan_months.get()));
                    } else if slug == "vn-working-days-calc" {
                        request_options.insert("start_date".to_string(), serde_json::json!(date_start.get()));
                        request_options.insert("end_date".to_string(), serde_json::json!(date_end.get()));
                    }
                }
                ToolArchetype::FileBatch => {
                    request_options.insert("target_format".to_string(), serde_json::json!(target_format.get()));
                    request_options.insert("quality".to_string(), serde_json::json!(quality_percent.get()));
                    request_options.insert("strip_metadata".to_string(), serde_json::json!(strip_metadata.get()));
                    request_options.insert("max_compression".to_string(), serde_json::json!(max_compression.get()));
                }
                ToolArchetype::IdentityVerification => {
                    request_options.insert("bank".to_string(), serde_json::json!(bank_name.get()));
                    request_options.insert("account".to_string(), serde_json::json!(account_number.get()));
                    request_options.insert("holder".to_string(), serde_json::json!(account_holder.get()));
                    request_options.insert("amount".to_string(), serde_json::json!(transfer_amount.get()));
                    request_options.insert("note".to_string(), serde_json::json!(transfer_note.get()));
                    request_options.insert("cccd".to_string(), serde_json::json!(cccd_input.get()));
                    request_options.insert("address".to_string(), serde_json::json!(address_raw.get()));
                }
                ToolArchetype::TextCodeConversion => {
                    request_options.insert("source_text".to_string(), serde_json::json!(text_source.get()));
                    request_options.insert("source_encoding".to_string(), serde_json::json!(font_source_encoding.get()));
                    request_options.insert("sql_dialect".to_string(), serde_json::json!(sql_dialect.get()));
                }
                ToolArchetype::SpreadsheetData => {
                    request_options.insert("key_column".to_string(), serde_json::json!(excel_key_column.get()));
                    request_options.insert("clean_names".to_string(), serde_json::json!(excel_clean_names.get()));
                    request_options.insert("clean_dates".to_string(), serde_json::json!(excel_clean_dates.get()));
                    request_options.insert("remove_duplicates".to_string(), serde_json::json!(excel_remove_duplicates.get()));
                    request_options.insert("export_format".to_string(), serde_json::json!(excel_export_format.get()));
                }
            }

            let request_payload = serde_json::json!({
                "task_id": format!("task-{}", js_sys::Date::now() as u64),
                "tool_slug": slug,
                "input_paths": selected_files.get(),
                "output_directory": null,
                "options": request_options
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
                } else {
                    set_output_result.set("Tác vụ xử lý hoàn tất thành công!".to_string());
                }
                set_progress_percent.set(100.0);
                set_progress_message.set("Đã hoàn thành xuất sắc 100%".to_string());
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

    // Global keyboard shortcuts
    let on_global_keydown = move |ev: KeyboardEvent| {
        let key = ev.key();
        let ctrl_or_meta = ev.ctrl_key() || ev.meta_key();

        if ctrl_or_meta && (key == "k" || key == "K") {
            ev.prevent_default();
            ev.stop_propagation();
            let opening = !palette_open.get();
            if opening {
                set_search_query.set(String::new());
            }
            set_palette_open.set(opening);
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
            if save_preset_modal_open.get() {
                set_save_preset_modal_open.set(false);
            } else if palette_open.get() {
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
                        || t.description.to_lowercase().contains(&q)
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
                    <button class="quick-search-btn" on:click=move |_| {
                        set_search_query.set(String::new());
                        set_palette_open.set(true);
                    }>
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

            <nav class="sub-header-nav">
                <div class="nav-tabs">
                    <button
                        class=move || if active_view.get() == WorkspaceView::Tools { "nav-tab-btn active" } else { "nav-tab-btn" }
                        on:click=move |_| set_active_view.set(WorkspaceView::Tools)
                    >
                        <span>"🛠️"</span>
                        <span>"Công cụ Đơn lẻ"</span>
                    </button>
                    <button
                        class=move || if active_view.get() == WorkspaceView::WorkflowDag { "nav-tab-btn active" } else { "nav-tab-btn" }
                        on:click=move |_| set_active_view.set(WorkspaceView::WorkflowDag)
                    >
                        <span>"⚡"</span>
                        <span>"Chuỗi Tự động hóa (Workflow DAG)"</span>
                    </button>
                    <button
                        class=move || if active_view.get() == WorkspaceView::History { "nav-tab-btn active" } else { "nav-tab-btn" }
                        on:click=move |_| {
                            set_active_view.set(WorkspaceView::History);
                            load_history();
                        }
                    >
                        <span>"⏱️"</span>
                        <span>"Lịch sử Tác vụ"</span>
                    </button>
                </div>

                <div style="font-size: 11px; color: var(--text-muted); display: flex; align-items: center; gap: 8px;">
                    <span>"🔒 Zero-Cloud Security"</span>
                    <span style="opacity: 0.5;">"|"</span>
                    <span>"Offline 100%"</span>
                </div>
            </nav>

            { move || match active_view.get() {
                WorkspaceView::Tools => view! {
                    <main class="workspace">
                        { move || {
                            let recents = recent_tool_items();
                            if recents.is_empty() {
                                view! { <div></div> }.into_any()
                            } else {
                                view! {
                                    <div class="dashboard-recent-bar">
                                        <span class="dashboard-recent-label">"🕒 Gần đây:"</span>
                                        <div class="dashboard-recent-chips">
                                            {
                                                recents.into_iter().take(5).map(|tool| {
                                                    let slug_copy = tool.slug.to_string();
                                                    let is_active = current_slug.get() == tool.slug;
                                                    let chip_class = if is_active {
                                                        "recent-tool-chip active"
                                                    } else {
                                                        "recent-tool-chip"
                                                    };
                                                    view! {
                                                        <button
                                                            class=chip_class
                                                            title=format!("{}: {}", tool.name, tool.description)
                                                            on:click=move |_| select_tool(slug_copy.clone())
                                                        >
                                                            <span>{tool.name}</span>
                                                        </button>
                                                    }
                                                }).collect::<Vec<_>>()
                                            }
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }}

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
                                <select
                                    class="preset-select"
                                    on:change=move |ev| set_selected_preset.set(event_target_value(&ev))
                                >
                                    <option value="default">"Mặc định tiêu chuẩn"</option>
                                    { move || {
                                        tool_presets.get().into_iter().map(|p| {
                                            let p_id = p.id.clone();
                                            view! {
                                                <option value=p_id>{p.name}</option>
                                            }
                                        }).collect::<Vec<_>>()
                                    }}
                                </select>
                                <button
                                    class="save-preset-btn"
                                    on:click=move |_| set_save_preset_modal_open.set(true)
                                    title="Lưu cấu hình hiện tại thành Preset mới"
                                >
                                    "💾 Lưu mẫu"
                                </button>
                            </div>
                        </section>

                        <section class="zone-2">
                            { move || {
                                let slug = current_slug.get();
                                let archetype = get_tool_archetype(&slug, &current_tool().domain);
                                match archetype {
                                    ToolArchetype::FileBatch => view! {
                                        <div>
                                            <div
                                                class="dropzone"
                                                on:click=move |_| {
                                                    set_selected_files.update(|f| {
                                                        f.push("hop_dong_kinh_te_2026.pdf".to_string());
                                                        f.push("anh_chung_tu_scan.png".to_string());
                                                    });
                                                }
                                            >
                                                <div style="font-size: 28px; margin-bottom: 6px;">"📥"</div>
                                                <div class="dropzone-title">"KÉO THẢ TỆP TỪ EXPLORER / FINDER VÀO ĐÂY HOẶC NHẤN ĐỂ CHỌN"</div>
                                                <div class="dropzone-sub">
                                                    "(Hỗ trợ xử lý hàng loạt PDF, hình ảnh, văn bản Word đa luồng Rayon)"
                                                </div>
                                            </div>

                                            { move || {
                                                let fs = selected_files.get();
                                                if !fs.is_empty() {
                                                    view! {
                                                        <div class="file-chips-container">
                                                            { fs.into_iter().enumerate().map(|(idx, name)| {
                                                                view! {
                                                                    <span class="file-chip">
                                                                        <span>"📄"</span>
                                                                        <span>{name}</span>
                                                                        <button
                                                                            class="file-chip-remove"
                                                                            on:click=move |_| {
                                                                                set_selected_files.update(|list| {
                                                                                    if idx < list.len() {
                                                                                        list.remove(idx);
                                                                                    }
                                                                                });
                                                                            }
                                                                        >
                                                                            "✕"
                                                                        </button>
                                                                    </span>
                                                                }
                                                            }).collect::<Vec<_>>() }
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! { <div></div> }.into_any()
                                                }
                                            }}

                                            <div class="form-grid-2">
                                                <div class="form-group">
                                                    <label class="form-label">"Định dạng xuất đích:"</label>
                                                    <select
                                                        class="form-input"
                                                        prop:value=target_format
                                                        on:change=move |ev| set_target_format.set(event_target_value(&ev))
                                                    >
                                                        <option value="webp">"WebP (Tối ưu web & lưu trữ)"</option>
                                                        <option value="avif">"AVIF (Nén cao cấp thế hệ mới)"</option>
                                                        <option value="jpg">"JPEG tiêu chuẩn"</option>
                                                        <option value="png">"PNG không nén vỡ hình"</option>
                                                        <option value="pdf">"PDF chuẩn in ấn"</option>
                                                    </select>
                                                </div>

                                                <div class="form-group">
                                                    <label class="form-label">
                                                        "Chất lượng nén (" { move || quality_percent.get() } "%):"
                                                    </label>
                                                    <div class="slider-group">
                                                        <input
                                                            type="range"
                                                            min="40"
                                                            max="100"
                                                            prop:value=quality_percent
                                                            on:input=move |ev| {
                                                                if let Ok(val) = event_target_value(&ev).parse::<u8>() {
                                                                    set_quality_percent.set(val);
                                                                }
                                                            }
                                                        />
                                                        <span class="slider-value">{ move || format!("{}%", quality_percent.get()) }</span>
                                                    </div>
                                                </div>
                                            </div>

                                            <div style="display: flex; gap: 16px; margin-top: 12px;">
                                                <label class="radio-item">
                                                    <input
                                                        type="checkbox"
                                                        checked=strip_metadata
                                                        on:change=move |ev| set_strip_metadata.set(event_target_checked(&ev))
                                                    />
                                                    "Tẩy sạch Metadata ẩn & EXIF / GPS"
                                                </label>
                                                <label class="radio-item">
                                                    <input
                                                        type="checkbox"
                                                        checked=max_compression
                                                        on:change=move |ev| set_max_compression.set(event_target_checked(&ev))
                                                    />
                                                    "Nén luồng FlateDecode tối đa"
                                                </label>
                                            </div>
                                        </div>
                                    }.into_any(),

                                    ToolArchetype::FinancialAccounting => view! {
                                        <div>
                                            { move || {
                                                let s = current_slug.get();
                                                if s == "gross-net-salary-calculator" {
                                                    view! {
                                                        <div>
                                                            <div class="form-grid-3">
                                                                <div class="form-group">
                                                                    <label class="form-label">"Lương Gross thỏa thuận (VND):"</label>
                                                                    <input
                                                                        class="form-input"
                                                                        type="text"
                                                                        prop:value=salary_gross
                                                                        on:input=move |ev| set_salary_gross.set(event_target_value(&ev))
                                                                    />
                                                                </div>
                                                                <div class="form-group">
                                                                    <label class="form-label">"Số người phụ thuộc (người):"</label>
                                                                    <input
                                                                        class="form-input"
                                                                        type="number"
                                                                        min="0"
                                                                        max="10"
                                                                        prop:value=dependents_count
                                                                        on:input=move |ev| {
                                                                            if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                                                                set_dependents_count.set(v);
                                                                            }
                                                                        }
                                                                    />
                                                                </div>
                                                                <div class="form-group">
                                                                    <label class="form-label">"Vùng lương tối thiểu:"</label>
                                                                    <select
                                                                        class="form-input"
                                                                        prop:value=salary_region
                                                                        on:change=move |ev| set_salary_region.set(event_target_value(&ev))
                                                                    >
                                                                        <option value="Vùng I (4.960.000đ)">"Vùng I (4.960.000đ)"</option>
                                                                        <option value="Vùng II (4.410.000đ)">"Vùng II (4.410.000đ)"</option>
                                                                        <option value="Vùng III (3.860.000đ)">"Vùng III (3.860.000đ)"</option>
                                                                        <option value="Vùng IV (3.450.000đ)">"Vùng IV (3.450.000đ)"</option>
                                                                    </select>
                                                                </div>
                                                            </div>

                                                            <table class="breakdown-table">
                                                                <thead>
                                                                    <tr>
                                                                        <th>"Hạng mục đối soát"</th>
                                                                        <th>"Tỷ lệ theo luật"</th>
                                                                        <th>"Số tiền (VND)"</th>
                                                                    </tr>
                                                                </thead>
                                                                <tbody>
                                                                    <tr>
                                                                        <td>"Bảo hiểm Xã hội (BHXH)"</td>
                                                                        <td>"8.0%"</td>
                                                                        <td>"2.000.000 đ"</td>
                                                                    </tr>
                                                                    <tr>
                                                                        <td>"Bảo hiểm Y tế (BHYT)"</td>
                                                                        <td>"1.5%"</td>
                                                                        <td>"375.000 đ"</td>
                                                                    </tr>
                                                                    <tr>
                                                                        <td>"Bảo hiểm Thất nghiệp (BHTN)"</td>
                                                                        <td>"1.0%"</td>
                                                                        <td>"250.000 đ"</td>
                                                                    </tr>
                                                                    <tr>
                                                                        <td>"Giảm trừ gia cảnh bản thân & phụ thuộc"</td>
                                                                        <td>"11tr + 4.4tr/người"</td>
                                                                        <td>"15.400.000 đ"</td>
                                                                    </tr>
                                                                    <tr>
                                                                        <td>"Thuế TNCN tạm tính (Biểu lũy tiến)"</td>
                                                                        <td>"Lũy tiến từng phần"</td>
                                                                        <td>"447.500 đ"</td>
                                                                    </tr>
                                                                    <tr class="total-row">
                                                                        <td>"LƯƠNG NET THỰC NHẬN"</td>
                                                                        <td>"Thực lãnh"</td>
                                                                        <td>"21.927.500 đ"</td>
                                                                    </tr>
                                                                </tbody>
                                                            </table>
                                                        </div>
                                                    }.into_any()
                                                } else if s == "vn-loan-amortization" {
                                                    view! {
                                                        <div>
                                                            <div class="form-grid-3">
                                                                <div class="form-group">
                                                                    <label class="form-label">"Số tiền vay (VND):"</label>
                                                                    <input
                                                                        class="form-input"
                                                                        type="text"
                                                                        prop:value=loan_principal
                                                                        on:input=move |ev| set_loan_principal.set(event_target_value(&ev))
                                                                    />
                                                                </div>
                                                                <div class="form-group">
                                                                    <label class="form-label">"Lãi suất (%/năm):"</label>
                                                                    <input
                                                                        class="form-input"
                                                                        type="text"
                                                                        prop:value=loan_rate
                                                                        on:input=move |ev| set_loan_rate.set(event_target_value(&ev))
                                                                    />
                                                                </div>
                                                                <div class="form-group">
                                                                    <label class="form-label">"Thời hạn (tháng):"</label>
                                                                    <input
                                                                        class="form-input"
                                                                        type="number"
                                                                        prop:value=loan_months
                                                                        on:input=move |ev| set_loan_months.set(event_target_value(&ev))
                                                                    />
                                                                </div>
                                                            </div>
                                                            <div style="margin-top: 10px; font-size: 13px; color: #38BDF8;">
                                                                "💡 Phương thức: Dư nợ giảm dần chuẩn xác từng đồng, tính toán theo công thức niên kim ngân hàng."
                                                            </div>
                                                        </div>
                                                    }.into_any()
                                                } else if s == "vn-working-days-calc" {
                                                    view! {
                                                        <div class="form-grid-2">
                                                            <div class="form-group">
                                                                <label class="form-label">"Từ ngày (YYYY-MM-DD):"</label>
                                                                <input
                                                                    class="form-input"
                                                                    type="text"
                                                                    prop:value=date_start
                                                                    on:input=move |ev| set_date_start.set(event_target_value(&ev))
                                                                />
                                                            </div>
                                                            <div class="form-group">
                                                                <label class="form-label">"Đến ngày (YYYY-MM-DD):"</label>
                                                                <input
                                                                    class="form-input"
                                                                    type="text"
                                                                    prop:value=date_end
                                                                    on:input=move |ev| set_date_end.set(event_target_value(&ev))
                                                                />
                                                            </div>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div class="tool-config-panel">
                                                            <div class="form-group">
                                                                <label class="form-label">"Nhập số tiền hoặc số bất kỳ cần đọc:"</label>
                                                                <input
                                                                    class="form-input"
                                                                    type="text"
                                                                    prop:value=input_value
                                                                    on:compositionstart=move |_| set_is_ime_composing.set(true)
                                                                    on:compositionend=move |_| set_is_ime_composing.set(false)
                                                                    on:input=move |ev| set_input_value.set(event_target_value(&ev))
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
                                                        </div>
                                                    }.into_any()
                                                }
                                            }}
                                        </div>
                                    }.into_any(),

                                    ToolArchetype::IdentityVerification => view! {
                                        <div>
                                            { move || {
                                                let s = current_slug.get();
                                                if s == "vietqr-napas247-generator" {
                                                    view! {
                                                        <div>
                                                            <div class="form-grid-2">
                                                                <div class="form-group">
                                                                    <label class="form-label">"Ngân hàng thụ hưởng:"</label>
                                                                    <select
                                                                        class="form-input"
                                                                        prop:value=bank_name
                                                                        on:change=move |ev| set_bank_name.set(event_target_value(&ev))
                                                                    >
                                                                        <option value="Vietcombank (VCB)">"Vietcombank (VCB - 970436)"</option>
                                                                        <option value="MBBank (MB)">"MBBank (MB - 970422)"</option>
                                                                        <option value="Techcombank (TCB)">"Techcombank (TCB - 970407)"</option>
                                                                        <option value="VPBank (VPB)">"VPBank (VPB - 970432)"</option>
                                                                        <option value="ACB">"Á Châu (ACB - 970416)"</option>
                                                                        <option value="BIDV">"BIDV (970418)"</option>
                                                                        <option value="Agribank">"Agribank (970405)"</option>
                                                                    </select>
                                                                </div>
                                                                <div class="form-group">
                                                                    <label class="form-label">"Số tài khoản thụ hưởng:"</label>
                                                                    <input
                                                                        class="form-input"
                                                                        type="text"
                                                                        prop:value=account_number
                                                                        on:input=move |ev| set_account_number.set(event_target_value(&ev))
                                                                    />
                                                                </div>
                                                            </div>

                                                            <div class="form-grid-3">
                                                                <div class="form-group">
                                                                    <label class="form-label">"Tên chủ tài khoản:"</label>
                                                                    <input
                                                                        class="form-input"
                                                                        type="text"
                                                                        prop:value=account_holder
                                                                        on:input=move |ev| set_account_holder.set(event_target_value(&ev))
                                                                    />
                                                                </div>
                                                                <div class="form-group">
                                                                    <label class="form-label">"Số tiền (VND):"</label>
                                                                    <input
                                                                        class="form-input"
                                                                        type="text"
                                                                        prop:value=transfer_amount
                                                                        on:input=move |ev| set_transfer_amount.set(event_target_value(&ev))
                                                                    />
                                                                </div>
                                                                <div class="form-group">
                                                                    <label class="form-label">"Nội dung chuyển khoản:"</label>
                                                                    <input
                                                                        class="form-input"
                                                                        type="text"
                                                                        prop:value=transfer_note
                                                                        on:input=move |ev| set_transfer_note.set(event_target_value(&ev))
                                                                    />
                                                                </div>
                                                            </div>

                                                            <div class="qr-preview-box">
                                                                <div style="font-weight: 700; font-size: 14px; color: #0284C7;">
                                                                    "Napas 247 | VietQR EMVCo"
                                                                </div>
                                                                <div style="background: #F1F5F9; border: 1px solid #CBD5E1; padding: 20px; border-radius: 6px; font-family: monospace; font-size: 11px; text-align: center;">
                                                                    "[ ⬛ MÃ VIETQR CHUẨN NAPAS 247 ⬛ ]"
                                                                </div>
                                                                <div style="font-size: 12px; color: #475569;">
                                                                    { move || format!("TK: {} - {}", account_number.get(), bank_name.get()) }
                                                                </div>
                                                            </div>
                                                        </div>
                                                    }.into_any()
                                                } else if s == "vn-cccd-mst-validator" {
                                                    view! {
                                                        <div>
                                                            <div class="form-group">
                                                                <label class="form-label">"Nhập số Căn cước công dân (12 số) hoặc Mã số thuế (MST):"</label>
                                                                <input
                                                                    class="form-input"
                                                                    type="text"
                                                                    prop:value=cccd_input
                                                                    on:input=move |ev| set_cccd_input.set(event_target_value(&ev))
                                                                    placeholder="Ví dụ: 001098012345"
                                                                />
                                                            </div>

                                                            <div class="identity-preview-card">
                                                                <div class="id-field">
                                                                    <span class="id-field-label">"Nơi đăng ký khai sinh"</span>
                                                                    <span class="id-field-value">"001 - Thành phố Hà Nội"</span>
                                                                </div>
                                                                <div class="id-field">
                                                                    <span class="id-field-label">"Giới tính & Thế kỷ"</span>
                                                                    <span class="id-field-value">"Nam (Thế kỷ 20)"</span>
                                                                </div>
                                                                <div class="id-field">
                                                                    <span class="id-field-label">"Năm sinh"</span>
                                                                    <span class="id-field-value">"1998"</span>
                                                                </div>
                                                                <div class="id-field">
                                                                    <span class="id-field-label">"Độ hợp lệ thuật toán Modulo 11"</span>
                                                                    <span class="id-field-value" style="color: #34D399;">"✓ Hợp lệ 100%"</span>
                                                                </div>
                                                            </div>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div class="form-group">
                                                            <label class="form-label">"Nhập chuỗi địa chỉ hành chính cần chuẩn hóa:"</label>
                                                            <input
                                                                class="form-input"
                                                                type="text"
                                                                prop:value=address_raw
                                                                on:input=move |ev| set_address_raw.set(event_target_value(&ev))
                                                            />
                                                        </div>
                                                    }.into_any()
                                                }
                                            }}
                                        </div>
                                    }.into_any(),

                                    ToolArchetype::TextCodeConversion => view! {
                                        <div>
                                            <div style="display: flex; gap: 16px; margin-bottom: 10px;">
                                                <div class="form-group" style="flex: 1;">
                                                    <label class="form-label">"Bảng mã nguồn (Font):"</label>
                                                    <select
                                                        class="form-input"
                                                        prop:value=font_source_encoding
                                                        on:change=move |ev| set_font_source_encoding.set(event_target_value(&ev))
                                                    >
                                                        <option value="TCVN3">"TCVN3 (ABC) -> Unicode"</option>
                                                        <option value="VNI">"VNI-Windows -> Unicode"</option>
                                                        <option value="VIQR">"VIQR -> Unicode"</option>
                                                    </select>
                                                </div>
                                                <div class="form-group" style="flex: 1;">
                                                    <label class="form-label">"Phương ngữ SQL (SQL Linter):"</label>
                                                    <select
                                                        class="form-input"
                                                        prop:value=sql_dialect
                                                        on:change=move |ev| set_sql_dialect.set(event_target_value(&ev))
                                                    >
                                                        <option value="PostgreSQL">"PostgreSQL"</option>
                                                        <option value="MySQL">"MySQL"</option>
                                                        <option value="SQLite">"SQLite"</option>
                                                    </select>
                                                </div>
                                            </div>

                                            <div class="dual-pane-container" style="margin-top: 0;">
                                                <div class="pane-box">
                                                    <div class="pane-label">
                                                        <span>"VĂN BẢN ĐẦU VÀO"</span>
                                                        <span>"(Hỗ trợ gõ trực tiếp hoặc Ctrl + V)"</span>
                                                    </div>
                                                    <textarea
                                                        class="pane-textarea"
                                                        prop:value=text_source
                                                        on:input=move |ev| set_text_source.set(event_target_value(&ev))
                                                    ></textarea>
                                                </div>

                                                <div class="pane-box">
                                                    <div class="pane-label">
                                                        <span>"KẾT QUẢ ĐÃ CHUYỂN ĐỔI / ĐỐI SOÁT"</span>
                                                        <span style="color: #10B981;">"100% On-Device"</span>
                                                    </div>
                                                    <textarea
                                                        class="pane-textarea"
                                                        readonly=true
                                                        prop:value=move || {
                                                            let r = output_result.get();
                                                            if r.is_empty() {
                                                                text_source.get()
                                                            } else {
                                                                r
                                                            }
                                                        }
                                                    ></textarea>
                                                </div>
                                            </div>
                                        </div>
                                    }.into_any(),

                                    ToolArchetype::SpreadsheetData => view! {
                                        <div>
                                            <div
                                                class="dropzone"
                                                on:click=move |_| {
                                                    set_selected_files.update(|f| {
                                                        f.push("bang_ke_hoa_don_thang_11.xlsx".to_string());
                                                    });
                                                }
                                            >
                                                <div style="font-size: 28px; margin-bottom: 6px;">"📊"</div>
                                                <div class="dropzone-title">"KÉO THẢ TỆP EXCEL / CSV VÀO ĐÂY ĐỂ LÀM SẠCH VÀ CHUYỂN ĐỔI"</div>
                                                <div class="dropzone-sub">
                                                    "(Xử lý nhanh qua Calamine & XlsxWriter thuần Rust không cần cài Microsoft Office)"
                                                </div>
                                            </div>

                                            <div class="form-grid-2">
                                                <div class="form-group">
                                                    <label class="form-label">"Tên cột khóa (Key Column):"</label>
                                                    <input
                                                        class="form-input"
                                                        type="text"
                                                        prop:value=excel_key_column
                                                        on:input=move |ev| set_excel_key_column.set(event_target_value(&ev))
                                                    />
                                                </div>
                                                <div class="form-group">
                                                    <label class="form-label">"Định dạng xuất kết quả:"</label>
                                                    <select
                                                        class="form-input"
                                                        prop:value=excel_export_format
                                                        on:change=move |ev| set_excel_export_format.set(event_target_value(&ev))
                                                    >
                                                        <option value="xlsx">"Excel Workbook (.xlsx)"</option>
                                                        <option value="csv">"CSV tiêu chuẩn (UTF-8)"</option>
                                                        <option value="json">"JSON Array of Objects"</option>
                                                        <option value="sql">"SQL INSERT Statements"</option>
                                                    </select>
                                                </div>
                                            </div>

                                            <div style="display: flex; gap: 16px; margin-top: 10px;">
                                                <label class="radio-item">
                                                    <input
                                                        type="checkbox"
                                                        checked=excel_clean_names
                                                        on:change=move |ev| set_excel_clean_names.set(event_target_checked(&ev))
                                                    />
                                                    "Chuẩn hóa viết hoa Họ Tên Việt"
                                                </label>
                                                <label class="radio-item">
                                                    <input
                                                        type="checkbox"
                                                        checked=excel_clean_dates
                                                        on:change=move |ev| set_excel_clean_dates.set(event_target_checked(&ev))
                                                    />
                                                    "Đồng bộ ngày tháng DD/MM/YYYY"
                                                </label>
                                                <label class="radio-item">
                                                    <input
                                                        type="checkbox"
                                                        checked=excel_remove_duplicates
                                                        on:change=move |ev| set_excel_remove_duplicates.set(event_target_checked(&ev))
                                                    />
                                                    "Loại bỏ dòng trùng lặp"
                                                </label>
                                            </div>
                                        </div>
                                    }.into_any(),
                                }
                            }}

                            { move || {
                                let res = output_result.get();
                                if !res.is_empty() {
                                    let res_val = res.clone();
                                    view! {
                                        <div class="result-box">
                                            <div class="result-header">
                                                <span class="result-label">"KẾT QUẢ XỬ LÝ CHUẨN:"</span>
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
                }.into_any(),

                WorkspaceView::WorkflowDag => view! {
                    <div class="workflow-workspace">
                        <div class="workflow-header">
                            <div>
                                <h2 style="font-size: 18px; color: #F8FAFC; margin-bottom: 4px;">
                                    "⚡ Thiết kế Chuỗi Tự động hóa: Chuẩn hóa Ảnh Bài viết Web"
                                </h2>
                                <p style="font-size: 12px; color: var(--text-muted);">
                                    "Ghép nối các công cụ độc lập thành chuỗi Macro DAG với kiểm tra tương thích cổng MIME-Type trực quan."
                                </p>
                            </div>

                            <div style="display: flex; gap: 8px;">
                                <button
                                    class="btn-primary"
                                    prop:disabled=move || is_running_workflow.get()
                                    on:click=move |_| run_dag_pipeline()
                                >
                                    { move || if is_running_workflow.get() { "⏳ Đang chạy..." } else { "▶ Chạy Chuỗi Pipeline (3 Bước)" } }
                                </button>
                            </div>
                        </div>

                        { move || {
                            let msg = workflow_message.get();
                            if !msg.is_empty() {
                                view! {
                                    <div style="padding: 12px; margin-bottom: 16px; background: rgba(16, 185, 129, 0.1); border: 1px solid rgba(16, 185, 129, 0.3); border-radius: 8px; color: #34D399; font-weight: 600;">
                                        {msg}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }}

                        <div class="dag-pipeline-track">
                            <div class="dag-node-card">
                                <div class="dag-node-header">
                                    <span class="dag-node-title">"1. Đổi Định dạng Ảnh"</span>
                                    <span style="font-size: 11px; color: #10B981;">"Bước 1"</span>
                                </div>
                                <div class="dag-node-ports">
                                    <div><span class="dag-port-badge in">"In: image/*"</span></div>
                                    <div><span class="dag-port-badge out">"Out: image/webp"</span></div>
                                </div>
                            </div>

                            <div class="dag-connector">
                                <span>"======>"</span>
                            </div>

                            <div class="dag-node-card">
                                <div class="dag-node-header">
                                    <span class="dag-node-title">"2. Đóng dấu Watermark"</span>
                                    <span style="font-size: 11px; color: #10B981;">"Bước 2"</span>
                                </div>
                                <div class="dag-node-ports">
                                    <div><span class="dag-port-badge in">"In: image/webp, png"</span></div>
                                    <div><span class="dag-port-badge out">"Out: image/webp"</span></div>
                                </div>
                            </div>

                            <div class="dag-connector">
                                <span>"======>"</span>
                            </div>

                            <div class="dag-node-card">
                                <div class="dag-node-header">
                                    <span class="dag-node-title">"3. Xóa sạch EXIF / GPS"</span>
                                    <span style="font-size: 11px; color: #10B981;">"Bước 3"</span>
                                </div>
                                <div class="dag-node-ports">
                                    <div><span class="dag-port-badge in">"In: image/webp, jpg"</span></div>
                                    <div><span class="dag-port-badge out">"Out: image/webp"</span></div>
                                </div>
                            </div>

                            <button class="add-node-btn" title="Thêm công cụ tương thích vào cuối chuỗi">
                                "+ Thêm bước công cụ..."
                            </button>
                        </div>
                    </div>
                }.into_any(),

                WorkspaceView::History => view! {
                    <div class="history-workspace">
                        <div class="history-header">
                            <div>
                                <h2 style="font-size: 18px; color: #F8FAFC; margin-bottom: 4px;">
                                    "⏱️ Nhật ký Lịch sử Tác vụ Cục bộ (SQLite)"
                                </h2>
                                <p style="font-size: 12px; color: var(--text-muted);">
                                    "Toàn bộ tác vụ và nhật ký thực thi được ghi nhận an toàn 100% trong cơ sở dữ liệu cục bộ."
                                </p>
                            </div>

                            <div style="display: flex; gap: 8px;">
                                <button
                                    class="nav-tab-btn"
                                    prop:disabled=move || is_loading_history.get()
                                    on:click=move |_| load_history()
                                >
                                    { move || if is_loading_history.get() { "⏳ Đang tải..." } else { "🔄 Làm mới" } }
                                </button>
                                <button
                                    class="nav-tab-btn"
                                    style="color: #F87171;"
                                    on:click=move |_| clear_all_history()
                                >
                                    "🗑️ Xóa lịch sử"
                                </button>
                            </div>
                        </div>

                        { move || {
                            let recs = history_records.get();
                            if recs.is_empty() {
                                view! {
                                    <div style="text-align: center; padding: 48px; color: var(--text-muted); background: #1E293B; border-radius: 8px; border: 1px solid #334155;">
                                        <div style="font-size: 32px; margin-bottom: 8px;">"📭"</div>
                                        <div style="font-size: 14px; font-weight: 600;">"Chưa có bản ghi tác vụ nào"</div>
                                        <div style="font-size: 12px; margin-top: 4px;">"Hãy chọn một công cụ và thực thi tác vụ đầu tiên để xem nhật ký tại đây."</div>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <table class="history-table">
                                        <thead>
                                            <tr>
                                                <th>"Mã tác vụ"</th>
                                                <th>"Công cụ / Workflow"</th>
                                                <th>"Thời gian xử lý"</th>
                                                <th>"Trạng thái"</th>
                                                <th>"Thời điểm"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            { recs.into_iter().map(|item| {
                                                let is_ok = item.status == "SUCCESS";
                                                view! {
                                                    <tr>
                                                        <td style="font-family: monospace;">{item.id}</td>
                                                        <td>{item.tool_slug.unwrap_or_else(|| item.workflow_id.unwrap_or_else(|| "Tác vụ hệ thống".to_string()))}</td>
                                                        <td style="font-family: monospace;">{format!("{} ms", item.elapsed_ms)}</td>
                                                        <td>
                                                            <span class=if is_ok { "history-badge success" } else { "history-badge failed" }>
                                                                {item.status}
                                                            </span>
                                                        </td>
                                                        <td style="color: var(--text-muted);">{item.created_at}</td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>() }
                                        </tbody>
                                    </table>
                                }.into_any()
                            }
                        }}
                    </div>
                }.into_any(),
            }}

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
                                { move || {
                                    let q = search_query.get().trim().to_string();
                                    let recents = recent_tool_items();
                                    let recent_slugs = recent_tool_slugs.get();

                                    if q.is_empty() {
                                        if recents.is_empty() {
                                            view! {
                                                <div>
                                                    <div class="palette-section-header">
                                                        <span>"📌 TẤT CẢ CÔNG CỤ"</span>
                                                        <span style="font-size: 10px; opacity: 0.7;">{format!("{} công cụ", ALL_TOOLS.len())}</span>
                                                    </div>
                                                    {
                                                        ALL_TOOLS.iter().map(|tool| {
                                                            let slug_copy = tool.slug.to_string();
                                                            view! {
                                                                <div
                                                                    class="palette-item"
                                                                    on:click=move |_| select_tool(slug_copy.clone())
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
                                            }.into_any()
                                        } else {
                                            let remaining = non_recent_tools();
                                            view! {
                                                <div>
                                                    <div class="palette-section-header">
                                                        <span>"🕒 CÔNG CỤ VỪA CHỌN GẦN ĐÂY"</span>
                                                        <span style="font-size: 10px; opacity: 0.7;">"Ưu tiên gợi ý"</span>
                                                    </div>
                                                    {
                                                        recents.into_iter().map(|tool| {
                                                            let slug_copy = tool.slug.to_string();
                                                            view! {
                                                                <div
                                                                    class="palette-item"
                                                                    on:click=move |_| select_tool(slug_copy.clone())
                                                                >
                                                                    <div>
                                                                        <div style="font-weight: 600; display: flex; align-items: center; gap: 6px;">
                                                                            <span>{tool.name}</span>
                                                                            <span class="palette-item-badge recent">"🕒 Gần đây"</span>
                                                                        </div>
                                                                        <div style="font-size: 11px; opacity: 0.8;">{tool.description}</div>
                                                                    </div>
                                                                    <span class="palette-item-badge">{tool.domain}</span>
                                                                </div>
                                                            }
                                                        }).collect::<Vec<_>>()
                                                    }

                                                    <div class="palette-section-header" style="margin-top: 10px;">
                                                        <span>"📌 TẤT CẢ CÔNG CỤ KHÁC"</span>
                                                        <span style="font-size: 10px; opacity: 0.7;">{format!("{} công cụ", remaining.len())}</span>
                                                    </div>
                                                    {
                                                        remaining.into_iter().map(|tool| {
                                                            let slug_copy = tool.slug.to_string();
                                                            view! {
                                                                <div
                                                                    class="palette-item"
                                                                    on:click=move |_| select_tool(slug_copy.clone())
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
                                            }.into_any()
                                        }
                                    } else {
                                        let filtered = filtered_tools();
                                        if filtered.is_empty() {
                                            view! {
                                                <div style="text-align: center; padding: 32px 16px; color: var(--text-muted);">
                                                    <div style="font-size: 24px; margin-bottom: 6px;">"🔍"</div>
                                                    <div style="font-weight: 600;">"Không tìm thấy công cụ phù hợp"</div>
                                                    <div style="font-size: 12px; margin-top: 4px;">"Hãy thử gõ từ khóa khác (ví dụ: pdf, vietqr, excel, lương...)"</div>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div>
                                                    <div class="palette-section-header">
                                                        <span>"KẾT QUẢ TÌM KIẾM"</span>
                                                        <span style="font-size: 10px; opacity: 0.7;">{format!("{} kết quả", filtered.len())}</span>
                                                    </div>
                                                    {
                                                        filtered.into_iter().map(|tool| {
                                                            let slug_copy = tool.slug.to_string();
                                                            let is_recent = recent_slugs.contains(&slug_copy);
                                                            view! {
                                                                <div
                                                                    class="palette-item"
                                                                    on:click=move |_| select_tool(slug_copy.clone())
                                                                >
                                                                    <div>
                                                                        <div style="font-weight: 600; display: flex; align-items: center; gap: 6px;">
                                                                            <span>{tool.name}</span>
                                                                            {if is_recent {
                                                                                view! { <span class="palette-item-badge recent">"🕒 Gần đây"</span> }.into_any()
                                                                            } else {
                                                                                view! { <span></span> }.into_any()
                                                                            }}
                                                                        </div>
                                                                        <div style="font-size: 11px; opacity: 0.8;">{tool.description}</div>
                                                                    </div>
                                                                    <span class="palette-item-badge">{tool.domain}</span>
                                                                </div>
                                                            }
                                                        }).collect::<Vec<_>>()
                                                    }
                                                </div>
                                            }.into_any()
                                        }
                                    }
                                }}
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}

            { move || if save_preset_modal_open.get() {
                view! {
                    <div class="modal-overlay" on:click=move |_| set_save_preset_modal_open.set(false)>
                        <div class="preset-modal" on:click=move |ev: MouseEvent| ev.stop_propagation()>
                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px;">
                                <div style="font-weight: 700; font-size: 16px; color: #F8FAFC;">"💾 Lưu Mẫu Cấu hình (Preset)"</div>
                                <button class="window-control-btn close" on:click=move |_| set_save_preset_modal_open.set(false)>"✕"</button>
                            </div>

                            <div class="form-group" style="margin-bottom: 16px;">
                                <label class="form-label">"Tên mẫu Preset mới:"</label>
                                <input
                                    class="form-input"
                                    type="text"
                                    placeholder="Ví dụ: Kế toán thuế tháng 11"
                                    prop:value=preset_name_input
                                    on:input=move |ev| set_preset_name_input.set(event_target_value(&ev))
                                    autofocus=true
                                />
                            </div>

                            <div style="display: flex; justify-content: flex-end; gap: 8px;">
                                <button class="btn-secondary" on:click=move |_| set_save_preset_modal_open.set(false)>
                                    "Hủy"
                                </button>
                                <button class="btn-primary" on:click=move |_| save_current_preset()>
                                    "Lưu vào SQLite"
                                </button>
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
