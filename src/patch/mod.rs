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
mod pattern;

use crate::error::{Diagnostic, DiagnosticLevel, Result, SpliceError};
use crate::symbol::Language as SymbolLanguage;
use crate::validate::{self, AnalyzerMode};
use ropey::Rope;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub use backup::{restore_from_manifest, BackupWriter, BackupManifest};
pub use batch_loader::load_batches_from_file;
pub use pattern::{find_pattern_in_files, apply_pattern_replace, PatternReplaceConfig, PatternReplaceResult};

/// Replacement to apply within a specific file.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

/// Apply a patch with full validation gates.
///
/// This function:
/// 1. Computes hash of original file
/// 2. Replaces [start..end] byte span with new_content
/// 3. Writes to temp file, fsyncs, atomic rename
/// 4. Runs tree-sitter reparse gate (language-specific)
/// 5. Runs compiler validation gate (language-specific)
/// 6. Runs rust-analyzer gate (if enabled and Rust)
/// 7. On any failure, rolls back atomically
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
pub fn apply_patch_with_validation(
    file_path: &Path,
    start: usize,
    end: usize,
    new_content: &str,
    workspace_dir: &Path,
    language: SymbolLanguage,
    analyzer_mode: AnalyzerMode,
) -> Result<(String, String)> {
    // Step 1: Read original file and compute hash
    let original = std::fs::read(file_path)?;
    let before_hash = compute_hash(&original);

    // Step 2: Validate span bounds
    if start > end || end > original.len() {
        return Err(SpliceError::InvalidSpan {
            file: file_path.to_path_buf(),
            start,
            end,
        });
    }

    // Step 3: Validate UTF-8 boundaries
    std::str::from_utf8(&original[start..end]).map_err(|_| SpliceError::InvalidSpan {
        file: file_path.to_path_buf(),
        start,
        end,
    })?;

    // Step 4: Apply byte-exact replacement using ropey
    let mut rope = Rope::from_str(std::str::from_utf8(&original)?);
    let start_char = rope.byte_to_char(start);
    let end_char = rope.byte_to_char(end);

    rope.remove(start_char..end_char);
    rope.insert(start_char, new_content);

    let patched_content = rope.to_string();

    // Step 5: Write to temp file in same directory (for atomic rename)
    let patched_bytes = patched_content.into_bytes();
    write_atomic(file_path, &patched_bytes, "patch")?;

    // Step 7: Run validation gates
    match run_validation_gates(file_path, workspace_dir, language, analyzer_mode) {
        Ok(_) => {}
        Err(e) => {
            log::warn!("Validation failed, rolling back patch: {:?}", e);

            if let Err(rollback_err) = write_atomic(file_path, &original, "rollback") {
                log::error!(
                    "Failed to restore {} during rollback: {}",
                    file_path.display(),
                    rollback_err
                );
            }
            return Err(e);
        }
    }

    // Step 9: Compute after hash and return
    let refreshed_bytes = std::fs::read(file_path)?;
    let after_hash = compute_hash(&refreshed_bytes);

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

        replacements.sort_by_key(|r| std::cmp::Reverse(r.start));
        let (original, before_hash) = read_with_hash(&file_path)?;
        validate_replacements(&file_path, &replacements, &original)?;
        let patched_bytes = apply_replacements(&original, &replacements)?;
        let after_hash = compute_hash(&patched_bytes);

        if let Err(write_err) = write_atomic(&file_path, &patched_bytes, "batch") {
            rollback_files(&applied);
            return Err(write_err);
        }

        applied.push(AppliedFile {
            file: file_path,
            original,
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
    let preview_workspace = clone_workspace_for_preview(workspace_root)?;
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

/// Run all validation gates in sequence.
///
/// Gates are executed in order:
/// 1. Tree-sitter reparse (syntax validation, language-specific)
/// 2. Compiler validation (language-specific)
/// 3. rust-analyzer (optional, Rust only)
///
/// If any gate fails, returns error immediately.
fn run_validation_gates(
    file_path: &Path,
    workspace_dir: &Path,
    language: SymbolLanguage,
    analyzer_mode: AnalyzerMode,
) -> Result<()> {
    // Gate 1: Tree-sitter reparse (language-specific)
    gate_tree_sitter_reparse(file_path, language)?;

    // Gate 2: Compiler validation (language-specific)
    gate_compiler_validation(file_path, workspace_dir, language)?;

    // Gate 3: rust-analyzer (Rust only, optional)
    if language == SymbolLanguage::Rust {
        use crate::validate::gate_rust_analyzer;
        gate_rust_analyzer(workspace_dir, analyzer_mode)?;
    }

    Ok(())
}

/// Tree-sitter reparse gate (language-specific).
///
/// Validates that the patched file can be parsed as valid syntax
/// for the given programming language.
fn gate_tree_sitter_reparse(file_path: &Path, language: SymbolLanguage) -> Result<()> {
    let source = std::fs::read(file_path)?;

    let mut parser = tree_sitter::Parser::new();
    let tree_sitter_lang = get_tree_sitter_language(language);

    parser
        .set_language(&tree_sitter_lang)
        .map_err(|e| SpliceError::Parse {
            file: file_path.to_path_buf(),
            message: format!("Failed to set language: {:?}", e),
        })?;

    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| SpliceError::ParseValidationFailed {
            file: file_path.to_path_buf(),
            message: "Parse failed - no tree returned".to_string(),
        })?;

    // Check for parse errors
    if tree.root_node().has_error() {
        return Err(SpliceError::ParseValidationFailed {
            file: file_path.to_path_buf(),
            message: format!(
                "Tree-sitter detected syntax errors in patched {} file",
                language.as_str()
            ),
        });
    }

    Ok(())
}

/// Get the appropriate tree-sitter language for the given SymbolLanguage.
fn get_tree_sitter_language(language: SymbolLanguage) -> tree_sitter::Language {
    match language {
        SymbolLanguage::Rust => tree_sitter_rust::language(),
        SymbolLanguage::Python => tree_sitter_python::language(),
        SymbolLanguage::C => tree_sitter_c::language(),
        SymbolLanguage::Cpp => tree_sitter_cpp::language(),
        SymbolLanguage::Java => tree_sitter_java::language(),
        SymbolLanguage::JavaScript => tree_sitter_javascript::language(),
        SymbolLanguage::TypeScript => tree_sitter_typescript::language_typescript(),
    }
}

/// Compiler validation gate (language-specific).
///
/// Validates that the patched file compiles using the appropriate
/// compiler for each language (via validate::gates::validate_file).
fn gate_compiler_validation(
    file_path: &Path,
    workspace_dir: &Path,
    language: SymbolLanguage,
) -> Result<()> {
    match language {
        SymbolLanguage::Rust => {
            // Rust: Use cargo check from workspace directory
            gate_cargo_check(workspace_dir)?;
        }
        _ => {
            // Other languages: Use validate_file which auto-detects language
            use crate::validate::gates::validate_file;

            let outcome = validate_file(file_path)?;
            let tool_metadata = tool_invocation_for_language(language)
                .map(|inv| validate::collect_tool_metadata(inv.binary, inv.version_args));

            if !outcome.is_valid {
                if !outcome.tool_available {
                    // Tool not available is a soft failure - we can't validate
                    // For now, we treat this as success but log a warning
                    log::warn!(
                        "Compiler validation tool not available for {}, skipping validation",
                        language.as_str()
                    );
                    return Ok(());
                }

                // Tool is available but validation failed
                let mut diagnostics = Vec::new();
                let tool_name = format!("{}-compiler", language.as_str());

                for err in outcome.errors {
                    let remediation = err
                        .code
                        .as_deref()
                        .and_then(validate::remediation_link_for_code);
                    diagnostics.push(
                        Diagnostic::new(&tool_name, DiagnosticLevel::Error, err.message)
                            .with_file(file_for_diagnostic(&err.file, file_path))
                            .with_position(nonzero(err.line), nonzero(err.column))
                            .with_code(err.code.clone())
                            .with_note(err.note.clone())
                            .with_tool_metadata(tool_metadata.as_ref())
                            .with_remediation(remediation),
                    );
                }

                for warn in outcome.warnings {
                    let remediation = warn
                        .code
                        .as_deref()
                        .and_then(validate::remediation_link_for_code);
                    diagnostics.push(
                        Diagnostic::new(&tool_name, DiagnosticLevel::Warning, warn.message)
                            .with_file(file_for_diagnostic(&warn.file, file_path))
                            .with_position(nonzero(warn.line), nonzero(warn.column))
                            .with_code(warn.code.clone())
                            .with_note(warn.note.clone())
                            .with_tool_metadata(tool_metadata.as_ref())
                            .with_remediation(remediation),
                    );
                }

                return Err(SpliceError::CompilerValidationFailed {
                    file: file_path.to_path_buf(),
                    language: language.as_str().to_string(),
                    diagnostics,
                });
            }
        }
    }

    Ok(())
}

fn file_for_diagnostic(reported: &str, fallback: &Path) -> PathBuf {
    if reported.is_empty() {
        fallback.to_path_buf()
    } else {
        PathBuf::from(reported)
    }
}

fn nonzero(value: usize) -> Option<usize> {
    if value == 0 {
        None
    } else {
        Some(value)
    }
}

struct ToolInvocation {
    binary: &'static str,
    version_args: &'static [&'static str],
}

fn tool_invocation_for_language(language: SymbolLanguage) -> Option<ToolInvocation> {
    match language {
        SymbolLanguage::Python => Some(ToolInvocation {
            binary: "python",
            version_args: &["--version"],
        }),
        SymbolLanguage::C => Some(ToolInvocation {
            binary: "gcc",
            version_args: &["--version"],
        }),
        SymbolLanguage::Cpp => Some(ToolInvocation {
            binary: "g++",
            version_args: &["--version"],
        }),
        SymbolLanguage::Java => Some(ToolInvocation {
            binary: "javac",
            version_args: &["-version"],
        }),
        SymbolLanguage::JavaScript => Some(ToolInvocation {
            binary: "node",
            version_args: &["--version"],
        }),
        SymbolLanguage::TypeScript => Some(ToolInvocation {
            binary: "tsc",
            version_args: &["--version"],
        }),
        _ => None,
    }
}

/// Cargo check gate (Rust-specific).
///
/// Validates that the workspace compiles after the patch.
fn gate_cargo_check(workspace_dir: &Path) -> Result<()> {
    use std::process::Command;

    let output = Command::new("cargo")
        .arg("check")
        .current_dir(workspace_dir)
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    let combined = format!("{}{}", stderr, stdout);

    if output.status.success() {
        return Ok(());
    }

    let compiler_errors = validate::parse_cargo_output(&stderr);
    let mut diagnostics = Vec::new();
    let cargo_meta = validate::collect_tool_metadata("cargo", &["--version"]);

    if compiler_errors.is_empty() {
        diagnostics.push(
            Diagnostic::new("cargo-check", DiagnosticLevel::Error, combined.clone())
                .with_file(workspace_dir.to_path_buf())
                .with_tool_metadata(Some(&cargo_meta)),
        );
    } else {
        for err in compiler_errors {
            let remediation = err
                .code
                .as_deref()
                .and_then(validate::remediation_link_for_code);
            diagnostics.push(
                Diagnostic::new("cargo-check", DiagnosticLevel::from(err.level), err.message)
                    .with_file(PathBuf::from(err.file))
                    .with_position(nonzero(err.line), nonzero(err.column))
                    .with_code(err.code.clone())
                    .with_note(err.note.clone())
                    .with_tool_metadata(Some(&cargo_meta))
                    .with_remediation(remediation),
            );
        }
    }

    Err(SpliceError::CargoCheckFailed {
        workspace: workspace_dir.to_path_buf(),
        output: combined,
        diagnostics,
    })
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
    let original = std::fs::read_to_string(file_path)?;

    if start > end || end > original.len() {
        return Err(SpliceError::InvalidSpan {
            file: file_path.to_path_buf(),
            start,
            end,
        });
    }

    // Validate that the span is within bounds
    if end > original.len() || start > end {
        return Err(SpliceError::InvalidSpan {
            file: file_path.to_path_buf(),
            start,
            end,
        });
    }

    let mut rope = Rope::from_str(&original);
    let start_char = rope.byte_to_char(start);
    let end_char = rope.byte_to_char(end);

    rope.remove(start_char..end_char);
    rope.insert(start_char, new_content);

    std::fs::write(file_path, rope.to_string())?;

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
        gate_tree_sitter_reparse(&file.file, language)?;
        if language == SymbolLanguage::Rust {
            requires_rust_validation = true;
        } else {
            gate_compiler_validation(&file.file, workspace_dir, language)?;
        }
    }

    if requires_rust_validation {
        gate_cargo_check(workspace_dir)?;
        if language == SymbolLanguage::Rust {
            if analyzer_mode != AnalyzerMode::Off {
                use crate::validate::gate_rust_analyzer;
                gate_rust_analyzer(workspace_dir, analyzer_mode)?;
            }
        }
    }

    Ok(())
}

