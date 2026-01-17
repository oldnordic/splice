//! High-level execution logging functions.
//!
//! This module provides a non-blocking logging interface for recording
//! Splice operations to the execution log database.

use crate::error::{Result, SpliceError};
use crate::execution::{ExecutionLog, ExecutionLogBuilder, init_execution_log_db, insert_execution_log};
use crate::output::OperationResult;
use rusqlite::Connection;
use std::path::PathBuf;

/// Environment variable to enable/disable execution logging.
const EXECUTION_LOG_ENV: &str = "SPLICE_EXECUTION_LOG";

/// Splice directory name.
const SPLICE_DIR: &str = ".splice";

/// Get execution log database path.
///
/// Returns the path to `.splice/operations.db` in the current directory.
pub fn db_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(SPLICE_DIR)
        .join("operations.db")
}

/// Check if execution log is enabled.
///
/// Checks the `SPLICE_EXECUTION_LOG` environment variable.
/// Default: `true` (enabled).
///
/// Set to `false`, `0`, or `no` to disable logging.
pub fn is_enabled() -> bool {
    std::env::var(EXECUTION_LOG_ENV)
        .map(|v| {
            let v_lower = v.to_lowercase();
            v_lower != "false" && v_lower != "0" && v_lower != "no"
        })
        .unwrap_or(true)
}

/// Initialize execution log database.
///
/// Creates the database file and tables if they don't exist.
/// This function is idempotent - calling it multiple times is safe.
///
/// # Returns
///
/// Returns a `rusqlite::Connection` to the database.
///
/// # Errors
///
/// Returns an error if:
/// - The directory cannot be created
/// - The database cannot be opened
/// - Table creation fails
pub fn init_db() -> Result<Connection> {
    let db_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(SPLICE_DIR);

    init_execution_log_db(&db_dir)
}

/// Record operation execution to log.
///
/// Extracts data from `OperationResult` and records it to the database.
/// This function is non-blocking - errors are logged but don't fail the operation.
///
/// # Arguments
///
/// * `result` - Operation result to record
/// * `duration_ms` - Operation duration in milliseconds
/// * `command_line` - Optional full command line for reproducibility
///
/// # Returns
///
/// Returns `Ok(())` if recording succeeds, or `Err` if it fails.
/// Callers should log errors as warnings, not fail the operation.
pub fn record_execution(
    result: &OperationResult,
    duration_ms: i64,
    command_line: Option<String>,
) -> Result<()> {
    let log_entry = build_log_entry(result, duration_ms, command_line, None)?;
    let conn = init_db()?;
    insert_execution_log(&conn, &log_entry)?;
    Ok(())
}

/// Record operation with parameters.
///
/// Same as `record_execution` but includes operation-specific parameters.
///
/// # Arguments
///
/// * `result` - Operation result to record
/// * `duration_ms` - Operation duration in milliseconds
/// * `command_line` - Optional full command line for reproducibility
/// * `parameters` - Operation-specific parameters (JSON)
pub fn record_execution_with_params(
    result: &OperationResult,
    duration_ms: i64,
    command_line: Option<String>,
    parameters: serde_json::Value,
) -> Result<()> {
    let log_entry = build_log_entry(result, duration_ms, command_line, Some(parameters))?;
    let conn = init_db()?;
    insert_execution_log(&conn, &log_entry)?;
    Ok(())
}

/// Record operation failure.
///
/// Records a failed operation to the execution log.
/// This is useful for tracking errors and debugging issues.
///
/// # Arguments
///
/// * `execution_id` - Unique operation identifier (UUID)
/// * `operation_type` - Type of operation (e.g., "patch", "delete")
/// * `error` - The error that occurred
/// * `duration_ms` - Operation duration before failure
/// * `command_line` - Optional full command line for reproducibility
pub fn record_execution_failure(
    execution_id: &str,
    operation_type: &str,
    error: &SpliceError,
    duration_ms: i64,
    command_line: Option<String>,
) -> Result<()> {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let created_at = chrono::Utc::now().timestamp();

    // Build error details JSON
    let error_details = serde_json::json!({
        "kind": std::format!("{:?}", std::error::Error::source(error).map_or_else(
            || error.to_string(),
            |e| e.to_string()
        )),
        "message": error.to_string(),
    });

    let log_entry = ExecutionLog {
        id: 0,
        execution_id: execution_id.to_string(),
        operation_type: operation_type.to_string(),
        status: "error".to_string(),
        timestamp,
        workspace: std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string()),
        command_line,
        parameters: None,
        result_summary: None,
        error_details: Some(error_details),
        duration_ms: Some(duration_ms),
        created_at,
    };

    let conn = init_db()?;
    insert_execution_log(&conn, &log_entry)?;
    Ok(())
}

