//! Suggested action engine with confidence assessment.
//!
//! This module provides action type recommendations and confidence levels
//! based on symbol uniqueness and ambiguity detection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of action to perform on a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    /// Delete a symbol by removing its definition.
    Delete,

    /// Replace a symbol's content with new code.
    Replace,

    /// Expand a symbol to include additional context.
    Expand,

    /// Query/search for symbols.
    Query,

    /// Read/retrieve symbol content.
    Read,
}

/// Confidence level in the action suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    /// High confidence: unique symbol with complete metadata.
    High,

    /// Medium confidence: some ambiguity or partial metadata.
    Medium,

    /// Low confidence: highly ambiguous with missing information.
    Low,
}

/// Suggested action with confidence assessment and reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    /// Type of action to perform.
    pub action_type: ActionType,

    /// Confidence level in this suggestion.
    pub confidence: Confidence,

    /// Human-readable explanation of why this action is suggested.
    pub reason: String,

    /// Optional additional parameters for the action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

impl Confidence {
    /// Calculate confidence based on resolution characteristics.
    ///
    /// # Arguments
    /// * `is_unique` - Symbol is uniquely resolved (single match)
    /// * `has_file` - File path is specified
    /// * `has_kind` - Symbol kind/type is available
    /// * `is_ambiguous` - AmbiguousSymbol error was encountered
    ///
    /// # Returns
    /// * `High` - Unique symbol with complete metadata
    /// * `Medium` - Some ambiguity or partial metadata
    /// * `Low` - Highly ambiguous with missing information
    pub fn calculate(is_unique: bool, has_file: bool, has_kind: bool, is_ambiguous: bool) -> Self {
        // High confidence: unique resolution with complete metadata
        if is_unique && has_file && has_kind && !is_ambiguous {
            return Confidence::High;
        }

        // Low confidence: name-only with multiple candidates or missing critical info
        if is_ambiguous || (!has_file && !is_unique) {
            return Confidence::Low;
        }

        // Medium confidence: partial metadata or some ambiguity
        if (has_file || has_kind) && !is_ambiguous {
            return Confidence::Medium;
        }

        // Default to low for uncertain cases
        Confidence::Low
    }
}

/// Suggest an action based on operation type and symbol information.
///
/// # Arguments
/// * `action_type` - Type of action to perform
/// * `symbol_name` - Name of the symbol
/// * `symbol_kind` - Kind/type of symbol (function, struct, etc.)
/// * `file_path` - Path to file containing symbol
/// * `confidence` - Confidence level of the resolution
///
/// # Returns
/// A `SuggestedAction` with type, confidence, reason, and optional parameters
pub fn suggest_action(
    action_type: ActionType,
    symbol_name: &str,
    symbol_kind: &str,
    file_path: &str,
    confidence: Confidence,
) -> SuggestedAction {
    let reason = generate_reason(action_type, symbol_name, symbol_kind, file_path);
    let params = generate_params(action_type);

    SuggestedAction {
        action_type,
        confidence,
        reason,
        params,
    }
}

/// Generate human-readable reason for the suggested action.
fn generate_reason(
    action_type: ActionType,
    symbol_name: &str,
    symbol_kind: &str,
    file_path: &str,
) -> String {
    match action_type {
        ActionType::Delete => {
            format!(
                "Delete symbol '{}' ({}) at {}",
                symbol_name, symbol_kind, file_path
            )
        }
        ActionType::Replace => {
            format!("Replace symbol '{}' with provided content", symbol_name)
        }
        ActionType::Expand => {
            format!(
                "Expand symbol '{}' to include full context (2 levels)",
                symbol_name
            )
        }
        ActionType::Query => {
            format!("Query for symbols matching specified labels")
        }
        ActionType::Read => {
            format!(
                "Read symbol '{}' ({}) at {}",
                symbol_name, symbol_kind, file_path
            )
        }
    }
}

