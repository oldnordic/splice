//! High-level execution logging functions.
//!
//! This module provides a non-blocking logging interface for recording
//! Splice operations to the execution log database.
//!
//! # Error Handling
//!
//! Execution logging functions return `Result<()>` to allow callers
//! to handle errors appropriately. For production use, logging failures
//! are non-fatal - callers should use `if let Err(e)` patterns to log
//! warnings without failing the primary operation.
//!
//! This design ensures:
//! - Logging failures don't break the primary operation
//! - Errors are visible in logs for debugging
//! - Callers can choose their error handling strategy

use crate::error::{Result, SpliceError};
use crate::execution::{
    init_execution_log_db, insert_execution_log, ExecutionLog, ExecutionLogBuilder,
};
use crate::output::OperationResult;
use rusqlite::Connection;
use std::path::PathBuf;

/// Environment variable to enable/disable execution logging.
const EXECUTION_LOG_ENV: &str = "SPLICE_EXECUTION_LOG";

/// Splice directory name.
const SPLICE_DIR: &str = ".splice";

/// Configuration for execution logging behavior.
///
/// This struct allows dependency injection for testing and programmatic
/// control of execution logging without relying on environment variables.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionLogConfig {
    /// Whether execution logging is enabled.
    ///
    /// If None, reads from SPLICE_EXECUTION_LOG environment variable.
    /// If Some(bool), uses the provided value explicitly.
    pub enabled: Option<bool>,
}

impl ExecutionLogConfig {
    /// Create a config that explicitly enables logging.
    #[must_use]
    pub fn enabled() -> Self {
        Self { enabled: Some(true) }
    }

    /// Create a config that explicitly disables logging.
    #[must_use]
    pub fn disabled() -> Self {
        Self { enabled: Some(false) }
    }

    /// Create a config that respects the environment variable.
    #[must_use]
    pub fn from_env() -> Self {
        Self { enabled: None }
    }

    /// Check if logging is enabled with this configuration.
    ///
    /// - If `enabled` is `Some(bool)`, returns that value
    /// - If `enabled` is `None`, reads from SPLICE_EXECUTION_LOG environment variable
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        match self.enabled {
            Some(explicit) => explicit,
            None => {
                // Read from environment variable
                std::env::var(EXECUTION_LOG_ENV)
                    .map(|v| {
                        let v_lower = v.to_lowercase();
                        v_lower != "false" && v_lower != "0" && v_lower != "no"
                    })
                    .unwrap_or(true) // Default: enabled
            }
        }
    }
}

impl Default for ExecutionLogConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

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

