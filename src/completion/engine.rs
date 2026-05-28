use crate::completion::context::CompletionContext;
use crate::completion::ranking::SuggestionRanker;
use crate::completion::types::{
    CompletionMetadata, CompletionRequest, CompletionResponse, CompletionSuggestion,
    SuggestionSource,
};
use crate::graph::MagellanIntegration;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Engine that provides code completion suggestions grounded in the database.
pub struct CompletionEngine {
    magellan: Arc<MagellanIntegration>,
    ranker: SuggestionRanker,
    /// Cached symbol count to avoid repeated queries
    symbol_count: usize,
    context_cache: Mutex<HashMap<CompletionCacheKey, CompletionContext>>,
}

type CompletionCacheKey = (PathBuf, usize, usize);

impl CompletionEngine {
    /// Create a new completion engine with the given Magellan integration and database path.
    pub fn new(magellan: Arc<MagellanIntegration>, db_path: &Path) -> Self {
        // Cache symbol count during initialization to avoid repeated queries
        let symbol_count = Self::get_symbol_count_once(db_path).unwrap_or(0);

        Self {
            magellan,
            ranker: SuggestionRanker::new(),
            symbol_count,
            context_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Complete code at cursor position
    pub fn complete_at_cursor(
        &self,
        request: CompletionRequest,
    ) -> anyhow::Result<CompletionResponse> {
        let start = Instant::now();

        let cache_key = (request.file_path.clone(), request.line, request.column);
        let cached_context = {
            self.context_cache
                .lock()
                .expect("invariant: mutex not poisoned")
                .get(&cache_key)
                .cloned()
        };
        let context = if let Some(context) = cached_context {
            context
        } else {
            let context = CompletionContext::analyze(
                &request.file_path,
                request.line,
                request.column,
                &self.magellan,
            )?;
            self.context_cache
                .lock()
                .expect("invariant: mutex not poisoned")
                .insert(cache_key, context.clone());
            context
        };

        // Get suggestions from database
        let mut suggestions = self.get_database_suggestions(&context)?;

        // Rank suggestions
        suggestions = self.ranker.rank_suggestions(suggestions, &context);

        // Limit results
        let max_results = request.max_results.unwrap_or(10);
        suggestions.truncate(max_results);

        let elapsed = start.elapsed();

        Ok(CompletionResponse {
            suggestions,
            metadata: CompletionMetadata {
                query_time_ms: elapsed.as_millis() as u64,
                total_symbols_indexed: self.symbol_count,
                database_version: 10, // Magellan v10+
                database_queries: 1,
            },
        })
    }

    fn get_database_suggestions(
        &self,
        context: &CompletionContext,
    ) -> anyhow::Result<Vec<CompletionSuggestion>> {
        let mut suggestions = Vec::new();

        // Query visible symbols
        for symbol in &context.visible_symbols {
            // Distinguish local symbols from imported symbols
            let (source, source_file, via_import) = if symbol.path
                == context.file_path.to_string_lossy()
            {
                // Local symbol (defined in current file)
                (SuggestionSource::Database, None, None)
            } else {
                // Imported symbol (defined in another file)
                // Convert path to use statement format
                // e.g., "/home/feanor/Projects/splice/src/patch/engine.rs" -> "use crate::patch::engine"
                let import_path = if symbol.path.contains("/src/") {
                    // Extract path relative to src/
                    let src_relative = symbol.path.split("/src/").nth(1).unwrap_or(&symbol.path);
                    let module_path = src_relative.trim_end_matches(".rs").replace("/", "::");

                    format!("use crate::{}", module_path)
                } else {
                    // Fallback for non-standard paths
                    format!("use {}", symbol.path)
                };

                (
                    SuggestionSource::Imported,
                    Some(symbol.path.clone()),
                    Some(import_path),
                )
            };

            suggestions.push(CompletionSuggestion {
                label: symbol.name.clone(),
                insert_text: symbol.name.clone(),
                detail: format!("{:?}", symbol.kind),
                kind: symbol.kind.clone(),
                score: 0.5, // Will be ranked
                source,
                grounded_in: vec![symbol.id.clone()],
                usage_count: 1,
                last_used: None,
                source_file,
                via_import,
            });
        }

        Ok(suggestions)
    }

    /// Get symbol count once (called during initialization)
    fn get_symbol_count_once(db_path: &Path) -> anyhow::Result<usize> {
        let query = "SELECT COUNT(*) FROM graph_entities";
        let conn = Connection::open(db_path)?;
        let mut stmt = conn.prepare(query)?;
        let count: usize = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }
}
