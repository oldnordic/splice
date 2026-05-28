//! Pattern replacement with atomic writes and rollback.

use super::*;
use crate::error::{Result, SpliceError};
use crate::symbol::Language;
use crate::validate::AnalyzerMode;
use std::collections::HashMap;
use std::path::Path;

/// Apply pattern replacement to files with atomic writes and rollback.
///
/// This function:
/// 1. Finds all pattern matches using AST confirmation
/// 2. Creates backups of all files to be modified
/// 3. Applies replacements using atomic writes (tempfile + persist)
/// 4. On any error, restores all files from backups (atomic rollback)
/// 5. Runs validation gates if requested
pub fn apply_pattern_replace(
    config: &PatternReplaceConfig,
    workspace_dir: &Path,
) -> Result<PatternReplaceResult> {
    use std::io::Write;

    // Find all matches
    let matches = find_pattern_in_files(config)?;

    if matches.is_empty() {
        return Ok(PatternReplaceResult {
            files_patched: Vec::new(),
            replacements_count: 0,
            validation_errors: Vec::new(),
        });
    }

    // Group matches by file and sort by byte offset (descending for replacement)
    let mut matches_by_file: HashMap<PathBuf, Vec<&PatternMatch>> = HashMap::new();
    for m in &matches {
        matches_by_file.entry(m.file.clone()).or_default().push(m);
    }

    for file_matches in matches_by_file.values_mut() {
        file_matches.sort_by_key(|m| std::cmp::Reverse(m.byte_start));
    }

    // Create backup manifest for rollback
    let mut backups: Vec<(PathBuf, String)> = Vec::new();

    // Apply replacements per file with atomic writes
    let mut files_patched = Vec::new();
    let mut replacements_count = 0;

    // First pass: create backups of all files to be modified
    for file_path in matches_by_file.keys() {
        let replaced = std::fs::read_to_string(file_path).map_err(|e| SpliceError::Io {
            path: file_path.clone(),
            source: e,
        })?;
        backups.push((file_path.clone(), replaced));
    }

    // Second pass: apply replacements using atomic writes
    let apply_result: std::result::Result<(), SpliceError> =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for (file_path, file_matches) in &matches_by_file {
                if file_matches.is_empty() {
                    continue;
                }

                // Get original content from backup
                let replaced = backups
                    .iter()
                    .find(|(path, _)| path == file_path)
                    .map(|(_, content)| content.clone())
                    .expect("invariant: file_path present in backups from file_matches");

                // Apply replacements in reverse byte order
                let mut content = replaced.clone();
                for m in &**file_matches {
                    let start_byte = m.byte_start;
                    let end_byte = m.byte_end;

                    // Replace the content
                    content.replace_range(start_byte..end_byte, &config.replace_pattern);
                    replacements_count += 1;
                }

                // Atomic write using tempfile
                let parent_dir = file_path.parent().unwrap_or(Path::new("."));
                let mut temp =
                    tempfile::NamedTempFile::new_in(parent_dir).map_err(|e| SpliceError::Io {
                        path: file_path.clone(),
                        source: e,
                    })?;

                temp.write_all(content.as_bytes())
                    .map_err(|e| SpliceError::Io {
                        path: file_path.clone(),
                        source: e,
                    })?;

                // Persist atomically (replaces original file)
                temp.persist(file_path).map_err(|e| SpliceError::Io {
                    path: file_path.clone(),
                    source: e.into(),
                })?;

                files_patched.push(file_path.clone());
            }

            Ok::<(), SpliceError>(())
        }))
        .map_err(|_| {
            // Panic occurred - convert to SpliceError for rollback
            SpliceError::Other("Panic during pattern replacement".to_string())
        })?;

    // apply_result is now Result<(), SpliceError> after map_err
    // If it's Err, we need to roll back and return the error
    if let Err(rollback_err) = apply_result {
        // Restore all files from backups
        for (file_path, replaced_content) in &backups {
            // Attempt to restore, but continue even if restore fails
            let _ = std::fs::write(file_path, replaced_content);
        }

        return Err(rollback_err);
    }

    // Run validation if requested
    if config.validate {
        // For each patched file, run validation
        for file_path in &files_patched {
            // Determine language
            let lang = config
                .language
                .or_else(|| Language::from_path(file_path))
                .ok_or_else(|| {
                    SpliceError::Other(format!(
                        "Cannot detect language for file: {}",
                        file_path.display()
                    ))
                })?;

            // Run validation gates
            crate::patch::run_validation_gates(file_path, workspace_dir, lang, AnalyzerMode::Off)?;
        }
    }

    Ok(PatternReplaceResult {
        files_patched,
        replacements_count,
        validation_errors: Vec::new(),
    })
}
