//! Command-line interface for Splice.
//!
//! This module handles argument parsing and user interface only.
//! NO logic or database operations are performed here.

use clap::Parser;
use serde::Serialize;
use serde_json::Value;

/// Splice: Span-safe refactoring kernel for Rust.
#[derive(Parser, Debug)]
#[command(name = "splice")]
#[command(author, version, about, long_about = None)]
#[command(subcommand_required = true)]
pub struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging.
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

/// Available Splice commands.
#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Delete a symbol by removing its definition.
    Delete {
        /// Path to the source file containing the symbol.
        #[arg(short, long)]
        file: std::path::PathBuf,

        /// Symbol name to delete.
        #[arg(short, long)]
        symbol: String,

        /// Optional symbol kind filter.
        #[arg(short, long)]
        kind: Option<SymbolKind>,

        /// Optional validation mode (off, os, path).
        #[arg(long, value_name = "MODE")]
        analyzer: Option<AnalyzerMode>,

        /// Optional language (auto-detect from extension by default).
        #[arg(long, value_name = "LANG")]
        language: Option<Language>,

        /// Create a backup before deleting.
        #[arg(long)]
        create_backup: bool,

        /// Optional operation ID for auditing (auto-generated UUID if not provided).
        #[arg(long)]
        operation_id: Option<String>,

        /// Optional JSON metadata to attach to this operation.
        #[arg(long)]
        metadata: Option<String>,
    },

    /// Apply a patch to a symbol's span.
    Patch {
        /// Path to the source file containing the symbol.
        #[arg(short = 'f', long, required_unless_present = "batch")]
        file: Option<std::path::PathBuf>,

        /// Symbol name to patch.
        #[arg(short = 's', long, required_unless_present = "batch")]
        symbol: Option<String>,

        /// Optional symbol kind filter.
        #[arg(short, long, conflicts_with = "batch")]
        kind: Option<SymbolKind>,

        /// Optional validation mode (off, os, path).
        #[arg(long, value_name = "MODE")]
        analyzer: Option<AnalyzerMode>,

        /// Path to file containing replacement content.
        #[arg(
            short = 'w',
            long = "with",
            value_name = "FILE",
            required_unless_present = "batch"
        )]
        with_: Option<std::path::PathBuf>,

        /// Optional language (auto-detect from extension by default).
        #[arg(long, value_name = "LANG")]
        language: Option<Language>,

        /// JSON file describing batch replacements.
        #[arg(long, value_name = "FILE")]
        batch: Option<std::path::PathBuf>,

        /// Run in preview mode without mutating the workspace.
        #[arg(long, conflicts_with = "batch")]
        preview: bool,

        /// Create a backup before patching.
        #[arg(long)]
        create_backup: bool,

        /// Optional operation ID for auditing (auto-generated UUID if not provided).
        #[arg(long)]
        operation_id: Option<String>,

        /// Optional JSON metadata to attach to this operation.
        #[arg(long)]
        metadata: Option<String>,
    },

    /// Execute a multi-step refactoring plan.
    Plan {
        /// Path to the plan.json file.
        #[arg(short, long)]
        file: std::path::PathBuf,
    },

    /// Undo a previous operation by restoring from a backup manifest.
    Undo {
        /// Path to the backup manifest file.
        #[arg(short, long)]
        manifest: std::path::PathBuf,
    },

    /// Apply a pattern replacement to multiple files.
    ApplyFiles {
        /// Glob pattern for matching files (e.g., "tests/**/*.rs" or "src/**/*.py").
        #[arg(short, long)]
        glob: String,

        /// Text pattern to find.
        #[arg(short, long)]
        find: String,

        /// Replacement text.
        #[arg(short, long)]
        replace: String,

        /// Optional language (auto-detect from extension by default).
        #[arg(long, value_name = "LANG")]
        language: Option<Language>,

        /// Skip validation gates (default: false).
        #[arg(long)]
        no_validate: bool,

        /// Create a backup before applying.
        #[arg(long)]
        create_backup: bool,

        /// Optional operation ID for auditing (auto-generated UUID if not provided).
        #[arg(long)]
        operation_id: Option<String>,

        /// Optional JSON metadata to attach to this operation.
        #[arg(long)]
        metadata: Option<String>,
    },
}

/// Symbol kind for filtering.
///
/// These are common symbol types across languages. Not all types are
/// available in all languages - the CLI will validate based on the
/// detected or specified language.
#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum SymbolKind {
    /// Function symbol.
    Function,
    /// Method symbol (function inside a class/struct).
    Method,
    /// Class/Struct symbol.
    Class,
    /// Struct symbol (Rust, C++).
    Struct,
    /// Interface symbol (Java, TypeScript).
    Interface,
    /// Enum symbol.
    Enum,
    /// Trait symbol (Rust).
    Trait,
    /// Impl block (Rust).
    Impl,
    /// Module/Namespace symbol.
    Module,
    /// Variable/Field symbol.
    Variable,
    /// Constructor symbol (Java, C++).
    Constructor,
    /// Type alias (TypeScript, Rust, Python).
    TypeAlias,
}

