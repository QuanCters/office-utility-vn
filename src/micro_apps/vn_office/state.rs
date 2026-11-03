use leptos::prelude::*;
use crate::app::ToolItem;
use crate::micro_apps::VN_OFFICE_TOOLS;

/// Các tham số quy định pháp lý dùng chung cho VN Office (TNCN, BHXH, Thuế, Lương).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VnLegalRegulations {
    /// Mức giảm trừ gia cảnh cho bản thân: 11.000.000 VND / tháng (Nghị quyết 954/2020/UBTVQH14)
    pub personal_deduction: u64,
    /// Mức giảm trừ gia cảnh cho mỗi người phụ thuộc: 4.400.000 VND / tháng
    pub dependent_deduction: u64,
    /// Tỷ lệ trích nộp BHXH của người lao động: 8.0%
    pub bhxh_employee_rate: f64,
    /// Tỷ lệ trích nộp BHYT của người lao động: 1.5%
    pub bhyt_employee_rate: f64,
    /// Tỷ lệ trích nộp BHTN của người lao động: 1.0%
    pub bhtn_employee_rate: f64,
    /// Tỷ lệ trích nộp BHXH của người sử dụng lao động: 17.5%
    pub bhxh_employer_rate: f64,
    /// Tỷ lệ trích nộp BHYT của người sử dụng lao động: 3.0%
    pub bhyt_employer_rate: f64,
    /// Tỷ lệ trích nộp BHTN của người sử dụng lao động: 1.0%
    pub bhtn_employer_rate: f64,
    /// Mức lương cơ sở: 2.340.000 VND (Nghị định 73/2024/NĐ-CP)
    pub base_salary: u64,
    /// Mức lương trần đóng BHXH/BHYT tối đa (20 x Lương cơ sở): 46.800.000 VND
    pub max_insurable_salary: u64,
}

impl Default for VnLegalRegulations {
    fn default() -> Self {
        Self {
            personal_deduction: 11_000_000,
            dependent_deduction: 4_400_000,
            bhxh_employee_rate: 0.08,
            bhyt_employee_rate: 0.015,
            bhtn_employee_rate: 0.01,
            bhxh_employer_rate: 0.175,
            bhyt_employer_rate: 0.03,
            bhtn_employer_rate: 0.01,
            base_salary: 2_340_000,
            max_insurable_salary: 46_800_000,
        }
    }
}

/// Thông tin tra cứu ngân hàng nhanh (mã BIN, tên viết tắt, tên đầy đủ, SWIFT).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BankReference {
    pub bin: &'static str,
    pub short_name: &'static str,
    pub full_name: &'static str,
    pub swift: &'static str,
}

/// Bản ghi lịch sử tính toán và đối soát dữ liệu gần nhất của VN Office.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VnCalculationRecord {
    pub id: String,
    pub tool_slug: String,
    pub tool_name: String,
    pub input_summary: String,
    pub output_result: String,
    pub timestamp: String,
}

