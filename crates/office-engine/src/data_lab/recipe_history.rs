use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use serde::{Deserialize, Serialize};

use super::excel_cleaner_normalizer::{
    clean_spaces_and_capitalize, normalize_date_string, split_vietnamese_name,
};
use super::excel_reader::{format_table_to_csv, read_tabular_rows};
use crate::OfficeTool;

/// Các hành động xử lý dữ liệu được hỗ trợ trong một bước Recipe
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecipeAction {
    CleanSpaces,
    NormalizeDates,
    SplitNames,
    Deduplicate,
    FuzzyMerge {
        col_idx: usize,
        mappings: HashMap<String, String>,
    },
    DropColumn {
        col_idx: usize,
    },
    CustomTransform {
        description: String,
    },
}

/// Một bước chuyển đổi trong kịch bản xử lý dữ liệu
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeStep {
    pub id: String,
    pub action: RecipeAction,
    pub description: String,
    pub created_at: String,
}

impl RecipeStep {
    pub fn new(id: impl Into<String>, action: RecipeAction, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            action,
            description: description.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Toàn bộ kịch bản / quy trình xử lý dữ liệu (Recipe)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataRecipe {
    pub name: String,
    pub version: String,
    pub description: String,
    pub steps: Vec<RecipeStep>,
}

impl DataRecipe {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: description.into(),
            steps: Vec::new(),
        }
    }

    pub fn add_step(&mut self, step: RecipeStep) {
        self.steps.push(step);
    }

    pub fn to_json(&self) -> Result<String, CommandError> {
        export_recipe_to_json(self)
    }

    pub fn from_json(json_str: &str) -> Result<Self, CommandError> {
        import_recipe_from_json(json_str)
    }
}

/// Báo cáo thống kê chi tiết sau khi thực thi Recipe
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeExecutionResult {
    pub total_steps: usize,
    pub rows_before: usize,
    pub rows_after: usize,
    pub executed_steps: Vec<String>,
}

/// Xuất DataRecipe sang chuỗi JSON định dạng chuẩn có thụt dòng
pub fn export_recipe_to_json(recipe: &DataRecipe) -> Result<String, CommandError> {
    serde_json::to_string_pretty(recipe).map_err(|e| CommandError::ValidationFailed {
        reason: format!("Không thể xuất Recipe sang JSON: {e}"),
    })
}

/// Nhập DataRecipe từ chuỗi JSON
pub fn import_recipe_from_json(json_str: &str) -> Result<DataRecipe, CommandError> {
    serde_json::from_str(json_str).map_err(|e| CommandError::ValidationFailed {
        reason: format!("Định dạng Recipe JSON không hợp lệ: {e}"),
    })
}

/// Bộ quản lý lịch sử thao tác với cơ chế Undo/Redo hai ngăn xếp
#[derive(Debug, Clone, Default)]
pub struct RecipeHistoryManager {
    undo_stack: Vec<RecipeStep>,
    redo_stack: Vec<RecipeStep>,
}

impl RecipeHistoryManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Ghi nhận một bước mới (xóa redo stack)
    pub fn record_step(&mut self, step: RecipeStep) {
        self.redo_stack.clear();
        self.undo_stack.push(step);
    }

    /// Hoàn tác bước gần nhất
    pub fn undo(&mut self) -> Option<RecipeStep> {
        let step = self.undo_stack.pop()?;
        self.redo_stack.push(step.clone());
        Some(step)
    }

    /// Làm lại bước vừa hoàn tác
    pub fn redo(&mut self) -> Option<RecipeStep> {
        let step = self.redo_stack.pop()?;
        self.undo_stack.push(step.clone());
        Some(step)
    }

    /// Kiểm tra xem có thể hoàn tác hay không
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Kiểm tra xem có thể làm lại hay không
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Danh sách các bước hiện đang có hiệu lực
    pub fn current_steps(&self) -> &[RecipeStep] {
        &self.undo_stack
    }

    /// Xóa toàn bộ lịch sử
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Đóng gói các bước hiện tại thành DataRecipe
    pub fn to_recipe(&self, name: &str, description: &str) -> DataRecipe {
        DataRecipe {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: description.to_string(),
            steps: self.undo_stack.clone(),
        }
    }

    /// Nạp các bước từ DataRecipe vào Undo stack
    pub fn from_recipe(recipe: &DataRecipe) -> Self {
        Self {
            undo_stack: recipe.steps.clone(),
            redo_stack: Vec::new(),
        }
    }
}

