pub mod gross_net_salary_calculator;
pub mod number_to_words;
pub mod vietnamese_number_to_words;
pub mod vietqr_napas247_generator;
pub mod vn_address_normalizer;
pub mod vn_cccd_mst_validator;
pub mod vn_einvoice_xml_reader;
pub mod vn_loan_amortization;
pub mod vn_working_days_calc;

pub use gross_net_salary_calculator::{
    calculate_gross_to_net, calculate_pit, GrossNetSalaryCalculatorTool, SalaryBreakdown,
    SalaryCalculatorConfig,
};
pub use number_to_words::{
    CurrencyUnit, Dialect, NumberToWordsConfig, VietnameseNumberToWordsTool,
    format_vietnamese_number,
};
pub use vietqr_napas247_generator::{
    build_vietqr_payload, crc16_ccitt_false, VietQrConfig, VietQrNapas247GeneratorTool,
};
pub use vn_address_normalizer::{
    jaro_winkler_similarity, normalize_vietnamese_address, AddressNormalizerOptions, AdminModel,
    NormalizedAddress, VnAddressNormalizerTool,
};
pub use vn_cccd_mst_validator::{
    process_cccd_or_mst, validate_and_decode_cccd, validate_mst, CccdDecoded,
    CccdMstValidatorOptions, MstValidationResult, ValidationOutput, VnCccdMstValidatorTool,
};
pub use vn_einvoice_xml_reader::{
    parse_einvoice_xml, EinvoiceXmlReaderOptions, InvoiceItem, InvoiceParty, InvoiceSummary,
    VnEinvoiceXmlReaderTool,
};
pub use vn_loan_amortization::{
    calculate_loan_amortization, AmortizationMethod, AmortizationPeriod, LoanAmortizationOptions,
    LoanAmortizationSummary, VnLoanAmortizationTool,
};
pub use vn_working_days_calc::{
    calculate_working_days, check_holiday, parse_date_flexible, HolidayDetail, VnWorkingDaysCalcTool,
    WorkingDaysOptions, WorkingDaysResult,
};

