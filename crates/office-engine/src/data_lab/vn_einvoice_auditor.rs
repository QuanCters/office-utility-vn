use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Thông tin một mặt hàng/dịch vụ trong hóa đơn điện tử TT78
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct InvoiceItem {
    pub stt: Option<u32>,
    pub ten_hang: String,
    pub don_vi_tinh: Option<String>,
    pub so_luong: Option<f64>,
    pub don_gia: Option<f64>,
    pub thanh_tien: f64,
    pub thue_suat: Option<String>,
}

/// Thông tin đơn vị (Bên bán hoặc Bên mua)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PartyInfo {
    pub mst: String,
    pub ten: String,
    pub dia_chi: String,
}

/// Dữ liệu bóc tách từ hóa đơn điện tử XML Thông tư 78/2021/TT-BTC & Nghị định 123/2020/NĐ-CP
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct VnEinvoiceData {
    pub ky_hieu: String,
    pub so_hoa_don: String,
    pub ngay_lap: String,
    pub don_vi_tiet_te: String,
    pub ben_ban: PartyInfo,
    pub ben_mua: PartyInfo,
    pub danh_sach_hang_hoa: Vec<InvoiceItem>,
    pub tong_tien_chua_thue: f64,
    pub tong_tien_thue: f64,
    pub tong_tien_thanh_toan: f64,
    pub tong_tien_chu: String,
    pub co_chu_ky_so: bool,
    #[serde(default)]
    pub file_path: Option<String>,
}

/// Bản ghi hóa đơn trong sổ sách / bảng kê nội bộ để đối soát 3 chiều
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LedgerInvoiceRecord {
    pub mst_ben_ban: String,
    pub so_hoa_don: String,
    pub tong_tien: f64,
    #[serde(default)]
    pub ky_hieu: Option<String>,
    #[serde(default)]
    pub ngay_lap: Option<String>,
    #[serde(default)]
    pub ghi_chu: Option<String>,
}

/// Trạng thái phân loại đối soát 3 chiều
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationStatus {
    /// Nhóm 1: Khớp MST bên bán, số HĐ và tổng tiền thanh toán (trong dung sai cho phép)
    KhopHoanToan,
    /// Nhóm 2: Khớp MST và số HĐ nhưng lệch tổng tiền thanh toán
    KhopMaLechTien,
    /// Nhóm 3: Hóa đơn có trong tệp XML nhưng thiếu trong bảng kê nội bộ
    ChiCoOXml,
    /// Nhóm 4: Hóa đơn có trong bảng kê nội bộ nhưng thiếu tệp XML
    ChiCoOBangKe,
}

/// Chi tiết một kết quả đối soát
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationItem {
    pub status: ReconciliationStatus,
    pub mst: String,
    pub so_hoa_don: String,
    pub tien_xml: Option<f64>,
    pub tien_bang_ke: Option<f64>,
    pub lech_tien: Option<f64>,
    pub ly_do: String,
    pub file_xml: Option<String>,
}

/// Báo cáo tổng hợp đối soát hóa đơn
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReconciliationReport {
    pub total_xml: usize,
    pub total_bang_ke: usize,
    pub khop_hoan_toan_count: usize,
    pub khop_ma_lech_tien_count: usize,
    pub chi_co_xml_count: usize,
    pub chi_co_bang_ke_count: usize,
    pub items: Vec<ReconciliationItem>,
    pub parsed_invoices: Vec<VnEinvoiceData>,
}

fn default_true() -> bool {
    true
}

/// Cấu hình tùy chọn cho công cụ bóc tách & đối soát hóa đơn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VnEinvoiceAuditorOptions {
    /// Dung sai số tiền cho phép khi đối soát (mặc định 0.0 VNĐ)
    #[serde(default)]
    pub tolerance_amount: f64,
    /// Xuất báo cáo tổng hợp ra tệp JSON
    #[serde(default = "default_true")]
    pub export_summary_json: bool,
    /// Danh sách bản ghi bảng kê để đối soát nếu có
    #[serde(default)]
    pub compare_records: Vec<LedgerInvoiceRecord>,
}

impl Default for VnEinvoiceAuditorOptions {
    fn default() -> Self {
        Self {
            tolerance_amount: 0.0,
            export_summary_json: true,
            compare_records: Vec::new(),
        }
    }
}

pub struct VnEinvoiceAuditorTool;

impl Default for VnEinvoiceAuditorTool {
    fn default() -> Self {
        Self
    }
}

