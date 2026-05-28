//! Pattern-based search and replace with AST confirmation.
//!
//! This module provides multi-file pattern replacement capabilities
//! using glob patterns for file discovery and tree-sitter for AST
//! confirmation to ensure replacements land on the intended tokens.

use crate::symbol::Language;
use serde::Serialize;
use std::path::PathBuf;

pub mod apply;
pub mod search;
pub use apply::*;
pub use search::*;

/// Configuration for pattern-based replacement.
#[derive(Debug, Clone)]
pub struct PatternReplaceConfig {
    /// Glob pattern for matching files.
    pub glob_pattern: String,
    /// Text pattern to find.
    pub find_pattern: String,
    /// Replacement text.
    pub replace_pattern: String,
    /// Optional language hint (auto-detected from file extension if not provided).
    pub language: Option<Language>,
    /// Whether to apply validation gates.
    pub validate: bool,
}

/// A match found during pattern search.
#[derive(Debug, Clone, Serialize)]
pub struct PatternMatch {
    /// File where the match was found.
    pub file: PathBuf,
    /// Byte start of the match.
    pub byte_start: usize,
    /// Byte end of the match.
    pub byte_end: usize,
    /// Line number (1-based).
    pub line: usize,
    /// Column number (0-based).
    pub column: usize,
    /// The matched text.
    pub matched_text: String,

    // Optional: Context fields (populated when requested)
    /// Context lines before the match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_before: Option<Vec<String>>,

    /// Context lines after the match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_after: Option<Vec<String>>,
}

/// Result of a pattern replacement operation.
#[derive(Debug, Clone)]
pub struct PatternReplaceResult {
    /// Files that were patched.
    pub files_patched: Vec<PathBuf>,
    /// Number of replacements made.
    pub replacements_count: usize,
    /// Validation errors (if any).
    pub validation_errors: Vec<String>,
}

#[cfg(test)]
#[path = "pattern_tests.rs"]
mod tests;
