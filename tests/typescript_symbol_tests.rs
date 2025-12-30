//! TypeScript symbol extraction tests.
//!
//! TDD for TypeScript Symbol Extraction
//! - interface_declaration
//! - type_alias_declaration
//! - enum_declaration
//! - namespace_declaration
//! - class with decorators
//! - exported symbols
//! - type annotations

use splice::ingest::typescript::{extract_typescript_symbols, TypeScriptSymbolKind};
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_interface_declaration() {
        // Source: interface User {
        //           name: string;
        //           age: number;
        //         }
        let source = b"interface User {\n  name: string;\n  age: number;\n}\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "User");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Interface);
    }

    #[test]
    fn test_extract_type_alias() {
        // Source: type UserId = string | number;
        let source = b"type UserId = string | number;\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "UserId");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::TypeAlias);
    }

    #[test]
    fn test_extract_enum_declaration() {
        // Source: enum Color {
        //           Red,
        //           Green,
        //           Blue
        //         }
        let source = b"enum Color {\n  Red,\n  Green,\n  Blue\n}\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Color");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Enum);
    }

    #[test]
    fn test_extract_namespace_declaration() {
        // Source: namespace Utils {
        //           export function helper() {}
        //         }
        let source = b"namespace Utils {\n  export function helper() {}\n}\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 2); // namespace + function
        assert_eq!(symbols[0].name, "Utils");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Namespace);
    }

    #[test]
    fn test_extract_class_declaration() {
        // Source: class Person {
        //           constructor(public name: string) {}
        //           greet(): void {}
        //         }
        let source =
            b"class Person {\n  constructor(public name: string) {}\n  greet(): void {}\n}\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        // Should extract: class, constructor, method
        assert!(!symbols.is_empty());
        assert_eq!(symbols[0].name, "Person");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Class);
    }

    #[test]
    fn test_extract_function_with_type_annotations() {
        // Source: function add(a: number, b: number): number {
        //           return a + b;
        //         }
        let source = b"function add(a: number, b: number): number {\n  return a + b;\n}\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "add");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Function);
        assert_eq!(symbols[0].parameters, vec!["a", "b"]);
    }

    #[test]
    fn test_extract_exported_interface() {
        // Source: export interface IUser {
        //           id: string;
        //         }
        let source = b"export interface IUser {\n  id: string;\n}\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "IUser");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Interface);
        assert!(symbols[0].is_exported);
    }

    #[test]
    fn test_extract_generic_function() {
        // Source: function identity<T>(arg: T): T {
        //           return arg;
        //         }
        let source = b"function identity<T>(arg: T): T {\n  return arg;\n}\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "identity");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Function);
    }

    #[test]
    fn test_extract_class_with_method() {
        // Source: class Calculator {
        //           add(a: number, b: number): number {
        //             return a + b;
        //           }
        //         }
        let source = b"class Calculator {\n  add(a: number, b: number): number {\n    return a + b;\n  }\n}\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "Calculator");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Class);
        assert_eq!(symbols[1].name, "add");
        assert_eq!(symbols[1].kind, TypeScriptSymbolKind::Method);
    }

    #[test]
    fn test_extract_async_function() {
        // Source: async function fetchData(): Promise<string> {
        //           return "data";
        //         }
        let source = b"async function fetchData(): Promise<string> {\n  return \"data\";\n}\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "fetchData");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Function);
        assert!(symbols[0].is_async);
    }

    #[test]
    fn test_symbol_has_byte_span() {
        // Source: interface Foo {}
        let source = b"interface Foo {}\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        // Should have byte span set
        assert!(symbols[0].byte_start < symbols[0].byte_end);
        assert_eq!(symbols[0].byte_start, 0); // Starts at beginning
    }

    #[test]
    fn test_extract_variable_declaration_with_type() {
        // Source: const count: number = 42;
        let source = b"const count: number = 42;\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "count");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Variable);
    }

    #[test]
    fn test_extract_arrow_function_assignment() {
        // Source: const greet = (name: string): string => `Hello ${name}`;
        let source = b"const greet = (name: string): string => `Hello ${name}`;\n";

        let symbols =
            extract_typescript_symbols(Path::new("/tmp/test.ts"), source).expect("Failed to parse");

        // Should extract the variable declaration
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "greet");
        assert_eq!(symbols[0].kind, TypeScriptSymbolKind::Variable);
    }
}
