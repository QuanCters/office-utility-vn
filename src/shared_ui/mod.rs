pub mod back_button;
pub mod keyboard_shortcuts;
pub mod progress_bar;
pub mod task_executor;
pub mod workflow_runner;

pub use back_button::BackButton;
pub use keyboard_shortcuts::{
    handle_scoped_keydown_event, is_input_focused_event, match_scoped_shortcut,
    use_back_shortcut, use_cancel_shortcut, use_run_shortcut,
    use_scoped_keyboard_shortcuts, ScopedShortcutAction, ScopedShortcutsHandlers,
};
pub use progress_bar::ProgressBar;
pub use task_executor::{
    cancel_task_ipc, execute_workflow_ipc, open_output_folder_ipc, validate_payload_before_dispatch,
    EmptyToolPayload, GenericToolPayload, TaskExecutor, ToolPayloadContract,
};
pub use workflow_runner::{WorkflowDag, WorkflowRunner, WorkflowStep};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_shortcuts_matcher() {
        // Ctrl+Enter / Cmd+Enter kích hoạt Run bất kể có đang focus ô input hay không
        assert_eq!(
            match_scoped_shortcut("Enter", true, false),
            ScopedShortcutAction::Run
        );
        assert_eq!(
            match_scoped_shortcut("Enter", true, true),
            ScopedShortcutAction::Run
        );

        // Escape kích hoạt Cancel
        assert_eq!(
            match_scoped_shortcut("Escape", false, false),
            ScopedShortcutAction::Cancel
        );
        assert_eq!(
            match_scoped_shortcut("Escape", false, true),
            ScopedShortcutAction::Cancel
        );

        // Backspace kích hoạt Back KHI VÀ CHỈ KHI không focus ô nhập liệu
        assert_eq!(
            match_scoped_shortcut("Backspace", false, false),
            ScopedShortcutAction::Back
        );
        assert_eq!(
            match_scoped_shortcut("Backspace", false, true),
            ScopedShortcutAction::None
        );

        // Các phím thông thường khác
        assert_eq!(
            match_scoped_shortcut("Enter", false, false),
            ScopedShortcutAction::None
        );
        assert_eq!(
            match_scoped_shortcut("k", true, false),
            ScopedShortcutAction::None
        );
        assert_eq!(
            match_scoped_shortcut("Tab", false, false),
            ScopedShortcutAction::None
        );
    }

    #[test]
    fn test_workflow_dag_serde() {
        let step1 = WorkflowStep {
            id: "step-1".to_string(),
            tool_slug: "vietnamese-number-to-words".to_string(),
            name: "Đọc Số Thành Chữ".to_string(),
            description: "Chuyển số tiền VNĐ thành chữ quy chuẩn".to_string(),
        };
        let step2 = WorkflowStep {
            id: "step-2".to_string(),
            tool_slug: "vietqr-napas247-generator".to_string(),
            name: "Tạo Mã VietQR".to_string(),
            description: "Tạo mã QR thanh toán Napas 247".to_string(),
        };

        let dag = WorkflowDag {
            id: "dag-sample".to_string(),
            title: "Quy trình Thanh toán Hóa đơn".to_string(),
            steps: vec![step1.clone(), step2.clone()],
        };

        // Kiểm tra tuần tự hóa JSON
        let json_str = serde_json::to_string(&dag).expect("serialize dag");
        assert!(json_str.contains("Quy trình Thanh toán Hóa đơn"));
        assert!(json_str.contains("vietnamese-number-to-words"));

        // Kiểm tra giải tuần tự hóa
        let deserialized: WorkflowDag = serde_json::from_str(&json_str).expect("deserialize dag");
        assert_eq!(deserialized, dag);
        assert_eq!(deserialized.steps.len(), 2);
        assert_eq!(deserialized.steps[0].tool_slug, "vietnamese-number-to-words");
        assert_eq!(deserialized.steps[1].tool_slug, "vietqr-napas247-generator");
    }
}
