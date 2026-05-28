use serde::{Deserialize, Serialize};

/// Error code with severity, location, and hint for diagnostics.
///
/// This struct is already defined in src/output.rs for JSON serialization.
/// This module provides the registry and construction functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCode {
    /// Error code (e.g., "SPL-E001")
    pub code: String,
    /// Severity level (error/warning/note)
    pub severity: String,
    /// Precise location (file:line:column), absent when no location is available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// What to do hint
    pub hint: String,
}

impl ErrorCode {
    /// Create a new error code.
    pub fn new(
        code: impl Into<String>,
        severity: impl Into<String>,
        location: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: severity.into(),
            location: Some(location.into()),
            hint: hint.into(),
        }
    }

    /// Create from SpliceErrorCode enum.
    pub fn from_splice_code(
        splice_code: SpliceErrorCode,
        file: Option<&str>,
        line: Option<usize>,
        column: Option<usize>,
    ) -> Self {
        let location = match (file, line, column) {
            (Some(f), Some(l), Some(c)) => Some(format!("{}:{}:{}", f, l, c)),
            (Some(f), Some(l), None) => Some(format!("{}:{}", f, l)),
            (Some(f), None, Some(c)) => Some(format!("{}:{}", f, c)),
            (Some(f), None, None) => Some(f.to_string()),
            (None, _, _) => None,
        };

        Self {
            code: splice_code.code(),
            severity: splice_code.severity(),
            location,
            hint: splice_code.hint(),
        }
    }
}

/// Severity level for error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Error - operation failed
    Error,
    /// Warning - potential issue
    Warning,
    /// Note - informational
    Note,
}

impl ErrorSeverity {
    /// Convert to string identifier.
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorSeverity::Error => "error",
            ErrorSeverity::Warning => "warning",
            ErrorSeverity::Note => "note",
        }
    }
}

/// Splice error code registry.
///
/// Error codes follow the format SPL-{E|W|N}-### where:
/// - SPL is the tool identifier
/// - E (error), W (warning), or N (note) indicates severity
/// - ### is a sequential number
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceErrorCode {
    // Symbol resolution errors (SPL-E001 to SPL-E010)
    /// Symbol not found in codebase (SPL-E001)
    SymbolNotFound,
    /// Symbol name is ambiguous without file context (SPL-E002)
    AmbiguousSymbol,
    /// Failed to locate symbol references (SPL-E003)
    ReferenceFailed,
    /// Reference could refer to multiple definitions (SPL-E004)
    AmbiguousReference,

    // Parse/AST errors (SPL-E011 to SPL-E020)
    /// Tree-sitter parsing error (SPL-E011)
    ParseError,
    /// Invalid UTF-8 encoding (SPL-E012)
    InvalidUtf8,
    /// Compiler syntax error (SPL-E013)
    InvalidSyntax,

    // Span errors (SPL-E021 to SPL-E030)
    /// Invalid byte span (SPL-E021)
    InvalidSpan,
    /// Invalid line range (SPL-E022)
    InvalidLineRange,
    /// Span extends beyond file bounds (SPL-E023)
    SpanOutOfBounds,

    // I/O errors (SPL-E031 to SPL-E040)
    /// Failed to read file (SPL-E031)
    FileReadError,
    /// Failed to write file (SPL-E032)
    FileWriteError,
    /// File not found (SPL-E033)
    FileNotFound,
    /// File was modified externally (SPL-E034)
    FileExternallyModified,
    /// Rename operation failed (SPL-E040)
    RenameFailed,

    // Warning-level errors (SPL-W001 to SPL-W010)
    /// Symbol exists in multiple files (SPL-W001)
    AmbiguousSymbolAsWarning,
    /// File skipped during ingestion (SPL-W002)
    FileSkipped,
    /// External modification detected (SPL-W003)
    FileExternallyModifiedWarning,

    // Validation errors (SPL-E041 to SPL-E050)
    /// Pre-verification check failed (SPL-E041)
    PreVerificationFailed,
    /// Parse validation failed after modification (SPL-E042)
    ParseValidationFailed,
    /// Compiler validation failed (SPL-E043)
    CompilerValidationFailed,

    // Plan execution errors (SPL-E051 to SPL-E060)
    /// Invalid plan JSON schema (SPL-E051)
    InvalidPlanSchema,
    /// Plan execution failed at step (SPL-E052)
    PlanExecutionFailed,
    /// Invalid batch JSON schema (SPL-E053)
    InvalidBatchSchema,

    // Graph/database errors (SPL-E061 to SPL-E070)
    /// Code graph database error (SPL-E061)
    GraphError,
    /// Database operation failed (SPL-E062)
    DatabaseError,

    // Execution log errors (SPL-E071 to SPL-E080)
    /// Execution log database error (SPL-E071)
    ExecutionLogError,
    /// Execution log entry not found (SPL-E072)
    ExecutionNotFound,

    // Validation/analyzer errors (SPL-E081 to SPL-E090)
    /// Requested analyzer not available (SPL-E081)
    AnalyzerNotAvailable,
    /// Analyzer reported diagnostics (SPL-E082)
    AnalyzerFailed,

    // Magellan integration errors (SPL-E091 to SPL-E100)
    /// Magellan integration error (SPL-E091)
    MagellanError,
}

