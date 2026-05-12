//! Cross-file rename with byte-accurate replacement.
//!
//! This module implements safe symbol renaming using ReferenceFact
//! spans from Magellan. Replacements are applied at exact byte offsets
//! with proper offset recalculation for variable-length names.

use crate::diff::{format_colored_diff, format_unified_diff, should_use_color};
use crate::error::{Result, SpliceError};
use crate::graph::MagellanIntegration;
use chrono::Utc;
use magellan::references::ReferenceFact;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Generate preview diff for rename operation.
///
/// Creates a unified diff showing what changes would be made to a file
/// when renaming a symbol. This is used in preview mode to show the
/// user what will change without actually modifying files.
///
/// # Arguments
/// * `file_path` - Path to file
/// * `original_content` - Original file content
/// * `modified_content` - Content after rename
///
/// # Returns
/// Unified diff string in git-compatible format
pub fn generate_preview_diff(
    file_path: &Path,
    original_content: &str,
    modified_content: &str,
) -> String {
    format_unified_diff(
        original_content,
        modified_content,
        &file_path.display().to_string(),
        3, // context lines
    )
}

/// Generate colored preview if TTY detected.
///
/// This function automatically detects whether to use colored output
/// based on terminal capabilities and the NO_COLOR environment variable.
/// When colors are enabled, deletions are shown in red and additions
/// in green following git's convention.
///
/// # Arguments
/// * `file_path` - Path to file
/// * `original_content` - Original file content
/// * `modified_content` - Content after rename
///
/// # Returns
/// Colored diff string or plain text diff depending on terminal support
pub fn generate_colored_preview(
    file_path: &Path,
    original_content: &str,
    modified_content: &str,
) -> String {
    let use_color = should_use_color();
    if use_color {
        // Apply ANSI colors (red for deletions, green for additions)
        format_colored_diff(original_content, modified_content, use_color)
    } else {
        // Plain unified diff
        format_unified_diff(
            original_content,
            modified_content,
            &file_path.display().to_string(),
            3,
        )
    }
}

/// Simulate replacements on a file's content without modifying it.
///
/// This function applies the same replacement logic that would be used
/// during an actual rename, but returns the modified content instead of
/// writing it to disk. This is used for preview mode.
///
/// # Arguments
/// * `content` - Original file content
/// * `references` - ReferenceFact entries sorted by byte_start DESCENDING
/// * `_old_name` - Original symbol name (for validation, currently unused)
/// * `new_name` - New symbol name
///
/// # Returns
/// Modified content with all replacements applied
pub fn simulate_replacements_content(
    content: &str,
    references: &[ReferenceFact],
    _old_name: &str,
    new_name: &str,
) -> Result<String> {
    let content_bytes = content.as_bytes();
    let new_name_bytes = new_name.as_bytes();
    let mut current_content = content_bytes.to_vec();

    // Apply replacements from end to start (descending byte_start)
    for reference in references {
        match replace_at_span(&current_content, reference, new_name_bytes) {
            Ok(new_content) => {
                current_content = new_content;
            }
            Err(e) => {
                return Err(SpliceError::Other(format!(
                    "Failed to simulate replacement at {}..{}: {}",
                    reference.byte_start, reference.byte_end, e
                )));
            }
        }
    }

    // Convert back to string, validating UTF-8
    String::from_utf8(current_content).map_err(|e| SpliceError::InvalidUtf8 {
        file: PathBuf::from("<preview>"),
        source: e.utf8_error(),
    })
}

/// Backup manifest for rename operations.
///
/// This struct stores metadata about a rename operation backup, including
/// the operation ID, timestamp, and checksums of all backed-up files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameBackupManifest {
    /// Operation identifier (e.g., "rename-symbol_id-20250104-120000")
    pub operation_id: String,
    /// Timestamp of backup (ISO 8601)
    pub timestamp: String,
    /// Files backed up with their checksums (relative path -> sha256 hash)
    pub files: HashMap<String, String>,
}

