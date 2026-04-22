//! CFG complexity analysis using Mirage's 4D spatial coordinates
//!
//! This module integrates with Mirage to provide CFG complexity metrics
//! before performing potentially risky edits.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Complexity analysis results from Mirage CFG data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityAnalysis {
    /// Maximum branch distance (coord_z) - indicates control flow complexity
    pub max_branch_distance: i64,
    /// Maximum dominator depth (coord_x) - indicates nesting depth
    pub max_dominator_depth: i64,
    /// Maximum loop nesting (coord_y) - indicates loop complexity
    pub max_loop_nesting: i64,
    /// Total number of basic blocks in the function
    pub total_blocks: usize,
    /// Whether cycles were detected in the function
    pub has_cycles: bool,
    /// Risk level assessment
    pub risk_level: RiskLevel,
}

/// Risk level for editing a function
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    /// Low risk: Simple control flow, no loops
    Low,
    /// Medium risk: Some complexity, moderate nesting
    Medium,
    /// High risk: Complex control flow, deep nesting, many branches
    High,
    /// Very high risk: Extremely complex, multiple cycles
    VeryHigh,
}

impl ComplexityAnalysis {
    /// Determine risk level based on 4D coordinates
    pub fn assess_risk_level(&self) -> RiskLevel {
        // High branch distance = complex control flow
        if self.max_branch_distance > 15 || self.has_cycles {
            return RiskLevel::VeryHigh;
        }

        // High dominator depth = deeply nested code
        if self.max_dominator_depth > 6 || self.max_branch_distance > 10 {
            return RiskLevel::High;
        }

        // Moderate complexity
        if self.max_dominator_depth > 3 || self.max_branch_distance > 5 || self.max_loop_nesting > 1 {
            return RiskLevel::Medium;
        }

        RiskLevel::Low
    }
}

/// Check function complexity using Mirage's 4D spatial coordinates
///
/// # Arguments
/// * `db_path` - Path to Magellan database (e.g., ".magellan/magellan.db")
/// * `function_name` - Name of the function to analyze
/// * `file_path` - Path to the source file (for context)
///
/// # Returns
/// * `Ok(ComplexityAnalysis)` - Complexity metrics
/// * `Err(SpliceError)` - Mirage not available or query failed
pub fn check_function_complexity(
    db_path: &Path,
    function_name: &str,
    _file_path: &Path,
) -> Result<ComplexityAnalysis> {
    // Call mirage CLI to get CFG with 4D coordinates
    let output = Command::new("mirage")
        .arg("--db")
        .arg(db_path)
        .arg("--output")
        .arg("json")
        .arg("cfg")
        .arg("--function")
        .arg(function_name)
        .output();

    let output = match output {
        Ok(output) => output,
        Err(e) => {
            // Mirage not available - return safe defaults
            log::warn!("Mirage not available for complexity analysis: {}", e);
            return Ok(ComplexityAnalysis::safe_default());
        }
    };

    if !output.status.success() {
        // Mirage query failed - function might not exist or other error
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::warn!("Mirage query failed: {}", stderr);
        return Ok(ComplexityAnalysis::safe_default());
    }

    // Parse JSON output
    let json_str = String::from_utf8_lossy(&output.stdout);
    let mirage_data: MirageData = match serde_json::from_str(&json_str) {
        Ok(data) => data,
        Err(e) => {
            log::warn!("Failed to parse Mirage output: {}", e);
            return Ok(ComplexityAnalysis::safe_default());
        }
    };

    // Extract 4D coordinates from blocks
    let blocks = mirage_data.data.blocks;
    let max_branch_distance = blocks.iter()
        .map(|b| b.coord_z)
        .max()
        .unwrap_or(0);

    let max_dominator_depth = blocks.iter()
        .map(|b| b.coord_x)
        .max()
        .unwrap_or(0);

    let max_loop_nesting = blocks.iter()
        .map(|b| b.coord_y)
        .max()
        .unwrap_or(0);

    let total_blocks = blocks.len();

    // Check for cycles (simple heuristic: if branch distance > 20, likely has cycles)
    let has_cycles = max_branch_distance > 20;

    let analysis = ComplexityAnalysis {
        max_branch_distance,
        max_dominator_depth,
        max_loop_nesting,
        total_blocks,
        has_cycles,
        risk_level: RiskLevel::Low, // Will be set below
    };

    // Set risk level based on coordinates
    let risk_level = analysis.assess_risk_level();

    Ok(ComplexityAnalysis {
        risk_level,
        ..analysis
    })
}

impl ComplexityAnalysis {
    /// Create a safe default analysis when Mirage is unavailable
    fn safe_default() -> Self {
        Self {
            max_branch_distance: 0,
            max_dominator_depth: 0,
            max_loop_nesting: 0,
            total_blocks: 0,
            has_cycles: false,
            risk_level: RiskLevel::Medium, // Conservative default
        }
    }
}

/// Mirage JSON response structure
#[derive(Debug, Deserialize)]
struct MirageData {
    data: CfgData,
}

#[derive(Debug, Deserialize)]
struct CfgData {
    blocks: Vec<Block>,
}

#[derive(Debug, Deserialize)]
struct Block {
    #[serde(default)]
    coord_x: i64,
    #[serde(default)]
    coord_y: i64,
    #[serde(default)]
    coord_z: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_assessment_low() {
        let analysis = ComplexityAnalysis {
            max_branch_distance: 2,
            max_dominator_depth: 1,
            max_loop_nesting: 0,
            total_blocks: 3,
            has_cycles: false,
            risk_level: RiskLevel::Low,
        };

        assert_eq!(analysis.assess_risk_level(), RiskLevel::Low);
    }

    #[test]
    fn test_risk_assessment_high() {
        let analysis = ComplexityAnalysis {
            max_branch_distance: 12,
            max_dominator_depth: 7,
            max_loop_nesting: 2,
            total_blocks: 15,
            has_cycles: false,
            risk_level: RiskLevel::Low,
        };

        assert_eq!(analysis.assess_risk_level(), RiskLevel::High);
    }
}
