//! Backend router - dispatches to SQLite or Geometric implementation.
//!
//! This module provides the public `CodeGraph` type that users interact with.
//! It's an enum that routes to either `CodeGraphSqlite` or `CodeGraphGeo`
//! based on the database file type.

use crate::error::{Result, SpliceError};
use crate::symbol::Language;
use sqlitegraph::NodeId;
use std::path::Path;

/// Backend type identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// SQLite-based backend
    SQLite,
    /// Geometric spatial backend
    Geometric,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::SQLite => write!(f, "sqlite"),
            BackendType::Geometric => write!(f, "geometric"),
        }
    }
}

/// Unified code graph handle that dispatches to the appropriate backend.
///
/// This is the primary type used by Splice workflows. It automatically
/// detects the backend type from the file extension and routes to:
/// - `CodeGraphSqlite` for `.db` files
/// - `CodeGraphGeo` for `.geo` files
///
/// # Example
/// ```no_run
/// use splice::graph::CodeGraph;
/// use std::path::Path;
///
/// // Automatically detects backend type
/// let graph = CodeGraph::open(Path::new("code.geo")).unwrap();
/// ```
pub enum CodeGraph {
    /// SQLite backend
    Sqlite(super::sqlite_impl::CodeGraphSqlite),
    /// Geometric backend
    #[cfg(feature = "geometric")]
    Geo(super::geo_impl::CodeGraphGeo),
}

impl CodeGraph {
    /// Open or create a code graph at the given path.
    ///
    /// Automatically detects backend type from file extension:
    /// - `.db` → SQLite backend
    /// - `.geo` → Geometric backend (requires geometric feature)
    pub fn open(path: &Path) -> Result<Self> {
        // Handle empty files
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() == 0 {
                std::fs::remove_file(path).map_err(|e| {
                    SpliceError::Other(format!(
                        "Failed to remove empty graph database {:?}: {}",
                        path, e
                    ))
                })?;
            }
        }

        // Route based on file extension
        #[cfg(feature = "geometric")]
        if Self::is_geometric_db(path) {
            return Ok(CodeGraph::Geo(super::geo_impl::CodeGraphGeo::open(path)?));
        }

