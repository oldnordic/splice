use crate::error::{Result, SpliceError};
use crate::io_ext;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Clone workspace to a temporary directory for preview operations.
///
/// Creates a temporary directory and recursively copies the workspace.
/// Also copies any local path dependencies (sibling directories referenced
/// in Cargo.toml) to ensure cargo check works in the preview environment.
///
/// If copying fails, the temp directory is automatically cleaned up by Drop.
///
/// # Returns
///
/// Returns `Ok(TempDir)` which will be cleaned up when dropped.
pub(crate) fn clone_workspace_for_preview(workspace_root: &Path) -> Result<TempDir> {
    let preview_dir = TempDir::new().map_err(|source| SpliceError::Io {
        path: std::env::temp_dir(),
        source,
    })?;
    let preview_path = preview_dir.path();

    // First, copy the workspace itself
    // Note: If copy_dir_recursive fails, preview_dir is dropped here
    // and automatically cleans up the temp directory
    copy_dir_recursive(workspace_root, preview_path)?;

    // Handle local path dependencies from Cargo.toml
    // Projects with dependencies like `llmgrep = { path = "../llmgrep" }`
    // need those sibling directories copied to the preview workspace parent
    if let Ok(local_deps) = extract_local_path_dependencies(workspace_root) {
        let preview_parent = preview_path.parent().unwrap_or(preview_path);

        for dep_path in local_deps {
            // For nested paths like ../sqlitegraph/sqlitegraph, we need to copy
            // the parent directory (sqlitegraph) so the relative path resolves.
            let (source_path, target_name) = if let Some(dep_parent) = dep_path.parent() {
                // Check if this is a nested path (grandparent is workspace's parent)
                if let Some(grandparent) = dep_parent.parent() {
                    if let Some(workspace_parent) = workspace_root.parent() {
                        if grandparent == workspace_parent {
                            // Nested path: use parent directory as source
                            let parent_name = dep_parent
                                .file_name()
                                .and_then(|n| n.to_str())
                                .ok_or_else(|| {
                                    SpliceError::Other(format!(
                                        "Invalid dependency parent path: {:?}",
                                        dep_parent
                                    ))
                                })?;
                            (dep_parent.to_path_buf(), parent_name.to_string())
                        } else {
                            // Normal path: use dep_path as-is
                            let dep_name = dep_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .ok_or_else(|| {
                                    SpliceError::Other(format!(
                                        "Invalid dependency path: {:?}",
                                        dep_path
                                    ))
                                })?;
                            (dep_path.clone(), dep_name.to_string())
                        }
                    } else {
                        // Can't determine workspace parent, use dep_path as-is
                        let dep_name =
                            dep_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .ok_or_else(|| {
                                    SpliceError::Other(format!(
                                        "Invalid dependency path: {:?}",
                                        dep_path
                                    ))
                                })?;
                        (dep_path.clone(), dep_name.to_string())
                    }
                } else {
                    // No grandparent, use dep_path as-is
                    let dep_name =
                        dep_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .ok_or_else(|| {
                                SpliceError::Other(format!(
                                    "Invalid dependency path: {:?}",
                                    dep_path
                                ))
                            })?;
                    (dep_path.clone(), dep_name.to_string())
                }
            } else {
                // No parent, use dep_path as-is
                let dep_name = dep_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| {
                        SpliceError::Other(format!("Invalid dependency path: {:?}", dep_path))
                    })?;
                (dep_path.clone(), dep_name.to_string())
            };

            let dep_dest = preview_parent.join(&target_name);

            // Skip if already exists or is the same as workspace
            if dep_dest.exists() || source_path == workspace_root {
                continue;
            }

            // Copy the dependency directory
            if let Err(e) = copy_dir_recursive(&source_path, &dep_dest) {
                log::warn!(
                    "Failed to copy local dependency {:?} to {:?}: {}",
                    source_path,
                    dep_dest,
                    e
                );
                // Non-fatal: preview may still work if dependency isn't used
            }
        }
    }

    Ok(preview_dir)
}

