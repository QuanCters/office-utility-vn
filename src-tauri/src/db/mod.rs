use std::path::Path;
use rusqlite::{params, Connection};
use office_shared::{
    AppPreferenceRecord, CommandError, TaskHistoryRecord, ToolPreset, ToolSettingRecord,
    WorkflowDag,
};

pub const SCHEMA_SQL: &str = r#"
-- Kích hoạt WAL Mode & Foreign Keys
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

-- 1. Bảng lưu cấu hình và trạng thái từng công cụ
CREATE TABLE IF NOT EXISTS tool_settings (
    slug TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    category TEXT NOT NULL CHECK(category IN ('DOCUMENT', 'MEDIA', 'DATA', 'VN_OFFICE', 'SYSTEM', 'HUB')),
    last_options_json TEXT NOT NULL DEFAULT '{}',
    is_pinned INTEGER NOT NULL DEFAULT 0,
    usage_count INTEGER NOT NULL DEFAULT 0,
    last_used_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_tool_pinned ON tool_settings(is_pinned DESC, usage_count DESC);

-- 2. Bảng lưu nhiều cấu hình mẫu (Named Tool Presets)
CREATE TABLE IF NOT EXISTS tool_presets (
    id TEXT PRIMARY KEY NOT NULL,
    tool_slug TEXT NOT NULL REFERENCES tool_settings(slug) ON DELETE CASCADE,
    preset_name TEXT NOT NULL,
    options_json TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(tool_slug, preset_name)
);

CREATE INDEX IF NOT EXISTS idx_presets_tool ON tool_presets(tool_slug, is_default DESC);

-- 3. Bảng lưu các chuỗi tự động hóa Workflow (Macro DAG)
CREATE TABLE IF NOT EXISTS workflows (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    dag_definition_json TEXT NOT NULL,
    is_favorite INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_workflow_favorite ON workflows(is_favorite DESC, updated_at DESC);

-- 4. Bảng lưu nhật ký tác vụ cục bộ (Task History)
CREATE TABLE IF NOT EXISTS task_history (
    id TEXT PRIMARY KEY NOT NULL,
    parent_task_id TEXT REFERENCES task_history(id) ON DELETE CASCADE,
    tool_slug TEXT REFERENCES tool_settings(slug) ON DELETE CASCADE,
    workflow_id TEXT REFERENCES workflows(id) ON DELETE SET NULL,
    input_files_json TEXT NOT NULL,
    output_files_json TEXT NOT NULL DEFAULT '[]',
    options_used_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL CHECK(status IN ('SUCCESS', 'PARTIAL_SUCCESS', 'FAILED', 'CANCELLED')),
    elapsed_ms INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (tool_slug IS NOT NULL OR workflow_id IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_history_created ON task_history(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_history_slug ON task_history(tool_slug, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_history_parent ON task_history(parent_task_id);

-- 5. Bảng lưu quy tắc pháp lý & hành chính Việt Nam cập nhật động
CREATE TABLE IF NOT EXISTS legal_rulesets (
    rule_key TEXT PRIMARY KEY NOT NULL,
    version TEXT NOT NULL,
    effective_date TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    is_custom_override INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- 6. Bảng lưu cấu hình tổng quan ứng dụng
CREATE TABLE IF NOT EXISTS app_preferences (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
"#;

/// Khởi tạo SQLite tại đường dẫn app_data_dir/office_utility.db
pub fn init_db(db_path: &Path) -> Result<Connection, CommandError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục lưu CSDL: {e}"),
        })?;
    }

    let conn = Connection::open(db_path).map_err(|e| CommandError::DatabaseError {
        details: format!("Không thể mở file CSDL SQLite: {e}"),
    })?;

    apply_schema(&conn)?;
    seed_default_data(&conn)?;

    Ok(conn)
}

/// Khởi tạo SQLite in-memory (phục vụ Unit Test)
pub fn init_in_memory() -> Result<Connection, CommandError> {
    let conn = Connection::open_in_memory().map_err(|e| CommandError::DatabaseError {
        details: format!("Không thể mở SQLite in-memory: {e}"),
    })?;

    apply_schema(&conn)?;
    seed_default_data(&conn)?;

    Ok(conn)
}

fn apply_schema(conn: &Connection) -> Result<(), CommandError> {
    conn.execute_batch(SCHEMA_SQL)
        .map_err(|e| CommandError::DatabaseError {
            details: format!("Lỗi thực thi DDL Schema: {e}"),
        })?;
    Ok(())
}

/// Nạp trước danh mục công cụ tiêu chuẩn và tham số pháp lý mặc định
pub fn seed_default_data(conn: &Connection) -> Result<(), CommandError> {
    let default_tools = [
        ("pdf-merge-split", "Ghép, Tách & Xoay trang PDF", "DOCUMENT", 1),
        ("pdf-protect-unlock", "Bảo mật & Mở khóa PDF", "DOCUMENT", 0),
        ("pdf-compress-shrink", "Nén giảm Dung lượng PDF", "DOCUMENT", 0),
        ("pdf-watermark-stamp", "Đóng dấu Watermark & Con dấu đỏ", "DOCUMENT", 0),
        ("pdf-metadata-scrub", "Tẩy sạch Metadata ẩn PDF", "DOCUMENT", 0),
        ("image-batch-convert", "Nén & Đổi Định dạng Ảnh Hàng loạt", "MEDIA", 1),
        ("image-exif-stripper", "Kiểm tra & Xóa Exif/GPS Ảnh", "MEDIA", 0),
        ("watermark-batch-studio", "Đóng dấu Bản quyền Ảnh Hàng loạt", "MEDIA", 0),
        ("vietnamese-font-converter", "Chuyển mã Font Tiếng Việt", "DATA", 0),
        ("spreadsheet-cleanse-normalize", "Làm sạch & Chuẩn hóa Bảng tính", "DATA", 0),
        ("tabular-format-converter", "Chuyển đổi Cấu trúc Dữ liệu", "DATA", 0),
        ("vietnamese-number-to-words", "Đọc Số Tiền Thành Chữ Chuẩn", "VN_OFFICE", 1),
        ("salary-gross-net-tax-calc", "Tính Lương Gross - Net & Thuế", "VN_OFFICE", 1),
        ("vietqr-napas-generator", "Tạo Mã VietQR Napas247", "VN_OFFICE", 1),
        ("vn-einvoice-xml-viewer", "Đọc Hóa đơn Điện tử XML VN", "VN_OFFICE", 0),
        ("universal-file-converter-hub", "Trạm Chuyển đổi Tệp Vạn năng", "HUB", 1),
    ];

    for (slug, name, category, is_pinned) in default_tools {
        conn.execute(
            r#"
            INSERT INTO tool_settings (slug, name, category, is_pinned)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(slug) DO UPDATE SET
                name = excluded.name,
                category = excluded.category;
            "#,
            params![slug, name, category, is_pinned],
        )
        .map_err(|e| CommandError::DatabaseError {
            details: format!("Lỗi seed tool {slug}: {e}"),
        })?;
    }

    // Seed thiết lập mặc định (app_preferences)
    conn.execute(
        r#"
        INSERT INTO app_preferences (key, value_json)
        VALUES ('general', '{"theme": "system", "auto_clean_temp": true, "max_ram_mb": 250}')
        ON CONFLICT(key) DO NOTHING;
        "#,
        [],
    )
    .map_err(|e| CommandError::DatabaseError {
        details: format!("Lỗi seed app_preferences: {e}"),
    })?;

    Ok(())
}

// ============================================================================
// CÁC HÀM TRUY VẤN DỮ LIỆU (QUERIES & MUTATIONS)
// ============================================================================

pub fn get_all_tool_settings(conn: &Connection) -> Result<Vec<ToolSettingRecord>, CommandError> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT slug, name, category, last_options_json, is_pinned, usage_count, last_used_at
            FROM tool_settings
            ORDER BY is_pinned DESC, usage_count DESC, name ASC;
            "#,
        )
        .map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ToolSettingRecord {
                slug: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                last_options_json: row.get(3)?,
                is_pinned: row.get::<_, i32>(4)? == 1,
                usage_count: row.get::<_, u32>(5)?,
                last_used_at: row.get(6)?,
            })
        })
        .map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?);
    }
    Ok(result)
}