impl SpliceErrorCode {
    /// Get the error code string (e.g., "SPL-E001").
    pub fn code(&self) -> String {
        match self {
            SpliceErrorCode::SymbolNotFound => "SPL-E001".to_string(),
            SpliceErrorCode::AmbiguousSymbol => "SPL-E002".to_string(),
            SpliceErrorCode::ReferenceFailed => "SPL-E003".to_string(),
            SpliceErrorCode::AmbiguousReference => "SPL-E004".to_string(),

            SpliceErrorCode::ParseError => "SPL-E011".to_string(),
            SpliceErrorCode::InvalidUtf8 => "SPL-E012".to_string(),
            SpliceErrorCode::InvalidSyntax => "SPL-E013".to_string(),

            SpliceErrorCode::InvalidSpan => "SPL-E021".to_string(),
            SpliceErrorCode::InvalidLineRange => "SPL-E022".to_string(),
            SpliceErrorCode::SpanOutOfBounds => "SPL-E023".to_string(),

            SpliceErrorCode::FileReadError => "SPL-E031".to_string(),
            SpliceErrorCode::FileWriteError => "SPL-E032".to_string(),
            SpliceErrorCode::FileNotFound => "SPL-E033".to_string(),
            SpliceErrorCode::FileExternallyModified => "SPL-E034".to_string(),
            SpliceErrorCode::RenameFailed => "SPL-E040".to_string(),

            SpliceErrorCode::AmbiguousSymbolAsWarning => "SPL-W001".to_string(),
            SpliceErrorCode::FileSkipped => "SPL-W002".to_string(),
            SpliceErrorCode::FileExternallyModifiedWarning => "SPL-W003".to_string(),

            SpliceErrorCode::PreVerificationFailed => "SPL-E041".to_string(),
            SpliceErrorCode::ParseValidationFailed => "SPL-E042".to_string(),
            SpliceErrorCode::CompilerValidationFailed => "SPL-E043".to_string(),

            SpliceErrorCode::InvalidPlanSchema => "SPL-E051".to_string(),
            SpliceErrorCode::PlanExecutionFailed => "SPL-E052".to_string(),
            SpliceErrorCode::InvalidBatchSchema => "SPL-E053".to_string(),

            SpliceErrorCode::GraphError => "SPL-E061".to_string(),
            SpliceErrorCode::DatabaseError => "SPL-E062".to_string(),

            SpliceErrorCode::ExecutionLogError => "SPL-E071".to_string(),
            SpliceErrorCode::ExecutionNotFound => "SPL-E072".to_string(),

            SpliceErrorCode::AnalyzerNotAvailable => "SPL-E081".to_string(),
            SpliceErrorCode::AnalyzerFailed => "SPL-E082".to_string(),
            SpliceErrorCode::MagellanError => "SPL-E091".to_string(),
        }
    }

    /// Get the severity level string.
    pub fn severity(&self) -> String {
        match self {
            // Error-level codes
            SpliceErrorCode::SymbolNotFound
            | SpliceErrorCode::ReferenceFailed
            | SpliceErrorCode::ParseError
            | SpliceErrorCode::InvalidUtf8
            | SpliceErrorCode::InvalidSyntax
            | SpliceErrorCode::InvalidSpan
            | SpliceErrorCode::InvalidLineRange
            | SpliceErrorCode::SpanOutOfBounds
            | SpliceErrorCode::FileReadError
            | SpliceErrorCode::FileWriteError
            | SpliceErrorCode::FileNotFound
            | SpliceErrorCode::PreVerificationFailed
            | SpliceErrorCode::ParseValidationFailed
            | SpliceErrorCode::CompilerValidationFailed
            | SpliceErrorCode::InvalidPlanSchema
            | SpliceErrorCode::PlanExecutionFailed
            | SpliceErrorCode::InvalidBatchSchema
            | SpliceErrorCode::GraphError
            | SpliceErrorCode::DatabaseError
            | SpliceErrorCode::ExecutionLogError
            | SpliceErrorCode::ExecutionNotFound
            | SpliceErrorCode::AnalyzerNotAvailable
            | SpliceErrorCode::AnalyzerFailed
            | SpliceErrorCode::MagellanError
            | SpliceErrorCode::RenameFailed => "error".to_string(),

            // Warning-level codes
            SpliceErrorCode::AmbiguousSymbol
            | SpliceErrorCode::AmbiguousReference
            | SpliceErrorCode::FileExternallyModified
            | SpliceErrorCode::AmbiguousSymbolAsWarning
            | SpliceErrorCode::FileSkipped
            | SpliceErrorCode::FileExternallyModifiedWarning => "warning".to_string(),
        }
    }

