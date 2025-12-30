//! Module path resolution tests.
//!
//! TDD for Phase 3: Module Path Resolution
//! - module_path → file_path index
//! - super and self reference handling
//! - absolute path resolution

use splice::resolve::module_resolver::{resolve_module_path, ModulePathIndex};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_index_returns_none() {
        let index = ModulePathIndex::new();

        let result = index.resolve("crate::foo");
        assert!(result.is_none());
    }

    #[test]
    fn test_insert_and_resolve_crate_root() {
        let mut index = ModulePathIndex::new();

        // Insert crate root → main.rs
        index.insert("crate", "/src/main.rs");

        let result = index.resolve("crate");
        assert_eq!(result, Some("/src/main.rs".to_string()));
    }

    #[test]
    fn test_insert_and_resolve_nested_module() {
        let mut index = ModulePathIndex::new();

        // Insert crate::foo → src/foo.rs
        index.insert("crate::foo", "/src/foo.rs");

        let result = index.resolve("crate::foo");
        assert_eq!(result, Some("/src/foo.rs".to_string()));
    }

    #[test]
    fn test_insert_and_resolve_deeply_nested() {
        let mut index = ModulePathIndex::new();

        // Insert crate::a::b::c → src/a/b/c.rs
        index.insert("crate::a::b::c", "/src/a/b/c.rs");

        let result = index.resolve("crate::a::b::c");
        assert_eq!(result, Some("/src/a/b/c.rs".to_string()));
    }

    #[test]
    fn test_resolve_nonexistent_returns_none() {
        let mut index = ModulePathIndex::new();

        index.insert("crate::foo", "/src/foo.rs");

        let result = index.resolve("crate::bar");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_super_from_crate_level() {
        let mut index = ModulePathIndex::new();

        // Current file is src/main.rs with module_path "crate"
        // super:: from crate level should fail (no parent)
        index.insert("crate", "/src/main.rs");

        let result = resolve_module_path(&index, "/src/main.rs", "super");
        assert!(result.is_none()); // No parent of crate
    }

    #[test]
    fn test_resolve_super_from_nested_module() {
        let mut index = ModulePathIndex::new();

        // src/main.rs has module_path "crate"
        // src/foo.rs has module_path "crate::foo"
        index.insert("crate", "/src/main.rs");
        index.insert("crate::foo", "/src/foo.rs");

        // super:: from crate::foo should resolve to crate
        let result = resolve_module_path(&index, "/src/foo.rs", "super");
        assert_eq!(result, Some("/src/main.rs".to_string()));
    }

    #[test]
    fn test_resolve_super_multiple_levels() {
        let mut index = ModulePathIndex::new();

        index.insert("crate", "/src/main.rs");
        index.insert("crate::a", "/src/a.rs");
        index.insert("crate::a::b", "/src/a/b.rs");

        // super::super:: from crate::a::b should resolve to crate
        let result = resolve_module_path(&index, "/src/a/b.rs", "super::super");
        assert_eq!(result, Some("/src/main.rs".to_string()));
    }

    #[test]
    fn test_resolve_self_returns_current_file() {
        let mut index = ModulePathIndex::new();

        index.insert("crate::foo", "/src/foo.rs");

        // self:: from crate::foo should resolve to crate::foo (itself)
        let result = resolve_module_path(&index, "/src/foo.rs", "self");
        assert_eq!(result, Some("/src/foo.rs".to_string()));
    }

    #[test]
    fn test_resolve_self_nested_symbol() {
        let mut index = ModulePathIndex::new();

        index.insert("crate::foo", "/src/foo.rs");

        // self::bar from crate::foo should resolve to crate::foo::bar
        // But we only store module paths, so this returns the module's file
        let result = resolve_module_path(&index, "/src/foo.rs", "self::bar");
        // self::bar means "bar in current module", so return current file
        assert_eq!(result, Some("/src/foo.rs".to_string()));
    }

    #[test]
    fn test_resolve_absolute_crate_path() {
        let mut index = ModulePathIndex::new();

        index.insert("crate", "/src/main.rs");
        index.insert("crate::utils", "/src/utils.rs");

        // Resolve crate::utils from anywhere
        let result = resolve_module_path(&index, "/src/main.rs", "crate::utils");
        assert_eq!(result, Some("/src/utils.rs".to_string()));
    }

    #[test]
    fn test_resolve_with_similar_prefixes() {
        let mut index = ModulePathIndex::new();

        index.insert("crate::foo", "/src/foo.rs");
        index.insert("crate::foobar", "/src/foobar.rs");

        // Should match exact path, not prefix
        let result = index.resolve("crate::foo");
        assert_eq!(result, Some("/src/foo.rs".to_string()));

        let result2 = index.resolve("crate::foobar");
        assert_eq!(result2, Some("/src/foobar.rs".to_string()));
    }

    #[test]
    fn test_index_get_current_module_path() {
        let mut index = ModulePathIndex::new();

        index.insert("crate::foo", "/src/foo.rs");

        // Get module path for a file
        let result = index.get_module_path("/src/foo.rs");
        assert_eq!(result, Some("crate::foo".to_string()));
    }

    #[test]
    fn test_index_get_current_module_path_unknown_file() {
        let index = ModulePathIndex::new();

        let result = index.get_module_path("/src/unknown.rs");
        assert!(result.is_none());
    }
}
