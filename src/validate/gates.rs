//! Per-language validation gates.
//!
//! Runs each language's native compiler/tool for syntax checking.
//! "Truth lives in execution" — validation is done by real compilers.

use crate::error::{Result, SpliceError};
use crate::ingest::detect::{detect_language, Language};
use std::path::Path;
use std::process::Command;

/// Outcome of validating a file with its language's compiler.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationOutcome {
    /// Whether the file passed validation.
    pub is_valid: bool,

    /// Errors found during validation.
    pub errors: Vec<ValidationError>,

    /// Warnings found during validation.
    pub warnings: Vec<ValidationError>,

    /// Whether the validation tool was available.
    pub tool_available: bool,
}

/// A validation error or warning from a compiler.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// File where the error occurred.
    pub file: String,

    /// Line number (1-based, or 0 if unknown).
    pub line: usize,

    /// Column number (0-based, or 0 if unknown).
    pub column: usize,

    /// Error message.
    pub message: String,
}

/// Validate a file using its language's native compiler.
///
/// Dispatches to the appropriate validation function based on file extension.
/// Returns `tool_available=false` if the compiler is not found (not an error).
///
/// # Examples
///
/// ```no_run
/// # use splice::validate::gates::validate_file;
/// # use std::path::Path;
/// let outcome = validate_file(Path::new("main.rs"))?;
/// if outcome.tool_available {
///     println!("Validation: {}", if outcome.is_valid { "PASS" } else { "FAIL" });
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn validate_file(path: &Path) -> Result<ValidationOutcome> {
    let language = detect_language(path).ok_or_else(|| {
        SpliceError::Other(format!("Cannot detect language for file: {:?}", path))
    })?;

    match language {
        Language::Rust => {
            // Rust validation already exists in validate/mod.rs
            // Return tool_unavailable for now (caller should use cargo check separately)
            Ok(ValidationOutcome {
                is_valid: false,
                errors: vec![],
                warnings: vec![],
                tool_available: false,
            })
        }
        Language::Python => validate_python(path),
        Language::C => validate_c(path),
        Language::Cpp => validate_cpp(path),
        Language::Java => validate_java(path),
        Language::JavaScript => validate_javascript(path),
        Language::TypeScript => validate_typescript(path),
    }
}

/// Validate a Python file using `python -m py_compile`.
fn validate_python(path: &Path) -> Result<ValidationOutcome> {
    let output = Command::new("python")
        .args(["-m", "py_compile", path.to_str().unwrap()])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                return Ok(ValidationOutcome {
                    is_valid: true,
                    errors: vec![],
                    warnings: vec![],
                    tool_available: true,
                });
            }

            // Parse Python error output
            let stderr = String::from_utf8_lossy(&result.stderr);
            let errors = parse_python_errors(&stderr, path);

            Ok(ValidationOutcome {
                is_valid: false,
                errors,
                warnings: vec![],
                tool_available: true,
            })
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(ValidationOutcome {
                    is_valid: false,
                    errors: vec![],
                    warnings: vec![],
                    tool_available: false,
                });
            }
            Err(SpliceError::Other(format!("Failed to run python: {}", e)))
        }
    }
}

/// Validate a C file using `gcc -fsyntax-only`.
fn validate_c(path: &Path) -> Result<ValidationOutcome> {
    let output = Command::new("gcc")
        .args(["-fsyntax-only", "-c", path.to_str().unwrap()])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                return Ok(ValidationOutcome {
                    is_valid: true,
                    errors: vec![],
                    warnings: vec![],
                    tool_available: true,
                });
            }

            // Parse GCC error output
            let stderr = String::from_utf8_lossy(&result.stderr);
            let (errors, warnings) = parse_gcc_output(&stderr);

            Ok(ValidationOutcome {
                is_valid: errors.is_empty(),
                errors,
                warnings,
                tool_available: true,
            })
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(ValidationOutcome {
                    is_valid: false,
                    errors: vec![],
                    warnings: vec![],
                    tool_available: false,
                });
            }
            Err(SpliceError::Other(format!("Failed to run gcc: {}", e)))
        }
    }
}

/// Validate a C++ file using `g++ -fsyntax-only`.
fn validate_cpp(path: &Path) -> Result<ValidationOutcome> {
    let output = Command::new("g++")
        .args(["-fsyntax-only", "-c", path.to_str().unwrap()])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                return Ok(ValidationOutcome {
                    is_valid: true,
                    errors: vec![],
                    warnings: vec![],
                    tool_available: true,
                });
            }

            // Parse g++ error output (same format as gcc)
            let stderr = String::from_utf8_lossy(&result.stderr);
            let (errors, warnings) = parse_gcc_output(&stderr);

            Ok(ValidationOutcome {
                is_valid: errors.is_empty(),
                errors,
                warnings,
                tool_available: true,
            })
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(ValidationOutcome {
                    is_valid: false,
                    errors: vec![],
                    warnings: vec![],
                    tool_available: false,
                });
            }
            Err(SpliceError::Other(format!("Failed to run g++: {}", e)))
        }
    }
}

