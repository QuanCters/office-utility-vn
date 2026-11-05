use leptos::prelude::*;
use crate::app::ToolItem;
use crate::micro_apps::{MicroAppId, ToolRegistry};

/// Phân loại 3 nhóm nghiệp vụ xử lý dữ liệu và bảng tính trong Data Lab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DataLabCategory {
    /// Bảng tính & Excel (cleaner, split, merge, joiner, unlocker)
    Spreadsheet,
    /// Chuyển đổi & Trộn tệp (data-structure-converter, docx-mailmerge, universal-converter, base64-codec)
    TransformMerge,
    /// Văn bản & Cấu trúc (diff-checker, list-set, pii-anonymizer, sql-formatter)
    TextStructure,
}

impl DataLabCategory {
    pub const ALL: [DataLabCategory; 3] = [
        DataLabCategory::Spreadsheet,
        DataLabCategory::TransformMerge,
        DataLabCategory::TextStructure,
    ];

    /// Tên hiển thị tiếng Việt của nhóm danh mục.
    pub fn label(self) -> &'static str {
        match self {
            DataLabCategory::Spreadsheet => "Bảng tính & Excel",
            DataLabCategory::TransformMerge => "Chuyển đổi & Trộn tệp",
            DataLabCategory::TextStructure => "Văn bản & Cấu trúc",
        }
    }

    /// Icon đại diện trực quan cho danh mục.
    pub fn icon(self) -> &'static str {
        match self {
            DataLabCategory::Spreadsheet => "📊",
            DataLabCategory::TransformMerge => "🔄",
            DataLabCategory::TextStructure => "📝",
        }
    }

    /// Danh sách slug công cụ thuộc nhóm nghiệp vụ.
    pub fn slugs(self) -> &'static [&'static str] {
        match self {
            DataLabCategory::Spreadsheet => &[
                "excel-cleaner-normalizer",
                "excel-split-by-column",
                "excel-multi-merger",
                "excel-visual-joiner",
                "excel-sheet-unlocker",
            ],
            DataLabCategory::TransformMerge => &[
                "data-structure-converter",
                "docx-template-mailmerge",
                "universal-file-converter-hub",
                "base64-file-codec",
            ],
            DataLabCategory::TextStructure => &[
                "diff-checker-text-sheets",
                "list-set-operations",
                "text-pii-anonymizer",
                "sql-formatter-linter",
            ],
        }
    }
}

/// Ánh xạ một slug công cụ sang nhóm danh mục Data Lab tương ứng.
pub fn category_for_tool(slug: &str) -> Option<DataLabCategory> {
    if slug == "universal-converter" {
        return Some(DataLabCategory::TransformMerge);
    }
    DataLabCategory::ALL.into_iter().find(|&cat| cat.slugs().contains(&slug))
}

/// Trạng thái hoạt động dùng chung (Shared Context) của Micro-App Data Lab.
///
/// Các file bảng tính, CSV, JSON, DOCX nạp vào một lần sẽ có sẵn cho toàn bộ 13 công cụ xử lý.
#[derive(Clone, Copy, Debug)]
pub struct DataLabState {
    /// Danh sách đường dẫn hoặc tên các tệp dữ liệu đã nạp vào workspace dùng chung.
    pub loaded_files: RwSignal<Vec<String>>,
    /// Chỉ số của file đang được kích hoạt trong danh sách loaded_files (None nếu chưa nạp file).
    pub active_file_index: RwSignal<Option<usize>>,
    /// Slug của công cụ Data Lab đang được chọn xử lý.
    pub active_tool: RwSignal<String>,
    /// Tên trang tính (sheet) hiện đang được kích hoạt trong file bảng tính.
    pub active_sheet: RwSignal<String>,
    /// Danh sách các sheet khả dụng trong file hiện tại.
    pub available_sheets: RwSignal<Vec<String>>,
    /// Cờ hiển thị chế độ Workflow Pipeline Macro DAG nội bộ.
    pub is_workflow_active: RwSignal<bool>,
    /// Trạng thái đang thực thi tác vụ.
    pub is_running: RwSignal<bool>,
    /// Thông điệp kết quả hoặc tiến trình thực thi.
    pub status_message: RwSignal<String>,
    /// Danh mục toolbar tab hiện tại đang được chọn xem.
    pub active_category: RwSignal<DataLabCategory>,
}

