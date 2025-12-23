//! Splice error types.
//!
//! All errors are typed and provide root cause information.

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for Splice operations.
#[derive(Error, Debug)]
pub enum SpliceError {
    /// I/O error during file operations.
    #[error("I/O error for path {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// SQLiteGraph backend error.
    #[error("Graph error: {0}")]
    Graph(#[from] sqlitegraph::SqliteGraphError),

    /// Tree-sitter parsing error.
    #[error("Parse error in {file}: {message}")]
    Parse {
        file: PathBuf,
        message: String,
    },

    /// Symbol not found in graph.
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    /// Symbol name is ambiguous without file context.
    #[error("Ambiguous symbol '{name}': found in multiple files: {files:?}")]
    AmbiguousSymbol {
        name: String,
        files: Vec<String>,
    },

    /// Invalid byte span.
    #[error("Invalid span ({start}, {end}) in {file}")]
    InvalidSpan {
        file: PathBuf,
        start: usize,
        end: usize,
    },

    /// Compiler validation failed.
    #[error("Compiler error: {0}")]
    CompilerError(String),

    /// Tree-sitter parse validation failed after patch.
    #[error("Parse validation failed: file '{file}' - {message}")]
    ParseValidationFailed {
        file: std::path::PathBuf,
        message: String,
    },

    /// Cargo check failed after patch.
    #[error("Cargo check failed in workspace '{workspace}': {output}")]
    CargoCheckFailed {
        workspace: std::path::PathBuf,
        output: String,
    },

    /// rust-analyzer not available.
    #[error("rust-analyzer not found: {mode}")]
    AnalyzerNotAvailable {
        mode: String,
    },

    /// rust-analyzer validation failed.
    #[error("rust-analyzer reported diagnostics: {output}")]
    AnalyzerFailed {
        output: String,
    },

    /// Invalid plan schema.
    #[error("Invalid plan schema: {message}")]
    InvalidPlanSchema {
        message: String,
    },

    /// Plan execution failed at step.
    #[error("Plan execution failed at step {step}: {error}")]
    PlanExecutionFailed {
        step: usize,
        error: String,
    },

    /// UTF-8 validation error.
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    /// Generic error with context.
    #[error("{0}")]
    Other(String),
}

impl From<std::io::Error> for SpliceError {
    fn from(err: std::io::Error) -> Self {
        SpliceError::Io {
            path: PathBuf::from("<unknown>"),
            source: err,
        }
    }
}

/// Result type alias for Splice operations.
pub type Result<T> = std::result::Result<T, SpliceError>;
