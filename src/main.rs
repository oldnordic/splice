//! Splice CLI binary
//!
//! This is the main entry point for the splice command-line interface.
//! The CLI is a thin adapter over existing APIs - NO logic is implemented here.

use std::process::ExitCode;

mod cmds;

/// Splice exit codes matching Magellan conventions.
///
/// Magellan exit codes:
/// - 0: success
/// - 1: generic error
/// - 2: usage error
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

fn main() -> ExitCode {
    cmds::helpers::install_broken_pipe_hook();

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
        } => cmds::delete::execute_delete(
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
            true,
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
            Some(batch_path) => cmds::patch_batch::execute_patch_batch(
                &batch_path,
                analyzer,
                analyzer_binary,
                language,
                create_backup,
                operation_id,
                metadata,
                json_output,
            ),
            None => cmds::patch::execute_single_patch(
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
                true,
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
        } => cmds::plan::execute_plan(&file, operation_id, metadata, json_output),

        splice::cli::Commands::Undo { manifest } => {
            cmds::plan::execute_undo(&manifest, json_output)
        }

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
        } => cmds::apply::execute_apply_files(
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
        } => cmds::query::execute_query(
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
        } => cmds::query::execute_get(
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
        } => cmds::log::execute_log(
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

        splice::cli::Commands::Explain { code } => cmds::log::execute_explain(code, json_output),

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
        } => cmds::search::execute_search(
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
            cmds::status::execute_status(&db, json_output, detect_backend)
        }

        splice::cli::Commands::Find {
            db,
            name,
            symbol_id,
            semantic_query,
            ambiguous,
            output,
        } => cmds::search::execute_find(&db, name, symbol_id, semantic_query.as_deref(), ambiguous, output, json_output),

        splice::cli::Commands::Refs {
            db,
            name,
            path,
            direction,
            output,
            impact_graph,
        } => cmds::search::execute_refs(
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
        } => cmds::search::execute_files(&db, symbols, output, json_output),

        splice::cli::Commands::Export {
            db,
            format: export_format,
            file,
        } => cmds::status::execute_export(&db, export_format, file.as_deref(), json_output),

        splice::cli::Commands::MigrateDb {
            db_path,
            backup,
            dry_run,
        } => cmds::log::execute_migrate_db(&db_path, backup, dry_run, json_output),

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
        } => cmds::rename::execute_rename(
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
            semantic_query,
            path,
            db,
            direction,
            max_depth,
            output,
            impact_graph,
        } => cmds::graph::execute_reachable(
            &symbol,
            semantic_query.as_deref(),
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
            semantic_query,
            path,
            db,
            exclude_public,
            group_by_file,
            output,
        } => cmds::dead_code::execute_dead_code(
            &entry,
            semantic_query.as_deref(),
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
        } => cmds::graph::execute_cycles(
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
        } => cmds::graph::execute_condense(&db, show_members, show_levels, output, json_output),

        splice::cli::Commands::Slice {
            target,
            semantic_query,
            path,
            db,
            direction,
            max_depth,
            output,
        } => cmds::graph::execute_slice(
            &target,
            semantic_query.as_deref(),
            &path,
            &db,
            &direction,
            max_depth,
            output,
            json_output,
        ),

        splice::cli::Commands::ValidateProof { proof, output } => {
            cmds::verify::execute_validate_proof(&proof, output, json_output)
        }

        splice::cli::Commands::Verify {
            before,
            after,
            detailed,
            output,
        } => cmds::verify::execute_verify(&before, &after, detailed, output, json_output),

        splice::cli::Commands::Batch {
            spec,
            db,
            dry_run,
            continue_on_error,
            rollback,
            analyzer,
            analyzer_binary,
        } => cmds::batch::execute_batch(
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
        } => cmds::batch::execute_complete(&file, line, column, max_results, &db, json_output),

        splice::cli::Commands::Snapshots(subcommand) => {
            cmds::snapshots::execute_snapshots(subcommand, json_output)
        }
    };

    // Handle result
    match result {
        Ok(payload) => match cmds::helpers::emit_success_payload(&payload, json_output) {
            Ok(()) => {
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
                    cmds::helpers::emit_error_payload(&payload, json_output);
                    SpliceExitCode::from_error(&err).as_exit_code()
                }
            }
        },
        Err(e) => {
            if matches!(e, splice::SpliceError::BrokenPipe) {
                SpliceExitCode::Success.as_exit_code()
            } else {
                let payload = splice::cli::CliErrorPayload::from_error(&e);
                cmds::helpers::emit_error_payload(&payload, json_output);
                SpliceExitCode::from_error(&e).as_exit_code()
            }
        }
    }
}