/// Hàm phân tích số tiền linh hoạt hỗ trợ số chuẩn W3C XML và định dạng Việt Nam
pub fn parse_amount(s: &str) -> Option<f64> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }

    let cleaned: String = trimmed
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '₫' && *c != '$' && *c != 'V' && *c != 'N' && *c != 'D')
        .collect();

    if cleaned.is_empty() {
        return None;
    }

    // 1. Chuẩn W3C XML Decimal: e.g. 11000000 hoặc 11000000.50
    if let Ok(v) = cleaned.parse::<f64>() {
        return Some(v);
    }

    // 2. Chứa cả dấu chấm và phẩy (vd: 11.000.000,50 hoặc 11,000,000.50)
    if cleaned.contains('.') && cleaned.contains(',') {
        let last_dot = cleaned.rfind('.').unwrap_or(0);
        let last_comma = cleaned.rfind(',').unwrap_or(0);
        if last_comma > last_dot {
            // Định dạng VN: chấm là phân cách nghìn, phẩy là thập phân
            let s_norm = cleaned.replace('.', "").replace(',', ".");
            return s_norm.parse::<f64>().ok();
        } else {
            // Định dạng US: phẩy là phân cách nghìn, chấm là thập phân
            let s_norm = cleaned.replace(',', "");
            return s_norm.parse::<f64>().ok();
        }
    } else if cleaned.contains('.') {
        if cleaned.matches('.').count() > 1 {
            let s_norm = cleaned.replace('.', "");
            return s_norm.parse::<f64>().ok();
        }
    } else if cleaned.contains(',') {
        if cleaned.matches(',').count() > 1 {
            let s_norm = cleaned.replace(',', "");
            return s_norm.parse::<f64>().ok();
        }
        let s_norm = cleaned.replace(',', ".");
        return s_norm.parse::<f64>().ok();
    }

    None
}

/// Chuẩn hóa số hóa đơn để so sánh (loại bỏ các số 0 ở đầu)
pub fn normalize_invoice_number(s: &str) -> String {
    let trimmed = s.trim();
    let stripped = trimmed.trim_start_matches('0');
    if stripped.is_empty() {
        if trimmed.is_empty() {
            String::new()
        } else {
            "0".to_string()
        }
    } else {
        stripped.to_string()
    }
}

