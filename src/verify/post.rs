//! Post-verification hooks for safe refactoring operations.
//!
//! Post-verification runs AFTER file modifications to:
//! - Validate syntax (tree-sitter reparse)
//! - Validate compilation (language-specific)
//! - Verify semantic preservation
//! - Check for unintended side effects
//! - Compare checksums to document actual changes

use crate::error::Result;
use crate::io_ext;
use std::path::Path;

/// Post-verification result.
#[derive(Debug, Clone, PartialEq)]
pub struct PostVerificationResult {
    /// Syntax validation passed (tree-sitter parse)
    pub syntax_ok: bool,

    /// Compiler validation passed (cargo check / rustc)
    /// NOTE: This catches semantic errors: type mismatches, borrow checker,
    /// unresolved references, missing methods, etc.
    pub compiler_ok: bool,

    /// Enhanced LSP diagnostics (rust-analyzer, advisory/non-blocking)
    /// Currently not implemented. Use compiler_ok for semantic validation.
    pub semantic_ok: bool,

    /// Checksums before and after
    pub before_checksum: String,
    /// After checksum
    pub after_checksum: String,

    /// Warnings (non-blocking issues)
    pub warnings: Vec<String>,

    /// Errors (blocking issues that would have failed validation)
    pub errors: Vec<String>,
}

