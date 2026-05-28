use serde::{Deserialize, Serialize};

// Magellan-compatible response types for delegated query commands.
//
// These types use Magellan field naming conventions (start_line vs line_start)
// for compatibility with Magellan's JSON output format.

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
        let symbol_id = crate::symbol_id::generate_v2(&info.name, &info.file_path, info.byte_start);
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
