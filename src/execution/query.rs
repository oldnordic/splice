//! Query functions for execution log.
//!
//! This module provides flexible querying capabilities for the execution audit trail.
//! Supports filtering by operation type, status, date range, and pagination.

use crate::error::{Result, SpliceError};
use crate::execution::base::ExecutionLog;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;

/// Query builder for execution log.
///
/// Provides a fluent interface for building complex queries with optional filters.
pub struct ExecutionQuery {
    operation_type: Option<String>,
    status: Option<String>,
    after: Option<i64>,
    before: Option<i64>,
    limit: Option<usize>,
    offset: Option<usize>,
    execution_id: Option<String>,
}

impl ExecutionQuery {
    /// Create a new query builder with no filters.
    pub fn new() -> Self {
        Self {
            operation_type: None,
            status: None,
            after: None,
            before: None,
            limit: None,
            offset: None,
            execution_id: None,
        }
    }

    /// Filter by operation type (e.g., "patch", "delete", "plan").
    pub fn with_operation_type(mut self, op: String) -> Self {
        self.operation_type = Some(op);
        self
    }

    /// Filter by status (e.g., "ok", "error", "partial").
    pub fn with_status(mut self, status: String) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter for operations after a Unix timestamp.
    pub fn after(mut self, timestamp: i64) -> Self {
        self.after = Some(timestamp);
        self
    }

    /// Filter for operations before a Unix timestamp.
    pub fn before(mut self, timestamp: i64) -> Self {
        self.before = Some(timestamp);
        self
    }

    /// Set maximum number of results to return.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set number of results to skip (for pagination).
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Filter by specific execution ID.
    pub fn for_execution(mut self, id: String) -> Self {
        self.execution_id = Some(id);
        self
    }

