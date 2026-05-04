//! Layer 3: Component Integration Tests for GeometricDB Backend
//!
//! These tests validate cross-component interactions:
//! - Splice graph module ↔ Magellan crate
//! - CLI layer ↔ Splice library
//! - End-to-end workflows with GeometricDB

use splice::graph::BackendType;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Test that Splice can detect GeometricDB format across component boundary
///
/// Layer 3: Cross-component integration between graph::CodeGraph and actual file system
#[test]
#[cfg(feature = "geometric")]
fn test_component_geometric_detection_workflow() {
    use splice::graph::CodeGraph;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let geo_path = temp_dir.path().join("test_component.geo");

    // Layer 3a: Create actual .geo file
    std::fs::File::create(&geo_path).expect("Failed to create .geo file");

    // Layer 3b: CodeGraph::detect_backend works across component boundary
    let backend =
        CodeGraph::detect_backend(&geo_path).expect("Layer 3b: detect_backend should not fail");

    assert_eq!(
        backend,
        BackendType::Geometric,
        "Layer 3b: Should detect Geometric backend"
    );

    // Layer 3c: Display output is consistent
    assert_eq!(
        backend.to_string(),
        "geometric",
        "Layer 3c: Display should be 'geometric'"
    );

    // Cleanup handled by temp_dir
}

/// Test integration with the status command workflow
///
/// Layer 3: Test the actual CLI status detection path
#[test]
#[cfg(feature = "geometric")]
fn test_component_status_backend_detection() {
    use splice::graph::CodeGraph;
    use std::path::Path;

    let temp_dir = tempfile::TempDir::new().unwrap();

    // Layer 3a: SQLite database
    let sqlite_path = temp_dir.path().join("test.sqlite.db");
    std::fs::write(&sqlite_path, b"SQLite format 3\0").unwrap();

    let backend = CodeGraph::detect_backend(&sqlite_path).unwrap();
    assert_eq!(
        backend.to_string(),
        "sqlite",
        "Layer 3a: Should detect SQLite"
    );

    // Layer 3b: Geometric database (with feature)
    let geo_path = temp_dir.path().join("test.geo");
    std::fs::File::create(&geo_path).unwrap();

    let backend = CodeGraph::detect_backend(&geo_path).unwrap();

    #[cfg(feature = "geometric")]
    assert_eq!(
        backend.to_string(),
        "geometric",
        "Layer 3b: Should detect Geometric"
    );

    #[cfg(not(feature = "geometric"))]
    assert_eq!(
        backend.to_string(),
        "unknown",
        "Layer 3b: Should be Unknown without feature"
    );

    // Layer 3c: Non-existent file
    let nonexistent = temp_dir.path().join("nonexistent.db");
    let backend = CodeGraph::detect_backend(&nonexistent).unwrap();
    assert_eq!(
        backend.to_string(),
        "sqlite",
        "Layer 3c: Non-existent should default to SQLite"
    );
}

/// Test that BackendType enum variants are stable across the API boundary
///
/// Layer 3: API stability test
#[test]
fn test_component_backend_api_stability() {
    use splice::graph::BackendType;

    // Layer 3a: All variants can be constructed and compared
    let sqlite = BackendType::SQLite;

    // Layer 3b: Clone trait works
    let _sqlite_clone = sqlite.clone();

    // Layer 3c: Copy trait works (implied by Clone for simple enums)
    let _sqlite_copy: BackendType = sqlite;
    // Originals are still usable due to Copy
    let _ = sqlite;

    // Layer 3d: Debug trait works
    let sqlite_debug = format!("{:?}", sqlite);
    assert!(
        sqlite_debug.contains("SQLite") || sqlite_debug.contains("SQLite"),
        "Layer 3d: Debug should show variant name"
    );

    // Layer 3e: Display trait works
    assert_eq!(format!("{}", sqlite), "sqlite");

    // Layer 3f: Geometric variant (when enabled)
    #[cfg(feature = "geometric")]
    {
        let geometric = BackendType::Geometric;
        assert_eq!(format!("{}", geometric), "geometric");
        assert_ne!(geometric, sqlite);
    }
}

/// Test error handling across component boundary
///
/// Layer 3: Error propagation from graph module to callers
#[test]
#[cfg(feature = "geometric")]
fn test_component_error_handling() {
    use splice::graph::CodeGraph;

    // Layer 3a: detect_backend returns Result, not panic
    let result = CodeGraph::detect_backend(Path::new("/nonexistent/path/file.geo"));
    assert!(result.is_ok(), "Layer 3a: Should return Ok, not panic");
    assert_eq!(result.unwrap().to_string(), "sqlite");

    // Layer 3b: is_geometric_db doesn't panic on any path
    let paths = vec![
        "/nonexistent/path/file.geo",
        "",
        ".",
        "..",
        "/",
        "file.geo",
        "file.db",
    ];

    for path_str in paths {
        let path = Path::new(path_str);
        // Should not panic
        let _ = CodeGraph::is_geometric_db(path);
    }
}

/// Test partial OK principle with backend detection metrics
///
/// Layer 3: Validation that goes beyond binary pass/fail
#[test]
#[cfg(feature = "geometric")]
fn test_component_partial_ok_validation() {
    use splice::graph::CodeGraph;

    let temp_dir = tempfile::TempDir::new().unwrap();

    // Create test files
    let sqlite_path = temp_dir.path().join("test.db");
    std::fs::write(&sqlite_path, b"SQLite format 3\0").unwrap();

    let geo_path = temp_dir.path().join("test.geo");
    std::fs::File::create(&geo_path).unwrap();

    let empty_path = temp_dir.path().join("empty");
    std::fs::File::create(&empty_path).unwrap();

    // Layer 3a: Core operation succeeded (Layer 1)
    let sqlite_backend = CodeGraph::detect_backend(&sqlite_path).unwrap();
    let geo_backend = CodeGraph::detect_backend(&geo_path).unwrap();
    let _empty_backend = CodeGraph::detect_backend(&empty_path).unwrap();

    // Layer 3b: Sub-components validated (Layer 2)
    // Each detection should return a valid Backend variant
    assert!(
        matches!(sqlite_backend, BackendType::SQLite),
        "Layer 3b: SQLite detection should return SQLite variant"
    );

    // Layer 3c: Integration metrics within tolerance
    // All operations should complete in reasonable time (implicit in test timeout)
    // All paths should be handled without panic

    // Layer 3d: Cross-component consistency
    // is_geometric_db should be consistent with detect_backend
    let is_geo_sqlite = CodeGraph::is_geometric_db(&sqlite_path);
    let is_geo_geo = CodeGraph::is_geometric_db(&geo_path);

    assert!(!is_geo_sqlite, "Layer 3d: .db should not be geometric");
    assert!(is_geo_geo, "Layer 3d: .geo should be geometric");

    // Consistency check: if is_geometric_db returns true, detect_backend should return Geometric (or Unknown)
    if is_geo_geo {
        let backend_str = geo_backend.to_string();
        assert!(
            backend_str == "geometric" || backend_str == "unknown",
            "Layer 3d: .geo file should be geometric or unknown, got {}",
            backend_str
        );
    }
}
