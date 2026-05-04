//! Compiler and AST validation.
//!
//! This module runs cargo check for project validation and rustc
//! for standalone snippet validation. No LSP integration.

pub mod gates;

use crate::error::{Diagnostic, DiagnosticLevel, Result, SpliceError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

/// rust-analyzer execution mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyzerMode {
    /// rust-analyzer disabled (default).
    Off,

    /// Use rust-analyzer from PATH.
    Path,

    /// Use rust-analyzer from explicit path.
    Explicit(String),
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

/// Result of validating a Rust code snippet with rustc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnippetValidation {
    /// Whether the snippet compiled without errors.
    pub is_valid: bool,
    /// Errors found during validation.
    pub errors: Vec<SnippetDiagnostic>,
    /// Warnings found during validation.
    pub warnings: Vec<SnippetDiagnostic>,
}

/// A diagnostic from rustc snippet validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnippetDiagnostic {
    /// Line number (1-indexed).
    pub line: usize,
    /// Column number (1-indexed).
    pub column: usize,
    /// Error/warning message.
    pub message: String,
    /// Optional error code (e.g., "E0425").
    pub code: Option<String>,
}

/// Validate a Rust code snippet using rustc.
///
/// This is used for the `create` command where the file does not yet
/// exist in a Cargo project. It compiles the snippet as a standalone
/// library crate to catch syntax and basic semantic errors.
pub fn validate_rust_snippet(
    code: &str,
    workspace_dir: Option<&Path>,
) -> Result<SnippetValidation> {
    use std::fs::File;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().map_err(|e| SpliceError::IoContext {
        context: "Failed to create temp directory for validation".to_string(),
        source: e,
    })?;

    let temp_file = temp_dir.path().join("splice_validate.rs");
    let mut file = File::create(&temp_file).map_err(|e| SpliceError::IoContext {
        context: "Failed to create temp file for validation".to_string(),
        source: e,
    })?;

    file.write_all(code.as_bytes())
        .map_err(|e| SpliceError::IoContext {
            context: "Failed to write code to temp file".to_string(),
            source: e,
        })?;

    file.flush().map_err(|e| SpliceError::IoContext {
        context: "Failed to flush temp file".to_string(),
        source: e,
    })?;

    let mut cmd = Command::new("rustc");
    if let Some(ws) = workspace_dir {
        cmd.current_dir(ws);
    }

    cmd.arg("--emit=metadata")
        .arg("--crate-type=lib")
        .arg(temp_file.to_str().unwrap());

    let output = match cmd.output() {
        Ok(output) => output,
        Err(e) => {
            log::debug!("rustc not available for snippet validation: {}", e);
            return Ok(SnippetValidation {
                is_valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
            });
        }
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    let (errors, warnings) = parse_rustc_snippet_output(&stderr);

    Ok(SnippetValidation {
        is_valid: errors.is_empty() && output.status.success() && !stderr.contains("error"),
        errors,
        warnings,
    })
}

fn parse_rustc_snippet_output(stderr: &str) -> (Vec<SnippetDiagnostic>, Vec<SnippetDiagnostic>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for line in stderr.lines() {
        if line.contains("error[") || line.starts_with("error:") {
            if let Some(diag) = parse_snippet_diagnostic(line, ErrorLevel::Error) {
                errors.push(diag);
            }
        } else if line.contains("warning:") {
            if let Some(diag) = parse_snippet_diagnostic(line, ErrorLevel::Warning) {
                warnings.push(diag);
            }
        }
    }

    (errors, warnings)
}

fn parse_snippet_diagnostic(line: &str, _level: ErrorLevel) -> Option<SnippetDiagnostic> {
    let code = if let Some(start) = line.find("error[") {
        let end = line[start..].find(']').unwrap_or(0);
        Some(line[start + 6..start + end].to_string())
    } else {
        None
    };

    let message = if let Some(colon_pos) = line.find("]:") {
        line[colon_pos + 2..].trim().to_string()
    } else if let Some(colon_pos) = line.find("warning:") {
        line[colon_pos + 8..].trim().to_string()
    } else if let Some(colon_pos) = line.find("error:") {
        line[colon_pos + 6..].trim().to_string()
    } else {
        line.to_string()
    };

    Some(SnippetDiagnostic {
        line: 1,
        column: 1,
        message,
        code,
    })
}

/// Run rust-analyzer and return all diagnostics (errors + warnings).
///
/// This is the shared implementation used by both `gate_rust_analyzer`
/// (blocking validation gate) and post-verification reporting.
///
/// # Returns
/// * `Ok(Vec<CompilerError>)` - All diagnostics found (may be empty)
/// * `Err(SpliceError::AnalyzerNotAvailable)` - rust-analyzer binary not found
/// * `Err(SpliceError::Other)` - Failed to run rust-analyzer
pub fn run_rust_analyzer(workspace_dir: &Path, mode: &AnalyzerMode) -> Result<Vec<CompilerError>> {
    if matches!(mode, AnalyzerMode::Off) {
        return Ok(Vec::new());
    }

    let analyzer_binary = match mode {
        AnalyzerMode::Path => "rust-analyzer".to_string(),
        AnalyzerMode::Explicit(path) => path.clone(),
        AnalyzerMode::Off => unreachable!(),
    };

    let output = Command::new(&analyzer_binary)
        .arg("diagnostics")
        .arg(workspace_dir)
        .output();

    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            let combined = format!("{}{}", stdout, stderr);
            let diagnostics = parse_rust_analyzer_diagnostics(&combined);
            Ok(diagnostics)
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(SpliceError::AnalyzerNotAvailable {
                    mode: format!("{:?}", mode),
                })
            } else {
                Err(SpliceError::Other(format!(
                    "Failed to invoke rust-analyzer: {}",
                    e
                )))
            }
        }
    }
}

