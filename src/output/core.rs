use serde::{Deserialize, Serialize};

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
    pub error: Option<super::span_result::ErrorDetails>,
    /// Preview report for dry-run operations (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_report: Option<serde_json::Value>,
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
            preview_report: None,
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
    pub fn error(mut self, message: String, error: super::span_result::ErrorDetails) -> Self {
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

    /// Set preview report for dry-run operations.
    pub fn with_preview_report(mut self, preview_report: serde_json::Value) -> Self {
        self.preview_report = Some(preview_report);
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
    pub spans: Vec<super::span_result::SpanResult>,
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
    pub spans: Vec<super::span_result::SpanResult>,
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
    pub symbols: Vec<super::span_result::SpanResult>,
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
    pub spans: Vec<super::span_result::SpanResult>,
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

/// Generate a stable span ID from file path and byte range.
pub fn generate_span_id(file_path: &str, byte_start: usize, byte_end: usize) -> String {
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

/// Normalize a checksum value to include the `sha256:` prefix.
pub fn normalize_checksum(value: String) -> String {
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
