//! Magellan integration layer with backend split.
//!
//! This module provides integration with Magellan for:
//! - Multi-language code indexing
//! - Label-based symbol queries
//! - Code chunk retrieval (no file re-reading)
//!
//! # Backend Split
//! - SQLite backend: `magellan::CodeGraph` for `.db` files
//! - Geometric backend: `magellan::graph::geometric_backend::GeometricBackend` for `.geo` files
//!
//! # Structure
//! - `types` — shared type definitions (SymbolInfo, CycleInfo, etc.)
//! - `symbol_lookup` — find_symbol_by_name, find_symbol_by_id, query_symbols_by_file
//! - `references` — get_call_relationships, get_all_references, index_references
//! - `reachability` — reachable_symbols, reverse_reachable_symbols
//! - `cycles` — detect_cycles, find_cycles_containing, condense_graph
//! - `slicing` — dead_symbols, forward_slice, backward_slice
//! - `dot` — generate_impact_dot, generate_refs_dot

pub(crate) mod cycles;
pub(crate) mod dot;
pub(crate) mod reachability;
pub(crate) mod references;
pub(crate) mod slicing;
pub(crate) mod symbol_lookup;
pub mod types;

pub use types::*;

use crate::error::{Result, SpliceError};
use magellan::{CodeGraph as MagellanGraph, SymbolKind};
use std::path::{Path, PathBuf};

fn normalize_lookup_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .expect("invariant: current directory always available")
            .join(path)
    }
}

fn parse_symbol_kind(kind: &str) -> SymbolKind {
    match kind {
        "fn" | "method" => SymbolKind::Function,
        "struct" | "class" => SymbolKind::Class,
        "enum" => SymbolKind::Enum,
        "trait" | "interface" => SymbolKind::Interface,
        "impl" => SymbolKind::Unknown,
        "module" | "namespace" => SymbolKind::Module,
        "variable" | "field" => SymbolKind::Unknown,
        "constant" | "const" => SymbolKind::Unknown,
        "type" | "typedef" => SymbolKind::TypeAlias,
        _ => SymbolKind::Unknown,
    }
}

/// Normalize a kind string to a canonical form for filtering.
///
/// This collapses equivalent terms (e.g., "function" and "fn") so that CLI
/// `--kind function` matches magellan's normalized "fn" kind and vice versa.
pub fn normalize_kind(kind: &str) -> String {
    let lower = kind.to_ascii_lowercase();
    match lower.as_str() {
        "function" | "fn" | "method" => "fn",
        "struct" | "class" => "struct",
        "enum" => "enum",
        "trait" | "interface" => "trait",
        "impl" => "impl",
        "module" | "namespace" => "module",
        "variable" | "field" | "const" | "constant" => "variable",
        "type" | "typedef" | "type_alias" => "type_alias",
        _ => &lower,
    }
    .to_string()
}

#[allow(missing_docs)]
pub struct MagellanIntegration {
    pub(crate) inner: MagellanGraph,
    #[cfg(feature = "geometric")]
    pub(crate) geo_inner: Option<magellan::graph::geometric_backend::GeometricBackend>,
    pub(crate) db_path: PathBuf,
    pub(crate) backend: IntegrationBackend,
}

#[allow(missing_docs)]
impl MagellanIntegration {
    pub fn backend_type(&self) -> IntegrationBackend {
        self.backend
    }

    pub fn is_geometric(&self) -> bool {
        #[cfg(feature = "geometric")]
        return matches!(self.backend, IntegrationBackend::Geometric);
        #[cfg(not(feature = "geometric"))]
        return false;
    }

    pub fn open(db_path: &Path) -> Result<Self> {
        #[cfg(feature = "geometric")]
        if Self::is_geometric_db(db_path) {
            return Self::open_geometric(db_path);
        }
        Self::open_sqlite(db_path)
    }

    fn open_sqlite(db_path: &Path) -> Result<Self> {
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", db_path)))?;

        let inner = MagellanGraph::open(db_path_str).map_err(|e| SpliceError::Magellan {
            context: format!("Failed to open Magellan SQLite graph at {}", db_path_str),
            source: e,
        })?;

        Ok(Self {
            inner,
            #[cfg(feature = "geometric")]
            geo_inner: None,
            db_path: db_path.to_path_buf(),
            backend: IntegrationBackend::Sqlite,
        })
    }

