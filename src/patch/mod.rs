//! Span-safe replacement engine with atomic writes and validation gates.
//!
//! This module provides byte-exact patching with:
//! - Atomic file replacement (write temp + fsync + rename)
//! - File hash validation (before/after)
//! - Tree-sitter reparse gate (multi-language)
//! - Compiler validation gate (multi-language)
//! - Optional rust-analyzer gate (Rust only)
//! - Automatic rollback on any failure

mod backup;
mod batch_loader;
pub mod pattern;
pub mod text_replace;

pub(crate) mod gates;
pub(crate) mod preview;

use crate::error::{Result, SpliceError};
use crate::io_ext;
use crate::symbol::Language as SymbolLanguage;
use crate::validate::AnalyzerMode;
use crate::verify;
use ropey::Rope;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub use backup::{restore_from_manifest, BackupManifest, BackupWriter};
pub use batch_loader::load_batches_from_file;
pub(crate) use gates::run_validation_gates;
pub use pattern::{
    apply_pattern_replace, find_pattern_in_files, PatternReplaceConfig, PatternReplaceResult,
};

#[cfg(test)]
use preview::should_skip_entry;

pub use text_replace::edit_file;

/// Replacement to apply within a specific file.
#[derive(Debug, Clone, Serialize)]
pub struct SpanReplacement {
    /// Absolute or workspace-relative file path.
    pub file: PathBuf,
    /// Start byte offset (inclusive).
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
    /// Replacement contents.
    pub content: String,
}

impl SpanReplacement {
    /// Create a new span replacement.
    pub fn new(file: PathBuf, start: usize, end: usize, content: String) -> Self {
        Self {
            file,
            start,
            end,
            content,
        }
    }
}

/// Collection of replacements that must succeed atomically.
#[derive(Debug, Clone)]
pub struct SpanBatch {
    replacements: Vec<SpanReplacement>,
}

impl SpanBatch {
    /// Create a batch from raw replacements.
    pub fn new(replacements: Vec<SpanReplacement>) -> Self {
        Self { replacements }
    }

    /// Borrow the replacements for inspection.
    pub fn replacements(&self) -> &[SpanReplacement] {
        &self.replacements
    }

    /// Add a replacement to the batch.
    pub fn push(&mut self, replacement: SpanReplacement) {
        self.replacements.push(replacement);
    }

    /// Returns true when the batch contains no work.
    pub fn is_empty(&self) -> bool {
        self.replacements.is_empty()
    }
}

/// Result summary for a patched file.
#[derive(Debug, Clone, Serialize)]
pub struct FilePatchSummary {
    /// Path of the patched file.
    pub file: PathBuf,
    /// SHA-256 before patching.
    pub before_hash: String,
    /// SHA-256 after patching.
    pub after_hash: String,
}

/// Preview metadata describing the diff produced by a patch.
#[derive(Debug, Clone, Serialize)]
pub struct PreviewReport {
    /// The file that would be patched.
    pub file: String,
    /// 1-based line number where the change begins.
    pub line_start: usize,
    /// 1-based line number where the change ends.
    pub line_end: usize,
    /// Number of lines added by the patch.
    pub lines_added: usize,
    /// Number of lines removed by the patch.
    pub lines_removed: usize,
    /// Number of bytes inserted.
    pub bytes_added: usize,
    /// Number of bytes removed.
    pub bytes_removed: usize,
}

