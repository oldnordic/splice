//! Magellan integration tests.
//!
//! Tests the Magellan v0.5.3 integration layer for:
//! - Multi-language file indexing (7 languages)
//! - Label-based symbol queries
//! - Code chunk retrieval without file re-reading
//! - Error handling at integration boundaries

use splice::graph::magellan_integration::{CodeChunk, MagellanIntegration, SymbolInfo};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

///////////////////////////////////////////////////////////////////////////////
// Test fixtures
///////////////////////////////////////////////////////////////////////////////

/// Create a temporary Magellan database for isolated testing.
fn create_temp_magellan_db() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Create a sample Rust source file with various symbol types.
fn create_sample_rust_file(dir: &Path) -> PathBuf {
    let file_path = dir.join("sample_rust.rs");
    let content = r#"
// Sample Rust file for testing Magellan integration

/// Documentation comment
pub struct MyStruct {
    pub field1: i32,
    pub field2: String,
}

impl MyStruct {
    /// Create a new instance
    pub fn new(field1: i32, field2: String) -> Self {
        Self { field1, field2 }
    }

    /// Get field1 value
    pub fn get_field1(&self) -> i32 {
        self.field1
    }
}

/// A public trait
pub trait MyTrait {
    /// Required method
    fn do_something(&self) -> i32;
}

/// Implementation of the trait
impl MyTrait for MyStruct {
    fn do_something(&self) -> i32 {
        self.field1
    }
}

/// A public function
pub fn my_function(x: i32, y: i32) -> i32 {
    x + y
}

/// A private module
mod private_module {
    pub fn internal_function() -> i32 {
        42
    }
}

/// A public enum
pub enum MyEnum {
    Variant1,
    Variant2(i32),
    Variant3 { x: i32, y: i32 },
}
"#;
    fs::write(&file_path, content).expect("Failed to write sample Rust file");
    file_path
}

/// Create a sample Python source file.
fn create_sample_python_file(dir: &Path) -> PathBuf {
    let file_path = dir.join("sample_python.py");
    let content = r#"
# Sample Python file for testing Magellan integration

class MyClass:
    """A simple class."""

    def __init__(self, value):
        self.value = value

    def get_value(self):
        return self.value

    async def async_method(self):
        return await self.some_async_operation()

    @staticmethod
    def static_method():
        return 42

def my_function(x, y):
    return x + y

async def my_async_function():
    await some_operation()
"#;
    fs::write(&file_path, content).expect("Failed to write sample Python file");
    file_path
}

/// Create a sample C source file.
fn create_sample_c_file(dir: &Path) -> PathBuf {
    let file_path = dir.join("sample_c.c");
    let content = r#"
#include <stdio.h>

struct MyStruct {
    int field1;
    int field2;
};

typedef struct {
    int x;
    int y;
} Point;

int my_function(int x, int y) {
    return x + y;
}

void process_struct(struct MyStruct* s) {
    s->field1 = 42;
}
"#;
    fs::write(&file_path, content).expect("Failed to write sample C file");
    file_path
}

/// Create a sample C++ source file.
fn create_sample_cpp_file(dir: &Path) -> PathBuf {
    let file_path = dir.join("sample_cpp.cpp");
    let content = r#"
#include <iostream>

class MyClass {
public:
    MyClass(int value) : value(value) {}

    int getValue() const {
        return value;
    }

    void setValue(int value) {
        this->value = value;
    }

private:
    int value;
};

namespace MyNamespace {
    class NamespacedClass {
    public:
        void method();
    };
}

template<typename T>
class TemplateClass {
public:
    T get_value() {
        return value;
    }

private:
    T value;
};
"#;
    fs::write(&file_path, content).expect("Failed to write sample C++ file");
    file_path
}

/// Create a sample Java source file.
fn create_sample_java_file(dir: &Path) -> PathBuf {
    let file_path = dir.join("SampleJava.java");
    let content = r#"
public class SampleJava {
    private int value;

    public SampleJava(int value) {
        this.value = value;
    }

    public int getValue() {
        return value;
    }

    public void setValue(int value) {
        this.value = value;
    }

    private void helperMethod() {
        System.out.println("Helper");
    }

    interface MyInterface {
        void doSomething();
    }

    static class InnerClass {
        public void innerMethod() {
        }
    }
}
"#;
    fs::write(&file_path, content).expect("Failed to write sample Java file");
    file_path
}

/// Create a sample JavaScript source file.
fn create_sample_javascript_file(dir: &Path) -> PathBuf {
    let file_path = dir.join("sample_javascript.js");
    let content = r#"
// Sample JavaScript file for testing

class MyClass {
    constructor(value) {
        this.value = value;
    }

    getValue() {
        return this.value;
    }

    async asyncMethod() {
        await someOperation();
        return this.value;
    }
}

function myFunction(x, y) {
    return x + y;
}

const arrowFunction = (x, y) => x + y;

class AnotherClass extends MyClass {
    constructor(value) {
        super(value);
    }

    getValue() {
        return super.getValue() * 2;
    }
}
"#;
    fs::write(&file_path, content).expect("Failed to write sample JavaScript file");
    file_path
}

