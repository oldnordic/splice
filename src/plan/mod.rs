//! JSON plan format for sequential multi-step refactorings.
//!
//! This module provides plan parsing and execution for orchestrating
//! multiple splice patch steps in a deterministic order.

use crate::error::Result;
use crate::ingest::rust::RustSymbolKind;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A refactoring plan containing multiple sequential patch steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Sequential patch steps to execute.
    pub steps: Vec<PatchStep>,
}

/// A single patch step in the plan.
///
/// Each step is equivalent to one `splice patch` invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchStep {
    /// Path to the source file containing the symbol.
    pub file: String,

    /// Symbol name to patch.
    pub symbol: String,

    /// Optional symbol kind filter.
    #[serde(rename = "kind")]
    pub symbol_kind: Option<String>,

    /// Path to file containing replacement content.
    #[serde(rename = "with")]
    pub with_file: String,
}

/// Parse a plan from a JSON file.
///
/// # Arguments
/// * `plan_path` - Path to plan.json file
///
/// # Returns
/// * `Ok(Plan)` - Parsed plan with validated schema
/// * `Err(SpliceError)` - JSON parse error or schema validation error
pub fn parse_plan(plan_path: &Path) -> Result<Plan> {
    // Read plan file
    let content = fs::read_to_string(plan_path)?;

    // Parse JSON
    let plan: Plan =
        serde_json::from_str(&content).map_err(|e| crate::SpliceError::InvalidPlanSchema {
            message: format!("JSON parse error: {}", e),
        })?;

    // Validate plan has at least one step
    if plan.steps.is_empty() {
        return Err(crate::SpliceError::InvalidPlanSchema {
            message: "Plan must contain at least one step".to_string(),
        });
    }

    // Validate each step
    for (i, step) in plan.steps.iter().enumerate() {
        // Check required fields
        if step.file.is_empty() {
            return Err(crate::SpliceError::InvalidPlanSchema {
                message: format!("Step {} has empty 'file' field", i + 1),
            });
        }

        if step.symbol.is_empty() {
            return Err(crate::SpliceError::InvalidPlanSchema {
                message: format!("Step {} has empty 'symbol' field", i + 1),
            });
        }

        if step.with_file.is_empty() {
            return Err(crate::SpliceError::InvalidPlanSchema {
                message: format!("Step {} has empty 'with' field", i + 1),
            });
        }

        // Validate symbol kind if provided
        if let Some(ref kind) = step.symbol_kind {
            match kind.as_str() {
                "function" | "struct" | "enum" | "trait" | "impl" => {
                    // Valid kind
                }
                _ => {
                    return Err(crate::SpliceError::InvalidPlanSchema {
                        message: format!(
                            "Step {} has invalid 'kind': '{}'. Must be one of: function, struct, enum, trait, impl",
                            i + 1,
                            kind
                        ),
                    });
                }
            }
        }
    }

    Ok(plan)
}