/// Apply a patch with comprehensive validation and automatic rollback.
///
/// This function:
/// 1. Pre-verification (file state, workspace resources, graph sync)
/// 2. Computes hash of original file
/// 3. Replaces [start..end] byte span with new_content
/// 4. Writes to temp file, fsyncs, atomic rename
/// 5. Runs tree-sitter reparse gate (language-specific)
/// 6. Runs compiler validation gate (language-specific)
/// 7. Runs rust-analyzer gate (if enabled and Rust)
/// 8. On any failure, rolls back atomically
///
/// # Rollback Behavior
///
/// If any validation gate fails after patching, the original content
/// is restored atomically. The rope mutation (remove + insert) happens
/// in memory first, then the result is written to a temp file. If
/// validation fails, we restore the original content.
///
/// # State Tracking
///
/// - `before_hash`: Content hash before patching
/// - `replaced`: Original bytes for rollback
/// - `after_hash`: Content hash after patching (for verification)
///
/// # Arguments
/// * `file_path` - Path to the file to patch
/// * `start` - Start byte offset (inclusive)
/// * `end` - End byte offset (exclusive)
/// * `new_content` - Replacement content
/// * `workspace_dir` - Directory containing project config for validation
/// * `language` - Programming language for validation gates
/// * `analyzer_mode` - rust-analyzer mode (off/path/explicit, Rust only)
///
/// # Returns
/// * `Ok((before_hash, after_hash))` - SHA-256 hashes before/after patch
/// * `Err(SpliceError)` - Validation failure with automatic rollback
#[allow(
    clippy::too_many_arguments,
    reason = "patch primitive: byte span + content + validation config"
)]
pub fn apply_patch_with_validation(
    file_path: &Path,
    start: usize,
    end: usize,
    new_content: &str,
    workspace_dir: &Path,
    language: SymbolLanguage,
    analyzer_mode: AnalyzerMode,
    strict: bool,
    skip: bool,
) -> Result<(String, String)> {
    // Step 0: Pre-verification before reading file
    // Note: skip=true for patch operations since they don't require a code graph database
    // (graph DB is only needed for query/get commands using Magellan)
    // strict and skip flags now passed from CLI
    let db_path = workspace_dir.join(".magellan/magellan.db"); // Updated to use Magellan convention
    let pre_checks =
        verify::pre_verify_patch(file_path, None, workspace_dir, &db_path, strict, skip)?;

    // Check for blocking failures
    for check in &pre_checks {
        if check.is_blocking() {
            return Err(SpliceError::PreVerificationFailed {
                check: format!("{:?}", check),
            });
        }
    }

    // Log warnings but don't fail
    for check in &pre_checks {
        if check.is_warning() {
            log::warn!("Pre-verification warning: {:?}", check);
        }
    }

    // Step 0.5: Check CFG complexity if Mirage available (non-blocking)
    // This uses Mirage's 4D spatial coordinates to assess function complexity
    if let Some(function_name) = extract_function_name_from_patch(new_content) {
        if let Ok(complexity) =
            crate::cfg_analysis::check_function_complexity(&db_path, &function_name, file_path)
        {
            match complexity.risk_level {
                crate::cfg_analysis::RiskLevel::VeryHigh => {
                    log::warn!(
                        "VERY HIGH COMPLEXITY: Function '{}' has branch distance={}, dominator depth={}, loop nesting={}. \
                        Consider manual review before automated refactoring.",
                        function_name,
                        complexity.max_branch_distance,
                        complexity.max_dominator_depth,
                        complexity.max_loop_nesting
                    );
                }
                crate::cfg_analysis::RiskLevel::High => {
                    log::warn!(
                        "HIGH COMPLEXITY: Function '{}' has branch distance={}, dominator depth={}. \
                        Automated refactoring may be risky.",
                        function_name,
                        complexity.max_branch_distance,
                        complexity.max_dominator_depth
                    );
                }
                crate::cfg_analysis::RiskLevel::Medium => {
                    log::info!(
                        "Medium complexity: Function '{}' (branch distance={}, dominator depth={})",
                        function_name,
                        complexity.max_branch_distance,
                        complexity.max_dominator_depth
                    );
                }
                crate::cfg_analysis::RiskLevel::Low => {
                    log::debug!(
                        "Low complexity: Function '{}' (branch distance={})",
                        function_name,
                        complexity.max_branch_distance
                    );
                }
            }
        }
        // Don't fail if Mirage unavailable - just log and continue
    }

    // Step 1: Read original file and compute hash
    let replaced = io_ext::read(file_path)?;
    let before_hash = compute_hash(&replaced);

    // Step 2: Validate span bounds
    if start > end || end > replaced.len() {
        return Err(SpliceError::InvalidSpan {
            file: file_path.to_path_buf(),
            start,
            end,
            file_size: replaced.len(),
        });
    }

    // Step 3: Validate UTF-8 boundaries
    std::str::from_utf8(&replaced[start..end]).map_err(|_| SpliceError::InvalidSpan {
        file: file_path.to_path_buf(),
        start,
        end,
        file_size: replaced.len(),
    })?;

    // Step 4: Apply byte-exact replacement using ropey
    // Note: rope.remove() and rope.insert() are in-memory operations.
    // If validation fails (Step 7), we rollback by restoring the original content.
    let mut rope = Rope::from_str(std::str::from_utf8(&replaced)?);
    let start_char = rope.byte_to_char(start);
    let end_char = rope.byte_to_char(end);

    // Mutate rope: remove old content, insert new content
    rope.remove(start_char..end_char);
    rope.insert(start_char, new_content);

    let patched_content = rope.to_string();

    // Step 5: Write to temp file in same directory (for atomic rename)
    let patched_bytes = patched_content.into_bytes();
    write_atomic(file_path, &patched_bytes, "patch")?;

    // Step 7: Run validation gates
    match gates::run_validation_gates(file_path, workspace_dir, language, analyzer_mode.clone()) {
        Ok(_) => {}
        Err(e) => {
            log::warn!("Validation failed, rolling back patch: {:?}", e);

            if let Err(rollback_err) = write_atomic(file_path, &replaced, "rollback") {
                log::error!(
                    "Failed to restore {} during rollback: {}",
                    file_path.display(),
                    rollback_err
                );
            }
            return Err(e);
        }
    }

    // Step 8: Compute after hash
    let refreshed_bytes = io_ext::read(file_path)?;
    let after_hash = compute_hash(&refreshed_bytes);

    // Step 9: Run post-verification to confirm expected changes
    let mut post_verify =
        verify::verify_after_patch(file_path, workspace_dir, &before_hash, analyzer_mode)?;

    // Step 9.1: Verify localized change (no unintended modifications)
    let localized = verify::verify_localized_change(file_path, &replaced, (start, end));

    match &localized {
        Ok(true) => {
            log::info!("Localized change verification passed");
        }
        Ok(false) => {
            log::warn!("Localized change verification detected modifications outside target span");
            post_verify.add_warning("File modified outside target span");
        }
        Err(e) => {
            log::warn!("Localized change verification failed: {}", e);
            post_verify.add_warning(format!("Could not verify localized change: {}", e));
        }
    }

    // Log warnings for user visibility
    for warning in &post_verify.warnings {
        log::warn!("Post-verification warning: {}", warning);
    }

    // Log errors (non-blocking, advisory)
    for error in &post_verify.errors {
        log::error!("Post-verification error: {}", error);
    }

    // Log post-verification status
    log::info!(
        "Post-verification: syntax={}, compiler={}, semantic={}, changed={}",
        post_verify.syntax_ok,
        post_verify.compiler_ok,
        post_verify.semantic_ok,
        post_verify.file_changed(),
    );

    Ok((before_hash, after_hash))
}

