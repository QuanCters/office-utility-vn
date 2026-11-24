use serde::{Deserialize, Serialize};

/// DTO biểu diễn NativeSaveGrant được tuần tự hóa từ Native Backend
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeSaveGrantDto {
    #[serde(alias = "grantId")]
    pub grant_id: String,
    #[serde(alias = "documentId")]
    pub document_id: String,
    #[serde(alias = "appInstanceId")]
    pub app_instance_id: String,
    #[serde(alias = "sessionGeneration")]
    pub session_generation: u64,
    #[serde(alias = "authorizedDestPath")]
    pub authorized_dest_path: String,
    #[serde(alias = "expectedRevision")]
    pub expected_revision: u64,
    #[serde(alias = "requestId")]
    pub request_id: String,
}

/// DTO biểu diễn SaveAsHandle được tuần tự hóa từ native_dialog_save_as
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveAsHandleDto {
    #[serde(alias = "documentId")]
    pub document_id: String,
    pub grant: NativeSaveGrantDto,
    #[serde(alias = "fileName")]
    pub file_name: String,
    #[serde(alias = "filePath")]
    pub file_path: String,
}

/// Ngữ cảnh lưu tài liệu được chụp trước khi yêu cầu grant
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveRequestContext {
    pub document_id: String,
    pub app_instance_id: String,
    pub session_generation: u64,
    pub authorized_dest_path: Option<String>,
    pub expected_revision: u64,
    pub request_id: String,
}

