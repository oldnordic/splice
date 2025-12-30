//! Java symbol extraction tests.

use splice::ingest::extract_java_symbols;
use std::path::Path;

#[test]
fn test_extract_simple_class() {
    let source = b"class MyClass {}\n";
    let path = Path::new("test.java");
    let result = extract_java_symbols(path, source);
    assert!(result.is_ok());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "MyClass");
    assert_eq!(symbols[0].kind.as_str(), "class");
}

#[test]
fn test_extract_class_with_method() {
    let source = b"class MyClass { void method() {} }\n";
    let path = Path::new("test.java");
    let result = extract_java_symbols(path, source);
    assert!(result.is_ok());
    let symbols = result.unwrap();
    // Class + method = 2 symbols
    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0].name, "MyClass");
    assert_eq!(symbols[0].kind.as_str(), "class");
    assert_eq!(symbols[1].name, "method");
    assert_eq!(symbols[1].kind.as_str(), "method");
}

#[test]
fn test_extract_class_with_field() {
    let source = b"class MyClass { private int field; }\n";
    let path = Path::new("test.java");
    let result = extract_java_symbols(path, source);
    assert!(result.is_ok());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0].name, "MyClass");
    assert_eq!(symbols[1].name, "field");
    assert_eq!(symbols[1].kind.as_str(), "field");
}

#[test]
fn test_extract_interface() {
    let source = b"interface MyInterface { void method(); }\n";
    let path = Path::new("test.java");
    let result = extract_java_symbols(path, source);
    assert!(result.is_ok());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0].name, "MyInterface");
    assert_eq!(symbols[0].kind.as_str(), "interface");
    assert_eq!(symbols[1].name, "method");
    assert_eq!(symbols[1].kind.as_str(), "method");
}

#[test]
fn test_extract_enum() {
    let source = b"enum Color { RED, GREEN, BLUE }\n";
    let path = Path::new("test.java");
    let result = extract_java_symbols(path, source);
    assert!(result.is_ok());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "Color");
    assert_eq!(symbols[0].kind.as_str(), "enum");
}

#[test]
fn test_extract_class_with_constructor() {
    let source = b"class Foo { Foo() {} }\n";
    let path = Path::new("test.java");
    let result = extract_java_symbols(path, source);
    assert!(result.is_ok());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0].name, "Foo");
    assert_eq!(symbols[0].kind.as_str(), "class");
    assert_eq!(symbols[1].name, "Foo");
    assert_eq!(symbols[1].kind.as_str(), "constructor");
}

#[test]
fn test_extract_method_with_parameters() {
    let source = b"class MyClass { void add(int a, int b) {} }\n";
    let path = Path::new("test.java");
    let result = extract_java_symbols(path, source);
    assert!(result.is_ok());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[1].parameters, vec!["a", "b"]);
}

#[test]
fn test_extract_public_class() {
    let source = b"public class MyClass {}\n";
    let path = Path::new("test.java");
    let result = extract_java_symbols(path, source);
    assert!(result.is_ok());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "MyClass");
    assert!(symbols[0].is_public);
}

#[test]
fn test_extract_static_method() {
    let source = b"class MyClass { static void method() {} }\n";
    let path = Path::new("test.java");
    let result = extract_java_symbols(path, source);
    assert!(result.is_ok());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[1].name, "method");
    assert!(symbols[1].is_static);
}
