use leptos::prelude::*;
use crate::micro_apps::data_lab::state::DataLabState;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["navigator", "clipboard"])]
    fn writeText(text: &str) -> js_sys::Promise;
}

/// Tiện ích sao chép văn bản vào clipboard an toàn cho cả WASM và Non-WASM
fn copy_to_clipboard(text: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = writeText(text);
    }
    let _ = text;
}

/// Tiện ích kích hoạt trình duyệt tải tệp dữ liệu về máy (WASM)
fn trigger_browser_download(filename: &str, content: &str, mime_type: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::window;
        if let Some(win) = window() {
            if let Some(doc) = win.document() {
                if let Ok(a) = doc.create_element("a") {
                    let encoded = js_sys::encode_uri_component(content);
                    let href = format!("data:{};charset=utf-8,{}", mime_type, encoded);
                    let _ = a.set_attribute("href", &href);
                    let _ = a.set_attribute("download", filename);
                    if let Ok(el) = a.dyn_into::<web_sys::HtmlElement>() {
                        el.click();
                    }
                }
            }
        }
    }
    let _ = (filename, content, mime_type);
}

/// Định nghĩa kiểu dữ liệu cột trong bảng
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ColumnType {
    Text,
    Number,
    Date,
    Other,
}

impl ColumnType {
    pub fn icon(&self) -> &'static str {
        match self {
            ColumnType::Text => "ABC",
            ColumnType::Number => "123",
            ColumnType::Date => "📅",
            ColumnType::Other => "⚡",
        }
    }
}

/// Metadata của một cột
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TableColumn {
    pub name: String,
    pub col_type: ColumnType,
}

/// Bước chuyển đổi dữ liệu trong Time-travel Recipe Engine
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RecipeStep {
    pub id: String,
    pub action: String,
    pub description: String,
    pub target_col: Option<String>,
    pub timestamp: String,
}

/// Thông tin hồ sơ phân tích chất lượng của 1 cột (Data Profiling)
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnProfile {
    pub total: usize,
    pub valid_count: usize,
    pub null_count: usize,
    pub valid_percentage: f32,
    pub null_percentage: f32,
    pub top_values: Vec<(String, usize, f32)>,
}

/// Cụm giá trị tương đồng trong thuật toán Fuzzy Clustering
#[derive(Clone, Debug, PartialEq)]
pub struct FuzzyCluster {
    pub canonical: String,
    pub variations: Vec<(String, usize)>,
    pub total_occurrences: usize,
}

// ==============================================================================
// THUẬT TOÁN NGHIỆP VỤ (BUSINESS LOGIC)
// ==============================================================================

/// Bảng danh sách họ kép phổ biến trong tiếng Việt
const VN_DOUBLE_SURNAMES: &[&str] = &[
    "nguyễn phước", "nguyễn phúc", "phan huy", "hoàng phủ", "võ đình", "đặng vũ",
    "ngô bảo", "trần đăng", "lê hoàng", "hồ xuân",
    "dương triệu", "trịnh thăng", "mai thúc", "vũ trọng", "cao bá", "đinh bộ",
];

/// Tách họ tên tiếng Việt nhận diện chính xác họ kép
pub fn split_vietnamese_full_name(full_name: &str) -> (String, String, String) {
    let parts: Vec<&str> = full_name.split_whitespace().collect();
    if parts.is_empty() {
        return (String::new(), String::new(), String::new());
    }
    if parts.len() == 1 {
        return (parts[0].to_string(), String::new(), parts[0].to_string());
    }
    if parts.len() == 2 {
        return (parts[0].to_string(), String::new(), parts[1].to_string());
    }

    let first_two_lower = format!("{} {}", parts[0], parts[1]).to_lowercase();
    let is_double_surname = VN_DOUBLE_SURNAMES.contains(&first_two_lower.as_str());

    if is_double_surname && parts.len() >= 3 {
        let ho = format!("{} {}", parts[0], parts[1]);
        let ten = parts[parts.len() - 1].to_string();
        let dem = if parts.len() > 3 {
            parts[2..parts.len() - 1].join(" ")
        } else {
            String::new()
        };
        (ho, dem, ten)
    } else {
        let ho = parts[0].to_string();
        let ten = parts[parts.len() - 1].to_string();
        let dem = parts[1..parts.len() - 1].join(" ");
        (ho, dem, ten)
    }
}

/// Chuẩn hóa ngày tháng hỗn hợp về chuẩn DD/MM/YYYY
pub fn normalize_date_cell(val: &str) -> String {
    let s = val.trim();
    if s.is_empty() {
        return String::new();
    }

    let parts: Vec<&str> = if s.contains('-') {
        s.split('-').collect()
    } else if s.contains('/') {
        s.split('/').collect()
    } else {
        return s.to_string();
    };

    if parts.len() != 3 {
        return s.to_string();
    }

    // YYYY-MM-DD hoặc YYYY/MM/DD
    if parts[0].len() == 4 {
        if let (Ok(y), Ok(m), Ok(d)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>(), parts[2].parse::<u32>()) {
            return format!("{:02}/{:02}/{:04}", d, m, y);
        }
    }

    // DD/MM/YYYY hoặc MM/DD/YYYY
    if parts[2].len() == 4 {
        if let (Ok(p0), Ok(p1), Ok(y)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>(), parts[2].parse::<u32>()) {
            if p1 > 12 && p0 <= 12 {
                return format!("{:02}/{:02}/{:04}", p1, p0, y);
            } else {
                return format!("{:02}/{:02}/{:04}", p0, p1, y);
            }
        }
    }

    s.to_string()
}

