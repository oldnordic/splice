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
        /// The file path that caused the I/O error.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// SQLiteGraph backend error.
    #[error("Graph error: {0}")]
    Graph(#[from] sqlitegraph::SqliteGraphError),

    /// Tree-sitter parsing error.
    #[error("Parse error in {file}: {message}")]
    Parse {
        /// The file that failed to parse.
        file: PathBuf,
        /// The parse error message.
        message: String,
    },

    /// Symbol not found in graph.
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    /// Symbol name is ambiguous without file context.
    #[error("Ambiguous symbol '{name}': found in multiple files: {files:?}")]
    AmbiguousSymbol {
        /// The ambiguous symbol name.
        name: String,
        /// List of files where this symbol was found.
        files: Vec<String>,
    },

    /// Reference finding failed.
    #[error("Failed to find references for symbol '{name}': {reason}")]
    ReferenceFailed {
        /// The symbol name.
        name: String,
        /// Reason for failure.
        reason: String,
    },

    /// Ambiguous reference detected.
    #[error(
        "Ambiguous reference to '{name}' at {file}:{line}:{col} - could refer to {candidates:?}"
    )]
    AmbiguousReference {
        /// The symbol name.
        name: String,
        /// File containing the ambiguous reference.
        file: String,
        /// Line number.
        line: usize,
        /// Column number.
        col: usize,
        /// Candidate definitions.
        candidates: Vec<String>,
    },

    /// Invalid byte span.
    #[error("Invalid span ({start}, {end}) in {file}")]
    InvalidSpan {
        /// The file containing the invalid span.
        file: PathBuf,
        /// Start byte offset.
        start: usize,
        /// End byte offset.
        end: usize,
    },

    /// Compiler validation failed.
    #[error("Compiler error: {0}")]
    CompilerError(String),

    /// Tree-sitter parse validation failed after patch.
    #[error("Parse validation failed: file '{file}' - {message}")]
    ParseValidationFailed {
        /// The file that failed validation.
        file: std::path::PathBuf,
        /// The validation error message.
        message: String,
    },

    /// Cargo check failed after patch.
    #[error("Cargo check failed in workspace '{workspace}': {output}")]
    CargoCheckFailed {
        /// The workspace directory.
        workspace: std::path::PathBuf,
        /// The cargo check output.
        output: String,
    },

    /// rust-analyzer not available.
    #[error("rust-analyzer not found: {mode}")]
    AnalyzerNotAvailable {
        /// The analyzer mode that was requested.
        mode: String,
    },

    /// rust-analyzer validation failed.
    #[error("rust-analyzer reported diagnostics: {output}")]
    AnalyzerFailed {
        /// The analyzer output.
        output: String,
    },

    /// Invalid plan schema.
    #[error("Invalid plan schema: {message}")]
    InvalidPlanSchema {
        /// The schema validation error message.
        message: String,
    },

    /// Plan execution failed at step.
    #[error("Plan execution failed at step {step}: {error}")]
    PlanExecutionFailed {
        /// The step number that failed.
        step: usize,
        /// The error that occurred.
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
