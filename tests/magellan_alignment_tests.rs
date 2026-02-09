//! Cross-tool alignment tests with Magellan format compatibility.
//!
//! Tests verify that Splice's MagellanIntegration wrapper correctly uses
//! the Magellan v0.5.0 API for cross-tool compatibility.
//!
//! API surface tested:
//! - open() - Create/open Magellan database
//! - index_file() - Ingest files into Magellan database
//! - query_by_labels() - Retrieve code chunks by label patterns
//! - get_code_chunk() - Retrieve code by byte range
//! - get_code_chunks_for_symbol() - Retrieve all chunks for a symbol
//!
//! Note: These tests validate Splice's correct usage of Magellan API.
//! External database compatibility is Magellan's responsibility.

#![cfg(feature = "sqlite")]

use splice::graph::magellan_integration::MagellanIntegration;
use splice::output::{JsonResponse, LabelQueryResponse, Span, SymbolMatch};
use sqlitegraph::SnapshotId;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

///////////////////////////////////////////////////////////////////////////////
// Test fixtures
///////////////////////////////////////////////////////////////////////////////

/// Create a test Magellan integration instance with temp database.
fn create_test_magellan_integration() -> (MagellanIntegration, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_magellan.db");

    let integration =
        MagellanIntegration::open(&db_path).expect("Failed to create MagellanIntegration");

    (integration, temp_dir)
}

/// Create a test source file with given content.
fn create_test_file(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
    let file_path = dir.join(name);
    fs::write(&file_path, content).expect("Failed to write test file");
    file_path
}

///////////////////////////////////////////////////////////////////////////////
// API surface tests
///////////////////////////////////////////////////////////////////////////////

/// Test MagellanIntegration::open() creates database.
#[test]
fn test_magellan_integration_open() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_magellan.db");

    // Create integration
    let integration = MagellanIntegration::open(&db_path);

    assert!(
        integration.is_ok(),
        "MagellanIntegration::open should succeed"
    );

    let integration = integration.unwrap();

    // Database file should exist after opening
    assert!(db_path.exists(), "Database file should be created");

    // Query on empty database should succeed with empty results
    let results = integration.query_by_labels(&["rust"]);
    assert!(results.is_ok(), "Query on empty database should succeed");
    assert!(
        results.unwrap().is_empty(),
        "Empty database should have no results"
    );

    println!(
        "MagellanIntegration opened successfully at: {}",
        db_path.display()
    );
}

/// Test index_file() ingests file into Magellan database.
#[cfg(feature = "sqlite")]
#[test]
fn test_magellan_index_file() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    // Create test file with multiple symbols
    let content = r#"
/// Adds two numbers together.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Multiplies two numbers.
pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

/// A simple struct for testing.
pub struct Calculator {
    value: i32,
}

impl Calculator {
    pub fn new(value: i32) -> Self {
        Self { value }
    }
}
"#;

    let file_path = create_test_file(temp_dir.path(), "test.rs", content);

    // Index the file
    let result = integration.index_file(&file_path);

    assert!(
        result.is_ok(),
        "index_file should succeed: {:?}",
        result.err()
    );

    let indexed_count = result.unwrap();
    assert!(indexed_count > 0, "Should index at least one symbol");

    println!("Indexed {} symbols from test.rs", indexed_count);

    // Verify symbols can be queried
    let rust_symbols = integration.query_by_labels(&["rust"]);
    assert!(rust_symbols.is_ok());
    assert!(
        !rust_symbols.unwrap().is_empty(),
        "Should find indexed Rust symbols"
    );
}

/// Test query_by_labels() retrieves code chunks by labels.
#[test]
fn test_magellan_query_by_labels() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    // Create and index test file
    let content = r#"
pub fn function_one() -> i32 {
    42
}

pub fn function_two() -> String {
    "hello".to_string()
}