/// Run rust-analyzer validation gate.
///
/// This function invokes `rust-analyzer diagnostics <workspace>` as an external
/// process, parses the structured diagnostic output, and returns an error only
/// if actual errors (not warnings) are found.
///
/// # Arguments
/// * `workspace_dir` - Directory containing Cargo.toml
/// * `mode` - Analyzer execution mode (off/path/explicit)
///
/// # Returns
/// * `Ok(())` - No errors found (warnings are logged but non-blocking)
/// * `Err(SpliceError::AnalyzerNotAvailable)` - rust-analyzer not found
/// * `Err(SpliceError::AnalyzerFailed)` - Errors detected
pub fn gate_rust_analyzer(workspace_dir: &Path, mode: AnalyzerMode) -> Result<()> {
    let diagnostics = run_rust_analyzer(workspace_dir, &mode)?;
    let analyzer_binary = match &mode {
        AnalyzerMode::Path => "rust-analyzer".to_string(),
        AnalyzerMode::Explicit(path) => path.clone(),
        AnalyzerMode::Off => return Ok(()),
    };
    let analyzer_meta = collect_tool_metadata(&analyzer_binary, &["--version"]);

    let (errors, non_errors): (Vec<_>, Vec<_>) = diagnostics
        .iter()
        .partition(|d| d.level == ErrorLevel::Error);

    for warn in &non_errors {
        log::warn!(
            "rust-analyzer: {} at {}:{}: {}",
            warn.code.as_deref().unwrap_or("warning"),
            warn.file,
            warn.line,
            warn.message
        );
    }

    if !errors.is_empty() {
        let structured_diagnostics = errors
            .into_iter()
            .map(|err| {
                let remediation = err.code.as_deref().and_then(remediation_link_for_code);
                Diagnostic::new(
                    "rust-analyzer",
                    DiagnosticLevel::from(err.level),
                    err.message.clone(),
                )
                .with_file(Path::new(&err.file).to_path_buf())
                .with_position(nonzero(err.line), nonzero(err.column))
                .with_code(err.code.clone())
                .with_tool_metadata(Some(&analyzer_meta))
                .with_remediation(remediation)
            })
            .collect();

        return Err(SpliceError::AnalyzerFailed {
            output: "rust-analyzer detected errors".to_string(),
            diagnostics: structured_diagnostics,
        });
    }

    Ok(())
}

