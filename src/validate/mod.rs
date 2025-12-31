//! Compiler and AST validation.
//!
//! This module runs cargo check and rust-analyzer to validate
//! that patches produce valid Rust code.

pub mod gates;

use crate::error::{Diagnostic, DiagnosticLevel, Result, SpliceError};
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

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
    Fail {
        /// List of compiler errors found.
        errors: Vec<CompilerError>,
    },
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

    /// Optional compiler/analyzer error code.
    pub code: Option<String>,

    /// Optional help/note text associated with this diagnostic.
    pub note: Option<String>,
}

/// Error level from compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorLevel {
    /// Error-level diagnostic.
    Error,
    /// Warning-level diagnostic.
    Warning,
    /// Note-level diagnostic.
    Note,
    /// Help-level diagnostic.
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
    let analyzer_meta = collect_tool_metadata(analyzer_binary, &["--version"]);

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
                let compiler_errors = parse_rust_analyzer_output(&combined);

                let diagnostics = if compiler_errors.is_empty() {
                    vec![
                        Diagnostic::new("rust-analyzer", DiagnosticLevel::Error, combined.clone())
                            .with_file(workspace_dir.to_path_buf())
                            .with_tool_metadata(Some(&analyzer_meta)),
                    ]
                } else {
                    compiler_errors
                        .into_iter()
                        .map(|err| {
                            let remediation =
                                err.code.as_deref().and_then(remediation_link_for_code);
                            Diagnostic::new(
                                "rust-analyzer",
                                DiagnosticLevel::from(err.level),
                                err.message,
                            )
                            .with_file(Path::new(&err.file).to_path_buf())
                            .with_position(nonzero(err.line), nonzero(err.column))
                            .with_code(err.code.clone())
                            .with_note(err.note.clone())
                            .with_tool_metadata(Some(&analyzer_meta))
                            .with_remediation(remediation)
                        })
                        .collect()
                };

                return Err(SpliceError::AnalyzerFailed {
                    output: combined,
                    diagnostics,
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
///
/// This function is public for testing purposes.
pub fn parse_cargo_output(output: &str) -> Vec<CompilerError> {
    parse_rust_style_output(output)
}

/// Parse rust-analyzer CLI output into CompilerError structs.
pub fn parse_rust_analyzer_output(output: &str) -> Vec<CompilerError> {
    parse_rust_style_output(output)
}

fn parse_rust_style_output(output: &str) -> Vec<CompilerError> {
    let mut errors = Vec::new();
    let mut pending_error: Option<PendingDiagnostic> = None;
    let mut last_index: Option<usize> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(diag) = parse_error_header(trimmed) {
            pending_error = Some(diag);
            continue;
        }

        if let Some((file, line_num, column)) = parse_location_line(trimmed) {
            if let Some(pending) = pending_error.take() {
                errors.push(CompilerError {
                    level: pending.level,
                    file,
                    line: line_num,
                    column,
                    message: pending.message,
                    code: pending.code,
                    note: None,
                });
                last_index = Some(errors.len() - 1);
            }
            continue;
        }

        if let Some(note) = parse_note_line(trimmed) {
            if let Some(idx) = last_index {
                if let Some(entry) = errors.get_mut(idx) {
                    entry.note = Some(match &entry.note {
                        Some(existing) => format!("{}\n{}", existing, note),
                        None => note,
                    });
                }
            }
            continue;
        }

        if let Some(help) = parse_help_line(trimmed) {
            if let Some(idx) = last_index {
                if let Some(entry) = errors.get_mut(idx) {
                    entry.note = Some(match &entry.note {
                        Some(existing) => format!("{}\n{}", existing, help),
                        None => help,
                    });
                }
            }
            continue;
        }
    }

    errors
}

/// Parse an error/warning header line.
fn parse_error_header(line: &str) -> Option<PendingDiagnostic> {
    if let Some(rest) = line.strip_prefix("error[") {
        if let Some(idx) = rest.find("]:") {
            let code = rest[..idx].to_string();
            let message = rest[idx + 2..].trim().to_string();
            return Some(PendingDiagnostic {
                level: ErrorLevel::Error,
                message,
                code: Some(code),
            });
        }
    } else if let Some(rest) = line.strip_prefix("error:") {
        return Some(PendingDiagnostic {
            level: ErrorLevel::Error,
            message: rest.trim().to_string(),
            code: None,
        });
    } else if let Some(rest) = line.strip_prefix("warning[") {
        if let Some(idx) = rest.find("]:") {
            let code = rest[..idx].to_string();
            let message = rest[idx + 2..].trim().to_string();
            return Some(PendingDiagnostic {
                level: ErrorLevel::Warning,
                message,
                code: Some(code),
            });
        }
    } else if let Some(rest) = line.strip_prefix("warning:") {
        return Some(PendingDiagnostic {
            level: ErrorLevel::Warning,
            message: rest.trim().to_string(),
            code: None,
        });
    }

    None
}

#[derive(Debug, Clone)]
struct PendingDiagnostic {
    level: ErrorLevel,
    message: String,
    code: Option<String>,
}

fn parse_note_line(line: &str) -> Option<String> {
    parse_labelled_line(line, "note")
}

fn parse_help_line(line: &str) -> Option<String> {
    parse_labelled_line(line, "help")
}

fn parse_labelled_line(line: &str, label: &str) -> Option<String> {
    let trimmed = line.trim_start_matches('|').trim();

    if let Some(rest) = trimmed.strip_prefix(label) {
        return Some(rest.trim_start_matches(':').trim().to_string());
    }

    if let Some(rest) = trimmed.strip_prefix(&format!("= {}", label)) {
        return Some(rest.trim_start_matches(':').trim().to_string());
    }

    None
}

/// Parse a location line like "   --> file.rs:line:column"
/// Returns (file, line, column) if successful.
fn parse_location_line(line: &str) -> Option<(String, usize, usize)> {
    let line = line.trim();

    // Match "   --> file:line:column" or "  --> file:line:column"
    if let Some(rest) = line.strip_prefix("-->") {
        let rest = rest.trim();
        // Parse "file:line:column"
        if let Some(colon_idx) = rest.rfind(':') {
            let column_str = &rest[colon_idx + 1..];
            let column = column_str.parse::<usize>().ok()?;

            let before_column = &rest[..colon_idx];
            if let Some(line_colon_idx) = before_column.rfind(':') {
                let line_str = &before_column[line_colon_idx + 1..];
                let line_num = line_str.parse::<usize>().ok()?;
                let file = before_column[..line_colon_idx].to_string();
                return Some((file, line_num, column));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rust_analyzer_output_extracts_file_line() {
        let sample = r#"
error[E0425]: cannot find function `missing_helper` in this scope
 --> src/lib.rs:2:5
  |
2 |     missing_helper(name)
  |     ^^^^^^^^^^^^^^ not found in this scope
help: consider importing `missing_helper`
"#;

        let errors = parse_rust_analyzer_output(sample);
        assert_eq!(errors.len(), 1, "expected one diagnostic");
        let diag = &errors[0];
        assert_eq!(diag.file, "src/lib.rs");
        assert_eq!(diag.line, 2);
        assert_eq!(diag.column, 5);
        assert!(
            diag.message.contains("missing_helper"),
            "diagnostic message should mention missing helper"
        );
        assert_eq!(diag.code.as_deref(), Some("E0425"));
        assert!(
            diag.note
                .as_deref()
                .map(|n| n.contains("consider importing"))
                .unwrap_or(false),
            "diagnostic note should capture help text"
        );
    }
}

fn nonzero(value: usize) -> Option<usize> {
    if value == 0 {
        None
    } else {
        Some(value)
    }
}

/// Metadata about an external validation tool.
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    /// Absolute path to the binary, if resolvable.
    pub path: Option<PathBuf>,
    /// Version string returned by the tool.
    pub version: Option<String>,
}

/// Resolve tool metadata (path + version) for diagnostics.
pub fn collect_tool_metadata(binary: &str, version_args: &[&str]) -> ToolMetadata {
    let path = which(binary).ok();
    let version = Command::new(binary)
        .args(version_args)
        .output()
        .ok()
        .and_then(|output| {
            let selected = if output.stdout.is_empty() {
                &output.stderr
            } else {
                &output.stdout
            };
            let text = String::from_utf8_lossy(selected).trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        });

    ToolMetadata { path, version }
}

/// Best-effort remediation link for a compiler/analyzer error code.
pub fn remediation_link_for_code(code: &str) -> Option<String> {
    if code.starts_with('E') && code.len() == 5 && code[1..].chars().all(|ch| ch.is_ascii_digit()) {
        Some(format!(
            "https://doc.rust-lang.org/error-index.html#{}",
            code
        ))
    } else if code.starts_with("TS")
        && code.len() > 2
        && code[2..].chars().all(|ch| ch.is_ascii_digit())
    {
        Some(format!("https://www.typescriptlang.org/errors/{}", code))
    } else {
        None
    }
}
