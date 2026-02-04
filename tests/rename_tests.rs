//! Integration tests for cross-file rename operations.
//!
//! These tests verify the complete rename workflow including:
//! - Single file rename
//! - Preview mode (no filesystem modifications)
//! - Backup creation and rollback
//! - Multi-file rename with transaction safety
//! - Cross-language rename (Rust, Python, C, C++, Java, JavaScript, TypeScript)

use splice::graph::rename::{
    apply_replacements_in_file, create_rename_backup, generate_colored_preview,
    generate_preview_diff, group_references_by_file, simulate_replacements,
    simulate_replacements_content, RenameBackupManifest, RenameTransaction,
};
use tempfile::TempDir;

/// Helper to create a test file with content
fn create_test_file(dir: &TempDir, path: &str, content: &str) -> std::path::PathBuf {
    let file_path = dir.path().join(path);
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&file_path, content).unwrap();
    file_path
}

/// Helper to create a ReferenceFact for testing
fn create_reference(
    file_path: &str,
    byte_start: usize,
    byte_end: usize,
) -> magellan::references::ReferenceFact {
    magellan::references::ReferenceFact {
        file_path: std::path::PathBuf::from(file_path),
        referenced_symbol: "old_name".to_string(),
        byte_start,
        byte_end,
        start_line: 1,
        start_col: byte_start,
        end_line: 1,
        end_col: byte_end,
    }
}

#[test]
fn test_rename_single_file() {
    let temp_dir = TempDir::new().unwrap();

    // Create test file with symbol usages
    let content = "fn old_name() {\n    old_name();\n    old_name();\n}\n";
    let file_path = create_test_file(&temp_dir, "test.rs", content);

    // Create references (descending order)
    let references = vec![
        create_reference(file_path.to_str().unwrap(), 36, 44),
        create_reference(file_path.to_str().unwrap(), 20, 28),
    ];

    // Apply replacements
    let count =
        apply_replacements_in_file(&file_path, "old_name", "new_name", &references).unwrap();

    assert_eq!(count, 2);

    // Verify result
    let result = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(
        result,
        "fn old_name() {\n    new_name();\n    new_name();\n}\n"
    );
}

#[test]
fn test_rename_preview_mode() {
    let temp_dir = TempDir::new().unwrap();

    // Create test file
    let content = "fn foo() {\n    println!(\"foo\");\n}\n";
    let file_path = create_test_file(&temp_dir, "lib.rs", content);

    // Create references - both the function name and the string (descending order)
    // "foo" at byte 3-6 (function name)
    // "foo" at byte 25-28 (inside the string: f at 25, o at 26-27, " at 28)
    let references = vec![
        create_reference(file_path.to_str().unwrap(), 25, 28),
        create_reference(file_path.to_str().unwrap(), 3, 6),
    ];

    // Simulate replacements without modifying files
    let modified = simulate_replacements_content(content, &references, "foo", "bar").unwrap();

    // Verify simulation produced expected result
    assert_eq!(modified, "fn bar() {\n    println!(\"bar\");\n}\n");

    // Verify original file was NOT modified (preview is pure)
    let original = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(original, content);
}

#[test]
fn test_rename_creates_backup() {
    let temp_dir = TempDir::new().unwrap();

    // Create test files
    let file1 = create_test_file(&temp_dir, "src/main.rs", "fn foo() {}\n");
    let file2 = create_test_file(&temp_dir, "src/lib.rs", "fn bar() {}\n");

    // Create backup
    let backup_dir = create_rename_backup(
        temp_dir.path(),
        "test_symbol",
        &[file1.clone(), file2.clone()],
    )
    .unwrap();

    // Verify backup directory structure
    assert!(backup_dir.exists());
    assert!(backup_dir.starts_with(temp_dir.path().join(".splice/backups")));

    // Verify operation ID format
    let dir_name = backup_dir.file_name().unwrap().to_str().unwrap();
    assert!(dir_name.starts_with("rename-test_symbol-"));

    // Verify manifest.json exists
    let manifest_path = backup_dir.join("manifest.json");
    assert!(manifest_path.exists());

    // Verify manifest contents
    let manifest: RenameBackupManifest =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();

    assert_eq!(manifest.files.len(), 2);
    assert!(manifest.files.contains_key("src/main.rs"));
    assert!(manifest.files.contains_key("src/lib.rs"));

    // Verify files were copied to backup
    let backup_file1 = backup_dir.join("src/main.rs");
    let backup_file2 = backup_dir.join("src/lib.rs");
    assert!(backup_file1.exists());
    assert!(backup_file2.exists());

    // Verify content matches
    let original_content = std::fs::read_to_string(&file1).unwrap();
    let backup_content = std::fs::read_to_string(&backup_file1).unwrap();
    assert_eq!(original_content, backup_content);
}