pub fn set_tool_pinned(
    conn: &Connection,
    slug: &str,
    is_pinned: bool,
) -> Result<(), CommandError> {
    let pinned_val = if is_pinned { 1 } else { 0 };
    let affected = conn
        .execute(
            "UPDATE tool_settings SET is_pinned = ?1 WHERE slug = ?2;",
            params![pinned_val, slug],
        )
        .map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;

    if affected == 0 {
        return Err(CommandError::ValidationFailed {
            reason: format!("Không tìm thấy công cụ '{slug}'"),
        });
    }
    Ok(())
}

pub fn record_tool_usage(
    conn: &Connection,
    slug: &str,
    last_options_json: &str,
) -> Result<(), CommandError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        r#"
        UPDATE tool_settings
        SET usage_count = usage_count + 1,
            last_used_at = ?1,
            last_options_json = ?2
        WHERE slug = ?3;
        "#,
        params![now, last_options_json, slug],
    )
    .map_err(|e| CommandError::DatabaseError {
        details: e.to_string(),
    })?;
    Ok(())
}

pub fn get_tool_presets(
    conn: &Connection,
    tool_slug: &str,
) -> Result<Vec<ToolPreset>, CommandError> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, tool_slug, preset_name, options_json, is_default, created_at
            FROM tool_presets
            WHERE tool_slug = ?1
            ORDER BY is_default DESC, created_at DESC;
            "#,
        )
        .map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;

    let rows = stmt
        .query_map(params![tool_slug], |row| {
            Ok(ToolPreset {
                id: row.get(0)?,
                tool_slug: row.get(1)?,
                preset_name: row.get(2)?,
                options_json: row.get(3)?,
                is_default: row.get::<_, i32>(4)? == 1,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?);
    }
    Ok(result)
}

pub fn save_tool_preset(conn: &Connection, preset: &ToolPreset) -> Result<String, CommandError> {
    let id = if preset.id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        preset.id.clone()
    };
    let is_default_val = if preset.is_default { 1 } else { 0 };

    conn.execute(
        r#"
        INSERT INTO tool_presets (id, tool_slug, preset_name, options_json, is_default)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(tool_slug, preset_name) DO UPDATE SET
            options_json = excluded.options_json,
            is_default = excluded.is_default;
        "#,
        params![id, preset.tool_slug, preset.preset_name, preset.options_json, is_default_val],
    )
    .map_err(|e| CommandError::DatabaseError {
        details: e.to_string(),
    })?;

    Ok(id)
}

