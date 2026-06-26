//! Edit command handler - text-replace with tree-sitter validation

use std::path::PathBuf;

use splice::cli::Language;
use splice::SpliceError;

/// Execute edit command with text-replace and validation
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
pub(crate) fn execute_edit(
    file_path: PathBuf,
    old_text: String,
    new_text: String,
    _language: Option<Language>,
    preview: bool,
    create_backup: bool,
    _operation_id: Option<String>,
    _metadata: Option<String>,
    db: Option<PathBuf>,
    unified: usize,
) -> Result<splice::cli::CliSuccessPayload, SpliceError> {
    // Auto-discover database if needed for logging
    let db_path = if let Some(db) = db {
        db
    } else {
        splice::graph::discover_db_path(None)?.path
    };

    // Execute the edit using the text_replace module
    let result = splice::patch::text_replace::edit_file(
        &file_path,
        &old_text,
        &new_text,
        Some(&db_path),
        preview,
        create_backup,
        unified,
    );

    match result {
        Ok(()) => Ok(splice::cli::CliSuccessPayload {
            status: "success",
            message: if preview {
                format!("Preview of changes to {}", file_path.display())
            } else {
                format!("Successfully edited {}", file_path.display())
            },
            data: Some(serde_json::json!({
                "file": file_path.display().to_string(),
                "operation": if preview { "edit_preview" } else { "edit" },
                "preview": preview
            })),
            already_emitted: false,
            has_pending_changes: preview,
        }),
        Err(e) => Err(e),
    }
}
