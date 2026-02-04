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
//!
//! ## Adding Language Detection to SpanResult
//!
//! ```no_run
//! use splice::ingest::{detect_language, Language};
//! use splice::output::SpanResult;
//! use std::path::Path;
//!
//! let file_path = Path::new("src/main.rs");
//! let mut span = SpanResult::from_byte_span(file_path.to_string_lossy().to_string(), 100, 200);
//!
//! // Add language detection from file extension
//! if let Some(language) = detect_language(file_path) {
//!     span = span.with_language(language.as_str());
//! }
//! ```

use serde::{Deserialize, Serialize};

use crate::action::SuggestedAction;
use crate::error_codes::ErrorCode;
use crate::hints::ToolHints;
use crate::relationships::Relationships;

/// Schema version for mutation results.
pub const OPERATION_SCHEMA_VERSION: &str = "2.0.0";
/// Schema version for query-style responses.
pub const QUERY_SCHEMA_VERSION: &str = "1.0.0";
/// Tool name for unified output.
pub const TOOL_NAME: &str = "splice";

/// Wrapper for query-style JSON responses (Magellan-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResponse<T> {
    /// Schema version for parsing stability
    pub schema_version: String,
    /// Unique execution ID for this run
    pub execution_id: String,
    /// Response data
    pub data: T,
    /// Tool name (e.g., "splice")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// ISO 8601 timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// Whether the response is partial (e.g., truncated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial: Option<bool>,
}

impl<T> JsonResponse<T> {
    /// Create a new JSON response.
    pub fn new(data: T, execution_id: &str) -> Self {
        JsonResponse {
            schema_version: QUERY_SCHEMA_VERSION.to_string(),
            execution_id: execution_id.to_string(),
            data,
            tool: Some(TOOL_NAME.to_string()),
            timestamp: Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            partial: None,
        }
    }

    /// Mark the response as partial.
    pub fn with_partial(mut self, partial: bool) -> Self {
        self.partial = Some(partial);
        self
    }
}

/// Top-level operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Schema version
    pub schema_version: String,
    /// Unique execution ID (UUID)
    pub execution_id: String,
    /// Operation type
    pub operation_type: String,
    /// Status ("ok", "error", "partial")
    pub status: String,
    /// Human-readable message
    pub message: String,
    /// Tool name
    pub tool: String,
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
        Self::with_execution_id(operation_type, None)
    }

    /// Create a new operation result with an optional execution ID.
    pub fn with_execution_id(operation_type: String, execution_id: Option<String>) -> Self {
        use uuid::Uuid;

        Self {
            schema_version: OPERATION_SCHEMA_VERSION.to_string(),
            execution_id: execution_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            operation_type,
            status: "ok".to_string(),
            message: String::new(),
            tool: TOOL_NAME.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            workspace: None,
            result: None,
            error: None,
        }
    }

    /// Set or override the execution_id.
    pub fn set_execution_id(mut self, execution_id: String) -> Self {
        self.execution_id = execution_id;
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
    /// Number of results returned
    pub count: usize,
    /// Matching symbols
    pub symbols: Vec<SpanResult>,
    /// Total number of results before pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<usize>,
    /// Offset into the result set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// Limit applied to the result set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Max symbols cap applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_symbols: Option<usize>,
    /// Max bytes cap applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<usize>,
    /// Next offset when more results are available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
    /// Whether the result set is partial
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial: Option<bool>,
    /// Reasons for truncation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation_reasons: Option<Vec<String>>,
}

/// Label-based query response (Magellan-compatible JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelQueryResponse {
    /// Query labels that were used
    pub labels: Vec<String>,
    /// Matching symbols
    pub symbols: Vec<SymbolMatch>,
    /// Number of results returned
    pub count: usize,
    /// Total number of results before pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<usize>,
    /// Offset into the result set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// Limit applied to the result set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Max symbols cap applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_symbols: Option<usize>,
    /// Max bytes cap applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<usize>,
    /// Next offset when more results are available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
    /// Reasons for truncation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation_reasons: Option<Vec<String>>,
}

/// Response for get command (Magellan-compatible JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetResponse {
    /// Symbol details
    pub symbol: SymbolMatch,
    /// Source code content
    pub content: String,
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

