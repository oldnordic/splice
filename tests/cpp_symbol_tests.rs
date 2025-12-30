//! C/C++ symbol extraction tests.
//!
//! TDD for Phase 6.4-6.5: C/C++ Symbol Extraction
//! - function_definition
//! - class_specifier
//! - struct_specifier
//! - namespace_definition
//! - enum_specifier
//! - nested methods in classes
//! - template functions/classes

use splice::ingest::cpp::{extract_cpp_symbols, CppSymbolKind};
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_function() {
        // Source: int foo() { return 42; }
        let source = b"int foo() { return 42; }\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "foo");
        assert_eq!(symbols[0].kind, CppSymbolKind::Function);
        assert_eq!(symbols[0].namespace_path, "");
        assert!(!symbols[0].is_template);
    }

    #[test]
    fn test_extract_function_with_parameters() {
        // Source: int add(int a, int b) { return a + b; }
        let source = b"int add(int a, int b) { return a + b; }\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "add");
        assert_eq!(symbols[0].kind, CppSymbolKind::Function);
        // Parameters should be stored
        assert_eq!(symbols[0].parameters, vec!["a", "b"]);
    }

    #[test]
    fn test_extract_class_definition() {
        // Source: class MyClass { public: void method(); };
        let source = b"class MyClass {\npublic:\n    void method();\n};\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "MyClass");
        assert_eq!(symbols[0].kind, CppSymbolKind::Class);
    }

    #[test]
    fn test_extract_struct_definition() {
        // Source: struct Point { int x; int y; };
        let source = b"struct Point {\n    int x;\n    int y;\n};\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Point");
        assert_eq!(symbols[0].kind, CppSymbolKind::Struct);
    }

    #[test]
    fn test_extract_namespace_definition() {
        // Source: namespace utils { void helper(); }
        let source = b"namespace utils {\n    void helper();\n}\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        // Should extract both namespace and function inside
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "utils");
        assert_eq!(symbols[0].kind, CppSymbolKind::Namespace);
        assert_eq!(symbols[1].name, "helper");
        assert_eq!(symbols[1].namespace_path, "utils");
    }

    #[test]
    fn test_extract_enum_definition() {
        // Source: enum Color { Red, Green, Blue };
        let source = b"enum Color { Red, Green, Blue };\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Color");
        assert_eq!(symbols[0].kind, CppSymbolKind::Enum);
    }

    #[test]
    fn test_extract_enum_class_definition() {
        // Source: enum class Color { Red, Green, Blue };
        let source = b"enum class Color { Red, Green, Blue };\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Color");
        assert_eq!(symbols[0].kind, CppSymbolKind::Enum);
    }

    #[test]
    fn test_extract_class_with_method() {
        // Source: class Baz { public: void method() {} };
        let source = b"class Baz {\npublic:\n    void method() {}\n};\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        // Should extract both class and method
        assert!(!symbols.is_empty());
        assert_eq!(symbols[0].name, "Baz");
        assert_eq!(symbols[0].kind, CppSymbolKind::Class);
    }

    #[test]
    fn test_extract_nested_class() {
        // Source: class Outer { public: class Inner { public: void method() {} }; };
        let source = b"class Outer {\npublic:\n    class Inner {\n    public:\n        void method() {}\n    };\n};\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert!(symbols.len() >= 2);

        // First symbol: the outer class
        assert_eq!(symbols[0].name, "Outer");
        assert_eq!(symbols[0].kind, CppSymbolKind::Class);

        // Second symbol: the inner class (nested)
        assert_eq!(symbols[1].name, "Inner");
        assert_eq!(symbols[1].kind, CppSymbolKind::Class);
        assert_eq!(symbols[1].namespace_path, "Outer");
    }

    #[test]
    fn test_extract_template_function() {
        // Source: template<typename T> T max(T a, T b) { return a > b ? a : b; }
        let source = b"template<typename T>\nT max(T a, T b) { return a > b ? a : b; }\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "max");
        assert_eq!(symbols[0].kind, CppSymbolKind::TemplateFunction);
        assert!(symbols[0].is_template);
    }

    #[test]
    fn test_extract_template_class() {
        // Source: template<typename T> class Vector { };
        let source = b"template<typename T>\nclass Vector {\n};\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Vector");
        assert_eq!(symbols[0].kind, CppSymbolKind::TemplateClass);
        assert!(symbols[0].is_template);
    }

    #[test]
    fn test_extract_multiple_functions() {
        // Source: void foo() {} void bar() {}
        let source = b"void foo() {}\nvoid bar() {}\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "foo");
        assert_eq!(symbols[1].name, "bar");
    }

    #[test]
    fn test_extract_function_in_namespace() {
        // Source: namespace utils { void helper() {} }
        let source = b"namespace utils {\n    void helper() {}\n}\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        // Should extract namespace and function
        assert!(symbols.len() >= 2);
        assert_eq!(symbols[0].name, "utils");
        assert_eq!(symbols[0].kind, CppSymbolKind::Namespace);
        assert_eq!(symbols[1].name, "helper");
        assert_eq!(symbols[1].namespace_path, "utils");
    }

    #[test]
    fn test_cpp_symbol_has_byte_span() {
        // Source: void foo() {}
        let source = b"void foo() {}\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        // Should have byte span set
        assert!(symbols[0].byte_start < symbols[0].byte_end);
        assert_eq!(symbols[0].byte_start, 0); // Starts at beginning
    }

    #[test]
    fn test_extract_class_with_multiple_methods() {
        // Source: class Calculator { public: int add(int a, int b); int sub(int a, int b); };
        let source = b"class Calculator {\npublic:\n    int add(int a, int b);\n    int sub(int a, int b);\n};\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert!(!symbols.is_empty());
        assert_eq!(symbols[0].name, "Calculator");
        assert_eq!(symbols[0].kind, CppSymbolKind::Class);
    }

    #[test]
    fn test_extract_namespace_with_nested_class() {
        // Source: namespace std { template<typename T> class vector { }; }
        let source = b"namespace std {\n    template<typename T>\n    class vector {\n    };\n}\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert!(symbols.len() >= 2);
        assert_eq!(symbols[0].name, "std");
        assert_eq!(symbols[0].kind, CppSymbolKind::Namespace);
        assert_eq!(symbols[1].name, "vector");
        assert_eq!(symbols[1].kind, CppSymbolKind::TemplateClass);
        assert_eq!(symbols[1].namespace_path, "std");
    }

    #[test]
    fn test_extract_empty_source() {
        // Source with no symbols
        let source = b"int x;\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        // No function/class/struct definitions, just a variable
        assert_eq!(symbols.len(), 0);
    }

    #[test]
    fn test_extract_function_returns_void() {
        // Source: void func() {}
        let source = b"void func() {}\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "func");
        assert_eq!(symbols[0].kind, CppSymbolKind::Function);
    }

    #[test]
    fn test_fully_qualified_name() {
        // Source: namespace ns { class MyClass { public: void method(); }; }
        let source = b"namespace ns {\n    class MyClass {\n    public:\n        void method();\n    };\n}\n";

        let symbols =
            extract_cpp_symbols(Path::new("/tmp/test.cpp"), source).expect("Failed to parse");

        assert!(symbols.len() >= 2);
        // Check namespace
        assert_eq!(symbols[0].name, "ns");
        assert_eq!(symbols[0].fully_qualified, "ns");
        // Check class in namespace
        assert_eq!(symbols[1].name, "MyClass");
        assert_eq!(symbols[1].fully_qualified, "ns::MyClass");
    }
}
