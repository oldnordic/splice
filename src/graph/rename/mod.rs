//! Cross-file rename with byte-accurate replacement.
//!
//! This module implements safe symbol renaming using ReferenceFact
//! spans from Magellan. Replacements are applied at exact byte offsets
//! with proper offset recalculation for variable-length names.

use crate::error::{Result, SpliceError};
use chrono::Utc;
use magellan::references::ReferenceFact;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) mod apply;
pub(crate) mod preview;
pub(crate) mod transaction;

pub use apply::{
    apply_replacements_in_file, apply_with_rollback, group_references_by_file, replace_at_span,
    simulate_replacements,
};
pub use preview::{generate_colored_preview, generate_preview_diff, simulate_replacements_content};
#[cfg(test)]
pub(crate) use transaction::sha256_checksum;
pub use transaction::{create_rename_backup, RenameBackupManifest, RenameTransaction};

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_reference(file_path: &str, byte_start: usize, byte_end: usize) -> ReferenceFact {
        ReferenceFact {
            file_path: PathBuf::from(file_path),
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
    fn test_replace_at_span_basic() {
        let content = b"fn old_name() { old_name(); }";
        let span = create_test_reference("test.rs", 3, 11);
        let new_name = b"new_name";

        let result = replace_at_span(content, &span, new_name).unwrap();
        assert_eq!(result, b"fn new_name() { old_name(); }");
    }

    #[test]
    fn test_replace_at_span_different_length() {
        let content = b"fn foo() {}";
        let span = create_test_reference("test.rs", 3, 6);
        let new_name = b"bar";

        let result = replace_at_span(content, &span, new_name).unwrap();
        assert_eq!(result, b"fn bar() {}");
    }

    #[test]
    fn test_replace_at_span_longer_name() {
        let content = b"fn foo() {}";
        let span = create_test_reference("test.rs", 3, 6);
        let new_name = b"baz_qux";

        let result = replace_at_span(content, &span, new_name).unwrap();
        assert_eq!(result, b"fn baz_qux() {}");
    }

    #[test]
    fn test_replace_at_span_shorter_name() {
        let content = b"function foo() {}";
        let span = create_test_reference("test.rs", 9, 12);
        let new_name = b"x";

        let result = replace_at_span(content, &span, new_name).unwrap();
        assert_eq!(result, b"function x() {}");
    }

    #[test]
    fn test_replace_at_span_invalid_start() {
        let content = b"fn foo() {}";
        let span = create_test_reference("test.rs", 100, 105);
        let new_name = b"bar";

        let result = replace_at_span(content, &span, new_name);
        assert!(result.is_err());
        match result.unwrap_err() {
            SpliceError::InvalidSpan { start, .. } => assert_eq!(start, 100),
            _ => panic!("Expected InvalidSpan error"),
        }
    }

    #[test]
    fn test_replace_at_span_invalid_end() {
        let content = b"fn foo() {}";
        let span = create_test_reference("test.rs", 0, 100);
        let new_name = b"bar";

        let result = replace_at_span(content, &span, new_name);
        assert!(result.is_err());
        match result.unwrap_err() {
            SpliceError::InvalidSpan { end, .. } => assert_eq!(end, 100),
            _ => panic!("Expected InvalidSpan error"),
        }
    }

    #[test]
    fn test_replace_at_span_empty_replacement() {
        let content = b"fn foo() {}";
        let span = create_test_reference("test.rs", 3, 6);
        let new_name = b"";

        let result = replace_at_span(content, &span, new_name).unwrap();
        assert_eq!(result, b"fn () {}");
    }

    #[test]
    fn test_group_references_by_file() {
        let references = vec![
            create_test_reference("/src/a.rs", 100, 103),
            create_test_reference("/src/b.rs", 50, 53),
            create_test_reference("/src/a.rs", 20, 23),
            create_test_reference("/src/b.rs", 10, 13),
        ];

        let grouped = group_references_by_file(&references);

        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains_key(PathBuf::from("/src/a.rs").as_path()));
        assert!(grouped.contains_key(PathBuf::from("/src/b.rs").as_path()));

        // Check that each file's refs are sorted descending by byte_start
        let a_refs = grouped.get(&PathBuf::from("/src/a.rs")).unwrap();
        assert_eq!(a_refs[0].byte_start, 100);
        assert_eq!(a_refs[1].byte_start, 20);

        let b_refs = grouped.get(&PathBuf::from("/src/b.rs")).unwrap();
        assert_eq!(b_refs[0].byte_start, 50);
        assert_eq!(b_refs[1].byte_start, 10);
    }

    #[test]
    fn test_simulate_replacements() {
        let references = vec![
            create_test_reference("/src/a.rs", 100, 103),
            create_test_reference("/src/b.rs", 50, 53),
            create_test_reference("/src/a.rs", 20, 23),
        ];

        let simulation = simulate_replacements(&references);

        assert_eq!(simulation.len(), 2);
        assert_eq!(simulation.get(&PathBuf::from("/src/a.rs")), Some(&2));
        assert_eq!(simulation.get(&PathBuf::from("/src/b.rs")), Some(&1));
    }

    #[test]
    fn test_utf8_multibyte_character_replacement() {
        // Content with multibyte UTF-8 characters
        // "世界" is 6 bytes (3 bytes per character)
        let content = "fn foo() { // 世界 }".as_bytes();
        let span = create_test_reference("test.rs", 3, 6);
        let new_name = b"bar";

        let result = replace_at_span(content, &span, new_name).unwrap();
        assert_eq!(result, b"fn bar() { // \xe4\xb8\x96\xe7\x95\x8c }");
    }

    #[test]
    fn test_apply_replacements_in_file_integration() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        // Write initial content
        let initial_content = b"fn old_name() {\n    old_name();\n    old_name();\n}\n";
        fs::write(file_path, initial_content).unwrap();

        // Create references (in descending byte_start order)
        // "old_name" appears at: 3-11, 20-28, 36-44
        // We skip the definition (3-11) and replace the usages
        let references = vec![
            create_test_reference(file_path.to_str().unwrap(), 36, 44),
            create_test_reference(file_path.to_str().unwrap(), 20, 28),
        ];

        // Apply replacements
        let count =
            apply_replacements_in_file(file_path, "old_name", "new_name", &references).unwrap();

        assert_eq!(count, 2);

        // Verify the result - definition is NOT changed
        let result_content = fs::read_to_string(file_path).unwrap();
        assert_eq!(
            result_content,
            "fn old_name() {\n    new_name();\n    new_name();\n}\n"
        );
    }

    #[test]
    fn test_apply_replacements_with_multibyte_utf8() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        // Content with multibyte UTF-8 comments
        let initial_content = "fn foo() { // 世界\n    foo(); // 世界\n}".as_bytes();
        fs::write(file_path, initial_content).unwrap();

        let references = vec![create_test_reference(file_path.to_str().unwrap(), 3, 6)];

        let count = apply_replacements_in_file(file_path, "foo", "bar", &references).unwrap();

        assert_eq!(count, 1);

        // Verify multibyte characters are preserved
        let result_content = fs::read_to_string(file_path).unwrap();
        assert!(result_content.contains("世界"));
        assert!(result_content.contains("bar()"));
    }

    #[test]
    fn test_multiple_replacements_same_file() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        // Multiple occurrences of the same symbol
        let initial_content = b"a = foo + foo * foo;";
        fs::write(file_path, initial_content).unwrap();

        // "foo" appears at byte 4, 10, 16 (each is 3 bytes long)
        // References must be in descending order
        let references = vec![
            create_test_reference(file_path.to_str().unwrap(), 16, 19),
            create_test_reference(file_path.to_str().unwrap(), 10, 13),
            create_test_reference(file_path.to_str().unwrap(), 4, 7),
        ];

        let count = apply_replacements_in_file(file_path, "foo", "bar", &references).unwrap();

        assert_eq!(count, 3);

        let result_content = fs::read_to_string(file_path).unwrap();
        assert_eq!(result_content, "a = bar + bar * bar;");
    }

    #[test]
    fn test_apply_replacements_empty_list() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        let initial_content = b"fn foo() {}";
        fs::write(file_path, initial_content).unwrap();

        let references: Vec<ReferenceFact> = vec![];

        let count = apply_replacements_in_file(file_path, "foo", "bar", &references).unwrap();

        assert_eq!(count, 0);

        // Content should be unchanged
        let result_content = fs::read_to_string(file_path).unwrap();
        assert_eq!(result_content, "fn foo() {}");
    }

    #[test]
    fn test_generate_preview_diff() {
        let original = "fn foo() {\n    println!(\"foo\");\n}\n";
        let modified = "fn bar() {\n    println!(\"bar\");\n}\n";
        let file_path = PathBuf::from("test.rs");

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
        let file_path = PathBuf::from("test.rs");

        let diff = generate_preview_diff(&file_path, content, content);

        // Should be empty when there are no changes
        assert!(diff.is_empty());
    }

    #[test]
    fn test_generate_colored_preview() {
        let original = "fn foo() {}\n";
        let modified = "fn bar() {}\n";
        let file_path = PathBuf::from("test.rs");

        // This will produce plain text in non-TTY environments (like CI)
        let colored = generate_colored_preview(&file_path, original, modified);

        // Should always contain diff content (colored or not)
        assert!(!colored.is_empty());

        // In plain text mode, should show +/- prefixes
        if !colored.contains('\x1b') {
            // No ANSI codes means plain text mode
            assert!(colored.contains("-fn foo()"));
            assert!(colored.contains("+fn bar()"));
        }
    }

    #[test]
    fn test_simulate_replacements_content() {
        let content = "fn foo() {\n    foo();\n}\n";
        let file_path_str = "test.rs";

        // Create references (in descending order)
        // "foo" appears at: 3-6, 16-19
        let references = vec![
            ReferenceFact {
                file_path: PathBuf::from(file_path_str),
                referenced_symbol: "foo".to_string(),
                byte_start: 15,
                byte_end: 18,
                start_line: 2,
                start_col: 4,
                end_line: 2,
                end_col: 7,
            },
            ReferenceFact {
                file_path: PathBuf::from(file_path_str),
                referenced_symbol: "foo".to_string(),
                byte_start: 3,
                byte_end: 6,
                start_line: 1,
                start_col: 3,
                end_line: 1,
                end_col: 6,
            },
        ];

        let result = simulate_replacements_content(content, &references, "foo", "bar").unwrap();

        assert_eq!(result, "fn bar() {\n    bar();\n}\n");
    }

    #[test]
    fn test_simulate_replacements_content_preserves_multibyte() {
        let content = "fn foo() { // 世界\n}\n";
        let references = vec![ReferenceFact {
            file_path: PathBuf::from("test.rs"),
            referenced_symbol: "foo".to_string(),
            byte_start: 3,
            byte_end: 6,
            start_line: 1,
            start_col: 3,
            end_line: 1,
            end_col: 6,
        }];

        let result = simulate_replacements_content(content, &references, "foo", "bar").unwrap();

        // Should preserve the multibyte UTF-8 characters
        assert!(result.contains("世界"));
        assert!(result.contains("bar()"));
    }

    #[test]
    fn test_create_rename_backup() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create test files
        let file1 = workspace_root.join("src").join("main.rs");
        let file2 = workspace_root.join("src").join("lib.rs");
        fs::create_dir_all(file1.parent().unwrap()).unwrap();
        fs::write(&file1, "fn foo() {}\n").unwrap();
        fs::write(&file2, "fn bar() {}\n").unwrap();

        // Create backup
        let backup_dir = create_rename_backup(
            workspace_root,
            "test_symbol",
            &[file1.clone(), file2.clone()],
        )
        .unwrap();

        // Verify backup directory exists
        assert!(backup_dir.exists());
        assert!(backup_dir.starts_with(workspace_root.join(".splice/backups")));

        // Verify backup contains manifest.json
        let manifest_path = backup_dir.join("manifest.json");
        assert!(manifest_path.exists());

        // Verify manifest contents
        let manifest_json = fs::read_to_string(&manifest_path).unwrap();
        let manifest: RenameBackupManifest = serde_json::from_str(&manifest_json).unwrap();

        assert!(manifest.operation_id.starts_with("rename-test_symbol-"));
        assert_eq!(manifest.files.len(), 2);
        assert!(manifest.files.contains_key("src/main.rs"));
        assert!(manifest.files.contains_key("src/lib.rs"));

        // Verify files were copied
        let backup_file1 = backup_dir.join("src").join("main.rs");
        let backup_file2 = backup_dir.join("src").join("lib.rs");
        assert!(backup_file1.exists());
        assert!(backup_file2.exists());

        // Verify content matches
        let original_content = fs::read_to_string(&file1).unwrap();
        let backup_content = fs::read_to_string(&backup_file1).unwrap();
        assert_eq!(original_content, backup_content);
    }

    #[test]
    fn test_create_rename_backup_nested_files() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create nested test files
        let file1 = workspace_root.join("src").join("api").join("handlers.rs");
        let file2 = workspace_root.join("tests").join("integration_test.rs");
        fs::create_dir_all(file1.parent().unwrap()).unwrap();
        fs::create_dir_all(file2.parent().unwrap()).unwrap();
        fs::write(&file1, "pub fn handler() {}\n").unwrap();
        fs::write(&file2, "#[test]\nfn test() {}\n").unwrap();

        // Create backup
        let backup_dir = create_rename_backup(
            workspace_root,
            "nested_test",
            &[file1.clone(), file2.clone()],
        )
        .unwrap();

        // Verify nested structure preserved
        let backup_file1 = backup_dir.join("src").join("api").join("handlers.rs");
        let backup_file2 = backup_dir.join("tests").join("integration_test.rs");
        assert!(backup_file1.exists());
        assert!(backup_file2.exists());

        // Verify manifest has correct relative paths
        let manifest_path = backup_dir.join("manifest.json");
        let manifest: RenameBackupManifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();

        assert!(manifest.files.contains_key("src/api/handlers.rs"));
        assert!(manifest.files.contains_key("tests/integration_test.rs"));
    }

    #[test]
    fn test_sha256_checksum() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        fs::write(file_path, "test content").unwrap();

        let checksum = sha256_checksum(file_path).unwrap();

        // SHA-256 of "test content" (without newline)
        assert_eq!(
            checksum,
            "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72"
        );
        assert_eq!(checksum.len(), 64); // SHA-256 produces 64 hex characters
    }

    #[test]
    fn test_rename_transaction_new() {
        let txn = RenameTransaction::new();

        assert!(txn.backup_dir.is_none());
        assert!(txn.workspace_root.is_none());
        assert_eq!(txn.modified_count(), 0);
    }

    #[test]
    fn test_rename_transaction_default() {
        let txn = RenameTransaction::default();

        assert_eq!(txn.modified_count(), 0);
    }

    #[test]
    fn test_rename_transaction_with_backup() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();
        let backup_dir = workspace_root.join(".splice/backups/test-backup");

        let txn =
            RenameTransaction::new().with_backup(backup_dir.clone(), workspace_root.to_path_buf());

        assert!(txn.backup_dir.as_ref().is_some());
        assert_eq!(txn.backup_dir.as_ref().unwrap(), &backup_dir);
        assert!(txn.workspace_root.as_ref().is_some());
    }

    #[test]
    fn test_rename_transaction_track_modified() {
        let mut txn = RenameTransaction::new();

        txn.track_modified(PathBuf::from("/path/to/file1.rs"));
        txn.track_modified(PathBuf::from("/path/to/file2.rs"));

        assert_eq!(txn.modified_count(), 2);
        assert_eq!(
            txn.modified_files(),
            &[
                PathBuf::from("/path/to/file1.rs"),
                PathBuf::from("/path/to/file2.rs")
            ]
        );
    }

    #[test]
    fn test_rename_transaction_rollback() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create original file
        let file_path = workspace_root.join("test.rs");
        let original_content = "fn original() {}\n";
        fs::write(&file_path, original_content).unwrap();

        // Create backup
        let backup_dir = workspace_root.join(".splice/backups/test-rollback");
        fs::create_dir_all(&backup_dir).unwrap();
        let backup_file = backup_dir.join("test.rs");
        fs::write(&backup_file, original_content).unwrap();

        // Create manifest
        let manifest = RenameBackupManifest {
            operation_id: "test-rollback".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            files: HashMap::from([("test.rs".to_string(), "dummy_checksum".to_string())]),
        };
        let manifest_path = backup_dir.join("manifest.json");
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        // Modify the file
        fs::write(&file_path, "fn modified() {}\n").unwrap();

        // Rollback
        let txn = RenameTransaction::new().with_backup(backup_dir, workspace_root.to_path_buf());
        txn.rollback().unwrap();

        // Verify file was restored
        let restored_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(restored_content, original_content);
    }

    #[test]
    fn test_apply_with_rollback_success() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        let mut txn = RenameTransaction::new();

        apply_with_rollback(file_path, || Ok(b"new content".to_vec()), &mut txn).unwrap();

        // Verify file was written
        let content = fs::read_to_string(file_path).unwrap();
        assert_eq!(content, "new content");

        // Verify transaction tracked the modification
        assert_eq!(txn.modified_count(), 1);
    }

    #[test]
    fn test_apply_with_rollback_rollback_on_error() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        // Write initial content
        let initial_content = "initial content\n";
        fs::write(file_path, initial_content).unwrap();

        let mut txn = RenameTransaction::new();

        // Simulate a failure during modification
        let result = apply_with_rollback(
            file_path,
            || Err(SpliceError::Other("Simulated failure".to_string())),
            &mut txn,
        );

        assert!(result.is_err());

        // Content should remain unchanged since write never happened
        let content = fs::read_to_string(file_path).unwrap();
        assert_eq!(content, initial_content);
    }
}
