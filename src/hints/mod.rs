//! Tool hints derivation for LLM guidance.
//!
//! This module provides behavioral flags for refactoring operations,
//! helping LLMs understand safe operations and potential side effects.

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