/// Replay tuần tự toàn bộ các bước trong recipe lên dữ liệu bảng tính
pub fn apply_recipe(
    rows: &[Vec<String>],
    recipe: &DataRecipe,
    cancel_flag: &AtomicBool,
) -> Result<Vec<Vec<String>>, CommandError> {
    let (output_rows, _) = apply_recipe_with_result(rows, recipe, cancel_flag)?;
    Ok(output_rows)
}

/// Replay các bước trong recipe kèm báo cáo thống kê RecipeExecutionResult
pub fn apply_recipe_with_result(
    rows: &[Vec<String>],
    recipe: &DataRecipe,
    cancel_flag: &AtomicBool,
) -> Result<(Vec<Vec<String>>, RecipeExecutionResult), CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let rows_before = rows.len();
    let mut current_rows: Vec<Vec<String>> = rows.to_vec();
    let mut executed_steps = Vec::new();

    for step in &recipe.steps {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        match &step.action {
            RecipeAction::CleanSpaces => {
                let mut next_rows = Vec::with_capacity(current_rows.len());
                for row in current_rows {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err(CommandError::TaskCancelled);
                    }
                    let cleaned_row: Vec<String> = row
                        .iter()
                        .map(|c| clean_spaces_and_capitalize(c))
                        .collect();
                    next_rows.push(cleaned_row);
                }
                current_rows = next_rows;
            }
            RecipeAction::NormalizeDates => {
                let mut next_rows = Vec::with_capacity(current_rows.len());
                for row in current_rows {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err(CommandError::TaskCancelled);
                    }
                    let normalized_row: Vec<String> = row
                        .iter()
                        .map(|c| normalize_date_string(c))
                        .collect();
                    next_rows.push(normalized_row);
                }
                current_rows = next_rows;
            }
            RecipeAction::SplitNames => {
                let mut next_rows = Vec::with_capacity(current_rows.len());
                for row in current_rows {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err(CommandError::TaskCancelled);
                    }
                    let mut new_row = row.clone();
                    if !row.is_empty() {
                        let parts = split_vietnamese_name(&row[0]);
                        if !parts.ho.is_empty() || !parts.ten.is_empty() {
                            new_row.push(parts.ho);
                            new_row.push(parts.dem);
                            new_row.push(parts.ten);
                        }
                    }
                    next_rows.push(new_row);
                }
                current_rows = next_rows;
            }
            RecipeAction::Deduplicate => {
                let mut seen = HashSet::new();
                let mut next_rows = Vec::with_capacity(current_rows.len());
                for row in current_rows {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err(CommandError::TaskCancelled);
                    }
                    let key = row.join("\t");
                    if seen.insert(key) {
                        next_rows.push(row);
                    }
                }
                current_rows = next_rows;
            }
            RecipeAction::FuzzyMerge { col_idx, mappings } => {
                let mut next_rows = Vec::with_capacity(current_rows.len());
                for mut row in current_rows {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err(CommandError::TaskCancelled);
                    }
                    if *col_idx < row.len() {
                        let cell = &row[*col_idx];
                        if let Some(target) = mappings.get(cell) {
                            row[*col_idx] = target.clone();
                        } else if let Some(target) = mappings.get(cell.trim()) {
                            row[*col_idx] = target.clone();
                        } else {
                            let cell_lower = cell.trim().to_lowercase();
                            for (k, v) in mappings {
                                if k.trim().to_lowercase() == cell_lower {
                                    row[*col_idx] = v.clone();
                                    break;
                                }
                            }
                        }
                    }
                    next_rows.push(row);
                }
                current_rows = next_rows;
            }
            RecipeAction::DropColumn { col_idx } => {
                let mut next_rows = Vec::with_capacity(current_rows.len());
                for mut row in current_rows {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err(CommandError::TaskCancelled);
                    }
                    if *col_idx < row.len() {
                        row.remove(*col_idx);
                    }
                    next_rows.push(row);
                }
                current_rows = next_rows;
            }
            RecipeAction::CustomTransform { description } => {
                let mut next_rows = Vec::with_capacity(current_rows.len());
                let desc_lower = description.to_lowercase();
                for row in current_rows {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err(CommandError::TaskCancelled);
                    }
                    let transformed_row: Vec<String> = if desc_lower.contains("upper")
                        || desc_lower.contains("hoa")
                    {
                        row.iter().map(|c| c.to_uppercase()).collect()
                    } else if desc_lower.contains("lower") || desc_lower.contains("thường") {
                        row.iter().map(|c| c.to_lowercase()).collect()
                    } else if desc_lower.contains("trim") {
                        row.iter().map(|c| c.trim().to_string()).collect()
                    } else {
                        row
                    };
                    next_rows.push(transformed_row);
                }
                current_rows = next_rows;
            }
        }

        executed_steps.push(format!("{}: {}", step.id, step.description));
    }

    let rows_after = current_rows.len();
    let result_meta = RecipeExecutionResult {
        total_steps: recipe.steps.len(),
        rows_before,
        rows_after,
        executed_steps,
    };

    Ok((current_rows, result_meta))
}

