//! Splice CLI binary
//!
//! This is the main entry point for the splice command-line interface.
//! The CLI is a thin adapter over existing APIs - NO logic is implemented here.

use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Splice exit codes matching Magellan conventions.
///
/// Magellan exit codes:
/// - 0: success
/// - 1: generic error
/// - 2: usage error (invalid arguments)
/// - 3: database error
/// - 4: file not found
/// - 5: validation error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpliceExitCode {
    /// Operation succeeded
    Success = 0,
    /// Generic error (catch-all)
    Error = 1,
    /// Usage error (invalid arguments, missing required args)
    Usage = 2,
    /// Database error (Magellan graph access failure)
    Database = 3,
    /// File not found (requested file doesn't exist)
    FileNotFound = 4,
    /// Validation error (pre-verification, compiler check failed)
    Validation = 5,
}

impl SpliceExitCode {
    /// Map SpliceError to appropriate exit code.
    ///
    /// Note: clap handles argument parsing errors and exits with code 2
    /// before application code runs. This maps application-level errors.
    pub fn from_error(error: &splice::SpliceError) -> Self {
        match error {
            // Database-specific errors
            splice::SpliceError::Graph(_) => Self::Database,
            splice::SpliceError::ExecutionLogError { .. } => Self::Database,
            splice::SpliceError::Magellan { .. } => Self::Database,

            // File access errors (Io, IoContext, FileExternallyModified)
            splice::SpliceError::Io { .. } | splice::SpliceError::IoContext { .. }
                if error.file_path().is_some() =>
            {
                Self::FileNotFound
            }
            splice::SpliceError::FileExternallyModified { .. } => Self::FileNotFound,

            // Validation errors (all validation-related variants)
            splice::SpliceError::ParseValidationFailed { .. } => Self::Validation,
            splice::SpliceError::CompilerValidationFailed { .. } => Self::Validation,
            splice::SpliceError::AnalyzerFailed { .. } => Self::Validation,
            splice::SpliceError::CargoCheckFailed { .. } => Self::Validation,
            splice::SpliceError::PreVerificationFailed { .. } => Self::Validation,

            // Usage/schema errors (invalid plan, batch, date format)
            splice::SpliceError::InvalidPlanSchema { .. } => Self::Usage,
            splice::SpliceError::InvalidBatchSchema { .. } => Self::Usage,
            splice::SpliceError::InvalidDateFormat { .. } => Self::Usage,

            // Broken pipe is success (pipelines handle SIGPIPE)
            splice::SpliceError::BrokenPipe => Self::Success,

            // Default to generic error for all other cases
            _ => Self::Error,
        }
    }

    /// Convert to std::process::ExitCode.
    pub fn as_exit_code(self) -> ExitCode {
        ExitCode::from(self as u8)
    }
}

/// Helper to log execution recording failures without failing the operation.
fn log_execution_error(operation: &str, err: &splice::SpliceError) {
    eprintln!(
        "Warning: Failed to record execution for '{}': {}",
        operation, err
    );
}

/// Count lines in a byte span within a file.
///
/// This helper function counts how many lines are covered by a byte span.
/// Used to calculate lines_removed for delete operations.
fn count_lines_in_span(file_path: &Path, start: usize, end: usize) -> usize {
    use std::fs;

    if end <= start {
        return 0;
    }

    // Read the file and count newlines in the span
    if let Ok(content) = fs::read_to_string(file_path) {
        let span_end = end.min(content.len());
        let span = &content[start..span_end];
        span.lines().count()
    } else {
        0
    }
}

/// Capture a graph snapshot before a refactoring operation.
fn capture_snapshot(db_path: &Path, operation: &str) -> Result<(), splice::SpliceError> {
    use splice::proof::data_structures::RefactoringProof;
    use splice::proof::generation::{create_metadata, generate_snapshot};
    use std::fs;

    // Generate snapshot from current database state
    let snapshot = generate_snapshot(db_path)?;

    // Create snapshots directory if it doesn't exist
    let snapshots_dir = PathBuf::from(".splice").join("snapshots");
    fs::create_dir_all(&snapshots_dir).map_err(|e| {
        splice::SpliceError::Other(format!("Failed to create snapshots dir: {}", e))
    })?;

    // Create minimal proof metadata
    let metadata = create_metadata(operation, db_path);

    // Create minimal RefactoringProof (only before snapshot, no after)
    let proof = RefactoringProof {
        metadata,
        before: snapshot.clone(),
        after: snapshot, // Use same snapshot for before/after (snapshot-only mode)
        invariants: vec![],
        checksums: None,
    };

    // Write proof to file
    let timestamp = chrono::Utc::now().timestamp();
    let filename = format!("snapshot-{}-{}.json", operation, timestamp);
    let snapshot_path = snapshots_dir.join(&filename);

    let json = serde_json::to_string_pretty(&proof)
        .map_err(|e| splice::SpliceError::Other(format!("Failed to serialize snapshot: {}", e)))?;

    fs::write(&snapshot_path, json)
        .map_err(|e| splice::SpliceError::Other(format!("Failed to write snapshot: {}", e)))?;

    eprintln!("Snapshot captured: {}", snapshot_path.display());
    Ok(())
}

/// Generate and output DOT graph for impact visualization.
fn execute_impact_graph(
    db_path: &Path,
    symbol_id: &str,
    direction: &splice::cli::ReachabilityDirection,
    max_depth: Option<usize>,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::{ImpactDotConfig, MagellanIntegration};

    let mut integration = MagellanIntegration::open(db_path)?;

    let config = ImpactDotConfig {
        show_symbol_kinds: true,
        max_depth,
        highlight_symbol: None,
    };

    let dot = integration.generate_impact_dot(symbol_id, direction, &config)?;

    // Output DOT to stdout
    println!("{}", dot);

    Ok(
        splice::cli::CliSuccessPayload::message_only("Impact graph generated".to_string())
            .already_emitted(),
    )
}

fn main() -> ExitCode {
    install_broken_pipe_hook();

    // Check platform and warn about limitations
    splice::platform::check_platform_support();

    // Parse CLI arguments
    let cli = splice::cli::parse_args();
    let json_output = cli.json_output();

    // Initialize logger if verbose
    if cli.verbose {
        env_logger::init();
    }

    // Execute command
    let result: Result<splice::cli::CliSuccessPayload, splice::SpliceError> = match cli.command {
        splice::cli::Commands::Delete {
            file,
            symbol,
            kind,
            analyzer,
            analyzer_binary,
            language,
            context_after,
            context_before,
            context,
            create_backup,
            relationships,
            dry_run,
            unified,
            operation_id,
            metadata,
            snapshot_before,
        } => execute_delete(
            &file,
            &symbol,
            kind,
            analyzer,
            analyzer_binary,
            language,
            context_before,
            context_after,
            context,
            create_backup,
            relationships,
            dry_run,
            unified,
            operation_id,
            metadata,
            snapshot_before,
            json_output,
            cli.strict,
            true, // skip_pre_verify: delete operations don't require graph DB by default
        ),

        splice::cli::Commands::Patch {
            file,
            symbol,
            kind,
            analyzer,
            analyzer_binary,
            with_: replacement_file,
            language,
            batch,
            context_after,
            context_before,
            context_both,
            preview,
            unified,
            create_backup,
            relationships,
            operation_id,
            metadata,
            db,
            snapshot_before,
            impact_graph,
        } => match batch {
            Some(batch_path) => execute_patch_batch(
                &batch_path,
                analyzer,
                analyzer_binary,
                language,
                create_backup,
                operation_id,
                metadata,
                json_output,
            ),
            None => execute_single_patch(
                file,
                symbol,
                kind,
                analyzer,
                analyzer_binary,
                replacement_file,
                language,
                context_before,
                context_after,
                context_both,
                preview,
                unified,
                create_backup,
                relationships,
                operation_id,
                metadata,
                db,
                snapshot_before,
                impact_graph,
                json_output,
                cli.strict,
                true, // skip_pre_verify: patch operations don't require graph DB by default
            ),
        },

        splice::cli::Commands::Create {
            file,
            validate_only,
            with_mod,
            workspace,
        } => {
            match splice::commands::cmd_create(
                &file,
                validate_only,
                with_mod,
                &workspace,
                json_output,
            ) {
                Ok(()) => Ok(splice::cli::CliSuccessPayload::message_only(
                    if validate_only {
                        "Validation complete (file not created)".to_string()
                    } else {
                        format!("File created: {}", file.display())
                    },
                )),
                Err(e) => Err(e),
            }
        }

        splice::cli::Commands::Plan {
            file,
            operation_id,
            metadata,
        } => execute_plan(&file, operation_id, metadata, json_output),

        splice::cli::Commands::Undo { manifest } => execute_undo(&manifest, json_output),

        splice::cli::Commands::ApplyFiles {
            glob,
            find,
            replace,
            language,
            context_after,
            context_before,
            context_both,
            no_validate,
            create_backup,
            operation_id,
            metadata,
            dry_run,
        } => execute_apply_files(
            &glob,
            &find,
            &replace,
            language,
            context_before,
            context_after,
            context_both,
            !no_validate,
            create_backup,
            operation_id,
            metadata,
            dry_run,
            json_output,
        ),

        splice::cli::Commands::Query {
            db,
            label,
            file,
            context_after,
            context_before,
            context_both,
            list,
            count,
            show_code,
            relationships,
            expand,
            expand_level,
        } => execute_query(
            &db,
            &label,
            file.as_deref(),
            context_before,
            context_after,
            context_both,
            list,
            count,
            show_code,
            relationships,
            expand,
            expand_level,
            json_output,
        ),

        splice::cli::Commands::Get {
            db,
            file,
            start,
            end,
            context_after,
            context_before,
            context_both,
            relationships,
            expand,
            expand_level,
        } => execute_get(
            &db,
            &file,
            start,
            end,
            context_before,
            context_after,
            context_both,
            relationships,
            expand,
            expand_level,
            json_output,
        ),

        splice::cli::Commands::Log {
            operation_type,
            status,
            after,
            before,
            limit,
            offset,
            execution_id,
            json,
            stats,
        } => execute_log(
            operation_type,
            status,
            after,
            before,
            limit,
            offset,
            execution_id,
            json,
            stats,
            json_output,
        ),

        splice::cli::Commands::Explain { code } => execute_explain(code, json_output),

        splice::cli::Commands::Search {
            pattern,
            path,
            language,
            glob,
            context_after,
            context_before,
            context_both,
            apply,
            replace,
            json,
        } => execute_search(
            &pattern,
            &path,
            language,
            glob,
            apply,
            replace.as_deref(),
            context_before,
            context_after,
            context_both,
            json_output || json,
        ),

        splice::cli::Commands::Status { db, detect_backend } => {
            execute_status(&db, json_output, detect_backend)
        }

        splice::cli::Commands::Find {
            db,
            name,
            symbol_id,
            ambiguous,
            output,
        } => execute_find(&db, name, symbol_id, ambiguous, output, json_output),

        splice::cli::Commands::Refs {
            db,
            name,
            path,
            direction,
            output,
            impact_graph,
        } => execute_refs(
            &db,
            &name,
            &path,
            direction,
            output,
            impact_graph,
            json_output,
        ),

        splice::cli::Commands::Files {
            db,
            symbols,
            output,
        } => execute_files(&db, symbols, output, json_output),

        splice::cli::Commands::Export {
            db,
            format: export_format,
            file,
        } => execute_export(&db, export_format, file.as_deref(), json_output),

        splice::cli::Commands::MigrateDb {
            db_path,
            backup,
            dry_run,
        } => execute_migrate_db(&db_path, backup, dry_run, json_output),

        splice::cli::Commands::Rename {
            symbol,
            name,
            file,
            to,
            db,
            preview,
            proof,
            backup_dir,
            no_backup,
            create_backup: _,
            snapshot_before,
            impact_graph,
        } => execute_rename(
            symbol.as_deref(),
            name.as_deref(),
            file.as_ref(),
            &to,
            &db,
            preview,
            proof,
            backup_dir.as_ref(),
            no_backup,
            snapshot_before,
            impact_graph,
            json_output,
        ),

        splice::cli::Commands::Reachable {
            symbol,
            path,
            db,
            direction,
            max_depth,
            output,
            impact_graph,
        } => execute_reachable(
            &symbol,
            &path,
            &db,
            &direction,
            max_depth,
            output,
            impact_graph,
            json_output,
        ),

        splice::cli::Commands::DeadCode {
            entry,
            path,
            db,
            exclude_public,
            group_by_file,
            output,
        } => execute_dead_code(
            &entry,
            &path,
            &db,
            exclude_public,
            group_by_file,
            output,
            json_output,
        ),

        splice::cli::Commands::Cycles {
            db,
            symbol,
            path,
            max_cycles,
            show_members,
            output,
        } => execute_cycles(
            &db,
            symbol.as_deref(),
            path.as_ref(),
            max_cycles,
            show_members,
            output,
            json_output,
        ),

        splice::cli::Commands::Condense {
            db,
            show_members,
            show_levels,
            output,
        } => execute_condense(&db, show_members, show_levels, output, json_output),

        splice::cli::Commands::Slice {
            target,
            path,
            db,
            direction,
            max_depth,
            output,
        } => execute_slice(
            &target,
            &path,
            &db,
            &direction,
            max_depth,
            output,
            json_output,
        ),

        splice::cli::Commands::ValidateProof { proof, output } => {
            execute_validate_proof(&proof, output, json_output)
        }

        splice::cli::Commands::Verify {
            before,
            after,
            detailed,
            output,
        } => execute_verify(&before, &after, detailed, output, json_output),

        splice::cli::Commands::Batch {
            spec,
            db,
            dry_run,
            continue_on_error,
            rollback,
            analyzer,
            analyzer_binary,
        } => execute_batch(
            &spec,
            db,
            dry_run,
            continue_on_error,
            rollback,
            analyzer,
            analyzer_binary,
            json_output,
        ),

        splice::cli::Commands::Complete {
            file,
            line,
            column,
            max_results,
            db,
        } => execute_complete(&file, line, column, max_results, &db, json_output),

        splice::cli::Commands::Snapshots(subcommand) => execute_snapshots(subcommand, json_output),
    };

    // Handle result
    match result {
        Ok(payload) => match emit_success_payload(&payload, json_output) {
            Ok(()) => {
                // For dry-run mode, return exit code 1 if changes are pending (git diff convention)
                if payload.has_pending_changes {
                    SpliceExitCode::Error.as_exit_code()
                } else {
                    SpliceExitCode::Success.as_exit_code()
                }
            }
            Err(err) => {
                if matches!(err, splice::SpliceError::BrokenPipe) {
                    SpliceExitCode::Success.as_exit_code()
                } else {
                    let payload = splice::cli::CliErrorPayload::from_error(&err);
                    emit_error_payload(&payload, json_output);
                    SpliceExitCode::from_error(&err).as_exit_code()
                }
            }
        },
        Err(e) => {
            if matches!(e, splice::SpliceError::BrokenPipe) {
                SpliceExitCode::Success.as_exit_code()
            } else {
                let payload = splice::cli::CliErrorPayload::from_error(&e);
                emit_error_payload(&payload, json_output);
                SpliceExitCode::from_error(&e).as_exit_code()
            }
        }
    }
}

fn install_broken_pipe_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if is_broken_pipe_panic(info) {
            std::process::exit(SpliceExitCode::Success as u8 as i32);
        }
        default_hook(info);
    }));
}

fn is_broken_pipe_panic(info: &std::panic::PanicHookInfo<'_>) -> bool {
    if let Some(err) = info.payload().downcast_ref::<std::io::Error>() {
        return err.kind() == std::io::ErrorKind::BrokenPipe;
    }

    let message = if let Some(msg) = info.payload().downcast_ref::<&str>() {
        *msg
    } else if let Some(msg) = info.payload().downcast_ref::<String>() {
        msg.as_str()
    } else {
        ""
    };

    if message.contains("Broken pipe") || message.contains("failed printing to stdout") {
        return true;
    }

    let info_message = info.to_string();
    info_message.contains("Broken pipe") || info_message.contains("failed printing to stdout")
}