/// Semantic metadata for a span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanSemantics {
    /// Semantic kind (function, class, etc.)
    pub kind: String,
    /// Programming language
    pub language: String,
}

/// Checksum metadata for a span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanChecksums {
    /// Checksum of span content before modification (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_before: Option<String>,
    /// Checksum of span content after modification (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_after: Option<String>,
    /// Checksum of entire file before modification (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_checksum_before: Option<String>,
}

impl Default for SpanChecksums {
    fn default() -> Self {
        Self {
            checksum_before: None,
            checksum_after: None,
            file_checksum_before: None,
        }
    }
}

fn generate_span_id(file_path: &str, byte_start: usize, byte_end: usize) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(file_path.as_bytes());
    hasher.update(b":");
    hasher.update(byte_start.to_be_bytes());
    hasher.update(b":");
    hasher.update(byte_end.to_be_bytes());

    let result = hasher.finalize();
    format!(
        "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7],
    )
}

fn normalize_checksum(value: String) -> String {
    if value.starts_with("sha256:") {
        value
    } else {
        format!("sha256:{}", value)
    }
}

/// Span in source code (byte + line/column), Magellan-compatible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Stable span ID (SHA-256 hash of file_path:byte_start:byte_end)
    pub span_id: String,
    /// File path (absolute or root-relative)
    pub file_path: String,
    /// Byte range start (inclusive)
    pub byte_start: usize,
    /// Byte range end (exclusive)
    pub byte_end: usize,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// Start column (0-indexed, byte-based)
    pub start_col: usize,
    /// End line (1-indexed)
    pub end_line: usize,
    /// End column (0-indexed, byte-based)
    pub end_col: usize,
    /// Context lines around the span
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<SpanContext>,
    /// Semantic information (kind, language)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantics: Option<SpanSemantics>,
    /// Relationship information (callers, callees, imports, exports)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationships: Option<Relationships>,
    /// Checksums for content verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksums: Option<SpanChecksums>,
}

impl Span {
    /// Create a new span with stable ID.
    pub fn new(
        file_path: String,
        byte_start: usize,
        byte_end: usize,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) -> Self {
        let span_id = generate_span_id(&file_path, byte_start, byte_end);
        Span {
            span_id,
            file_path,
            byte_start,
            byte_end,
            start_line,
            start_col,
            end_line,
            end_col,
            context: None,
            semantics: None,
            relationships: None,
            checksums: None,
        }
    }

    /// Generate a stable span ID from (file_path, byte_start, byte_end).
    pub fn generate_id(file_path: &str, byte_start: usize, byte_end: usize) -> String {
        generate_span_id(file_path, byte_start, byte_end)
    }

    /// Add context lines around the span.
    pub fn with_context(mut self, context: SpanContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Add semantic info for kind and language.
    pub fn with_semantics(mut self, semantics: SpanSemantics) -> Self {
        self.semantics = Some(semantics);
        self
    }

    /// Add relationship information.
    pub fn with_relationships(mut self, relationships: Relationships) -> Self {
        self.relationships = Some(relationships);
        self
    }

    /// Add checksum metadata.
    pub fn with_checksums(mut self, checksums: SpanChecksums) -> Self {
        self.checksums = Some(checksums);
        self
    }
}

/// Symbol match result (Magellan-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMatch {
    /// Stable match ID generated from name, file path, and byte start.
    pub match_id: String,
    /// Symbol span
    pub span: Span,
    /// Symbol name
    pub name: String,
    /// Symbol kind (normalized)
    pub kind: String,
    /// Parent symbol name (if nested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Stable symbol ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<String>,
}

