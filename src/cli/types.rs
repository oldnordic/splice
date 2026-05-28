//! CLI type definitions (enums and value types).

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

/// Output format for query results.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text (default)
    Human,
    /// Compact JSON
    Json,
    /// Pretty-printed JSON
    Pretty,
}

/// Call direction for relationship queries.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDirection {
    /// Show callers (what calls this symbol)
    In,
    /// Show callees (what this symbol calls)
    Out,
    /// Show both callers and callees
    Both,
}

/// Reachability analysis direction.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachabilityDirection {
    /// Forward: symbols this symbol calls (callees)
    Forward,
    /// Reverse: symbols that call this symbol (callers)
    Reverse,
    /// Both directions
    Both,
}

/// Slice direction.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceDirection {
    /// Forward slice: what this symbol affects
    Forward,
    /// Backward slice: what affects this symbol
    Backward,
}

/// Export format for graph data.
#[derive(clap::ValueEnum, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON array format (default)
    #[default]
    Json,
    /// JSON Lines (newline-delimited JSON)
    Jsonl,
    /// CSV format with headers
    Csv,
}

impl OutputFormat {
    /// Check if JSON output is requested (either Json or Pretty variant)
    pub fn is_json(self) -> bool {
        matches!(self, Self::Json | Self::Pretty)
    }

    /// Format a value as JSON based on this format setting
    pub fn format_json<T: serde::Serialize>(&self, value: &T) -> Result<String, String> {
        match self {
            Self::Json => serde_json::to_string(value).map_err(|e| e.to_string()),
            Self::Pretty => serde_json::to_string_pretty(value).map_err(|e| e.to_string()),
            Self::Human => Err("Human format requested but format_json called".to_string()),
        }
    }
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

/// Rollback mode for batch operations.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliRollbackMode {
    /// Auto: rollback on failure if --db is provided
    Auto,
    /// Never: never rollback, even on failure
    Never,
    /// Always: rollback after successful batch (for testing)
    Always,
}