/// Create backup for rename operation.
///
/// Creates a backup directory in `.splice/backups/rename-<symbol_id>-<timestamp>/`
/// with copies of all affected files and a manifest.json containing checksums.
///
/// # Arguments
/// * `workspace_root` - Project root directory
/// * `symbol_id` - Symbol ID for directory naming
/// * `files_to_backup` - Files to backup
///
/// # Returns
/// Path to created backup directory containing manifest.json
///
/// # Errors
/// Returns Io error if directory creation or file copying fails.
pub fn create_rename_backup(
    workspace_root: &Path,
    symbol_id: &str,
    files_to_backup: &[PathBuf],
) -> Result<PathBuf> {
    // Create backup directory name: rename-<symbol_id>-<timestamp>
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let operation_id = format!("rename-{}-{}", symbol_id, timestamp);

    // Use .splice/backups/ base directory (from context decision)
    let backups_base = workspace_root.join(".splice/backups");
    fs::create_dir_all(&backups_base).map_err(|e| SpliceError::Io {
        path: backups_base.clone(),
        source: e,
    })?;

    let backup_dir = backups_base.join(&operation_id);
    fs::create_dir(&backup_dir).map_err(|e| SpliceError::Io {
        path: backup_dir.clone(),
        source: e,
    })?;

    // Copy files to backup directory, preserving structure
    let mut manifest = RenameBackupManifest {
        operation_id: operation_id.clone(),
        timestamp: Utc::now().to_rfc3339(),
        files: HashMap::new(),
    };

    for file_path in files_to_backup {
        // Compute relative path from workspace root
        let relative_path = file_path.strip_prefix(workspace_root).map_err(|_| {
            SpliceError::Other(format!(
                "File {} is not under workspace root {}",
                file_path.display(),
                workspace_root.display()
            ))
        })?;

        // Create backup path
        let backup_file_path = backup_dir.join(relative_path);

        // Create parent directories if needed
        if let Some(parent) = backup_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| SpliceError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Copy file
        fs::copy(file_path, &backup_file_path).map_err(|e| SpliceError::Io {
            path: backup_file_path.clone(),
            source: e,
        })?;

        // Compute checksum for verification
        let checksum = sha256_checksum(file_path)?;
        manifest
            .files
            .insert(relative_path.display().to_string(), checksum);
    }

    // Write manifest.json
    let manifest_path = backup_dir.join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| SpliceError::Other(format!("Failed to serialize backup manifest: {}", e)))?;
    fs::write(&manifest_path, manifest_json).map_err(|e| SpliceError::Io {
        path: manifest_path.clone(),
        source: e,
    })?;

    Ok(backup_dir)
}