/// Execute the delete command.
///
/// This function is a thin adapter that:
/// 1. Extracts symbols from source file using language-aware dispatcher
/// 2. Finds all references to the symbol (same-file and cross-file)
/// 3. Optionally creates a backup if requested
/// 4. Deletes all references first (in reverse byte order per file)
/// 5. Deletes the definition last
/// 6. Applies each deletion with validation gates
///
/// All logic is delegated to existing APIs.
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
fn execute_delete(
    file_path: &Path,
    symbol_name: &str,
    kind: Option<splice::cli::SymbolKind>,
    analyzer: Option<splice::cli::AnalyzerMode>,
    analyzer_binary: Option<std::path::PathBuf>,
    language: Option<splice::cli::Language>,
    context_before: usize,
    context_after: usize,
    context: usize,
    create_backup: bool,
    relationships: bool,
    dry_run: bool,
    unified: usize,
    operation_id: Option<String>,
    metadata: Option<String>,
    snapshot_before: bool,
    json_output: bool,
    strict: bool,
    skip_pre_verify: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use ropey::Rope;
    use splice::execution::log;
    use splice::format_colored_diff;
    use splice::format_diff_summary;
    use splice::format_unified_diff;
    use splice::graph::CodeGraph;
    use splice::patch::apply_patch_with_validation;
    use splice::resolve::references::find_references;
    use splice::should_use_color;
    use splice::symbol::{Language as SymbolLanguage, Symbol};
    use splice::validate::AnalyzerMode as ValidateAnalyzerMode;

    // Resolve context counts from -A/-B/-C flags
    let (ctx_before, ctx_after) =
        splice::resolve_context_counts(context_before, context_after, context);

    // Capture snapshot before operation if requested
    if snapshot_before {
        eprintln!("Warning: --snapshot-before is not yet supported for delete operations");
    }

    // Start timing
    let start = std::time::Instant::now();
    let command_line = std::env::args().collect::<Vec<_>>().join(" ");

    // Determine language (from CLI flag or auto-detect from file extension)
    let symbol_lang = language
        .map(|l| l.to_symbol_language())
        .or_else(|| SymbolLanguage::from_path(file_path));

    let symbol_lang = symbol_lang.ok_or_else(|| splice::SpliceError::Parse {
        file: file_path.to_path_buf(),
        message: "Cannot detect language - unknown file extension".to_string(),
    })?;

    // Step 1: Read source file
    let source = std::fs::read(file_path).map_err(|source| splice::SpliceError::Io {
        path: file_path.to_path_buf(),
        source,
    })?;

    // Step 2: Extract symbols using language-aware dispatcher
    let symbols = extract_symbols_with_language(file_path, &source, symbol_lang)?;

    // Step 3: Create in-memory graph (for reference finding API compatibility)
    let graph_db_path = file_path
        .parent()
        .ok_or_else(|| {
            splice::SpliceError::Other(format!("File path has no parent: {}", file_path.display()))
        })?
        .join(".splice_graph.db");
    let mut code_graph = CodeGraph::open(&graph_db_path)?;

    // Step 4: Store symbols in graph with language metadata and line/col
    for symbol in &symbols {
        code_graph.store_symbol_with_file_and_language(
            file_path,
            symbol.name(),
            symbol.kind(),
            symbol.language(),
            symbol.byte_start(),
            symbol.byte_end(),
            symbol.line_start(),
            symbol.line_end(),
            symbol.col_start(),
            symbol.col_end(),
        )?;
    }

    // Step 5: Convert CLI kind to string for resolution
    // Note: This will be used in Phase 4 for multi-language reference finding
    let _kind_str = kind.map(|k| match k {
        splice::cli::SymbolKind::Function => "function",
        splice::cli::SymbolKind::Method => "method",
        splice::cli::SymbolKind::Class => "class",
        splice::cli::SymbolKind::Struct => "struct",
        splice::cli::SymbolKind::Interface => "interface",
        splice::cli::SymbolKind::Enum => "enum",
        splice::cli::SymbolKind::Trait => "trait",
        splice::cli::SymbolKind::Impl => "impl",
        splice::cli::SymbolKind::Module => "module",
        splice::cli::SymbolKind::Variable => "variable",
        splice::cli::SymbolKind::Constructor => "constructor",
        splice::cli::SymbolKind::TypeAlias => "type_alias",
    });

    // Step 6: Find all references to the symbol
    // Note: Reference finding is still Rust-only (Phase 4 will add multi-language)
    let ref_set = find_references(&code_graph, file_path, symbol_name, None)?;

    // Step 7: Determine workspace directory (parent of source file)
    let workspace_dir = file_path.parent().ok_or_else(|| {
        splice::SpliceError::Other("Cannot determine workspace directory".to_string())
    })?;

    // Step 8: Convert CLI analyzer mode to validate analyzer mode (default to Off)
    let analyzer_mode = match analyzer {
        Some(splice::cli::AnalyzerMode::Off) => ValidateAnalyzerMode::Off,
        Some(splice::cli::AnalyzerMode::Os) => ValidateAnalyzerMode::Path,
        Some(splice::cli::AnalyzerMode::Path) => {
            if let Some(binary) = analyzer_binary {
                ValidateAnalyzerMode::Explicit(binary.to_string_lossy().to_string())
            } else {
                ValidateAnalyzerMode::Path
            }
        }
        None => ValidateAnalyzerMode::Off,
    };

    // Step 9: Group references by file and sort by byte offset (descending for deletion)
    let mut refs_by_file: HashMap<String, Vec<&splice::resolve::references::Reference>> =
        HashMap::new();
    for r in &ref_set.references {
        refs_by_file.entry(r.file_path.clone()).or_default().push(r);
    }

    // Sort each file's references by byte offset descending
    for refs in refs_by_file.values_mut() {
        refs.sort_by_key(|r| std::cmp::Reverse(r.byte_start));
    }

    // Step 10: Dry-run mode - preview what would be deleted
    if dry_run {
        // Read original file content
        let replaced_content =
            std::fs::read_to_string(file_path).map_err(|source| splice::SpliceError::Io {
                path: file_path.to_path_buf(),
                source,
            })?;

        // Simulate deletion by removing the span using ropey
        let mut rope = Rope::from_str(&replaced_content);
        let def = &ref_set.definition;
        let start_char = rope.byte_to_char(def.byte_start);
        let end_char = rope.byte_to_char(def.byte_end);
        rope.remove(start_char..end_char);
        let after_content = rope.to_string();

        // Count lines removed
        let lines_removed = if def.byte_end > def.byte_start {
            replaced_content[def.byte_start..def.byte_end]
                .lines()
                .count()
        } else {
            0
        };

        // Print summary header in git-style format
        let summary_header = format_diff_summary(1, 0, lines_removed);
        if !summary_header.is_empty() {
            println!("{}", summary_header);
        }

        // Print empty line separator
        println!();

        // Print unified diff with colors (unless JSON mode)
        let use_color = !json_output && should_use_color();
        let diff_output = if use_color {
            format_colored_diff(&replaced_content, &after_content, true)
        } else {
            format_unified_diff(
                &replaced_content,
                &after_content,
                &file_path.to_string_lossy(),
                unified,
            )
        };

        if !diff_output.is_empty() {
            print!("{}", diff_output);
        }

        let message = format!("Previewed deletion of '{}' (dry-run)", symbol_name,);

        // Record execution for dry-run
        let duration_ms = start.elapsed().as_millis() as i64;
        let parameters = serde_json::json!({
            "file": file_path.to_string_lossy(),
            "symbol": symbol_name,
            "kind": _kind_str,
            "create_backup": false,
            "dry_run": true,
        });
        if let Err(e) = log::record_execution_with_params(
            &splice::output::OperationResult::with_execution_id(
                "delete".to_string(),
                operation_id.clone(),
            )
            .success(message.clone()),
            duration_ms,
            Some(command_line),
            parameters,
        ) {
            log_execution_error("delete (dry-run)", &e);
        }

        // Mark as having pending changes if lines would be removed (git diff exit code convention)
        let has_changes = lines_removed > 0;
        let mut payload = splice::cli::CliSuccessPayload::message_only(message).already_emitted();
        if has_changes {
            payload = payload.with_pending_changes();
        }
        return Ok(payload);
    }

    // Step 11: Create backup if requested
    let backup_manifest_path = if create_backup {
        use splice::patch::BackupWriter;

        let workspace_root = find_workspace_root(file_path)?;
        let mut backup_writer = BackupWriter::new(&workspace_root, operation_id.clone())?;

        // Backup the file containing the definition
        backup_writer.backup_file(file_path)?;

        // Backup all files that contain references
        for file_path_str in refs_by_file.keys() {
            let path = Path::new(file_path_str);
            if path != file_path {
                backup_writer.backup_file(path)?;
            }
        }

        Some(backup_writer.finalize()?)
    } else {
        None
    };

    // Step 12: Delete references from each file
    let mut deleted_count = 0;
    let mut files_modified = Vec::new();

    for (file_path_str, refs) in refs_by_file {
        let path = Path::new(&file_path_str);

        // Detect language for this file
        let file_lang = SymbolLanguage::from_path(path).unwrap_or(symbol_lang);

        // Delete each reference in this file (highest byte offset first)
        for r in refs {
            apply_patch_with_validation(
                path,
                r.byte_start,
                r.byte_end,
                "", // Delete = replace with empty
                workspace_dir,
                file_lang,
                analyzer_mode.clone(),
                strict,
                skip_pre_verify,
            )?;
            deleted_count += 1;
        }

        files_modified.push(file_path_str);
    }

    // Step 11: Delete the definition itself
    let def = &ref_set.definition;
    apply_patch_with_validation(
        file_path,
        def.byte_start,
        def.byte_end,
        "", // Delete = replace with empty
        workspace_dir,
        symbol_lang,
        analyzer_mode.clone(),
        strict,
        skip_pre_verify,
    )?;
    deleted_count += 1;

    // Track the definition file as modified
    let def_file_path = file_path.to_str().unwrap_or("").to_string();
    if !files_modified.contains(&def_file_path) {
        files_modified.push(def_file_path);
    }

    // Step 12: Return success message
    let base_message = if ref_set.has_glob_ambiguity {
        format!(
            "Deleted '{}' ({} references + definition) across {} file(s). WARNING: glob imports detected - some references may have been missed.",
            symbol_name,
            deleted_count - 1,
            files_modified.len()
        )
    } else {
        format!(
            "Deleted '{}' ({} references + definition) across {} file(s).",
            symbol_name,
            deleted_count - 1,
            files_modified.len()
        )
    };

    // Collect span IDs (byte ranges) for all deleted spans
    let mut span_ids: Vec<serde_json::Value> = Vec::new();
    for r in &ref_set.references {
        span_ids.push(json!({
            "file": r.file_path,
            "byte_start": r.byte_start,
            "byte_end": r.byte_end,
        }));
    }
    // Add definition span
    span_ids.push(json!({
        "file": file_path.to_string_lossy(),
        "byte_start": def.byte_start,
        "byte_end": def.byte_end,
    }));

    // Build response data
    let mut response_data = serde_json::Map::new();
    if let Some(manifest_path) = backup_manifest_path {
        response_data.insert(
            "backup_manifest".to_string(),
            json!(manifest_path.to_string_lossy()),
        );
    }
    if let Some(ref op_id) = operation_id {
        response_data.insert("operation_id".to_string(), json!(op_id));
    }
    if let Some(meta) = metadata {
        // Try to parse as JSON, if fails include as string
        if let Ok(parsed) = serde_json::from_str::<Value>(&meta) {
            response_data.insert("metadata".to_string(), parsed);
        } else {
            response_data.insert("metadata".to_string(), json!(meta));
        }
    }
    response_data.insert("span_ids".to_string(), json!(span_ids));
    response_data.insert("files_modified".to_string(), json!(files_modified));

    // Record execution for regular output
    let duration_ms = start.elapsed().as_millis() as i64;
    let parameters = serde_json::json!({
        "file": file_path.to_string_lossy(),
        "symbol": symbol_name,
        "kind": _kind_str,
        "create_backup": create_backup,
    });
    if let Err(e) = log::record_execution_with_params(
        &splice::output::OperationResult::with_execution_id(
            "delete".to_string(),
            operation_id.clone(),
        )
        .success(base_message.clone()),
        duration_ms,
        Some(command_line.clone()),
        parameters,
    ) {
        log_execution_error("delete", &e);
    }

    // Check if JSON output is requested
    if json_output {
        use splice::action::SuggestedAction;
        use splice::action::{ActionType, Confidence};
        use splice::checksum;
        use splice::context;
        use splice::hints::{derive_tool_hints, ToolHintOperation};
        use splice::ingest::semantic_kind::SemanticKind;
        use splice::ingest::{detect as ingest_detect, dispatch};
        use splice::output::{DeleteResult, OperationData, OperationResult, SpanResult};
        use splice::resolve::resolve_symbol;
        use splice::symbol::AnySymbol;
        use std::path::Path;

        // Resolve the definition to get match_id
        let resolved_def = resolve_symbol(&code_graph, Some(file_path), _kind_str, symbol_name)?;

        // Detect language for semantic kind detection
        let detected_language = ingest_detect::detect_language(file_path);

        // Read file for semantic kind detection
        let file_contents = std::fs::read(file_path).unwrap_or_default();

        // Create span results for all deleted spans with rich metadata
        let mut spans: Vec<SpanResult> = Vec::new();

        // Add definition span with rich metadata
        let mut def_span = SpanResult::from(resolved_def.clone());

        // Extract context for definition span
        if let Ok(ctx) = context::extract_context_asymmetric(
            file_path,
            def.byte_start,
            def.byte_end,
            ctx_before,
            ctx_after,
        ) {
            def_span = def_span.with_context(ctx);
        }

        // Add semantic kind and language if available
        if let Some(lang) = detected_language {
            // Try to detect semantic kind from tree-sitter parse
            let sem_kind_str = if let Ok(symbols) =
                dispatch::extract_symbols(file_path, &file_contents)
            {
                // Find the symbol that matches our definition
                symbols
                    .iter()
                    .find(|s| s.byte_start() == def.byte_start && s.byte_end() == def.byte_end)
                    .map(|s| {
                        // Map symbol kind to semantic kind string
                        match s {
                            AnySymbol::Rust(rust_sym) => match rust_sym.kind {
                                splice::ingest::rust::RustSymbolKind::Function => "function",
                                splice::ingest::rust::RustSymbolKind::Struct => "type",
                                splice::ingest::rust::RustSymbolKind::Enum => "enum",
                                splice::ingest::rust::RustSymbolKind::Trait => "trait",
                                splice::ingest::rust::RustSymbolKind::Impl => "trait",
                                splice::ingest::rust::RustSymbolKind::Module => "module",
                                splice::ingest::rust::RustSymbolKind::TypeAlias => "type_alias",
                                _ => "unknown",
                            },
                            AnySymbol::Python(py_sym) => match py_sym.kind {
                                splice::ingest::python::PythonSymbolKind::Function => "function",
                                splice::ingest::python::PythonSymbolKind::Class => "type",
                                splice::ingest::python::PythonSymbolKind::Method => "function",
                                _ => "unknown",
                            },
                            AnySymbol::Java(java_sym) => match java_sym.kind {
                                splice::ingest::java::JavaSymbolKind::Class => "type",
                                splice::ingest::java::JavaSymbolKind::Method => "function",
                                splice::ingest::java::JavaSymbolKind::Interface => "trait",
                                splice::ingest::java::JavaSymbolKind::Enum => "enum",
                                _ => "unknown",
                            },
                            AnySymbol::JavaScript(js_sym) => match js_sym.kind {
                                splice::ingest::javascript::JavaScriptSymbolKind::Function => {
                                    "function"
                                }
                                splice::ingest::javascript::JavaScriptSymbolKind::Class => "type",
                                splice::ingest::javascript::JavaScriptSymbolKind::Method => {
                                    "function"
                                }
                                _ => "unknown",
                            },
                            AnySymbol::TypeScript(ts_sym) => match ts_sym.kind {
                                splice::ingest::typescript::TypeScriptSymbolKind::Function => {
                                    "function"
                                }
                                splice::ingest::typescript::TypeScriptSymbolKind::Class => "type",
                                splice::ingest::typescript::TypeScriptSymbolKind::Method => {
                                    "function"
                                }
                                splice::ingest::typescript::TypeScriptSymbolKind::Interface => {
                                    "trait"
                                }
                                _ => "unknown",
                            },
                            AnySymbol::Cpp(cpp_sym) => match cpp_sym.kind {
                                splice::ingest::cpp::CppSymbolKind::Class => "type",
                                splice::ingest::cpp::CppSymbolKind::Struct => "type",
                                splice::ingest::cpp::CppSymbolKind::Function => "function",
                                splice::ingest::cpp::CppSymbolKind::Method => "function",
                                _ => "unknown",
                            },
                        }
                    })
                    .unwrap_or("unknown")
            } else {
                "unknown"
            };

            def_span = def_span.with_semantic_info(sem_kind_str, lang.as_str());

            // Infer SemanticKind from the semantic kind string
            let sem_kind = match sem_kind_str {
                "function" => SemanticKind::Function,
                "type" => SemanticKind::Type,
                "trait" => SemanticKind::Trait,
                "enum" => SemanticKind::Enum,
                "module" => SemanticKind::Module,
                "type_alias" => SemanticKind::TypeAlias,
                "constant" => SemanticKind::Constant,
                _ => SemanticKind::Unknown,
            };

            // Infer is_public from semantic kind (default to true for functions, types, traits, enums)
            let is_public = matches!(
                sem_kind,
                SemanticKind::Function
                    | SemanticKind::Type
                    | SemanticKind::Trait
                    | SemanticKind::Enum
            );

            // Derive tool hints for delete operation
            let hints = derive_tool_hints(sem_kind, is_public, ToolHintOperation::DeleteBody);
            def_span = def_span.with_tool_hints(hints);

            // Determine confidence based on whether there are callers
            let has_callers = !ref_set.references.is_empty();
            let confidence = if has_callers {
                Confidence::Medium
            } else {
                Confidence::High
            };

            // Generate suggested action for delete
            let reason = if has_callers {
                format!(
                    "Delete symbol '{}' ({}) at {} - has {} callers, may break dependencies",
                    symbol_name,
                    sem_kind_str,
                    file_path.to_string_lossy(),
                    ref_set.references.len()
                )
            } else {
                format!(
                    "Delete symbol '{}' ({}) at {} - safe to delete, no callers",
                    symbol_name,
                    sem_kind_str,
                    file_path.to_string_lossy()
                )
            };

            let action = SuggestedAction {
                action_type: ActionType::Delete,
                confidence,
                reason,
                params: {
                    let mut p = std::collections::HashMap::new();
                    p.insert(
                        "remove_references".to_string(),
                        serde_json::Value::Bool(true),
                    );
                    Some(p)
                },
            };
            def_span = def_span.with_suggested_action(action);
        }

        // Add checksums for definition span
        if let Ok(cs) = checksum::checksum_span(file_path, def.byte_start, def.byte_end) {
            def_span = def_span.with_checksum_before(cs.value);
        }
        if let Ok(file_cs) = checksum::checksum_file(file_path) {
            def_span = def_span.with_file_checksum_before(file_cs.value);
        }

        // Query relationships if flag is set
        if relationships {
            use splice::relationships::{
                get_callees, get_callers, get_exports, get_imports, RelationshipCache,
                Relationships,
            };
            use sqlitegraph::NodeId;

            let mut cache = RelationshipCache::new();
            let node_id = NodeId::from(resolved_def.node_id.as_i64());

            let callers = get_callers(&code_graph, node_id, &mut cache).unwrap_or_default();
            let callees = get_callees(&code_graph, node_id, &mut cache).unwrap_or_default();
            let imports = get_imports(&code_graph, file_path, &mut cache).unwrap_or_default();
            let exports = get_exports(&code_graph, file_path, &mut cache).unwrap_or_default();

            let rels = Relationships {
                callers,
                callees,
                imports,
                exports,
                cycle_detected: false,
                error_code: None,
            };
            def_span = def_span.with_relationships(rels);
        }

        spans.push(def_span);

        // Add reference spans with rich metadata
        for r in &ref_set.references {
            let ref_path = Path::new(&r.file_path);
            let mut ref_span =
                SpanResult::from_byte_span(r.file_path.clone(), r.byte_start, r.byte_end);

            // Extract context for reference span
            if let Ok(ctx) = context::extract_context_asymmetric(
                ref_path,
                r.byte_start,
                r.byte_end,
                ctx_before,
                ctx_after,
            ) {
                ref_span = ref_span.with_context(ctx);
            }

            // Detect language and semantic kind for reference
            if let Some(ref_lang) = ingest_detect::detect_language(ref_path) {
                // For references, we use a generic semantic kind
                ref_span = ref_span.with_semantic_info("reference", ref_lang.as_str());
            }

            // Add checksums for reference span
            if let Ok(cs) = checksum::checksum_span(ref_path, r.byte_start, r.byte_end) {
                ref_span = ref_span.with_checksum_before(cs.value);
            }
            if let Ok(file_cs) = checksum::checksum_file(ref_path) {
                ref_span = ref_span.with_file_checksum_before(file_cs.value);
            }

            spans.push(ref_span);
        }

        // Sort spans deterministically by file_path, then byte_start
        spans.sort();

        // Calculate total bytes removed
        let total_bytes_removed: usize = ref_set
            .references
            .iter()
            .map(|r| r.byte_end - r.byte_start)
            .sum::<usize>()
            + (def.byte_end - def.byte_start);

        // Calculate total lines removed
        let total_lines_removed: usize = {
            // Count lines in definition
            let def_lines = if def.byte_end > def.byte_start {
                count_lines_in_span(file_path, def.byte_start, def.byte_end)
            } else {
                0
            };

            // Count lines in each reference
            let ref_lines: usize = ref_set
                .references
                .iter()
                .map(|r| {
                    if r.byte_end > r.byte_start {
                        count_lines_in_span(Path::new(&r.file_path), r.byte_start, r.byte_end)
                    } else {
                        0
                    }
                })
                .sum();

            def_lines + ref_lines
        };

        // Compute file checksum before deletion
        let file_checksum_before = checksum::checksum_file(file_path)
            .map(|cs| cs.value)
            .unwrap_or_else(|_| "checksum-failed".to_string());

        // Compute checksums for each removed span (for legacy field)
        let mut span_checksums: Vec<String> = Vec::new();

        // Add definition span checksum
        if let Ok(cs) = checksum::checksum_span(file_path, def.byte_start, def.byte_end) {
            span_checksums.push(cs.value);
        }

        // Add reference span checksums
        for r in &ref_set.references {
            if let Ok(cs) =
                checksum::checksum_span(Path::new(&r.file_path), r.byte_start, r.byte_end)
            {
                span_checksums.push(cs.value);
            }
        }

        // Create delete result
        let delete_result = DeleteResult {
            file: file_path.to_string_lossy().to_string(),
            symbol: symbol_name.to_string(),
            kind: _kind_str.unwrap_or("unknown").to_string(),
            spans,
            bytes_removed: total_bytes_removed,
            lines_removed: total_lines_removed,
            references_removed: deleted_count - 1,
            file_checksum_before,
            span_checksums,
        };

        // Create operation result with operation_id from CLI or generate new UUID
        let result = OperationResult::with_execution_id("delete".to_string(), operation_id.clone())
            .success(base_message.clone())
            .with_result(OperationData::Delete(delete_result));

        // Record execution
        let duration_ms = start.elapsed().as_millis() as i64;
        let parameters = serde_json::json!({
            "file": file_path.to_string_lossy(),
            "symbol": symbol_name,
            "kind": _kind_str,
            "create_backup": create_backup,
        });
        if let Err(e) = log::record_execution_with_params(
            &result,
            duration_ms,
            Some(command_line.clone()),
            parameters,
        ) {
            log_execution_error("delete", &e);
        }

        // Output structured JSON directly
        println!("{}", serde_json::to_string_pretty(&result).unwrap());

        // Return a dummy payload marked as already emitted
        return Ok(
            splice::cli::CliSuccessPayload::message_only("OK".to_string()).already_emitted(),
        );
    }

    Ok(splice::cli::CliSuccessPayload::with_data(
        base_message,
        serde_json::Value::Object(response_data),
    ))
}