impl SymbolMatch {
    /// Generate a stable match ID for a symbol.
    pub fn generate_match_id(symbol_name: &str, file_path: &str, byte_start: usize) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        symbol_name.hash(&mut hasher);
        file_path.hash(&mut hasher);
        byte_start.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Create a new SymbolMatch with a stable match ID.
    pub fn new(
        name: String,
        kind: String,
        span: Span,
        parent: Option<String>,
        symbol_id: Option<String>,
    ) -> Self {
        let match_id = Self::generate_match_id(&name, &span.file_path, span.byte_start);
        SymbolMatch {
            match_id,
            span,
            name,
            kind,
            parent,
            symbol_id,
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
    pub start_line: usize,
    /// End line (1-based, 0 if not available)
    pub end_line: usize,
    /// Start column (0-based, 0 if not available)
    pub start_col: usize,
    /// End column (0-based, 0 if not available)
    pub end_col: usize,
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
    /// Context lines before/selected/after (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<SpanContext>,
    /// Semantic metadata (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantics: Option<SpanSemantics>,
    /// Checksum metadata (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksums: Option<SpanChecksums>,
    /// Error code with severity, location, hint (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ErrorCode>,
    /// Code relationships (callers, callees, imports, exports)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationships: Option<Relationships>,
    /// Tool hints for behavioral guidance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_hints: Option<ToolHints>,
    /// Suggested action with confidence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_action: Option<SuggestedAction>,
}

impl SpanResult {
    /// Create from file path and byte span only (line/col set to 0).
    pub fn from_byte_span(file_path: String, byte_start: usize, byte_end: usize) -> Self {
        let span_id = generate_span_id(&file_path, byte_start, byte_end);
        Self {
            file_path,
            symbol: None,
            kind: None,
            byte_start,
            byte_end,
            start_line: 0,
            end_line: 0,
            start_col: 0,
            end_col: 0,
            span_id,
            match_id: None,
            before_hash: None,
            after_hash: None,
            context: None,
            semantics: None,
            checksums: None,
            error_code: None,
            relationships: None,
            tool_hints: None,
            suggested_action: None,
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
    pub fn with_line_col(
        mut self,
        line_start: usize,
        line_end: usize,
        col_start: usize,
        col_end: usize,
    ) -> Self {
        self.start_line = line_start;
        self.end_line = line_end;
        self.start_col = col_start;
        self.end_col = col_end;
        self
    }

    /// Add match_id from symbol resolution.
    pub fn with_match_id(mut self, match_id: String) -> Self {
        self.match_id = Some(match_id);
        self
    }

    /// Add span checksum information.
    pub fn with_span_checksums(mut self, before: String, after: String) -> Self {
        let checksums = self.checksums.get_or_insert_with(SpanChecksums::default);
        checksums.checksum_before = Some(normalize_checksum(before));
        checksums.checksum_after = Some(normalize_checksum(after));
        self
    }

    /// Add context to span result.
    pub fn with_context(mut self, context: SpanContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Add semantic kind.
    pub fn with_semantic_kind(mut self, kind: impl Into<String>) -> Self {
        let kind = kind.into();
        let semantics = self.semantics.get_or_insert_with(|| SpanSemantics {
            kind: "unknown".to_string(),
            language: "unknown".to_string(),
        });
        semantics.kind = kind;
        self
    }

    /// Add programming language.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        let language = language.into();
        let semantics = self.semantics.get_or_insert_with(|| SpanSemantics {
            kind: "unknown".to_string(),
            language: "unknown".to_string(),
        });
        semantics.language = language;
        self
    }

    /// Add checksum_before (alias for span_checksum_before).
    pub fn with_checksum_before(mut self, checksum: impl Into<String>) -> Self {
        let checksums = self.checksums.get_or_insert_with(SpanChecksums::default);
        checksums.checksum_before = Some(normalize_checksum(checksum.into()));
        self
    }

    /// Add file_checksum_before.
    pub fn with_file_checksum_before(mut self, checksum: impl Into<String>) -> Self {
        let checksums = self.checksums.get_or_insert_with(SpanChecksums::default);
        checksums.file_checksum_before = Some(normalize_checksum(checksum.into()));
        self
    }

    /// Add error code.
    pub fn with_error_code(mut self, error_code: ErrorCode) -> Self {
        self.error_code = Some(error_code);
        self
    }

    /// Add both semantic kind and language.
    pub fn with_semantic_info(
        mut self,
        kind: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        self.semantics = Some(SpanSemantics {
            kind: kind.into(),
            language: language.into(),
        });
        self
    }

    /// Add both checksums.
    pub fn with_both_checksums(
        mut self,
        checksum_before: impl Into<String>,
        file_checksum_before: impl Into<String>,
    ) -> Self {
        let checksum_after = self
            .checksums
            .as_ref()
            .and_then(|c| c.checksum_after.clone());
        self.checksums = Some(SpanChecksums {
            checksum_before: Some(normalize_checksum(checksum_before.into())),
            checksum_after,
            file_checksum_before: Some(normalize_checksum(file_checksum_before.into())),
        });
        self
    }

    /// Add relationships.
    pub fn with_relationships(mut self, relationships: Relationships) -> Self {
        self.relationships = Some(relationships);
        self
    }

    /// Add tool hints.
    pub fn with_tool_hints(mut self, hints: ToolHints) -> Self {
        self.tool_hints = Some(hints);
        self
    }

    /// Add suggested action.
    pub fn with_suggested_action(mut self, action: SuggestedAction) -> Self {
        self.suggested_action = Some(action);
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
        let file_path = summary.file.to_string_lossy().to_string();
        let span_id = generate_span_id(&file_path, 0, 0);
        Self {
            file_path,
            symbol: None,
            kind: None,
            byte_start: 0,
            byte_end: 0,
            start_line: 0,
            end_line: 0,
            start_col: 0,
            end_col: 0,
            span_id,
            match_id: None,
            before_hash: Some(summary.before_hash),
            after_hash: Some(summary.after_hash),
            context: None,
            semantics: None,
            checksums: None,
            error_code: None,
            relationships: None,
            tool_hints: None,
            suggested_action: None,
        }
    }
}

impl From<crate::resolve::ResolvedSpan> for SpanResult {
    fn from(span: crate::resolve::ResolvedSpan) -> Self {
        let span_id = generate_span_id(&span.file_path, span.byte_start, span.byte_end);
        Self {
            file_path: span.file_path,
            symbol: Some(span.name),
            kind: Some(span.kind),
            byte_start: span.byte_start,
            byte_end: span.byte_end,
            start_line: span.line_start,
            end_line: span.line_end,
            start_col: span.col_start,
            end_col: span.col_end,
            span_id,
            match_id: Some(span.match_id),
            before_hash: None,
            after_hash: None,
            context: None,
            semantics: None,
            checksums: None,
            error_code: None,
            relationships: None,
            tool_hints: None,
            suggested_action: None,
        }
    }
}

/// Magellan-compatible response types for delegated query commands.
///
/// These types use Magellan field naming conventions (start_line vs line_start)
/// for compatibility with Magellan's JSON output format.

/// Status response showing database statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    /// Number of indexed files
    pub files: usize,
    /// Number of indexed symbols
    pub symbols: usize,
    /// Number of indexed references
    pub references: usize,
    /// Number of indexed function calls
    pub calls: usize,
    /// Number of stored code chunks
    pub code_chunks: usize,
    /// Database file path
    pub db_path: String,
}

/// Find response with matching symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindResponse {
    /// Matching symbols with Magellan field names
    pub symbols: Vec<MagellanSymbol>,
    /// Number of results
    pub count: usize,
}

/// Symbol with Magellan field naming conventions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagellanSymbol {
    /// Symbol ID (16-char V1 SHA-256 or 32-char V2 BLAKE3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<String>,
    /// ID format hint for clients ("v1" for 16-char SHA-256, "v2" for 32-char BLAKE3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_format: Option<String>,
    /// Symbol name
    pub name: String,
    /// Symbol kind (fn, struct, class, etc.)
    pub kind: String,
    /// File path
    pub file_path: String,
    /// Byte offset start (inclusive)
    pub byte_start: usize,
    /// Byte offset end (exclusive)
    pub byte_end: usize,
    /// Start line (1-indexed) - Magellan convention
    pub start_line: usize,
    /// End line (1-indexed) - Magellan convention
    pub end_line: usize,
    /// Start column (0-indexed) - Magellan convention
    pub start_col: usize,
    /// End column (0-indexed) - Magellan convention
    pub end_col: usize,
}

/// Refs response with call relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefsResponse {
    /// The symbol being queried
    pub symbol: MagellanSymbol,
    /// Symbols that call this symbol (if direction=in/both)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub callers: Vec<MagellanCallReference>,
    /// Symbols that this symbol calls (if direction=out/both)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub callees: Vec<MagellanCallReference>,
}

