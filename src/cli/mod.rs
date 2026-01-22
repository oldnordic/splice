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

    /// Output structured JSON (default: human-readable).
    #[arg(long, global = true)]
    json: bool,

    /// Enable strict pre-verification (warnings become errors).
    #[arg(long, global = true)]
    pub strict: bool,

    /// Skip pre-verification checks (dangerous!).
    #[arg(long, global = true, hide = true)]
    pub skip_pre_verify: bool,
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

        /// Number of context lines after the match.
        #[arg(short = 'A', long, value_name = "N", default_value = "0")]
        context_after: usize,

        /// Number of context lines before the match.
        #[arg(short = 'B', long, value_name = "N", default_value = "0")]
        context_before: usize,

        /// Number of context lines before and after the match (default: 3).
        #[arg(short = 'C', long, value_name = "N", default_value = "3")]
        context: usize,

        /// Create a backup before deleting.
        #[arg(long)]
        create_backup: bool,

        /// Include relationship information in output.
        #[arg(long)]
        relationships: bool,

        /// Preview deletion without applying changes.
        #[arg(short = 'n', long = "dry-run")]
        dry_run: bool,

        /// Number of context lines in unified diff (default: 3).
        #[arg(short = 'U', long, value_name = "N", default_value = "3")]
        unified: usize,

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

        /// Number of context lines after the match.
        #[arg(short = 'A', long, value_name = "N", default_value = "0")]
        context_after: usize,

        /// Number of context lines before the match.
        #[arg(short = 'B', long, value_name = "N", default_value = "0")]
        context_before: usize,

        /// Number of context lines before and after the match (default: 3).
        #[arg(short = 'C', long, value_name = "N", default_value = "3")]
        context_both: usize,

        /// Preview changes without applying (alias: --dry-run, -n).
        #[arg(short = 'n', long = "dry-run", alias = "preview", conflicts_with = "batch")]
        preview: bool,

        /// Number of context lines in unified diff (default: 3).
        #[arg(short = 'U', long, value_name = "N", default_value = "3")]
        unified: usize,

        /// Create a backup before patching.
        #[arg(long)]
        create_backup: bool,

        /// Include relationship information in output.
        #[arg(long)]
        relationships: bool,

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

        /// Optional operation ID for auditing (auto-generated UUID if not provided).
        #[arg(long)]
        operation_id: Option<String>,

        /// Optional JSON metadata to attach to this operation.
        #[arg(long)]
        metadata: Option<String>,
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

        /// Number of context lines before/after spans (default: 3).
        #[arg(long, value_name = "N", default_value = "3")]
        context_lines: usize,

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

    /// Query symbols by labels (uses Magellan integration).
    Query {
        /// Path to the Magellan database.
        #[arg(short, long)]
        db: std::path::PathBuf,

        /// Labels to query (can be specified multiple times).
        /// Examples: rust, python, fn, struct, class, method, etc.
        #[arg(short, long)]
        label: Vec<String>,

        /// Number of context lines after the match.
        #[arg(short = 'A', long, value_name = "N", default_value = "0")]
        context_after: usize,

        /// Number of context lines before the match.
        #[arg(short = 'B', long, value_name = "N", default_value = "0")]
        context_before: usize,

        /// Number of context lines before and after the match (default: 3).
        #[arg(short = 'C', long, value_name = "N", default_value = "3")]
        context_both: usize,

        /// List all available labels.
        #[arg(long)]
        list: bool,

        /// Count entities with specified label(s).
        #[arg(long)]
        count: bool,

        /// Show source code for each result.
        #[arg(long)]
        show_code: bool,

        /// Include relationship information in output.
        #[arg(long)]
        relationships: bool,
    },

    /// Get code chunks from the database (uses Magellan integration).
    Get {
        /// Path to the Magellan database.
        #[arg(short, long)]
        db: std::path::PathBuf,

        /// Path to the source file.
        #[arg(short, long)]
        file: std::path::PathBuf,

        /// Start byte offset.
        #[arg(long)]
        start: usize,

        /// End byte offset.
        #[arg(long)]
        end: usize,

        /// Number of context lines after the match.
        #[arg(short = 'A', long, value_name = "N", default_value = "0")]
        context_after: usize,

        /// Number of context lines before the match.
        #[arg(short = 'B', long, value_name = "N", default_value = "0")]
        context_before: usize,

        /// Number of context lines before and after the match (default: 3).
        #[arg(short = 'C', long, value_name = "N", default_value = "3")]
        context_both: usize,

        /// Include relationship information in output.
        #[arg(long)]
        relationships: bool,
    },

    /// Query execution log.
    Log {
        /// Filter by operation type (patch, delete, batch, plan, apply-files, query).
        #[arg(short, long)]
        operation_type: Option<String>,

        /// Filter by status (ok, error, partial).
        #[arg(short, long)]
        status: Option<String>,

        /// Show operations after this date (ISO 8601 or Unix timestamp).
        #[arg(long)]
        after: Option<String>,

        /// Show operations before this date (ISO 8601 or Unix timestamp).
        #[arg(long)]
        before: Option<String>,

        /// Maximum number of results.
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Skip first N results.
        #[arg(long, default_value = "0")]
        offset: usize,

        /// Get specific execution by ID.
        #[arg(short, long)]
        execution_id: Option<String>,

        /// Output as JSON.
        #[arg(short, long)]
        json: bool,

        /// Show statistics only.
        #[arg(long)]
        stats: bool,
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

impl Cli {
    /// Check if JSON output mode is enabled.
    pub fn json_output(&self) -> bool {
        self.json
    }
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
    /// Whether this payload has already been emitted (for --json mode).
    #[serde(skip)]
    pub already_emitted: bool,
    /// Whether changes are pending (for dry-run mode exit codes).
    #[serde(skip)]
    pub has_pending_changes: bool,
}

impl CliSuccessPayload {
    /// Construct a payload containing only the message.
    pub fn message_only(message: String) -> Self {
        Self {
            status: "ok",
            message,
            data: None,
            already_emitted: false,
            has_pending_changes: false,
        }
    }

    /// Construct a payload with structured data.
    pub fn with_data(message: String, data: Value) -> Self {
        Self {
            status: "ok",
            message,
            data: Some(data),
            already_emitted: false,
            has_pending_changes: false,
        }
    }

    /// Mark this payload as already emitted (for --json mode).
    pub fn already_emitted(mut self) -> Self {
        self.already_emitted = true;
        self
    }

    /// Mark this payload as having pending changes (for dry-run exit codes).
    pub fn with_pending_changes(mut self) -> Self {
        self.has_pending_changes = true;
        self
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
    /// Optional structured error code (SPL-E### format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<crate::ErrorCode>,
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

        // Try to create structured error code from SpliceError
        let error_code = crate::error_codes::SpliceErrorCode::from_splice_error(error)
            .map(|splice_code| {
                // Extract line and column from error if available
                let (_line, _column) = if error.file_path().is_some() {
                    // Try to get line/column from error context
                    (None, None) // TODO: Extract from error when available
                } else {
                    (None, None)
                };
                crate::ErrorCode::from_splice_code(
                    splice_code,
                    file.as_deref(),
                    _line,
                    _column,
                )
            });

        CliErrorPayload {
            status: "error",
            error: ErrorDetails {
                kind: error.kind(),
                message: error.to_string(),
                symbol,
                file,
                hint,
                diagnostics,
                error_code,
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