/// Execute the patch command.
///
/// This function is a thin adapter that:
/// 1. Extracts symbols from source file using language-aware dispatcher
/// 2. Resolves the target symbol to its byte span
/// 3. Reads replacement content from file
/// 4. Applies patch with validation gates
///
/// All logic is delegated to existing APIs.
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
fn execute_single_patch(
    file_path: Option<PathBuf>,
    symbol_name: Option<String>,
    kind: Option<splice::cli::SymbolKind>,
    analyzer: Option<splice::cli::AnalyzerMode>,
    analyzer_binary: Option<std::path::PathBuf>,
    replacement_file: Option<PathBuf>,
    language: Option<splice::cli::Language>,
    context_before: usize,
    context_after: usize,
    context_both: usize,
    preview: bool,
    unified: usize,
    create_backup: bool,
    relationships: bool,
    operation_id: Option<String>,
    metadata: Option<String>,
    db: Option<PathBuf>,
    snapshot_before: bool,
    impact_graph: bool,
    json_output: bool,
    strict: bool,
    skip_pre_verify: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    let file_path = require_patch_arg("--file", file_path)?;
    let symbol_name = require_patch_arg("--symbol", symbol_name)?;
    let replacement_file = require_patch_arg("--with", replacement_file)?;

    execute_patch(
        &file_path,
        &symbol_name,
        kind,
        analyzer,
        analyzer_binary,
        &replacement_file,
        language,
        context_before,
        context_after,
        context_both,
        preview,
        unified,
        create_backup,
        relationships,
        operation_id,
        metadata,
        db,
        snapshot_before,
        impact_graph,
        json_output,
        strict,
        skip_pre_verify,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
fn execute_patch(
    file_path: &Path,
    symbol_name: &str,
    kind: Option<splice::cli::SymbolKind>,
    analyzer: Option<splice::cli::AnalyzerMode>,
    analyzer_binary: Option<std::path::PathBuf>,
    replacement_file: &Path,
    language: Option<splice::cli::Language>,
    context_before: usize,
    context_after: usize,
    context_both: usize,
    preview: bool,
    unified: usize,
    create_backup: bool,
    relationships: bool,
    operation_id: Option<String>,
    metadata: Option<String>,
    db: Option<PathBuf>,
    snapshot_before: bool,
    impact_graph: bool,
    json_output: bool,
    strict: bool,
    skip_pre_verify: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::execution::log;
    use splice::format_colored_diff;
    use splice::format_diff_summary;
    use splice::format_unified_diff;
    use splice::graph::CodeGraph;
    use splice::patch::{
        apply_patch_with_validation, compute_preview_report, preview_patch_with_content,
        FilePatchSummary,
    };
    use splice::resolve::resolve_symbol;
    use splice::should_use_color;
    use splice::symbol::{Language as SymbolLanguage, Symbol};
    use splice::validate::AnalyzerMode as ValidateAnalyzerMode;

    // Resolve context counts from -A/-B/-C flags
    let (ctx_before, ctx_after) =
        splice::resolve_context_counts(context_before, context_after, context_both);

    // Capture snapshot before operation if requested
    if snapshot_before {
        if let Some(db_path) = &db {
            if let Err(e) = capture_snapshot(db_path, "patch") {
                eprintln!("Warning: Failed to capture snapshot: {}", e);
            }
        } else {
            eprintln!("Warning: --snapshot-before requires --db flag for snapshot capture");
        }
    }

    // Start timing
    let start = std::time::Instant::now();
    let command_line = std::env::args().collect::<Vec<_>>().join(" ");

    // Determine language (from CLI flag or auto-detect from file extension)
    let symbol_lang = language
        .map(|l| l.to_symbol_language())
        .or_else(|| SymbolLanguage::from_path(file_path));

    let symbol_lang = symbol_lang.ok_or_else(|| splice::SpliceError::Parse {
        file: file_path.to_path_buf(),
        message: "Cannot detect language - unknown file extension".to_string(),
    })?;

    // Step 1: Read source file
    let source = std::fs::read(file_path).map_err(|source| splice::SpliceError::Io {
        path: file_path.to_path_buf(),
        source,
    })?;

    // Step 2: Extract symbols using language-aware dispatcher
    let symbols = extract_symbols_with_language(file_path, &source, symbol_lang)?;

    // Normalize path so graph store and lookup use the same absolute path.
    // resolve_symbol normalizes internally; without this, symbols stored with
    // a relative path are invisible to an absolute-path lookup.
    let file_path_buf = splice::resolve::normalize_lookup_path(file_path);
    let file_path = file_path_buf.as_path();

    // Step 3: Use provided db path, or create temp db in file's parent directory
    let graph_db_path = if let Some(db_path) = db {
        db_path
    } else {
        // Fallback: create .splice_graph.db in the file's parent directory
        file_path
            .parent()
            .ok_or_else(|| {
                splice::SpliceError::Other(format!(
                    "File path has no parent: {}",
                    file_path.display()
                ))
            })?
            .join(".splice_graph.db")
    };
    let mut code_graph = CodeGraph::open(&graph_db_path)?;

    // Step 4: Store symbols in graph with language metadata and line/col
    for symbol in &symbols {
        code_graph.store_symbol_with_file_and_language(
            file_path,
            symbol.name(),
            symbol.kind(),
            symbol.language(),
            symbol.byte_start(),
            symbol.byte_end(),
            symbol.line_start(),
            symbol.line_end(),
            symbol.col_start(),
            symbol.col_end(),
        )?;
    }

    // Step 5: Convert CLI kind to string for resolution
    let kind_str = kind.map(|k| match k {
        splice::cli::SymbolKind::Function => "function",
        splice::cli::SymbolKind::Method => "method",
        splice::cli::SymbolKind::Class => "class",
        splice::cli::SymbolKind::Struct => "struct",
        splice::cli::SymbolKind::Interface => "interface",
        splice::cli::SymbolKind::Enum => "enum",
        splice::cli::SymbolKind::Trait => "trait",
        splice::cli::SymbolKind::Impl => "impl",
        splice::cli::SymbolKind::Module => "module",
        splice::cli::SymbolKind::Variable => "variable",
        splice::cli::SymbolKind::Constructor => "constructor",
        splice::cli::SymbolKind::TypeAlias => "type_alias",
    });

    // Step 6: Resolve symbol to span
    let resolved = resolve_symbol(&code_graph, Some(file_path), kind_str, symbol_name)?;

    // Step 7: Read replacement content
    let replacement_content =
        std::fs::read_to_string(replacement_file).map_err(|source| splice::SpliceError::Io {
            path: replacement_file.to_path_buf(),
            source,
        })?;

    // Step 8: Determine workspace directory (parent of source file)
    let workspace_dir = file_path.parent().ok_or_else(|| {
        splice::SpliceError::Other("Cannot determine workspace directory".to_string())
    })?;
    let workspace_root = find_workspace_root(file_path)?;

    // Step 9: Convert CLI analyzer mode to validate analyzer mode (default to Off)
    let analyzer_mode = match analyzer {
        Some(splice::cli::AnalyzerMode::Off) => ValidateAnalyzerMode::Off,
        Some(splice::cli::AnalyzerMode::Os) => ValidateAnalyzerMode::Path,
        Some(splice::cli::AnalyzerMode::Path) => {
            if let Some(binary) = analyzer_binary {
                ValidateAnalyzerMode::Explicit(binary.to_string_lossy().to_string())
            } else {
                ValidateAnalyzerMode::Path
            }
        }
        None => ValidateAnalyzerMode::Off,
    };

    // Step 10: Create backup if requested (skip for preview mode)
    let backup_manifest_path = if create_backup && !preview {
        use splice::patch::BackupWriter;

        let mut backup_writer = BackupWriter::new(&workspace_root, operation_id.clone())?;
        backup_writer.backup_file(file_path)?;
        Some(backup_writer.finalize()?)
    } else {
        None
    };

    // Generate impact graph if requested (in preview mode only)
    if impact_graph {
        use splice::cli::ReachabilityDirection;
        use splice::graph::magellan_integration::{ImpactDotConfig, MagellanIntegration};

        // Open Magellan database for impact graph generation
        let mut magellan = MagellanIntegration::open(&graph_db_path)?;

        // Generate symbol_id for impact graph
        let symbol_id = format!("{}:{}", file_path.display(), symbol_name);

        let config = ImpactDotConfig {
            show_symbol_kinds: true,
            max_depth: Some(10),
            highlight_symbol: Some(symbol_name.to_string()),
        };

        let dot =
            magellan.generate_impact_dot(&symbol_id, &ReachabilityDirection::Both, &config)?;
        println!("{}", dot);

        // Return early - impact graph is the complete output
        return Ok(splice::cli::CliSuccessPayload::message_only(
            "Impact graph generated".to_string(),
        )
        .already_emitted());
    }

    if preview {
        // Dry-run mode: show unified diff with summary header
        let (_summary, report, before_content, after_content) = preview_patch_with_content(
            file_path,
            resolved.byte_start,
            resolved.byte_end,
            &replacement_content,
            &workspace_root,
            symbol_lang,
            analyzer_mode,
        )?;

        // Check if JSON output is requested for preview
        if json_output {
            use std::collections::HashMap;

            // Build data object using consistent pattern with build_success_payload
            let mut data_map: HashMap<String, serde_json::Value> = HashMap::new();

            // Add symbol at top level (not part of PreviewReport struct)
            data_map.insert("symbol".to_string(), serde_json::json!(symbol_name));

            // Add preview_report using the full struct (consistent with other commands)
            data_map.insert(
                "preview_report".to_string(),
                serde_json::to_value(&report).expect("preview report should serialize"),
            );

            // Add files array
            data_map.insert(
                "files".to_string(),
                serde_json::json!([{
                    "file": file_path.to_string_lossy().to_string(),
                }]),
            );

            let message = format!(
                "Previewed patch '{}' at bytes {}..{} (dry-run)",
                symbol_name, resolved.byte_start, resolved.byte_end,
            );

            // Record execution for preview (without full OperationResult)
            let duration_ms = start.elapsed().as_millis() as i64;
            let parameters = serde_json::json!({
                "file": file_path.to_string_lossy(),
                "symbol": symbol_name,
                "kind": kind_str,
                "preview": true,
                "create_backup": create_backup,
                "dry_run": true,
            });
            // Use a simple operation result for logging only
            use splice::output::OperationResult;
            let log_result =
                OperationResult::with_execution_id("patch".to_string(), operation_id.clone())
                    .success(message.clone());
            if let Err(e) = log::record_execution_with_params(
                &log_result,
                duration_ms,
                Some(command_line),
                parameters,
            ) {
                log_execution_error("patch (preview)", &e);
            }

            // Output JSON using CliSuccessPayload pattern (same as find, refs, etc.)
            return Ok(splice::cli::CliSuccessPayload::with_data(
                message,
                serde_json::Value::Object(data_map.into_iter().collect()),
            ));
        }

        // Non-JSON preview: print unified diff
        // Print summary header in git-style format
        let summary_header = format_diff_summary(1, report.lines_added, report.lines_removed);
        if !summary_header.is_empty() {
            println!("{}", summary_header);
        }

        // Print empty line separator
        println!();

        // Print unified diff with colors
        let diff_output = if should_use_color() {
            format_colored_diff(&before_content, &after_content, true)
        } else {
            format_unified_diff(
                &before_content,
                &after_content,
                &file_path.to_string_lossy(),
                unified,
            )
        };

        if !diff_output.is_empty() {
            print!("{}", diff_output);
        }

        let message = format!(
            "Previewed patch '{}' at bytes {}..{} (dry-run)",
            symbol_name, resolved.byte_start, resolved.byte_end,
        );

        // Record execution for preview
        let duration_ms = start.elapsed().as_millis() as i64;
        let parameters = serde_json::json!({
            "file": file_path.to_string_lossy(),
            "symbol": symbol_name,
            "kind": kind_str,
            "preview": true,
            "create_backup": create_backup,
            "dry_run": true,
        });
        if let Err(e) = log::record_execution_with_params(
            &splice::output::OperationResult::with_execution_id(
                "patch".to_string(),
                operation_id.clone(),
            )
            .success(message.clone()),
            duration_ms,
            Some(command_line),
            parameters,
        ) {
            log_execution_error("patch (dry-run)", &e);
        }

        // Mark as having pending changes if lines were added or removed (git diff exit code convention)
        let has_changes = report.lines_added > 0 || report.lines_removed > 0;
        let mut payload = splice::cli::CliSuccessPayload::message_only(message).already_emitted();
        if has_changes {
            payload = payload.with_pending_changes();
        }
        return Ok(payload);
    }

    let (before_hash, after_hash) = apply_patch_with_validation(
        file_path,
        resolved.byte_start,
        resolved.byte_end,
        &replacement_content,
        workspace_dir,
        symbol_lang,
        analyzer_mode,
        strict,
        skip_pre_verify,
    )?;

    // Calculate line counts for the patch
    let line_report = compute_preview_report(
        file_path,
        resolved.byte_start,
        resolved.byte_end,
        &replacement_content,
    )?;

    let summary = FilePatchSummary {
        file: file_path.to_path_buf(),
        before_hash,
        after_hash,
    };

    // Check if JSON output is requested
    if json_output {
        use splice::action::SuggestedAction;
        use splice::action::{ActionType, Confidence};
        use splice::checksum;
        use splice::context;
        use splice::hints::{derive_tool_hints, ToolHintOperation};
        use splice::ingest::semantic_kind::SemanticKind;
        use splice::ingest::{detect as ingest_detect, dispatch};
        use splice::output::{OperationData, OperationResult, PatchResult, SpanResult};
        use splice::symbol::AnySymbol;

        // Detect language for semantic kind detection
        let detected_language = ingest_detect::detect_language(file_path);

        // Read file for semantic kind detection
        let file_contents = std::fs::read(file_path).unwrap_or_default();

        // Compute span checksums before and after
        let span_checksum_before =
            checksum::checksum_span(file_path, resolved.byte_start, resolved.byte_end)
                .map(|cs| cs.value)
                .unwrap_or_else(|_| "checksum-failed".to_string());

        // After checksum: read the patched span
        // Note: This is approximate since the span may have changed size
        let span_checksum_after = if let Ok(after_cs) =
            checksum::checksum_span(file_path, resolved.byte_start, resolved.byte_end)
        {
            after_cs.value
        } else {
            "checksum-failed".to_string()
        };

        // Create span result with rich metadata
        let mut span = SpanResult::from(resolved.clone())
            .with_hashes(summary.before_hash.clone(), summary.after_hash.clone())
            .with_span_checksums(span_checksum_before.clone(), span_checksum_after);

        // Extract context for the span
        if let Ok(ctx) = context::extract_context_asymmetric(
            file_path,
            resolved.byte_start,
            resolved.byte_end,
            ctx_before,
            ctx_after,
        ) {
            span = span.with_context(ctx);
        }

        // Add semantic kind and language if available
        if let Some(lang) = detected_language {
            // Try to detect semantic kind from tree-sitter parse
            let sem_kind_str = if let Ok(symbols) =
                dispatch::extract_symbols(file_path, &file_contents)
            {
                // Find the symbol that matches our definition
                symbols
                    .iter()
                    .find(|s| {
                        s.byte_start() == resolved.byte_start && s.byte_end() == resolved.byte_end
                    })
                    .map(|s| {
                        // Map symbol kind to semantic kind string
                        match s {
                            AnySymbol::Rust(rust_sym) => match rust_sym.kind {
                                splice::ingest::rust::RustSymbolKind::Function => "function",
                                splice::ingest::rust::RustSymbolKind::Struct => "type",
                                splice::ingest::rust::RustSymbolKind::Enum => "enum",
                                splice::ingest::rust::RustSymbolKind::Trait => "trait",
                                splice::ingest::rust::RustSymbolKind::Impl => "trait",
                                splice::ingest::rust::RustSymbolKind::Module => "module",
                                splice::ingest::rust::RustSymbolKind::TypeAlias => "type_alias",
                                _ => "unknown",
                            },
                            AnySymbol::Python(py_sym) => match py_sym.kind {
                                splice::ingest::python::PythonSymbolKind::Function => "function",
                                splice::ingest::python::PythonSymbolKind::Class => "type",
                                splice::ingest::python::PythonSymbolKind::Method => "function",
                                _ => "unknown",
                            },
                            AnySymbol::Java(java_sym) => match java_sym.kind {
                                splice::ingest::java::JavaSymbolKind::Class => "type",
                                splice::ingest::java::JavaSymbolKind::Method => "function",
                                splice::ingest::java::JavaSymbolKind::Interface => "trait",
                                splice::ingest::java::JavaSymbolKind::Enum => "enum",
                                _ => "unknown",
                            },
                            AnySymbol::JavaScript(js_sym) => match js_sym.kind {
                                splice::ingest::javascript::JavaScriptSymbolKind::Function => {
                                    "function"
                                }
                                splice::ingest::javascript::JavaScriptSymbolKind::Class => "type",
                                splice::ingest::javascript::JavaScriptSymbolKind::Method => {
                                    "function"
                                }
                                _ => "unknown",
                            },
                            AnySymbol::TypeScript(ts_sym) => match ts_sym.kind {
                                splice::ingest::typescript::TypeScriptSymbolKind::Function => {
                                    "function"
                                }
                                splice::ingest::typescript::TypeScriptSymbolKind::Class => "type",
                                splice::ingest::typescript::TypeScriptSymbolKind::Method => {
                                    "function"
                                }
                                splice::ingest::typescript::TypeScriptSymbolKind::Interface => {
                                    "trait"
                                }
                                _ => "unknown",
                            },
                            AnySymbol::Cpp(cpp_sym) => match cpp_sym.kind {
                                splice::ingest::cpp::CppSymbolKind::Class => "type",
                                splice::ingest::cpp::CppSymbolKind::Struct => "type",
                                splice::ingest::cpp::CppSymbolKind::Function => "function",
                                splice::ingest::cpp::CppSymbolKind::Method => "function",
                                _ => "unknown",
                            },
                        }
                    })
                    .unwrap_or("unknown")
            } else {
                "unknown"
            };

            span = span.with_semantic_info(sem_kind_str, lang.as_str());

            // Infer SemanticKind from the semantic kind string
            let sem_kind = match sem_kind_str {
                "function" => SemanticKind::Function,
                "type" => SemanticKind::Type,
                "trait" => SemanticKind::Trait,
                "enum" => SemanticKind::Enum,
                "module" => SemanticKind::Module,
                "type_alias" => SemanticKind::TypeAlias,
                "constant" => SemanticKind::Constant,
                _ => SemanticKind::Unknown,
            };

            // Infer is_public from semantic kind (default to true for functions, types, traits, enums)
            let is_public = matches!(
                sem_kind,
                SemanticKind::Function
                    | SemanticKind::Type
                    | SemanticKind::Trait
                    | SemanticKind::Enum
            );

            // Derive tool hints for replace operation
            let hints = derive_tool_hints(sem_kind, is_public, ToolHintOperation::ReplaceBody);
            span = span.with_tool_hints(hints);

            // Determine confidence (High for unique symbols resolved successfully)
            let confidence = Confidence::High;

            // Generate suggested action for replace
            let reason = format!(
                "Replace symbol '{}' ({}) at {} with provided content",
                symbol_name,
                sem_kind_str,
                file_path.to_string_lossy()
            );

            let action = SuggestedAction {
                action_type: ActionType::Replace,
                confidence,
                reason,
                params: {
                    let mut p = std::collections::HashMap::new();
                    p.insert(
                        "preserve_signature".to_string(),
                        serde_json::Value::Bool(true),
                    );
                    Some(p)
                },
            };
            span = span.with_suggested_action(action);
        }

        // Add checksum_before and file_checksum_before
        if let Ok(file_cs) = checksum::checksum_file(file_path) {
            span = span.with_both_checksums(span_checksum_before, file_cs.value);
        }

        // Query relationships if flag is set
        if relationships {
            use splice::relationships::{
                get_callees, get_callers, get_exports, get_imports, RelationshipCache,
                Relationships,
            };
            use sqlitegraph::NodeId;

            let mut cache = RelationshipCache::new();
            let node_id = NodeId::from(resolved.node_id.as_i64());

            let callers = get_callers(&code_graph, node_id, &mut cache).unwrap_or_default();
            let callees = get_callees(&code_graph, node_id, &mut cache).unwrap_or_default();
            let imports = get_imports(&code_graph, file_path, &mut cache).unwrap_or_default();
            let exports = get_exports(&code_graph, file_path, &mut cache).unwrap_or_default();

            let rels = Relationships {
                callers,
                callees,
                imports,
                exports,
                cycle_detected: false,
                error_code: None,
            };
            span = span.with_relationships(rels);
        }

        // Create patch result
        let patch_result = PatchResult {
            file: file_path.to_string_lossy().to_string(),
            symbol: symbol_name.to_string(),
            kind: resolved.kind.to_string(),
            spans: vec![span],
            before_hash: summary.before_hash.clone(),
            after_hash: summary.after_hash.clone(),
            lines_added: line_report.lines_added,
            lines_removed: line_report.lines_removed,
        };

        let message = format!(
            "Patched '{}' at bytes {}..{} (hash: {} -> {})",
            symbol_name,
            resolved.byte_start,
            resolved.byte_end,
            summary.before_hash,
            summary.after_hash
        );

        // Create operation result with operation_id from CLI or generate new UUID
        let result = OperationResult::with_execution_id("patch".to_string(), operation_id.clone())
            .success(message.clone())
            .with_workspace(workspace_root.to_string_lossy().to_string())
            .with_result(OperationData::Patch(patch_result));

        // Record execution
        let duration_ms = start.elapsed().as_millis() as i64;
        let parameters = serde_json::json!({
            "file": file_path.to_string_lossy(),
            "symbol": symbol_name,
            "kind": kind_str,
            "preview": false,
            "create_backup": create_backup,
        });
        if let Err(e) = log::record_execution_with_params(
            &result,
            duration_ms,
            Some(command_line.clone()),
            parameters,
        ) {
            log_execution_error("patch", &e);
        }

        // Output structured JSON directly
        println!("{}", serde_json::to_string_pretty(&result).unwrap());

        // Return a dummy payload marked as already emitted
        return Ok(
            splice::cli::CliSuccessPayload::message_only("OK".to_string()).already_emitted(),
        );
    }

    // Default output (backward compatible)
    let message = format!(
        "Patched '{}' at bytes {}..{} (hash: {} -> {})",
        symbol_name,
        resolved.byte_start,
        resolved.byte_end,
        summary.before_hash,
        summary.after_hash
    );

    // Record execution
    let duration_ms = start.elapsed().as_millis() as i64;
    let parameters = serde_json::json!({
        "file": file_path.to_string_lossy(),
        "symbol": symbol_name,
        "kind": kind_str,
        "preview": false,
        "create_backup": create_backup,
    });
    if let Err(e) = log::record_execution_with_params(
        &splice::output::OperationResult::with_execution_id(
            "patch".to_string(),
            operation_id.clone(),
        )
        .success(message.clone()),
        duration_ms,
        Some(command_line),
        parameters,
    ) {
        log_execution_error("patch", &e);
    }

    // Build span ID
    let span_id = json!({
        "file": file_path.to_string_lossy(),
        "byte_start": resolved.byte_start,
        "byte_end": resolved.byte_end,
    });

    // Build response data
    let mut response_data = serde_json::Map::new();
    response_data.insert(
        "files".to_string(),
        json!([{
            "file": file_path.to_string_lossy(),
            "before_hash": summary.before_hash,
            "after_hash": summary.after_hash,
        }]),
    );
    response_data.insert("span_ids".to_string(), json!([span_id]));
    if let Some(manifest_path) = backup_manifest_path {
        response_data.insert(
            "backup_manifest".to_string(),
            json!(manifest_path.to_string_lossy()),
        );
    }
    if let Some(ref op_id) = operation_id {
        response_data.insert("operation_id".to_string(), json!(op_id));
    }
    if let Some(meta) = metadata {
        // Try to parse as JSON, if fails include as string
        if let Ok(parsed) = serde_json::from_str::<Value>(&meta) {
            response_data.insert("metadata".to_string(), parsed);
        } else {
            response_data.insert("metadata".to_string(), json!(meta));
        }
    }

    Ok(splice::cli::CliSuccessPayload::with_data(
        message,
        serde_json::Value::Object(response_data),
    ))
}

/// Execute a batch patch command driven by a JSON manifest.
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
fn execute_patch_batch(
    batch_path: &Path,
    analyzer: Option<splice::cli::AnalyzerMode>,
    analyzer_binary: Option<std::path::PathBuf>,
    language: Option<splice::cli::Language>,
    create_backup: bool,
    operation_id: Option<String>,
    metadata: Option<String>,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::execution::log;
    use splice::patch::{apply_batch_with_validation, load_batches_from_file};
    use splice::validate::AnalyzerMode as ValidateAnalyzerMode;

    // Start timing
    let start = std::time::Instant::now();
    let command_line = std::env::args().collect::<Vec<_>>().join(" ");

    let absolute_batch = if batch_path.is_absolute() {
        batch_path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|err| {
                splice::SpliceError::Other(format!("Failed to resolve current directory: {}", err))
            })?
            .join(batch_path)
    };

    let workspace_dir = absolute_batch.parent().ok_or_else(|| {
        splice::SpliceError::Other(
            "Cannot determine workspace directory from --batch path".to_string(),
        )
    })?;
    let workspace_dir = workspace_dir.to_path_buf();

    let symbol_language = language
        .ok_or_else(|| {
            splice::SpliceError::Other(
                "The --language flag is required when --batch is used".to_string(),
            )
        })?
        .to_symbol_language();

    let analyzer_mode = match analyzer {
        Some(splice::cli::AnalyzerMode::Off) => ValidateAnalyzerMode::Off,
        Some(splice::cli::AnalyzerMode::Os) => ValidateAnalyzerMode::Path,
        Some(splice::cli::AnalyzerMode::Path) => {
            if let Some(binary) = analyzer_binary {
                ValidateAnalyzerMode::Explicit(binary.to_string_lossy().to_string())
            } else {
                ValidateAnalyzerMode::Path
            }
        }
        None => ValidateAnalyzerMode::Off,
    };

    let batches = load_batches_from_file(&absolute_batch)?;
    let batch_count = batches.len();

    // Create backup if requested
    let backup_manifest_path = if create_backup {
        use splice::patch::BackupWriter;

        let workspace_root = find_workspace_root(&absolute_batch)?;

        // Collect all files that will be patched
        let mut files_to_backup: std::collections::HashSet<PathBuf> =
            std::collections::HashSet::new();
        for batch in &batches {
            for replacement in batch.replacements() {
                files_to_backup.insert(replacement.file.clone());
            }
        }

        let mut backup_writer = BackupWriter::new(&workspace_root, operation_id.clone())?;
        for file in files_to_backup {
            backup_writer.backup_file(&file)?;
        }
        Some(backup_writer.finalize()?)
    } else {
        None
    };

    let summaries =
        apply_batch_with_validation(&batches, &workspace_dir, symbol_language, analyzer_mode)?;

    // Check if JSON output is requested
    if _json_output {
        use splice::output::{
            ApplyFilesResult, FilePatternResult, OperationData, OperationResult, SpanResult,
        };

        // Build file results with spans
        let mut file_results: Vec<FilePatternResult> = Vec::new();
        for summary in &summaries {
            // Find all spans for this file
            let mut spans: Vec<SpanResult> = Vec::new();
            for batch in &batches {
                for replacement in batch.replacements() {
                    if replacement.file == summary.file {
                        spans.push(SpanResult::from_byte_span(
                            replacement.file.to_string_lossy().to_string(),
                            replacement.start,
                            replacement.end,
                        ));
                    }
                }
            }

            file_results.push(FilePatternResult {
                file: summary.file.to_string_lossy().to_string(),
                matches: spans.len(),
                replacements: spans.len(),
                spans,
                before_hash: summary.before_hash.clone(),
                after_hash: summary.after_hash.clone(),
            });
        }

        // Sort file_results deterministically by file path
        file_results.sort();

        // Sort spans within each file deterministically
        for result in &mut file_results {
            result.spans.sort();
        }

        // Create batch result structure (reuse ApplyFilesResult)
        let apply_result = ApplyFilesResult {
            glob_pattern: absolute_batch.to_string_lossy().to_string(),
            find_pattern: "batch".to_string(),
            replace_pattern: "patch".to_string(),
            files_matched: file_results.len(),
            files_modified: summaries.len(),
            files: file_results,
        };

        let message = format!(
            "Patched {} file(s) across {} batch(es).",
            summaries.len(),
            batch_count
        );

        // Record execution (before apply_result is moved)
        let duration_ms = start.elapsed().as_millis() as i64;
        let parameters = serde_json::json!({
            "batch_file": absolute_batch.to_string_lossy(),
            "file_count": apply_result.files.len(),
            "span_count": apply_result.files.iter().map(|f| f.matches).sum::<usize>(),
        });

        // Create operation result with operation_id from CLI or generate new UUID
        let result = OperationResult::with_execution_id("batch".to_string(), operation_id.clone())
            .success(message.clone())
            .with_result(OperationData::ApplyFiles(apply_result));

        if let Err(e) = log::record_execution_with_params(
            &result,
            duration_ms,
            Some(command_line.clone()),
            parameters,
        ) {
            log_execution_error("batch", &e);
        }

        // Output structured JSON directly
        println!("{}", serde_json::to_string_pretty(&result).unwrap());

        // Return a dummy payload marked as already emitted
        return Ok(
            splice::cli::CliSuccessPayload::message_only("OK".to_string()).already_emitted(),
        );
    }

    let files_data: Vec<_> = summaries
        .iter()
        .map(|summary| {
            json!({
                "file": summary.file.to_string_lossy(),
                "before_hash": summary.before_hash,
                "after_hash": summary.after_hash,
            })
        })
        .collect();

    // Collect span_ids from all batches
    let mut span_ids: Vec<serde_json::Value> = Vec::new();
    for batch in &batches {
        for replacement in batch.replacements() {
            span_ids.push(json!({
                "file": replacement.file.to_string_lossy(),
                "byte_start": replacement.start,
                "byte_end": replacement.end,
            }));
        }
    }

    let mut response_data = json!({
        "batch_file": absolute_batch.to_string_lossy(),
        "batches_applied": batch_count,
        "files": files_data,
        "span_ids": span_ids,
    });

    if let Some(manifest_path) = &backup_manifest_path {
        response_data["backup_manifest"] = json!(manifest_path.to_string_lossy());
    }

    if let Some(ref op_id) = operation_id {
        response_data["operation_id"] = json!(op_id);
    }

    if let Some(meta) = metadata {
        // Try to parse as JSON, if fails include as string
        if let Ok(parsed) = serde_json::from_str::<Value>(&meta) {
            response_data["metadata"] = parsed;
        } else {
            response_data["metadata"] = json!(meta);
        }
    }

    // Record execution for regular output
    let message = format!(
        "Patched {} file(s) across {} batch(es).",
        summaries.len(),
        batch_count
    );
    let duration_ms = start.elapsed().as_millis() as i64;
    let parameters = serde_json::json!({
        "batch_file": absolute_batch.to_string_lossy(),
        "file_count": summaries.len(),
        "span_count": span_ids.len(),
    });
    if let Err(e) = log::record_execution_with_params(
        &splice::output::OperationResult::with_execution_id(
            "batch".to_string(),
            operation_id.clone(),
        )
        .success(message.clone()),
        duration_ms,
        Some(command_line.clone()),
        parameters,
    ) {
        log_execution_error("batch", &e);
    }

    Ok(splice::cli::CliSuccessPayload::with_data(
        message,
        response_data,
    ))
}

/// Execute the plan command.
///
/// This function is a thin adapter that:
/// 1. Reads the plan.json file
/// 2. Calls execute_plan from the plan module
/// 3. Outputs structured JSON if requested
///
/// All logic is delegated to the plan module.
fn execute_plan(
    plan_path: &Path,
    operation_id: Option<String>,
    metadata: Option<String>,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::execution::log;
    use splice::output::{OperationData, OperationResult, PlanResult, StepResult};
    use splice::plan::execute_plan;

    // Start timing
    let start = std::time::Instant::now();
    let command_line = std::env::args().collect::<Vec<_>>().join(" ");

    // Determine workspace directory (parent of plan file)
    let workspace_dir = plan_path.parent().ok_or_else(|| {
        splice::SpliceError::Other(
            "Cannot determine workspace directory from plan path".to_string(),
        )
    })?;

    // Execute plan
    let messages = execute_plan(plan_path, workspace_dir)?;
    let step_count = messages.len();

    // Check if JSON output is requested
    if _json_output {
        // Create step results from messages
        let steps: Vec<StepResult> = messages
            .iter()
            .enumerate()
            .map(|(idx, msg)| StepResult {
                step: idx + 1,
                status: "ok".to_string(),
                message: msg.clone(),
                file: plan_path.to_string_lossy().to_string(),
                symbol: "plan".to_string(),
            })
            .collect();

        // Create plan result
        let plan_result = PlanResult {
            total_steps: messages.len(),
            steps_completed: messages.len(),
            steps,
            files_affected: {
                let mut files = vec![plan_path.to_string_lossy().to_string()];
                files.sort();
                files
            },
            total_bytes_changed: 0, // Not tracked in current implementation
        };

        let message = format!(
            "Plan executed successfully: {} steps completed",
            messages.len()
        );

        // Create operation result with operation_id from CLI or generate new UUID
        let result = OperationResult::with_execution_id("plan".to_string(), operation_id.clone())
            .success(message)
            .with_result(OperationData::Plan(plan_result));

        // Output structured JSON directly
        println!("{}", serde_json::to_string_pretty(&result).unwrap());

        // Record execution for JSON output
        let duration_ms = start.elapsed().as_millis() as i64;
        let parameters = serde_json::json!({
            "plan_file": plan_path.to_string_lossy(),
            "step_count": step_count,
        });
        if let Err(e) = log::record_execution_with_params(
            &splice::output::OperationResult::with_execution_id(
                "plan".to_string(),
                operation_id.clone(),
            )
            .success(format!(
                "Plan executed successfully: {} steps completed",
                step_count
            )),
            duration_ms,
            Some(command_line.clone()),
            parameters,
        ) {
            log_execution_error("plan", &e);
        }

        // Return a dummy payload marked as already emitted
        return Ok(
            splice::cli::CliSuccessPayload::message_only("OK".to_string()).already_emitted(),
        );
    }

    // Legacy output
    let mut response_data = serde_json::Map::new();
    response_data.insert("steps_completed".to_string(), json!(messages.len()));

    if let Some(ref op_id) = operation_id {
        response_data.insert("operation_id".to_string(), json!(op_id));
    }

    if let Some(ref meta) = metadata {
        // Try to parse as JSON, if fails include as string
        if let Ok(parsed) = serde_json::from_str::<Value>(meta) {
            response_data.insert("metadata".to_string(), parsed);
        } else {
            response_data.insert("metadata".to_string(), json!(meta));
        }
    }

    // Record execution for regular output
    let duration_ms = start.elapsed().as_millis() as i64;
    let message = format!(
        "Plan executed successfully: {} steps completed",
        messages.len()
    );
    let parameters = serde_json::json!({
        "plan_file": plan_path.to_string_lossy(),
        "step_count": step_count,
    });
    if let Err(e) = log::record_execution_with_params(
        &splice::output::OperationResult::with_execution_id(
            "plan".to_string(),
            operation_id.clone(),
        )
        .success(message.clone()),
        duration_ms,
        Some(command_line.clone()),
        parameters,
    ) {
        log_execution_error("plan", &e);
    }

    Ok(splice::cli::CliSuccessPayload::with_data(
        message,
        serde_json::Value::Object(response_data),
    ))
}

/// Execute the undo command.
///
/// This function restores files from a backup manifest created during
/// a previous splice operation.
fn execute_undo(
    manifest_path: &Path,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::patch::restore_from_manifest;

    // Determine workspace directory (parent of manifest's parent directory)
    // The manifest is at .splice-backup/<operation_id>/manifest.json
    // The workspace root is the parent of .splice-backup
    let backup_dir = manifest_path.parent().ok_or_else(|| {
        splice::SpliceError::Other("Manifest has no parent directory".to_string())
    })?;

    let splice_backup_dir = backup_dir.parent().ok_or_else(|| {
        splice::SpliceError::Other("Backup directory has no parent directory".to_string())
    })?;

    let workspace_root = splice_backup_dir.parent().ok_or_else(|| {
        splice::SpliceError::Other("Cannot determine workspace root from manifest path".to_string())
    })?;

    // Restore from backup
    let restored_count = restore_from_manifest(manifest_path, workspace_root)?;

    Ok(splice::cli::CliSuccessPayload::message_only(format!(
        "Restored {} file(s) from backup.",
        restored_count
    )))
}

/// Execute the apply-files command.
///
/// This function applies a text pattern replacement to multiple files
/// matching a glob pattern, with AST confirmation to ensure replacements
/// land on valid code tokens.
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
#[allow(unused_variables, reason = "stub args reserved for future expansion")]
fn execute_apply_files(
    glob_pattern: &str,
    find_pattern: &str,
    replace_pattern: &str,
    language: Option<splice::cli::Language>,
    context_before: usize,
    context_after: usize,
    context_both: usize,
    validate: bool,
    create_backup: bool,
    operation_id: Option<String>,
    metadata: Option<String>,
    dry_run: bool,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::execution::log;
    use splice::patch::{
        apply_pattern_replace, find_pattern_in_files, BackupWriter, PatternReplaceConfig,
    };

    // Start timing
    let start = std::time::Instant::now();
    let command_line = std::env::args().collect::<Vec<_>>().join(" ");

    // Get current directory as workspace root
    let workspace_root = env::current_dir().map_err(|err| {
        splice::SpliceError::Other(format!("Failed to resolve current directory: {}", err))
    })?;

    // Convert CLI language to symbol language
    let symbol_language = language.map(|l| l.to_symbol_language());

    // Dry-run path: find matches, summarize what would change, do NOT apply or back up.
    if dry_run {
        let find_config = PatternReplaceConfig {
            glob_pattern: glob_pattern.to_string(),
            find_pattern: find_pattern.to_string(),
            replace_pattern: replace_pattern.to_string(),
            language: symbol_language,
            validate: false,
        };
        let matches = find_pattern_in_files(&find_config)?;

        let mut files_to_patch: std::collections::BTreeSet<std::path::PathBuf> =
            std::collections::BTreeSet::new();
        for m in &matches {
            files_to_patch.insert(m.file.clone());
        }

        let mut response_data = serde_json::Map::new();
        response_data.insert("dry_run".to_string(), serde_json::Value::Bool(true));
        response_data.insert(
            "files_would_patch".to_string(),
            json!(files_to_patch.iter().collect::<Vec<_>>()),
        );
        response_data.insert("matches_count".to_string(), json!(matches.len()));

        let message = format!(
            "[dry-run] Would replace {} occurrence(s) of {:?} across {} file(s). No changes written.",
            matches.len(),
            find_pattern,
            files_to_patch.len()
        );

        return Ok(splice::cli::CliSuccessPayload::with_data(
            message,
            serde_json::Value::Object(response_data),
        ));
    }

    // Create backup if requested
    let backup_manifest_path = if create_backup {
        let mut backup_writer = BackupWriter::new(&workspace_root, operation_id.clone())?;

        // First, find all matching files to back up
        let find_config = PatternReplaceConfig {
            glob_pattern: glob_pattern.to_string(),
            find_pattern: find_pattern.to_string(),
            replace_pattern: replace_pattern.to_string(),
            language: symbol_language,
            validate: false,
        };
        let matches = find_pattern_in_files(&find_config)?;

        // Backup each file that will be modified
        for m in &matches {
            backup_writer.backup_file(&m.file)?;
        }

        Some(backup_writer.finalize()?)
    } else {
        None
    };

    // Create configuration for pattern replacement
    let config = PatternReplaceConfig {
        glob_pattern: glob_pattern.to_string(),
        find_pattern: find_pattern.to_string(),
        replace_pattern: replace_pattern.to_string(),
        language: symbol_language,
        validate,
    };

    // Apply the pattern replacement
    let result = apply_pattern_replace(&config, &workspace_root)?;

    // Build response data
    let mut response_data = serde_json::Map::new();
    response_data.insert("files_patched".to_string(), json!(result.files_patched));
    response_data.insert(
        "replacements_count".to_string(),
        json!(result.replacements_count),
    );
    if let Some(manifest_path) = backup_manifest_path {
        response_data.insert(
            "backup_manifest".to_string(),
            json!(manifest_path.to_string_lossy()),
        );
    }
    if let Some(ref op_id) = operation_id {
        response_data.insert("operation_id".to_string(), json!(op_id));
    }
    if let Some(meta) = metadata {
        // Try to parse as JSON, if fails include as string
        if let Ok(parsed) = serde_json::from_str::<Value>(&meta) {
            response_data.insert("metadata".to_string(), parsed);
        } else {
            response_data.insert("metadata".to_string(), json!(meta));
        }
    }

    let message = format!(
        "Applied replacements to {} file(s) ({} replacements).",
        result.files_patched.len(),
        result.replacements_count
    );

    // Record execution
    let duration_ms = start.elapsed().as_millis() as i64;
    let file_count = result.files_patched.len();
    let parameters = serde_json::json!({
        "glob": glob_pattern,
        "find": find_pattern,
        "replace": replace_pattern,
        "language": language.map(|l| l.as_str().to_string()),
        "file_count": file_count,
    });
    if let Err(e) = log::record_execution_with_params(
        &splice::output::OperationResult::with_execution_id(
            "apply-files".to_string(),
            operation_id.clone(),
        )
        .success(message.clone()),
        duration_ms,
        Some(command_line.clone()),
        parameters,
    ) {
        log_execution_error("apply-files", &e);
    }

    Ok(splice::cli::CliSuccessPayload::with_data(
        message,
        serde_json::Value::Object(response_data),
    ))
}

/// Execute the query command.
///
/// This function queries symbols by labels using Magellan integration.
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
#[allow(unused_variables, reason = "stub args reserved for future expansion")]
fn execute_query(
    db_path: &Path,
    labels: &[String],
    file_filter: Option<&str>,
    context_before: usize,
    context_after: usize,
    context_both: usize,
    list: bool,
    count: bool,
    show_code: bool,
    relationships: bool,
    expand: bool,
    expand_level: usize,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::execution::log;
    use splice::graph::magellan_integration::MagellanIntegration;

    // Resolve context counts from -A/-B/-C flags
    let (ctx_before, ctx_after) =
        splice::resolve_context_counts(context_before, context_after, context_both);

    // Start timing
    let start = std::time::Instant::now();
    let command_line = std::env::args().collect::<Vec<_>>().join(" ");

    // Open Magellan integration
    let integration = MagellanIntegration::open(db_path)?;

    // List all labels mode (SQLite backend only)
    #[cfg(feature = "sqlite")]
    if list {
        let all_labels = integration.get_all_labels()?;
        write_stdout_line(&format!("{} labels in use:", all_labels.len()))?;
        for label in &all_labels {
            let count = integration.count_by_label(label)?;
            write_stdout_line(&format!("  {} ({})", label, count))?;
        }

        // Record execution for list mode
        let duration_ms = start.elapsed().as_millis() as i64;
        let label_count = all_labels.len();
        let message = format!("Listed {} labels", label_count);
        let parameters = serde_json::json!({
            "db": db_path.to_string_lossy(),
            "list": true,
            "label_count": label_count,
        });
        if let Err(e) = log::record_execution_with_params(
            &splice::output::OperationResult::new("query".to_string()).success(message.clone()),
            duration_ms,
            Some(command_line.clone()),
            parameters,
        ) {
            log_execution_error("query", &e);
        }

        return Ok(splice::cli::CliSuccessPayload::message_only(message));
    }

    #[cfg(not(feature = "sqlite"))]
    if list {
        return Err(splice::SpliceError::Other(
            "The --list flag requires SQLite backend. \
             Use default SQLite backend: `cargo build` (no --features flag)"
                .to_string(),
        ));
    }

    // Count mode (SQLite backend only)
    #[cfg(feature = "sqlite")]
    if count {
        if labels.is_empty() {
            return Err(splice::SpliceError::Other(
                "--count requires at least one --label".to_string(),
            ));
        }

        let mut counts = serde_json::Map::new();
        for label in labels {
            let entity_count = integration.count_by_label(label)?;
            counts.insert(label.clone(), json!(entity_count));
        }

        // Record execution for count mode
        let duration_ms = start.elapsed().as_millis() as i64;
        let labels_count = labels.len();
        let message = format!("Counted entities for {} label(s)", labels_count);
        let parameters = serde_json::json!({
            "db": db_path.to_string_lossy(),
            "count": true,
            "labels": labels,
        });
        if let Err(e) = log::record_execution_with_params(
            &splice::output::OperationResult::new("query".to_string()).success(message.clone()),
            duration_ms,
            Some(command_line.clone()),
            parameters,
        ) {
            log_execution_error("query", &e);
        }

        return Ok(splice::cli::CliSuccessPayload::with_data(
            message,
            json!(counts),
        ));
    }

    #[cfg(not(feature = "sqlite"))]
    if count {
        return Err(splice::SpliceError::Other(
            "The --count flag requires SQLite backend. \
             Use default SQLite backend: `cargo build` (no --features flag)"
                .to_string(),
        ));
    }

    // Query mode - get symbols by label(s)
    // All label query modes require SQLite backend
    #[cfg(not(feature = "sqlite"))]
    return Err(splice::SpliceError::Other(
        "Label queries require SQLite backend. \
         Use default SQLite backend: `cargo build` (no --features flag)"
            .to_string(),
    ));

    #[cfg(feature = "sqlite")]
    if labels.is_empty() {
        return Err(splice::SpliceError::Other(
            "No labels specified. Use --label <LABEL> or --list to see all labels".to_string(),
        ));
    }

    #[cfg(feature = "sqlite")]
    let labels_ref: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    #[cfg(feature = "sqlite")]
    let mut results = integration.query_by_labels(&labels_ref)?;

    #[cfg(feature = "sqlite")]
    // Filter by file path if --file flag provided
    if let Some(file_pattern) = file_filter {
        results.retain(|r| r.file_path.contains(file_pattern));
        if results.is_empty() {
            return Err(splice::SpliceError::Other(format!(
                "No symbols found with labels {:?} in file pattern '{}'",
                labels, file_pattern
            )));
        }
    }

    #[cfg(feature = "sqlite")]
    // Sort results deterministically by file_path, then byte_start
    results.sort_by(|a, b| {
        a.file_path
            .cmp(&b.file_path)
            .then_with(|| a.byte_start.cmp(&b.byte_start))
    });

    #[cfg(feature = "sqlite")]
    if results.is_empty() {
        if labels.len() == 1 {
            write_stdout_line(&format!("No symbols found with label '{}'", labels[0]))?;
        } else {
            write_stdout_line(&format!(
                "No symbols found with labels: {}",
                labels.join(", ")
            ))?;
        }

        // Record execution for empty results
        let duration_ms = start.elapsed().as_millis() as i64;
        let message = "No symbols found".to_string();
        let parameters = serde_json::json!({
            "db": db_path.to_string_lossy(),
            "labels": labels,
            "results_count": 0,
        });
        if let Err(e) = log::record_execution_with_params(
            &splice::output::OperationResult::new("query".to_string()).success(message.clone()),
            duration_ms,
            Some(command_line.clone()),
            parameters,
        ) {
            log_execution_error("query", &e);
        }

        return Ok(splice::cli::CliSuccessPayload::message_only(message));
    }

    // Check if JSON output is requested
    #[cfg(feature = "sqlite")]
    if _json_output {
        use splice::action::{suggest_action, ActionType, Confidence};
        use splice::checksum;
        use splice::context;
        use splice::hints::{derive_tool_hints, ToolHintOperation};
        use splice::ingest::detect as ingest_detect;
        use splice::ingest::semantic_kind::SemanticKind;
        use splice::output::{OperationData, OperationResult, QueryResult, SpanResult};

        // Open CodeGraph for relationship queries if flag is set
        let code_graph = if relationships {
            Some(splice::graph::CodeGraph::open(db_path)?)
        } else {
            None
        };

        // Build rich span results with tool_hints and suggested_action
        let mut symbols: Vec<SpanResult> = Vec::new();

        for r in &results {
            let path = std::path::Path::new(&r.file_path);

            // Apply expansion if requested (use expand_to_body_with_docs to include doc comments)
            let (expanded_start, expanded_end) = if expand && expand_level > 0 {
                use splice::expand::expand_to_body_with_docs;
                use splice::ingest::detect as ingest_detect;
                use splice::symbol::Language;

                // Detect language from file path
                let lang = ingest_detect::detect_language(path);

                // Only proceed if language detection succeeded
                match lang {
                    Some(detected_lang) => {
                        let language = match detected_lang {
                            ingest_detect::Language::Rust => Language::Rust,
                            ingest_detect::Language::Python => Language::Python,
                            ingest_detect::Language::C => Language::C,
                            ingest_detect::Language::Cpp => Language::Cpp,
                            ingest_detect::Language::Java => Language::Java,
                            ingest_detect::Language::JavaScript => Language::JavaScript,
                            ingest_detect::Language::TypeScript => Language::TypeScript,
                        };

                        // Try to expand the symbol including doc comments
                        match expand_to_body_with_docs(path, r.byte_start, language) {
                            Ok((exp_start, exp_end)) => (exp_start, exp_end),
                            Err(_) => (r.byte_start, r.byte_end), // Fall back to original span on error
                        }
                    }
                    None => (r.byte_start, r.byte_end), // Language detection failed, use original span
                }
            } else {
                (r.byte_start, r.byte_end)
            };

            // Create base span result (use expanded span if expansion was requested)
            let (span_start, span_end) = (expanded_start, expanded_end);
            let mut span = SpanResult::from_byte_span(r.file_path.clone(), span_start, span_end)
                .with_symbol(r.name.clone(), r.kind.clone());

            // Add context if requested (use expanded span for context extraction)
            if ctx_before > 0 || ctx_after > 0 {
                if let Ok(ctx) = context::extract_context_asymmetric(
                    path, span_start, span_end, ctx_before, ctx_after,
                ) {
                    span = span.with_context(ctx);
                }
            }

            // Detect language and infer semantic kind from Magellan's kind string
            let (sem_kind, is_public) = if let Some(lang) = ingest_detect::detect_language(path) {
                // Map Magellan kind strings to SemanticKind
                let sem_kind = match r.kind.as_str() {
                    "fn" | "function" | "method" => SemanticKind::Function,
                    "struct" | "class" | "type" => SemanticKind::Type,
                    "trait" | "interface" => SemanticKind::Trait,
                    "enum" => SemanticKind::Enum,
                    "module" => SemanticKind::Module,
                    "const" | "static" => SemanticKind::Constant,
                    _ => SemanticKind::Unknown,
                };

                // Infer is_public from semantic kind (default to true for functions, types)
                let is_public = matches!(
                    sem_kind,
                    SemanticKind::Function
                        | SemanticKind::Type
                        | SemanticKind::Trait
                        | SemanticKind::Enum
                );

                span = span.with_semantic_info(sem_kind.as_str(), lang.as_str());

                (sem_kind, is_public)
            } else {
                (SemanticKind::Unknown, false)
            };

            // Derive tool hints
            let hints = derive_tool_hints(sem_kind, is_public, ToolHintOperation::Query);
            span = span.with_tool_hints(hints);

            // Generate suggested action
            let action = suggest_action(
                ActionType::Query,
                &r.name,
                &r.kind,
                &r.file_path,
                Confidence::High,
            );
            span = span.with_suggested_action(action);

            // Add checksums (use expanded span for checksum calculation)
            if let Ok(cs) = checksum::checksum_span(path, span_start, span_end) {
                span = span.with_checksum_before(cs.value);
            }
            if let Ok(file_cs) = checksum::checksum_file(path) {
                span = span.with_file_checksum_before(file_cs.value);
            }

            // Query relationships if flag is set
            if relationships {
                if let Some(ref graph) = code_graph {
                    use splice::relationships::{
                        get_callees, get_callers, get_exports, get_imports, RelationshipCache,
                        Relationships,
                    };
                    use sqlitegraph::NodeId;

                    let mut cache = RelationshipCache::new();
                    let node_id = NodeId::from(r.entity_id);

                    let callers = get_callers(graph, node_id, &mut cache).unwrap_or_default();
                    let callees = get_callees(graph, node_id, &mut cache).unwrap_or_default();
                    let imports = get_imports(graph, path, &mut cache).unwrap_or_default();
                    let exports = get_exports(graph, path, &mut cache).unwrap_or_default();

                    let rels = Relationships {
                        callers,
                        callees,
                        imports,
                        exports,
                        cycle_detected: false,
                        error_code: None,
                    };
                    span = span.with_relationships(rels);
                }
            }

            symbols.push(span);
        }

        // Sort spans deterministically
        symbols.sort();

        // Create query result
        let query_result = QueryResult {
            labels: labels.to_vec(),
            count: symbols.len(),
            symbols,
            total_count: None,
            offset: None,
            limit: None,
            max_symbols: None,
            max_bytes: None,
            next_offset: None,
            partial: None,
            truncation_reasons: None,
        };

        let results_count = query_result.count;

        // Create operation result
        let result = OperationResult::new("query".to_string())
            .success(format!("Found {} symbols", results_count))
            .with_result(OperationData::Query(query_result));

        // Output structured JSON directly
        println!("{}", serde_json::to_string_pretty(&result).unwrap());

        // Record execution
        let duration_ms = start.elapsed().as_millis() as i64;
        let parameters = serde_json::json!({
            "db": db_path.to_string_lossy(),
            "labels": labels,
            "show_code": show_code,
            "results_count": results_count,
        });
        if let Err(e) =
            log::record_execution_with_params(&result, duration_ms, Some(command_line), parameters)
        {
            log_execution_error("query", &e);
        }

        return Ok(
            splice::cli::CliSuccessPayload::message_only("OK".to_string()).already_emitted(),
        );
    }

    // Build response data (for non-JSON output)
    #[cfg(feature = "sqlite")]
    // Pre-calculate expanded boundaries for all results if expansion is requested
    struct ExpandedResult {
        result: splice::graph::magellan_integration::SymbolInfo,
        expanded_start: usize,
        expanded_end: usize,
    }

    #[cfg(feature = "sqlite")]
    let expanded_results: Vec<ExpandedResult> = results
        .iter()
        .map(|r| {
            // Apply expansion if requested
            let (exp_start, exp_end) = if expand && expand_level > 0 {
                use splice::expand::expand_to_body_with_docs;
                use splice::ingest::detect as ingest_detect;
                use splice::symbol::Language;

                let path = std::path::Path::new(&r.file_path);
                let lang = ingest_detect::detect_language(path);

                match lang {
                    Some(detected_lang) => {
                        let language = match detected_lang {
                            ingest_detect::Language::Rust => Language::Rust,
                            ingest_detect::Language::Python => Language::Python,
                            ingest_detect::Language::C => Language::C,
                            ingest_detect::Language::Cpp => Language::Cpp,
                            ingest_detect::Language::Java => Language::Java,
                            ingest_detect::Language::JavaScript => Language::JavaScript,
                            ingest_detect::Language::TypeScript => Language::TypeScript,
                        };

                        match expand_to_body_with_docs(path, r.byte_start, language) {
                            Ok((start, end)) => (start, end),
                            Err(_) => (r.byte_start, r.byte_end),
                        }
                    }
                    None => (r.byte_start, r.byte_end),
                }
            } else {
                (r.byte_start, r.byte_end)
            };

            ExpandedResult {
                result: r.clone(),
                expanded_start: exp_start,
                expanded_end: exp_end,
            }
        })
        .collect();

    #[cfg(feature = "sqlite")]
    let symbols_data: Vec<serde_json::Value> = expanded_results
        .iter()
        .map(|er| {
            let mut data = json!({
                "entity_id": er.result.entity_id,
                "name": er.result.name,
                "file_path": er.result.file_path,
                "kind": er.result.kind,
                "byte_start": er.result.byte_start,
                "byte_end": er.result.byte_end,
            });

            // Include expanded span if expansion was performed
            if expand
                && expand_level > 0
                && (er.expanded_start != er.result.byte_start
                    || er.expanded_end != er.result.byte_end)
            {
                data["expanded_byte_start"] = json!(er.expanded_start);
                data["expanded_byte_end"] = json!(er.expanded_end);
            }

            data
        })
        .collect();

    #[cfg(feature = "sqlite")]
    // Print results to console
    if labels.len() == 1 {
        write_stdout_line(&format!(
            "{} symbols with label '{}':",
            expanded_results.len(),
            labels[0]
        ))?;
    } else {
        write_stdout_line(&format!(
            "{} symbols with labels [{}]:",
            expanded_results.len(),
            labels.join(", ")
        ))?;
    }

    #[cfg(feature = "sqlite")]
    for er in &expanded_results {
        write_stdout_line("")?;
        write_stdout_line(&format!(
            "  {} ({}) in {} [{}-{}]",
            er.result.name,
            er.result.kind,
            er.result.file_path,
            er.result.byte_start,
            er.result.byte_end
        ))?;

        // Show context if requested (use expanded span for context extraction)
        if !show_code && (ctx_before > 0 || ctx_after > 0) {
            use splice::context;
            let path = std::path::Path::new(&er.result.file_path);
            if let Ok(ctx) = context::extract_context_asymmetric(
                path,
                er.expanded_start,
                er.expanded_end,
                ctx_before,
                ctx_after,
            ) {
                if !ctx.before.is_empty() {
                    write_stdout_line(&format!("  Context ({} lines before):", ctx.before.len()))?;
                    for line in &ctx.before {
                        write_stdout_line(&format!("    {}", line))?;
                    }
                }
                if !ctx.after.is_empty() {
                    write_stdout_line(&format!("  Context ({} lines after):", ctx.after.len()))?;
                    for line in &ctx.after {
                        write_stdout_line(&format!("    {}", line))?;
                    }
                }
            }
        }

        // Show code chunk if requested
        if show_code {
            let path = std::path::Path::new(&er.result.file_path);
            // Use expanded span for code retrieval
            if let Ok(Some(code)) =
                integration.get_code_chunk(path, er.expanded_start, er.expanded_end)
            {
                // Show context before code chunk if context flags are set
                if ctx_before > 0 || ctx_after > 0 {
                    use splice::context;
                    if let Ok(ctx) = context::extract_context_asymmetric(
                        path,
                        er.expanded_start,
                        er.expanded_end,
                        ctx_before,
                        ctx_after,
                    ) {
                        if !ctx.before.is_empty() {
                            write_stdout_line(&format!(
                                "  Context ({} lines before):",
                                ctx.before.len()
                            ))?;
                            for line in &ctx.before {
                                write_stdout_line(&format!("    {}", line))?;
                            }
                        }
                    }
                }

                write_stdout_line("  Code:")?;
                for line in code.lines() {
                    write_stdout_line(&format!("    {}", line))?;
                }

                // Show context after code chunk if context flags are set
                if ctx_before > 0 || ctx_after > 0 {
                    use splice::context;
                    if let Ok(ctx) = context::extract_context_asymmetric(
                        path,
                        er.expanded_start,
                        er.expanded_end,
                        ctx_before,
                        ctx_after,
                    ) {
                        if !ctx.after.is_empty() {
                            write_stdout_line(&format!(
                                "  Context ({} lines after):",
                                ctx.after.len()
                            ))?;
                            for line in &ctx.after {
                                write_stdout_line(&format!("    {}", line))?;
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "sqlite")]
    // Record execution for normal query
    let duration_ms = start.elapsed().as_millis() as i64;
    #[cfg(feature = "sqlite")]
    let results_count = results.len();
    #[cfg(feature = "sqlite")]
    let message = format!("Found {} symbols", results_count);
    #[cfg(feature = "sqlite")]
    let parameters = serde_json::json!({
        "db": db_path.to_string_lossy(),
        "labels": labels,
        "show_code": show_code,
        "results_count": results_count,
    });
    #[cfg(feature = "sqlite")]
    if let Err(e) = log::record_execution_with_params(
        &splice::output::OperationResult::new("query".to_string()).success(message.clone()),
        duration_ms,
        Some(command_line.clone()),
        parameters,
    ) {
        log_execution_error("query", &e);
    }

    #[cfg(feature = "sqlite")]
    {
        Ok(splice::cli::CliSuccessPayload::with_data(
            message,
            json!(symbols_data),
        ))
    }

    #[cfg(not(feature = "sqlite"))]
    {
        Ok(splice::cli::CliSuccessPayload::message_only(
            "Label queries require SQLite backend".to_string(),
        ))
    }
}

/// Execute the get command.
///
/// This function retrieves code chunks from the database using Magellan integration.
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
#[allow(unused_variables, reason = "stub args reserved for future expansion")]
fn execute_get(
    db_path: &Path,
    file_path: &Path,
    start: usize,
    end: usize,
    context_before: usize,
    context_after: usize,
    context_both: usize,
    relationships: bool,
    expand: bool,
    expand_level: usize,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::MagellanIntegration;

    // Resolve context counts from -A/-B/-C flags
    let (ctx_before, ctx_after) =
        splice::resolve_context_counts(context_before, context_after, context_both);

    // Apply expansion if requested (use expand_to_body_with_docs to include doc comments)
    let (expanded_start, expanded_end) = if expand && expand_level > 0 {
        use splice::expand::expand_to_body_with_docs;
        use splice::ingest::detect as ingest_detect;
        use splice::symbol::Language;

        // Detect language from file path
        let lang = ingest_detect::detect_language(file_path);

        // Only proceed if language detection succeeded
        match lang {
            Some(detected_lang) => {
                let language = match detected_lang {
                    ingest_detect::Language::Rust => Language::Rust,
                    ingest_detect::Language::Python => Language::Python,
                    ingest_detect::Language::C => Language::C,
                    ingest_detect::Language::Cpp => Language::Cpp,
                    ingest_detect::Language::Java => Language::Java,
                    ingest_detect::Language::JavaScript => Language::JavaScript,
                    ingest_detect::Language::TypeScript => Language::TypeScript,
                };

                // Try to expand the symbol including doc comments
                match expand_to_body_with_docs(file_path, start, language) {
                    Ok((exp_start, exp_end)) => (exp_start, exp_end),
                    Err(_) => (start, end), // Fall back to original span on error
                }
            }
            None => (start, end), // Language detection failed, use original span
        }
    } else {
        (start, end)
    };

    // Open Magellan integration
    let integration = MagellanIntegration::open(db_path)?;

    // Get code chunk (use expanded span if expansion was requested and successful)
    let code = integration.get_code_chunk(file_path, expanded_start, expanded_end)?;

    match code {
        Some(content) => {
            // Check if JSON output is requested
            if _json_output {
                use splice::action::{suggest_action, ActionType, Confidence};
                use splice::checksum;
                use splice::context;
                use splice::hints::{derive_tool_hints, ToolHintOperation};
                use splice::ingest::detect as ingest_detect;
                use splice::ingest::semantic_kind::SemanticKind;
                use splice::output::{OperationData, OperationResult, SpanResult};

                // Create span result with tool_hints and suggested_action
                // Use expanded span if expansion was requested
                let (span_start, span_end) = (expanded_start, expanded_end);
                let mut span = SpanResult::from_byte_span(
                    file_path.to_string_lossy().to_string(),
                    span_start,
                    span_end,
                );

                // Add context if requested (use expanded span for context extraction)
                if ctx_before > 0 || ctx_after > 0 {
                    if let Ok(ctx) = context::extract_context_asymmetric(
                        file_path, span_start, span_end, ctx_before, ctx_after,
                    ) {
                        span = span.with_context(ctx);
                    }
                }

                // Detect language and infer semantic kind
                let (sem_kind, is_public) =
                    if let Some(lang) = ingest_detect::detect_language(file_path) {
                        // Default to Function for get operations (most common case)
                        let sem_kind = SemanticKind::Function;
                        let is_public = true; // Default to public for get operations

                        span = span.with_semantic_info(sem_kind.as_str(), lang.as_str());

                        (sem_kind, is_public)
                    } else {
                        (SemanticKind::Unknown, false)
                    };

                // Derive tool hints
                let hints = derive_tool_hints(sem_kind, is_public, ToolHintOperation::Get);
                span = span.with_tool_hints(hints);

                // Generate suggested action
                let action = suggest_action(
                    ActionType::Read,
                    "code_chunk",
                    "unknown",
                    &file_path.to_string_lossy(),
                    Confidence::High,
                );
                span = span.with_suggested_action(action);

                // Add checksums (use expanded span for checksum calculation)
                if let Ok(cs) = checksum::checksum_span(file_path, span_start, span_end) {
                    span = span.with_checksum_before(cs.value);
                }
                if let Ok(file_cs) = checksum::checksum_file(file_path) {
                    span = span.with_file_checksum_before(file_cs.value);
                }

                // Query relationships if flag is set
                if relationships {
                    use splice::relationships::{
                        get_exports, get_imports, RelationshipCache, Relationships,
                    };

                    let code_graph = splice::graph::CodeGraph::open(db_path)?;
                    let mut cache = RelationshipCache::new();

                    let imports =
                        get_imports(&code_graph, file_path, &mut cache).unwrap_or_default();
                    let exports =
                        get_exports(&code_graph, file_path, &mut cache).unwrap_or_default();

                    let rels = Relationships {
                        callers: vec![],
                        callees: vec![],
                        imports,
                        exports,
                        cycle_detected: false,
                        error_code: None,
                    };
                    span = span.with_relationships(rels);
                }

                // Create operation result with span as data
                let result = OperationResult::new("get".to_string())
                    .success(format!("Retrieved code chunk ({} bytes)", content.len()))
                    .with_result(OperationData::Query(splice::output::QueryResult {
                        labels: vec![],
                        count: 1,
                        symbols: vec![span],
                        total_count: None,
                        offset: None,
                        limit: None,
                        max_symbols: None,
                        max_bytes: None,
                        next_offset: None,
                        partial: None,
                        truncation_reasons: None,
                    }));

                // Output structured JSON directly
                println!("{}", serde_json::to_string_pretty(&result).unwrap());

                return Ok(
                    splice::cli::CliSuccessPayload::message_only("OK".to_string())
                        .already_emitted(),
                );
            }

            // Print to console (non-JSON output)
            // Show context before if requested (use expanded span for context extraction)
            if ctx_before > 0 || ctx_after > 0 {
                use splice::context;
                if let Ok(ctx) = context::extract_context_asymmetric(
                    file_path,
                    expanded_start,
                    expanded_end,
                    ctx_before,
                    ctx_after,
                ) {
                    if !ctx.before.is_empty() {
                        write_stdout_line(&format!(
                            "Context ({} lines before):",
                            ctx.before.len()
                        ))?;
                        for line in &ctx.before {
                            write_stdout_line(&format!("  {}", line))?;
                        }
                    }
                }
            }

            // Write the actual content
            write_stdout_bytes(content.as_bytes())?;
            write_stdout_bytes(b"\n")?;

            // Show context after if requested (use expanded span for context extraction)
            if ctx_before > 0 || ctx_after > 0 {
                use splice::context;
                if let Ok(ctx) = context::extract_context_asymmetric(
                    file_path,
                    expanded_start,
                    expanded_end,
                    ctx_before,
                    ctx_after,
                ) {
                    if !ctx.after.is_empty() {
                        write_stdout_line(&format!("Context ({} lines after):", ctx.after.len()))?;
                        for line in &ctx.after {
                            write_stdout_line(&format!("  {}", line))?;
                        }
                    }
                }
            }

            // Return success with both original and expanded spans in response data
            let mut response_data = json!({
                "file": file_path.to_string_lossy(),
                "byte_start": start,
                "byte_end": end,
                "content_length": content.len(),
            });

            // Include expanded span info if expansion was performed
            if expand && expand_level > 0 && (expanded_start != start || expanded_end != end) {
                response_data["expanded_byte_start"] = json!(expanded_start);
                response_data["expanded_byte_end"] = json!(expanded_end);
            }

            Ok(splice::cli::CliSuccessPayload::with_data(
                format!("Retrieved code chunk ({} bytes)", content.len()),
                response_data,
            ))
        }
        None => Ok(splice::cli::CliSuccessPayload::message_only(format!(
            "No code chunk found at {}:{}-{}",
            file_path.display(),
            start,
            end
        ))),
    }
}

/// Execute the `log` command.
///
/// This function queries the execution log and displays results.
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
fn execute_log(
    operation_type: Option<String>,
    status: Option<String>,
    after: Option<String>,
    before: Option<String>,
    limit: usize,
    offset: usize,
    execution_id: Option<String>,
    json: bool,
    stats: bool,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::execution::{
        get_execution, get_execution_stats, init_execution_log_db, ExecutionQuery,
    };
    use splice::SpliceError;

    // Get splice directory
    let splice_dir = std::path::PathBuf::from(".splice");
    let conn = init_execution_log_db(&splice_dir)?;

    // Handle --execution-id
    if let Some(id) = execution_id {
        let log = get_execution(&conn, &id)?
            .ok_or_else(|| SpliceError::ExecutionNotFound { execution_id: id })?;

        if json || json_output {
            let json_output = serde_json::to_string_pretty(&log).map_err(|e| {
                SpliceError::Other(format!("failed to serialize execution to JSON: {}", e))
            })?;
            println!("{}", json_output);

            return Ok(splice::cli::CliSuccessPayload::with_data(
                "Execution details".to_string(),
                json!({ "execution_id": log.execution_id }),
            ));
        } else {
            // Table format for single execution
            println!("Execution Details:");
            println!("  ID: {}", log.execution_id);
            println!("  Type: {}", log.operation_type);
            println!("  Status: {}", log.status);
            println!("  Time: {}", log.timestamp);
            if let Some(workspace) = &log.workspace {
                println!("  Workspace: {}", workspace);
            }
            if let Some(cmd) = &log.command_line {
                println!("  Command: {}", cmd);
            }
            if let Some(duration) = log.duration_ms {
                println!("  Duration: {}ms", duration);
            }

            return Ok(splice::cli::CliSuccessPayload::message_only(
                "Execution details retrieved".to_string(),
            ));
        }
    }

    // Handle --stats
    if stats {
        let stats = get_execution_stats(&conn)?;

        if json || json_output {
            let json_output = serde_json::to_string_pretty(&stats).map_err(|e| {
                SpliceError::Other(format!("failed to serialize stats to JSON: {}", e))
            })?;
            println!("{}", json_output);

            return Ok(splice::cli::CliSuccessPayload::with_data(
                "Execution statistics".to_string(),
                json!({ "total_operations": stats.total_operations }),
            ));
        } else {
            // Human-readable stats
            println!("Execution Statistics:");
            println!("  Total operations: {}", stats.total_operations);

            println!("  By type:");
            for (op_type, count) in &stats.by_type {
                println!("    {}: {}", op_type, count);
            }

            println!("  By status:");
            for (status, count) in &stats.by_status {
                println!("    {}: {}", status, count);
            }

            if let Some(oldest) = &stats.oldest_execution {
                println!("  Oldest: {}", oldest);
            }
            if let Some(newest) = &stats.newest_execution {
                println!("  Newest: {}", newest);
            }

            return Ok(splice::cli::CliSuccessPayload::message_only(
                "Statistics retrieved".to_string(),
            ));
        }
    }

    // Build query from filters
    let mut query = ExecutionQuery::new().with_limit(limit).with_offset(offset);

    if let Some(op_type) = operation_type {
        query = query.with_operation_type(op_type);
    }

    if let Some(s) = status {
        query = query.with_status(s);
    }

    // Parse date filters
    if let Some(after_str) = after {
        let timestamp = parse_date(&after_str)?;
        query = query.after(timestamp);
    }

    if let Some(before_str) = before {
        let timestamp = parse_date(&before_str)?;
        query = query.before(timestamp);
    }

    let logs = query.execute(&conn)?;

    if json || json_output {
        let json_output = serde_json::to_string_pretty(&logs)
            .map_err(|e| SpliceError::Other(format!("failed to serialize logs to JSON: {}", e)))?;
        println!("{}", json_output);

        Ok(splice::cli::CliSuccessPayload::with_data(
            format!("{} executions", logs.len()),
            json!({ "count": logs.len() }),
        ))
    } else {
        // Table format
        if logs.is_empty() {
            println!("No executions found matching criteria.");
            return Ok(splice::cli::CliSuccessPayload::message_only(
                "No executions found".to_string(),
            ));
        }

        // Print header
        println!(
            "{:<10} {:<8} {:<8} {:<20} {:<10} Message",
            "ID", "Type", "Status", "Time", "Duration"
        );
        println!("{}", "-".repeat(100));

        // Print rows
        for log in &logs {
            use splice::execution::format_table_row;
            println!("{}", format_table_row(log));
        }

        println!("\nShowing {} of {} executions", logs.len(), logs.len());

        Ok(splice::cli::CliSuccessPayload::message_only(format!(
            "Retrieved {} executions",
            logs.len()
        )))
    }
}

/// Execute the `splice explain` command.
///
/// Prints detailed documentation for the specified error code.
fn execute_explain(
    code: String,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    if json_output {
        // In JSON mode, return structured output
        let explanation = splice::get_error_explanation(&code)
            .unwrap_or("Unknown error code")
            .to_string();

        let payload = splice::cli::CliSuccessPayload::with_data(
            format!("Error code explanation: {}", code),
            serde_json::json!({
                "code": code,
                "explanation": explanation,
            }),
        );
        return Ok(payload);
    }

    // Human-readable output
    match splice::get_error_explanation(&code) {
        Some(explanation) => {
            println!("{}", explanation.trim());
        }
        None => {
            eprintln!("Unknown error code: {}", code);
            eprintln!();
            eprintln!("Error codes follow the format SPL-E### (e.g., SPL-E001).");
            eprintln!("Run `splice explain --list` to see all error codes.");
            eprintln!();
            eprintln!("For compiler error codes, see:");
            eprintln!("  Rust: https://doc.rust-lang.org/error-index.html");
            eprintln!("  TypeScript: https://www.typescriptlang.org/errors/");
            return Err(splice::SpliceError::Other(format!(
                "Unknown error code: {}",
                code
            )));
        }
    }

    Ok(splice::cli::CliSuccessPayload::message_only(format!(
        "Explained {}",
        code
    )))
}

#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
fn execute_search(
    pattern: &str,
    path: &Path,
    language: Option<splice::cli::Language>,
    glob: Option<String>,
    apply: bool,
    replace: Option<&str>,
    context_before: usize,
    context_after: usize,
    context_both: usize,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::context::extract_context_asymmetric;
    use splice::context::resolve_context_counts;
    use splice::patch::pattern;

    // Resolve context counts from -A/-B/-C flags
    let (ctx_before, ctx_after) =
        resolve_context_counts(context_before, context_after, context_both);

    // Convert CLI language to symbol language
    let symbol_lang = language.map(|l: splice::cli::Language| l.to_symbol_language());

    // Build glob pattern(s) from user input or construct from path and language.
    //
    // NOTE: the `glob` crate does NOT support `{a,b,c}` brace expansion.
    // When no language is given, we iterate per extension instead of relying on braces.
    const ALL_EXTENSIONS: &[&str] = &[
        "rs", "py", "c", "cpp", "h", "hpp", "cc", "cxx", "java", "js", "mjs", "cjs", "ts", "tsx",
    ];
    let glob_patterns: Vec<String> = if let Some(g) = glob {
        // User provided explicit glob pattern
        vec![g]
    } else if let Some(lang) = language {
        let ext = match lang {
            splice::cli::Language::Rust => "rs",
            splice::cli::Language::Python => "py",
            splice::cli::Language::C => "c",
            splice::cli::Language::Cpp => "cpp",
            splice::cli::Language::Java => "java",
            splice::cli::Language::JavaScript => "js",
            splice::cli::Language::TypeScript => "ts",
        };
        if path.is_dir() {
            vec![format!("{}/**/*.{}", path.display(), ext)]
        } else {
            vec![path.display().to_string()]
        }
    } else if path.is_dir() {
        ALL_EXTENSIONS
            .iter()
            .map(|ext| format!("{}/**/*.{}", path.display(), ext))
            .collect()
    } else {
        vec![path.display().to_string()]
    };

    // Check if this is an apply operation
    let apply_replace = apply && replace.is_some();

    // If apply mode, perform replacement and return summary
    if apply_replace {
        let current_dir = std::env::current_dir().map_err(|source| splice::SpliceError::Io {
            path: std::path::PathBuf::from("."),
            source,
        })?;
        let mut total_replacements = 0usize;
        let mut all_files: std::collections::BTreeSet<PathBuf> = std::collections::BTreeSet::new();
        for gp in &glob_patterns {
            let config = pattern::PatternReplaceConfig {
                glob_pattern: gp.clone(),
                find_pattern: pattern.to_string(),
                replace_pattern: replace.unwrap_or_default().to_string(),
                language: symbol_lang,
                validate: false,
            };
            let result = pattern::apply_pattern_replace(&config, &current_dir)?;
            total_replacements += result.replacements_count;
            all_files.extend(result.files_patched);
        }

        Ok(splice::cli::CliSuccessPayload::message_only(format!(
            "Applied {} replacement(s) across {} file(s)",
            total_replacements,
            all_files.len()
        )))
    } else {
        // Search-only mode: collect matches across all glob patterns
        let mut matches = Vec::new();
        for gp in &glob_patterns {
            let config = pattern::PatternReplaceConfig {
                glob_pattern: gp.clone(),
                find_pattern: pattern.to_string(),
                replace_pattern: String::new(),
                language: symbol_lang,
                validate: false,
            };
            matches.extend(pattern::find_pattern_in_files(&config)?);
        }

        if json_output {
            let results: Vec<Value> = matches
                .into_iter()
                .map(|m| {
                    // Extract context if requested
                    let (context_before_opt, context_selected_opt, context_after_opt) =
                        if ctx_before > 0 || ctx_after > 0 {
                            match extract_context_asymmetric(
                                &m.file,
                                m.byte_start,
                                m.byte_end,
                                ctx_before,
                                ctx_after,
                            ) {
                                Ok(ctx) => (Some(ctx.before), Some(ctx.selected), Some(ctx.after)),
                                Err(_) => (None, None, None),
                            }
                        } else {
                            (None, None, None)
                        };

                    let mut result = json!({
                        "file": m.file.to_string_lossy().to_string(),
                        "byte_start": m.byte_start,
                        "byte_end": m.byte_end,
                        "line": m.line,
                        "column": m.column,
                        "matched_text": m.matched_text,
                    });

                    // Add context fields if present
                    if let (Some(before), Some(selected), Some(after)) =
                        (context_before_opt, context_selected_opt, context_after_opt)
                    {
                        if let Some(obj) = result.as_object_mut() {
                            obj.insert("context_before".to_string(), json!(before));
                            obj.insert("context_selected".to_string(), json!(selected));
                            obj.insert("context_after".to_string(), json!(after));
                        }
                    }

                    result
                })
                .collect();

            // Wrap results in structured output for LLM consumption
            let output = json!({
                "status": "ok",
                "message": format!("Found {} occurrence(s) of '{}'", results.len(), pattern),
                "matches": results,
                "pattern": pattern,
                "count": results.len(),
            });

            let payload = serde_json::to_string_pretty(&output).map_err(|e| {
                splice::SpliceError::Other(format!("Failed to serialize JSON: {}", e))
            })?;
            println!("{}", payload);

            Ok(splice::cli::CliSuccessPayload::message_only("OK".to_string()).already_emitted())
        } else {
            // Human-readable output
            for m in &matches {
                println!(
                    "{}:{}:{}: {}",
                    m.file.display(),
                    m.line,
                    m.column,
                    m.matched_text
                );

                // Show context if requested
                if ctx_before > 0 || ctx_after > 0 {
                    if let Ok(ctx) = extract_context_asymmetric(
                        &m.file,
                        m.byte_start,
                        m.byte_end,
                        ctx_before,
                        ctx_after,
                    ) {
                        if !ctx.before.is_empty() {
                            println!("  Context ({} line(s) before):", ctx.before.len());
                            for (i, line) in ctx.before.iter().enumerate() {
                                println!("  {}: {}", m.line - ctx.before.len() + i, line);
                            }
                        }

                        if !ctx.selected.is_empty() {
                            for (i, line) in ctx.selected.iter().enumerate() {
                                println!("  {}: {}", m.line + i, line);
                            }
                        }

                        if !ctx.after.is_empty() {
                            println!("  Context ({} line(s) after):", ctx.after.len());
                            for (i, line) in ctx.after.iter().enumerate() {
                                println!("  {}: {}", m.line + ctx.selected.len() + i, line);
                            }
                        }

                        println!();
                    }
                }
            }

            Ok(splice::cli::CliSuccessPayload::message_only(format!(
                "Found {} occurrence(s) of '{}'",
                matches.len(),
                pattern
            )))
        }
    }
}

/// Execute the status command.
///
/// Shows database statistics from Magellan integration.
fn execute_status(
    db_path: &Path,
    json_output: bool,
    detect_backend: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::MagellanIntegration;

    if !db_path.exists() {
        return Err(splice::SpliceError::Magellan {
            context: format!("Database not found: {}", db_path.display()),
            source: anyhow::anyhow!("No database file at the specified path"),
        });
    }

    // If detect_backend flag is set, report backend and exit early
    if detect_backend {
        let backend = splice::graph::CodeGraph::detect_backend(db_path)?;
        if json_output {
            let output = json!({
                "backend": backend.to_string(),
                "database": db_path.to_string_lossy(),
            });
            return Ok(splice::cli::CliSuccessPayload::with_data(
                format!("Backend: {}", backend),
                output,
            ));
        } else {
            return Ok(splice::cli::CliSuccessPayload::message_only(format!(
                "Backend: {}\nDatabase: {}",
                backend,
                db_path.display()
            )));
        }
    }

    let integration = MagellanIntegration::open(db_path)?;
    let stats = integration.get_statistics()?;

    let data = serde_json::json!({
        "files": stats.files,
        "symbols": stats.symbols,
        "references": stats.references,
        "calls": stats.calls,
        "code_chunks": stats.code_chunks,
        "db_path": db_path.to_string_lossy(),
    });

    if json_output {
        Ok(splice::cli::CliSuccessPayload::with_data(
            format!(
                "Database has {} files, {} symbols",
                stats.files, stats.symbols
            ),
            data,
        ))
    } else {
        let message = format!(
            "Database statistics:\n  Files: {}\n  Symbols: {}\n  References: {}\n  Calls: {}\n  Code chunks: {}",
            stats.files, stats.symbols, stats.references, stats.calls, stats.code_chunks
        );
        Ok(splice::cli::CliSuccessPayload::message_only(message))
    }
}

/// Execute the find command.
///
/// Finds symbols by name or 16-character symbol ID.
fn execute_find(
    db_path: &Path,
    name: Option<String>,
    symbol_id: Option<String>,
    ambiguous: bool,
    _output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::MagellanIntegration;

    let mut integration = MagellanIntegration::open(db_path)?;

    let results = if let Some(id) = symbol_id {
        // Search by symbol ID
        match integration.find_symbol_by_id(&id)? {
            Some(symbol) => vec![symbol],
            None => {
                return Err(splice::SpliceError::symbol_not_found(
                    format!("ID '{}'", id),
                    Some(db_path),
                ))
            }
        }
    } else if let Some(ref n) = name {
        // Search by name
        integration.find_symbol_by_name(n, ambiguous)?
    } else {
        return Err(splice::SpliceError::Other(
            "--name or --symbol-id required".to_string(),
        ));
    };

    if results.is_empty() {
        return Err(splice::SpliceError::symbol_not_found(
            name.as_deref().unwrap_or("unknown"),
            Some(db_path),
        ));
    }

    let count = results.len();

    if json_output {
        let symbols_data: Vec<serde_json::Value> = results
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "kind": s.kind,
                    "file_path": s.file_path,
                    "byte_start": s.byte_start,
                    "byte_end": s.byte_end,
                    "start_line": s.start_line,
                    "end_line": s.end_line,
                })
            })
            .collect();

        Ok(splice::cli::CliSuccessPayload::with_data(
            format!("Found {} symbol(s)", count),
            serde_json::json!({ "symbols": symbols_data, "count": count }),
        ))
    } else {
        let lines: Vec<String> = results
            .iter()
            .map(|s| {
                let line = s.start_line.unwrap_or(s.byte_start);
                format!("{} :: {} at {}:{}", s.kind, s.name, s.file_path, line)
            })
            .collect();
        let message = format!("Found {} symbol(s):\n{}", count, lines.join("\n"));
        Ok(splice::cli::CliSuccessPayload::message_only(message))
    }
}

/// Execute the refs command.
///
/// Shows call relationships for a symbol.
fn execute_refs(
    db_path: &Path,
    name: &str,
    path: &Path,
    direction: splice::cli::CallDirection,
    _output: splice::cli::OutputFormat,
    impact_graph: bool,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::{
        CallDirection, ImpactDotConfig, MagellanIntegration,
    };

    let mut integration = MagellanIntegration::open(db_path)?;

    // Handle impact graph generation
    if impact_graph {
        let config = ImpactDotConfig::default();
        let dot = integration.generate_refs_dot(name, path, &config)?;
        println!("{}", dot);
        return Ok(splice::cli::CliSuccessPayload::message_only(
            "Impact graph generated".to_string(),
        )
        .already_emitted());
    }

    let magellan_direction = match direction {
        splice::cli::CallDirection::In => CallDirection::In,
        splice::cli::CallDirection::Out => CallDirection::Out,
        splice::cli::CallDirection::Both => CallDirection::Both,
    };

    let relationships = integration.get_call_relationships(path, name, magellan_direction)?;

    if json_output {
        let callers_data: Vec<serde_json::Value> = relationships
            .callers
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.symbol.name,
                    "kind": c.symbol.kind,
                    "file_path": c.symbol.file_path,
                })
            })
            .collect();

        let callees_data: Vec<serde_json::Value> = relationships
            .callees
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.symbol.name,
                    "kind": c.symbol.kind,
                    "file_path": c.symbol.file_path,
                })
            })
            .collect();

        Ok(splice::cli::CliSuccessPayload::with_data(
            format!("Call relationships for {}", name),
            serde_json::json!({
                "symbol": {
                    "name": relationships.symbol.name,
                    "kind": relationships.symbol.kind,
                    "file_path": relationships.symbol.file_path,
                },
                "callers": callers_data,
                "callees": callees_data,
            }),
        ))
    } else {
        let mut lines = vec![format!(
            "Symbol: {} :: {}",
            relationships.symbol.kind, relationships.symbol.name
        )];

        if !relationships.callers.is_empty() {
            lines.push(format!("  Callers ({}):", relationships.callers.len()));
            for caller in &relationships.callers {
                lines.push(format!(
                    "    - {} :: {} at {}",
                    caller.symbol.kind, caller.symbol.name, caller.symbol.file_path
                ));
            }
        }

        if !relationships.callees.is_empty() {
            lines.push(format!("  Callees ({}):", relationships.callees.len()));
            for callee in &relationships.callees {
                lines.push(format!(
                    "    - {} :: {} at {}",
                    callee.symbol.kind, callee.symbol.name, callee.symbol.file_path
                ));
            }
        }

        let message = if lines.len() == 1 {
            format!("{} (no relationships found)", lines[0])
        } else {
            lines.join("\n")
        };

        Ok(splice::cli::CliSuccessPayload::message_only(message))
    }
}