    /// Get the hint message for this error code.
    pub fn hint(&self) -> String {
        match self {
            SpliceErrorCode::SymbolNotFound => "Check that the symbol name is spelled correctly and exists in the codebase. Use --file to specify the file if the symbol is defined in multiple files.".to_string(),
            SpliceErrorCode::AmbiguousSymbol => "The symbol name is defined in multiple files. Use --file to specify which file to use.".to_string(),
            SpliceErrorCode::ReferenceFailed => "Failed to locate symbol references. Ensure the codebase has been indexed and the symbol is accessible.".to_string(),
            SpliceErrorCode::AmbiguousReference => "The reference could refer to multiple definitions. Qualify the reference with module/type information to resolve ambiguity.".to_string(),

            SpliceErrorCode::ParseError => "Check file syntax and ensure it's valid source code for the detected language.".to_string(),
            SpliceErrorCode::InvalidUtf8 => "The file contains invalid UTF-8 encoding. Ensure the file is saved as UTF-8.".to_string(),
            SpliceErrorCode::InvalidSyntax => "Check the file syntax and fix any errors reported by the compiler.".to_string(),

            SpliceErrorCode::InvalidSpan => "The byte range is invalid. Ensure start <= end and both are within file bounds.".to_string(),
            SpliceErrorCode::InvalidLineRange => "The line range is invalid. Ensure line numbers are positive and within file bounds.".to_string(),
            SpliceErrorCode::SpanOutOfBounds => "The span extends beyond the file. Check file hasn't been modified since indexing.".to_string(),

            SpliceErrorCode::FileReadError => "Check file permissions and ensure the file exists and is readable.".to_string(),
            SpliceErrorCode::FileWriteError => "Check disk space, file permissions, and ensure the file is not locked by another process.".to_string(),
            SpliceErrorCode::FileNotFound => "The specified file does not exist. Check the file path.".to_string(),
            SpliceErrorCode::FileExternallyModified => "The file was modified by another process. Re-index the codebase and retry.".to_string(),
            SpliceErrorCode::RenameFailed => "Rename operation failed. Check that the symbol exists, has valid references, and the new name is not already in use.".to_string(),

            SpliceErrorCode::AmbiguousSymbolAsWarning => "The symbol name is defined in multiple files. Use --file to specify which file to use.".to_string(),
            SpliceErrorCode::FileSkipped => "File was skipped during ingestion. Check file type and permissions.".to_string(),
            SpliceErrorCode::FileExternallyModifiedWarning => "The file was modified by another process. Changes may not be reflected.".to_string(),

            SpliceErrorCode::PreVerificationFailed => "A pre-verification check failed. Review the check details and fix the reported issue.".to_string(),
            SpliceErrorCode::ParseValidationFailed => "The file failed to parse after modification. Revert the change and fix the syntax error.".to_string(),
            SpliceErrorCode::CompilerValidationFailed => "Compiler reported errors. Fix the compilation errors before proceeding.".to_string(),

            SpliceErrorCode::InvalidPlanSchema => "The plan JSON schema is invalid. Check the plan file format.".to_string(),
            SpliceErrorCode::PlanExecutionFailed => "A step in the plan failed. Review the error details and fix the issue.".to_string(),
            SpliceErrorCode::InvalidBatchSchema => "The batch JSON schema is invalid. Check the batch file format.".to_string(),

            SpliceErrorCode::GraphError => "The code graph database error. Check database permissions and integrity.".to_string(),
            SpliceErrorCode::DatabaseError => "Database operation failed. Check database connection and permissions.".to_string(),

            SpliceErrorCode::ExecutionLogError => "Failed to access execution log. Check log database permissions.".to_string(),
            SpliceErrorCode::ExecutionNotFound => "Execution ID not found in log. Verify the execution ID is correct.".to_string(),

            SpliceErrorCode::AnalyzerNotAvailable => "The requested analyzer is not available. Install the analyzer or use a different validation mode.".to_string(),
            SpliceErrorCode::AnalyzerFailed => "The analyzer reported diagnostics. Fix the reported issues.".to_string(),

            SpliceErrorCode::MagellanError => {
                "Check that the Magellan database file exists and is readable. \
                 If the error mentions a schema mismatch, re-index with \
                 `magellan watch --root ./src --db <db> --scan-initial`."
                    .to_string()
            }
        }
    }

