use crate::completion::context::CompletionContext;
use crate::completion::types::{CompletionSuggestion, SuggestionSource, SymbolKind};

/// Ranks completion suggestions by relevance.
pub struct SuggestionRanker {
    weights: RankingWeights,
}

/// Weights for ranking factors.
#[derive(Debug, Clone)]
pub struct RankingWeights {
    /// Weight for usage frequency score.
    pub frequency: f32,
    /// Weight for proximity to cursor score.
    pub proximity: f32,
    /// Weight for symbol kind match score.
    pub kind_match: f32,
    /// Weight for text match score.
    pub text_match: f32,
    /// Weight for import proximity score.
    pub import_proximity: f32,
}

impl Default for SuggestionRanker {
    fn default() -> Self {
        Self {
            weights: RankingWeights {
                frequency: 0.25,
                proximity: 0.25,
                kind_match: 0.15,
                text_match: 0.20,
                import_proximity: 0.15,
            },
        }
    }
}

impl SuggestionRanker {
    /// Create a ranker with default weights.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rank suggestions by relevance to the given context.
    pub fn rank_suggestions(
        &self,
        suggestions: Vec<CompletionSuggestion>,
        context: &CompletionContext,
    ) -> Vec<CompletionSuggestion> {
        // Filter by current token if user is typing something
        let mut filtered = if let Some(ref token) = context.current_token {
            let token_lower = token.to_lowercase();
            suggestions
                .into_iter()
                .filter(|s| {
                    s.label.to_lowercase().starts_with(&token_lower)
                        || s.label.to_lowercase().contains(&token_lower)
                })
                .collect()
        } else {
            suggestions
        };

        for suggestion in &mut filtered {
            suggestion.score = self.calculate_score(suggestion, context);
        }

        let mut ranked: Vec<CompletionSuggestion> = filtered;
        ranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        ranked
    }

    fn calculate_score(
        &self,
        suggestion: &CompletionSuggestion,
        context: &CompletionContext,
    ) -> f32 {
        let frequency_score = self.normalize_frequency(suggestion.usage_count);
        let proximity_score = self.calculate_proximity(suggestion, context);
        let kind_score = self.calculate_kind_match(suggestion, context);
        let text_match_score = self.calculate_text_match(suggestion, context);
        let import_score = self.calculate_import_proximity(suggestion, context);

        self.weights.frequency * frequency_score
            + self.weights.proximity * proximity_score
            + self.weights.kind_match * kind_score
            + self.weights.text_match * text_match_score
            + self.weights.import_proximity * import_score
    }

    fn normalize_frequency(&self, count: usize) -> f32 {
        1.0 - (1.0 / (1.0 + count as f32))
    }

    fn calculate_proximity(
        &self,
        suggestion: &CompletionSuggestion,
        context: &CompletionContext,
    ) -> f32 {
        // Check if suggestion is in same function/module
        if let Some(ref func) = context.enclosing_function {
            // Same function = high proximity
            if suggestion.grounded_in.contains(&func.id) {
                return 1.0;
            }
        }

        // Check if suggestion is imported (cross-file but in scope)
        if matches!(suggestion.source, SuggestionSource::Imported) {
            return 0.85; // High: explicitly imported
        }

        // Same file = medium proximity
        if matches!(suggestion.source, SuggestionSource::Database) {
            return 0.6; // Medium: same file
        }

        0.3 // Low: other sources
    }

    fn calculate_kind_match(
        &self,
        suggestion: &CompletionSuggestion,
        _context: &CompletionContext,
    ) -> f32 {
        // Functions preferred when in function body
        // Structs preferred when in type context
        match suggestion.kind {
            SymbolKind::Function => 0.8,
            SymbolKind::Struct => 0.7,
            SymbolKind::Enum => 0.7,
            SymbolKind::Trait => 0.6,
            SymbolKind::Module => 0.5,
            _ => 0.4,
        }
    }

    fn calculate_text_match(
        &self,
        suggestion: &CompletionSuggestion,
        context: &CompletionContext,
    ) -> f32 {
        if let Some(ref token) = context.current_token {
            let label_lower = suggestion.label.to_lowercase();
            let token_lower = token.to_lowercase();

            // Exact prefix match = highest score
            if label_lower.starts_with(&token_lower) {
                return 1.0;
            }

            // Contains match = medium score
            if label_lower.contains(&token_lower) {
                return 0.7;
            }

            // No match = low score (shouldn't happen due to filtering)
            0.3
        } else {
            // No token filter = neutral score
            0.5
        }
    }

    fn calculate_import_proximity(
        &self,
        suggestion: &CompletionSuggestion,
        _context: &CompletionContext,
    ) -> f32 {
        if matches!(suggestion.source, SuggestionSource::Imported) {
            // Explicit imports get high score
            if suggestion.via_import.is_some() {
                return 1.0;
            }
            return 0.7;
        }

        0.0 // Not imported
    }
}