/// Apply multiple span replacements atomically across files.
///
/// All replacements are made durable before running validation gates. Any tree-sitter,
/// compiler, or analyzer failure restores every file to its original bytes before returning
/// the error.
pub fn apply_batch_with_validation(
    batches: &[SpanBatch],
    workspace_dir: &Path,
    language: SymbolLanguage,
    analyzer_mode: AnalyzerMode,
) -> Result<Vec<FilePatchSummary>> {
    if batches.is_empty() {
        return Ok(Vec::new());
    }

    let mut grouped: BTreeMap<PathBuf, Vec<SpanReplacement>> = BTreeMap::new();
    for batch in batches {
        for replacement in batch.replacements() {
            grouped
                .entry(replacement.file.clone())
                .or_default()
                .push(replacement.clone());
        }
    }

    let mut applied = Vec::new();

    for (file_path, mut replacements) in grouped {
        if replacements.is_empty() {
            continue;
        }

        // Pre-verify each file
        let pre_check = verify::verify_file_ready(&file_path, None, workspace_dir);
        if pre_check.is_blocking() {
            log::warn!(
                "Skipping {:?}: pre-verification failed: {:?}",
                file_path,
                pre_check
            );
            continue;
        }

        replacements.sort_by_key(|r| std::cmp::Reverse(r.start));
        let (replaced, before_hash) = read_with_hash(&file_path)?;
        validate_replacements(&file_path, &replacements, &replaced)?;
        let patched_bytes = apply_replacements(&replaced, &replacements)?;
        let after_hash = compute_hash(&patched_bytes);

        if let Err(write_err) = write_atomic(&file_path, &patched_bytes, "batch") {
            rollback_files(&applied);
            return Err(write_err);
        }

        applied.push(AppliedFile {
            file: file_path,
            replaced,
            before_hash,
            after_hash,
        });
    }

    let validation = run_batch_validations(&applied, workspace_dir, language, analyzer_mode);
    if let Err(err) = validation {
        rollback_files(&applied);
        return Err(err);
    }

    Ok(applied
        .into_iter()
        .map(|file| FilePatchSummary {
            file: file.file,
            before_hash: file.before_hash,
            after_hash: file.after_hash,
        })
        .collect())
}

