use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Phương pháp gom cụm dữ liệu mờ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClusteringMethod {
    /// Chuẩn hóa chữ thường, bỏ dấu tiếng Việt, tokenize và sắp xếp từ (OpenRefine Key Collision)
    #[default]
    Fingerprint,
    /// Khoảng cách chỉnh sửa ký tự Levenshtein
    Levenshtein,
}

/// Một cụm các giá trị tương đồng phát hiện được
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterGroup {
    /// Giá trị đại diện chuẩn (biến thể xuất hiện nhiều nhất)
    pub canonical: String,
    /// Danh sách các biến thể trong cụm kèm số lần xuất hiện: (biến thể, số lần)
    pub variants: Vec<(String, usize)>,
    /// Tổng số lần xuất hiện của tất cả biến thể trong cụm
    pub total_occurrences: usize,
}

/// Cấu hình tùy chọn gom cụm dữ liệu mờ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzyClusterOptions {
    #[serde(default)]
    pub method: ClusteringMethod,
    #[serde(default)]
    pub column_index: usize,
    #[serde(default = "default_distance_threshold")]
    pub distance_threshold: usize,
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
    #[serde(default)]
    pub apply_merge: bool,
    #[serde(default)]
    pub merge_mappings: HashMap<String, String>,
}

fn default_distance_threshold() -> usize {
    2
}

fn default_similarity_threshold() -> f32 {
    0.8
}

impl Default for FuzzyClusterOptions {
    fn default() -> Self {
        Self {
            method: ClusteringMethod::default(),
            column_index: 0,
            distance_threshold: default_distance_threshold(),
            similarity_threshold: default_similarity_threshold(),
            apply_merge: false,
            merge_mappings: HashMap::new(),
        }
    }
}

/// Báo cáo kết quả gom cụm dữ liệu mờ
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FuzzyClusterReport {
    pub column_name: String,
    pub total_clusters: usize,
    pub clusters: Vec<ClusterGroup>,
}

pub struct FuzzyClusteringTool;

impl Default for FuzzyClusteringTool {
    fn default() -> Self {
        Self
    }
}

/// Loại bỏ dấu tiếng Việt, đưa về ký tự ASCII không dấu chuẩn (hỗ trợ cả NFC & NFD combining marks)
pub fn remove_vietnamese_diacritics(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        // Bỏ qua các combining diacritical marks (NFD)
        if ('\u{0300}'..='\u{036F}').contains(&c) {
            continue;
        }

        let mapped = match c {
            'à' | 'á' | 'ả' | 'ã' | 'ạ' | 'ă' | 'ằ' | 'ắ' | 'ẳ' | 'ẵ' | 'ặ' | 'â' | 'ầ' | 'ấ' | 'ẩ' | 'ẫ' | 'ậ' => 'a',
            'À' | 'Á' | 'Ả' | 'Ã' | 'Ạ' | 'Ă' | 'Ằ' | 'Ắ' | 'Ẳ' | 'Ẵ' | 'Ặ' | 'Â' | 'Ầ' | 'Ấ' | 'Ẩ' | 'Ẫ' | 'Ậ' => 'A',
            'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' => 'e',
            'È' | 'É' | 'Ẻ' | 'Ẽ' | 'Ẹ' | 'Ê' | 'Ề' | 'Ế' | 'Ể' | 'Ễ' | 'Ệ' => 'E',
            'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' => 'i',
            'Ì' | 'Í' | 'Ỉ' | 'Ĩ' | 'Ị' => 'I',
            'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ờ' | 'ớ' | 'ở' | 'ỡ' | 'ợ' => 'o',
            'Ò' | 'Ó' | 'Ỏ' | 'Õ' | 'Ọ' | 'Ô' | 'Ồ' | 'Ố' | 'Ổ' | 'Ỗ' | 'Ộ' | 'Ơ' | 'Ờ' | 'Ớ' | 'Ở' | 'Ỡ' | 'Ợ' => 'O',
            'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' => 'u',
            'Ù' | 'Ú' | 'Ủ' | 'Ũ' | 'Ụ' | 'Ư' | 'Ừ' | 'Ứ' | 'Ử' | 'Ữ' | 'Ự' => 'U',
            'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' => 'y',
            'Ỳ' | 'Ý' | 'Ỷ' | 'Ỹ' | 'Ỵ' => 'Y',
            'đ' => 'd',
            'Đ' => 'D',
            other => other,
        };
        result.push(mapped);
    }
    result
}

