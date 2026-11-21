use office_utility_vn_ui::micro_apps::MicroAppId;
use office_utility_vn_ui::platform::{
    validate_origin, validate_structure, AppAvailability, CraftHostTransport, DocumentStateEvent,
    GridCraftAdapter, IntegrationCapability, MicroAppAdapter, MicroAppRegistry, PdfCraftAdapter,
    RawBridgeMessage, RuntimeKind, TransportError,
};
use serde_json::json;

#[test]
fn test_registry_descriptors_metadata() {
    let all = MicroAppRegistry::all();
    assert_eq!(all.len(), 4, "Must register exactly 4 micro-app descriptors");

    let vn = MicroAppRegistry::get(MicroAppId::VnOffice).expect("VnOffice must exist");
    assert_eq!(vn.name, "VN Office");
    assert_eq!(vn.runtime, RuntimeKind::NativeLeptos);
    assert_eq!(vn.entry_point, None);
    assert_eq!(vn.availability, AppAvailability::Available);

    let pdf = MicroAppRegistry::get(MicroAppId::PdfStudio).expect("PdfStudio must exist");
    assert_eq!(pdf.name, "PDF Studio");
    assert_eq!(pdf.runtime, RuntimeKind::IframeHosted);
    assert_eq!(pdf.entry_point, Some("/pdfcraft/index.html"));
    assert!(pdf.capabilities.contains(&IntegrationCapability::DocumentOpen));
    assert!(pdf.capabilities.contains(&IntegrationCapability::DocumentSave));
    assert!(pdf.capabilities.contains(&IntegrationCapability::DirtyNotification));

    let grid = MicroAppRegistry::get(MicroAppId::DataLab).expect("DataLab must exist");
    assert_eq!(grid.name, "Data Lab");
    assert_eq!(grid.runtime, RuntimeKind::IframeHosted);
    assert_eq!(grid.entry_point, Some("/gridcraft/index.html"));
    assert!(grid.capabilities.contains(&IntegrationCapability::CommandApi));

    let iframes = MicroAppRegistry::by_runtime(RuntimeKind::IframeHosted);
    assert_eq!(iframes.len(), 2, "PdfStudio and DataLab are iframe hosted");
}

#[test]
fn test_wrong_origin_rejection() {
    let expected = "http://tauri.localhost";
    let evil = "http://malicious-site.com";

    let res = validate_origin(expected, evil);
    assert!(res.is_err());
    match res.unwrap_err() {
        TransportError::InvalidOrigin { expected: exp, actual } => {
            assert_eq!(exp, expected);
            assert_eq!(actual, evil);
        }
        other => panic!("Expected InvalidOrigin, got {other:?}"),
    }
}

#[test]
fn test_identical_origin_acceptance() {
    let origin = "http://tauri.localhost";
    assert!(validate_origin(origin, origin).is_ok());

    let dev_origin = "http://localhost:1420";
    assert!(validate_origin(dev_origin, dev_origin).is_ok());
}

#[test]
fn test_empty_or_wildcard_origin_rejection() {
    assert!(validate_origin("", "http://localhost:1420").is_err());
    assert!(validate_origin("http://localhost:1420", "").is_err());

    let transport = CraftHostTransport::new("*", "test-iframe");
    let send_res = transport.post_to_iframe("test", json!({}), None);
    assert!(send_res.is_err());
}

#[test]
fn test_malformed_payload_rejection() {
    // Non-object
    let non_obj = json!("not an object");
    assert!(validate_structure(&non_obj).is_err());

    let arr = json!([1, 2, 3]);
    assert!(validate_structure(&arr).is_err());

    // Missing "type" field
    let missing_type = json!({ "data": 123 });
    assert!(validate_structure(&missing_type).is_err());

    // Empty "type" string
    let empty_type = json!({ "type": "  ", "data": 123 });
    assert!(validate_structure(&empty_type).is_err());
}

#[test]
fn test_valid_structure_parsing() {
    let valid = json!({
        "type": "gridcraft:ready",
        "version": "0.19.1",
        "file": false
    });
    let parsed = validate_structure(&valid).expect("Must parse valid payload");
    assert_eq!(parsed.type_name, "gridcraft:ready");
    assert_eq!(parsed.payload.get("version").unwrap().as_str().unwrap(), "0.19.1");
}

