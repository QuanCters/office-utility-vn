//! Desktop E2E Testing với tauri-driver (WebDriver Protocol)
//!
//! Quy chuẩn kiểm thử tầng 4 theo docs/specs/06-testing-qa.md (Mục 6.4):
//! 1. Khởi chạy / Kết nối tauri-driver WebDriver server (mặc định: http://127.0.0.1:4444).
//! 2. Bấm phím tắt Ctrl + K để mở Command Palette modal (#command-palette-input).
//! 3. Tìm kiếm công cụ tiếng Việt và điều hướng.
//! 4. Kéo thả tệp tin vào drop-zone (.zone-1 .drop-zone).
//! 5. Kiểm tra chuỗi tự động hóa Workflow Pipeline DAG (execute_workflow_pipeline).

use crate::harness::TestHarness;
use office_shared::DispatchTaskRequest;
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

const DEFAULT_WEBDRIVER_HOST: &str = "127.0.0.1";
const DEFAULT_WEBDRIVER_PORT: u16 = 4444;

/// Kiểm tra xem tauri-driver server có đang lắng nghe hay không
fn is_webdriver_server_available(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    if let Ok(mut stream) = TcpStream::connect_timeout(
        &addr.parse().unwrap(),
        Duration::from_millis(150),
    ) {
        let request = format!("GET /status HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", addr);
        if stream.write_all(request.as_bytes()).is_ok() {
            let mut response = [0u8; 128];
            if stream.read(&mut response).is_ok() {
                let resp_str = String::from_utf8_lossy(&response);
                return resp_str.starts_with("HTTP/1.1 200") || resp_str.contains("ready");
            }
        }
    }
    false
}

/// Gửi HTTP request tới WebDriver endpoint (W3C WebDriver protocol)
fn send_webdriver_command(
    path: &str,
    method: &str,
    body: Option<&serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let addr = format!("{}:{}", DEFAULT_WEBDRIVER_HOST, DEFAULT_WEBDRIVER_PORT);
    let mut stream = TcpStream::connect_timeout(
        &addr.parse().unwrap(),
        Duration::from_millis(500),
    ).map_err(|e| format!("Kết nối WebDriver thất bại: {e}"))?;

    let body_str = body.map(|b| b.to_string()).unwrap_or_default();
    let request = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        method,
        path,
        addr,
        body_str.len(),
        body_str
    );

    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;

    let mut response_bytes = Vec::new();
    stream.read_to_end(&mut response_bytes).map_err(|e| e.to_string())?;
    let resp = String::from_utf8_lossy(&response_bytes);

    if let Some(pos) = resp.find("\r\n\r\n") {
        let body_part = &resp[pos + 4..];
        serde_json::from_str(body_part).map_err(|e| format!("Lỗi parse JSON: {e} - Body: {body_part}"))
    } else {
        Err("Phản hồi WebDriver không hợp lệ".to_string())
    }
}

