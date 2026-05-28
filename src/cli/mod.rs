//! Command-line interface for Splice.
//!
//! This module handles argument parsing and user interface only.
//! NO logic or database operations are performed here.

use clap::Parser;
use serde::Serialize;
use serde_json::Value;

pub mod commands;
pub mod types;

pub use commands::*;
pub use types::*;

/// Splice: Span-safe refactoring kernel for Rust.
#[derive(Parser, Debug)]
#[command(name = "splice")]
#[command(
    author,
    version,
    about,
    long_about = "
Splice: Span-safe refactoring kernel for Rust.

Query Commands (Magellan-delegated):
  status, find, refs, files, query    Query code graph database

Graph Algorithm Commands:
  reachable, dead-code, cycles         Analyze code structure
  condense, slice                      Impact analysis and slicing

Edit Commands:
  delete, patch, plan, apply-files     Modify code with span safety

Export Commands:
  log, undo, export                     Export and restore operations

Validation Commands:
  explain, search, get                  Validate and explain code

Use 'splice help <command>' for more information on a specific command.

Options:
  -v, --verbose           Enable verbose logging
  -o, --output <FORMAT>   Output format (human, json, pretty)
      --json              Output JSON (deprecated: use --output json)
      --strict            Enable strict pre-verification
  -h, --help              Print help
  -V, --version           Print version
"
)]
#[command(subcommand_required = true)]
pub struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output format (human, json, pretty)
    #[arg(short, long, global = true, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,

    /// Output structured JSON (deprecated: use --output json instead)
    #[arg(long, global = true, hide = true)]
    json: bool,

    /// Enable strict pre-verification (warnings become errors).
    #[arg(long, global = true)]
    pub strict: bool,

    /// Skip pre-verification checks (dangerous!).
    #[arg(long, global = true, hide = true)]
    pub skip_pre_verify: bool,
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
        // --json flag takes precedence for backward compatibility
        if self.json {
            return true;
        }
        self.output.is_json()
    }

    /// Get the output format setting.
    pub fn output_format(&self) -> OutputFormat {
        // --json flag overrides to Json format for backward compat
        if self.json {
            return OutputFormat::Json;
        }
        self.output
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
    /// Optional explain command for this error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain_command: Option<String>,
}

impl CliErrorPayload {
    /// Build payload from a SpliceError instance.
    pub fn from_error(error: &crate::SpliceError) -> Self {
        let symbol = error.symbol().map(|s| s.to_string());
        let file = error
            .file_path()
            .and_then(|p| p.to_str().map(|s| s.to_string()));
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
        let error_code =
            crate::error_codes::SpliceErrorCode::from_splice_error(error).map(|splice_code| {
                // Extract line and column from error using location() helper
                let (file, line, column) = error.location();
                crate::ErrorCode::from_splice_code(splice_code, file, line, column)
            });

        // Generate explain command if error_code is present
        let explain_command = error_code
            .as_ref()
            .map(|ec| format!("splice explain --code {}", ec.code));

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
                explain_command,
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
            file: diag
                .file
                .as_ref()
                .and_then(|p| p.to_str().map(|s| s.to_string())),
            line: diag.line,
            column: diag.column,
            code: diag.code,
            note: diag.note,
            tool_path: diag
                .tool_path
                .as_ref()
                .and_then(|p| p.to_str().map(|s| s.to_string())),
            tool_version: diag.tool_version,
            remediation: diag.remediation,
        }
    }
}

// CLI tests
#[cfg(test)]
mod tests;

// Re-export Magellan-compatible response types for external use
pub use crate::output::{
    CallExport, ExportData, ExportResponse, FileExport, FilesResponse, FindResponse,
    MagellanCallReference, MagellanFileMetadata, MagellanSpan, MagellanSymbol, ReferenceExport,
    RefsResponse, StatusResponse, SymbolExport, EXPORT_SCHEMA_VERSION,
};
