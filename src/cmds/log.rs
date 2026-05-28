//! Log, explain, and migrate command handlers.

use std::path::Path;

use serde_json::json;

use super::helpers::parse_date;

#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
pub(crate) fn execute_log(
    operation_type: Option<String>,
    status: Option<String>,
    after: Option<String>,
    before: Option<String>,
    limit: usize,
    offset: usize,
    execution_id: Option<String>,
    json: bool,
    stats: bool,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::execution::{
        get_execution, get_execution_stats, init_execution_log_db, ExecutionQuery,
    };
    use splice::SpliceError;

    let splice_dir = std::path::PathBuf::from(".splice");
    let conn = init_execution_log_db(&splice_dir)?;

    if let Some(id) = execution_id {
        let log = get_execution(&conn, &id)?
            .ok_or_else(|| SpliceError::ExecutionNotFound { execution_id: id })?;

        if json || json_output {
            let json_output = serde_json::to_string_pretty(&log).map_err(|e| {
                SpliceError::Other(format!("failed to serialize execution to JSON: {}", e))
            })?;
            println!("{}", json_output);

            return Ok(splice::cli::CliSuccessPayload::with_data(
                "Execution details".to_string(),
                json!({ "execution_id": log.execution_id }),
            ));
        } else {
            println!("Execution Details:");
            println!("  ID: {}", log.execution_id);
            println!("  Type: {}", log.operation_type);
            println!("  Status: {}", log.status);
            println!("  Time: {}", log.timestamp);
            if let Some(workspace) = &log.workspace {
                println!("  Workspace: {}", workspace);
            }
            if let Some(cmd) = &log.command_line {
                println!("  Command: {}", cmd);
            }
            if let Some(duration) = log.duration_ms {
                println!("  Duration: {}ms", duration);
            }

            return Ok(splice::cli::CliSuccessPayload::message_only(
                "Execution details retrieved".to_string(),
            ));
        }
    }

    if stats {
        let stats = get_execution_stats(&conn)?;

        if json || json_output {
            let json_output = serde_json::to_string_pretty(&stats).map_err(|e| {
                SpliceError::Other(format!("failed to serialize stats to JSON: {}", e))
            })?;
            println!("{}", json_output);

            return Ok(splice::cli::CliSuccessPayload::with_data(
                "Execution statistics".to_string(),
                json!({ "total_operations": stats.total_operations }),
            ));
        } else {
            println!("Execution Statistics:");
            println!("  Total operations: {}", stats.total_operations);

            println!("  By type:");
            for (op_type, count) in &stats.by_type {
                println!("    {}: {}", op_type, count);
            }

            println!("  By status:");
            for (status, count) in &stats.by_status {
                println!("    {}: {}", status, count);
            }

            if let Some(oldest) = &stats.oldest_execution {
                println!("  Oldest: {}", oldest);
            }
            if let Some(newest) = &stats.newest_execution {
                println!("  Newest: {}", newest);
            }

            return Ok(splice::cli::CliSuccessPayload::message_only(
                "Statistics retrieved".to_string(),
            ));
        }
    }

    let mut query = ExecutionQuery::new().with_limit(limit).with_offset(offset);

    if let Some(op_type) = operation_type {
        query = query.with_operation_type(op_type);
    }

    if let Some(s) = status {
        query = query.with_status(s);
    }

    if let Some(after_str) = after {
        let timestamp = parse_date(&after_str)?;
        query = query.after(timestamp);
    }

    if let Some(before_str) = before {
        let timestamp = parse_date(&before_str)?;
        query = query.before(timestamp);
    }

    let logs = query.execute(&conn)?;

    if json || json_output {
        let json_output = serde_json::to_string_pretty(&logs)
            .map_err(|e| SpliceError::Other(format!("failed to serialize logs to JSON: {}", e)))?;
        println!("{}", json_output);

        Ok(splice::cli::CliSuccessPayload::with_data(
            format!("{} executions", logs.len()),
            json!({ "count": logs.len() }),
        ))
    } else {
        if logs.is_empty() {
            println!("No executions found matching criteria.");
            return Ok(splice::cli::CliSuccessPayload::message_only(
                "No executions found".to_string(),
            ));
        }

        println!(
            "{:<10} {:<8} {:<8} {:<20} {:<10} Message",
            "ID", "Type", "Status", "Time", "Duration"
        );
        println!("{}", "-".repeat(100));

        for log in &logs {
            use splice::execution::format_table_row;
            println!("{}", format_table_row(log));
        }

        println!("\nShowing {} of {} executions", logs.len(), logs.len());

        Ok(splice::cli::CliSuccessPayload::message_only(format!(
            "Retrieved {} executions",
            logs.len()
        )))
    }
}