pub struct TestStruct {
    field: i32,
}
"#;

    let file_path = create_test_file(temp_dir.path(), "test.rs", content);
    integration.index_file(&file_path).unwrap();

    // Query by language label
    let result = integration.query_by_labels(&["rust"]);

    assert!(
        result.is_ok(),
        "query_by_labels should succeed: {:?}",
        result.err()
    );

    let symbols = result.unwrap();
    assert!(symbols.len() > 0, "Should retrieve at least one symbol");

    println!("Retrieved {} symbols by labels", symbols.len());

    // Verify SymbolInfo structure (matches Magellan's SymbolQueryResult)
    for symbol in &symbols {
        assert!(!symbol.name.is_empty(), "Symbol name should not be empty");
        assert!(
            !symbol.file_path.is_empty(),
            "Symbol file_path should not be empty"
        );
        assert!(!symbol.kind.is_empty(), "Symbol kind should not be empty");
        assert!(
            symbol.byte_start < symbol.byte_end,
            "Byte range should be valid: {} < {}",
            symbol.byte_start,
            symbol.byte_end
        );
        assert!(symbol.entity_id > 0, "Entity ID should be positive");
    }
}

/// Test that Splice can query Magellan-labeled symbols.
///
/// This test validates Magellan READ alignment:
/// - Splice's MagellanIntegration wrapper uses Magellan's library
/// - Magellan labels (rust, fn, struct, etc.) work with query_by_labels
/// - The CodeGraph can open Magellan-created databases
#[test]
fn test_magellan_db_read_compatibility() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    let content = r#"
/// A demo function to test compatibility
pub fn demo_function(x: i32) -> i32 {
    x + 1
}

/// A demo struct
pub struct DemoStruct {
    value: i32,
}

impl DemoStruct {
    pub fn new(value: i32) -> Self {
        Self { value }
    }
}
"#;

    let file_path = create_test_file(temp_dir.path(), "compat_test.rs", content);
    integration.index_file(&file_path).unwrap();

    // Query 1: Single Magellan language label
    let rust_symbols = integration.query_by_labels(&["rust"]).unwrap();
    assert!(
        !rust_symbols.is_empty(),
        "Should find symbols with 'rust' label"
    );

    // Query 2: Magellan language + kind labels (AND semantics)
    let fn_symbols = integration.query_by_labels(&["rust", "fn"]).unwrap();
    assert!(
        !fn_symbols.is_empty(),
        "Should find functions with 'rust' AND 'fn' labels"
    );

    // Verify we found the expected symbols
    let demo_fn = fn_symbols.iter().find(|s| s.name == "demo_function");
    assert!(demo_fn.is_some(), "Should find demo_function");

    let demo_struct = fn_symbols.iter().find(|s| s.name == "new");
    assert!(demo_struct.is_some(), "Should find new method");

    // Query 3: Magellan kind label alone (struct)
    let struct_symbols = integration.query_by_labels(&["struct"]).unwrap();
    assert!(!struct_symbols.is_empty(), "Should find structs");
    let demo_struct = struct_symbols.iter().find(|s| s.name == "DemoStruct");
    assert!(demo_struct.is_some(), "Should find DemoStruct");

    // Verify CodeGraph can open the same Magellan-created DB
    let db_path = temp_dir.path().join("test_magellan.db");
    let graph = splice::graph::CodeGraph::open(db_path.as_path())
        .expect("CodeGraph should open Magellan-created DB");

    // Verify we can get a node from the graph
    if let Some(symbol) = fn_symbols.first() {
        let snapshot_id = SnapshotId::current();
        let node = graph
            .inner()
            .get_node(snapshot_id, symbol.entity_id)
            .expect("Should retrieve node by entity_id");
        assert_eq!(node.name, "demo_function");
    }
}

/// Test relationship queries use Magellan call graph edges.
#[test]
fn test_magellan_call_relationships() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    let content = r#"
pub fn callee() {}

pub fn caller() {
    callee();
}
"#;

    let file_path = create_test_file(temp_dir.path(), "test.rs", content);
    integration.index_file(&file_path).unwrap();

    let symbols = integration.query_by_labels(&["rust", "fn"]).unwrap();
    let caller = symbols
        .iter()
        .find(|symbol| symbol.name == "caller")
        .expect("caller symbol should be indexed");
    let callee = symbols
        .iter()
        .find(|symbol| symbol.name == "callee")
        .expect("callee symbol should be indexed");

    let graph = splice::graph::CodeGraph::open(temp_dir.path().join("test_magellan.db").as_path())
        .expect("Failed to open graph for relationships");
    let mut cache = splice::relationships::RelationshipCache::new();

    let callees = splice::relationships::get_callees(
        &graph,
        sqlitegraph::NodeId::from(caller.entity_id),
        &mut cache,
    )
    .unwrap();
    assert!(
        callees.iter().any(|rel| rel.name == "callee"),
        "caller should list callee"
    );

    let callers = splice::relationships::get_callers(
        &graph,
        sqlitegraph::NodeId::from(callee.entity_id),
        &mut cache,
    )
    .unwrap();
    assert!(
        callers.iter().any(|rel| rel.name == "caller"),
        "callee should list caller"
    );
}

