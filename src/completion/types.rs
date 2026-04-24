use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Request for code completion at cursor position
#[derive(Debug, Clone, Deserialize)]
pub struct CompletionRequest {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub max_results: Option<usize>,
}

/// Completion response with grounded suggestions
#[derive(Debug, Clone, Serialize)]
pub struct CompletionResponse {
    pub suggestions: Vec<CompletionSuggestion>,
    pub metadata: CompletionMetadata,
}

/// A single completion suggestion grounded in database
#[derive(Debug, Clone, Serialize)]
pub struct CompletionSuggestion {
    /// Display label for user
    pub label: String,
    /// Text to insert (may include placeholders like ${1:data})
    pub insert_text: String,
    /// Detail information (type signature, docs, etc)
    pub detail: String,
    /// Kind of symbol (function, struct, enum, etc)
    pub kind: SymbolKind,
    /// Relevance score [0, 1]
    pub score: f32,
    /// Where this suggestion came from
    pub source: SuggestionSource,
    /// Database IDs proving this exists (grounding)
    pub grounded_in: Vec<String>,
    /// How many times this pattern appears
    pub usage_count: usize,
    /// When this was last used (Unix timestamp)
    pub last_used: Option<i64>,
    /// Source file path (if cross-file import)
    pub source_file: Option<String>,
    /// Import statement that makes this available (e.g., "use crate::action::calculate")
    pub via_import: Option<String>,
}

/// Source of suggestion
#[derive(Debug, Clone, Serialize)]
pub enum SuggestionSource {
    /// Direct database query result (same file)
    Database,
    /// From imported module (cross-file)
    Imported,
    /// Extracted from recurring patterns
    Pattern,
    /// Found via code similarity
    Similarity,
    /// Project convention
    Convention,
}

/// Symbol kind enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    Variable,
    Constant,
    TypeAlias,
    Constructor,
}

/// Metadata about completion request
#[derive(Debug, Clone, Serialize)]
pub struct CompletionMetadata {
    /// Query time in milliseconds
    pub query_time_ms: u64,
    /// Total symbols in database
    pub total_symbols_indexed: usize,
    /// Database schema version
    pub database_version: u32,
    /// Number of database queries executed
    pub database_queries: usize,
}

/// Result of symbol lookup
#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: String,
    pub name: String,
    pub kind: SymbolKind,
    pub path: String,
    pub line: usize,
    pub column: usize,
}
