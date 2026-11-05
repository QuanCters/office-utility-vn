use std::sync::atomic::AtomicBool;
use office_shared::{CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress};

pub mod data_lab;
pub mod doc_suite;
pub mod media_studio;
pub mod sys_hub;
pub mod vn_office;

pub use doc_suite::pdf_page_organizer::{
    get_pdf_page_info, PageAction, PageInfo, PdfPageOrganizerOptions, PdfPageOrganizerTool,
};

/// Trait chuẩn cho mọi công cụ thuộc các phân hệ trong `office-engine`
pub trait OfficeTool: Send + Sync {
    /// Định danh slug của công cụ (ví dụ: "vietnamese-number-to-words")
    fn slug(&self) -> &'static str;

    /// Tên hiển thị tiếng Việt của công cụ
    fn name(&self) -> &'static str;

    /// Mô tả tóm tắt chức năng
    fn description(&self) -> &'static str;

    /// Thực thi tác vụ xử lý
    fn execute(
        &self,
        request: &DispatchTaskRequest,
        cancel_flag: &AtomicBool,
        on_progress: Option<&dyn Fn(TaskProgress)>,
    ) -> Result<DispatchTaskResponse, CommandError>;
}