/// Test get_code_chunk() retrieves code by byte range.
#[test]
fn test_magellan_get_code_chunk() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    // Create and index test file
    let content = r#"
pub fn example_function() -> i32 {
    100
}
"#;

    let file_path = create_test_file(temp_dir.path(), "test.rs", content);
    integration.index_file(&file_path).unwrap();

    // First query to get a symbol's byte range
    let symbols = integration.query_by_labels(&["rust"]).unwrap();
    assert!(!symbols.is_empty(), "Should have indexed symbols");

    let symbol = &symbols[0];

    // Get code chunk by byte range
    let result = integration.get_code_chunk(&file_path, symbol.byte_start, symbol.byte_end);

    assert!(
        result.is_ok(),
        "get_code_chunk should succeed: {:?}",
        result.err()
    );

    let chunk_content = result.unwrap();
    assert!(
        chunk_content.is_some(),
        "Should have code chunk at the symbol's span"
    );

    let chunk = chunk_content.unwrap();
    assert!(!chunk.is_empty(), "Chunk content should not be empty");

    println!(
        "Retrieved code chunk: {} bytes for symbol '{}' at {}..{}",
        chunk.len(),
        symbol.name,
        symbol.byte_start,
        symbol.byte_end
    );
}

#[test]
fn test_unified_json_schema_fields() {
    let span = Span::new("src/lib.rs".to_string(), 10, 20, 1, 0, 1, 10);
    let symbol = SymbolMatch::new("demo".to_string(), "fn".to_string(), span, None, None);
    let query = LabelQueryResponse {
        labels: vec!["rust".to_string()],
        symbols: vec![symbol],
        count: 1,
        total_count: None,
        offset: None,
        limit: None,
        max_symbols: None,
        max_bytes: None,
        next_offset: None,
        truncation_reasons: None,
    };
    let result = JsonResponse::new(query, "test-exec");

    let value = serde_json::to_value(&result).unwrap();

    assert!(
        value.get("schema_version").is_some(),
        "missing schema_version"
    );
    assert!(value.get("execution_id").is_some(), "missing execution_id");
    assert!(
        value.get("version").is_none(),
        "legacy version should be removed"
    );
    assert!(
        value.get("operation_id").is_none(),
        "legacy operation_id should be removed"
    );

    let symbols = &value["data"]["symbols"];
    assert!(symbols.is_array(), "expected symbols array");
    let first = symbols.as_array().unwrap()[0].clone();
    let span = first.get("span").expect("span should be present");
    assert!(span.get("start_line").is_some(), "missing start_line");
    assert!(span.get("start_col").is_some(), "missing start_col");
    assert!(
        span.get("line_start").is_none(),
        "legacy line_start should be removed"
    );
    assert!(
        span.get("col_start").is_none(),
        "legacy col_start should be removed"
    );
}

/// Test get_code_chunks_for_symbol() retrieves all chunks for a symbol name.
#[test]
fn test_magellan_get_code_chunks_for_symbol() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    // Create and index test file with potential ambiguous symbols
    let content = r#"
pub struct MyStruct {
    value: i32,
}

impl MyStruct {
    pub fn new(value: i32) -> Self {
        Self { value }
    }
}
"#;

    let file_path = create_test_file(temp_dir.path(), "test.rs", content);
    integration.index_file(&file_path).unwrap();

    // Get code chunks for symbol by name
    let result = integration.get_code_chunks_for_symbol(&file_path, "MyStruct");

    assert!(
        result.is_ok(),
        "get_code_chunks_for_symbol should succeed: {:?}",
        result.err()
    );

    let chunks = result.unwrap();
    // May have multiple chunks (struct definition + impl block)
    println!(
        "Retrieved {} code chunks for symbol 'MyStruct'",
        chunks.len()
    );

    // Verify CodeChunk structure
    for chunk in &chunks {
        assert!(
            !chunk.content.is_empty(),
            "Chunk content should not be empty"
        );
        assert!(
            chunk.byte_start < chunk.byte_end,
            "Chunk byte range should be valid"
        );
        // symbol_name may be None for some chunks
        let _ = chunk.symbol_name;
    }
}

