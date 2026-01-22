//! Error code registry for Splice diagnostics.
//!
//! Provides stable error codes in SPL-E### format with severity levels,
//! precise locations, and actionable hints for LLM consumption.

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
    /// Precise location (file:line:column)
    pub location: String,
    /// What to do hint
    pub hint: String,
}

impl ErrorCode {
    /// Create a new error code.
    pub fn new(code: impl Into<String>, severity: impl Into<String>, location: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: severity.into(),
            location: location.into(),
            hint: hint.into(),
        }
    }

    /// Create from SpliceErrorCode enum.
    pub fn from_splice_code(splice_code: SpliceErrorCode, file: Option<&str>, line: Option<usize>, column: Option<usize>) -> Self {
        let location = match (file, line, column) {
            (Some(f), Some(l), Some(c)) => format!("{}:{}:{}", f, l, c),
            (Some(f), Some(l), None) => format!("{}:{}", f, l),
            (Some(f), None, Some(c)) => format!("{}:{}", f, c),
            (Some(f), None, None) => f.to_string(),
            (None, _, _) => "<unknown>".to_string(),
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
        }
    }

    /// Get the severity level string.
    pub fn severity(&self) -> String {
        // All current codes are errors
        "error".to_string()
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
        }
    }

    /// Convert from SpliceError to SpliceErrorCode.
    pub fn from_splice_error(error: &crate::SpliceError) -> Option<Self> {
        match error {
            crate::SpliceError::SymbolNotFound { .. } => Some(SpliceErrorCode::SymbolNotFound),
            crate::SpliceError::AmbiguousSymbol { .. } => Some(SpliceErrorCode::AmbiguousSymbol),
            crate::SpliceError::ReferenceFailed { .. } => Some(SpliceErrorCode::ReferenceFailed),
            crate::SpliceError::AmbiguousReference { .. } => Some(SpliceErrorCode::AmbiguousReference),

            crate::SpliceError::Parse { .. } => Some(SpliceErrorCode::ParseError),
            crate::SpliceError::InvalidUtf8 { .. } => Some(SpliceErrorCode::InvalidUtf8),
            crate::SpliceError::CompilerError(_) => Some(SpliceErrorCode::InvalidSyntax),

            crate::SpliceError::InvalidSpan { .. } => Some(SpliceErrorCode::InvalidSpan),
            crate::SpliceError::InvalidLineRange { .. } => Some(SpliceErrorCode::InvalidLineRange),
            crate::SpliceError::FileExternallyModified { .. } => Some(SpliceErrorCode::FileExternallyModified),

            crate::SpliceError::PreVerificationFailed { .. } => Some(SpliceErrorCode::PreVerificationFailed),
            crate::SpliceError::ParseValidationFailed { .. } => Some(SpliceErrorCode::ParseValidationFailed),
            crate::SpliceError::CompilerValidationFailed { .. } => Some(SpliceErrorCode::CompilerValidationFailed),

            crate::SpliceError::InvalidPlanSchema { .. } => Some(SpliceErrorCode::InvalidPlanSchema),
            crate::SpliceError::PlanExecutionFailed { .. } => Some(SpliceErrorCode::PlanExecutionFailed),
            crate::SpliceError::InvalidBatchSchema { .. } => Some(SpliceErrorCode::InvalidBatchSchema),

            crate::SpliceError::Graph(_) => Some(SpliceErrorCode::GraphError),

            crate::SpliceError::ExecutionLogError { .. } => Some(SpliceErrorCode::ExecutionLogError),
            crate::SpliceError::ExecutionNotFound { .. } => Some(SpliceErrorCode::ExecutionNotFound),

            crate::SpliceError::AnalyzerNotAvailable { .. } => Some(SpliceErrorCode::AnalyzerNotAvailable),
            crate::SpliceError::AnalyzerFailed { .. } => Some(SpliceErrorCode::AnalyzerFailed),

            // I/O errors - map to specific codes
            crate::SpliceError::Io { .. } | crate::SpliceError::IoContext { .. } => Some(SpliceErrorCode::FileReadError),

            // Other errors - no specific code
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_format() {
        assert_eq!(SpliceErrorCode::SymbolNotFound.code(), "SPL-E001");
        assert_eq!(SpliceErrorCode::AmbiguousSymbol.code(), "SPL-E002");
        assert_eq!(SpliceErrorCode::ParseError.code(), "SPL-E011");
        assert_eq!(SpliceErrorCode::InvalidSpan.code(), "SPL-E021");
        assert_eq!(SpliceErrorCode::FileReadError.code(), "SPL-E031");
        assert_eq!(SpliceErrorCode::PreVerificationFailed.code(), "SPL-E041");
        assert_eq!(SpliceErrorCode::InvalidPlanSchema.code(), "SPL-E051");
        assert_eq!(SpliceErrorCode::GraphError.code(), "SPL-E061");
        assert_eq!(SpliceErrorCode::ExecutionLogError.code(), "SPL-E071");
        assert_eq!(SpliceErrorCode::AnalyzerNotAvailable.code(), "SPL-E081");
    }

    #[test]
    fn test_error_code_severity() {
        assert_eq!(SpliceErrorCode::SymbolNotFound.severity(), "error");
        assert_eq!(SpliceErrorCode::ParseError.severity(), "error");
        assert_eq!(SpliceErrorCode::InvalidSpan.severity(), "error");
    }

    #[test]
    fn test_error_code_has_hint() {
        let code = SpliceErrorCode::SymbolNotFound;
        let hint = code.hint();
        assert!(!hint.is_empty());
        assert!(hint.contains("symbol") || hint.contains("file"));
    }

    #[test]
    fn test_error_code_from_splice_error() {
        use crate::SpliceError;

        let symbol_error = SpliceError::symbol_not_found("foo", None);
        let code = SpliceErrorCode::from_splice_error(&symbol_error);
        assert_eq!(code, Some(SpliceErrorCode::SymbolNotFound));

        let ambiguous_error = SpliceError::AmbiguousSymbol {
            name: "foo".to_string(),
            files: vec!["a.rs".to_string(), "b.rs".to_string()],
        };
        let code = SpliceErrorCode::from_splice_error(&ambiguous_error);
        assert_eq!(code, Some(SpliceErrorCode::AmbiguousSymbol));
    }

    #[test]
    fn test_error_code_construction() {
        let code = ErrorCode::new(
            "SPL-E001",
            "error",
            "src/main.rs:10:5",
            "Check symbol spelling"
        );

        assert_eq!(code.code, "SPL-E001");
        assert_eq!(code.severity, "error");
        assert_eq!(code.location, "src/main.rs:10:5");
        assert_eq!(code.hint, "Check symbol spelling");
    }

    #[test]
    fn test_error_code_from_splice_code() {
        let error_code = ErrorCode::from_splice_code(
            SpliceErrorCode::SymbolNotFound,
            Some("src/main.rs"),
            Some(10),
            Some(5),
        );

        assert_eq!(error_code.code, "SPL-E001");
        assert_eq!(error_code.severity, "error");
        assert_eq!(error_code.location, "src/main.rs:10:5");
        assert!(!error_code.hint.is_empty());
    }

    #[test]
    fn test_error_code_location_formats() {
        // Full location
        let ec = ErrorCode::from_splice_code(
            SpliceErrorCode::InvalidSpan,
            Some("test.rs"),
            Some(5),
            Some(10),
        );
        assert_eq!(ec.location, "test.rs:5:10");

        // File only
        let ec = ErrorCode::from_splice_code(
            SpliceErrorCode::FileReadError,
            Some("test.rs"),
            None,
            None,
        );
        assert_eq!(ec.location, "test.rs");

        // Unknown location
        let ec = ErrorCode::from_splice_code(
            SpliceErrorCode::GraphError,
            None,
            None,
            None,
        );
        assert_eq!(ec.location, "<unknown>");
    }

    #[test]
    fn test_error_severity_as_str() {
        assert_eq!(ErrorSeverity::Error.as_str(), "error");
        assert_eq!(ErrorSeverity::Warning.as_str(), "warning");
        assert_eq!(ErrorSeverity::Note.as_str(), "note");
    }
}
