pub mod data_structure_converter;
pub mod diff_checker_text_sheets;
pub mod excel_cleaner_normalizer;
pub mod excel_multi_merger;
pub mod excel_sheet_unlocker;
pub mod excel_split_by_column;
pub mod excel_visual_joiner;
pub mod list_set_operations;
pub mod vietnamese_font_converter;

pub use data_structure_converter::{
    DataConverterOptions, DataStructureConverterTool,
};
pub use diff_checker_text_sheets::{
    compute_diff, DataDiffOptions, DiffCheckerTextSheetsTool, DiffItem, DiffKind, DiffReport,
};
pub use excel_cleaner_normalizer::{
    split_vietnamese_name, ExcelCleanOptions, ExcelCleanerNormalizerTool, VietnameseNameParts,
};
pub use excel_multi_merger::{
    merge_append_rows, ExcelMergeOptions, ExcelMultiMergerTool, MergeMode,
};
pub use excel_sheet_unlocker::{
    strip_protection_tags, unlock_excel_bytes, ExcelSheetUnlockerTool, ExcelUnlockOptions,
};
pub use excel_split_by_column::{
    split_table_by_column, ExcelSplitByColumnTool, ExcelSplitOptions,
};
pub use excel_visual_joiner::{
    perform_hash_join, ExcelJoinOptions, ExcelVisualJoinerTool, JoinType,
};
pub use list_set_operations::{
    compute_set_operation, ListSetOperationsTool, ListSetOptions, SetOperation,
};
pub use vietnamese_font_converter::{FontConvertOptions, VietnameseFontConverterTool};
