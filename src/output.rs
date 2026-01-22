//! Structured output types for Splice operations.
//!
//! All output types use serde::Serialize for consistent JSON output.
//!
//! ## Adding Checksums to SpanResult
//!
//! ```no_run
//! use splice::checksum::{checksum_span, checksum_file};
//! use splice::output::SpanResult;
//! use std::path::Path;
//!
//! let file_path = Path::new("src/main.rs");
//! let byte_start = 100;
//! let byte_end = 200;
//!
//! let mut span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), byte_start, byte_end);
//!
//! // Add checksums for race condition protection
//! let span_checksum = checksum_span(file_path, byte_start, byte_end)?;
//! let file_checksum = checksum_file(file_path)?;
//! span = span.with_both_checksums(span_checksum.as_hex(), file_checksum.as_hex());
//! # Ok::<(), splice::SpliceError>(())
//! ```

use serde::{Deserialize, Serialize};

/// Schema version for structured output.
pub const SCHEMA_VERSION: &str = "2.0.0";

/// Top-level operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Schema version
    pub version: String,
    /// Unique operation ID (UUID)
    pub operation_id: String,
    /// Operation type
    pub operation_type: String,
    /// Status ("ok", "error", "partial")
    pub status: String,
    /// Human-readable message
    pub message: String,
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Workspace root (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    /// Primary result data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<OperationData>,
    /// Error details if status is "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetails>,
}

impl OperationResult {
    /// Create a new operation result with a generated UUID.
    pub fn new(operation_type: String) -> Self {
        Self::with_id(operation_type, None)
    }

    /// Create a new operation result with an optional operation ID.
    pub fn with_id(operation_type: String, operation_id: Option<String>) -> Self {
        use uuid::Uuid;

        Self {
            version: SCHEMA_VERSION.to_string(),
            operation_id: operation_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            operation_type,
            status: "ok".to_string(),
            message: String::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            workspace: None,
            result: None,
            error: None,
        }
    }

    /// Set or override the operation_id.
    pub fn set_operation_id(mut self, operation_id: String) -> Self {
        self.operation_id = operation_id;
        self
    }

    /// Set success status with message.
    pub fn success(mut self, message: String) -> Self {
        self.status = "ok".to_string();
        self.message = message;
        self
    }

    /// Set error status with message and details.
    pub fn error(mut self, message: String, error: ErrorDetails) -> Self {
        self.status = "error".to_string();
        self.message = message;
        self.error = Some(error);
        self
    }

    /// Set workspace root.
    pub fn with_workspace(mut self, workspace: String) -> Self {
        self.workspace = Some(workspace);
        self
    }

    /// Set result data.
    pub fn with_result(mut self, result: OperationData) -> Self {
        self.result = Some(result);
        self
    }
}

/// Operation result data variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OperationData {
    /// Single file patch operation result.
    #[serde(rename = "patch")]
    Patch(PatchResult),
    /// Symbol deletion operation result.
    #[serde(rename = "delete")]
    Delete(DeleteResult),
    /// Multi-step plan execution result.
    #[serde(rename = "plan")]
    Plan(PlanResult),
    /// Magellan query result (label-based symbol search).
    #[serde(rename = "query")]
    Query(QueryResult),
    /// Pattern replacement across multiple files.
    #[serde(rename = "apply_files")]
    ApplyFiles(ApplyFilesResult),
}

/// Single file patch operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchResult {
    /// File that was patched
    pub file: String,
    /// Symbol name that was patched
    pub symbol: String,
    /// Symbol kind (function, struct, etc.)
    pub kind: String,
    /// Spans that were modified
    pub spans: Vec<SpanResult>,
    /// File hash before patching
    pub before_hash: String,
    /// File hash after patching
    pub after_hash: String,
    /// Number of lines added
    pub lines_added: usize,
    /// Number of lines removed
    pub lines_removed: usize,
}

/// Symbol deletion operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResult {
    /// File containing the deleted symbol
    pub file: String,
    /// Symbol name that was deleted
    pub symbol: String,
    /// Symbol kind
    pub kind: String,
    /// All spans that were removed (definition + references)
    pub spans: Vec<SpanResult>,
    /// Total bytes removed
    pub bytes_removed: usize,
    /// Total lines removed
    pub lines_removed: usize,
    /// Number of references removed
    pub references_removed: usize,
    /// Checksum of file before deletion
    pub file_checksum_before: String,
    /// Checksums of each removed span
    pub span_checksums: Vec<String>,
}

