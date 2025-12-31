use super::{SpanBatch, SpanReplacement};
use crate::error::{Result, SpliceError};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct BatchSpec {
    batches: Vec<BatchEntry>,
}

#[derive(Debug, Deserialize)]
struct BatchEntry {
    replacements: Vec<ReplacementSpec>,
}

#[derive(Debug, Deserialize)]
struct ReplacementSpec {
    file: String,
    start: usize,
    end: usize,
    #[serde(default)]
    content: Option<String>,
    #[serde(rename = "with", default)]
    with_file: Option<String>,
}

/// Load span batches from a JSON manifest.
pub fn load_batches_from_file(batch_path: &Path) -> Result<Vec<SpanBatch>> {
    let contents = fs::read_to_string(batch_path)?;
    let spec: BatchSpec =
        serde_json::from_str(&contents).map_err(|err| SpliceError::InvalidBatchSchema {
            message: format!("JSON parse error: {}", err),
        })?;

    if spec.batches.is_empty() {
        return Err(SpliceError::InvalidBatchSchema {
            message: "Batch file must contain at least one entry".to_string(),
        });
    }

    let base_dir = batch_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut batches = Vec::with_capacity(spec.batches.len());
    for (index, batch) in spec.batches.into_iter().enumerate() {
        if batch.replacements.is_empty() {
            return Err(SpliceError::InvalidBatchSchema {
                message: format!("Batch {} contains no replacements", index + 1),
            });
        }

        let mut replacements = Vec::with_capacity(batch.replacements.len());
        for (replacement_idx, replacement) in batch.replacements.into_iter().enumerate() {
            let content = resolve_content(&base_dir, &replacement).map_err(|message| {
                SpliceError::InvalidBatchSchema {
                    message: format!(
                        "Batch {} replacement {}: {}",
                        index + 1,
                        replacement_idx + 1,
                        message
                    ),
                }
            })?;

            let file_path = resolve_path(&base_dir, &replacement.file);

            replacements.push(SpanReplacement::new(
                file_path,
                replacement.start,
                replacement.end,
                content,
            ));
        }

        batches.push(SpanBatch::new(replacements));
    }

    Ok(batches)
}

fn resolve_path(base_dir: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn resolve_content(base_dir: &Path, spec: &ReplacementSpec) -> std::result::Result<String, String> {
    match (&spec.content, &spec.with_file) {
        (Some(inline), None) => Ok(inline.to_string()),
        (None, Some(with_file)) => {
            let path = resolve_path(base_dir, with_file);
            fs::read_to_string(&path)
                .map_err(|err| format!("Failed to read '{}': {}", path.display(), err))
        }
        (Some(_), Some(_)) => Err("Specify only one of 'content' or 'with'".to_string()),
        (None, None) => Err("Replacement requires either 'content' or 'with' field".to_string()),
    }
}