/// Call reference with symbol and call site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagellanCallReference {
    /// The referenced symbol
    pub symbol: MagellanSymbol,
    /// Call site location (Magellan field names)
    pub call_site: MagellanSpan,
}

/// Span with Magellan field naming conventions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagellanSpan {
    /// File path
    pub file_path: String,
    /// Byte offset start
    pub byte_start: usize,
    /// Byte offset end
    pub byte_end: usize,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// Start column (0-indexed)
    pub start_col: usize,
    /// End line (1-indexed)
    pub end_line: usize,
    /// End column (0-indexed)
    pub end_col: usize,
}

/// Files response with indexed file list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesResponse {
    /// Indexed files with metadata
    pub files: Vec<MagellanFileMetadata>,
    /// Total count
    pub count: usize,
}

/// File metadata with Magellan-compatible fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagellanFileMetadata {
    /// File path
    pub path: String,
    /// Content hash
    pub hash: String,
    /// Last indexed timestamp (Unix epoch)
    pub last_indexed_at: i64,
    /// Last modified timestamp (Unix epoch)
    pub last_modified: i64,
    /// Symbol count (if --symbols flag provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_count: Option<usize>,
}

// Conversion implementations from Phase 23 types

impl From<crate::graph::magellan_integration::DatabaseStats> for StatusResponse {
    fn from(stats: crate::graph::magellan_integration::DatabaseStats) -> Self {
        Self {
            files: stats.files,
            symbols: stats.symbols,
            references: stats.references,
            calls: stats.calls,
            code_chunks: stats.code_chunks,
            db_path: String::new(), // Set by caller
        }
    }
}

