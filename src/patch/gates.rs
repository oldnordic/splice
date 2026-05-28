use crate::error::{Diagnostic, DiagnosticLevel, Result, SpliceError};
use crate::io_ext;
use crate::symbol::Language as SymbolLanguage;
use crate::validate::{self, AnalyzerMode};
use std::path::{Path, PathBuf};

/// Run all validation gates in sequence.
///
/// Gates are executed in order:
/// 1. Tree-sitter reparse (syntax validation, language-specific)
/// 2. Compiler validation (language-specific)
/// 3. rust-analyzer (optional, Rust only)
///
/// If any gate fails, returns error immediately.
pub(crate) fn run_validation_gates(
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
pub(crate) fn gate_tree_sitter_reparse(file_path: &Path, language: SymbolLanguage) -> Result<()> {
    let source = io_ext::read(file_path)?;

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
        SymbolLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
        SymbolLanguage::Python => tree_sitter_python::LANGUAGE.into(),
        SymbolLanguage::C => tree_sitter_c::LANGUAGE.into(),
        SymbolLanguage::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        SymbolLanguage::Java => tree_sitter_java::LANGUAGE.into(),
        SymbolLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        SymbolLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
    }
}

/// Compiler validation gate (language-specific).
///
/// Validates that the patched file compiles using the appropriate
/// compiler for each language (via validate::gates::validate_file).
pub(crate) fn gate_compiler_validation(
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
/// Uses a 60-second timeout to prevent hanging on large projects.
pub(crate) fn gate_cargo_check(workspace_dir: &Path) -> Result<()> {
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    // Spawn cargo check in a separate thread to allow timeout
    let workspace_path = workspace_dir.to_path_buf();
    let thread_workspace = workspace_path.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    thread::spawn(move || {
        let output = Command::new("cargo")
            .args(["check", "--color=never"])
            .current_dir(&thread_workspace)
            .output();
        let _ = tx.send(output);
    });

    // Wait for completion with 120-second timeout (cargo check can take 50+ seconds on large projects)
    let output = match rx.recv_timeout(Duration::from_secs(120)) {
        Ok(result) => result.map_err(|source| SpliceError::Io {
            path: workspace_path.clone(),
            source,
        })?,
        Err(_) => {
            return Err(SpliceError::Other(
                "cargo check timed out after 120 seconds".to_string(),
            ));
        }
    };

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