impl Default for DataLabState {
    fn default() -> Self {
        Self::new()
    }
}

impl DataLabState {
    /// Khởi tạo trạng thái Data Lab với công cụ mặc định "excel-cleaner-normalizer".
    pub fn new() -> Self {
        Self::with_initial_tool(None)
    }

    /// Khởi tạo trạng thái Data Lab với công cụ ban đầu tùy chọn.
    pub fn with_initial_tool(initial_tool: Option<String>) -> Self {
        let initial_slug = initial_tool
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "excel-cleaner-normalizer".to_string());

        let category = category_for_tool(&initial_slug).unwrap_or(DataLabCategory::Spreadsheet);

        Self {
            loaded_files: RwSignal::new(Vec::new()),
            active_file_index: RwSignal::new(None),
            active_tool: RwSignal::new(initial_slug),
            active_sheet: RwSignal::new("Sheet1".to_string()),
            available_sheets: RwSignal::new(vec![
                "Sheet1".to_string(),
                "Sheet2".to_string(),
                "Sheet3".to_string(),
            ]),
            is_workflow_active: RwSignal::new(false),
            is_running: RwSignal::new(false),
            status_message: RwSignal::new(String::new()),
            active_category: RwSignal::new(category),
        }
    }

    /// Thêm một tệp dữ liệu vào danh sách làm việc chung.
    /// Nếu chưa có tệp nào đang được chọn, tự động đặt tệp đầu tiên làm active.
    pub fn add_file(&self, path: impl Into<String>) {
        let path_str = path.into();
        if path_str.trim().is_empty() {
            return;
        }
        self.loaded_files.update(|f| f.push(path_str));
        if self.active_file_index.get_untracked().is_none() {
            self.active_file_index.set(Some(0));
        }
    }

    /// Thêm nhiều tệp dữ liệu vào danh sách làm việc chung.
    pub fn add_files(&self, paths: impl IntoIterator<Item = impl Into<String>>) {
        let mut added = false;
        self.loaded_files.update(|f| {
            for p in paths {
                let s = p.into();
                if !s.trim().is_empty() {
                    f.push(s);
                    added = true;
                }
            }
        });
        if added && self.active_file_index.get_untracked().is_none() {
            self.active_file_index.set(Some(0));
        }
    }

    /// Xóa một tệp khỏi danh sách làm việc theo chỉ số và điều chỉnh active_file_index an toàn.
    pub fn remove_file(&self, index: usize) {
        let initial_len = self.loaded_files.with(|f| f.len());
        if index >= initial_len {
            return;
        }

        self.loaded_files.update(|files| {
            files.remove(index);
        });

        let new_len = self.loaded_files.with(|f| f.len());
        if new_len == 0 {
            self.active_file_index.set(None);
        } else {
            self.active_file_index.update(|opt_idx| {
                if let Some(active) = *opt_idx {
                    if active == index {
                        if active >= new_len {
                            *opt_idx = Some(new_len - 1);
                        } else {
                            *opt_idx = Some(active);
                        }
                    } else if active > index {
                        *opt_idx = Some(active - 1);
                    }
                }
            });
        }
    }

    /// Xóa toàn bộ danh sách tệp dữ liệu đã nạp.
    pub fn clear_files(&self) {
        self.loaded_files.set(Vec::new());
        self.active_file_index.set(None);
    }

    /// Lấy đường dẫn tệp dữ liệu đang được kích hoạt (nếu có).
    pub fn active_file_path(&self) -> Option<String> {
        let idx = self.active_file_index.get()?;
        self.loaded_files.with(|files| files.get(idx).cloned())
    }

    /// Lấy đường dẫn tệp dữ liệu đang được kích hoạt mà không theo dõi phản ứng (untracked).
    pub fn active_file_path_untracked(&self) -> Option<String> {
        let idx = self.active_file_index.get_untracked()?;
        self.loaded_files.with_untracked(|files| files.get(idx).cloned())
    }

    /// Lấy tên tệp rút gọn (basename) của tệp dữ liệu đang kích hoạt.
    pub fn active_file_name(&self) -> Option<String> {
        self.active_file_path().map(|p| {
            p.rsplit(['/', '\\'])
                .next()
                .unwrap_or(&p)
                .to_string()
        })
    }

    /// Lấy tên tệp rút gọn (basename) mà không theo dõi phản ứng (untracked).
    pub fn active_file_name_untracked(&self) -> Option<String> {
        self.active_file_path_untracked().map(|p| {
            p.rsplit(['/', '\\'])
                .next()
                .unwrap_or(&p)
                .to_string()
        })
    }

    /// Lấy phần mở rộng định dạng của tệp đang kích hoạt (chữ thường, ví dụ: "xlsx", "csv", "json").
    pub fn active_file_extension(&self) -> Option<String> {
        self.active_file_name().and_then(|name| {
            name.rsplit('.')
                .next()
                .filter(|ext| *ext != name)
                .map(|ext| ext.to_lowercase())
        })
    }

    /// Chọn một tệp cụ thể trong danh sách theo chỉ số index.
    pub fn select_file(&self, index: usize) {
        let len = self.loaded_files.with(|f| f.len());
        if index < len {
            self.active_file_index.set(Some(index));
        }
    }

    /// Chọn một công cụ Data Lab, tự động đồng bộ danh mục toolbar và thoát chế độ workflow.
    pub fn select_tool(&self, slug: &str) {
        self.active_tool.set(slug.to_string());
        self.is_workflow_active.set(false);
        if let Some(cat) = category_for_tool(slug) {
            self.active_category.set(cat);
        }
    }

    /// Chuyển tab danh mục toolbar. Nếu công cụ hiện tại không thuộc tab mới, chọn công cụ đầu tiên của tab.
    pub fn select_category(&self, cat: DataLabCategory) {
        self.active_category.set(cat);
        self.is_workflow_active.set(false);
        let cur_tool = self.active_tool.get();
        if !cat.slugs().contains(&cur_tool.as_str())
            && cur_tool != "universal-converter"
            && let Some(&first_slug) = cat.slugs().first()
        {
            self.active_tool.set(first_slug.to_string());
        }
    }

    /// Đổi trang tính (sheet) hiện tại đang chọn.
    pub fn set_sheet(&self, sheet: impl Into<String>) {
        self.active_sheet.set(sheet.into());
    }

    /// Chọn trang tính (alias cho set_sheet).
    pub fn select_sheet(&self, sheet: impl Into<String>) {
        self.set_sheet(sheet);
    }

    /// Lấy tên trang tính hiện đang kích hoạt.
    pub fn active_sheet_name(&self) -> String {
        self.active_sheet.get()
    }

    /// Mở giao diện chuỗi quy trình Workflow Macro DAG.
    pub fn open_workflow(&self) {
        self.is_workflow_active.set(true);
    }

    /// Đóng giao diện Workflow và trở về công cụ đơn lẻ.
    pub fn close_workflow(&self) {
        self.is_workflow_active.set(false);
    }

    /// Kiểm tra xem chế độ Workflow có đang mở không.
    pub fn is_workflow(&self) -> bool {
        self.is_workflow_active.get()
    }

    /// Kiểm tra xem có tác vụ nào đang thực thi không.
    pub fn is_running(&self) -> bool {
        self.is_running.get_untracked()
    }

    /// Số lượng file đã nạp vào workspace.
    pub fn loaded_files_count(&self) -> usize {
        self.loaded_files.with_untracked(|f| f.len())
    }

    /// Kiểm tra xem đã có ít nhất một file nạp vào chưa.
    pub fn has_files(&self) -> bool {
        self.loaded_files.with_untracked(|f| !f.is_empty())
    }

    /// Lấy slug của công cụ hiện tại.
    pub fn active_tool_slug(&self) -> String {
        self.active_tool.get()
    }

    /// Tìm kiếm metadata ToolItem từ slug trong ToolRegistry (hỗ trợ alias universal-converter).
    pub fn find_tool(&self, slug: &str) -> Option<&'static ToolItem> {
        let lookup_slug = if slug == "universal-converter" {
            "universal-file-converter-hub"
        } else {
            slug
        };
        self.tools().iter().find(|t| t.slug == lookup_slug)
    }

    /// Danh sách công cụ thuộc phạm vi Data Lab (ủy quyền từ ToolRegistry).
    pub fn tools(&self) -> &'static [ToolItem] {
        ToolRegistry::micro_app_tools(MicroAppId::DataLab)
    }

    /// Danh sách công cụ khả dụng (alias cho tools()).
    pub fn available_tools(&self) -> &'static [ToolItem] {
        self.tools()
    }
}
