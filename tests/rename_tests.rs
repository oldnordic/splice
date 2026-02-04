//! Integration tests for cross-file rename operations.
//!
//! These tests verify the complete rename workflow including:
//! - Single file rename
//! - Preview mode (no filesystem modifications)
//! - Backup creation and rollback
//! - Multi-file rename with transaction safety

use splice::graph::rename::{
    apply_replacements_in_file, create_rename_backup, generate_colored_preview,
    generate_preview_diff, group_references_by_file, RenameBackupManifest,
    RenameTransaction, simulate_replacements, simulate_replacements_content,
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
    let count = apply_replacements_in_file(&file_path, "old_name", "new_name", &references).unwrap();

    assert_eq!(count, 2);

    // Verify result
    let result = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(result, "fn old_name() {\n    new_name();\n    new_name();\n}\n");
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
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();

    // Modify the file
    std::fs::write(&file_path, "fn modified() {}\n").unwrap();

    // Rollback
    let txn = RenameTransaction::new()
        .with_backup(backup_dir, temp_dir.path().to_path_buf());
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

    let modified1 = simulate_replacements_content(&content1, &refs1, "old_name", "new_name").unwrap();
    let modified2 = simulate_replacements_content(&content2, &refs2, "old_name", "new_name").unwrap();

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
    let file2 = create_test_file(&temp_dir, "tests/integration_test.rs", "#[test]\nfn test() {}\n");

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
    let manifest: RenameBackupManifest = serde_json::from_str(
        &std::fs::read_to_string(backup_dir.join("manifest.json")).unwrap(),
    )
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
