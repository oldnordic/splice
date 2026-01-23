//! Splice error types.
//!
//! All errors are typed and provide root cause information.

use crate::suggestions;
use crate::validate;
use std::path::{Path, PathBuf};
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

    /// Generic I/O error with context string.
    #[error("I/O error: {context}")]
    IoContext {
        /// Contextual description of the operation.
        context: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Stdout closed early (broken pipe).
    #[error("stdout pipe closed")]
    BrokenPipe,

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
    #[error("{message}")]
    SymbolNotFound {
        /// Human-readable message with file context if available.
        message: String,
        /// Symbol name that was requested.
        symbol: String,
        /// Optional file path where lookup happened.
        file: Option<PathBuf>,
        /// Actionable hint for CLI display.
        hint: String,
    },

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
    #[error("Invalid span ({start}, {end}) in {file} (file size: {file_size})")]
    InvalidSpan {
        /// The file containing the invalid span.
        file: PathBuf,
        /// Start byte offset.
        start: usize,
        /// End byte offset.
        end: usize,
        /// File size (bytes).
        file_size: usize,
    },

    /// Invalid line range.
    #[error(
        "Invalid line range ({line_start}, {line_end}) in {file} (total lines: {total_lines})"
    )]
    InvalidLineRange {
        /// The file containing the invalid range.
        file: PathBuf,
        /// Start line (1-based).
        line_start: usize,
        /// End line (1-based).
        line_end: usize,
        /// Total lines in file.
        total_lines: usize,
    },

    /// Invalid UTF-8 content.
    #[error("Invalid UTF-8 in file {file}: {source}")]
    InvalidUtf8 {
        /// The file with invalid UTF-8.
        file: PathBuf,
        /// The underlying UTF-8 error.
        #[source]
        source: std::str::Utf8Error,
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
    #[error("Cargo check failed in workspace '{workspace}'")]
    CargoCheckFailed {
        /// The workspace directory.
        workspace: std::path::PathBuf,
        /// The cargo check output (raw string).
        output: String,
        /// Parsed diagnostics from cargo output.
        diagnostics: Vec<Diagnostic>,
    },

    /// Compiler validation failed (multi-language).
    #[error("Compiler validation failed for {language} in file '{file}'")]
    CompilerValidationFailed {
        /// The file that failed validation.
        file: std::path::PathBuf,
        /// The programming language.
        language: String,
        /// Structured diagnostics returned by the compiler.
        diagnostics: Vec<Diagnostic>,
    },

    /// rust-analyzer not available.
    #[error("rust-analyzer not found: {mode}")]
    AnalyzerNotAvailable {
        /// The analyzer mode that was requested.
        mode: String,
    },

    /// rust-analyzer validation failed.
    #[error("rust-analyzer reported diagnostics")]
    AnalyzerFailed {
        /// The analyzer output (raw text).
        output: String,
        /// Structured diagnostics parsed from the output.
        diagnostics: Vec<Diagnostic>,
    },

    /// Invalid plan schema.
    #[error("Invalid plan schema: {message}")]
    InvalidPlanSchema {
        /// The schema validation error message.
        message: String,
    },

    /// Invalid batch JSON schema.
    #[error("Invalid batch schema: {message}")]
    InvalidBatchSchema {
        /// The validation error message.
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

    /// Pre-verification failed.
    #[error("Pre-verification failed: {check}")]
    PreVerificationFailed {
        /// The check that failed.
        check: String,
    },

    /// File was externally modified.
    #[error("File modified externally: {file}")]
    FileExternallyModified {
        /// The file that was modified.
        file: String,
    },

    /// Insufficient disk space.
    #[error("Insufficient disk space: needed {needed} bytes, available {available} bytes")]
    InsufficientDiskSpace {
        /// Bytes needed.
        needed: u64,
        /// Bytes available.
        available: u64,
    },

    /// Execution log database error.
    #[error("Execution log database error: {message}")]
    ExecutionLogError {
        /// Error message describing what went wrong.
        message: String,
        /// Underlying error source.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Failed to record execution in log.
    #[error("Failed to record execution: {execution_id}")]
    ExecutionRecordFailed {
        /// Execution ID that failed to record.
        execution_id: String,
        /// Underlying error source.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Execution log entry not found.
    #[error("Execution log not found: {execution_id}")]
    ExecutionNotFound {
        /// Execution ID that was not found.
        execution_id: String,
    },

    /// Invalid date format provided for query.
    #[error("Invalid date format: {input}")]
    InvalidDateFormat {
        /// The input that could not be parsed.
        input: String,
    },

    /// Query execution error.
    #[error("Query error: {message}")]
    QueryError {
        /// Error message describing what went wrong.
        message: String,
        /// Underlying error source.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
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

/// Severity level for diagnostics emitted by validation gates.
#[derive(Debug, Clone, Copy)]
pub enum DiagnosticLevel {
    /// An error that blocks execution.
    Error,
    /// A warning emitted by the underlying tool.
    Warning,
    /// Informational note from compiler.
    Note,
    /// Help message from compiler.
    Help,
}

impl DiagnosticLevel {
    /// Convert diagnostic level to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Note => "note",
            DiagnosticLevel::Help => "help",
        }
    }
}

/// Structured diagnostic emitted by compilers or analyzers.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Tool that produced this diagnostic (cargo-check, tree-sitter, etc.).
    pub tool: String,
    /// Severity level.
    pub level: DiagnosticLevel,
    /// Diagnostic message.
    pub message: String,
    /// Optional file path for the diagnostic.
    pub file: Option<PathBuf>,
    /// Optional 1-based line number.
    pub line: Option<usize>,
    /// Optional 0-based column number.
    pub column: Option<usize>,
    /// Optional compiler/analyzer error code.
    pub code: Option<String>,
    /// Optional hint/help text provided by the tool.
    pub note: Option<String>,
    /// Optional fully-resolved tool path.
    pub tool_path: Option<PathBuf>,
    /// Optional tool version string.
    pub tool_version: Option<String>,
    /// Optional remediation link or message.
    pub remediation: Option<String>,
}

impl Diagnostic {
    /// Construct a diagnostic with the provided fields.
    pub fn new(
        tool: impl Into<String>,
        level: DiagnosticLevel,
        message: impl Into<String>,
    ) -> Self {
        Self {
            tool: tool.into(),
            level,
            message: message.into(),
            file: None,
            line: None,
            column: None,
            code: None,
            note: None,
            tool_path: None,
            tool_version: None,
            remediation: None,
        }
    }

    /// Attach file information.
    pub fn with_file(mut self, file: PathBuf) -> Self {
        self.file = Some(file);
        self
    }

    /// Attach optional line/column info.
    pub fn with_position(mut self, line: Option<usize>, column: Option<usize>) -> Self {
        self.line = line;
        self.column = column;
        self
    }

    /// Attach optional diagnostic code.
    pub fn with_code(mut self, code: Option<String>) -> Self {
        self.code = code;
        self
    }

    /// Attach optional hint/help text.
    pub fn with_note(mut self, note: Option<String>) -> Self {
        self.note = note;
        self
    }

    /// Attach tool metadata (path/version).
    pub fn with_tool_metadata(mut self, metadata: Option<&crate::validate::ToolMetadata>) -> Self {
        if let Some(meta) = metadata {
            self.tool_path = meta.path.clone();
            self.tool_version = meta.version.clone();
        }
        self
    }

    /// Attach remediation link or text.
    pub fn with_remediation(mut self, remediation: Option<String>) -> Self {
        self.remediation = remediation;
        self
    }
}

impl SpliceError {
    /// Helper for constructing a SymbolNotFound variant with standard messaging.
    pub fn symbol_not_found(symbol: impl Into<String>, file: Option<&Path>) -> Self {
        let symbol = symbol.into();
        let (message, hint) = match file {
            Some(path) => (
                format!("Symbol '{}' not found in {}", symbol, path.display()),
                format!(
                    "Ensure '{}' exists in {} or adjust the --symbol flag",
                    symbol,
                    path.display()
                ),
            ),
            None => (
                format!("Symbol '{}' not found", symbol),
                format!(
                    "Ensure '{}' is ingested and spelled correctly or pass --file",
                    symbol
                ),
            ),
        };

        SpliceError::SymbolNotFound {
            message,
            symbol,
            file: file.map(|p| p.to_path_buf()),
            hint,
        }
    }

    /// Create SymbolNotFound with fuzzy match suggestions.
    ///
    /// Uses Levenshtein distance to find similar symbol names and include
    /// them in the error hint. This provides "did you mean" functionality.
    ///
    /// # Arguments
    /// * `symbol` - The symbol name that was not found
    /// * `file` - Optional file path for context
    /// * `candidates` - All available symbol names for fuzzy matching
    pub fn symbol_not_found_with_suggestions(
        symbol: impl Into<String>,
        file: Option<&Path>,
        candidates: &[String],
    ) -> Self {
        let symbol = symbol.into();
        let (message, hint) = if let Some(path) = file {
            (
                format!("Symbol '{}' not found in {}", symbol, path.display()),
                format!(
                    "Ensure '{}' exists in {} or adjust the --symbol flag",
                    symbol,
                    path.display()
                ),
            )
        } else {
            (format!("Symbol '{}' not found", symbol), String::new())
        };

        // Try to find similar symbols
        let suggestions = suggestions::suggest_similar_symbols(&symbol, candidates, 3);

        let hint = if suggestions.is_empty() {
            if hint.is_empty() {
                format!(
                    "Symbol '{}' not found. Run `splice ingest` to index the codebase.",
                    symbol
                )
            } else {
                hint
            }
        } else {
            format!("Did you mean: {}?", suggestions.join(", "))
        };

        SpliceError::SymbolNotFound {
            message,
            symbol,
            file: file.map(|p| p.to_path_buf()),
            hint,
        }
    }

    /// Kind identifier for structured logging / CLI output.
    pub fn kind(&self) -> &'static str {
        match self {
            SpliceError::Io { .. } | SpliceError::IoContext { .. } => "Io",
            SpliceError::BrokenPipe => "BrokenPipe",
            SpliceError::Graph(_) => "Graph",
            SpliceError::Parse { .. } => "Parse",
            SpliceError::SymbolNotFound { .. } => "SymbolNotFound",
            SpliceError::AmbiguousSymbol { .. } => "AmbiguousSymbol",
            SpliceError::ReferenceFailed { .. } => "ReferenceFailed",
            SpliceError::AmbiguousReference { .. } => "AmbiguousReference",
            SpliceError::InvalidSpan { .. } => "InvalidSpan",
            SpliceError::InvalidLineRange { .. } => "InvalidLineRange",
            SpliceError::InvalidUtf8 { .. } => "InvalidUtf8",
            SpliceError::CompilerError(_) => "CompilerError",
            SpliceError::ParseValidationFailed { .. } => "ParseValidationFailed",
            SpliceError::CargoCheckFailed { .. } => "CargoCheckFailed",
            SpliceError::CompilerValidationFailed { .. } => "CompilerValidationFailed",
            SpliceError::AnalyzerNotAvailable { .. } => "AnalyzerNotAvailable",
            SpliceError::AnalyzerFailed { .. } => "AnalyzerFailed",
            SpliceError::InvalidPlanSchema { .. } => "InvalidPlanSchema",
            SpliceError::InvalidBatchSchema { .. } => "InvalidBatchSchema",
            SpliceError::PlanExecutionFailed { .. } => "PlanExecutionFailed",
            SpliceError::Utf8(_) => "Utf8",
            SpliceError::Other(_) => "Other",
            SpliceError::PreVerificationFailed { .. } => "PreVerificationFailed",
            SpliceError::FileExternallyModified { .. } => "FileExternallyModified",
            SpliceError::InsufficientDiskSpace { .. } => "InsufficientDiskSpace",
            SpliceError::ExecutionLogError { .. } => "ExecutionLogError",
            SpliceError::ExecutionRecordFailed { .. } => "ExecutionRecordFailed",
            SpliceError::ExecutionNotFound { .. } => "ExecutionNotFound",
            SpliceError::InvalidDateFormat { .. } => "InvalidDateFormat",
            SpliceError::QueryError { .. } => "QueryError",
        }
    }

    /// Optional symbol context for structured output.
    pub fn symbol(&self) -> Option<&str> {
        match self {
            SpliceError::SymbolNotFound { symbol, .. } => Some(symbol.as_str()),
            SpliceError::AmbiguousSymbol { name, .. } => Some(name.as_str()),
            SpliceError::ReferenceFailed { name, .. } => Some(name.as_str()),
            SpliceError::AmbiguousReference { name, .. } => Some(name.as_str()),
            _ => None,
        }
    }

    /// Optional path context for structured output.
    pub fn file_path(&self) -> Option<&Path> {
        match self {
            SpliceError::Parse { file, .. } => Some(file.as_path()),
            SpliceError::InvalidSpan { file, .. } => Some(file.as_path()),
            SpliceError::InvalidLineRange { file, .. } => Some(file.as_path()),
            SpliceError::InvalidUtf8 { file, .. } => Some(file.as_path()),
            SpliceError::ParseValidationFailed { file, .. } => Some(file.as_path()),
            SpliceError::CargoCheckFailed { workspace, .. } => Some(workspace.as_path()),
            SpliceError::CompilerValidationFailed { file, .. } => Some(file.as_path()),
            SpliceError::SymbolNotFound {
                file: Some(file), ..
            } => Some(file.as_path()),
            SpliceError::AmbiguousReference { file, .. } => Some(Path::new(file)),
            SpliceError::FileExternallyModified { file } => Some(Path::new(file)),
            _ => None,
        }
    }

    /// Optional hint for CLI consumers.
    pub fn hint(&self) -> Option<&str> {
        match self {
            SpliceError::SymbolNotFound { hint, .. } => Some(hint.as_str()),
            SpliceError::AmbiguousSymbol { .. } => {
                Some("Pass --file to disambiguate symbols defined in multiple files")
            }
            SpliceError::AmbiguousReference { .. } => {
                Some("Qualify the reference to resolve ambiguity")
            }
            SpliceError::ReferenceFailed { .. } => {
                Some("Check ingest logs; reference resolver could not complete")
            }
            _ => None,
        }
    }

    /// Extract location (file, line, column) from error if available.
    ///
    /// Returns (Option<&str>, Option<usize>, Option<usize>) for (file, line, column).
    /// Line is 1-based, column is 0-based (matching compiler conventions).
    pub fn location(&self) -> (Option<&str>, Option<usize>, Option<usize>) {
        match self {
            // Errors with explicit line/column
            SpliceError::AmbiguousReference {
                file, line, col, ..
            } => (Some(file.as_str()), Some(*line), Some(*col)),

            // Errors with file but no line/column (use None for now)
            SpliceError::Parse { file, .. }
            | SpliceError::InvalidSpan { file, .. }
            | SpliceError::InvalidLineRange { file, .. }
            | SpliceError::InvalidUtf8 { file, .. }
            | SpliceError::ParseValidationFailed { file, .. }
            | SpliceError::CompilerValidationFailed { file, .. } => {
                (Some(file.to_str().unwrap_or("<invalid>")), None, None)
            }

            // SymbolNotFound with optional file
            SpliceError::SymbolNotFound { file, .. } => file
                .as_ref()
                .and_then(|f| f.to_str())
                .map(|f| (Some(f), None, None))
                .unwrap_or((None, None, None)),

            // Errors with path context
            SpliceError::FileExternallyModified { file } => (Some(file.as_str()), None, None),

            // No location available
            _ => (None, None, None),
        }
    }

    /// Structured diagnostics emitted by this error, if available.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        match self {
            SpliceError::ParseValidationFailed { file, message } => {
                vec![
                    Diagnostic::new("tree-sitter", DiagnosticLevel::Error, message.clone())
                        .with_file(file.clone()),
                ]
            }
            SpliceError::CompilerValidationFailed { diagnostics, .. } => diagnostics.clone(),
            SpliceError::CargoCheckFailed {
                workspace,
                output,
                diagnostics,
            } => {
                if diagnostics.is_empty() {
                    vec![
                        Diagnostic::new("cargo-check", DiagnosticLevel::Error, output.clone())
                            .with_file(workspace.clone()),
                    ]
                } else {
                    diagnostics.clone()
                }
            }
            SpliceError::AnalyzerFailed {
                output,
                diagnostics,
            } => {
                if diagnostics.is_empty() {
                    vec![
                        Diagnostic::new("rust-analyzer", DiagnosticLevel::Error, output.clone())
                            .with_file(PathBuf::from("<workspace>")),
                    ]
                } else {
                    diagnostics.clone()
                }
            }
            SpliceError::CompilerError(message) => {
                vec![Diagnostic::new(
                    "compiler",
                    DiagnosticLevel::Error,
                    message.clone(),
                )]
            }
            _ => Vec::new(),
        }
    }
}