    /// Execute the query and return matching execution logs.
    ///
    /// # Arguments
    ///
    /// * `conn` - Database connection
    ///
    /// # Returns
    ///
    /// Returns a vector of execution logs matching the query criteria.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database query fails
    /// - JSON deserialization fails
    pub fn execute(&self, conn: &Connection) -> Result<Vec<ExecutionLog>> {
        let mut query = String::from(
            "SELECT id, execution_id, operation_type, status, timestamp, workspace,
             command_line, parameters, result_summary, error_details, duration_ms, created_at
             FROM execution_log WHERE 1=1",
        );

        let mut params_vec: Vec<String> = Vec::new();

        if let Some(ref op_type) = self.operation_type {
            query.push_str(" AND operation_type = ?");
            params_vec.push(op_type.clone());
        }

        if let Some(ref status) = self.status {
            query.push_str(" AND status = ?");
            params_vec.push(status.clone());
        }

        if let Some(after_ts) = self.after {
            query.push_str(" AND created_at >= ?");
            params_vec.push(after_ts.to_string());
        }

        if let Some(before_ts) = self.before {
            query.push_str(" AND created_at <= ?");
            params_vec.push(before_ts.to_string());
        }

        if let Some(ref exec_id) = self.execution_id {
            query.push_str(" AND execution_id = ?");
            params_vec.push(exec_id.clone());
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = self.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = self.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| SpliceError::Other(format!("failed to prepare query: {}", e)))?;

        let params: Vec<&dyn rusqlite::ToSql> = params_vec
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let logs = stmt
            .query_map(params.as_slice(), |row| {
                let parameters_json: Option<String> = row.get(6)?;
                let result_summary_json: Option<String> = row.get(7)?;
                let error_details_json: Option<String> = row.get(8)?;

                Ok(ExecutionLog {
                    id: row.get(0)?,
                    execution_id: row.get(1)?,
                    operation_type: row.get(2)?,
                    status: row.get(3)?,
                    timestamp: row.get(4)?,
                    workspace: row.get(5)?,
                    command_line: row.get(6)?,
                    parameters: parameters_json
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .or(Some(serde_json::Value::Null)),
                    result_summary: result_summary_json
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .or(Some(serde_json::Value::Null)),
                    error_details: error_details_json
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .or(Some(serde_json::Value::Null)),
                    duration_ms: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })
            .map_err(|e| SpliceError::Other(format!("query execution failed: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| SpliceError::Other(format!("row extraction failed: {}", e)))?;

        Ok(logs)
    }
}

impl Default for ExecutionQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary statistics for execution log.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionStats {
    /// Total number of operations in the log.
    pub total_operations: i64,
    /// Count of operations grouped by type (patch, delete, plan, etc.).
    pub by_type: HashMap<String, i64>,
    /// Count of operations grouped by status (ok, error, partial).
    pub by_status: HashMap<String, i64>,
    /// Timestamp of the oldest execution (ISO 8601).
    pub oldest_execution: Option<String>,
    /// Timestamp of the newest execution (ISO 8601).
    pub newest_execution: Option<String>,
}

/// Get execution by execution_id.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `execution_id` - Execution ID to look up
///
/// # Returns
///
/// Returns `Some(ExecutionLog)` if found, `None` if not found.
///
/// # Errors
///
/// Returns an error if database query fails.
pub fn get_execution(conn: &Connection, execution_id: &str) -> Result<Option<ExecutionLog>> {
    let query = "
        SELECT id, execution_id, operation_type, status, timestamp, workspace,
               command_line, parameters, result_summary, error_details, duration_ms, created_at
        FROM execution_log
        WHERE execution_id = ?
    ";

    let result = conn
        .query_row(query, params![execution_id], |row| {
            let parameters_json: Option<String> = row.get(6)?;
            let result_summary_json: Option<String> = row.get(7)?;
            let error_details_json: Option<String> = row.get(8)?;

            Ok(ExecutionLog {
                id: row.get(0)?,
                execution_id: row.get(1)?,
                operation_type: row.get(2)?,
                status: row.get(3)?,
                timestamp: row.get(4)?,
                workspace: row.get(5)?,
                command_line: row.get(6)?,
                parameters: parameters_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .or(Some(serde_json::Value::Null)),
                result_summary: result_summary_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .or(Some(serde_json::Value::Null)),
                error_details: error_details_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .or(Some(serde_json::Value::Null)),
                duration_ms: row.get(9)?,
                created_at: row.get(10)?,
            })
        })
        .optional();

    result.map_err(|e| SpliceError::Other(format!("failed to get execution: {}", e)))
}

/// Get recent executions.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `limit` - Maximum number of executions to return
///
/// # Returns
///
/// Returns a vector of the most recent execution logs.
///
/// # Errors
///
/// Returns an error if database query fails.
pub fn get_recent_executions(conn: &Connection, limit: usize) -> Result<Vec<ExecutionLog>> {
    ExecutionQuery::new().with_limit(limit).execute(conn)
}

/// Get execution statistics.
///
/// # Arguments
///
/// * `conn` - Database connection
///
/// # Returns
///
/// Returns aggregated statistics about all executions.
///
/// # Errors
///
/// Returns an error if database query fails.
pub fn get_execution_stats(conn: &Connection) -> Result<ExecutionStats> {
    // Get total count
    let total_operations: i64 = conn
        .query_row("SELECT COUNT(*) FROM execution_log", [], |row| row.get(0))
        .map_err(|e| SpliceError::Other(format!("failed to get total count: {}", e)))?;

    // Get count by operation type
    let mut by_type = HashMap::new();
    {
        let mut type_stmt = conn
            .prepare("SELECT operation_type, COUNT(*) FROM execution_log GROUP BY operation_type")
            .map_err(|e| SpliceError::Other(format!("failed to prepare type query: {}", e)))?;

        let type_rows = type_stmt
            .query_map([], |row| {
                let op_type: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((op_type, count))
            })
            .map_err(|e| SpliceError::Other(format!("failed to query by type: {}", e)))?;

        for (op_type, count) in type_rows.flatten() {
            by_type.insert(op_type, count);
        }
    }

    // Get count by status
    let mut by_status = HashMap::new();
    {
        let mut status_stmt = conn
            .prepare("SELECT status, COUNT(*) FROM execution_log GROUP BY status")
            .map_err(|e| SpliceError::Other(format!("failed to prepare status query: {}", e)))?;

        let status_rows = status_stmt
            .query_map([], |row| {
                let status: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((status, count))
            })
            .map_err(|e| SpliceError::Other(format!("failed to query by status: {}", e)))?;

        for (status, count) in status_rows.flatten() {
            by_status.insert(status, count);
        }
    }

    // Get oldest and newest timestamps
    let oldest_execution: Option<String> = conn
        .query_row(
            "SELECT timestamp FROM execution_log ORDER BY created_at ASC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| SpliceError::Other(format!("failed to get oldest execution: {}", e)))?;

    let newest_execution: Option<String> = conn
        .query_row(
            "SELECT timestamp FROM execution_log ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| SpliceError::Other(format!("failed to get newest execution: {}", e)))?;

    Ok(ExecutionStats {
        total_operations,
        by_type,
        by_status,
        oldest_execution,
        newest_execution,
    })
}

/// Format execution log as table row.
///
/// # Arguments
///
/// * `log` - Execution log to format
///
/// # Returns
///
/// Returns a formatted string suitable for table display.
pub fn format_table_row(log: &ExecutionLog) -> String {
    let duration = log
        .duration_ms
        .map_or("".to_string(), |ms| format!("{}ms", ms));

    // Truncate execution_id for display
    let id_short = &log.execution_id[..8.min(log.execution_id.len())];

    // Extract message from result_summary or use operation_type
    let message = log
        .result_summary
        .as_ref()
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or(&log.operation_type);

    format!(
        "{:<10} {:<8} {:<8} {:<20} {:<10} {}",
        id_short, log.operation_type, log.status, log.timestamp, duration, message
    )
}

/// Format execution logs as JSON.
///
/// # Arguments
///
/// * `logs` - Execution logs to format
///
/// # Returns
///
/// Returns a JSON string containing all logs.
///
/// # Errors
///
/// Returns an error if JSON serialization fails.
pub fn format_json(logs: &[ExecutionLog]) -> Result<String> {
    serde_json::to_string_pretty(&logs)
        .map_err(|e| SpliceError::Other(format!("failed to serialize logs to JSON: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::base::{
        init_execution_log_db, insert_execution_log, ExecutionLogBuilder,
    };
    use tempfile::TempDir;

    fn setup_test_db() -> (Connection, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_dir = temp_dir.path();
        let conn = init_execution_log_db(db_dir).unwrap();
        (conn, temp_dir)
    }

    fn insert_test_log(conn: &Connection, operation_type: &str, status: &str) -> String {
        let execution_id = uuid::Uuid::new_v4().to_string();
        let log = ExecutionLogBuilder::new(execution_id.clone(), operation_type.to_string())
            .status(status.to_string())
            .duration_ms(100)
            .build();
        insert_execution_log(conn, &log).unwrap();
        execution_id
    }

    #[test]
    fn test_query_builder_new() {
        let (conn, _temp) = setup_test_db();

        // Insert test data
        insert_test_log(&conn, "patch", "ok");
        insert_test_log(&conn, "delete", "error");

        let logs = ExecutionQuery::new().execute(&conn).unwrap();
        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn test_query_by_operation_type() {
        let (conn, _temp) = setup_test_db();

        insert_test_log(&conn, "patch", "ok");
        insert_test_log(&conn, "delete", "ok");
        insert_test_log(&conn, "patch", "error");

        let logs = ExecutionQuery::new()
            .with_operation_type("patch".to_string())
            .execute(&conn)
            .unwrap();

        assert_eq!(logs.len(), 2);
        assert!(logs.iter().all(|l| l.operation_type == "patch"));
    }

    #[test]
    fn test_query_by_status() {
        let (conn, _temp) = setup_test_db();

        insert_test_log(&conn, "patch", "ok");
        insert_test_log(&conn, "delete", "error");
        insert_test_log(&conn, "plan", "ok");

        let logs = ExecutionQuery::new()
            .with_status("ok".to_string())
            .execute(&conn)
            .unwrap();

        assert_eq!(logs.len(), 2);
        assert!(logs.iter().all(|l| l.status == "ok"));
    }

    #[test]
    fn test_query_date_range() {
        let (conn, _temp) = setup_test_db();

        let now = chrono::Utc::now().timestamp();

        insert_test_log(&conn, "patch", "ok");

        let logs = ExecutionQuery::new()
            .after(now - 10)
            .before(now + 10)
            .execute(&conn)
            .unwrap();

        assert_eq!(logs.len(), 1);
    }

    #[test]
    fn test_query_with_limit_offset() {
        let (conn, _temp) = setup_test_db();

        for _ in 0..5 {
            insert_test_log(&conn, "patch", "ok");
        }

        let logs = ExecutionQuery::new().with_limit(2).execute(&conn).unwrap();

        assert_eq!(logs.len(), 2);

        let logs2 = ExecutionQuery::new()
            .with_limit(2)
            .with_offset(2)
            .execute(&conn)
            .unwrap();

        assert_eq!(logs2.len(), 2);
        assert_ne!(logs[0].execution_id, logs2[0].execution_id);
    }

    #[test]
    fn test_get_execution_by_id() {
        let (conn, _temp) = setup_test_db();

        let execution_id = insert_test_log(&conn, "patch", "ok");

        let log = get_execution(&conn, &execution_id).unwrap();
        assert!(log.is_some());
        assert_eq!(log.unwrap().execution_id, execution_id);

        // Test non-existent ID
        let log2 = get_execution(&conn, "non-existent").unwrap();
        assert!(log2.is_none());
    }

    #[test]
    fn test_execution_stats() {
        let (conn, _temp) = setup_test_db();

        insert_test_log(&conn, "patch", "ok");
        insert_test_log(&conn, "delete", "error");
        insert_test_log(&conn, "patch", "ok");
        insert_test_log(&conn, "plan", "partial");

        let stats = get_execution_stats(&conn).unwrap();

        assert_eq!(stats.total_operations, 4);
        assert_eq!(stats.by_type.get("patch"), Some(&2));
        assert_eq!(stats.by_type.get("delete"), Some(&1));
        assert_eq!(stats.by_type.get("plan"), Some(&1));
        assert_eq!(stats.by_status.get("ok"), Some(&2));
        assert_eq!(stats.by_status.get("error"), Some(&1));
        assert_eq!(stats.by_status.get("partial"), Some(&1));
        assert!(stats.oldest_execution.is_some());
        assert!(stats.newest_execution.is_some());
    }

    #[test]
    fn test_format_functions() {
        let execution_id = uuid::Uuid::new_v4().to_string();
        let log = ExecutionLogBuilder::new(execution_id.clone(), "patch".to_string())
            .status("ok".to_string())
            .duration_ms(123)
            .result_summary(serde_json::json!({"message": "Patched function foo"}))
            .build();

        let table_row = format_table_row(&log);
        assert!(table_row.contains("patch"));
        assert!(table_row.contains("ok"));
        assert!(table_row.contains("123ms"));

        let json_output = format_json(&[log]).unwrap();
        assert!(json_output.contains("patch"));
        assert!(json_output.contains("123"));
    }
}