/// Parse rust-analyzer `diagnostics` output into structured errors.
///
/// Expected format:
/// `at crate <crate>, file <path>: <severity> Ra("<code>", <severity>) from LineCol { line: N, col: N } to LineCol { line: N, col: N }: <message>`
///
/// Progress spam (backspaces, percentage, "processing") is filtered out.
fn parse_rust_analyzer_diagnostics(output: &str) -> Vec<CompilerError> {
    use regex::Regex;

    let mut errors = Vec::new();

    let re = match Regex::new(
        r#"at crate [^,]+, file ([^:]+): (\w+) Ra\("([^"]*)", \w+\) from LineCol \{ line: (\d+), col: (\d+) \} to LineCol \{ line: \d+, col: \d+ \}: (.+)"#,
    ) {
        Ok(re) => re,
        Err(_) => return errors,
    };

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Skip progress spam: backspaces, "processing", percentage markers
        if trimmed.contains('\x08') || trimmed.contains("processing") || trimmed.contains('%') {
            continue;
        }

        if let Some(caps) = re.captures(trimmed) {
            let file = caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let severity_str = caps.get(2).map(|m| m.as_str()).unwrap_or("Error");
            let code = caps.get(3).map(|m| m.as_str().to_string());
            let line_num = caps
                .get(4)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(1);
            let column = caps
                .get(5)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let message = caps
                .get(6)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            let level = match severity_str {
                "Error" => ErrorLevel::Error,
                "Warning" => ErrorLevel::Warning,
                "WeakWarning" => ErrorLevel::Warning,
                "Information" => ErrorLevel::Note,
                _ => ErrorLevel::Warning,
            };

            errors.push(CompilerError {
                level,
                file,
                line: line_num,
                column,
                code,
                message,
                note: None,
            });
        }
    }

    errors
}

