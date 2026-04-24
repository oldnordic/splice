//! Minimal shared types for code intelligence backends.
//!
//! This module defines shared types used by both SQLite and Geometric backends.
//! The actual backend implementations are in:
//! - `sqlite_impl.rs` - SQLite/V3 implementation
//! - `geo_impl.rs` - Geometric implementation
//! - `router.rs` - Public CodeGraph enum that dispatches to backends

use sqlitegraph::NodeId;

/// Symbol information returned by backend queries.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// Node ID in the graph
    pub node_id: NodeId,
    /// Symbol name
    pub name: String,
    /// File path containing the symbol
    pub file_path: String,
    /// Symbol kind (function, struct, etc.)
    pub kind: String,
    /// Byte start offset
    pub byte_start: usize,
    /// Byte end offset
    pub byte_end: usize,
    /// Start line (1-indexed)
    pub line_start: usize,
    /// End line (1-indexed)
    pub line_end: usize,
    /// Start column (0-indexed)
    pub col_start: usize,
    /// End column (0-indexed)
    pub col_end: usize,
    /// Programming language
    pub language: Option<String>,
}
