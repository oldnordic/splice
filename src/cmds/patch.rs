//! Patch command handlers.

use std::path::{Path, PathBuf};

use serde_json::json;
use serde_json::Value;

use super::helpers::{
    capture_snapshot, extract_symbols_with_language, log_execution_error, require_patch_arg,
};

#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
pub(crate) fn execute_single_patch(
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
pub(crate) fn execute_patch(
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
    let workspace_root = splice::workspace::find_workspace_root(file_path)?;

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
        println!(
            "{}",
            serde_json::to_string_pretty(&result)
                .expect("invariant: serde_json serialization never fails on serializable types")
        );

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