/// Xóa khoảng trắng thừa và nén nhiều khoảng trắng liên tiếp
pub fn trim_whitespace_cell(val: &str) -> String {
    val.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Chuyển chuỗi tiếng Việt thành chữ thường không dấu để so sánh tương đồng
pub fn remove_vietnamese_accents(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let ch = match c {
            'a' | 'A' | 'á' | 'Á' | 'à' | 'À' | 'ả' | 'Ả' | 'ã' | 'Ã' | 'ạ' | 'Ạ'
            | 'ă' | 'Ă' | 'ắ' | 'Ắ' | 'ằ' | 'Ằ' | 'ẳ' | 'Ẳ' | 'ẵ' | 'Ẵ' | 'ặ' | 'Ặ'
            | 'â' | 'Â' | 'ấ' | 'Ấ' | 'ầ' | 'Ầ' | 'ẩ' | 'Ẩ' | 'ẫ' | 'Ẫ' | 'ậ' | 'Ậ' => 'a',
            'e' | 'E' | 'é' | 'É' | 'è' | 'È' | 'ẻ' | 'Ẻ' | 'ẽ' | 'Ẽ' | 'ẹ' | 'Ẹ'
            | 'ê' | 'Ê' | 'ế' | 'Ế' | 'ề' | 'Ề' | 'ể' | 'Ể' | 'ễ' | 'Ễ' | 'ệ' | 'Ệ' => 'e',
            'i' | 'I' | 'í' | 'Í' | 'ì' | 'Ì' | 'ỉ' | 'Ỉ' | 'ĩ' | 'Ĩ' | 'ị' | 'Ị' => 'i',
            'o' | 'O' | 'ó' | 'Ó' | 'ò' | 'Ò' | 'ỏ' | 'Ỏ' | 'õ' | 'Õ' | 'ọ' | 'Ọ'
            | 'ô' | 'Ô' | 'ố' | 'Ố' | 'ồ' | 'Ồ' | 'ổ' | 'Ổ' | 'ỗ' | 'Ỗ' | 'ộ' | 'Ộ'
            | 'ơ' | 'Ơ' | 'ớ' | 'Ớ' | 'ờ' | 'Ờ' | 'ở' | 'Ở' | 'ỡ' | 'Ỡ' | 'ợ' | 'Ợ' => 'o',
            'u' | 'U' | 'ú' | 'Ú' | 'ù' | 'Ù' | 'ủ' | 'Ủ' | 'ũ' | 'Ũ' | 'ụ' | 'Ụ'
            | 'ư' | 'Ư' | 'ứ' | 'Ứ' | 'ừ' | 'Ừ' | 'ử' | 'Ử' | 'ữ' | 'Ữ' | 'ự' | 'Ự' => 'u',
            'y' | 'Y' | 'ý' | 'Ý' | 'ỳ' | 'Ỳ' | 'ỷ' | 'Ỷ' | 'ỹ' | 'Ỹ' | 'ỵ' | 'Ỵ' => 'y',
            'd' | 'D' | 'đ' | 'Đ' => 'd',
            other => other.to_ascii_lowercase(),
        };
        out.push(ch);
    }
    out
}

/// Đơn giản hóa chuỗi để gom cụm Fuzzy
pub fn simplify_for_cluster(s: &str) -> String {
    let no_accent = remove_vietnamese_accents(s);
    no_accent
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Tính khoảng cách Levenshtein giữa 2 chuỗi
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0; b_chars.len() + 1]; a_chars.len() + 1];
    for i in 0..=a_chars.len() {
        dp[i][0] = i;
    }
    for j in 0..=b_chars.len() {
        dp[0][j] = j;
    }
    for (i, ca) in a_chars.iter().enumerate() {
        for (j, cb) in b_chars.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            dp[i + 1][j + 1] = (dp[i][j + 1] + 1)
                .min(dp[i + 1][j] + 1)
                .min(dp[i][j] + cost);
        }
    }
    dp[a_chars.len()][b_chars.len()]
}

/// Thuật toán phát hiện các cụm sai chính tả / biến thể gõ dấu trong cột
pub fn detect_fuzzy_clusters(col_idx: usize, rows: &[Vec<String>]) -> Vec<FuzzyCluster> {
    let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for row in rows {
        if let Some(val) = row.get(col_idx) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                *freq.entry(trimmed.to_string()).or_insert(0) += 1;
            }
        }
    }

    if freq.len() <= 1 {
        return Vec::new();
    }

    let items: Vec<(String, usize)> = freq.into_iter().collect();
    let mut visited = vec![false; items.len()];
    let mut clusters = Vec::new();

    for i in 0..items.len() {
        if visited[i] {
            continue;
        }
        let mut group = vec![items[i].clone()];
        visited[i] = true;

        let norm_i = simplify_for_cluster(&items[i].0);

        for j in (i + 1)..items.len() {
            if visited[j] {
                continue;
            }
            let norm_j = simplify_for_cluster(&items[j].0);

            let is_match = norm_i == norm_j
                || (norm_i.len() > 3 && norm_j.len() > 3 && levenshtein(&norm_i, &norm_j) <= 2)
                || (norm_i.starts_with(&norm_j) || norm_j.starts_with(&norm_i));

            if is_match {
                group.push(items[j].clone());
                visited[j] = true;
            }
        }

        if group.len() > 1 {
            let canonical = group
                .iter()
                .max_by_key(|(val, count)| (*count, val.chars().filter(|c| !c.is_ascii()).count()))
                .map(|(val, _)| val.clone())
                .unwrap_or_else(|| items[i].0.clone());

            let total_occurrences = group.iter().map(|(_, c)| *c).sum();
            clusters.push(FuzzyCluster {
                canonical,
                variations: group,
                total_occurrences,
            });
        }
    }

    clusters
}

/// Tính toán Profile chất lượng và phân phối dữ liệu cho một cột
pub fn calculate_column_profile(col_idx: usize, rows: &[Vec<String>]) -> ColumnProfile {
    let total = rows.len();
    if total == 0 {
        return ColumnProfile {
            total: 0,
            valid_count: 0,
            null_count: 0,
            valid_percentage: 100.0,
            null_percentage: 0.0,
            top_values: Vec::new(),
        };
    }

    let mut freq_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut null_count = 0;

    for row in rows {
        let val = row.get(col_idx).map(|s| s.trim()).unwrap_or("");
        if val.is_empty() {
            null_count += 1;
        } else {
            *freq_map.entry(val.to_string()).or_insert(0) += 1;
        }
    }

    let valid_count = total.saturating_sub(null_count);
    let valid_percentage = (valid_count as f32 / total as f32) * 100.0;
    let null_percentage = (null_count as f32 / total as f32) * 100.0;

    let mut top_values: Vec<(String, usize, f32)> = freq_map
        .into_iter()
        .map(|(val, count)| {
            let pct = (count as f32 / total as f32) * 100.0;
            (val, count, pct)
        })
        .collect();

    top_values.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    ColumnProfile {
        total,
        valid_count,
        null_count,
        valid_percentage,
        null_percentage,
        top_values,
    }
}

