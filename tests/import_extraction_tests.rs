//! Import extraction tests.
//!
//! TDD for Phase 2: Import Extraction
//! - ImportFact struct
//! - ImportKind enum
//! - use statement extraction from Rust AST

use splice::ingest::imports::{extract_rust_imports, ImportKind};
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_crate_use() {
        // Source: use crate::foo::bar;
        let source = b"use crate::foo::bar;\n";

        let imports =
            extract_rust_imports(Path::new("/tmp/test.rs"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::UseCrate);
        assert_eq!(imports[0].path, vec!["crate", "foo"]);
        assert_eq!(imports[0].imported_names, vec!["bar"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_super_use() {
        // Source: use super::baz;
        let source = b"use super::baz;\n";

        let imports =
            extract_rust_imports(Path::new("/tmp/test.rs"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::UseSuper);
        assert_eq!(imports[0].path, vec!["super"]);
        assert_eq!(imports[0].imported_names, vec!["baz"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_self_use() {
        // Source: use self::inner::Item;
        let source = b"use self::inner::Item;\n";

        let imports =
            extract_rust_imports(Path::new("/tmp/test.rs"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::UseSelf);
        assert_eq!(imports[0].path, vec!["self", "inner"]);
        assert_eq!(imports[0].imported_names, vec!["Item"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_plain_use() {
        // Source: use std::collections::HashMap;
        let source = b"use std::collections::HashMap;\n";

        let imports =
            extract_rust_imports(Path::new("/tmp/test.rs"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::PlainUse);
        assert_eq!(imports[0].path, vec!["std", "collections"]);
        assert_eq!(imports[0].imported_names, vec!["HashMap"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_glob_import() {
        // Source: use crate::module::*;
        let source = b"use crate::module::*;\n";

        let imports =
            extract_rust_imports(Path::new("/tmp/test.rs"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::UseCrate);
        assert_eq!(imports[0].path, vec!["crate", "module"]);
        // For glob imports, imported_names contains "*"
        assert!(imports[0].is_glob);
    }

    #[test]
    fn test_extract_braced_import_single() {
        // Source: use crate::module::{Item};
        let source = b"use crate::module::{Item};\n";

        let imports =
            extract_rust_imports(Path::new("/tmp/test.rs"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::UseCrate);
        assert_eq!(imports[0].path, vec!["crate", "module"]);
        assert_eq!(imports[0].imported_names, vec!["Item"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_braced_import_multiple() {
        // Source: use crate::module::{foo, bar, baz};
        let source = b"use crate::module::{foo, bar, baz};\n";

        let imports =
            extract_rust_imports(Path::new("/tmp/test.rs"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::UseCrate);
        assert_eq!(imports[0].path, vec!["crate", "module"]);
        assert_eq!(imports[0].imported_names, vec!["foo", "bar", "baz"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_renamed_import() {
        // Source: use crate::foo as Bar;
        let source = b"use crate::foo as Bar;\n";

        let imports =
            extract_rust_imports(Path::new("/tmp/test.rs"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::UseCrate);
        assert_eq!(imports[0].path, vec!["crate"]);
        // Original name is "foo", renamed to "Bar"
        // imported_names should contain the local name after "as"
        assert_eq!(imports[0].imported_names, vec!["Bar"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_extract_multiple_use_statements() {
        // Source with multiple imports
        let source = b"
            use crate::foo;
            use std::vec::Vec;
            use super::bar;
        ";

        let imports =
            extract_rust_imports(Path::new("/tmp/test.rs"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 3);

        // Check first import
        assert_eq!(imports[0].import_kind, ImportKind::UseCrate);
        assert_eq!(imports[0].path, vec!["crate"]);
        assert_eq!(imports[0].imported_names, vec!["foo"]);

        // Check second import
        assert_eq!(imports[1].import_kind, ImportKind::PlainUse);
        assert_eq!(imports[1].path, vec!["std", "vec"]);
        assert_eq!(imports[1].imported_names, vec!["Vec"]);

        // Check third import
        assert_eq!(imports[2].import_kind, ImportKind::UseSuper);
        assert_eq!(imports[2].path, vec!["super"]);
        assert_eq!(imports[2].imported_names, vec!["bar"]);
    }

    #[test]
    fn test_extract_extern_crate_use() {
        // Source: extern crate serde; use serde::Serialize;
        let source = b"extern crate serde;\nuse serde::Serialize;\n";

        let imports =
            extract_rust_imports(Path::new("/tmp/test.rs"), source).expect("Failed to parse");

        // extern crate is a declaration, not a use statement
        // Only the use statement should be extracted
        assert_eq!(imports.len(), 1);
        // use serde::Serialize; is a plain use (no crate/super/self keyword)
        assert_eq!(imports[0].import_kind, ImportKind::PlainUse);
        assert_eq!(imports[0].path, vec!["serde"]);
        assert_eq!(imports[0].imported_names, vec!["Serialize"]);
        assert!(!imports[0].is_glob);
    }

    #[test]
    fn test_import_has_byte_span() {
        // Source: use crate::foo;
        let source = b"use crate::foo;\n";

        let imports =
            extract_rust_imports(Path::new("/tmp/test.rs"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        // Byte span is (0, 15) for "use crate::foo;" (15 bytes, tree-sitter end is exclusive)
        assert_eq!(imports[0].byte_span, (0, 15));
    }

    #[test]
    fn test_extract_nested_path_use() {
        // Source: use crate::a::b::c::Item;
        let source = b"use crate::a::b::c::Item;\n";

        let imports =
            extract_rust_imports(Path::new("/tmp/test.rs"), source).expect("Failed to parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].import_kind, ImportKind::UseCrate);
        assert_eq!(imports[0].path, vec!["crate", "a", "b", "c"]);
        assert_eq!(imports[0].imported_names, vec!["Item"]);
        assert!(!imports[0].is_glob);
    }
}
