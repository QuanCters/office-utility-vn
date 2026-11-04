pub mod base64_file_codec;
pub mod data_profiler;
pub mod data_structure_converter;
pub mod diff_checker_text_sheets;
pub mod encoding_mojibake_fixer;
pub mod excel_cleaner_normalizer;
pub mod excel_multi_merger;
pub mod excel_reader;
pub mod excel_sheet_unlocker;
pub mod excel_split_by_column;
pub mod excel_visual_joiner;
pub mod fuzzy_clustering;
pub mod list_set_operations;
pub mod recipe_history;
pub mod sql_formatter_linter;
pub mod text_pii_anonymizer;
pub mod vietnamese_font_converter;

pub use base64_file_codec::{
    stream_decode_base64, stream_encode_to_base64, Base64CodecOptions, Base64FileCodecTool,
    CodecMode,
};
pub use data_profiler::{
    infer_column_type, parse_date_flexible, profile_tabular_rows, ColumnProfile, ColumnType,
    DataProfileReport, DataProfilerOptions, DataProfilerTool, ValueFrequency,
};

pub use data_structure_converter::{
    DataConverterOptions, DataStructureConverterTool,
};
pub use diff_checker_text_sheets::{
    compute_diff, DataDiffOptions, DiffCheckerTextSheetsTool, DiffItem, DiffKind, DiffReport,
};
pub use encoding_mojibake_fixer::{
    fix_mojibake_text, EncodingMojibakeFixerTool, MojibakeFixOptions,
};
pub use excel_cleaner_normalizer::{
    split_vietnamese_name, ExcelCleanOptions, ExcelCleanerNormalizerTool, VietnameseNameParts,
};
pub use excel_multi_merger::{
    merge_append_rows, ExcelMergeOptions, ExcelMultiMergerTool, MergeMode,
};
pub use excel_reader::{format_table_to_csv, read_tabular_rows};
pub use excel_sheet_unlocker::{
    strip_protection_tags, unlock_excel_bytes, ExcelSheetUnlockerTool, ExcelUnlockOptions,
};
pub use excel_split_by_column::{
    split_table_by_column, ExcelSplitByColumnTool, ExcelSplitOptions,
};
pub use excel_visual_joiner::{
    perform_hash_join, ExcelJoinOptions, ExcelVisualJoinerTool, JoinType,
};
pub use fuzzy_clustering::{
    apply_cluster_merges, cluster_column_values, compute_fingerprint, levenshtein_distance,
    remove_vietnamese_diacritics, ClusterGroup, ClusteringMethod, FuzzyClusterOptions,
    FuzzyClusterReport, FuzzyClusteringTool,
};
pub use list_set_operations::{
    compute_set_operation, ListSetOperationsTool, ListSetOptions, SetOperation,
};
pub use recipe_history::{
    apply_recipe, apply_recipe_with_result, export_recipe_to_json, import_recipe_from_json,
    DataRecipe, RecipeAction, RecipeExecutionResult, RecipeHistoryManager, RecipeReplayTool,
    RecipeStep,
};
pub use sql_formatter_linter::{
    format_sql_query, lint_sql_syntax, SqlDialect, SqlFormatterLinterTool, SqlFormatterOptions,
    SqlLintResult,
};
pub use text_pii_anonymizer::{
    anonymize_pii_text, PiiAnonymizerOptions, TextPiiAnonymizerTool,
};
pub use vietnamese_font_converter::{FontConvertOptions, VietnameseFontConverterTool};