/// Validate a Java file using `javac`.
fn validate_java(path: &Path) -> Result<ValidationOutcome> {
    let output = Command::new("javac")
        .args([path.to_str().unwrap()])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                return Ok(ValidationOutcome {
                    is_valid: true,
                    errors: vec![],
                    warnings: vec![],
                    tool_available: true,
                });
            }

            // Parse javac error output
            let stderr = String::from_utf8_lossy(&result.stderr);
            let errors = parse_javac_errors(&stderr, path);

            Ok(ValidationOutcome {
                is_valid: false,
                errors,
                warnings: vec![],
                tool_available: true,
            })
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(ValidationOutcome {
                    is_valid: false,
                    errors: vec![],
                    warnings: vec![],
                    tool_available: false,
                });
            }
            Err(SpliceError::Other(format!("Failed to run javac: {}", e)))
        }
    }
}

/// Validate a JavaScript file using `node --check`.
fn validate_javascript(path: &Path) -> Result<ValidationOutcome> {
    // node --check is available in Node 16+
    let output = Command::new("node")
        .args(["--check", path.to_str().unwrap()])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                return Ok(ValidationOutcome {
                    is_valid: true,
                    errors: vec![],
                    warnings: vec![],
                    tool_available: true,
                });
            }

            // Parse node error output
            let stderr = String::from_utf8_lossy(&result.stderr);
            let errors = parse_node_errors(&stderr, path);

            Ok(ValidationOutcome {
                is_valid: false,
                errors,
                warnings: vec![],
                tool_available: true,
            })
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(ValidationOutcome {
                    is_valid: false,
                    errors: vec![],
                    warnings: vec![],
                    tool_available: false,
                });
            }
            // If node exists but --check failed, it might be an older version
            // Return tool_unavailable in that case
            Ok(ValidationOutcome {
                is_valid: false,
                errors: vec![],
                warnings: vec![],
                tool_available: false,
            })
        }
    }
}

/// Validate a TypeScript file using `tsc --noEmit`.
fn validate_typescript(path: &Path) -> Result<ValidationOutcome> {
    // tsc --noEmit validates TypeScript without generating output files
    // We need to run it in the directory containing tsconfig.json (if it exists)
    let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));

    let output = Command::new("tsc")
        .args(["--noEmit", path.to_str().unwrap()])
        .current_dir(parent_dir)
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                return Ok(ValidationOutcome {
                    is_valid: true,
                    errors: vec![],
                    warnings: vec![],
                    tool_available: true,
                });
            }

            // Parse tsc error output
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stdout = String::from_utf8_lossy(&result.stdout);
            let errors = parse_tsc_errors(&stderr, &stdout, path);

            Ok(ValidationOutcome {
                is_valid: errors.is_empty(),
                errors,
                warnings: vec![],
                tool_available: true,
            })
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(ValidationOutcome {
                    is_valid: false,
                    errors: vec![],
                    warnings: vec![],
                    tool_available: false,
                });
            }
            Err(SpliceError::Other(format!("Failed to run tsc: {}", e)))
        }
    }
}

/// Parse Python error output from py_compile.
///
/// Format (multi-line):
///   File "test.py", line 1
///     def foo(
///            ^
///   SyntaxError: '(' was never closed
fn parse_python_errors(output: &str, file: &Path) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let lines: Vec<&str> = output.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // Look for: File "test.py", line 1
        if line.contains("File \"") && line.contains(", line ") {
            if let Some(line_end) = line.rfind(", line ") {
                let after_line = &line[line_end + 7..];
                let line_str = after_line.trim().trim_end_matches('"');
                if let Ok(line_num) = line_str.parse::<usize>() {
                    // Look ahead for SyntaxError on following lines
                    let mut message = String::new();
                    for line in lines.iter().take(lines.len().min(i + 5)).skip(i + 1) {
                        if line.contains("SyntaxError:") {
                            if let Some(msg_start) = line.find("SyntaxError: ") {
                                message = line[msg_start + 12..].trim().to_string();
                            }
                            break;
                        }
                    }

                    if !message.is_empty() {
                        errors.push(ValidationError {
                            file: file.display().to_string(),
                            line: line_num,
                            column: 0,
                            message,
                        });
                    }
                }
            }
        }

        // Also handle single-line format: "SyntaxError: <msg>"
        if line.contains("SyntaxError:") && errors.is_empty() {
            if let Some(msg_start) = line.find("SyntaxError: ") {
                let message = line[msg_start + 12..].trim().to_string();
                errors.push(ValidationError {
                    file: file.display().to_string(),
                    line: 0,
                    column: 0,
                    message,
                });
            }
        }

        i += 1;
    }

    if errors.is_empty() && !output.trim().is_empty() {
        // Fallback: include entire output as message
        errors.push(ValidationError {
            file: file.display().to_string(),
            line: 0,
            column: 0,
            message: output.trim().to_string(),
        });
    }

    errors
}