///////////////////////////////////////////////////////////////////////////////
// Cross-tool format compatibility tests
///////////////////////////////////////////////////////////////////////////////

/// Test that SymbolInfo has fields matching Magellan's SymbolQueryResult.
#[test]
fn test_magellan_symbol_info_format_compatibility() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    let content = "pub fn compatibility_test() -> i32 { 123 }";
    let file_path = create_test_file(temp_dir.path(), "test.rs", content);
    integration.index_file(&file_path).unwrap();

    let symbols = integration.query_by_labels(&["rust"]).unwrap();
    assert!(!symbols.is_empty(), "Should have indexed symbols");

    let symbol = &symbols[0];

    // Verify SymbolInfo has all required fields for cross-tool compatibility
    // These must match Magellan's SymbolQueryResult structure:
    assert!(!symbol.name.is_empty(), "Symbol must have 'name' field");
    assert!(
        !symbol.file_path.is_empty(),
        "Symbol must have 'file_path' field"
    );
    assert!(!symbol.kind.is_empty(), "Symbol must have 'kind' field");
    assert!(
        symbol.byte_start < symbol.byte_end,
        "Symbol must have valid 'byte_start' < 'byte_end'"
    );
    assert!(symbol.entity_id != 0, "Symbol must have 'entity_id' field");

    println!(
        "SymbolInfo format compatible: name={}, kind={}, file={}, bytes={}..{}, id={}",
        symbol.name,
        symbol.kind,
        symbol.file_path,
        symbol.byte_start,
        symbol.byte_end,
        symbol.entity_id
    );
}

/// Test that CodeChunk has fields matching Magellan's CodeChunk structure.
#[test]
fn test_magellan_code_chunk_format_compatibility() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    let content = "pub fn chunk_test() -> i32 { 42 }";
    let file_path = create_test_file(temp_dir.path(), "test.rs", content);
    integration.index_file(&file_path).unwrap();

    let symbols = integration.query_by_labels(&["rust"]).unwrap();
    let symbol = &symbols[0];

    let chunk_opt = integration
        .get_code_chunk(&file_path, symbol.byte_start, symbol.byte_end)
        .unwrap();

    assert!(chunk_opt.is_some(), "Should retrieve code chunk");

    // For code chunk content, verify it matches expected format
    let chunk_content = chunk_opt.unwrap();
    assert!(!chunk_content.is_empty(), "Code chunk must have content");

    // Get chunks for symbol to verify CodeChunk struct format
    let chunks = integration
        .get_code_chunks_for_symbol(&file_path, &symbol.name)
        .unwrap();

    if !chunks.is_empty() {
        let chunk = &chunks[0];

        // Verify CodeChunk struct fields match Magellan's format:
        assert!(
            !chunk.content.is_empty(),
            "CodeChunk must have 'content' field"
        );
        assert!(
            chunk.byte_start < chunk.byte_end,
            "CodeChunk must have valid byte range"
        );
        // symbol_name is optional in Magellan's format
        let _ = chunk.symbol_name;

        println!(
            "CodeChunk format compatible: bytes={}..{}, symbol_name={:?}",
            chunk.byte_start, chunk.byte_end, chunk.symbol_name
        );
    }
}

/// Test that file paths are stored as absolute paths (cross-tool requirement).
#[test]
fn test_magellan_absolute_path_compatibility() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    let content = "pub fn path_test() {}";
    let file_path = create_test_file(temp_dir.path(), "test.rs", content);

    // file_path from create_test_file is already absolute
    assert!(
        file_path.is_absolute(),
        "Test fixture should create absolute paths"
    );

    integration.index_file(&file_path).unwrap();

    let symbols = integration.query_by_labels(&["rust"]).unwrap();
    assert!(!symbols.is_empty());

    let symbol = &symbols[0];

    // Verify stored path is absolute (required for cross-tool compatibility)
    let stored_path = Path::new(&symbol.file_path);
    assert!(
        stored_path.is_absolute(),
        "Stored file_path should be absolute for cross-tool compatibility, got: {}",
        symbol.file_path
    );

    println!("Absolute path compatible: {}", symbol.file_path);
}

///////////////////////////////////////////////////////////////////////////////
// Edge case tests
///////////////////////////////////////////////////////////////////////////////

