//! Tool hints derivation for LLM guidance.
//!
//! This module provides behavioral flags for refactoring operations,
//! helping LLMs understand safe operations and potential side effects.

use crate::ingest::SemanticKind;
use serde::Serialize;

/// Operation type for tool hint derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolHintOperation {
    /// Delete function body only
    DeleteBody,
    /// Change function signature (parameters, return type)
    ChangeSignature,
    /// Change type definition (struct fields, enum variants)
    ChangeType,
    /// Replace entire function body
    ReplaceBody,
}

/// Behavioral hints for refactoring operations.
///
/// Provides LLMs with guidance on safe operations and potential side effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolHints {
    /// Whether this operation requires full context (closures, impl blocks).
    pub requires_full_context: bool,

    /// Whether this operation must be applied atomically (always true for splice).
    pub apply_atomically: bool,

    /// Whether this operation may break tests (public functions, trait changes).
    pub may_break_tests: bool,

    /// Whether this operation requires compilation (type-changing operations).
    pub requires_compilation: bool,
}

impl Default for ToolHints {
    fn default() -> Self {
        Self {
            requires_full_context: false,
            apply_atomically: true,  // All splice operations are atomic
            may_break_tests: false,
            requires_compilation: false,
        }
    }
}

impl ToolHints {
    /// Create a new ToolHints with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set requires_full_context flag.
    pub fn with_requires_full_context(mut self, value: bool) -> Self {
        self.requires_full_context = value;
        self
    }

    /// Set may_break_tests flag.
    pub fn with_may_break_tests(mut self, value: bool) -> Self {
        self.may_break_tests = value;
        self
    }

    /// Set requires_compilation flag.
    pub fn with_requires_compilation(mut self, value: bool) -> Self {
        self.requires_compilation = value;
        self
    }
}

/// Derive tool hints from symbol metadata and operation type.
///
/// Analyzes the semantic kind, visibility, and operation to determine
/// appropriate behavioral flags for LLM guidance.
///
/// # Arguments
///
/// * `semantic_kind` - The kind of symbol (function, type, trait, etc.)
/// * `is_public` - Whether the symbol is publicly visible
/// * `operation` - The type of operation being performed
///
/// # Returns
///
/// ToolHints with appropriate flags set based on static analysis
pub fn derive_tool_hints(
    semantic_kind: SemanticKind,
    is_public: bool,
    operation: ToolHintOperation,
) -> ToolHints {
    // Determine requires_full_context
    // True for: closures, impl blocks (traits from impl_item)
    let requires_full_context = matches!(
        (semantic_kind, operation),
        (SemanticKind::Function, ToolHintOperation::DeleteBody) |
        (SemanticKind::Function, ToolHintOperation::ReplaceBody) |
        (SemanticKind::Trait, _)
    );

    // apply_atomically is always true (all splice operations are atomic)
    let apply_atomically = true;

    // Determine may_break_tests using static heuristic
    // True for: public functions, trait signatures, impl blocks
    let may_break_tests = match (semantic_kind, is_public) {
        (SemanticKind::Function, true) => true,
        (SemanticKind::Trait, true) => true,
        (SemanticKind::Type, true) => true,
        _ => false,
    };

    // Determine requires_compilation
    // True for: type-changing operations, public signature changes
    let requires_compilation = match operation {
        ToolHintOperation::ChangeType => true,
        ToolHintOperation::ChangeSignature if is_public => true,
        ToolHintOperation::DeleteBody if is_public => true,
        _ => false,
    };

    ToolHints {
        requires_full_context,
        apply_atomically,
        may_break_tests,
        requires_compilation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_hints_default() {
        let hints = ToolHints::default();
        assert_eq!(hints.requires_full_context, false);
        assert_eq!(hints.apply_atomically, true);
        assert_eq!(hints.may_break_tests, false);
        assert_eq!(hints.requires_compilation, false);
    }

    #[test]
    fn test_tool_hints_builder() {
        let hints = ToolHints::new()
            .with_requires_full_context(true)
            .with_may_break_tests(true)
            .with_requires_compilation(true);

        assert_eq!(hints.requires_full_context, true);
        assert_eq!(hints.apply_atomically, true);  // Always true
        assert_eq!(hints.may_break_tests, true);
        assert_eq!(hints.requires_compilation, true);
    }

    #[test]
    fn test_tool_hints_serialization() {
        let hints = ToolHints::new()
            .with_requires_full_context(true)
            .with_may_break_tests(false);

        let json = serde_json::to_string(&hints).unwrap();
        assert!(json.contains("\"requires_full_context\":true"));
        assert!(json.contains("\"apply_atomically\":true"));
        assert!(json.contains("\"may_break_tests\":false"));
    }
}