    #[cfg(feature = "geometric")]
    fn open_geometric(db_path: &Path) -> Result<Self> {
        use magellan::graph::geometric_backend::GeometricBackend;

        let geo = GeometricBackend::open(db_path).map_err(|e| SpliceError::Magellan {
            context: format!("Failed to open Geometric backend at {:?}", db_path),
            source: e,
        })?;

        let inner = MagellanGraph::open(":memory:").map_err(|e| SpliceError::Magellan {
            context: "Failed to create in-memory SQLite for geometric backend".to_string(),
            source: e,
        })?;

        Ok(Self {
            inner,
            geo_inner: Some(geo),
            db_path: db_path.to_path_buf(),
            backend: IntegrationBackend::Geometric,
        })
    }

    #[cfg(feature = "geometric")]
    pub fn geo_inner(&self) -> Option<&magellan::graph::geometric_backend::GeometricBackend> {
        self.geo_inner.as_ref()
    }

    #[cfg(feature = "geometric")]
    pub fn geo_inner_mut(
        &mut self,
    ) -> Option<&mut magellan::graph::geometric_backend::GeometricBackend> {
        self.geo_inner.as_mut()
    }

    pub fn index_file(&mut self, file_path: &Path) -> Result<usize> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        let source = std::fs::read(file_path).map_err(|e| {
            SpliceError::Other(format!("Failed to read file {:?}: {}", file_path, e))
        })?;