/// Thẩm định nghiêm ngặt tính toàn vẹn của Save Grant trước khi ghi đĩa
pub fn validate_native_save_grant(
    grant: &NativeSaveGrantDto,
    ctx: &SaveRequestContext,
) -> Result<(), String> {
    if grant.request_id != ctx.request_id {
        return Err(format!(
            "Grant request_id '{}' không khớp với context request_id '{}'",
            grant.request_id, ctx.request_id
        ));
    }
    if grant.document_id != ctx.document_id {
        return Err(format!(
            "Grant document_id '{}' không khớp với context document_id '{}'",
            grant.document_id, ctx.document_id
        ));
    }
    if grant.app_instance_id != ctx.app_instance_id {
        return Err(format!(
            "Grant app_instance_id '{}' không khớp với context app_instance_id '{}'",
            grant.app_instance_id, ctx.app_instance_id
        ));
    }
    if grant.session_generation != ctx.session_generation {
        return Err(format!(
            "Grant session_generation ({}) không khớp với context session_generation ({})",
            grant.session_generation, ctx.session_generation
        ));
    }
    if grant.expected_revision != ctx.expected_revision {
        return Err(format!(
            "Grant expected_revision ({}) không khớp với context expected_revision ({})",
            grant.expected_revision, ctx.expected_revision
        ));
    }
    if let Some(ref expected_dest) = ctx.authorized_dest_path {
        let grant_norm = grant.authorized_dest_path.replace('\\', "/");
        let ctx_norm = expected_dest.replace('\\', "/");
        if grant_norm != ctx_norm {
            return Err(format!(
                "Grant authorized_dest_path '{}' không khớp với context expected destination '{}'",
                grant.authorized_dest_path, expected_dest
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_save_grant_passes() {
        let grant = NativeSaveGrantDto {
            grant_id: "grant-123".to_string(),
            document_id: "doc-abc".to_string(),
            app_instance_id: "app-inst-1".to_string(),
            session_generation: 1,
            authorized_dest_path: "/data/docs/report.xlsx".to_string(),
            expected_revision: 2,
            request_id: "req-999".to_string(),
        };

        let ctx = SaveRequestContext {
            document_id: "doc-abc".to_string(),
            app_instance_id: "app-inst-1".to_string(),
            session_generation: 1,
            authorized_dest_path: Some("/data/docs/report.xlsx".to_string()),
            expected_revision: 2,
            request_id: "req-999".to_string(),
        };

        assert!(validate_native_save_grant(&grant, &ctx).is_ok());
    }

    #[test]
    fn test_mismatched_request_id_fails() {
        let grant = NativeSaveGrantDto {
            grant_id: "grant-123".to_string(),
            document_id: "doc-abc".to_string(),
            app_instance_id: "app-inst-1".to_string(),
            session_generation: 1,
            authorized_dest_path: "/data/docs/report.xlsx".to_string(),
            expected_revision: 2,
            request_id: "req-DIFFERENT".to_string(),
        };

        let ctx = SaveRequestContext {
            document_id: "doc-abc".to_string(),
            app_instance_id: "app-inst-1".to_string(),
            session_generation: 1,
            authorized_dest_path: Some("/data/docs/report.xlsx".to_string()),
            expected_revision: 2,
            request_id: "req-999".to_string(),
        };

        let err = validate_native_save_grant(&grant, &ctx).unwrap_err();
        assert!(err.contains("request_id"));
    }

    #[test]
    fn test_mismatched_document_id_fails() {
        let grant = NativeSaveGrantDto {
            grant_id: "grant-123".to_string(),
            document_id: "doc-WRONG".to_string(),
            app_instance_id: "app-inst-1".to_string(),
            session_generation: 1,
            authorized_dest_path: "/data/docs/report.xlsx".to_string(),
            expected_revision: 2,
            request_id: "req-999".to_string(),
        };

        let ctx = SaveRequestContext {
            document_id: "doc-abc".to_string(),
            app_instance_id: "app-inst-1".to_string(),
            session_generation: 1,
            authorized_dest_path: Some("/data/docs/report.xlsx".to_string()),
            expected_revision: 2,
            request_id: "req-999".to_string(),
        };

        let err = validate_native_save_grant(&grant, &ctx).unwrap_err();
        assert!(err.contains("document_id"));
    }

    #[test]
    fn test_mismatched_destination_path_fails() {
        let grant = NativeSaveGrantDto {
            grant_id: "grant-123".to_string(),
            document_id: "doc-abc".to_string(),
            app_instance_id: "app-inst-1".to_string(),
            session_generation: 1,
            authorized_dest_path: "/data/docs/other_file.xlsx".to_string(),
            expected_revision: 2,
            request_id: "req-999".to_string(),
        };

        let ctx = SaveRequestContext {
            document_id: "doc-abc".to_string(),
            app_instance_id: "app-inst-1".to_string(),
            session_generation: 1,
            authorized_dest_path: Some("/data/docs/report.xlsx".to_string()),
            expected_revision: 2,
            request_id: "req-999".to_string(),
        };

        let err = validate_native_save_grant(&grant, &ctx).unwrap_err();
        assert!(err.contains("authorized_dest_path"));
    }

    #[test]
    fn test_mismatched_session_generation_fails() {
        let grant = NativeSaveGrantDto {
            grant_id: "grant-123".to_string(),
            document_id: "doc-abc".to_string(),
            app_instance_id: "app-inst-1".to_string(),
            session_generation: 99,
            authorized_dest_path: "/data/docs/report.xlsx".to_string(),
            expected_revision: 2,
            request_id: "req-999".to_string(),
        };

        let ctx = SaveRequestContext {
            document_id: "doc-abc".to_string(),
            app_instance_id: "app-inst-1".to_string(),
            session_generation: 1,
            authorized_dest_path: Some("/data/docs/report.xlsx".to_string()),
            expected_revision: 2,
            request_id: "req-999".to_string(),
        };

        let err = validate_native_save_grant(&grant, &ctx).unwrap_err();
        assert!(err.contains("session_generation"));
    }

    #[test]
    fn test_mismatched_expected_revision_fails() {
        let grant = NativeSaveGrantDto {
            grant_id: "grant-123".to_string(),
            document_id: "doc-abc".to_string(),
            app_instance_id: "app-inst-1".to_string(),
            session_generation: 1,
            authorized_dest_path: "/data/docs/report.xlsx".to_string(),
            expected_revision: 1,
            request_id: "req-999".to_string(),
        };

        let ctx = SaveRequestContext {
            document_id: "doc-abc".to_string(),
            app_instance_id: "app-inst-1".to_string(),
            session_generation: 1,
            authorized_dest_path: Some("/data/docs/report.xlsx".to_string()),
            expected_revision: 2,
            request_id: "req-999".to_string(),
        };

        let err = validate_native_save_grant(&grant, &ctx).unwrap_err();
        assert!(err.contains("expected_revision"));
    }
}
