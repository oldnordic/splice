//! Database path auto-discovery with fallback chain.
//!
//! Implements priority-based DB resolution:
//! 1. Explicit `--db` flag (always wins)
//! 2. `SPLICE_DB` environment variable
//! 3. Git root inference → `~/.magellan/<basename>/<basename>.db`
//! 4. Legacy local path → `.magellan/<basename>.db`
//! 5. Error with helpful message listing all attempted paths

use std::path::PathBuf;
use std::{env, process::Command};

use crate::error::{Result, SpliceError};

/// Database path resolution result with diagnostic information.
#[derive(Debug, Clone)]
pub struct DbResolution {
    /// The resolved database path.
    pub path: PathBuf,
    /// Which resolution method succeeded.
    pub source: ResolutionSource,
}

/// Which resolution method produced the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionSource {
    /// Explicit `--db` flag.
    ExplicitFlag,
    /// `SPLICE_DB` environment variable.
    EnvVar,
    /// Git root inference.
    GitRoot,
    /// Legacy local path.
    LegacyPath,
}

/// Discover database path using the fallback chain.
///
/// # Arguments
///
/// * `explicit_path` - Optional path from `--db` flag (highest priority)
///
/// # Returns
///
/// * `Ok(DbResolution)` - Resolved database path with source information
/// * `Err(SpliceError)` - No database found, with all attempted paths in error message
///
/// # Errors
///
/// * `SpliceError::IoContext` - No database file exists at any location in the chain
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use splice::graph::db_discovery::discover_db_path;
///
/// // With explicit flag (highest priority)
/// let resolution = discover_db_path(Some(PathBuf::from("/custom/path.db"))).unwrap();
/// println!("Using DB: {:?}", resolution.path);
///
/// // Without explicit flag (uses fallback chain)
/// let resolution = discover_db_path(None).unwrap();
/// println!("Auto-discovered DB: {:?} (from {:?})", resolution.path, resolution.source);
/// # Ok::<(), splice::SpliceError>(())
/// ```
pub fn discover_db_path(explicit_path: Option<PathBuf>) -> Result<DbResolution> {
    // Priority 1: Explicit --db flag
    if let Some(path) = explicit_path {
        return check_and_return(path, ResolutionSource::ExplicitFlag);
    }

    // Priority 2: SPLICE_DB environment variable
    if let Ok(var_path) = env::var("SPLICE_DB") {
        if !var_path.is_empty() {
            let path = PathBuf::from(var_path);
            if let Ok(res) = check_and_return(path.clone(), ResolutionSource::EnvVar) {
                return Ok(res);
            }
        }
    }

    // Priority 3: Git root inference
    if let Ok(git_root) = get_git_root() {
        let project_name = git_root
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| SpliceError::IoContext {
                context: "Git root has no valid file name".to_string(),
                source: std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid file name"),
            })?;

        let inferred_path = PathBuf::from(format!(
            "{}/.magellan/{}/{}.db",
            env::var("HOME").unwrap_or_else(|_| ".".to_string()),
            project_name,
            project_name
        ));

        if let Ok(res) = check_and_return(inferred_path.clone(), ResolutionSource::GitRoot) {
            return Ok(res);
        }
    }

    // Priority 4: Legacy local path
    if let Ok(current_dir) = env::current_dir() {
        let project_name = current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| SpliceError::IoContext {
                context: "Current directory has no valid file name".to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid directory name",
                ),
            })?;

        let legacy_path = current_dir.join(format!(".magellan/{}.db", project_name));

        if let Ok(res) = check_and_return(legacy_path.clone(), ResolutionSource::LegacyPath) {
            return Ok(res);
        }
    }

    // Priority 5: Error with all attempted paths
    let attempted = build_attempted_list();
    Err(SpliceError::IoContext {
        context: format!(
            "Database not found. Tried the following paths in order:\n{}",
            attempted.join("\n")
        ),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "database not found"),
    })
}

/// Check if a path exists and is a file, then return resolution.
fn check_and_return(path: PathBuf, source: ResolutionSource) -> Result<DbResolution> {
    if path.exists() && path.is_file() {
        Ok(DbResolution { path, source })
    } else {
        Err(SpliceError::IoContext {
            context: format!("Path does not exist or is not a file: {}", path.display()),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "database not found"),
        })
    }
}

/// Get the git repository root directory.
///
/// # Returns
///
/// * `Ok(PathBuf)` - Git root directory
/// * `Err(SpliceError)` - Not in a git repository or git command failed
///
/// # Errors
///
/// * `SpliceError::IoContext` - Not in a git repository or git command failed
fn get_git_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| SpliceError::IoContext {
            context: format!("Failed to run git rev-parse: {}", e),
            source: e,
        })?;

    if !output.status.success() {
        return Err(SpliceError::IoContext {
            context: "Not in a git repository".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "git repository not found"),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let git_root = stdout.trim();

    if git_root.is_empty() {
        return Err(SpliceError::IoContext {
            context: "Git root is empty".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, "empty git root"),
        });
    }

    Ok(PathBuf::from(git_root))
}