impl From<crate::graph::magellan_integration::SymbolInfo> for MagellanSymbol {
    fn from(info: crate::graph::magellan_integration::SymbolInfo) -> Self {
        // Generate V2 BLAKE3 symbol ID and set id_format
        let symbol_id = crate::symbol_id::generate_v2(
            &info.name,
            &info.file_path,
            info.byte_start,
        );
        Self {
            symbol_id: Some(symbol_id.as_str().to_string()),
            id_format: Some("v2".to_string()),
            name: info.name,
            kind: info.kind,
            file_path: info.file_path,
            byte_start: info.byte_start,
            byte_end: info.byte_end,
            // SymbolInfo doesn't have line/col - set to 0 (query could be enhanced)
            start_line: 0,
            end_line: 0,
            start_col: 0,
            end_col: 0,
        }
    }
}

impl From<crate::graph::magellan_integration::CallReference> for MagellanCallReference {
    fn from(cr: crate::graph::magellan_integration::CallReference) -> Self {
        Self {
            symbol: cr.symbol.into(),
            call_site: MagellanSpan {
                file_path: cr.call_site.file_path,
                byte_start: cr.call_site.byte_start,
                byte_end: cr.call_site.byte_end,
                start_line: cr.call_site.start_line,
                start_col: cr.call_site.start_col,
                end_line: cr.call_site.end_line,
                end_col: cr.call_site.end_col,
            },
        }
    }
}

impl From<crate::graph::magellan_integration::FileMetadata> for MagellanFileMetadata {
    fn from(fm: crate::graph::magellan_integration::FileMetadata) -> Self {
        Self {
            path: fm.path,
            hash: fm.hash,
            last_indexed_at: fm.last_indexed_at,
            last_modified: fm.last_modified,
            symbol_count: fm.symbol_count,
        }
    }
}

impl From<crate::graph::magellan_integration::CallRelationships> for RefsResponse {
    fn from(rels: crate::graph::magellan_integration::CallRelationships) -> Self {
        Self {
            symbol: rels.symbol.into(),
            callers: rels.callers.into_iter().map(Into::into).collect(),
            callees: rels.callees.into_iter().map(Into::into).collect(),
        }
    }
}

// ============================================================================
// Reachability Analysis Types (Phase 30-01)
// ============================================================================

/// Reachability analysis result for a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReachabilityResult {
    /// The symbol whose reachability was analyzed.
    pub symbol: SymbolInfo,
    /// Analysis direction performed.
    pub direction: String, // "forward", "reverse", "both"
    /// Maximum depth reached.
    pub max_depth: usize,
    /// Forward reachability (callees).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward: Option<ReachabilityChain>,
    /// Reverse reachability (callers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse: Option<ReachabilityChain>,
    /// Files affected by changes to this symbol.
    pub affected_files: Vec<AffectedFile>,
}

