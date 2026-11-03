pub mod layout;
pub mod sidebar;
pub mod state;

pub use layout::{VnOfficeLayout, VnOfficeLegalPanel};
pub use sidebar::VnOfficeSidebar;
pub use state::{BankReference, VnCalculationRecord, VnLegalRegulations, VnOfficeState, VnToolInputs, POPULAR_VN_BANKS};

use leptos::prelude::*;

/// Micro-App 1: VN Office — Trợ lý Nghiệp vụ Kế toán, Thuế, Ngân hàng & Văn bản quy chuẩn Việt Nam.
///
/// Quản lý 11 công cụ nghiệp vụ:
/// 1. `vietnamese-number-to-words` (Đọc số tiền thành chữ chuẩn)
/// 2. `gross-net-salary-calculator` (Tính lương Gross - Net & Thuế)
/// 3. `vietqr-napas247-generator` (Tạo mã VietQR Napas247)
/// 4. `vn-einvoice-xml-reader` (Đọc hóa đơn điện tử XML VN)
/// 5. `vn-cccd-mst-validator` (Kiểm tra hợp lệ CCCD & Mã số thuế)
/// 6. `vn-address-normalizer` (Chuẩn hóa địa giới hành chính VN)
/// 7. `vn-working-days-calc` (Tính ngày làm việc trừ ngày lễ VN)
/// 8. `vn-loan-amortization` (Bảng tính trả nợ vay ngân hàng)
/// 9. `vn-bank-bin-mapper` (Tra cứu mã định danh ngân hàng)
/// 10. `vietnamese-font-converter` (Chuyển mã font TCVN3/VNI sang Unicode)
/// 11. `encoding-mojibake-fixer` (Sửa lỗi hiển thị chữ Mojibake)
#[component]
pub fn VnOfficeApp(
    #[prop(into, optional)] state: Option<VnOfficeState>,
    #[prop(into, optional)] initial_tool: Option<String>,
    #[prop(into)] on_back: Callback<()>,
) -> impl IntoView {
    let app_state = state.unwrap_or_else(|| VnOfficeState::with_initial_tool(initial_tool.clone()));
    if let Some(ref slug) = initial_tool {
        app_state.select_tool(slug);
    }

    view! {
        <layout::VnOfficeLayout state=app_state on_back=on_back />
    }
}
