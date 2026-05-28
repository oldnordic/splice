//! Query and get command handlers.

use serde_json::json;
use std::path::Path;

use super::helpers::{log_execution_error, write_stdout_bytes, write_stdout_line};

/// Execute the query command.
///
/// This function queries symbols by labels using Magellan integration.
#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
#[allow(unused_variables, reason = "stub args reserved for future expansion")]
pub(crate) fn execute_query(
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
        println!(
            "{}",
            serde_json::to_string_pretty(&result)
                .expect("invariant: serde_json serialization never fails on serializable types")
        );

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
pub(crate) fn execute_get(
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
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).expect(
                        "invariant: serde_json serialization never fails on serializable types"
                    )
                );

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
