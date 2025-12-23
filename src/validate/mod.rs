//! Compiler and AST validation.
//!
//! This module runs cargo check and rust-analyzer to validate
//! that patches produce valid Rust code.

use crate::error::{Result, SpliceError};
use std::path::Path;
use std::process::Command;

/// rust-analyzer execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyzerMode {
    /// rust-analyzer disabled (default).
    Off,

    /// Use rust-analyzer from PATH.
    Path,

    /// Use rust-analyzer from explicit path.
    Explicit(&'static str),
}

/// Validation result from cargo check.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Validation passed (no errors).
    Pass,

    /// Validation failed with compiler errors.
    Fail { errors: Vec<CompilerError> },
}

/// Represents a compiler error or warning.
#[derive(Debug, Clone, PartialEq)]
pub struct CompilerError {
    /// Error level (error, warning, etc.).
    pub level: ErrorLevel,

    /// File path where the error occurred.
    pub file: String,

    /// Line number.
    pub line: usize,

    /// Column number.
    pub column: usize,

    /// Error message.
    pub message: String,
}

/// Error level from compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorLevel {
    Error,
    Warning,
    Note,
    Help,
}

/// Run rust-analyzer validation gate.
///
/// This function invokes rust-analyzer as an external process and treats
/// ANY diagnostic output as a failure. No LSP parsing, no JSON, just
/// simple pass/fail based on diagnostic presence.
///
/// # Arguments
/// * `workspace_dir` - Directory containing Cargo.toml
/// * `mode` - Analyzer execution mode (off/path/explicit)
///
/// # Returns
/// * `Ok(())` - No diagnostics found
/// * `Err(SpliceError::AnalyzerNotAvailable)` - rust-analyzer not found
/// * `Err(SpliceError::AnalyzerFailed)` - Diagnostics detected
pub fn gate_rust_analyzer(workspace_dir: &Path, mode: AnalyzerMode) -> Result<()> {
    // If analyzer is off, skip gate
    if matches!(mode, AnalyzerMode::Off) {
        return Ok(());
    }

    // Determine rust-analyzer binary path
    let analyzer_binary = match mode {
        AnalyzerMode::Path => "rust-analyzer",
        AnalyzerMode::Explicit(path) => path,
        AnalyzerMode::Off => unreachable!(),
    };

    // Invoke rust-analyzer to check for diagnostics
    // We use "analyze" command which outputs diagnostics to stdout
    let output = Command::new(analyzer_binary)
        .args(["check", "--workspace"])
        .current_dir(workspace_dir)
        .output();

    match output {
        Ok(result) => {
            // rust-analyzer exits with 0 even if diagnostics are present
            // We need to check stdout/stderr for any diagnostic output
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);

            // Combine stdout and stderr
            let combined = format!("{}{}", stdout, stderr);

            // If there's ANY output, treat it as a failure
            if !combined.trim().is_empty() {
                return Err(SpliceError::AnalyzerFailed {
                    output: combined,
                });
            }

            Ok(())
        }
        Err(e) => {
            // Failed to invoke rust-analyzer
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(SpliceError::AnalyzerNotAvailable {
                    mode: format!("{:?}", mode),
                });
            }

            // Other I/O error
            Err(SpliceError::Other(format!(
                "Failed to invoke rust-analyzer: {}",
                e
            )))
        }
    }
}

/// Run cargo check and parse the output.
///
/// Returns ValidationResult::Pass if no errors, or Fail with error details.
pub fn validate_with_cargo(project_dir: &Path) -> Result<ValidationResult> {
    let output = Command::new("cargo")
        .args(["check", "--message-format=short"])
        .current_dir(project_dir)
        .output()?;

    if output.status.success() {
        return Ok(ValidationResult::Pass);
    }

    // Parse stderr for error messages
    let stderr = String::from_utf8_lossy(&output.stderr);
    let errors = parse_cargo_output(&stderr);

    Ok(ValidationResult::Fail { errors })
}

/// Parse cargo check output into CompilerError structs.
fn parse_cargo_output(output: &str) -> Vec<CompilerError> {
    let mut errors = Vec::new();

    for line in output.lines() {
        // Try to parse standard cargo error format:
        // error[E123]: message
        //    --> file.rs:line:column
        if let Some(error) = parse_cargo_line(line) {
            errors.push(error);
        }
    }

    errors
}

/// Parse a single line of cargo output.
fn parse_cargo_line(line: &str) -> Option<CompilerError> {
    // TODO: Implement proper cargo output parsing in Task 1
    // For now, return None as placeholder
    None
}
