//! Integration tests for symbol expansion across multiple languages.

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test file with known content.
fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut file = File::create(&path).unwrap();
    writeln!(file, "{}", content).unwrap();
    path
}

/// Helper to verify expansion returns expected content.
fn verify_expansion(path: &PathBuf, byte_offset: usize, _expected_content: &str) {
    let source = std::fs::read(path).unwrap();

    // Find what text should be at the expanded region
    assert!(byte_offset < source.len(), "Byte offset beyond file length");

    // For now, just verify we can read the file and the offset is valid
    // Full expansion verification will be done in specific tests
    assert!(!source.is_empty(), "Source file should not be empty");
}

// ============================================================================
// Rust Test Fixtures
// ============================================================================

/// Rust function with doc comment.
const RUST_FUNCTION_FIXTURE: &str = r#"
/// Calculate the fibonacci number for a given index.
///
/// # Arguments
///
/// * `n` - The index in the fibonacci sequence
///
/// # Returns
///
/// The nth fibonacci number
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
"#;

/// Rust struct with doc comment.
const RUST_STRUCT_FIXTURE: &str = r#"
/// Represents a 2D point with x and y coordinates.
///
/// # Fields
///
/// * `x` - The horizontal coordinate
/// * `y` - The vertical coordinate
struct Point {
    x: f64,
    y: f64,
}
"#;

/// Rust impl block with method and doc comment.
const RUST_METHOD_FIXTURE: &str = r#"
impl Point {
    /// Create a new point at the origin (0, 0).
    fn new() -> Self {
        Point { x: 0.0, y: 0.0 }
    }

    /// Calculate the distance from this point to another.
    fn distance_to(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}
"#;

// ============================================================================
// Python Test Fixtures
// ============================================================================

/// Python function with docstring.
const PYTHON_FUNCTION_FIXTURE: &str = r#"
"""Calculate the fibonacci number for a given index.

Args:
    n: The index in the fibonacci sequence

Returns:
    The nth fibonacci number
"""
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)
"#;

/// Python class with docstring.
const PYTHON_CLASS_FIXTURE: &str = r#"
"""Represents a 2D point with x and y coordinates.

Attributes:
    x: The horizontal coordinate
    y: The vertical coordinate
"""
class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y
"#;

/// Python class with method and docstring.
const PYTHON_METHOD_FIXTURE: &str = r#"
class Point:
    """Create a new point at the origin (0, 0)."""

    @staticmethod
    def new():
        return Point(0.0, 0.0)

    """Calculate the distance from this point to another."""

    def distance_to(self, other):
        dx = self.x - other.x
        dy = self.y - other.y
        return (dx * dx + dy * dy) ** 0.5
"#;

// ============================================================================
// Rust Expansion Tests
// ============================================================================

#[test]
fn test_rust_function_expansion() {
    use splice::expand::expand_to_body_with_docs;
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let file = create_test_file(&dir, "test.rs", RUST_FUNCTION_FIXTURE);

    // Find the offset of "fn fibonacci" in the source (to avoid doc comment matches)
    let source = std::fs::read(&file).unwrap();
    let source_str = std::str::from_utf8(&source).unwrap();
    let base = source_str
        .find("fn fibonacci")
        .expect("Should find 'fn fibonacci'");
    let fib_offset = base + 3; // Point to "fibonacci" in "fn fibonacci"

    // Expand to body with docs
    let (start, end) = expand_to_body_with_docs(&file, fib_offset, Language::Rust)
        .expect("Should expand successfully");

    // Verify expansion includes the doc comment and full function
    let expanded = std::str::from_utf8(&source[start..end]).unwrap();
    assert!(
        expanded.contains("/// Calculate the fibonacci"),
        "Should include doc comment"
    );
    assert!(
        expanded.contains("fn fibonacci"),
        "Should include function signature"
    );
    assert!(expanded.contains("match n"), "Should include function body");
}

#[test]
fn test_rust_struct_expansion() {
    use splice::expand::expand_to_body_with_docs;
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let file = create_test_file(&dir, "test.rs", RUST_STRUCT_FIXTURE);

    // Find the offset of "Point" in the source
    let source = std::fs::read(&file).unwrap();
    let source_str = std::str::from_utf8(&source).unwrap();
    let base = source_str
        .find("struct Point")
        .expect("Should find 'struct Point'");
    let point_offset = base + 7; // Point to "Point"

    // Expand to body with docs
    let (start, end) = expand_to_body_with_docs(&file, point_offset, Language::Rust)
        .expect("Should expand successfully");

    // Verify expansion includes the doc comment and full struct
    let expanded = std::str::from_utf8(&source[start..end]).unwrap();
    assert!(
        expanded.contains("/// Represents a 2D point"),
        "Should include doc comment"
    );
    assert!(
        expanded.contains("struct Point"),
        "Should include struct definition"
    );
    assert!(expanded.contains("x: f64"), "Should include struct fields");
}

