//! Enhanced symbol extraction tests.
//!
//! TDD for Phase 1: Enhanced Symbol Storage
//! - module_path tracking
//! - fully_qualified name building
//! - visibility extraction

use splice::ingest::rust::{extract_rust_symbols, RustSymbolKind, Visibility};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_private_function_has_visibility() {
        // Source with private function (no pub modifier)
        let source = b"fn private_fn() {}\n";

        let symbols = extract_rust_symbols(std::path::Path::new("/tmp/test.rs"), source)
            .expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "private_fn");
        assert_eq!(symbols[0].kind, RustSymbolKind::Function);
        assert_eq!(symbols[0].visibility, Visibility::Private);
    }

    #[test]
    fn test_extract_public_function_has_pub_visibility() {
        // Source with public function
        let source = b"pub fn public_fn() {}\n";

        let symbols = extract_rust_symbols(std::path::Path::new("/tmp/test.rs"), source)
            .expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "public_fn");
        assert_eq!(symbols[0].visibility, Visibility::Public);
    }

    #[test]
    fn test_extract_pub_crate_function_has_restricted_visibility() {
        // Source with pub(crate) function
        let source = b"pub(crate) fn crate_fn() {}\n";

        let symbols = extract_rust_symbols(std::path::Path::new("/tmp/test.rs"), source)
            .expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "crate_fn");
        assert_eq!(
            symbols[0].visibility,
            Visibility::Restricted("pub(crate)".to_string())
        );
    }

    #[test]
    fn test_extract_public_struct() {
        // Source with public struct
        let source = b"pub struct MyStruct;\n";

        let symbols = extract_rust_symbols(std::path::Path::new("/tmp/test.rs"), source)
            .expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "MyStruct");
        assert_eq!(symbols[0].kind, RustSymbolKind::Struct);
        assert_eq!(symbols[0].visibility, Visibility::Public);
    }

    #[test]
    fn test_extract_module_declaration() {
        // Source with module declaration
        let source = b"mod my_module;\n";

        let symbols = extract_rust_symbols(std::path::Path::new("/tmp/test.rs"), source)
            .expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "my_module");
        assert_eq!(symbols[0].kind, RustSymbolKind::Module);
        // Module declarations have no explicit pub modifier, detected as Private
        // (though they can be imported within the crate)
        assert_eq!(symbols[0].visibility, Visibility::Private);
    }

    #[test]
    fn test_fully_qualified_name_for_crate_level_function() {
        // Source with crate-level function (no module)
        let source = b"pub fn crate_fn() {}\n";

        let symbols = extract_rust_symbols(std::path::Path::new("/tmp/test.rs"), source)
            .expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        // At crate level, fully_qualified = "crate::<name>"
        assert_eq!(symbols[0].fully_qualified, "crate::crate_fn");
        assert_eq!(symbols[0].module_path, "crate");
    }

    #[test]
    fn test_multiple_symbols_extracted() {
        // Source with multiple symbols
        let source = b"
            pub struct Foo;
            pub fn bar() {}
            mod baz {}
        ";

        let symbols = extract_rust_symbols(std::path::Path::new("/tmp/test.rs"), source)
            .expect("Failed to parse");

        assert_eq!(symbols.len(), 3);

        // Check struct
        assert_eq!(symbols[0].name, "Foo");
        assert_eq!(symbols[0].kind, RustSymbolKind::Struct);

        // Check function
        assert_eq!(symbols[1].name, "bar");
        assert_eq!(symbols[1].kind, RustSymbolKind::Function);

        // Check module
        assert_eq!(symbols[2].name, "baz");
        assert_eq!(symbols[2].kind, RustSymbolKind::Module);
    }
}
