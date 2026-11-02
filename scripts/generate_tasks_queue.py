#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Script phân rã yêu cầu từ 02-srs.md và sinh hàng đợi công việc tasks_queue.jsonl
cho hệ sinh thái Office Utility VN.

Phân nhóm theo 5 miền:
  1. doc-suite
  2. media-studio
  3. data-lab
  4. vn-office
  5. sys-hub

Sắp xếp theo độ ưu tiên MoSCoW:
  Must-have -> Should-have -> Could-have

Mỗi payload là một Atomic Task Payload chứa đầy đủ ngữ cảnh để Subagent tự viết:
  - Unit Test (cargo test -p office-engine <slug>)
  - Mã nguồn Rust Engine triển khai trait OfficeTool
  - Giao diện Leptos 0.8 CSR Vùng 2 (Zone 2)
mà không cần đọc lại 7 file tài liệu đặc tả.
"""

import os
import sys
import json
import re
import argparse
from pathlib import Path

# Mapping MoSCoW & Roadmap Phase theo 01-introduction.md & 02-srs.md
PRIORITY_MAP = {
    # Phase 1: Core Foundation & MVP (20 công cụ Must-have)
    "FR-DOC-01": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-DOC-02": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-DOC-04": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-DOC-06": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-DOC-08": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-MED-01": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-MED-04": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-MED-05": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-DAT-01": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-DAT-02": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-DAT-04": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-DAT-08": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-DAT-13": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-VNO-01": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-VNO-02": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-VNO-03": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-VNO-04": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-VNO-05": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-VNO-07": ("Must-have", "Phase 1: Core Foundation & MVP", 1),
    "FR-VNO-09": ("Must-have", "Phase 1: Core Foundation & MVP", 1),

    # Phase 2: Advanced Suite & Workflow DAG (23 công cụ Should-have)
    "FR-DOC-03": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DOC-07": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DOC-09": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DOC-10": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DOC-11": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DOC-12": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DOC-13": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DOC-14": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DOC-15": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DOC-16": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DOC-17": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DOC-18": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DOC-19": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DAT-03": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DAT-05": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DAT-06": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DAT-07": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DAT-09": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DAT-10": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DAT-11": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-DAT-12": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-VNO-06": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-VNO-08": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),
    "FR-HUB-01": ("Should-have", "Phase 2: Advanced Suite & Workflow DAG", 2),

    # Phase 3: Offline AI, Media Pro & Diagnostics (15 công cụ Could-have)
    "FR-DOC-05": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-MED-02": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-MED-03": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-MED-06": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-MED-07": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-MED-08": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-MED-09": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-SYS-01": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-SYS-02": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-SYS-03": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-UTL-01": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-UTL-02": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-UTL-03": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
    "FR-UTL-04": ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3),
}

DOMAIN_MAP = {
    "DOC": ("doc-suite", "DOCUMENT", 1),
    "MED": ("media-studio", "MEDIA", 2),
    "DAT": ("data-lab", "DATA", 3),
    "VNO": ("vn-office", "VN_OFFICE", 4),
    "SYS": ("sys-hub", "SYSTEM", 5),
    "HUB": ("sys-hub", "HUB", 5),
    "UTL": ("sys-hub", "UTILITY", 5),
}

USER_STORY_MAP = {
    "FR-DOC-01": ("US-02", "Bảo mật On-Device & Ghép/Tách PDF"),
    "FR-DOC-02": ("US-02", "Bảo mật On-Device & Mã hóa/Mở khóa"),
    "FR-DOC-03": ("US-03", "Chuyển đổi PDF sang Markdown định dạng chuẩn"),
    "FR-DOC-04": ("US-03", "Nén giảm dung lượng PDF không mất nét"),
    "FR-DOC-05": ("US-10", "OCR Ngoại tuyến Linh hoạt (OS Native + ONNX)"),
    "FR-DOC-06": ("US-02", "Đóng dấu Watermark & Bản quyền PDF"),
    "FR-DOC-07": ("US-02", "Đánh số trang & Bates Numbering hồ sơ"),
    "FR-DOC-08": ("US-02", "Tẩy sạch Metadata ẩn bảo mật PDF"),
    "FR-DOC-09": ("US-02", "Làm phẳng biểu mẫu AcroForms PDF"),
    "FR-DOC-10": ("US-03", "Cắt viền trắng thừa trang tài liệu PDF"),
    "FR-DOC-11": ("US-03", "Dàn trang in sách gấp đôi & N-up"),
    "FR-DOC-12": ("US-02", "So sánh đối soát hai bản PDF"),
    "FR-DOC-13": ("US-03", "Trộn hợp đồng Mailmerge hàng loạt cực tốc"),
    "FR-DOC-14": ("US-02", "Tạo cây mục lục Bookmark TOC đa cấp"),
    "FR-DOC-15": ("US-03", "Ghép các trang PDF thành ảnh dài duy nhất"),
    "FR-DOC-16": ("US-03", "Chuyển đổi PDF sang đen trắng tiết kiệm mực"),
    "FR-DOC-17": ("US-02", "Phục hồi tệp PDF bị lỗi chỉ mục xref"),
    "FR-DOC-18": ("US-03", "Biên dịch Markdown sang PDF chuẩn A4"),
    "FR-DOC-19": ("US-02", "Trích xuất bảng biểu từ PDF sang Excel"),

    "FR-MED-01": ("US-03", "Nén & Đổi định dạng ảnh hàng loạt đa luồng"),
    "FR-MED-02": ("US-05", "Cắt ảnh thẻ chuẩn 300 DPI và dàn trang in"),
    "FR-MED-03": ("US-02", "Làm mờ dữ liệu nhạy cảm giấy tờ tùy thân"),
    "FR-MED-04": ("US-02", "Kiểm tra và xóa sạch Exif/GPS ảnh"),
    "FR-MED-05": ("US-03", "Đóng dấu bản quyền ảnh hàng loạt"),
    "FR-MED-06": ("US-05", "Làm đẹp ảnh chụp màn hình đăng mạng xã hội"),
    "FR-MED-07": ("US-05", "Phẳng hóa tài liệu chụp nghiêng 4 góc"),
    "FR-MED-08": ("US-03", "Cắt bỏ khoảng lặng file ghi âm cuộc họp"),
    "FR-MED-09": ("US-03", "Chuyển video ngắn sang GIF/WebP động"),

    "FR-DAT-01": ("US-04", "Chuyển mã Font TCVN3/VNI sang Unicode"),
    "FR-DAT-02": ("US-04", "Làm sạch & Chuẩn hóa bảng tính Excel"),
    "FR-DAT-03": ("US-02", "Đối chiếu sai khác văn bản và bảng tính"),
    "FR-DAT-04": ("US-03", "Chuyển đổi dữ liệu Excel/CSV sang JSON/SQL/MD"),
    "FR-DAT-05": ("US-03", "Tách bảng tính Excel lớn theo cột chỉ định"),
    "FR-DAT-06": ("US-03", "Gộp nhiều file Excel / Sheet vào một Workbook"),
    "FR-DAT-07": ("US-03", "Nối trực quan hai bảng tính (Visual Join)"),
    "FR-DAT-08": ("US-02", "Gỡ khóa bảo vệ chỉnh sửa Sheet Excel"),
    "FR-DAT-09": ("US-03", "Phép toán tập hợp danh sách khách hàng"),
    "FR-DAT-10": ("US-04", "Sửa lỗi hiển thị chữ tiếng Việt Mojibake"),
    "FR-DAT-11": ("US-02", "Ẩn danh hóa dữ liệu nhạy cảm PII"),
    "FR-DAT-12": ("US-03", "Định dạng và lint câu lệnh truy vấn SQL"),
    "FR-DAT-13": ("US-03", "Mã hóa và giải mã Base64 cho tệp tin"),

    "FR-VNO-01": ("US-04", "Đọc số tiền thành chữ tiếng Việt chuẩn kế toán"),
    "FR-VNO-02": ("US-04", "Tính lương Gross sang Net & Biểu thuế TNCN"),
    "FR-VNO-03": ("US-04", "Tạo mã VietQR chuẩn Napas247 EMVCo"),
    "FR-VNO-04": ("US-04", "Đọc file XML hóa đơn điện tử Thông tư 78"),
    "FR-VNO-05": ("US-04", "Kiểm tra tính hợp lệ số CCCD và Mã số thuế"),
    "FR-VNO-06": ("US-04", "Chuẩn hóa địa giới hành chính VN 2 cấp và 3 cấp"),
    "FR-VNO-07": ("US-04", "Tính số ngày làm việc trừ lịch Âm và ngày lễ VN"),
    "FR-VNO-08": ("US-04", "Bảng tính lịch trả nợ vay ngân hàng chi tiết"),
    "FR-VNO-09": ("US-04", "Tra cứu mã định danh ngân hàng Napas BIN"),

    "FR-SYS-01": ("US-11", "Kiểm tra bàn phím kẹt phím & chuột double-click"),
    "FR-SYS-02": ("US-11", "Kiểm tra điểm chết màn hình, camera & micro"),
    "FR-SYS-03": ("US-11", "Thông số phần cứng CPU, RAM, Disk của hệ thống"),
    "FR-HUB-01": ("US-09", "Trạm chuyển đổi tệp vạn năng dựa trên Magic Bytes"),
    "FR-UTL-01": ("US-12", "Bộ tạo liên kết UTM tiếp thị chiến dịch"),
    "FR-UTL-02": ("US-05", "Đổi font chữ Unicode nghệ thuật đăng mạng xã hội"),
    "FR-UTL-03": ("US-02", "Quay số ngẫu nhiên minh bạch với mã băm SHA-256"),
    "FR-UTL-04": ("US-04", "Tính điểm hòa vốn và lãi ròng bán hàng TMĐT"),
}

# Chi tiết bổ sung cho từng tool để đóng gói Atomic Task Payload hoàn chỉnh
TOOL_DETAILS = {
    "FR-DOC-01": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfMergeSplitOptions { pub mode: MergeSplitMode, pub page_ranges: Option<String>, pub rotation_deg: Option<i32> }",
        "zone2_controls": [
            {"id": "mode", "type": "SegmentedControl", "label": "Chế độ", "options": ["Ghép tệp (Merge)", "Tách trang (Split)", "Xoay trang (Rotate)"]},
            {"id": "page_ranges", "type": "ImeSafeTextInput", "label": "Dải trang", "placeholder": "VD: 1-3, 5, 8-10"},
            {"id": "rotation_deg", "type": "Select", "label": "Góc xoay", "options": ["90°", "180°", "270°"]}
        ],
        "test_scenarios": [
            "Ghép 2 file PDF hợp lệ và xác thực tổng số trang đúng",
            "Tách trang theo dải '1-3, 5' và kiểm tra số trang file kết quả",
            "Nạp file không phải PDF -> trả về CommandError::InvalidFileSignature",
            "Bật cancel_flag giữa chừng -> dừng trong < 100ms trả về CommandError::TaskCancelled"
        ]
    },
    "FR-DOC-02": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfProtectOptions { pub action: ProtectAction, pub user_password: Option<zeroize::Zeroizing<String>>, pub owner_password: Option<zeroize::Zeroizing<String>>, pub encryption: String }",
        "zone2_controls": [
            {"id": "action", "type": "RadioGroup", "label": "Hành động", "options": ["Đặt mật khẩu bảo vệ", "Gỡ bỏ mật khẩu"]},
            {"id": "user_password", "type": "ImeSafeTextInput", "label": "Mật khẩu người xem (User Password)", "is_password": True},
            {"id": "encryption", "type": "Select", "label": "Tiêu chuẩn mã hóa", "options": ["AES-256 (Khuyến nghị)", "AES-128"]}
        ],
        "test_scenarios": [
            "Mã hóa PDF bằng AES-256 và xác nhận không mở được nếu thiếu mật khẩu",
            "Mở khóa PDF có mật khẩu khi cung cấp đúng mật khẩu",
            "Thử mở khóa bằng mật khẩu sai -> trả về CommandError::ValidationFailed",
            "Kiểm tra bộ nhớ RAM được tẩy sạch mật khẩu bằng zeroize"
        ]
    },
    "FR-DOC-03": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["text/markdown"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfToMarkdownOptions { pub extract_tables: bool, pub heading_scale: f32 }",
        "zone2_controls": [
            {"id": "extract_tables", "type": "Checkbox", "label": "Tự động nhận diện và tái tạo bảng Markdown"},
            {"id": "heading_scale", "type": "Slider", "label": "Ngưỡng kích thước font nhận dạng tiêu đề", "min": 1.2, "max": 2.5}
        ],
        "test_scenarios": [
            "Trích xuất luồng văn bản PDF và tái tạo đúng định dạng Markdown tiêu đề #, ##",
            "Nhận diện bảng biểu trong PDF và xuất cú pháp bảng kẻ Markdown chuẩn",
            "Kiểm tra file PDF rỗng hoặc hỏng"
        ]
    },
    "FR-DOC-04": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfCompressOptions { pub target_dpi: u32, pub compression_level: u32, pub strip_unreferenced: bool }",
        "zone2_controls": [
            {"id": "target_dpi", "type": "RadioGroup", "label": "Mức nén ảnh nhúng", "options": ["72 DPI (Chia sẻ Web/Email siêu nhẹ)", "150 DPI (Đọc màn hình sắc nét)", "300 DPI (In ấn tiêu chuẩn)"]},
            {"id": "strip_unreferenced", "type": "Checkbox", "label": "Loại bỏ đối tượng mồ côi (Unreferenced Objects)"}
        ],
        "test_scenarios": [
            "Nén file PDF chứa nhiều ảnh lớn, kiểm tra dung lượng giảm tối thiểu 40%",
            "Xác thực luồng FlateDecode được tái nén tối đa",
            "Kiểm tra kiểm soát trần bộ nhớ RAM < 250 MB"
        ]
    },
    "FR-DOC-05": {
        "input_mimes": ["application/pdf", "image/jpeg", "image/png"],
        "output_mimes": ["text/plain", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "text/markdown"],
        "magic_bytes": {"hex": "25 50 44 46 / FF D8 FF / 89 50 4E 47", "ascii": "Multi", "offset": 0},
        "options_struct": "VietnameseOcrOptions { pub engine: OcrEngineMode, pub output_format: OcrOutputFormat }",
        "zone2_controls": [
            {"id": "engine", "type": "RadioGroup", "label": "Động cơ OCR", "options": ["OS Native OCR (0 MB, cực tốc)", "VietOCR On-Demand ONNX (~12 MB, chính xác cao)"]},
            {"id": "output_format", "type": "Select", "label": "Định dạng xuất", "options": [".txt (Văn bản thuần)", ".docx (Microsoft Word)", ".md (Markdown)"]}
        ],
        "test_scenarios": [
            "OCR văn bản tiếng Việt có dấu bằng động cơ nhanh OS",
            "Kiểm tra cơ chế tải và xác thực SHA-256 gói mở rộng ONNX nếu chọn chế độ Accurate",
            "Bóc tách đúng dấu thanh tiếng Việt không lỗi font"
        ]
    },
    "FR-DOC-06": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfWatermarkOptions { pub text: Option<String>, pub stamp_image_path: Option<String>, pub opacity: f32, pub angle: f32, pub position: String }",
        "zone2_controls": [
            {"id": "text", "type": "ImeSafeTextInput", "label": "Nội dung chữ bản quyền", "placeholder": "BẢN QUYỀN - KHÔNG SAO CHÉP"},
            {"id": "opacity", "type": "Slider", "label": "Độ trong suốt (Opacity)", "min": 0.1, "max": 1.0, "step": 0.05},
            {"id": "angle", "type": "Slider", "label": "Góc xoay", "min": 0, "max": 90, "default": 45}
        ],
        "test_scenarios": [
            "Chèn chữ watermark xoay chéo với độ trong suốt ExtGState hợp lệ",
            "Nhúng con dấu ảnh PNG có kênh Alpha vào góc dưới trang",
            "Kiểm tra không làm mất nội dung text gốc bên dưới"
        ]
    },
    "FR-DOC-07": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfPageNumberOptions { pub pattern: String, pub position: String, pub skip_cover: bool, pub start_num: u32 }",
        "zone2_controls": [
            {"id": "pattern", "type": "ImeSafeTextInput", "label": "Mẫu số trang", "placeholder": "Trang {page}/{total} hoặc Bates {number:06}"},
            {"id": "position", "type": "Select", "label": "Vị trí", "options": ["Góc dưới giữa", "Góc dưới phải", "Góc trên phải", "Góc dưới trái"]},
            {"id": "skip_cover", "type": "Checkbox", "label": "Bỏ qua trang bìa đầu tiên"}
        ],
        "test_scenarios": [
            "Đánh số trang dạng 'Trang 1/10' bỏ qua trang bìa",
            "Đánh số Bates Numbering 'Bates 000001' cho hồ sơ pháp lý",
            "Nhúng subset font tiếng Việt không gây vỡ ký tự Unicode"
        ]
    },
    "FR-DOC-08": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfScrubberOptions { pub remove_info_dict: bool, pub remove_xmp: bool }",
        "zone2_controls": [
            {"id": "remove_info_dict", "type": "Checkbox", "label": "Xóa thông tin Tác giả, Phần mềm, Ngày tạo (Info Dictionary)"},
            {"id": "remove_xmp", "type": "Checkbox", "label": "Gỡ bỏ luồng XML Metadata nhạy cảm (XMP Metadata)"}
        ],
        "test_scenarios": [
            "Xóa sạch Trailer Info dictionary và XMP stream trong Catalog",
            "Kiểm tra file sau khi xóa không còn chứa tên tác giả hoặc lịch sử phần mềm tạo",
            "Mở file trên trình đọc PDF chuẩn không báo lỗi cấu trúc"
        ]
    },
    "FR-DOC-09": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfFlattenOptions { pub remove_interactive_forms: bool }",
        "zone2_controls": [
            {"id": "remove_interactive_forms", "type": "Checkbox", "label": "Làm phẳng AcroForms và xóa Widget Annotations chống chỉnh sửa"}
        ],
        "test_scenarios": [
            "Hợp nhất Appearance Stream của Form vào nội dung tĩnh",
            "Xác nhận các ô nhập liệu sau khi làm phẳng không còn con trỏ gõ chữ",
            "Kiểm tra bảo toàn các ký tự tiếng Việt đã điền trong form"
        ]
    },
    "FR-DOC-10": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfCropOptions { pub margin_padding_pt: f32, pub uniform_all_pages: bool }",
        "zone2_controls": [
            {"id": "margin_padding_pt", "type": "Slider", "label": "Khoảng đệm an toàn sau khi cắt (pt)", "min": 0, "max": 36, "default": 10},
            {"id": "uniform_all_pages", "type": "Checkbox", "label": "Áp dụng đồng bộ khung cắt lớn nhất cho toàn bộ trang"}
        ],
        "test_scenarios": [
            "Quét tọa độ Bounding Box nội dung thực tế qua kết xuất nhanh",
            "Cập nhật lại CropBox và MediaBox chính xác",
            "Kiểm tra cắt trang sách scan có viền đen/trắng thừa"
        ]
    },
    "FR-DOC-11": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfImpositionOptions { pub mode: ImpositionMode, pub paper_size: String }",
        "zone2_controls": [
            {"id": "mode", "type": "Select", "label": "Kiểu dàn trang", "options": ["Đóng quyển Saddle-Stitch (A4 -> A5)", "Ghép 2 trang trên 1 mặt (2-up)", "Ghép 4 trang trên 1 mặt (4-up)"]},
            {"id": "paper_size", "type": "Select", "label": "Khổ giấy in", "options": ["A4", "A3"]}
        ],
        "test_scenarios": [
            "Tính toán ma trận biến đổi cm ghép 2-up trên 1 mặt giấy A4",
            "Sắp xếp thứ tự trang theo thuật toán đóng sách lồng Saddle-Stitch",
            "Kiểm tra số trang không chia hết cho 4 tự động thêm trang trắng"
        ]
    },
    "FR-DOC-12": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf", "image/png"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfDiffOptions { pub mode: DiffMode, pub highlight_color: String }",
        "zone2_controls": [
            {"id": "mode", "type": "RadioGroup", "label": "Chế độ đối soát", "options": ["Text Diff (So sánh câu chữ nhanh)", "Visual Pixel Diff (So sánh hình ảnh từng pixel)"]},
            {"id": "highlight_color", "type": "Select", "label": "Màu tô nổi bật sai khác", "options": ["Đỏ cảnh báo (#rose-600)", "Vàng cam (#amber-500)"]}
        ],
        "test_scenarios": [
            "So sánh hai hợp đồng PDF có sửa một số từ ngữ -> báo cáo vị trí khác biệt",
            "Render bitmap so sánh từng cặp trang và tô đỏ vùng pixel thay đổi",
            "Xác nhận xử lý 2 file có số lượng trang khác nhau"
        ]
    },
    "FR-DOC-13": {
        "input_mimes": ["application/vnd.openxmlformats-officedocument.wordprocessingml.document", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "output_mimes": ["application/zip", "inode/directory"],
        "magic_bytes": {"hex": "50 4B 03 04", "ascii": "PK..", "offset": 0},
        "options_struct": "MailmergeOptions { pub template_path: String, pub data_excel_path: String, pub output_filename_pattern: String }",
        "zone2_controls": [
            {"id": "template_path", "type": "FilePicker", "label": "Tệp mẫu Word (.docx)"},
            {"id": "data_excel_path", "type": "FilePicker", "label": "Bảng tính danh sách Excel (.xlsx)"},
            {"id": "output_filename_pattern", "type": "ImeSafeTextInput", "label": "Quy tắc đặt tên file", "placeholder": "HopDong_{{ho_ten}}_{{ma_nv}}"}
        ],
        "test_scenarios": [
            "Thuật toán XML Run Normalization hợp nhất các thẻ <w:r> bị Word cắt vụn",
            "Trộn 1.000 dòng Excel vào file Word mẫu với tốc độ > 1.000 file/giây",
            "Kiểm tra không bị lỗi corrupt file docx khi placeholder chứa ký tự đặc biệt tiếng Việt"
        ]
    },
    "FR-DOC-14": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfBookmarkOptions { pub bookmarks_json: String }",
        "zone2_controls": [
            {"id": "bookmarks_input", "type": "TextArea", "label": "Danh sách mục lục", "placeholder": "1. Mở đầu - Trang 1\\n  1.1. Bối cảnh - Trang 2\\n2. Nội dung chính - Trang 5"}
        ],
        "test_scenarios": [
            "Xây dựng cây từ điển /Outlines đa cấp trong PDF Catalog",
            "Liên kết từng nút bookmark đến đúng Page Object ID chỉ định",
            "Kiểm tra bookmark tiếng Việt Unicode hiển thị chuẩn trong Adobe Reader"
        ]
    },
    "FR-DOC-15": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["image/jpeg", "image/png", "image/webp"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfToLongImageOptions { pub output_format: ImageFormat, pub dpi: u32, pub add_divider: bool }",
        "zone2_controls": [
            {"id": "output_format", "type": "Select", "label": "Định dạng ảnh đích", "options": ["WebP (Siêu nhẹ)", "JPG (Phổ biến)", "PNG (Không nén)"]},
            {"id": "dpi", "type": "Slider", "label": "Độ phân giải", "min": 100, "max": 300, "default": 150},
            {"id": "add_divider", "type": "Checkbox", "label": "Thêm đường kẻ mảnh phân cách các trang"}
        ],
        "test_scenarios": [
            "Render từng trang PDF qua pdfium-render thành bitmap sắc nét",
            "Ghép nối tiếp theo chiều dọc thành một ảnh duy nhất có đường phân cách trang",
            "Kiểm tra bảo vệ RAM không bị tràn bộ nhớ khi ghép tài liệu > 50 trang"
        ]
    },
    "FR-DOC-16": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfGrayscaleOptions { pub convert_images: bool }",
        "zone2_controls": [
            {"id": "convert_images", "type": "Checkbox", "label": "Chuyển cả các ảnh nhúng sang DeviceGray 8-bit"}
        ],
        "test_scenarios": [
            "Chuyển đổi toàn bộ toán tử màu rg/RG sang g/G",
            "Chuyển đổi không gian màu XObject Image từ DeviceRGB sang DeviceGray",
            "Kiểm tra file sau khi chuyển mở và in hoàn hảo mà không tốn mực màu"
        ]
    },
    "FR-DOC-17": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfRepairOptions { pub scan_depth: String }",
        "zone2_controls": [
            {"id": "scan_depth", "type": "RadioGroup", "label": "Chế độ quét", "options": ["Quét tuyến tính tái tạo xref", "Trích xuất đối tượng còn nguyên vẹn"]}
        ],
        "test_scenarios": [
            "Quét toàn bộ mảng byte để định vị lại offset của các cặp từ khóa obj / endobj",
            "Tái tạo bảng tham chiếu chéo xref và trailer bị hỏng",
            "Khôi phục thành công file PDF bị mất đuôi EOF"
        ]
    },
    "FR-DOC-18": {
        "input_mimes": ["text/markdown", "text/plain"],
        "output_mimes": ["application/pdf"],
        "magic_bytes": {"hex": "23 20 / 2D 2D", "ascii": "#  / --", "offset": 0},
        "options_struct": "MarkdownToPdfOptions { pub page_size: String, pub font_size: f32, pub theme: String }",
        "zone2_controls": [
            {"id": "page_size", "type": "Select", "label": "Khổ giấy", "options": ["A4", "A5", "Letter"]},
            {"id": "theme", "type": "Select", "label": "Chủ đề dàn trang", "options": ["Hành chính công vụ VN", "Tài liệu kỹ thuật hiện đại", "Sách báo"]}
        ],
        "test_scenarios": [
            "Phân tích Markdown AST và kết xuất trực tiếp PDF chuẩn A4",
            "Nhúng sẵn phông chữ tiếng Việt Unicode, tự ngắt trang và kẻ bảng",
            "Kiểm tra cú pháp khối code có tô màu cú pháp nhẹ"
        ]
    },
    "FR-DOC-19": {
        "input_mimes": ["application/pdf"],
        "output_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "text/csv"],
        "magic_bytes": {"hex": "25 50 44 46", "ascii": "%PDF-", "offset": 0},
        "options_struct": "PdfTableExtractorOptions { pub page_range: Option<String>, pub output_csv: bool }",
        "zone2_controls": [
            {"id": "page_range", "type": "ImeSafeTextInput", "label": "Trang cần trích bảng", "placeholder": "Tất cả hoặc VD: 1, 3-5"},
            {"id": "output_csv", "type": "Checkbox", "label": "Đồng thời xuất thêm bản sao định dạng CSV"}
        ],
        "test_scenarios": [
            "Phân tích tọa độ (x, y) của cụm từ và đường kẻ vector (m, l, re)",
            "Khôi phục chính xác cấu trúc hàng/cột và ghi ra tệp .xlsx",
            "Kiểm tra xử lý ô bị gộp (Merged Cells) trong bảng"
        ]
    },

    # MEDIA STUDIO
    "FR-MED-01": {
        "input_mimes": ["image/jpeg", "image/png", "image/webp", "image/heic", "image/avif"],
        "output_mimes": ["image/webp", "image/avif", "image/jpeg"],
        "magic_bytes": {"hex": "FF D8 FF / 89 50 4E 47 / 52 49 46 46 / 66 74 79 70", "ascii": "Image", "offset": 0},
        "options_struct": "ImageBatchOptions { pub target_format: String, pub quality: u8, pub max_width: Option<u32>, pub strip_exif: bool }",
        "zone2_controls": [
            {"id": "target_format", "type": "RadioGroup", "label": "Định dạng đích", "options": ["WebP (Khuyến nghị web)", "AVIF (Nén tối đa)", "JPG (Tương thích)"]},
            {"id": "quality", "type": "Slider", "label": "Chất lượng nén", "min": 50, "max": 100, "default": 85},
            {"id": "max_width", "type": "Select", "label": "Thu nhỏ kích thước (Resize)", "options": ["Giữ nguyên", "Full HD (1920px)", "HD (1280px)"]},
            {"id": "strip_exif", "type": "Checkbox", "label": "Xóa thông tin EXIF/GPS bảo vệ quyền riêng tư", "default": True}
        ],
        "test_scenarios": [
            "Giải mã HEIC qua OS Native API và nén sang WebP giảm 70% dung lượng",
            "Chạy rayon song song 100 ảnh trên CPU đa lõi không nghẽn UI",
            "Kiểm tra NFR-SEC-05: Chặn ảnh bom > 16384x16384 pixels"
        ]
    },
    "FR-MED-02": {
        "input_mimes": ["image/jpeg", "image/png"],
        "output_mimes": ["image/jpeg", "image/png"],
        "magic_bytes": {"hex": "FF D8 FF / 89 50 4E 47", "ascii": "JPG/PNG", "offset": 0},
        "options_struct": "PhotoIdCropOptions { pub id_size: IdPhotoSize, pub sheet_size: String }",
        "zone2_controls": [
            {"id": "id_size", "type": "RadioGroup", "label": "Kích thước ảnh thẻ", "options": ["3x4 cm (Hồ sơ, bằng lái)", "4x6 cm (Hộ chiếu, CCCD)"]},
            {"id": "sheet_layout", "type": "Select", "label": "Dàn trang in", "options": ["Ghép 8 ảnh lên khổ in 10x15 cm", "Chỉ xuất 1 ảnh đơn lẻ"]}
        ],
        "test_scenarios": [
            "Căn chỉnh khuôn mặt chuẩn tỷ lệ hộ chiếu và xuất độ phân giải 300 DPI",
            "Dàn nhiều ảnh lên khổ giấy in 10x15 cm với đường cắt viền",
            "Kiểm tra tỷ lệ pixel/inch chính xác khi in"
        ]
    },
    "FR-MED-03": {
        "input_mimes": ["image/jpeg", "image/png"],
        "output_mimes": ["image/jpeg", "image/png"],
        "magic_bytes": {"hex": "FF D8 FF / 89 50 4E 47", "ascii": "JPG/PNG", "offset": 0},
        "options_struct": "ImageRedactOptions { pub redact_type: RedactType, pub regions: Vec<RectArea> }",
        "zone2_controls": [
            {"id": "redact_type", "type": "RadioGroup", "label": "Kiểu che mờ", "options": ["Gaussian Blur (Mờ mượt)", "Pixelate (Khảm điểm ô vuông)", "Hộp đen che tuyệt đối"]},
            {"id": "auto_detect", "type": "Checkbox", "label": "Tự động nhận diện vùng số CCCD và mã QR"}
        ],
        "test_scenarios": [
            "Áp dụng bộ lọc làm mờ vĩnh viễn không thể khôi phục ngược",
            "Tô đen hoàn toàn vùng số CMND/CCCD trên ảnh giấy tờ",
            "Kiểm tra không lưu cache ảnh gốc chưa che mờ"
        ]
    },
    "FR-MED-04": {
        "input_mimes": ["image/jpeg", "image/png", "image/webp"],
        "output_mimes": ["image/jpeg", "image/png", "image/webp"],
        "magic_bytes": {"hex": "FF D8 FF / 89 50 4E 47 / 52 49 46 46", "ascii": "Image", "offset": 0},
        "options_struct": "ExifInspectorOptions { pub strip_all: bool }",
        "zone2_controls": [
            {"id": "exif_table", "type": "MetadataTable", "label": "Thông số máy ảnh, tọa độ GPS, ngày chụp"},
            {"id": "strip_all", "type": "Checkbox", "label": "Tước bỏ hoàn toàn APP1/EXIF/XMP (Lossless Exif Stripping)", "default": True}
        ],
        "test_scenarios": [
            "Đọc chính xác tọa độ GPS, ngày giờ, model máy ảnh",
            "Sử dụng img-parts tước bỏ toàn bộ metadata mà không giải mã nén lại ảnh",
            "Kiểm tra file sau khi tước bỏ giảm dung lượng và không còn GPS"
        ]
    },
    "FR-MED-05": {
        "input_mimes": ["image/jpeg", "image/png", "image/webp"],
        "output_mimes": ["image/jpeg", "image/png", "image/webp"],
        "magic_bytes": {"hex": "FF D8 FF / 89 50 4E 47", "ascii": "Image", "offset": 0},
        "options_struct": "WatermarkBatchOptions { pub watermark_type: String, pub opacity: f32, pub position: String }",
        "zone2_controls": [
            {"id": "watermark_type", "type": "RadioGroup", "label": "Kiểu đóng dấu", "options": ["Chữ chéo lặp lại toàn bức ảnh", "Logo PNG ở 9 góc cố định"]},
            {"id": "opacity", "type": "Slider", "label": "Độ mờ", "min": 0.1, "max": 1.0, "default": 0.4}
        ],
        "test_scenarios": [
            "Chèn chữ bản quyền dạng lưới lặp lại (Tiled Watermark) song song đa luồng",
            "Chèn logo PNG có kênh Alpha vào góc dưới phải",
            "Kiểm tra màu sắc ảnh gốc không bị biến dạng"
        ]
    },
    "FR-MED-06": {
        "input_mimes": ["image/png", "image/jpeg"],
        "output_mimes": ["image/png"],
        "magic_bytes": {"hex": "89 50 4E 47 / FF D8 FF", "ascii": "PNG/JPG", "offset": 0},
        "options_struct": "ScreenshotBeautifierOptions { pub theme: String, pub shadow_blur: f32, pub padding: u32 }",
        "zone2_controls": [
            {"id": "theme", "type": "Select", "label": "Khung cửa sổ", "options": ["macOS Dark Mode", "macOS Light Mode", "Windows 11 Mica", "Không khung"]},
            {"id": "background", "type": "Select", "label": "Nền Gradient", "options": ["Tím hoàng hôn", "Xanh đại dương", "Đơn sắc trong suốt"]},
            {"id": "shadow", "type": "Slider", "label": "Độ đổ bóng mềm (Drop Shadow)", "min": 0, "max": 50, "default": 20}
        ],
        "test_scenarios": [
            "Kết xuất khung viền bo tròn góc anti-aliased bằng tiny-skia",
            "Tạo hiệu ứng đổ bóng mềm Gaussian Drop Shadow",
            "Kiểm tra xuất file PNG sắc nét"
        ]
    },
    "FR-MED-07": {
        "input_mimes": ["image/jpeg", "image/png"],
        "output_mimes": ["image/jpeg", "image/png"],
        "magic_bytes": {"hex": "FF D8 FF / 89 50 4E 47", "ascii": "JPG/PNG", "offset": 0},
        "options_struct": "PerspectiveScanOptions { pub corners: [Point2D; 4], pub auto_enhance: bool }",
        "zone2_controls": [
            {"id": "corner_editor", "type": "InteractiveQuad", "label": "Kéo thả 4 góc tài liệu trên ảnh"},
            {"id": "auto_enhance", "type": "Checkbox", "label": "Tự động cân bằng trắng và tăng tương phản chữ", "default": True}
        ],
        "test_scenarios": [
            "Tính toán ma trận đồng nhất 3x3 nắn phẳng góc nghiêng",
            "Tăng tương phản ký tự đen trên nền giấy trắng",
            "Kiểm tra tọa độ 4 điểm không suy biến"
        ]
    },
    "FR-MED-08": {
        "input_mimes": ["audio/wav", "audio/mpeg"],
        "output_mimes": ["audio/wav"],
        "magic_bytes": {"hex": "52 49 46 46 / 49 44 33", "ascii": "Audio", "offset": 0},
        "options_struct": "AudioTrimOptions { pub silence_threshold_db: f32, pub min_silence_duration_ms: u32 }",
        "zone2_controls": [
            {"id": "silence_threshold_db", "type": "Slider", "label": "Ngưỡng năng lượng âm thanh xem là im lặng (dB)", "min": -60, "max": -20, "default": -40},
            {"id": "min_silence_duration_ms", "type": "Slider", "label": "Thời lượng khoảng lặng tối thiểu cần cắt (ms)", "min": 200, "max": 2000, "default": 500}
        ],
        "test_scenarios": [
            "Giải mã audio bằng symphonia và tính RMS energy theo cửa sổ 50ms",
            "Loại bỏ các đoạn dưới -40 dB và ghép liền mạch các đoạn có tiếng nói",
            "Kiểm tra không bị tiếng lách cách (click/pop) tại điểm nối"
        ]
    },
    "FR-MED-09": {
        "input_mimes": ["video/mp4", "video/quicktime"],
        "output_mimes": ["image/gif", "image/webp"],
        "magic_bytes": {"hex": "66 74 79 70", "ascii": "ftyp", "offset": 4},
        "options_struct": "VideoToGifOptions { pub fps: u32, pub max_width: u32, pub output_format: String }",
        "zone2_controls": [
            {"id": "fps", "type": "Slider", "label": "Tốc độ khung hình (FPS)", "min": 10, "max": 30, "default": 15},
            {"id": "max_width", "type": "Select", "label": "Chiều rộng ảnh", "options": ["480px (Tối ưu chat Zalo)", "720px", "Giữ nguyên"]},
            {"id": "output_format", "type": "RadioGroup", "label": "Định dạng", "options": ["WebP động (Dung lượng siêu nhẹ)", "GIF cổ điển"]}
        ],
        "test_scenarios": [
            "Trích xuất khung hình RGBA qua OS Native Video Decoder",
            "Lượng tử hóa bảng màu NeuQuant tạo GIF màu sắc mượt",
            "Kiểm tra video ngắn 10 giây xuất file < 5 MB"
        ]
    },

    # DATA LAB
    "FR-DAT-01": {
        "input_mimes": ["text/plain", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "output_mimes": ["text/plain", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "magic_bytes": {"hex": "Multi", "ascii": "Text/Docx", "offset": 0},
        "options_struct": "FontConvertOptions { pub source_encoding: Option<String>, pub target_encoding: String }",
        "zone2_controls": [
            {"id": "source_encoding", "type": "Select", "label": "Bảng mã nguồn", "options": ["Tự động nhận diện", "TCVN3 (ABC)", "VNI-Windows", "Vietware", "Unicode tổ hợp (NFD)"]},
            {"id": "target_encoding", "type": "Select", "label": "Bảng mã đích", "options": ["Unicode dựng sẵn (NFC - Khuyến nghị)"]}
        ],
        "test_scenarios": [
            "Chuyển chuỗi văn bản TCVN3 sang Unicode NFC với tốc độ > 100.000 dòng/giây",
            "Chuyển đổi bảng mã trong file .docx giữ nguyên in đậm, in nghiêng",
            "Chuyển đổi file .xlsx không làm hỏng công thức tính"
        ]
    },
    "FR-DAT-02": {
        "input_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "text/csv"],
        "output_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "magic_bytes": {"hex": "50 4B 03 04", "ascii": "PK..", "offset": 0},
        "options_struct": "ExcelCleanOptions { pub normalize_dates: bool, pub split_vietnamese_names: bool, pub trim_spaces: bool, pub deduplicate: bool }",
        "zone2_controls": [
            {"id": "normalize_dates", "type": "Checkbox", "label": "Đưa ngày tháng về chuẩn DD/MM/YYYY", "default": True},
            {"id": "split_vietnamese_names", "type": "Checkbox", "label": "Tách cột Họ / Tên đệm / Tên theo quy tắc họ kép VN", "default": True},
            {"id": "trim_spaces", "type": "Checkbox", "label": "Xóa khoảng trắng thừa và viết hoa chữ cái đầu tên riêng", "default": True},
            {"id": "deduplicate", "type": "Checkbox", "label": "Loại bỏ dòng dữ liệu trùng lặp"}
        ],
        "test_scenarios": [
            "Tách tên 'Nguyễn Thị Thu Trang' thành Họ: Nguyễn, Đệm: Thị Thu, Tên: Trang",
            "Tách tên họ kép 'Nguyễn Phước Vĩnh Lộc' chuẩn xác",
            "Chuẩn hóa ngày hỗn hợp '2026/05/12', '12-5-2026' về '12/05/2026'"
        ]
    },
    "FR-DAT-03": {
        "input_mimes": ["text/plain", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "output_mimes": ["text/html", "application/json"],
        "magic_bytes": {"hex": "Multi", "ascii": "Diff", "offset": 0},
        "options_struct": "DataDiffOptions { pub ignore_whitespace: bool, pub case_sensitive: bool }",
        "zone2_controls": [
            {"id": "file_a", "type": "FilePicker", "label": "Tệp gốc A"},
            {"id": "file_b", "type": "FilePicker", "label": "Tệp so sánh B"},
            {"id": "ignore_whitespace", "type": "Checkbox", "label": "Bỏ qua khoảng trắng thừa"}
        ],
        "test_scenarios": [
            "Thuật toán Myers/Patience Diff so sánh từng ô bảng tính bị sửa đổi",
            "Tô màu trực quan các dòng được thêm mới, xóa bỏ hoặc sửa giá trị",
            "Kiểm tra so sánh hai văn bản dài có dấu tiếng Việt"
        ]
    },
    "FR-DAT-04": {
        "input_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "text/csv"],
        "output_mimes": ["application/json", "text/plain", "text/markdown"],
        "magic_bytes": {"hex": "Multi", "ascii": "Data", "offset": 0},
        "options_struct": "DataConverterOptions { pub target_format: String, pub table_name: Option<String> }",
        "zone2_controls": [
            {"id": "target_format", "type": "Select", "label": "Định dạng chuyển đổi", "options": ["Mảng JSON Objects", "Câu lệnh SQL INSERT INTO", "Bảng kẻ Markdown"]},
            {"id": "table_name", "type": "ImeSafeTextInput", "label": "Tên bảng SQL (nếu chọn xuất SQL)", "placeholder": "users"}
        ],
        "test_scenarios": [
            "Chuyển bảng tính Excel 10.000 dòng sang mảng JSON chuẩn trong < 500ms",
            "Tạo câu lệnh SQL INSERT INTO chuẩn hóa kiểu dữ liệu số, ngày, chuỗi",
            "Xuất bảng Markdown hoàn hảo"
        ]
    },
    "FR-DAT-05": {
        "input_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "output_mimes": ["application/zip", "inode/directory"],
        "magic_bytes": {"hex": "50 4B 03 04", "ascii": "PK..", "offset": 0},
        "options_struct": "ExcelSplitOptions { pub split_column_name: String }",
        "zone2_controls": [
            {"id": "split_column_name", "type": "ImeSafeTextInput", "label": "Tên cột dùng làm khóa tách file", "placeholder": "VD: MaPhongBan, ChiNhanh"}
        ],
        "test_scenarios": [
            "Tách bảng lương 5.000 dòng theo cột 'Mã Phòng Ban' thành 20 file nhỏ",
            "Bảo tồn nguyên vẹn header tiêu đề và định dạng ô ở các file con",
            "Chạy xuất file song song qua rayon"
        ]
    },
    "FR-DAT-06": {
        "input_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "text/csv"],
        "output_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "magic_bytes": {"hex": "50 4B 03 04", "ascii": "PK..", "offset": 0},
        "options_struct": "ExcelMergeOptions { pub mode: MergeMode, pub add_source_column: bool }",
        "zone2_controls": [
            {"id": "mode", "type": "RadioGroup", "label": "Kiểu gộp", "options": ["Gộp nối tiếp dòng (Append Rows)", "Gộp thành nhiều Sheet trong 1 Workbook"]},
            {"id": "add_source_column", "type": "Checkbox", "label": "Thêm cột ghi rõ tên file nguồn", "default": True}
        ],
        "test_scenarios": [
            "Gộp 10 file báo cáo có cùng cấu trúc cột thành 1 file tổng hợp",
            "Thêm cột truy vết tên file nguồn vào đầu mỗi dòng",
            "Kiểm tra khớp cột ngay cả khi thứ tự cột trong các file bị xáo trộn nhẹ"
        ]
    },
    "FR-DAT-07": {
        "input_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "text/csv"],
        "output_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "magic_bytes": {"hex": "50 4B 03 04", "ascii": "PK..", "offset": 0},
        "options_struct": "ExcelJoinOptions { pub join_type: JoinType, pub key_a: String, pub key_b: String }",
        "zone2_controls": [
            {"id": "join_type", "type": "Select", "label": "Kiểu kết hợp (Join)", "options": ["INNER JOIN (Chỉ lấy dòng khớp)", "LEFT JOIN (Giữ nguyên bảng A)", "FULL OUTER JOIN", "ANTI JOIN (Tìm dòng lệch)"]},
            {"id": "key_a", "type": "ImeSafeTextInput", "label": "Cột khóa ở Bảng A"},
            {"id": "key_b", "type": "ImeSafeTextInput", "label": "Cột khóa ở Bảng B"}
        ],
        "test_scenarios": [
            "Khớp nối Hash Join hai bảng tính trong bộ nhớ không cần hàm VLOOKUP",
            "ANTI JOIN phát hiện chính xác các mã nhân viên có trong bảng lương nhưng thiếu trong danh sách nhân sự",
            "Kiểm tra hiệu năng với bảng tính 50.000 dòng"
        ]
    },
    "FR-DAT-08": {
        "input_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "output_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "magic_bytes": {"hex": "50 4B 03 04", "ascii": "PK..", "offset": 0},
        "options_struct": "ExcelUnlockOptions {}",
        "zone2_controls": [
            {"id": "info", "type": "InfoBadge", "label": "Tự động gỡ thẻ khóa bảo vệ sheet (<sheetProtection>) mà không cần mật khẩu"}
        ],
        "test_scenarios": [
            "Giải nén luồng XML OpenXML và loại bỏ thẻ <sheetProtection>",
            "Xác nhận file sau khi mở cho phép chỉnh sửa mọi ô tính",
            "Kiểm tra không làm mất công thức hoặc biểu đồ trong sheet"
        ]
    },
    "FR-DAT-09": {
        "input_mimes": ["text/plain"],
        "output_mimes": ["text/plain"],
        "magic_bytes": {"hex": "Text", "ascii": "List", "offset": 0},
        "options_struct": "ListSetOptions { pub operation: SetOperation }",
        "zone2_controls": [
            {"id": "operation", "type": "Select", "label": "Phép toán tập hợp", "options": ["Giao nhau (A ∩ B - Phần tử có ở cả 2)", "Hợp nhất (A ∪ B - Toàn bộ không trùng lặp)", "Hiệu (A \\ B - Chỉ có ở danh sách A)", "Hiệu đối xứng (A △ B - Chỉ có ở 1 trong 2)"]},
            {"id": "list_a", "type": "TextArea", "label": "Danh sách A"},
            {"id": "list_b", "type": "TextArea", "label": "Danh sách B"}
        ],
        "test_scenarios": [
            "Thực hiện phép toán tập hợp trên 100.000 số điện thoại trong dưới 10ms",
            "Lọc trùng lặp danh sách email chuẩn xác",
            "Kiểm tra không phân biệt chữ hoa/thường nếu có tùy chọn"
        ]
    },
    "FR-DAT-10": {
        "input_mimes": ["text/plain"],
        "output_mimes": ["text/plain"],
        "magic_bytes": {"hex": "Text", "ascii": "Text", "offset": 0},
        "options_struct": "MojibakeFixOptions {}",
        "zone2_controls": [
            {"id": "input_text", "type": "TextArea", "label": "Đoạn văn bản bị lỗi hiển thị font"},
            {"id": "auto_detect", "type": "Checkbox", "label": "Tự động đảo ngược chu kỳ giải mã nhầm UTF-8/Windows-1252", "default": True}
        ],
        "test_scenarios": [
            "Khôi phục chuỗi 'Tiáº¿ng Viá»‡t' về 'Tiếng Việt'",
            "Xử lý đúng chuỗi lỗi bảng mã ISO-8859-1",
            "Bảo toàn chuỗi gốc nếu không phát hiện mẫu mojibake"
        ]
    },
    "FR-DAT-11": {
        "input_mimes": ["text/plain", "text/csv", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "output_mimes": ["text/plain", "text/csv", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "magic_bytes": {"hex": "Multi", "ascii": "PII", "offset": 0},
        "options_struct": "PiiAnonymizerOptions { pub mask_cccd: bool, pub mask_phone: bool, pub mask_mst: bool, pub mask_bank: bool }",
        "zone2_controls": [
            {"id": "mask_cccd", "type": "Checkbox", "label": "Ẩn số CMND/CCCD ([CCCD_ẨN])", "default": True},
            {"id": "mask_phone", "type": "Checkbox", "label": "Ẩn số điện thoại di động VN ([SĐT_ẨN])", "default": True},
            {"id": "mask_mst", "type": "Checkbox", "label": "Ẩn Mã số thuế ([MST_ẨN])", "default": True},
            {"id": "mask_bank", "type": "Checkbox", "label": "Ẩn số tài khoản ngân hàng ([STK_ẨN])", "default": True}
        ],
        "test_scenarios": [
            "Thuật toán Aho-Corasick & Regex quét và thay thế CCCD 12 số trong văn bản",
            "Thay thế số điện thoại đầu số VN (09x, 08x, 03x, 07x) bằng nhãn ẩn",
            "Kiểm tra xử lý hàng loạt trên cột file Excel"
        ]
    },
    "FR-DAT-12": {
        "input_mimes": ["text/plain"],
        "output_mimes": ["text/plain"],
        "magic_bytes": {"hex": "Text", "ascii": "SQL", "offset": 0},
        "options_struct": "SqlFormatterOptions { pub dialect: SqlDialect, pub uppercase_keywords: bool }",
        "zone2_controls": [
            {"id": "dialect", "type": "Select", "label": "Phương ngữ SQL", "options": ["PostgreSQL", "MySQL", "SQLite", "T-SQL (SQL Server)"]},
            {"id": "uppercase_keywords", "type": "Checkbox", "label": "Viết hoa toàn bộ từ khóa (SELECT, FROM, WHERE...)", "default": True}
        ],
        "test_scenarios": [
            "Kiểm tra cú pháp AST bằng sqlparser và bắt lỗi sai ngữ pháp",
            "Căn chỉnh thụt lề câu lệnh lồng JOIN phức tạp sắc nét",
            "Kiểm tra bảo toàn các chuỗi tiếng Việt trong mệnh đề WHERE N'...'"
        ]
    },
    "FR-DAT-13": {
        "input_mimes": ["*/*"],
        "output_mimes": ["text/plain", "*/*"],
        "magic_bytes": {"hex": "Multi", "ascii": "Base64", "offset": 0},
        "options_struct": "Base64CodecOptions { pub mode: CodecMode, pub data_uri_prefix: bool }",
        "zone2_controls": [
            {"id": "mode", "type": "RadioGroup", "label": "Chế độ", "options": ["Mã hóa Tệp sang chuỗi Base64 / Data URI", "Giải mã chuỗi Base64 về Tệp gốc"]},
            {"id": "data_uri_prefix", "type": "Checkbox", "label": "Bao gồm tiền tố Data URI (VD: data:image/png;base64,...)"}
        ],
        "test_scenarios": [
            "Mã hóa theo luồng tệp ảnh 50 MB sang Base64 không tăng vọt RAM",
            "Giải mã chuỗi Data URI về đúng tệp nhị phân gốc",
            "Kiểm tra tính toàn vẹn mã băm SHA-256 trước và sau giải mã"
        ]
    },

    # VN OFFICE
    "FR-VNO-01": {
        "input_mimes": ["text/plain", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "output_mimes": ["text/plain"],
        "magic_bytes": {"hex": "Text", "ascii": "Num", "offset": 0},
        "options_struct": "NumberToWordsConfig { pub dialect: Dialect, pub currency: CurrencyUnit, pub use_dong_chan: bool, pub capitalize_first: bool, pub negative_prefix: String }",
        "zone2_controls": [
            {"id": "dialect", "type": "RadioGroup", "label": "Phương ngữ", "options": ["Chuẩn miền Bắc (nghìn/lẻ/mốt/tư)", "Chuẩn miền Nam (ngàn/linh/tư)"]},
            {"id": "currency", "type": "Select", "label": "Đơn vị tiền tệ", "options": ["Việt Nam Đồng (đồng)", "Đô la Mỹ (đô la Mỹ / xu)", "Euro (euro / cents)", "Không đơn vị"]},
            {"id": "use_dong_chan", "type": "Checkbox", "label": "Thêm hậu tố 'chẵn' khi không có số thập phân", "default": True},
            {"id": "capitalize_first", "type": "Checkbox", "label": "Viết hoa chữ cái đầu câu", "default": True}
        ],
        "test_scenarios": [
            "Đọc số 105: Bắc là 'Một trăm lẻ năm', Nam là 'Một trăm linh năm'",
            "Đọc các số lớn hàng tỷ tỷ '1000000000000000000' chuẩn xác",
            "Đọc số tiền kế toán có đuôi 'đồng chẵn' và số âm 'âm một triệu đồng'"
        ]
    },
    "FR-VNO-02": {
        "input_mimes": ["application/json", "text/plain"],
        "output_mimes": ["application/json", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "magic_bytes": {"hex": "Text", "ascii": "Salary", "offset": 0},
        "options_struct": "SalaryCalculatorConfig { pub gross_amount: rust_decimal::Decimal, pub region: u8, pub dependents: u32, pub custom_ruleset_id: Option<String> }",
        "zone2_controls": [
            {"id": "gross_amount", "type": "ImeSafeTextInput", "label": "Mức lương (VND)", "placeholder": "VD: 25,000,000"},
            {"id": "region", "type": "Select", "label": "Vùng lương", "options": ["Vùng I (Hà Nội, TP.HCM...)", "Vùng II", "Vùng III", "Vùng IV"]},
            {"id": "dependents", "type": "NumberInput", "label": "Số người phụ thuộc", "default": 0},
            {"id": "edit_ruleset_btn", "type": "Button", "label": "⚙️ Tùy chỉnh biểu thuế / mức giảm trừ gia cảnh (legal_rulesets)"}
        ],
        "test_scenarios": [
            "Tính chính xác các mức khấu trừ BHXH (8%), BHYT (1.5%), BHTN (1%) theo trần lương cơ sở",
            "Tính thuế TNCN theo 7 bậc lũy tiến từng phần",
            "Đọc quy tắc pháp lý động từ bảng legal_rulesets trong SQLite"
        ]
    },
    "FR-VNO-03": {
        "input_mimes": ["application/json", "text/plain"],
        "output_mimes": ["image/png", "text/plain"],
        "magic_bytes": {"hex": "Text", "ascii": "QR", "offset": 0},
        "options_struct": "VietQrConfig { pub bin: String, pub account_number: String, pub amount: Option<u64>, pub message: Option<String> }",
        "zone2_controls": [
            {"id": "bank_select", "type": "Select", "label": "Ngân hàng thụ hưởng (60+ ngân hàng VN)", "options": ["Vietcombank (970436)", "MBBank (970422)", "Techcombank (970407)", "ACB (970416)..."]},
            {"id": "account_number", "type": "ImeSafeTextInput", "label": "Số tài khoản ngân hàng", "placeholder": "VD: 1903..."},
            {"id": "amount", "type": "ImeSafeTextInput", "label": "Số tiền chuyển khoản (VND - Tùy chọn)", "placeholder": "VD: 500,000"},
            {"id": "message", "type": "ImeSafeTextInput", "label": "Nội dung chuyển khoản", "placeholder": "Thanh toan tien hang"}
        ],
        "test_scenarios": [
            "Dựng chuỗi EMVCo Merchant Presented QR từ ID 00 đến 63",
            "Tính toán mã kiểm tra CRC-16/CCITT-FALSE chuẩn Napas247",
            "Sinh ảnh mã QR có logo ngân hàng và thông tin tài khoản sắc nét"
        ]
    },
    "FR-VNO-04": {
        "input_mimes": ["application/xml", "text/xml"],
        "output_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "application/pdf"],
        "magic_bytes": {"hex": "3C 3F 78 6D 6C", "ascii": "<?xml", "offset": 0},
        "options_struct": "EinvoiceXmlReaderOptions { pub verify_signature: bool }",
        "zone2_controls": [
            {"id": "preview_invoice", "type": "InvoiceVisualCard", "label": "Bản thể hiện trực quan hóa đơn VAT"},
            {"id": "export_items_excel", "type": "Button", "label": "📊 Trích xuất danh mục hàng hóa ra Excel"},
            {"id": "signature_badge", "type": "StatusBadge", "label": "Kiểm tra tính hợp lệ thẻ chữ ký số <ds:Signature>"}
        ],
        "test_scenarios": [
            "Phân tích cú pháp XML hóa đơn Thông tư 78/2021 và Nghị định 123/2020",
            "Trích xuất thông tin người bán, người mua, danh sách mặt hàng, thuế suất",
            "Kiểm tra phát hiện chữ ký số hợp lệ trong thẻ <ds:Signature>"
        ]
    },
    "FR-VNO-05": {
        "input_mimes": ["text/plain"],
        "output_mimes": ["application/json"],
        "magic_bytes": {"hex": "Text", "ascii": "CCCD", "offset": 0},
        "options_struct": "CccdMstValidatorOptions { pub input_string: String }",
        "zone2_controls": [
            {"id": "input_string", "type": "ImeSafeTextInput", "label": "Nhập dãy số CCCD (12 số) hoặc Mã số thuế (10 hoặc 13 số)", "placeholder": "VD: 001098012345 hoặc 0101234567"}
        ],
        "test_scenarios": [
            "Giải mã 6 số đầu CCCD: 3 số đầu là mã tỉnh, 1 số là thế kỷ & giới tính, 2 số là năm sinh",
            "Kiểm tra thuật toán Modulo 11 với vector trọng số kiểm tra tính hợp lệ của Mã số thuế",
            "Phát hiện chính xác số CCCD/MST giả mạo hoặc gõ sai ký tự"
        ]
    },
    "FR-VNO-06": {
        "input_mimes": ["text/plain", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "output_mimes": ["application/json", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        "magic_bytes": {"hex": "Text", "ascii": "Address", "offset": 0},
        "options_struct": "AddressNormalizerOptions { pub target_model: AdminModel, pub fuzzy_threshold: f64 }",
        "zone2_controls": [
            {"id": "target_model", "type": "RadioGroup", "label": "Mô hình địa giới hành chính", "options": ["Mô hình 3 cấp truyền thống (Tỉnh - Huyện - Xã)", "Mô hình 2 cấp mới sau sáp nhập (Tỉnh - Phường/Xã mới)"]},
            {"id": "address_input", "type": "ImeSafeTextInput", "label": "Địa chỉ gõ tự do", "placeholder": "VD: sn 12 ngo 45 le trong tan thanh xuan hn"}
        ],
        "test_scenarios": [
            "Khớp chuỗi mờ Jaro-Winkler với từ điển hành chính SQLite",
            "Ánh xạ chính xác các từ viết tắt 'hn' -> Hà Nội, 'q.tb' -> Quận Tân Bình",
            "Chuyển đổi qua lại giữa mô hình địa giới cũ và mới sau sáp nhập"
        ]
    },
    "FR-VNO-07": {
        "input_mimes": ["application/json", "text/plain"],
        "output_mimes": ["application/json", "text/calendar"],
        "magic_bytes": {"hex": "Text", "ascii": "Calendar", "offset": 0},
        "options_struct": "WorkingDaysOptions { pub start_date: String, pub end_date: String, pub exclude_saturday: bool }",
        "zone2_controls": [
            {"id": "date_range", "type": "DateRangePicker", "label": "Khoảng ngày bắt đầu - kết thúc"},
            {"id": "exclude_saturday", "type": "Checkbox", "label": "Nghỉ Thứ Bảy và Chủ Nhật", "default": True},
            {"id": "export_ics", "type": "Button", "label": "📅 Xuất tệp lịch .ics cho Outlook / Google Calendar"}
        ],
        "test_scenarios": [
            "Tính toán lịch Âm - Dương tự động nhận diện Tết Nguyên Đán và Giỗ Tổ Hùng Vương 10/3 Âm lịch",
            "Trừ chính xác các ngày nghỉ lễ chính thức 30/4, 1/5, 2/9",
            "Xuất tệp lịch .ics chuẩn RFC 5545"
        ]
    },
    "FR-VNO-08": {
        "input_mimes": ["application/json", "text/plain"],
        "output_mimes": ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "application/pdf"],
        "magic_bytes": {"hex": "Text", "ascii": "Loan", "offset": 0},
        "options_struct": "LoanAmortizationOptions { pub principal: rust_decimal::Decimal, pub annual_rate: rust_decimal::Decimal, pub term_months: u32, pub method: AmortizationMethod }",
        "zone2_controls": [
            {"id": "principal", "type": "ImeSafeTextInput", "label": "Số tiền vay (VND)", "placeholder": "1,000,000,000"},
            {"id": "annual_rate", "type": "ImeSafeTextInput", "label": "Lãi suất (%/năm)", "placeholder": "8.5"},
            {"id": "term_months", "type": "NumberInput", "label": "Kỳ hạn vay (tháng)", "default": 120},
            {"id": "method", "type": "RadioGroup", "label": "Phương thức trả nợ", "options": ["Dư nợ giảm dần (Gốc đều, lãi giảm)", "Niên kim cố định (Trả góp đều mỗi tháng)"]}
        ],
        "test_scenarios": [
            "Tính toán số học thập phân rust_decimal chính xác từng đồng",
            "Tính toán bảng lịch trả nợ chi tiết theo phương thức dư nợ giảm dần",
            "Xuất bảng kế hoạch trả nợ ra Excel và PDF"
        ]
    },
    "FR-VNO-09": {
        "input_mimes": ["text/plain"],
        "output_mimes": ["application/json"],
        "magic_bytes": {"hex": "Text", "ascii": "Bank", "offset": 0},
        "options_struct": "BankBinLookupOptions { pub query: String }",
        "zone2_controls": [
            {"id": "query", "type": "ImeSafeTextInput", "label": "Tìm kiếm ngân hàng", "placeholder": "Gõ tên tắt (VCB, TCB, MB), tên đầy đủ hoặc mã BIN (970436)..."}
        ],
        "test_scenarios": [
            "Tra cứu tức thì theo mã BIN 6 số Napas",
            "Tìm kiếm theo tên viết tắt VCB -> Vietcombank, SWIFT BFTVVNVX",
            "Kiểm tra dữ liệu danh bạ hơn 60 ngân hàng nạp từ SQLite"
        ]
    },

    # SYS & HUB & UTILITY
    "FR-SYS-01": {
        "input_mimes": ["ui/events"],
        "output_mimes": ["application/json"],
        "magic_bytes": {"hex": "DOM", "ascii": "Input", "offset": 0},
        "options_struct": "HardwareInputTestOptions {}",
        "zone2_controls": [
            {"id": "keyboard_canvas", "type": "VirtualKeyboardVisualizer", "label": "Bàn phím ảo trực quan phát hiện kẹt phím"},
            {"id": "mouse_click_tester", "type": "MouseClickBox", "label": "Vùng bấm chuột đo khoảng thời gian micro-giây phát hiện Double-Click"}
        ],
        "test_scenarios": [
            "Nhận diện chính xác sự kiện keydown/keyup và N-Key Rollover",
            "Đo khoảng cách delta_t giữa 2 lần mousedown để bắt lỗi double-click khi delta_t < 80ms",
            "Kiểm tra không bị rò rỉ bộ nhớ listener DOM"
        ]
    },
    "FR-SYS-02": {
        "input_mimes": ["media/devices"],
        "output_mimes": ["ui/render"],
        "magic_bytes": {"hex": "Hardware", "ascii": "Media", "offset": 0},
        "options_struct": "DisplayMediaTestOptions {}",
        "zone2_controls": [
            {"id": "fullscreen_color_btn", "type": "Button", "label": "🖥️ Chiếu màn hình toàn cảnh đơn sắc (Đỏ, Xanh, Trắng, Đen) kiểm tra điểm chết"},
            {"id": "webcam_mic_preview", "type": "MediaStreamView", "label": "Xem thử webcam và biểu đồ sóng âm FFT micro thời gian thực"}
        ],
        "test_scenarios": [
            "Kích hoạt Fullscreen API đổi màu nền tuần tự",
            "Mở luồng WebRTC cục bộ cho camera và vẽ thanh biên độ âm thanh",
            "Đảm bảo giải phóng tài nguyên media stream khi đóng cửa sổ"
        ]
    },
    "FR-SYS-03": {
        "input_mimes": ["system/query"],
        "output_mimes": ["application/json"],
        "magic_bytes": {"hex": "Sysinfo", "ascii": "System", "offset": 0},
        "options_struct": "SystemHardwareQueryOptions {}",
        "zone2_controls": [
            {"id": "sys_dashboard", "type": "DashboardCards", "label": "Thông số CPU, Xung nhịp, Dung lượng RAM, Dung lượng Ổ đĩa, Mức tiêu thụ của ứng dụng"}
        ],
        "test_scenarios": [
            "Truy vấn thông số CPU, RAM khả dụng qua crate sysinfo",
            "Đo mức tiêu thụ tài nguyên của chính tiến trình ứng dụng",
            "Cập nhật chu kỳ mỗi 1 giây không gây tải CPU"
        ]
    },
    "FR-HUB-01": {
        "input_mimes": ["*/*"],
        "output_mimes": ["*/*"],
        "magic_bytes": {"hex": "Any", "ascii": "Infer", "offset": 0},
        "options_struct": "UniversalConverterOptions { pub target_extension: String }",
        "zone2_controls": [
            {"id": "universal_dropzone", "type": "LargeDropzone", "label": "📥 Thả bất kỳ tệp tài liệu, hình ảnh, âm thanh hoặc dữ liệu vào đây"},
            {"id": "target_suggestions", "type": "TagSuggestions", "label": "Gợi ý các định dạng chuyển đổi khả thi sau khi đọc Magic Bytes"}
        ],
        "test_scenarios": [
            "Đọc 64 byte đầu qua infer xác định đúng MIME thực tế của file bị đổi sai đuôi",
            "Tra cứu ma trận ánh xạ và gợi ý danh sách định dạng đích khả thi",
            "Điều hướng chính xác tới công cụ xử lý tương ứng"
        ]
    },
    "FR-UTL-01": {
        "input_mimes": ["application/json", "text/plain"],
        "output_mimes": ["text/plain", "image/png"],
        "magic_bytes": {"hex": "Text", "ascii": "URL", "offset": 0},
        "options_struct": "UtmBuilderOptions { pub base_url: String, pub source: String, pub medium: String, pub campaign: String, pub term: Option<String>, pub content: Option<String> }",
        "zone2_controls": [
            {"id": "base_url", "type": "ImeSafeTextInput", "label": "Đường dẫn URL gốc", "placeholder": "https://example.com/khuyen-mai"},
            {"id": "utm_source", "type": "ImeSafeTextInput", "label": "Nguồn chiến dịch (utm_source)", "placeholder": "facebook, google, zalo"},
            {"id": "utm_medium", "type": "ImeSafeTextInput", "label": "Kênh tiếp thị (utm_medium)", "placeholder": "cpc, banner, email"},
            {"id": "utm_campaign", "type": "ImeSafeTextInput", "label": "Tên chiến dịch (utm_campaign)", "placeholder": "tet_2026"}
        ],
        "test_scenarios": [
            "Ghép tham số truy vấn chuẩn RFC 3986 không bị lỗi mã hóa ký tự",
            "Sinh mã QR tương ứng từ link UTM hoàn chỉnh",
            "Lưu cấu hình chiến dịch thành Preset vào bảng tool_presets"
        ]
    },
    "FR-UTL-02": {
        "input_mimes": ["text/plain"],
        "output_mimes": ["text/plain"],
        "magic_bytes": {"hex": "Text", "ascii": "Text", "offset": 0},
        "options_struct": "UnicodeStylerOptions { pub style: FontStyle }",
        "zone2_controls": [
            {"id": "input_text", "type": "TextArea", "label": "Đoạn văn bản cần đổi kiểu chữ"},
            {"id": "style_picker", "type": "RadioGroup", "label": "Kiểu chữ nghệ thuật", "options": ["Bold Serif (𝐁𝐨𝐥𝐝)", "Italic (𝐼𝑡𝑎𝑙𝑖𝑐)", "Script (𝒮𝒸𝓇𝒾𝓅𝓉)", "Monospace (𝙼𝚘𝚗𝚘)", "Khoanh tròn (Ⓒⓘⓡⓒⓛⓔ)"]}
        ],
        "test_scenarios": [
            "Ánh xạ ký tự sang bảng Unicode Mathematical Alphanumeric Symbols",
            "Bảo tồn các ký tự tiếng Việt có dấu thanh không bị mất",
            "Kiểm tra sao chép kết quả vào clipboard"
        ]
    },
    "FR-UTL-03": {
        "input_mimes": ["application/json", "text/plain"],
        "output_mimes": ["application/json", "text/plain"],
        "magic_bytes": {"hex": "Text", "ascii": "Raffle", "offset": 0},
        "options_struct": "CryptographicRaffleOptions { pub participants: Vec<String>, pub seed: String, pub winners_count: usize }",
        "zone2_controls": [
            {"id": "participants", "type": "TextArea", "label": "Danh sách người tham gia (mỗi người 1 dòng)"},
            {"id": "seed", "type": "ImeSafeTextInput", "label": "Hạt giống công khai (Public Seed - VD: Tiêu đề báo chí)", "placeholder": "GiaVangSJC_02112026"},
            {"id": "winners_count", "type": "NumberInput", "label": "Số lượng giải trúng", "default": 1}
        ],
        "test_scenarios": [
            "Sinh số ngẫu nhiên an toàn mật mã học từ nguồn OsRng phần cứng",
            "Tạo chuỗi biên bản kết quả kèm mã băm SHA-256 để kiểm toán minh bạch",
            "Kiểm tra hạt giống công khai sinh cùng một thứ tự giải khi tái kiểm tra"
        ]
    },
    "FR-UTL-04": {
        "input_mimes": ["application/json", "text/plain"],
        "output_mimes": ["application/json"],
        "magic_bytes": {"hex": "Text", "ascii": "Profit", "offset": 0},
        "options_struct": "EcomProfitOptions { pub cogs: rust_decimal::Decimal, pub selling_price: rust_decimal::Decimal, pub platform_fee_pct: rust_decimal::Decimal, pub return_rate_pct: rust_decimal::Decimal }",
        "zone2_controls": [
            {"id": "cogs", "type": "ImeSafeTextInput", "label": "Giá vốn hàng bán (VND)", "placeholder": "100,000"},
            {"id": "selling_price", "type": "ImeSafeTextInput", "label": "Giá bán dự kiến (VND)", "placeholder": "199,000"},
            {"id": "platform", "type": "Select", "label": "Sàn TMĐT", "options": ["Shopee (Phí cố định + Phí thanh toán + Freeship Extra)", "TikTok Shop", "Lazada", "Tùy chỉnh"]},
            {"id": "return_rate", "type": "Slider", "label": "Tỷ lệ đơn hoàn dự kiến (%)", "min": 0, "max": 20, "default": 3}
        ],
        "test_scenarios": [
            "Tính toán chi tiết phí sàn, phí thanh toán, thuế hộ kinh doanh 1.5%",
            "Tính giá bán hòa vốn dựa trên tỷ lệ hàng hoàn",
            "Tính lãi ròng thực tế bằng số học thập phân rust_decimal"
        ]
    }
}

def parse_srs_markdown(srs_path: str):
    """
    Quét trực tiếp toàn bộ bảng markdown trong 02-srs.md
    để bóc tách các dòng | **FR-* |
    """
    if not os.path.isfile(srs_path):
        raise FileNotFoundError(f"Không tìm thấy tệp đặc tả SRS: {srs_path}")

    with open(srs_path, "r", encoding="utf-8") as f:
        content = f.read()

    rows = []
    for line in content.splitlines():
        line_str = line.strip()
        if line_str.startswith("| **FR-"):
            parts = [p.strip() for p in line_str.split("|")[1:-1]]
            if len(parts) >= 7:
                code = parts[0].replace("*", "")
                name = parts[1]
                slug = parts[2].replace("`", "")
                crate = parts[3]
                inp = parts[4]
                outp = parts[5]
                desc = parts[6]
                rows.append({
                    "code": code,
                    "name": name,
                    "slug": slug,
                    "crate": crate,
                    "input": inp,
                    "output": outp,
                    "desc": desc
                })
    return rows

def build_atomic_task_payload(row: dict) -> dict:
    code = row["code"]
    slug = row["slug"]
    name = row["name"]
    crate = row["crate"]
    desc = row["desc"]

    # Xác định prefix miền
    prefix = code.split("-")[1]
    domain, category, domain_order = DOMAIN_MAP.get(prefix, ("sys-hub", "SYSTEM", 5))

    # Xác định MoSCoW & Roadmap Phase
    priority, phase, priority_order = PRIORITY_MAP.get(code, ("Could-have", "Phase 3: Offline AI, Media Pro & Diagnostics", 3))

    # User Story
    us_code, us_title = USER_STORY_MAP.get(code, ("US-03", "Tiện ích văn phòng"))

    # Chi tiết kỹ thuật & DTO
    details = TOOL_DETAILS.get(code, {
        "input_mimes": ["*/*"],
        "output_mimes": ["*/*"],
        "magic_bytes": {"hex": "Custom", "ascii": "Custom", "offset": 0},
        "options_struct": f"{slug.replace('-', '_').title().replace('_', '')}Options {{}}",
        "zone2_controls": [{"id": "input", "type": "ImeSafeTextInput", "label": "Tham số đầu vào"}],
        "test_scenarios": ["Kiểm thử chức năng chính", "Kiểm thử đầu vào không hợp lệ", "Kiểm thử hủy tác vụ AtomicBool"]
    })

    slug_underscore = slug.replace("-", "_")
    domain_folder = domain.replace("-", "_")

    payload = {
        "task_id": f"TASK-{code}",
        "tool_code": code,
        "slug": slug,
        "name": name,
        "domain": domain,
        "category": category,
        "priority": priority,
        "priority_order": priority_order,
        "domain_order": domain_order,
        "phase": phase,
        "user_story": {
            "code": us_code,
            "title": us_title
        },
        "specification": {
            "crate_engine": crate,
            "inputs_raw": row["input"],
            "outputs_raw": row["output"],
            "technical_description": desc,
            "acceptance_criteria": [
                f"AC-01 (100% On-Device): Thực thi hoàn toàn ngoại tuyến, cấm std::net/tokio::net/reqwest.",
                f"AC-02 (Non-blocking UI 60 FPS): Mọi tính toán điều phối qua rayon threadpool hoặc tokio::task::spawn_blocking.",
                f"AC-03 (Báo cáo tiến độ): Bắn TaskProgress qua tauri::ipc::Channel định kỳ 100-200ms.",
                f"AC-04 (Hủy tác vụ tức thì): Kiểm tra cờ cancel_flag: &AtomicBool mỗi chu kỳ lặp để thoát trong < 100ms."
            ]
        },
        "io_contract": {
            "input_mimes": details["input_mimes"],
            "output_mimes": details["output_mimes"],
            "magic_bytes": details["magic_bytes"],
            "max_in_memory_mb": 250,
            "temp_cache_dir_template": "{app_cache_dir}/temp_jobs/{task_id}"
        },
        "engine_pod": {
            "crate_target": "crates/office-engine",
            "module_path": f"crates/office-engine/src/{domain_folder}/{slug_underscore}.rs",
            "mod_declaration": f"pub mod {slug_underscore};",
            "trait_impl": "OfficeTool",
            "trait_signature": (
                "pub trait OfficeTool: Send + Sync {\n"
                "    fn slug(&self) -> &'static str;\n"
                "    fn name(&self) -> &'static str;\n"
                "    fn description(&self) -> &'static str;\n"
                "    fn execute(&self, request: &DispatchTaskRequest, cancel_flag: &AtomicBool, on_progress: Option<&dyn Fn(TaskProgress)>) -> Result<DispatchTaskResponse, CommandError>;\n"
                "}"
            ),
            "options_struct_definition": details["options_struct"],
            "execution_rules": {
                "threading": "rayon work-stealing threadpool / tokio::task::spawn_blocking",
                "progress_interval_ms": 200,
                "cancellation_check": "cancel_flag.load(Ordering::Relaxed) inside loop -> Err(CommandError::TaskCancelled)",
                "memory_limits": "Max RAM < 250 MB; Image limits max 16384x16384; Zip bomb ratio max 1:100",
                "security_zeroize": "Bọc mật khẩu/khóa nhạy cảm bằng zeroize::Zeroizing"
            }
        },
        "unit_tests": {
            "test_command": f"cargo test -p office-engine {slug_underscore}",
            "verification_script": f"./scripts/verify_pod.sh {slug}",
            "required_test_cases": [
                {"name": f"test_{slug_underscore}_happy_path", "description": details["test_scenarios"][0]},
                {"name": f"test_{slug_underscore}_edge_case", "description": details["test_scenarios"][1] if len(details["test_scenarios"]) > 1 else "Kiểm thử biên dữ liệu"},
                {"name": f"test_{slug_underscore}_invalid_magic_bytes", "description": "Kiểm thử tệp giả mạo magic bytes, kỳ vọng trả về CommandError::InvalidFileSignature"},
                {"name": f"test_{slug_underscore}_cooperative_cancellation", "description": "Kiểm thử kích hoạt AtomicBool hủy tác vụ, kỳ vọng dừng dưới 100ms trả về CommandError::TaskCancelled"},
                {"name": f"test_{slug_underscore}_security_guardrails", "description": "Kiểm định NFR-SEC: Cấm std::net, tokio::net, cấm .unwrap()/.expect() trong luồng nghiệp vụ"}
            ]
        },
        "leptos_ui_zone2": {
            "component_path": f"crates/office-ui/src/components/{domain_folder}/{slug_underscore}_zone2.rs",
            "framework": "Leptos 0.8 CSR WebAssembly",
            "reactive_rules": "Dùng signal() hoặc RwSignal::new(), TUYỆT ĐỐI CẤM cú pháp Leptos 0.5/0.6 (cx: Scope, create_signal).",
            "ime_safe_guideline": "Bắt buộc dùng ImeSafeTextInput lắng nghe compositionstart/compositionend để bảo vệ bộ gõ tiếng Việt (UniKey, EVKey, OpenKey).",
            "dropzone": {
                "accepted_extensions": [m.split("/")[-1] for m in details["input_mimes"] if "/" in m],
                "tauri_drag_drop_event": "tauri://drag-drop",
                "metadata_inspection_command": "inspect_file_metadata"
            },
            "controls": details["zone2_controls"],
            "hotkeys": {
                "dispatch": "Ctrl + Enter",
                "cancel": "Esc",
                "save_preset": "Ctrl + S",
                "open_output_folder": "Ctrl + O"
            },
            "sqlite_preset": {
                "table": "tool_presets",
                "tool_slug": slug,
                "supports_default": True
            }
        },
        "nfr_guardrails": {
            "nfr_sec_01": "100% On-Device & Zero Network Egress",
            "nfr_sec_03": "In-Memory Scrubbing with Zeroize",
            "nfr_sec_04": "Deep Magic Bytes Validation via infer",
            "nfr_sec_05": "Decompression & Image Bomb Defense",
            "nfr_perf_02": "Non-blocking Main Thread UI (60 FPS)",
            "nfr_perf_03": "Progress Streaming via Channel (100-200ms)",
            "nfr_com_02": "IME-Safe Typing (UniKey / EVKey)"
        }
    }
    return payload

def main():
    parser = argparse.ArgumentParser(description="Sinh tệp hàng đợi tasks_queue.jsonl từ 02-srs.md")
    parser.add_argument("--srs", default="office-utility-vn/docs/specs/02-srs.md", help="Đường dẫn tới 02-srs.md")
    parser.add_argument("--output", default="office-utility-vn/tasks_queue.jsonl", help="Đường dẫn tệp tasks_queue.jsonl đầu ra")
    parser.add_argument("--copy-to-root", action="store_true", help="Đồng bộ bản sao sang tasks_queue.jsonl ở thư mục gốc")
    parser.add_argument("--sort", choices=["domain-moscow", "moscow-domain"], default="domain-moscow",
                        help="Tiêu chí sắp xếp: domain-moscow (phân 5 miền, mỗi miền sắp Must->Should->Could) hoặc moscow-domain (ưu tiên MoSCoW toàn cục)")

    args = parser.parse_args()

    srs_path = args.srs
    if not os.path.isfile(srs_path):
        # Thử tìm tương đối
        alt_path = "docs/specs/02-srs.md"
        if os.path.isfile(alt_path):
            srs_path = alt_path
        else:
            print(f"❌ Không tìm thấy tệp đặc tả 02-srs.md tại: {srs_path}")
            sys.exit(1)

    print(f"📖 Đang đọc bảng Markdown trong: {srs_path}...")
    rows = parse_srs_markdown(srs_path)
    print(f"✅ Đã quét thành công {len(rows)} công cụ từ bảng đặc tả chức năng.")

    payloads = [build_atomic_task_payload(row) for row in rows]

    if args.sort == "domain-moscow":
        # Sắp xếp theo Miền trước (1->5), trong mỗi miền sắp theo MoSCoW (Must:1 -> Should:2 -> Could:3), sau đó theo mã FR
        payloads.sort(key=lambda x: (x["domain_order"], x["priority_order"], x["tool_code"]))
    else:
        # Sắp xếp theo MoSCoW trước (Must:1 -> Should:2 -> Could:3), trong mỗi mức sắp theo Miền (1->5), sau đó theo mã FR
        payloads.sort(key=lambda x: (x["priority_order"], x["domain_order"], x["tool_code"]))

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, "w", encoding="utf-8") as f:
        for p in payloads:
            f.write(json.dumps(p, ensure_ascii=False) + "\n")

    print(f"🎉 Đã sinh tệp hàng đợi công việc: {output_path} ({len(payloads)} tác vụ)")

    if args.copy_to_root:
        try:
            root_path = Path("tasks_queue.jsonl")
            with open(root_path, "w", encoding="utf-8") as f:
                for p in payloads:
                    f.write(json.dumps(p, ensure_ascii=False) + "\n")
            print(f"📋 Đã đồng bộ bản sao sang: {root_path}")
        except Exception as e:
            print(f"⚠️ Không thể ghi tasks_queue.jsonl ở root ({e})")

    # Báo cáo thống kê
    domain_counts = {}
    priority_counts = {}
    for p in payloads:
        d = p["domain"]
        pr = p["priority"]
        domain_counts[d] = domain_counts.get(d, 0) + 1
        priority_counts[pr] = priority_counts.get(pr, 0) + 1

    print("\n--- BÁO CÁO PHÂN PHỐI 5 MIỀN ---")
    for d, cnt in domain_counts.items():
        print(f"  • {d:15}: {cnt:2} công cụ")

    print("\n--- BÁO CÁO PHÂN BỔ ĐỘ ƯU TIÊN MoSCoW ---")
    for pr, cnt in priority_counts.items():
        print(f"  • {pr:15}: {cnt:2} công cụ")

    print(f"\n✨ TỔNG CỘNG: {len(payloads)} Atomic Task Payloads sẵn sàng cho Subagent!")

if __name__ == "__main__":
    main()