/// Preview a patch by cloning the workspace, applying the change, and validating there.
pub fn preview_patch(
    file_path: &Path,
    start: usize,
    end: usize,
    new_content: &str,
    workspace_root: &Path,
    language: SymbolLanguage,
    analyzer_mode: AnalyzerMode,
) -> Result<(FilePatchSummary, PreviewReport)> {
    let preview_workspace = preview::clone_workspace_for_preview(workspace_root)?;
    let relative = file_path
        .strip_prefix(workspace_root)
        .map_err(|_| SpliceError::Other("File not under workspace root".to_string()))?;
    let preview_file = preview_workspace.path().join(relative);

    let (before_hash, after_hash) = apply_patch_with_validation(
        &preview_file,
        start,
        end,
        new_content,
        preview_workspace.path(),
        language,
        analyzer_mode,
        false, // strict: preview mode doesn't need strict validation
        true,  // skip: preview mode also doesn't need graph DB
    )?;

    let preview_report = compute_preview_report(file_path, start, end, new_content)?;

    Ok((
        FilePatchSummary {
            file: file_path.to_path_buf(),
            before_hash,
            after_hash,
        },
        preview_report,
    ))
}

/// Preview a patch and return before/after content for diff generation.
///
/// This extends the `preview_patch()` functionality by also returning the original
/// and patched file contents, enabling unified diff generation in dry-run mode.
///
/// # Arguments
/// Same parameters as `preview_patch()`:
/// * `file_path` - Path to the file to preview
/// * `start` - Start byte offset (inclusive)
/// * `end` - End byte offset (exclusive)
/// * `new_content` - Replacement content
/// * `workspace_root` - Root directory of the workspace
/// * `language` - Programming language for validation gates
/// * `analyzer_mode` - rust-analyzer mode (off/path/explicit, Rust only)
///
/// # Returns
/// Result containing a tuple of:
/// * `FilePatchSummary` - Hash information
/// * `PreviewReport` - Line/byte change statistics
/// * `String` - Original file content (before patch)
/// * `String` - Patched file content (after patch)
///
/// # Examples
/// ```ignore
/// use splice::patch::preview_patch_with_content;
///
/// let (summary, report, before, after) = preview_patch_with_content(
///     &file_path,
///     start,
///     end,
///     new_content,
///     &workspace_root,
///     language,
///     analyzer_mode,
/// )?;
/// ```
pub fn preview_patch_with_content(
    file_path: &Path,
    start: usize,
    end: usize,
    new_content: &str,
    workspace_root: &Path,
    language: SymbolLanguage,
    analyzer_mode: AnalyzerMode,
) -> Result<(FilePatchSummary, PreviewReport, String, String)> {
    // Ensure both paths are absolute for consistent prefix stripping
    let file_path = std::fs::canonicalize(file_path).map_err(|e| SpliceError::Io {
        path: file_path.to_path_buf(),
        source: e,
    })?;
    let workspace_root = std::fs::canonicalize(workspace_root).map_err(|e| SpliceError::Io {
        path: workspace_root.to_path_buf(),
        source: e,
    })?;

    let preview_workspace = preview::clone_workspace_for_preview(&workspace_root)?;
    let relative = file_path.strip_prefix(&workspace_root).map_err(|_| {
        SpliceError::Other(format!(
            "File {} not under workspace root {}",
            file_path.display(),
            workspace_root.display()
        ))
    })?;
    let preview_file = preview_workspace.path().join(relative);

    // Read original file content before patching
    let before_content = io_ext::read_to_string(&preview_file)?;

    let (before_hash, after_hash) = apply_patch_with_validation(
        &preview_file,
        start,
        end,
        new_content,
        preview_workspace.path(),
        language,
        analyzer_mode,
        false, // strict: preview mode doesn't need strict validation
        true,  // skip: preview mode also doesn't need graph DB
    )?;

    // Read patched file content
    let after_content = io_ext::read_to_string(&preview_file)?;

    let preview_report = compute_preview_report(&file_path, start, end, new_content)?;

    Ok((
        FilePatchSummary {
            file: file_path.to_path_buf(),
            before_hash,
            after_hash,
        },
        preview_report,
        before_content,
        after_content,
    ))
}