impl From<validate::ErrorLevel> for DiagnosticLevel {
    fn from(level: validate::ErrorLevel) -> Self {
        match level {
            validate::ErrorLevel::Error => DiagnosticLevel::Error,
            validate::ErrorLevel::Warning => DiagnosticLevel::Warning,
            validate::ErrorLevel::Note => DiagnosticLevel::Note,
            validate::ErrorLevel::Help => DiagnosticLevel::Help,
        }
    }
}

// Context helper extensions for SpliceError
impl SpliceError {
    /// Add additional context to this error.
    ///
    /// This method provides a chainable way to add context to errors,
    /// similar to anyhow's context. It returns the same error with
    /// an enhanced message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use splice::error::{Result, SpliceError};
    ///
    /// fn do_something() -> Result<()> {
    ///     let err = SpliceError::Other("base error".to_string())
    ///         .with_context("while doing important work");
    ///     Err(err)
    /// }
    /// ```
    pub fn with_context(self, context: impl Into<String>) -> Self {
        let ctx = context.into();
        match self {
            SpliceError::Other(msg) => SpliceError::Other(format!("{}: {}", ctx, msg)),
            SpliceError::Parse { file, message } => SpliceError::Parse {
                file,
                message: format!("{}: {}", ctx, message),
            },
            SpliceError::Io { path, source } => SpliceError::Io { path, source },
            SpliceError::IoContext { context: _, source } => SpliceError::IoContext {
                context: ctx,
                source,
            },
            other => other,
        }
    }