/// Chuẩn hóa mã số thuế (loại bỏ khoảng trắng, dấu gạch ngang, đưa về chữ in hoa)
pub fn normalize_tax_code(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_uppercase()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XmlSection {
    Root,
    GeneralInfo, // TTChung
    Seller,      // NBan
    Buyer,       // NMua
    ItemList,    // DSHHDVu
    Item,        // HHDVu
    Payment,     // TToan
    Signature,   // DSCks / Signature
}

fn apply_xml_text_value(
    invoice: &mut VnEinvoiceData,
    current_item: &mut Option<InvoiceItem>,
    section_stack: &[XmlSection],
    current_tag: &str,
    text_val: &str,
    has_valid_invoice_element: &mut bool,
) {
    if text_val.is_empty() {
        return;
    }
    let current_sec = section_stack.last().copied().unwrap_or(XmlSection::Root);
    match current_sec {
        XmlSection::Item => {
            if let Some(item) = current_item {
                match current_tag {
                    "STT" => {
                        item.stt = text_val.parse::<u32>().ok();
                    }
                    "THHDVu" => {
                        item.ten_hang.push_str(text_val);
                    }
                    "DVTinh" => {
                        item.don_vi_tinh = Some(text_val.to_string());
                    }
                    "SLuong" => {
                        item.so_luong = parse_amount(text_val);
                    }
                    "DGia" => {
                        item.don_gia = parse_amount(text_val);
                    }
                    "Tien" => {
                        if let Some(amt) = parse_amount(text_val) {
                            item.thanh_tien = amt;
                        }
                    }
                    "TSuat" => {
                        item.thue_suat = Some(text_val.to_string());
                    }
                    _ => {}
                }
            }
        }
        XmlSection::Seller => {
            match current_tag {
                "MST" | "MSTNBan" => {
                    invoice.ben_ban.mst.push_str(text_val);
                }
                "Ten" | "TenNBan" => {
                    invoice.ben_ban.ten.push_str(text_val);
                }
                "DChi" | "DChiNBan" => {
                    invoice.ben_ban.dia_chi.push_str(text_val);
                }
                _ => {}
            }
        }
        XmlSection::Buyer => {
            match current_tag {
                "MST" | "MSTNMua" => {
                    invoice.ben_mua.mst.push_str(text_val);
                }
                "Ten" | "TenNMua" => {
                    invoice.ben_mua.ten.push_str(text_val);
                }
                "DChi" | "DChiNMua" => {
                    invoice.ben_mua.dia_chi.push_str(text_val);
                }
                _ => {}
            }
        }
        XmlSection::Payment => {
            match current_tag {
                "TgTCThue" => {
                    if let Some(amt) = parse_amount(text_val) {
                        invoice.tong_tien_chua_thue = amt;
                    }
                }
                "TgTThue" => {
                    if let Some(amt) = parse_amount(text_val) {
                        invoice.tong_tien_thue = amt;
                    }
                }
                "TgTTTBSo" => {
                    if let Some(amt) = parse_amount(text_val) {
                        invoice.tong_tien_thanh_toan = amt;
                    }
                }
                "TgTTTBChu" => {
                    invoice.tong_tien_chu.push_str(text_val);
                }
                _ => {}
            }
        }
        XmlSection::GeneralInfo | XmlSection::Root | XmlSection::ItemList | XmlSection::Signature => {
            match current_tag {
                "KHHDon" => {
                    invoice.ky_hieu.push_str(text_val);
                    *has_valid_invoice_element = true;
                }
                "SHDon" => {
                    invoice.so_hoa_don.push_str(text_val);
                    *has_valid_invoice_element = true;
                }
                "NLap" => {
                    invoice.ngay_lap.push_str(text_val);
                }
                "DVTTe" => {
                    invoice.don_vi_tiet_te.push_str(text_val);
                }
                "TgTCThue" => {
                    if let Some(amt) = parse_amount(text_val) {
                        invoice.tong_tien_chua_thue = amt;
                    }
                }
                "TgTThue" => {
                    if let Some(amt) = parse_amount(text_val) {
                        invoice.tong_tien_thue = amt;
                    }
                }
                "TgTTTBSo" => {
                    if let Some(amt) = parse_amount(text_val) {
                        invoice.tong_tien_thanh_toan = amt;
                    }
                }
                "TgTTTBChu" => {
                    invoice.tong_tien_chu.push_str(text_val);
                }
                _ => {}
            }
        }
    }
}

/// Bóc tách tệp XML hóa đơn điện tử chuẩn Thông tư 78
pub fn parse_einvoice_xml(xml_content: &str) -> Result<VnEinvoiceData, CommandError> {
    let trimmed = xml_content.trim();
    if trimmed.is_empty() {
        return Err(CommandError::ValidationFailed {
            reason: "Nội dung tệp hóa đơn XML rỗng".to_string(),
        });
    }

    let mut reader = Reader::from_str(trimmed);
    reader.config_mut().trim_text(true);

    let mut invoice = VnEinvoiceData::default();
    let mut current_item: Option<InvoiceItem> = None;
    let mut section_stack: Vec<XmlSection> = vec![XmlSection::Root];
    let mut current_tag = String::new();
    let mut buf = Vec::new();
    let mut has_valid_invoice_element = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let raw_name = e.name();
                let full_tag = raw_name.as_ref().to_string();
                let local_tag = if let Some(idx) = full_tag.find(':') {
                    full_tag[idx + 1..].to_string()
                } else {
                    full_tag.clone()
                };

                if full_tag.contains("Signature") || local_tag == "Signature" || local_tag == "DSCks" {
                    invoice.co_chu_ky_so = true;
                    section_stack.push(XmlSection::Signature);
                } else if local_tag == "TTChung" {
                    has_valid_invoice_element = true;
                    section_stack.push(XmlSection::GeneralInfo);
                } else if local_tag == "NBan" {
                    has_valid_invoice_element = true;
                    section_stack.push(XmlSection::Seller);
                } else if local_tag == "NMua" {
                    has_valid_invoice_element = true;
                    section_stack.push(XmlSection::Buyer);
                } else if local_tag == "DSHHDVu" {
                    has_valid_invoice_element = true;
                    section_stack.push(XmlSection::ItemList);
                } else if local_tag == "HHDVu" {
                    has_valid_invoice_element = true;
                    current_item = Some(InvoiceItem::default());
                    section_stack.push(XmlSection::Item);
                } else if local_tag == "TToan" {
                    has_valid_invoice_element = true;
                    section_stack.push(XmlSection::Payment);
                }

                current_tag = local_tag;
            }
            Ok(Event::Empty(ref e)) => {
                let raw_name = e.name();
                let full_tag = raw_name.as_ref().to_string();
                let local_tag = if let Some(idx) = full_tag.find(':') {
                    &full_tag[idx + 1..]
                } else {
                    &full_tag
                };

                if full_tag.contains("Signature") || local_tag == "Signature" || local_tag == "DSCks" {
                    invoice.co_chu_ky_so = true;
                }
            }
            Ok(Event::End(ref e)) => {
                let raw_name = e.name();
                let full_tag = raw_name.as_ref().to_string();
                let local_tag = if let Some(idx) = full_tag.find(':') {
                    &full_tag[idx + 1..]
                } else {
                    &full_tag
                };

                match local_tag {
                    "HHDVu" => {
                        if let Some(item) = current_item.take() {
                            invoice.danh_sach_hang_hoa.push(item);
                        }
                        if section_stack.last() == Some(&XmlSection::Item) {
                            section_stack.pop();
                        }
                    }
                    "TTChung" => {
                        if section_stack.last() == Some(&XmlSection::GeneralInfo) {
                            section_stack.pop();
                        }
                    }
                    "NBan" => {
                        if section_stack.last() == Some(&XmlSection::Seller) {
                            section_stack.pop();
                        }
                    }
                    "NMua" => {
                        if section_stack.last() == Some(&XmlSection::Buyer) {
                            section_stack.pop();
                        }
                    }
                    "DSHHDVu" => {
                        if section_stack.last() == Some(&XmlSection::ItemList) {
                            section_stack.pop();
                        }
                    }
                    "TToan" => {
                        if section_stack.last() == Some(&XmlSection::Payment) {
                            section_stack.pop();
                        }
                    }
                    "DSCks" | "Signature" => {
                        if section_stack.last() == Some(&XmlSection::Signature) {
                            section_stack.pop();
                        }
                    }
                    _ => {}
                }

                current_tag.clear();
            }
            Ok(Event::Text(ref e)) => {
                let text_val = match quick_xml::escape::unescape(e.as_ref()) {
                    Ok(cow) => cow.trim().to_string(),
                    Err(_) => e.as_ref().trim().to_string(),
                };
                apply_xml_text_value(
                    &mut invoice,
                    &mut current_item,
                    &section_stack,
                    &current_tag,
                    &text_val,
                    &mut has_valid_invoice_element,
                );
            }
            Ok(Event::CData(ref e)) => {
                let text_val = e.as_ref().trim().to_string();
                apply_xml_text_value(
                    &mut invoice,
                    &mut current_item,
                    &section_stack,
                    &current_tag,
                    &text_val,
                    &mut has_valid_invoice_element,
                );
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(CommandError::ValidationFailed {
                    reason: format!("Lỗi phân tích cú pháp tệp XML: {e}"),
                });
            }
            _ => {}
        }
        buf.clear();
    }

    invoice.ky_hieu = invoice.ky_hieu.trim().to_string();
    invoice.so_hoa_don = invoice.so_hoa_don.trim().to_string();
    invoice.ngay_lap = invoice.ngay_lap.trim().to_string();
    invoice.don_vi_tiet_te = if invoice.don_vi_tiet_te.trim().is_empty() {
        "VND".to_string()
    } else {
        invoice.don_vi_tiet_te.trim().to_string()
    };
    invoice.ben_ban.mst = invoice.ben_ban.mst.trim().to_string();
    invoice.ben_ban.ten = invoice.ben_ban.ten.trim().to_string();
    invoice.ben_ban.dia_chi = invoice.ben_ban.dia_chi.trim().to_string();
    invoice.ben_mua.mst = invoice.ben_mua.mst.trim().to_string();
    invoice.ben_mua.ten = invoice.ben_mua.ten.trim().to_string();
    invoice.ben_mua.dia_chi = invoice.ben_mua.dia_chi.trim().to_string();
    invoice.tong_tien_chu = invoice.tong_tien_chu.trim().to_string();

    // Tự động tính toán tổng thanh toán nếu thẻ tổng thanh toán bằng số bị thiếu
    if invoice.tong_tien_thanh_toan == 0.0 {
        if invoice.tong_tien_chua_thue > 0.0 || invoice.tong_tien_thue > 0.0 {
            invoice.tong_tien_thanh_toan = invoice.tong_tien_chua_thue + invoice.tong_tien_thue;
        } else if !invoice.danh_sach_hang_hoa.is_empty() {
            invoice.tong_tien_thanh_toan =
                invoice.danh_sach_hang_hoa.iter().map(|i| i.thanh_tien).sum();
        }
    }

    if !has_valid_invoice_element
        && invoice.ky_hieu.is_empty()
        && invoice.so_hoa_don.is_empty()
        && invoice.ben_ban.mst.is_empty()
        && invoice.ben_ban.ten.is_empty()
    {
        return Err(CommandError::ValidationFailed {
            reason: "Tệp XML không chứa cấu trúc hóa đơn điện tử TT78 hợp lệ".to_string(),
        });
    }

    Ok(invoice)
}