/// Execute the files command.
///
/// Lists all indexed files.
fn execute_files(
    db_path: &Path,
    with_symbol_counts: bool,
    _output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::MagellanIntegration;

    let mut integration = MagellanIntegration::open(db_path)?;
    let files = integration.list_indexed_files(with_symbol_counts)?;

    let count = files.len();

    if json_output {
        let files_data: Vec<serde_json::Value> = files
            .iter()
            .map(|f| {
                let mut obj = serde_json::json!({
                    "path": f.path,
                    "hash": f.hash,
                    "last_indexed_at": f.last_indexed_at,
                    "last_modified": f.last_modified,
                });
                if let Some(symbol_count) = f.symbol_count {
                    obj["symbol_count"] = serde_json::json!(symbol_count);
                }
                obj
            })
            .collect();

        Ok(splice::cli::CliSuccessPayload::with_data(
            format!("{} indexed files", count),
            serde_json::json!({ "files": files_data, "count": count }),
        ))
    } else {
        let lines: Vec<String> = files
            .iter()
            .map(|f| {
                if let Some(cnt) = f.symbol_count {
                    format!("  {} ({} symbols)", f.path, cnt)
                } else {
                    format!("  {}", f.path)
                }
            })
            .collect();
        let message = format!("{} indexed files:\n{}", count, lines.join("\n"));
        Ok(splice::cli::CliSuccessPayload::message_only(message))
    }
}