#[test]
fn test_generate_preview_diff() {
    let original = "fn foo() {\n    println!(\"foo\");\n}\n";
    let modified = "fn bar() {\n    println!(\"bar\");\n}\n";
    let file_path = std::path::PathBuf::from("test.rs");

    let diff = generate_preview_diff(&file_path, original, modified);

    // Should contain unified diff headers
    assert!(diff.contains("--- a/test.rs"));
    assert!(diff.contains("+++ b/test.rs"));

    // Should show the changes
    assert!(diff.contains("-fn foo()"));
    assert!(diff.contains("+fn bar()"));
    assert!(diff.contains("-    println!(\"foo\");"));
    assert!(diff.contains("+    println!(\"bar\");"));
}

#[test]
fn test_generate_preview_diff_no_changes() {
    let content = "fn foo() {}\n";
    let file_path = std::path::PathBuf::from("test.rs");

    let diff = generate_preview_diff(&file_path, content, content);

    // Should be empty when there are no changes
    assert!(diff.is_empty());
}

#[test]
fn test_generate_colored_preview() {
    let original = "fn foo() {}\n";
    let modified = "fn bar() {}\n";
    let file_path = std::path::PathBuf::from("test.rs");

    let colored = generate_colored_preview(&file_path, original, modified);

    // Should always contain diff content (colored or plain)
    assert!(!colored.is_empty());

    // In plain text mode (non-TTY), should show +/- prefixes
    if !colored.contains('\x1b') {
        assert!(colored.contains("-fn foo()"));
        assert!(colored.contains("+fn bar()"));
    }
}

#[test]
fn test_group_references_by_file() {
    let references = vec![
        create_reference("/src/a.rs", 100, 103),
        create_reference("/src/b.rs", 50, 53),
        create_reference("/src/a.rs", 20, 23),
        create_reference("/src/b.rs", 10, 13),
    ];

    let grouped = group_references_by_file(&references);

    assert_eq!(grouped.len(), 2);

    // Verify each file's refs are sorted descending by byte_start
    let a_refs = grouped.get(&std::path::PathBuf::from("/src/a.rs")).unwrap();
    assert_eq!(a_refs[0].byte_start, 100);
    assert_eq!(a_refs[1].byte_start, 20);

    let b_refs = grouped.get(&std::path::PathBuf::from("/src/b.rs")).unwrap();
    assert_eq!(b_refs[0].byte_start, 50);
    assert_eq!(b_refs[1].byte_start, 10);
}

#[test]
fn test_simulate_replacements() {
    let references = vec![
        create_reference("/src/a.rs", 100, 103),
        create_reference("/src/b.rs", 50, 53),
        create_reference("/src/a.rs", 20, 23),
    ];

    let simulation = simulate_replacements(&references);

    assert_eq!(simulation.len(), 2);
    assert_eq!(
        simulation.get(&std::path::PathBuf::from("/src/a.rs")),
        Some(&2)
    );
    assert_eq!(
        simulation.get(&std::path::PathBuf::from("/src/b.rs")),
        Some(&1)
    );
}