pub fn delete_tool_preset(conn: &Connection, preset_id: &str) -> Result<(), CommandError> {
    conn.execute(
        "DELETE FROM tool_presets WHERE id = ?1;",
        params![preset_id],
    )
    .map_err(|e| CommandError::DatabaseError {
        details: e.to_string(),
    })?;
    Ok(())
}

pub fn save_workflow(conn: &Connection, workflow: &WorkflowDag) -> Result<String, CommandError> {
    let id = if workflow.id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        workflow.id.clone()
    };
    let dag_json = serde_json::to_string(workflow).map_err(|e| CommandError::ValidationFailed {
        reason: format!("Lỗi serialize workflow DAG: {e}"),
    })?;

    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO workflows (id, name, description, dag_definition_json, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?5)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            description = excluded.description,
            dag_definition_json = excluded.dag_definition_json,
            updated_at = excluded.updated_at;
        "#,
        params![id, workflow.name, workflow.description, dag_json, now],
    )
    .map_err(|e| CommandError::DatabaseError {
        details: e.to_string(),
    })?;

    Ok(id)
}

pub fn get_all_workflows(conn: &Connection) -> Result<Vec<WorkflowDag>, CommandError> {
    let mut stmt = conn
        .prepare("SELECT dag_definition_json FROM workflows ORDER BY is_favorite DESC, updated_at DESC;")
        .map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;

    let rows = stmt
        .query_map([], |row| {
            let json_str: String = row.get(0)?;
            Ok(json_str)
        })
        .map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;

    let mut result = Vec::new();
    for r in rows {
        let json_str = r.map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;
        let dag: WorkflowDag = serde_json::from_str(&json_str).map_err(|e| {
            CommandError::DatabaseError {
                details: format!("Không thể parse JSON Workflow: {e}"),
            }
        })?;
        result.push(dag);
    }
    Ok(result)
}