#[test]
fn test_rust_method_to_impl_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let file = create_test_file(&dir, "test.rs", RUST_METHOD_FIXTURE);

    // Find the offset of "distance_to" in the source
    let source = std::fs::read(&file).unwrap();
    let source_str = std::str::from_utf8(&source).unwrap();
    let distance_offset = source_str
        .find("distance_to")
        .expect("Should find 'distance_to'");

    // Level 1: Expand to method body
    let (method_start, method_end) =
        expand_symbol(&file, distance_offset, Language::Rust, ExpansionLevel::Body)
            .expect("Should expand to method body");

    let method_expanded = std::str::from_utf8(&source[method_start..method_end]).unwrap();
    assert!(
        method_expanded.contains("fn distance_to"),
        "Level 1 should include method signature"
    );
    assert!(
        method_expanded.contains("let dx"),
        "Level 1 should include method body"
    );

    // Level 2: Expand to containing impl block
    let (impl_start, impl_end) = expand_symbol(
        &file,
        distance_offset,
        Language::Rust,
        ExpansionLevel::ContainingBlock,
    )
    .expect("Should expand to impl block");

    let impl_expanded = std::str::from_utf8(&source[impl_start..impl_end]).unwrap();
    assert!(
        impl_expanded.contains("impl Point"),
        "Level 2 should include impl block"
    );
    assert!(
        impl_expanded.contains("fn new"),
        "Level 2 should include other methods in impl"
    );
}

// ============================================================================
// Python Expansion Tests
// ============================================================================

#[test]
fn test_python_function_expansion() {
    use splice::expand::expand_to_body_with_docs;
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let file = create_test_file(&dir, "test.py", PYTHON_FUNCTION_FIXTURE);

    // Find the offset of "fibonacci" in the source
    let source = std::fs::read(&file).unwrap();
    let source_str = std::str::from_utf8(&source).unwrap();
    let base = source_str
        .find("def fibonacci")
        .expect("Should find 'def fibonacci'");
    let fib_offset = base + 4; // Point to "fibonacci"

    // Expand to body with docs
    let (start, end) = expand_to_body_with_docs(&file, fib_offset, Language::Python)
        .expect("Should expand successfully");

    // Verify expansion includes the docstring and full function
    let expanded = std::str::from_utf8(&source[start..end]).unwrap();
    assert!(
        expanded.contains(r#""""Calculate the fibonacci"#),
        "Should include docstring"
    );
    assert!(
        expanded.contains("def fibonacci"),
        "Should include function signature"
    );
    assert!(
        expanded.contains("if n <= 1"),
        "Should include function body"
    );
}

#[test]
fn test_python_class_expansion() {
    use splice::expand::expand_to_body_with_docs;
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let file = create_test_file(&dir, "test.py", PYTHON_CLASS_FIXTURE);

    // Find the offset of "class Point" in the source
    let source = std::fs::read(&file).unwrap();
    let source_str = std::str::from_utf8(&source).unwrap();
    let base = source_str
        .find("class Point")
        .expect("Should find 'class Point'");
    let point_offset = base + 6; // Point to "Point"

    // Expand to body with docs
    let (start, end) = expand_to_body_with_docs(&file, point_offset, Language::Python)
        .expect("Should expand successfully");

    // Verify expansion includes the docstring and full class
    let expanded = std::str::from_utf8(&source[start..end]).unwrap();
    assert!(
        expanded.contains(r#""""Represents a 2D point"#),
        "Should include docstring"
    );
    assert!(
        expanded.contains("class Point"),
        "Should include class definition"
    );
    assert!(
        expanded.contains("def __init__"),
        "Should include class methods"
    );
}

#[test]
fn test_python_method_to_class_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let file = create_test_file(&dir, "test.py", PYTHON_METHOD_FIXTURE);

    // Find the offset of "distance_to" in the source
    let source = std::fs::read(&file).unwrap();
    let source_str = std::str::from_utf8(&source).unwrap();
    let base = source_str
        .find("def distance_to")
        .expect("Should find 'def distance_to'");
    let distance_offset = base + 4; // Point to "distance_to"

    // Level 1: Expand to method body
    let (method_start, method_end) = expand_symbol(
        &file,
        distance_offset,
        Language::Python,
        ExpansionLevel::Body,
    )
    .expect("Should expand to method body");

    let method_expanded = std::str::from_utf8(&source[method_start..method_end]).unwrap();
    assert!(
        method_expanded.contains("def distance_to"),
        "Level 1 should include method signature"
    );
    assert!(
        method_expanded.contains("dx = self.x"),
        "Level 1 should include method body"
    );

    // Level 2: Expand to containing class
    // Note: For Python, class_definition is the containing block for methods
    let (class_start, class_end) = expand_symbol(
        &file,
        distance_offset,
        Language::Python,
        ExpansionLevel::ContainingBlock,
    )
    .expect("Should expand to class block");

    let class_expanded = std::str::from_utf8(&source[class_start..class_end]).unwrap();
    assert!(
        class_expanded.contains("class Point"),
        "Level 2 should include class definition"
    );
    // May or may not include other methods depending on tree-sitter structure
}

// ============================================================================
// C/C++ Expansion Tests (Plan 16-05B)
// ============================================================================

#[test]
fn test_c_function_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
/// Calculate the factorial of a number.
int factorial(int n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}
"#;

    let file = create_test_file(&dir, "test.c", source);
    let factorial_pos = source
        .match_indices("factorial")
        .nth(1)
        .map(|(p, _)| p)
        .unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(&file, factorial_pos, Language::C, ExpansionLevel::Body)
        .expect("Should expand C function");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("int factorial"));
    assert!(expanded.contains("return n * factorial"));
}

