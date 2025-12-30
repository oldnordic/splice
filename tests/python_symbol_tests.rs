//! Python symbol extraction tests.
//!
//! TDD for Phase 5.4-5.5: Python Symbol Extraction
//! - function_definition
//! - class_definition
//! - async functions
//! - nested methods in classes
//! - module-level variables

use splice::ingest::python::{extract_python_symbols, PythonSymbolKind};
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_function() {
        // Source: def foo():
        //         pass
        let source = b"def foo():\n    pass\n";

        let symbols =
            extract_python_symbols(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "foo");
        assert_eq!(symbols[0].kind, PythonSymbolKind::Function);
        assert_eq!(symbols[0].module_path, "module"); // Default module path
        assert!(!symbols[0].is_async);
    }

    #[test]
    fn test_extract_class_definition() {
        // Source: class Bar:
        //         pass
        let source = b"class Bar:\n    pass\n";

        let symbols =
            extract_python_symbols(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Bar");
        assert_eq!(symbols[0].kind, PythonSymbolKind::Class);
    }

    #[test]
    fn test_extract_class_with_method() {
        // Source: class Baz:
        //         def method(self):
        //             pass
        let source = b"class Baz:\n    def method(self):\n        pass\n";

        let symbols =
            extract_python_symbols(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 2);

        // First symbol: the class
        assert_eq!(symbols[0].name, "Baz");
        assert_eq!(symbols[0].kind, PythonSymbolKind::Class);

        // Second symbol: the method (nested under class)
        assert_eq!(symbols[1].name, "method");
        assert_eq!(symbols[1].kind, PythonSymbolKind::Function);
        assert_eq!(symbols[1].module_path, "module::Baz");
    }

    #[test]
    fn test_extract_async_function() {
        // Source: async def async_func():
        //         pass
        let source = b"async def async_func():\n    pass\n";

        let symbols =
            extract_python_symbols(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "async_func");
        assert_eq!(symbols[0].kind, PythonSymbolKind::Function);
        assert!(symbols[0].is_async);
    }

    #[test]
    fn test_extract_function_with_parameters() {
        // Source: def greet(name: str) -> str:
        //         return name
        let source = b"def greet(name: str) -> str:\n    return name\n";

        let symbols =
            extract_python_symbols(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "greet");
        assert_eq!(symbols[0].kind, PythonSymbolKind::Function);
        // Parameters should be stored
        assert_eq!(symbols[0].parameters, vec!["name"]);
    }

    #[test]
    fn test_extract_multiple_functions() {
        // Source: def foo(): pass
        //         def bar(): pass
        let source = b"def foo():\n    pass\n\ndef bar():\n    pass\n";

        let symbols =
            extract_python_symbols(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "foo");
        assert_eq!(symbols[1].name, "bar");
    }

    #[test]
    fn test_extract_nested_class_with_method() {
        // Source: class Outer:
        //         class Inner:
        //             def method(self):
        //                 pass
        let source =
            b"class Outer:\n    class Inner:\n        def method(self):\n            pass\n";

        let symbols =
            extract_python_symbols(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 3);
        assert_eq!(symbols[0].name, "Outer");
        assert_eq!(symbols[0].kind, PythonSymbolKind::Class);
        assert_eq!(symbols[1].name, "Inner");
        assert_eq!(symbols[1].kind, PythonSymbolKind::Class);
        assert_eq!(symbols[1].module_path, "module::Outer");
        assert_eq!(symbols[2].name, "method");
        assert_eq!(symbols[2].kind, PythonSymbolKind::Function);
        assert_eq!(symbols[2].module_path, "module::Outer::Inner");
    }

    #[test]
    fn test_symbol_has_byte_span() {
        // Source: def foo(): pass
        let source = b"def foo(): pass\n";

        let symbols =
            extract_python_symbols(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        // Should have byte span set
        assert!(symbols[0].byte_start < symbols[0].byte_end);
        assert_eq!(symbols[0].byte_start, 0); // Starts at beginning
    }

    #[test]
    fn test_extract_class_with_multiple_methods() {
        // Source: class Calculator:
        //         def add(self, a, b): return a + b
        //         def subtract(self, a, b): return a - b
        let source = b"class Calculator:\n    def add(self, a, b): return a + b\n    def subtract(self, a, b): return a - b\n";

        let symbols =
            extract_python_symbols(Path::new("/tmp/test.py"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 3);
        assert_eq!(symbols[0].name, "Calculator");
        assert_eq!(symbols[1].name, "add");
        assert_eq!(symbols[2].name, "subtract");
        // Both methods should be under the Calculator module path
        assert_eq!(symbols[1].module_path, "module::Calculator");
        assert_eq!(symbols[2].module_path, "module::Calculator");
    }
}
