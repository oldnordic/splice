//! Batch operation specification schema.
//!
//! Defines the YAML format for multi-file refactoring operations.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level batch specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSpec {
    /// Batch metadata
    #[serde(default)]
    pub metadata: BatchMetadata,
    /// List of operations to execute
    pub operations: Vec<BatchOperation>,
    /// Execution mode (stop on error vs continue)
    #[serde(default)]
    pub mode: ExecutionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetadata {
    /// Human-readable description
    pub description: Option<String>,
    /// Author/creator
    pub author: Option<String>,
    /// Version of spec format
    #[serde(default = "default_spec_version")]
    pub version: String,
}

impl Default for BatchMetadata {
    fn default() -> Self {
        Self {
            description: None,
            author: None,
            version: default_spec_version(),
        }
    }
}

fn default_spec_version() -> String {
    "1.0".to_string()
}

/// Execution mode for batch operations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Stop at first error (default)
    StopOnError,
    /// Continue on error, report all failures
    ContinueOnError,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::StopOnError
    }
}

/// A single batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BatchOperation {
    /// Patch operation - replace symbol body
    Patch(PatchOp),
    /// Delete operation - remove symbol definition
    Delete(DeleteOp),
    /// Rename operation - rename symbol across all references
    Rename(RenameOp),
}

/// Patch operation specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchOp {
    /// Source file path
    pub file: PathBuf,
    /// Symbol name to patch
    pub symbol: String,
    /// Optional symbol kind filter
    pub kind: Option<String>,
    /// Path to file containing replacement content
    pub with: PathBuf,
    /// Optional: snapshot before this operation
    #[serde(default)]
    pub snapshot_before: bool,
}

/// Delete operation specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteOp {
    /// Source file path
    pub file: PathBuf,
    /// Symbol name to delete
    pub symbol: String,
    /// Optional symbol kind filter
    pub kind: Option<String>,
    /// Optional: snapshot before this operation
    #[serde(default)]
    pub snapshot_before: bool,
}

/// Rename operation specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameOp {
    /// Source file path where symbol is defined
    pub file: PathBuf,
    /// Current symbol name
    pub from: String,
    /// New symbol name
    pub to: String,
    /// Optional: limit to specific files only
    pub files: Option<Vec<PathBuf>>,
    /// Optional: snapshot before this operation
    #[serde(default)]
    pub snapshot_before: bool,
}

/// Parse a batch spec from YAML file.
pub fn parse_batch_spec(path: &PathBuf) -> Result<BatchSpec, BatchSpecError> {
    let content = std::fs::read_to_string(path).map_err(|e| BatchSpecError::Io {
        path: path.clone(),
        source: e,
    })?;

    let spec: BatchSpec =
        serde_yaml::from_str(&content).map_err(|e| BatchSpecError::ParseError {
            path: path.clone(),
            reason: e.to_string(),
        })?;

    // Validate spec
    validate_spec(&spec)?;

    Ok(spec)
}

fn validate_spec(spec: &BatchSpec) -> Result<(), BatchSpecError> {
    if spec.operations.is_empty() {
        return Err(BatchSpecError::EmptyOperations);
    }

    for (idx, op) in spec.operations.iter().enumerate() {
        validate_operation(op, idx)?;
    }

    Ok(())
}

fn validate_operation(op: &BatchOperation, idx: usize) -> Result<(), BatchSpecError> {
    match op {
        BatchOperation::Patch(p) => {
            if p.symbol.is_empty() {
                return Err(BatchSpecError::InvalidOperation {
                    index: idx,
                    reason: "patch operation requires non-empty symbol name".to_string(),
                });
            }
            // Check file paths exist if specified
            if !p.file.exists() {
                return Err(BatchSpecError::FileNotFound {
                    index: idx,
                    path: p.file.clone(),
                });
            }
        }
        BatchOperation::Delete(d) => {
            if d.symbol.is_empty() {
                return Err(BatchSpecError::InvalidOperation {
                    index: idx,
                    reason: "delete operation requires non-empty symbol name".to_string(),
                });
            }
        }
        BatchOperation::Rename(r) => {
            if r.from.is_empty() || r.to.is_empty() {
                return Err(BatchSpecError::InvalidOperation {
                    index: idx,
                    reason: "rename operation requires both 'from' and 'to' names".to_string(),
                });
            }
        }
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum BatchSpecError {
    #[error("IO error reading {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse {path}: {reason}")]
    ParseError { path: PathBuf, reason: String },

    #[error("Batch spec contains no operations")]
    EmptyOperations,

    #[error("Operation {index} is invalid: {reason}")]
    InvalidOperation { index: usize, reason: String },

    #[error("Operation {index} references non-existent file: {path}")]
    FileNotFound { index: usize, path: PathBuf },
}