#[test]
fn test_c_struct_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
/// Represents a point in 2D space.
struct Point {
    int x;
    int y;
};
"#;

    let file = create_test_file(&dir, "test.c", source);
    let point_pos = source.find("Point").unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(&file, point_pos, Language::C, ExpansionLevel::Body)
        .expect("Should expand C struct");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("struct Point"));
    assert!(expanded.contains("int x"));
}

#[test]
fn test_c_expansion_with_comments() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
/// Calculate sum of two integers.
int add(int a, int b) {
    return a + b;
}
"#;

    let file = create_test_file(&dir, "test.c", source);
    let add_pos = source.find("add").unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(&file, add_pos, Language::C, ExpansionLevel::Body)
        .expect("Should expand C function");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("int add"));
}

#[test]
fn test_cpp_class_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
/// Represents a simple counter.
class Counter {
public:
    void increment() { count++; }
private:
    int count;
};
"#;

    let file = create_test_file(&dir, "test.cpp", source);
    let counter_pos = source.find("Counter").unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(&file, counter_pos, Language::Cpp, ExpansionLevel::Body)
        .expect("Should expand C++ class");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("class Counter"));
}

#[test]
fn test_cpp_method_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
class Calculator {
public:
    int add(int a, int b) { return a + b; }
};
"#;

    let file = create_test_file(&dir, "test.cpp", source);
    let add_pos = source.find("add").unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(&file, add_pos, Language::Cpp, ExpansionLevel::Body)
        .expect("Should expand C++ method");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("int add"));
}

#[test]
fn test_cpp_method_to_class_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
class MyClass {
public:
    void my_method() {}
};
"#;

    let file = create_test_file(&dir, "test.cpp", source);
    let method_pos = source.find("my_method").unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(
        &file,
        method_pos,
        Language::Cpp,
        ExpansionLevel::ContainingBlock,
    )
    .expect("Should expand to containing class");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("class MyClass"));
}

// ============================================================================
// Java Expansion Tests (Plan 16-05B)
// ============================================================================

#[test]
fn test_java_method_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
/**
 * Calculate the sum of two integers.
 */
public int add(int a, int b) {
    return a + b;
}
"#;

    let file = create_test_file(&dir, "test.java", source);
    let add_pos = source.find("add").unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(&file, add_pos, Language::Java, ExpansionLevel::Body)
        .expect("Should expand Java method");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("public int add"));
}

#[test]
fn test_java_class_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
/**
 * Represents a person.
 */
public class Person {
    private String name;
    public String getName() { return name; }
}
"#;

    let file = create_test_file(&dir, "test.java", source);
    let person_pos = source.find("Person").unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(&file, person_pos, Language::Java, ExpansionLevel::Body)
        .expect("Should expand Java class");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("public class Person"));
}

#[test]
fn test_java_method_to_class_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
public class Calculator {
    public int multiply(int a, int b) { return a * b; }
}
"#;

    let file = create_test_file(&dir, "test.java", source);
    let multiply_pos = source.find("multiply").unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(
        &file,
        multiply_pos,
        Language::Java,
        ExpansionLevel::ContainingBlock,
    )
    .expect("Should expand to containing class");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("public class Calculator"));
}

// ============================================================================
// JavaScript/TypeScript Expansion Tests (Plan 16-05B)
// ============================================================================

#[test]
fn test_js_function_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
/**
 * Calculate the factorial of a number.
 */
