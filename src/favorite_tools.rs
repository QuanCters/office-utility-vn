pub const FAVORITE_TOOLS_STORAGE_KEY: &str = "office_utility_favorite_tools_v1";
pub const DEFAULT_MAX_FAVORITES: usize = 10;

/// Thêm hoặc xóa công cụ khỏi danh sách yêu thích.
/// - Nếu đã có trong danh sách -> xóa bỏ (unpin).
/// - Nếu chưa có -> thêm vào cuối danh sách (tối đa `DEFAULT_MAX_FAVORITES`).
/// - Bỏ qua slug rỗng.
pub fn toggle_favorite(existing: &[String], slug: &str) -> Vec<String> {
    let clean_slug = slug.trim();
    if clean_slug.is_empty() {
        return existing.to_vec();
    }

    if let Some(pos) = existing.iter().position(|s| s == clean_slug) {
        let mut updated = existing.to_vec();
        updated.remove(pos);
        updated
    } else {
        let mut updated = existing.to_vec();
        if updated.len() < DEFAULT_MAX_FAVORITES {
            updated.push(clean_slug.to_string());
        }
        updated
    }
}

/// Kiểm tra một công cụ có nằm trong danh sách yêu thích hay không.
pub fn is_favorite(favorites: &[String], slug: &str) -> bool {
    let clean = slug.trim();
    favorites.iter().any(|s| s == clean)
}

/// Chuyển đổi danh sách slug yêu thích sang chuỗi JSON an toàn.
pub fn serialize_favorite_tools(slugs: &[String]) -> String {
    serde_json::to_string(slugs).unwrap_or_else(|_| "[]".to_string())
}

/// Phân tích chuỗi JSON lưu trữ thành danh sách slug yêu thích, không bao giờ panic.
pub fn deserialize_favorite_tools(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

/// Ghi danh sách yêu thích vào LocalStorage của Webview.
pub fn save_favorites_to_storage(slugs: &[String]) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.local_storage() {
                let serialized = serialize_favorite_tools(slugs);
                let _ = storage.set_item(FAVORITE_TOOLS_STORAGE_KEY, &serialized);
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = slugs;
    }
}

/// Đọc danh sách yêu thích từ LocalStorage của Webview khi khởi động app.
pub fn load_favorites_from_storage() -> Vec<String> {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.local_storage() {
                if let Ok(Some(val)) = storage.get_item(FAVORITE_TOOLS_STORAGE_KEY) {
                    let mut list = deserialize_favorite_tools(&val);
                    if list.len() > DEFAULT_MAX_FAVORITES {
                        list.truncate(DEFAULT_MAX_FAVORITES);
                    }
                    return list;
                }
            }
        }
    }
    // Mặc định ban đầu khi chưa cấu hình: 5 công cụ phổ biến
    vec![
        "vietnamese-number-to-words".to_string(),
        "vietqr-napas247-generator".to_string(),
        "pdf-merge-split".to_string(),
        "gross-net-salary-calculator".to_string(),
        "image-batch".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_add_new_favorite() {
        let existing = vec!["tool-a".to_string()];
        let result = toggle_favorite(&existing, "tool-b");
        assert_eq!(result, vec!["tool-a", "tool-b"]);
    }

    #[test]
    fn test_toggle_remove_existing_favorite() {
        let existing = vec!["tool-a".to_string(), "tool-b".to_string()];
        let result = toggle_favorite(&existing, "tool-a");
        assert_eq!(result, vec!["tool-b"]);
    }

    #[test]
    fn test_toggle_empty_slug_noop() {
        let existing = vec!["tool-a".to_string()];
        let result = toggle_favorite(&existing, "  ");
        assert_eq!(result, existing);
    }

    #[test]
    fn test_toggle_respects_max_limit() {
        let existing: Vec<String> = (0..10).map(|i| format!("tool-{}", i)).collect();
        let result = toggle_favorite(&existing, "tool-new");
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_is_favorite_true_and_false() {
        let favs = vec!["pdf-merge".to_string()];
        assert!(is_favorite(&favs, "pdf-merge"));
        assert!(!is_favorite(&favs, "pdf-split"));
    }

    #[test]
    fn test_deserialize_corrupt_favorites_fallback() {
        let corrupt = "{not valid json[";
        let result = deserialize_favorite_tools(corrupt);
        assert!(result.is_empty());
    }

    #[test]
    fn test_serialize_and_deserialize_favorites_roundtrip() {
        let original = vec!["pdf-merge-split".to_string(), "vietqr-napas247-generator".to_string()];
        let json_str = serialize_favorite_tools(&original);
        let restored = deserialize_favorite_tools(&json_str);
        assert_eq!(restored, original);
    }
}