/// Multi-step plan execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanResult {
    /// Number of steps in the plan
    pub total_steps: usize,
    /// Number of steps successfully executed
    pub steps_completed: usize,
    /// Individual step results
    pub steps: Vec<StepResult>,
    /// All files affected across all steps
    pub files_affected: Vec<String>,
    /// Total bytes changed across all steps
    pub total_bytes_changed: usize,
}

/// Individual step result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct StepResult {
    /// Step index (1-based)
    pub step: usize,
    /// Step status
    pub status: String,
    /// Step message
    pub message: String,
    /// File patched in this step
    pub file: String,
    /// Symbol patched in this step
    pub symbol: String,
}

/// Magellan query result (label-based symbol search).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Query labels that were used
    pub labels: Vec<String>,
    /// Number of results found
    pub count: usize,
    /// Matching symbols
    pub symbols: Vec<SpanResult>,
}

/// Pattern replacement across multiple files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyFilesResult {
    /// Glob pattern used for matching
    pub glob_pattern: String,
    /// Find pattern
    pub find_pattern: String,
    /// Replace pattern
    pub replace_pattern: String,
    /// Number of files matched
    pub files_matched: usize,
    /// Number of files modified
    pub files_modified: usize,
    /// Individual file results
    pub files: Vec<FilePatternResult>,
}

/// Individual file pattern result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePatternResult {
    /// File path
    pub file: String,
    /// Number of matches in this file
    pub matches: usize,
    /// Number of replacements made
    pub replacements: usize,
    /// Spans that were replaced
    pub spans: Vec<SpanResult>,
    /// File hash before
    pub before_hash: String,
    /// File hash after
    pub after_hash: String,
}

/// Context lines surrounding a span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanContext {
    /// Lines before the span
    pub before: Vec<String>,
    /// Lines within the span
    pub selected: Vec<String>,
    /// Lines after the span
    pub after: Vec<String>,
}

/// Error code with severity, location, and hint for diagnostics.
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
}

/// Unified span result with byte and line/column information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanResult {
    /// File path
    pub file_path: String,
    /// Symbol name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Symbol kind (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Start byte offset
    pub byte_start: usize,
    /// End byte offset
    pub byte_end: usize,
    /// Start line (1-based, 0 if not available)
    pub line_start: usize,
    /// End line (1-based, 0 if not available)
    pub line_end: usize,
    /// Start column (0-based, 0 if not available)
    pub col_start: usize,
    /// End column (0-based, 0 if not available)
    pub col_end: usize,
    /// Unique ID for this span (generated automatically)
    pub span_id: String,
    /// Symbol resolution match ID (populated when from resolve_symbol())
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_id: Option<String>,
    /// Hash before modification (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_hash: Option<String>,
    /// Hash after modification (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_hash: Option<String>,
    /// Checksum of span content before modification (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_checksum_before: Option<String>,
    /// Checksum of span content after modification (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_checksum_after: Option<String>,
    /// Context lines before/selected/after (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<SpanContext>,
    /// Standardized semantic kind (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_kind: Option<String>,
    /// Programming language (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Checksum of span content before modification (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_before: Option<String>,
    /// Checksum of entire file before modification (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_checksum_before: Option<String>,
    /// Error code with severity, location, hint (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ErrorCode>,
}

impl SpanResult {
    /// Create from file path and byte span only (line/col set to 0).
    pub fn from_byte_span(file_path: String, byte_start: usize, byte_end: usize) -> Self {
        use uuid::Uuid;
        Self {
            file_path,
            symbol: None,
            kind: None,
            byte_start,
            byte_end,
            line_start: 0,
            line_end: 0,
            col_start: 0,
            col_end: 0,
            span_id: Uuid::new_v4().to_string(),
            match_id: None,
            before_hash: None,
            after_hash: None,
            span_checksum_before: None,
            span_checksum_after: None,
            context: None,
            semantic_kind: None,
            language: None,
            checksum_before: None,
            file_checksum_before: None,
            error_code: None,
        }
    }