/// Generate optional parameters for the action.
fn generate_params(action_type: ActionType) -> Option<HashMap<String, serde_json::Value>> {
    let mut params = HashMap::new();

    match action_type {
        ActionType::Delete => {
            params.insert(
                "remove_references".to_string(),
                serde_json::Value::Bool(true),
            );
        }
        ActionType::Replace => {
            params.insert(
                "preserve_signature".to_string(),
                serde_json::Value::Bool(true),
            );
        }
        ActionType::Expand => {
            params.insert("levels".to_string(), serde_json::Value::Number(2.into()));
        }
        ActionType::Query => {
            params.insert(
                "include_context".to_string(),
                serde_json::Value::Bool(false),
            );
        }
        ActionType::Read => {
            params.insert(
                "include_metadata".to_string(),
                serde_json::Value::Bool(true),
            );
        }
    }

    Some(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_high() {
        // Unique symbol with complete metadata
        let confidence = Confidence::calculate(true, true, true, false);
        assert_eq!(confidence, Confidence::High);
    }

    #[test]
    fn test_confidence_medium() {
        // Has file but not unique
        let confidence = Confidence::calculate(false, true, true, false);
        assert_eq!(confidence, Confidence::Medium);

        // Has kind but not file
        let confidence = Confidence::calculate(true, false, true, false);
        assert_eq!(confidence, Confidence::Medium);
    }

    #[test]
    fn test_confidence_low() {
        // Ambiguous symbol
        let confidence = Confidence::calculate(false, false, false, true);
        assert_eq!(confidence, Confidence::Low);

        // No file, not unique
        let confidence = Confidence::calculate(false, false, false, false);
        assert_eq!(confidence, Confidence::Low);
    }

    #[test]
    fn test_action_type_serialization() {
        // Test that action types serialize to lowercase
        let delete = serde_json::to_string(&ActionType::Delete).unwrap();
        assert_eq!(delete, "\"delete\"");

        let replace = serde_json::to_string(&ActionType::Replace).unwrap();
        assert_eq!(replace, "\"replace\"");

        let expand = serde_json::to_string(&ActionType::Expand).unwrap();
        assert_eq!(expand, "\"expand\"");
    }

    #[test]
    fn test_confidence_serialization() {
        // Test that confidence levels serialize to lowercase
        let high = serde_json::to_string(&Confidence::High).unwrap();
        assert_eq!(high, "\"high\"");

        let medium = serde_json::to_string(&Confidence::Medium).unwrap();
        assert_eq!(medium, "\"medium\"");

        let low = serde_json::to_string(&Confidence::Low).unwrap();
        assert_eq!(low, "\"low\"");
    }

    #[test]
    fn test_suggest_action_delete() {
        let action = suggest_action(
            ActionType::Delete,
            "my_function",
            "function",
            "src/lib.rs",
            Confidence::High,
        );

        assert_eq!(action.action_type, ActionType::Delete);
        assert_eq!(action.confidence, Confidence::High);
        assert!(action.reason.contains("Delete symbol"));
        assert!(action.reason.contains("my_function"));
        assert!(action.params.is_some());

        let params = action.params.unwrap();
        assert_eq!(
            params.get("remove_references"),
            Some(&serde_json::Value::Bool(true))
        );
    }

    #[test]
    fn test_suggest_action_replace() {
        let action = suggest_action(
            ActionType::Replace,
            "my_struct",
            "struct",
            "src/types.rs",
            Confidence::Medium,
        );

        assert_eq!(action.action_type, ActionType::Replace);
        assert_eq!(action.confidence, Confidence::Medium);
        assert!(action.reason.contains("Replace symbol"));

        let params = action.params.unwrap();
        assert_eq!(
            params.get("preserve_signature"),
            Some(&serde_json::Value::Bool(true))
        );
    }

    #[test]
    fn test_suggest_action_expand() {
        let action = suggest_action(
            ActionType::Expand,
            "MyClass",
            "class",
            "src/main.py",
            Confidence::High,
        );

        assert_eq!(action.action_type, ActionType::Expand);
        assert!(action.reason.contains("Expand symbol"));

        let params = action.params.unwrap();
        assert_eq!(
            params.get("levels"),
            Some(&serde_json::Value::Number(2.into()))
        );
    }

    #[test]
    fn test_suggested_action_serialization() {
        let action = SuggestedAction {
            action_type: ActionType::Delete,
            confidence: Confidence::High,
            reason: "Test reason".to_string(),
            params: None,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"action_type\":\"delete\""));
        assert!(json.contains("\"confidence\":\"high\""));
        assert!(json.contains("\"reason\":\"Test reason\""));
        // params should be omitted when None
        assert!(!json.contains("\"params\""));
    }
}