/// Compute SHA-256 hash of file contents.
fn compute_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Replace byte span without validation (legacy method for backward compatibility).
///
/// This is a simple span replacement without validation gates.
/// Prefer `apply_patch_with_validation` for all new code.
pub fn replace_span(file_path: &Path, start: usize, end: usize, new_content: &str) -> Result<()> {
    let replaced = io_ext::read_to_string(file_path)?;
    let file_size = replaced.len();

    if start > end || end > file_size {
        return Err(SpliceError::InvalidSpan {
            file: file_path.to_path_buf(),
            start,
            end,
            file_size,
        });
    }

    // Validate that the span is within bounds
    if end > file_size || start > end {
        return Err(SpliceError::InvalidSpan {
            file: file_path.to_path_buf(),
            start,
            end,
            file_size,
        });
    }

    let mut rope = Rope::from_str(&replaced);
    let start_char = rope.byte_to_char(start);
    let end_char = rope.byte_to_char(end);

    rope.remove(start_char..end_char);
    rope.insert(start_char, new_content);

    io_ext::write(file_path, rope.to_string())?;

    Ok(())
}

fn run_batch_validations(
    files: &[AppliedFile],
    workspace_dir: &Path,
    language: SymbolLanguage,
    analyzer_mode: AnalyzerMode,
) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    let mut requires_rust_validation = false;
    for file in files {
        gates::gate_tree_sitter_reparse(&file.file, language)?;
        if language == SymbolLanguage::Rust {
            requires_rust_validation = true;
        } else {
            gates::gate_compiler_validation(&file.file, workspace_dir, language)?;
        }
    }

    if requires_rust_validation {
        gates::gate_cargo_check(workspace_dir)?;
        if language == SymbolLanguage::Rust && analyzer_mode != AnalyzerMode::Off {
            use crate::validate::gate_rust_analyzer;
            gate_rust_analyzer(workspace_dir, analyzer_mode)?;
        }
    }

    Ok(())
}

fn validate_replacements(
    file_path: &Path,
    replacements: &[SpanReplacement],
    replaced: &[u8],
) -> Result<()> {
    if replacements.is_empty() {
        return Ok(());
    }
    let file_len = replaced.len();

    let mut sorted = replacements.to_vec();
    sorted.sort_by_key(|r| r.start);

    let mut previous_end: Option<usize> = None;
    for replacement in &sorted {
        if replacement.start > replacement.end || replacement.end > file_len {
            return Err(SpliceError::InvalidSpan {
                file: file_path.to_path_buf(),
                start: replacement.start,
                end: replacement.end,
                file_size: file_len,
            });
        }

        std::str::from_utf8(&replaced[replacement.start..replacement.end]).map_err(|_| {
            SpliceError::InvalidSpan {
                file: file_path.to_path_buf(),
                start: replacement.start,
                end: replacement.end,
                file_size: file_len,
            }
        })?;

        if let Some(prev_end) = previous_end {
            if replacement.start < prev_end {
                return Err(SpliceError::Other(format!(
                    "Overlapping replacements detected in {}",
                    file_path.display()
                )));
            }
        }
        previous_end = Some(replacement.end);
    }

    Ok(())
}

fn apply_replacements(replaced: &[u8], replacements: &[SpanReplacement]) -> Result<Vec<u8>> {
    let content = std::str::from_utf8(replaced)?;
    let mut rope = Rope::from_str(content);

    for replacement in replacements {
        let start_char = rope.byte_to_char(replacement.start);
        let end_char = rope.byte_to_char(replacement.end);
        rope.remove(start_char..end_char);
        rope.insert(start_char, &replacement.content);
    }

    Ok(rope.to_string().into_bytes())
}

fn read_with_hash(path: &Path) -> Result<(Vec<u8>, String)> {
    let data = io_ext::read(path)?;
    let hash = compute_hash(&data);
    Ok((data, hash))
}

fn rollback_files(files: &[AppliedFile]) {
    for file in files.iter().rev() {
        if let Err(err) = write_atomic(&file.file, &file.replaced, "rollback") {
            log::error!("Rollback failed for {}: {}", file.file.display(), err);
        }
    }
}

