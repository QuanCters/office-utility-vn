pub const RECENT_TOOLS_STORAGE_KEY: &str = "office_utility_recent_tools_v1";
pub const DEFAULT_MAX_RECENT_TOOLS: usize = 5;

/// Cập nhật danh sách công cụ gần nhất theo giải thuật MRU (Most Recently Used).
/// - Nếu `new_slug` đã tồn tại trong danh sách, đẩy lên vị trí đầu (index 0).
/// - Nếu `new_slug` chưa có, thêm vào vị trí đầu.
/// - Cắt tỉa danh sách khi vượt quá `limit`.
pub fn update_recent_list(existing: &[String], new_slug: &str, limit: usize) -> Vec<String> {
    let clean_slug = new_slug.trim();
    if clean_slug.is_empty() {
        return existing.to_vec();
    }

    let mut result = Vec::with_capacity(limit.max(existing.len() + 1));
    result.push(clean_slug.to_string());

    for item in existing {
        if item != clean_slug {
            result.push(item.clone());
        }
    }

    if result.len() > limit {
        result.truncate(limit);
    }

    result
}

/// Chuyển đổi mảng slug sang chuỗi JSON lưu vào LocalStorage.
pub fn serialize_recent_tools(slugs: &[String]) -> String {
    serde_json::to_string(slugs).unwrap_or_else(|_| "[]".to_string())
}

/// Phân tích chuỗi JSON từ LocalStorage thành mảng slug an toàn, không bao giờ panic.
pub fn deserialize_recent_tools(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

/// Ghi danh sách slug vào LocalStorage của Webview.
pub fn save_recent_to_storage(slugs: &[String]) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.local_storage() {
                let serialized = serialize_recent_tools(slugs);
                let _ = storage.set_item(RECENT_TOOLS_STORAGE_KEY, &serialized);
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = slugs;
    }
}

/// Đọc danh sách slug từ LocalStorage của Webview khi khởi động app.
pub fn load_recent_from_storage() -> Vec<String> {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.local_storage() {
                if let Ok(Some(val)) = storage.get_item(RECENT_TOOLS_STORAGE_KEY) {
                    let mut list = deserialize_recent_tools(&val);
                    if list.len() > DEFAULT_MAX_RECENT_TOOLS {
                        list.truncate(DEFAULT_MAX_RECENT_TOOLS);
                    }
                    return list;
                }
            }
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_recent_list_insert_and_ordering() {
        let existing = vec!["tool-b".to_string(), "tool-c".to_string()];
        let updated = update_recent_list(&existing, "tool-a", DEFAULT_MAX_RECENT_TOOLS);
        assert_eq!(updated, vec!["tool-a", "tool-b", "tool-c"]);
    }

    #[test]
    fn test_update_recent_list_dedup_and_promote() {
        let existing = vec!["tool-a".to_string(), "tool-b".to_string(), "tool-c".to_string()];
        let updated = update_recent_list(&existing, "tool-c", DEFAULT_MAX_RECENT_TOOLS);
        assert_eq!(updated, vec!["tool-c", "tool-a", "tool-b"]);
    }

    #[test]
    fn test_update_recent_list_capacity_limit() {
        let existing = vec![
            "t1".to_string(),
            "t2".to_string(),
            "t3".to_string(),
            "t4".to_string(),
            "t5".to_string(),
            "t6".to_string(),
        ];
        let updated = update_recent_list(&existing, "t-new", DEFAULT_MAX_RECENT_TOOLS);
        assert_eq!(updated.len(), 5);
        assert_eq!(updated[0], "t-new");
        assert_eq!(updated[4], "t4"); // t5, t6 must be dropped
    }

    #[test]
    fn test_serialize_and_deserialize_roundtrip() {
        let original = vec!["pdf-merge-split".to_string(), "vietqr-napas247-generator".to_string()];
        let json_str = serialize_recent_tools(&original);
        let restored = deserialize_recent_tools(&json_str);
        assert_eq!(restored, original);
    }

    #[test]
    fn test_deserialize_corrupt_data_fallback() {
        let corrupt = "invalid-non-json-string-12345";
        let restored = deserialize_recent_tools(corrupt);
        assert!(restored.is_empty(), "Phải trả về mảng rỗng an toàn khi JSON hỏng");
    }

    #[test]
    fn test_update_recent_list_empty_slug_noop() {
        let existing = vec!["tool-a".to_string()];
        let updated = update_recent_list(&existing, "   ", 6);
        assert_eq!(updated, existing);
    }
}