/// Create a sample TypeScript source file.
fn create_sample_typescript_file(dir: &Path) -> PathBuf {
    let file_path = dir.join("sample_typescript.ts");
    let content = r#"
// Sample TypeScript file for testing

interface MyInterface {
    getValue(): number;
    setValue(value: number): void;
}

class MyClass implements MyInterface {
    private value: number;

    constructor(value: number) {
        this.value = value;
    }

    public getValue(): number {
        return this.value;
    }

    public setValue(value: number): void {
        this.value = value;
    }
}

type MyType = string | number;

interface GenericInterface<T> {
    get(): T;
    set(value: T): void;
}

class GenericClass<T> implements GenericInterface<T> {
    private value: T;

    constructor(value: T) {
        this.value = value;
    }

    get(): T {
        return this.value;
    }

    set(value: T): void {
        this.value = value;
    }
}
"#;
    fs::write(&file_path, content).expect("Failed to write sample TypeScript file");
    file_path
}

/// Create sample files for all 7 supported languages.
fn create_multilang_workspace(dir: &Path) -> Vec<PathBuf> {
    vec![
        create_sample_rust_file(dir),
        create_sample_python_file(dir),
        create_sample_c_file(dir),
        create_sample_cpp_file(dir),
        create_sample_java_file(dir),
        create_sample_javascript_file(dir),
        create_sample_typescript_file(dir),
    ]
}

/// Count symbols by label in the database.
fn count_symbols_by_label(db: &MagellanIntegration, label: &str) -> usize {
    db.count_by_label(label).unwrap_or(0)
}

/// Verify that a code chunk matches expected content.
fn verify_code_chunk(
    db: &MagellanIntegration,
    file: &Path,
    start: usize,
    end: usize,
    expected_substring: &str,
) -> bool {
    match db.get_code_chunk(file, start, end) {
        Ok(Some(chunk)) => chunk.contains(expected_substring),
        _ => false,
    }
}

///////////////////////////////////////////////////////////////////////////////
// Task 1: Fixture tests (4 tests)
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_temp_db_creation() {
    let temp_dir = create_temp_magellan_db();
    let db_path = temp_dir.path().join("test.db");

    // Should be able to create and open database
    let db = MagellanIntegration::open(&db_path);
    assert!(db.is_ok(), "Failed to open Magellan database");

    let db = db.unwrap();
    // Query on empty database should return empty results
    let results = db.query_by_labels(&["rust"]);
    assert!(results.is_ok(), "Query failed on empty database");
    assert!(results.unwrap().is_empty(), "Empty database should have no results");
}

#[test]
fn test_sample_file_creation() {
    let temp_dir = create_temp_magellan_db();
    let rust_file = create_sample_rust_file(temp_dir.path());

    // Verify file exists
    assert!(rust_file.exists(), "Sample Rust file should exist");

    // Verify file has content
    let content = fs::read_to_string(&rust_file).unwrap();
    assert!(content.contains("pub struct MyStruct"), "File should contain struct");
    assert!(content.contains("pub fn my_function"), "File should contain function");
    assert!(content.contains("pub trait MyTrait"), "File should contain trait");
    assert!(content.contains("pub enum MyEnum"), "File should contain enum");
}

#[test]
fn test_multilang_workspace_creation() {
    let temp_dir = create_temp_magellan_db();
    let files = create_multilang_workspace(temp_dir.path());

    // Should create 7 files
    assert_eq!(files.len(), 7, "Should create 7 language files");

    // All files should exist
    for file in &files {
        assert!(file.exists(), "File {:?} should exist", file);
    }

    // Verify file extensions
    let extensions: Vec<_> = files
        .iter()
        .filter_map(|f| f.extension())
        .map(|e| e.to_string_lossy().to_string())
        .collect();

    assert!(extensions.contains(&"rs".to_string()), "Should have .rs file");
    assert!(extensions.contains(&"py".to_string()), "Should have .py file");
    assert!(extensions.contains(&"c".to_string()), "Should have .c file");
    assert!(extensions.contains(&"cpp".to_string()), "Should have .cpp file");
    assert!(extensions.contains(&"java".to_string()), "Should have .java file");
    assert!(extensions.contains(&"js".to_string()), "Should have .js file");
    assert!(extensions.contains(&"ts".to_string()), "Should have .ts file");
}

#[test]
fn test_helper_count_by_label() {
    let temp_dir = create_temp_magellan_db();
    let db_path = temp_dir.path().join("test.db");

    let db = MagellanIntegration::open(&db_path).unwrap();

    // Count on empty database should be 0
    let count = count_symbols_by_label(&db, "rust");
    assert_eq!(count, 0, "Empty database should have 0 symbols");

    // Count for non-existent label should also be 0
    let count = count_symbols_by_label(&db, "nonexistent");
    assert_eq!(count, 0, "Non-existent label should have 0 symbols");
}
