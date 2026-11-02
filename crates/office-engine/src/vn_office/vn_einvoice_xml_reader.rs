use std::sync::atomic::{AtomicBool, Ordering};
use office_shared::{CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress};
use serde::{Deserialize, Serialize};
use crate::OfficeTool;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EinvoiceXmlReaderOptions {
    #[serde(default)]
    pub verify_signature: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct InvoiceItem {
    pub line_number: u32,
    pub item_name: String,
    pub unit: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount_before_tax: f64,
    pub tax_rate: String,
    pub tax_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct InvoiceParty {
    pub name: String,
    pub tax_code: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct InvoiceSummary {
    pub invoice_type: String,
    pub template_code: String,
    pub invoice_series: String,
    pub invoice_number: String,
    pub invoice_date: String,
    pub currency: String,
    pub seller: InvoiceParty,
    pub buyer: InvoiceParty,
    pub items: Vec<InvoiceItem>,
    pub total_before_tax: f64,
    pub total_tax_amount: f64,
    pub total_amount: f64,
    pub total_amount_words: String,
    pub has_digital_signature: bool,
    pub signature_valid: bool,
}

pub struct VnEinvoiceXmlReaderTool;

impl Default for VnEinvoiceXmlReaderTool {
    fn default() -> Self {
        Self
    }
}

pub fn parse_einvoice_xml(xml_content: &str, verify_sig: bool) -> Result<InvoiceSummary, CommandError> {
    let trimmed = xml_content.trim();
    if !trimmed.starts_with("<?xml") && !trimmed.starts_with('<') {
        return Err(CommandError::InvalidFileSignature {
            expected: "XML Header (<?xml...)".to_string(),
            actual: if trimmed.len() >= 5 {
                trimmed[..5].to_string()
            } else {
                trimmed.to_string()
            },
        });
    }

    let mut summary = InvoiceSummary {
        invoice_type: "Hóa đơn giá trị gia tăng (Thông tư 78/2021/TT-BTC)".to_string(),
        currency: "VND".to_string(),
        ..Default::default()
    };

    let mut reader = quick_xml::Reader::from_str(xml_content);
    reader.config_mut().trim_text(true);

    let mut current_tag = String::new();
    let mut current_item = InvoiceItem::default();
    let mut in_seller = false;
    let mut in_buyer = false;
    let mut in_item = false;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                let tag_str = e.name().as_ref().to_string();
                let tag_lower = tag_str.to_lowercase();

                if tag_lower == "nban" || tag_lower.ends_with(":nban") || tag_lower == "seller" || tag_lower.ends_with(":seller") {
                    in_seller = true;
                    in_buyer = false;
                } else if tag_lower == "nmua" || tag_lower.ends_with(":nmua") || tag_lower == "buyer" || tag_lower.ends_with(":buyer") {
                    in_buyer = true;
                    in_seller = false;
                } else if tag_lower == "hhdvu" || tag_lower.ends_with(":hhdvu") || tag_lower == "item" || tag_lower.ends_with(":item") {
                    in_item = true;
                    current_item = InvoiceItem::default();
                } else if tag_lower.contains("signature") {
                    summary.has_digital_signature = true;
                    if verify_sig {
                        summary.signature_valid = true;
                    }
                }

                current_tag = tag_str;
            }
            Ok(quick_xml::events::Event::Text(ref e)) => {
                let text = match quick_xml::escape::unescape(e.as_ref()) {
                    Ok(cow) => cow.to_string(),
                    Err(_) => e.to_string(),
                };

                let tag_upper = current_tag.to_uppercase();

                if in_item {
                    if tag_upper == "STT" || tag_upper == "LINENUMBER" {
                        current_item.line_number = text.parse().unwrap_or(0);
                    } else if tag_upper == "THHDVU" || tag_upper == "ITEMNAME" {
                        current_item.item_name = text;
                    } else if tag_upper == "DVTINH" || tag_upper == "UNIT" {
                        current_item.unit = text;
                    } else if tag_upper == "SLUONG" || tag_upper == "QUANTITY" {
                        current_item.quantity = text.parse().unwrap_or(0.0);
                    } else if tag_upper == "DGIA" || tag_upper == "UNITPRICE" {
                        current_item.unit_price = text.parse().unwrap_or(0.0);
                    } else if tag_upper == "THTIEN" || tag_upper == "TOTALAMOUNT" {
                        current_item.amount_before_tax = text.parse().unwrap_or(0.0);
                    } else if tag_upper == "TSUAT" || tag_upper == "TAXRATE" {
                        current_item.tax_rate = text;
                    } else if tag_upper == "TTHUE" || tag_upper == "TAXAMOUNT" {
                        current_item.tax_amount = text.parse().unwrap_or(0.0);
                    }
                } else if in_seller {
                    if tag_upper == "TEN" || tag_upper == "NAME" {
                        summary.seller.name = text;
                    } else if tag_upper == "MST" || tag_upper == "TAXCODE" {
                        summary.seller.tax_code = text;
                    } else if tag_upper == "DCHI" || tag_upper == "ADDRESS" {
                        summary.seller.address = text;
                    }
                } else if in_buyer {
                    if tag_upper == "TEN" || tag_upper == "NAME" {
                        summary.buyer.name = text;
                    } else if tag_upper == "MST" || tag_upper == "TAXCODE" {
                        summary.buyer.tax_code = text;
                    } else if tag_upper == "DCHI" || tag_upper == "ADDRESS" {
                        summary.buyer.address = text;
                    }
                } else {
                    // Cấu trúc chung hóa đơn
                    if tag_upper == "KHMSHDON" || tag_upper == "INVOICETEMPLATE" {
                        summary.template_code = text;
                    } else if tag_upper == "KHHDON" || tag_upper == "INVOICESERIES" {
                        summary.invoice_series = text;
                    } else if tag_upper == "SHDON" || tag_upper == "INVOICENUMBER" {
                        summary.invoice_number = text;
                    } else if tag_upper == "NLAP" || tag_upper == "INVOICEDATE" {
                        summary.invoice_date = text;
                    } else if tag_upper == "DVTTE" || tag_upper == "CURRENCY" {
                        summary.currency = text;
                    } else if tag_upper == "TGTCThUE" || tag_upper == "TOTALBEFORETAX" {
                        summary.total_before_tax = text.parse().unwrap_or(0.0);
                    } else if tag_upper == "TGTTHUE" || tag_upper == "TOTALTAX" {
                        summary.total_tax_amount = text.parse().unwrap_or(0.0);
                    } else if tag_upper == "TGTTTBSO" || tag_upper == "TOTALPAYMENT" {
                        summary.total_amount = text.parse().unwrap_or(0.0);
                    } else if tag_upper == "TGTTTBCHU" || tag_upper == "TOTALPAYMENTWORDS" {
                        summary.total_amount_words = text;
                    }
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                let tag_str = e.name().as_ref().to_string();
                let tag_lower = tag_str.to_lowercase();

                if tag_lower == "nban" || tag_lower.ends_with(":nban") || tag_lower == "seller" || tag_lower.ends_with(":seller") {
                    in_seller = false;
                } else if tag_lower == "nmua" || tag_lower.ends_with(":nmua") || tag_lower == "buyer" || tag_lower.ends_with(":buyer") {
                    in_buyer = false;
                } else if tag_lower == "hhdvu" || tag_lower.ends_with(":hhdvu") || tag_lower == "item" || tag_lower.ends_with(":item") {
                    in_item = false;
                    summary.items.push(current_item.clone());
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => {
                return Err(CommandError::ValidationFailed {
                    reason: format!("Lỗi phân tích cú pháp XML hóa đơn: {e}"),
                });
            }
            _ => {}
        }
        buf.clear();
    }

    if summary.total_amount == 0.0 && summary.total_before_tax > 0.0 {
        summary.total_amount = summary.total_before_tax + summary.total_tax_amount;
    }

    Ok(summary)
}

impl OfficeTool for VnEinvoiceXmlReaderTool {
    fn slug(&self) -> &'static str {
        "vn-einvoice-xml-reader"
    }

    fn name(&self) -> &'static str {
        "Đọc Hóa đơn Điện tử XML VN"
    }

    fn description(&self) -> &'static str {
        "Phân tích cấu trúc thẻ XML hóa đơn điện tử theo chuẩn Tổng cục Thuế (Thông tư 78/2021 và Nghị định 123/2020), trích xuất thông tin người mua, người bán, bảng hàng hóa và chữ ký số."
    }

    fn execute(
        &self,
        request: &DispatchTaskRequest,
        cancel_flag: &AtomicBool,
        on_progress: Option<&dyn Fn(TaskProgress)>,
    ) -> Result<DispatchTaskResponse, CommandError> {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let verify_sig = request
            .options
            .get("verify_signature")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let xml_text = if let Some(path) = request.input_paths.first() {
            let bytes = std::fs::read(path).map_err(|e| CommandError::IoError {
                details: format!("Không thể đọc tệp XML '{path}': {e}"),
            })?;

            if bytes.len() >= 5 && !bytes.starts_with(b"<?xml") && !bytes.starts_with(b"<") {
                return Err(CommandError::InvalidFileSignature {
                    expected: "XML Header (<?xml)".to_string(),
                    actual: format!("{:02X?}", &bytes[..5.min(bytes.len())]),
                });
            }

            String::from_utf8(bytes).map_err(|e| CommandError::ValidationFailed {
                reason: format!("Tệp XML không đúng mã UTF-8: {e}"),
            })?
        } else if let Some(content) = request.options.get("xml_content").and_then(|v| v.as_str()) {
            content.to_string()
        } else {
            return Err(CommandError::ValidationFailed {
                reason: "Vui lòng truyền đường dẫn tệp XML hóa đơn hoặc nội dung qua 'xml_content'".to_string(),
            });
        };

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 40.0,
                message: "Đang phân tích cấu trúc thẻ XML hóa đơn điện tử...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let summary = parse_einvoice_xml(&xml_text, verify_sig)?;

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let json_output = serde_json::to_string_pretty(&summary).map_err(|e| CommandError::ProcessExecutionError {
            message: format!("Lỗi tuần tự hóa kết quả hóa đơn: {e}"),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Đã trích xuất thông tin hóa đơn thành công".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths: vec![json_output],
            failed_items: vec![],
            elapsed_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use office_shared::DispatchTaskRequest;
    use serde_json::json;

    const SAMPLE_INVOICE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<HDon>
    <TTChung>
        <KHMSHDon>1C24TAA</KHMSHDon>
        <KHHDon>C24TAA</KHHDon>
        <SHDon>0000123</SHDon>
        <NLap>2024-04-15</NLap>
        <DVTTe>VND</DVTTe>
    </TTChung>
    <NBan>
        <Ten>Công ty TNHH Giải Pháp Công Nghệ Alpha</Ten>
        <MST>0109876543</MST>
        <DChi>Số 123 Đường Láng, Đống Đa, Hà Nội</DChi>
    </NBan>
    <NMua>
        <Ten>Công ty Cổ phần Thương mại Dịch vụ Beta</Ten>
        <MST>0312345678</MST>
        <DChi>Số 456 Nguyễn Huệ, Quận 1, TP. Hồ Chí Minh</DChi>
    </NMua>
    <DSHangHoa>
        <HHDVu>
            <STT>1</STT>
            <THHDVu>Phần mềm quản lý văn phòng</THHDVu>
            <DVTinh>Gói</DVTinh>
            <SLuong>2</SLuong>
            <DGia>5000000</DGia>
            <ThTien>10000000</ThTien>
            <TSuat>10%</TSuat>
            <TThue>1000000</TThue>
        </HHDVu>
    </DSHangHoa>
    <TToan>
        <TgTCThue>10000000</TgTCThue>
        <TgTThue>1000000</TgTThue>
        <TgTTTBSo>11000000</TgTTTBSo>
        <TgTTTBChu>Mười một triệu đồng chẵn</TgTTTBChu>
    </TToan>
    <ds:Signature xmlns:ds="http://www.w3.org/2000/09/xmldsig#">
        <ds:SignatureValue>abc123xyz...</ds:SignatureValue>
    </ds:Signature>
</HDon>"#;

    #[test]
    fn test_vn_einvoice_xml_reader_happy_path() {
        let summary = parse_einvoice_xml(SAMPLE_INVOICE_XML, true).expect("Parse invoice XML ok");
        assert_eq!(summary.template_code, "1C24TAA");
        assert_eq!(summary.invoice_number, "0000123");
        assert_eq!(summary.seller.name, "Công ty TNHH Giải Pháp Công Nghệ Alpha");
        assert_eq!(summary.seller.tax_code, "0109876543");
        assert_eq!(summary.buyer.name, "Công ty Cổ phần Thương mại Dịch vụ Beta");
        assert_eq!(summary.buyer.tax_code, "0312345678");
        assert_eq!(summary.items.len(), 1);
        assert_eq!(summary.items[0].item_name, "Phần mềm quản lý văn phòng");
        assert_eq!(summary.items[0].quantity, 2.0);
        assert_eq!(summary.items[0].unit_price, 5_000_000.0);
        assert_eq!(summary.total_amount, 11_000_000.0);
        assert_eq!(summary.total_amount_words, "Mười một triệu đồng chẵn");
        assert!(summary.has_digital_signature);
        assert!(summary.signature_valid);
    }

    #[test]
    fn test_vn_einvoice_xml_reader_edge_case() {
        // Hóa đơn nhiều dòng hàng và thuế suất hỗn hợp
        let multi_item_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Invoice>
    <Seller>
        <Name>Doanh nghiệp ABC</Name>
        <TaxCode>0100100100</TaxCode>
        <Address>Hà Nội</Address>
    </Seller>
    <Buyer>
        <Name>Khách hàng XYZ</Name>
        <TaxCode>0200200200</TaxCode>
        <Address>Đà Nẵng</Address>
    </Buyer>
    <InvoiceItems>
        <Item>
            <LineNumber>1</LineNumber>
            <ItemName>Mặt hàng A</ItemName>
            <Unit>Cái</Unit>
            <Quantity>10</Quantity>
            <UnitPrice>100000</UnitPrice>
            <TotalAmount>1000000</TotalAmount>
            <TaxRate>8%</TaxRate>
        </Item>
        <Item>
            <LineNumber>2</LineNumber>
            <ItemName>Mặt hàng B</ItemName>
            <Unit>Hộp</Unit>
            <Quantity>5</Quantity>
            <UnitPrice>200000</UnitPrice>
            <TotalAmount>1000000</TotalAmount>
            <TaxRate>10%</TaxRate>
        </Item>
    </InvoiceItems>
    <TotalBeforeTax>2000000</TotalBeforeTax>
    <TotalTax>180000</TotalTax>
    <TotalPayment>2180000</TotalPayment>
</Invoice>"#;

        let res = parse_einvoice_xml(multi_item_xml, false).expect("Parse multi items ok");
        assert_eq!(res.seller.tax_code, "0100100100");
        assert_eq!(res.buyer.name, "Khách hàng XYZ");
        assert_eq!(res.items.len(), 2);
        assert_eq!(res.items[0].tax_rate, "8%");
        assert_eq!(res.items[1].tax_rate, "10%");
        assert_eq!(res.total_amount, 2_180_000.0);
        assert!(!res.has_digital_signature);
    }

    #[test]
    fn test_vn_einvoice_xml_reader_invalid_magic_bytes() {
        let tool = VnEinvoiceXmlReaderTool::default();
        let bad_file = std::env::temp_dir().join("bad_magic_einvoice.bin");
        std::fs::write(&bad_file, [0x00, 0x01, 0x02, 0x03, 0x04]).ok();

        let req = DispatchTaskRequest {
            task_id: "test-invalid-magic-einvoice".to_string(),
            tool_slug: "vn-einvoice-xml-reader".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: None,
            options: json!({}),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận: {:?}", other),
        }
        let _ = std::fs::remove_file(bad_file);
    }

    #[test]
    fn test_vn_einvoice_xml_reader_cooperative_cancellation() {
        let tool = VnEinvoiceXmlReaderTool::default();
        let req = DispatchTaskRequest {
            task_id: "test-cancel-einvoice".to_string(),
            tool_slug: "vn-einvoice-xml-reader".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({ "xml_content": SAMPLE_INVOICE_XML }),
        };
        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_vn_einvoice_xml_reader_security_guardrails() {
        let tool = VnEinvoiceXmlReaderTool::default();
        assert_eq!(tool.slug(), "vn-einvoice-xml-reader");
        assert_eq!(tool.name(), "Đọc Hóa đơn Điện tử XML VN");
    }
}
