//! Command-line interface for Splice.
//!
//! This module handles argument parsing and user interface only.
//! NO logic or database operations are performed here.

use clap::Parser;

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

        /// Optional symbol kind filter (function, struct, enum, trait, impl).
        #[arg(short, long)]
        kind: Option<SymbolKind>,

        /// Optional rust-analyzer validation mode (off, os, path).
        #[arg(long, value_name = "MODE")]
        analyzer: Option<AnalyzerMode>,
    },

    /// Apply a patch to a symbol's span.
    Patch {
        /// Path to the source file containing the symbol.
        #[arg(short, long)]
        file: std::path::PathBuf,

        /// Symbol name to patch.
        #[arg(short, long)]
        symbol: String,

        /// Optional symbol kind filter (function, struct, enum, trait, impl).
        #[arg(short, long)]
        kind: Option<SymbolKind>,

        /// Optional rust-analyzer validation mode (off, os, path).
        #[arg(long, value_name = "MODE")]
        analyzer: Option<AnalyzerMode>,

        /// Path to file containing replacement content.
        #[arg(short, long, value_name = "FILE")]
        with_: std::path::PathBuf,
    },

    /// Execute a multi-step refactoring plan.
    Plan {
        /// Path to the plan.json file.
        #[arg(short, long)]
        file: std::path::PathBuf,
    },
}

/// Symbol kind for filtering.
#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum SymbolKind {
    /// Function symbol.
    Function,
    /// Struct symbol.
    Struct,
    /// Enum symbol.
    Enum,
    /// Trait symbol.
    Trait,
    /// Impl block.
    Impl,
}

/// rust-analyzer validation mode.
#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum AnalyzerMode {
    /// Disable rust-analyzer validation (default).
    Off,

    /// Use rust-analyzer from PATH.
    Os,

    /// Use rust-analyzer from explicit path.
    Path,
}

/// Parse command-line arguments.
///
/// This function is the entry point for CLI argument parsing.
/// It returns the parsed Cli struct or exits on error.
pub fn parse_args() -> Cli {
    Cli::parse()
}
