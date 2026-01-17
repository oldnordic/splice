//! Execution log infrastructure for Splice operations.
//!
//! This module provides persistent audit trail storage for all Splice operations.
//! Execution logs are stored in a separate SQLite database (`.splice/operations.db`)
//! to enable independent management from the code graph database.

use crate::error::{Result, SpliceError};
use rusqlite::{params, Connection};
use std::path::Path;

/// Execution log database filename.
pub const DB_FILENAME: &str = "operations.db";

/// Initialize the execution log database at the given path.
///
/// Creates the database file and all necessary tables if they don't exist.
///
/// # Arguments
///
/// * `db_dir` - Directory containing the database file (typically `.splice/`)
///
/// # Returns
///
/// Returns a `rusqlite::Connection` to the database.
///
/// # Errors
///
/// Returns an error if:
/// - The directory doesn't exist and cannot be created
/// - The database file cannot be created or opened
/// - Table creation fails
pub fn init_execution_log_db(db_dir: &Path) -> Result<Connection> {
    // Ensure directory exists
    if !db_dir.exists() {
        std::fs::create_dir_all(db_dir).map_err(|e| SpliceError::IoContext {
            context: format!("failed to create execution log directory: {}", db_dir.display()),
            source: e,
        })?;
    }

    let db_path = db_dir.join(DB_FILENAME);
    let conn = Connection::open(&db_path).map_err(|e| SpliceError::ExecutionLogError {
        message: format!(
            "failed to open execution log database: {}",
            db_path.display()
        ),
        source: Some(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
    })?;

    // Create tables
    create_tables(&conn)?;

    Ok(conn)
}

/// Create all necessary tables in the execution log database.
///
/// This function creates:
/// - `execution_log` table for storing operation records
/// - Indexes for efficient querying
fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS execution_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            execution_id TEXT NOT NULL UNIQUE,
            operation_type TEXT NOT NULL,
            status TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            workspace TEXT,
            command_line TEXT,
            parameters TEXT,
            result_summary TEXT,
            error_details TEXT,
            duration_ms INTEGER,
            created_at INTEGER NOT NULL
        )
        "#,
        [],
    )
    .map_err(|e| SpliceError::Other(format!("failed to create execution_log table: {}", e)))?;

    // Create indexes for efficient querying
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_execution_log_execution_id ON execution_log(execution_id)",
        [],
    )
    .map_err(|e| SpliceError::Other(format!("failed to create execution_id index: {}", e)))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_execution_log_operation_type ON execution_log(operation_type)",
        [],
    )
    .map_err(|e| SpliceError::Other(format!("failed to create operation_type index: {}", e)))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_execution_log_timestamp ON execution_log(created_at)",
        [],
    )
    .map_err(|e| SpliceError::Other(format!("failed to create timestamp index: {}", e)))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_execution_log_status ON execution_log(status)",
        [],
    )
    .map_err(|e| SpliceError::Other(format!("failed to create status index: {}", e)))?;

    Ok(())
}

/// Execution log entry.
///
/// Represents a single operation in the audit trail.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionLog {
    /// Database row ID
    pub id: i64,
    /// Unique operation identifier (UUID)
    pub execution_id: String,
    /// Operation type (e.g., "patch", "delete", "plan")
    pub operation_type: String,
    /// Operation status ("ok", "error", "partial")
    pub status: String,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Workspace root path (if available)
    pub workspace: Option<String>,
    /// Full command line for reproducibility
    pub command_line: Option<String>,
    /// Operation-specific parameters (JSON)
    pub parameters: Option<serde_json::Value>,
    /// Summary of results (JSON)
    pub result_summary: Option<serde_json::Value>,
    /// Error details if status != "ok" (JSON)
    pub error_details: Option<serde_json::Value>,
    /// Operation duration in milliseconds
    pub duration_ms: Option<i64>,
    /// Unix timestamp for sorting
    pub created_at: i64,
}

/// Builder for creating `ExecutionLog` entries.
///
/// Provides a fluent interface for constructing execution log records.
pub struct ExecutionLogBuilder {
    execution_id: String,
    operation_type: String,
    status: String,
    timestamp: String,
    workspace: Option<String>,
    command_line: Option<String>,
    parameters: Option<serde_json::Value>,
    result_summary: Option<serde_json::Value>,
    error_details: Option<serde_json::Value>,
    duration_ms: Option<i64>,
}

