//! Batch and complete command handlers.

use std::path::Path;

#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
pub(crate) fn execute_batch(
    spec_path: &std::path::Path,
    db_path: Option<std::path::PathBuf>,
    dry_run: bool,
    continue_on_error: bool,
    rollback: splice::cli::CliRollbackMode,
    analyzer: Option<splice::cli::AnalyzerMode>,
    analyzer_binary: Option<std::path::PathBuf>,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::batch::{parse_batch_spec, BatchExecutor, ExecutionMode, RollbackMode};

    let spec = parse_batch_spec(&spec_path.to_path_buf())?;

    let mode = if continue_on_error {
        ExecutionMode::ContinueOnError
    } else {
        spec.mode
    };

    let rollback_mode = match rollback {
        splice::cli::CliRollbackMode::Auto => {
            if db_path.is_some() {
                splice::batch::RollbackMode::OnFailure
            } else {
                eprintln!("Warning: Automatic rollback requires --db flag");
                eprintln!("         Batch will execute without automatic rollback");
                splice::batch::RollbackMode::Never
            }
        }
        splice::cli::CliRollbackMode::Never => splice::batch::RollbackMode::Never,
        splice::cli::CliRollbackMode::Always => {
            if db_path.is_some() {
                splice::batch::RollbackMode::Always
            } else {
                eprintln!("Warning: 'Always' rollback mode requires --db flag");
                eprintln!("         Batch will execute without automatic rollback");
                splice::batch::RollbackMode::Never
            }
        }
    };

    let analyzer_mode = match analyzer {
        Some(splice::cli::AnalyzerMode::Off) => splice::validate::AnalyzerMode::Off,
        Some(splice::cli::AnalyzerMode::Os) => splice::validate::AnalyzerMode::Path,
        Some(splice::cli::AnalyzerMode::Path) => {
            if let Some(binary) = analyzer_binary {
                splice::validate::AnalyzerMode::Explicit(binary.to_string_lossy().to_string())
            } else {
                splice::validate::AnalyzerMode::Path
            }
        }
        None => splice::validate::AnalyzerMode::Off,
    };

    let use_transaction = rollback_mode != RollbackMode::Never && db_path.is_some();

    let mut executor = BatchExecutor::new(dry_run, db_path.clone(), analyzer_mode);

    let (batch_result, rolled_back, rollback_snapshot) = if use_transaction {
        let txn_result = executor.execute_transaction(&spec, dry_run, rollback_mode)?;
        (
            txn_result.batch_result,
            txn_result.rolled_back,
            txn_result
                .rollback_snapshot
                .map(|p| p.to_string_lossy().to_string()),
        )
    } else {
        let result = executor.execute(&spec)?;
        (result, false, None)
    };

    let mut payload = serde_json::Map::new();
    payload.insert(
        "spec_path".to_string(),
        serde_json::json!(spec_path.to_string_lossy()),
    );
    payload.insert(
        "total_operations".to_string(),
        serde_json::json!(batch_result.total_operations),
    );
    payload.insert(
        "successful".to_string(),
        serde_json::json!(batch_result.successful),
    );
    payload.insert("failed".to_string(), serde_json::json!(batch_result.failed));
    payload.insert(
        "duration_ms".to_string(),
        serde_json::json!(batch_result.total_duration_ms),
    );
    payload.insert("rolled_back".to_string(), serde_json::json!(rolled_back));
    if let Some(snapshot) = rollback_snapshot {
        payload.insert("rollback_snapshot".to_string(), serde_json::json!(snapshot));
    }

    let ops_json: Vec<serde_json::Value> = batch_result
        .operations
        .into_iter()
        .map(|op| {
            let mut obj = serde_json::Map::new();
            obj.insert("index".to_string(), serde_json::json!(op.index));
            obj.insert("type".to_string(), serde_json::json!(op.op_type));
            obj.insert("success".to_string(), serde_json::json!(op.success));
            if let Some(error) = op.error {
                obj.insert("error".to_string(), serde_json::json!(error));
            }
            obj.insert("duration_ms".to_string(), serde_json::json!(op.duration_ms));
            serde_json::Value::Object(obj)
        })
        .collect();
    payload.insert("operations".to_string(), serde_json::json!(ops_json));

    if batch_result.failed > 0 && mode == ExecutionMode::StopOnError {
        return Err(splice::SpliceError::Other(format!(
            "Batch execution stopped: {} operation(s) failed",
            batch_result.failed
        )));
    }

    Ok(splice::cli::CliSuccessPayload {
        status: "ok",
        message: if dry_run {
            format!(
                "Batch preview complete: {} operations",
                batch_result.total_operations
            )
        } else {
            format!(
                "Batch complete: {} succeeded, {} failed",
                batch_result.successful, batch_result.failed
            )
        },
        data: Some(serde_json::Value::Object(payload)),
        already_emitted: false,
        has_pending_changes: dry_run,
    })
}

pub(crate) fn execute_complete(
    file: &Path,
    line: usize,
    column: usize,
    max_results: usize,
    db: &Path,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::completion::engine::CompletionEngine;
    use splice::completion::types::CompletionRequest;
    use splice::graph::MagellanIntegration;
    use std::sync::Arc;

    let db_path = if db.is_absolute() {
        db.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| {
                splice::SpliceError::Other(format!("Failed to get current directory: {}", e))
            })?
            .join(db)
    };

    let db_path = db_path.canonicalize().map_err(|e| {
        splice::SpliceError::Other(format!(
            "Failed to resolve database path {}: {}",
            db_path.display(),
            e
        ))
    })?;

    let file_path = if file.is_absolute() {
        file.to_path_buf()
    } else {
        std::env::current_dir()
            .expect("invariant: current directory always available")
            .join(file)
            .canonicalize()
            .map_err(|e| {
                splice::SpliceError::Other(format!("Failed to resolve file path: {}", e))
            })?
    };

    #[allow(
        clippy::arc_with_non_send_sync,
        reason = "single-threaded shared ownership for completion engine"
    )]
    let magellan = Arc::new(MagellanIntegration::open(&db_path)?);

    let engine = CompletionEngine::new(magellan.clone(), &db_path);

    let request = CompletionRequest {
        file_path,
        line,
        column,
        max_results: Some(max_results),
    };

    let response = engine
        .complete_at_cursor(request)
        .map_err(|e| splice::SpliceError::Other(format!("Completion failed: {}", e)))?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&response)
                .expect("invariant: serde_json serialization never fails on serializable types")
        );
    } else {
        println!(
            "Grounded Completions ({} suggestions):",
            response.suggestions.len()
        );
        println!();

        for (i, suggestion) in response.suggestions.iter().enumerate() {
            println!("{}. {}", i + 1, suggestion.label);
            println!("   Detail: {}", suggestion.detail);
            println!("   Score: {:.2}", suggestion.score);
            println!("   Source: {:?}", suggestion.source);
            println!("   Grounded in: {:?}", suggestion.grounded_in);
            if suggestion.usage_count > 1 {
                println!("   Used {} times", suggestion.usage_count);
            }
            println!();
        }

        println!("Metadata:");
        println!("  Query time: {} ms", response.metadata.query_time_ms);
        println!(
            "  Total symbols: {}",
            response.metadata.total_symbols_indexed
        );
        println!("  Database version: {}", response.metadata.database_version);
        println!("  Database queries: {}", response.metadata.database_queries);
    }

    Ok(splice::cli::CliSuccessPayload {
        status: "ok",
        message: format!("Generated {} completions", response.suggestions.len()),
        data: None,
        already_emitted: true,
        has_pending_changes: false,
    })
}