/// Tạo khóa Fingerprint chuẩn hóa theo OpenRefine:
/// Chuyển chữ thường, bỏ dấu tiếng Việt, tokenize theo ký tự phi chữ số,
/// sắp xếp các token và loại bỏ trùng lặp, sau đó ghép lại.
/// Giúp "Hà Nội", "Hanoi", "Ha Noi" gom lại thành cùng một cụm ("hanoi").
pub fn compute_fingerprint(text: &str) -> String {
    let lower = text.to_lowercase();
    let unaccented = remove_vietnamese_diacritics(&lower);

    let mut tokens: Vec<&str> = unaccented
        .split(|c: char| !c.is_alphanumeric())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if tokens.is_empty() {
        return String::new();
    }

    tokens.sort_unstable();
    tokens.dedup();

    tokens.join("")
}

/// Tính khoảng cách sửa đổi Levenshtein giữa 2 chuỗi theo ký tự Unicode (char)
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let chars1: Vec<char> = s1.chars().collect();
    let chars2: Vec<char> = s2.chars().collect();
    let len1 = chars1.len();
    let len2 = chars2.len();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut prev_row: Vec<usize> = (0..=len2).collect();
    let mut curr_row: Vec<usize> = vec![0; len2 + 1];

    for i in 0..len1 {
        curr_row[0] = i + 1;
        for j in 0..len2 {
            let cost = if chars1[i] == chars2[j] { 0 } else { 1 };
            curr_row[j + 1] = (prev_row[j + 1] + 1)
                .min(curr_row[j] + 1)
                .min(prev_row[j] + cost);
        }
        prev_row.copy_from_slice(&curr_row);
    }

    prev_row[len2]
}

/// Cấu trúc Disjoint Set Union (DSU) phục vụ gom nhóm Connected Components
struct DisjointSet {
    parent: Vec<usize>,
}

impl DisjointSet {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
        }
    }

    fn find(&mut self, i: usize) -> usize {
        let mut root = i;
        while root != self.parent[root] {
            root = self.parent[root];
        }
        let mut curr = i;
        while curr != root {
            let nxt = self.parent[curr];
            self.parent[curr] = root;
            curr = nxt;
        }
        root
    }

    fn union(&mut self, i: usize, j: usize) {
        let root_i = self.find(i);
        let root_j = self.find(j);
        if root_i != root_j {
            self.parent[root_i] = root_j;
        }
    }
}