/// Thuật toán đối soát 3 chiều giữa danh sách hóa đơn XML và bảng kê nội bộ
pub fn reconcile_invoices(
    xml_invoices: &[VnEinvoiceData],
    ledger_records: &[LedgerInvoiceRecord],
    tolerance: f64,
    cancel_flag: &AtomicBool,
) -> Result<ReconciliationReport, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let mut report = ReconciliationReport {
        total_xml: xml_invoices.len(),
        total_bang_ke: ledger_records.len(),
        ..Default::default()
    };
    report.parsed_invoices = xml_invoices.to_vec();

    let tol = if tolerance < 0.0 { 0.0 } else { tolerance };
    let mut matched_ledger_indices: HashSet<usize> = HashSet::new();

    // 1. Quét từng hóa đơn XML để tìm bản ghi tương ứng trong bảng kê
    for inv in xml_invoices {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let inv_mst = normalize_tax_code(&inv.ben_ban.mst);
        let inv_shd = normalize_invoice_number(&inv.so_hoa_don);

        let mut found_idx = None;
        for (idx, rec) in ledger_records.iter().enumerate() {
            if matched_ledger_indices.contains(&idx) {
                continue;
            }
            let rec_mst = normalize_tax_code(&rec.mst_ben_ban);
            let rec_shd = normalize_invoice_number(&rec.so_hoa_don);

            if inv_mst == rec_mst && inv_shd == rec_shd {
                found_idx = Some(idx);
                break;
            }
        }

        if let Some(idx) = found_idx {
            matched_ledger_indices.insert(idx);
            let rec = &ledger_records[idx];
            let diff = (inv.tong_tien_thanh_toan - rec.tong_tien).abs();

            if diff <= tol {
                report.khop_hoan_toan_count += 1;
                report.items.push(ReconciliationItem {
                    status: ReconciliationStatus::KhopHoanToan,
                    mst: inv.ben_ban.mst.clone(),
                    so_hoa_don: inv.so_hoa_don.clone(),
                    tien_xml: Some(inv.tong_tien_thanh_toan),
                    tien_bang_ke: Some(rec.tong_tien),
                    lech_tien: Some(diff),
                    ly_do: "Khớp hoàn toàn thông tin MST, số hóa đơn và tổng tiền thanh toán".to_string(),
                    file_xml: inv.file_path.clone(),
                });
            } else {
                report.khop_ma_lech_tien_count += 1;
                report.items.push(ReconciliationItem {
                    status: ReconciliationStatus::KhopMaLechTien,
                    mst: inv.ben_ban.mst.clone(),
                    so_hoa_don: inv.so_hoa_don.clone(),
                    tien_xml: Some(inv.tong_tien_thanh_toan),
                    tien_bang_ke: Some(rec.tong_tien),
                    lech_tien: Some(inv.tong_tien_thanh_toan - rec.tong_tien),
                    ly_do: format!(
                        "Khớp MST và số HĐ nhưng lệch tiền: XML={:.2}, Bảng kê={:.2}, Chênh lệch={:.2}",
                        inv.tong_tien_thanh_toan, rec.tong_tien, diff
                    ),
                    file_xml: inv.file_path.clone(),
                });
            }
        } else {
            report.chi_co_xml_count += 1;
            report.items.push(ReconciliationItem {
                status: ReconciliationStatus::ChiCoOXml,
                mst: inv.ben_ban.mst.clone(),
                so_hoa_don: inv.so_hoa_don.clone(),
                tien_xml: Some(inv.tong_tien_thanh_toan),
                tien_bang_ke: None,
                lech_tien: None,
                ly_do: "Hóa đơn có trong tệp XML nhưng thiếu trong bảng kê nội bộ".to_string(),
                file_xml: inv.file_path.clone(),
            });
        }
    }

    // 2. Quét các bản ghi bảng kê còn lại không tìm thấy tệp XML
    for (idx, rec) in ledger_records.iter().enumerate() {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        if !matched_ledger_indices.contains(&idx) {
            report.chi_co_bang_ke_count += 1;
            report.items.push(ReconciliationItem {
                status: ReconciliationStatus::ChiCoOBangKe,
                mst: rec.mst_ben_ban.clone(),
                so_hoa_don: rec.so_hoa_don.clone(),
                tien_xml: None,
                tien_bang_ke: Some(rec.tong_tien),
                lech_tien: None,
                ly_do: "Hóa đơn có trong bảng kê nội bộ nhưng thiếu tệp hóa đơn XML".to_string(),
                file_xml: None,
            });
        }
    }

    Ok(report)
}

