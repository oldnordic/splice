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

/// Available Splice commands.
#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Delete a symbol by removing its definition.
    #[command(display_order = 200)]
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
    #[command(display_order = 201)]
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
        #[arg(
            short = 'n',
            long = "dry-run",
            alias = "preview",
            conflicts_with = "batch"
        )]
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

        /// Path to codegraph database (required for symbol resolution).
        #[arg(short = 'd', long, value_name = "FILE")]
        db: Option<std::path::PathBuf>,
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

        /// Number of context lines after the match.
        #[arg(short = 'A', long, value_name = "N", default_value = "0")]
        context_after: usize,

        /// Number of context lines before the match.
        #[arg(short = 'B', long, value_name = "N", default_value = "0")]
        context_before: usize,

        /// Number of context lines before and after the match (default: 3).
        #[arg(short = 'C', long, value_name = "N", default_value = "3")]
        context_both: usize,

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
    #[command(display_order = 104)]
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

        /// Expand symbol to full body.
        #[arg(long)]
        expand: bool,

        /// Expansion level (0=none, 1=body, 2=containing block).
        #[arg(long = "expand-level", value_name = "N", default_value = "1")]
        expand_level: usize,
    },

    /// Get code chunks from the database (uses Magellan integration).
    #[command(display_order = 105)]
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

        /// Expand symbol to full body.
        #[arg(long)]
        expand: bool,

        /// Expansion level (0=none, 1=body, 2=containing block).
        #[arg(long = "expand-level", value_name = "N", default_value = "1")]
        expand_level: usize,
    },

    /// Query execution log.
    #[command(display_order = 300)]
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

    /// Explain an error code with detailed documentation.
    #[command(display_order = 400)]
    Explain {
        /// Error code to explain (e.g., SPL-E001, SPL-E002)
        #[arg(short, long, value_name = "CODE")]
        code: String,
    },

    /// Search for code patterns in files.
    #[command(display_order = 401)]
    Search {
        /// Text pattern to search for.
        #[arg(short, long)]
        pattern: String,

        /// Files or directories to search (defaults to current directory).
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: std::path::PathBuf,

        /// Optional language filter (auto-detect if not specified).
        #[arg(long, value_name = "LANG")]
        language: Option<Language>,

        /// Glob pattern for file filtering (e.g., "src/**/*.rs", "tests/**/*.py").
        /// If not specified, searches all supported file types in path.
        #[arg(short = 'g', long, value_name = "GLOB")]
        glob: Option<String>,

        /// Number of context lines after the match.
        #[arg(short = 'A', long, value_name = "N", default_value = "0")]
        context_after: usize,

        /// Number of context lines before the match.
        #[arg(short = 'B', long, value_name = "N", default_value = "0")]
        context_before: usize,

        /// Number of context lines before and after the match (default: 2).
        #[arg(short = 'C', long, value_name = "N", default_value = "2")]
        context_both: usize,

        /// Apply replacement to all matches (atomic with rollback on failure).
        #[arg(long, requires = "replace")]
        apply: bool,

        /// Replacement text (required with --apply).
        #[arg(short = 'r', long, value_name = "TEXT")]
        replace: Option<String>,

        /// Output results as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Show database statistics (files, symbols, refs, calls, chunks)
    ///
    /// Use --detect-backend to check which backend format the database uses.
    #[command(display_order = 100)]
    Status {
        /// Path to the Magellan database
        #[arg(short, long)]
        db: std::path::PathBuf,

        /// Detect and report the backend format (sqlite or native-v2)
        #[arg(long, default_value = "false")]
        detect_backend: bool,
    },

    /// Find symbols by name or 16-character symbol ID
    #[command(display_order = 101)]
    Find {
        /// Path to the Magellan database
        #[arg(short, long)]
        db: std::path::PathBuf,

        /// Symbol name to search
        #[arg(short, long, conflicts_with = "symbol_id")]
        name: Option<String>,

        /// 16-character hex symbol ID
        #[arg(long, conflicts_with = "name")]
        symbol_id: Option<String>,

        /// Return all matches (default: first match only)
        #[arg(short, long)]
        ambiguous: bool,

        /// Output format (human, json, pretty)
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        output: OutputFormat,
    },

    /// Show call relationships for a symbol
    #[command(display_order = 102)]
    Refs {
        /// Path to the Magellan database
        #[arg(short, long)]
        db: std::path::PathBuf,

        /// Symbol name
        #[arg(short, long)]
        name: String,

        /// File path containing the symbol
        #[arg(short, long)]
        path: std::path::PathBuf,

        /// Direction: in (callers), out (callees), both (default)
        #[arg(long, value_enum, default_value_t = CallDirection::Both)]
        direction: CallDirection,

        /// Output format (human, json, pretty)
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        output: OutputFormat,
    },

    /// List all indexed files
    #[command(display_order = 103)]
    Files {
        /// Path to the Magellan database
        #[arg(short, long)]
        db: std::path::PathBuf,

        /// Include symbol count per file
        #[arg(long)]
        symbols: bool,

        /// Output format (human, json, pretty)
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        output: OutputFormat,
    },

    /// Export graph data in JSON, JSONL, or CSV format
    #[command(display_order = 106)]
    Export {
        /// Path to the Magellan database
        #[arg(short, long)]
        db: std::path::PathBuf,

        /// Export format (json, jsonl, csv)
        #[arg(short, long, value_enum, default_value_t = ExportFormat::Json)]
        format: ExportFormat,

        /// Output file path (writes to stdout if not specified)
        #[arg(long)]
        file: Option<std::path::PathBuf>,
    },

    /// Migrate Magellan database to latest schema version
    #[command(display_order = 107)]
    MigrateDb {
        /// Path to the Magellan database
        #[arg(short, long, default_value = ".codemcp/codegraph.db")]
        db_path: std::path::PathBuf,

        /// Create backup before migrating
        #[arg(long, default_value = "true")]
        backup: bool,

        /// Check migration status without migrating
        #[arg(long)]
        dry_run: bool,
    },

    /// Migrate a database from SQLite to native-v2 format
    #[command(display_order = 109)]
    Migrate {
        /// Path to the source database (SQLite format)
        #[arg(short = 's', long, value_name = "PATH")]
        source: std::path::PathBuf,

        /// Path to the destination database (will be created in native-v2 format)
        #[arg(short = 'd', long, value_name = "PATH")]
        dest: std::path::PathBuf,

        /// Show progress during migration
        #[arg(long, default_value = "true")]
        progress: bool,

        /// Skip post-migration verification (faster, but less safe)
        #[arg(long)]
        skip_verify: bool,
    },

    /// Rename a symbol across all files using byte-accurate references
    #[command(display_order = 110)]
    Rename {
        /// Symbol ID (32-char BLAKE3 or 16-char SHA-256)
        #[arg(short, long, conflicts_with = "name")]
        symbol: Option<String>,

        /// Symbol name (requires --file)
        #[arg(short, long, conflicts_with = "symbol")]
        name: Option<String>,

        /// File path for symbol name resolution (required with --name)
        #[arg(short, long)]
        file: Option<std::path::PathBuf>,

        /// New name for the symbol
        #[arg(short, long)]
        to: String,

        /// Path to Magellan database (default: .codemcp/codegraph.db)
        #[arg(short, long, default_value = ".codemcp/codegraph.db")]
        db: std::path::PathBuf,

        /// Preview changes without applying
        #[arg(short = 'n', long = "dry-run")]
        preview: bool,

        /// Generate proof file (requires --preview)
        #[arg(long)]
        proof: bool,

        /// Override backup directory (default: .splice/backups/)
        #[arg(long)]
        backup_dir: Option<std::path::PathBuf>,

        /// Skip backup creation
        #[arg(long)]
        no_backup: bool,
        /// Create backup before rename (default: true for safety, use --no-backup to skip)
        #[arg(long, default_value = "true")]
        create_backup: bool,
    },

    /// Show reachability analysis for a symbol (caller/callee chains)
    #[command(display_order = 111)]
    Reachable {
        /// Symbol name to analyze
        #[arg(short, long)]
        symbol: String,

        /// File path containing the symbol
        #[arg(short, long)]
        path: std::path::PathBuf,

        /// Path to Magellan database (default: .codemcp/codegraph.db)
        #[arg(short, long, default_value = ".codemcp/codegraph.db")]
        db: std::path::PathBuf,

        /// Analysis direction: forward (callees), reverse (callers), both
        #[arg(long, value_enum, default_value_t = ReachabilityDirection::Forward)]
        direction: ReachabilityDirection,

        /// Maximum depth to traverse (default: 10)
        #[arg(long, default_value = "10")]
        max_depth: usize,

        /// Output format (human, json, pretty)
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        output: OutputFormat,
    },

    /// Detect dead code (unreachable symbols) from entry points
    #[command(display_order = 112)]
    DeadCode {
        /// Entry point symbol name (e.g., "main", "MyApp::run")
        #[arg(short, long)]
        entry: String,

        /// File path containing the entry point symbol
        #[arg(short, long)]
        path: std::path::PathBuf,

        /// Path to Magellan database (default: .codemcp/codegraph.db)
        #[arg(short, long, default_value = ".codemcp/codegraph.db")]
        db: std::path::PathBuf,

        /// Exclude public symbols from dead code list
        #[arg(long)]
        exclude_public: bool,

        /// Group results by file (default: true for human output)
        #[arg(long, default_value = "true")]
        group_by_file: bool,

        /// Output format (human, json, pretty)
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        output: OutputFormat,
    },

    /// Detect cycles in the call graph
    #[command(display_order = 113)]
    Cycles {
        /// Path to Magellan database (default: .codemcp/codegraph.db)
        #[arg(short, long, default_value = ".codemcp/codegraph.db")]
        db: std::path::PathBuf,

        /// Optional: find cycles containing this specific symbol
        #[arg(short, long)]
        symbol: Option<String>,

        /// Optional: file path for symbol resolution (required with --symbol)
        #[arg(short, long)]
        path: Option<std::path::PathBuf>,

        /// Maximum number of cycles to return (default: 100)
        #[arg(short, long, default_value = "100")]
        max_cycles: usize,

        /// Show cycle members (default: true)
        #[arg(long, default_value = "true")]
        show_members: bool,

        /// Output format (human, json, pretty)
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        output: OutputFormat,
    },

    /// Analyze condensation graph (SCCs collapsed to DAG)
    #[command(display_order = 114)]
    Condense {
        /// Path to Magellan database (default: .codemcp/codegraph.db)
        #[arg(short, long, default_value = ".codemcp/codegraph.db")]
        db: std::path::PathBuf,

        /// Show SCC members (default: true for human output)
        #[arg(long, default_value = "true")]
        show_members: bool,

        /// Show topological levels
        #[arg(long)]
        show_levels: bool,

        /// Output format (human, json, pretty)
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        output: OutputFormat,
    },

    /// Perform program slicing (forward/backward impact analysis)
    #[command(display_order = 115)]
    Slice {
        /// Target symbol to slice from
        #[arg(short, long)]
        target: String,

        /// File path containing the target symbol
        #[arg(short, long)]
        path: std::path::PathBuf,

        /// Path to Magellan database (default: .codemcp/codegraph.db)
        #[arg(short, long, default_value = ".codemcp/codegraph.db")]
        db: std::path::PathBuf,

        /// Slice direction: forward (what this affects) or backward (what affects this)
        #[arg(long, value_enum, default_value_t = SliceDirection::Forward)]
        direction: SliceDirection,

        /// Maximum depth to traverse (default: unlimited)
        #[arg(long)]
        max_depth: Option<usize>,

        /// Output format (human, json, pretty)
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        output: OutputFormat,
    },

    /// Validate proof checksums for refactoring audit trail
    #[command(display_order = 116)]
    ValidateProof {
        /// Path to the proof JSON file
        #[arg(short, long)]
        proof: std::path::PathBuf,

        /// Output format (human, json, pretty)
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        output: OutputFormat,
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

// Re-export Magellan-compatible response types for external use
pub use crate::output::{
    CallExport, ExportData, ExportResponse, FileExport, FilesResponse, FindResponse,
    MagellanCallReference, MagellanFileMetadata, MagellanSpan, MagellanSymbol, ReferenceExport,
    RefsResponse, StatusResponse, SymbolExport, EXPORT_SCHEMA_VERSION,
};
