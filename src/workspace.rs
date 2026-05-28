//! Workspace detection using magellan's manifest parsers.
//!
//! Finds the project root by walking parent directories and checking for
//! recognized manifest files (`Cargo.toml`, `pyproject.toml`, `go.mod`,
//! `package.json`, `tsconfig.json`, `pom.xml`, `CMakeLists.txt`) via
//! magellan's `manifest` module.
//!
//! Boundary logic prevents walking into system directories (/, /tmp, $TMPDIR)
//! or past $HOME when the file is outside the home directory.

use crate::error::SpliceError;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[cfg(feature = "sqlite")]
use magellan::manifest::{
    detect_include_paths_from_root, CMakeManifest, CargoManifest, GoModuleManifest, MavenManifest,
    PackageJsonManifest, PyprojectManifest,
};

fn build_boundary_set(file_path: &Path) -> HashSet<PathBuf> {
    let mut set = HashSet::new();
    set.insert(PathBuf::from("/"));
    set.insert(PathBuf::from("/tmp"));
    if let Some(tmpdir) = std::env::var_os("TMPDIR") {
        set.insert(PathBuf::from(tmpdir));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home_path = PathBuf::from(&home);
        if !file_path.starts_with(&home_path) {
            set.insert(home_path);
        }
    }
    set
}

#[cfg(feature = "sqlite")]
const MANIFEST_FILES: &[&str] = &[
    "Cargo.toml",
    "pyproject.toml",
    "go.mod",
    "package.json",
    "tsconfig.json",
    "pom.xml",
    "CMakeLists.txt",
];

#[cfg(feature = "sqlite")]
fn has_manifest(dir: &Path) -> bool {
    MANIFEST_FILES.iter().any(|m| dir.join(m).exists())
}

/// Find the project root by walking parent directories looking for manifest files.
///
/// Uses the same marker set as magellan's manifest module (Cargo.toml,
/// pyproject.toml, go.mod, package.json, tsconfig.json, pom.xml, CMakeLists.txt).
/// Stops at boundary directories (/, /tmp, $TMPDIR, and $HOME when the file
/// is outside the home directory).
///
/// # Errors
///
/// Returns `SpliceError::Other` if no manifest is found before hitting a boundary.
#[cfg(feature = "sqlite")]
pub fn find_workspace_root(path: &Path) -> Result<PathBuf, SpliceError> {
    let absolute_path = std::fs::canonicalize(path).map_err(|e| SpliceError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    let boundaries = build_boundary_set(&absolute_path);

    let mut current = absolute_path.parent();
    while let Some(dir) = current {
        if boundaries.contains(dir) {
            break;
        }
        if has_manifest(dir) {
            return Ok(dir.to_path_buf());
        }
        current = dir.parent();
    }

    Err(SpliceError::Other(format!(
        "No project marker found in any ancestor of {} within $HOME or before /tmp",
        path.display()
    )))
}

#[cfg(not(feature = "sqlite"))]
pub fn find_workspace_root(path: &Path) -> Result<PathBuf, SpliceError> {
    let absolute_path = std::fs::canonicalize(path).map_err(|e| SpliceError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    absolute_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| SpliceError::Other("Cannot determine workspace root".to_string()))
}

/// Detect the primary project language from the manifest files at the given root.
///
/// Returns `"rust"`, `"python"`, `"go"`, `"javascript"`, `"java"`, or `"c"`,
/// or `None` if no recognized manifest is found.
#[cfg(feature = "sqlite")]
pub fn detect_project_language(root: &Path) -> Option<&'static str> {
    if let Ok(m) = CargoManifest::parse(root) {
        if m.package_name.is_some() {
            return Some("rust");
        }
    }
    if let Ok(m) = PyprojectManifest::parse(root) {
        if m.package_name.is_some() {
            return Some("python");
        }
    }
    if let Ok(m) = GoModuleManifest::parse(root) {
        if m.module_name.is_some() {
            return Some("go");
        }
    }
    if let Ok(m) = PackageJsonManifest::parse(root) {
        if m.name.is_some() {
            return Some("javascript");
        }
    }
    if let Ok(m) = MavenManifest::parse(root) {
        if m.artifact_id.is_some() {
            return Some("java");
        }
    }
    if let Ok(m) = CMakeManifest::parse(root) {
        if m.project_name.is_some() {
            return Some("c");
        }
    }
    None
}

#[cfg(not(feature = "sqlite"))]
pub fn detect_project_language(_root: &Path) -> Option<&'static str> {
    None
}

/// Detect source include paths from the project manifest at the given root.
///
/// Delegates to `magellan::manifest::detect_include_paths_from_root`.
#[cfg(feature = "sqlite")]
pub fn detect_include_paths(root: &Path) -> Vec<String> {
    detect_include_paths_from_root(root)
}

#[cfg(not(feature = "sqlite"))]
pub fn detect_include_paths(_root: &Path) -> Vec<String> {
    vec!["src/".to_string()]
}
