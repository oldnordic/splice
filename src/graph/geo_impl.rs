//! Geometric backend implementation.
//!
//! This module provides the CodeIntelBackend trait implementation
//! for .geo databases using magellan's GeometricBackend directly.

/// Geometric code graph implementation.
///
/// Wraps magellan's GeometricBackend and provides the CodeIntelBackend
/// trait implementation for Splice workflows.
#[cfg(feature = "geometric")]
pub struct CodeGraphGeo {
    /// The underlying geometric backend
    inner: magellan::graph::geometric_backend::GeometricBackend,
    /// Database path
    db_path: std::path::PathBuf,
    /// Cache for symbol name → Vec<NodeId> mapping (using entity_id as NodeId)
    symbol_cache: HashMap<String, Vec<NodeId>>,
}

#[cfg(feature = "geometric")]
impl CodeGraphGeo {
    /// Open an existing geometric database.
    pub fn open(path: &Path) -> Result<Self> {
        use magellan::graph::geometric_backend::GeometricBackend;

        let inner = GeometricBackend::open(path).map_err(|e| {
            SpliceError::Other(format!(
                "Failed to open geometric backend at {:?}: {}",
                path, e
            ))
        })?;

        Ok(Self {
            inner,
            db_path: path.to_path_buf(),
            symbol_cache: HashMap::new(),
        })
    }

    /// Get the database path.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Access the underlying GeometricBackend.
    pub fn inner(&self) -> &magellan::graph::geometric_backend::GeometricBackend {
        &self.inner
    }

    /// Access the underlying GeometricBackend mutably.
    pub fn inner_mut(&mut self) -> &mut magellan::graph::geometric_backend::GeometricBackend {
        &mut self.inner
    }

    /// Get all files in the database.
    pub fn get_all_files(&self) -> Vec<String> {
        self.inner
            .get_all_files()
            .into_iter()
            .map(|(path, _, _)| path)
            .collect()
    }

    /// Get code chunk for a symbol.
    pub fn get_code_chunk(
        &self,
        file_path: &Path,
        byte_start: usize,
        byte_end: usize,
    ) -> Result<Option<String>> {
        // Convert to magellan's CodeChunk format
        let path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;
        let chunks = self
            .inner
            .get_code_chunks(path_str)
            .map_err(|e| SpliceError::Other(format!("Failed to get code chunks: {}", e)))?;

        // Find chunk matching the span
        for chunk in chunks {
            if chunk.byte_start == byte_start && chunk.byte_end == byte_end {
                return Ok(Some(chunk.content));
            }
        }

        Ok(None)
    }
}

#[cfg(feature = "geometric")]
// Inherent methods
impl CodeGraphGeo {
    pub fn find_symbol_in_file(&self, file_path: &str, name: &str) -> Option<NodeId> {
        // Check cache first
        let cache_key = format!("{}::{}", file_path, name);
        if let Some(ids) = self.symbol_cache.get(&cache_key) {
            return ids.first().copied();
        }

        // Use geometric backend's find_symbol_id_by_name_and_path
        match self.inner.find_symbol_id_by_name_and_path(name, file_path) {
            Some(id) => Some(NodeId(id as i64)),
            None => None,
        }
    }

    pub fn find_symbols_by_name(&self, name: &str) -> Vec<(NodeId, Option<String>)> {
        // Use geometric backend's find_symbols_by_name_info
        let symbols = self.inner.find_symbols_by_name_info(name);
        symbols
            .into_iter()
            .map(|info| (NodeId(info.id as i64), Some(info.file_path)))
            .collect()
    }

    pub fn all_symbol_names(&self) -> Vec<String> {
        use std::collections::HashSet;
        let mut names = HashSet::new();

        // Collect from cache
        for key in self.symbol_cache.keys() {
            if let Some(name) = key.split("::").last() {
                names.insert(name.to_string());
            } else {
                names.insert(key.clone());
            }
        }

        // Collect from geometric backend
        match self.inner.get_all_symbols() {
            Ok(symbols) => {
                for symbol in symbols {
                    names.insert(symbol.name);
                }
            }
            Err(_) => {}
        }

        names.into_iter().collect()
    }

    pub fn get_span(&self, node_id: NodeId) -> Result<(usize, usize)> {
        let entity_id = node_id.0 as u64;

        match self.inner.find_symbol_by_id_info(entity_id) {
            Some(info) => Ok((info.byte_start as usize, info.byte_end as usize)),
            None => Err(SpliceError::SymbolNotFound {
                message: format!("Symbol with entity_id {} not found", entity_id),
                symbol: format!("entity_{}", entity_id),
                file: None,
                hint: "Symbol may have been deleted".to_string(),
            }),
        }
    }

    pub fn store_symbol(
        &mut self,
        _name: &str,
        _kind: &str,
        _language: Language,
        _byte_start: usize,
        _byte_end: usize,
        _line_start: usize,
        _line_end: usize,
        _col_start: usize,
        _col_end: usize,
    ) -> Result<NodeId> {
        // Geometric backend stores symbols during indexing, not individually
        Err(SpliceError::Other(
            "Direct symbol storage not supported on geometric backend. Use magellan watch to index files.".to_string()
        ))
    }

    pub fn store_symbol_with_file_and_language(
        &mut self,
        _file_path: &Path,
        _name: &str,
        _kind: &str,
        _language: Language,
        _byte_start: usize,
        _byte_end: usize,
        _line_start: usize,
        _line_end: usize,
        _col_start: usize,
        _col_end: usize,
    ) -> Result<NodeId> {
        // Same limitation as store_symbol
        Err(SpliceError::Other(
            "Direct symbol storage not supported on geometric backend. Use magellan watch to index files.".to_string()
        ))
    }

    pub fn backend_type(&self) -> BackendType {
        BackendType::Geometric
    }
}