pub fn add_task_history(
    conn: &Connection,
    record: &TaskHistoryRecord,
) -> Result<(), CommandError> {
    let id = if record.id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        record.id.clone()
    };

    let now = if record.created_at.is_empty() {
        chrono::Utc::now().to_rfc3339()
    } else {
        record.created_at.clone()
    };

    conn.execute(
        r#"
        INSERT INTO task_history (
            id, parent_task_id, tool_slug, workflow_id,
            input_files_json, output_files_json, options_used_json,
            status, elapsed_ms, error_message, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11);
        "#,
        params![
            id,
            record.parent_task_id,
            record.tool_slug,
            record.workflow_id,
            record.input_files_json,
            record.output_files_json,
            record.options_used_json,
            record.status,
            record.elapsed_ms as i64,
            record.error_message,
            now
        ],
    )
    .map_err(|e| CommandError::DatabaseError {
        details: e.to_string(),
    })?;

    Ok(())
}

pub fn get_task_history(
    conn: &Connection,
    limit: u32,
    offset: u32,
) -> Result<Vec<TaskHistoryRecord>, CommandError> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, parent_task_id, tool_slug, workflow_id,
                   input_files_json, output_files_json, options_used_json,
                   status, elapsed_ms, error_message, created_at
            FROM task_history
            ORDER BY created_at DESC
            LIMIT ?1 OFFSET ?2;
            "#,
        )
        .map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;

    let rows = stmt
        .query_map(params![limit, offset], |row| {
            Ok(TaskHistoryRecord {
                id: row.get(0)?,
                parent_task_id: row.get(1)?,
                tool_slug: row.get(2)?,
                workflow_id: row.get(3)?,
                input_files_json: row.get(4)?,
                output_files_json: row.get(5)?,
                options_used_json: row.get(6)?,
                status: row.get(7)?,
                elapsed_ms: row.get::<_, i64>(8)? as u64,
                error_message: row.get(9)?,
                created_at: row.get(10)?,
            })
        })
        .map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?);
    }
    Ok(result)
}

pub fn clear_task_history(conn: &Connection) -> Result<(), CommandError> {
    conn.execute("DELETE FROM task_history;", [])
        .map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;
    Ok(())
}

pub fn get_app_preference(
    conn: &Connection,
    key: &str,
) -> Result<Option<AppPreferenceRecord>, CommandError> {
    let mut stmt = conn
        .prepare("SELECT key, value_json, updated_at FROM app_preferences WHERE key = ?1;")
        .map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;

    let mut rows = stmt
        .query(params![key])
        .map_err(|e| CommandError::DatabaseError {
            details: e.to_string(),
        })?;

    if let Some(row) = rows.next().map_err(|e| CommandError::DatabaseError {
        details: e.to_string(),
    })? {
        Ok(Some(AppPreferenceRecord {
            key: row.get(0).map_err(|e| CommandError::DatabaseError { details: e.to_string() })?,
            value_json: row.get(1).map_err(|e| CommandError::DatabaseError { details: e.to_string() })?,
            updated_at: row.get(2).map_err(|e| CommandError::DatabaseError { details: e.to_string() })?,
        }))
    } else {
        Ok(None)
    }
}