/// Programming language.
#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum Language {
    /// Rust (.rs)
    Rust,
    /// Python (.py)
    Python,
    /// C (.c, .h)
    C,
    /// C++ (.cpp, .hpp, .cc, .cxx)
    Cpp,
    /// Java (.java)
    Java,
    /// JavaScript (.js, .mjs, .cjs)
    JavaScript,
    /// TypeScript (.ts, .tsx)
    TypeScript,
}

impl Language {
    /// Convert to string identifier.
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Java => "java",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
        }
    }

    /// Convert to symbol module Language.
    pub fn to_symbol_language(self) -> crate::symbol::Language {
        match self {
            Language::Rust => crate::symbol::Language::Rust,
            Language::Python => crate::symbol::Language::Python,
            Language::C => crate::symbol::Language::C,
            Language::Cpp => crate::symbol::Language::Cpp,
            Language::Java => crate::symbol::Language::Java,
            Language::JavaScript => crate::symbol::Language::JavaScript,
            Language::TypeScript => crate::symbol::Language::TypeScript,
        }
    }
}

/// Validation mode.
#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum AnalyzerMode {
    /// Disable validation (default).
    Off,

    /// Use analyzer from PATH.
    Os,

    /// Use analyzer from explicit path.
    Path,
}

/// Parse command-line arguments.
///
/// This function is the entry point for CLI argument parsing.
/// It returns the parsed Cli struct or exits on error.
pub fn parse_args() -> Cli {
    Cli::parse()
}

/// JSON success payload for CLI responses.
#[derive(Serialize)]
pub struct CliSuccessPayload {
    /// Status indicator ("ok").
    pub status: &'static str,
    /// Human-readable message.
    pub message: String,
    /// Optional structured data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl CliSuccessPayload {
    /// Construct a payload containing only the message.
    pub fn message_only(message: String) -> Self {
        Self {
            status: "ok",
            message,
            data: None,
        }
    }

    /// Construct a payload with structured data.
    pub fn with_data(message: String, data: Value) -> Self {
        Self {
            status: "ok",
            message,
            data: Some(data),
        }
    }
}

/// JSON error payload for CLI responses.
#[derive(Serialize)]
pub struct CliErrorPayload {
    /// Status indicator ("error").
    pub status: &'static str,
    /// Structured error details.
    pub error: ErrorDetails,
}

/// Details for a CLI error payload.
#[derive(Serialize)]
pub struct ErrorDetails {
    /// Error kind identifier (SymbolNotFound, etc.).
    pub kind: &'static str,
    /// Human-readable message.
    pub message: String,
    /// Optional symbol context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Optional file context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Optional hint for remediation steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Optional diagnostics emitted by validation gates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<DiagnosticPayload>>,
}

impl CliErrorPayload {
    /// Build payload from a SpliceError instance.
    pub fn from_error(error: &crate::SpliceError) -> Self {
        let symbol = error.symbol().map(|s| s.to_string());
        let file = error
            .file_path()
            .map(|path| path.to_string_lossy().to_string());
        let hint = error.hint().map(|h| h.to_string());
        let diagnostics = {
            let diagnostics = error.diagnostics();
            if diagnostics.is_empty() {
                None
            } else {
                Some(
                    diagnostics
                        .into_iter()
                        .map(DiagnosticPayload::from)
                        .collect(),
                )
            }
        };

        CliErrorPayload {
            status: "error",
            error: ErrorDetails {
                kind: error.kind(),
                message: error.to_string(),
                symbol,
                file,
                hint,
                diagnostics,
            },
        }
    }
}

/// JSON representation of a diagnostic.
#[derive(Serialize)]
pub struct DiagnosticPayload {
    /// Tool emitting the diagnostic.
    pub tool: String,
    /// Severity level ("error", "warning").
    pub level: String,
    /// Diagnostic message.
    pub message: String,
    /// Optional file path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Optional line (1-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Optional column (0-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    /// Optional compiler/analyzer error code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Optional hint/help text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Optional absolute path to the tool binary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_path: Option<String>,
    /// Optional tool version string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_version: Option<String>,
    /// Optional remediation link or text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

impl From<crate::error::Diagnostic> for DiagnosticPayload {
    fn from(diag: crate::error::Diagnostic) -> Self {
        DiagnosticPayload {
            tool: diag.tool,
            level: diag.level.as_str().to_string(),
            message: diag.message,
            file: diag.file.map(|p| p.to_string_lossy().to_string()),
            line: diag.line,
            column: diag.column,
            code: diag.code,
            note: diag.note,
            tool_path: diag.tool_path.map(|p| p.to_string_lossy().to_string()),
            tool_version: diag.tool_version,
            remediation: diag.remediation,
        }
    }
}
