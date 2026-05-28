use super::*;
use crate::graph::MagellanIntegration;

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