/// Initialize execution log database in a specific directory.
///
/// This is useful for testing where you want to avoid changing the current directory.
///
/// # Arguments
///
/// * `base_dir` - Base directory where `.splice` will be created
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
pub fn init_db_in_dir(base_dir: &std::path::Path) -> Result<Connection> {
    let db_dir = base_dir.join(SPLICE_DIR);
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
    let mut builder =
        ExecutionLogBuilder::new(result.execution_id.clone(), result.operation_type.clone())
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
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    /// Get a lock for synchronizing environment variable access in tests.
    ///
    /// The SPLICE_EXECUTION_LOG environment variable is process-global state.
    /// Tests that modify this variable must hold this lock for their entire
    /// duration to prevent race conditions with parallel test execution.
    ///
    /// # Example
    /// ```ignore
    /// #[test]
    /// fn test_something() {
    ///     let _guard = env_lock().lock().unwrap();
    ///     std::env::set_var(EXECUTION_LOG_ENV, "false");
    ///     // ... test code ...
    ///     // _guard dropped here, after test completes
    /// }
    /// ```
    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_db_path() {
        let path = db_path();
        assert!(path.ends_with(".splice/operations.db"));
    }

    #[test]
    fn test_is_enabled_default() {
        let _guard = env_lock().lock().unwrap();
        // Clear environment variable to test default
        std::env::remove_var(EXECUTION_LOG_ENV);
        assert!(is_enabled(), "Execution log should be enabled by default");
        std::env::set_var(EXECUTION_LOG_ENV, "true"); // Reset to default
    }

    #[test]
    fn test_is_enabled_false() {
        let _guard = env_lock().lock().unwrap();
        std::env::set_var(EXECUTION_LOG_ENV, "false");
        assert!(
            !is_enabled(),
            "Execution log should be disabled when set to false"
        );
        std::env::set_var(EXECUTION_LOG_ENV, "true"); // Reset to default
    }

    #[test]
    fn test_is_enabled_true() {
        let _guard = env_lock().lock().unwrap();
        std::env::set_var(EXECUTION_LOG_ENV, "TRUE"); // Test case insensitivity
        assert!(
            is_enabled(),
            "Execution log should be enabled when set to TRUE"
        );
        std::env::set_var(EXECUTION_LOG_ENV, "true"); // Reset to default
    }

    #[test]
    fn test_init_db_creates_tables() {
        let temp_dir = TempDir::new().unwrap();

        // Use init_db_in_dir to avoid changing current directory (fixes parallel test race)
        let conn = init_db_in_dir(temp_dir.path()).unwrap();

        // Verify table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='execution_log'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(table_exists, "execution_log table should be created");
    }

    #[test]
    fn test_record_execution() {
        let temp_dir = TempDir::new().unwrap();

        use crate::execution::insert_execution_log;
        use crate::output::OperationResult;
        use uuid::Uuid;

        let execution_id = Uuid::new_v4().to_string();
        let result =
            OperationResult::with_execution_id("patch".to_string(), Some(execution_id.clone()))
                .success("Test operation".to_string())
                .with_workspace("/test/workspace".to_string());

        let command_line = Some("splice patch test_symbol".to_string());

        // Use init_db_in_dir to avoid changing current directory (fixes parallel test race)
        let conn = init_db_in_dir(temp_dir.path()).unwrap();

        // Build log entry and insert directly
        let log_entry = build_log_entry(&result, 1234, command_line, None).unwrap();
        let insert_result = insert_execution_log(&conn, &log_entry);
        assert!(
            insert_result.is_ok(),
            "Recording should succeed: {:?}",
            insert_result.err()
        );
    }

    #[test]
    fn test_record_execution_with_params() {
        let temp_dir = TempDir::new().unwrap();

        use crate::execution::insert_execution_log;
        use crate::output::OperationResult;
        use uuid::Uuid;

        let execution_id = Uuid::new_v4().to_string();
        let result =
            OperationResult::with_execution_id("delete".to_string(), Some(execution_id.clone()))
                .success("Test delete".to_string());

        let parameters = serde_json::json!({
            "file": "/test/file.rs",
            "symbol": "test_function",
        });

        // Use init_db_in_dir to avoid changing current directory (fixes parallel test race)
        let conn = init_db_in_dir(temp_dir.path()).unwrap();

        // Build log entry and insert directly
        let log_entry = build_log_entry(&result, 567, None, Some(parameters)).unwrap();
        let insert_result = insert_execution_log(&conn, &log_entry);
        assert!(
            insert_result.is_ok(),
            "Recording with params should succeed: {:?}",
            insert_result.err()
        );
    }

    #[test]
    fn test_record_execution_failure() {
        let temp_dir = TempDir::new().unwrap();

        use crate::execution::{insert_execution_log, ExecutionLog};
        use uuid::Uuid;

        let execution_id = Uuid::new_v4().to_string();
        let error = SpliceError::Other("Test error".to_string());

        // Build the log entry directly (same as record_execution_failure does)
        let timestamp = chrono::Utc::now().to_rfc3339();
        let created_at = chrono::Utc::now().timestamp();

        let error_details = serde_json::json!({
            "kind": std::format!("{:?}", std::error::Error::source(&error).map_or_else(
                || error.to_string(),
                |e| e.to_string()
            )),
            "message": error.to_string(),
        });

        let log_entry = ExecutionLog {
            id: 0,
            execution_id: execution_id.clone(),
            operation_type: "patch".to_string(),
            status: "error".to_string(),
            timestamp,
            workspace: temp_dir.path().to_str().map(|s| s.to_string()),
            command_line: Some("splice patch test".to_string()),
            parameters: None,
            result_summary: None,
            error_details: Some(error_details),
            duration_ms: Some(100),
            created_at,
        };

        // Use init_db_in_dir to avoid changing current directory (fixes parallel test race)
        let conn = init_db_in_dir(temp_dir.path()).unwrap();

        // Insert directly
        let insert_result = insert_execution_log(&conn, &log_entry);
        assert!(
            insert_result.is_ok(),
            "Recording failure should succeed: {:?}",
            insert_result.err()
        );
    }
}