impl ExecutionLogBuilder {
    /// Create a new builder with required fields.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - Unique operation identifier (UUID)
    /// * `operation_type` - Type of operation (e.g., "patch", "delete")
    pub fn new(execution_id: String, operation_type: String) -> Self {
        Self {
            execution_id,
            operation_type,
            status: "ok".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            workspace: None,
            command_line: None,
            parameters: None,
            result_summary: None,
            error_details: None,
            duration_ms: None,
        }
    }

    /// Set the operation status.
    pub fn status(mut self, status: String) -> Self {
        self.status = status;
        self
    }

    /// Set the timestamp (ISO 8601).
    pub fn timestamp(mut self, timestamp: String) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Set the workspace root path.
    pub fn workspace(mut self, workspace: String) -> Self {
        self.workspace = Some(workspace);
        self
    }

    /// Set the command line.
    pub fn command_line(mut self, command_line: String) -> Self {
        self.command_line = Some(command_line);
        self
    }

    /// Set the operation parameters (JSON).
    pub fn parameters(mut self, parameters: serde_json::Value) -> Self {
        self.parameters = Some(parameters);
        self
    }

    /// Set the result summary (JSON).
    pub fn result_summary(mut self, result_summary: serde_json::Value) -> Self {
        self.result_summary = Some(result_summary);
        self
    }

    /// Set the error details (JSON).
    pub fn error_details(mut self, error_details: serde_json::Value) -> Self {
        self.error_details = Some(error_details);
        self
    }

    /// Set the operation duration in milliseconds.
    pub fn duration_ms(mut self, duration_ms: i64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Build the `ExecutionLog` entry.
    ///
    /// Converts the builder into an `ExecutionLog` instance.
    pub fn build(self) -> ExecutionLog {
        let created_at = chrono::Utc::now().timestamp();

        ExecutionLog {
            id: 0, // Will be set by database
            execution_id: self.execution_id,
            operation_type: self.operation_type,
            status: self.status,
            timestamp: self.timestamp,
            workspace: self.workspace,
            command_line: self.command_line,
            parameters: self.parameters,
            result_summary: self.result_summary,
            error_details: self.error_details,
            duration_ms: self.duration_ms,
            created_at,
        }
    }
}

/// Insert an execution log entry into the database.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `log` - Execution log entry to insert
///
/// # Returns
///
/// Returns the database row ID of the inserted entry.
///
/// # Errors
///
/// Returns an error if:
/// - The execution_id already exists in the database
/// - JSON serialization fails
/// - Database insertion fails
pub fn insert_execution_log(conn: &Connection, log: &ExecutionLog) -> Result<i64> {
    let parameters_json = log
        .parameters
        .as_ref()
        .map(|v| serde_json::to_string(v))
        .transpose()
        .map_err(|e| {
            SpliceError::Other(format!("failed to serialize parameters to JSON: {}", e))
        })?;

    let result_summary_json = log
        .result_summary
        .as_ref()
        .map(|v| serde_json::to_string(v))
        .transpose()
        .map_err(|e| {
            SpliceError::Other(format!("failed to serialize result_summary to JSON: {}", e))
        })?;

    let error_details_json = log
        .error_details
        .as_ref()
        .map(|v| serde_json::to_string(v))
        .transpose()
        .map_err(|e| {
            SpliceError::Other(format!("failed to serialize error_details to JSON: {}", e))
        })?;

    conn.execute(
        r#"
        INSERT INTO execution_log (
            execution_id, operation_type, status, timestamp, workspace,
            command_line, parameters, result_summary, error_details,
            duration_ms, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        params![
            &log.execution_id,
            &log.operation_type,
            &log.status,
            &log.timestamp,
            &log.workspace,
            &log.command_line,
            &parameters_json,
            &result_summary_json,
            &error_details_json,
            &log.duration_ms,
            &log.created_at,
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            SpliceError::Other(format!(
                "execution_id '{}' already exists in execution log",
                log.execution_id
            ))
        } else {
            SpliceError::Other(format!("failed to insert execution log: {}", e))
        }
    })?;

    Ok(conn.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_table_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_dir = temp_dir.path();

        let conn = init_execution_log_db(db_dir).unwrap();

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
    fn test_execution_log_builder() {
        let execution_id = uuid::Uuid::new_v4().to_string();
        let builder = ExecutionLogBuilder::new(execution_id.clone(), "patch".to_string())
            .status("ok".to_string())
            .workspace("/path/to/workspace".to_string())
            .command_line("splice patch foo bar".to_string())
            .duration_ms(1234);

        let log = builder.build();

        assert_eq!(log.execution_id, execution_id);
        assert_eq!(log.operation_type, "patch");
        assert_eq!(log.status, "ok");
        assert_eq!(log.workspace, Some("/path/to/workspace".to_string()));
        assert_eq!(log.command_line, Some("splice patch foo bar".to_string()));
        assert_eq!(log.duration_ms, Some(1234));
        assert!(log.created_at > 0);
    }

    #[test]
    fn test_indexes_created() {
        let temp_dir = TempDir::new().unwrap();
        let db_dir = temp_dir.path();

        let conn = init_execution_log_db(db_dir).unwrap();

        // Verify indexes exist
        let mut check_index = |index_name: &str| -> bool {
            conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='{}'",
                    index_name
                ),
                [],
                |row| row.get(0),
            )
            .unwrap()
        };

        assert!(
            check_index("idx_execution_log_execution_id"),
            "execution_id index should exist"
        );
        assert!(
            check_index("idx_execution_log_operation_type"),
            "operation_type index should exist"
        );
        assert!(
            check_index("idx_execution_log_timestamp"),
            "timestamp index should exist"
        );
        assert!(
            check_index("idx_execution_log_status"),
            "status index should exist"
        );
    }