/// Execute the export command.
///
/// Exports graph data (files, symbols, references, calls) in the specified format.
fn execute_export(
    db_path: &Path,
    format: splice::cli::ExportFormat,
    output: Option<&Path>,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::MagellanIntegration;
    use splice::output::{ExportData, ExportResponse, EXPORT_SCHEMA_VERSION};
    use splice::symbol_id::generate_symbol_id;

    // Open Magellan integration
    let mut integration = MagellanIntegration::open(db_path)?;

    // Collect all graph data
    let files = integration.list_indexed_files(false)?;

    // Collect symbols from all files (limited to avoid OOM)
    let mut all_symbols = Vec::new();

    // Collect symbols from first 100 files (for safety)
    for file_metadata in files.iter().take(100) {
        let file_path = std::path::PathBuf::from(&file_metadata.path);
        if let Ok(symbols) = integration.query_symbols_by_file(&file_path, None, false, false) {
            for swr in symbols {
                let sym = swr.symbol;
                let symbol_id = generate_symbol_id(&sym.name, &sym.file_path, sym.byte_start);
                all_symbols.push(splice::output::SymbolExport {
                    symbol_id: symbol_id.to_string(),
                    id_format: Some(if symbol_id.is_v1() { "v1" } else { "v2" }.to_string()),
                    name: sym.name,
                    kind: sym.kind,
                    file_path: sym.file_path,
                    byte_start: sym.byte_start,
                    byte_end: sym.byte_end,
                    start_line: 0,
                    end_line: 0,
                    start_col: 0,
                    end_col: 0,
                });
            }
        }
    }

    // Build export response
    let export_data = ExportData {
        files: files
            .iter()
            .map(|f| splice::output::FileExport {
                path: f.path.clone(),
                hash: f.hash.clone(),
                last_indexed_at: f.last_indexed_at,
                last_modified: f.last_modified,
            })
            .collect(),
        symbols: all_symbols,
        references: vec![], // References require more complex queries
        calls: vec![],      // Calls require more complex queries
    };

    let response = ExportResponse {
        schema_version: EXPORT_SCHEMA_VERSION.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        db_path: db_path.to_string_lossy().to_string(),
        data: export_data,
    };

    // Write output
    if let Some(path) = output {
        let file = std::fs::File::create(path).map_err(|source| splice::SpliceError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let writer = std::io::BufWriter::new(file);
        write_export(&response, format, writer)?;
    } else {
        let stdout = std::io::stdout();
        let writer = std::io::BufWriter::new(stdout.lock());
        write_export(&response, format, writer)?;
    }

    let file_count = response.data.files.len();
    let symbol_count = response.data.symbols.len();

    Ok(splice::cli::CliSuccessPayload::with_data(
        format!("Exported {} files, {} symbols", file_count, symbol_count),
        serde_json::json!({"files": file_count, "symbols": symbol_count}),
    ))
}

/// Write export data in the specified format.
fn write_export<W: std::io::Write>(
    response: &splice::output::ExportResponse,
    format: splice::cli::ExportFormat,
    mut writer: std::io::BufWriter<W>,
) -> Result<(), splice::SpliceError> {
    use std::io::Write;

    match format {
        splice::cli::ExportFormat::Json => {
            serde_json::to_writer_pretty(&mut writer, response).map_err(|e| {
                splice::SpliceError::Other(format!("JSON serialization error: {}", e))
            })?;
        }
        splice::cli::ExportFormat::Jsonl => {
            // Write header with schema version
            writeln!(
                writer,
                r#"{{"schema_version": "{}", "type": "header"}}"#,
                response.schema_version
            )
            .map_err(|e| splice::SpliceError::Other(format!("Write error: {}", e)))?;

            // Write files with type tag
            for file in &response.data.files {
                let json = serde_json::to_string(file).map_err(|e| {
                    splice::SpliceError::Other(format!("JSON serialization error: {}", e))
                })?;
                writeln!(writer, r#"{{"type": "file", "data": {}}}"#, json)
                    .map_err(|e| splice::SpliceError::Other(format!("Write error: {}", e)))?;
            }

            // Write symbols with type tag
            for symbol in &response.data.symbols {
                let json = serde_json::to_string(symbol).map_err(|e| {
                    splice::SpliceError::Other(format!("JSON serialization error: {}", e))
                })?;
                writeln!(writer, r#"{{"type": "symbol", "data": {}}}"#, json)
                    .map_err(|e| splice::SpliceError::Other(format!("Write error: {}", e)))?;
            }
        }
        splice::cli::ExportFormat::Csv => {
            use csv::Writer;

            // Write files section
            writeln!(writer, "# Files")
                .map_err(|e| splice::SpliceError::Other(format!("Write error: {}", e)))?;
            {
                let mut wtr = Writer::from_writer(&mut writer);
                for file in &response.data.files {
                    wtr.serialize(file).map_err(|e| {
                        splice::SpliceError::Other(format!("CSV write error: {}", e))
                    })?;
                }
                wtr.flush()
                    .map_err(|e| splice::SpliceError::Other(format!("CSV flush error: {}", e)))?;
            }

            // Write symbols section
            writeln!(writer, "\n# Symbols")
                .map_err(|e| splice::SpliceError::Other(format!("Write error: {}", e)))?;
            {
                let mut wtr = Writer::from_writer(&mut writer);
                for symbol in &response.data.symbols {
                    wtr.serialize(symbol).map_err(|e| {
                        splice::SpliceError::Other(format!("CSV write error: {}", e))
                    })?;
                }
                wtr.flush()
                    .map_err(|e| splice::SpliceError::Other(format!("CSV flush error: {}", e)))?;
            }
        }
    }

    writer
        .flush()
        .map_err(|e| splice::SpliceError::Other(format!("Flush error: {}", e)))?;

    Ok(())
}

/// Execute the migrate-db command.
///
/// This function:
/// 1. Checks migration status in dry-run mode
/// 2. Creates backup if requested (default: true)
/// 3. Opens database (triggers Magellan auto-migration)
/// 4. Reports migration results
fn execute_migrate_db(
    db_path: &Path,
    backup: bool,
    dry_run: bool,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::migrate::{check_schema_version, migrate_database};

    if dry_run {
        // Check migration status without migrating
        match check_schema_version(db_path) {
            Ok(version) => {
                let needs_migration = version < 6;

                if needs_migration {
                    println!("Current schema: v{}", version);
                    println!("Target schema: v6");
                    println!("Migration required: yes");
                    println!(
                        "\nTo migrate, run: splice migrate-db --db-path {}",
                        db_path.display()
                    );
                } else {
                    println!("Current schema: v{}", version);
                    println!("Target schema: v6");
                    println!("Migration required: no (already on v6 or later)");
                }

                return Ok(splice::cli::CliSuccessPayload::message_only(format!(
                    "Schema check complete: v{}",
                    version
                )));
            }
            Err(e) => {
                return Err(splice::SpliceError::Other(format!(
                    "Error checking schema version: {}",
                    e
                )))
            }
        }
    }

    // Perform migration
    match migrate_database(db_path, backup, false) {
        Ok(result) => {
            if let Some(ref backup_path) = result.backup_path {
                println!("Backup created: {}", backup_path.display());
            }
            println!(
                "Database migrated: v{} -> v{}",
                result.previous_version, result.new_version
            );
            println!("You can now use Magellan 2.0.0 features");

            Ok(splice::cli::CliSuccessPayload::with_data(
                format!(
                    "Migrated database: v{} -> v{}",
                    result.previous_version, result.new_version
                ),
                serde_json::json!({
                    "previous_version": result.previous_version,
                    "new_version": result.new_version,
                    "backup_path": result.backup_path,
                    "symbols_migrated": result.symbols_migrated,
                }),
            ))
        }
        Err(e) => Err(splice::SpliceError::Other(format!(
            "Migration failed: {}",
            e
        ))),
    }
}

/// Execute the rename command.
///
/// This function is a stub for the cross-file rename implementation.
/// Full implementation will be provided in plan 29-02.
///
/// Current stub behavior:
/// - Validates that either --symbol or --name+--file is provided
/// - Opens Magellan database
/// - Resolves symbol (ID-first with name+path fallback)
/// - Validates symbol exists and has references
///
/// # Arguments
/// * `symbol_id` - Optional 32-char BLAKE3 or 16-char SHA-256 symbol ID
/// * `name` - Optional symbol name (requires file)
/// * `file` - Optional file path for symbol name resolution
/// * `new_name` - New name for the symbol
/// * `db_path` - Path to Magellan database
/// * `preview` - Preview changes without applying
/// * `backup_dir` - Optional override for backup directory
/// * `no_backup` - Skip backup creation
/// * `json_output` - Output JSON format
///
/// # Returns
/// Result with success payload or error
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
fn execute_rename(
    symbol_id: Option<&str>,
    name: Option<&str>,
    file: Option<&PathBuf>,
    new_name: &str,
    db_path: &Path,
    preview: bool,
    proof: bool,
    _backup_dir: Option<&PathBuf>,
    no_backup: bool,
    snapshot_before: bool,
    impact_graph: bool,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::MagellanIntegration;
    use splice::proof::generation::generate_snapshot;
    use splice::proof::{generate_proof, write_proof};

    // Capture snapshot before operation if requested
    if snapshot_before {
        if let Err(e) = capture_snapshot(db_path, "rename") {
            eprintln!("Warning: Failed to capture snapshot: {}", e);
        }
    }

    // Validate input: either --symbol or (--name AND --file) must be provided
    let (lookup_id, lookup_name, lookup_file) = match (symbol_id, name, file) {
        (Some(id), None, None) => (Some(id), None, None),
        (None, Some(n), Some(f)) => (None, Some(n), Some(f)),
        (None, None, _) => {
            return Err(splice::SpliceError::RenameFailed {
                reason: "Either --symbol or --name (with --file) must be provided".to_string(),
                symbol: new_name.to_string(),
            });
        }
        (Some(_), Some(_), _) => {
            return Err(splice::SpliceError::RenameFailed {
                reason: "--symbol and --name are mutually exclusive".to_string(),
                symbol: new_name.to_string(),
            });
        }
        (None, Some(_), None) => {
            return Err(splice::SpliceError::RenameFailed {
                reason: "--file is required when using --name".to_string(),
                symbol: name.unwrap().to_string(),
            });
        }
        _ => {
            return Err(splice::SpliceError::RenameFailed {
                reason: if symbol_id.is_some() && file.is_some() {
                    "--file is not needed with --symbol (the symbol ID uniquely identifies the target)".to_string()
                } else if name.is_some() && file.is_none() {
                    "--file is required when using --name".to_string()
                } else {
                    "Provide either --symbol <id> or --name <name> --file <path>".to_string()
                },
                symbol: new_name.to_string(),
            });
        }
    };

    // Open Magellan database
    let mut magellan =
        MagellanIntegration::open(db_path).map_err(|e| splice::SpliceError::RenameFailed {
            reason: format!("Failed to open database: {}", e),
            symbol: new_name.to_string(),
        })?;

    // Resolve symbol (ID-first with name+path fallback)
    let symbol_info = if let Some(id) = lookup_id {
        // Lookup by symbol ID (32-char BLAKE3 or 16-char SHA-256)
        magellan
            .find_symbol_by_id(id)
            .map_err(|e| splice::SpliceError::RenameFailed {
                reason: format!("Failed to lookup symbol ID: {}", e),
                symbol: id.to_string(),
            })?
            .ok_or_else(|| splice::SpliceError::RenameFailed {
                reason: format!("Symbol ID '{}' not found in database", id),
                symbol: id.to_string(),
            })?
    } else {
        // Lookup by name+path
        let name_str = lookup_name.unwrap();
        let file_path = lookup_file.unwrap();

        // First, find ALL matches (ambiguous=true) to provide complete error context
        let mut all_matches = magellan.find_symbol_by_name(name_str, true).map_err(|e| {
            splice::SpliceError::RenameFailed {
                reason: format!("Failed to lookup symbol name: {}", e),
                symbol: name_str.to_string(),
            }
        })?;

        if all_matches.is_empty() {
            return Err(splice::SpliceError::RenameFailed {
                reason: format!(
                    "Symbol '{}' not found in file '{}'",
                    name_str,
                    file_path.display()
                ),
                symbol: name_str.to_string(),
            });
        }

        // Check if symbol name is ambiguous across multiple files
        if all_matches.len() > 1 {
            // Filter to just the specified file
            let file_path_str = file_path.to_string_lossy().to_string();
            all_matches.retain(|s| s.file_path == file_path_str);

            if all_matches.is_empty() {
                // Symbol exists in other files but not in the specified one
                let all_files: Vec<String> = magellan
                    .find_symbol_by_name(name_str, true)
                    .map_err(|e| splice::SpliceError::RenameFailed {
                        reason: format!("Failed to lookup symbol name: {}", e),
                        symbol: name_str.to_string(),
                    })?
                    .into_iter()
                    .map(|s| s.file_path)
                    .collect();

                return Err(splice::SpliceError::RenameFailed {
                    reason: format!(
                        "Symbol '{}' not found in file '{}' (found in {} other file(s): {})",
                        name_str,
                        file_path.display(),
                        all_files.len(),
                        all_files.join(", ")
                    ),
                    symbol: name_str.to_string(),
                });
            }

            if all_matches.len() > 1 {
                // Still multiple matches in the same file - return AmbiguousSymbol error
                let files: Vec<String> = all_matches
                    .iter()
                    .map(|s| format!("{}:{}", s.file_path, s.kind))
                    .collect();
                return Err(splice::SpliceError::AmbiguousSymbol {
                    name: name_str.to_string(),
                    files,
                });
            }
        }

        all_matches.remove(0)
    };

    // Generate before snapshot if proof generation is requested
    let before_snapshot = if proof {
        Some(
            generate_snapshot(db_path).map_err(|e| splice::SpliceError::RenameFailed {
                reason: format!("Failed to generate before snapshot: {}", e),
                symbol: new_name.to_string(),
            })?,
        )
    } else {
        None
    };

    // Pre-flight validation: symbol must exist and have references
    let entity_id = symbol_info.entity_id;
    let mut references =
        magellan
            .get_all_references(entity_id)
            .map_err(|e| splice::SpliceError::RenameFailed {
                reason: format!("Failed to get references: {}", e),
                symbol: symbol_info.name.clone(),
            })?;

    // Include the definition *name* site in the rename.
    // Magellan's byte_start/byte_end covers the full declaration body.
    // We locate the first occurrence of the symbol name within that span
    // to produce a name-only offset that won't corrupt the declaration.
    let decl_content =
        fs::read(&symbol_info.file_path).map_err(|e| splice::SpliceError::RenameFailed {
            reason: format!(
                "Failed to read definition file '{}': {}",
                symbol_info.file_path, e
            ),
            symbol: symbol_info.name.clone(),
        })?;
    let decl_slice = &decl_content[symbol_info.byte_start..symbol_info.byte_end];
    if let Some(name_offset) = decl_slice
        .windows(symbol_info.name.len())
        .position(|w| w == symbol_info.name.as_bytes())
    {
        let name_byte_start = symbol_info.byte_start + name_offset;
        let name_byte_end = name_byte_start + symbol_info.name.len();
        references.push(magellan::references::ReferenceFact {
            file_path: PathBuf::from(&symbol_info.file_path),
            referenced_symbol: symbol_info.name.clone(),
            byte_start: name_byte_start,
            byte_end: name_byte_end,
            start_line: symbol_info.start_line.unwrap_or(1),
            start_col: 0,
            end_line: symbol_info.end_line.unwrap_or(1),
            end_col: 0,
        });
    } else {
        return Err(splice::SpliceError::RenameFailed {
            reason: format!(
                "Symbol name '{}' not found inside its own declaration span ({}..{}) in {}",
                symbol_info.name,
                symbol_info.byte_start,
                symbol_info.byte_end,
                symbol_info.file_path
            ),
            symbol: symbol_info.name,
        });
    }

    // Sort references for safe in-order replacement
    // Descending by byte_start within each file prevents offset shifts
    splice::graph::MagellanIntegration::sort_references_for_replacement(&mut references);

    // Group references by file
    use splice::graph::rename;
    let grouped = rename::group_references_by_file(&references);

    // Calculate stats for preview
    let files_affected: Vec<&PathBuf> = grouped.keys().collect();
    let total_references = references.len();

    // PREVIEW MODE: Generate unified diffs without filesystem writes
    // Preview is pure - no backup creation, no file modifications
    if preview {
        // Generate impact graph if requested
        if impact_graph {
            use splice::cli::ReachabilityDirection;
            use splice::graph::magellan_integration::ImpactDotConfig;
            let symbol_id = format!("{}:{}", symbol_info.file_path, symbol_info.name);
            let config = ImpactDotConfig {
                show_symbol_kinds: true,
                max_depth: Some(10),
                highlight_symbol: Some(symbol_info.name.clone()),
            };
            let dot =
                magellan.generate_impact_dot(&symbol_id, &ReachabilityDirection::Both, &config)?;
            println!("{}", dot);
        }

        let mut diffs = Vec::new();
        for (file_path, refs) in &grouped {
            let content = fs::read_to_string(file_path).map_err(|e| splice::SpliceError::Io {
                path: file_path.clone(),
                source: e,
            })?;
            let modified =
                rename::simulate_replacements_content(&content, refs, &symbol_info.name, new_name)?;
            let diff = rename::generate_colored_preview(file_path, &content, &modified);
            diffs.push(diff);
        }

        let summary = format!(
            "Preview: {} files, {} references\n\n{}",
            files_affected.len(),
            total_references,
            diffs.join("\n")
        );
        return Ok(splice::cli::CliSuccessPayload::message_only(summary).with_pending_changes());
    }

    // DETERMINE WORKSPACE ROOT
    let workspace_root = std::env::current_dir()
        .map_err(|e| splice::SpliceError::Other(format!("Failed to get workspace root: {}", e)))?;

    // CREATE BACKUP (unless skipped)
    let backup_dir_path = if !no_backup {
        let files_to_backup: Vec<PathBuf> = files_affected.iter().map(|p| (**p).clone()).collect();
        Some(rename::create_rename_backup(
            &workspace_root,
            symbol_id.unwrap_or("unknown"),
            &files_to_backup,
        )?)
    } else {
        None
    };

    // APPLY REPLACEMENTS WITH TRANSACTION
    let mut transaction = rename::RenameTransaction::new();
    if let Some(backup_path) = backup_dir_path.as_ref() {
        transaction = transaction.with_backup(backup_path.clone(), workspace_root.clone());
    }

    let mut modified_count = 0;
    let mut last_error: Option<splice::SpliceError> = None;

    for (file_path, refs) in grouped {
        match rename::apply_replacements_in_file(&file_path, &symbol_info.name, new_name, &refs) {
            Ok(count) => {
                if count > 0 {
                    modified_count += 1;
                    transaction.track_modified(file_path.to_path_buf());
                }
            }
            Err(e) => {
                // Rollback on error
                last_error = Some(e);
                break;
            }
        }
    }

    // If any file failed, rollback the entire transaction
    if let Some(error) = last_error {
        let _ = transaction.rollback(); // Best-effort rollback
        return Err(error);
    }

    let message = if let Some(ref backup_path) = backup_dir_path {
        format!(
            "Renamed '{}' to '{}' in {} files\nBackup: {}",
            symbol_info.name,
            new_name,
            modified_count,
            backup_path.display()
        )
    } else {
        format!(
            "Renamed '{}' to '{}' in {} files (no backup)",
            symbol_info.name, new_name, modified_count
        )
    };

    // Generate after snapshot and write proof if requested
    if proof {
        if let Some(before) = before_snapshot {
            let after_snapshot =
                generate_snapshot(db_path).map_err(|e| splice::SpliceError::RenameFailed {
                    reason: format!("Failed to generate after snapshot: {}", e),
                    symbol: new_name.to_string(),
                })?;

            // Use generate_proof to create complete proof with invariant validation
            let proof_data =
                generate_proof("rename", db_path, before, after_snapshot).map_err(|e| {
                    splice::SpliceError::RenameFailed {
                        reason: format!("Failed to generate proof: {}", e),
                        symbol: new_name.to_string(),
                    }
                })?;

            // Check invariant validation results
            let failed_invariants: Vec<_> =
                proof_data.invariants.iter().filter(|c| !c.passed).collect();

            let invariant_status = if failed_invariants.is_empty() {
                "All invariants passed".to_string()
            } else {
                let failed_names: Vec<&str> = failed_invariants
                    .iter()
                    .map(|c| c.invariant_name.as_str())
                    .collect();
                format!(
                    "Warning: {} invariant(s) failed: {}",
                    failed_invariants.len(),
                    failed_names.join(", ")
                )
            };

            let proof_dir = std::path::PathBuf::from(".splice/proofs");
            let proof_path = write_proof(&proof_data, &proof_dir).map_err(|e| {
                splice::SpliceError::RenameFailed {
                    reason: format!("Failed to write proof: {}", e),
                    symbol: new_name.to_string(),
                }
            })?;

            return Ok(splice::cli::CliSuccessPayload::with_data(
                format!(
                    "{}\nProof written to: {}\nInvariant status: {}",
                    message,
                    proof_path.display(),
                    invariant_status
                ),
                serde_json::json!({
                    "old_name": symbol_info.name,
                    "new_name": new_name,
                    "files_modified": modified_count,
                    "total_references": total_references,
                    "backup": backup_dir_path.as_ref().map(|p| p.display().to_string()),
                    "proof": proof_path.display().to_string(),
                    "invariants": {
                        "total": proof_data.invariants.len(),
                        "passed": proof_data.invariants.iter().filter(|c| c.passed).count(),
                        "failed": failed_invariants.len(),
                        "details": proof_data.invariants.iter().map(|c| serde_json::json!({
                            "name": c.invariant_name,
                            "passed": c.passed,
                            "violations": c.violations.len()
                        })).collect::<Vec<_>>()
                    }
                }),
            ));
        }
    }

    Ok(splice::cli::CliSuccessPayload::with_data(
        message,
        serde_json::json!({
            "old_name": symbol_info.name,
            "new_name": new_name,
            "files_modified": modified_count,
            "total_references": total_references,
            "backup": backup_dir_path.as_ref().map(|p| p.display().to_string()),
        }),
    ))
}

/// Parse date string to Unix timestamp.
///
/// Accepts either Unix timestamp (integer) or ISO 8601 format.
fn parse_date(input: &str) -> Result<i64, splice::SpliceError> {
    use splice::SpliceError;

    // Try Unix timestamp first
    if let Ok(ts) = input.parse::<i64>() {
        return Ok(ts);
    }

    // Try ISO 8601
    chrono::DateTime::parse_from_rfc3339(input)
        .map(|dt| dt.timestamp())
        .map_err(|_| SpliceError::InvalidDateFormat {
            input: input.to_string(),
        })
}

/// Execute reachability analysis for a symbol.
///
/// # Arguments
/// * `symbol` - Symbol name to analyze
/// * `path` - File path containing the symbol
/// * `db_path` - Path to Magellan database
/// * `direction` - Analysis direction (forward/reverse/both)
/// * `max_depth` - Maximum traversal depth
/// * `output` - Output format
/// * `json_output` - Whether to output JSON
///
/// # Returns
/// Result with success payload or error
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
fn execute_reachable(
    symbol: &str,
    path: &Path,
    db_path: &Path,
    direction: &splice::cli::ReachabilityDirection,
    max_depth: usize,
    output: splice::cli::OutputFormat,
    impact_graph: bool,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::MagellanIntegration;
    use splice::output::{AffectedFile, ReachabilityChain, ReachabilityResult, SymbolInfo};

    // Get path string early
    let path_str = path
        .to_str()
        .ok_or_else(|| splice::SpliceError::Other("Invalid UTF-8 in path".to_string()))?;

    // Handle impact graph generation
    if impact_graph {
        return execute_impact_graph(
            db_path,
            &format!("{}:{}", path_str, symbol),
            direction,
            Some(max_depth),
        );
    }

    // Open database for operations
    let mut integration = MagellanIntegration::open(db_path)?;

    // Get root symbol info using backend-neutral method
    let root_symbol_info = match integration.find_symbol_by_path_and_name(path, symbol)? {
        Some(info) => info,
        None => {
            return Err(splice::SpliceError::SymbolNotFound {
                message: format!("Symbol '{}' not found in '{}'", symbol, path_str),
                symbol: symbol.to_string(),
                file: Some(path.to_path_buf()),
                hint: format!("Run 'splice find --name {}' to locate the symbol", symbol),
            });
        }
    };

    // Collect reachability based on direction
    let (forward_symbols, reverse_symbols) = match direction {
        splice::cli::ReachabilityDirection::Forward => {
            let symbols = integration.reachable_symbols(path, symbol, max_depth)?;
            (symbols, Vec::new())
        }
        splice::cli::ReachabilityDirection::Reverse => {
            let symbols = integration.reverse_reachable_symbols(path, symbol, max_depth)?;
            (Vec::new(), symbols)
        }
        splice::cli::ReachabilityDirection::Both => {
            let forward = integration.reachable_symbols(path, symbol, max_depth)?;
            let reverse = integration.reverse_reachable_symbols(path, symbol, max_depth)?;
            (forward, reverse)
        }
    };

    // Build affected files list
    let mut affected_files = std::collections::HashMap::new();
    affected_files.insert(root_symbol_info.file_path.clone(), (0, true)); // root file

    for reachable in &forward_symbols {
        let entry = affected_files
            .entry(reachable.symbol.file_path.clone())
            .or_insert((0, false));
        entry.0 += 1;
    }
    for reachable in &reverse_symbols {
        let entry = affected_files
            .entry(reachable.symbol.file_path.clone())
            .or_insert((0, false));
        entry.0 += 1;
    }

    let affected_files_vec: Vec<AffectedFile> = affected_files
        .into_iter()
        .map(|(path, (count, is_root))| AffectedFile {
            path,
            symbol_count: count,
            is_root,
        })
        .collect();

    // Build result
    let result = ReachabilityResult {
        symbol: SymbolInfo {
            symbol_id: None, // Can be added later if needed
            id_format: None,
            name: root_symbol_info.name.clone(),
            kind: root_symbol_info.kind.clone(),
            file_path: root_symbol_info.file_path.clone(),
            byte_start: root_symbol_info.byte_start,
            byte_end: root_symbol_info.byte_end,
        },
        direction: format!("{:?}", direction).to_lowercase(),
        max_depth,
        forward: if forward_symbols.is_empty() {
            None
        } else {
            Some(ReachabilityChain {
                count: forward_symbols.len(),
                depth: forward_symbols.iter().map(|s| s.depth).max().unwrap_or(0),
                symbols: forward_symbols
                    .into_iter()
                    .map(|s| splice::output::ReachableSymbol {
                        symbol: SymbolInfo {
                            symbol_id: None,
                            id_format: None,
                            name: s.symbol.name,
                            kind: s.symbol.kind,
                            file_path: s.symbol.file_path,
                            byte_start: s.symbol.byte_start,
                            byte_end: s.symbol.byte_end,
                        },
                        depth: s.depth,
                        path: s.path,
                    })
                    .collect(),
            })
        },
        reverse: if reverse_symbols.is_empty() {
            None
        } else {
            Some(ReachabilityChain {
                count: reverse_symbols.len(),
                depth: reverse_symbols.iter().map(|s| s.depth).max().unwrap_or(0),
                symbols: reverse_symbols
                    .into_iter()
                    .map(|s| splice::output::ReachableSymbol {
                        symbol: SymbolInfo {
                            symbol_id: None,
                            id_format: None,
                            name: s.symbol.name,
                            kind: s.symbol.kind,
                            file_path: s.symbol.file_path,
                            byte_start: s.symbol.byte_start,
                            byte_end: s.symbol.byte_end,
                        },
                        depth: s.depth,
                        path: s.path,
                    })
                    .collect(),
            })
        },
        affected_files: affected_files_vec,
    };

    // Format output
    if output.is_json() || json_output {
        let json = output
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json);
        Ok(splice::cli::CliSuccessPayload::message_only(
            "Reachability analysis complete".to_string(),
        )
        .already_emitted())
    } else {
        // Human-readable output
        println!("Reachability Analysis for '{}' in {}", symbol, path_str);
        println!("Direction: {:?}", direction);
        println!("Max Depth: {}", max_depth);
        println!();

        if let Some(ref forward) = result.forward {
            println!(
                "Forward Reachability ({} callees, depth {}):",
                forward.count, forward.depth
            );
            for s in &forward.symbols {
                println!(
                    "  {} (depth {}): {}",
                    s.symbol.name, s.depth, s.symbol.file_path
                );
            }
            println!();
        }

        if let Some(ref reverse) = result.reverse {
            println!(
                "Reverse Reachability ({} callers, depth {}):",
                reverse.count, reverse.depth
            );
            for s in &reverse.symbols {
                println!(
                    "  {} (depth {}): {}",
                    s.symbol.name, s.depth, s.symbol.file_path
                );
            }
            println!();
        }

        println!("Affected Files ({}):", result.affected_files.len());
        for file in &result.affected_files {
            let marker = if file.is_root { " [root]" } else { "" };
            println!("  {}{} ({} symbols)", file.path, marker, file.symbol_count);
        }

        Ok(splice::cli::CliSuccessPayload::message_only(
            "Reachability analysis complete".to_string(),
        ))
    }
}

/// Execute the cycles command.
///
/// Detects cycles in the call graph using Tarjan's SCC algorithm.
fn execute_cycles(
    db_path: &Path,
    symbol: Option<&str>,
    path: Option<&PathBuf>,
    max_cycles: usize,
    show_members: bool,
    output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::MagellanIntegration;
    use splice::output::{CycleDetectionResult, CycleInfo, SymbolInfo};

    let mut integration = MagellanIntegration::open(db_path)?;

    let (cycles, queried_symbol) = if let (Some(sym_name), Some(sym_path)) = (symbol, path) {
        // Find cycles containing specific symbol
        let cycles = integration.find_cycles_containing(sym_path, sym_name, max_cycles)?;

        // Get queried symbol info using backend-neutral method
        let queried_symbol = match integration.find_symbol_by_path_and_name(sym_path, sym_name)? {
            Some(info) => Some(SymbolInfo {
                symbol_id: None,
                id_format: None,
                name: info.name,
                kind: info.kind,
                file_path: info.file_path,
                byte_start: info.byte_start,
                byte_end: info.byte_end,
            }),
            None => None,
        };

        (cycles, queried_symbol)
    } else {
        // Find all cycles
        (integration.detect_cycles(max_cycles)?, None)
    };

    let total_cycles = cycles.len();
    let truncated = total_cycles >= max_cycles;

    let result_cycles: Vec<CycleInfo> = cycles
        .into_iter()
        .map(|c| CycleInfo {
            id: c.id,
            size: c.size,
            members: c
                .members
                .into_iter()
                .map(|s| SymbolInfo {
                    symbol_id: None,
                    id_format: None,
                    name: s.name,
                    kind: s.kind,
                    file_path: s.file_path,
                    byte_start: s.byte_start,
                    byte_end: s.byte_end,
                })
                .collect(),
            representative: SymbolInfo {
                symbol_id: None,
                id_format: None,
                name: c.representative.name,
                kind: c.representative.kind,
                file_path: c.representative.file_path,
                byte_start: c.representative.byte_start,
                byte_end: c.representative.byte_end,
            },
            is_self_loop: c.is_self_loop,
        })
        .collect();

    let result = CycleDetectionResult {
        total_cycles,
        max_cycles,
        truncated,
        cycles: result_cycles,
        queried_symbol,
    };

    // Format output
    if output.is_json() || json_output {
        let json = output
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json);
        Ok(
            splice::cli::CliSuccessPayload::message_only("Cycle detection complete".to_string())
                .already_emitted(),
        )
    } else {
        // Human-readable output
        if let Some(ref qs) = result.queried_symbol {
            println!("Cycles containing '{}' in {}", qs.name, qs.file_path);
        } else {
            println!("Call Graph Cycle Detection");
        }
        println!();

        if result.total_cycles == 0 {
            println!("No cycles detected in the call graph.");
        } else {
            println!("Found {} cycle(s):", result.total_cycles);
            if result.truncated {
                println!("(showing first {} cycles)", result.max_cycles);
            }
            println!();

            for cycle in &result.cycles {
                println!("Cycle {} [{}]:", cycle.id, cycle.size);
                println!(
                    "  Representative: {} ({})",
                    cycle.representative.name, cycle.representative.kind
                );
                if cycle.is_self_loop {
                    println!("  Type: Self-loop");
                }
                if show_members {
                    println!("  Members:");
                    for member in &cycle.members {
                        println!("    - {} in {}", member.name, member.file_path);
                    }
                }
                println!();
            }
        }

        Ok(splice::cli::CliSuccessPayload::message_only(
            "Cycle detection complete".to_string(),
        ))
    }
}

/// Execute the condense command.
///
/// Analyzes the condensation graph (SCCs collapsed to DAG).
fn execute_condense(
    db_path: &Path,
    show_members: bool,
    show_levels: bool,
    output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::MagellanIntegration;
    use splice::output::{CondensationResult, CondensedScc, LevelInfo, SccEdge, SymbolInfo};

    let mut integration = MagellanIntegration::open(db_path)?;
    let graph = integration.condense_graph()?;

    // Build SCC result with optional members
    let sccs: Vec<CondensedScc> = graph
        .sccs
        .into_iter()
        .map(|scc| {
            let members = if show_members {
                // Members would need to be tracked during condensation
                // For now, leave as None - could be populated from scc_members
                None
            } else {
                None
            };

            CondensedScc {
                id: scc.id,
                size: scc.size,
                is_cycle: scc.is_cycle,
                members,
                representative: SymbolInfo {
                    symbol_id: None,
                    id_format: None,
                    name: scc.representative.name,
                    kind: scc.representative.kind,
                    file_path: scc.representative.file_path,
                    byte_start: scc.representative.byte_start,
                    byte_end: scc.representative.byte_end,
                },
            }
        })
        .collect();

    let edges: Vec<SccEdge> = graph
        .edges
        .into_iter()
        .map(|e| SccEdge {
            from: e.from,
            to: e.to,
            weight: e.weight,
        })
        .collect();

    let levels: Option<Vec<LevelInfo>> = if show_levels {
        Some(
            graph
                .levels
                .into_iter()
                .map(|l| LevelInfo {
                    level: l.level,
                    scc_ids: l.scc_ids,
                    count: l.count,
                })
                .collect(),
        )
    } else {
        None
    };

    let result = CondensationResult {
        scc_count: graph.scc_count,
        cycle_scc_count: graph.cycle_scc_count,
        singleton_count: graph.singleton_count,
        sccs,
        edges,
        levels,
    };

    // Format output
    if output.is_json() || json_output {
        let json = output
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json);
        Ok(splice::cli::CliSuccessPayload::message_only(
            "Condensation analysis complete".to_string(),
        )
        .already_emitted())
    } else {
        // Human-readable output
        println!("Condensation Graph Analysis");
        println!();
        println!("Summary:");
        println!("  Total SCCs: {}", result.scc_count);
        println!("  Cycle SCCs (size > 1): {}", result.cycle_scc_count);
        println!("  Singleton SCCs: {}", result.singleton_count);
        println!();

        if show_levels {
            if let Some(ref levels) = result.levels {
                println!("Topological Levels:");
                for level in levels {
                    println!("  Level {} ({} SCCs):", level.level, level.count);
                    for scc_id in &level.scc_ids {
                        if let Some(scc) = result.sccs.iter().find(|s| &s.id == scc_id) {
                            println!(
                                "    {} - {} (size: {}, cycle: {})",
                                scc.id, scc.representative.name, scc.size, scc.is_cycle
                            );
                        }
                    }
                }
                println!();
            }
        }

        println!("Cycle SCCs:");
        let cycle_sccs: Vec<_> = result.sccs.iter().filter(|s| s.is_cycle).collect();
        if cycle_sccs.is_empty() {
            println!("  (none)");
        } else {
            for scc in cycle_sccs {
                println!(
                    "  {} - {} (size: {})",
                    scc.id, scc.representative.name, scc.size
                );
            }
        }
        println!();

        println!("Edges in Condensed Graph: {}", result.edges.len());

        Ok(splice::cli::CliSuccessPayload::message_only(
            "Condensation analysis complete".to_string(),
        ))
    }
}

