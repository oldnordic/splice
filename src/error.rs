//! Splice error types.
//!
//! All errors are typed and provide root cause information.

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

    /// Kind identifier for structured logging / CLI output.
    pub fn kind(&self) -> &'static str {
        match self {
            SpliceError::Io { .. } => "Io",
            SpliceError::Graph(_) => "Graph",
            SpliceError::Parse { .. } => "Parse",
            SpliceError::SymbolNotFound { .. } => "SymbolNotFound",
            SpliceError::AmbiguousSymbol { .. } => "AmbiguousSymbol",
            SpliceError::ReferenceFailed { .. } => "ReferenceFailed",
            SpliceError::AmbiguousReference { .. } => "AmbiguousReference",
            SpliceError::InvalidSpan { .. } => "InvalidSpan",
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
            SpliceError::ParseValidationFailed { file, .. } => Some(file.as_path()),
            SpliceError::CargoCheckFailed { workspace, .. } => Some(workspace.as_path()),
            SpliceError::CompilerValidationFailed { file, .. } => Some(file.as_path()),
            SpliceError::SymbolNotFound {
                file: Some(file), ..
            } => Some(file.as_path()),
            SpliceError::AmbiguousReference { file, .. } => Some(Path::new(file)),
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