impl OfficeTool for VnEinvoiceAuditorTool {
    fn slug(&self) -> &'static str {
        "vn-einvoice-auditor"
    }

    fn name(&self) -> &'static str {
        "Bóc tách & Đối soát Hóa đơn Điện tử XML TT78"
    }

    fn description(&self) -> &'static str {
        "Bóc tách tự động dữ liệu hóa đơn điện tử XML theo chuẩn Thông tư 78/2021/TT-BTC & Nghị định 123/2020/NĐ-CP và thực thi đối soát 3 chiều với bảng kê nội bộ."
    }

    fn execute(
        &self,
        request: &DispatchTaskRequest,
        cancel_flag: &AtomicBool,
        on_progress: Option<&dyn Fn(TaskProgress)>,
    ) -> Result<DispatchTaskResponse, CommandError> {
        let start_time = Instant::now();

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let options: VnEinvoiceAuditorOptions =
            serde_json::from_value(request.options.clone()).unwrap_or_default();

        let mut xml_invoices = Vec::new();
        let mut failed_items = Vec::new();

        // 1. Thu thập dữ liệu XML từ input_paths hoặc options
        if request.input_paths.is_empty() {
            if let Some(xml_raw) = request.options.get("xml_content").and_then(|v| v.as_str()) {
                let invoice = parse_einvoice_xml(xml_raw)?;
                xml_invoices.push(invoice);
            } else {
                return Err(CommandError::ValidationFailed {
                    reason: "Không có tệp hóa đơn XML đầu vào để bóc tách hoặc đối soát".to_string(),
                });
            }
        } else {
            let total_inputs = request.input_paths.len();
            for (idx, path_str) in request.input_paths.iter().enumerate() {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                let p = Path::new(path_str);
                if !p.exists() {
                    return Err(CommandError::FileNotFound {
                        path: path_str.clone(),
                    });
                }

                let raw_bytes = fs::read(p).map_err(|e| CommandError::IoError {
                    details: format!("Không thể đọc tệp {}: {e}", p.display()),
                })?;

                // Kiểm tra Magic Bytes chống tệp thực thi độc hại
                if raw_bytes.starts_with(b"\x7fELF") || raw_bytes.starts_with(b"MZ") {
                    return Err(CommandError::InvalidFileSignature {
                        expected: "Hóa đơn điện tử XML (TT78)".to_string(),
                        actual: "Tệp nhị phân thực thi bị chặn".to_string(),
                    });
                }

                let xml_str = String::from_utf8_lossy(&raw_bytes).to_string();
                match parse_einvoice_xml(&xml_str) {
                    Ok(mut inv) => {
                        inv.file_path = Some(path_str.clone());
                        xml_invoices.push(inv);
                    }
                    Err(err) => {
                        if total_inputs == 1 {
                            return Err(err);
                        }
                        failed_items.push(FileFailureDetail {
                            input_path: path_str.clone(),
                            reason: format!("{err}"),
                        });
                    }
                }

                if let Some(cb) = on_progress {
                    let pct = ((idx + 1) as f32 / total_inputs as f32) * 50.0;
                    cb(TaskProgress {
                        task_id: request.task_id.clone(),
                        current_step: (idx + 1) as u32,
                        total_steps: (total_inputs * 2) as u32,
                        percentage: pct,
                        message: format!("Đang bóc tách XML hóa đơn ({}/{})", idx + 1, total_inputs),
                        processed_files: xml_invoices.len(),
                        failed_files: failed_items.len(),
                        total_files: total_inputs,
                        is_completed: false,
                    });
                }
            }
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        // 2. Thực thi đối soát 3 chiều
        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 3,
                total_steps: 4,
                percentage: 75.0,
                message: "Đang thực hiện thuật toán đối soát 3 chiều...".to_string(),
                processed_files: xml_invoices.len(),
                failed_files: failed_items.len(),
                total_files: request.input_paths.len().max(1),
                is_completed: false,
            });
        }

        let report = reconcile_invoices(
            &xml_invoices,
            &options.compare_records,
            options.tolerance_amount,
            cancel_flag,
        )?;

        // 3. Xuất tệp kết quả JSON nếu được yêu cầu
        let mut output_paths = Vec::new();
        if options.export_summary_json {
            let out_dir = match &request.output_directory {
                Some(d) => PathBuf::from(d),
                None => std::env::temp_dir().join(format!("einvoice_audit_{}", request.task_id)),
            };

            if let Err(e) = fs::create_dir_all(&out_dir) {
                return Err(CommandError::IoError {
                    details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
                });
            }

            let report_file = out_dir.join("einvoice_reconciliation_report.json");
            let json_data = serde_json::to_string_pretty(&report).map_err(|e| {
                CommandError::ProcessExecutionError {
                    message: format!("Không thể serialize báo cáo JSON: {e}"),
                }
            })?;

            fs::write(&report_file, json_data.as_bytes()).map_err(|e| CommandError::IoError {
                details: format!("Lỗi ghi tệp báo cáo {}: {e}", report_file.display()),
            })?;

            output_paths.push(report_file.to_string_lossy().to_string());
        }

        let total_files_count = request.input_paths.len().max(1);
        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 4,
                total_steps: 4,
                percentage: 100.0,
                message: format!(
                    "Hoàn tất: Khớp hoàn toàn {} HĐ, lệch tiền {} HĐ, thiếu bảng kê {} HĐ, thiếu XML {} HĐ",
                    report.khop_hoan_toan_count,
                    report.khop_ma_lech_tien_count,
                    report.chi_co_xml_count,
                    report.chi_co_bang_ke_count
                ),
                processed_files: xml_invoices.len(),
                failed_files: failed_items.len(),
                total_files: total_files_count,
                is_completed: true,
            });
        }

        let status = if failed_items.is_empty() {
            "SUCCESS".to_string()
        } else if !xml_invoices.is_empty() {
            "PARTIAL_SUCCESS".to_string()
        } else {
            "FAILED".to_string()
        };

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status,
            output_paths,
            failed_items,
            elapsed_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const SAMPLE_TT78_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<HDon>
  <DLHDon Id="HD01">
    <TTChung>
      <KHHDon>1C24TYY</KHHDon>
      <SHDon>0001234</SHDon>
      <NLap>2024-03-15</NLap>
      <DVTTe>VND</DVTTe>
    </TTChung>
    <NDHDon>
      <NBan>
        <Ten>CÔNG TY TNHH GIẢI PHÁP SỐ VIỆT</Ten>
        <MST>0101234567</MST>
        <DChi>Tầng 5, Tòa nhà Tech, Cầu Giấy, Hà Nội</DChi>
      </NBan>
      <NMua>
        <Ten>CÔNG TY CỔ PHẦN THƯƠNG MẠI HOA SEN</Ten>
        <MST>0309876543</MST>
        <DChi>Quận 1, TP. Hồ Chí Minh</DChi>
      </NMua>
      <DSHHDVu>
        <HHDVu>
          <STT>1</STT>
          <THHDVu>Phần mềm Quản lý Văn phòng Điện tử</THHDVu>
          <DVTinh>Bộ</DVTinh>
          <SLuong>2</SLuong>
          <DGia>5000000</DGia>
          <Tien>10000000</Tien>
          <TSuat>10%</TSuat>
        </HHDVu>
      </DSHHDVu>
      <TToan>
        <TgTCThue>10000000</TgTCThue>
        <TgTThue>1000000</TgTThue>
        <TgTTTBSo>11000000</TgTTTBSo>
        <TgTTTBChu>Mười một triệu đồng chẵn</TgTTTBChu>
      </TToan>
    </NDHDon>
    <DSCks>
      <ds:Signature xmlns:ds="http://www.w3.org/2000/09/xmldsig#">
        <ds:SignatureValue>MIIESgYJKoZIhvcNAQcCoIIEODCCBDQCAQExDzANBglghkgBZQMEAgEFADCB</ds:SignatureValue>
      </ds:Signature>
    </DSCks>
  </DLHDon>