/// Parse GCC/g++ error output.
///
/// Format: `<file>:<line>:<col>: error: <msg>` or `warning: <msg>`
fn parse_gcc_output(output: &str) -> (Vec<ValidationError>, Vec<ValidationError>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for line in output.lines() {
        // Parse: "file:line:col: error: message"
        if line.contains(": error: ") {
            if let Some(error) = parse_gcc_line(line) {
                errors.push(error);
            }
        }
        // Parse: "file:line:col: warning: message"
        else if line.contains(": warning: ") {
            if let Some(warning) = parse_gcc_line(line) {
                warnings.push(warning);
            }
        }
    }

    (errors, warnings)
}

/// Parse a single GCC error/warning line.
fn parse_gcc_line(line: &str) -> Option<ValidationError> {
    // Format: "file:line:col: error: message" or "file:line:col: warning: message"
    let parts: Vec<&str> = line.splitn(4, ':').collect();
    if parts.len() >= 4 {
        let file = parts[0].trim();
        let line_num = parts[1].trim().parse::<usize>().ok()?;
        let column = parts[2].trim().parse::<usize>().ok()?;
        let rest = parts[3..].join(":");
        let message = rest.trim().to_string();

        return Some(ValidationError {
            file: file.to_string(),
            line: line_num,
            column,
            message,
        });
    }
    None
}

/// Parse javac error output.
///
/// Format: `<file>:<line>: error: <msg>`
fn parse_javac_errors(output: &str, file: &Path) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for line in output.lines() {
        if line.contains(": error: ") {
            // Simplified parsing for javac output
            if let Some(colon_idx) = line.find(':') {
                let after_file = &line[colon_idx + 1..];
                if let Some(second_colon) = after_file.find(':') {
                    let line_str = &after_file[..second_colon];
                    if let Ok(line_num) = line_str.trim().parse::<usize>() {
                        let after_line = &after_file[second_colon + 1..];
                        if let Some(error_idx) = after_line.find("error: ") {
                            let message = after_line[error_idx + 7..].trim().to_string();
                            errors.push(ValidationError {
                                file: file.display().to_string(),
                                line: line_num,
                                column: 0,
                                message,
                            });
                        }
                    }
                }
            }
        }
    }

    if errors.is_empty() && !output.is_empty() {
        errors.push(ValidationError {
            file: file.display().to_string(),
            line: 0,
            column: 0,
            message: output.trim().to_string(),
        });
    }

    errors
}

/// Parse node --check error output.
///
/// Format: `<file>:<line> (<col>) <msg>` or `<file>:<line> <msg>`
fn parse_node_errors(output: &str, file: &Path) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for line in output.lines() {
        if line.contains(file.to_str().unwrap_or("")) {
            // Parse: "file:line (col) message" or "file:line message"
            if let Some(first_colon) = line.find(':') {
                let after_file = &line[first_colon + 1..];
                if let Some(space_idx) = after_file.find(' ') {
                    let line_str = &after_file[..space_idx];
                    if let Ok(line_num) = line_str.parse::<usize>() {
                        let after_line = &after_file[space_idx + 1..];
                        // Check for "(col)" format
                        let (column, message) = if after_line.starts_with('(') {
                            if let Some(close_paren) = after_line.find(')') {
                                let col_str = &after_line[1..close_paren];
                                let col = col_str.parse::<usize>().unwrap_or(0);
                                let msg = &after_line[close_paren + 1..];
                                (col, msg.trim())
                            } else {
                                (0, after_line.trim())
                            }
                        } else {
                            (0, after_line.trim())
                        };

                        errors.push(ValidationError {
                            file: file.display().to_string(),
                            line: line_num,
                            column,
                            message: message.to_string(),
                        });
                    }
                }
            }
        }
    }

    if errors.is_empty() && !output.is_empty() {
        errors.push(ValidationError {
            file: file.display().to_string(),
            line: 0,
            column: 0,
            message: output.trim().to_string(),
        });
    }

    errors
}

