//! Search, find, refs, and files command handlers.

use std::path::{Path, PathBuf};

use serde_json::json;
use serde_json::Value;

#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
pub(crate) fn execute_search(
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

    let (ctx_before, ctx_after) =
        resolve_context_counts(context_before, context_after, context_both);

    let symbol_lang = language.map(|l: splice::cli::Language| l.to_symbol_language());

    const ALL_EXTENSIONS: &[&str] = &[
        "rs", "py", "c", "cpp", "h", "hpp", "cc", "cxx", "java", "js", "mjs", "cjs", "ts", "tsx",
    ];
    let glob_patterns: Vec<String> = if let Some(g) = glob {
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

    let apply_replace = apply && replace.is_some();

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
            for m in &matches {
                println!(
                    "{}:{}:{}: {}",
                    m.file.display(),
                    m.line,
                    m.column,
                    m.matched_text
                );

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

pub(crate) fn execute_find(
    db_path: &Path,
    name: Option<String>,
    symbol_id: Option<String>,
    semantic_query: Option<&str>,
    ambiguous: bool,
    _output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::MagellanIntegration;

    let mut integration = MagellanIntegration::open(db_path)?;

    let results = if let Some(id) = symbol_id {
        match integration.find_symbol_by_id(&id)? {
            Some(symbol) => vec![symbol],
            None => {
                return Err(splice::SpliceError::symbol_not_found(
                    format!("ID '{}'", id),
                    Some(db_path),
                ))
            }
        }
    } else if let Some(query) = semantic_query {
        let top_k = if ambiguous { 20 } else { 5 };
        match splice::query::semantic::resolve_semantic_to_symbols(db_path, query, top_k)
            .map_err(|e| splice::SpliceError::Other(format!("Semantic search failed: {e}")))?
        {
            Some(symbols) => symbols,
            None => {
                return Err(splice::SpliceError::Other(
                    "No semantic matches found (ensure HNSW index exists and embeddings are generated)"
                        .to_string(),
                ));
            }
        }
    } else if let Some(ref n) = name {
        integration.find_symbol_by_name(n, ambiguous)?
    } else {
        return Err(splice::SpliceError::Other(
            "--name, --symbol-id, or --semantic-query required".to_string(),
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

pub(crate) fn execute_refs(
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

pub(crate) fn execute_files(
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