/// Danh sách hơn 50 ngân hàng Việt Nam phục vụ tra cứu nhanh NAPAS 247 và mã BIN.
pub const POPULAR_VN_BANKS: &[BankReference] = &[
    BankReference { bin: "970436", short_name: "VCB", full_name: "Vietcombank (Ngoại thương VN)", swift: "BFTVVNVX" },
    BankReference { bin: "970415", short_name: "CTG", full_name: "VietinBank (Công thương VN)", swift: "ICBVVNVX" },
    BankReference { bin: "970418", short_name: "BIDV", full_name: "BIDV (Đầu tư và Phát triển)", swift: "BIDVVNVX" },
    BankReference { bin: "970405", short_name: "VBA", full_name: "Agribank (Nông nghiệp & PTNT)", swift: "VBAAVNVX" },
    BankReference { bin: "970407", short_name: "TCB", full_name: "Techcombank (Kỹ thương VN)", swift: "VTCBVNVX" },
    BankReference { bin: "970422", short_name: "MB", full_name: "MBBank (Quân đội)", swift: "MBBCVNVX" },
    BankReference { bin: "970416", short_name: "ACB", full_name: "ACB (Á Châu)", swift: "ASCBVNVX" },
    BankReference { bin: "970432", short_name: "VPB", full_name: "VPBank (Việt Nam Thịnh Vượng)", swift: "VPBVVNVX" },
    BankReference { bin: "970423", short_name: "TPB", full_name: "TPBank (Tiên Phong)", swift: "TPBVVNVX" },
    BankReference { bin: "970403", short_name: "STB", full_name: "Sacombank (Sài Gòn Thương Tín)", swift: "SGCBVNVX" },
    BankReference { bin: "970437", short_name: "HDB", full_name: "HDBank (Phát triển TP.HCM)", swift: "HDBCVNVX" },
    BankReference { bin: "970448", short_name: "OCB", full_name: "OCB (Phương Đông)", swift: "OCBCVNVX" },
    BankReference { bin: "970441", short_name: "VIB", full_name: "VIB (Quốc tế)", swift: "VNIBVNVX" },
    BankReference { bin: "970443", short_name: "SHB", full_name: "SHB (Sài Gòn - Hà Nội)", swift: "SHBVVNVX" },
    BankReference { bin: "970429", short_name: "SCB", full_name: "SCB (Sài Gòn)", swift: "SACLVNVX" },
    BankReference { bin: "970454", short_name: "VCCB", full_name: "BVBank (Bản Việt)", swift: "VCCBVNVX" },
    BankReference { bin: "970428", short_name: "NAB", full_name: "Nam A Bank (Nam Á)", swift: "NAMAVNVX" },
    BankReference { bin: "970431", short_name: "EIB", full_name: "Eximbank (Xuất Nhập khẩu)", swift: "EBVIVNVX" },
    BankReference { bin: "970440", short_name: "SEAB", full_name: "SeABank (Đông Nam Á)", swift: "SEABVNVX" },
    BankReference { bin: "970425", short_name: "ABB", full_name: "An Bình (ABBANK)", swift: "ABBKVNVX" },
    BankReference { bin: "970426", short_name: "MSB", full_name: "MSB (Hàng Hải)", swift: "MMSBVNVX" },
    BankReference { bin: "970438", short_name: "BVB", full_name: "BaoViet Bank (Bảo Việt)", swift: "BVBKVNVX" },
    BankReference { bin: "970414", short_name: "OJB", full_name: "OceanBank (Đại Dương)", swift: "OCEBVNVX" },
    BankReference { bin: "970408", short_name: "GPB", full_name: "GPBank (Dầu Khí Toàn Cầu)", swift: "GPBNVNVX" },
    BankReference { bin: "970400", short_name: "SGB", full_name: "Saigonbank (Sài Gòn Công thương)", swift: "SGICVNVX" },
    BankReference { bin: "970452", short_name: "KLB", full_name: "Kienlongbank (Kiên Long)", swift: "KLBLVNVX" },
    BankReference { bin: "970430", short_name: "PGB", full_name: "PGBank (Thịnh vượng và Phát triển)", swift: "PGBLVNVX" },
    BankReference { bin: "970449", short_name: "LPB", full_name: "LPBank (Lộc Phát VN)", swift: "LPBKVNVX" },
    BankReference { bin: "970434", short_name: "IVB", full_name: "Indovina Bank", swift: "INDOVNVX" },
    BankReference { bin: "970455", short_name: "IBK", full_name: "Industrial Bank of Korea HN", swift: "IBKOVNVX" },
    BankReference { bin: "970456", short_name: "IBK-HCM", full_name: "Industrial Bank of Korea HCM", swift: "IBKOVNVX" },
    BankReference { bin: "970458", short_name: "UOB", full_name: "United Overseas Bank VN", swift: "UOVBVNVX" },
    BankReference { bin: "970457", short_name: "Wooribank", full_name: "Woori Bank Việt Nam", swift: "HVBCVNVX" },
    BankReference { bin: "970421", short_name: "VRB", full_name: "Việt - Nga (VRB)", swift: "VRBAVNVX" },
    BankReference { bin: "970442", short_name: "HLBVN", full_name: "Hong Leong Bank VN", swift: "HLBBVNVX" },
    BankReference { bin: "970410", short_name: "SCVN", full_name: "Standard Chartered VN", swift: "SCBLVNVX" },
    BankReference { bin: "970439", short_name: "PBVN", full_name: "Public Bank Vietnam", swift: "VIDOVNVX" },
    BankReference { bin: "970424", short_name: "SHBVN", full_name: "Shinhan Bank Việt Nam", swift: "SHBKVNVX" },
    BankReference { bin: "970409", short_name: "BAB", full_name: "Bac A Bank (Bắc Á)", swift: "NASCVNVX" },
    BankReference { bin: "970446", short_name: "COOP", full_name: "Co-opBank (Hợp tác xã)", swift: "CPBKVNVX" },
    BankReference { bin: "970462", short_name: "KBHN", full_name: "Kookmin Bank Hà Nội", swift: "CZNBVNVX" },
    BankReference { bin: "970463", short_name: "KBHCM", full_name: "Kookmin Bank TP.HCM", swift: "CZNBVNVX" },
    BankReference { bin: "970466", short_name: "KEBHN", full_name: "KEB Hana Bank Hà Nội", swift: "KOEXVNVX" },
    BankReference { bin: "970467", short_name: "KEBHCM", full_name: "KEB Hana Bank TP.HCM", swift: "KOEXVNVX" },
    BankReference { bin: "970468", short_name: "CIMB", full_name: "CIMB Bank Việt Nam", swift: "CIBBVNVX" },
    BankReference { bin: "970470", short_name: "DBS", full_name: "DBS Bank HCM", swift: "DBSSVNVX" },
    BankReference { bin: "970469", short_name: "Nonghyup", full_name: "Nonghyup Bank HN", swift: "NACFVNVX" },
    BankReference { bin: "546034", short_name: "VBSP", full_name: "NH Chính sách Xã hội", swift: "VBSPVNVX" },
    BankReference { bin: "546035", short_name: "VDB", full_name: "NH Phát triển Việt Nam", swift: "VDBKVNVX" },
];