#[test]
fn test_rename_transaction_rollback() {
    let temp_dir = TempDir::new().unwrap();

    // Create original file
    let file_path = create_test_file(&temp_dir, "test.rs", "fn original() {}\n");
    let original_content = "fn original() {}\n";

    // Create backup directory
    let backup_dir = temp_dir.path().join(".splice/backups/test-rollback");
    std::fs::create_dir_all(&backup_dir).unwrap();
    let backup_file = backup_dir.join("test.rs");
    std::fs::write(&backup_file, original_content).unwrap();

    // Create manifest
    let manifest = RenameBackupManifest {
        operation_id: "test-rollback".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        files: std::collections::HashMap::from([(
            "test.rs".to_string(),
            "dummy_checksum".to_string(),
        )]),
    };
    let manifest_path = backup_dir.join("manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    // Modify the file
    std::fs::write(&file_path, "fn modified() {}\n").unwrap();

    // Rollback
    let txn = RenameTransaction::new().with_backup(backup_dir, temp_dir.path().to_path_buf());
    txn.rollback().unwrap();

    // Verify file was restored
    let restored_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(restored_content, original_content);
}

#[test]
fn test_rename_transaction_with_multiple_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple test files
    let file1 = create_test_file(&temp_dir, "src/a.rs", "fn foo() {}\n");
    let file2 = create_test_file(&temp_dir, "src/b.rs", "fn foo() {}\n");

    // Create references for both files
    let refs1 = vec![create_reference(file1.to_str().unwrap(), 3, 6)];
    let refs2 = vec![create_reference(file2.to_str().unwrap(), 3, 6)];

    // Apply replacements
    apply_replacements_in_file(&file1, "foo", "bar", &refs1).unwrap();
    apply_replacements_in_file(&file2, "foo", "bar", &refs2).unwrap();

    // Verify both files were modified
    let content1 = std::fs::read_to_string(&file1).unwrap();
    let content2 = std::fs::read_to_string(&file2).unwrap();
    assert_eq!(content1, "fn bar() {}\n");
    assert_eq!(content2, "fn bar() {}\n");
}

#[test]
fn test_rename_with_multibyte_utf8() {
    let temp_dir = TempDir::new().unwrap();

    // Create file with multibyte UTF-8 characters
    let content = "fn foo() { // 世界\n}\n";
    let file_path = create_test_file(&temp_dir, "test.rs", content);

    // Create reference
    let references = vec![create_reference(file_path.to_str().unwrap(), 3, 6)];

    // Apply replacement
    apply_replacements_in_file(&file_path, "foo", "bar", &references).unwrap();

    // Verify multibyte characters are preserved
    let result = std::fs::read_to_string(&file_path).unwrap();
    assert!(result.contains("世界"));
    assert!(result.contains("bar()"));
}

#[test]
fn test_rename_preview_with_multiple_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple test files
    let file1 = create_test_file(&temp_dir, "src/a.rs", "fn old_name() {}\n");
    let file2 = create_test_file(&temp_dir, "src/b.rs", "fn old_name() {}\n");

    // Create references
    let refs1 = vec![create_reference(file1.to_str().unwrap(), 3, 11)];
    let refs2 = vec![create_reference(file2.to_str().unwrap(), 3, 11)];

    // Simulate replacements (preview mode)
    let content1 = std::fs::read_to_string(&file1).unwrap();
    let content2 = std::fs::read_to_string(&file2).unwrap();

    let modified1 =
        simulate_replacements_content(&content1, &refs1, "old_name", "new_name").unwrap();
    let modified2 =
        simulate_replacements_content(&content2, &refs2, "old_name", "new_name").unwrap();

    // Verify previews show changes
    assert_eq!(modified1, "fn new_name() {}\n");
    assert_eq!(modified2, "fn new_name() {}\n");

    // Verify original files unchanged (preview is pure)
    let original1 = std::fs::read_to_string(&file1).unwrap();
    let original2 = std::fs::read_to_string(&file2).unwrap();
    assert_eq!(original1, "fn old_name() {}\n");
    assert_eq!(original2, "fn old_name() {}\n");
}

#[test]
fn test_backup_preserves_directory_structure() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested test files
    let file1 = create_test_file(&temp_dir, "src/api/handlers.rs", "pub fn handler() {}\n");
    let file2 = create_test_file(
        &temp_dir,
        "tests/integration_test.rs",
        "#[test]\nfn test() {}\n",
    );

    // Create backup
    let backup_dir = create_rename_backup(
        temp_dir.path(),
        "nested_test",
        &[file1.clone(), file2.clone()],
    )
    .unwrap();

    // Verify nested structure preserved
    let backup_file1 = backup_dir.join("src/api/handlers.rs");
    let backup_file2 = backup_dir.join("tests/integration_test.rs");
    assert!(backup_file1.exists());
    assert!(backup_file2.exists());

    // Verify manifest has correct relative paths
    let manifest: RenameBackupManifest =
        serde_json::from_str(&std::fs::read_to_string(backup_dir.join("manifest.json")).unwrap())
            .unwrap();

    assert!(manifest.files.contains_key("src/api/handlers.rs"));
    assert!(manifest.files.contains_key("tests/integration_test.rs"));
}