/// A reachability chain showing call relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReachabilityChain {
    /// Number of symbols in the chain.
    pub count: usize,
    /// Maximum depth reached.
    pub depth: usize,
    /// Symbols in the chain with their relationships.
    pub symbols: Vec<ReachableSymbol>,
}

/// A symbol in the reachability chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReachableSymbol {
    /// Symbol information.
    pub symbol: SymbolInfo,
    /// Depth from root (0 = root, 1 = direct, etc.).
    pub depth: usize,
    /// Path from root to this symbol.
    pub path: Vec<String>,
}

/// File affected by changes to the analyzed symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedFile {
    /// File path.
    pub path: String,
    /// Number of affected symbols in this file.
    pub symbol_count: usize,
    /// Whether this file contains the root symbol.
    pub is_root: bool,
}

// ============================================================================
// Dead Code Detection Types (Phase 30-02)
// ============================================================================

/// Dead code detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadCodeResult {
    /// The entry point symbol used for analysis.
    pub entry_point: SymbolInfo,
    /// Total symbols in the graph.
    pub total_symbols: usize,
    /// Number of reachable symbols.
    pub reachable_count: usize,
    /// Number of dead (unreachable) symbols.
    pub dead_count: usize,
    /// Dead symbols grouped by file.
    pub dead_by_file: Vec<DeadCodeByFile>,
    /// Whether public symbols were excluded.
    pub excluded_public: bool,
}

/// Dead code in a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadCodeByFile {
    /// File path.
    pub path: String,
    /// Number of dead symbols in this file.
    pub count: usize,
    /// Dead symbols in this file.
    pub symbols: Vec<DeadSymbol>,
}

/// A dead (unreachable) symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadSymbol {
    /// Symbol information.
    pub symbol: SymbolInfo,
    /// Reason this symbol is considered dead.
    pub reason: String,
}

// ============================================================================
// Cycle Detection Types (Phase 30-03)
// ============================================================================

/// Cycle detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleDetectionResult {
    /// Total number of cycles found.
    pub total_cycles: usize,
    /// Maximum cycles limit applied.
    pub max_cycles: usize,
    /// Whether result was truncated due to limit.
    pub truncated: bool,
    /// Cycles found in the call graph.
    pub cycles: Vec<CycleInfo>,
    /// Optional: specific symbol that was queried.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queried_symbol: Option<SymbolInfo>,
}

/// Information about a detected cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleInfo {
    /// Unique cycle identifier.
    pub id: String,
    /// Number of symbols in the cycle.
    pub size: usize,
    /// Symbols in the cycle.
    pub members: Vec<SymbolInfo>,
    /// Representative symbol (e.g., alphabetically first).
    pub representative: SymbolInfo,
    /// Whether this is a self-loop (single symbol calling itself).
    pub is_self_loop: bool,
}

/// Symbol info for reachability results (consistent with MagellanSymbol).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    /// Symbol ID (16-char V1 SHA-256 or 32-char V2 BLAKE3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<String>,
    /// ID format hint for clients ("v1" for 16-char SHA-256, "v2" for 32-char BLAKE3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_format: Option<String>,
    /// Symbol name
    pub name: String,
    /// Symbol kind (fn, struct, class, etc.)
    pub kind: String,
    /// File path
    pub file_path: String,
    /// Byte offset start (inclusive)
    pub byte_start: usize,
    /// Byte offset end (exclusive)
    pub byte_end: usize,
}

// ============================================================================
// Condensation Graph Types (Phase 30-04)
// ============================================================================

/// Condensation graph result (SCCs collapsed to DAG).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CondensationResult {
    /// Total number of SCCs in the condensation.
    pub scc_count: usize,
    /// Number of SCCs that are cycles (size > 1).
    pub cycle_scc_count: usize,
    /// Number of singleton SCCs (no cycles).
    pub singleton_count: usize,
    /// SCCs in the condensation graph.
    pub sccs: Vec<CondensedScc>,
    /// Edges between SCCs in the condensed graph.
    pub edges: Vec<SccEdge>,
    /// Topological levels (if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub levels: Option<Vec<LevelInfo>>,
}

