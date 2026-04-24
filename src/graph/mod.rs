//! Code graph integration layer with split backend support.
//!
//! This module provides a typed interface to code graph databases,
//! supporting three backends:
//! - SQLite (.db files) - Traditional sqlitegraph backend
//! - Native-V3 (.v3 files) - High-performance binary backend  
//! - Geometric (.geo files) - Spatial indexing and CFG analysis
//!
//! The architecture uses clean separation:
//! - `backend.rs` - Minimal shared trait (`CodeIntelBackend`)
//! - `sqlite_impl.rs` - SQLite/V3 implementation (`CodeGraphSqlite`)
//! - `geo_impl.rs` - Geometric implementation (`CodeGraphGeo`)
//! - `router.rs` - Public enum that dispatches to appropriate backend

// Backend implementations
pub mod backend;
pub mod geo_impl;
pub mod router;
pub mod sqlite_impl;

// Legacy modules (to be migrated or kept for compatibility)
pub mod magellan_integration;
pub mod migrate;
pub mod rename;
pub mod schema;

// Re-export the public API
pub use backend::SymbolInfo;
pub use router::{BackendType, CodeGraph};
pub use sqlite_impl::CodeGraphSqlite;

#[cfg(feature = "geometric")]
pub use geo_impl::CodeGraphGeo;

// Re-export MagellanIntegration for existing code that needs it
pub use magellan_integration::MagellanIntegration;

// Legacy: Deprecated Backend enum - use BackendType instead
#[deprecated(since = "2.6.0", note = "Use BackendType instead")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    SQLite,
    NativeV3,
    #[cfg(feature = "geometric")]
    Geometric,
    Unknown,
}

#[allow(deprecated)]
impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backend::SQLite => write!(f, "sqlite"),
            Backend::NativeV3 => write!(f, "native-v3"),
            #[cfg(feature = "geometric")]
            Backend::Geometric => write!(f, "geometric"),
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

#[cfg(test)]
mod backend_detection_tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_detect_backend_sqlite() {
        let temp_file = NamedTempFile::new().unwrap();
        let detected = CodeGraph::detect_backend(temp_file.path()).unwrap();
        assert!(matches!(detected, BackendType::SQLite));
    }

    #[test]
    fn test_detect_backend_geometric() {
        let geo_path = std::path::Path::new("test.geo");
        let detected = CodeGraph::detect_backend(geo_path).unwrap();
        assert!(matches!(detected, BackendType::Geometric));
    }

    #[test]
    fn test_detect_backend_nonexistent() {
        let path = std::path::Path::new("/nonexistent/path/code.db");
        let detected = CodeGraph::detect_backend(path).unwrap();
        // Non-existent files default to SQLite
        assert!(matches!(detected, BackendType::SQLite));
    }

    #[test]
    fn test_is_geometric_db_helper() {
        assert!(CodeGraph::is_geometric_db(std::path::Path::new("code.geo")));
        assert!(!CodeGraph::is_geometric_db(std::path::Path::new("code.db")));
        assert!(!CodeGraph::is_geometric_db(std::path::Path::new("code.v3")));
    }

    #[test]
    fn test_detect_backend_empty_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        // Write empty SQLite header
        write!(temp_file, "SQLite format 3\0").unwrap();
        let detected = CodeGraph::detect_backend(temp_file.path()).unwrap();
        assert!(matches!(detected, BackendType::SQLite));
    }

    #[test]
    #[cfg(feature = "geometric")]
    fn test_backend_display_geometric() {
        use std::fmt::Write;
        let mut s = String::new();
        write!(&mut s, "{}", BackendType::Geometric).unwrap();
        assert_eq!(s, "geometric");
    }

    #[test]
    fn test_detect_backend_v3_extension() {
        let path = std::path::Path::new("test.v3");
        let detected = CodeGraph::detect_backend(path).unwrap();
        assert!(matches!(detected, BackendType::NativeV3));
    }
}
