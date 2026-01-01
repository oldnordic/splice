//! Splice CLI binary
//!
//! This is the main entry point for the splice command-line interface.
//! The CLI is a thin adapter over existing APIs - NO logic is implemented here.

use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    // Parse CLI arguments
    let cli = splice::cli::parse_args();

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
            language,
            create_backup,
            operation_id,
            metadata,
        } => execute_delete(&file, &symbol, kind, analyzer, language, create_backup, operation_id, metadata),

        splice::cli::Commands::Patch {
            file,
            symbol,
            kind,
            analyzer,
            with_: replacement_file,
            language,
            batch,
            preview,
            create_backup,
            operation_id,
            metadata,
        } => match batch {
            Some(batch_path) => execute_patch_batch(&batch_path, analyzer, language, create_backup, operation_id, metadata),
            None => execute_single_patch(
                file,
                symbol,
                kind,
                analyzer,
                replacement_file,
                language,
                preview,
                create_backup,
                operation_id,
                metadata,
            ),
        },

        splice::cli::Commands::Plan { file } => execute_plan(&file),

        splice::cli::Commands::Undo { manifest } => execute_undo(&manifest),

        splice::cli::Commands::ApplyFiles {
            glob,
            find,
            replace,
            language,
            no_validate,
            create_backup,
            operation_id,
            metadata,
        } => execute_apply_files(&glob, &find, &replace, language, !no_validate, create_backup, operation_id, metadata),

        splice::cli::Commands::Query {
            db,
            label,
            list,
            count,
            show_code,
        } => execute_query(&db, &label, list, count, show_code),

        splice::cli::Commands::Get {
            db,
            file,
            start,
            end,
        } => execute_get(&db, &file, start, end),
    };

    // Handle result
    match result {
        Ok(payload) => {
            emit_success_payload(&payload);
            ExitCode::SUCCESS
        }
        Err(e) => {
            let payload = splice::cli::CliErrorPayload::from_error(&e);
            emit_error_payload(&payload);
            ExitCode::from(1)
        }
    }
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
fn execute_delete(
    file_path: &Path,
    symbol_name: &str,
    kind: Option<splice::cli::SymbolKind>,
    analyzer: Option<splice::cli::AnalyzerMode>,
    language: Option<splice::cli::Language>,
    create_backup: bool,
    operation_id: Option<String>,
    metadata: Option<String>,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::CodeGraph;
    use splice::patch::apply_patch_with_validation;
    use splice::resolve::references::find_references;
    use splice::symbol::{Language as SymbolLanguage, Symbol};
    use splice::validate::AnalyzerMode as ValidateAnalyzerMode;

    // Determine language (from CLI flag or auto-detect from file extension)
    let symbol_lang = language
        .map(|l| l.to_symbol_language())
        .or_else(|| SymbolLanguage::from_path(file_path));

    let symbol_lang = symbol_lang.ok_or_else(|| splice::SpliceError::Parse {
        file: file_path.to_path_buf(),
        message: "Cannot detect language - unknown file extension".to_string(),
    })?;

    // Step 1: Read source file
    let source = std::fs::read(file_path)?;

    // Step 2: Extract symbols using language-aware dispatcher
    let symbols = extract_symbols_with_language(file_path, &source, symbol_lang)?;

    // Step 3: Create in-memory graph (for reference finding API compatibility)
    let graph_db_path = file_path.parent().unwrap().join(".splice_graph.db");
    let mut code_graph = CodeGraph::open(&graph_db_path)?;

    // Step 4: Store symbols in graph with language metadata
    for symbol in &symbols {
        code_graph.store_symbol_with_file_and_language(
            file_path,
            symbol.name(),
            symbol.kind(),
            symbol.language(),
            symbol.byte_start(),
            symbol.byte_end(),
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
            return Err(splice::SpliceError::Other(
                "Explicit analyzer path not yet supported".to_string(),
            ));
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

    // Step 10: Create backup if requested
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

    // Step 11: Delete references from each file
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
                analyzer_mode,
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
        analyzer_mode,
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
        response_data.insert("backup_manifest".to_string(), json!(manifest_path.to_string_lossy()));
    }
    if let Some(op_id) = operation_id {
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

    Ok(splice::cli::CliSuccessPayload::with_data(base_message, serde_json::Value::Object(response_data)))
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
fn execute_single_patch(
    file_path: Option<PathBuf>,
    symbol_name: Option<String>,
    kind: Option<splice::cli::SymbolKind>,
    analyzer: Option<splice::cli::AnalyzerMode>,
    replacement_file: Option<PathBuf>,
    language: Option<splice::cli::Language>,
    preview: bool,
    create_backup: bool,
    operation_id: Option<String>,
    metadata: Option<String>,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    let file_path = require_patch_arg("--file", file_path)?;
    let symbol_name = require_patch_arg("--symbol", symbol_name)?;
    let replacement_file = require_patch_arg("--with", replacement_file)?;

    execute_patch(
        &file_path,
        &symbol_name,
        kind,
        analyzer,
        &replacement_file,
        language,
        preview,
        create_backup,
        operation_id,
        metadata,
    )
}

fn execute_patch(
    file_path: &Path,
    symbol_name: &str,
    kind: Option<splice::cli::SymbolKind>,
    analyzer: Option<splice::cli::AnalyzerMode>,
    replacement_file: &Path,
    language: Option<splice::cli::Language>,
    preview: bool,
    create_backup: bool,
    operation_id: Option<String>,
    metadata: Option<String>,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::CodeGraph;
    use splice::patch::{apply_patch_with_validation, preview_patch, FilePatchSummary};
    use splice::resolve::resolve_symbol;
    use splice::symbol::{Language as SymbolLanguage, Symbol};
    use splice::validate::AnalyzerMode as ValidateAnalyzerMode;

    // Determine language (from CLI flag or auto-detect from file extension)
    let symbol_lang = language
        .map(|l| l.to_symbol_language())
        .or_else(|| SymbolLanguage::from_path(file_path));

    let symbol_lang = symbol_lang.ok_or_else(|| splice::SpliceError::Parse {
        file: file_path.to_path_buf(),
        message: "Cannot detect language - unknown file extension".to_string(),
    })?;

    // Step 1: Read source file
    let source = std::fs::read(file_path)?;

    // Step 2: Extract symbols using language-aware dispatcher
    let symbols = extract_symbols_with_language(file_path, &source, symbol_lang)?;

    // Step 3: Create in-memory graph
    let graph_db_path = file_path.parent().unwrap().join(".splice_graph.db");
    let mut code_graph = CodeGraph::open(&graph_db_path)?;

    // Step 4: Store symbols in graph with language metadata
    for symbol in &symbols {
        code_graph.store_symbol_with_file_and_language(
            file_path,
            symbol.name(),
            symbol.kind(),
            symbol.language(),
            symbol.byte_start(),
            symbol.byte_end(),
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
    let replacement_content = std::fs::read_to_string(replacement_file)?;

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
            return Err(splice::SpliceError::Other(
                "Explicit analyzer path not yet supported".to_string(),
            ));
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

    if preview {
        let (summary, report) = preview_patch(
            file_path,
            resolved.byte_start,
            resolved.byte_end,
            &replacement_content,
            &workspace_root,
            symbol_lang,
            analyzer_mode,
        )?;
        let message = format!(
            "Previewed patch '{}' at bytes {}..{} (hash: {} -> {})",
            symbol_name,
            resolved.byte_start,
            resolved.byte_end,
            summary.before_hash,
            summary.after_hash
        );
        return Ok(build_success_payload(message, vec![summary], Some(report)));
    }

    let (before_hash, after_hash) = apply_patch_with_validation(
        file_path,
        resolved.byte_start,
        resolved.byte_end,
        &replacement_content,
        workspace_dir,
        symbol_lang,
        analyzer_mode,
    )?;

    let summary = FilePatchSummary {
        file: file_path.to_path_buf(),
        before_hash,
        after_hash,
    };

    let message = format!(
        "Patched '{}' at bytes {}..{} (hash: {} -> {})",
        symbol_name,
        resolved.byte_start,
        resolved.byte_end,
        summary.before_hash,
        summary.after_hash
    );

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
        response_data.insert("backup_manifest".to_string(), json!(manifest_path.to_string_lossy()));
    }
    if let Some(op_id) = operation_id {
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

    Ok(splice::cli::CliSuccessPayload::with_data(message, serde_json::Value::Object(response_data)))
}

/// Execute a batch patch command driven by a JSON manifest.
fn execute_patch_batch(
    batch_path: &Path,
    analyzer: Option<splice::cli::AnalyzerMode>,
    language: Option<splice::cli::Language>,
    create_backup: bool,
    operation_id: Option<String>,
    metadata: Option<String>,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::patch::{apply_batch_with_validation, load_batches_from_file};
    use splice::validate::AnalyzerMode as ValidateAnalyzerMode;

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
            return Err(splice::SpliceError::Other(
                "Explicit analyzer path not yet supported".to_string(),
            ));
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
        let mut files_to_backup: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
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

    if let Some(op_id) = operation_id {
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

    Ok(splice::cli::CliSuccessPayload::with_data(
        format!(
            "Patched {} file(s) across {} batch(es).",
            summaries.len(),
            batch_count
        ),
        response_data,
    ))
}

/// Execute the plan command.
///
/// This function is a thin adapter that:
/// 1. Reads the plan.json file
/// 2. Calls execute_plan from the plan module
///
/// All logic is delegated to the plan module.
fn execute_plan(plan_path: &Path) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::plan::execute_plan;

    // Determine workspace directory (parent of plan file)
    let workspace_dir = plan_path.parent().ok_or_else(|| {
        splice::SpliceError::Other(
            "Cannot determine workspace directory from plan path".to_string(),
        )
    })?;

    // Execute plan
    let messages = execute_plan(plan_path, workspace_dir)?;

    // Return summary message
    Ok(splice::cli::CliSuccessPayload::message_only(format!(
        "Plan executed successfully: {} steps completed",
        messages.len()
    )))
}

/// Execute the undo command.
///
/// This function restores files from a backup manifest created during
/// a previous splice operation.
fn execute_undo(manifest_path: &Path) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::patch::restore_from_manifest;

    // Determine workspace directory (parent of manifest's parent directory)
    // The manifest is at .splice-backup/<operation_id>/manifest.json
    // The workspace root is the parent of .splice-backup
    let backup_dir = manifest_path.parent().ok_or_else(|| {
        splice::SpliceError::Other("Manifest has no parent directory".to_string())
    })?;

    let splice_backup_dir = backup_dir.parent().ok_or_else(|| {
        splice::SpliceError::Other(
            "Backup directory has no parent directory".to_string()
        )
    })?;

    let workspace_root = splice_backup_dir.parent().ok_or_else(|| {
        splice::SpliceError::Other(
            "Cannot determine workspace root from manifest path".to_string()
        )
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
fn execute_apply_files(
    glob_pattern: &str,
    find_pattern: &str,
    replace_pattern: &str,
    language: Option<splice::cli::Language>,
    validate: bool,
    create_backup: bool,
    operation_id: Option<String>,
    metadata: Option<String>,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::patch::{apply_pattern_replace, find_pattern_in_files, BackupWriter, PatternReplaceConfig};

    // Get current directory as workspace root
    let workspace_root = env::current_dir()
        .map_err(|err| {
            splice::SpliceError::Other(format!("Failed to resolve current directory: {}", err))
        })?;

    // Convert CLI language to symbol language
    let symbol_language = language.map(|l| l.to_symbol_language());

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
    response_data.insert("replacements_count".to_string(), json!(result.replacements_count));
    if let Some(manifest_path) = backup_manifest_path {
        response_data.insert("backup_manifest".to_string(), json!(manifest_path.to_string_lossy()));
    }
    if let Some(op_id) = operation_id {
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

    Ok(splice::cli::CliSuccessPayload::with_data(message, serde_json::Value::Object(response_data)))
}

/// Execute the query command.
///
/// This function queries symbols by labels using Magellan integration.
fn execute_query(
    db_path: &Path,
    labels: &[String],
    list: bool,
    count: bool,
    show_code: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::MagellanIntegration;

    // Open Magellan integration
    let integration = MagellanIntegration::open(db_path)?;

    // List all labels mode
    if list {
        let all_labels = integration.get_all_labels()?;
        println!("{} labels in use:", all_labels.len());
        for label in &all_labels {
            let count = integration.count_by_label(label)?;
            println!("  {} ({})", label, count);
        }
        return Ok(splice::cli::CliSuccessPayload::message_only(format!(
            "Listed {} labels",
            all_labels.len()
        )));
    }

    // Count mode
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
        return Ok(splice::cli::CliSuccessPayload::with_data(
            format!("Counted entities for {} label(s)", labels.len()),
            json!(counts),
        ));
    }

    // Query mode - get symbols by label(s)
    if labels.is_empty() {
        return Err(splice::SpliceError::Other(
            "No labels specified. Use --label <LABEL> or --list to see all labels".to_string(),
        ));
    }

    let labels_ref: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    let results = integration.query_by_labels(&labels_ref)?;

    if results.is_empty() {
        if labels.len() == 1 {
            println!("No symbols found with label '{}'", labels[0]);
        } else {
            println!("No symbols found with labels: {}", labels.join(", "));
        }
        return Ok(splice::cli::CliSuccessPayload::message_only(
            "No symbols found".to_string(),
        ));
    }

    // Build response data
    let symbols_data: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            json!({
                "entity_id": r.entity_id,
                "name": r.name,
                "file_path": r.file_path,
                "kind": r.kind,
                "byte_start": r.byte_start,
                "byte_end": r.byte_end,
            })
        })
        .collect();

    // Print results to console
    if labels.len() == 1 {
        println!("{} symbols with label '{}':", results.len(), labels[0]);
    } else {
        println!(
            "{} symbols with labels [{}]:",
            results.len(),
            labels.join(", ")
        );
    }

    for result in &results {
        println!();
        println!(
            "  {} ({}) in {} [{}-{}]",
            result.name, result.kind, result.file_path, result.byte_start, result.byte_end
        );

        // Show code chunk if requested
        if show_code {
            let path = std::path::Path::new(&result.file_path);
            if let Ok(Some(code)) = integration.get_code_chunk(path, result.byte_start, result.byte_end) {
                for line in code.lines() {
                    println!("    {}", line);
                }
            }
        }
    }

    Ok(splice::cli::CliSuccessPayload::with_data(
        format!("Found {} symbols", results.len()),
        json!(symbols_data),
    ))
}

/// Execute the get command.
///
/// This function retrieves code chunks from the database using Magellan integration.
fn execute_get(
    db_path: &Path,
    file_path: &Path,
    start: usize,
    end: usize,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::MagellanIntegration;

    // Open Magellan integration
    let integration = MagellanIntegration::open(db_path)?;

    // Get code chunk
    let code = integration.get_code_chunk(file_path, start, end)?;

    match code {
        Some(content) => {
            // Print to console
            println!("{}", content);

            // Return success
            Ok(splice::cli::CliSuccessPayload::with_data(
                format!("Retrieved code chunk ({} bytes)", content.len()),
                json!({
                    "file": file_path.to_string_lossy(),
                    "byte_start": start,
                    "byte_end": end,
                    "content_length": content.len(),
                }),
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

/// Emit JSON payload for successful CLI responses.
fn emit_success_payload(payload: &splice::cli::CliSuccessPayload) {
    match serde_json::to_string(payload) {
        Ok(json) => println!("{}", json),
        Err(err) => {
            let fallback = json!({
                "status": "ok",
                "message": payload.message.clone(),
            });
            println!("{}", fallback.to_string());
            eprintln!("Serialization warning: {}", err);
        }
    }
}

/// Emit JSON payload for CLI errors.
fn emit_error_payload(payload: &splice::cli::CliErrorPayload) {
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
            eprintln!("{}", fallback.to_string());
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

fn build_success_payload(
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

fn find_workspace_root(path: &Path) -> Result<PathBuf, splice::SpliceError> {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.join("Cargo.toml").exists() {
            return Ok(dir.to_path_buf());
        }
        current = dir.parent();
    }

    Err(splice::SpliceError::Other(format!(
        "Cannot find Cargo.toml for {}",
        path.display()
    )))
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
