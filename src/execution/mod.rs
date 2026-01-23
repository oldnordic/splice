//! Execution log infrastructure for Splice operations.
//!
//! This module provides persistent audit trail storage for all Splice operations.
//! Execution logs are stored in a separate SQLite database (`.splice/operations.db`)
//! to enable independent management from the code graph database.

pub mod base;
pub mod log;
pub mod query;

// Re-export commonly used types from base module
pub use base::{
    init_execution_log_db, insert_execution_log, ExecutionLog, ExecutionLogBuilder, DB_FILENAME,
};

// Re-export query types
pub use query::{
    format_json, format_table_row, get_execution, get_execution_stats, get_recent_executions,
    ExecutionQuery, ExecutionStats,
};
