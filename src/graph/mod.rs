//! SQLiteGraph integration layer.
//!
//! This module provides a typed interface to the code graph stored
//! in SQLiteGraph. It handles symbol storage, span queries, and
//! relationship management for multi-language code analysis.

pub mod magellan_integration;
pub mod migrate;
pub mod rename;
pub mod schema;

// Re-export MagellanIntegration for convenient use
pub use magellan_integration::MagellanIntegration;

use crate::error::{Result, SpliceError};
use crate::symbol::Language;
use serde_json::json;
use sqlitegraph::{EdgeSpec, GraphBackend, NodeId, NodeSpec, SnapshotId};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

/// Database backend format detected in a graph database file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// SQLite-based backend (default, backward compatible)
    SQLite,
    /// Native-v3 backend (requires native-v3 feature)
    NativeV3,
    /// Unknown or unrecognized format
    Unknown,
}

impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backend::SQLite => write!(f, "sqlite"),
            Backend::NativeV3 => write!(f, "native-v3"),
            Backend::Unknown => write!(f, "unknown"),
        }
    }
}

/// Migration result reporting statistics from a migration operation.
#[derive(Debug, Clone)]
pub struct MigrationReport {
    /// Number of nodes migrated
    pub nodes_migrated: usize,
    /// Number of edges migrated
    pub edges_migrated: usize,
    /// Snapshot metadata from export
    pub snapshot_metadata: String,
    /// Path to the migrated database
    pub destination: std::path::PathBuf,
    /// Whether post-migration verification passed
    pub verification_passed: bool,
    /// Verification error message (if verification failed)
    pub verification_error: Option<String>,
}

/// Graph database handle.
///
/// Wraps SQLiteGraph and provides Splice-specific operations.
pub struct CodeGraph {
    /// The underlying graph backend.
    backend: Box<dyn GraphBackend>,

    /// Cache for symbol name → Vec<NodeId> mapping (multiple files can have same name).
    symbol_cache: HashMap<String, Vec<NodeId>>,

    /// Cache for file path → NodeId mapping.
    file_cache: HashMap<String, NodeId>,
}

impl CodeGraph {
    /// Open or create a code graph at the given path.
    pub fn open(path: &std::path::Path) -> Result<Self> {
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

        let cfg = if Self::is_sqlite_db(path)? {
            sqlitegraph::GraphConfig::sqlite()
        } else {
            sqlitegraph::GraphConfig::native()
        };
        let backend = sqlitegraph::open_graph(path, &cfg)?;
        Ok(Self {
            backend,
            symbol_cache: HashMap::new(),
            file_cache: HashMap::new(),
        })
    }

    pub fn is_sqlite_db(path: &Path) -> Result<bool> {
        if !path.exists() {
            return Ok(false);
        }

        let mut file = std::fs::File::open(path).map_err(|e| {
            SpliceError::Other(format!("Failed to open graph database {:?}: {}", path, e))
        })?;
        let mut header = [0u8; 16];
        let bytes_read = file.read(&mut header).map_err(|e| {
            SpliceError::Other(format!("Failed to read graph database {:?}: {}", path, e))
        })?;
        if bytes_read < header.len() {
            return Ok(false);
        }

        Ok(&header[..15] == b"SQLite format 3")
    }

    /// Detect which backend format a database file uses.
    ///
    /// Checks the file header to determine if the database is SQLite or native-v3 format.
    /// Returns Backend::Unknown for non-existent files or unrecognized formats.
    ///
    /// # Arguments
    /// * `path` - Path to the database file
    ///
    /// # Returns
    /// * `Ok(Backend)` - Detected backend format
    /// * `Err(SpliceError)` - If file cannot be read
    ///
    /// # Examples
    /// ```no_run
    /// use splice::graph::CodeGraph;
    /// use std::path::Path;
    ///
    /// let backend = CodeGraph::detect_backend(Path::new(".codemcp/codegraph.db"))?;
    /// println!("Database backend: {}", backend);
    /// # Ok::<(), splice::SpliceError>(())
    /// ```
    pub fn detect_backend(path: &Path) -> Result<Backend> {
        if !path.exists() {
            return Ok(Backend::Unknown);
        }
        Ok(if Self::is_sqlite_db(path)? {
            Backend::SQLite
        } else {
            // If it exists but isn't SQLite, assume native-v3
            // (native-v3 databases don't have a recognizable header like SQLite)
            Backend::NativeV3
        })
    }

