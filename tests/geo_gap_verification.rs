//! VERIFICATION OF EXACT REMAINING GAP
//!
//! This test documents the specific gap preventing .geo workflow completion.

/// The gap: magellan::CodeGraph::open() cannot open .geo files
///
/// ERROR: DB_COMPAT: not a sqlite database: ./code.geo
///
/// Root cause: magellan's CodeGraph::open() runs db_compat::preflight_sqlitegraph_compat()
/// which expects SQLite format. .geo files use GEODB binary format.
#[test]
#[cfg(feature = "geometric")]
fn verify_exact_gap_magellan_cannot_open_geo() {
    use magellan::CodeGraph as MagellanGraph;

    let geo_path = Path::new("./code.geo");
    if !geo_path.exists() {
        eprintln!("SKIP: ./code.geo not found");
        return;
    }

    // This is what MagellanIntegration::open() calls internally
    let result = MagellanGraph::open(geo_path);

    // EXPECTED FAILURE: magellan::CodeGraph doesn't support .geo format
    assert!(
        result.is_err(),
        "CONFIRMED: magellan::CodeGraph cannot open .geo files (expected)"
    );

    let err = result.err().unwrap();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("DB_COMPAT") || err_msg.contains("not a sqlite database"),
        "Error should be DB_COMPAT/sqlite mismatch: {}",
        err_msg
    );

    eprintln!("CONFIRMED GAP: {}", err_msg);
}

/// Verify GeometricBackend CAN open .geo files (if accessible)
#[test]
#[cfg(feature = "geometric")]
fn verify_geometric_backend_can_open_geo() {
    use magellan::graph::geometric_backend::GeometricBackend;

    let geo_path = Path::new("./code.geo");
    if !geo_path.exists() {
        eprintln!("SKIP: ./code.geo not found");
        return;
    }

    // GeometricBackend::open() is the correct API for .geo files
    let result = GeometricBackend::open(geo_path);

    // This SHOULD succeed - GeometricBackend understands GEODB format
    assert!(
        result.is_ok(),
        "GeometricBackend should open .geo files, got: {:?}",
        result.err()
    );

    eprintln!("CONFIRMED: GeometricBackend CAN open .geo files");
}
