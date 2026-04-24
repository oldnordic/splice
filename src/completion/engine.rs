use crate::completion::types::{
    CompletionRequest, CompletionResponse, CompletionSuggestion,
    SuggestionSource, CompletionMetadata,
};
use crate::completion::context::CompletionContext;
use crate::completion::ranking::SuggestionRanker;
use crate::graph::MagellanIntegration;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use rusqlite::Connection;

pub struct CompletionEngine {
    magellan: Arc<MagellanIntegration>,
    db_path: PathBuf,
    ranker: SuggestionRanker,
}

impl CompletionEngine {
    pub fn new(magellan: Arc<MagellanIntegration>, db_path: &Path) -> Self {
        Self {
            magellan,
            db_path: db_path.to_path_buf(),
            ranker: SuggestionRanker::new(),
        }
    }

    /// Complete code at cursor position
    pub fn complete_at_cursor(
        &self,
        request: CompletionRequest,
    ) -> anyhow::Result<CompletionResponse> {
        let start = Instant::now();

        // Analyze context
        let context = CompletionContext::analyze(
            &request.file_path,
            request.line,
            request.column,
            &self.magellan,
        )?;

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
                total_symbols_indexed: self.get_symbol_count()?,
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
            suggestions.push(CompletionSuggestion {
                label: symbol.name.clone(),
                insert_text: symbol.name.clone(),
                detail: format!("{:?}", symbol.kind),
                kind: symbol.kind.clone(),
                score: 0.5, // Will be ranked
                source: SuggestionSource::Database,
                grounded_in: vec![symbol.id.clone()],
                usage_count: 1,
                last_used: None,
                source_file: None,  // Will be filled later for imports
                via_import: None,    // Will be filled later for imports
            });
        }

        Ok(suggestions)
    }

    fn get_symbol_count(&self) -> anyhow::Result<usize> {
        let query = "SELECT COUNT(*) FROM graph_entities";
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(query)?;
        let count: usize = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }
}