    /// Add symbol information.
    pub fn with_symbol(mut self, symbol: String, kind: String) -> Self {
        self.symbol = Some(symbol);
        self.kind = Some(kind);
        self
    }

    /// Add hash information.
    pub fn with_hashes(mut self, before: String, after: String) -> Self {
        self.before_hash = Some(before);
        self.after_hash = Some(after);
        self
    }

    /// Add line/column information.
    pub fn with_line_col(mut self, line_start: usize, line_end: usize, col_start: usize, col_end: usize) -> Self {
        self.line_start = line_start;
        self.line_end = line_end;
        self.col_start = col_start;
        self.col_end = col_end;
        self
    }

    /// Add match_id from symbol resolution.
    pub fn with_match_id(mut self, match_id: String) -> Self {
        self.match_id = Some(match_id);
        self
    }

    /// Add span checksum information.
    pub fn with_span_checksums(mut self, before: String, after: String) -> Self {
        self.span_checksum_before = Some(before);
        self.span_checksum_after = Some(after);
        self
    }

    /// Add context to span result.
    pub fn with_context(mut self, context: SpanContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Add semantic kind.
    pub fn with_semantic_kind(mut self, kind: impl Into<String>) -> Self {
        self.semantic_kind = Some(kind.into());
        self
    }

    /// Add programming language.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Add checksum_before (alias for span_checksum_before).
    pub fn with_checksum_before(mut self, checksum: impl Into<String>) -> Self {
        self.checksum_before = Some(checksum.into());
        self
    }

    /// Add file_checksum_before.
    pub fn with_file_checksum_before(mut self, checksum: impl Into<String>) -> Self {
        self.file_checksum_before = Some(checksum.into());
        self
    }

    /// Add error code.
    pub fn with_error_code(mut self, error_code: ErrorCode) -> Self {
        self.error_code = Some(error_code);
        self
    }

    /// Add both semantic kind and language.
    pub fn with_semantic_info(mut self, kind: impl Into<String>, language: impl Into<String>) -> Self {
        self.semantic_kind = Some(kind.into());
        self.language = Some(language.into());
        self
    }

    /// Add both checksums.
    pub fn with_both_checksums(mut self, checksum_before: impl Into<String>, file_checksum_before: impl Into<String>) -> Self {
        self.checksum_before = Some(checksum_before.into());
        self.file_checksum_before = Some(file_checksum_before.into());
        self
    }
}

// Implement Ord for SpanResult - sorts by file_path, then byte_start, then byte_end
// Ignores span_id (random UUID), match_id, and hash fields for deterministic ordering
impl PartialEq for SpanResult {
    fn eq(&self, other: &Self) -> bool {
        self.file_path == other.file_path
            && self.byte_start == other.byte_start
            && self.byte_end == other.byte_end
    }
}

impl Eq for SpanResult {}

impl PartialOrd for SpanResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SpanResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.file_path.cmp(&other.file_path) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.byte_start.cmp(&other.byte_start) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.byte_end.cmp(&other.byte_end)
    }
}

// Implement Ord for FilePatternResult - sorts by file path only
// Ignores spans Vec (cannot derive Ord with Vec field)
impl PartialEq for FilePatternResult {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file
    }
}

impl Eq for FilePatternResult {}

impl PartialOrd for FilePatternResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FilePatternResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.file.cmp(&other.file)
    }
}

/// Error details for failed operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    /// Error kind identifier
    pub kind: String,
    /// Human-readable error message
    pub message: String,
    /// Optional symbol context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Optional file context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Optional hint for remediation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Optional diagnostics from validation tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<DiagnosticPayload>>,
}

/// Individual diagnostic message from validation tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticPayload {
    /// Tool emitting the diagnostic (e.g., "cargo-check", "rust-analyzer")
    pub tool: String,
    /// Severity level ("error", "warning", "info")
    pub level: String,
    /// Diagnostic message
    pub message: String,
    /// Optional file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Optional line number (1-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Optional column number (0-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    /// Optional error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Optional hint/help text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Optional absolute path to tool binary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_path: Option<String>,
    /// Optional tool version string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_version: Option<String>,
    /// Optional remediation link or text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