/// Tập hợp các tín hiệu lưu giữ giá trị nhập liệu và kết quả tính toán
/// cho toàn bộ 11 công cụ của VN Office, đảm bảo không bị mất trạng thái khi điều hướng qua lại Launcher.
#[derive(Clone, Copy, Debug)]
pub struct VnToolInputs {
    // 1. vietnamese-number-to-words
    pub num_input: (ReadSignal<String>, WriteSignal<String>),
    pub num_dialect: (ReadSignal<String>, WriteSignal<String>),
    pub num_dong_chan: (ReadSignal<bool>, WriteSignal<bool>),
    pub num_result: (ReadSignal<String>, WriteSignal<String>),
    pub is_ime_composing: (ReadSignal<bool>, WriteSignal<bool>),

    // 2. gross-net-salary-calculator
    pub salary_input: (ReadSignal<String>, WriteSignal<String>),
    pub salary_region: (ReadSignal<u8>, WriteSignal<u8>),
    pub salary_dependents: (ReadSignal<u32>, WriteSignal<u32>),
    pub salary_result: (ReadSignal<String>, WriteSignal<String>),

    // 3. vietqr-napas247-generator
    pub qr_bank_bin: (ReadSignal<String>, WriteSignal<String>),
    pub qr_account: (ReadSignal<String>, WriteSignal<String>),
    pub qr_amount: (ReadSignal<String>, WriteSignal<String>),
    pub qr_message: (ReadSignal<String>, WriteSignal<String>),
    pub qr_payload: (ReadSignal<String>, WriteSignal<String>),

    // 4. vn-einvoice-xml-reader
    pub xml_content: (ReadSignal<String>, WriteSignal<String>),
    pub xml_verify: (ReadSignal<bool>, WriteSignal<bool>),
    pub xml_result: (ReadSignal<String>, WriteSignal<String>),

    // 5. vn-cccd-mst-validator
    pub cccd_input: (ReadSignal<String>, WriteSignal<String>),
    pub cccd_result: (ReadSignal<String>, WriteSignal<String>),

    // 6. vn-address-normalizer
    pub addr_input: (ReadSignal<String>, WriteSignal<String>),
    pub addr_model: (ReadSignal<String>, WriteSignal<String>),
    pub addr_result: (ReadSignal<String>, WriteSignal<String>),

    // 7. vn-working-days-calc
    pub wd_start: (ReadSignal<String>, WriteSignal<String>),
    pub wd_end: (ReadSignal<String>, WriteSignal<String>),
    pub wd_exclude_sat: (ReadSignal<bool>, WriteSignal<bool>),
    pub wd_result: (ReadSignal<String>, WriteSignal<String>),

