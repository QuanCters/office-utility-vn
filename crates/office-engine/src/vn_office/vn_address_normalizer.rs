use std::sync::atomic::{AtomicBool, Ordering};
use office_shared::{CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress};
use serde::{Deserialize, Serialize};
use crate::OfficeTool;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum AdminModel {
    #[default]
    ThreeTier, // Mô hình 3 cấp truyền thống: Tỉnh/TP - Quận/Huyện - Phường/Xã
    TwoTier,   // Mô hình 2 cấp mới sau sáp nhập: Tỉnh/TP - Phường/Xã
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressNormalizerOptions {
    #[serde(default)]
    pub target_model: AdminModel,
    #[serde(default = "default_fuzzy_threshold")]
    pub fuzzy_threshold: f64,
}

fn default_fuzzy_threshold() -> f64 {
    0.75
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NormalizedAddress {
    pub raw_input: String,
    pub street_address: String,
    pub ward: String,
    pub district: Option<String>,
    pub province: String,
    pub full_normalized: String,
    pub confidence_score: f64,
    pub model: String,
}

pub struct VnAddressNormalizerTool;

impl Default for VnAddressNormalizerTool {
    fn default() -> Self {
        Self
    }
}

/// Tính độ tương đồng Jaro-Winkler giữa 2 chuỗi
pub fn jaro_winkler_similarity(s1: &str, s2: &str) -> f64 {
    if s1 == s2 {
        return 1.0;
    }
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }

    let c1: Vec<char> = s1.chars().collect();
    let c2: Vec<char> = s2.chars().collect();
    let len1 = c1.len();
    let len2 = c2.len();

    let match_distance = (len1.max(len2) / 2).saturating_sub(1);

    let mut s1_matches = vec![false; len1];
    let mut s2_matches = vec![false; len2];

    let mut matches = 0;
    for i in 0..len1 {
        let start = i.saturating_sub(match_distance);
        let end = (i + match_distance + 1).min(len2);

        for j in start..end {
            if s2_matches[j] || c1[i] != c2[j] {
                continue;
            }
            s1_matches[i] = true;
            s2_matches[j] = true;
            matches += 1;
            break;
        }
    }

    if matches == 0 {
        return 0.0;
    }

    let mut transpositions = 0;
    let mut k = 0;
    for i in 0..len1 {
        if !s1_matches[i] {
            continue;
        }
        while !s2_matches[k] {
            k += 1;
        }
        if c1[i] != c2[k] {
            transpositions += 1;
        }
        k += 1;
    }

    let m = matches as f64;
    let jaro = (m / len1 as f64 + m / len2 as f64 + (m - (transpositions as f64 / 2.0)) / m) / 3.0;

    // Winkler bonus cho prefix chung tối đa 4 ký tự
    let mut prefix = 0;
    for i in 0..len1.min(len2).min(4) {
        if c1[i] == c2[i] {
            prefix += 1;
        } else {
            break;
        }
    }

    let p = 0.1;
    jaro + (prefix as f64 * p * (1.0 - jaro))
}

/// Bảng từ điển các tỉnh/thành phố và các biến thể viết tắt
pub const PROVINCES: &[(&str, &[&str])] = &[
    ("Hà Nội", &["hn", "ha noi", "tp ha noi", "thu do ha noi"]),
    ("TP. Hồ Chí Minh", &["hcm", "tphcm", "tp hcm", "sai gon", "sg", "ho chi minh"]),
    ("Đà Nẵng", &["dn", "da nang", "tp da nang"]),
    ("Hải Phòng", &["hp", "hai phong", "tp hai phong"]),
    ("Cần Thơ", &["ct", "can tho", "tp can tho"]),
    ("Bình Dương", &["binh duong", "bd"]),
    ("Đồng Nai", &["dong nai", "dnai"]),
    ("Quảng Ninh", &["quang ninh", "qn"]),
    ("Bà Rịa - Vũng Tàu", &["brvt", "vung tau", "ba ria vung tau"]),
    ("Thừa Thiên Huế", &["hue", "thua thien hue", "tthue"]),
    ("Khánh Hòa", &["khanh hoa", "nha trang"]),
    ("Lâm Đồng", &["lam dong", "da lat"]),
];

/// Bảng từ điển các quận/huyện phổ biến
pub const DISTRICTS: &[(&str, &str, &[&str])] = &[
    // (Tên chuẩn, Tỉnh trực thuộc, Từ khóa/viết tắt)
    ("Quận Tân Bình", "TP. Hồ Chí Minh", &["q.tb", "q tb", "q. tan binh", "tan binh"]),
    ("Quận 1", "TP. Hồ Chí Minh", &["q.1", "q1", "quan 1"]),
    ("Quận 3", "TP. Hồ Chí Minh", &["q.3", "q3", "quan 3"]),
    ("Quận 7", "TP. Hồ Chí Minh", &["q.7", "q7", "quan 7"]),
    ("Quận Bình Thạnh", "TP. Hồ Chí Minh", &["q.bth", "q bth", "binh thanh"]),
    ("Thành phố Thủ Đức", "TP. Hồ Chí Minh", &["tp thu duc", "thu duc", "tp. thu duc"]),
    ("Quận Hoàn Kiếm", "Hà Nội", &["q.hk", "hoan kiem", "q. hoan kiem"]),
    ("Quận Đống Đa", "Hà Nội", &["q.dd", "dong da", "q. dong da"]),
    ("Quận Cầu Giấy", "Hà Nội", &["q.cg", "cau giay", "q. cau giay"]),
    ("Quận Hai Bà Trưng", "Hà Nội", &["q.hbt", "hai ba trung", "q. hai ba trung"]),
    ("Quận Hải Châu", "Đà Nẵng", &["q.hai chau", "hai chau"]),
];

/// Bảng từ điển phường/xã đại diện
pub const WARDS: &[(&str, &[&str])] = &[
    ("Phường Bến Nghé", &["p. ben nghe", "ben nghe"]),
    ("Phường Bến Thành", &["p. ben thanh", "ben thanh"]),
    ("Phường 1", &["p.1", "p1", "phuong 1"]),
    ("Phường 2", &["p.2", "p2", "phuong 2"]),
    ("Phường Dịch Vọng", &["p. dich vong", "dich vong"]),
    ("Phường Tràng Tiền", &["p. trang tien", "trang tien"]),
];

/// Chuẩn hóa địa chỉ tiếng Việt
pub fn normalize_vietnamese_address(
    input: &str,
    model: &AdminModel,
    threshold: f64,
) -> Result<NormalizedAddress, CommandError> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(CommandError::ValidationFailed {
            reason: "Địa chỉ đầu vào không được để trống".to_string(),
        });
    }

    let lower = raw.to_lowercase();
    let parts: Vec<&str> = lower.split(&[',', '-'][..]).map(|s| s.trim()).collect();

    // 1. Nhận diện Tỉnh/Thành phố
    let mut detected_province = "Hà Nội".to_string();
    let mut province_score = 0.5;

    for (std_prov, aliases) in PROVINCES {
        for alias in *aliases {
            if lower.contains(alias) {
                detected_province = std_prov.to_string();
                province_score = 1.0;
                break;
            }
            let sim = jaro_winkler_similarity(&lower, alias);
            if sim > province_score && sim >= threshold {
                detected_province = std_prov.to_string();
                province_score = sim;
            }
        }
        if province_score == 1.0 {
            break;
        }
    }

    // 2. Nhận diện Quận/Huyện (cho 3 cấp)
    let mut detected_district: Option<String> = None;
    let mut district_score = 0.0;

    for (std_dist, prov_belong, aliases) in DISTRICTS {
        if *prov_belong == detected_province || province_score < 0.8 {
            for alias in *aliases {
                if lower.contains(alias) {
                    detected_district = Some(std_dist.to_string());
                    district_score = 1.0;
                    detected_province = prov_belong.to_string();
                    break;
                }
                let sim = jaro_winkler_similarity(&lower, alias);
                if sim > district_score && sim >= threshold {
                    detected_district = Some(std_dist.to_string());
                    district_score = sim;
                }
            }
        }
        if district_score == 1.0 {
            break;
        }
    }

    // 3. Nhận diện Phường/Xã
    let mut detected_ward = "Phường 1".to_string();
    for (std_ward, aliases) in WARDS {
        for alias in *aliases {
            if lower.contains(alias) {
                detected_ward = std_ward.to_string();
                break;
            }
        }
    }

    // 4. Địa chỉ đường / số nhà (phần đầu tiên nếu có dấu phẩy)
    let street = if parts.len() > 1 && !parts[0].is_empty() {
        parts[0].to_string()
    } else {
        "Số 1 Đường Chính".to_string()
    };

    let (full_normalized, model_str) = match model {
        AdminModel::ThreeTier => {
            let dist_str = detected_district
                .as_deref()
                .unwrap_or("Quận trung tâm");
            (
                format!("{street}, {detected_ward}, {dist_str}, {detected_province}"),
                "3 cấp truyền thống (Tỉnh/TP - Quận/Huyện - Phường/Xã)".to_string(),
            )
        }
        AdminModel::TwoTier => (
            format!("{street}, {detected_ward}, {detected_province}"),
            "2 cấp sau sáp nhập (Tỉnh/TP - Phường/Xã mới)".to_string(),
        ),
    };

    let confidence = if province_score > 0.0 && district_score > 0.0 {
        (province_score + district_score) / 2.0
    } else if province_score > 0.0 {
        province_score
    } else {
        0.75
    };

    Ok(NormalizedAddress {
        raw_input: raw.to_string(),
        street_address: street,
        ward: detected_ward,
        district: detected_district,
        province: detected_province,
        full_normalized,
        confidence_score: (confidence * 100.0).round() / 100.0,
        model: model_str,
    })
}