#[test]
fn test_pdfcraft_adapter_protocol_parsing() {
    let adapter = PdfCraftAdapter;
    assert_eq!(adapter.app_id(), MicroAppId::PdfStudio);
    assert_eq!(adapter.prefix(), "pdfcraft");

    // Ready
    let msg_ready = RawBridgeMessage {
        type_name: "pdfcraft:ready".to_string(),
        payload: json!({ "type": "pdfcraft:ready", "version": "1.0.0" }),
        binary_data: None,
    };
    match adapter.parse_incoming(&msg_ready).unwrap() {
        DocumentStateEvent::Ready { version } => assert_eq!(version, "1.0.0"),
        other => panic!("Expected Ready, got {other:?}"),
    }

    // Dirty
    let msg_dirty = RawBridgeMessage {
        type_name: "pdfcraft:dirty".to_string(),
        payload: json!({ "type": "pdfcraft:dirty", "dirty": true }),
        binary_data: None,
    };
    match adapter.parse_incoming(&msg_dirty).unwrap() {
        DocumentStateEvent::Dirty { dirty } => assert!(dirty),
        other => panic!("Expected Dirty, got {other:?}"),
    }

    // Save
    let sample_pdf_bytes = vec![0x25, 0x50, 0x44, 0x46]; // %PDF
    let msg_save = RawBridgeMessage {
        type_name: "pdfcraft:save".to_string(),
        payload: json!({ "type": "pdfcraft:save", "name": "contract.pdf", "saveAs": true }),
        binary_data: Some(sample_pdf_bytes.clone()),
    };
    match adapter.parse_incoming(&msg_save).unwrap() {
        DocumentStateEvent::SaveRequested { name, save_as, bytes, request_id } => {
            assert_eq!(name, "contract.pdf");
            assert!(save_as);
            assert_eq!(bytes, sample_pdf_bytes);
            assert_eq!(request_id, None);
        }
        other => panic!("Expected SaveRequested, got {other:?}"),
    }

    // Failed
    let msg_failed = RawBridgeMessage {
        type_name: "pdfcraft:failed".to_string(),
        payload: json!({ "type": "pdfcraft:failed", "error": "WebGL Context Lost" }),
        binary_data: None,
    };
    match adapter.parse_incoming(&msg_failed).unwrap() {
        DocumentStateEvent::Failed { error } => assert_eq!(error, "WebGL Context Lost"),
        other => panic!("Expected Failed, got {other:?}"),
    }

    // Format Open
    let open_msg = adapter.format_open("doc.pdf", &[1, 2, 3]).unwrap();
    assert_eq!(open_msg.type_name, "pdfcraft:open");
    assert_eq!(open_msg.binary_data, Some(vec![1, 2, 3]));
}

#[test]
fn test_gridcraft_adapter_protocol_parsing() {
    let adapter = GridCraftAdapter;
    assert_eq!(adapter.app_id(), MicroAppId::DataLab);
    assert_eq!(adapter.prefix(), "gridcraft");

    // Ready
    let msg_ready = RawBridgeMessage {
        type_name: "gridcraft:ready".to_string(),
        payload: json!({ "type": "gridcraft:ready", "version": "0.19.1", "file": false }),
        binary_data: None,
    };
    match adapter.parse_incoming(&msg_ready).unwrap() {
        DocumentStateEvent::Ready { version } => assert_eq!(version, "0.19.1"),
        other => panic!("Expected Ready, got {other:?}"),
    }

    // Save (saveAs is false in GridCraft, but includes confirmed id)
    let sample_xlsx = vec![0x50, 0x4B, 0x03, 0x04]; // PK zip
    let msg_save = RawBridgeMessage {
        type_name: "gridcraft:save".to_string(),
        payload: json!({ "type": "gridcraft:save", "name": "sheet.xlsx", "saveAs": false, "id": "save-1-1" }),
        binary_data: Some(sample_xlsx.clone()),
    };
    match adapter.parse_incoming(&msg_save).unwrap() {
        DocumentStateEvent::SaveRequested { name, save_as, bytes, request_id } => {
            assert_eq!(name, "sheet.xlsx");
            assert!(!save_as);
            assert_eq!(bytes, sample_xlsx);
            assert_eq!(request_id.as_deref(), Some("save-1-1"));
        }
        other => panic!("Expected SaveRequested, got {other:?}"),
    }

    // Format Saved & Failed
    let saved_msg = GridCraftAdapter::format_saved("save-1-1");
    assert_eq!(saved_msg.type_name, "gridcraft:saved");
    assert_eq!(saved_msg.payload.get("id").and_then(|v| v.as_str()), Some("save-1-1"));

    let failed_msg = GridCraftAdapter::format_failed("EACCES: permission denied", Some("save-1-1"));
    assert_eq!(failed_msg.type_name, "gridcraft:failed");
    assert_eq!(failed_msg.payload.get("error").and_then(|v| v.as_str()), Some("EACCES: permission denied"));
    assert_eq!(failed_msg.payload.get("id").and_then(|v| v.as_str()), Some("save-1-1"));

    // Format Open
    let open_msg = adapter.format_open("sample.xlsx", &[9, 9, 9]).unwrap();
    assert_eq!(open_msg.type_name, "gridcraft:open");
    assert_eq!(open_msg.binary_data, Some(vec![9, 9, 9]));
}

#[test]
fn test_unrecognized_message_rejection() {
    let adapter = GridCraftAdapter;
    let foreign = RawBridgeMessage {
        type_name: "unrelated:event".to_string(),
        payload: json!({ "type": "unrelated:event" }),
        binary_data: None,
    };
    assert!(adapter.parse_incoming(&foreign).is_err());
}
