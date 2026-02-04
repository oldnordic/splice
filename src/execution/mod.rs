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
    generate_delegated_execution_id, init_execution_log_db, insert_execution_log, ExecutionLog,
    ExecutionLogBuilder, DB_FILENAME,
};

// Re-export query types
pub use query::{
    format_json, format_table_row, get_execution, get_execution_stats, get_recent_executions,
    ExecutionQuery, ExecutionStats,
};

// Re-export log types
pub use log::{
    db_path, init_db, init_db_in_dir, is_enabled, is_enabled_with_config, record_execution,
    record_execution_failure, record_execution_with_params, ExecutionLogConfig,
};
