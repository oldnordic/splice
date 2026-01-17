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
    DB_FILENAME,
    ExecutionLog,
    ExecutionLogBuilder,
    init_execution_log_db,
    insert_execution_log,
};

// Re-export query types
pub use query::{
    ExecutionQuery,
    ExecutionStats,
    get_execution,
    get_recent_executions,
    get_execution_stats,
    format_table_row,
    format_json,
};
