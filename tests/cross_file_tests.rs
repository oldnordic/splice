//! Cross-file symbol resolution tests.
//!
//! TDD for Phase 4: Cross-File Symbol Resolution
//! - Check local symbols first
//! - Query import edges from current file
//! - Follow import paths to target files
//! - Search for symbol in target files
//! - Return resolved symbol

use splice::ingest::imports::{ImportFact, ImportKind};
use splice::resolve::cross_file::{CrossFileResolver, ResolvedSymbol};
use splice::resolve::module_resolver::ModulePathIndex;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_resolver_returns_none() {
        let index = ModulePathIndex::new();
        let resolver = CrossFileResolver::new(&index);

        let result = resolver.resolve_symbol("/src/main.rs", "foo");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_local_symbol_found() {
        // Local symbols (defined in same file) should be found
        let mut index = ModulePathIndex::new();
        index.insert("crate", "/src/main.rs");

        let mut resolver = CrossFileResolver::new(&index);

        // Register a local symbol
        resolver.add_local_symbol("/src/main.rs", "local_fn", "function");

        let result = resolver.resolve_symbol("/src/main.rs", "local_fn");
        assert_eq!(
            result,
            Some(ResolvedSymbol {
                name: "local_fn".to_string(),
                file_path: "/src/main.rs".to_string(),
                kind: "function".to_string(),
            })
        );
    }

    #[test]
    fn test_resolve_imported_symbol() {
        let mut index = ModulePathIndex::new();
        index.insert("crate", "/src/main.rs");
        index.insert("crate::utils", "/src/utils.rs");

        let mut resolver = CrossFileResolver::new(&index);

        // Add import: main.rs imports foo from crate::utils
        // use crate::utils::foo;
        let import = ImportFact {
            file_path: "/src/main.rs".into(),
            import_kind: ImportKind::UseCrate,
            path: vec!["crate".to_string(), "utils".to_string()],
            imported_names: vec!["foo".to_string()],
            is_glob: false,
            is_reexport: false,
            byte_span: (0, 20),
        };
        resolver.add_import(import);

        // Register symbol in target file
        resolver.add_local_symbol("/src/utils.rs", "foo", "function");

        // Resolve from main.rs
        let result = resolver.resolve_symbol("/src/main.rs", "foo");
        assert_eq!(
            result,
            Some(ResolvedSymbol {
                name: "foo".to_string(),
                file_path: "/src/utils.rs".to_string(),
                kind: "function".to_string(),
            })
        );
    }

    #[test]
    fn test_resolve_from_braced_import() {
        let mut index = ModulePathIndex::new();
        index.insert("crate", "/src/main.rs");
        index.insert("crate::utils", "/src/utils.rs");

        let mut resolver = CrossFileResolver::new(&index);

        // Import: use crate::utils::{foo, bar};
        let import = ImportFact {
            file_path: "/src/main.rs".into(),
            import_kind: ImportKind::UseCrate,
            path: vec!["crate".to_string(), "utils".to_string()],
            imported_names: vec!["foo".to_string(), "bar".to_string()],
            is_glob: false,
            is_reexport: false,
            byte_span: (0, 30),
        };
        resolver.add_import(import);

        resolver.add_local_symbol("/src/utils.rs", "foo", "function");
        resolver.add_local_symbol("/src/utils.rs", "bar", "function");

        let result = resolver.resolve_symbol("/src/main.rs", "foo");
        assert_eq!(
            result,
            Some(ResolvedSymbol {
                name: "foo".to_string(),
                file_path: "/src/utils.rs".to_string(),
                kind: "function".to_string(),
            })
        );
    }

    #[test]
    fn test_resolve_from_glob_import() {
        let mut index = ModulePathIndex::new();
        index.insert("crate", "/src/main.rs");
        index.insert("crate::utils", "/src/utils.rs");

        let mut resolver = CrossFileResolver::new(&index);

        // Import: use crate::utils::*;
        let import = ImportFact {
            file_path: "/src/main.rs".into(),
            import_kind: ImportKind::UseCrate,
            path: vec!["crate".to_string(), "utils".to_string()],
            imported_names: vec!["*".to_string()],
            is_glob: true,
            is_reexport: false,
            byte_span: (0, 25),
        };
        resolver.add_import(import);

        resolver.add_local_symbol("/src/utils.rs", "helper", "function");

        let result = resolver.resolve_symbol("/src/main.rs", "helper");
        assert_eq!(
            result,
            Some(ResolvedSymbol {
                name: "helper".to_string(),
                file_path: "/src/utils.rs".to_string(),
                kind: "function".to_string(),
            })
        );
    }

    #[test]
    fn test_local_symbol_shadows_import() {
        let mut index = ModulePathIndex::new();
        index.insert("crate", "/src/main.rs");
        index.insert("crate::utils", "/src/utils.rs");

        let mut resolver = CrossFileResolver::new(&index);

        // Add import of foo from utils
        let import = ImportFact {
            file_path: "/src/main.rs".into(),
            import_kind: ImportKind::UseCrate,
            path: vec!["crate".to_string()],
            imported_names: vec!["foo".to_string()],
            is_glob: false,
            is_reexport: false,
            byte_span: (0, 20),
        };
        resolver.add_import(import);

        // Register local foo (should shadow import)
        resolver.add_local_symbol("/src/main.rs", "foo", "function");
        resolver.add_local_symbol("/src/utils.rs", "foo", "function");

        let result = resolver.resolve_symbol("/src/main.rs", "foo");
        // Should return local, not imported
        assert_eq!(
            result,
            Some(ResolvedSymbol {
                name: "foo".to_string(),
                file_path: "/src/main.rs".to_string(), // Local file, not utils
                kind: "function".to_string(),
            })
        );
    }

    #[test]
    fn test_resolve_unimported_symbol_returns_none() {
        let mut index = ModulePathIndex::new();
        index.insert("crate", "/src/main.rs");
        index.insert("crate::utils", "/src/utils.rs");

        let mut resolver = CrossFileResolver::new(&index);

        // Symbol exists in utils but not imported
        resolver.add_local_symbol("/src/utils.rs", "internal", "function");

        let result = resolver.resolve_symbol("/src/main.rs", "internal");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_self_import() {
        let mut index = ModulePathIndex::new();
        index.insert("crate::foo", "/src/foo.rs");

        let mut resolver = CrossFileResolver::new(&index);

        // Import: use self::bar;
        let import = ImportFact {
            file_path: "/src/foo.rs".into(),
            import_kind: ImportKind::UseSelf,
            path: vec!["self".to_string()],
            imported_names: vec!["bar".to_string()],
            is_glob: false,
            is_reexport: false,
            byte_span: (0, 15),
        };
        resolver.add_import(import);

        resolver.add_local_symbol("/src/foo.rs", "bar", "function");

        let result = resolver.resolve_symbol("/src/foo.rs", "bar");
        // self::bar resolves to local symbol
        assert_eq!(
            result,
            Some(ResolvedSymbol {
                name: "bar".to_string(),
                file_path: "/src/foo.rs".to_string(),
                kind: "function".to_string(),
            })
        );
    }

    #[test]
    fn test_resolve_super_import() {
        let mut index = ModulePathIndex::new();
        index.insert("crate", "/src/main.rs");
        index.insert("crate::inner", "/src/inner.rs");

        let mut resolver = CrossFileResolver::new(&index);

        // Import: use super::parent_func;
        let import = ImportFact {
            file_path: "/src/inner.rs".into(),
            import_kind: ImportKind::UseSuper,
            path: vec!["super".to_string()],
            imported_names: vec!["parent_func".to_string()],
            is_glob: false,
            is_reexport: false,
            byte_span: (0, 25),
        };
        resolver.add_import(import);

        resolver.add_local_symbol("/src/main.rs", "parent_func", "function");

        let result = resolver.resolve_symbol("/src/inner.rs", "parent_func");
        assert_eq!(
            result,
            Some(ResolvedSymbol {
                name: "parent_func".to_string(),
                file_path: "/src/main.rs".to_string(),
                kind: "function".to_string(),
            })
        );
    }

    #[test]
    fn test_resolve_ambiguous_returns_first() {
        let mut index = ModulePathIndex::new();
        index.insert("crate", "/src/main.rs");
        index.insert("crate::a", "/src/a.rs");
        index.insert("crate::b", "/src/b.rs");

        let mut resolver = CrossFileResolver::new(&index);

        // Import foo from both a and b (ambiguous in real code, but we return first match)
        let import_a = ImportFact {
            file_path: "/src/main.rs".into(),
            import_kind: ImportKind::UseCrate,
            path: vec!["crate".to_string(), "a".to_string()],
            imported_names: vec!["foo".to_string()],
            is_glob: false,
            is_reexport: false,
            byte_span: (0, 20),
        };
        resolver.add_import(import_a);

        let import_b = ImportFact {
            file_path: "/src/main.rs".into(),
            import_kind: ImportKind::UseCrate,
            path: vec!["crate".to_string(), "b".to_string()],
            imported_names: vec!["foo".to_string()],
            is_glob: false,
            is_reexport: false,
            byte_span: (20, 40),
        };
        resolver.add_import(import_b);

        resolver.add_local_symbol("/src/a.rs", "foo", "function");
        resolver.add_local_symbol("/src/b.rs", "foo", "function");

        let result = resolver.resolve_symbol("/src/main.rs", "foo");
        // Returns first match (a.rs in this case)
        assert!(result.is_some());
        assert_eq!(result.as_ref().unwrap().file_path, "/src/a.rs");
    }

    #[test]
    fn test_resolve_renamed_import() {
        let mut index = ModulePathIndex::new();
        index.insert("crate", "/src/main.rs");
        index.insert("crate::utils", "/src/utils.rs");

        let mut resolver = CrossFileResolver::new(&index);

        // Import: use crate::utils::helper as util;
        let import = ImportFact {
            file_path: "/src/main.rs".into(),
            import_kind: ImportKind::UseCrate,
            path: vec!["crate".to_string(), "utils".to_string()],
            imported_names: vec!["util".to_string()], // Local alias
            is_glob: false,
            is_reexport: false,
            byte_span: (0, 35),
        };
        resolver.add_import(import);

        // Original name is in utils.rs
        resolver.add_local_symbol("/src/utils.rs", "helper", "function");

        let result = resolver.resolve_symbol("/src/main.rs", "util");
        assert_eq!(
            result,
            Some(ResolvedSymbol {
                name: "helper".to_string(), // Returns original name
                file_path: "/src/utils.rs".to_string(),
                kind: "function".to_string(),
            })
        );
    }

    #[test]
    fn test_resolve_nested_path_import() {
        let mut index = ModulePathIndex::new();
        index.insert("crate", "/src/main.rs");
        index.insert("crate::a", "/src/a.rs");
        index.insert("crate::a::b", "/src/a/b.rs");

        let mut resolver = CrossFileResolver::new(&index);

        // Import: use crate::a::b::deep_func;
        let import = ImportFact {
            file_path: "/src/main.rs".into(),
            import_kind: ImportKind::UseCrate,
            path: vec!["crate".to_string(), "a".to_string(), "b".to_string()],
            imported_names: vec!["deep_func".to_string()],
            is_glob: false,
            is_reexport: false,
            byte_span: (0, 35),
        };
        resolver.add_import(import);

        resolver.add_local_symbol("/src/a/b.rs", "deep_func", "function");

        let result = resolver.resolve_symbol("/src/main.rs", "deep_func");
        assert_eq!(
            result,
            Some(ResolvedSymbol {
                name: "deep_func".to_string(),
                file_path: "/src/a/b.rs".to_string(),
                kind: "function".to_string(),
            })
        );
    }
}