    // 8. vn-loan-amortization
    pub loan_principal: (ReadSignal<String>, WriteSignal<String>),
    pub loan_rate: (ReadSignal<String>, WriteSignal<String>),
    pub loan_term: (ReadSignal<u32>, WriteSignal<u32>),
    pub loan_method: (ReadSignal<String>, WriteSignal<String>),
    pub loan_result: (ReadSignal<String>, WriteSignal<String>),

    // 9. vn-bank-bin-mapper
    pub bank_query: (ReadSignal<String>, WriteSignal<String>),
    pub bank_result: (ReadSignal<String>, WriteSignal<String>),

    // 10. vietnamese-font-converter
    pub font_source: (ReadSignal<String>, WriteSignal<String>),
    pub font_target: (ReadSignal<String>, WriteSignal<String>),

    // 11. encoding-mojibake-fixer
    pub mojibake_auto: (ReadSignal<bool>, WriteSignal<bool>),
}

impl Default for VnToolInputs {
    fn default() -> Self {
        Self {
            num_input: signal("1250000".to_string()),
            num_dialect: signal("North".to_string()),
            num_dong_chan: signal(true),
            num_result: signal("Một triệu hai trăm năm mươi nghìn đồng chẵn.".to_string()),
            is_ime_composing: signal(false),

            salary_input: signal("25000000".to_string()),
            salary_region: signal(1u8),
            salary_dependents: signal(0u32),
            salary_result: signal(String::new()),

            qr_bank_bin: signal("970436".to_string()),
            qr_account: signal("0011001234567".to_string()),
            qr_amount: signal("500000".to_string()),
            qr_message: signal("Chuyen khoan thanh toan".to_string()),
            qr_payload: signal(String::new()),

            xml_content: signal(String::new()),
            xml_verify: signal(true),
            xml_result: signal(String::new()),

            cccd_input: signal(String::new()),
            cccd_result: signal(String::new()),

            addr_input: signal(String::new()),
            addr_model: signal("ThreeTier".to_string()),
            addr_result: signal(String::new()),

            wd_start: signal("2026-01-01".to_string()),
            wd_end: signal("2026-01-31".to_string()),
            wd_exclude_sat: signal(true),
            wd_result: signal(String::new()),

            loan_principal: signal("100000000".to_string()),
            loan_rate: signal("8.5".to_string()),
            loan_term: signal(12u32),
            loan_method: signal("ReducingBalance".to_string()),
            loan_result: signal(String::new()),

            bank_query: signal(String::new()),
            bank_result: signal(String::new()),

            font_source: signal("auto".to_string()),
            font_target: signal("unicode".to_string()),

            mojibake_auto: signal(true),
        }
    }
}

/// Trạng thái hoạt động của Micro-App VN Office.
#[derive(Clone, Copy, Debug)]
pub struct VnOfficeState {
    /// Slug của công cụ nghiệp vụ đang được chọn.
    pub active_tool: RwSignal<String>,
    /// Cờ hiển thị chế độ Workflow Pipeline Macro DAG nội bộ.
    pub is_workflow_active: RwSignal<bool>,
    /// Trạng thái đang thực thi tác vụ.
    pub is_running: RwSignal<bool>,
    /// Thông điệp kết quả hoặc trạng thái thực thi.
    pub status_message: RwSignal<String>,
    /// Các tham số quy định pháp lý dùng chung.
    pub regulations: RwSignal<VnLegalRegulations>,
    /// Bộ lọc tìm kiếm nhanh trong sidebar nội bộ.
    pub sidebar_filter: RwSignal<String>,
    /// Cờ hiển thị / ẩn panel tra cứu quy định pháp lý bên phải.
    pub is_legal_panel_open: RwSignal<bool>,
    /// Lịch sử các kết quả tính toán gần nhất của micro-app.
    pub calculation_history: RwSignal<Vec<VnCalculationRecord>>,
    /// Dữ liệu nhập liệu và kết quả in-memory cho toàn bộ 11 công cụ.
    pub tool_inputs: VnToolInputs,
}

impl Default for VnOfficeState {
    fn default() -> Self {
        Self::new()
    }
}

impl VnOfficeState {
    /// Khởi tạo trạng thái với công cụ mặc định đầu tiên: "vietnamese-number-to-words".
    pub fn new() -> Self {
        Self::with_initial_tool(None)
    }

