//! Shared types for Magellan integration.

use magellan::SymbolQueryResult;

/// Backend type identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Backend type for Magellan integration.
pub enum IntegrationBackend {
    /// SQLite database backend.
    Sqlite,
    /// Geometric spatial backend.
    #[cfg(feature = "geometric")]
    Geometric,
}

/// Symbol information extracted from Magellan's SymbolQueryResult.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// Entity ID in the graph database.
    pub entity_id: i64,
    /// Symbol name.
    pub name: String,
    /// File path containing the symbol.
    pub file_path: String,
    /// Symbol kind (e.g., "fn", "struct", "class").
    pub kind: String,
    /// Byte offset where the symbol starts.
    pub byte_start: usize,
    /// Byte offset where the symbol ends.
    pub byte_end: usize,
    /// Line number where the symbol starts (1-indexed).
    pub start_line: Option<usize>,
    /// Line number where the symbol ends (1-indexed).
    pub end_line: Option<usize>,
}

/// Symbol with optional call relationship context.
#[derive(Debug, Clone)]
pub struct SymbolWithRelations {
    /// The symbol's basic information.
    pub symbol: SymbolInfo,
    /// Symbols that call this symbol (if --with-callers flag).
    pub callers: Vec<SymbolInfo>,
    /// Symbols that this symbol calls (if --with-callees flag).
    pub callees: Vec<SymbolInfo>,
}

/// Direction for call relationship traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDirection {
    /// Get callers only (symbols that call this symbol).
    In,
    /// Get callees only (symbols that this symbol calls).
    Out,
    /// Get both callers and callees.
    Both,
}

/// Location of a call in source code.
#[derive(Debug, Clone)]
pub struct CallSite {
    /// File path containing the call.
    pub file_path: String,
    /// Byte offset where call starts.
    pub byte_start: usize,
    /// Byte offset where call ends.
    pub byte_end: usize,
    /// Line number where call starts (1-indexed).
    pub start_line: usize,
    /// Column number where call starts (0-indexed).
    pub start_col: usize,
    /// Line number where call ends (1-indexed).
    pub end_line: usize,
    /// Column number where call ends (0-indexed).
    pub end_col: usize,
}

/// A call relationship reference with symbol and call site.
#[derive(Debug, Clone)]
pub struct CallReference {
    /// The symbol being referenced (caller or callee).
    pub symbol: SymbolInfo,
    /// Location of the call site.
    pub call_site: CallSite,
}

/// Call relationships for a symbol.
#[derive(Debug, Clone)]
pub struct CallRelationships {
    /// The symbol whose relationships are being queried.
    pub symbol: SymbolInfo,
    /// Symbols that call this symbol (if direction is In or Both).
    pub callers: Vec<CallReference>,
    /// Symbols that this symbol calls (if direction is Out or Both).
    pub callees: Vec<CallReference>,
}

/// A symbol in reachability analysis with depth and path.
#[derive(Debug, Clone)]
pub struct ReachableSymbol {
    /// The symbol's basic information.
    pub symbol: SymbolInfo,
    /// Depth from root (0 = root, 1 = direct relationship, etc.).
    pub depth: usize,
    /// Call path from root to this symbol.
    pub path: Vec<String>,
}

/// A dead (unreachable) symbol.
#[derive(Debug, Clone)]
pub struct DeadSymbol {
    /// The symbol's basic information.
    pub symbol: SymbolInfo,
    /// Reason why this symbol is considered dead.
    pub reason: String,
}

/// Information about a detected cycle.
#[derive(Debug, Clone)]
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

/// Condensation graph result (SCCs collapsed to DAG).
#[derive(Debug, Clone)]
pub struct CondensationGraph {
    /// Total number of SCCs.
    pub scc_count: usize,
    /// Number of SCCs that are cycles.
    pub cycle_scc_count: usize,
    /// Number of singleton SCCs.
    pub singleton_count: usize,
    /// SCCs in the graph.
    pub sccs: Vec<CondensedScc>,
    /// Edges between SCCs.
    pub edges: Vec<SccEdge>,
    /// Topological levels.
    pub levels: Vec<LevelInfo>,
}

