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

        /// Optional symbol kind filter.
        #[arg(short, long)]
        kind: Option<SymbolKind>,

        /// Optional validation mode (off, os, path).
        #[arg(long, value_name = "MODE")]
        analyzer: Option<AnalyzerMode>,

        /// Optional language (auto-detect from extension by default).
        #[arg(long, value_name = "LANG")]
        language: Option<Language>,
    },

    /// Apply a patch to a symbol's span.
    Patch {
        /// Path to the source file containing the symbol.
        #[arg(short, long)]
        file: std::path::PathBuf,

        /// Symbol name to patch.
        #[arg(short, long)]
        symbol: String,

        /// Optional symbol kind filter.
        #[arg(short, long)]
        kind: Option<SymbolKind>,

        /// Optional validation mode (off, os, path).
        #[arg(long, value_name = "MODE")]
        analyzer: Option<AnalyzerMode>,

        /// Path to file containing replacement content.
        #[arg(short, long, value_name = "FILE")]
        with_: std::path::PathBuf,

        /// Optional language (auto-detect from extension by default).
        #[arg(long, value_name = "LANG")]
        language: Option<Language>,
    },

    /// Execute a multi-step refactoring plan.
    Plan {
        /// Path to the plan.json file.
        #[arg(short, long)]
        file: std::path::PathBuf,
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
