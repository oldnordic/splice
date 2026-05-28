//! Delete command handler.

use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};

use super::helpers::{count_lines_in_span, extract_symbols_with_language, log_execution_error};

#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
pub(crate) fn execute_delete(
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

        let workspace_root = splice::workspace::find_workspace_root(file_path)?;
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

    Ok(splice::cli::CliSuccessPayload::with_data(
        base_message,
        serde_json::Value::Object(response_data),
    ))
}