        self.inner
            .index_file(file_path_str, &source)
            .map_err(|e| SpliceError::Other(format!("Failed to index file {:?}: {}", file_path, e)))
    }

    #[cfg(feature = "sqlite")]
    pub fn query_by_labels(&self, labels: &[&str]) -> Result<Vec<SymbolInfo>> {
        let labels_ref: Vec<&str> = labels.to_vec();
        self.inner
            .get_symbols_by_labels(&labels_ref)
            .map_err(|e| {
                SpliceError::Other(format!("Failed to query by labels {:?}: {}", labels, e))
            })
            .map(|results| results.into_iter().map(SymbolInfo::from).collect())
    }

    #[cfg(feature = "sqlite")]
    pub fn get_all_labels(&self) -> Result<Vec<String>> {
        self.inner
            .get_all_labels()
            .map_err(|e| SpliceError::Other(format!("Failed to get labels: {}", e)))
    }

    #[cfg(feature = "sqlite")]
    pub fn count_by_label(&self, label: &str) -> Result<usize> {
        self.inner
            .count_entities_by_label(label)
            .map_err(|e| SpliceError::Other(format!("Failed to count label {}: {}", label, e)))
    }

    pub fn get_code_chunk(
        &self,
        file_path: &Path,
        start: usize,
        end: usize,
    ) -> Result<Option<CodeChunk>> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        self.inner
            .get_code_chunk_by_span(file_path_str, start, end)
            .map_err(|e| SpliceError::Other(format!("Failed to get code chunk: {}", e)))
            .map(|opt_chunk| opt_chunk.map(CodeChunk::from))
    }

    pub fn get_code_chunks_for_symbol(
        &self,
        file_path: &Path,
        symbol_name: &str,
    ) -> Result<Vec<CodeChunk>> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        self.inner
            .get_code_chunks_for_symbol(file_path_str, symbol_name)
            .map_err(|e| {
                SpliceError::Other(format!(
                    "Failed to get code chunks for symbol {}: {}",
                    symbol_name, e
                ))
            })
            .map(|chunks| chunks.into_iter().map(CodeChunk::from).collect())
    }

    pub fn inner(&self) -> &MagellanGraph {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut MagellanGraph {
        &mut self.inner
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn get_statistics(&self) -> Result<DatabaseStats> {
        match self.backend {
            IntegrationBackend::Sqlite => self.get_statistics_sqlite(),
            #[cfg(feature = "geometric")]
            IntegrationBackend::Geometric => self.get_statistics_geometric(),
        }
    }

    fn get_statistics_sqlite(&self) -> Result<DatabaseStats> {
        let files = self
            .inner
            .count_files()
            .map_err(|e| SpliceError::Other(format!("Failed to count files: {}", e)))?;
        let symbols = self
            .inner
            .count_symbols()
            .map_err(|e| SpliceError::Other(format!("Failed to count symbols: {}", e)))?;
        let references = self
            .inner
            .count_references()
            .map_err(|e| SpliceError::Other(format!("Failed to count references: {}", e)))?;
        let code_chunks = self
            .inner
            .count_chunks()
            .map_err(|e| SpliceError::Other(format!("Failed to count code chunks: {}", e)))?;

        let calls = self.count_call_nodes()?;

        Ok(DatabaseStats {
            files,
            symbols,
            references,
            calls,
            code_chunks,
        })
    }

    #[cfg(feature = "geometric")]
    fn get_statistics_geometric(&self) -> Result<DatabaseStats> {
        if let Some(ref geo) = self.geo_inner {
            let stats = geo.get_geometric_stats();
            Ok(DatabaseStats {
                files: stats.file_count,
                symbols: stats.symbol_count,
                references: 0,
                calls: 0,
                code_chunks: stats.cfg_block_count,
            })
        } else {
            Err(SpliceError::Other(
                "Geometric backend not initialized".to_string(),
            ))
        }
    }

    fn count_call_nodes(&self) -> Result<usize> {
        use rusqlite::Connection;

        let conn = Connection::open(&self.db_path).map_err(|e| {
            SpliceError::Other(format!("Failed to open database for Call counting: {}", e))
        })?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM graph_entities WHERE kind = 'Call'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| SpliceError::Other(format!("Failed to count Call nodes: {}", e)))?;

        Ok(count as usize)
    }

    #[cfg(feature = "geometric")]
    fn is_geometric_db(path: &Path) -> bool {
        path.extension().is_some_and(|ext| ext == "geo")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_open_and_query() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let integration = MagellanIntegration::open(&db_path).unwrap();

        let results = integration.query_by_labels(&["rust"]).unwrap();
        assert!(results.is_empty());

        let labels = integration.get_all_labels().unwrap();
        assert!(labels.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_count_by_label() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let integration = MagellanIntegration::open(&db_path).unwrap();

        let count = integration.count_by_label("rust").unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_sort_references_for_replacement() {
        use std::path::PathBuf;

        let mut references = vec![
            magellan::references::ReferenceFact {
                file_path: PathBuf::from("/src/b.rs"),
                referenced_symbol: "foo".to_string(),
                byte_start: 50,
                byte_end: 53,
                start_line: 2,
                start_col: 0,
                end_line: 2,
                end_col: 3,
            },
            magellan::references::ReferenceFact {
                file_path: PathBuf::from("/src/a.rs"),
                referenced_symbol: "foo".to_string(),
                byte_start: 100,
                byte_end: 103,
                start_line: 5,
                start_col: 0,
                end_line: 5,
                end_col: 3,
            },
            magellan::references::ReferenceFact {
                file_path: PathBuf::from("/src/a.rs"),
                referenced_symbol: "foo".to_string(),
                byte_start: 20,
                byte_end: 23,
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 3,
            },
            magellan::references::ReferenceFact {
                file_path: PathBuf::from("/src/b.rs"),
                referenced_symbol: "foo".to_string(),
                byte_start: 10,
                byte_end: 13,
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 3,
            },
        ];

        MagellanIntegration::sort_references_for_replacement(&mut references);

        assert_eq!(references[0].file_path, PathBuf::from("/src/a.rs"));
        assert_eq!(references[0].byte_start, 100);

        assert_eq!(references[1].file_path, PathBuf::from("/src/a.rs"));
        assert_eq!(references[1].byte_start, 20);

        assert_eq!(references[2].file_path, PathBuf::from("/src/b.rs"));
        assert_eq!(references[2].byte_start, 50);

        assert_eq!(references[3].file_path, PathBuf::from("/src/b.rs"));
        assert_eq!(references[3].byte_start, 10);
    }

    #[test]
    fn test_validate_utf8_span_valid() {
        let content = "Hello, world!";
        let file_path = Path::new("/test.rs");

        assert!(
            MagellanIntegration::validate_utf8_span(content.as_bytes(), 0, 5, file_path).is_ok()
        );

        assert!(MagellanIntegration::validate_utf8_span(
            content.as_bytes(),
            0,
            content.len(),
            file_path
        )
        .is_ok());
    }

    #[test]
    fn test_validate_utf8_span_out_of_bounds() {
        let content = "Hello";
        let file_path = Path::new("/test.rs");

        assert!(
            MagellanIntegration::validate_utf8_span(content.as_bytes(), 10, 15, file_path).is_err()
        );

        assert!(
            MagellanIntegration::validate_utf8_span(content.as_bytes(), 0, 10, file_path).is_err()
        );
    }

    #[test]
    fn test_validate_utf8_span_multibyte_boundary() {
        let content = "Hello 世界";
        let file_path = Path::new("/test.rs");

        assert!(
            MagellanIntegration::validate_utf8_span(content.as_bytes(), 0, 8, file_path).is_err()
        );

        assert!(
            MagellanIntegration::validate_utf8_span(content.as_bytes(), 6, 9, file_path).is_ok()
        );
    }

    #[test]
    fn test_validate_utf8_span_invalid_utf8() {
        let content: &[u8] = &[0xFF, 0xFE, 0xFD];
        let file_path = Path::new("/test.rs");

        assert!(MagellanIntegration::validate_utf8_span(content, 0, 1, file_path).is_err());
    }

    #[cfg(feature = "sqlite")]
    fn setup_cross_file_cycle_db(db_path: &std::path::Path) {
        use rusqlite::{params, Connection};
        let _integration = MagellanIntegration::open(db_path).unwrap();

        let conn = Connection::open(db_path).unwrap();

        conn.execute(
            "INSERT INTO graph_entities(kind, name, file_path, data) VALUES ('Symbol', 'func_a', '/src/a.rs', '{}')",
            [],
        ).unwrap();
        let sym_a = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO graph_entities(kind, name, file_path, data) VALUES ('Symbol', 'func_b', '/src/b.rs', '{}')",
            [],
        ).unwrap();
        let sym_b = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO graph_entities(kind, name, file_path, data) VALUES ('Call', 'func_a calls func_b', '/src/a.rs', '{\"caller\":\"func_a\",\"callee\":\"func_b\"}')",
            [],
        ).unwrap();
        let call_a_b = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO graph_entities(kind, name, file_path, data) VALUES ('Call', 'func_b calls func_a', '/src/b.rs', '{\"caller\":\"func_b\",\"callee\":\"func_a\"}')",
            [],
        ).unwrap();
        let call_b_a = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO graph_edges(from_id, to_id, edge_type, data) VALUES (?1, ?2, 'CALLS', '{}')",
            params![call_a_b, sym_b],
        ).unwrap();
        conn.execute(
            "INSERT INTO graph_edges(from_id, to_id, edge_type, data) VALUES (?1, ?2, 'CALLS', '{}')",
            params![call_b_a, sym_a],
        ).unwrap();
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_all_call_edges_sqlite_cross_file() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        setup_cross_file_cycle_db(&db_path);

        let integration = MagellanIntegration::open(&db_path).unwrap();
        let edges = integration.all_call_edges_sqlite().unwrap();

        assert_eq!(edges.len(), 2, "expected 2 call edges, got {}", edges.len());

        let a_to_b = edges
            .iter()
            .find(|(cf, cn, _, _)| cf == "/src/a.rs" && cn == "func_a");
        assert!(a_to_b.is_some(), "missing func_a → func_b edge");
        let a_to_b = a_to_b.unwrap();
        assert_eq!(a_to_b.2, "/src/b.rs", "callee file should be /src/b.rs");
        assert_eq!(a_to_b.3, "func_b", "callee name should be func_b");

        let b_to_a = edges
            .iter()
            .find(|(cf, cn, _, _)| cf == "/src/b.rs" && cn == "func_b");
        assert!(b_to_a.is_some(), "missing func_b → func_a edge");
        let b_to_a = b_to_a.unwrap();
        assert_eq!(b_to_a.2, "/src/a.rs", "callee file should be /src/a.rs");
        assert_eq!(b_to_a.3, "func_a", "callee name should be func_a");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_detect_cycles_cross_file() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        setup_cross_file_cycle_db(&db_path);

        let mut integration = MagellanIntegration::open(&db_path).unwrap();
        let cycles = integration.detect_cycles(10).unwrap();

        assert_eq!(
            cycles.len(),
            1,
            "expected 1 cross-file cycle, got {}",
            cycles.len()
        );
        assert_eq!(cycles[0].size, 2, "cycle should have 2 members");
        assert!(!cycles[0].is_self_loop);

        let names: Vec<&str> = cycles[0].members.iter().map(|m| m.name.as_str()).collect();
        assert!(
            names.contains(&"func_a"),
            "func_a missing from cycle members"
        );
        assert!(
            names.contains(&"func_b"),
            "func_b missing from cycle members"
        );
    }
}