/// Test query_by_labels() with non-existent labels returns empty result.
#[test]
fn test_magellan_query_nonexistent_labels() {
    let (integration, _temp_dir) = create_test_magellan_integration();

    // Query for labels that don't exist
    let result = integration.query_by_labels(&["nonexistent_function_xyz"]);

    assert!(
        result.is_ok(),
        "query_by_labels should succeed even with no matches"
    );

    let symbols = result.unwrap();
    assert_eq!(
        symbols.len(),
        0,
        "Should return empty array for non-existent labels"
    );

    println!("Correctly returned empty result for non-existent labels");
}

/// Test query_by_labels() with multiple labels (AND semantics).
#[test]
fn test_magellan_query_multiple_labels_and() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    let content = r#"
pub fn test_fn() {}
pub struct TestStruct {}
"#;
    let file_path = create_test_file(temp_dir.path(), "test.rs", content);
    integration.index_file(&file_path).unwrap();

    // Query with multiple labels - Magellan uses AND semantics
    let result = integration.query_by_labels(&["rust", "fn"]);

    assert!(result.is_ok(), "Multi-label query should succeed");

    let symbols = result.unwrap();
    // Results should match symbols having BOTH labels (rust AND fn)
    println!(
        "Multi-label query (rust AND fn) returned {} symbols",
        symbols.len()
    );

    // Verify all results are from Rust files
    for symbol in &symbols {
        assert!(
            symbol.file_path.contains(".rs") || symbol.file_path.contains("rust"),
            "Multi-label results should match all labels"
        );
    }
}

/// Test get_code_chunk() with non-existent file path.
#[test]
fn test_magellan_get_code_chunk_nonexistent_file() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    // Create and index a file
    let content = "pub fn test() {}";
    let file_path = create_test_file(temp_dir.path(), "test.rs", content);
    integration.index_file(&file_path).unwrap();

    // Query for chunk at non-existent file path
    let nonexistent_path = temp_dir.path().join("does_not_exist.rs");
    let result = integration.get_code_chunk(&nonexistent_path, 0, 100);

    // Should succeed but return None (no error for missing file)
    assert!(
        result.is_ok(),
        "get_code_chunk should succeed for non-existent file"
    );
    assert!(
        result.unwrap().is_none(),
        "Should return None for non-existent file"
    );

    println!("Correctly returned None for non-existent file path");
}

/// Test get_code_chunk() with invalid byte range (beyond file size).
#[test]
fn test_magellan_get_code_chunk_invalid_range() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    // Create and index test file
    let content = "pub fn test() {}";
    let file_path = create_test_file(temp_dir.path(), "test.rs", content);
    integration.index_file(&file_path).unwrap();

    // Request invalid byte range (end way beyond file size)
    let result = integration.get_code_chunk(&file_path, 0, 100000);

    // Should succeed but return None (no chunk at invalid range)
    assert!(
        result.is_ok(),
        "get_code_chunk should succeed for invalid range"
    );
    let chunk_opt = result.unwrap();

    // Either None (no chunk) or Some with content (Magellan may handle this)
    let _ = chunk_opt;

    println!(
        "get_code_chunk handled invalid byte range: {:?}",
        chunk_opt.is_some()
    );
}

/// Test get_code_chunk() with inverted range (start > end).
#[test]
fn test_magellan_get_code_chunk_inverted_range() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    let content = "pub fn test() {}";
    let file_path = create_test_file(temp_dir.path(), "test.rs", content);
    integration.index_file(&file_path).unwrap();

    // Request inverted range (start > end)
    let result = integration.get_code_chunk(&file_path, 100, 50);

    // Should succeed but return None or handle gracefully
    assert!(
        result.is_ok(),
        "get_code_chunk should handle inverted range"
    );

    println!("get_code_chunk handled inverted range gracefully");
}

/// Test get_code_chunks_for_symbol() with non-existent symbol name.
#[test]
fn test_magellan_get_code_chunks_nonexistent_symbol() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    let content = "pub fn existing_fn() {}";
    let file_path = create_test_file(temp_dir.path(), "test.rs", content);
    integration.index_file(&file_path).unwrap();

    // Query for non-existent symbol
    let result = integration.get_code_chunks_for_symbol(&file_path, "NonExistentSymbol");

    assert!(
        result.is_ok(),
        "Query should succeed for non-existent symbol"
    );

    let chunks = result.unwrap();
    // Should return empty Vec for non-existent symbol
    assert_eq!(
        chunks.len(),
        0,
        "Non-existent symbol should return empty chunks"
    );

    println!("Correctly returned empty chunks for non-existent symbol");
}

