use office_utility_vn_ui::app::ToolItem;
use office_utility_vn_ui::micro_apps::{micro_app_tools, MicroAppId};
use office_utility_vn_ui::shared_ui::{
    match_scoped_shortcut, ScopedShortcutAction, ScopedShortcutsHandlers, WorkflowDag, WorkflowStep,
};

#[test]
fn test_match_scoped_shortcuts_rules() {
    // 1. Phím Ctrl+Enter / Cmd+Enter: kích hoạt Run
    assert_eq!(
        match_scoped_shortcut("Enter", true, false),
        ScopedShortcutAction::Run
    );
    assert_eq!(
        match_scoped_shortcut("Enter", true, true),
        ScopedShortcutAction::Run
    );

    // 2. Phím Escape: kích hoạt Cancel (kể cả khi đang focus input để đóng dropdown / hủy)
    assert_eq!(
        match_scoped_shortcut("Escape", false, false),
        ScopedShortcutAction::Cancel
    );
    assert_eq!(
        match_scoped_shortcut("Escape", false, true),
        ScopedShortcutAction::Cancel
    );

    // 3. Phím Backspace: KHI KHÔNG focus ô nhập => Quay lại màn hình chính (Back)
    assert_eq!(
        match_scoped_shortcut("Backspace", false, false),
        ScopedShortcutAction::Back
    );

    // 4. Phím Backspace: KHI ĐANG focus ô nhập => KHÔNG quay lại, cho phép người dùng xóa ký tự (None)
    assert_eq!(
        match_scoped_shortcut("Backspace", false, true),
        ScopedShortcutAction::None
    );

    // 5. Phím ngẫu nhiên khác
    assert_eq!(
        match_scoped_shortcut("a", false, false),
        ScopedShortcutAction::None
    );
    assert_eq!(
        match_scoped_shortcut("Space", false, false),
        ScopedShortcutAction::None
    );
}

#[test]
fn test_scoped_shortcuts_handlers_default() {
    let handlers = ScopedShortcutsHandlers::default();
    assert!(handlers.on_run.is_none());
    assert!(handlers.on_cancel.is_none());
    assert!(handlers.on_back.is_none());
}

#[test]
fn test_workflow_dag_structure_and_json_roundtrip() {
    let vn_tools = micro_app_tools(MicroAppId::VnOffice);
    assert!(!vn_tools.is_empty(), "VN Office must have tools");

    let steps = vn_tools
        .iter()
        .take(3)
        .enumerate()
        .map(|(idx, tool)| WorkflowStep {
            id: format!("step-{}", idx + 1),
            tool_slug: tool.slug.to_string(),
            name: tool.name.to_string(),
            description: tool.description.to_string(),
        })
        .collect::<Vec<_>>();

    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].id, "step-1");
    assert_eq!(steps[1].id, "step-2");
    assert_eq!(steps[2].id, "step-3");

    let dag = WorkflowDag {
        id: "vn-office-pipeline-1".to_string(),
        title: "Chuỗi Xử lý Thuế & Bảng lương Tự động".to_string(),
        steps: steps.clone(),
    };

    let serialized = serde_json::to_string(&dag).expect("json serialize");
    let deserialized: WorkflowDag = serde_json::from_str(&serialized).expect("json deserialize");

    assert_eq!(dag, deserialized);
    assert_eq!(deserialized.steps.len(), 3);
    assert_eq!(deserialized.steps[0].name, steps[0].name);
}

#[test]
fn test_workflow_steps_scoped_to_available_tools() {
    let pdf_tools: &'static [ToolItem] = micro_app_tools(MicroAppId::PdfStudio);
    let sample_tool = &pdf_tools[0];

    // Chỉ các công cụ có trong available_tools mới được coi là hợp lệ
    let step = WorkflowStep {
        id: "step-1".to_string(),
        tool_slug: sample_tool.slug.to_string(),
        name: sample_tool.name.to_string(),
        description: sample_tool.description.to_string(),
    };

    let is_valid = pdf_tools.iter().any(|t| t.slug == step.tool_slug);
    assert!(is_valid, "Step tool_slug must be part of pdf_tools");

    // Công cụ không thuộc PDF Studio (ví dụ tool của VN Office) sẽ không hợp lệ
    let non_pdf_step = WorkflowStep {
        id: "step-invalid".to_string(),
        tool_slug: "vietnamese-number-to-words".to_string(),
        name: "Đọc Số Thành Chữ".to_string(),
        description: "Invalid for PDF Studio".to_string(),
    };

    let is_non_pdf_valid = pdf_tools.iter().any(|t| t.slug == non_pdf_step.tool_slug);
    assert!(!is_non_pdf_valid, "Cross-app tool must not validate inside PDF Studio scope");
}

#[test]
fn test_progress_bar_percentage_normalization_logic() {
    let normalize = |p: f32| -> f32 {
        let val = if p <= 1.0 { p * 100.0 } else { p };
        val.clamp(0.0, 100.0)
    };

    // 0.0 -> 0%
    assert_eq!(normalize(0.0), 0.0);
    // 0.45 -> 45%
    assert!((normalize(0.45) - 45.0).abs() < 0.001);
    // 1.0 -> 100%
    assert_eq!(normalize(1.0), 100.0);
    // 75.0 -> 75%
    assert_eq!(normalize(75.0), 75.0);
    // âm -> clamp 0%
    assert_eq!(normalize(-0.5), 0.0);
    // vượt ngưỡng -> clamp 100%
    assert_eq!(normalize(150.0), 100.0);
}

#[test]
fn test_workflow_dag_conversion_to_backend_nodes_and_edges() {
    let steps = vec![
        WorkflowStep {
            id: "node-1".to_string(),
            tool_slug: "markdown-to-styled-pdf".to_string(),
            name: "Biên dịch Markdown sang PDF".to_string(),
            description: "Step 1".to_string(),
        },
        WorkflowStep {
            id: "node-2".to_string(),
            tool_slug: "pdf-page-number-header".to_string(),
            name: "Đánh số trang PDF".to_string(),
            description: "Step 2".to_string(),
        },
        WorkflowStep {
            id: "node-3".to_string(),
            tool_slug: "pdf-compress".to_string(),
            name: "Nén giảm dung lượng PDF".to_string(),
            description: "Step 3".to_string(),
        },
    ];

    let dag = WorkflowDag {
        id: "pipeline-test-1".to_string(),
        title: "Chuỗi Xử lý Văn bản Xuất bản".to_string(),
        steps: steps.clone(),
    };

    // Tạo các edges tuần tự
    let edges = (0..dag.steps.len() - 1)
        .map(|i| (dag.steps[i].id.clone(), dag.steps[i + 1].id.clone()))
        .collect::<Vec<_>>();

    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0], ("node-1".to_string(), "node-2".to_string()));
    assert_eq!(edges[1], ("node-2".to_string(), "node-3".to_string()));
}
