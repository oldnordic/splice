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

/// Get detailed explanation for an error code.
///
/// Provides expanded documentation following the `rustc --explain` pattern.
/// Explanations include:
/// - Error code name and description
/// - Common causes
/// - What to do (step-by-step)
/// - Related error codes
///
/// # Arguments
/// * `code` - Error code string (e.g., "SPL-E001", "SPL-E002")
///
/// # Returns
/// * `Some(&str)` with explanation if code is known
/// * `None` if code is unknown
///
/// # Examples
/// ```
/// use splice::error_codes::get_error_explanation;
///
/// let explanation = get_error_explanation("SPL-E001");
/// assert!(explanation.is_some());
/// assert!(explanation.unwrap().contains("Symbol Not Found"));
/// ```
pub fn get_error_explanation(code: &str) -> Option<&'static str> {
    match code {
        // Symbol resolution errors
        "SPL-E001" => Some(
            r#"
Symbol Not Found (SPL-E001)

The specified symbol could not be found in the codebase.

POSSIBLE CAUSES:
- The symbol name is misspelled
- The symbol hasn't been ingested into the code graph
- The symbol exists in multiple files (use --file to disambiguate)
- The symbol is defined in a file that hasn't been indexed

WHAT TO DO:
1. Check the symbol name is spelled correctly
2. Run `magellan watch --root ./src --db <db> --scan-initial` to ensure the codebase is indexed
3. Use `splice query` to search for symbols by label
4. Use `splice delete --file <path>` to specify which file
5. Use `splice explain SPL-E002` for help with ambiguous symbols

RELATED: SPL-E002 (Ambiguous Symbol)
"#,
        ),

        "SPL-E002" => Some(
            r#"
Ambiguous Symbol (SPL-E002)

The symbol name exists in multiple files, making it ambiguous which one
to use without additional context.

POSSIBLE CAUSES:
- Common symbol names like `main`, `run`, `process` used in multiple files
- Multiple files define the same function/struct name
- File context was not specified for the operation

WHAT TO DO:
1. Use `--file <path>` to specify which file contains the symbol
2. Use `splice query --db <db> --label <label>` to list all symbols
3. Rename one of the conflicting symbols if appropriate
4. Use fully-qualified names if supported by the language

RELATED: SPL-E001 (Symbol Not Found)
"#,
        ),

        "SPL-E003" => Some(
            r#"
Reference Failed (SPL-E003)

Failed to locate symbol references in the codebase.

POSSIBLE CAUSES:
- The code graph is incomplete or outdated
- Reference edges haven't been created during ingestion
- The symbol exists but references weren't indexed

WHAT TO DO:
1. Re-run `magellan watch --root ./src --db <db> --scan-initial` to rebuild the code graph
2. Check that the source files haven't been modified since indexing
3. Use `splice log --operation-type ingest` to check ingestion status
4. Report this as a bug if the issue persists

RELATED: SPL-E061 (Graph Error)
"#,
        ),

        "SPL-E004" => Some(
            r#"
Ambiguous Reference (SPL-E004)

A reference could refer to multiple definitions, making it ambiguous
which one is being referenced.

POSSIBLE CAUSES:
- Multiple symbols with the same name in scope
- Import/use statements bring in conflicting names
- Type inference cannot determine which overload to use

WHAT TO DO:
1. Qualify the reference with module/type information
2. Use explicit imports instead of wildcards
3. Rename one of the conflicting symbols if appropriate
4. Check the error message for candidate definitions

RELATED: SPL-E002 (Ambiguous Symbol)
"#,
        ),

        // Parse/AST errors
        "SPL-E011" => Some(
            r#"
Parse Error (SPL-E011)

Tree-sitter failed to parse the source file.

POSSIBLE CAUSES:
- Invalid syntax for the detected language
- Incomplete source file (e.g., missing closing brace)
- File encoding issues (should be UTF-8)
- Corrupted or truncated file

WHAT TO DO:
1. Check file syntax is valid for the language
2. Ensure the file is complete and properly formatted
3. Verify the file is saved as UTF-8 encoding
4. Use a language server or compiler to check for syntax errors
5. Re-ingest the file after fixing syntax

RELATED: SPL-E012 (Invalid UTF-8), SPL-E013 (Invalid Syntax)
"#,
        ),

        "SPL-E012" => Some(
            r#"
Invalid UTF-8 (SPL-E012)

The file contains invalid UTF-8 encoding.

POSSIBLE CAUSES:
- File was saved with a different encoding (e.g., Latin-1, UTF-16)
- File contains binary data or corruption
- File was transferred without preserving encoding

WHAT TO DO:
1. Convert the file to UTF-8 encoding
2. Use `file --mime-encoding <path>` to check current encoding
3. Use `iconv` or similar tool to convert encodings
4. Ensure your editor is configured to save files as UTF-8

RELATED: SPL-E011 (Parse Error)
"#,
        ),

        "SPL-E013" => Some(
            r#"
Invalid Syntax (SPL-E013)

The compiler reported a syntax error in the source file.

POSSIBLE CAUSES:
- Typo or mistake in the source code
- Incomplete statement or block
- Language feature used incorrectly
- Missing imports or dependencies

WHAT TO DO:
1. Check the file syntax and fix any errors reported by the compiler
2. Use the compiler's error messages for specific line/column information
3. Run `cargo check` (Rust) or equivalent for your language
4. Refer to language documentation for correct syntax

RELATED: SPL-E011 (Parse Error), SPL-E043 (Compiler Validation Failed)
"#,
        ),

        // Span errors
        "SPL-E021" => Some(
            r#"
Invalid Span (SPL-E021)

The byte range specified is invalid.

POSSIBLE CAUSES:
- Start position is after end position
- Byte offsets are negative or extremely large
- Span was calculated incorrectly

WHAT TO DO:
1. Ensure start <= end for byte ranges
2. Check that byte offsets are within file bounds
3. Verify the span calculation logic
4. Use `splice query` to get valid spans for symbols

RELATED: SPL-E022 (Invalid Line Range), SPL-E023 (Span Out of Bounds)
"#,
        ),

        "SPL-E022" => Some(
            r#"
Invalid Line Range (SPL-E022)

The line range specified is invalid.

POSSIBLE CAUSES:
- Start line is after end line
- Line numbers are zero or negative
- Line numbers exceed file's total lines

WHAT TO DO:
1. Ensure line_start <= line_end
2. Use 1-based line numbering (first line is 1, not 0)
3. Check the file's total line count
4. Use `splice get` to retrieve valid line ranges

RELATED: SPL-E021 (Invalid Span), SPL-E023 (Span Out of Bounds)
"#,
        ),

        "SPL-E023" => Some(
            r#"
Span Out of Bounds (SPL-E023)

The span extends beyond the file's boundaries.

POSSIBLE CAUSES:
- File was modified since the span was calculated
- Byte offsets were calculated incorrectly
- File size has changed

WHAT TO DO:
1. Re-index the codebase with `magellan watch --root ./src --db <db> --scan-initial`
2. Check that the file hasn't been modified externally
3. Verify the file size matches expectations
4. Use checksums to detect file modifications

RELATED: SPL-E034 (File Externally Modified)
"#,
        ),

        // I/O errors
        "SPL-E031" => Some(
            r#"
File Read Error (SPL-E031)

Failed to read a file.

POSSIBLE CAUSES:
- File permissions prevent reading
- File is locked by another process
- File doesn't exist
- Insufficient system resources

WHAT TO DO:
1. Check file permissions with `ls -l <path>`
2. Ensure the file exists and is not locked
3. Check disk space and system resources
4. Verify the file path is correct

RELATED: SPL-E033 (File Not Found), SPL-E032 (File Write Error)
"#,
        ),

        "SPL-E032" => Some(
            r#"
File Write Error (SPL-E032)

Failed to write to a file.

POSSIBLE CAUSES:
- Insufficient disk space
- File permissions prevent writing
- File is locked by another process
- Directory doesn't exist

WHAT TO DO:
1. Check available disk space with `df -h`
2. Verify write permissions with `ls -ld <dir>`
3. Ensure the file is not open in another program
4. Create parent directories if needed

RELATED: SPL-E031 (File Read Error), SPL-E034 (File Externally Modified)
"#,
        ),

        "SPL-E033" => Some(
            r#"
File Not Found (SPL-E033)

The specified file does not exist.

POSSIBLE CAUSES:
- File path is incorrect or misspelled
- File was deleted or moved
- Relative path is used from wrong directory

WHAT TO DO:
1. Check the file path is correct
2. Use absolute paths if relative paths are problematic
3. Verify the file exists with `ls <path>`
4. Run `magellan watch --root ./src --db <db> --scan-initial` to re-index if file was moved

RELATED: SPL-E031 (File Read Error)
"#,
        ),

        "SPL-E034" => Some(
            r#"
File Externally Modified (SPL-E034)

The file was modified by another process since being indexed.

POSSIBLE CAUSES:
- File was edited in another editor or IDE
- Build process generated or modified the file
- Code formatter or linter modified the file

WHAT TO DO:
1. Re-index the codebase with `magellan watch --root ./src --db <db> --scan-initial`
2. Check for background processes modifying files
3. Use file checksums to detect modifications
4. Consider using file watchers to detect external changes

RELATED: SPL-E023 (Span Out of Bounds)
"#,
        ),

        // Validation errors
        "SPL-E041" => Some(
            r#"
Pre-Verification Failed (SPL-E041)

A pre-verification check failed before applying changes.

POSSIBLE CAUSES:
- Pre-condition checks detected an issue
- Validation gate found a blocking problem
- Resource constraints (disk space, etc.)

WHAT TO DO:
1. Review the specific check that failed
2. Fix the reported issue
3. Use `--skip-pre-verify` with caution (dangerous)
4. Check system resources if applicable

RELATED: SPL-E042 (Parse Validation Failed), SPL-E043 (Compiler Validation Failed)
"#,
        ),

        "SPL-E042" => Some(
            r#"
Parse Validation Failed (SPL-E042)

The file failed to parse after modification.

POSSIBLE CAUSES:
- Modification introduced a syntax error
- Incomplete replacement or deletion
- File was corrupted during write

WHAT TO DO:
1. Revert the change using `splice undo`
2. Check the modification for syntax errors
3. Use `--dry-run` to preview changes before applying
4. Ensure the replacement content is complete and valid

RELATED: SPL-E011 (Parse Error), SPL-E041 (Pre-Verification Failed)
"#,
        ),

        "SPL-E043" => Some(
            r#"
Compiler Validation Failed (SPL-E043)

Compiler reported errors after modification.

POSSIBLE CAUSES:
- Modification introduced compilation errors
- Type errors or missing imports
- Breaking changes to API usage

WHAT TO DO:
1. Check compiler output for specific errors
2. Fix compilation errors before proceeding
3. Use `--dry-run` to preview changes
4. Run compiler manually for detailed error messages

RELATED: SPL-E013 (Invalid Syntax), SPL-E081 (Analyzer Not Available)
"#,
        ),

        // Plan execution errors
        "SPL-E051" => Some(
            r#"
Invalid Plan Schema (SPL-E051)

The plan JSON schema is invalid.

POSSIBLE CAUSES:
- JSON syntax errors in plan file
- Missing required fields
- Invalid data types for fields
- Schema version mismatch

WHAT TO DO:
1. Validate JSON syntax with `jq . < plan.json`
2. Check the plan file format in documentation
3. Ensure all required fields are present
4. Verify schema version matches Splice version

RELATED: SPL-E053 (Invalid Batch Schema)
"#,
        ),

        "SPL-E052" => Some(
            r#"
Plan Execution Failed (SPL-E052)

A step in the plan failed during execution.

POSSIBLE CAUSES:
- One of the plan steps encountered an error
- Resource constraints during execution
- File system issues

WHAT TO DO:
1. Review the specific step that failed
2. Check the error message for details
3. Fix the underlying issue and re-run the plan
4. Use `--dry-run` to preview plan execution

RELATED: SPL-E051 (Invalid Plan Schema)
"#,
        ),

        "SPL-E053" => Some(
            r#"
Invalid Batch Schema (SPL-E053)

The batch JSON schema is invalid.

POSSIBLE CAUSES:
- JSON syntax errors in batch file
- Missing required fields
- Invalid data types for fields
- Incorrect array or object structure

WHAT TO DO:
1. Validate JSON syntax with `jq . < batch.json`
2. Check the batch file format in documentation
3. Ensure all required fields are present
4. Verify the file/replacement structure

RELATED: SPL-E051 (Invalid Plan Schema)
"#,
        ),

        // Graph/database errors
        "SPL-E061" => Some(
            r#"
Graph Error (SPL-E061)

The code graph database encountered an error.

POSSIBLE CAUSES:
- Database corruption
- Insufficient permissions
- Database locked by another process
- Incompatible database version

WHAT TO DO:
1. Check database permissions with `ls -l codegraph.db`
2. Ensure no other process is using the database
3. Try rebuilding the database with `magellan watch --root ./src --db <db> --scan-initial`
4. Check database version compatibility

RELATED: SPL-E062 (Database Error), SPL-E003 (Reference Failed)
"#,
        ),

        "SPL-E062" => Some(
            r#"
Database Error (SPL-E062)

A database operation failed.

POSSIBLE CAUSES:
- Query execution error
- Constraint violation
- Transaction failure
- Connection issues

WHAT TO DO:
1. Check database integrity
2. Rebuild the database if needed
3. Verify sufficient disk space
4. Check for concurrent access issues

RELATED: SPL-E061 (Graph Error)
"#,
        ),

        // Execution log errors
        "SPL-E071" => Some(
            r#"
Execution Log Error (SPL-E071)

Failed to access the execution log database.

POSSIBLE CAUSES:
- Database permissions issue
- Execution log database corrupted
- Database locked by another process

WHAT TO DO:
1. Check execution log permissions
2. Ensure Splice has write access to log directory
3. Close other processes that might be using the log
4. Re-create the log database if corrupted

RELATED: SPL-E062 (Database Error)
"#,
        ),

        "SPL-E072" => Some(
            r#"
Execution Not Found (SPL-E072)

The specified execution ID was not found in the log.

POSSIBLE CAUSES:
- Execution ID is incorrect or misspelled
- Execution was logged to a different database
- Execution log was cleared or truncated

WHAT TO DO:
1. Verify the execution ID is correct
2. Use `splice log --stats` to list recent executions
3. Check the execution log database location
4. Re-run the operation if needed

RELATED: SPL-E071 (Execution Log Error)
"#,
        ),

        // Analyzer errors
        "SPL-E081" => Some(
            r#"
Analyzer Not Available (SPL-E081)

The requested analyzer is not available.

POSSIBLE CAUSES:
- rust-analyzer or other tool is not installed
- Analyzer is not in PATH
- Invalid analyzer path specified

WHAT TO DO:
1. Install the required analyzer (e.g., `rustup component add rust-analyzer`)
2. Ensure the analyzer is in your PATH
3. Use `--analyzer path` to specify explicit path
4. Use `--analyzer off` to disable analyzer validation

RELATED: SPL-E082 (Analyzer Failed)
"#,
        ),

        "SPL-E082" => Some(
            r#"
Analyzer Failed (SPL-E082)

The analyzer reported diagnostics.

POSSIBLE CAUSES:
- Code has issues detected by the analyzer
- Analyzer configuration problems
- False positives from analyzer

WHAT TO DO:
1. Review the analyzer diagnostics
2. Fix legitimate issues reported
3. Consider disabling analyzer for false positives
4. Update analyzer version if outdated

RELATED: SPL-E043 (Compiler Validation Failed), SPL-E081 (Analyzer Not Available)
"#,
        ),

        "SPL-E091" => Some(
            r#"
Magellan Error (SPL-E091)

An error occurred opening or querying the Magellan code graph.

POSSIBLE CAUSES:
- Database file does not exist
- Insufficient permissions to read the database
- Database schema is older than the one this splice binary expects
  (e.g., "DB_COMPAT: sqlitegraph schema mismatch: ... found=3, expected=4")
- Magellan internal error

WHAT TO DO:
1. Check that the database file exists: ls -l <db_path>
2. Verify file permissions: readable by current user
3. If the underlying error mentions a schema mismatch, re-index with a magellan
   version matching this splice (run from the project root):
       magellan watch --root ./src --db .magellan/<project>.db --scan-initial
   This rewrites the database against the current schema.
4. To incrementally sync a stale database (no schema change), use:
       magellan refresh --db .magellan/<project>.db
5. Run `magellan status --db <db_path>` to confirm the database is readable
   by the magellan binary in PATH (if magellan can read it but splice cannot,
   the cause is almost certainly a schema version mismatch).

RELATED: SPL-E061 (Graph Error), SPL-E031 (File Read Error)
"#,
        ),

        // Unknown code
        _ => None,
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
    fn test_error_code_coverage() {
        use crate::SpliceError;
        use std::path::PathBuf;

        // Test all error-level SpliceError variants produce error codes
        let mut mapped_count = 0;

        // Symbol resolution errors
        let error = SpliceError::symbol_not_found("test", None);
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::AmbiguousSymbol {
            name: "foo".to_string(),
            files: vec!["a.rs".to_string()],
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::ReferenceFailed {
            name: "foo".to_string(),
            reason: "test".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::AmbiguousReference {
            name: "foo".to_string(),
            file: "test.rs".to_string(),
            line: 1,
            col: 1,
            candidates: vec!["a::foo".to_string()],
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Parse/AST errors
        let error = SpliceError::Parse {
            file: PathBuf::from("test.rs"),
            message: "test error".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // InvalidUtf8 - create by converting invalid bytes
        let invalid_bytes = vec![0xffu8, 0xfe];
        let invalid_utf8 = std::str::from_utf8(&invalid_bytes).unwrap_err();
        let error = SpliceError::InvalidUtf8 {
            file: PathBuf::from("test.rs"),
            source: invalid_utf8,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::CompilerError("syntax error".to_string());
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Span errors
        let error = SpliceError::InvalidSpan {
            file: PathBuf::from("test.rs"),
            start: 0,
            end: 10,
            file_size: 100,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::InvalidLineRange {
            file: PathBuf::from("test.rs"),
            line_start: 1,
            line_end: 10,
            total_lines: 20,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::FileExternallyModified {
            file: "test.rs".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // I/O errors
        let error = SpliceError::Io {
            path: PathBuf::from("test.rs"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::IoContext {
            context: "test context".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "test"),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::InsufficientDiskSpace {
            needed: 1000,
            available: 100,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Validation errors
        let error = SpliceError::PreVerificationFailed {
            check: "test check".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::ParseValidationFailed {
            file: PathBuf::from("test.rs"),
            message: "validation failed".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::CompilerValidationFailed {
            file: PathBuf::from("test.rs"),
            language: "rust".to_string(),
            diagnostics: vec![],
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Plan execution errors
        let error = SpliceError::InvalidPlanSchema {
            message: "invalid schema".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::PlanExecutionFailed {
            step: 1,
            error: "failed".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::InvalidBatchSchema {
            message: "invalid batch".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Graph/database errors
        let error = SpliceError::Graph(sqlitegraph::SqliteGraphError::connection("test error"));
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Execution log errors
        let error = SpliceError::ExecutionLogError {
            message: "log error".to_string(),
            source: None,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::ExecutionNotFound {
            execution_id: "test-id".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::ExecutionRecordFailed {
            execution_id: "test-id".to_string(),
            source: None,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Analyzer errors
        let error = SpliceError::AnalyzerNotAvailable {
            mode: "path".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        let error = SpliceError::AnalyzerFailed {
            output: "failed".to_string(),
            diagnostics: vec![],
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Query errors
        let error = SpliceError::QueryError {
            message: "query failed".to_string(),
            source: None,
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Date format errors
        let error = SpliceError::InvalidDateFormat {
            input: "invalid".to_string(),
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Cargo check failed (Rust-specific compiler validation)
        let error = SpliceError::CargoCheckFailed {
            workspace: PathBuf::from("/workspace"),
            output: "check failed".to_string(),
            diagnostics: vec![],
        };
        assert!(SpliceErrorCode::from_splice_error(&error).is_some());
        mapped_count += 1;

        // Intentionally unmapped errors
        let error = SpliceError::BrokenPipe;
        assert!(
            SpliceErrorCode::from_splice_error(&error).is_none(),
            "BrokenPipe should not have an error code (terminal state)"
        );

        // Utf8 variant - intentionally unmapped
        let invalid_bytes = vec![0xffu8, 0xfe];
        let invalid_utf8 = std::str::from_utf8(&invalid_bytes).unwrap_err();
        let error = SpliceError::Utf8(invalid_utf8);
        assert!(
            SpliceErrorCode::from_splice_error(&error).is_none(),
            "Utf8 variant should not have an error code (covered by InvalidUtf8)"
        );

        let error = SpliceError::Other("generic error".to_string());
        assert!(
            SpliceErrorCode::from_splice_error(&error).is_none(),
            "Other variant should not have an error code (generic catchall)"
        );

        // Verify we have at least 22 error-level variants mapped
        assert!(
            mapped_count >= 22,
            "Expected at least 22 error-level variants to be mapped, got {}",
            mapped_count
        );

        println!("Total error-level variants mapped: {}", mapped_count);
    }

    #[test]
    fn test_explain_command_generation() {
        use crate::SpliceError;

        // Test that explain_command is generated for errors with error codes
        let symbol_error = SpliceError::symbol_not_found("foo", None);
        let payload = crate::cli::CliErrorPayload::from_error(&symbol_error);

        // Should have error_code
        assert!(payload.error.error_code.is_some());
        // Should have explain_command
        assert!(payload.error.explain_command.is_some());

        let explain_cmd = payload.error.explain_command.as_ref().unwrap();
        assert_eq!(explain_cmd, "splice explain --code SPL-E001");
        assert!(explain_cmd.contains("splice explain --code"));

        // Test that BrokenPipe (no error code) doesn't have explain_command
        let broken_pipe_error = SpliceError::BrokenPipe;
        let payload = crate::cli::CliErrorPayload::from_error(&broken_pipe_error);

        assert!(payload.error.error_code.is_none());
        assert!(payload.error.explain_command.is_none());

        // Test format for various error codes
        let test_cases = vec![
            (
                SpliceError::Parse {
                    file: std::path::PathBuf::from("test.rs"),
                    message: "parse error".to_string(),
                },
                "SPL-E011",
            ),
            (
                SpliceError::InvalidSpan {
                    file: std::path::PathBuf::from("test.rs"),
                    start: 0,
                    end: 10,
                    file_size: 100,
                },
                "SPL-E021",
            ),
            (
                SpliceError::Io {
                    path: std::path::PathBuf::from("test.rs"),
                    source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
                },
                "SPL-E031",
            ),
        ];

        for (error, expected_code) in test_cases {
            let payload = crate::cli::CliErrorPayload::from_error(&error);
            assert!(payload.error.error_code.is_some());
            assert!(payload.error.explain_command.is_some());

            let explain_cmd = payload.error.explain_command.as_ref().unwrap();
            let expected = format!("splice explain --code {}", expected_code);
            assert_eq!(explain_cmd, &expected);
        }
    }

    #[test]
    fn test_error_code_construction() {
        let code = ErrorCode::new(
            "SPL-E001",
            "error",
            "src/main.rs:10:5",
            "Check symbol spelling",
        );

        assert_eq!(code.code, "SPL-E001");
        assert_eq!(code.severity, "error");
        assert_eq!(code.location, Some("src/main.rs:10:5".to_string()));
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
        assert_eq!(error_code.location, Some("src/main.rs:10:5".to_string()));
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
        assert_eq!(ec.location, Some("test.rs:5:10".to_string()));

        // File only
        let ec = ErrorCode::from_splice_code(
            SpliceErrorCode::FileReadError,
            Some("test.rs"),
            None,
            None,
        );
        assert_eq!(ec.location, Some("test.rs".to_string()));

        // Unknown location
        let ec = ErrorCode::from_splice_code(SpliceErrorCode::GraphError, None, None, None);
        assert_eq!(ec.location, None);
    }

    #[test]
    fn test_error_severity_as_str() {
        assert_eq!(ErrorSeverity::Error.as_str(), "error");
        assert_eq!(ErrorSeverity::Warning.as_str(), "warning");
        assert_eq!(ErrorSeverity::Note.as_str(), "note");
    }

    #[test]
    fn test_error_code_severity_error() {
        // Test that error-level codes return "error"
        assert_eq!(SpliceErrorCode::SymbolNotFound.severity(), "error");
        assert_eq!(SpliceErrorCode::ReferenceFailed.severity(), "error");
        assert_eq!(SpliceErrorCode::ParseError.severity(), "error");
        assert_eq!(SpliceErrorCode::InvalidUtf8.severity(), "error");
        assert_eq!(SpliceErrorCode::InvalidSyntax.severity(), "error");
        assert_eq!(SpliceErrorCode::InvalidSpan.severity(), "error");
        assert_eq!(SpliceErrorCode::InvalidLineRange.severity(), "error");
        assert_eq!(SpliceErrorCode::SpanOutOfBounds.severity(), "error");
        assert_eq!(SpliceErrorCode::FileReadError.severity(), "error");
        assert_eq!(SpliceErrorCode::FileWriteError.severity(), "error");
        assert_eq!(SpliceErrorCode::FileNotFound.severity(), "error");
        assert_eq!(SpliceErrorCode::PreVerificationFailed.severity(), "error");
        assert_eq!(SpliceErrorCode::ParseValidationFailed.severity(), "error");
        assert_eq!(
            SpliceErrorCode::CompilerValidationFailed.severity(),
            "error"
        );
        assert_eq!(SpliceErrorCode::InvalidPlanSchema.severity(), "error");
        assert_eq!(SpliceErrorCode::PlanExecutionFailed.severity(), "error");
        assert_eq!(SpliceErrorCode::InvalidBatchSchema.severity(), "error");
        assert_eq!(SpliceErrorCode::GraphError.severity(), "error");
        assert_eq!(SpliceErrorCode::DatabaseError.severity(), "error");
        assert_eq!(SpliceErrorCode::ExecutionLogError.severity(), "error");
        assert_eq!(SpliceErrorCode::ExecutionNotFound.severity(), "error");
        assert_eq!(SpliceErrorCode::AnalyzerNotAvailable.severity(), "error");
        assert_eq!(SpliceErrorCode::AnalyzerFailed.severity(), "error");
    }

    #[test]
    fn test_error_code_severity_warning() {
        // Test that warning-level codes return "warning"
        assert_eq!(SpliceErrorCode::AmbiguousSymbol.severity(), "warning");
        assert_eq!(SpliceErrorCode::AmbiguousReference.severity(), "warning");
        assert_eq!(
            SpliceErrorCode::FileExternallyModified.severity(),
            "warning"
        );
        assert_eq!(
            SpliceErrorCode::AmbiguousSymbolAsWarning.severity(),
            "warning"
        );
        assert_eq!(SpliceErrorCode::FileSkipped.severity(), "warning");
        assert_eq!(
            SpliceErrorCode::FileExternallyModifiedWarning.severity(),
            "warning"
        );
    }

    #[test]
    fn test_error_code_all_have_severity() {
        // Test that all SpliceErrorCode variants have valid severity
        let all_codes = [
            SpliceErrorCode::SymbolNotFound,
            SpliceErrorCode::AmbiguousSymbol,
            SpliceErrorCode::ReferenceFailed,
            SpliceErrorCode::AmbiguousReference,
            SpliceErrorCode::ParseError,
            SpliceErrorCode::InvalidUtf8,
            SpliceErrorCode::InvalidSyntax,
            SpliceErrorCode::InvalidSpan,
            SpliceErrorCode::InvalidLineRange,
            SpliceErrorCode::SpanOutOfBounds,
            SpliceErrorCode::FileReadError,
            SpliceErrorCode::FileWriteError,
            SpliceErrorCode::FileNotFound,
            SpliceErrorCode::FileExternallyModified,
            SpliceErrorCode::AmbiguousSymbolAsWarning,
            SpliceErrorCode::FileSkipped,
            SpliceErrorCode::FileExternallyModifiedWarning,
            SpliceErrorCode::PreVerificationFailed,
            SpliceErrorCode::ParseValidationFailed,
            SpliceErrorCode::CompilerValidationFailed,
            SpliceErrorCode::InvalidPlanSchema,
            SpliceErrorCode::PlanExecutionFailed,
            SpliceErrorCode::InvalidBatchSchema,
            SpliceErrorCode::GraphError,
            SpliceErrorCode::DatabaseError,
            SpliceErrorCode::ExecutionLogError,
            SpliceErrorCode::ExecutionNotFound,
            SpliceErrorCode::AnalyzerNotAvailable,
            SpliceErrorCode::AnalyzerFailed,
        ];

        for code in all_codes.iter() {
            let severity = code.severity();
            assert!(
                severity == "error" || severity == "warning" || severity == "note",
                "Code {:?} has invalid severity: {}",
                code,
                severity
            );
        }
    }

    #[test]
    fn test_warning_code_format() {
        // Test that warning codes use SPL-W### format
        assert_eq!(SpliceErrorCode::AmbiguousSymbolAsWarning.code(), "SPL-W001");
        assert_eq!(SpliceErrorCode::FileSkipped.code(), "SPL-W002");
        assert_eq!(
            SpliceErrorCode::FileExternallyModifiedWarning.code(),
            "SPL-W003"
        );
    }

    #[test]
    fn test_warning_code_from_splice_code() {
        // Test that warning codes produce proper ErrorCode with warning severity
        let warning_code = ErrorCode::from_splice_code(
            SpliceErrorCode::AmbiguousSymbolAsWarning,
            Some("src/main.rs"),
            Some(10),
            Some(5),
        );

        assert_eq!(warning_code.code, "SPL-W001");
        assert_eq!(warning_code.severity, "warning");
        assert_eq!(warning_code.location, Some("src/main.rs:10:5".to_string()));
        assert!(!warning_code.hint.is_empty());
    }
}
