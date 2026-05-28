use serde::{Deserialize, Serialize};

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
// Program Slicing Types (Phase 30-05)
// ============================================================================

/// Program slicing result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceResult {
    /// The target symbol.
    pub target: SymbolInfo,
    /// Slice direction performed.
    pub direction: String, // "forward" or "backward"
    /// Maximum depth (None = unlimited).
    pub max_depth: Option<usize>,
    /// Symbols in the slice.
    pub symbols: Vec<SlicedSymbol>,
    /// Files affected by this slice.
    pub affected_files: Vec<AffectedFile>,
    /// Statistics about the slice.
    pub stats: SliceStats,
}

/// A symbol in the program slice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlicedSymbol {
    /// The symbol.
    pub symbol: SymbolInfo,
    /// Distance from target (0 = target itself).
    pub distance: usize,
    /// Whether this is the target symbol.
    pub is_target: bool,
    /// Relationship type (calls, called_by, etc.).
    pub relationship: String,
}

/// Statistics about the slice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceStats {
    /// Total symbols in the slice.
    pub total_symbols: usize,
    /// Maximum distance from target.
    pub max_distance: usize,
    /// Number of affected files.
    pub affected_file_count: usize,
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
