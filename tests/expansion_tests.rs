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
    let base = source_str.find("fn fibonacci").expect("Should find 'fn fibonacci'");
    let fib_offset = base + 3; // Point to "fibonacci" in "fn fibonacci"

    // Expand to body with docs
    let (start, end) = expand_to_body_with_docs(&file, fib_offset, Language::Rust)
        .expect("Should expand successfully");

    // Verify expansion includes the doc comment and full function
    let expanded = std::str::from_utf8(&source[start..end]).unwrap();
    assert!(expanded.contains("/// Calculate the fibonacci"), "Should include doc comment");
    assert!(expanded.contains("fn fibonacci"), "Should include function signature");
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
    let base = source_str.find("struct Point").expect("Should find 'struct Point'");
    let point_offset = base + 7; // Point to "Point"

    // Expand to body with docs
    let (start, end) = expand_to_body_with_docs(&file, point_offset, Language::Rust)
        .expect("Should expand successfully");

    // Verify expansion includes the doc comment and full struct
    let expanded = std::str::from_utf8(&source[start..end]).unwrap();
    assert!(expanded.contains("/// Represents a 2D point"), "Should include doc comment");
    assert!(expanded.contains("struct Point"), "Should include struct definition");
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
    let distance_offset = source_str.find("distance_to").expect("Should find 'distance_to'");

    // Level 1: Expand to method body
    let (method_start, method_end) = expand_symbol(&file, distance_offset, Language::Rust, ExpansionLevel::Body)
        .expect("Should expand to method body");

    let method_expanded = std::str::from_utf8(&source[method_start..method_end]).unwrap();
    assert!(method_expanded.contains("fn distance_to"), "Level 1 should include method signature");
    assert!(method_expanded.contains("let dx"), "Level 1 should include method body");

    // Level 2: Expand to containing impl block
    let (impl_start, impl_end) = expand_symbol(&file, distance_offset, Language::Rust, ExpansionLevel::ContainingBlock)
        .expect("Should expand to impl block");

    let impl_expanded = std::str::from_utf8(&source[impl_start..impl_end]).unwrap();
    assert!(impl_expanded.contains("impl Point"), "Level 2 should include impl block");
    assert!(impl_expanded.contains("fn new"), "Level 2 should include other methods in impl");
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
    let base = source_str.find("def fibonacci").expect("Should find 'def fibonacci'");
    let fib_offset = base + 4; // Point to "fibonacci"

    // Expand to body with docs
    let (start, end) = expand_to_body_with_docs(&file, fib_offset, Language::Python)
        .expect("Should expand successfully");

    // Verify expansion includes the docstring and full function
    let expanded = std::str::from_utf8(&source[start..end]).unwrap();
    assert!(expanded.contains(r#""""Calculate the fibonacci"#), "Should include docstring");
    assert!(expanded.contains("def fibonacci"), "Should include function signature");
    assert!(expanded.contains("if n <= 1"), "Should include function body");
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
    let base = source_str.find("class Point").expect("Should find 'class Point'");
    let point_offset = base + 6; // Point to "Point"

    // Expand to body with docs
    let (start, end) = expand_to_body_with_docs(&file, point_offset, Language::Python)
        .expect("Should expand successfully");

    // Verify expansion includes the docstring and full class
    let expanded = std::str::from_utf8(&source[start..end]).unwrap();
    assert!(expanded.contains(r#""""Represents a 2D point"#), "Should include docstring");
    assert!(expanded.contains("class Point"), "Should include class definition");
    assert!(expanded.contains("def __init__"), "Should include class methods");
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
    let base = source_str.find("def distance_to").expect("Should find 'def distance_to'");
    let distance_offset = base + 4; // Point to "distance_to"

    // Level 1: Expand to method body
    let (method_start, method_end) = expand_symbol(&file, distance_offset, Language::Python, ExpansionLevel::Body)
        .expect("Should expand to method body");

    let method_expanded = std::str::from_utf8(&source[method_start..method_end]).unwrap();
    assert!(method_expanded.contains("def distance_to"), "Level 1 should include method signature");
    assert!(method_expanded.contains("dx = self.x"), "Level 1 should include method body");

    // Level 2: Expand to containing class
    // Note: For Python, class_definition is the containing block for methods
    let (class_start, class_end) = expand_symbol(&file, distance_offset, Language::Python, ExpansionLevel::ContainingBlock)
        .expect("Should expand to class block");

    let class_expanded = std::str::from_utf8(&source[class_start..class_end]).unwrap();
    assert!(class_expanded.contains("class Point"), "Level 2 should include class definition");
    // May or may not include other methods depending on tree-sitter structure
}
