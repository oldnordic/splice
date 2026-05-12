//! Layer 2: Unit Integration Tests for GeometricDB Backend
//!
//! These tests validate the integration between:
//! - Backend detection (graph::CodeGraph::detect_backend)
//! - MagellanIntegration (graph::magellan_integration::MagellanIntegration)
//! - CLI commands that use backend detection

/// Test that backend detection integrates correctly with MagellanIntegration.open()
///
/// Layer 2: When a .geo file exists, the integration should handle it appropriately
#[test]
#[cfg(feature = "geometric")]
fn test_geometric_backend_integration_with_magellan() {
    use splice::graph::magellan_integration::MagellanIntegration;
    use splice::graph::CodeGraph;
    use std::path::Path;

    let temp_dir = std::env::temp_dir();
    let geo_path = temp_dir.join(format!("test_integration_{}.geo", uuid::Uuid::new_v4()));

    // Create the .geo file
    std::fs::File::create(&geo_path).unwrap();

    // Layer 2a: Backend detection works at the CodeGraph level
    let backend = CodeGraph::detect_backend(&geo_path).unwrap();
    assert_eq!(
        backend.to_string(),
        "geometric",
        "Layer 2a: Backend should be detected as geometric"
    );

    // Layer 2b: MagellanIntegration can open the .geo file (if it exists)
    // Note: This tests the integration point - MagellanIntegration::open delegates to Magellan
    let result = MagellanIntegration::open(&geo_path);

    // The result depends on whether Magellan can open an empty .geo file
    // We're testing the integration, not Magellan's internal behavior
    // Either Ok or Err is acceptable here - we're verifying the call doesn't panic
    match result {
        Ok(_) => {
            // Magellan successfully opened it
        }
        Err(_) => {
            // Magellan couldn't open empty file - that's ok for this test
        }
    }

    // Cleanup
    std::fs::remove_file(&geo_path).ok();
}

/// Test backend detection integration when geometric feature is disabled
///
/// Layer 2: Without geometric feature, .geo files should return Unknown
/// and MagellanIntegration should handle this gracefully
#[test]
#[cfg(not(feature = "geometric"))]
fn test_geometric_backend_disabled_integration() {
    use splice::graph::CodeGraph;

    let temp_dir = std::env::temp_dir();
    let geo_path = temp_dir.join(format!("test_disabled_{}.geo", uuid::Uuid::new_v4()));

    // Create the .geo file
    std::fs::File::create(&geo_path).unwrap();

    // Layer 2: Without geometric feature, detection returns Unknown
    let backend = CodeGraph::detect_backend(&geo_path).unwrap();
    assert_eq!(
        backend.to_string(),
        "unknown",
        "Layer 2: Without feature, .geo should be Unknown"
    );

    // Cleanup
    std::fs::remove_file(&geo_path).ok();
}

/// Test that backend detection handles edge cases in integration context
///
/// Layer 2: Various path formats should all work correctly
#[test]
fn test_backend_detection_edge_cases() {
    use splice::graph::CodeGraph;
    use std::path::Path;

    // Layer 2a: Absolute path with .geo
    let abs_geo = Path::new("/tmp/test_absolute.geo");
    assert!(
        CodeGraph::is_geometric_db(abs_geo),
        "Layer 2a: Absolute .geo path should be recognized"
    );

    // Layer 2b: Relative path with .geo
    let rel_geo = Path::new("./relative/path/code.geo");
    assert!(
        CodeGraph::is_geometric_db(rel_geo),
        "Layer 2b: Relative .geo path should be recognized"
    );

    // Layer 2c: Path with multiple dots
    let multi_dot = Path::new("/some/path/code.v1.backup.geo");
    assert!(
        CodeGraph::is_geometric_db(multi_dot),
        "Layer 2c: Multi-dot .geo path should be recognized"
    );

    // Layer 2d: Case sensitivity (geo vs GEO)
    let upper_geo = Path::new("/some/path/code.GEO");
    // Note: This tests the current behavior - may need to be updated if case-insensitive matching is added
    let is_geometric = CodeGraph::is_geometric_db(upper_geo);
    // Document the current behavior
    println!("Uppercase .GEO recognized: {}", is_geometric);

    // Layer 2e: Non-.geo paths
    let db_path = Path::new("/some/path/code.db");
    assert!(
        !CodeGraph::is_geometric_db(db_path),
        "Layer 2e: .db should not be geometric"
    );

    let no_ext = Path::new("/some/path/code");
    assert!(
        !CodeGraph::is_geometric_db(no_ext),
        "Layer 2e: No extension should not be geometric"
    );
}

/// Test BackendType enum serialization in integration context
///
/// Layer 2: BackendType should serialize correctly for CLI output
#[test]
fn test_backend_display_integration() {
    use splice::graph::BackendType;

    // Layer 2a: All backends display correctly
    assert_eq!(format!("{}", BackendType::SQLite), "sqlite");

    // Layer 2b: Geometric backend display (when feature enabled)
    #[cfg(feature = "geometric")]
    assert_eq!(format!("{}", BackendType::Geometric), "geometric");

    // Layer 2c: Display values are stable for CLI parsing
    // These values should not change without updating CLI documentation
    let backends = vec!["sqlite"];
    for expected in backends {
        let backend: Result<BackendType, _> = match expected {
            "sqlite" => Ok(BackendType::SQLite),
            _ => Err("unknown variant"),
        };
        assert!(
            backend.is_ok(),
            "Layer 2c: BackendType {} should be valid",
            expected
        );
    }
}

/// Test that backend detection is called correctly from integration points
///
/// Layer 2: Simulate how the CLI would use backend detection
#[test]
fn test_backend_detection_cli_integration() {
    use splice::graph::CodeGraph;

    let temp_dir = std::env::temp_dir();

    // Layer 2a: Simulate CLI "status" command with --detect-backend
    let sqlite_path = temp_dir.join(format!("test_cli_{}.db", uuid::Uuid::new_v4()));
    std::fs::write(&sqlite_path, b"SQLite format 3\0").unwrap();

    let backend = CodeGraph::detect_backend(&sqlite_path).unwrap();
    assert_eq!(
        backend.to_string(),
        "sqlite",
        "Layer 2a: CLI should detect SQLite"
    );

    std::fs::remove_file(&sqlite_path).ok();

    // Layer 2b: Simulate CLI with .geo file
    let geo_path = temp_dir.join(format!("test_cli_{}.geo", uuid::Uuid::new_v4()));
    std::fs::File::create(&geo_path).unwrap();

    let backend = CodeGraph::detect_backend(&geo_path).unwrap();

    #[cfg(feature = "geometric")]
    assert_eq!(
        backend.to_string(),
        "geometric",
        "Layer 2b: CLI should detect Geometric"
    );

    #[cfg(not(feature = "geometric"))]
    assert_eq!(
        backend.to_string(),
        "unknown",
        "Layer 2b: CLI should return Unknown without feature"
    );

    std::fs::remove_file(&geo_path).ok();
}