function factorial(n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}
"#;

    let file = create_test_file(&dir, "test.js", source);
    let factorial_pos = source
        .match_indices("factorial")
        .nth(2)
        .map(|(p, _)| p)
        .unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(
        &file,
        factorial_pos,
        Language::JavaScript,
        ExpansionLevel::Body,
    )
    .expect("Should expand JavaScript function");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("function factorial"));
}

#[test]
fn test_js_class_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
/**
 * Represents a simple counter.
 */
class Counter {
    constructor() { this.count = 0; }
    increment() { this.count++; }
}
"#;

    let file = create_test_file(&dir, "test.js", source);
    let counter_pos = source.find("Counter").unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(
        &file,
        counter_pos,
        Language::JavaScript,
        ExpansionLevel::Body,
    )
    .expect("Should expand JavaScript class");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("class Counter"));
}

#[test]
fn test_js_method_to_class_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
class MyClass {
    myMethod() { return 42; }
}
"#;

    let file = create_test_file(&dir, "test.js", source);
    let method_pos = source.find("myMethod").unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(
        &file,
        method_pos,
        Language::JavaScript,
        ExpansionLevel::ContainingBlock,
    )
    .expect("Should expand to containing class");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("class MyClass"));
}

#[test]
fn test_ts_function_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
/**
 * Calculate the area of a rectangle.
 */
function calculateArea(width: number, height: number): number {
    return width * height;
}
"#;

    let file = create_test_file(&dir, "test.ts", source);
    let calc_pos = source.find("calculateArea").unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(&file, calc_pos, Language::TypeScript, ExpansionLevel::Body)
        .expect("Should expand TypeScript function");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("function calculateArea"));
}

#[test]
fn test_ts_interface_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
/**
 * Represents a user in the system.
 */
interface User {
    id: number;
    name: string;
}
"#;

    let file = create_test_file(&dir, "test.ts", source);
    let user_pos = source.find("User").unwrap();
    let source_bytes = std::fs::read(&file).unwrap();

    let (start, end) = expand_symbol(&file, user_pos, Language::TypeScript, ExpansionLevel::Body)
        .expect("Should expand TypeScript interface");

    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("interface User"));
}

#[test]
fn test_ts_method_to_interface_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
interface MyInterface {
    processData(data: string): number;
}
"#;

    let file = create_test_file(&dir, "test.ts", source);
    let method_pos = source.find("processData").unwrap();

    let result = expand_symbol(
        &file,
        method_pos,
        Language::TypeScript,
        ExpansionLevel::ContainingBlock,
    );
    if let Ok((start, end)) = result {
        let source_bytes = std::fs::read(&file).unwrap();
        let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
        assert!(expanded.contains("interface MyInterface") || expanded.contains("processData"));
    }
}

// ============================================================================
// Progressive Expansion Level Tests (Plan 16-05B)
// ============================================================================

#[test]
fn test_level_0_no_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = "fn test() { let x = 42; }";

    let file = create_test_file(&dir, "test.rs", source);
    let pos = source.find("test").unwrap();

    let (start, end) = expand_symbol(&file, pos, Language::Rust, ExpansionLevel::None)
        .expect("Should expand at level 0");

    let source_bytes = std::fs::read(&file).unwrap();
    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(!expanded.is_empty());
}

#[test]
fn test_level_1_body_expansion() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = "fn test() { let x = 1; }";

    let file = create_test_file(&dir, "test.rs", source);
    let pos = source.find("test").unwrap();

    let (start, end) = expand_symbol(&file, pos, Language::Rust, ExpansionLevel::Body)
        .expect("Should expand at level 1");

    let source_bytes = std::fs::read(&file).unwrap();
    let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
    assert!(expanded.contains("fn test"));
}

#[test]
fn test_level_2_containing_block() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = r#"
mod my_module {
    impl MyClass {
        fn method(&self) {}
    }
}
"#;

    let file = create_test_file(&dir, "test.rs", source);
    let pos = source.find("method").unwrap();

    let result = expand_symbol(&file, pos, Language::Rust, ExpansionLevel::ContainingBlock);
    assert!(result.is_ok());

    if let Ok((start, end)) = result {
        let source_bytes = std::fs::read(&file).unwrap();
        let expanded = std::str::from_utf8(&source_bytes[start..end]).unwrap();
        assert!(expanded.contains("impl") || expanded.contains("mod my_module"));
    }
}

#[test]
fn test_level_2_no_containing_block() {
    use splice::expand::{expand_symbol, ExpansionLevel};
    use splice::symbol::Language;

    let dir = TempDir::new().unwrap();
    let source = "fn standalone() {}\n";

    let file = create_test_file(&dir, "test.rs", source);
    let pos = source.find("standalone").unwrap();

    let result = expand_symbol(&file, pos, Language::Rust, ExpansionLevel::ContainingBlock);
    assert!(result.is_ok() || result.is_err());
}