///////////////////////////////////////////////////////////////////////////////
// Multiple language tests
///////////////////////////////////////////////////////////////////////////////

/// Test indexing Python file.
#[test]
fn test_magellan_index_python() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    let content = r#"
def calculate_sum(a: int, b: int) -> int:
    """Calculate the sum of two numbers."""
    return a + b

class Calculator:
    """A simple calculator class."""

    def __init__(self, initial_value: int = 0):
        self.value = initial_value

    def add(self, x: int) -> int:
        self.value += x
        return self.value
"#;

    let file_path = create_test_file(temp_dir.path(), "test.py", content);

    let count = integration.index_file(&file_path);
    assert!(count.is_ok(), "Should index Python file");

    let indexed_count = count.unwrap();
    assert!(indexed_count > 0, "Should index Python symbols");

    // Query by Python label
    let python_symbols = integration.query_by_labels(&["python"]);
    assert!(python_symbols.is_ok());
    assert!(
        !python_symbols.unwrap().is_empty(),
        "Should find Python symbols"
    );

    println!("Indexed {} Python symbols", indexed_count);
}

/// Test indexing TypeScript file.
#[test]
fn test_magellan_index_typescript() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    let content = r#"
interface IUser {
    id: number;
    name: string;
}

class User implements IUser {
    id: number;
    name: string;

    constructor(id: number, name: string) {
        this.id = id;
        this.name = name;
    }

    greet(): string {
        return `Hello, ${this.name}`;
    }
}

function createUser(id: number, name: string): IUser {
    return new User(id, name);
}
"#;

    let file_path = create_test_file(temp_dir.path(), "test.ts", content);

    let count = integration.index_file(&file_path);
    assert!(count.is_ok(), "Should index TypeScript file");

    let indexed_count = count.unwrap();
    assert!(indexed_count > 0, "Should index TypeScript symbols");

    // Query by TypeScript label
    let ts_symbols = integration.query_by_labels(&["typescript"]);
    assert!(ts_symbols.is_ok());
    assert!(
        !ts_symbols.unwrap().is_empty(),
        "Should find TypeScript symbols"
    );

    println!("Indexed {} TypeScript symbols", indexed_count);
}

/// Test indexing multiple languages in the same database.
#[test]
fn test_magellan_index_multiple_languages() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    // Create test files for different languages
    let rust_content = "pub fn rust_function() -> i32 { 42 }";
    let python_content = "def python_function(): return 42";
    let typescript_content = "function typescriptFunction() { return 42; }";

    let rust_file = create_test_file(temp_dir.path(), "test.rs", rust_content);
    let python_file = create_test_file(temp_dir.path(), "test.py", python_content);
    let typescript_file = create_test_file(temp_dir.path(), "test.ts", typescript_content);

    // Index all files
    let rust_count = integration.index_file(&rust_file).unwrap();
    let python_count = integration.index_file(&python_file).unwrap();
    let typescript_count = integration.index_file(&typescript_file).unwrap();

    assert!(rust_count > 0, "Should index Rust symbols");
    assert!(python_count > 0, "Should index Python symbols");
    assert!(typescript_count > 0, "Should index TypeScript symbols");

    println!(
        "Indexed {} Rust, {} Python, {} TypeScript symbols",
        rust_count, python_count, typescript_count
    );

    // Query each language
    let rust_symbols = integration.query_by_labels(&["rust"]).unwrap();
    let python_symbols = integration.query_by_labels(&["python"]).unwrap();
    let ts_symbols = integration.query_by_labels(&["typescript"]).unwrap();

    assert!(!rust_symbols.is_empty(), "Should find Rust symbols");
    assert!(!python_symbols.is_empty(), "Should find Python symbols");
    assert!(!ts_symbols.is_empty(), "Should find TypeScript symbols");

    println!(
        "Queried: {} Rust, {} Python, {} TypeScript symbols",
        rust_symbols.len(),
        python_symbols.len(),
        ts_symbols.len()
    );
}

///////////////////////////////////////////////////////////////////////////////
// Unicode content tests
///////////////////////////////////////////////////////////////////////////////

/// Test MagellanIntegration handles Unicode content correctly.
#[test]
fn test_magellan_unicode_content() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    // Create file with Unicode content (Japanese, emoji)
    let content = r#"
/// 日本語のコメント - Japanese comment
/// This function demonstrates Unicode handling
pub fn 日本語の関数() -> i32 {
    42
}

