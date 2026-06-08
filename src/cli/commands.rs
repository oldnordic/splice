//! CLI command definitions (clap subcommand enums).

use super::*;

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

        /// Path to rust-analyzer binary (used with --analyzer path).
        #[arg(long, value_name = "PATH")]
        analyzer_binary: Option<std::path::PathBuf>,

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

        /// Capture graph snapshot before deleting.
        #[arg(long)]
        snapshot_before: bool,
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

        /// Path to rust-analyzer binary (used with --analyzer path).
        #[arg(long, value_name = "PATH")]
        analyzer_binary: Option<std::path::PathBuf>,

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

        /// Capture graph snapshot before patching.
        #[arg(long)]
        snapshot_before: bool,

        /// Generate DOT graph output for visualization (requires --preview)
        #[arg(long, requires = "preview")]
        impact_graph: bool,
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

        /// Preview changes without applying — prints what would change and exits.
        #[arg(short = 'n', long = "dry-run", visible_alias = "preview")]
        dry_run: bool,
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

        /// Filter results by file path (optional).
        /// Can be a glob pattern: "src/main.rs", "src/**/*.rs", etc.
        #[arg(long)]
        file: Option<String>,

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

        /// Detect and report the backend format (sqlite only)
        #[arg(long, default_value = "false")]
        detect_backend: bool,
    },

    /// Find symbols by name, ID, or natural-language semantic query
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

        /// Natural-language query resolved via semantic search (HNSW embeddings)
        #[arg(long)]
        semantic_query: Option<String>,

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

        /// Generate DOT graph output for visualization
        #[arg(long)]
        impact_graph: bool,
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
        #[arg(short, long = "db", default_value = ".magellan/magellan.db")]
        db_path: std::path::PathBuf,

        /// Create backup before migrating
        #[arg(long, default_value = "true")]
        backup: bool,

        /// Check migration status without migrating
        #[arg(long)]
        dry_run: bool,
    },

    /// Rename a symbol across all files using byte-accurate references
    #[command(display_order = 110)]
    Rename {
        /// Symbol ID (32-char BLAKE3 or 16-char SHA-256)
        #[arg(short, long, conflicts_with = "name")]
        symbol: Option<String>,

        /// Symbol name (requires --file)
        #[arg(long, conflicts_with = "symbol")]
        name: Option<String>,

        /// File path for symbol name resolution (required with --name)
        #[arg(short, long)]
        file: Option<std::path::PathBuf>,

        /// New name for the symbol
        #[arg(short, long)]
        to: String,

        /// Path to Magellan database (default: .magellan/magellan.db)
        #[arg(short, long, default_value = ".magellan/magellan.db")]
        db: std::path::PathBuf,

        /// Preview changes without applying
        #[arg(short = 'n', long = "dry-run")]
        preview: bool,

        /// Generate proof file (requires --dry-run)
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

        /// Capture graph snapshot before renaming.
        #[arg(long)]
        snapshot_before: bool,

        /// Generate DOT graph output for visualization (requires --preview)
        #[arg(long, requires = "preview")]
        impact_graph: bool,
    },

    /// Show reachability analysis for a symbol (caller/callee chains)
    #[command(display_order = 111)]
    Reachable {
        /// Symbol name to analyze
        #[arg(short, long, default_value = "")]
        symbol: String,

        /// Natural-language query resolved via semantic search (HNSW embeddings)
        #[arg(long)]
        semantic_query: Option<String>,

        /// File path containing the symbol
        #[arg(short, long)]
        path: std::path::PathBuf,

        /// Path to Magellan database (default: .magellan/magellan.db)
        #[arg(short, long, default_value = ".magellan/magellan.db")]
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

        /// Generate DOT graph output for visualization
        #[arg(long)]
        impact_graph: bool,
    },

    /// Detect dead code (unreachable symbols) from entry points
    #[command(display_order = 112)]
    DeadCode {
        /// Entry point symbol name (e.g., "main", "MyApp::run")
        #[arg(short, long, default_value = "")]
        entry: String,

        /// Natural-language query resolved via semantic search (HNSW embeddings)
        #[arg(long)]
        semantic_query: Option<String>,

        /// File path containing the entry point symbol
        #[arg(short, long)]
        path: std::path::PathBuf,

        /// Path to Magellan database (default: .magellan/magellan.db)
        #[arg(short, long, default_value = ".magellan/magellan.db")]
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
        /// Path to Magellan database (default: .magellan/magellan.db)
        #[arg(short, long, default_value = ".magellan/magellan.db")]
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
        /// Path to Magellan database (default: .magellan/magellan.db)
        #[arg(short, long, default_value = ".magellan/magellan.db")]
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
        #[arg(short, long, default_value = "")]
        target: String,

        /// Natural-language query resolved via semantic search (HNSW embeddings)
        #[arg(long)]
        semantic_query: Option<String>,

        /// File path containing the target symbol
        #[arg(short, long)]
        path: std::path::PathBuf,

        /// Path to Magellan database (default: .magellan/magellan.db)
        #[arg(short, long, default_value = ".magellan/magellan.db")]
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

    /// Compare two snapshots and report differences
    #[command(display_order = 117)]
    Verify {
        /// Path to the "before" snapshot file
        #[arg(short = 'b', long)]
        before: std::path::PathBuf,

        /// Path to the "after" snapshot file
        #[arg(short = 'a', long)]
        after: std::path::PathBuf,

        /// Show detailed symbol-by-symbol differences
        #[arg(long)]
        detailed: bool,

        /// Output format (human, json, pretty)
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        output: OutputFormat,
    },

    /// Execute batch operations from YAML spec
    #[command(display_order = 250)]
    Batch {
        /// Path to the batch specification YAML file
        #[arg(short = 'f', long)]
        spec: std::path::PathBuf,

        /// Database path for snapshot/impact analysis (required for rollback)
        #[arg(short = 'd', long)]
        db: Option<std::path::PathBuf>,

        /// Preview changes without applying (alias: --dry-run, -n)
        #[arg(short = 'n', long = "dry-run")]
        dry_run: bool,

        /// Continue on error instead of stopping
        #[arg(long = "continue-on-error")]
        continue_on_error: bool,

        /// Rollback mode: auto, never, always
        #[arg(long, value_enum, default_value_t = CliRollbackMode::Auto)]
        rollback: CliRollbackMode,

        /// Optional validation mode (off, os, path).
        #[arg(long, value_name = "MODE")]
        analyzer: Option<AnalyzerMode>,

        /// Path to rust-analyzer binary (used with --analyzer path).
        #[arg(long, value_name = "PATH")]
        analyzer_binary: Option<std::path::PathBuf>,
    },

    /// Create a new file with validation
    #[command(display_order = 105)]
    Create {
        /// Path to the file to create
        #[arg(short, long)]
        file: std::path::PathBuf,

        /// Validate only (don't write file)
        #[arg(short = 'V', long)]
        validate_only: bool,

        /// Add module declaration to parent module
        #[arg(short = 'm', long)]
        with_mod: bool,

        /// Workspace directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        workspace: std::path::PathBuf,
    },

    /// Get grounded code completions using Magellan database
    #[command(display_order = 119)]
    Complete {
        /// Path to the source file
        #[arg(short, long)]
        file: std::path::PathBuf,

        /// Line number (1-based)
        #[arg(short, long)]
        line: usize,

        /// Column number (1-based)
        #[arg(short, long)]
        column: usize,

        /// Maximum number of suggestions
        #[arg(short, long, default_value = "10")]
        max_results: usize,

        /// Path to Magellan database
        #[arg(short, long, default_value = ".magellan/splice.db")]
        db: std::path::PathBuf,
    },

    /// Manage code graph snapshots
    #[command(display_order = 120, subcommand)]
    Snapshots(SnapshotsCommands),
}

/// Snapshot management subcommands.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum SnapshotsCommands {
    /// List all snapshots
    List {
        /// Filter by operation type (patch, delete, rename)
        #[arg(short, long)]
        operation: Option<String>,

        /// Maximum number of snapshots to show
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Show total disk usage
        #[arg(long)]
        disk_usage: bool,

        /// Output format (human, json, pretty)
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        output: OutputFormat,
    },

    /// Delete a specific snapshot
    Delete {
        /// Snapshot ID (timestamp or filename)
        #[arg(short, long)]
        id: String,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Clean up old snapshots (keep N most recent)
    Cleanup {
        /// Number of recent snapshots to keep (default: 10)
        #[arg(short = 'k', long, default_value = "10")]
        keep: usize,

        /// Show what would be deleted without deleting
        #[arg(long)]
        dry_run: bool,

        /// Confirm deletion of more than 50 snapshots. Without this flag,
        /// a bulk-delete refuses to run and asks for either `--yes` or `--dry-run`.
        #[arg(long)]
        yes: bool,
    },
}
