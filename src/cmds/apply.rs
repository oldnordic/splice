//! Apply-files command handler.

use std::env;

use serde_json::{json, Value};

use super::helpers::log_execution_error;

#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
#[allow(unused_variables, reason = "stub args reserved for future expansion")]
pub(crate) fn execute_apply_files(
    glob_pattern: &str,
    find_pattern: &str,
    replace_pattern: &str,
    language: Option<splice::cli::Language>,
    context_before: usize,
    context_after: usize,
    context_both: usize,
    validate: bool,
    create_backup: bool,
    operation_id: Option<String>,
    metadata: Option<String>,
    dry_run: bool,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::execution::log;
    use splice::patch::{
        apply_pattern_replace, find_pattern_in_files, BackupWriter, PatternReplaceConfig,
    };

    let start = std::time::Instant::now();
    let command_line = std::env::args().collect::<Vec<_>>().join(" ");

    let workspace_root = env::current_dir().map_err(|err| {
        splice::SpliceError::Other(format!("Failed to resolve current directory: {}", err))
    })?;

    let symbol_language = language.map(|l| l.to_symbol_language());

    if dry_run {
        let find_config = PatternReplaceConfig {
            glob_pattern: glob_pattern.to_string(),
            find_pattern: find_pattern.to_string(),
            replace_pattern: replace_pattern.to_string(),
            language: symbol_language,
            validate: false,
        };
        let matches = find_pattern_in_files(&find_config)?;

        let mut files_to_patch: std::collections::BTreeSet<std::path::PathBuf> =
            std::collections::BTreeSet::new();
        for m in &matches {
            files_to_patch.insert(m.file.clone());
        }

        let mut response_data = serde_json::Map::new();
        response_data.insert("dry_run".to_string(), serde_json::Value::Bool(true));
        response_data.insert(
            "files_would_patch".to_string(),
            json!(files_to_patch.iter().collect::<Vec<_>>()),
        );
        response_data.insert("matches_count".to_string(), json!(matches.len()));

        let message = format!(
            "[dry-run] Would replace {} occurrence(s) of {:?} across {} file(s). No changes written.",
            matches.len(),
            find_pattern,
            files_to_patch.len()
        );

        return Ok(splice::cli::CliSuccessPayload::with_data(
            message,
            serde_json::Value::Object(response_data),
        ));
    }

    let backup_manifest_path = if create_backup {
        let mut backup_writer = BackupWriter::new(&workspace_root, operation_id.clone())?;

        let find_config = PatternReplaceConfig {
            glob_pattern: glob_pattern.to_string(),
            find_pattern: find_pattern.to_string(),
            replace_pattern: replace_pattern.to_string(),
            language: symbol_language,
            validate: false,
        };
        let matches = find_pattern_in_files(&find_config)?;

        for m in &matches {
            backup_writer.backup_file(&m.file)?;
        }

        Some(backup_writer.finalize()?)
    } else {
        None
    };

    let config = PatternReplaceConfig {
        glob_pattern: glob_pattern.to_string(),
        find_pattern: find_pattern.to_string(),
        replace_pattern: replace_pattern.to_string(),
        language: symbol_language,
        validate,
    };

    let result = apply_pattern_replace(&config, &workspace_root)?;

    let mut response_data = serde_json::Map::new();
    response_data.insert("files_patched".to_string(), json!(result.files_patched));
    response_data.insert(
        "replacements_count".to_string(),
        json!(result.replacements_count),
    );
    if let Some(manifest_path) = backup_manifest_path {
        response_data.insert(
            "backup_manifest".to_string(),
            json!(manifest_path.to_string_lossy()),
        );
    }
    if let Some(ref op_id) = operation_id {
        response_data.insert("operation_id".to_string(), json!(op_id));
    }
    if let Some(meta) = metadata {
        if let Ok(parsed) = serde_json::from_str::<Value>(&meta) {
            response_data.insert("metadata".to_string(), parsed);
        } else {
            response_data.insert("metadata".to_string(), json!(meta));
        }
    }

    let message = format!(
        "Applied replacements to {} file(s) ({} replacements).",
        result.files_patched.len(),
        result.replacements_count
    );

    let duration_ms = start.elapsed().as_millis() as i64;
    let file_count = result.files_patched.len();
    let parameters = serde_json::json!({
        "glob": glob_pattern,
        "find": find_pattern,
        "replace": replace_pattern,
        "language": language.map(|l| l.as_str().to_string()),
        "file_count": file_count,
    });
    if let Err(e) = log::record_execution_with_params(
        &splice::output::OperationResult::with_execution_id(
            "apply-files".to_string(),
            operation_id.clone(),
        )
        .success(message.clone()),
        duration_ms,
        Some(command_line.clone()),
        parameters,
    ) {
        log_execution_error("apply-files", &e);
    }

    Ok(splice::cli::CliSuccessPayload::with_data(
        message,
        serde_json::Value::Object(response_data),
    ))
}
