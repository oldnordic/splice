//! Magellan integration layer with backend split (Batch 1 implementation).
//!
//! This is the new split backend implementation that supports both:
//! - SQLite backend (for .db files)
//! - Geometric backend (for .geo files)
//!
//! Batch 1 implements:
//! - open backend
//! - find symbol by name
//! - get code chunk
//! - symbols in file / list indexed files

use crate::error::{Result, SpliceError};
use magellan::{SymbolKind, SymbolQueryResult};
use std::path::{Path, PathBuf};

/// Backend type identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationBackend {
    Sqlite,
    Geometric,
}

/// Backend-specific inner enum.
pub enum MagellanBackend {
    Sqlite(magellan::CodeGraph),
    #[cfg(feature = "geometric")]
    Geo(magellan::graph::geometric_backend::GeometricBackend),
}

/// Wrapper around Magellan's backends with Splice-specific extensions.
pub struct MagellanIntegrationSplit {
    backend: MagellanBackend,
    db_path: PathBuf,
}

impl MagellanIntegrationSplit {
    /// Get the backend type.
    pub fn backend_type(&self) -> IntegrationBackend {
        match &self.backend {
            MagellanBackend::Sqlite(_) => IntegrationBackend::Sqlite,
            #[cfg(feature = "geometric")]
            MagellanBackend::Geo(_) => IntegrationBackend::Geometric,
        }
    }

    /// Check if using geometric backend.
    pub fn is_geometric(&self) -> bool {
        matches!(self.backend_type(), IntegrationBackend::Geometric)
    }

    /// Open or create a Magellan code graph at the given path.
    pub fn open(db_path: &Path) -> Result<Self> {
        #[cfg(feature = "geometric")]
        if Self::is_geometric_db(db_path) {
            return Self::open_geometric(db_path);
        }
        Self::open_sqlite(db_path)
    }

    /// Open SQLite backend.
    fn open_sqlite(db_path: &Path) -> Result<Self> {
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", db_path)))?;

        let inner = magellan::CodeGraph::open(db_path_str).map_err(|e| SpliceError::Magellan {
            context: format!("Failed to open Magellan SQLite graph at {}", db_path_str),
            source: e,
        })?;

        Ok(Self {
            backend: MagellanBackend::Sqlite(inner),
            db_path: db_path.to_path_buf(),
        })
    }

    /// Open Geometric backend.
    #[cfg(feature = "geometric")]
    fn open_geometric(db_path: &Path) -> Result<Self> {
        use magellan::graph::geometric_backend::GeometricBackend;

        let inner = GeometricBackend::open(db_path).map_err(|e| SpliceError::Magellan {
            context: format!("Failed to open Geometric backend at {:?}", db_path),
            source: e,
        })?;

        Ok(Self {
            backend: MagellanBackend::Geo(inner),
            db_path: db_path.to_path_buf(),
        })
    }

    /// Batch 1: Get code chunk by exact byte span.
    pub fn get_code_chunk(
        &self,
        file_path: &Path,
        start: usize,
        end: usize,
    ) -> Result<Option<crate::graph::magellan_integration::CodeChunk>> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        match &self.backend {
            MagellanBackend::Sqlite(inner) => {
                inner
                    .get_code_chunk_by_span(file_path_str, start, end)
                    .map_err(|e| SpliceError::Other(format!("Failed to get code chunk: {}", e)))
                    .map(|opt_chunk| opt_chunk.map(crate::graph::magellan_integration::CodeChunk::from))
            }
            #[cfg(feature = "geometric")]
            MagellanBackend::Geo(inner) => {
                let chunks = inner.get_code_chunks(file_path_str)
                    .map_err(|e| SpliceError::Other(format!("Failed to get code chunks: {}", e)))?;
                let chunk = chunks.into_iter()
                    .find(|c| c.byte_start == start && c.byte_end == end);
                Ok(chunk.map(|c| crate::graph::magellan_integration::CodeChunk {
                    file_path: c.file_path,
                    content: c.content,
                    byte_start: c.byte_start,
                    byte_end: c.byte_end,
                    symbol_name: c.symbol_name,
                    symbol_kind: c.symbol_kind,
                }))
            }
        }
    }
}
