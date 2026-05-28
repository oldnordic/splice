use serde::{Deserialize, Serialize};

use crate::action::SuggestedAction;
use crate::error_codes::ErrorCode;
use crate::hints::ToolHints;
use crate::relationships::Relationships;

use super::core::*;

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
impl PartialEq for super::core::FilePatternResult {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file
    }
}

impl Eq for super::core::FilePatternResult {}

impl PartialOrd for super::core::FilePatternResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for super::core::FilePatternResult {
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