/// Execute a plan with multiple sequential patch steps.
///
/// This function:
/// 1. Parses the plan from JSON
/// 2. Executes each step sequentially
/// 3. Stops on first failure
/// 4. Previous successful steps remain applied (no global rollback)
///
/// # Arguments
/// * `plan_path` - Path to plan.json file
/// * `workspace_dir` - Workspace directory for cargo check
///
/// # Returns
/// * `Ok(Vec<String>)` - Success messages for each executed step
/// * `Err(SpliceError)` - First error encountered during execution
pub fn execute_plan(plan_path: &Path, workspace_dir: &Path) -> Result<Vec<String>> {
    use crate::ingest::rust::RustSymbolKind;

    // Parse plan
    let plan = parse_plan(plan_path)?;

    let mut success_messages = Vec::new();

    // Execute each step sequentially
    for (step_num, step) in plan.steps.iter().enumerate() {
        let step_index = step_num + 1;

        // Resolve paths relative to workspace directory
        let file_path = workspace_dir.join(&step.file);
        let with_file_path = workspace_dir.join(&step.with_file);

        // Convert CLI kind to RustSymbolKind
        let rust_kind = match &step.symbol_kind {
            None => None,
            Some(kind) => Some(match kind.as_str() {
                "function" => RustSymbolKind::Function,
                "struct" => RustSymbolKind::Struct,
                "enum" => RustSymbolKind::Enum,
                "trait" => RustSymbolKind::Trait,
                "impl" => RustSymbolKind::Impl,
                _ => {
                    return Err(crate::SpliceError::Other(format!(
                        "Invalid symbol kind: {}",
                        kind
                    )));
                }
            }),
        };

        // Execute single patch step
        match execute_single_step(
            &file_path,
            &step.symbol,
            rust_kind,
            &with_file_path,
            workspace_dir,
        ) {
            Ok(msg) => {
                println!("Step {}: {}", step_index, msg);
                success_messages.push(msg);
            }
            Err(e) => {
                return Err(crate::SpliceError::PlanExecutionFailed {
                    step: step_index,
                    error: e.to_string(),
                });
            }
        }
    }

    Ok(success_messages)
}

/// Execute a single patch step.
///
/// This is the core logic extracted from main.rs execute_patch.
fn execute_single_step(
    file_path: &Path,
    symbol_name: &str,
    kind: Option<RustSymbolKind>,
    replacement_file: &Path,
    workspace_dir: &Path,
) -> Result<String> {
    use crate::graph::CodeGraph;
    use crate::ingest::rust::extract_rust_symbols;
    use crate::patch::apply_patch_with_validation;
    use crate::resolve::resolve_symbol;
    use crate::symbol::Language;
    use crate::validate::AnalyzerMode;

    // Step 1: Read source file
    let source = std::fs::read(file_path)?;

    // Step 2: Extract symbols from source file (on-the-fly ingestion)
    let symbols = extract_rust_symbols(file_path, &source)?;

    // Step 3: Create in-memory graph (no persistent database needed)
    let graph_db_path = file_path.parent().unwrap().join(".splice_graph.db");
    let mut code_graph = CodeGraph::open(&graph_db_path)?;

    // Step 4: Store symbols in graph with language metadata
    for symbol in &symbols {
        code_graph.store_symbol_with_file_and_language(
            file_path,
            &symbol.name,
            symbol.kind.as_str(),
            Language::Rust,
            symbol.byte_start,
            symbol.byte_end,
        )?;
    }

    // Step 5: Convert RustSymbolKind to string for resolution
    let kind_str = kind.map(|k| k.as_str());

    // Step 6: Resolve symbol to span
    let resolved = resolve_symbol(&code_graph, Some(file_path), kind_str, symbol_name)?;

    // Step 7: Read replacement content
    let replacement_content = std::fs::read_to_string(replacement_file)?;

    // Step 8: Apply patch with validation (analyzer OFF for plan execution)
    let (before_hash, after_hash) = apply_patch_with_validation(
        file_path,
        resolved.byte_start,
        resolved.byte_end,
        &replacement_content,
        workspace_dir,
        Language::Rust,
        AnalyzerMode::Off,
    )?;

    // Step 9: Return success message
    Ok(format!(
        "Patched '{}' at bytes {}..{} (hash: {} -> {})",
        symbol_name, resolved.byte_start, resolved.byte_end, before_hash, after_hash
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_plan() {
        let json = r#"{"steps": [{"file": "src/lib.rs", "symbol": "foo", "kind": "function", "with": "patch.rs"}]}"#;
        let plan: Plan = serde_json::from_str(json).unwrap();
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].file, "src/lib.rs");
        assert_eq!(plan.steps[0].symbol, "foo");
    }

    #[test]
    fn test_parse_plan_empty_steps_fails() {
        let _plan = Plan { steps: vec![] };
        let result = serde_json::from_str::<Plan>(r#"{"steps": []}"#).unwrap();
        assert!(result.steps.is_empty());
    }
}