/// Execute the slice command.
///
/// Performs forward or backward program slicing for impact analysis.
fn execute_slice(
    target: &str,
    path: &Path,
    db_path: &Path,
    direction: &splice::cli::SliceDirection,
    max_depth: Option<usize>,
    output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::MagellanIntegration;
    use splice::output::{AffectedFile, SliceResult, SliceStats, SlicedSymbol, SymbolInfo};
    use std::collections::HashMap;

    let mut integration = MagellanIntegration::open(db_path)?;

    // Perform slice
    let sliced_symbols = match direction {
        splice::cli::SliceDirection::Forward => {
            integration.forward_slice(path, target, max_depth)?
        }
        splice::cli::SliceDirection::Backward => {
            integration.backward_slice(path, target, max_depth)?
        }
    };

    // Get target symbol info using backend-neutral method
    let target_symbol_info = match integration.find_symbol_by_path_and_name(path, target)? {
        Some(info) => info,
        None => {
            return Err(splice::SpliceError::Other(
                "Target symbol not found".to_string(),
            ));
        }
    };

    let target_symbol = SymbolInfo {
        symbol_id: None,
        id_format: None,
        name: target_symbol_info.name,
        kind: target_symbol_info.kind,
        file_path: target_symbol_info.file_path,
        byte_start: target_symbol_info.byte_start,
        byte_end: target_symbol_info.byte_end,
    };

    // Compute affected files
    let mut affected_files: HashMap<String, usize> = HashMap::new();
    for ss in &sliced_symbols {
        *affected_files
            .entry(ss.symbol.file_path.clone())
            .or_insert(0) += 1;
    }

    let target_file_path = target_symbol.file_path.clone();
    let affected_files_result: Vec<AffectedFile> = affected_files
        .into_iter()
        .map(|(path, count)| AffectedFile {
            is_root: path == target_file_path,
            path,
            symbol_count: count,
        })
        .collect();

    // Compute stats
    let max_distance = sliced_symbols.iter().map(|s| s.distance).max().unwrap_or(0);

    let stats = SliceStats {
        total_symbols: sliced_symbols.len(),
        max_distance,
        affected_file_count: affected_files_result.len(),
    };

    let symbols_result: Vec<SlicedSymbol> = sliced_symbols
        .into_iter()
        .map(|ss| SlicedSymbol {
            symbol: SymbolInfo {
                symbol_id: None,
                id_format: None,
                name: ss.symbol.name,
                kind: ss.symbol.kind,
                file_path: ss.symbol.file_path,
                byte_start: ss.symbol.byte_start,
                byte_end: ss.symbol.byte_end,
            },
            distance: ss.distance,
            is_target: ss.is_target,
            relationship: ss.relationship,
        })
        .collect();

    let result = SliceResult {
        target: target_symbol,
        direction: format!("{:?}", direction).to_lowercase(),
        max_depth,
        symbols: symbols_result,
        affected_files: affected_files_result,
        stats,
    };

    // Format output
    if output.is_json() || json_output {
        let json = output
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json);
        Ok(
            splice::cli::CliSuccessPayload::message_only("Program slice complete".to_string())
                .already_emitted(),
        )
    } else {
        // Human-readable output
        println!(
            "Program Slice: {} {} from '{}'",
            result.direction,
            result
                .max_depth
                .map_or("(unlimited)".to_string(), |d| format!("(max depth {})", d)),
            result.target.name
        );
        println!();
        println!("Statistics:");
        println!("  Total symbols in slice: {}", result.stats.total_symbols);
        println!("  Max distance: {}", result.stats.max_distance);
        println!("  Affected files: {}", result.stats.affected_file_count);
        println!();

        println!("Affected Files:");
        for file in &result.affected_files {
            let marker = if file.is_root { " [target]" } else { "" };
            println!("  {}{} ({} symbols)", file.path, marker, file.symbol_count);
        }
        println!();

        println!("Symbols in Slice:");
        for ss in &result.symbols {
            let target_marker = if ss.is_target { " [TARGET]" } else { "" };
            println!(
                "  [d={:2}] {}{} in {} ({})",
                ss.distance, ss.symbol.name, target_marker, ss.symbol.file_path, ss.relationship
            );
        }

        Ok(splice::cli::CliSuccessPayload::message_only(
            "Program slice complete".to_string(),
        ))
    }
}