/// Parse tsc (TypeScript Compiler) error output.
///
/// Format: `<file>(<line>,<col>): error TS<code>: <msg>`
/// Or: `<file>(<line>,<col>): <msg>`
fn parse_tsc_errors(stderr: &str, stdout: &str, file: &Path) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Combine stderr and stdout (tsc outputs to both)
    let combined = format!("{}\n{}", stderr, stdout);

    for line in combined.lines() {
        // Parse: "file.ts(line,col): error TS<code>: message"
        // or "file.ts(line,col): message"
        if line.contains(file.to_str().unwrap_or("")) && (line.contains(": error ") || line.contains("TS")) {
            // Try to extract line and column
            if let Some(open_paren) = line.find('(') {
                let after_paren = &line[open_paren + 1..];
                if let Some(comma) = after_paren.find(',') {
                    let line_str = &after_paren[..comma];
                    if let Ok(line_num) = line_str.trim().parse::<usize>() {
                        let after_comma = &after_paren[comma + 1..];
                        if let Some(close_paren) = after_comma.find(')') {
                            let col_str = &after_comma[..close_paren];
                            if let Ok(column) = col_str.trim().parse::<usize>() {
                                // Extract message after ") error " or ") TS<number>: "
                                let after_close = &line[open_paren + close_paren + 2..];
                                let message = if let Some(error_idx) = after_close.find("error ") {
                                    after_close[error_idx + 6..].trim().to_string()
                                } else if let Some(ts_idx) = after_close.find("TS") {
                                    // TS<code>: format
                                    if let Some(colon_after_ts) = after_close[ts_idx..].find(':') {
                                        after_close[ts_idx + colon_after_ts + 1..].trim().to_string()
                                    } else {
                                        after_close.trim().to_string()
                                    }
                                } else {
                                    after_close.trim().to_string()
                                };

                                if !message.is_empty() {
                                    errors.push(ValidationError {
                                        file: file.display().to_string(),
                                        line: line_num,
                                        column,
                                        message,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if errors.is_empty() && !combined.trim().is_empty() {
        // Fallback: include entire output as message
        errors.push(ValidationError {
            file: file.display().to_string(),
            line: 0,
            column: 0,
            message: combined.trim().to_string(),
        });
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_python_syntax_error() {
        let output = "  File \"test.py\", line 1\n    SyntaxError: invalid syntax\n";
        let path = Path::new("test.py");
        let errors = parse_python_errors(output, path);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 1);
        assert_eq!(errors[0].message, "invalid syntax");
    }

    #[test]
    fn test_parse_gcc_error() {
        let output = "test.c:3:5: error: expected ';' before '}'\n";
        let (errors, _warnings) = parse_gcc_output(output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 3);
        assert_eq!(errors[0].column, 5);
        assert!(errors[0].message.contains("expected ';'"));
    }

    #[test]
    fn test_parse_gcc_warning() {
        let output = "test.c:5:10: warning: unused variable 'x'\n";
        let (_errors, warnings) = parse_gcc_output(output);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].line, 5);
        assert_eq!(warnings[0].column, 10);
    }

    #[test]
    fn test_parse_javac_error() {
        let output = "Main.java:3: error: class, interface, or enum expected\n";
        let path = Path::new("Main.java");
        let errors = parse_javac_errors(output, path);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 3);
        assert!(errors[0]
            .message
            .contains("class, interface, or enum expected"));
    }

    #[test]
    fn test_parse_node_error() {
        let output = "test.js:2 (5) SyntaxError: Unexpected token\n";
        let path = Path::new("test.js");
        let errors = parse_node_errors(output, path);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 2);
        assert_eq!(errors[0].column, 5);
    }

    #[test]
    fn test_validation_outcome_has_tool_available() {
        // Test that ValidationOutcome has tool_available field
        let outcome = ValidationOutcome {
            is_valid: false,
            errors: vec![],
            warnings: vec![],
            tool_available: false,
        };
        assert!(!outcome.tool_available);
    }

    #[test]
    fn test_parse_tsc_error() {
        let output = "test.ts(2,5): error TS1002: Unterminated string literal\n";
        let path = Path::new("test.ts");
        let errors = parse_tsc_errors(output, "", path);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 2);
        assert_eq!(errors[0].column, 5);
        assert!(errors[0].message.contains("Unterminated string literal"));
    }

    #[test]
    fn test_parse_tsc_error_with_stderr() {
        let stderr = "";
        let stdout = "test.ts(1,1): error TS2304: Cannot find name 'foo'\n";
        let path = Path::new("test.ts");
        let errors = parse_tsc_errors(stderr, stdout, path);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 1);
        assert_eq!(errors[0].column, 1);
        assert!(errors[0].message.contains("Cannot find name"));
    }
}