/// Run cargo check and parse the output.
///
/// Returns ValidationResult::Pass if no errors, or Fail with error details.
pub fn validate_with_cargo(project_dir: &Path) -> Result<ValidationResult> {
    let output = Command::new("cargo")
        .args(["check", "--message-format=short", "--color=never"])
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

/// Parse TypeScript compiler (tsc) output into CompilerError structs.
///
/// TypeScript error format:
///   file.ts(line,col): error TSXXXX: message
///   file.ts(line,col): warning TSXXXX: message
///
/// # Examples
/// ```text
/// test.ts(2,5): error TS1002: Unterminated string literal
/// another.ts(10,12): warning TS2304: Cannot find name 'foo'
/// ```
///
/// This function is public for testing purposes.
pub fn parse_typescript_output(output: &str) -> Vec<CompilerError> {
    use regex::Regex;

    // TypeScript format: file(line,col): error/warning TSXXXX: message
    // Example: test.ts(2,5): error TS1002: Unterminated string
    let re = match Regex::new(r"^(.+?)\((\d+),(\d+)\): (error|warning) (TS\d+): (.+)$") {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };

    let mut errors = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(caps) = re.captures(trimmed) {
            let file = caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let line_num = caps
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(1);
            let column = caps
                .get(3)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let level_str = caps.get(4).map(|m| m.as_str()).unwrap_or("error");
            let code = caps.get(5).map(|m| m.as_str().to_string());
            let message = caps
                .get(6)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            let level = match level_str {
                "error" => ErrorLevel::Error,
                "warning" => ErrorLevel::Warning,
                _ => ErrorLevel::Error,
            };

            errors.push(CompilerError {
                level,
                file,
                line: line_num,
                column,
                code,
                message,
                note: None,
            });
        }
    }

    errors
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
    fn parse_rust_analyzer_weak_warning() {
        let sample = r#"at crate validation_tests, file /home/Projects/splice/tests/validation_tests.rs: WeakWarning Ra("inactive-code", WeakWarning) from LineCol { line: 6, col: 0 } to LineCol { line: 80, col: 1 }: code is inactive due to #[cfg] directives: test is disabled"#;

        let diags = parse_rust_analyzer_diagnostics(sample);
        assert_eq!(diags.len(), 1, "expected one diagnostic");
        let diag = &diags[0];
        assert_eq!(diag.file, "/home/Projects/splice/tests/validation_tests.rs");
        assert_eq!(diag.line, 6);
        assert_eq!(diag.column, 0);
        assert_eq!(diag.level, ErrorLevel::Warning);
        assert_eq!(diag.code.as_deref(), Some("inactive-code"));
        assert!(
            diag.message.contains("inactive due to #[cfg]"),
            "message should mention cfg: {}",
            diag.message
        );
    }

    #[test]
    fn parse_rust_analyzer_error() {
        let sample = r#"at crate mycrate, file /home/Projects/app/src/lib.rs: Error Ra("E0425", Error) from LineCol { line: 10, col: 4 } to LineCol { line: 10, col: 18 }: cannot find function `foo` in this scope"#;

        let diags = parse_rust_analyzer_diagnostics(sample);
        assert_eq!(diags.len(), 1, "expected one diagnostic");
        let diag = &diags[0];
        assert_eq!(diag.file, "/home/Projects/app/src/lib.rs");
        assert_eq!(diag.line, 10);
        assert_eq!(diag.column, 4);
        assert_eq!(diag.level, ErrorLevel::Error);
        assert_eq!(diag.code.as_deref(), Some("E0425"));
        assert!(diag.message.contains("cannot find function"));
    }

    #[test]
    fn parse_rust_analyzer_filters_progress_spam() {
        let sample = "0/172 0% processing /home/Projects/splice/src/lib.rs\x08\x08\x08\x08\nat crate foo, file /home/Projects/app/src/lib.rs: WeakWarning Ra(\"unused\", WeakWarning) from LineCol { line: 1, col: 0 } to LineCol { line: 1, col: 10 }: unused import";

        let diags = parse_rust_analyzer_diagnostics(sample);
        // Should filter the progress line but keep the diagnostic on the next line
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].file, "/home/Projects/app/src/lib.rs");
    }

    #[test]
    fn parse_rust_analyzer_mixed_output() {
        let sample = r#"info: falling back to "/usr/bin/rust-analyzer"
0/172 0% processing /home/Projects/splice/benches/graph_benchmarks.rs
at crate validation_tests, file /home/Projects/splice/tests/validation_tests.rs: WeakWarning Ra("inactive-code", WeakWarning) from LineCol { line: 6, col: 0 } to LineCol { line: 80, col: 1 }: code is inactive due to #[cfg] directives: test is disabled
1/172 0% processing /home/Projects/splice/tests/validation_tests.rs
at crate mycrate, file /home/Projects/app/src/lib.rs: Error Ra("E0425", Error) from LineCol { line: 10, col: 4 } to LineCol { line: 10, col: 18 }: cannot find function `foo` in this scope"#;

        let diags = parse_rust_analyzer_diagnostics(sample);
        assert_eq!(diags.len(), 2);

        let warn = &diags[0];
        assert_eq!(warn.level, ErrorLevel::Warning);
        assert_eq!(warn.code.as_deref(), Some("inactive-code"));

        let err = &diags[1];
        assert_eq!(err.level, ErrorLevel::Error);
        assert_eq!(err.code.as_deref(), Some("E0425"));
    }

    #[test]
    fn parse_rust_analyzer_empty_output() {
        let diags = parse_rust_analyzer_diagnostics("");
        assert!(diags.is_empty());

        let diags = parse_rust_analyzer_diagnostics(
            "info: rust-analyzer started\ninfo: rust-analyzer finished",
        );
        assert!(diags.is_empty());
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