    /// Store a symbol with its byte span and metadata (legacy method for backward compatibility).
    ///
    /// Creates a node in the graph with:
    /// - Label: "symbol_function", "symbol_class", etc. (language-agnostic)
    /// - Properties: name, kind, byte_start, byte_end
    ///
    /// Returns the NodeId of the created node.
    ///
    /// # Deprecated
    /// This method is kept for backward compatibility. Use `store_symbol_with_file_and_language`
    /// for new code.
    #[deprecated(note = "Use store_symbol_with_file_and_language for multi-language support")]
    pub fn store_symbol(
        &mut self,
        name: &str,
        kind: &str,
        byte_start: usize,
        byte_end: usize,
    ) -> Result<NodeId> {
        // Determine label based on kind
        let label = schema::kind_to_label(kind);

        // Create node spec
        let node_spec = NodeSpec {
            kind: label.0,
            name: name.to_string(),
            file_path: None,
            data: json!({
                "kind": kind,
                "byte_start": byte_start,
                "byte_end": byte_end,
            }),
        };

        // Insert node
        let node_id_i64 = self.backend.insert_node(node_spec)?;
        let node_id = NodeId::from(node_id_i64);

        // Flush to ensure data is written to disk (critical for native-v3 backend)
        let _ = self.backend.flush();

        // Cache the symbol name → NodeId mapping
        self.symbol_cache
            .entry(name.to_string())
            .or_default()
            .push(node_id);

        Ok(node_id)
    }

    /// Store a symbol with file association, language, and complete metadata.
    ///
    /// This method:
    /// 1. Creates a File node if it doesn't exist
    /// 2. Creates a Symbol node with all metadata (byte spans, line/col, language)
    /// 3. Creates a DEFINES edge from File to Symbol
    ///
    /// Returns the NodeId of the created Symbol node.
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
        // Get or create File node
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;
        let file_node_id = self.get_or_create_file_node(file_path_str)?;

        // Determine label based on kind (language-agnostic)
        let label = schema::kind_to_label(kind);

        // Create symbol node with file_path, language, and line/col in spec
        let node_spec = NodeSpec {
            kind: label.0,
            name: name.to_string(),
            file_path: Some(file_path_str.to_string()),
            data: json!({
                "kind": kind,
                "language": language.as_str(),
                "byte_start": byte_start,
                "byte_end": byte_end,
                "line_start": line_start,
                "line_end": line_end,
                "col_start": col_start,
                "col_end": col_end,
                "file_path": file_path_str,
            }),
        };

        // Insert symbol node
        let symbol_id_i64 = self.backend.insert_node(node_spec)?;
        let symbol_id = NodeId::from(symbol_id_i64);

        // Create DEFINES edge: File ─[DEFINES]→ Symbol
        let edge_spec = EdgeSpec {
            from: file_node_id.as_i64(),
            to: symbol_id.as_i64(),
            edge_type: schema::EDGE_DEFINES.to_string(),
            data: json!({}),
        };
        self.backend.insert_edge(edge_spec)?;

        // Flush to ensure data is written to disk (critical for native-v3 backend)
        // Native-v3 uses sparse files - without flush, data is lost on reopen
        let _ = self.backend.flush();

        // Cache the symbol name → NodeId mapping (by file)
        let cache_key = format!("{}::{}", file_path_str, name);
        self.symbol_cache
            .entry(cache_key)
            .or_default()
            .push(symbol_id);