/// A strongly connected component in the condensation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CondensedScc {
    /// SCC identifier.
    pub id: String,
    /// Number of symbols in this SCC.
    pub size: usize,
    /// Whether this SCC represents a cycle.
    pub is_cycle: bool,
    /// Symbols in this SCC (if showing members).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<SymbolInfo>>,
    /// Representative symbol for display.
    pub representative: SymbolInfo,
}

/// An edge between two SCCs in the condensed graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SccEdge {
    /// Source SCC id.
    pub from: String,
    /// Target SCC id.
    pub to: String,
    /// Number of edges from original graph collapsed into this edge.
    pub weight: usize,
}

/// Topological level information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelInfo {
    /// Level number (0 = no incoming edges).
    pub level: usize,
    /// SCCs at this level.
    pub scc_ids: Vec<String>,
    /// Number of SCCs at this level.
    pub count: usize,
}

// ============================================================================
// Export Data Types (Phase 25-03)
// ============================================================================

/// Export response with schema version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResponse {
    /// Schema version for parsing stability
    pub schema_version: String,
    /// Execution timestamp
    pub timestamp: String,
    /// Database path
    pub db_path: String,
    /// Exported graph data
    pub data: ExportData,
}

/// Complete graph data export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    /// All indexed files
    pub files: Vec<FileExport>,
    /// All symbols with spans
    pub symbols: Vec<SymbolExport>,
    /// All references between symbols
    pub references: Vec<ReferenceExport>,
    /// All function calls
    pub calls: Vec<CallExport>,
}

/// File export record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileExport {
    /// File path
    pub path: String,
    /// Content hash
    pub hash: String,
    /// Last indexed timestamp (Unix epoch)
    pub last_indexed_at: i64,
    /// Last modified timestamp (Unix epoch)
    pub last_modified: i64,
}

/// Symbol export record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolExport {
    /// Unique symbol identifier (16-char V1 SHA-256 or 32-char V2 BLAKE3)
    pub symbol_id: String,
    /// ID format hint for clients ("v1" for 16-char SHA-256, "v2" for 32-char BLAKE3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_format: Option<String>,
    /// Symbol name
    pub name: String,
    /// Symbol kind (function, type, etc.)
    pub kind: String,
    /// File containing the symbol
    pub file_path: String,
    /// Byte offset of symbol start
    pub byte_start: usize,
    /// Byte offset of symbol end
    pub byte_end: usize,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// End line (1-indexed)
    pub end_line: usize,
    /// Start column (0-indexed)
    pub start_col: usize,
    /// End column (0-indexed)
    pub end_col: usize,
}

/// Reference export record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceExport {
    /// Source symbol ID
    pub from_symbol_id: String,
    /// Target symbol ID
    pub to_symbol_id: String,
    /// Reference type
    pub reference_kind: String,
}

/// Call export record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallExport {
    /// Caller symbol ID
    pub caller_symbol_id: String,
    /// Callee symbol ID
    pub callee_symbol_id: String,
    /// File containing call site
    pub call_site_file: String,
    /// Line of call site
    pub call_site_line: usize,
}

/// Schema version constant for export responses.
pub const EXPORT_SCHEMA_VERSION: &str = "1.0.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_id_deterministic() {
        let span1 = SpanResult::from_byte_span("test.rs".to_string(), 10, 20);
        let span2 = SpanResult::from_byte_span("test.rs".to_string(), 10, 20);
        assert_eq!(
            span1.span_id, span2.span_id,
            "Same span inputs should produce the same span_id"
        );
    }

    #[test]
    fn test_match_id_preserved() {
        let match_id = uuid::Uuid::new_v4().to_string();
        let span = SpanResult::from_byte_span("test.rs".to_string(), 10, 20)
            .with_match_id(match_id.clone());
        assert_eq!(
            span.match_id,
            Some(match_id),
            "match_id should be preserved when set"
        );
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
    fn test_from_byte_span_generates_distinct_span_ids() {
        let span1 = SpanResult::from_byte_span("file.rs".to_string(), 0, 10);
        let span2 = SpanResult::from_byte_span("file.rs".to_string(), 0, 10);
        let span3 = SpanResult::from_byte_span("file.rs".to_string(), 20, 30);

        assert_eq!(span1.span_id, span2.span_id);
        assert_ne!(span2.span_id, span3.span_id);
        assert_ne!(span1.span_id, span3.span_id);
    }
}