#[test]
fn test_transaction_track_modified() {
    let mut txn = RenameTransaction::new();

    txn.track_modified(std::path::PathBuf::from("/path/to/file1.rs"));
    txn.track_modified(std::path::PathBuf::from("/path/to/file2.rs"));

    assert_eq!(txn.modified_count(), 2);
    assert_eq!(
        txn.modified_files(),
        &[
            std::path::PathBuf::from("/path/to/file1.rs"),
            std::path::PathBuf::from("/path/to/file2.rs")
        ]
    );
}

// ============================================================================
// Cross-Language Rename Tests (Tasks 1-4)
// ============================================================================

/// Helper to find the byte spans of a symbol in a file.
///
/// This is a simplified approach for testing when Magellan's reference
/// extraction is not available for certain languages (e.g., Rust).
/// It finds all occurrences of a symbol name in the source code.
fn find_symbol_spans(source: &str, symbol_name: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut offset = 0;

    while let Some(pos) = source[offset..].find(symbol_name) {
        let abs_pos = offset + pos;

        // Check if this looks like an identifier (not part of a larger word)
        let before_ok = abs_pos == 0
            || !source
                .chars()
                .nth(abs_pos - 1)
                .map_or(false, |c| c.is_alphanumeric() || c == '_');
        let after_ok = abs_pos + symbol_name.len() >= source.len()
            || !source
                .chars()
                .nth(abs_pos + symbol_name.len())
                .map_or(false, |c| c.is_alphanumeric() || c == '_');

        if before_ok && after_ok {
            spans.push((abs_pos, abs_pos + symbol_name.len()));
        }

        offset = abs_pos + symbol_name.len();
    }

    // Sort by byte_start descending for safe replacement
    spans.sort_by(|a, b| b.0.cmp(&a.0));
    spans
}

#[test]
fn test_rename_rust_function() {
    let temp_dir = TempDir::new().unwrap();

    // Create test Rust file with recursive call
    let content = "fn old_name() {\n    old_name();\n}\n";
    let file_path = create_test_file(&temp_dir, "src/main.rs", content);

    // Find all spans of "old_name" manually (since Magellan doesn't extract Rust references yet)
    let spans = find_symbol_spans(content, "old_name");
    assert_eq!(spans.len(), 2, "Should find 2 occurrences of old_name");

    // Create ReferenceFact entries from the spans
    let file_path_str = file_path.to_str().unwrap();
    let refs: Vec<magellan::references::ReferenceFact> = spans
        .into_iter()
        .map(|(start, end)| magellan::references::ReferenceFact {
            file_path: std::path::PathBuf::from(file_path_str),
            referenced_symbol: "old_name".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1, // Simplified - we don't compute actual line numbers
            start_col: start,
            end_line: 1,
            end_col: end,
        })
        .collect();

    // Apply replacements
    let grouped = group_references_by_file(&refs);
    for (file_path, refs) in grouped {
        apply_replacements_in_file(&file_path, "old_name", "new_name", &refs).unwrap();
    }

    // Verify result
    let result = std::fs::read_to_string(&file_path).unwrap();
    assert!(
        result.contains("new_name()"),
        "Result should contain new_name()"
    );
    assert!(
        !result.contains("old_name()"),
        "Result should not contain old_name()"
    );
}

#[test]
fn test_rename_python_function() {
    let temp_dir = TempDir::new().unwrap();

    // Create test Python file with function and call
    let content = "def old_name():\n    old_name()\n";
    let file_path = create_test_file(&temp_dir, "test.py", content);

    // Find all spans of "old_name" manually
    let spans = find_symbol_spans(content, "old_name");
    assert_eq!(spans.len(), 2, "Should find 2 occurrences of old_name");

    // Create ReferenceFact entries from the spans
    let file_path_str = file_path.to_str().unwrap();
    let refs: Vec<magellan::references::ReferenceFact> = spans
        .into_iter()
        .map(|(start, end)| magellan::references::ReferenceFact {
            file_path: std::path::PathBuf::from(file_path_str),
            referenced_symbol: "old_name".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1,
            start_col: start,
            end_line: 1,
            end_col: end,
        })
        .collect();

    // Apply replacements
    let grouped = group_references_by_file(&refs);
    for (file_path, refs) in grouped {
        apply_replacements_in_file(&file_path, "old_name", "new_name", &refs).unwrap();
    }

    // Verify result
    let result = std::fs::read_to_string(&file_path).unwrap();
    assert!(
        result.contains("new_name"),
        "Result should contain new_name"
    );
    assert!(
        !result.contains("old_name"),
        "Result should not contain old_name"
    );
}