pub(crate) fn execute_explain(
    code: String,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    if json_output {
        let explanation = splice::get_error_explanation(&code)
            .unwrap_or("Unknown error code")
            .to_string();

        let payload = splice::cli::CliSuccessPayload::with_data(
            format!("Error code explanation: {}", code),
            serde_json::json!({
                "code": code,
                "explanation": explanation,
            }),
        );
        return Ok(payload);
    }

    match splice::get_error_explanation(&code) {
        Some(explanation) => {
            println!("{}", explanation.trim());
        }
        None => {
            eprintln!("Unknown error code: {}", code);
            eprintln!();
            eprintln!("Error codes follow the format SPL-E### (e.g., SPL-E001).");
            eprintln!("Run `splice explain --list` to see all error codes.");
            eprintln!();
            eprintln!("For compiler error codes, see:");
            eprintln!("  Rust: https://doc.rust-lang.org/error-index.html");
            eprintln!("  TypeScript: https://www.typescriptlang.org/errors/");
            return Err(splice::SpliceError::Other(format!(
                "Unknown error code: {}",
                code
            )));
        }
    }

    Ok(splice::cli::CliSuccessPayload::message_only(format!(
        "Explained {}",
        code
    )))
}

pub(crate) fn execute_migrate_db(
    db_path: &Path,
    backup: bool,
    dry_run: bool,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::migrate::{check_schema_version, migrate_database};

    if dry_run {
        match check_schema_version(db_path) {
            Ok(version) => {
                let needs_migration = version < 6;

                if needs_migration {
                    println!("Current schema: v{}", version);
                    println!("Target schema: v6");
                    println!("Migration required: yes");
                    println!(
                        "\nTo migrate, run: splice migrate-db --db-path {}",
                        db_path.display()
                    );
                } else {
                    println!("Current schema: v{}", version);
                    println!("Target schema: v6");
                    println!("Migration required: no (already on v6 or later)");
                }

                return Ok(splice::cli::CliSuccessPayload::message_only(format!(
                    "Schema check complete: v{}",
                    version
                )));
            }
            Err(e) => {
                return Err(splice::SpliceError::Other(format!(
                    "Error checking schema version: {}",
                    e
                )))
            }
        }
    }

    match migrate_database(db_path, backup, false) {
        Ok(result) => {
            if let Some(ref backup_path) = result.backup_path {
                println!("Backup created: {}", backup_path.display());
            }
            println!(
                "Database migrated: v{} -> v{}",
                result.previous_version, result.new_version
            );
            println!("You can now use Magellan 2.0.0 features");

            Ok(splice::cli::CliSuccessPayload::with_data(
                format!(
                    "Migrated database: v{} -> v{}",
                    result.previous_version, result.new_version
                ),
                serde_json::json!({
                    "previous_version": result.previous_version,
                    "new_version": result.new_version,
                    "backup_path": result.backup_path,
                    "symbols_migrated": result.symbols_migrated,
                }),
            ))
        }
        Err(e) => Err(splice::SpliceError::Other(format!(
            "Migration failed: {}",
            e
        ))),
    }
}