/// Extract local path dependencies from Cargo.toml.
///
/// Returns paths to sibling directories that are local dependencies,
/// e.g., `../llmgrep` from `llmgrep = { path = "../llmgrep" }`.
fn extract_local_path_dependencies(workspace_root: &Path) -> Result<Vec<PathBuf>> {
    let cargo_toml_path = workspace_root.join("Cargo.toml");
    let cargo_content = io_ext::read_to_string(&cargo_toml_path)?;

    let mut local_deps = Vec::new();
    let mut seen_deps = HashSet::new();

    // Simple string-based parsing for path dependencies
    // Match patterns like: `dep_name = { path = "../something" }`
    for line in cargo_content.lines() {
        let line = line.trim();
        // Look for inline table dependencies with path
        if line.contains("{") && line.contains("path") {
            // Extract the path value
            if let Some(start) = line.find("path = \"") {
                let start_idx = start + 8; // "path = \"".len()
                if let Some(end) = line[start_idx..].find('"') {
                    let rel_path = &line[start_idx..start_idx + end];
                    if rel_path.starts_with("..") {
                        let dep_path = workspace_root.join(rel_path);
                        if dep_path.exists() {
                            // Get the canonical path and track what we've seen
                            if let Ok(canonical) = dep_path.canonicalize() {
                                if !seen_deps.contains(&canonical) {
                                    seen_deps.insert(canonical.clone());
                                    local_deps.push(canonical);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Also check for workspace members and their dependencies
    if let Some(parent) = workspace_root.parent() {
        // Check for a workspace Cargo.toml in parent directory
        let workspace_cargo = parent.join("Cargo.toml");
        if workspace_cargo.exists() {
            if let Ok(ws_content) = fs::read_to_string(&workspace_cargo) {
                // Find workspace members
                if let Some(start) = ws_content.find("members = [") {
                    let members_start = start + 11;
                    if let Some(end) = ws_content[members_start..].find(']') {
                        let members_str = &ws_content[members_start..members_start + end];
                        for member in members_str.split(',') {
                            let member = member.trim().trim_matches('"').trim_matches('\'');
                            let member_path = parent.join(member);
                            if member_path.exists() && member_path != workspace_root {
                                if let Ok(canonical) = member_path.canonicalize() {
                                    if !seen_deps.contains(&canonical) {
                                        seen_deps.insert(canonical.clone());
                                        local_deps.push(canonical);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(local_deps)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).map_err(|source| SpliceError::Io {
        path: dst.to_path_buf(),
        source,
    })?;

    let read_dir = fs::read_dir(src).map_err(|source| SpliceError::Io {
        path: src.to_path_buf(),
        source,
    })?;
    for entry in read_dir {
        let entry = entry.map_err(|source| SpliceError::Io {
            path: src.to_path_buf(),
            source,
        })?;
        if should_skip_entry(&entry.file_name()) {
            continue;
        }

        let entry_path = entry.path();
        let dest = dst.join(entry.file_name());
        let file_type = entry.file_type().map_err(|source| SpliceError::Io {
            path: entry_path.clone(),
            source,
        })?;

        if file_type.is_dir() {
            copy_dir_recursive(&entry_path, &dest)?;
        } else if file_type.is_file() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|source| SpliceError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::copy(&entry_path, &dest).map_err(|source| SpliceError::Io {
                path: entry_path.clone(),
                source,
            })?;
        }
    }

    Ok(())
}

pub(crate) fn should_skip_entry(name: &OsStr) -> bool {
    matches!(
        name.to_string_lossy().as_ref(),
        ".git"
            | ".splice-backup"
            | "target"
            | "node_modules"
            | "dist"
            | "build"
            | "__pycache__"
            | ".venv"
            | "venv"
            | ".pytest_cache"
            | ".mypy_cache"
            | ".tox"
            | ".next"
            | ".nuxt"
            | ".cache"
            | ".gradle"
            | ".idea"
            | ".vscode"
            | ".splice_graph.db"
            | ".splice_graph.db-shm"
            | ".splice_graph.db-wal"
            | "codegraph.db"
            | "magellan.db"
            | "operations.db"
            | "splice_map.db"
            | "syncore_code_graph.db"
            | "syncore_code_graph.db-shm"
            | "syncore_code_graph.db-wal"
    )
}