pub fn set_app_preference(
    conn: &Connection,
    key: &str,
    value_json: &str,
) -> Result<(), CommandError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        r#"
        INSERT INTO app_preferences (key, value_json, updated_at)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(key) DO UPDATE SET
            value_json = excluded.value_json,
            updated_at = excluded.updated_at;
        "#,
        params![key, value_json, now],
    )
    .map_err(|e| CommandError::DatabaseError {
        details: e.to_string(),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_wal_and_schema_initialization() {
        let conn = init_in_memory().expect("Khởi tạo SQLite in-memory thất bại");

        // Kiểm tra chế độ WAL hoặc foreign keys
        let fk_enabled: i32 = conn
            .query_row("PRAGMA foreign_keys;", [], |r| r.get(0))
            .expect("Lỗi query PRAGMA");
        assert_eq!(fk_enabled, 1, "Foreign keys phải được bật");

        // Kiểm tra 4 bảng chuẩn + presets
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        assert!(tables.contains(&"tool_settings".to_string()));
        assert!(tables.contains(&"tool_presets".to_string()));
        assert!(tables.contains(&"workflows".to_string()));
        assert!(tables.contains(&"task_history".to_string()));
        assert!(tables.contains(&"app_preferences".to_string()));
    }

    #[test]
    fn test_tool_settings_and_pinning() {
        let conn = init_in_memory().unwrap();
        let tools = get_all_tool_settings(&conn).unwrap();
        assert!(!tools.is_empty(), "Phải seed sẵn các công cụ mặc định");

        let pdf_tool = tools.iter().find(|t| t.slug == "pdf-merge-split").unwrap();
        assert!(pdf_tool.is_pinned);

        // Đổi trạng thái ghim
        set_tool_pinned(&conn, "pdf-merge-split", false).unwrap();
        let tools_after = get_all_tool_settings(&conn).unwrap();
        let pdf_after = tools_after.iter().find(|t| t.slug == "pdf-merge-split").unwrap();
        assert!(!pdf_after.is_pinned);
    }

    #[test]
    fn test_task_history_crud() {
        let conn = init_in_memory().unwrap();

        let record = TaskHistoryRecord {
            id: "task-001".to_string(),
            parent_task_id: None,
            tool_slug: Some("vietnamese-number-to-words".to_string()),
            workflow_id: None,
            input_files_json: "[]".to_string(),
            output_files_json: "[\"Một trăm nghìn đồng\"]".to_string(),
            options_used_json: "{}".to_string(),
            status: "SUCCESS".to_string(),
            elapsed_ms: 45,
            error_message: None,
            created_at: "".to_string(),
        };

        add_task_history(&conn, &record).unwrap();

        let history = get_task_history(&conn, 10, 0).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "task-001");
        assert_eq!(history[0].status, "SUCCESS");

        clear_task_history(&conn).unwrap();
        let empty_history = get_task_history(&conn, 10, 0).unwrap();
        assert!(empty_history.is_empty());
    }

    #[test]
    fn test_presets_and_workflows_crud() {
        let conn = init_in_memory().unwrap();

        let preset = ToolPreset {
            id: "".to_string(),
            tool_slug: "vietnamese-number-to-words".to_string(),
            preset_name: "Chuẩn kế toán MISA".to_string(),
            options_json: "{\"dialect\": \"North\"}".to_string(),
            is_default: true,
            created_at: "".to_string(),
        };

        let preset_id = save_tool_preset(&conn, &preset).unwrap();
        assert!(!preset_id.is_empty());

        let presets = get_tool_presets(&conn, "vietnamese-number-to-words").unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].preset_name, "Chuẩn kế toán MISA");

        // Workflow DAG
        let workflow = WorkflowDag {
            id: "".to_string(),
            name: "Chuỗi xử lý văn phòng".to_string(),
            description: Some("Quy trình tự động hóa mẫu".to_string()),
            nodes: vec![],
            edges: vec![],
        };

        let wf_id = save_workflow(&conn, &workflow).unwrap();
        assert!(!wf_id.is_empty());

        let workflows = get_all_workflows(&conn).unwrap();
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].name, "Chuỗi xử lý văn phòng");
    }
}