impl PostVerificationResult {
    /// Create a new post-verification result.
    pub fn new(
        syntax_ok: bool,
        compiler_ok: bool,
        before_checksum: String,
        after_checksum: String,
    ) -> Self {
        Self {
            syntax_ok,
            compiler_ok,
            semantic_ok: true, // Default to true (advisory)
            before_checksum,
            after_checksum,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add a warning.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Add an error.
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    /// Check if file changed (checksums differ).
    pub fn file_changed(&self) -> bool {
        self.before_checksum != self.after_checksum
    }
}

/// Checksum difference comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct ChecksumDiff {
    /// Checksums are different (change occurred)
    pub changed: bool,
    /// Estimated size of change (bytes, negative = smaller)
    pub estimated_delta: i64,
}

/// Compare checksums to document what changed.
pub fn checksum_diff(before_checksum: &str, after_checksum: &str) -> ChecksumDiff {
    let changed = before_checksum != after_checksum;

    // Delta estimation isn't implemented — actual file sizes would be required.
    let estimated_delta = 0;

    ChecksumDiff {
        changed,
        estimated_delta,
    }
}

/// Verify file after patching.
///
/// Runs:
/// - Syntax validation (via tree-sitter if available)
/// - Compiler validation (via cargo check for Rust)
/// - Checksum verification (confirm expected changes)
/// - Semantic checks (rust-analyzer diagnostics - advisory)
///
/// Returns a PostVerificationResult with detailed status.
pub fn verify_after_patch(
    file_path: &Path,
    workspace_root: &Path,
    expected_before: &str,
    analyzer_mode: crate::validate::AnalyzerMode,
) -> Result<PostVerificationResult> {
    use crate::checksum::checksum_file;

    let mut result = PostVerificationResult::new(
        false, // syntax_ok - will be set below
        false, // compiler_ok - will be set below
        expected_before.to_string(),
        String::new(), // after_checksum - will be set below
    );

    // Compute after checksum
    match checksum_file(file_path) {
        Ok(after) => {
            result.after_checksum = after.as_hex().to_string();

            // Check if file actually changed
            if !result.file_changed() {
                result.add_warning("File checksum unchanged - no modification detected");
            }
        }
        Err(e) => {
            result.add_error(format!("Failed to compute after checksum: {}", e));
            return Ok(result); // Return with error logged
        }
    }

    // Syntax validation: parse with tree-sitter
    use crate::syntax_validator::validate_syntax;
    match std::fs::read(file_path) {
        Ok(source) => {
            result.syntax_ok = match validate_syntax(file_path, &source) {
                Ok(valid) => {
                    if !valid {
                        result.add_error(
                            "Syntax validation failed: tree-sitter detected parse errors"
                                .to_string(),
                        );
                    }
                    valid
                }
                Err(e) => {
                    log::warn!("Syntax validation error: {}", e);
                    false
                }
            };
        }
        Err(e) => {
            result.add_error(format!("Failed to read file for syntax validation: {}", e));
            result.syntax_ok = false;
        }
    }

    // Compiler validation: run cargo check for Rust projects
    result.compiler_ok = match run_cargo_check(workspace_root) {
        Ok(output) => {
            if output.status.success() {
                log::info!("Cargo check passed");
                true
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                result.add_error(format!("Cargo check failed: {}", stderr));
                log::warn!("Cargo check failed: {}", stderr);
                false
            }
        }
        Err(e) => {
            // Don't block if cargo check unavailable (not a Rust project or cargo not installed)
            result.add_warning(format!("Unable to run cargo check: {}", e));
            log::debug!("Unable to run cargo check: {}", e);
            true // Allow patch to proceed
        }
    };

    // Semantic validation via rust-analyzer (advisory, non-blocking for result)
    if workspace_root.join("Cargo.toml").exists()
        && analyzer_mode != crate::validate::AnalyzerMode::Off
    {
        match crate::validate::run_rust_analyzer(workspace_root, &analyzer_mode) {
            Ok(diagnostics) => {
                let (errors, warnings): (Vec<_>, Vec<_>) = diagnostics
                    .iter()
                    .partition(|d| d.level == crate::validate::ErrorLevel::Error);

                for warn in &warnings {
                    result.add_warning(format!(
                        "rust-analyzer [{}] at {}:{}: {}",
                        warn.code.as_deref().unwrap_or("warning"),
                        warn.file,
                        warn.line,
                        warn.message
                    ));
                }

                for err in &errors {
                    result.add_error(format!(
                        "rust-analyzer [{}] at {}:{}: {}",
                        err.code.as_deref().unwrap_or("error"),
                        err.file,
                        err.line,
                        err.message
                    ));
                }

                result.semantic_ok = errors.is_empty();
            }
            Err(e) => {
                result.add_warning(format!("rust-analyzer unavailable: {}", e));
                result.semantic_ok = true; // Advisory: don't block if analyzer can't run
            }
        }
    } else {
        result.semantic_ok = true;
    }

    Ok(result)
}

/// Verify that changes were localized to the target span.
///
/// Reads current file, masks out the target span region, and verifies
/// that non-target regions match the original content.
pub fn verify_localized_change(
    file_path: &Path,
    replaced_content: &[u8],
    target_span: (usize, usize),
) -> Result<bool> {
    let current = io_ext::read(file_path)?;

    // Check bytes before target span
    if target_span.0 > 0 && target_span.0 <= replaced_content.len() {
        let before_replaced = &replaced_content[..target_span.0];
        let before_current = current.get(..target_span.0);

        if before_current != Some(before_replaced) {
            log::warn!("File modified before target span");
            return Ok(false);
        }
    }

    // Check bytes after target span
    let after_start = target_span.1.min(replaced_content.len());
    if after_start < replaced_content.len() {
        let after_replaced = &replaced_content[after_start..];
        let after_current = current.get(after_start..);

        if after_current != Some(after_replaced) {
            log::warn!("File modified after target span");
            return Ok(false);
        }
    }

    Ok(true)
}

/// Run cargo check to validate Rust compilation
///
/// This helper function runs `cargo check --quiet` in the workspace directory
/// to verify that the code compiles after a patch operation.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace directory
///
/// # Returns
/// * `Ok(Output)` - Process output from cargo check
/// * `Err(SpliceError)` - Failed to run cargo check
///
/// # Notes
/// - Uses --quiet flag to suppress build output (only show errors)
/// - Non-Rust projects or missing cargo will return an error
/// - Caller should decide whether to block on failure (we don't block here)
fn run_cargo_check(workspace_dir: &Path) -> Result<std::process::Output> {
    use std::process::Command;

    if !workspace_dir.join("Cargo.toml").exists() {
        return Err(crate::error::SpliceError::IoContext {
            context: format!(
                "Cannot run cargo check outside a Rust package: {}",
                workspace_dir.display()
            ),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "Cargo.toml not found"),
        });
    }

    let output = Command::new("cargo")
        .args(["check", "--quiet", "--color=never"])
        .current_dir(workspace_dir)
        .output()
        .map_err(|e| crate::error::SpliceError::IoContext {
            context: format!("Failed to run cargo check: {}", e),
            source: e,
        })?;

    Ok(output)
}