#[test]
fn test_rename_javascript_function() {
    let temp_dir = TempDir::new().unwrap();

    // Create test JavaScript file
    let content = "function old_name() {\n    old_name();\n}\n";
    let file_path = create_test_file(&temp_dir, "test.js", content);

    // Find all spans of "old_name" manually
    let spans = find_symbol_spans(content, "old_name");
    assert_eq!(spans.len(), 2, "Should find 2 occurrences of old_name");

    // Create ReferenceFact entries from the spans
    let file_path_str = file_path.to_str().unwrap();
    let refs: Vec<magellan::references::ReferenceFact> = spans
        .into_iter()
        .map(|(start, end)| magellan::references::ReferenceFact {
            file_path: std::path::PathBuf::from(file_path_str),
            referenced_symbol: "old_name".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1,
            start_col: start,
            end_line: 1,
            end_col: end,
        })
        .collect();

    // Apply replacements
    let grouped = group_references_by_file(&refs);
    for (file_path, refs) in grouped {
        apply_replacements_in_file(&file_path, "old_name", "new_name", &refs).unwrap();
    }

    // Verify result
    let result = std::fs::read_to_string(&file_path).unwrap();
    assert!(
        result.contains("new_name"),
        "Result should contain new_name"
    );
    assert!(
        !result.contains("old_name"),
        "Result should not contain old_name"
    );
}

#[test]
fn test_rename_typescript_function() {
    let temp_dir = TempDir::new().unwrap();

    // Create test TypeScript file with type annotation
    let content = "function old_name(): void {\n    old_name();\n}\n";
    let file_path = create_test_file(&temp_dir, "test.ts", content);

    // Find all spans of "old_name" manually
    let spans = find_symbol_spans(content, "old_name");
    assert_eq!(spans.len(), 2, "Should find 2 occurrences of old_name");

    // Create ReferenceFact entries from the spans
    let file_path_str = file_path.to_str().unwrap();
    let refs: Vec<magellan::references::ReferenceFact> = spans
        .into_iter()
        .map(|(start, end)| magellan::references::ReferenceFact {
            file_path: std::path::PathBuf::from(file_path_str),
            referenced_symbol: "old_name".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1,
            start_col: start,
            end_line: 1,
            end_col: end,
        })
        .collect();

    // Apply replacements
    let grouped = group_references_by_file(&refs);
    for (file_path, refs) in grouped {
        apply_replacements_in_file(&file_path, "old_name", "new_name", &refs).unwrap();
    }

    // Verify result
    let result = std::fs::read_to_string(&file_path).unwrap();
    assert!(
        result.contains("new_name"),
        "Result should contain new_name"
    );
    assert!(
        !result.contains("old_name"),
        "Result should not contain old_name"
    );
}

#[test]
fn test_rename_c_function() {
    let temp_dir = TempDir::new().unwrap();

    // Create test C file
    let content = "void old_name() {\n    old_name();\n}\n";
    let file_path = create_test_file(&temp_dir, "test.c", content);

    // Find all spans of "old_name" manually
    let spans = find_symbol_spans(content, "old_name");
    assert_eq!(spans.len(), 2, "Should find 2 occurrences of old_name");

    // Create ReferenceFact entries from the spans
    let file_path_str = file_path.to_str().unwrap();
    let refs: Vec<magellan::references::ReferenceFact> = spans
        .into_iter()
        .map(|(start, end)| magellan::references::ReferenceFact {
            file_path: std::path::PathBuf::from(file_path_str),
            referenced_symbol: "old_name".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1,
            start_col: start,
            end_line: 1,
            end_col: end,
        })
        .collect();

    // Apply replacements
    let grouped = group_references_by_file(&refs);
    for (file_path, refs) in grouped {
        apply_replacements_in_file(&file_path, "old_name", "new_name", &refs).unwrap();
    }

    // Verify result
    let result = std::fs::read_to_string(&file_path).unwrap();
    assert!(
        result.contains("new_name"),
        "Result should contain new_name"
    );
    assert!(
        !result.contains("old_name"),
        "Result should not contain old_name"
    );
}

