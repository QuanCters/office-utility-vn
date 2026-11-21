use crate::micro_apps::MicroAppId;
use serde::{Deserialize, Serialize};

/// Phương thức runtime mà micro-app sử dụng để hiển thị.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeKind {
    /// Ứng dụng Leptos CSR thuần túy (VN Office, Media Studio).
    NativeLeptos,
    /// Ứng dụng WebAssembly Canvas nhúng qua iframe sandbox cùng origin (PDF Studio, Data Lab).
    IframeHosted,
}

/// Các khả năng tích hợp mà micro-app hỗ trợ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntegrationCapability {
    /// Nhận và mở tệp tài liệu nhị phân từ Host (gridcraft:open, pdfcraft:open).
    DocumentOpen,
    /// Bắn thông điệp yêu cầu lưu tệp kèm mảng nhị phân (gridcraft:save, pdfcraft:save).
    DocumentSave,
    /// Báo cáo trạng thái có thay đổi chưa lưu (dirty notification).
    DirtyNotification,
    /// Giao diện điều khiển lệnh hai chiều qua window object (window.gridcraft).
    CommandApi,
    /// Hỗ trợ xuất định dạng PDF.
    ExportPdf,
}

/// Trạng thái sẵn sàng hoạt động của Micro-App trong Desktop Shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppAvailability {
    /// Sẵn sàng phục vụ người dùng chính thức trong production.
    Available,
    /// Tính năng đang trong giai đoạn thử nghiệm/spike kỹ thuật.
    Experimental,
    /// Dự kiến triển khai theo lộ trình.
    UnderDevelopment,
}

/// Mô tả cấu hình và đặc tả tích hợp tối thiểu của một Micro-App.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MicroAppDescriptor {
    pub id: MicroAppId,
    pub name: &'static str,
    pub runtime: RuntimeKind,
    pub entry_point: Option<&'static str>,
    pub capabilities: &'static [IntegrationCapability],
    pub availability: AppAvailability,
}

/// Bộ đăng ký trung tâm mở rộng cho Micro-Apps (không trùng lặp với ToolRegistry).
pub struct MicroAppRegistry;

impl MicroAppRegistry {
    pub const DESCRIPTORS: &'static [MicroAppDescriptor] = &[
        MicroAppDescriptor {
            id: MicroAppId::VnOffice,
            name: "VN Office",
            runtime: RuntimeKind::NativeLeptos,
            entry_point: None,
            capabilities: &[
                IntegrationCapability::DocumentOpen,
                IntegrationCapability::DocumentSave,
            ],
            availability: AppAvailability::Available,
        },
        MicroAppDescriptor {
            id: MicroAppId::PdfStudio,
            name: "PDF Studio",
            runtime: RuntimeKind::IframeHosted,
            entry_point: Some("/pdfcraft/index.html"),
            capabilities: &[
                IntegrationCapability::DocumentOpen,
                IntegrationCapability::DocumentSave,
                IntegrationCapability::DirtyNotification,
                IntegrationCapability::ExportPdf,
            ],
            availability: AppAvailability::Available,
        },
        MicroAppDescriptor {
            id: MicroAppId::MediaStudio,
            name: "Media Studio",
            runtime: RuntimeKind::NativeLeptos,
            entry_point: None,
            capabilities: &[
                IntegrationCapability::DocumentOpen,
                IntegrationCapability::DocumentSave,
            ],
            availability: AppAvailability::Available,
        },
        MicroAppDescriptor {
            id: MicroAppId::DataLab,
            name: "Data Lab",
            runtime: RuntimeKind::IframeHosted,
            entry_point: Some("/gridcraft/index.html"),
            capabilities: &[
                IntegrationCapability::DocumentOpen,
                IntegrationCapability::DocumentSave,
                IntegrationCapability::DirtyNotification,
                IntegrationCapability::CommandApi,
                IntegrationCapability::ExportPdf,
            ],
            availability: AppAvailability::Available,
        },
    ];

    pub fn get(id: MicroAppId) -> Option<&'static MicroAppDescriptor> {
        Self::DESCRIPTORS.iter().find(|d| d.id == id)
    }

    pub fn all() -> &'static [MicroAppDescriptor] {
        Self::DESCRIPTORS
    }

    pub fn by_runtime(runtime: RuntimeKind) -> Vec<&'static MicroAppDescriptor> {
        Self::DESCRIPTORS.iter().filter(|d| d.runtime == runtime).collect()
    }
}