    /// Attach a path to this error.
    ///
    /// This is useful for errors that occur during file operations
    /// where the path isn't already attached.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use splice::error::{Result, SpliceError};
    /// use std::path::Path;
    ///
    /// fn process_file(path: &Path) -> Result<()> {
    ///     let err = SpliceError::Other("failed".to_string()).with_path(path);
    ///     Err(err)
    /// }
    /// ```
    pub fn with_path(self, path: impl AsRef<std::path::Path>) -> Self {
        let path_buf = path.as_ref().to_path_buf();
        match self {
            SpliceError::Other(msg) => SpliceError::Parse {
                file: path_buf.clone(),
                message: msg,
            },
            SpliceError::Parse { file: _, message } => SpliceError::Parse {
                file: path_buf,
                message,
            },
            SpliceError::SymbolNotFound {
                message,
                symbol,
                file: _,
                hint,
            } => SpliceError::SymbolNotFound {
                message,
                symbol,
                file: Some(path_buf),
                hint,
            },
            other => other,
        }
    }

    /// Create an I/O error with path context.
    pub fn io_with_path(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        SpliceError::Io {
            path: path.into(),
            source,
        }
    }

    /// Create a parse error with file context.
    pub fn parse_with_file(file: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        SpliceError::Parse {
            file: file.into(),
            message: message.into(),
        }
    }

