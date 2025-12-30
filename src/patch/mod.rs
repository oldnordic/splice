//! Span-safe replacement engine with atomic writes and validation gates.
//!
//! This module provides byte-exact patching with:
//! - Atomic file replacement (write temp + fsync + rename)
//! - File hash validation (before/after)
//! - Tree-sitter reparse gate (multi-language)
//! - Compiler validation gate (multi-language)
//! - Optional rust-analyzer gate (Rust only)
//! - Automatic rollback on any failure

use crate::error::{Result, SpliceError};
use crate::symbol::Language as SymbolLanguage;
use crate::validate::AnalyzerMode;
use ropey::Rope;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Write;
use std::path::Path;

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
    match run_validation_gates(file_path, workspace_dir, language, analyzer_mode) {
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
    // SAFETY: tree-sitter language functions are always valid
    // We use unsafe to call the language() function from the C ABI
    // This is the standard way to get tree-sitter languages
    unsafe {
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
}

/// Compiler validation gate (language-specific).
///
/// Validates that the patched file compiles using the appropriate
/// compiler for each language (via validate::gates::validate_file).
fn gate_compiler_validation(file_path: &Path, workspace_dir: &Path, language: SymbolLanguage) -> Result<()> {
    match language {
        SymbolLanguage::Rust => {
            // Rust: Use cargo check from workspace directory
            gate_cargo_check(workspace_dir)?;
        }
        _ => {
            // Other languages: Use validate_file which auto-detects language
            use crate::validate::gates::validate_file;

            let outcome = validate_file(file_path)?;

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
                let error_messages: Vec<String> = outcome
                    .errors
                    .into_iter()
                    .map(|e| format!("{}:{}: {}", e.file, e.line, e.message))
                    .collect();

                return Err(SpliceError::CompilerValidationFailed {
                    file: file_path.to_path_buf(),
                    language: language.as_str().to_string(),
                    errors: error_messages,
                });
            }
        }
    }

    Ok(())
}

/// Cargo check gate (Rust-specific).
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
