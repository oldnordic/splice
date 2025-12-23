//! Span-safe replacement engine with atomic writes and validation gates.
//!
//! This module provides byte-exact patching with:
//! - Atomic file replacement (write temp + fsync + rename)
//! - File hash validation (before/after)
//! - Tree-sitter reparse gate
//! - Cargo check gate
//! - Optional rust-analyzer gate
//! - Automatic rollback on any failure

use crate::error::{Result, SpliceError};
use crate::validate::AnalyzerMode;
use ropey::Rope;
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Apply a patch with full validation gates.
///
/// This function:
/// 1. Computes hash of original file
/// 2. Replaces [start..end] byte span with new_content
/// 3. Writes to temp file, fsyncs, atomic rename
/// 4. Runs tree-sitter reparse gate
/// 5. Runs cargo check gate
/// 6. Runs rust-analyzer gate (if enabled)
/// 7. On any failure, rolls back atomically
///
/// # Arguments
/// * `file_path` - Path to the file to patch
/// * `start` - Start byte offset (inclusive)
/// * `end` - End byte offset (exclusive)
/// * `new_content` - Replacement content
/// * `workspace_dir` - Directory containing Cargo.toml for validation
/// * `analyzer_mode` - rust-analyzer mode (off/path/explicit)
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
    let file_dir = file_path
        .parent()
        .ok_or_else(|| SpliceError::Other("File has no parent directory".to_string()))?;

    let temp_path = file_dir.join(format!(
        ".{}.tmp",
        file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("tmp")
    ));

    // Write patched content to temp file
    {
        let mut temp_file = File::create(&temp_path)?;
        temp_file.write_all(patched_content.as_bytes())?;
        temp_file.sync_all()?; // fsync for durability
    }

    // Step 6: Atomic rename over original
    std::fs::rename(&temp_path, file_path)?;

    // Step 7: Run validation gates
    match run_validation_gates(file_path, workspace_dir, analyzer_mode) {
        Ok(_) => {}
        Err(e) => {
            log::warn!("Validation failed, rolling back patch: {:?}", e);

            // Restore original content atomically
            let rollback_temp = file_dir.join(format!(
                ".{}.rollback.tmp",
                file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("tmp")
            ));

            let mut rollback_file = File::create(&rollback_temp)?;
            rollback_file.write_all(&original)?;
            rollback_file.sync_all()?;

            std::fs::rename(&rollback_temp, file_path)?;

            return Err(e);
        }
    }

    // Step 9: Compute after hash and return
    let patched_bytes = std::fs::read(file_path)?;
    let after_hash = compute_hash(&patched_bytes);

    Ok((before_hash, after_hash))
}

/// Run all validation gates in sequence.
///
/// Gates are executed in order:
/// 1. Tree-sitter reparse (syntax validation)
/// 2. Cargo check (semantic validation)
/// 3. rust-analyzer (optional lint validation)
///
/// If any gate fails, returns error immediately.
fn run_validation_gates(
    file_path: &Path,
    workspace_dir: &Path,
    analyzer_mode: AnalyzerMode,
) -> Result<()> {
    // Gate 1: Tree-sitter reparse
    gate_tree_sitter_reparse(file_path)?;

    // Gate 2: Cargo check
    gate_cargo_check(workspace_dir)?;

    // Gate 3: rust-analyzer (optional)
    use crate::validate::gate_rust_analyzer;
    gate_rust_analyzer(workspace_dir, analyzer_mode)?;

    Ok(())
}

/// Tree-sitter reparse gate.
///
/// Validates that the patched file can be parsed as valid Rust.
fn gate_tree_sitter_reparse(file_path: &Path) -> Result<()> {
    let source = std::fs::read(file_path)?;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::language())
        .map_err(|e| SpliceError::Parse {
            file: file_path.to_path_buf(),
            message: format!("Failed to set Rust language: {:?}", e),
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
            message: "Tree-sitter detected syntax errors in patched file".to_string(),
        });
    }

    Ok(())
}

/// Cargo check gate.
///
/// Validates that the workspace compiles after the patch.
fn gate_cargo_check(workspace_dir: &Path) -> Result<()> {
    use std::process::Command;

    let output = Command::new("cargo")
        .args(["check", "--message-format=short"])
        .current_dir(workspace_dir)
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    let combined = format!("{}{}", stderr, stdout);

    Err(SpliceError::CargoCheckFailed {
        workspace: workspace_dir.to_path_buf(),
        output: combined,
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

    std::str::from_utf8(&original.as_bytes()[start..end]).map_err(|_| SpliceError::InvalidSpan {
        file: file_path.to_path_buf(),
        start,
        end,
    })?;

    let mut rope = Rope::from_str(&original);
    let start_char = rope.byte_to_char(start);
    let end_char = rope.byte_to_char(end);

    rope.remove(start_char..end_char);
    rope.insert(start_char, new_content);

    std::fs::write(file_path, rope.to_string())?;

    Ok(())
}

/// Validate that a span aligns with UTF-8 boundaries.
/// Validate that a span aligns with UTF-8 boundaries.
pub fn validate_utf8_span(source: &str, start: usize, end: usize) -> Result<()> {
    std::str::from_utf8(&source.as_bytes()[start..end]).map_err(|_| SpliceError::InvalidSpan {
        file: std::path::PathBuf::from("<unknown>"),
        start,
        end,
    })?;
    Ok(())
}