fn write_atomic(file_path: &Path, content: &[u8], suffix: &str) -> Result<()> {
    let temp_path = temp_path_for(file_path, suffix)?;
    let mut temp_file = File::create(&temp_path).map_err(|source| SpliceError::Io {
        path: temp_path.clone(),
        source,
    })?;
    temp_file
        .write_all(content)
        .map_err(|source| SpliceError::Io {
            path: temp_path.clone(),
            source,
        })?;
    temp_file.sync_all().map_err(|source| SpliceError::Io {
        path: temp_path.clone(),
        source,
    })?;
    std::fs::rename(&temp_path, file_path).map_err(|source| SpliceError::Io {
        path: file_path.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn temp_path_for(file_path: &Path, suffix: &str) -> Result<PathBuf> {
    let file_dir = file_path
        .parent()
        .ok_or_else(|| SpliceError::Other("File has no parent directory".to_string()))?;
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("tmp");
    Ok(file_dir.join(format!(".{}.{}.tmp", file_name, suffix)))
}

struct AppliedFile {
    file: PathBuf,
    replaced: Vec<u8>,
    before_hash: String,
    after_hash: String,
}

/// Calculate line counts for a patch operation.
///
/// This is a public function so it can be reused in different contexts
/// (preview mode, actual patch, JSON output, etc.).
///
/// # Arguments
/// * `file_path` - Path to the file
/// * `start` - Byte offset where the replacement starts
/// * `end` - Byte offset where the replacement ends
/// * `new_content` - The new content to insert
///
/// # Returns
/// * `PreviewReport` - Contains line counts, byte counts, and line numbers
pub fn compute_preview_report(
    file_path: &Path,
    start: usize,
    end: usize,
    new_content: &str,
) -> Result<PreviewReport> {
    let replaced = io_ext::read(file_path)?;
    let source = std::str::from_utf8(&replaced)?;
    let rope = Rope::from_str(source);

    let start_line = rope.byte_to_line(start);
    let end_line = if end == start {
        start_line
    } else if end == replaced.len() {
        rope.len_lines().saturating_sub(1)
    } else {
        rope.byte_to_line(end)
    };

    let lines_removed = if end > start {
        source[start..end].lines().count()
    } else {
        0
    };
    let lines_added = if new_content.is_empty() {
        0
    } else {
        new_content.lines().count()
    };

    let bytes_removed = end.saturating_sub(start);
    let bytes_added = new_content.len();

    Ok(PreviewReport {
        file: file_path.to_string_lossy().into_owned(),
        line_start: start_line + 1,
        line_end: if lines_removed == 0 {
            start_line + 1
        } else {
            end_line + 1
        },
        lines_added,
        lines_removed,
        bytes_added,
        bytes_removed,
    })
}

/// Validate that a span aligns with UTF-8 boundaries.
pub fn validate_utf8_span(file_path: &Path, source: &str, start: usize, end: usize) -> Result<()> {
    let file_size = source.len();

    // Validate that the span is within bounds
    if end > file_size || start > end {
        return Err(SpliceError::InvalidSpan {
            file: file_path.to_path_buf(),
            start,
            end,
            file_size,
        });
    }
    // If source is valid UTF-8, any slice of it is also valid UTF-8
    let _ = &source[start..end];
    Ok(())
}

/// Extract function name from patch content for CFG complexity analysis
///
/// This is a simple heuristic that looks for common function declaration patterns.
/// It's used to query Mirage for CFG complexity metrics before applying a patch.
///
/// # Arguments
/// * `patch_content` - The new content being patched in
///
/// # Returns
/// * `Some(function_name)` - If a function declaration is found
/// * `None` - If no function declaration detected
///
/// # Notes
/// - Uses simple regex patterns (not full parsing)
/// - Supports `fn name`, `pub fn name`, `async fn name`, etc.
/// - Falls back gracefully if not found (Mirage check is optional)
fn extract_function_name_from_patch(patch_content: &str) -> Option<String> {
    // Look for Rust function declarations
    // Patterns: "fn name(", "pub fn name(", "async fn name(", etc.
    use regex::Regex;

    // Lazy-init regex to avoid compiling on every call
    let fn_regex =
        Regex::new(r"(?m)^(?:pub\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)\s*\(").ok()?;

    fn_regex
        .captures(patch_content)
        .map(|caps| caps[1].to_string())
}

#[cfg(test)]
#[path = "patch_tests.rs"]
mod tests;