fn validate_replacements(
    file_path: &Path,
    replacements: &[SpanReplacement],
    original: &[u8],
) -> Result<()> {
    if replacements.is_empty() {
        return Ok(());
    }
    let file_len = original.len();

    let mut sorted = replacements.to_vec();
    sorted.sort_by_key(|r| r.start);

    let mut previous_end: Option<usize> = None;
    for replacement in &sorted {
        if replacement.start > replacement.end || replacement.end > file_len {
            return Err(SpliceError::InvalidSpan {
                file: file_path.to_path_buf(),
                start: replacement.start,
                end: replacement.end,
            });
        }

        std::str::from_utf8(&original[replacement.start..replacement.end]).map_err(|_| {
            SpliceError::InvalidSpan {
                file: file_path.to_path_buf(),
                start: replacement.start,
                end: replacement.end,
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

fn apply_replacements(original: &[u8], replacements: &[SpanReplacement]) -> Result<Vec<u8>> {
    let content = std::str::from_utf8(original)?;
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
    let data = std::fs::read(path)?;
    let hash = compute_hash(&data);
    Ok((data, hash))
}

fn rollback_files(files: &[AppliedFile]) {
    for file in files.iter().rev() {
        if let Err(err) = write_atomic(&file.file, &file.original, "rollback") {
            log::error!(
                "Rollback failed for {}: {}",
                file.file.display(),
                err.to_string()
            );
        }
    }
}

fn write_atomic(file_path: &Path, content: &[u8], suffix: &str) -> Result<()> {
    let temp_path = temp_path_for(file_path, suffix)?;
    let mut temp_file = File::create(&temp_path)?;
    temp_file.write_all(content)?;
    temp_file.sync_all()?;
    std::fs::rename(&temp_path, file_path)?;
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
    original: Vec<u8>,
    before_hash: String,
    after_hash: String,
}

fn clone_workspace_for_preview(workspace_root: &Path) -> Result<TempDir> {
    let preview_dir = TempDir::new()?;
    copy_dir_recursive(workspace_root, preview_dir.path())?;
    Ok(preview_dir)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if should_skip_entry(&entry.file_name()) {
            continue;
        }

        let dest = dst.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else if file_type.is_file() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &dest)?;
        }
    }

    Ok(())
}

fn should_skip_entry(name: &OsStr) -> bool {
    matches!(
        name.to_string_lossy().as_ref(),
        ".git"
            | ".splice-backup"
            | "target"
            | "node_modules"
            | ".splice_graph.db"
            | ".splice_graph.db-shm"
            | ".splice_graph.db-wal"
            | "codegraph.db"
            | "magellan.db"
            | "operations.db"
            | "splice_map.db"
            | "syncore_code_graph.db"
            | "syncore_code_graph.db-shm"
            | "syncore_code_graph.db-wal"
    )
}

fn compute_preview_report(
    file_path: &Path,
    start: usize,
    end: usize,
    new_content: &str,
) -> Result<PreviewReport> {
    let original = fs::read(file_path)?;
    let source = std::str::from_utf8(&original)?;
    let rope = Rope::from_str(source);

    let start_line = rope.byte_to_line(start);
    let end_line = if end == start {
        start_line
    } else if end == original.len() {
        rope.len_lines().saturating_sub(1)
    } else {
        rope.byte_to_line(end)
    };

    let lines_removed = if end > start {
        (&source[start..end]).lines().count()
    } else {
        0
    };
    let lines_added = if new_content.is_empty() {
        0
    } else {
        new_content.lines().count()
    };

    let bytes_removed = end.saturating_sub(start);
    let bytes_added = new_content.as_bytes().len();

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
pub fn validate_utf8_span(source: &str, start: usize, end: usize) -> Result<()> {
    // Validate that the span is within bounds
    if end > source.len() || start > end {
        return Err(SpliceError::InvalidSpan {
            file: std::path::PathBuf::from("<unknown>"),
            start,
            end,
        });
    }
    // If source is valid UTF-8, any slice of it is also valid UTF-8
    let _ = &source[start..end];
    Ok(())
}