pub struct RecipeReplayTool;

impl Default for RecipeReplayTool {
    fn default() -> Self {
        Self
    }
}

impl OfficeTool for RecipeReplayTool {
    fn slug(&self) -> &'static str {
        "recipe-replay"
    }

    fn name(&self) -> &'static str {
        "Tái thi hành Quy trình Dữ liệu (Recipe Replayer)"
    }

    fn description(&self) -> &'static str {
        "Tái áp dụng toàn bộ các bước làm sạch, gom cụm và chuẩn hóa đã lưu (Recipe JSON) lên bảng tính mới một cách tự động."
    }

    fn execute(
        &self,
        request: &DispatchTaskRequest,
        cancel_flag: &AtomicBool,
        on_progress: Option<&dyn Fn(TaskProgress)>,
    ) -> Result<DispatchTaskResponse, CommandError> {
        let start_time = Instant::now();

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let recipe: DataRecipe = if let Some(rj) = request
            .options
            .get("recipe_json")
            .and_then(|v| v.as_str())
        {
            DataRecipe::from_json(rj)?
        } else if let Some(rj_obj) = request.options.get("recipe") {
            if let Some(rj_str) = rj_obj.as_str() {
                DataRecipe::from_json(rj_str)?
            } else {
                serde_json::from_value(rj_obj.clone()).map_err(|e| {
                    CommandError::ValidationFailed {
                        reason: format!("Cấu hình 'recipe' không hợp lệ: {e}"),
                    }
                })?
            }
        } else {
            serde_json::from_value(request.options.clone()).map_err(|e| {
                CommandError::ValidationFailed {
                    reason: format!(
                        "Không tìm thấy cấu hình Recipe hợp lệ (recipe_json hoặc recipe): {e}"
                    ),
                }
            })?
        };

        // Xử lý trực tiếp từ chuỗi nếu không có input_paths
        if request.input_paths.is_empty() {
            let text_val = request
                .options
                .get("text")
                .or_else(|| request.options.get("csv"))
                .and_then(|v| v.as_str());

            if let Some(text) = text_val {
                let raw_rows = read_tabular_rows(text.as_bytes(), cancel_flag)?;
                let processed_rows = apply_recipe(&raw_rows, &recipe, cancel_flag)?;
                let out_csv = format_table_to_csv(&processed_rows);

                return Ok(DispatchTaskResponse {
                    task_id: request.task_id.clone(),
                    status: "SUCCESS".to_string(),
                    output_paths: vec![out_csv.trim_end_matches('\n').to_string()],
                    failed_items: vec![],
                    elapsed_ms: start_time.elapsed().as_millis() as u64,
                });
            } else {
                return Err(CommandError::ValidationFailed {
                    reason: "Không có tệp đầu vào hoặc nội dung bảng tính cần xử lý".to_string(),
                });
            }
        }

        let total_files = request.input_paths.len();
        let mut output_paths = Vec::new();
        let mut failed_items = Vec::new();

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("recipe_replay_{}", request.task_id)),
        };

        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(CommandError::IoError {
                details: format!("Không thể tạo thư mục đầu ra {}: {e}", out_dir.display()),
            });
        }

        for (idx, in_path_str) in request.input_paths.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let in_path = Path::new(in_path_str);
            if !in_path.exists() {
                failed_items.push(FileFailureDetail {
                    input_path: in_path_str.clone(),
                    reason: "Tệp nguồn không tồn tại".to_string(),
                });
                continue;
            }

            let raw_bytes = match fs::read(in_path) {
                Ok(b) => b,
                Err(e) => {
                    failed_items.push(FileFailureDetail {
                        input_path: in_path_str.clone(),
                        reason: e.to_string(),
                    });
                    continue;
                }
            };

            // Kiểm tra magic bytes: Chặn các tệp nhị phân thực thi ELF hoặc PE (MZ)
            if raw_bytes.starts_with(b"\x7fELF") || raw_bytes.starts_with(b"MZ") {
                return Err(CommandError::InvalidFileSignature {
                    expected: "XLSX hoặc CSV".to_string(),
                    actual: "Tệp thực thi nhị phân bị chặn".to_string(),
                });
            }

            let raw_rows = match read_tabular_rows(&raw_bytes, cancel_flag) {
                Ok(r) => r,
                Err(e) => {
                    failed_items.push(FileFailureDetail {
                        input_path: in_path_str.clone(),
                        reason: format!("Lỗi đọc bảng tính: {e}"),
                    });
                    continue;
                }
            };

            let processed_rows = match apply_recipe(&raw_rows, &recipe, cancel_flag) {
                Ok(r) => r,
                Err(CommandError::TaskCancelled) => return Err(CommandError::TaskCancelled),
                Err(e) => {
                    failed_items.push(FileFailureDetail {
                        input_path: in_path_str.clone(),
                        reason: format!("Lỗi áp dụng recipe: {e}"),
                    });
                    continue;
                }
            };

            let formatted_csv = format_table_to_csv(&processed_rows);

            let filename = match in_path.file_name() {
                Some(s) => s.to_string_lossy().to_string(),
                None => format!("replayed_{}.csv", idx),
            };
            let out_file = out_dir.join(format!("replayed_{}", filename));

            if let Err(e) = fs::write(&out_file, formatted_csv.as_bytes()) {
                failed_items.push(FileFailureDetail {
                    input_path: in_path_str.clone(),
                    reason: e.to_string(),
                });
                continue;
            }

            output_paths.push(out_file.to_string_lossy().to_string());

            if let Some(cb) = on_progress {
                cb(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: (idx + 1) as u32,
                    total_steps: total_files as u32,
                    percentage: ((idx + 1) as f32 / total_files as f32) * 100.0,
                    message: format!("Đã tái thi hành quy trình cho: {}", filename),
                    processed_files: idx + 1,
                    failed_files: failed_items.len(),
                    total_files,
                    is_completed: false,
                });
            }
        }

        let status = if failed_items.is_empty() {
            "SUCCESS".to_string()
        } else if output_paths.is_empty() {
            "FAILED".to_string()
        } else {
            "PARTIAL_SUCCESS".to_string()
        };

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status,
            output_paths,
            failed_items,
            elapsed_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_recipe_history_manager_undo_redo() {
        let mut manager = RecipeHistoryManager::new();
        assert!(!manager.can_undo());
        assert!(!manager.can_redo());
        assert_eq!(manager.current_steps().len(), 0);

        let step1 = RecipeStep::new("step-1", RecipeAction::CleanSpaces, "Dọn dẹp khoảng trắng");
        manager.record_step(step1.clone());

        assert!(manager.can_undo());
        assert!(!manager.can_redo());
        assert_eq!(manager.current_steps().len(), 1);

        let step2 = RecipeStep::new("step-2", RecipeAction::Deduplicate, "Loại bỏ dòng trùng lặp");
        manager.record_step(step2.clone());

        assert_eq!(manager.current_steps().len(), 2);
        assert!(manager.can_undo());

        // Undo bước 2
        let undone = manager.undo();
        assert_eq!(undone, Some(step2.clone()));
        assert_eq!(manager.current_steps().len(), 1);
        assert!(manager.can_redo());

        // Redo bước 2
        let redone = manager.redo();
        assert_eq!(redone, Some(step2.clone()));
        assert_eq!(manager.current_steps().len(), 2);
        assert!(!manager.can_redo());

        // Undo cả hai bước
        assert_eq!(manager.undo(), Some(step2));
        assert_eq!(manager.undo(), Some(step1.clone()));
        assert_eq!(manager.undo(), None);
        assert!(!manager.can_undo());
        assert!(manager.can_redo());

        // Ghi bước mới phải xóa redo stack
        let step3 = RecipeStep::new("step-3", RecipeAction::NormalizeDates, "Chuẩn hóa ngày");
        manager.record_step(step3);
        assert!(!manager.can_redo());
        assert_eq!(manager.current_steps().len(), 1);
    }

    #[test]
    fn test_recipe_json_roundtrip() {
        let mut recipe = DataRecipe::new(
            "Quy trình chuẩn hóa nhân sự",
            "1.0.0",
            "Tự động chuẩn hóa tên, ngày tháng và loại bỏ dữ liệu rác",
        );

        recipe.add_step(RecipeStep::new(
            "s1",
            RecipeAction::CleanSpaces,
            "Làm sạch khoảng trắng",
        ));
        recipe.add_step(RecipeStep::new(
            "s2",
            RecipeAction::NormalizeDates,
            "Chuẩn hóa ngày về DD/MM/YYYY",
        ));
        recipe.add_step(RecipeStep::new(
            "s3",
            RecipeAction::SplitNames,
            "Tách Họ và Tên",
        ));
        recipe.add_step(RecipeStep::new(
            "s4",
            RecipeAction::Deduplicate,
            "Khử trùng lặp",
        ));

        let mut mappings = HashMap::new();
        mappings.insert("Hà Lội".to_string(), "Hà Nội".to_string());
        recipe.add_step(RecipeStep::new(
            "s5",
            RecipeAction::FuzzyMerge {
                col_idx: 1,
                mappings,
            },
            "Sửa lỗi chính tả tỉnh thành",
        ));

        recipe.add_step(RecipeStep::new(
            "s6",
            RecipeAction::DropColumn { col_idx: 2 },
            "Bỏ cột ghi chú",
        ));
        recipe.add_step(RecipeStep::new(
            "s7",
            RecipeAction::CustomTransform {
                description: "Chuyển thành chữ hoa (UPPERCASE)".to_string(),
            },
            "In hoa",
        ));

        let json_str = recipe.to_json().expect("Export JSON thành công");
        assert!(json_str.contains("Quy trình chuẩn hóa nhân sự"));

        let imported = DataRecipe::from_json(&json_str).expect("Import JSON thành công");
        assert_eq!(imported.name, recipe.name);
        assert_eq!(imported.version, recipe.version);
        assert_eq!(imported.steps.len(), recipe.steps.len());
        assert_eq!(imported, recipe);
    }

    #[test]
    fn test_apply_recipe_pipeline() {
        let cancel = AtomicBool::new(false);

        // Dữ liệu ban đầu: 3 dòng với khoảng trắng thừa và dòng 2 là bản sao của dòng 1
        let rows = vec![
            vec!["  nguyễn  thị  thu  trang  ".to_string(), "2026-11-02".to_string()],
            vec!["  nguyễn  thị  thu  trang  ".to_string(), "2026-11-02".to_string()],
            vec!["  trần  bá  lộc  ".to_string(), "2026-05-15".to_string()],
        ];

        let mut recipe = DataRecipe::new(
            "Pipeline Test",
            "1.0",
            "Chuỗi CleanSpaces -> SplitNames -> Deduplicate",
        );
        recipe.add_step(RecipeStep::new("1", RecipeAction::CleanSpaces, "Trim & Capitalize"));
        recipe.add_step(RecipeStep::new("2", RecipeAction::SplitNames, "Tách họ đệm tên"));
        recipe.add_step(RecipeStep::new("3", RecipeAction::Deduplicate, "Khử trùng"));

        let (result_rows, meta) =
            apply_recipe_with_result(&rows, &recipe, &cancel).expect("Áp dụng recipe thành công");

        assert_eq!(meta.total_steps, 3);
        assert_eq!(meta.rows_before, 3);
        assert_eq!(meta.rows_after, 2); // Đã lọc bỏ dòng trùng
        assert_eq!(result_rows.len(), 2);

        // Dòng 1: Nguyễn Thị Thu Trang
        assert_eq!(result_rows[0][0], "Nguyễn Thị Thu Trang");
        assert_eq!(result_rows[0][1], "2026-11-02");
        assert_eq!(result_rows[0][2], "Nguyễn");
        assert_eq!(result_rows[0][3], "Thị Thu");
        assert_eq!(result_rows[0][4], "Trang");

        // Dòng 2: Trần Bá Lộc (3 từ: Họ, Đệm, Tên)
        assert_eq!(result_rows[1][0], "Trần Bá Lộc");
        assert_eq!(result_rows[1][1], "2026-05-15");
        assert_eq!(result_rows[1][2], "Trần");
        assert_eq!(result_rows[1][3], "Bá");
        assert_eq!(result_rows[1][4], "Lộc");
    }

    #[test]
    fn test_apply_recipe_fuzzy_merge_and_drop_col() {
        let cancel = AtomicBool::new(false);
        let rows = vec![
            vec!["Hà Lội".to_string(), "100".to_string(), "Bỏ tôi".to_string()],
            vec!["Sài Gòn".to_string(), "200".to_string(), "Bỏ tôi".to_string()],
        ];

        let mut mappings = HashMap::new();
        mappings.insert("Hà Lội".to_string(), "Hà Nội".to_string());

        let mut recipe = DataRecipe::new("Fuzzy & Drop", "1.0", "FuzzyMerge & DropColumn");
        recipe.add_step(RecipeStep::new(
            "1",
            RecipeAction::FuzzyMerge {
                col_idx: 0,
                mappings,
            },
            "Sửa lỗi chính tả",
        ));
        recipe.add_step(RecipeStep::new(
            "2",
            RecipeAction::DropColumn { col_idx: 2 },
            "Xóa cột 2",
        ));

        let result = apply_recipe(&rows, &recipe, &cancel).expect("Áp dụng thành công");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec!["Hà Nội", "100"]);
        assert_eq!(result[1], vec!["Sài Gòn", "200"]);
    }

    #[test]
    fn test_apply_recipe_cancellation() {
        let cancel = AtomicBool::new(true); // Đã bật cờ hủy
        let rows = vec![vec!["Nguyễn Văn A".to_string()]];
        let mut recipe = DataRecipe::new("Cancel Test", "1.0", "Test hủy");
        recipe.add_step(RecipeStep::new("1", RecipeAction::CleanSpaces, "Clean"));

        let err = apply_recipe(&rows, &recipe, &cancel).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_recipe_replay_tool_happy_path_text() {
        let tool = RecipeReplayTool;
        assert_eq!(tool.slug(), "recipe-replay");

        let mut recipe = DataRecipe::new("Test Tool", "1.0", "Test Tool");
        recipe.add_step(RecipeStep::new("1", RecipeAction::CleanSpaces, "Trim"));
        let recipe_json = recipe.to_json().expect("to_json");

        let req = DispatchTaskRequest {
            task_id: "test-task-replay-1".to_string(),
            tool_slug: "recipe-replay".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "recipe_json": recipe_json,
                "text": "  nguyễn   văn   a  \n  trần   thị   b  "
            }),
        };

        let cancel = AtomicBool::new(false);
        let resp = tool.execute(&req, &cancel, None).expect("Thực thi thành công");
        assert_eq!(resp.status, "SUCCESS");
        assert_eq!(resp.output_paths.len(), 1);
        let csv_output = &resp.output_paths[0];
        assert!(csv_output.contains("Nguyễn Văn A"));
        assert!(csv_output.contains("Trần Thị B"));
    }

    #[test]
    fn test_recipe_replay_tool_invalid_magic_bytes() {
        let tool = RecipeReplayTool;
        let tmp = std::env::temp_dir().join("test_recipe_replay_invalid");
        let _ = fs::create_dir_all(&tmp);

        let elf_path = tmp.join("fake_binary.xlsx");
        fs::write(&elf_path, b"\x7fELF\x02\x01\x01\x00").expect("write elf");

        let mut recipe = DataRecipe::new("Test", "1.0", "");
        recipe.add_step(RecipeStep::new("1", RecipeAction::CleanSpaces, ""));

        let req = DispatchTaskRequest {
            task_id: "task-fake-recipe-excel".to_string(),
            tool_slug: "recipe-replay".to_string(),
            input_paths: vec![elf_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "recipe": recipe }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_recipe_replay_tool_cooperative_cancellation() {
        let tool = RecipeReplayTool;
        let mut recipe = DataRecipe::new("Cancel", "1.0", "");
        recipe.add_step(RecipeStep::new("1", RecipeAction::CleanSpaces, ""));

        let req = DispatchTaskRequest {
            task_id: "task-cancel-replay".to_string(),
            tool_slug: "recipe-replay".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({
                "recipe": recipe,
                "text": "Nguyễn Văn A"
            }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }
}