/// Gom cụm các giá trị trong một cột theo phương pháp chỉ định (Fingerprint hoặc Levenshtein)
pub fn cluster_column_values(
    values: &[String],
    method: ClusteringMethod,
    distance_threshold: usize,
    cancel_flag: &AtomicBool,
) -> Result<Vec<ClusterGroup>, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    // Đếm tần suất xuất hiện của từng giá trị duy nhất (bỏ qua giá trị rỗng)
    let mut freq_map: BTreeMap<String, usize> = BTreeMap::new();
    for val in values {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            *freq_map.entry(trimmed.to_string()).or_insert(0) += 1;
        }
    }

    if freq_map.len() <= 1 {
        return Ok(Vec::new());
    }

    let mut clusters = Vec::new();

    match method {
        ClusteringMethod::Fingerprint => {
            let mut groups: BTreeMap<String, Vec<(String, usize)>> = BTreeMap::new();
            for (val, count) in freq_map {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }
                let fp = compute_fingerprint(&val);
                if !fp.is_empty() {
                    groups.entry(fp).or_default().push((val, count));
                }
            }

            for (_fp, mut variants) in groups {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                if variants.len() >= 2 {
                    variants.sort_by(|a, b| {
                        b.1.cmp(&a.1)
                            .then_with(|| b.0.len().cmp(&a.0.len()))
                            .then_with(|| a.0.cmp(&b.0))
                    });

                    let canonical = variants[0].0.clone();
                    let total_occurrences = variants.iter().map(|(_, c)| *c).sum();

                    clusters.push(ClusterGroup {
                        canonical,
                        variants,
                        total_occurrences,
                    });
                }
            }
        }
        ClusteringMethod::Levenshtein => {
            let mut items: Vec<(String, usize)> = freq_map.into_iter().collect();
            items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

            let n = items.len();
            let mut dsu = DisjointSet::new(n);

            let char_lens: Vec<usize> = items.iter().map(|(s, _)| s.chars().count()).collect();

            for i in 0..n {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                for j in (i + 1)..n {
                    if char_lens[i].abs_diff(char_lens[j]) > distance_threshold {
                        continue;
                    }

                    let dist = levenshtein_distance(&items[i].0, &items[j].0);
                    if dist <= distance_threshold {
                        dsu.union(i, j);
                    }
                }
            }

            let mut groups: BTreeMap<usize, Vec<(String, usize)>> = BTreeMap::new();
            for i in 0..n {
                let root = dsu.find(i);
                groups.entry(root).or_default().push(items[i].clone());
            }

            for (_root, mut variants) in groups {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Err(CommandError::TaskCancelled);
                }

                if variants.len() >= 2 {
                    variants.sort_by(|a, b| {
                        b.1.cmp(&a.1)
                            .then_with(|| b.0.len().cmp(&a.0.len()))
                            .then_with(|| a.0.cmp(&b.0))
                    });

                    let canonical = variants[0].0.clone();
                    let total_occurrences = variants.iter().map(|(_, c)| *c).sum();

                    clusters.push(ClusterGroup {
                        canonical,
                        variants,
                        total_occurrences,
                    });
                }
            }
        }
    }

    clusters.sort_by(|a, b| b.total_occurrences.cmp(&a.total_occurrences));

    Ok(clusters)
}

/// Áp dụng bảng ánh xạ gộp cụm vào các dòng dữ liệu bảng tính
pub fn apply_cluster_merges(
    rows: &[Vec<String>],
    col_idx: usize,
    mappings: &HashMap<String, String>,
    cancel_flag: &AtomicBool,
) -> Result<Vec<Vec<String>>, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    let mut result_rows = Vec::with_capacity(rows.len());

    for (idx, row) in rows.iter().enumerate() {
        if idx % 256 == 0 && cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let mut new_row = row.clone();
        if col_idx < new_row.len() {
            let cell = &new_row[col_idx];
            if let Some(target) = mappings.get(cell) {
                new_row[col_idx] = target.clone();
            } else if let Some(target) = mappings.get(cell.trim()) {
                new_row[col_idx] = target.clone();
            }
        }
        result_rows.push(new_row);
    }

    Ok(result_rows)
}