        // Default to SQLite backend
        Ok(CodeGraph::Sqlite(
            super::sqlite_impl::CodeGraphSqlite::open(path)?,
        ))
    }

    /// Check if a path is a geometric database.
    pub fn is_geometric_db(path: &Path) -> bool {
        path.extension().map_or(false, |ext| ext == "geo")
    }

    /// Detect backend type without opening.
    pub fn detect_backend(path: &Path) -> Result<BackendType> {
        if Self::is_geometric_db(path) {
            #[cfg(feature = "geometric")]
            return Ok(BackendType::Geometric);
            #[cfg(not(feature = "geometric"))]
            return Err(SpliceError::Other(
                "Geometric backend not compiled in. Build with --features geometric".to_string(),
            ));
        }

        // Check file header for SQLite
        if path.exists() {
            use std::io::Read;
            let mut file = std::fs::File::open(path)?;
            let mut header = [0u8; 16];
            let bytes_read = file.read(&mut header)?;
            if bytes_read >= 16 && header.starts_with(b"SQLite format 3\0") {
                return Ok(BackendType::SQLite);
            }
            // Check for geometric magic
            if header.starts_with(b"GEODB\0\0\0") {
                #[cfg(feature = "geometric")]
                return Ok(BackendType::Geometric);
                #[cfg(not(feature = "geometric"))]
                return Err(SpliceError::Other(
                    "Geometric backend not compiled in".to_string(),
                ));
            }
        }

        // Default to SQLite for new files
        Ok(BackendType::SQLite)
    }

    /// Get the backend type.
    pub fn backend_type(&self) -> BackendType {
        match self {
            CodeGraph::Sqlite(sqlite) => sqlite.backend_type(),
            #[cfg(feature = "geometric")]
            CodeGraph::Geo(geo) => geo.backend_type(),
        }
    }

    /// Check if this is a geometric backend.
    pub fn is_geometric(&self) -> bool {
        matches!(self.backend_type(), BackendType::Geometric)
    }

    /// Find a symbol by name within a specific file.
    pub fn find_symbol_in_file(&self, file_path: &str, name: &str) -> Option<NodeId> {
        match self {
            CodeGraph::Sqlite(sqlite) => sqlite.find_symbol_in_file(file_path, name),
            #[cfg(feature = "geometric")]
            CodeGraph::Geo(geo) => geo.find_symbol_in_file(file_path, name),
        }
    }

    /// Find all symbols with a given name across all files.
    pub fn find_symbols_by_name(&self, name: &str) -> Vec<(NodeId, Option<String>)> {
        match self {
            CodeGraph::Sqlite(sqlite) => sqlite.find_symbols_by_name(name),
            #[cfg(feature = "geometric")]
            CodeGraph::Geo(geo) => geo.find_symbols_by_name(name),
        }
    }

    /// Get all unique symbol names in the graph.
    pub fn all_symbol_names(&self) -> Vec<String> {
        match self {
            CodeGraph::Sqlite(sqlite) => sqlite.all_symbol_names(),
            #[cfg(feature = "geometric")]
            CodeGraph::Geo(geo) => geo.all_symbol_names(),
        }
    }

    /// Get the byte span (start, end) for a symbol.
    pub fn get_span(&self, node_id: NodeId) -> Result<(usize, usize)> {
        match self {
            CodeGraph::Sqlite(sqlite) => sqlite.get_span(node_id),
            #[cfg(feature = "geometric")]
            CodeGraph::Geo(geo) => geo.get_span(node_id),
        }
    }

    /// Store a symbol with full metadata.
    #[allow(clippy::too_many_arguments)]
    pub fn store_symbol(
        &mut self,
        name: &str,
        kind: &str,
        language: Language,
        byte_start: usize,
        byte_end: usize,
        line_start: usize,
        line_end: usize,
        col_start: usize,
        col_end: usize,
    ) -> Result<NodeId> {
        match self {
            CodeGraph::Sqlite(sqlite) => sqlite.store_symbol(
                name, kind, language, byte_start, byte_end, line_start, line_end, col_start,
                col_end,
            ),
            #[cfg(feature = "geometric")]
            CodeGraph::Geo(geo) => geo.store_symbol(
                name, kind, language, byte_start, byte_end, line_start, line_end, col_start,
                col_end,
            ),
        }
    }

    /// Store a symbol with file context and language.
    pub fn store_symbol_with_file_and_language(
        &mut self,
        file_path: &Path,
        name: &str,
        kind: &str,
        language: Language,
        byte_start: usize,
        byte_end: usize,
        line_start: usize,
        line_end: usize,
        col_start: usize,
        col_end: usize,
    ) -> Result<NodeId> {
        match self {
            CodeGraph::Sqlite(sqlite) => sqlite.store_symbol_with_file_and_language(
                file_path, name, kind, language, byte_start, byte_end, line_start, line_end,
                col_start, col_end,
            ),
            #[cfg(feature = "geometric")]
            CodeGraph::Geo(geo) => geo.store_symbol_with_file_and_language(
                file_path, name, kind, language, byte_start, byte_end, line_start, line_end,
                col_start, col_end,
            ),
        }
    }

    /// Access the underlying sqlitegraph backend.
    ///
    /// Returns error for geometric backend.
    pub fn inner(&self) -> Result<&dyn sqlitegraph::GraphBackend> {
        match self {
            CodeGraph::Sqlite(sqlite) => Ok(sqlite.inner()),
            #[cfg(feature = "geometric")]
            CodeGraph::Geo(_) => Err(SpliceError::Other(
                "SQLite backend not available. Use geometric() for .geo files".to_string(),
            )),
        }
    }

    /// Access the underlying sqlitegraph backend mutably.
    ///
    /// Returns error for geometric backend.
    pub fn inner_mut(&mut self) -> Result<&mut dyn sqlitegraph::GraphBackend> {
        match self {
            CodeGraph::Sqlite(sqlite) => Ok(sqlite.inner_mut()),
            #[cfg(feature = "geometric")]
            CodeGraph::Geo(_) => Err(SpliceError::Other(
                "SQLite backend not available. Use geometric_mut() for .geo files".to_string(),
            )),
        }
    }

    /// Access the SQLite backend (if using SQLite).
    pub fn as_sqlite(&self) -> Option<&super::sqlite_impl::CodeGraphSqlite> {
        match self {
            CodeGraph::Sqlite(sqlite) => Some(sqlite),
            #[cfg(feature = "geometric")]
            _ => None,
        }
    }

    /// Access the SQLite backend mutably (if using SQLite).
    pub fn as_sqlite_mut(&mut self) -> Option<&mut super::sqlite_impl::CodeGraphSqlite> {
        match self {
            CodeGraph::Sqlite(sqlite) => Some(sqlite),
            #[cfg(feature = "geometric")]
            _ => None,
        }
    }

    /// Access the Geometric backend (if using Geometric).
    #[cfg(feature = "geometric")]
    pub fn as_geo(&self) -> Option<&super::geo_impl::CodeGraphGeo> {
        match self {
            CodeGraph::Geo(geo) => Some(geo),
            _ => None,
        }
    }

    /// Access the Geometric backend mutably (if using Geometric).
    #[cfg(feature = "geometric")]
    pub fn as_geo_mut(&mut self) -> Option<&mut super::geo_impl::CodeGraphGeo> {
        match self {
            CodeGraph::Geo(geo) => Some(geo),
            _ => None,
        }
    }

    /// Access the GeometricBackend (if using geometric).
    #[cfg(feature = "geometric")]
    pub fn geometric(&self) -> Result<&magellan::graph::geometric_backend::GeometricBackend> {
        match self {
            CodeGraph::Geo(geo) => Ok(geo.inner()),
            _ => Err(SpliceError::Other(
                "Geometric backend not available. Use inner() for .db files".to_string(),
            )),
        }
    }

    /// Access the GeometricBackend mutably (if using geometric).
    #[cfg(feature = "geometric")]
    pub fn geometric_mut(
        &mut self,
    ) -> Result<&mut magellan::graph::geometric_backend::GeometricBackend> {
        match self {
            CodeGraph::Geo(geo) => Ok(geo.inner_mut()),
            _ => Err(SpliceError::Other(
                "Geometric backend not available. Use inner_mut() for .db files".to_string(),
            )),
        }
    }

    /// Restore from snapshot is disabled.
    pub fn restore_from_snapshot(
        _db_path: &Path,
        _snapshot_path: &Path,
    ) -> Result<crate::proof::storage::RestoreResult> {
        Err(SpliceError::Other(
            "Database snapshot restore is disabled.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_detect_backend_sqlite() {
        let temp_file = NamedTempFile::new().unwrap();
        let backend = CodeGraph::detect_backend(temp_file.path()).unwrap();
        assert!(matches!(backend, BackendType::SQLite));
    }

    #[test]
    #[cfg(feature = "geometric")]
    fn test_detect_backend_geometric() {
        let path = Path::new("test.geo");
        let backend = CodeGraph::detect_backend(path).unwrap();
        assert!(matches!(backend, BackendType::Geometric));
    }

    #[test]
    fn test_is_geometric_db() {
        assert!(CodeGraph::is_geometric_db(Path::new("code.geo")));
        assert!(!CodeGraph::is_geometric_db(Path::new("code.db")));
        assert!(!CodeGraph::is_geometric_db(Path::new(
            ".magellan/splice.db"
        )));
    }

    #[test]
    fn test_backend_type_roundtrip() {
        let temp_file = NamedTempFile::new().unwrap();
        let graph = CodeGraph::open(temp_file.path()).unwrap();
        assert!(matches!(graph.backend_type(), BackendType::SQLite));
    }
}
