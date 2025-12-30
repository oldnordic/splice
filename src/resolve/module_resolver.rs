//! Module path resolution.
//!
//! This module provides module_path → file_path indexing and resolution.
//! Handles absolute paths, super/self references, and relative imports.

use std::collections::HashMap;

/// Index mapping module paths to file paths.
///
/// # Example
/// ```
/// # use splice::resolve::module_resolver::ModulePathIndex;
/// let mut index = ModulePathIndex::new();
/// index.insert("crate::foo", "/src/foo.rs");
///
/// let file = index.resolve("crate::foo");
/// assert_eq!(file, Some("/src/foo.rs".to_string()));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ModulePathIndex {
    /// Maps module_path → file_path
    /// e.g., "crate::foo::bar" → "/src/foo/bar.rs"
    module_to_file: HashMap<String, String>,

    /// Reverse maps file_path → module_path
    /// e.g., "/src/foo.rs" → "crate::foo"
    file_to_module: HashMap<String, String>,
}

impl ModulePathIndex {
    /// Create a new empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a module path → file path mapping.
    ///
    /// # Arguments
    /// * `module_path` - Rust module path (e.g., "crate::foo::bar")
    /// * `file_path` - File system path (e.g., "/src/foo/bar.rs")
    pub fn insert(&mut self, module_path: &str, file_path: &str) {
        self.module_to_file
            .insert(module_path.to_string(), file_path.to_string());
        self.file_to_module
            .insert(file_path.to_string(), module_path.to_string());
    }

    /// Resolve a module path to its file path.
    ///
    /// Returns `None` if the module path is not in the index.
    pub fn resolve(&self, module_path: &str) -> Option<String> {
        self.module_to_file.get(module_path).cloned()
    }

    /// Get the module path for a given file path.
    ///
    /// Returns `None` if the file path is not in the index.
    pub fn get_module_path(&self, file_path: &str) -> Option<String> {
        self.file_to_module.get(file_path).cloned()
    }

    /// Get the current module path for a file, resolving "self" references.
    fn get_current_module(&self, file_path: &str) -> Option<String> {
        self.file_to_module.get(file_path).cloned()
    }

    /// Get the parent module path for a given module path.
    ///
    /// # Examples
    /// - "crate::foo::bar" → "crate::foo"
    /// - "crate::foo" → "crate"
    /// - "crate" → None (no parent)
    fn get_parent_module(module_path: &str) -> Option<String> {
        module_path.rfind("::").map(|last_colon_pos| module_path[..last_colon_pos].to_string())
    }
}

/// Resolve a module path to a file path, handling relative references.
///
/// # Arguments
/// * `index` - The module path index
/// * `current_file` - The current file path (for resolving super/self)
/// * `module_path` - The module path to resolve (may contain super/self)
///
/// # Returns
/// * `Some(file_path)` - If the module can be resolved
/// * `None` - If the module cannot be found
///
/// # Examples
/// ```
/// # use splice::resolve::module_resolver::{ModulePathIndex, resolve_module_path};
/// let mut index = ModulePathIndex::new();
/// index.insert("crate", "/src/main.rs");
/// index.insert("crate::foo", "/src/foo.rs");
///
/// // Absolute path
/// assert_eq!(
///     resolve_module_path(&index, "/src/main.rs", "crate::foo"),
///     Some("/src/foo.rs".to_string())
/// );
///
/// // Super reference
/// assert_eq!(
///     resolve_module_path(&index, "/src/foo.rs", "super"),
///     Some("/src/main.rs".to_string())
/// );
/// ```
pub fn resolve_module_path(
    index: &ModulePathIndex,
    current_file: &str,
    module_path: &str,
) -> Option<String> {
    // Handle self references
    if module_path == "self" {
        return Some(current_file.to_string());
    }

    // Handle self::prefix (self::bar) - resolve in current module
    if let Some(rest) = module_path.strip_prefix("self::") {
        if rest.is_empty() {
            return Some(current_file.to_string());
        }
        // self::bar means "bar in current module"
        // We don't track symbols within modules yet, so return current file
        return Some(current_file.to_string());
    }

    // Handle super references
    if module_path == "super" {
        let current_module = index.get_current_module(current_file)?;
        let parent_module = ModulePathIndex::get_parent_module(&current_module)?;
        return index.resolve(&parent_module);
    }

    // Handle super::super::...
    if module_path.starts_with("super::") {
        let current_module = index.get_current_module(current_file)?;
        let mut target_module = current_module;
        let mut super_count = 0;

        // Count super:: prefixes
        for part in module_path.split("::") {
            if part == "super" {
                super_count += 1;
            } else {
                break;
            }
        }

        // Walk up super_count levels
        for _ in 0..super_count {
            match ModulePathIndex::get_parent_module(&target_module) {
                Some(parent) => target_module = parent,
                None => return None,
            }
        }

        // If there are remaining parts after super::, append them
        let remaining_parts: Vec<&str> = module_path.split("::").skip(super_count).collect();

        if remaining_parts.is_empty() || remaining_parts.iter().all(|s| s.is_empty()) {
            return index.resolve(&target_module);
        }

        let full_path = format!("{}::{}", target_module, remaining_parts.join("::"));
        return index.resolve(&full_path);
    }

    // Handle absolute paths (crate::, or just starting with identifier)
    index.resolve(module_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_parent_module() {
        assert_eq!(
            ModulePathIndex::get_parent_module("crate::foo::bar"),
            Some("crate::foo".to_string())
        );
        assert_eq!(
            ModulePathIndex::get_parent_module("crate::foo"),
            Some("crate".to_string())
        );
        assert_eq!(ModulePathIndex::get_parent_module("crate"), None);
    }
}