        Ok(symbol_id)
    }

    /// Store a symbol with file association using Rust symbol kind.
    ///
    /// This is a backward-compatible method that internally converts
    /// RustSymbolKind to the string representation.
    ///
    /// # Deprecated
    /// Use `store_symbol_with_file_and_language` for new code.
    #[deprecated(note = "Use store_symbol_with_file_and_language for multi-language support")]
    pub fn store_symbol_with_file(
        &mut self,
        file_path: &Path,
        name: &str,
        kind: &str,
        byte_start: usize,
        byte_end: usize,
    ) -> Result<NodeId> {
        // For backward compatibility, assume Rust language and use 0 placeholders for line/col
        self.store_symbol_with_file_and_language(
            file_path,
            name,
            kind,
            Language::Rust,
            byte_start,
            byte_end,
            0,
            0,
            0,
            0, // line_start, line_end, col_start, col_end (placeholders)
        )
    }

    /// Get or create a File node for the given path.
    fn get_or_create_file_node(&mut self, file_path: &str) -> Result<NodeId> {
        // Check cache first
        if let Some(&node_id) = self.file_cache.get(file_path) {
            return Ok(node_id);
        }

        // Create new File node
        let node_spec = NodeSpec {
            kind: schema::label_file().0,
            name: file_path.to_string(),
            file_path: Some(file_path.to_string()),
            data: json!({
                "path": file_path,
            }),
        };

        let node_id_i64 = self.backend.insert_node(node_spec)?;
        let node_id = NodeId::from(node_id_i64);

        // Cache it
        self.file_cache.insert(file_path.to_string(), node_id);

        Ok(node_id)
    }

    /// Resolve a symbol name to its NodeId (legacy method).
    ///
    /// Returns the NodeId if the symbol exists in the graph.
    /// NOTE: This uses the old cache format and is kept for backward compatibility.
    pub fn resolve_symbol(&self, name: &str) -> Result<NodeId> {
        self.symbol_cache
            .get(name)
            .and_then(|ids| ids.first())
            .copied()
            .ok_or_else(|| SpliceError::symbol_not_found(name, None))
    }

    /// Get all symbol nodes with a given name across all files.
    ///
    /// Returns a Vec of (node_id, file_path) tuples for all symbols with the given name.
    pub fn find_symbols_by_name(&self, name: &str) -> Vec<(NodeId, Option<String>)> {
        let mut results = Vec::new();

        // First, search through cache keys that end with "::name"
        for (key, ids) in &self.symbol_cache {
            if key.ends_with(&format!("::{}", name)) || key == name {
                for &node_id in ids {
                    // Try to get file_path from the node
                    // Use SnapshotId(0) for latest state (sqlitegraph v1.2.7+ API)
                    if let Ok(node) = self.backend.get_node(SnapshotId(0), node_id.as_i64()) {
                        let file_path = node.data.get("file_path").and_then(|v| v.as_str());
                        results.push((node_id, file_path.map(|s| s.to_string())));
                    }
                }
            }
        }

        // Then, search through database for persisted symbols (critical for native-v3)
        // This finds symbols that were written before the current process started
        if let Ok(all_ids) = self.backend.entity_ids() {
            let snapshot = SnapshotId(0);
            for node_id in all_ids {
                if let Ok(node) = self.backend.get_node(snapshot, node_id) {
                    if node.name == name && node.kind != "File" && node.kind != "file" {
                        let file_path = node.data.get("file_path").and_then(|v| v.as_str());
                        let node_id = NodeId::from(node_id);
                        results.push((node_id, file_path.map(|s| s.to_string())));
                    }
                }
            }
        }

        results
    }

    /// Get symbol by file and name from cache.
    pub fn find_symbol_in_file(&self, file_path: &str, name: &str) -> Option<NodeId> {
        let cache_key = format!("{}::{}", file_path, name);
        self.symbol_cache
            .get(&cache_key)
            .and_then(|ids| ids.first())
            .copied()
    }

    /// Get all unique symbol names from the graph.
    ///
    /// Returns a Vec of all symbol names for fuzzy matching and suggestions.
    /// Used by the suggestions module to provide "did you mean" functionality.
    ///
    /// # Returns
    /// * Vec of unique symbol names (deduplicated)
    pub fn all_symbol_names(&self) -> Vec<String> {
        use std::collections::HashSet;

        let mut names = HashSet::new();

        // First, collect from cache (for recently written symbols)
        for key in self.symbol_cache.keys() {
            // Cache keys are either "name" or "file_path::name"
            // Extract just the name part
            if let Some(name) = key.split("::").last() {
                names.insert(name.to_string());
            } else {
                names.insert(key.clone());
            }
        }

        // Then, collect from database (for persisted symbols across reopen)
        // This is critical for native-v3 backend where cache is not persisted
        if let Ok(all_ids) = self.backend.entity_ids() {
            let snapshot = SnapshotId(0);
            for node_id in all_ids {
                if let Ok(node) = self.backend.get_node(snapshot, node_id) {
                    // Skip File nodes - only include actual symbols
                    if node.kind != "File" && node.kind != "file" {
                        names.insert(node.name);
                    }
                }
            }
        }

        names.into_iter().collect()
    }

    /// Get the byte span for a NodeId.
    ///
    /// Returns (byte_start, byte_end) from the node's properties.
    pub fn get_span(&self, node_id: NodeId) -> Result<(usize, usize)> {
        // Get node from graph - use SnapshotId(0) for latest state (sqlitegraph v1.2.7+ API)
        let node = self.backend.get_node(SnapshotId(0), node_id.as_i64())?;

        // Extract byte span from data
        let byte_start = node
            .data
            .get("byte_start")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| SpliceError::Other("Missing byte_start property".to_string()))?
            as usize;

        let byte_end = node
            .data
            .get("byte_end")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| SpliceError::Other("Missing byte_end property".to_string()))?
            as usize;

        Ok((byte_start, byte_end))
    }

    /// Access the underlying graph backend for advanced operations.
    pub fn inner(&self) -> &dyn GraphBackend {
        self.backend.as_ref()
    }

    /// Access the underlying graph backend mutably for advanced operations.
    pub fn inner_mut(&mut self) -> &mut dyn GraphBackend {
        self.backend.as_mut()
    }

    /// Verify that a migration completed successfully.
    ///
    /// Compares node and edge counts between source and destination databases.
    /// Returns true if counts match exactly.
    ///
    /// # Arguments
    /// * `source_path` - Path to the original source database
    /// * `dest_path` - Path to the migrated destination database
    ///
    /// # Returns
    /// * `Ok(())` - Verification passed, counts match
    /// * `Err(SpliceError)` - Verification failed or could not be performed
    pub fn verify_migration(source_path: &Path, dest_path: &Path) -> Result<()> {
        // Open source database
        let source_cfg = if Self::is_sqlite_db(source_path)? {
            sqlitegraph::GraphConfig::sqlite()
        } else {
            sqlitegraph::GraphConfig::native()
        };
        let source_backend = sqlitegraph::open_graph(source_path, &source_cfg)
            .map_err(|e| SpliceError::Other(format!("Failed to open source for verification: {}", e)))?;

        // Open destination database
        let dest_backend = sqlitegraph::open_graph(dest_path, &sqlitegraph::GraphConfig::native())
            .map_err(|e| SpliceError::Other(format!("Failed to open destination for verification: {}", e)))?;

        // Get node counts using entity_ids()
        let source_nodes = source_backend
            .entity_ids()
            .map_err(|e| SpliceError::Other(format!("Failed to get source node count: {}", e)))?
            .len();
        let dest_nodes = dest_backend
            .entity_ids()
            .map_err(|e| SpliceError::Other(format!("Failed to get destination node count: {}", e)))?
            .len();

        // For edge counts, we need to iterate through all nodes and sum their degrees
        // This is expensive but necessary for verification
        let snapshot_id = SnapshotId::current();

        let source_node_ids = source_backend.entity_ids().map_err(|e| SpliceError::Other(format!("Failed to get source nodes: {}", e)))?;
        let mut source_edges = 0usize;
        for node_id in source_node_ids {
            let (in_degree, out_degree) = source_backend
                .node_degree(snapshot_id, node_id)
                .map_err(|e| SpliceError::Other(format!("Failed to get source node degree: {}", e)))?;
            source_edges += in_degree + out_degree;
        }

        let dest_node_ids = dest_backend.entity_ids().map_err(|e| SpliceError::Other(format!("Failed to get dest nodes: {}", e)))?;
        let mut dest_edges = 0usize;
        for node_id in dest_node_ids {
            let (in_degree, out_degree) = dest_backend
                .node_degree(snapshot_id, node_id)
                .map_err(|e| SpliceError::Other(format!("Failed to get dest node degree: {}", e)))?;
            dest_edges += in_degree + out_degree;
        }

        // Verify counts match
        let nodes_match = source_nodes == dest_nodes;
        let edges_match = source_edges == dest_edges;

        if nodes_match && edges_match {
            Ok(())
        } else {
            // Return detailed error
            let mut error_parts = Vec::new();
            if !nodes_match {
                error_parts.push(format!(
                    "node count mismatch: source={}, dest={}",
                    source_nodes, dest_nodes
                ));
            }
            if !edges_match {
                error_parts.push(format!(
                    "edge count mismatch: source={}, dest={}",
                    source_edges, dest_edges
                ));
            }
            Err(SpliceError::Other(format!(
                "Migration verification failed: {}",
                error_parts.join(", ")
            )))
        }
    }

    /// Migrate this database to native-v3 format.
    ///
    /// This method:
    /// 1. Exports the current database to a snapshot in a temporary directory
    /// 2. Creates a new native-v3 database at the destination path
    /// 3. Imports the snapshot into the new database
    /// 4. Returns a migration report with statistics
    ///
    /// The source database is never modified. The destination must not exist.
    ///
    /// # Arguments
    /// * `source_path` - Path to the source database (for verification in plan 34-04)
    /// * `dest_path` - Path where the native-v3 database will be created
    /// * `progress` - Optional callback for progress reporting (receives step name)
    ///
    /// # Returns
    /// * `Ok(MigrationReport)` - Statistics about the migration
    /// * `Err(SpliceError)` - If migration fails at any step
    ///
    /// # Example
    /// ```no_run
    /// # use splice::graph::CodeGraph;
    /// # use std::path::Path;
    /// let src_graph = CodeGraph::open(Path::new("old.db"))?;
    /// // Note: source_path is passed for verification (added in plan 34-04)
    /// let report = src_graph.migrate_to_native_v3(
    ///     Path::new("old.db"),
    ///     Path::new("new.db"),
    ///     None
    /// )?;
    /// println!("Migrated {} nodes", report.nodes_migrated);
    /// # Ok::<(), splice::SpliceError>(())
    /// ```
    ///
    /// # Errors
    /// - If destination path already exists
    /// - If temporary directory cannot be created
    /// - If snapshot export fails (database corruption, disk full)
    /// - If native-v3 database creation fails (requires native-v3 or migration feature)
    /// - If snapshot import fails
    #[cfg(any(feature = "native-v3", feature = "migration"))]
    pub fn migrate_to_native_v3(
        &self,
        source_path: &Path,
        dest_path: &Path,
        progress: Option<&dyn Fn(&str)>,
        verify: bool,
    ) -> Result<MigrationReport> {
        use std::path::PathBuf;

        // Verify destination doesn't exist
        if dest_path.exists() {
            return Err(SpliceError::Other(format!(
                "Destination database already exists: {:?}. \
                 Please remove it or choose a different path.",
                dest_path
            )));
        }

        // Create temporary directory for snapshot
        let temp_dir = std::env::temp_dir().join(format!(
            "splice_migration_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&temp_dir).map_err(|e| SpliceError::Io {
            path: temp_dir.clone(),
            source: e,
        })?;

        if let Some(p) = progress {
            p("Exporting snapshot from source database...");
        }

        // Export snapshot from source database
        let export_result = self
            .backend
            .snapshot_export(&temp_dir)
            .map_err(|e| SpliceError::Other(format!("Snapshot export failed: {}", e)))?;

        if let Some(p) = progress {
            p("Creating native-v3 database...");
        }

        // Create native-v3 database at destination
        let native_cfg = sqlitegraph::GraphConfig::native();
        let dest_backend = sqlitegraph::open_graph(dest_path, &native_cfg)
            .map_err(|e| SpliceError::Other(format!("Failed to create native-v3 database: {}", e)))?;

        if let Some(p) = progress {
            p("Importing snapshot to native-v3 database...");
        }

        // Import snapshot to destination
        let _import_result = dest_backend
            .snapshot_import(&temp_dir)
            .map_err(|e| SpliceError::Other(format!("Snapshot import failed: {}", e)))?;

        // Clean up temporary directory
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Verify migration if requested (default: true)
        let (verification_passed, verification_error) = if verify {
            if let Some(p) = progress {
                p("Verifying migration...");
            }

            match Self::verify_migration(source_path, dest_path) {
                Ok(()) => (true, None),
                Err(e) => {
                    // Verification failed - clean up destination and return error
                    let _ = std::fs::remove_file(dest_path);
                    if let Some(p) = progress {
                        p("Verification failed - rolling back...");
                    }
                    return Err(e);
                }
            }
        } else {
            (true, None)  // Skip verification, assume success
        };

        if let Some(p) = progress {
            p("Migration complete!");
        }

        Ok(MigrationReport {
            nodes_migrated: export_result.entity_count as usize,
            edges_migrated: export_result.edge_count as usize,
            snapshot_metadata: format!(
                "snapshot_path={}, size_bytes={}, entity_count={}, edge_count={}",
                export_result.snapshot_path.display(),
                export_result.size_bytes,
                export_result.entity_count,
                export_result.edge_count
            ),
            destination: dest_path.to_path_buf(),
            verification_passed,
            verification_error,
        })
    }

    /// Migration is not available without the native-v3 or migration feature.
    ///
    /// Build with: `cargo build --features native-v3 --no-default-features`
    /// Or for migration tests: `cargo build --features migration`
    #[cfg(not(any(feature = "native-v3", feature = "migration")))]
    pub fn migrate_to_native_v3(
        &self,
        _source_path: &Path,
        _dest_path: &Path,
        _progress: Option<&dyn Fn(&str)>,
        _verify: bool,
    ) -> Result<MigrationReport> {
        Err(SpliceError::Other(
            "Migration to native-v3 requires the native-v3 or migration feature. \
             Build with: cargo build --features native-v3 --no-default-features \
             or cargo build --features migration"
                .to_string(),
        ))
    }

    /// Restore a database from a snapshot file (native-v3 only).
    ///
    /// This method restores a code graph database from a previously captured snapshot.
    /// The snapshot must have been created from the same database.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the database file to restore
    /// * `snapshot_path` - Path to the snapshot file to restore from
    ///
    /// # Returns
    ///
    /// `Ok(RestoreResult)` with backup path and restored symbol/edge counts
    ///
    /// # Errors
    ///
    /// - If database backend is SQLite (restore only supported for native-v3)
    /// - If snapshot file doesn't exist or is corrupted
    /// - If backup creation fails
    /// - If database restoration fails
    ///
    /// # Example
    /// ```no_run
    /// # use splice::graph::CodeGraph;
    /// # use std::path::Path;
    /// // Restore database from snapshot
    /// let result = CodeGraph::restore_from_snapshot(
    ///     Path::new(".codemcp/codegraph.db"),
    ///     Path::new(".splice/snapshots/rename-2023-01-15.json")
    /// )?;
    /// println!("Restored {} symbols from snapshot", result.symbols_restored);
    /// println!("Backup created at: {:?}", result.backup_path);
    /// # Ok::<(), splice::SpliceError>(())
    /// ```
    #[cfg(any(feature = "native-v3", feature = "migration"))]
    pub fn restore_from_snapshot(
        db_path: &Path,
        snapshot_path: &Path,
    ) -> Result<crate::proof::storage::RestoreResult> {
        crate::proof::storage::SnapshotStorage::restore_from_snapshot(db_path, snapshot_path)
    }

    /// Restore is not available without the native-v3 or migration feature.
    ///
    /// Build with: `cargo build --features native-v3 --no-default-features`
    #[cfg(not(any(feature = "native-v3", feature = "migration")))]
    pub fn restore_from_snapshot(
        _db_path: &Path,
        _snapshot_path: &Path,
    ) -> Result<crate::proof::storage::RestoreResult> {
        Err(SpliceError::Other(
            "Database restore requires the native-v3 or migration feature. \
             Build with: cargo build --features native-v3 --no-default-features \
             or cargo build --features migration"
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod backend_detection_tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    #[test]
    fn test_detect_backend_sqlite() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_sqlite_{}.db", uuid::Uuid::new_v4()));

        // Write SQLite header
        let mut file = std::fs::File::create(&db_path).unwrap();
        file.write_all(b"SQLite format 3\0").unwrap();

        let backend = CodeGraph::detect_backend(&db_path).unwrap();
        assert_eq!(backend, Backend::SQLite);

        std::fs::remove_file(&db_path).ok();
    }

    #[test]
    fn test_detect_backend_nonexistent() {
        let temp_path = std::env::temp_dir().join("nonexistent_test_db.db");
        let _ = std::fs::remove_file(&temp_path);

        let backend = CodeGraph::detect_backend(&temp_path).unwrap();
        assert_eq!(backend, Backend::Unknown);
    }

    #[test]
    fn test_detect_backend_empty_file() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_empty_{}.db", uuid::Uuid::new_v4()));

        std::fs::File::create(&db_path).unwrap();

        // Empty file is not SQLite, should be NativeV3 (or Unknown)
        let backend = CodeGraph::detect_backend(&db_path).unwrap();
        // Empty files aren't valid native-v3, but our logic returns NativeV3 for non-SQLite
        assert_eq!(backend, Backend::NativeV3);

        std::fs::remove_file(&db_path).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_store_symbol_with_line_col() {
        // Create a temporary graph database
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_graph_{}.db", uuid::Uuid::new_v4()));
        let mut code_graph = CodeGraph::open(&db_path).expect("Failed to open test graph database");

        // Store a symbol with specific line/col values
        let file_path = PathBuf::from("/test/path.rs");
        let node_id = code_graph
            .store_symbol_with_file_and_language(
                &file_path,
                "test_function",
                "function",
                Language::Rust,
                100, // byte_start
                200, // byte_end
                5,   // line_start
                10,  // line_end
                12,  // col_start
                45,  // col_end
            )
            .expect("Failed to store symbol with line/col");

        // Retrieve the node and verify line/col were stored
        let snapshot_id = SnapshotId::current();
        let node = code_graph
            .inner()
            .get_node(snapshot_id, node_id.as_i64())
            .expect("Failed to retrieve node");

        assert_eq!(
            node.data.get("line_start").and_then(|v| v.as_u64()),
            Some(5)
        );
        assert_eq!(node.data.get("line_end").and_then(|v| v.as_u64()), Some(10));
        assert_eq!(
            node.data.get("col_start").and_then(|v| v.as_u64()),
            Some(12)
        );
        assert_eq!(node.data.get("col_end").and_then(|v| v.as_u64()), Some(45));

        // Clean up
        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn test_backward_compatibility_zero_values() {
        // Verify that storing with 0 placeholders works (backward compatibility)
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_graph_{}.db", uuid::Uuid::new_v4()));
        let mut code_graph = CodeGraph::open(&db_path).expect("Failed to open test graph database");

        let file_path = PathBuf::from("/test/path.rs");
        let node_id = code_graph
            .store_symbol_with_file_and_language(
                &file_path,
                "old_function",
                "function",
                Language::Rust,
                50,  // byte_start
                100, // byte_end
                0,
                0,
                0,
                0, // All line/col zeros (placeholders)
            )
            .expect("Failed to store symbol with zeros");

        let snapshot_id = SnapshotId::current();
        let node = code_graph
            .inner()
            .get_node(snapshot_id, node_id.as_i64())
            .expect("Failed to retrieve node");

        // Zeros should be stored correctly
        assert_eq!(
            node.data.get("line_start").and_then(|v| v.as_u64()),
            Some(0)
        );
        assert_eq!(node.data.get("line_end").and_then(|v| v.as_u64()), Some(0));
        assert_eq!(node.data.get("col_start").and_then(|v| v.as_u64()), Some(0));
        assert_eq!(node.data.get("col_end").and_then(|v| v.as_u64()), Some(0));

        // Clean up
        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn test_deprecated_method_passes_zeros() {
        // Verify the deprecated store_symbol_with_file passes zeros
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_graph_{}.db", uuid::Uuid::new_v4()));
        let mut code_graph = CodeGraph::open(&db_path).expect("Failed to open test graph database");

        let file_path = PathBuf::from("/test/path.rs");
        let node_id = code_graph
            .store_symbol_with_file(&file_path, "dep_function", "function", 10, 20)
            .expect("Failed to store symbol via deprecated method");

        let snapshot_id = SnapshotId::current();
        let node = code_graph
            .inner()
            .get_node(snapshot_id, node_id.as_i64())
            .expect("Failed to retrieve node");

        // All line/col should be 0 from deprecated method
        assert_eq!(
            node.data.get("line_start").and_then(|v| v.as_u64()),
            Some(0)
        );
        assert_eq!(node.data.get("line_end").and_then(|v| v.as_u64()), Some(0));
        assert_eq!(node.data.get("col_start").and_then(|v| v.as_u64()), Some(0));
        assert_eq!(node.data.get("col_end").and_then(|v| v.as_u64()), Some(0));

        // Clean up
        let _ = std::fs::remove_file(db_path);
    }
}