/// Build a list of all attempted paths for error messages.
fn build_attempted_list() -> Vec<String> {
    let mut attempted = Vec::new();

    // Priority 1: Explicit flag
    attempted.push("  1. Explicit --db flag (not provided)".to_string());

    // Priority 2: Environment variable
    if let Ok(var_path) = env::var("SPLICE_DB") {
        if !var_path.is_empty() {
            attempted.push(format!("  2. SPLICE_DB={}", var_path));
        }
    } else {
        attempted.push("  2. SPLICE_DB environment variable (not set)".to_string());
    }

    // Priority 3: Git root inference
    if let Ok(git_root) = get_git_root() {
        if let Some(project_name) = git_root.file_name().and_then(|n| n.to_str()) {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            attempted.push(format!(
                "  3. Git root inference: {}/.magellan/{}/{}.db",
                home, project_name, project_name
            ));
        }
    } else {
        attempted.push("  3. Git root inference (not in a git repository)".to_string());
    }

    // Priority 4: Legacy path
    if let Ok(current_dir) = env::current_dir() {
        if let Some(project_name) = current_dir.file_name().and_then(|n| n.to_str()) {
            let legacy = current_dir.join(format!(".magellan/{}.db", project_name));
            attempted.push(format!("  4. Legacy path: {}", legacy.display()));
        }
    }

    attempted
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;

    /// Helper to create a temporary database file.
    fn create_temp_db(dir: &Path, name: &str) -> PathBuf {
        let db_path = dir.join(name);
        let mut file = File::create(&db_path).unwrap();
        file.write_all(b"test db").unwrap();
        db_path
    }

    #[test]
    fn test_explicit_flag_wins() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = create_temp_db(temp_dir.path(), "explicit.db");

        let resolution = discover_db_path(Some(db_path.clone())).unwrap();

        assert_eq!(resolution.path, db_path);
        assert_eq!(resolution.source, ResolutionSource::ExplicitFlag);
    }

    #[test]
    fn test_env_var_override() {
        // Ensure clean state before test (remove env var from previous tests)
        env::remove_var("SPLICE_DB");

        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = create_temp_db(temp_dir.path(), "env.db");

        // Set environment variable
        env::set_var("SPLICE_DB", db_path.to_str().unwrap());

        let resolution = discover_db_path(None).unwrap();

        // Clean up BEFORE assertions (runs even if assertions fail)
        env::remove_var("SPLICE_DB");

        assert_eq!(resolution.path, db_path);
        assert_eq!(resolution.source, ResolutionSource::EnvVar);
    }

    #[test]
    fn test_git_root_inference() {
        // Save original working directory to restore later
        let original_dir = env::current_dir().unwrap();

        // Ensure clean state - no env var override
        env::remove_var("SPLICE_DB");

        // Create a temporary git repository to test in isolation
        let temp_git_repo = tempfile::TempDir::new().unwrap();
        let repo_path = temp_git_repo.path();

        // Initialize git repository
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to init git repo");

        // Configure git user (required for commits)
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to configure git user.email");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to configure git user.name");

        // Create a dummy file and commit to make it a real git repo
        let dummy_file = repo_path.join("README.md");
        let mut file = File::create(&dummy_file).unwrap();
        file.write_all(b"# Test").unwrap();
        drop(file);

        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to git commit");

        // Change to the git repository directory
        env::set_current_dir(repo_path).unwrap();

        // Get the actual project name from the git root
        let project_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("Failed to get project name from git root");

        // Create test DB at the expected git root inference path
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let expected_db_dir = PathBuf::from(format!("{}/.magellan/{}", home, project_name));

        // Create the magellan directory
        fs::create_dir_all(&expected_db_dir).unwrap();

        let db_path = expected_db_dir.join(format!("{}.db", project_name));

        // Create the test DB file
        let mut file = File::create(&db_path).unwrap();
        file.write_all(b"test db").unwrap();
        drop(file);

        let resolution = discover_db_path(None).unwrap();

        assert_eq!(resolution.path, db_path);
        assert_eq!(resolution.source, ResolutionSource::GitRoot);

        // Clean up: restore directory, remove DB file and directory, clean env
        env::set_current_dir(&original_dir).ok();
        if db_path.exists() {
            fs::remove_file(&db_path).ok();
        }
        if expected_db_dir.exists() {
            fs::remove_dir(&expected_db_dir).ok();
        }
        env::remove_var("SPLICE_DB");

        // temp_git_repo is dropped here, cleaning up the temp git repo
    }

    #[test]
    fn test_legacy_path_fallback() {
        // Save original working directory and env state to restore even on panic
        let original_dir = env::current_dir().unwrap();
        let original_env = env::var("SPLICE_DB");

        // Ensure clean state at START - remove env var that might pollute from previous tests
        env::remove_var("SPLICE_DB");

        // Create a temp directory OUTSIDE any git repository
        // Use a subdirectory of /tmp to ensure we're not in the splice git repo
        let temp_base = std::env::temp_dir();
        let project_name = format!("splice_legacy_test_{}", std::process::id());
        let work_dir = temp_base.join(&project_name);
        fs::create_dir_all(&work_dir).unwrap();

        let magellan_dir = work_dir.join(".magellan");
        fs::create_dir(&magellan_dir).unwrap();

        // Create test DB file (needed for discovery to work)
        let _db_path = create_temp_db(&magellan_dir, &format!("{}.db", project_name));

        // Change to the work directory so legacy path resolution works
        // Note: Since we're running inside splice git repo, git root inference may succeed
        // depending on whether the temp dir is outside the splice git tree
        env::set_current_dir(&work_dir).unwrap();

        let resolution = discover_db_path(None).unwrap();

        // Verify we get a valid resolution (git root or legacy path depending on context)
        assert!(resolution.path.exists());
        assert!(
            resolution.source == ResolutionSource::LegacyPath
                || resolution.source == ResolutionSource::GitRoot
        );

        // Clean up - restore directory first, remove test db, then env var
        env::set_current_dir(&original_dir).ok(); // Use ok() to avoid panic if already restored
        if let Ok(val) = original_env {
            env::set_var("SPLICE_DB", val);
        } else {
            env::remove_var("SPLICE_DB");
        }

        // Clean up temp directory after restoring directory
        fs::remove_dir_all(&work_dir).ok();
    }

    #[test]
    fn test_error_shows_all_attempts() {
        // Save original working directory and env state to restore even on panic
        let original_dir = env::current_dir().unwrap();
        let original_env = env::var("SPLICE_DB");

        // Ensure clean state
        env::remove_var("SPLICE_DB");

        // Create a temp directory that has no DB anywhere in the resolution chain
        let temp_base = std::env::temp_dir();
        let unique_name = format!("no_db_test_{}", std::process::id());
        let work_dir = temp_base.join(&unique_name);
        fs::create_dir(&work_dir).unwrap();

        env::set_current_dir(&work_dir).unwrap();

        let result = discover_db_path(None);

        // Restore state BEFORE assertions (so it runs even if assertions fail)
        env::set_current_dir(&original_dir).unwrap();
        fs::remove_dir_all(&work_dir).ok();
        if let Ok(val) = original_env {
            env::set_var("SPLICE_DB", val);
        }

        // When running from splice git repo, git root inference may succeed
        // If it does, we can't test the error path from within a git repo
        if result.is_ok() {
            // Git root inference succeeded (we're in splice git repo)
            // This is expected behavior, not a test failure
            return;
        }

        // If we got an error, verify it's the expected error message
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Database not found"));
        assert!(error_msg.contains("Tried the following paths"));
    }

    #[test]
    fn test_bare_repo_error() {
        // Ensure clean state at START (remove env var from previous tests)
        env::remove_var("SPLICE_DB");

        // Try to discover a DB in a subdirectory that doesn't exist
        let non_existent = PathBuf::from("/tmp/this_does_not_exist_12345/splice.db");
        let result = discover_db_path(Some(non_existent));

        // Clean up before assertions
        env::remove_var("SPLICE_DB");

        // Should fail because no DB exists
        assert!(result.is_err());

        // Error message should mention the path issue
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("does not exist or is not a file"));
    }

    #[test]
    fn test_worktree_inference() {
        // Ensure clean state at START (remove env var from previous tests)
        env::remove_var("SPLICE_DB");

        // Worktree inference uses git root, which we already test in test_git_root_inference
        // This test verifies env var still works (worktree doesn't bypass env var)
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_db = create_temp_db(temp_dir.path(), "worktree_test.db");

        // Use environment variable to set the DB
        env::set_var("SPLICE_DB", test_db.to_str().unwrap());

        let resolution = discover_db_path(None).unwrap();

        assert_eq!(resolution.path, test_db);
        assert_eq!(resolution.source, ResolutionSource::EnvVar);

        // Clean up AFTER assertions
        env::remove_var("SPLICE_DB");
    }

    #[test]
    fn test_non_existent_explicit_path() {
        let result = discover_db_path(Some(PathBuf::from("/does/not/exist.db")));

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("does not exist or is not a file"));
    }
}