#[test]
fn test_rename_cpp_method() {
    let temp_dir = TempDir::new().unwrap();

    // Create test C++ file with class method
    let content = "class MyClass {\npublic:\n    void old_name() { old_name(); }\n};\n";
    let file_path = create_test_file(&temp_dir, "test.cpp", content);

    // Find all spans of "old_name" manually
    let spans = find_symbol_spans(content, "old_name");
    assert_eq!(spans.len(), 2, "Should find 2 occurrences of old_name");

    // Create ReferenceFact entries from the spans
    let file_path_str = file_path.to_str().unwrap();
    let refs: Vec<magellan::references::ReferenceFact> = spans
        .into_iter()
        .map(|(start, end)| magellan::references::ReferenceFact {
            file_path: std::path::PathBuf::from(file_path_str),
            referenced_symbol: "old_name".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1,
            start_col: start,
            end_line: 1,
            end_col: end,
        })
        .collect();

    // Apply replacements
    let grouped = group_references_by_file(&refs);
    for (file_path, refs) in grouped {
        apply_replacements_in_file(&file_path, "old_name", "new_name", &refs).unwrap();
    }

    // Verify result
    let result = std::fs::read_to_string(&file_path).unwrap();
    assert!(
        result.contains("new_name"),
        "Result should contain new_name"
    );
    assert!(
        !result.contains("old_name"),
        "Result should not contain old_name"
    );
}

#[test]
fn test_rename_java_method() {
    let temp_dir = TempDir::new().unwrap();

    // Create test Java file
    let content = "public class Test {\n    void old_name() {\n        old_name();\n    }\n}\n";
    let file_path = create_test_file(&temp_dir, "Test.java", content);

    // Find all spans of "old_name" manually
    let spans = find_symbol_spans(content, "old_name");
    assert_eq!(spans.len(), 2, "Should find 2 occurrences of old_name");

    // Create ReferenceFact entries from the spans
    let file_path_str = file_path.to_str().unwrap();
    let refs: Vec<magellan::references::ReferenceFact> = spans
        .into_iter()
        .map(|(start, end)| magellan::references::ReferenceFact {
            file_path: std::path::PathBuf::from(file_path_str),
            referenced_symbol: "old_name".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1,
            start_col: start,
            end_line: 1,
            end_col: end,
        })
        .collect();

    // Apply replacements
    let grouped = group_references_by_file(&refs);
    for (file_path, refs) in grouped {
        apply_replacements_in_file(&file_path, "old_name", "new_name", &refs).unwrap();
    }

    // Verify result
    let result = std::fs::read_to_string(&file_path).unwrap();
    assert!(
        result.contains("new_name"),
        "Result should contain new_name"
    );
    assert!(
        !result.contains("old_name"),
        "Result should not contain old_name"
    );
}

// ============================================================================
// Multi-File Cross-Language Tests (Task 5)
// ============================================================================

#[test]
fn test_rename_cross_file_rust() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple Rust files
    let main_content = "mod lib;\nfn main() {\n    lib::old_name();\n}\n";
    let main_rs = create_test_file(&temp_dir, "src/main.rs", main_content);
    let lib_content = "pub fn old_name() {\n    old_name();\n}\n";
    let lib_rs = create_test_file(&temp_dir, "src/lib.rs", lib_content);

    // Manually find all occurrences of "old_name" across files
    // main.rs: 1 occurrence (in lib::old_name())
    // lib.rs: 2 occurrences (definition + recursive call)
    let main_spans = find_symbol_spans(main_content, "old_name");
    let lib_spans = find_symbol_spans(lib_content, "old_name");

    assert_eq!(main_spans.len(), 1, "Should find 1 occurrence in main.rs");
    assert_eq!(lib_spans.len(), 2, "Should find 2 occurrences in lib.rs");

    // Create ReferenceFact entries for both files
    let mut refs = Vec::new();

    for (start, end) in main_spans {
        refs.push(magellan::references::ReferenceFact {
            file_path: main_rs.clone(),
            referenced_symbol: "old_name".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1,
            start_col: start,
            end_line: 1,
            end_col: end,
        });
    }

    for (start, end) in lib_spans {
        refs.push(magellan::references::ReferenceFact {
            file_path: lib_rs.clone(),
            referenced_symbol: "old_name".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1,
            start_col: start,
            end_line: 1,
            end_col: end,
        });
    }

    // Apply replacements
    let grouped = group_references_by_file(&refs);
    for (file_path, refs) in grouped {
        apply_replacements_in_file(&file_path, "old_name", "new_name", &refs).unwrap();
    }

    // Verify both files were updated
    let main_content = std::fs::read_to_string(&main_rs).unwrap();
    let lib_content = std::fs::read_to_string(&lib_rs).unwrap();

    assert!(
        main_content.contains("new_name()"),
        "main.rs should contain new_name"
    );
    assert!(
        lib_content.contains("new_name()"),
        "lib.rs should contain new_name"
    );
    assert!(
        !main_content.contains("old_name"),
        "main.rs should not contain old_name"
    );
    assert!(
        !lib_content.contains("old_name"),
        "lib.rs should not contain old_name"
    );
}