    /// Khởi tạo trạng thái với công cụ ban đầu tùy chọn.
    pub fn with_initial_tool(initial_tool: Option<String>) -> Self {
        let initial_slug = initial_tool
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "vietnamese-number-to-words".to_string());

        Self {
            active_tool: RwSignal::new(initial_slug),
            is_workflow_active: RwSignal::new(false),
            is_running: RwSignal::new(false),
            status_message: RwSignal::new(String::new()),
            regulations: RwSignal::new(VnLegalRegulations::default()),
            sidebar_filter: RwSignal::new(String::new()),
            is_legal_panel_open: RwSignal::new(true),
            calculation_history: RwSignal::new(Vec::new()),
            tool_inputs: VnToolInputs::default(),
        }
    }

    /// Chọn một công cụ nghiệp vụ và đóng chế độ workflow.
    pub fn select_tool(&self, slug: &str) {
        self.active_tool.set(slug.to_string());
        self.is_workflow_active.set(false);
    }

    /// Mở màn hình thiết kế chuỗi Workflow Macro DAG.
    pub fn open_workflow(&self) {
        self.is_workflow_active.set(true);
    }

    /// Đóng màn hình workflow trở về công cụ hiện tại.
    pub fn close_workflow(&self) {
        self.is_workflow_active.set(false);
    }

    /// Lấy slug của công cụ đang được kích hoạt.
    pub fn active_tool_slug(&self) -> String {
        self.active_tool.get()
    }

    /// Kiểm tra xem chế độ workflow có đang hiển thị hay không.
    pub fn is_workflow(&self) -> bool {
        self.is_workflow_active.get()
    }

    /// Kiểm tra xem tác vụ có đang chạy hay không.
    pub fn is_running(&self) -> bool {
        self.is_running.get()
    }

    /// Lấy chuỗi lọc sidebar hiện tại.
    pub fn sidebar_filter(&self) -> String {
        self.sidebar_filter.get()
    }

    /// Kiểm tra xem panel tra cứu pháp lý có đang mở hay không.
    pub fn is_legal_panel_open(&self) -> bool {
        self.is_legal_panel_open.get()
    }

    /// Lấy tham số quy định pháp lý hiện tại.
    pub fn regulations(&self) -> VnLegalRegulations {
        self.regulations.get()
    }

    /// Danh sách tĩnh 11 công cụ thuộc phạm vi VN Office.
    pub fn tools(&self) -> &'static [ToolItem] {
        VN_OFFICE_TOOLS
    }

    /// Tìm kiếm thông tin công cụ theo slug.
    pub fn find_tool(&self, slug: &str) -> Option<&'static ToolItem> {
        VN_OFFICE_TOOLS.iter().find(|t| t.slug == slug)
    }

    /// Lấy danh sách ngân hàng mẫu.
    pub fn bank_references(&self) -> &'static [BankReference] {
        POPULAR_VN_BANKS
    }

    /// Ghi nhận một kết quả tính toán vào lịch sử gần nhất (tối đa lưu 50 mục).
    pub fn record_calculation(&self, tool_slug: &str, tool_name: &str, input_summary: &str, output_result: &str) {
        let record = VnCalculationRecord {
            id: format!("calc-{}", self.calculation_history.with(|h| h.len() + 1)),
            tool_slug: tool_slug.to_string(),
            tool_name: tool_name.to_string(),
            input_summary: input_summary.to_string(),
            output_result: output_result.to_string(),
            timestamp: "Vừa xong".to_string(),
        };
        self.record_calculation_item(record);
    }

    /// Thêm trực tiếp một bản ghi tính toán vào lịch sử.
    pub fn record_calculation_item(&self, record: VnCalculationRecord) {
        self.calculation_history.update(|h| {
            h.insert(0, record);
            if h.len() > 50 {
                h.truncate(50);
            }
        });
    }

    /// Lấy danh sách các bản ghi tính toán gần nhất.
    pub fn calculation_history(&self) -> Vec<VnCalculationRecord> {
        self.calculation_history.get()
    }

    /// Lấy bản ghi tính toán mới nhất (nếu có).
    pub fn latest_calculation(&self) -> Option<VnCalculationRecord> {
        self.calculation_history.with(|h| h.first().cloned())
    }

    /// Xóa toàn bộ lịch sử tính toán.
    pub fn clear_calculation_history(&self) {
        self.calculation_history.set(Vec::new());
    }
}
