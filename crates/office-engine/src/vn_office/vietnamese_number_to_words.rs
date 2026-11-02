pub use super::number_to_words::*;
pub use crate::OfficeTool;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OfficeTool;
    use std::sync::atomic::AtomicBool;
    use office_shared::DispatchTaskRequest;
    use serde_json::json;

    #[test]
    fn test_vietnamese_number_to_words_happy_path() {
        let tool = VietnameseNumberToWordsTool::default();
        let config = NumberToWordsConfig {
            dialect: Dialect::North,
            currency: CurrencyUnit::VND,
            use_dong_chan: true,
            capitalize_first: true,
            negative_prefix: "âm".to_string(),
        };

        let result = format_vietnamese_number("1250000", &config).expect("format ok");
        assert_eq!(result, "Một triệu hai trăm năm mươi nghìn đồng chẵn");

        let req = DispatchTaskRequest {
            task_id: "test-happy-1".to_string(),
            tool_slug: "vietnamese-number-to-words".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "number": "500000",
                "use_dong_chan": true
            }),
        };
        let cancel = AtomicBool::new(false);
        let resp = tool.execute(&req, &cancel, None).expect("execute ok");
        assert_eq!(resp.output_paths.len(), 1);
        assert_eq!(resp.output_paths[0], "năm trăm nghìn đồng chẵn");
    }

    #[test]
    fn test_vietnamese_number_to_words_edge_case() {
        let config = NumberToWordsConfig {
            dialect: Dialect::South,
            currency: CurrencyUnit::VND,
            use_dong_chan: false,
            capitalize_first: false,
            negative_prefix: "âm".to_string(),
        };

        // Số cực lớn 10^12 và số âm
        let res_trillion = format_vietnamese_number("1000000000000", &config).expect("trillion ok");
        assert_eq!(res_trillion, "một ngàn tỷ đồng");

        let res_negative = format_vietnamese_number("-50000", &config).expect("negative ok");
        assert_eq!(res_negative, "âm năm mươi ngàn đồng");
    }

    #[test]
    fn test_vietnamese_number_to_words_invalid_magic_bytes() {
        let tool = VietnameseNumberToWordsTool::default();
        let req = DispatchTaskRequest {
            task_id: "test-invalid-1".to_string(),
            tool_slug: "vietnamese-number-to-words".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "number": "abc123xyz"
            }),
        };
        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            office_shared::CommandError::ValidationFailed { .. } => {}
            other => panic!("Kỳ vọng ValidationFailed nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_vietnamese_number_to_words_cooperative_cancellation() {
        let tool = VietnameseNumberToWordsTool::default();
        let req = DispatchTaskRequest {
            task_id: "test-cancel".to_string(),
            tool_slug: "vietnamese-number-to-words".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "number": "123456789"
            }),
        };
        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, office_shared::CommandError::TaskCancelled);
    }

    #[test]
    fn test_vietnamese_number_to_words_security_guardrails() {
        let tool = VietnameseNumberToWordsTool::default();
        assert_eq!(tool.slug(), "vietnamese-number-to-words");
        assert_eq!(tool.name(), "Đọc Số Tiền Thành Chữ Chuẩn");
    }
}
