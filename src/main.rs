//! Splice CLI binary
//!
//! This is the main entry point for the splice command-line interface.
//! The CLI is a thin adapter over existing APIs - NO logic is implemented here.

use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Parse CLI arguments
    let cli = splice::cli::parse_args();

    // Initialize logger if verbose
    if cli.verbose {
        env_logger::init();
    }

    // Execute command
    let result = match cli.command {
        splice::cli::Commands::Delete {
            file,
            symbol,
            kind,
            analyzer,
        } => execute_delete(&file, &symbol, kind, analyzer),

        splice::cli::Commands::Patch {
            file,
            symbol,
            kind,
            analyzer,
            with_: replacement_file,
        } => execute_patch(&file, &symbol, kind, analyzer, &replacement_file),

        splice::cli::Commands::Plan { file } => execute_plan(&file),
    };

    // Handle result
    match result {
        Ok(msg) => {
            println!("{}", msg);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(1)
        }
    }
}

/// Execute the delete command.
///
/// This function is a thin adapter that:
/// 1. Extracts symbols from source file
/// 2. Finds all references to the symbol (same-file and cross-file)
/// 3. Deletes all references first (in reverse byte order per file)
/// 4. Deletes the definition last
/// 5. Applies each deletion with validation gates
///
/// All logic is delegated to existing APIs.
fn execute_delete(
    file_path: &Path,
    symbol_name: &str,
    kind: Option<splice::cli::SymbolKind>,
    analyzer: Option<splice::cli::AnalyzerMode>,
) -> Result<String, splice::SpliceError> {
    use splice::graph::CodeGraph;
    use splice::ingest::rust::{extract_rust_symbols, RustSymbolKind};
    use splice::patch::apply_patch_with_validation;
    use splice::resolve::references::find_references;
    use splice::validate::AnalyzerMode as ValidateAnalyzerMode;

    // Step 1: Read source file
    let source = std::fs::read(file_path)?;

    // Step 2: Extract symbols from source file (on-the-fly ingestion)
    let symbols = extract_rust_symbols(file_path, &source)?;

    // Step 3: Create in-memory graph (for reference finding API compatibility)
    let graph_db_path = file_path.parent().unwrap().join(".splice_graph.db");
    let mut code_graph = CodeGraph::open(&graph_db_path)?;

    // Step 4: Store symbols in graph
    for symbol in &symbols {
        code_graph.store_symbol_with_file(
            file_path,
            &symbol.name,
            symbol.kind,
            symbol.byte_start,
            symbol.byte_end,
        )?;
    }

    // Step 5: Convert CLI kind to RustSymbolKind
    let rust_kind = kind.map(|k| match k {
        splice::cli::SymbolKind::Function => RustSymbolKind::Function,
        splice::cli::SymbolKind::Struct => RustSymbolKind::Struct,
        splice::cli::SymbolKind::Enum => RustSymbolKind::Enum,
        splice::cli::SymbolKind::Trait => RustSymbolKind::Trait,
        splice::cli::SymbolKind::Impl => RustSymbolKind::Impl,
    });

    // Step 6: Find all references to the symbol
    let ref_set = find_references(&code_graph, file_path, symbol_name, rust_kind)?;

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
        refs_by_file
            .entry(r.file_path.clone())
            .or_default()
            .push(r);
    }

    // Sort each file's references by byte offset descending
    for refs in refs_by_file.values_mut() {
        refs.sort_by_key(|r| std::cmp::Reverse(r.byte_start));
    }

    // Step 10: Delete references from each file
    let mut deleted_count = 0;
    let mut files_modified = Vec::new();

    for (file_path_str, refs) in refs_by_file {
        let path = Path::new(&file_path_str);

        // Delete each reference in this file (highest byte offset first)
        for r in refs {
            apply_patch_with_validation(
                path,
                r.byte_start,
                r.byte_end,
                "", // Delete = replace with empty
                workspace_dir,
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
        analyzer_mode,
    )?;
    deleted_count += 1;

    // Track the definition file as modified
    let def_file_path = file_path.to_str().unwrap_or("").to_string();
    if !files_modified.contains(&def_file_path) {
        files_modified.push(def_file_path);
    }

    // Step 12: Return success message
    if ref_set.has_glob_ambiguity {
        Ok(format!(
            "Deleted '{}' ({} references + definition) across {} file(s). WARNING: glob imports detected - some references may have been missed.",
            symbol_name,
            deleted_count - 1,
            files_modified.len()
        ))
    } else {
        Ok(format!(
            "Deleted '{}' ({} references + definition) across {} file(s).",
            symbol_name,
            deleted_count - 1,
            files_modified.len()
        ))
    }
}

/// Execute the patch command.
///
/// This function is a thin adapter that:
/// 1. Extracts symbols from source file
/// 2. Resolves the target symbol to its byte span
/// 3. Reads replacement content from file
/// 4. Applies patch with validation gates
///
/// All logic is delegated to existing APIs.
fn execute_patch(
    file_path: &Path,
    symbol_name: &str,
    kind: Option<splice::cli::SymbolKind>,
    analyzer: Option<splice::cli::AnalyzerMode>,
    replacement_file: &Path,
) -> Result<String, splice::SpliceError> {
    use splice::graph::CodeGraph;
    use splice::ingest::rust::{extract_rust_symbols, RustSymbolKind};
    use splice::patch::apply_patch_with_validation;
    use splice::resolve::resolve_symbol;
    use splice::validate::AnalyzerMode as ValidateAnalyzerMode;

    // Step 1: Read source file
    let source = std::fs::read(file_path)?;

    // Step 2: Extract symbols from source file (on-the-fly ingestion)
    let symbols = extract_rust_symbols(file_path, &source)?;

    // Step 3: Create in-memory graph (no persistent database needed)
    let graph_db_path = file_path.parent().unwrap().join(".splice_graph.db");
    let mut code_graph = CodeGraph::open(&graph_db_path)?;

    // Step 4: Store symbols in graph
    for symbol in &symbols {
        code_graph.store_symbol_with_file(
            file_path,
            &symbol.name,
            symbol.kind,
            symbol.byte_start,
            symbol.byte_end,
        )?;
    }

    // Step 5: Convert CLI kind to RustSymbolKind
    let rust_kind = kind.map(|k| match k {
        splice::cli::SymbolKind::Function => RustSymbolKind::Function,
        splice::cli::SymbolKind::Struct => RustSymbolKind::Struct,
        splice::cli::SymbolKind::Enum => RustSymbolKind::Enum,
        splice::cli::SymbolKind::Trait => RustSymbolKind::Trait,
        splice::cli::SymbolKind::Impl => RustSymbolKind::Impl,
    });

    // Step 6: Resolve symbol to span
    let resolved = resolve_symbol(&code_graph, Some(file_path), rust_kind, symbol_name)?;

    // Step 7: Read replacement content
    let replacement_content = std::fs::read_to_string(replacement_file)?;

    // Step 8: Determine workspace directory (parent of source file)
    let workspace_dir = file_path.parent().ok_or_else(|| {
        splice::SpliceError::Other("Cannot determine workspace directory".to_string())
    })?;

    // Step 9: Convert CLI analyzer mode to validate analyzer mode (default to Off)
    let analyzer_mode = match analyzer {
        Some(splice::cli::AnalyzerMode::Off) => ValidateAnalyzerMode::Off,
        Some(splice::cli::AnalyzerMode::Os) => ValidateAnalyzerMode::Path,
        Some(splice::cli::AnalyzerMode::Path) => {
            // TODO: Support explicit path in future extension
            return Err(splice::SpliceError::Other(
                "Explicit analyzer path not yet supported".to_string(),
            ));
        }
        None => ValidateAnalyzerMode::Off, // Default to OFF
    };

    // Step 10: Apply patch with validation
    let (before_hash, after_hash) = apply_patch_with_validation(
        file_path,
        resolved.byte_start,
        resolved.byte_end,
        &replacement_content,
        workspace_dir,
        analyzer_mode,
    )?;

    // Step 11: Return success message
    Ok(format!(
        "Patched '{}' at bytes {}..{} (hash: {} -> {})",
        symbol_name, resolved.byte_start, resolved.byte_end, before_hash, after_hash
    ))
}

/// Execute the plan command.
///
/// This function is a thin adapter that:
/// 1. Reads the plan.json file
/// 2. Calls execute_plan from the plan module
///
/// All logic is delegated to the plan module.
fn execute_plan(plan_path: &Path) -> Result<String, splice::SpliceError> {
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
    Ok(format!(
        "Plan executed successfully: {} steps completed",
        messages.len()
    ))
}
