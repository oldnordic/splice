//! Magellan-based ingestion.
//!
//! This module provides ingestion using Magellan v0.5.0's multi-language parsers.

use crate::error::Result;
use crate::graph::magellan_integration::MagellanIntegration;
use std::path::Path;

/// Magellan-based ingestor.
///
/// Uses Magellan's parsers for multi-language code indexing (7 languages).
pub struct MagellanIngestor {
    integration: MagellanIntegration,
}

impl MagellanIngestor {
    /// Create a new Magellan-based ingestor.
    pub fn new(db_path: &Path) -> Result<Self> {
        Ok(Self {
            integration: MagellanIntegration::open(db_path)?,
        })
    }

    /// Index a file using Magellan's parsers.
    ///
    /// Supports 7 languages: Rust, Python, C, C++, Java, JavaScript, TypeScript.
    pub fn index_file(&mut self, file_path: &Path) -> Result<usize> {
        self.integration.index_file(file_path)
    }

    /// Query symbols by labels.
    pub fn query_by_labels(&self, labels: &[&str]) -> Result<Vec<crate::graph::magellan_integration::SymbolInfo>> {
        self.integration.query_by_labels(labels)
    }

    /// Get code chunk without re-reading the file.
    pub fn get_code(&self, file_path: &Path, start: usize, end: usize) -> Result<Option<String>> {
        self.integration.get_code_chunk(file_path, start, end)
    }

    /// Access the underlying Magellan integration.
    pub fn integration(&self) -> &MagellanIntegration {
        &self.integration
    }

    /// Access the underlying Magellan integration mutably.
    pub fn integration_mut(&mut self) -> &mut MagellanIntegration {
        &mut self.integration
    }
}

/// Convenience function to ingest a file using Magellan.
///
/// This creates a temporary Magellan graph, indexes the file,
/// and returns the number of symbols indexed.
pub fn ingest_file_with_magellan(db_path: &Path, file_path: &Path) -> Result<usize> {
    let mut ingestor = MagellanIngestor::new(db_path)?;
    ingestor.index_file(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_ingestor() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let ingestor = MagellanIngestor::new(&db_path).unwrap();
        assert_eq!(ingestor.query_by_labels(&["rust"]).unwrap().len(), 0);
    }
}