impl OfficeTool for FuzzyClusteringTool {
    fn slug(&self) -> &'static str {
        "fuzzy-clustering"
    }

    fn name(&self) -> &'static str {
        "Gom cụm Dữ liệu Mờ (Fuzzy Clustering)"
    }

    fn description(&self) -> &'static str {
        "Tự động phát hiện và gộp các giá trị gõ sai chính tả, biến thể viết tắt hoặc khác dấu tiếng Việt tương tự tính năng Cluster & Edit của OpenRefine."
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

        let options: FuzzyClusterOptions = serde_json::from_value(request.options.clone())
            .unwrap_or_default();

        // Xử lý trực tiếp từ chuỗi nếu không có input_paths
        if request.input_paths.is_empty() {
            if let Some(text_val) = request.options.get("text").and_then(|v| v.as_str()) {
                let rows = super::excel_reader::read_tabular_rows(text_val.as_bytes(), cancel_flag)?;
                if rows.is_empty() {
                    return Ok(DispatchTaskResponse {
                        task_id: request.task_id.clone(),
                        status: "SUCCESS".to_string(),
                        output_paths: vec![],
                        failed_items: vec![],
                        elapsed_ms: start_time.elapsed().as_millis() as u64,
                    });
                }

                let col_idx = options.column_index;
                let column_name = rows[0].get(col_idx).cloned().unwrap_or_else(|| format!("Cột {}", col_idx + 1));
                let data_rows = if rows.len() > 1 { &rows[1..] } else { &rows[..] };
                let values: Vec<String> = data_rows
                    .iter()
                    .filter_map(|r| r.get(col_idx).cloned())
                    .collect();

                let clusters = cluster_column_values(&values, options.method, options.distance_threshold, cancel_flag)?;

                if options.apply_merge {
                    let merged_rows = apply_cluster_merges(&rows, col_idx, &options.merge_mappings, cancel_flag)?;
                    let csv_out = super::excel_reader::format_table_to_csv(&merged_rows);
                    return Ok(DispatchTaskResponse {
                        task_id: request.task_id.clone(),
                        status: "SUCCESS".to_string(),
                        output_paths: vec![csv_out.trim_end_matches('\n').to_string()],
                        failed_items: vec![],
                        elapsed_ms: start_time.elapsed().as_millis() as u64,
                    });
                } else {
                    let report = FuzzyClusterReport {
                        column_name,
                        total_clusters: clusters.len(),
                        clusters,
                    };
                    let json_out = serde_json::to_string_pretty(&report).map_err(|e| {
                        CommandError::ProcessExecutionError {
                            message: format!("Không thể serialize báo cáo gom cụm: {e}"),
                        }
                    })?;
                    return Ok(DispatchTaskResponse {
                        task_id: request.task_id.clone(),
                        status: "SUCCESS".to_string(),
                        output_paths: vec![json_out],
                        failed_items: vec![],
                        elapsed_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
            } else if let Some(vals_arr) = request.options.get("values").and_then(|v| v.as_array()) {
                let values: Vec<String> = vals_arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();

                let clusters = cluster_column_values(&values, options.method, options.distance_threshold, cancel_flag)?;
                let report = FuzzyClusterReport {
                    column_name: format!("Cột {}", options.column_index + 1),
                    total_clusters: clusters.len(),
                    clusters,
                };
                let json_out = serde_json::to_string_pretty(&report).map_err(|e| {
                    CommandError::ProcessExecutionError {
                        message: format!("Không thể serialize báo cáo: {e}"),
                    }
                })?;
                return Ok(DispatchTaskResponse {
                    task_id: request.task_id.clone(),
                    status: "SUCCESS".to_string(),
                    output_paths: vec![json_out],
                    failed_items: vec![],
                    elapsed_ms: start_time.elapsed().as_millis() as u64,
                });
            } else {
                return Err(CommandError::ValidationFailed {
                    reason: "Không có tệp đầu vào hoặc dữ liệu cần phân tích gom cụm".to_string(),
                });
            }
        }

        let total_files = request.input_paths.len();
        let mut output_paths = Vec::new();
        let mut failed_items = Vec::new();

        let out_dir = match &request.output_directory {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir().join(format!("fuzzy_cluster_{}", request.task_id)),
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

            // Kiểm tra magic bytes phòng vệ nhị phân: cấm ELF, MZ thực thi
            if raw_bytes.starts_with(b"\x7fELF") || raw_bytes.starts_with(b"MZ") {
                return Err(CommandError::InvalidFileSignature {
                    expected: "XLSX hoặc CSV".to_string(),
                    actual: "Tệp thực thi nhị phân bị chặn".to_string(),
                });
            }

            let rows = match super::excel_reader::read_tabular_rows(&raw_bytes, cancel_flag) {
                Ok(r) => r,
                Err(e) => {
                    failed_items.push(FileFailureDetail {
                        input_path: in_path_str.clone(),
                        reason: format!("Lỗi đọc bảng tính: {e:?}"),
                    });
                    continue;
                }
            };

            if rows.is_empty() {
                continue;
            }

            let col_idx = options.column_index;
            let column_name = rows[0].get(col_idx).cloned().unwrap_or_else(|| format!("Cột {}", col_idx + 1));
            let data_rows = if rows.len() > 1 { &rows[1..] } else { &rows[..] };
            let values: Vec<String> = data_rows
                .iter()
                .filter_map(|r| r.get(col_idx).cloned())
                .collect();

            let clusters = match cluster_column_values(&values, options.method, options.distance_threshold, cancel_flag) {
                Ok(c) => c,
                Err(e) => {
                    failed_items.push(FileFailureDetail {
                        input_path: in_path_str.clone(),
                        reason: format!("Lỗi gom cụm: {e:?}"),
                    });
                    continue;
                }
            };

            let filename = in_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("file_{}.csv", idx));

            if options.apply_merge {
                let merged_rows = match apply_cluster_merges(&rows, col_idx, &options.merge_mappings, cancel_flag) {
                    Ok(m) => m,
                    Err(e) => {
                        failed_items.push(FileFailureDetail {
                            input_path: in_path_str.clone(),
                            reason: format!("Lỗi áp dụng gộp cụm: {e:?}"),
                        });
                        continue;
                    }
                };
                let csv_out = super::excel_reader::format_table_to_csv(&merged_rows);
                let out_file = out_dir.join(format!("clustered_{}", filename));
                if let Err(e) = fs::write(&out_file, csv_out) {
                    failed_items.push(FileFailureDetail {
                        input_path: in_path_str.clone(),
                        reason: format!("Không thể ghi tệp đầu ra {}: {e}", out_file.display()),
                    });
                    continue;
                }
                output_paths.push(out_file.to_string_lossy().to_string());
            } else {
                let report = FuzzyClusterReport {
                    column_name,
                    total_clusters: clusters.len(),
                    clusters,
                };
                let json_out = match serde_json::to_string_pretty(&report) {
                    Ok(j) => j,
                    Err(e) => {
                        failed_items.push(FileFailureDetail {
                            input_path: in_path_str.clone(),
                            reason: format!("Lỗi serialize báo cáo: {e}"),
                        });
                        continue;
                    }
                };
                let out_file = out_dir.join(format!("cluster_report_{}.json", filename));
                if let Err(e) = fs::write(&out_file, json_out) {
                    failed_items.push(FileFailureDetail {
                        input_path: in_path_str.clone(),
                        reason: format!("Không thể ghi tệp báo cáo {}: {e}", out_file.display()),
                    });
                    continue;
                }
                output_paths.push(out_file.to_string_lossy().to_string());
            }

            if let Some(cb) = on_progress {
                cb(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: (idx + 1) as u32,
                    total_steps: total_files as u32,
                    percentage: ((idx + 1) as f32 / total_files as f32) * 100.0,
                    message: format!("Đã phân tích {}/{} tệp", idx + 1, total_files),
                    processed_files: idx + 1,
                    failed_files: failed_items.len(),
                    total_files,
                    is_completed: idx + 1 == total_files,
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

    #[test]
    fn test_remove_vietnamese_diacritics() {
        assert_eq!(remove_vietnamese_diacritics("Hà Nội"), "Ha Noi");
        assert_eq!(remove_vietnamese_diacritics("Đà Nẵng"), "Da Nang");
        assert_eq!(remove_vietnamese_diacritics("TP. Hồ Chí Minh"), "TP. Ho Chi Minh");
        assert_eq!(remove_vietnamese_diacritics("Công ty TNHH"), "Cong ty TNHH");
    }

    #[test]
    fn test_compute_fingerprint() {
        let fp1 = compute_fingerprint("Hà Nội");
        let fp2 = compute_fingerprint("Ha Noi");
        let fp3 = compute_fingerprint("hanoi");
        let fp4 = compute_fingerprint("Nội, Hà");

        assert_eq!(fp1, "hanoi");
        assert_eq!(fp2, "hanoi");
        assert_eq!(fp3, "hanoi");
        assert_eq!(fp4, "hanoi");
    }

    #[test]
    fn test_cluster_vietnamese_fingerprint() {
        let cancel = AtomicBool::new(false);
        let values = vec![
            "Hà Nội".to_string(),
            "Ha Noi".to_string(),
            "hanoi".to_string(),
            "Hà Nội".to_string(),
            "Đà Nẵng".to_string(),
            "Da Nang".to_string(),
            "Sài Gòn".to_string(),
        ];

        let clusters = cluster_column_values(&values, ClusteringMethod::Fingerprint, 2, &cancel)
            .expect("clustering should succeed");

        assert_eq!(clusters.len(), 2);

        let hn_cluster = clusters.iter().find(|c| c.canonical == "Hà Nội" || c.canonical == "Ha Noi" || c.canonical == "hanoi").expect("must find Hanoi cluster");
        assert_eq!(hn_cluster.total_occurrences, 4);
        assert_eq!(hn_cluster.variants.len(), 3);
        assert_eq!(hn_cluster.canonical, "Hà Nội");

        let dn_cluster = clusters.iter().find(|c| c.canonical == "Đà Nẵng" || c.canonical == "Da Nang").expect("must find Da Nang cluster");
        assert_eq!(dn_cluster.total_occurrences, 2);
        assert_eq!(dn_cluster.variants.len(), 2);
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("Công ty TNHH", "Cộng ty TNHH"), 1);
        assert_eq!(levenshtein_distance("Microsoft", "Microsft"), 1);
        assert_eq!(levenshtein_distance("Apple", "Appel"), 2);
        assert_eq!(levenshtein_distance("", "test"), 4);
        assert_eq!(levenshtein_distance("test", "test"), 0);
    }

    #[test]
    fn test_cluster_levenshtein() {
        let cancel = AtomicBool::new(false);
        let values = vec![
            "Công ty TNHH".to_string(),
            "Cộng ty TNHH".to_string(),
            "Công ty TNHH".to_string(),
            "Doanh nghiệp Tư nhân".to_string(),
        ];

        let clusters = cluster_column_values(&values, ClusteringMethod::Levenshtein, 1, &cancel)
            .expect("clustering should succeed");

        assert_eq!(clusters.len(), 1);
        let cluster = &clusters[0];
        assert_eq!(cluster.canonical, "Công ty TNHH");
        assert_eq!(cluster.total_occurrences, 3);
        assert_eq!(cluster.variants.len(), 2);
    }

    #[test]
    fn test_apply_cluster_merges() {
        let cancel = AtomicBool::new(false);
        let rows = vec![
            vec!["Tên công ty".to_string(), "Địa chỉ".to_string()],
            vec!["Cộng ty TNHH".to_string(), "Ha Noi".to_string()],
            vec!["Công ty TNHH".to_string(), "Hà Nội".to_string()],
            vec!["Doanh nghiệp ABC".to_string(), "hanoi".to_string()],
        ];

        let mut mappings = HashMap::new();
        mappings.insert("Cộng ty TNHH".to_string(), "Công ty TNHH".to_string());

        let merged = apply_cluster_merges(&rows, 0, &mappings, &cancel).expect("merge should succeed");
        assert_eq!(merged[1][0], "Công ty TNHH");
        assert_eq!(merged[2][0], "Công ty TNHH");
        assert_eq!(merged[3][0], "Doanh nghiệp ABC");

        let mut addr_mappings = HashMap::new();
        addr_mappings.insert("Ha Noi".to_string(), "Hà Nội".to_string());
        addr_mappings.insert("hanoi".to_string(), "Hà Nội".to_string());

        let merged_addr = apply_cluster_merges(&rows, 1, &addr_mappings, &cancel).expect("merge addr");
        assert_eq!(merged_addr[1][1], "Hà Nội");
        assert_eq!(merged_addr[2][1], "Hà Nội");
        assert_eq!(merged_addr[3][1], "Hà Nội");
    }

    #[test]
    fn test_cooperative_cancellation() {
        let cancel = AtomicBool::new(true);
        let values = vec!["Hà Nội".to_string(), "Ha Noi".to_string()];
        let res = cluster_column_values(&values, ClusteringMethod::Fingerprint, 2, &cancel);
        assert_eq!(res, Err(CommandError::TaskCancelled));

        let rows = vec![vec!["A".to_string()]];
        let mappings = HashMap::new();
        let res_merge = apply_cluster_merges(&rows, 0, &mappings, &cancel);
        assert_eq!(res_merge, Err(CommandError::TaskCancelled));
    }

    #[test]
    fn test_office_tool_execute_text_report() {
        let tool = FuzzyClusteringTool;
        let cancel = AtomicBool::new(false);

        let csv_content = "Name,City\nNguyen Van A,Hà Nội\nTran Thi B,Ha Noi\nLe Van C,hanoi\nPham Van D,Da Nang\n";
        let request = DispatchTaskRequest {
            task_id: "test-task-1".to_string(),
            tool_slug: "fuzzy-clustering".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: serde_json::json!({
                "text": csv_content,
                "column_index": 1,
                "method": "fingerprint",
                "apply_merge": false
            }),
        };

        let response = tool.execute(&request, &cancel, None).expect("execute ok");
        assert_eq!(response.status, "SUCCESS");
        assert_eq!(response.output_paths.len(), 1);

        let report_str = &response.output_paths[0];
        let report: FuzzyClusterReport = serde_json::from_str(report_str).expect("parse report json");
        assert_eq!(report.column_name, "City");
        assert_eq!(report.total_clusters, 1);
        assert_eq!(report.clusters[0].canonical, "Hà Nội");
        assert_eq!(report.clusters[0].variants.len(), 3);
    }

    #[test]
    fn test_office_tool_execute_text_merge() {
        let tool = FuzzyClusteringTool;
        let cancel = AtomicBool::new(false);

        let csv_content = "Name,City\nNguyen Van A,Hà Nội\nTran Thi B,Ha Noi\nLe Van C,hanoi\n";
        let request = DispatchTaskRequest {
            task_id: "test-task-2".to_string(),
            tool_slug: "fuzzy-clustering".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: serde_json::json!({
                "text": csv_content,
                "column_index": 1,
                "method": "fingerprint",
                "apply_merge": true,
                "merge_mappings": {
                    "Ha Noi": "Hà Nội",
                    "hanoi": "Hà Nội"
                }
            }),
        };

        let response = tool.execute(&request, &cancel, None).expect("execute merge ok");
        assert_eq!(response.status, "SUCCESS");
        assert_eq!(response.output_paths.len(), 1);

        let merged_csv = &response.output_paths[0];
        assert!(merged_csv.contains("Tran Thi B,Hà Nội"));
        assert!(merged_csv.contains("Le Van C,Hà Nội"));
    }

    #[test]
    fn test_security_malicious_binary_blocked() {
        let tool = FuzzyClusteringTool;
        let cancel = AtomicBool::new(false);

        let temp_file = std::env::temp_dir().join("malicious_test.xlsx");
        let _ = fs::write(&temp_file, b"\x7fELFfakecontent");

        let request = DispatchTaskRequest {
            task_id: "test-sec".to_string(),
            tool_slug: "fuzzy-clustering".to_string(),
            input_paths: vec![temp_file.to_string_lossy().to_string()],
            output_directory: None,
            options: serde_json::json!({}),
        };

        let res = tool.execute(&request, &cancel, None);
        let _ = fs::remove_file(&temp_file);

        match res {
            Err(CommandError::InvalidFileSignature { expected, actual }) => {
                assert_eq!(expected, "XLSX hoặc CSV");
                assert!(actual.contains("bị chặn"));
            }
            other => panic!("Mong đợi InvalidFileSignature nhưng nhận được: {other:?}"),
        }
    }
}