/// Xuất dữ liệu bảng sang định dạng CSV có UTF-8 BOM
pub fn table_to_csv(columns: &[TableColumn], rows: &[Vec<String>]) -> String {
    let mut out = String::from("\u{FEFF}");
    let header_line = columns
        .iter()
        .map(|c| format!("\"{}\"", c.name.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(",");
    out.push_str(&header_line);
    out.push('\n');

    for row in rows {
        let row_line = row
            .iter()
            .map(|cell| format!("\"{}\"", cell.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(",");
        out.push_str(&row_line);
        out.push('\n');
    }

    out
}

/// Tạo bộ dữ liệu mẫu demo phong phú gồm 25 dòng
pub fn create_demo_table() -> (Vec<TableColumn>, Vec<Vec<String>>) {
    let columns = vec![
        TableColumn { name: "Mã NV".to_string(), col_type: ColumnType::Number },
        TableColumn { name: "Họ và tên".to_string(), col_type: ColumnType::Text },
        TableColumn { name: "Ngày sinh".to_string(), col_type: ColumnType::Date },
        TableColumn { name: "Tỉnh thành".to_string(), col_type: ColumnType::Text },
        TableColumn { name: "Phòng ban".to_string(), col_type: ColumnType::Text },
        TableColumn { name: "Lương cơ bản".to_string(), col_type: ColumnType::Number },
    ];

    let rows = vec![
        vec!["NV001".into(), "  Nguyễn   Văn An  ".into(), "1990-05-15".into(), "Hà Nội".into(), "Kỹ thuật".into(), "22000000".into()],
        vec!["NV002".into(), "Trần  Thị  Mai ".into(), "12/08/1992".into(), "Ha Noi".into(), "Kế toán".into(), "18500000".into()],
        vec!["NV003".into(), "Lê   Hoàng   Nam".into(), "1988/04/25".into(), "Hà Nôi".into(), "Kinh doanh".into(), "25000000".into()],
        vec!["NV004".into(), "Phan  Huy  Chú  ".into(), "1995-11-03".into(), "Hanoi".into(), "R&D".into(), "30000000".into()],
        vec!["NV005".into(), " Đặng  Vũ   Linh".into(), "20/09/1996".into(), "TP. Hồ Chí Minh".into(), "Kỹ thuật".into(), "24000000".into()],
        vec!["NV006".into(), "Hoàng  Phủ  Ngọc Tường  ".into(), "1985-12-01".into(), "TP HCM".into(), "Ban Giám đốc".into(), "45000000".into()],
        vec!["NV007".into(), "Võ   Đình  Tú".into(), "15/07/1994".into(), "Ho Chi Minh".into(), "Kỹ thuật".into(), "21000000".into()],
        vec!["NV008".into(), "Phạm   Minh   Đức ".into(), "1991/01/30".into(), "TP.HCM".into(), "Marketing".into(), "19000000".into()],
        vec!["NV009".into(), "Bùi  Thị   Ngọc Lan".into(), "18/04/1993".into(), "Đà Nẵng".into(), "Nhân sự".into(), "17000000".into()],
        vec!["NV010".into(), "  Ngô  Bảo   Châu ".into(), "1972-06-28".into(), "Da Nang".into(), "Nghiên cứu".into(), "50000000".into()],
        vec!["NV011".into(), "Đỗ   Mỹ  Linh".into(), "1996/10/13".into(), "Đa Nang".into(), "Truyền thông".into(), "20000000".into()],
        vec!["NV012".into(), "Dương  Triệu   Vũ".into(), "01/01/1984".into(), "Cần Thơ".into(), "Kinh doanh".into(), "23000000".into()],
        vec!["NV013".into(), "Nguyễn  Phước   Vĩnh Lộc".into(), "1992-09-02".into(), "Can Tho".into(), "Tài chính".into(), "28000000".into()],
        vec!["NV014".into(), "Trịnh   Thăng  Bình ".into(), "1988/11/23".into(), "Hải Phòng".into(), "Marketing".into(), "22500000".into()],
        vec!["NV015".into(), "Lý  Thái   Tổ  ".into(), "1974-03-10".into(), "Hai Phong".into(), "Quản lý".into(), "38000000".into()],
        vec!["NV016".into(), "Hồ   Xuân   Hương".into(), "1980/02/14".into(), "Hà Nội".into(), "Hành chính".into(), "16500000".into()],
        vec!["NV017".into(), "Huỳnh   Tấn   Phát ".into(), "1987-08-19".into(), "TP. Hồ Chí Minh".into(), "Kỹ thuật".into(), "26000000".into()],
        vec!["NV018".into(), "Đinh   Bộ   Lĩnh ".into(), "1979/07/07".into(), "Ninh Bình".into(), "Vận hành".into(), "27000000".into()],
        vec!["NV019".into(), "Mai   Thúc   Loan".into(), "1983-12-25".into(), "Nghệ An".into(), "Kinh doanh".into(), "19500000".into()],
        vec!["NV020".into(), "Vũ   Trọng  Phụng  ".into(), "1986/10/20".into(), "Hà Nội".into(), "Nội dung".into(), "21500000".into()],
        vec!["NV021".into(), "Nguyễn   Du".into(), "1965-01-03".into(), "".into(), "Cố vấn".into(), "40000000".into()],
        vec!["NV022".into(), "Cao   Bá   Quát  ".into(), "1982/06/18".into(), "Hà Nội".into(), "".into(), "23000000".into()],
        vec!["NV023".into(), "Lê  Lợi ".into(), "1985-09-10".into(), "Thanh Hóa".into(), "Ban Giám đốc".into(), "".into()],
        vec!["NV024".into(), "Trần  Hưng  Đạo".into(), "".into(), "Nam Định".into(), "An ninh".into(), "35000000".into()],
        vec!["NV025".into(), "Nguyễn  Trãi".into(), "1980/08/15".into(), "Hà Nội".into(), "Chiến lược".into(), "36000000".into()],
    ];

    (columns, rows)
}

// ==============================================================================
// COMPONENT NHẬP LIỆU AN TOÀN IME TIẾNG VIỆT (NFR-COM-02)
// ==============================================================================

#[component]
fn ImeSafeSearchInput(
    value: RwSignal<String>,
    #[prop(into)] placeholder: String,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let is_composing = RwSignal::new(false);
    view! {
        <input
            type="text"
            class=class
            placeholder=placeholder
            prop:value=move || value.get()
            on:compositionstart=move |_| is_composing.set(true)
            on:compositionend=move |ev| {
                is_composing.set(false);
                value.set(event_target_value(&ev));
            }
            on:input=move |ev| {
                if !is_composing.get_untracked() {
                    value.set(event_target_value(&ev));
                }
            }
        />
    }
}

// ==============================================================================
// COMPONENT CHÍNH: SpreadsheetGridFacetView
// ==============================================================================

/// Component giao diện Bảng tính kết hợp Facet & Profiling Sidebar (mô hình OpenRefine).
#[component]
pub fn SpreadsheetGridFacetView(state: DataLabState) -> impl IntoView {
    let (initial_cols, initial_rows) = create_demo_table();

    // Trạng thái dữ liệu bảng hiện tại
    let columns = RwSignal::new(initial_cols);
    let rows = RwSignal::new(initial_rows);

    // Lịch sử Time-travel Undo / Redo
    let history_stack = RwSignal::new(vec![(
        columns.get_untracked(),
        rows.get_untracked(),
        RecipeStep {
            id: "step-0".to_string(),
            action: "initial_load".to_string(),
            description: "Nạp bảng dữ liệu ban đầu".to_string(),
            target_col: None,
            timestamp: "00:00:00".to_string(),
        },
    )]);
    let history_index = RwSignal::new(0usize);

    // Lựa chọn cột và lọc Facet
    let selected_col_idx = RwSignal::new(1usize); // Mặc định chọn cột Họ và tên
    let facet_search = RwSignal::new(String::new());
    let active_filter = RwSignal::new(Option::<(usize, String)>::None);

    // Phân trang
    let current_page = RwSignal::new(1usize);
    let page_size = RwSignal::new(15usize);

    // Hộp thoại Modal & Toast
    let is_fuzzy_modal_open = RwSignal::new(false);
    let is_history_modal_open = RwSignal::new(false);
    let is_import_modal_open = RwSignal::new(false);
    let import_recipe_text = RwSignal::new(String::new());
    let toast_message = RwSignal::new(Option::<String>::None);

    // Hàm tiện ích hiển thị Toast
    let show_toast = move |msg: String| {
        toast_message.set(Some(msg));
    };

    // Hàm áp dụng snapshot mới vào History Stack
    let push_history_step = move |new_cols: Vec<TableColumn>, new_rows: Vec<Vec<String>>, action: &str, desc: &str, target: Option<String>| {
        let cur_idx = history_index.get();
        let step = RecipeStep {
            id: format!("step-{}", cur_idx + 1),
            action: action.to_string(),
            description: desc.to_string(),
            target_col: target,
            timestamp: "Thao tác vừa xong".to_string(),
        };

        history_stack.update(|stack| {
            if cur_idx + 1 < stack.len() {
                stack.truncate(cur_idx + 1);
            }
            stack.push((new_cols.clone(), new_rows.clone(), step));
        });

        history_index.set(cur_idx + 1);
        columns.set(new_cols);
        rows.set(new_rows);
        current_page.set(1);
    };

    // Hoàn tác (Undo)
    let on_undo = move || {
        let cur_idx = history_index.get();
        if cur_idx > 0 {
            let next_idx = cur_idx - 1;
            history_index.set(next_idx);
            history_stack.with(|stack| {
                if let Some((c, r, _)) = stack.get(next_idx) {
                    columns.set(c.clone());
                    rows.set(r.clone());
                }
            });
            show_toast("↩ Đã hoàn tác thao tác trước đó.".to_string());
        }
    };

    // Làm lại (Redo)
    let _on_redo = move || {
        let cur_idx = history_index.get();
        let max_idx = history_stack.with(|s| s.len().saturating_sub(1));
        if cur_idx < max_idx {
            let next_idx = cur_idx + 1;
            history_index.set(next_idx);
            history_stack.with(|stack| {
                if let Some((c, r, _)) = stack.get(next_idx) {
                    columns.set(c.clone());
                    rows.set(r.clone());
                }
            });
            show_toast("↪ Đã làm lại thao tác kế tiếp.".to_string());
        }
    };

    // Tải lại bộ dữ liệu Demo
    let on_reload_demo = move || {
        let (demo_c, demo_r) = create_demo_table();
        push_history_step(demo_c, demo_r, "reload_demo", "Nạp lại dữ liệu mẫu demo 25 dòng", None);
        active_filter.set(None);
        show_toast("✓ Đã nạp lại bảng dữ liệu mẫu demo.".to_string());
    };

    // Thao tác 1: Làm sạch khoảng trắng thừa
    let on_clean_whitespace = move || {
        let cur_cols = columns.get();
        let mut cur_rows = rows.get();
        for row in cur_rows.iter_mut() {
            for cell in row.iter_mut() {
                *cell = trim_whitespace_cell(cell);
            }
        }
        push_history_step(cur_cols, cur_rows, "trim_spaces", "Làm sạch khoảng trắng thừa trên toàn bộ bảng", None);
        show_toast("✓ Đã xóa khoảng trắng thừa và chuẩn hóa khoảng cách các ô.".to_string());
    };

    // Thao tác 2: Chuẩn hóa ngày tháng
    let on_normalize_dates = move || {
        let cur_cols = columns.get();
        let mut cur_rows = rows.get();
        let date_col_indices: Vec<usize> = cur_cols
            .iter()
            .enumerate()
            .filter(|(_, c)| c.col_type == ColumnType::Date || c.name.to_lowercase().contains("ngày"))
            .map(|(idx, _)| idx)
            .collect();

        for row in cur_rows.iter_mut() {
            for &idx in &date_col_indices {
                if let Some(cell) = row.get_mut(idx) {
                    *cell = normalize_date_cell(cell);
                }
            }
        }
        push_history_step(cur_cols, cur_rows, "normalize_dates", "Chuẩn hóa định dạng ngày sang DD/MM/YYYY", None);
        show_toast("✓ Đã chuẩn hóa ngày tháng về định dạng chuẩn DD/MM/YYYY.".to_string());
    };

    // Thao tác 3: Tách họ tên tiếng Việt
    let on_split_names = move || {
        let cur_cols = columns.get();
        let mut cur_rows = rows.get();

        let name_col_opt = cur_cols.iter().position(|c| {
            let n = c.name.to_lowercase();
            n.contains("họ và tên") || n.contains("họ tên") || n == "tên"
        });

        if let Some(idx) = name_col_opt {
            let mut new_cols = cur_cols.clone();
            new_cols.remove(idx);
            new_cols.insert(idx, TableColumn { name: "Họ".to_string(), col_type: ColumnType::Text });
            new_cols.insert(idx + 1, TableColumn { name: "Tên đệm".to_string(), col_type: ColumnType::Text });
            new_cols.insert(idx + 2, TableColumn { name: "Tên".to_string(), col_type: ColumnType::Text });

            for row in cur_rows.iter_mut() {
                let old_name = if idx < row.len() { row.remove(idx) } else { String::new() };
                let (ho, dem, ten) = split_vietnamese_full_name(&old_name);
                row.insert(idx, ho);
                row.insert(idx + 1, dem);
                row.insert(idx + 2, ten);
            }

            push_history_step(new_cols, cur_rows, "split_vietnamese_names", "Tách cột Họ và tên thành Họ, Tên đệm, Tên", Some("Họ và tên".to_string()));
            selected_col_idx.set(idx);
            show_toast("✓ Đã tách cột Họ / Tên đệm / Tên theo quy tắc họ kép tiếng Việt.".to_string());
        } else {
            show_toast("⚠️ Không tìm thấy cột chứa Họ và tên trong bảng.".to_string());
        }
    };

    // Thao tác 4: Áp dụng gom cụm Fuzzy (Cluster & Edit)
    let on_apply_fuzzy_merge = move || {
        let col_idx = selected_col_idx.get();
        let cur_cols = columns.get();
        let mut cur_rows = rows.get();

        let clusters = detect_fuzzy_clusters(col_idx, &cur_rows);
        if clusters.is_empty() {
            show_toast("Không phát hiện thêm cụm tương đồng nào để gộp.".to_string());
            is_fuzzy_modal_open.set(false);
            return;
        }

        let mut replace_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for cluster in &clusters {
            for (var, _) in &cluster.variations {
                replace_map.insert(var.clone(), cluster.canonical.clone());
            }
        }

        for row in cur_rows.iter_mut() {
            if let Some(cell) = row.get_mut(col_idx) {
                let trimmed = cell.trim();
                if let Some(target) = replace_map.get(trimmed) {
                    *cell = target.clone();
                }
            }
        }

        let col_name = cur_cols.get(col_idx).map(|c| c.name.clone()).unwrap_or_else(|| "Cột".to_string());
        let desc = format!("Gom cụm và chuẩn hóa biến thể chính tả cho cột {}", col_name);
        push_history_step(cur_cols, cur_rows, "fuzzy_cluster_merge", &desc, Some(col_name));
        is_fuzzy_modal_open.set(false);
        show_toast("✓ Đã gộp các cụm sai chính tả về giá trị chuẩn!".to_string());
    };

    // Xuất CSV
    let on_export_csv = move || {
        let cur_cols = columns.get();
        let cur_rows = rows.get();
        let csv_data = table_to_csv(&cur_cols, &cur_rows);
        copy_to_clipboard(&csv_data);
        trigger_browser_download("data_export.csv", &csv_data, "text/csv");
        show_toast("✓ Đã tải tệp CSV và sao chép dữ liệu vào Clipboard!".to_string());
    };

    // Xuất Recipe JSON
    let on_export_recipe = move || {
        let steps: Vec<RecipeStep> = history_stack.with(|s| {
            s.iter().map(|(_, _, step)| step.clone()).collect()
        });
        if let Ok(json_str) = serde_json::to_string_pretty(&steps) {
            copy_to_clipboard(&json_str);
            trigger_browser_download("recipe_workflow.json", &json_str, "application/json");
            show_toast("✓ Đã sao chép Recipe JSON và tải về máy!".to_string());
        }
    };

    // Nhập và Replay Recipe JSON
    let on_execute_import_recipe = move || {
        let raw = import_recipe_text.get();
        if let Ok(steps) = serde_json::from_str::<Vec<RecipeStep>>(&raw) {
            for step in steps {
                match step.action.as_str() {
                    "trim_spaces" => {
                        let cur_c = columns.get();
                        let mut cur_r = rows.get();
                        for r in cur_r.iter_mut() {
                            for c in r.iter_mut() {
                                *c = trim_whitespace_cell(c);
                            }
                        }
                        push_history_step(cur_c, cur_r, "trim_spaces", &step.description, None);
                    }
                    "normalize_dates" => {
                        let cur_c = columns.get();
                        let mut cur_r = rows.get();
                        let date_indices: Vec<usize> = cur_c.iter().enumerate()
                            .filter(|(_, c)| c.col_type == ColumnType::Date || c.name.to_lowercase().contains("ngày"))
                            .map(|(i, _)| i).collect();
                        for r in cur_r.iter_mut() {
                            for &i in &date_indices {
                                if let Some(cell) = r.get_mut(i) {
                                    *cell = normalize_date_cell(cell);
                                }
                            }
                        }
                        push_history_step(cur_c, cur_r, "normalize_dates", &step.description, None);
                    }
                    _ => {}
                }
            }
            is_import_modal_open.set(false);
            show_toast("✓ Đã áp dụng thành công chuỗi Recipe biến đổi!".to_string());
        } else {
            show_toast("❌ Cú pháp Recipe JSON không hợp lệ.".to_string());
        }
    };

    view! {
        <div class="spreadsheet-grid-view">
            // Header trên cùng của Spreadsheet Data Grid
            <header class="grid-top-bar">
                <div class="grid-file-info">
                    <span style="font-size: 16px;">"📊"</span>
                    <span style="font-weight: 700;">
                        {move || state.active_file_name().unwrap_or_else(|| "demo_nhan_vien_vn.xlsx".to_string())}
                    </span>
                    <span class="grid-stat-chip">
                        {move || format!("{} dòng", rows.with(|r| r.len()))}
                    </span>
                    <span class="grid-stat-chip">
                        {move || format!("{} cột", columns.with(|c| c.len()))}
                    </span>
                    <span class="grid-stat-chip" style="color: var(--emerald);">
                        "🔒 100% On-Device"
                    </span>
                </div>

                <div style="display: flex; align-items: center; gap: 8px;">
                    // Dropdown chọn Sheet
                    <label style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-muted);">
                        <span>"Trang tính:"</span>
                        <select
                            class="pagination-btn"
                            prop:value=move || state.active_sheet.get()
                            on:change=move |ev| state.set_sheet(event_target_value(&ev))
                        >
                            {move || state.available_sheets.get().into_iter().map(|sheet| {
                                view! {
                                    <option value=sheet.clone()>{sheet.clone()}</option>
                                }
                            }).collect::<Vec<_>>()}
                        </select>
                    </label>

                    // Nút Tải lại mẫu demo nếu muốn khám phá
                    <button
                        type="button"
                        class="pagination-btn"
                        on:click=move |_| on_reload_demo()
                        title="Tải lại bảng dữ liệu nhân sự mẫu với các lỗi thực tế"
                    >
                        "📥 Tải mẫu Demo"
                    </button>

                    // Nút Xuất CSV / Excel
                    <button
                        type="button"
                        class="pagination-btn"
                        style="background: var(--brand-primary); color: var(--text-white); font-weight: 600; border-color: var(--brand-primary);"
                        on:click=move |_| on_export_csv()
                        title="Xuất bảng dữ liệu hiện tại sang tệp CSV UTF-8 BOM"
                    >
                        "📤 Xuất CSV / Excel"
                    </button>
                </div>
            </header>

            // Time-travel Bar (Undo/Redo & Recipe Engine)
            <div class="grid-top-bar" style="background: var(--bg-surface); border-bottom: 1px solid var(--border-subtle); padding: 4px 12px;">
                <div class="time-travel-bar">
                    <button
                        type="button"
                        class="time-travel-btn"
                        disabled=move || history_index.get() == 0
                        on:click=move |_| on_undo()
                        title="Hoàn tác bước trước"
                    >
                        "↩ Hoàn tác"
                    </button>

                    <button
                        type="button"
                        class="time-travel-btn"
                        disabled=move || history_index.get() >= history_stack.with(|s| s.len().saturating_sub(1))
                        on:click=move |_| on_redo()
                        title="Làm lại bước tiếp theo"
                    >
                        "↪ Làm lại"
                    </button>

                    <span style="font-size: 12px; color: var(--border-subtle);">"|"</span>

                    <span style="font-size: 12px; color: var(--text-muted); font-weight: 500;">
                        {move || {
                            let idx = history_index.get();
                            let total = history_stack.with(|s| s.len());
                            let desc = history_stack.with(|s| s.get(idx).map(|(_, _, step)| step.description.clone()).unwrap_or_default());
                            format!("Bước {}/{}: {}", idx, total.saturating_sub(1), desc)
                        }}
                    </span>
                </div>

                <div style="display: flex; align-items: center; gap: 6px;">
                    <button
                        type="button"
                        class="time-travel-btn"
                        on:click=move |_| is_history_modal_open.set(true)
                        title="Xem toàn bộ lịch sử các bước đã áp dụng"
                    >
                        "📜 Lịch sử thao tác"
                    </button>

                    <button
                        type="button"
                        class="time-travel-btn"
                        on:click=move |_| on_export_recipe()
                        title="Xuất công thức các bước đã thực hiện dạng JSON"
                    >
                        "📋 Xuất Recipe JSON"
                    </button>

                    <button
                        type="button"
                        class="time-travel-btn"
                        on:click=move |_| is_import_modal_open.set(true)
                        title="Nhập chuỗi Recipe JSON để tái áp dụng liên hoàn"
                    >
                        "📥 Nhập Recipe"
                    </button>
                </div>
            </div>

            // Thân chính: Bố cục 2 phần (Trái: Facet/Profiling Sidebar, Phải: Interactive Data Grid)
            <div class="grid-workspace-split">
                // Bên trái: Facet & Profiling Sidebar
                <aside class="facet-sidebar">
                    <div class="facet-header">
                        <span>"🔍 Facets & Bộ lọc"</span>
                        <span style="font-size: 12px; font-weight: normal; color: var(--text-muted);">
                            {move || format!("{} cột", columns.with(|c| c.len()))}
                        </span>
                    </div>

                    <div class="facet-body">
                        // Nhóm các thao tác làm sạch nhanh (Quick Action Buttons)
                        <div>
                            <div class="facet-section-title">"⚡ THAO TÁC LÀM SẠCH NHANH"</div>
                            <div class="facet-actions-grid">
                                <button
                                    type="button"
                                    class="facet-action-btn"
                                    on:click=move |_| on_clean_whitespace()
                                    title="Xóa khoảng trắng đầu cuối và nén nhiều khoảng trắng liên tiếp"
                                >
                                    <span>"✨"</span>
                                    <span>"Làm sạch khoảng trắng"</span>
                                </button>

                                <button
                                    type="button"
                                    class="facet-action-btn"
                                    on:click=move |_| on_normalize_dates()
                                    title="Đưa ngày tháng YYYY-MM-DD, MM/DD/YYYY về chuẩn DD/MM/YYYY"
                                >
                                    <span>"📅"</span>
                                    <span>"Chuẩn hóa ngày (DD/MM/YYYY)"</span>
                                </button>

                                <button
                                    type="button"
                                    class="facet-action-btn"
                                    on:click=move |_| on_split_names()
                                    title="Tách cột Họ và tên thành Họ, Tên đệm, Tên nhận diện họ kép"
                                >
                                    <span>"🇻🇳"</span>
                                    <span>"Tách họ tên tiếng Việt"</span>
                                </button>

                                <button
                                    type="button"
                                    class="facet-action-btn"
                                    on:click=move |_| is_fuzzy_modal_open.set(true)
                                    title="Gom cụm các biến thể chính tả, gõ dấu hoặc không dấu (Cluster & Edit)"
                                >
                                    <span>"🔍"</span>
                                    <span>"Gom cụm Fuzzy (Cluster & Edit)"</span>
                                </button>
                            </div>
                        </div>

                        // Danh sách các cột để chọn phân tích Facet
                        <div>
                            <div class="facet-section-title">"📋 CHỌN CỘT PHÂN TÍCH"</div>
                            <div>
                                {move || {
                                    let cur_sel = selected_col_idx.get();
                                    columns.get().into_iter().enumerate().map(|(idx, col)| {
                                        let is_active = idx == cur_sel;
                                        view! {
                                            <button
                                                type="button"
                                                class=if is_active { "facet-col-btn active" } else { "facet-col-btn" }
                                                on:click=move |_| {
                                                    selected_col_idx.set(idx);
                                                    facet_search.set(String::new());
                                                }
                                            >
                                                <div style="display: flex; align-items: center; gap: 6px;">
                                                    <span class="col-type-badge">{col.col_type.icon()}</span>
                                                    <span style="font-weight: 600;">{col.name}</span>
                                                </div>
                                                <span style="font-size: 12px; color: var(--text-muted);">
                                                    {move || {
                                                        let prof = calculate_column_profile(idx, &rows.get());
                                                        format!("{}/{}", prof.valid_count, prof.total)
                                                    }}
                                                </span>
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()
                                }}
                            </div>
                        </div>

                        // Bảng phân phối tần suất các giá trị của cột đang chọn
                        <div>
                            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px;">
                                <div class="facet-section-title" style="margin-bottom: 0;">
                                    "📊 PHÂN PHỐI TẦN SUẤT"
                                </div>
                                {move || {
                                    if active_filter.get().is_some() {
                                        view! {
                                            <button
                                                type="button"
                                                class="facet-clear-filter-btn"
                                                on:click=move |_| active_filter.set(None)
                                            >
                                                "✕ Bỏ lọc"
                                            </button>
                                        }.into_any()
                                    } else {
                                        view! { <span></span> }.into_any()
                                    }
                                }}
                            </div>

                            // Banner hiển thị trạng thái đang lọc (nếu có)
                            {move || {
                                if let Some((f_col, f_val)) = active_filter.get() {
                                    let col_name = columns.with(|c| c.get(f_col).map(|col| col.name.clone()).unwrap_or_default());
                                    view! {
                                        <div class="facet-filter-chip">
                                            <span>{format!("Đang lọc: {} = \"{}\"", col_name, f_val)}</span>
                                            <button
                                                type="button"
                                                class="facet-clear-filter-btn"
                                                on:click=move |_| active_filter.set(None)
                                            >
                                                "✕"
                                            </button>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }
                            }}

                            // Ô tìm kiếm giá trị trong Facet (IME-Safe)
                            <div style="margin-bottom: 8px;">
                                <ImeSafeSearchInput
                                    value=facet_search
                                    placeholder="Tìm giá trị trong cột..."
                                    class="facet-search-box"
                                />
                            </div>

                            // Danh sách các giá trị phân phối tần suất
                            <div class="facet-dist-list">
                                {move || {
                                    let idx = selected_col_idx.get();
                                    let profile = calculate_column_profile(idx, &rows.get());
                                    let search = facet_search.get().trim().to_lowercase();

                                    let filtered_top: Vec<_> = profile.top_values.into_iter()
                                        .filter(|(val, _, _)| search.is_empty() || val.to_lowercase().contains(&search))
                                        .collect();

                                    if filtered_top.is_empty() {
                                        view! {
                                            <div style="font-size: 12px; color: var(--text-muted); text-align: center; padding: 12px;">
                                                "Không có giá trị nào phù hợp."
                                            </div>
                                        }.into_any()
                                    } else {
                                        let max_count = filtered_top.first().map(|(_, c, _)| *c).unwrap_or(1).max(1);
                                        filtered_top.into_iter().map(|(val, count, pct)| {
                                            let val_for_filter = val.clone();
                                            let bar_width = format!("{:.1}%", (count as f32 / max_count as f32) * 100.0);
                                            let is_currently_filtered = active_filter.get().map(|(c, v)| c == idx && v == val).unwrap_or(false);

                                            view! {
                                                <div
                                                    class="facet-dist-item"
                                                    style=if is_currently_filtered { "border: 1px solid var(--brand-primary); cursor: pointer;" } else { "cursor: pointer;" }
                                                    on:click=move |_| {
                                                        if is_currently_filtered {
                                                            active_filter.set(None);
                                                        } else {
                                                            active_filter.set(Some((idx, val_for_filter.clone())));
                                                            current_page.set(1);
                                                        }
                                                    }
                                                    title=format!("Nhấp để lọc dòng theo \"{}\"", val)
                                                >
                                                    <div class="facet-dist-fill" style=format!("width: {};", bar_width)></div>
                                                    <span class="facet-dist-val">{val.clone()}</span>
                                                    <span class="facet-dist-count">
                                                        {format!("{} ({:.1}%)", count, pct)}
                                                    </span>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>().into_any()
                                    }
                                }}
                            </div>
                        </div>
                    </div>
                </aside>

                // Bên phải: Interactive Data Grid
                <main class="data-grid-main">
                    <div class="data-grid-table-wrap">
                        <table class="data-grid-table">
                            <thead>
                                <tr>
                                    <th style="width: 44px; text-align: center;">"#"</th>
                                    {move || {
                                        let cur_rows = rows.get();
                                        columns.get().into_iter().enumerate().map(|(col_idx, col)| {
                                            let profile = calculate_column_profile(col_idx, &cur_rows);
                                            let valid_w = format!("{:.1}%", profile.valid_percentage);
                                            let tooltip = format!(
                                                "Hợp lệ: {:.1}% ({}/{}) | Trống/Null: {:.1}% ({}/{})",
                                                profile.valid_percentage, profile.valid_count, profile.total,
                                                profile.null_percentage, profile.null_count, profile.total
                                            );

                                            view! {
                                                <th>
                                                    <div class="col-header-wrap">
                                                        <div class="col-header-top">
                                                            <div style="display: flex; align-items: center; gap: 6px;">
                                                                <span class="col-type-badge">{col.col_type.icon()}</span>
                                                                <span>{col.name}</span>
                                                            </div>
                                                        </div>
                                                        // Mini-profiling Bar dưới mỗi tiêu đề cột
                                                        <div class="col-profiling-bar" title=tooltip>
                                                            <div class="profile-valid-fill" style=format!("width: {};", valid_w)></div>
                                                        </div>
                                                    </div>
                                                </th>
                                            }
                                        }).collect::<Vec<_>>()
                                    }}
                                </tr>
                            </thead>
                            <tbody>
                                {move || {
                                    let all_rows = rows.get();
                                    let filter = active_filter.get();
                                    let p_size = page_size.get();
                                    let cur_p = current_page.get();

                                    // Lọc dòng nếu active_filter có giá trị
                                    let indexed_rows: Vec<(usize, Vec<String>)> = all_rows
                                        .into_iter()
                                        .enumerate()
                                        .filter(|(_, r)| {
                                            if let Some((f_col, ref f_val)) = filter {
                                                r.get(f_col).map(|cell| cell.trim() == f_val.as_str()).unwrap_or(false)
                                            } else {
                                                true
                                            }
                                        })
                                        .collect();

                                    let total_filtered = indexed_rows.len();
                                    let start_idx = (cur_p.saturating_sub(1)) * p_size;
                                    let page_rows = indexed_rows.into_iter().skip(start_idx).take(p_size);

                                    if total_filtered == 0 {
                                        view! {
                                            <tr>
                                                <td colspan="10" style="text-align: center; padding: 32px; color: var(--text-muted);">
                                                    "Không có dòng dữ liệu nào khớp với bộ lọc."
                                                </td>
                                            </tr>
                                        }.into_any()
                                    } else {
                                        page_rows.map(|(orig_idx, row)| {
                                            let is_highlighted = filter.is_some();
                                            view! {
                                                <tr class=if is_highlighted { "grid-row-highlight" } else { "" }>
                                                    <td style="text-align: center; color: var(--text-muted); font-size: 12px;">
                                                        {orig_idx + 1}
                                                    </td>
                                                    {row.into_iter().map(|cell| {
                                                        let is_empty = cell.trim().is_empty();
                                                        view! {
                                                            <td>
                                                                {if is_empty {
                                                                    view! {
                                                                        <span class="cell-empty-val">"(trống)"</span>
                                                                    }.into_any()
                                                                } else {
                                                                    view! {
                                                                        <span>{cell}</span>
                                                                    }.into_any()
                                                                }}
                                                            </td>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </tr>
                                            }
                                        }).collect::<Vec<_>>().into_any()
                                    }
                                }}
                            </tbody>
                        </table>
                    </div>

                    // Thanh phân trang ở chân bảng (Pagination Bar)
                    <footer class="grid-pagination-bar">
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="color: var(--text-muted);">
                                {move || {
                                    let all_len = rows.with(|r| r.len());
                                    if let Some((f_col, ref f_val)) = active_filter.get() {
                                        let col_name = columns.with(|c| c.get(f_col).map(|col| col.name.clone()).unwrap_or_default());
                                        let match_count = rows.with(|r| {
                                            r.iter().filter(|row| row.get(f_col).map(|c| c.trim() == f_val.as_str()).unwrap_or(false)).count()
                                        });
                                        format!("Hiển thị {}/{} dòng (Lọc theo {}: \"{}\")", match_count, all_len, col_name, f_val)
                                    } else {
                                        format!("Tổng cộng {} dòng dữ liệu", all_len)
                                    }
                                }}
                            </span>
                        </div>

                        <div style="display: flex; align-items: center; gap: 8px;">
                            // Chọn số dòng mỗi trang
                            <label style="display: flex; align-items: center; gap: 4px; color: var(--text-muted);">
                                <span>"Số dòng:"</span>
                                <select
                                    class="pagination-btn"
                                    prop:value=move || page_size.get().to_string()
                                    on:change=move |ev| {
                                        if let Ok(sz) = event_target_value(&ev).parse::<usize>() {
                                            page_size.set(sz);
                                            current_page.set(1);
                                        }
                                    }
                                >
                                    <option value="10">"10"</option>
                                    <option value="15">"15"</option>
                                    <option value="25">"25"</option>
                                    <option value="50">"50"</option>
                                </select>
                            </label>

                            // Nút chuyển trang Trước / Sau
                            <button
                                type="button"
                                class="pagination-btn"
                                disabled=move || current_page.get() <= 1
                                on:click=move |_| current_page.update(|p| *p = p.saturating_sub(1))
                            >
                                "◀ Trước"
                            </button>

                            <span style="font-weight: 600; color: var(--text-main);">
                                {move || {
                                    let filter = active_filter.get();
                                    let filtered_len = rows.with(|r| {
                                        if let Some((f_col, ref f_val)) = filter {
                                            r.iter().filter(|row| row.get(f_col).map(|c| c.trim() == f_val.as_str()).unwrap_or(false)).count()
                                        } else {
                                            r.len()
                                        }
                                    });
                                    let total_pages = (filtered_len + page_size.get().saturating_sub(1)) / page_size.get().max(1);
                                    format!("Trang {} / {}", current_page.get(), total_pages.max(1))
                                }}
                            </span>

                            <button
                                type="button"
                                class="pagination-btn"
                                disabled=move || {
                                    let filter = active_filter.get();
                                    let filtered_len = rows.with(|r| {
                                        if let Some((f_col, ref f_val)) = filter {
                                            r.iter().filter(|row| row.get(f_col).map(|c| c.trim() == f_val.as_str()).unwrap_or(false)).count()
                                        } else {
                                            r.len()
                                        }
                                    });
                                    let total_pages = (filtered_len + page_size.get().saturating_sub(1)) / page_size.get().max(1);
                                    current_page.get() >= total_pages.max(1)
                                }
                                on:click=move |_| current_page.update(|p| *p += 1)
                            >
                                "Sau ▶"
                            </button>
                        </div>
                    </footer>
                </main>
            </div>

            // MODAL 1: Gom cụm Fuzzy (Cluster & Edit)
            {move || {
                if is_fuzzy_modal_open.get() {
                    let col_idx = selected_col_idx.get();
                    let cur_cols = columns.get();
                    let col_name = cur_cols.get(col_idx).map(|c| c.name.clone()).unwrap_or_else(|| "Cột".to_string());
                    let clusters = detect_fuzzy_clusters(col_idx, &rows.get());

                    view! {
                        <div class="fuzzy-modal-backdrop">
                            <div class="fuzzy-modal-card">
                                <div class="fuzzy-modal-header">
                                    <div style="display: flex; align-items: center; gap: 8px;">
                                        <span>"🔍"</span>
                                        <span>{format!("Gom cụm & Chuẩn hóa Chính tả — Cột \"{}\"", col_name)}</span>
                                    </div>
                                    <button
                                        type="button"
                                        class="pagination-btn"
                                        on:click=move |_| is_fuzzy_modal_open.set(false)
                                    >
                                        "✕"
                                    </button>
                                </div>

                                <div class="fuzzy-modal-body">
                                    <p style="font-size: 12px; color: var(--text-muted); margin: 0 0 10px 0;">
                                        "Thuật toán phát hiện các giá trị có lỗi chính tả, sai dấu hoặc biến thể không dấu tương đồng trong tiếng Việt và đề xuất giá trị chuẩn để hợp nhất."
                                    </p>

                                    {if clusters.is_empty() {
                                        view! {
                                            <div style="padding: 24px; text-align: center; color: var(--text-muted); font-size: 12px;">
                                                "Không phát hiện biến thể chính tả nào trong cột này."
                                            </div>
                                        }.into_any()
                                    } else {
                                        clusters.into_iter().map(|cluster| {
                                            view! {
                                                <div class="fuzzy-cluster-card">
                                                    <div class="fuzzy-cluster-title">
                                                        {format!("Cụm chuẩn: \"{}\" (Tổng {} dòng)", cluster.canonical, cluster.total_occurrences)}
                                                    </div>
                                                    <div style="font-size: 12px; color: var(--text-muted); margin-bottom: 6px;">
                                                        "Các biến thể tương đồng phát hiện được:"
                                                    </div>
                                                    <div class="fuzzy-cluster-items">
                                                        {cluster.variations.into_iter().map(|(val, count)| {
                                                            view! {
                                                                <span class="fuzzy-cluster-item-tag">
                                                                    {format!("{} ({})", val, count)}
                                                                </span>
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>().into_any()
                                    }}
                                </div>

                                <div class="fuzzy-modal-footer">
                                    <button
                                        type="button"
                                        class="pagination-btn"
                                        on:click=move |_| is_fuzzy_modal_open.set(false)
                                    >
                                        "Hủy bỏ"
                                    </button>
                                    <button
                                        type="button"
                                        class="pagination-btn"
                                        style="background: var(--brand-primary); color: var(--text-white); font-weight: 600;"
                                        on:click=move |_| on_apply_fuzzy_merge()
                                    >
                                        "✓ Áp dụng gom cụm (Merge All)"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}

            // MODAL 2: Xem toàn bộ Lịch sử các bước Recipe
            {move || {
                if is_history_modal_open.get() {
                    let steps: Vec<RecipeStep> = history_stack.with(|s| {
                        s.iter().map(|(_, _, step)| step.clone()).collect()
                    });

                    view! {
                        <div class="fuzzy-modal-backdrop">
                            <div class="recipe-history-modal-card">
                                <div class="fuzzy-modal-header">
                                    <span>"📜 Lịch sử thao tác (Recipe History)"</span>
                                    <button
                                        type="button"
                                        class="pagination-btn"
                                        on:click=move |_| is_history_modal_open.set(false)
                                    >
                                        "✕"
                                    </button>
                                </div>

                                <div class="fuzzy-modal-body">
                                    {steps.into_iter().enumerate().map(|(idx, step)| {
                                        let cur_active = history_index.get() == idx;
                                        view! {
                                            <div
                                                class="recipe-step-item"
                                                style=if cur_active { "border-color: var(--brand-primary); background: rgba(56, 189, 248, 0.08);" } else { "" }
                                            >
                                                <div class="recipe-step-badge">{idx}</div>
                                                <div style="flex: 1;">
                                                    <div style="font-weight: 600; color: var(--text-main);">
                                                        {step.description}
                                                    </div>
                                                    <div style="font-size: 12px; color: var(--text-muted); margin-top: 2px;">
                                                        {format!("Hành động: {} • ID: {}", step.action, step.id)}
                                                    </div>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>

                                <div class="fuzzy-modal-footer">
                                    <button
                                        type="button"
                                        class="pagination-btn"
                                        on:click=move |_| is_history_modal_open.set(false)
                                    >
                                        "Đóng"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}

            // MODAL 3: Nhập Recipe JSON
            {move || {
                if is_import_modal_open.get() {
                    view! {
                        <div class="fuzzy-modal-backdrop">
                            <div class="fuzzy-modal-card">
                                <div class="fuzzy-modal-header">
                                    <span>"📥 Nhập Recipe JSON để Tái áp dụng"</span>
                                    <button
                                        type="button"
                                        class="pagination-btn"
                                        on:click=move |_| is_import_modal_open.set(false)
                                    >
                                        "✕"
                                    </button>
                                </div>

                                <div class="fuzzy-modal-body">
                                    <p style="font-size: 12px; color: var(--text-muted); margin: 0 0 8px 0;">
                                        "Dán chuỗi Recipe JSON chứa các bước làm sạch dữ liệu để thực thi liên hoàn:"
                                    </p>
                                    <textarea
                                        class="facet-search-box"
                                        style="height: 180px; font-family: monospace; font-size: 12px; resize: vertical;"
                                        placeholder="[{\"id\": \"step-1\", \"action\": \"trim_spaces\", ...}]"
                                        prop:value=move || import_recipe_text.get()
                                        on:input=move |ev| import_recipe_text.set(event_target_value(&ev))
                                    ></textarea>
                                </div>

                                <div class="fuzzy-modal-footer">
                                    <button
                                        type="button"
                                        class="pagination-btn"
                                        on:click=move |_| is_import_modal_open.set(false)
                                    >
                                        "Hủy"
                                    </button>
                                    <button
                                        type="button"
                                        class="pagination-btn"
                                        style="background: var(--brand-primary); color: var(--text-white); font-weight: 600;"
                                        on:click=move |_| on_execute_import_recipe()
                                    >
                                        "✓ Thực thi Recipe"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}

            // Thông báo Toast góc dưới
            {move || {
                if let Some(msg) = toast_message.get() {
                    view! {
                        <div
                            class="toast-notification"
                            on:click=move |_| toast_message.set(None)
                            title="Nhấp để đóng thông báo"
                        >
                            <span>"ℹ️"</span>
                            <span>{msg}</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}
        </div>
    }
}