</HDon>"#;

    #[test]
    fn test_vn_einvoice_auditor_happy_path() {
        let invoice = parse_einvoice_xml(SAMPLE_TT78_XML).expect("Phải bóc tách thành công XML");
        assert_eq!(invoice.ky_hieu, "1C24TYY");
        assert_eq!(invoice.so_hoa_don, "0001234");
        assert_eq!(invoice.ngay_lap, "2024-03-15");
        assert_eq!(invoice.don_vi_tiet_te, "VND");
        assert_eq!(invoice.ben_ban.mst, "0101234567");
        assert_eq!(invoice.ben_ban.ten, "CÔNG TY TNHH GIẢI PHÁP SỐ VIỆT");
        assert_eq!(invoice.ben_mua.mst, "0309876543");
        assert_eq!(invoice.ben_mua.ten, "CÔNG TY CỔ PHẦN THƯƠNG MẠI HOA SEN");
        assert_eq!(invoice.danh_sach_hang_hoa.len(), 1);
        let item = &invoice.danh_sach_hang_hoa[0];
        assert_eq!(item.ten_hang, "Phần mềm Quản lý Văn phòng Điện tử");
        assert_eq!(item.thanh_tien, 10000000.0);
        assert_eq!(item.thue_suat.as_deref(), Some("10%"));
        assert_eq!(invoice.tong_tien_chua_thue, 10000000.0);
        assert_eq!(invoice.tong_tien_thue, 1000000.0);
        assert_eq!(invoice.tong_tien_thanh_toan, 11000000.0);
        assert_eq!(invoice.tong_tien_chu, "Mười một triệu đồng chẵn");
        assert!(invoice.co_chu_ky_so);

        // Kiểm tra execute qua options
        let tool = VnEinvoiceAuditorTool::default();
        let cancel = AtomicBool::new(false);
        let req = DispatchTaskRequest {
            task_id: "test_happy_path".to_string(),
            tool_slug: "vn-einvoice-auditor".to_string(),
            input_paths: Vec::new(),
            output_directory: None,
            options: serde_json::json!({
                "xml_content": SAMPLE_TT78_XML,
                "compare_records": [
                    {
                        "mst_ben_ban": "0101234567",
                        "so_hoa_don": "1234",
                        "tong_tien": 11000000.0
                    }
                ]
            }),
        };

        let res = tool.execute(&req, &cancel, None).expect("Execute thành công");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);
    }

    #[test]
    fn test_vn_einvoice_auditor_three_way_reconciliation() {
        let cancel = AtomicBool::new(false);

        let inv1 = VnEinvoiceData {
            so_hoa_don: "0001234".to_string(),
            tong_tien_thanh_toan: 11000000.0,
            ben_ban: PartyInfo {
                mst: "0101234567".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let inv2 = VnEinvoiceData {
            so_hoa_don: "0001235".to_string(),
            tong_tien_thanh_toan: 5000000.0,
            ben_ban: PartyInfo {
                mst: "0101234567".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let inv3 = VnEinvoiceData {
            so_hoa_don: "0009999".to_string(),
            tong_tien_thanh_toan: 2000000.0,
            ben_ban: PartyInfo {
                mst: "0109999999".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let xml_list = vec![inv1, inv2, inv3];

        let ledger1 = LedgerInvoiceRecord {
            mst_ben_ban: "0101234567".to_string(),
            so_hoa_don: "1234".to_string(),
            tong_tien: 11000000.0,
            ..Default::default()
        };

        let ledger2 = LedgerInvoiceRecord {
            mst_ben_ban: "0101234567".to_string(),
            so_hoa_don: "1235".to_string(),
            tong_tien: 4800000.0,
            ..Default::default()
        };

        let ledger3 = LedgerInvoiceRecord {
            mst_ben_ban: "0301112223".to_string(),
            so_hoa_don: "7777".to_string(),
            tong_tien: 3000000.0,
            ..Default::default()
        };

        let ledger_list = vec![ledger1, ledger2, ledger3];

        let report = reconcile_invoices(&xml_list, &ledger_list, 0.0, &cancel)
            .expect("Đối soát thành công");

        assert_eq!(report.total_xml, 3);
        assert_eq!(report.total_bang_ke, 3);
        assert_eq!(report.khop_hoan_toan_count, 1);
        assert_eq!(report.khop_ma_lech_tien_count, 1);
        assert_eq!(report.chi_co_xml_count, 1);
        assert_eq!(report.chi_co_bang_ke_count, 1);
    }

    #[test]
    fn test_vn_einvoice_auditor_cooperative_cancellation() {
        let cancel = AtomicBool::new(true);
        let tool = VnEinvoiceAuditorTool::default();
        let req = DispatchTaskRequest {
            task_id: "test_cancel".to_string(),
            tool_slug: "vn-einvoice-auditor".to_string(),
            input_paths: Vec::new(),
            output_directory: None,
            options: serde_json::json!({ "xml_content": SAMPLE_TT78_XML }),
        };

        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_vn_einvoice_auditor_invalid_magic_bytes() {
        let tool = VnEinvoiceAuditorTool::default();
        let cancel = AtomicBool::new(false);

        let temp_dir = std::env::temp_dir();
        let malicious_file = temp_dir.join("malicious_elf.xml");
        {
            let mut f = fs::File::create(&malicious_file).expect("Tạo tệp giả lập ELF");
            f.write_all(b"\x7fELF\x02\x01\x01\x00malicious_payload")
                .expect("Ghi payload");
        }

        let req = DispatchTaskRequest {
            task_id: "test_magic_bytes".to_string(),
            tool_slug: "vn-einvoice-auditor".to_string(),
            input_paths: vec![malicious_file.to_string_lossy().to_string()],
            output_directory: None,
            options: serde_json::json!({}),
        };

        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { expected, actual } => {
                assert_eq!(expected, "Hóa đơn điện tử XML (TT78)");
                assert_eq!(actual, "Tệp nhị phân thực thi bị chặn");
            }
            _ => panic!("Kỳ vọng lỗi InvalidFileSignature nhưng nhận: {:?}", err),
        }

        let _ = fs::remove_file(malicious_file);
    }

    #[test]
    fn test_vn_einvoice_auditor_security_guardrails() {
        // 1. Kiểm tra XML rỗng
        let res_empty = parse_einvoice_xml("");
        assert!(matches!(res_empty, Err(CommandError::ValidationFailed { .. })));

        // 2. Kiểm tra XML hỏng cú pháp
        let res_broken = parse_einvoice_xml("<HDon><DLHDon><Unclosed");
        assert!(matches!(res_broken, Err(CommandError::ValidationFailed { .. })));

        // 3. Kiểm tra XML không phải hóa đơn
        let res_not_invoice = parse_einvoice_xml("<root><item>không phải hóa đơn</item></root>");
        assert!(matches!(res_not_invoice, Err(CommandError::ValidationFailed { .. })));
    }
}
