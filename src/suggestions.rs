//! Symbol suggestion functions using fuzzy matching.
//!
//! Provides "did you mean" functionality for SymbolNotFound errors
//! using Levenshtein distance to find similar symbol names.

use strsim::levenshtein;

/// Find symbols similar to target within max edit distance.
///
/// Returns up to 5 suggestions sorted by similarity (closest first).
/// Uses prefix filtering to avoid expensive distance calculation for
/// obviously-different symbols.
///
/// # Arguments
/// * `target` - The symbol name that was not found
/// * `candidates` - All available symbol names to search
/// * `max_distance` - Maximum edit distance (default: 3)
///
/// # Performance
/// - Filters by first character match before computing distance
/// - Early-excludes candidates with distance > max_distance
/// - Returns at most 5 suggestions to avoid overwhelming output
///
/// # Examples
/// ```
/// use splice::suggestions::suggest_similar_symbols;
///
/// let symbols = vec!["foo".to_string(), "foobar".to_string()];
/// let suggestions = suggest_similar_symbols("fooo", &symbols, 2);
/// assert_eq!(suggestions, vec!["foo".to_string()]);
/// ```
pub fn suggest_similar_symbols(
    target: &str,
    candidates: &[String],
    max_distance: usize,
) -> Vec<String> {
    // Quick checks for performance
    if target.is_empty() || candidates.is_empty() {
        return Vec::new();
    }

    // Get first char of target for prefix filtering
    let target_first = match target.chars().next() {
        Some(c) => c,
        None => return Vec::new(),
    };

    let mut suggestions: Vec<_> = candidates
        .iter()
        .filter(|symbol| {
            // Quick prefix check to avoid expensive distance calculation
            // Only compute distance for symbols starting with same character
            symbol.starts_with(target_first)
        })
        .map(|symbol| {
            let dist = levenshtein(target, symbol);
            (symbol.clone(), dist)
        })
        .filter(|(_, dist)| {
            // Exclude exact matches (distance 0) and distant matches
            *dist > 0 && *dist <= max_distance
        })
        .collect();

    // Sort by distance (closest matches first)
    suggestions.sort_by_key(|(_, dist)| *dist);

    // Take top 5 suggestions
    suggestions.truncate(5);
    suggestions.into_iter().map(|(name, _)| name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_similar_symbols_exact_match_excluded() {
        let symbols = vec!["foo".to_string()];
        let suggestions = suggest_similar_symbols("foo", &symbols, 3);
        assert_eq!(suggestions.len(), 0, "Exact match should not be suggested");
    }

    #[test]
    fn test_suggest_similar_single_edit() {
        let symbols = vec![
            "foo".to_string(),
            "foobar".to_string(),
            "bar".to_string(),
            "baz".to_string(),
        ];
        let suggestions = suggest_similar_symbols("fooo", &symbols, 2);
        assert_eq!(suggestions, vec!["foo".to_string()]);
    }

    #[test]
    fn test_suggest_similar_transposition() {
        let symbols = vec!["apple".to_string(), "banana".to_string()];
        let suggestions = suggest_similar_symbols("appel", &symbols, 2);
        assert_eq!(suggestions, vec!["apple".to_string()]);
    }

    #[test]
    fn test_suggest_similar_empty_target() {
        let symbols = vec!["foo".to_string()];
        let suggestions = suggest_similar_symbols("", &symbols, 3);
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_suggest_similar_limit_to_five() {
        let symbols = vec![
            "foo1".to_string(),
            "foo2".to_string(),
            "foo3".to_string(),
            "foo4".to_string(),
            "foo5".to_string(),
            "foo6".to_string(),
        ];
        let suggestions = suggest_similar_symbols("foox", &symbols, 3);
        assert!(suggestions.len() <= 5, "Should limit to 5 suggestions");
    }
}
