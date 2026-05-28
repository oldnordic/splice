use super::rust::find_references_in_file;
use crate::error::{Result, SpliceError};
use crate::ingest::imports::extract_rust_imports;
use crate::ingest::rust::RustSymbol;
use ropey::Rope;
use std::path::{Path, PathBuf};

/// Represents a re-export of a symbol.
#[derive(Debug, Clone)]
struct Reexport {
    /// The module path that re-exports the symbol (e.g., "crate::mod_a")
    reexporting_module: String,
    /// The name the symbol is re-exported as (might differ with `as`)
    _reexported_name: String,
    /// The original module path being re-exported (e.g., "crate::utils")
    _replaced_module: String,
    /// The original symbol name being re-exported
    _replaced_name: String,
}

/// Build a map of all re-exports in the workspace.
///
/// Returns a map from (module_path, symbol_name) to list of re-exports.
fn build_reexport_map(
    workspace_root: &Path,
    rust_files: &[PathBuf],
) -> Result<std::collections::HashMap<(String, String), Vec<Reexport>>> {
    let mut reexport_map: std::collections::HashMap<(String, String), Vec<Reexport>> =
        std::collections::HashMap::new();

    for file_path in rust_files {
        let source = match std::fs::read(file_path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let imports = match extract_rust_imports(file_path, &source) {
            Ok(i) => i,
            Err(_) => continue,
        };

        // Get the module path of this file
        let module_path = match module_path_from_file(workspace_root, file_path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Process re-exports (imports with is_reexport = true)
        for import in imports {
            if !import.is_reexport {
                continue;
            }

            // Build the full path of the module being re-exported
            let imported_module = import.path.join("::");

            // For each re-exported name, record the re-export
            for name in &import.imported_names {
                if name == "*" {
                    // Glob re-export - everything from the imported module is re-exported
                    // We'll handle this as a special case by checking against the module path only
                    continue;
                }

                // The re-export is: `pub use <imported_module>::<name> as <name>` (or similar)
                // This creates a re-export from `module_path` of the symbol `<imported_module>::<name>`
                let reexport = Reexport {
                    reexporting_module: module_path.clone(),
                    _reexported_name: name.clone(),
                    _replaced_module: imported_module.clone(),
                    _replaced_name: name.clone(),
                };

                let key = (imported_module.clone(), name.clone());
                reexport_map.entry(key).or_default().push(reexport);
            }
        }
    }

    Ok(reexport_map)
}

/// Get the module path for a file path.
///
/// Converts /path/to/workspace/src/utils/helpers.rs to "crate::utils::helpers"
fn module_path_from_file(workspace_root: &Path, file_path: &Path) -> Result<String> {
    // Get the relative path from workspace root
    let relative = file_path
        .strip_prefix(workspace_root)
        .map_err(|_| SpliceError::Other("File not in workspace".to_string()))?;

    // Convert to string and handle src/ directory
    let path_str = relative
        .to_str()
        .ok_or_else(|| SpliceError::Other("Invalid UTF-8 in path".to_string()))?;

    // Remove .rs extension and convert slashes to ::
    let module_path = path_str
        .trim_end_matches(".rs")
        .replace("/", "::")
        .replace("\\", "::");

    // Remove mod.rs if present (Rust convention for module modules)
    let module_path = module_path.replace("::mod", "");

    // Add crate:: prefix if not present
    let module_path = if module_path.starts_with("crate::") {
        module_path
    } else if module_path.starts_with("lib::") || module_path.starts_with("src::") {
        // Remove leading lib or src, add crate
        let rest = module_path
            .split("::")
            .skip(1)
            .collect::<Vec<_>>()
            .join("::");
        format!("crate::{}", rest)
    } else {
        format!("crate::{}", module_path)
    };

    Ok(module_path)
}

/// Check if a module re-exports the target symbol.
///
/// Returns true if the given module re-exports the symbol from the original module.
fn module_reexports_symbol(
    module_path: &str,
    target_module: &str,
    target_symbol: &str,
    reexport_map: &std::collections::HashMap<(String, String), Vec<Reexport>>,
) -> bool {
    // Check if this module directly re-exports the target symbol
    let key = (target_module.to_string(), target_symbol.to_string());
    if let Some(reexports) = reexport_map.get(&key) {
        for reexport in reexports {
            if reexport.reexporting_module == module_path {
                return true;
            }
        }
    }

    false
}

/// Find cross-file references to a symbol.
///
/// This function:
/// 1. Finds the workspace root (directory containing Cargo.toml)
/// 2. Finds all .rs files in the workspace
/// 3. Builds a re-export map to track which modules re-export which symbols
/// 4. For each file, extracts imports and checks if they match the target symbol's module
///    (including modules that re-export the symbol)
/// 5. For matching files, searches for references to the symbol
///
/// # Arguments
/// * `definition_file` - Path to the file containing the symbol definition
/// * `target_symbol` - The symbol to find references for
///
/// # Returns
/// * Vector of references from other files
/// * Boolean indicating if glob imports were found (reduces confidence)
pub(crate) fn find_cross_file_references(
    definition_file: &Path,
    target_symbol: &RustSymbol,
) -> Result<(Vec<super::Reference>, bool)> {
    let mut all_references = Vec::new();
    let mut has_glob_ambiguity = false;

    // Step 1: Find workspace root
    let workspace_root = find_workspace_root(definition_file)?;

    // Step 2: Find all .rs files in workspace
    let rust_files = find_all_rust_files(&workspace_root)?;

    // Step 3: Build re-export map to track re-exported symbols
    let reexport_map = match build_reexport_map(&workspace_root, &rust_files) {
        Ok(m) => m,
        Err(e) => {
            // Log error but continue without re-export tracking
            eprintln!("Warning: failed to build re-export map: {}", e);
            std::collections::HashMap::new()
        }
    };

    // Step 4: Get the module path of the target symbol
    let target_module = &target_symbol.module_path;

    // Step 5: For each file (except the definition file), check imports and search
    for file_path in rust_files {
        // Skip the definition file (already handled in same-file search)
        if file_path == definition_file {
            continue;
        }

        // Read source
        let source = match std::fs::read(&file_path) {
            Ok(s) => s,
            Err(_) => continue, // Skip files we can't read
        };

        // Extract imports from this file
        let imports = match extract_rust_imports(&file_path, &source) {
            Ok(i) => i,
            Err(_) => continue, // Skip files that fail to parse
        };

        // Check if any import matches the target module directly
        let (matches, has_glob) =
            import_matches_module(&imports, target_module, &target_symbol.name);

        // Also check if any import is from a module that re-exports the target symbol
        let matches_reexport =
            check_reexport_matches(&imports, target_module, &target_symbol.name, &reexport_map);

        if has_glob {
            has_glob_ambiguity = true;
        }

        if matches || matches_reexport {
            // This file imports from the target module (or a re-exporting module), search for references
            let rope = Rope::from_str(std::str::from_utf8(&source)?);
            let refs = find_references_in_file(&source, &rope, target_symbol, &file_path)?;
            all_references.extend(refs);
        }
    }

    Ok((all_references, has_glob_ambiguity))
}

/// Check if any import is from a module that re-exports the target symbol.
fn check_reexport_matches(
    imports: &[crate::ingest::imports::ImportFact],
    target_module: &str,
    target_symbol: &str,
    reexport_map: &std::collections::HashMap<(String, String), Vec<Reexport>>,
) -> bool {
    for import in imports {
        let imported_module = import.path.join("::");

        // For each name imported, check if it's a re-export of our target symbol
        for name in &import.imported_names {
            if name == "*" {
                // Glob import - check if this module re-exports the target
                if module_reexports_symbol(
                    &imported_module,
                    target_module,
                    target_symbol,
                    reexport_map,
                ) {
                    return true;
                }
            } else if name == target_symbol {
                // Check if this is a re-export of our target symbol
                if module_reexports_symbol(
                    &imported_module,
                    target_module,
                    target_symbol,
                    reexport_map,
                ) {
                    return true;
                }
            }
        }
    }
    false
}

/// Find the workspace root by searching upward for a project marker.
///
/// Walks parent directories looking for any of `Cargo.toml`, `pyproject.toml`,
/// `package.json`, `go.mod`, `pom.xml`, `build.gradle`, or `setup.py`. Stops
/// walking before entering boundary directories (`/`, `/tmp`, `$TMPDIR`, or
/// `$HOME` when the file is outside `$HOME`) to avoid picking a stray
/// ancestor manifest above the working directory as the workspace root.
pub(crate) fn find_workspace_root(start_path: &Path) -> Result<PathBuf> {
    const MARKERS: &[&str] = &[
        "Cargo.toml",
        "pyproject.toml",
        "package.json",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "setup.py",
    ];

    let boundaries = build_workspace_boundary_set(start_path);

    let mut current = start_path.parent();
    while let Some(dir) = current {
        if boundaries.contains(dir) {
            break;
        }
        for marker in MARKERS {
            if dir.join(marker).exists() {
                return Ok(dir.to_path_buf());
            }
        }
        current = dir.parent();
    }

    Err(SpliceError::Other(format!(
        "No project marker (Cargo.toml, pyproject.toml, package.json, go.mod, pom.xml, build.gradle, setup.py) found in any ancestor of {} within $HOME or before /tmp",
        start_path.display()
    )))
}

/// Build the set of directories that bound the upward search for a project
/// marker. We never look INTO these directories as workspace candidates.
fn build_workspace_boundary_set(file_path: &Path) -> std::collections::HashSet<PathBuf> {
    let mut set = std::collections::HashSet::new();
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

/// Find all .rs files in the workspace directory.
///
/// Excludes common build/output directories:
/// - target/
/// - .git/
/// - Any directory starting with "."
fn find_all_rust_files(workspace_root: &Path) -> Result<Vec<PathBuf>> {
    let mut rust_files = Vec::new();

    fn visit_dirs(dir: &Path, rust_files: &mut Vec<PathBuf>) -> Result<()> {
        // Skip certain directories
        if dir
            .file_name()
            .map(|n| n.to_str().unwrap_or(""))
            .unwrap_or("")
            == "target"
        {
            return Ok(());
        }
        if dir
            .file_name()
            .map(|n| n.to_str().unwrap_or(""))
            .unwrap_or("")
            == ".git"
        {
            return Ok(());
        }
        // Skip hidden directories
        if dir
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
        {
            return Ok(());
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(()), // Skip directories we can't read
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();

            if path.is_dir() {
                visit_dirs(&path, rust_files)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                rust_files.push(path);
            }
        }
        Ok(())
    }

    visit_dirs(workspace_root, &mut rust_files)?;
    Ok(rust_files)
}

/// Check if any import in the list matches the target module and symbol.
///
/// Returns (matches, has_glob) where:
/// - matches: true if any import could reference the target symbol
/// - has_glob: true if a glob import was found (reduces confidence)
fn import_matches_module(
    imports: &[crate::ingest::imports::ImportFact],
    target_module: &str,
    target_symbol_name: &str,
) -> (bool, bool) {
    let mut matches = false;
    let mut has_glob = false;

    for import in imports {
        if import.is_glob {
            has_glob = true;
            // Check if glob import matches the module
            let import_path = import.path.join("::");
            if import_path_matches_target(&import_path, target_module) {
                matches = true;
            }
        } else {
            // Check if any imported name matches our target symbol
            if import
                .imported_names
                .contains(&target_symbol_name.to_string())
            {
                let import_path = import.path.join("::");
                if import_path_matches_target(&import_path, target_module) {
                    matches = true;
                }
            }
        }
    }

    (matches, has_glob)
}

/// Check if an import path matches the target module.
///
/// The import path matches if:
/// - It exactly equals the target module
/// - It's a parent of the target module (e.g., "crate::utils" matches "crate::utils::helpers")
pub(crate) fn import_path_matches_target(import_path: &str, target_module: &str) -> bool {
    // Direct match
    if import_path == target_module {
        return true;
    }

    // Import is a parent module
    // e.g., import "crate::utils" matches target "crate::utils::helper"
    if target_module.starts_with(&format!("{}::", import_path)) {
        return true;
    }

    // Target is a parent module
    // e.g., import "crate::utils::helper" matches target "crate::utils"
    // (This happens when importing a specific symbol from the module)
    if import_path.starts_with(&format!("{}::", target_module)) {
        return true;
    }

    false
}