    /// Convert from SpliceError to SpliceErrorCode.
    pub fn from_splice_error(error: &crate::SpliceError) -> Option<Self> {
        match error {
            crate::SpliceError::SymbolNotFound { .. } => Some(SpliceErrorCode::SymbolNotFound),
            crate::SpliceError::AmbiguousSymbol { .. } => Some(SpliceErrorCode::AmbiguousSymbol),
            crate::SpliceError::ReferenceFailed { .. } => Some(SpliceErrorCode::ReferenceFailed),
            crate::SpliceError::AmbiguousReference { .. } => {
                Some(SpliceErrorCode::AmbiguousReference)
            }

            crate::SpliceError::Parse { .. } => Some(SpliceErrorCode::ParseError),
            crate::SpliceError::InvalidUtf8 { .. } => Some(SpliceErrorCode::InvalidUtf8),
            crate::SpliceError::CompilerError(_) => Some(SpliceErrorCode::InvalidSyntax),

            crate::SpliceError::InvalidSpan { .. } => Some(SpliceErrorCode::InvalidSpan),
            crate::SpliceError::InvalidLineRange { .. } => Some(SpliceErrorCode::InvalidLineRange),
            crate::SpliceError::FileExternallyModified { .. } => {
                Some(SpliceErrorCode::FileExternallyModified)
            }

            crate::SpliceError::PreVerificationFailed { .. } => {
                Some(SpliceErrorCode::PreVerificationFailed)
            }
            crate::SpliceError::ParseValidationFailed { .. } => {
                Some(SpliceErrorCode::ParseValidationFailed)
            }
            crate::SpliceError::CompilerValidationFailed { .. } => {
                Some(SpliceErrorCode::CompilerValidationFailed)
            }

            crate::SpliceError::InvalidPlanSchema { .. } => {
                Some(SpliceErrorCode::InvalidPlanSchema)
            }
            crate::SpliceError::PlanExecutionFailed { .. } => {
                Some(SpliceErrorCode::PlanExecutionFailed)
            }
            crate::SpliceError::InvalidBatchSchema { .. } => {
                Some(SpliceErrorCode::InvalidBatchSchema)
            }

            crate::SpliceError::Graph(_) => Some(SpliceErrorCode::GraphError),

            crate::SpliceError::ExecutionLogError { .. } => {
                Some(SpliceErrorCode::ExecutionLogError)
            }
            crate::SpliceError::ExecutionNotFound { .. } => {
                Some(SpliceErrorCode::ExecutionNotFound)
            }

            crate::SpliceError::AnalyzerNotAvailable { .. } => {
                Some(SpliceErrorCode::AnalyzerNotAvailable)
            }
            crate::SpliceError::AnalyzerFailed { .. } => Some(SpliceErrorCode::AnalyzerFailed),

            // I/O errors - map to specific codes
            crate::SpliceError::Io { .. } | crate::SpliceError::IoContext { .. } => {
                Some(SpliceErrorCode::FileReadError)
            }
            crate::SpliceError::InsufficientDiskSpace { .. } => {
                Some(SpliceErrorCode::FileWriteError)
            }

            // Query and log errors
            crate::SpliceError::QueryError { .. } => Some(SpliceErrorCode::DatabaseError),
            crate::SpliceError::ExecutionRecordFailed { .. } => {
                Some(SpliceErrorCode::ExecutionLogError)
            }

            // Date format and plan errors
            crate::SpliceError::InvalidDateFormat { .. } => {
                Some(SpliceErrorCode::InvalidPlanSchema)
            }

            // Rust-specific compiler validation
            crate::SpliceError::CargoCheckFailed { .. } => {
                Some(SpliceErrorCode::CompilerValidationFailed)
            }

            // Magellan integration errors
            crate::SpliceError::Magellan { .. } => Some(SpliceErrorCode::MagellanError),

            // Rename operation errors
            crate::SpliceError::RenameFailed { .. } => Some(SpliceErrorCode::RenameFailed),

            // Batch operation errors
            crate::SpliceError::InvalidBatchSpec { .. } => {
                Some(SpliceErrorCode::InvalidBatchSchema)
            }
            crate::SpliceError::BatchOperationFailed { .. } => {
                Some(SpliceErrorCode::PlanExecutionFailed)
            }

            // Intentionally unmapped errors:
            // - BrokenPipe: terminal state, not user-fixable
            // - Utf8: covered by InvalidUtf8 variant
            // - Other: generic catchall, no specific code applicable
            crate::SpliceError::BrokenPipe
            | crate::SpliceError::Utf8(_)
            | crate::SpliceError::Other(_) => None,
        }
    }
}