/// Execute the validate-proof command.
///
/// Validates SHA-256 checksums in a refactoring proof file to ensure
/// audit trail integrity.
///
/// This function:
/// 1. Reads the proof JSON file
/// 2. Deserializes it into a RefactoringProof
/// 3. Validates all checksums (before, after, proof hash)
/// 4. Outputs validation result
fn execute_validate_proof(
    proof_path: &Path,
    output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use serde_json::json;
    use splice::proof::validate_proof_file;

    // Validate proof file
    let is_valid = validate_proof_file(proof_path)?;

    // Format output
    if output.is_json() || json_output {
        let result = json!({
            "proof_path": proof_path.to_string_lossy(),
            "checksums_valid": is_valid,
            "status": if is_valid { "valid" } else { "no_checksums" }
        });

        let json_output = output
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);

        Ok(splice::cli::CliSuccessPayload::message_only(if is_valid {
            "Proof checksums are valid".to_string()
        } else {
            "Proof has no checksums".to_string()
        })
        .already_emitted())
    } else {
        // Human-readable output
        println!("Proof Validation: {}", proof_path.display());
        println!();

        if is_valid {
            println!("Status: VALID ✓");
            println!();
            println!("All SHA-256 checksums verified:");
            println!("  ✓ Before snapshot hash");
            println!("  ✓ After snapshot hash");
            println!("  ✓ Overall proof hash");
            println!();
            println!("Audit trail integrity is confirmed.");
        } else {
            println!("Status: NO CHECKSUMS ⚠");
            println!();
            println!("This proof does not include checksums.");
            println!("Checksums were added in Splice 2.2.4.");
            println!("Proof may have been generated with an older version.");
        }

        Ok(splice::cli::CliSuccessPayload::message_only(if is_valid {
            "Proof validation complete".to_string()
        } else {
            "Proof validation complete (no checksums)".to_string()
        }))
    }
}

fn execute_verify(
    before_path: &Path,
    after_path: &Path,
    detailed: bool,
    output_format: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use serde_json::json;
    use splice::proof::{compare_snapshots, storage::SnapshotStorage};

    // Load snapshots
    let storage = SnapshotStorage::new()?;
    let before = storage.load_snapshot(before_path)?;
    let after = storage.load_snapshot(after_path)?;

    // Compare snapshots
    let diff = compare_snapshots(&before, &after)?;

    // Determine if there are differences
    let has_symbol_changes =
        diff.symbols_added > 0 || diff.symbols_removed > 0 || diff.symbols_modified > 0;
    let has_edge_changes = diff.edges_added > 0 || diff.edges_removed > 0;
    let has_invariant_failures = diff.invariant_results.iter().any(|c| !c.passed);
    let has_differences = has_symbol_changes || has_edge_changes || has_invariant_failures;

    // Format output
    if output_format.is_json() || json_output {
        // JSON output - return complete diff
        let result = json!(diff);
        let json_output = output_format
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);

        let mut payload = splice::cli::CliSuccessPayload::with_data(
            format!(
                "Snapshot comparison: {} differences",
                if has_differences { "found" } else { "none" }
            ),
            result,
        )
        .already_emitted();

        if has_differences {
            payload = payload.with_pending_changes();
        }

        Ok(payload)
    } else {
        // Human-readable output
        println!("Snapshot Comparison");
        println!();
        println!("Before: {}", before_path.display());
        println!("After: {}", after_path.display());
        println!();

        // Summary
        println!("Summary:");
        if !has_differences {
            println!("  No differences detected - snapshots are identical");
        } else {
            if diff.symbols_added > 0 {
                println!("  Symbols added: {}", diff.symbols_added);
            }
            if diff.symbols_removed > 0 {
                println!("  Symbols removed: {}", diff.symbols_removed);
            }
            if diff.symbols_modified > 0 {
                println!("  Symbols modified: {}", diff.symbols_modified);
            }
            if diff.edges_added > 0 {
                println!("  Edges added: {}", diff.edges_added);
            }
            if diff.edges_removed > 0 {
                println!("  Edges removed: {}", diff.edges_removed);
            }
            if has_invariant_failures {
                let failed = diff.invariant_results.iter().filter(|c| !c.passed).count();
                println!("  Invariant failures: {}", failed);
            }
        }
        println!();

        // Detailed symbol differences
        if detailed && !diff.symbol_details.is_empty() {
            println!("Symbol Details:");
            for sym_diff in &diff.symbol_details {
                let change_marker = match sym_diff.change_type {
                    splice::proof::ChangeType::Added => "+",
                    splice::proof::ChangeType::Removed => "-",
                    splice::proof::ChangeType::Modified => "~",
                    splice::proof::ChangeType::Unchanged => " ",
                };
                println!("  {} {} ({})", change_marker, sym_diff.name, sym_diff.id);

                if sym_diff.change_type == splice::proof::ChangeType::Modified {
                    if let Some(before) = &sym_diff.before {
                        println!("    Before: {} @ {}", before.name, before.file_path);
                    }
                    if let Some(after) = &sym_diff.after {
                        println!("    After:  {} @ {}", after.name, after.file_path);
                    }
                }
            }
            println!();
        }

        // Invariant results
        if has_invariant_failures {
            println!("Invariant Validation:");
            for check in &diff.invariant_results {
                if !check.passed {
                    println!("  FAILED: {}", check.invariant_name);
                    for violation in &check.violations {
                        println!("    - {}: {}", violation.subject, violation.message);
                    }
                }
            }
            println!();
        }

        let mut payload = splice::cli::CliSuccessPayload::message_only(if has_differences {
            "Snapshots differ".to_string()
        } else {
            "Snapshots are identical".to_string()
        });

        if has_differences {
            payload = payload.with_pending_changes();
        }

        Ok(payload)
    }
}

/// Execute batch operations from YAML spec.
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
fn execute_batch(
    spec_path: &std::path::Path,
    db_path: Option<std::path::PathBuf>,
    dry_run: bool,
    continue_on_error: bool,
    rollback: splice::cli::CliRollbackMode,
    analyzer: Option<splice::cli::AnalyzerMode>,
    analyzer_binary: Option<std::path::PathBuf>,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::batch::{parse_batch_spec, BatchExecutor, ExecutionMode, RollbackMode};

    // Parse batch spec
    let spec = parse_batch_spec(&spec_path.to_path_buf())?;

    // Override execution mode if flag is set
    let mode = if continue_on_error {
        ExecutionMode::ContinueOnError
    } else {
        spec.mode
    };

    // Determine rollback mode
    let rollback_mode = match rollback {
        splice::cli::CliRollbackMode::Auto => {
            if db_path.is_some() {
                splice::batch::RollbackMode::OnFailure
            } else {
                eprintln!("Warning: Automatic rollback requires --db flag");
                eprintln!("         Batch will execute without automatic rollback");
                splice::batch::RollbackMode::Never
            }
        }
        splice::cli::CliRollbackMode::Never => splice::batch::RollbackMode::Never,
        splice::cli::CliRollbackMode::Always => {
            if db_path.is_some() {
                splice::batch::RollbackMode::Always
            } else {
                eprintln!("Warning: 'Always' rollback mode requires --db flag");
                eprintln!("         Batch will execute without automatic rollback");
                splice::batch::RollbackMode::Never
            }
        }
    };

    // Convert analyzer CLI flags to validation mode
    let analyzer_mode = match analyzer {
        Some(splice::cli::AnalyzerMode::Off) => splice::validate::AnalyzerMode::Off,
        Some(splice::cli::AnalyzerMode::Os) => splice::validate::AnalyzerMode::Path,
        Some(splice::cli::AnalyzerMode::Path) => {
            if let Some(binary) = analyzer_binary {
                splice::validate::AnalyzerMode::Explicit(binary.to_string_lossy().to_string())
            } else {
                splice::validate::AnalyzerMode::Path
            }
        }
        None => splice::validate::AnalyzerMode::Off,
    };

    // Determine if we should use transaction mode
    let use_transaction = rollback_mode != RollbackMode::Never && db_path.is_some();

    // Create executor and run with transaction if rollback is enabled
    let mut executor = BatchExecutor::new(dry_run, db_path.clone(), analyzer_mode);

    let (batch_result, rolled_back, rollback_snapshot) = if use_transaction {
        // Use transaction mode with rollback
        let txn_result = executor.execute_transaction(&spec, dry_run, rollback_mode)?;
        (
            txn_result.batch_result,
            txn_result.rolled_back,
            txn_result
                .rollback_snapshot
                .map(|p| p.to_string_lossy().to_string()),
        )
    } else {
        // Simple execution without rollback
        let result = executor.execute(&spec)?;
        (result, false, None)
    };

    // Build response
    let mut payload = serde_json::Map::new();
    payload.insert(
        "spec_path".to_string(),
        serde_json::json!(spec_path.to_string_lossy()),
    );
    payload.insert(
        "total_operations".to_string(),
        serde_json::json!(batch_result.total_operations),
    );
    payload.insert(
        "successful".to_string(),
        serde_json::json!(batch_result.successful),
    );
    payload.insert("failed".to_string(), serde_json::json!(batch_result.failed));
    payload.insert(
        "duration_ms".to_string(),
        serde_json::json!(batch_result.total_duration_ms),
    );
    payload.insert("rolled_back".to_string(), serde_json::json!(rolled_back));
    if let Some(snapshot) = rollback_snapshot {
        payload.insert("rollback_snapshot".to_string(), serde_json::json!(snapshot));
    }

    // Add operation results
    let ops_json: Vec<serde_json::Value> = batch_result
        .operations
        .into_iter()
        .map(|op| {
            let mut obj = serde_json::Map::new();
            obj.insert("index".to_string(), serde_json::json!(op.index));
            obj.insert("type".to_string(), serde_json::json!(op.op_type));
            obj.insert("success".to_string(), serde_json::json!(op.success));
            if let Some(error) = op.error {
                obj.insert("error".to_string(), serde_json::json!(error));
            }
            obj.insert("duration_ms".to_string(), serde_json::json!(op.duration_ms));
            serde_json::Value::Object(obj)
        })
        .collect();
    payload.insert("operations".to_string(), serde_json::json!(ops_json));

    if batch_result.failed > 0 && mode == ExecutionMode::StopOnError {
        return Err(splice::SpliceError::Other(format!(
            "Batch execution stopped: {} operation(s) failed",
            batch_result.failed
        )));
    }

    Ok(splice::cli::CliSuccessPayload {
        status: "ok",
        message: if dry_run {
            format!(
                "Batch preview complete: {} operations",
                batch_result.total_operations
            )
        } else {
            format!(
                "Batch complete: {} succeeded, {} failed",
                batch_result.successful, batch_result.failed
            )
        },
        data: Some(serde_json::Value::Object(payload)),
        already_emitted: false,
        has_pending_changes: dry_run,
    })
}