    /// Create an "Other" error with a message.
    pub fn other(msg: impl Into<String>) -> Self {
        SpliceError::Other(msg.into())
    }
}

/// Convert byte offset to line/column position.
///
/// Line is 1-based (first line is 1), column is 0-based (first column is 0).
/// This matches compiler conventions (rustc, clang, tsc).
///
/// # Arguments
/// * `source` - Full source text
/// * `byte_offset` - Byte offset within source
///
/// # Returns
/// * (line, column) where line is 1-based and column is 0-based
pub fn byte_offset_to_line_column(source: &str, byte_offset: usize) -> (usize, usize) {
    let text = &source[..byte_offset.min(source.len())];

    // Count newlines to get line number (1-based)
    let line = text.matches('\n').count() + 1;

    // Get column: if we end on a newline, column is 0 (start of next line)
    // Otherwise, count characters in the current line
    let column = if text.ends_with('\n') {
        0
    } else {
        text.lines().last().map(|l| l.chars().count()).unwrap_or(0)
    };

    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_offset_to_line_column() {
        let source = "fn main() {\n    println!(\"hello\");\n}";

        // Offset 0: start of file, line 1, column 0
        assert_eq!(byte_offset_to_line_column(source, 0), (1, 0));

        // Offset 10: after '{' character, still on line 1, column 10
        assert_eq!(byte_offset_to_line_column(source, 10), (1, 10));

        // Offset 11: before '\n' (still on line 1), at position of '{'
        assert_eq!(byte_offset_to_line_column(source, 11), (1, 11));

        // Offset 12: after '\n' (on line 2), column 0
        assert_eq!(byte_offset_to_line_column(source, 12), (2, 0));

        // Offset 13: first space on line 2, line 2, column 1
        assert_eq!(byte_offset_to_line_column(source, 13), (2, 1));

        // Offset 16: first 'p' of "println" (after 4 spaces), line 2, column 4
        assert_eq!(byte_offset_to_line_column(source, 16), (2, 4));

        // Test multi-line behavior
        let multi_line = "line 1\nline 2\nline 3\n";
        assert_eq!(byte_offset_to_line_column(multi_line, 0), (1, 0));
        assert_eq!(byte_offset_to_line_column(multi_line, 6), (1, 6)); // before first \n
        assert_eq!(byte_offset_to_line_column(multi_line, 7), (2, 0)); // after first \n
        assert_eq!(byte_offset_to_line_column(multi_line, 13), (2, 6)); // before second \n
        assert_eq!(byte_offset_to_line_column(multi_line, 14), (3, 0)); // after second \n
    }
}