#[test]
fn test_rename_cross_file_python() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple Python files
    let main_content = "from lib import old_name\nold_name()\n";
    let main_py = create_test_file(&temp_dir, "main.py", main_content);
    let lib_content = "def old_name():\n    old_name()\n";
    let lib_py = create_test_file(&temp_dir, "lib.py", lib_content);

    // Find all occurrences of "old_name" across files
    // main.py: 2 occurrences (import + call)
    // lib.py: 2 occurrences (definition + recursive call)
    let main_spans = find_symbol_spans(main_content, "old_name");
    let lib_spans = find_symbol_spans(lib_content, "old_name");

    assert_eq!(main_spans.len(), 2, "Should find 2 occurrences in main.py");
    assert_eq!(lib_spans.len(), 2, "Should find 2 occurrences in lib.py");

    // Create ReferenceFact entries for both files
    let mut refs = Vec::new();

    for (start, end) in main_spans {
        refs.push(magellan::references::ReferenceFact {
            file_path: main_py.clone(),
            referenced_symbol: "old_name".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1,
            start_col: start,
            end_line: 1,
            end_col: end,
        });
    }

    for (start, end) in lib_spans {
        refs.push(magellan::references::ReferenceFact {
            file_path: lib_py.clone(),
            referenced_symbol: "old_name".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1,
            start_col: start,
            end_line: 1,
            end_col: end,
        });
    }

    // Apply replacements
    let grouped = group_references_by_file(&refs);
    for (file_path, refs) in grouped {
        apply_replacements_in_file(&file_path, "old_name", "new_name", &refs).unwrap();
    }

    // Verify both files were updated
    let main_content = std::fs::read_to_string(&main_py).unwrap();
    let lib_content = std::fs::read_to_string(&lib_py).unwrap();

    assert!(
        main_content.contains("new_name"),
        "main.py should contain new_name"
    );
    assert!(
        lib_content.contains("new_name"),
        "lib.py should contain new_name"
    );
    assert!(
        !main_content.contains("old_name"),
        "main.py should not contain old_name"
    );
    assert!(
        !lib_content.contains("old_name"),
        "lib.py should not contain old_name"
    );
}

// ============================================================================
// Byte-Accuracy Tests (Task 6 - Part of Must-Haves)
// ============================================================================

#[test]
fn test_rename_byte_accuracy_no_false_positives() {
    let temp_dir = TempDir::new().unwrap();

    // Create file with similar-looking but different symbols
    let content = "fn foo() {\n    let foo_bar = 1;\n    foo();\n}\n";
    let file_path = create_test_file(&temp_dir, "test.rs", content);

    // Find exact byte spans of "foo" (not "foo_bar")
    let spans = find_symbol_spans(content, "foo");
    // This should find 2 occurrences: "foo" at position 3 and "foo" at position ~30
    // But NOT "foo_bar" because our helper checks word boundaries
    assert_eq!(
        spans.len(),
        2,
        "Should find exactly 2 'foo' occurrences (not foo_bar)"
    );

    // Create references for all "foo" occurrences
    let file_path_str = file_path.to_str().unwrap();
    let references: Vec<magellan::references::ReferenceFact> = spans
        .into_iter()
        .map(|(start, end)| magellan::references::ReferenceFact {
            file_path: std::path::PathBuf::from(file_path_str),
            referenced_symbol: "foo".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1,
            start_col: start,
            end_line: 1,
            end_col: end,
        })
        .collect();

    // Apply replacements
    apply_replacements_in_file(&file_path, "foo", "baz", &references).unwrap();

    // Verify result
    let result = std::fs::read_to_string(&file_path).unwrap();
    assert!(result.contains("fn baz()"), "Should rename foo to baz");
    assert!(result.contains("baz();"), "Should rename foo call");
    assert!(result.contains("foo_bar"), "Should NOT rename foo_bar");
    assert!(
        !result.contains("fn foo()"),
        "Should not have original foo()"
    );
}

