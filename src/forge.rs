//! High-level API for external consumers (forge, agents, tool integrations).
//!
//! This module provides convenience functions that combine symbol resolution,
//! reference discovery, and span-safe editing into single calls. Designed for
//! programmatic use where the caller doesn't want to manage `MagellanIntegration`
//! or low-level span arithmetic directly.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use splice::forge;
//!
//! let db = Path::new(".magellan/myproject.db");
//!
//! // Patch a symbol with new content
//! let report = forge::patch_symbol_in_file(
//!     Path::new("src/lib.rs"),
//!     "my_function",
//!     "pub fn my_function() -> i32 { 42 }",
//!     db,
//! ).unwrap();
//!
//! // Rename across all files
//! let result = forge::rename_symbol_across_files("old_name", "new_name", db).unwrap();
//! ```

use crate::error::{Result, SpliceError};
use crate::graph::magellan_integration::MagellanIntegration;
use crate::graph::rename::{apply_replacements_in_file, group_references_by_file};
use crate::patch::replace_span;
use std::path::Path;

/// Result of a symbol patch operation.
#[derive(Debug, Clone)]
pub struct PatchResult {
    /// File that was patched.
    pub file: std::path::PathBuf,
    /// Symbol that was patched.
    pub symbol: String,
    /// Byte span that was replaced.
    pub byte_start: usize,
    /// Byte end of the replacement.
    pub byte_end: usize,
    /// Number of lines in the new content.
    pub lines_added: usize,
}

/// Result of a rename operation across files.
#[derive(Debug, Clone)]
pub struct RenameResult {
    /// Files that were modified.
    pub files_changed: Vec<std::path::PathBuf>,
    /// Total number of replacements applied.
    pub replacements: usize,
}

/// Resolved symbol span from the code graph.
#[derive(Debug, Clone)]
pub struct SymbolSpan {
    /// Symbol name.
    pub name: String,
    /// File path.
    pub file: std::path::PathBuf,
    /// Symbol kind (e.g. "fn", "struct", "class").
    pub kind: String,
    /// Byte offset where the symbol starts.
    pub byte_start: usize,
    /// Byte offset where the symbol ends.
    pub byte_end: usize,
    /// Start line (1-indexed), if available.
    pub start_line: Option<usize>,
    /// End line (1-indexed), if available.
    pub end_line: Option<usize>,
}

impl From<crate::graph::magellan_integration::SymbolInfo> for SymbolSpan {
    fn from(info: crate::graph::magellan_integration::SymbolInfo) -> Self {
        SymbolSpan {
            name: info.name,
            file: std::path::PathBuf::from(info.file_path),
            kind: info.kind,
            byte_start: info.byte_start,
            byte_end: info.byte_end,
            start_line: info.start_line,
            end_line: info.end_line,
        }
    }
}

/// Resolve a symbol to its byte span using `MagellanIntegration`.
///
/// Uses `find_symbol_by_path_and_name` for file-scoped lookup, falling back
/// to `find_symbol_by_name` for project-wide resolution.
///
/// # Errors
///
/// Returns `SpliceError::SymbolNotFound` if the symbol cannot be resolved.
/// Returns `SpliceError::Other` if the database cannot be opened.
pub fn resolve_symbol_span(
    file_path: &Path,
    symbol_name: &str,
    db_path: &Path,
) -> Result<SymbolSpan> {
    let mut integration = MagellanIntegration::open(db_path)?;

    if let Ok(Some(info)) = integration.find_symbol_by_path_and_name(file_path, symbol_name) {
        return Ok(SymbolSpan::from(info));
    }

    let matches = integration.find_symbol_by_name(symbol_name, false)?;
    match matches.len() {
        0 => Err(SpliceError::symbol_not_found(symbol_name, Some(file_path))),
        1 => Ok(SymbolSpan::from(
            matches.into_iter().next().expect("invariant: length is 1"),
        )),
        _ => Err(SpliceError::Other(format!(
            "Symbol '{}' is ambiguous: {} matches found. Specify a file path to disambiguate.",
            symbol_name,
            matches.len()
        ))),
    }
}

/// Patch a symbol with new content.
///
/// Resolves the symbol via `MagellanIntegration`, then applies a span-safe
/// byte replacement. The file is modified in place.
///
/// # Errors
///
/// Returns `SpliceError::SymbolNotFound` if the symbol cannot be resolved.
/// Returns `SpliceError::InvalidSpan` if the byte span is out of bounds.
pub fn patch_symbol_in_file(
    file_path: &Path,
    symbol_name: &str,
    new_content: &str,
    db_path: &Path,
) -> Result<PatchResult> {
    let span = resolve_symbol_span(file_path, symbol_name, db_path)?;

    replace_span(&span.file, span.byte_start, span.byte_end, new_content)?;

    Ok(PatchResult {
        file: span.file,
        symbol: span.name,
        byte_start: span.byte_start,
        byte_end: span.byte_start + new_content.len(),
        lines_added: new_content.lines().count(),
    })
}

/// Rename a symbol across all files in the project.
///
/// Uses `MagellanIntegration` to find all references to the symbol,
/// then applies span-safe replacements in each file.
///
/// # Errors
///
/// Returns `SpliceError::SymbolNotFound` if no references are found.
pub fn rename_symbol_across_files(
    old_name: &str,
    new_name: &str,
    db_path: &Path,
) -> Result<RenameResult> {
    let mut integration = MagellanIntegration::open(db_path)?;

    let matches = integration.find_symbol_by_name(old_name, true)?;
    if matches.is_empty() {
        return Err(SpliceError::symbol_not_found(old_name, None));
    }

    let mut all_refs = Vec::new();
    for symbol_info in &matches {
        if let Ok(refs) = integration.get_all_references(symbol_info.entity_id) {
            all_refs.extend(refs);
        }
    }

    if all_refs.is_empty() {
        for symbol_info in &matches {
            all_refs.push(magellan::references::ReferenceFact {
                file_path: std::path::PathBuf::from(&symbol_info.file_path),
                referenced_symbol: old_name.to_string(),
                byte_start: symbol_info.byte_start,
                byte_end: symbol_info.byte_end,
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 0,
            });
        }
    }

    let by_file = group_references_by_file(&all_refs);
    let mut files_changed = Vec::new();
    let mut total_replacements = 0usize;

    for (file_path, refs) in by_file {
        match apply_replacements_in_file(&file_path, old_name, new_name, &refs) {
            Ok(count) if count > 0 => {
                files_changed.push(file_path);
                total_replacements += count;
            }
            Ok(_) => {}
            Err(_) => continue,
        }
    }

    Ok(RenameResult {
        files_changed,
        replacements: total_replacements,
    })
}

/// Delete a symbol from a file.
///
/// Resolves the symbol's byte span and replaces it with an empty string,
/// effectively removing it. Use with caution — does not check for references.
///
/// # Errors
///
/// Returns `SpliceError::SymbolNotFound` if the symbol cannot be resolved.
pub fn delete_symbol_from_file(
    file_path: &Path,
    symbol_name: &str,
    db_path: &Path,
) -> Result<PatchResult> {
    let span = resolve_symbol_span(file_path, symbol_name, db_path)?;

    replace_span(&span.file, span.byte_start, span.byte_end, "")?;

    Ok(PatchResult {
        file: span.file,
        symbol: span.name,
        byte_start: span.byte_start,
        byte_end: span.byte_start,
        lines_added: 0,
    })
}