/// Build execution log entry from operation result.
fn build_log_entry(
    result: &OperationResult,
    duration_ms: i64,
    command_line: Option<String>,
    parameters: Option<serde_json::Value>,
) -> Result<ExecutionLog> {
    let mut builder = ExecutionLogBuilder::new(
        result.operation_id.clone(),
        result.operation_type.clone(),
    )
    .status(result.status.clone())
    .timestamp(result.timestamp.clone())
    .duration_ms(duration_ms);

    if let Some(ref workspace) = result.workspace {
        builder = builder.workspace(workspace.clone());
    }

    if let Some(cmd) = command_line {
        builder = builder.command_line(cmd);
    }

    if let Some(params) = parameters {
        builder = builder.parameters(params);
    }

    // Build result summary JSON
    let result_summary = serde_json::json!({
        "message": result.message,
    });
    builder = builder.result_summary(result_summary);

    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_db_path() {
        let path = db_path();
        assert!(path.ends_with(".splice/operations.db"));
    }

    #[test]
    fn test_is_enabled_default() {
        // Clear environment variable to test default
        std::env::remove_var(EXECUTION_LOG_ENV);
        assert!(is_enabled(), "Execution log should be enabled by default");
        std::env::set_var(EXECUTION_LOG_ENV, "true"); // Reset to default
    }

    #[test]
    fn test_is_enabled_false() {
        std::env::set_var(EXECUTION_LOG_ENV, "false");
        assert!(!is_enabled(), "Execution log should be disabled when set to false");
        std::env::set_var(EXECUTION_LOG_ENV, "true"); // Reset to default
    }

    #[test]
    fn test_is_enabled_true() {
        std::env::set_var(EXECUTION_LOG_ENV, "TRUE"); // Test case insensitivity
        assert!(is_enabled(), "Execution log should be enabled when set to TRUE");
        std::env::set_var(EXECUTION_LOG_ENV, "true"); // Reset to default
    }

    #[test]
    fn test_init_db_creates_tables() {
        let temp_dir = TempDir::new().unwrap();
        let _db_dir = temp_dir.path().join(".splice");

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let conn = init_db().unwrap();

        // Verify table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='execution_log'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(table_exists, "execution_log table should be created");

        // Restore original directory before temp_dir is dropped
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_record_execution() {
        let temp_dir = TempDir::new().unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        use crate::output::OperationResult;
        use uuid::Uuid;

        let execution_id = Uuid::new_v4().to_string();
        let result = OperationResult::with_id("patch".to_string(), Some(execution_id.clone()))
            .success("Test operation".to_string())
            .with_workspace("/test/workspace".to_string());

        let command_line = Some("splice patch test_symbol".to_string());

        // Test that recording doesn't fail
        let recording_result = record_execution(&result, 1234, command_line);
        assert!(recording_result.is_ok(), "Recording should succeed: {:?}", recording_result.err());

        // Restore original directory before temp_dir is dropped
        let _ = std::env::set_current_dir(original_dir);
    }

    #[test]
    fn test_record_execution_with_params() {
        let temp_dir = TempDir::new().unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        use crate::output::OperationResult;
        use uuid::Uuid;

        let execution_id = Uuid::new_v4().to_string();
        let result = OperationResult::with_id("delete".to_string(), Some(execution_id.clone()))
            .success("Test delete".to_string());

        let parameters = serde_json::json!({
            "file": "/test/file.rs",
            "symbol": "test_function",
        });

        // Test that recording doesn't fail
        let recording_result = record_execution_with_params(&result, 567, None, parameters);
        assert!(recording_result.is_ok(), "Recording with params should succeed: {:?}", recording_result.err());

        // Restore original directory before temp_dir is dropped
        let _ = std::env::set_current_dir(original_dir);
    }

    #[test]
    fn test_record_execution_failure() {
        let temp_dir = TempDir::new().unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        use uuid::Uuid;

        let execution_id = Uuid::new_v4().to_string();
        let error = SpliceError::Other("Test error".to_string());

        // Test that recording failure doesn't fail
        let recording_result = record_execution_failure(&execution_id, "patch", &error, 100, Some("splice patch test".to_string()));
        assert!(recording_result.is_ok(), "Recording failure should succeed: {:?}", recording_result.err());

        // Restore original directory before temp_dir is dropped
        let _ = std::env::set_current_dir(original_dir);
    }
}