    #[test]
    fn test_insert_execution_log() {
        let temp_dir = TempDir::new().unwrap();
        let db_dir = temp_dir.path();

        let conn = init_execution_log_db(db_dir).unwrap();

        let execution_id = uuid::Uuid::new_v4().to_string();
        let log = ExecutionLogBuilder::new(execution_id.clone(), "delete".to_string())
            .status("error".to_string())
            .error_details(serde_json::json!({"message": "test error"}))
            .build();

        let row_id = insert_execution_log(&conn, &log).unwrap();

        assert!(row_id > 0, "insert should return valid row ID");

        // Verify the log was inserted
        let retrieved: String = conn
            .query_row(
                "SELECT execution_id FROM execution_log WHERE id = ?1",
                params![row_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(retrieved, execution_id);
    }

    #[test]
    fn test_unique_execution_id() {
        let temp_dir = TempDir::new().unwrap();
        let db_dir = temp_dir.path();

        let conn = init_execution_log_db(db_dir).unwrap();

        let execution_id = uuid::Uuid::new_v4().to_string();
        let log1 = ExecutionLogBuilder::new(execution_id.clone(), "patch".to_string()).build();

        insert_execution_log(&conn, &log1).unwrap();

        // Try to insert with same execution_id - should fail
        let log2 = ExecutionLogBuilder::new(execution_id, "delete".to_string()).build();
        let result = insert_execution_log(&conn, &log2);

        assert!(result.is_err(), "duplicate execution_id should fail");
    }

    #[test]
    fn test_execution_log_error_display() {
        use crate::error::SpliceError;

        let error = SpliceError::ExecutionLogError {
            message: "database corrupted".to_string(),
            source: None,
        };

        let error_string = error.to_string();
        assert!(
            error_string.contains("Execution log database error"),
            "error should contain descriptive message"
        );
        assert!(
            error_string.contains("database corrupted"),
            "error should contain specific message"
        );
    }

    #[test]
    fn test_execution_not_found_error() {
        use crate::error::SpliceError;

        let execution_id = "non-existent-id";
        let error = SpliceError::ExecutionNotFound {
            execution_id: execution_id.to_string(),
        };

        let error_string = error.to_string();
        assert!(
            error_string.contains(execution_id),
            "error should contain execution ID"
        );
        assert!(error_string.contains("not found"), "error should indicate not found");
    }
}
