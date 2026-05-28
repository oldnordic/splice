//! Patch batch command handler.

use std::env;
use std::path::{Path, PathBuf};

use serde_json::json;
use serde_json::Value;

use super::helpers::log_execution_error;

#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
pub(crate) fn execute_patch_batch(
    batch_path: &Path,
    analyzer: Option<splice::cli::AnalyzerMode>,
    analyzer_binary: Option<std::path::PathBuf>,
    language: Option<splice::cli::Language>,
    create_backup: bool,
    operation_id: Option<String>,
    metadata: Option<String>,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::execution::log;
    use splice::patch::{apply_batch_with_validation, load_batches_from_file};
    use splice::validate::AnalyzerMode as ValidateAnalyzerMode;

    // Start timing
    let start = std::time::Instant::now();
    let command_line = std::env::args().collect::<Vec<_>>().join(" ");

    let absolute_batch = if batch_path.is_absolute() {
        batch_path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|err| {
                splice::SpliceError::Other(format!("Failed to resolve current directory: {}", err))
            })?
            .join(batch_path)
    };

    let workspace_dir = absolute_batch.parent().ok_or_else(|| {
        splice::SpliceError::Other(
            "Cannot determine workspace directory from --batch path".to_string(),
        )
    })?;
    let workspace_dir = workspace_dir.to_path_buf();

    let symbol_language = language
        .ok_or_else(|| {
            splice::SpliceError::Other(
                "The --language flag is required when --batch is used".to_string(),
            )
        })?
        .to_symbol_language();

    let analyzer_mode = match analyzer {
        Some(splice::cli::AnalyzerMode::Off) => ValidateAnalyzerMode::Off,
        Some(splice::cli::AnalyzerMode::Os) => ValidateAnalyzerMode::Path,
        Some(splice::cli::AnalyzerMode::Path) => {
            if let Some(binary) = analyzer_binary {
                ValidateAnalyzerMode::Explicit(binary.to_string_lossy().to_string())
            } else {
                ValidateAnalyzerMode::Path
            }
        }
        None => ValidateAnalyzerMode::Off,
    };

    let batches = load_batches_from_file(&absolute_batch)?;
    let batch_count = batches.len();

    // Create backup if requested
    let backup_manifest_path = if create_backup {
        use splice::patch::BackupWriter;

        let workspace_root = splice::workspace::find_workspace_root(&absolute_batch)?;

        // Collect all files that will be patched
        let mut files_to_backup: std::collections::HashSet<PathBuf> =
            std::collections::HashSet::new();
        for batch in &batches {
            for replacement in batch.replacements() {
                files_to_backup.insert(replacement.file.clone());
            }
        }

        let mut backup_writer = BackupWriter::new(&workspace_root, operation_id.clone())?;
        for file in files_to_backup {
            backup_writer.backup_file(&file)?;
        }
        Some(backup_writer.finalize()?)
    } else {
        None
    };

    let summaries =
        apply_batch_with_validation(&batches, &workspace_dir, symbol_language, analyzer_mode)?;

    // Check if JSON output is requested
    if _json_output {
        use splice::output::{
            ApplyFilesResult, FilePatternResult, OperationData, OperationResult, SpanResult,
        };

        // Build file results with spans
        let mut file_results: Vec<FilePatternResult> = Vec::new();
        for summary in &summaries {
            // Find all spans for this file
            let mut spans: Vec<SpanResult> = Vec::new();
            for batch in &batches {
                for replacement in batch.replacements() {
                    if replacement.file == summary.file {
                        spans.push(SpanResult::from_byte_span(
                            replacement.file.to_string_lossy().to_string(),
                            replacement.start,
                            replacement.end,
                        ));
                    }
                }
            }

            file_results.push(FilePatternResult {
                file: summary.file.to_string_lossy().to_string(),
                matches: spans.len(),
                replacements: spans.len(),
                spans,
                before_hash: summary.before_hash.clone(),
                after_hash: summary.after_hash.clone(),
            });
        }

        // Sort file_results deterministically by file path
        file_results.sort();

        // Sort spans within each file deterministically
        for result in &mut file_results {
            result.spans.sort();
        }

        // Create batch result structure (reuse ApplyFilesResult)
        let apply_result = ApplyFilesResult {
            glob_pattern: absolute_batch.to_string_lossy().to_string(),
            find_pattern: "batch".to_string(),
            replace_pattern: "patch".to_string(),
            files_matched: file_results.len(),
            files_modified: summaries.len(),
            files: file_results,
        };

        let message = format!(
            "Patched {} file(s) across {} batch(es).",
            summaries.len(),
            batch_count
        );

        // Record execution (before apply_result is moved)
        let duration_ms = start.elapsed().as_millis() as i64;
        let parameters = serde_json::json!({
            "batch_file": absolute_batch.to_string_lossy(),
            "file_count": apply_result.files.len(),
            "span_count": apply_result.files.iter().map(|f| f.matches).sum::<usize>(),
        });

        // Create operation result with operation_id from CLI or generate new UUID
        let result = OperationResult::with_execution_id("batch".to_string(), operation_id.clone())
            .success(message.clone())
            .with_result(OperationData::ApplyFiles(apply_result));

        if let Err(e) = log::record_execution_with_params(
            &result,
            duration_ms,
            Some(command_line.clone()),
            parameters,
        ) {
            log_execution_error("batch", &e);
        }

        // Output structured JSON directly
        println!(
            "{}",
            serde_json::to_string_pretty(&result)
                .expect("invariant: serde_json serialization never fails on serializable types")
        );

        // Return a dummy payload marked as already emitted
        return Ok(
            splice::cli::CliSuccessPayload::message_only("OK".to_string()).already_emitted(),
        );
    }

    let files_data: Vec<_> = summaries
        .iter()
        .map(|summary| {
            json!({
                "file": summary.file.to_string_lossy(),
                "before_hash": summary.before_hash,
                "after_hash": summary.after_hash,
            })
        })
        .collect();

    // Collect span_ids from all batches
    let mut span_ids: Vec<serde_json::Value> = Vec::new();
    for batch in &batches {
        for replacement in batch.replacements() {
            span_ids.push(json!({
                "file": replacement.file.to_string_lossy(),
                "byte_start": replacement.start,
                "byte_end": replacement.end,
            }));
        }
    }

    let mut response_data = json!({
        "batch_file": absolute_batch.to_string_lossy(),
        "batches_applied": batch_count,
        "files": files_data,
        "span_ids": span_ids,
    });

    if let Some(manifest_path) = &backup_manifest_path {
        response_data["backup_manifest"] = json!(manifest_path.to_string_lossy());
    }

    if let Some(ref op_id) = operation_id {
        response_data["operation_id"] = json!(op_id);
    }

    if let Some(meta) = metadata {
        // Try to parse as JSON, if fails include as string
        if let Ok(parsed) = serde_json::from_str::<Value>(&meta) {
            response_data["metadata"] = parsed;
        } else {
            response_data["metadata"] = json!(meta);
        }
    }

    // Record execution for regular output
    let message = format!(
        "Patched {} file(s) across {} batch(es).",
        summaries.len(),
        batch_count
    );
    let duration_ms = start.elapsed().as_millis() as i64;
    let parameters = serde_json::json!({
        "batch_file": absolute_batch.to_string_lossy(),
        "file_count": summaries.len(),
        "span_count": span_ids.len(),
    });
    if let Err(e) = log::record_execution_with_params(
        &splice::output::OperationResult::with_execution_id(
            "batch".to_string(),
            operation_id.clone(),
        )
        .success(message.clone()),
        duration_ms,
        Some(command_line.clone()),
        parameters,
    ) {
        log_execution_error("batch", &e);
    }

    Ok(splice::cli::CliSuccessPayload::with_data(
        message,
        response_data,
    ))
}
