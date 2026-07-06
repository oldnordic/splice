//! Database migration helpers for Magellan schema upgrades.
//!
//! Splice delegates actual migrations to `magellan::CodeGraph::open()`, then
//! reports the real schema versions before and after opening.

use crate::error::{Result, SpliceError};
use magellan::CodeGraph as MagellanGraph;
use magellan::MAGELLAN_SCHEMA_VERSION;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Result of a database migration operation.
#[derive(Debug, Clone)]
pub struct MigrationResult {
    /// Previous schema version before migration.
    pub previous_version: i64,
    /// New schema version after migration.
    pub new_version: i64,
    /// Path to the backup file (if created).
    pub backup_path: Option<PathBuf>,
    /// Number of symbols migrated (approximate).
    pub symbols_migrated: usize,
}

/// Check the current schema version of a Magellan database.
///
/// Reads the `magellan_meta` table to get the current schema version.
/// Returns an error if the database doesn't exist or has no metadata.
///
/// # Arguments
/// * `db_path` - Path to the Magellan database file
///
/// # Returns
/// * `Ok(i64)` - The current schema version
/// * `Err(SpliceError)` - If the database can't be read or has no metadata
pub fn check_schema_version(db_path: &Path) -> Result<i64> {
    if !db_path.exists() {
        return Err(SpliceError::Other(format!(
            "Database not found: {}",
            db_path.display()
        )));
    }

    read_schema_version(db_path)
}

/// Migrate a Magellan database to the latest schema version.
///
/// This function:
/// 1. Creates a backup if `backup` is true (default for safety)
/// 2. Opens the database (which triggers Magellan's auto-migration)
/// 3. Returns migration result with version info and stats
///
/// # Arguments
/// * `db_path` - Path to the Magellan database file
/// * `backup` - Whether to create a backup before migrating (recommended: true)
/// * `dry_run` - If true, only checks version without migrating
///
/// # Returns
/// * `Ok(MigrationResult)` - Migration outcome with version and stats
/// * `Err(SpliceError)` - If migration or backup fails
pub fn migrate_database(db_path: &Path, backup: bool, dry_run: bool) -> Result<MigrationResult> {
    if !db_path.exists() {
        return Err(SpliceError::Other(format!(
            "Database not found: {}",
            db_path.display()
        )));
    }

    let previous_version = check_schema_version(db_path)?;

    if dry_run {
        return Ok(MigrationResult {
            previous_version,
            new_version: MAGELLAN_SCHEMA_VERSION,
            backup_path: None,
            symbols_migrated: 0,
        });
    }

    // Create backup if requested
    let backup_path = if backup {
        Some(create_backup(db_path, previous_version)?)
    } else {
        None
    };

    // Open the database - this triggers auto-migration
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", db_path)))?;

    let _graph = MagellanGraph::open(db_path_str).map_err(|e| SpliceError::Magellan {
        context: format!("Failed to open database at {}", db_path_str),
        source: e,
    })?;

    let new_version = check_schema_version(db_path)?;
    let symbols_migrated = 0;

    Ok(MigrationResult {
        previous_version,
        new_version,
        backup_path,
        symbols_migrated,
    })
}

/// Create a timestamped backup of a database file.
///
/// Creates a timestamped backup file with the format:
/// `<db_path>.backup.v<schema_version>.<YYYYmmdd_HHMMSS>`
///
/// # Arguments
/// * `db_path` - Path to the database file to backup
///
/// # Returns
/// * `Ok(PathBuf)` - Path to the created backup file
/// * `Err(SpliceError)` - If backup creation fails
pub fn create_backup(db_path: &Path, schema_version: i64) -> Result<PathBuf> {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let file_name = db_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", db_path)))?;
    let backup_path =
        db_path.with_file_name(format!("{file_name}.backup.v{schema_version}.{timestamp}"));

    // Copy the database file
    std::fs::copy(db_path, &backup_path).map_err(|e| SpliceError::Io {
        path: backup_path.clone(),
        source: e,
    })?;

    Ok(backup_path)
}

fn read_schema_version(db_path: &Path) -> Result<i64> {
    let conn = Connection::open(db_path).map_err(|e| {
        SpliceError::Other(format!(
            "Failed to open database {}: {}",
            db_path.display(),
            e
        ))
    })?;

    conn.query_row(
        "SELECT magellan_schema_version FROM magellan_meta WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .map_err(|e| {
        SpliceError::Other(format!(
            "Failed to read magellan schema version from {}: {}",
            db_path.display(),
            e
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_check_schema_version_nonexistent() {
        let temp_db = std::env::temp_dir().join("nonexistent_test_db.db");
        let _ = fs::remove_file(&temp_db);

        let result = check_schema_version(&temp_db);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_backup() {
        let temp_dir = std::env::temp_dir();
        let test_db = temp_dir.join("test_backup.db");

        // Clean up any existing files
        let _ = fs::remove_file(&test_db);

        // Create a test database file
        fs::write(&test_db, b"test data").unwrap();

        // Create backup
        let result = create_backup(&test_db, 20);
        assert!(result.is_ok());
        let backup_db = result.unwrap();

        // Verify backup exists
        assert!(backup_db.exists());
        assert!(
            backup_db
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap()
                .starts_with("test_backup.db.backup.v20."),
            "unexpected backup name: {}",
            backup_db.display()
        );
        assert_eq!(fs::read_to_string(&backup_db).unwrap(), "test data");

        // Clean up
        let _ = fs::remove_file(&test_db);
        let _ = fs::remove_file(&backup_db);
    }

    #[test]
    fn test_migrate_database_dry_run() {
        let temp_db = std::env::temp_dir().join("test_dry_run.db");
        let _ = fs::remove_file(&temp_db);

        let conn = Connection::open(&temp_db).unwrap();
        conn.execute(
            "CREATE TABLE magellan_meta (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                magellan_schema_version INTEGER NOT NULL,
                sqlitegraph_schema_version INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO magellan_meta (id, magellan_schema_version, sqlitegraph_schema_version, created_at)
             VALUES (1, 19, 0, 0)",
            [],
        )
        .unwrap();
        drop(conn);

        // Test dry-run mode
        let result = migrate_database(&temp_db, false, true);
        assert!(result.is_ok());

        let migration_result = result.unwrap();
        assert_eq!(migration_result.previous_version, 19);
        assert_eq!(migration_result.new_version, MAGELLAN_SCHEMA_VERSION);
        assert!(migration_result.backup_path.is_none());
        assert_eq!(migration_result.symbols_migrated, 0);

        // Clean up
        let _ = fs::remove_file(&temp_db);
    }
}