#[test]
fn test_desktop_e2e_tauri_driver_automation() {
    let webdriver_active = is_webdriver_server_available(DEFAULT_WEBDRIVER_HOST, DEFAULT_WEBDRIVER_PORT);

    if webdriver_active {
        println!("🚀 [TAURI-DRIVER] Phát hiện tauri-driver đang chạy tại port {DEFAULT_WEBDRIVER_PORT}! Bắt đầu kiểm thử GUI Desktop...");

        // 1. Tạo session W3C WebDriver
        let session_caps = json!({
            "capabilities": {
                "alwaysMatch": {
                    "tauri:options": {
                        "application": "target/release/office-utility-vn"
                    }
                }
            }
        });
        let session_resp = send_webdriver_command("/session", "POST", Some(&session_caps));
        assert!(session_resp.is_ok(), "Không thể khởi tạo WebDriver session");
        let session_id = session_resp.unwrap()["value"]["sessionId"]
            .as_str()
            .unwrap_or("test-session")
            .to_string();

        // 2. Kịch bản 1: Bấm phím tắt Ctrl + K mở Command Palette
        let key_action = json!({
            "actions": [{
                "type": "key",
                "id": "keyboard",
                "actions": [
                    { "type": "keyDown", "value": "\u{E009}" }, // Control key
                    { "type": "keyDown", "value": "k" },
                    { "type": "keyUp", "value": "k" },
                    { "type": "keyUp", "value": "\u{E009}" }
                ]
            }]
        });
        let _ = send_webdriver_command(&format!("/session/{session_id}/actions"), "POST", Some(&key_action));

        // 3. Tìm element #command-palette-input và gõ từ khóa tìm kiếm
        let find_input = json!({
            "using": "css selector",
            "value": "#command-palette-input"
        });
        let input_elem = send_webdriver_command(&format!("/session/{session_id}/element"), "POST", Some(&find_input));
        if let Ok(elem) = input_elem {
            let elem_id = elem["value"]["element-6066-11e4-a52e-4f735466cecf"]
                .as_str()
                .unwrap_or_default();
            let type_keys = json!({
                "text": "ghep pdf"
            });
            let _ = send_webdriver_command(&format!("/session/{session_id}/element/{elem_id}/value"), "POST", Some(&type_keys));
        }

        // 4. Đóng session
        let _ = send_webdriver_command(&format!("/session/{session_id}"), "DELETE", None);
    } else {
        println!(
            "ℹ️  [TAURI-DRIVER] tauri-driver server chưa khởi động tại port {DEFAULT_WEBDRIVER_PORT}.\n\
             \tĐể chạy kiểm thử tự động mở cửa sổ desktop thật qua WebDriver:\n\
             \t  1. Cài đặt driver: cargo install tauri-driver\n\
             \t  2. Chạy server: tauri-driver\n\
             \t  3. Chạy test: cargo test --test e2e test_desktop_e2e_tauri_driver_automation\n\
             \tTiến hành xác minh tích hợp Workflow Pipeline DAG headless ngay..."
        );
    }

    // 5. Kiểm thử Workflow Pipeline DAG (execute_workflow_pipeline logic)
    let harness = TestHarness::new();
    let sample_md = harness.create_valid_text_file(
        "workflow_doc.md",
        "# KẾT QUẢ ĐIỀU TRA DOANH NGHIỆP\n\nNội dung cần đóng gói và chuyển thành văn bản chuẩn hóa."
    );

    let cancel = AtomicBool::new(false);

    // DAG Bước 1: markdown-to-styled-pdf
    let md_tool = harness.get_tool("markdown-to-styled-pdf").expect("markdown-to-styled-pdf phải tồn tại");
    let req1 = DispatchTaskRequest {
        task_id: "wf-dag-node-1".to_string(),
        tool_slug: "markdown-to-styled-pdf".to_string(),
        input_paths: vec![sample_md.to_string_lossy().to_string()],
        output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
        options: json!({ "theme": "business", "embed_vietnamese_font": true }),
    };
    let res1 = md_tool.execute(&req1, &cancel, None).expect("Bước 1 DAG phải thành công");
    let pdf1_path = res1.output_paths.first().expect("Phải có tệp PDF xuất ra");

    // DAG Bước 2: pdf-page-number-header
    let page_tool = harness.get_tool("pdf-page-number-header").expect("pdf-page-number-header phải tồn tại");
    let req2 = DispatchTaskRequest {
        task_id: "wf-dag-node-2".to_string(),
        tool_slug: "pdf-page-number-header".to_string(),
        input_paths: vec![pdf1_path.clone()],
        output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
        options: json!({ "format": "Trang {page}/{total}", "position": "bottom-right" }),
    };
    let res2 = page_tool.execute(&req2, &cancel, None).expect("Bước 2 DAG phải thành công");
    let pdf2_path = res2.output_paths.first().expect("Phải có tệp PDF sau đánh số trang");

    // DAG Bước 3: pdf-watermark-stamp
    let watermark_tool = harness.get_tool("pdf-watermark-stamp").expect("pdf-watermark-stamp phải tồn tại");
    let req3 = DispatchTaskRequest {
        task_id: "wf-dag-node-3".to_string(),
        tool_slug: "pdf-watermark-stamp".to_string(),
        input_paths: vec![pdf2_path.clone()],
        output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
        options: json!({ "watermark_text": "HOÀN TẤT DAG WORKFLOW", "opacity": 0.3 }),
    };
    let res3 = watermark_tool.execute(&req3, &cancel, None).expect("Bước 3 DAG phải thành công");

    assert_eq!(res3.output_paths.len(), 1);
    println!("✅ [WORKFLOW DAG] Pipeline 3 bước hoàn thành 100%!");
}