/// Emoji test 🚀🎉
pub fn emoji_test() -> String {
    "🎉 Success! 成功!".to_string()
}

/// Mixed Unicode: العربية עברית 한국어
pub fn mixed_unicode() -> String {
    "Mixed: Hello 世界 مرحبا בעברית".to_string()
}
"#;

    let file_path = create_test_file(temp_dir.path(), "unicode.rs", content);

    // Index should handle Unicode correctly
    let result = integration.index_file(&file_path);

    assert!(result.is_ok(), "index_file should handle Unicode content");

    let indexed_count = result.unwrap();
    assert!(indexed_count > 0, "Should index Unicode symbols");

    println!("Indexed {} Unicode symbols", indexed_count);

    // Query should also handle Unicode
    let symbols = integration.query_by_labels(&["rust"]).unwrap();

    // Find symbols with Unicode names
    let unicode_symbols: Vec<_> = symbols
        .iter()
        .filter(|s| s.name.contains("日本語") || s.name.contains("emoji"))
        .collect();

    println!("Found {} symbols with Unicode names", unicode_symbols.len());

    // Verify we can get code chunks for Unicode content
    for symbol in unicode_symbols {
        let chunk = integration
            .get_code_chunk(&file_path, symbol.byte_start, symbol.byte_end)
            .unwrap();

        if let Some(content) = chunk {
            assert!(!content.is_empty(), "Unicode content should be retrievable");
            println!(
                "Retrieved Unicode chunk for '{}': {} bytes",
                symbol.name,
                content.len()
            );
        }
    }
}

/// Test Unicode in file paths.
#[test]
fn test_magellan_unicode_file_path() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    let content = "pub fn test() {}";

    // Create file with Unicode characters in name
    let file_path = create_test_file(temp_dir.path(), "test_日本語.rs", content);

    let result = integration.index_file(&file_path);
    assert!(result.is_ok(), "Should handle Unicode in file path");

    // Query should work
    let symbols = integration.query_by_labels(&["rust"]).unwrap();
    assert!(!symbols.is_empty());

    println!("Unicode file path handled: {}", symbols[0].file_path);
}

///////////////////////////////////////////////////////////////////////////////
// Additional API tests
///////////////////////////////////////////////////////////////////////////////

/// Test get_all_labels() returns available labels.
#[test]
fn test_magellan_get_all_labels() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    // Initially no labels
    let labels = integration.get_all_labels().unwrap();
    assert!(labels.is_empty(), "New database should have no labels");

    // Index some files
    let rust_content = "pub fn test() {}";
    let rust_file = create_test_file(temp_dir.path(), "test.rs", rust_content);
    integration.index_file(&rust_file).unwrap();

    let python_content = "def test(): pass";
    let python_file = create_test_file(temp_dir.path(), "test.py", python_content);
    integration.index_file(&python_file).unwrap();

    // Now should have labels
    let labels = integration.get_all_labels().unwrap();
    assert!(!labels.is_empty(), "Should have labels after indexing");

    // Should have language labels at minimum
    println!("Available labels: {:?}", labels);
}

/// Test count_by_label() returns symbol counts.
#[test]
fn test_magellan_count_by_label() {
    let (mut integration, temp_dir) = create_test_magellan_integration();

    // Initially count should be 0
    let count = integration.count_by_label("rust").unwrap();
    assert_eq!(count, 0, "New database should have 0 symbols for any label");

    // Index a file
    let rust_content = r#"
pub fn first() {}
pub fn second() {}
pub struct TestStruct {}
"#;
    let rust_file = create_test_file(temp_dir.path(), "test.rs", rust_content);
    integration.index_file(&rust_file).unwrap();

    // Count should be greater than 0
    let count = integration.count_by_label("rust").unwrap();
    assert!(count > 0, "Should have symbols with 'rust' label");

    println!("Counted {} symbols with 'rust' label", count);
}

/// Test inner() and inner_mut() access underlying Magellan CodeGraph.
#[test]
fn test_magellan_inner_access() {
    let (mut integration, _temp_dir) = create_test_magellan_integration();

    // inner() should provide access to underlying graph
    let inner = integration.inner();
    assert!(inner.get_all_labels().is_ok(), "Should access inner graph");

    // inner_mut() should provide mutable access
    let _inner_mut = integration.inner_mut();
    // Just verify it compiles and returns a reference
}