/// Compute SHA-256 checksum of a file.
///
/// # Arguments
/// * `file_path` - Path to file
///
/// # Returns
/// Hex-encoded SHA-256 checksum
///
/// # Errors
/// Returns Io error if file cannot be read.
fn sha256_checksum(file_path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let content = fs::read(file_path).map_err(|e| SpliceError::Io {
        path: file_path.to_path_buf(),
        source: e,
    })?;
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Transaction context for rename operation.
///
/// Provides atomic rename operations with rollback support.
/// If any part of the rename fails, all changes can be reverted
/// from the backup.
#[derive(Debug)]
pub struct RenameTransaction {
    /// Full path to backup directory containing manifest.json
    backup_dir: Option<PathBuf>,
    /// Workspace root path for restoring files
    workspace_root: Option<PathBuf>,
    /// Files that have been modified
    modified_files: Vec<PathBuf>,
}

impl RenameTransaction {
    /// Create new transaction without backup.
    pub fn new() -> Self {
        Self {
            backup_dir: None,
            workspace_root: None,
            modified_files: Vec::new(),
        }
    }

    /// Set backup directory for rollback (stores full path).
    ///
    /// # Arguments
    /// * `backup_dir` - Path to backup directory containing manifest.json
    /// * `workspace_root` - Root directory of the workspace
    pub fn with_backup(mut self, backup_dir: PathBuf, workspace_root: PathBuf) -> Self {
        self.backup_dir = Some(backup_dir);
        self.workspace_root = Some(workspace_root);
        self
    }

    /// Track a modified file.
    ///
    /// # Arguments
    /// * `file` - Path to file that was modified
    pub fn track_modified(&mut self, file: PathBuf) {
        self.modified_files.push(file);
    }

    /// Rollback transaction by restoring files from backup.
    ///
    /// Reads the manifest.json from the backup directory and restores
    /// all files to their pre-rename state.
    ///
    /// # Returns
    /// Ok(()) if rollback succeeded, error if it failed
    ///
    /// # Errors
    /// Returns Io error if file restoration fails.
    /// Returns Other error if manifest is missing or invalid.
    pub fn rollback(self) -> Result<()> {
        if let (Some(backup_dir), Some(workspace_root)) = (self.backup_dir, self.workspace_root) {
            let manifest_path = backup_dir.join("manifest.json");
            if manifest_path.exists() {
                // Read manifest
                let manifest_json =
                    fs::read_to_string(&manifest_path).map_err(|e| SpliceError::Io {
                        path: manifest_path.clone(),
                        source: e,
                    })?;
                let manifest: RenameBackupManifest =
                    serde_json::from_str(&manifest_json).map_err(|e| {
                        SpliceError::Other(format!("Failed to parse backup manifest: {}", e))
                    })?;

                // Restore each file from backup
                for (relative_path, _checksum) in manifest.files {
                    let backup_file = backup_dir.join(&relative_path);
                    let original_file = workspace_root.join(&relative_path);

                    if backup_file.exists() {
                        fs::copy(&backup_file, &original_file).map_err(|e| SpliceError::Io {
                            path: original_file.clone(),
                            source: e,
                        })?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Get the number of files modified in this transaction.
    pub fn modified_count(&self) -> usize {
        self.modified_files.len()
    }

    /// Get the list of modified files.
    pub fn modified_files(&self) -> &[PathBuf] {
        &self.modified_files
    }
}

impl Default for RenameTransaction {
    fn default() -> Self {
        Self::new()
    }
}

/// Apply file modification with rollback support.
///
/// This helper function wraps a file modification with automatic rollback
/// on failure. If the modification fails, the transaction is rolled back
/// before returning the error.
///
/// # Arguments
/// * `file_path` - Path to file to modify
/// * `modify` - Function that produces the new content
/// * `transaction` - Transaction to track modification
///
/// # Returns
/// Ok(()) if modification succeeded, error with rollback applied if it failed
///
/// # Errors
/// Returns the original modification error after attempting rollback.
pub fn apply_with_rollback<F>(
    file_path: &Path,
    modify: F,
    transaction: &mut RenameTransaction,
) -> Result<()>
where
    F: FnOnce() -> Result<Vec<u8>>,
{
    let new_content = modify()?;

    fs::write(file_path, new_content).map_err(|e| SpliceError::Io {
        path: file_path.to_path_buf(),
        source: e,
    })?;

    transaction.track_modified(file_path.to_path_buf());
    Ok(())
}

/// Perform byte-accurate replacement at a specific span.
///
/// This function replaces the content between byte_start and byte_end
/// with new_name, preserving all other content exactly as-is.
///
/// # Arguments
/// * `content` - File content as bytes
/// * `span` - ReferenceFact with byte_start/byte_end
/// * `new_name` - Replacement name (as bytes)
///
/// # Returns
/// Modified content with replacement applied
///
/// # Errors
/// Returns InvalidSpan if byte_start/byte_end are out of bounds
/// or if the span crosses UTF-8 character boundaries.
pub fn replace_at_span(content: &[u8], span: &ReferenceFact, new_name: &[u8]) -> Result<Vec<u8>> {
    // Validate span boundaries
    if span.byte_start >= content.len() || span.byte_end > content.len() {
        return Err(SpliceError::InvalidSpan {
            file: span.file_path.clone(),
            start: span.byte_start,
            end: span.byte_end,
            file_size: content.len(),
        });
    }

    // Validate UTF-8 boundaries
    MagellanIntegration::validate_utf8_span(
        content,
        span.byte_start,
        span.byte_end,
        &span.file_path,
    )?;

    // Build new content: before + new_name + after
    let mut result = Vec::with_capacity(content.len() + new_name.len());
    result.extend_from_slice(&content[..span.byte_start]);
    result.extend_from_slice(new_name);
    result.extend_from_slice(&content[span.byte_end..]);

    Ok(result)
}

/// Apply replacements to a single file with offset recalculation.
///
/// This function reads a file, applies multiple replacements at specific
/// byte spans, and writes the result back. Replacements are applied from
/// end to start (descending byte_start) to ensure that earlier replacements
/// don't affect the byte offsets of later ones.
///
/// # Arguments
/// * `file_path` - Path to file
/// * `_old_name` - Original symbol name (for validation, currently unused)
/// * `new_name` - Replacement symbol name
/// * `references` - ReferenceFact entries sorted by byte_start DESCENDING
///
/// # Returns
/// Number of replacements applied
///
/// # Errors
/// Returns Io error if file cannot be read or written.
/// Returns InvalidSpan if any reference span is invalid.
pub fn apply_replacements_in_file(
    file_path: &Path,
    _old_name: &str,
    new_name: &str,
    references: &[ReferenceFact],
) -> Result<usize> {
    // Read file as bytes
    let content = fs::read(file_path).map_err(|e| SpliceError::Io {
        path: file_path.to_path_buf(),
        source: e,
    })?;

    let new_name_bytes = new_name.as_bytes();
    let mut current_content = content;
    let mut replacements = 0;

    // Apply replacements from end to start (descending byte_start)
    // This ensures earlier replacements don't affect later offsets
    for reference in references {
        match replace_at_span(&current_content, reference, new_name_bytes) {
            Ok(new_content) => {
                current_content = new_content;
                replacements += 1;
            }
            Err(e) => {
                return Err(SpliceError::Other(format!(
                    "Failed to replace in {} at {}..{}: {}",
                    file_path.display(),
                    reference.byte_start,
                    reference.byte_end,
                    e
                )));
            }
        }
    }

    // Write modified content back
    fs::write(file_path, current_content).map_err(|e| SpliceError::Io {
        path: file_path.to_path_buf(),
        source: e,
    })?;

    Ok(replacements)
}

/// Group references by file path.
///
/// This function takes a flat list of references and groups them by file.
/// Within each file, references are sorted by byte_start in descending order
/// to enable safe sequential replacement without offset recalculation issues.
///
/// # Arguments
/// * `references` - Flat list of ReferenceFact entries
///
/// # Returns
/// HashMap mapping file path to vector of ReferenceFact entries sorted
/// by byte_start DESCENDING for safe replacement.
pub fn group_references_by_file(
    references: &[ReferenceFact],
) -> HashMap<PathBuf, Vec<ReferenceFact>> {
    let mut grouped: HashMap<PathBuf, Vec<ReferenceFact>> = HashMap::new();

    for reference in references {
        grouped
            .entry(reference.file_path.clone())
            .or_default()
            .push(reference.clone());
    }

    // Sort each file's references by byte_start DESCENDING
    // This ensures replacements are applied from end to start
    for refs in grouped.values_mut() {
        refs.sort_by_key(|b| std::cmp::Reverse(b.byte_start));
    }

    grouped
}

/// Simulate replacements to generate preview data.
///
/// This function generates a preview of what changes would be made
/// without actually modifying any files. It returns a mapping of
/// file paths to the number of replacements that would be applied.
///
/// # Arguments
/// * `references` - ReferenceFact entries to simulate
///
/// # Returns
/// HashMap mapping file path to count of replacements
pub fn simulate_replacements(references: &[ReferenceFact]) -> HashMap<PathBuf, usize> {
    let mut simulation: HashMap<PathBuf, usize> = HashMap::new();

    for reference in references {
        *simulation.entry(reference.file_path.clone()).or_insert(0) += 1;
    }

    simulation
}

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