/// Execute code completion command.
fn execute_complete(
    file: &Path,
    line: usize,
    column: usize,
    max_results: usize,
    db: &Path,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::completion::engine::CompletionEngine;
    use splice::completion::types::CompletionRequest;
    use splice::graph::MagellanIntegration;
    use std::sync::Arc;

    let db_path = if db.is_absolute() {
        db.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| {
                splice::SpliceError::Other(format!("Failed to get current directory: {}", e))
            })?
            .join(db)
    };

    let db_path = db_path.canonicalize().map_err(|e| {
        splice::SpliceError::Other(format!(
            "Failed to resolve database path {}: {}",
            db_path.display(),
            e
        ))
    })?;

    // Convert file path to absolute path (database stores absolute paths)
    let file_path = if file.is_absolute() {
        file.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap()
            .join(file)
            .canonicalize()
            .map_err(|e| {
                splice::SpliceError::Other(format!("Failed to resolve file path: {}", e))
            })?
    };

    // Open Magellan database
    #[allow(
        clippy::arc_with_non_send_sync,
        reason = "single-threaded shared ownership for completion engine"
    )]
    let magellan = Arc::new(MagellanIntegration::open(&db_path)?);

    // Create completion engine
    let engine = CompletionEngine::new(magellan.clone(), &db_path);

    // Create completion request
    let request = CompletionRequest {
        file_path,
        line,
        column,
        max_results: Some(max_results),
    };

    // Execute completion
    let response = engine
        .complete_at_cursor(request)
        .map_err(|e| splice::SpliceError::Other(format!("Completion failed: {}", e)))?;

    // Output results
    if json_output {
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else {
        println!(
            "Grounded Completions ({} suggestions):",
            response.suggestions.len()
        );
        println!();

        for (i, suggestion) in response.suggestions.iter().enumerate() {
            println!("{}. {}", i + 1, suggestion.label);
            println!("   Detail: {}", suggestion.detail);
            println!("   Score: {:.2}", suggestion.score);
            println!("   Source: {:?}", suggestion.source);
            println!("   Grounded in: {:?}", suggestion.grounded_in);
            if suggestion.usage_count > 1 {
                println!("   Used {} times", suggestion.usage_count);
            }
            println!();
        }

        println!("Metadata:");
        println!("  Query time: {} ms", response.metadata.query_time_ms);
        println!(
            "  Total symbols: {}",
            response.metadata.total_symbols_indexed
        );
        println!("  Database version: {}", response.metadata.database_version);
        println!("  Database queries: {}", response.metadata.database_queries);
    }

    Ok(splice::cli::CliSuccessPayload {
        status: "ok",
        message: format!("Generated {} completions", response.suggestions.len()),
        data: None,
        already_emitted: true,
        has_pending_changes: false,
    })
}

/// Execute snapshot management commands.
fn execute_snapshots(
    cmd: splice::cli::SnapshotsCommands,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    match cmd {
        splice::cli::SnapshotsCommands::List {
            operation,
            limit,
            disk_usage,
            output,
        } => execute_snapshots_list(operation, limit, disk_usage, output, json_output),
        splice::cli::SnapshotsCommands::Delete { id, force } => {
            execute_snapshots_delete(&id, force, json_output)
        }
        splice::cli::SnapshotsCommands::Cleanup { keep, dry_run, yes } => {
            execute_snapshots_cleanup(keep, dry_run, yes, json_output)
        }
    }
}

/// Execute snapshots list command.
fn execute_snapshots_list(
    operation_filter: Option<String>,
    limit: Option<usize>,
    disk_usage: bool,
    output_format: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use serde_json::json;
    use splice::proof::storage::SnapshotStorage;

    let storage = SnapshotStorage::new()?;
    let snapshots = storage.list_snapshots_filtered(operation_filter.as_deref(), limit)?;

    // Calculate total disk usage if requested
    let total_size = if disk_usage {
        Some(storage.get_total_size()?)
    } else {
        None
    };

    if output_format.is_json() || json_output {
        // JSON output
        let snapshot_data: Vec<serde_json::Value> = snapshots
            .iter()
            .map(|meta| {
                json!({
                    "operation": meta.operation,
                    "timestamp": meta.timestamp,
                    "snapshot_path": meta.snapshot_path,
                    "symbols_count": meta.symbols_count,
                    "edges_count": meta.edges_count,
                })
            })
            .collect();

        let mut result = json!({
            "snapshots": snapshot_data,
            "count": snapshots.len(),
        });

        if let Some(size) = total_size {
            result["total_size_bytes"] = json!(size);
        }

        let json_output = output_format
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);

        Ok(splice::cli::CliSuccessPayload::with_data(
            format!("Listed {} snapshots", snapshots.len()),
            result,
        )
        .already_emitted())
    } else {
        // Human-readable output
        if snapshots.is_empty() {
            println!("No snapshots found.");
            return Ok(splice::cli::CliSuccessPayload::message_only(
                "No snapshots found".to_string(),
            ));
        }

        println!("Snapshots ({} total)", snapshots.len());
        println!();

        for meta in &snapshots {
            let timestamp_str = chrono::DateTime::from_timestamp(meta.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            println!(
                "  {} | {} | {} symbols, {} edges",
                timestamp_str, meta.operation, meta.symbols_count, meta.edges_count,
            );
            println!("    Path: {}", meta.snapshot_path.display());
        }

        if let Some(size) = total_size {
            println!();
            println!("Total disk usage: {} bytes ({} MB)", size, size / 1_048_576,);
        }

        Ok(splice::cli::CliSuccessPayload::message_only(format!(
            "Listed {} snapshots",
            snapshots.len()
        )))
    }
}

/// Execute snapshots delete command.
fn execute_snapshots_delete(
    snapshot_id: &str,
    force: bool,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::proof::storage::SnapshotStorage;

    let storage = SnapshotStorage::new()?;

    // Find the snapshot first to show what will be deleted
    let snapshot_info = storage.get_by_id(snapshot_id)?;

    if snapshot_info.is_none() {
        return Err(splice::SpliceError::Other(format!(
            "Snapshot '{}' not found",
            snapshot_id
        )));
    }

    let (path, meta) = snapshot_info.unwrap();

    // Confirm deletion unless --force is set
    if !force
        && !confirm_action(&format!(
            "Delete snapshot '{}' from {}? (y/N): ",
            meta.operation,
            chrono::DateTime::from_timestamp(meta.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        ))?
    {
        println!("Deletion cancelled.");
        return Ok(splice::cli::CliSuccessPayload::message_only(
            "Deletion cancelled".to_string(),
        ));
    }

    // Delete the snapshot
    let deleted = storage.delete_by_id(snapshot_id)?;

    if !deleted {
        return Err(splice::SpliceError::Other(format!(
            "Failed to delete snapshot '{}'",
            snapshot_id
        )));
    }

    if json_output {
        let result = serde_json::json!({
            "deleted": true,
            "snapshot_id": snapshot_id,
            "snapshot_path": path,
        });
        println!("{}", result);
        Ok(splice::cli::CliSuccessPayload::with_data(
            format!("Deleted snapshot '{}'", snapshot_id),
            result,
        )
        .already_emitted())
    } else {
        println!("Deleted snapshot: {}", path.display());
        Ok(splice::cli::CliSuccessPayload::message_only(format!(
            "Deleted snapshot '{}'",
            snapshot_id
        )))
    }
}

/// Execute snapshots cleanup command.
fn execute_snapshots_cleanup(
    keep: usize,
    dry_run: bool,
    yes: bool,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::proof::storage::SnapshotStorage;

    const BULK_DELETE_THRESHOLD: usize = 50;

    let storage = SnapshotStorage::new()?;
    let snapshots = storage.list_snapshots()?;

    if snapshots.len() <= keep {
        let message = format!(
            "Only {} snapshots exist (keeping {}), nothing to clean up",
            snapshots.len(),
            keep
        );

        if json_output {
            let result = serde_json::json!({
                "deleted_count": 0,
                "kept_count": snapshots.len(),
                "keep": keep,
                "dry_run": dry_run,
            });
            println!("{}", result);
            return Ok(splice::cli::CliSuccessPayload::with_data(message, result).already_emitted());
        } else {
            println!("{}", message);
            return Ok(splice::cli::CliSuccessPayload::message_only(message));
        }
    }

    let to_delete_count = snapshots.len() - keep;
    let to_delete = &snapshots[keep..];

    if dry_run {
        let message = format!(
            "Would delete {} snapshots (keeping {} most recent)",
            to_delete_count, keep
        );

        if json_output {
            let snapshot_paths: Vec<String> = to_delete
                .iter()
                .map(|s| s.snapshot_path.to_string_lossy().to_string())
                .collect();

            let result = serde_json::json!({
                "dry_run": true,
                "would_delete_count": to_delete_count,
                "kept_count": keep,
                "to_delete": snapshot_paths,
            });
            println!("{}", result);
            return Ok(splice::cli::CliSuccessPayload::with_data(message, result).already_emitted());
        } else {
            println!("{}", message);
            println!();
            println!("Snapshots to delete:");
            for meta in to_delete {
                println!("  - {} ({})", meta.snapshot_path.display(), meta.operation);
            }
            return Ok(splice::cli::CliSuccessPayload::message_only(message));
        }
    }

    // Actually delete
    if !yes && to_delete_count > BULK_DELETE_THRESHOLD {
        return Err(splice::SpliceError::Other(format!(
            "Refusing to delete {} snapshots (threshold {}). Re-run with --yes to confirm \
             the bulk delete, or use --dry-run to preview what would be removed.",
            to_delete_count, BULK_DELETE_THRESHOLD
        )));
    }
    let deleted_paths = storage.cleanup_old_snapshots(keep)?;

    if json_output {
        let deleted_paths_str: Vec<String> = deleted_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let result = serde_json::json!({
            "deleted_count": deleted_paths.len(),
            "kept_count": keep,
            "deleted_paths": deleted_paths_str,
        });
        println!("{}", result);
        Ok(splice::cli::CliSuccessPayload::with_data(
            format!("Deleted {} snapshots", deleted_paths.len()),
            result,
        )
        .already_emitted())
    } else {
        println!(
            "Deleted {} snapshots (kept {} most recent)",
            deleted_paths.len(),
            keep
        );
        for path in &deleted_paths {
            println!("  - {}", path.display());
        }
        Ok(splice::cli::CliSuccessPayload::message_only(format!(
            "Deleted {} snapshots",
            deleted_paths.len()
        )))
    }
}

/// Prompt user for confirmation (y/N).
fn confirm_action(prompt: &str) -> Result<bool, splice::SpliceError> {
    use std::io::{self, Write};

    print!("{}", prompt);
    io::stdout()
        .flush()
        .map_err(|e| splice::SpliceError::IoContext {
            context: "Failed to flush stdout".to_string(),
            source: e,
        })?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| splice::SpliceError::IoContext {
            context: "Failed to read user input".to_string(),
            source: e,
        })?;

    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

fn execute_dead_code(
    entry: &str,
    path: &Path,
    db_path: &Path,
    exclude_public: bool,
    group_by_file: bool,
    output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::MagellanIntegration;
    use splice::output::{DeadCodeByFile, DeadCodeResult, SymbolInfo};
    use std::collections::HashMap;

    // Get path string early
    let path_str = path
        .to_str()
        .ok_or_else(|| splice::SpliceError::Other("Invalid UTF-8 in path".to_string()))?;

    // Open database for operations
    let mut integration = MagellanIntegration::open(db_path)?;

    // Get entry point symbol info using backend-neutral method
    let entry_symbol_info = match integration.find_symbol_by_path_and_name(path, entry)? {
        Some(info) => info,
        None => {
            return Err(splice::SpliceError::SymbolNotFound {
                message: format!("Entry point '{}' not found in '{}'", entry, path_str),
                symbol: entry.to_string(),
                file: Some(path.to_path_buf()),
                hint: "Ensure the entry point symbol exists in the specified file".to_string(),
            });
        }
    };

    // Get total symbol count using backend-neutral statistics
    let total_symbols = integration.get_statistics()?.symbols;

    // Detect dead code
    let dead_symbols = integration.dead_symbols(path, entry, exclude_public)?;

    let reachable_count = total_symbols.saturating_sub(dead_symbols.len());
    let dead_count = dead_symbols.len();

    // Group by file if requested
    let dead_by_file = if group_by_file {
        let mut by_file: HashMap<String, Vec<splice::graph::magellan_integration::DeadSymbol>> =
            HashMap::new();
        for ds in dead_symbols {
            by_file
                .entry(ds.symbol.file_path.clone())
                .or_default()
                .push(ds);
        }

        by_file
            .into_iter()
            .map(|(path, symbols)| {
                let count = symbols.len();
                let output_symbols = symbols
                    .into_iter()
                    .map(|ds| splice::output::DeadSymbol {
                        symbol: SymbolInfo {
                            symbol_id: None,
                            id_format: None,
                            name: ds.symbol.name,
                            kind: ds.symbol.kind,
                            file_path: ds.symbol.file_path,
                            byte_start: ds.symbol.byte_start,
                            byte_end: ds.symbol.byte_end,
                        },
                        reason: ds.reason,
                    })
                    .collect();
                DeadCodeByFile {
                    path,
                    count,
                    symbols: output_symbols,
                }
            })
            .collect()
    } else {
        vec![DeadCodeByFile {
            path: "all".to_string(),
            count: dead_count,
            symbols: dead_symbols
                .into_iter()
                .map(|ds| splice::output::DeadSymbol {
                    symbol: SymbolInfo {
                        symbol_id: None,
                        id_format: None,
                        name: ds.symbol.name,
                        kind: ds.symbol.kind,
                        file_path: ds.symbol.file_path,
                        byte_start: ds.symbol.byte_start,
                        byte_end: ds.symbol.byte_end,
                    },
                    reason: ds.reason,
                })
                .collect(),
        }]
    };

    let result = DeadCodeResult {
        entry_point: SymbolInfo {
            symbol_id: None,
            id_format: None,
            name: entry_symbol_info.name.clone(),
            kind: entry_symbol_info.kind.clone(),
            file_path: entry_symbol_info.file_path.clone(),
            byte_start: entry_symbol_info.byte_start,
            byte_end: entry_symbol_info.byte_end,
        },
        total_symbols,
        reachable_count,
        dead_count,
        dead_by_file,
        excluded_public: exclude_public,
    };

    // Format output
    if output.is_json() || json_output {
        let json = output
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json);
        Ok(
            splice::cli::CliSuccessPayload::message_only(
                "Dead code detection complete".to_string(),
            )
            .already_emitted(),
        )
    } else {
        // Human-readable output
        println!("Dead Code Detection");
        println!("Entry Point: {} in {}", entry, path_str);
        println!();
        println!("Statistics:");
        println!("  Total symbols: {}", total_symbols);
        println!("  Reachable: {}", reachable_count);
        println!("  Dead (unreachable): {}", dead_count);
        println!();

        if dead_count == 0 {
            println!("No dead code found - all symbols are reachable from the entry point.");
        } else {
            for file_group in &result.dead_by_file {
                println!("{} ({} dead symbols):", file_group.path, file_group.count);
                for ds in &file_group.symbols {
                    println!("  - {} ({})", ds.symbol.name, ds.symbol.kind);
                }
                println!();
            }
        }

        Ok(splice::cli::CliSuccessPayload::message_only(
            "Dead code detection complete".to_string(),
        ))
    }
}

fn write_stdout_bytes(bytes: &[u8]) -> Result<(), splice::SpliceError> {
    use std::io::{self, Write};

    let mut stdout = io::stdout();
    if let Err(err) = stdout.write_all(bytes) {
        if err.kind() == io::ErrorKind::BrokenPipe {
            return Err(splice::SpliceError::BrokenPipe);
        }
        return Err(splice::SpliceError::Io {
            path: PathBuf::from("<stdout>"),
            source: err,
        });
    }
    Ok(())
}

fn write_stdout_line(line: &str) -> Result<(), splice::SpliceError> {
    write_stdout_bytes(line.as_bytes())?;
    write_stdout_bytes(b"\n")
}

/// Emit JSON payload for successful CLI responses.
fn emit_success_payload(
    payload: &splice::cli::CliSuccessPayload,
    _json_output: bool,
) -> Result<(), splice::SpliceError> {
    // If payload was already emitted (e.g., --json mode with OperationResult), skip
    if payload.already_emitted {
        return Ok(());
    }

    match serde_json::to_string(payload) {
        Ok(json) => write_stdout_line(&json),
        Err(err) => {
            let fallback = json!({
                "status": "ok",
                "message": payload.message.clone(),
            });
            write_stdout_line(&fallback.to_string())?;
            eprintln!("Serialization warning: {}", err);
            Ok(())
        }
    }
}

/// Emit JSON payload for CLI errors.
fn emit_error_payload(payload: &splice::cli::CliErrorPayload, _json_output: bool) {
    match serde_json::to_string(payload) {
        Ok(json) => eprintln!("{}", json),
        Err(err) => {
            let fallback = json!({
                "status": "error",
                "error": {
                    "kind": "SerializationFailure",
                    "message": err.to_string()
                }
            });
            eprintln!("{}", fallback);
        }
    }
}

fn require_patch_arg<T>(flag: &str, value: Option<T>) -> Result<T, splice::SpliceError> {
    value.ok_or_else(|| {
        splice::SpliceError::Other(format!(
            "{} is required unless --batch <file> is provided",
            flag
        ))
    })
}

fn _build_success_payload(
    message: String,
    files: Vec<splice::patch::FilePatchSummary>,
    preview_report: Option<splice::patch::PreviewReport>,
) -> splice::cli::CliSuccessPayload {
    let file_values: Vec<Value> = files
        .iter()
        .map(|summary| {
            json!({
                "file": summary.file.to_string_lossy(),
                "before_hash": summary.before_hash,
                "after_hash": summary.after_hash,
            })
        })
        .collect();

    let mut data = Map::new();
    data.insert("files".to_string(), Value::Array(file_values));

    if let Some(report) = preview_report {
        data.insert(
            "preview_report".to_string(),
            serde_json::to_value(report).expect("preview report should serialize"),
        );
    }

    splice::cli::CliSuccessPayload::with_data(message, Value::Object(data))
}

/// Walk parent directories looking for a project marker (Cargo.toml,
/// pyproject.toml, package.json, etc.). Stop walking before entering
/// boundary directories like `/tmp`, `$TMPDIR`, or `$HOME` (only when the
/// file is outside `$HOME`). Without a stop boundary, a stray ancestor
/// Cargo.toml above the working directory (e.g. left over in `/tmp`)
/// would be falsely picked as the workspace root.
fn find_workspace_root(path: &Path) -> Result<PathBuf, splice::SpliceError> {
    let absolute_path = std::fs::canonicalize(path).map_err(|e| splice::SpliceError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    const MARKERS: &[&str] = &[
        "Cargo.toml",
        "pyproject.toml",
        "package.json",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "setup.py",
    ];

    let boundaries = build_boundary_set(&absolute_path);

    let mut current = absolute_path.parent();
    while let Some(dir) = current {
        // A boundary directory (e.g. /tmp) is never considered as the workspace root.
        if boundaries.contains(dir) {
            break;
        }
        for marker in MARKERS {
            if dir.join(marker).exists() {
                return Ok(dir.to_path_buf());
            }
        }
        current = dir.parent();
    }

    Err(splice::SpliceError::Other(format!(
        "No project marker (Cargo.toml, pyproject.toml, package.json, go.mod, pom.xml, build.gradle, setup.py) found in any ancestor of {} within $HOME or before /tmp",
        path.display()
    )))
}

/// Build the set of directories that bound the upward search for a project
/// marker. We never look INTO these directories as workspace candidates.
fn build_boundary_set(file_path: &Path) -> std::collections::HashSet<PathBuf> {
    let mut set = std::collections::HashSet::new();
    set.insert(PathBuf::from("/"));
    set.insert(PathBuf::from("/tmp"));
    if let Some(tmpdir) = std::env::var_os("TMPDIR") {
        set.insert(PathBuf::from(tmpdir));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home_path = PathBuf::from(&home);
        // Only treat $HOME as a boundary if the file is NOT under $HOME.
        // Otherwise legitimate $HOME-rooted projects would fail.
        if !file_path.starts_with(&home_path) {
            set.insert(home_path);
        }
    }
    set
}
/// Extract symbols with explicit language (helper function).
fn extract_symbols_with_language(
    path: &Path,
    source: &[u8],
    language: splice::symbol::Language,
) -> Result<Vec<SymbolWrapper>, splice::SpliceError> {
    use splice::ingest::{
        extract_cpp_symbols, extract_java_symbols, extract_javascript_symbols,
        extract_python_symbols, extract_rust_symbols, extract_typescript_symbols,
    };

    match language {
        splice::symbol::Language::Rust => {
            let symbols = extract_rust_symbols(path, source)?;
            Ok(symbols.into_iter().map(SymbolWrapper::Rust).collect())
        }
        splice::symbol::Language::Python => {
            let symbols = extract_python_symbols(path, source)?;
            Ok(symbols.into_iter().map(SymbolWrapper::Python).collect())
        }
        splice::symbol::Language::C | splice::symbol::Language::Cpp => {
            let symbols = extract_cpp_symbols(path, source)?;
            Ok(symbols.into_iter().map(SymbolWrapper::Cpp).collect())
        }
        splice::symbol::Language::Java => {
            let symbols = extract_java_symbols(path, source)?;
            Ok(symbols.into_iter().map(SymbolWrapper::Java).collect())
        }
        splice::symbol::Language::JavaScript => {
            let symbols = extract_javascript_symbols(path, source)?;
            Ok(symbols.into_iter().map(SymbolWrapper::JavaScript).collect())
        }
        splice::symbol::Language::TypeScript => {
            let symbols = extract_typescript_symbols(path, source)?;
            Ok(symbols.into_iter().map(SymbolWrapper::TypeScript).collect())
        }
    }
}

/// Wrapper enum for language-specific symbols that implements Symbol trait.
enum SymbolWrapper {
    Rust(splice::ingest::rust::RustSymbol),
    Python(splice::ingest::python::PythonSymbol),
    Cpp(splice::ingest::cpp::CppSymbol),
    Java(splice::ingest::java::JavaSymbol),
    JavaScript(splice::ingest::javascript::JavaScriptSymbol),
    TypeScript(splice::ingest::typescript::TypeScriptSymbol),
}

impl splice::symbol::Symbol for SymbolWrapper {
    fn name(&self) -> &str {
        match self {
            SymbolWrapper::Rust(s) => s.name(),
            SymbolWrapper::Python(s) => s.name(),
            SymbolWrapper::Cpp(s) => s.name(),
            SymbolWrapper::Java(s) => s.name(),
            SymbolWrapper::JavaScript(s) => s.name(),
            SymbolWrapper::TypeScript(s) => s.name(),
        }
    }

    fn kind(&self) -> &str {
        match self {
            SymbolWrapper::Rust(s) => s.kind(),
            SymbolWrapper::Python(s) => s.kind(),
            SymbolWrapper::Cpp(s) => s.kind(),
            SymbolWrapper::Java(s) => s.kind(),
            SymbolWrapper::JavaScript(s) => s.kind(),
            SymbolWrapper::TypeScript(s) => s.kind(),
        }
    }

    fn byte_start(&self) -> usize {
        match self {
            SymbolWrapper::Rust(s) => s.byte_start(),
            SymbolWrapper::Python(s) => s.byte_start(),
            SymbolWrapper::Cpp(s) => s.byte_start(),
            SymbolWrapper::Java(s) => s.byte_start(),
            SymbolWrapper::JavaScript(s) => s.byte_start(),
            SymbolWrapper::TypeScript(s) => s.byte_start(),
        }
    }

    fn byte_end(&self) -> usize {
        match self {
            SymbolWrapper::Rust(s) => s.byte_end(),
            SymbolWrapper::Python(s) => s.byte_end(),
            SymbolWrapper::Cpp(s) => s.byte_end(),
            SymbolWrapper::Java(s) => s.byte_end(),
            SymbolWrapper::JavaScript(s) => s.byte_end(),
            SymbolWrapper::TypeScript(s) => s.byte_end(),
        }
    }

    fn line_start(&self) -> usize {
        match self {
            SymbolWrapper::Rust(s) => s.line_start(),
            SymbolWrapper::Python(s) => s.line_start(),
            SymbolWrapper::Cpp(s) => s.line_start(),
            SymbolWrapper::Java(s) => s.line_start(),
            SymbolWrapper::JavaScript(s) => s.line_start(),
            SymbolWrapper::TypeScript(s) => s.line_start(),
        }
    }

    fn line_end(&self) -> usize {
        match self {
            SymbolWrapper::Rust(s) => s.line_end(),
            SymbolWrapper::Python(s) => s.line_end(),
            SymbolWrapper::Cpp(s) => s.line_end(),
            SymbolWrapper::Java(s) => s.line_end(),
            SymbolWrapper::JavaScript(s) => s.line_end(),
            SymbolWrapper::TypeScript(s) => s.line_end(),
        }
    }

    fn col_start(&self) -> usize {
        match self {
            SymbolWrapper::Rust(s) => s.col_start(),
            SymbolWrapper::Python(s) => s.col_start(),
            SymbolWrapper::Cpp(s) => s.col_start(),
            SymbolWrapper::Java(s) => s.col_start(),
            SymbolWrapper::JavaScript(s) => s.col_start(),
            SymbolWrapper::TypeScript(s) => s.col_start(),
        }
    }

    fn col_end(&self) -> usize {
        match self {
            SymbolWrapper::Rust(s) => s.col_end(),
            SymbolWrapper::Python(s) => s.col_end(),
            SymbolWrapper::Cpp(s) => s.col_end(),
            SymbolWrapper::Java(s) => s.col_end(),
            SymbolWrapper::JavaScript(s) => s.col_end(),
            SymbolWrapper::TypeScript(s) => s.col_end(),
        }
    }

    fn fully_qualified(&self) -> &str {
        match self {
            SymbolWrapper::Rust(s) => s.fully_qualified(),
            SymbolWrapper::Python(s) => s.fully_qualified(),
            SymbolWrapper::Cpp(s) => s.fully_qualified(),
            SymbolWrapper::Java(s) => s.fully_qualified(),
            SymbolWrapper::JavaScript(s) => s.fully_qualified(),
            SymbolWrapper::TypeScript(s) => s.fully_qualified(),
        }
    }

    fn language(&self) -> splice::symbol::Language {
        match self {
            SymbolWrapper::Rust(_) => splice::symbol::Language::Rust,
            SymbolWrapper::Python(_) => splice::symbol::Language::Python,
            SymbolWrapper::Cpp(_) => splice::symbol::Language::Cpp,
            SymbolWrapper::Java(_) => splice::symbol::Language::Java,
            SymbolWrapper::JavaScript(_) => splice::symbol::Language::JavaScript,
            SymbolWrapper::TypeScript(_) => splice::symbol::Language::TypeScript,
        }
    }
}