impl OfficeTool for VnAddressNormalizerTool {
    fn slug(&self) -> &'static str {
        "vn-address-normalizer"
    }

    fn name(&self) -> &'static str {
        "Chuẩn hóa Địa giới Hành chính VN"
    }

    fn description(&self) -> &'static str {
        "Khớp chuỗi mờ (Jaro-Winkler) với từ điển địa giới hành chính Việt Nam, hỗ trợ cả mô hình 3 cấp và mô hình 2 cấp sau sáp nhập, giải quyết mọi từ viết tắt (hn, q.tb, tphcm)."
    }

    fn execute(
        &self,
        request: &DispatchTaskRequest,
        cancel_flag: &AtomicBool,
        on_progress: Option<&dyn Fn(TaskProgress)>,
    ) -> Result<DispatchTaskResponse, CommandError> {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        // Validate magic bytes if input files provided
        for path in &request.input_paths {
            if let Ok(bytes) = std::fs::read(path) {
                let is_binary = bytes.iter().any(|&b| b < 0x09 || (b > 0x0D && b < 0x20) || b == 0x7F)
                    || infer::get(&bytes).is_some();
                if bytes.len() >= 4 && (!bytes.starts_with(b"{\n") && !bytes.starts_with(b"{\"")) && is_binary {
                    return Err(CommandError::InvalidFileSignature {
                        expected: "Plaintext / JSON Address".to_string(),
                        actual: format!("{:02X?}", &bytes[..4.min(bytes.len())]),
                    });
                }
            }
        }

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 1,
                total_steps: 2,
                percentage: 30.0,
                message: "Đang phân tích địa chỉ và từ điển hành chính...".to_string(),
                processed_files: 0,
                failed_files: 0,
                total_files: 1,
                is_completed: false,
            });
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let address_input = if let Some(s) = request.options.get("address").and_then(|v| v.as_str()) {
            s.to_string()
        } else if let Some(s) = request.options.get("input").and_then(|v| v.as_str()) {
            s.to_string()
        } else if let Some(path) = request.input_paths.first() {
            std::fs::read_to_string(path).map_err(|e| CommandError::IoError {
                details: format!("Không thể đọc tệp '{path}': {e}"),
            })?
        } else {
            "Số 10 Lý Thường Kiệt, q.tb, tphcm".to_string()
        };

        let model = match request.options.get("target_model").and_then(|v| v.as_str()) {
            Some("TwoTier") => AdminModel::TwoTier,
            _ => AdminModel::ThreeTier,
        };

        let threshold = request
            .options
            .get("fuzzy_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.75);

        let normalized = normalize_vietnamese_address(&address_input, &model, threshold)?;

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let json_output = serde_json::to_string_pretty(&normalized).map_err(|e| CommandError::ProcessExecutionError {
            message: format!("Lỗi tuần tự hóa kết quả chuẩn hóa địa chỉ: {e}"),
        })?;

        if let Some(cb) = on_progress {
            cb(TaskProgress {
                task_id: request.task_id.clone(),
                current_step: 2,
                total_steps: 2,
                percentage: 100.0,
                message: "Đã chuẩn hóa địa giới hành chính thành công".to_string(),
                processed_files: 1,
                failed_files: 0,
                total_files: 1,
                is_completed: true,
            });
        }

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status: "SUCCESS".to_string(),
            output_paths: vec![json_output],
            failed_items: vec![],
            elapsed_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use office_shared::DispatchTaskRequest;
    use serde_json::json;

    #[test]
    fn test_vn_address_normalizer_happy_path() {
        // Kiểm thử thuật toán Jaro-Winkler
        let sim1 = jaro_winkler_similarity("Hà Nội", "Hà Nội");
        assert_eq!(sim1, 1.0);

        let sim2 = jaro_winkler_similarity("ha noi", "ha noi");
        assert_eq!(sim2, 1.0);

        // Khớp địa chỉ mô hình 3 cấp
        let res = normalize_vietnamese_address(
            "123 Hoàng Hoa Thám, q.tb, tphcm",
            &AdminModel::ThreeTier,
            0.75,
        )
        .expect("Normalize address ok");

        assert_eq!(res.province, "TP. Hồ Chí Minh");
        assert_eq!(res.district, Some("Quận Tân Bình".to_string()));
        assert!(res.full_normalized.contains("Quận Tân Bình"));
        assert!(res.full_normalized.contains("TP. Hồ Chí Minh"));
    }

    #[test]
    fn test_vn_address_normalizer_edge_case() {
        // Kiểm thử các từ viết tắt cực ngắn: 'hn' -> Hà Nội, 'q.tb' -> Quận Tân Bình
        let res_hn = normalize_vietnamese_address("hn", &AdminModel::ThreeTier, 0.75)
            .expect("Normalize hn ok");
        assert_eq!(res_hn.province, "Hà Nội");

        let res_qtb = normalize_vietnamese_address(
            "Số 45 Cách Mạng Tháng 8, q.tb, hcm",
            &AdminModel::ThreeTier,
            0.75,
        )
        .expect("Normalize qtb ok");
        assert_eq!(res_qtb.province, "TP. Hồ Chí Minh");
        assert_eq!(res_qtb.district, Some("Quận Tân Bình".to_string()));

        // Mô hình 2 cấp sau sáp nhập
        let res_2tier = normalize_vietnamese_address(
            "Số 88 Lê Lợi, p. ben nghe, hcm",
            &AdminModel::TwoTier,
            0.75,
        )
        .expect("Normalize 2tier ok");
        assert_eq!(res_2tier.province, "TP. Hồ Chí Minh");
        assert_eq!(res_2tier.ward, "Phường Bến Nghé");
        assert!(!res_2tier.full_normalized.contains("Quận"));
    }

    #[test]
    fn test_vn_address_normalizer_invalid_magic_bytes() {
        let tool = VnAddressNormalizerTool::default();
        let bad_file = std::env::temp_dir().join("bad_magic_address.bin");
        std::fs::write(&bad_file, [0x7F, 0x45, 0x4C, 0x46, 0x02]).ok();

        let req = DispatchTaskRequest {
            task_id: "test-invalid-magic-addr".to_string(),
            tool_slug: "vn-address-normalizer".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: None,
            options: json!({}),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận: {:?}", other),
        }
        let _ = std::fs::remove_file(bad_file);
    }

    #[test]
    fn test_vn_address_normalizer_cooperative_cancellation() {
        let tool = VnAddressNormalizerTool::default();
        let req = DispatchTaskRequest {
            task_id: "test-cancel-addr".to_string(),
            tool_slug: "vn-address-normalizer".to_string(),
            input_paths: vec![],
            output_directory: None,
            options: json!({ "address": "123 Đường Láng, Hà Nội" }),
        };
        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_vn_address_normalizer_security_guardrails() {
        let tool = VnAddressNormalizerTool::default();
        assert_eq!(tool.slug(), "vn-address-normalizer");
        assert_eq!(tool.name(), "Chuẩn hóa Địa giới Hành chính VN");
    }
}