/// A condensed SCC.
#[derive(Debug, Clone)]
/// A condensed strongly connected component.
pub struct CondensedScc {
    /// Unique identifier for the SCC.
    pub id: String,
    /// Number of symbols in the SCC.
    pub size: usize,
    /// Whether the SCC contains a cycle.
    pub is_cycle: bool,
    /// Member symbols, if expanded.
    pub members: Option<Vec<SymbolInfo>>,
    /// Representative symbol for the SCC.
    pub representative: SymbolInfo,
}

/// Edge between SCCs.
#[derive(Debug, Clone)]
/// Edge between two strongly connected components.
pub struct SccEdge {
    /// Source SCC identifier.
    pub from: String,
    /// Target SCC identifier.
    pub to: String,
    /// Number of edges between the SCCs.
    pub weight: usize,
}

/// Topological level.
#[derive(Debug, Clone)]
/// Topological level in the condensation graph.
pub struct LevelInfo {
    /// Level number in topological order.
    pub level: usize,
    /// SCC identifiers at this level.
    pub scc_ids: Vec<String>,
    /// Number of SCCs at this level.
    pub count: usize,
}

/// Configuration for DOT graph generation.
#[derive(Debug, Clone)]
pub struct ImpactDotConfig {
    /// Show symbol kinds in node labels (e.g., "main (fn)").
    pub show_symbol_kinds: bool,
    /// Maximum depth for traversal (None = unlimited).
    pub max_depth: Option<usize>,
    /// Symbol to highlight in graph (fillcolor=lightblue).
    pub highlight_symbol: Option<String>,
}

impl Default for ImpactDotConfig {
    fn default() -> Self {
        Self {
            show_symbol_kinds: true,
            max_depth: Some(10),
            highlight_symbol: None,
        }
    }
}

/// A symbol in a program slice.
#[derive(Debug, Clone)]
pub struct SlicedSymbol {
    /// The symbol.
    pub symbol: SymbolInfo,
    /// Distance from target.
    pub distance: usize,
    /// Whether this is the target symbol.
    pub is_target: bool,
    /// Relationship type.
    pub relationship: String,
}

/// File metadata with optional symbol count.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// Path to the file.
    pub path: String,
    /// Content hash of the file.
    pub hash: String,
    /// Unix timestamp when file was last indexed.
    pub last_indexed_at: i64,
    /// Unix timestamp when file was last modified.
    pub last_modified: i64,
    /// Symbol count if requested (None if --symbols flag not provided).
    pub symbol_count: Option<usize>,
}

impl From<SymbolQueryResult> for SymbolInfo {
    fn from(result: SymbolQueryResult) -> Self {
        Self {
            entity_id: result.entity_id,
            name: result.name,
            file_path: result.file_path,
            kind: result.kind,
            byte_start: result.byte_start,
            byte_end: result.byte_end,
            start_line: None,
            end_line: None,
        }
    }
}

/// Code chunk with content and metadata.
#[derive(Debug, Clone)]
pub struct CodeChunk {
    /// Source code content.
    pub content: String,
    /// File path containing this chunk.
    pub file_path: String,
    /// Byte offset where the chunk starts.
    pub byte_start: usize,
    /// Byte offset where the chunk ends.
    pub byte_end: usize,
    /// Symbol name if this chunk belongs to a specific symbol.
    pub symbol_name: Option<String>,
    /// Symbol kind if available.
    pub symbol_kind: Option<String>,
}

impl CodeChunk {
    /// Return the length of the chunk content in bytes.
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Check if the chunk content is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Return the chunk content as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.content.as_bytes()
    }

    /// Iterate over lines in the chunk content.
    pub fn lines(&self) -> std::str::Lines<'_> {
        self.content.lines()
    }
}

impl From<magellan::CodeChunk> for CodeChunk {
    fn from(chunk: magellan::CodeChunk) -> Self {
        Self {
            content: chunk.content,
            file_path: chunk.file_path,
            byte_start: chunk.byte_start,
            byte_end: chunk.byte_end,
            symbol_name: chunk.symbol_name,
            symbol_kind: chunk.symbol_kind,
        }
    }
}

/// Database statistics for Magellan graph.
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    /// Number of indexed files.
    pub files: usize,
    /// Number of indexed symbols.
    pub symbols: usize,
    /// Number of indexed references.
    pub references: usize,
    /// Number of indexed function calls.
    pub calls: usize,
    /// Number of stored code chunks.
    pub code_chunks: usize,
}
