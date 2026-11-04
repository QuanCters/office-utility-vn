use office_utility_vn_ui::shared_ui::{
    validate_payload_before_dispatch, EmptyToolPayload, GenericToolPayload, ToolPayloadContract,
};
use serde_json::json;

#[test]
fn test_empty_tool_payload_contract() {
    let payload = EmptyToolPayload;
    assert_eq!(payload.validate(), Ok(()));
    assert_eq!(payload.to_payload(), json!({}));

    let result = validate_payload_before_dispatch(&payload);
    assert_eq!(result, Ok(json!({})));
}

#[test]
fn test_generic_tool_payload_without_validator() {
    let data = json!({ "dpi": 300, "format": "png" });
    let payload = GenericToolPayload {
        data: data.clone(),
        validator: None,
    };
    assert_eq!(payload.validate(), Ok(()));
    assert_eq!(payload.to_payload(), data);

    let result = validate_payload_before_dispatch(&payload);
    assert_eq!(result, Ok(data));
}

fn sample_dpi_validator(v: &serde_json::Value) -> Result<(), String> {
    let dpi = v.get("dpi").and_then(|d| d.as_u64()).unwrap_or(0);
    if !(72..=1200).contains(&dpi) {
        Err("DPI phải nằm trong khoảng từ 72 đến 1200".to_string())
    } else {
        Ok(())
    }
}

#[test]
fn test_generic_tool_payload_validation_success() {
    let payload = GenericToolPayload {
        data: json!({ "dpi": 300 }),
        validator: Some(sample_dpi_validator),
    };
    assert_eq!(payload.validate(), Ok(()));
    let result = validate_payload_before_dispatch(&payload);
    assert_eq!(result, Ok(json!({ "dpi": 300 })));
}

#[test]
fn test_generic_tool_payload_validation_failure() {
    let payload = GenericToolPayload {
        data: json!({ "dpi": 30 }),
        validator: Some(sample_dpi_validator),
    };
    let val_res = payload.validate();
    assert!(val_res.is_err());
    assert_eq!(
        val_res.unwrap_err(),
        "DPI phải nằm trong khoảng từ 72 đến 1200"
    );

    let dispatch_res = validate_payload_before_dispatch(&payload);
    assert!(dispatch_res.is_err());
    assert_eq!(
        dispatch_res.unwrap_err(),
        "DPI phải nằm trong khoảng từ 72 đến 1200"
    );
}

struct SalaryTaxPayload {
    gross_salary: f64,
    dependents: u32,
}

impl ToolPayloadContract for SalaryTaxPayload {
    fn validate(&self) -> Result<(), String> {
        if self.gross_salary < 0.0 {
            return Err("Lương gộp không thể là số âm".to_string());
        }
        Ok(())
    }

    fn to_payload(&self) -> serde_json::Value {
        json!({
            "gross_salary": self.gross_salary,
            "dependents": self.dependents,
        })
    }
}

#[test]
fn test_custom_struct_implementing_tool_payload_contract() {
    let valid = SalaryTaxPayload {
        gross_salary: 25_000_000.0,
        dependents: 1,
    };
    assert_eq!(valid.validate(), Ok(()));
    assert_eq!(
        validate_payload_before_dispatch(&valid),
        Ok(json!({ "gross_salary": 25000000.0, "dependents": 1 }))
    );

    let invalid = SalaryTaxPayload {
        gross_salary: -5_000_000.0,
        dependents: 0,
    };
    assert_eq!(
        invalid.validate(),
        Err("Lương gộp không thể là số âm".to_string())
    );
    assert_eq!(
        validate_payload_before_dispatch(&invalid),
        Err("Lương gộp không thể là số âm".to_string())
    );
}

#[test]
fn test_dyn_tool_payload_contract_dispatch() {
    let empty = EmptyToolPayload;
    let contract: &dyn ToolPayloadContract = &empty;
    assert_eq!(validate_payload_before_dispatch(contract), Ok(json!({})));
}

#[test]
fn test_validator_callback_error_handling() {
    use leptos::prelude::*;

    let validator = Callback::new(|val: serde_json::Value| -> Result<(), String> {
        let count = val.get("count").and_then(|c| c.as_i64()).unwrap_or(0);
        if count <= 0 {
            Err("Số lượng phải lớn hơn 0".to_string())
        } else {
            Ok(())
        }
    });

    let invalid_input = json!({ "count": 0 });
    let err = validator.run(invalid_input).unwrap_err();
    let formatted = format!("⚠️ Lỗi tham số: {err}");
    assert_eq!(formatted, "⚠️ Lỗi tham số: Số lượng phải lớn hơn 0");

    let valid_input = json!({ "count": 5 });
    assert!(validator.run(valid_input).is_ok());
}