#[test]
fn test_rename_byte_accuracy_substring() {
    let temp_dir = TempDir::new().unwrap();

    // Create file where old name is substring of another identifier
    let content = "fn bar() {\n    let bar_baz = bar();\n}\n";
    let file_path = create_test_file(&temp_dir, "test.rs", content);

    // Find exact byte spans of "bar" (not "bar_baz")
    let spans = find_symbol_spans(content, "bar");
    // This should find 2 occurrences: "bar" at position 3 and "bar" at position ~27
    // But NOT "bar_baz" because our helper checks word boundaries
    assert_eq!(
        spans.len(),
        2,
        "Should find exactly 2 'bar' occurrences (not bar_baz)"
    );

    // Create references for all "bar" occurrences
    let file_path_str = file_path.to_str().unwrap();
    let references: Vec<magellan::references::ReferenceFact> = spans
        .into_iter()
        .map(|(start, end)| magellan::references::ReferenceFact {
            file_path: std::path::PathBuf::from(file_path_str),
            referenced_symbol: "bar".to_string(),
            byte_start: start,
            byte_end: end,
            start_line: 1,
            start_col: start,
            end_line: 1,
            end_col: end,
        })
        .collect();

    // Apply replacements
    apply_replacements_in_file(&file_path, "bar", "qux", &references).unwrap();

    // Verify result
    let result = std::fs::read_to_string(&file_path).unwrap();
    assert!(result.contains("fn qux()"), "Should rename bar to qux");
    assert!(result.contains("qux();"), "Should rename bar() call");
    assert!(
        result.contains("bar_baz"),
        "Should NOT rename bar_baz identifier"
    );
    assert!(
        !result.contains("fn bar()"),
        "Should not have original bar()"
    );
}

// ============================================================================
// Preview Purity Tests (Task 5 - Part of Must-Haves)
// ============================================================================

#[test]
fn test_preview_no_backup_created() {
    let temp_dir = TempDir::new().unwrap();

    let file_path = create_test_file(&temp_dir, "test.rs", "fn foo() {}\n");
    let original_content = "fn foo() {}\n";

    // Simulate replacements (preview mode)
    let references = vec![create_reference(file_path.to_str().unwrap(), 3, 6)];
    let _modified =
        simulate_replacements_content(original_content, &references, "foo", "bar").unwrap();

    // Verify no backup directory was created
    let backups_base = temp_dir.path().join(".splice/backups");
    assert!(
        !backups_base.exists(),
        "Preview mode should not create backup directory"
    );

    // Verify original file unchanged
    let original = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(original, original_content);
}

#[test]
fn test_preview_no_filesystem_modifications() {
    let temp_dir = TempDir::new().unwrap();

    let file_path = create_test_file(&temp_dir, "test.rs", "fn foo() {}\n");
    let original_content = "fn foo() {}\n";
    let original_mtime = std::fs::metadata(&file_path).unwrap().modified().unwrap();

    // Simulate replacements (preview mode)
    let references = vec![create_reference(file_path.to_str().unwrap(), 3, 6)];
    let modified =
        simulate_replacements_content(original_content, &references, "foo", "bar").unwrap();

    // Verify preview shows the change
    assert_eq!(modified, "fn bar() {}\n");

    // Verify file metadata unchanged (no modification)
    let current_mtime = std::fs::metadata(&file_path).unwrap().modified().unwrap();
    assert_eq!(
        original_mtime, current_mtime,
        "File mtime should not change in preview mode"
    );

    // Verify content unchanged
    let current_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(current_content, original_content);
}