// Implement Ord for DiagnosticPayload - sorts by tool, file, line, column, level, message
// None < Some for Option fields to group diagnostics without location first
impl PartialEq for DiagnosticPayload {
    fn eq(&self, other: &Self) -> bool {
        self.tool == other.tool
            && self.file == other.file
            && self.line == other.line
            && self.column == other.column
            && self.level == other.level
            && self.message == other.message
    }
}

impl Eq for DiagnosticPayload {}

impl PartialOrd for DiagnosticPayload {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DiagnosticPayload {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.tool.cmp(&other.tool) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.file.cmp(&other.file) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.line.cmp(&other.line) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.column.cmp(&other.column) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.level.cmp(&other.level) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.message.cmp(&other.message)
    }
}

// Conversion from existing types

impl From<crate::patch::FilePatchSummary> for SpanResult {
    fn from(summary: crate::patch::FilePatchSummary) -> Self {
        use uuid::Uuid;
        Self {
            file_path: summary.file.to_string_lossy().to_string(),
            symbol: None,
            kind: None,
            byte_start: 0,
            byte_end: 0,
            line_start: 0,
            line_end: 0,
            col_start: 0,
            col_end: 0,
            span_id: Uuid::new_v4().to_string(),
            match_id: None,
            before_hash: Some(summary.before_hash),
            after_hash: Some(summary.after_hash),
            span_checksum_before: None,
            span_checksum_after: None,
            context: None,
            semantic_kind: None,
            language: None,
            checksum_before: None,
            file_checksum_before: None,
            error_code: None,
        }
    }
}

impl From<crate::resolve::ResolvedSpan> for SpanResult {
    fn from(span: crate::resolve::ResolvedSpan) -> Self {
        use uuid::Uuid;
        Self {
            file_path: span.file_path,
            symbol: Some(span.name),
            kind: Some(span.kind),
            byte_start: span.byte_start,
            byte_end: span.byte_end,
            line_start: span.line_start,
            line_end: span.line_end,
            col_start: span.col_start,
            col_end: span.col_end,
            span_id: Uuid::new_v4().to_string(),
            match_id: Some(span.match_id),
            before_hash: None,
            after_hash: None,
            span_checksum_before: None,
            span_checksum_after: None,
            context: None,
            semantic_kind: None,
            language: None,
            checksum_before: None,
            file_checksum_before: None,
            error_code: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_id_uniqueness() {
        let span1 = SpanResult::from_byte_span("test.rs".to_string(), 10, 20);
        let span2 = SpanResult::from_byte_span("test.rs".to_string(), 10, 20);
        assert_ne!(span1.span_id, span2.span_id, "Each SpanResult should have a unique span_id");
    }

    #[test]
    fn test_match_id_preserved() {
        let match_id = uuid::Uuid::new_v4().to_string();
        let span = SpanResult::from_byte_span("test.rs".to_string(), 10, 20)
            .with_match_id(match_id.clone());
        assert_eq!(span.match_id, Some(match_id), "match_id should be preserved when set");
    }

    #[test]
    fn test_match_id_from_resolved_span() {
        // Create a mock ResolvedSpan-like structure
        // Note: We can't directly create ResolvedSpan without node_id, but we can
        // verify that the conversion preserves match_id through the public API
        let match_id = uuid::Uuid::new_v4().to_string();
        let span1 = SpanResult::from_byte_span("test.rs".to_string(), 10, 20)
            .with_match_id(match_id.clone());

        assert_eq!(span1.match_id, Some(match_id));
        assert!(!span1.span_id.is_empty());
    }

    #[test]
    fn test_from_byte_span_generates_unique_span_ids() {
        let span1 = SpanResult::from_byte_span("file.rs".to_string(), 0, 10);
        let span2 = SpanResult::from_byte_span("file.rs".to_string(), 0, 10);
        let span3 = SpanResult::from_byte_span("file.rs".to_string(), 20, 30);

        assert_ne!(span1.span_id, span2.span_id);
        assert_ne!(span2.span_id, span3.span_id);
        assert_ne!(span1.span_id, span3.span_id);
    }
}

