//! Reference tracking methods for MagellanIntegration.

use crate::error::{Result, SpliceError};
use std::path::Path;

use super::normalize_lookup_path;
use super::types::*;
use super::MagellanIntegration;

impl MagellanIntegration {
    /// Get call relationships for a symbol.
    ///
    /// # Arguments
    /// * `file_path` - Path to file containing the symbol
    /// * `name` - Symbol name
    /// * `direction` - Which relationships to fetch (In/Out/Both)
    ///
    /// # Returns
    /// CallRelationships containing the symbol and its relationships.
    pub fn get_call_relationships(
        &mut self,
        file_path: &Path,
        name: &str,
        direction: CallDirection,
    ) -> Result<CallRelationships> {
        let normalized = normalize_lookup_path(file_path);
        let path_str = normalized.to_str().ok_or_else(|| {
            SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", normalized))
        })?;

        // Get the target symbol info first
        let symbol_facts = self.inner.symbol_extents(path_str, name).map_err(|e| {
            SpliceError::Other(format!(
                "Failed to find symbol {} in {}: {}",
                name, path_str, e
            ))
        })?;

        if symbol_facts.is_empty() {
            return Err(SpliceError::Other(format!(
                "Symbol '{}' not found in file '{}'",
                name, path_str
            )));
        }

        let (entity_id, fact) = &symbol_facts[0];
        let target_symbol = SymbolInfo {
            entity_id: *entity_id,
            name: fact.name.clone().unwrap_or_else(|| name.to_string()),
            file_path: fact.file_path.to_string_lossy().to_string(),
            kind: fact.kind_normalized.clone(),
            byte_start: fact.byte_start,
            byte_end: fact.byte_end,
            start_line: None,
            end_line: None,
        };

        let (callers, callees) = match direction {
            CallDirection::In => {
                let calls = self
                    .inner
                    .callers_of_symbol(path_str, name)
                    .map_err(|e| SpliceError::Other(format!("Failed to get callers: {}", e)))?;
                (self.resolve_call_facts_to_references(calls)?, Vec::new())
            }
            CallDirection::Out => {
                let calls = self
                    .inner
                    .calls_from_symbol(path_str, name)
                    .map_err(|e| SpliceError::Other(format!("Failed to get callees: {}", e)))?;
                (Vec::new(), self.resolve_call_facts_to_references(calls)?)
            }
            CallDirection::Both => {
                let callers_facts = self
                    .inner
                    .callers_of_symbol(path_str, name)
                    .map_err(|e| SpliceError::Other(format!("Failed to get callers: {}", e)))?;
                let callees_facts = self
                    .inner
                    .calls_from_symbol(path_str, name)
                    .map_err(|e| SpliceError::Other(format!("Failed to get callees: {}", e)))?;
                (
                    self.resolve_call_facts_to_references(callers_facts)?,
                    self.resolve_call_facts_to_references(callees_facts)?,
                )
            }
        };

        Ok(CallRelationships {
            symbol: target_symbol,
            callers,
            callees,
        })
    }

    /// Resolve CallFact vectors to CallReference vectors with symbol info.
    fn resolve_call_facts_to_references(
        &mut self,
        call_facts: Vec<magellan::references::CallFact>,
    ) -> Result<Vec<CallReference>> {
        let mut references = Vec::new();
        let mut seen: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

        for fact in call_facts {
            // Resolve the referenced symbol (caller or callee depending on context)
            let ref_name = &fact.callee;
            let ref_path_str = fact.file_path.to_string_lossy();

            // Get symbol info for the referenced symbol
            let symbol_infos = self
                .inner
                .symbol_extents(&ref_path_str, ref_name)
                .map_err(|e| {
                    SpliceError::Other(format!("Failed to resolve symbol {}: {}", ref_name, e))
                })?;

            for (entity_id, symbol_fact) in symbol_infos {
                let symbol = SymbolInfo {
                    entity_id,
                    name: symbol_fact.name.clone().unwrap_or_else(|| ref_name.clone()),
                    file_path: symbol_fact.file_path.to_string_lossy().to_string(),
                    kind: symbol_fact.kind_normalized.clone(),
                    byte_start: symbol_fact.byte_start,
                    byte_end: symbol_fact.byte_end,
                    start_line: None,
                    end_line: None,
                };

                let key = (symbol.name.clone(), symbol.file_path.clone());
                if !seen.insert(key) {
                    continue;
                }

                let call_site = CallSite {
                    file_path: fact.file_path.to_string_lossy().to_string(),
                    byte_start: fact.byte_start,
                    byte_end: fact.byte_end,
                    start_line: fact.start_line,
                    start_col: fact.start_col,
                    end_line: fact.end_line,
                    end_col: fact.end_col,
                };

                references.push(CallReference { symbol, call_site });
            }
        }

        Ok(references)
    }

    /// Sort references for safe in-order replacement.
    ///
    /// References are sorted by (file_path, byte_start) with descending
    /// byte_start within each file. This ensures that replacing earlier
    /// references doesn't affect byte offsets of later ones.
    ///
    /// This is critical when the new name has different byte length.
    ///
    /// # Arguments
    /// * `references` - References to sort in-place
    pub fn sort_references_for_replacement(references: &mut [magellan::references::ReferenceFact]) {
        references.sort_by(|a, b| {
            // First by file path (ascending) for logical grouping
            match a.file_path.cmp(&b.file_path) {
                std::cmp::Ordering::Equal => {
                    // Then by byte_start within file (descending)
                    // Descending order prevents offset shifts from affecting later replacements
                    b.byte_start.cmp(&a.byte_start)
                }
                other => other,
            }
        });
    }

    /// Validate that a byte span is on UTF-8 character boundaries.
    ///
    /// # Arguments
    /// * `content` - File content as bytes
    /// * `byte_start` - Start offset
    /// * `byte_end` - End offset
    /// * `file_path` - Path to file (for error reporting)
    ///
    /// # Returns
    /// Ok(()) if span is valid, Err with description if invalid
    pub fn validate_utf8_span(
        content: &[u8],
        byte_start: usize,
        byte_end: usize,
        file_path: &Path,
    ) -> Result<()> {
        if byte_start >= content.len() || byte_end > content.len() {
            return Err(SpliceError::InvalidSpan {
                file: file_path.to_path_buf(),
                start: byte_start,
                end: byte_end,
                file_size: content.len(),
            });
        }

        // Convert to str for char_boundary checking
        // SAFETY: We only validate boundaries, not content validity
        let content_str = std::str::from_utf8(content)
            .map_err(|_| SpliceError::Other("File content is not valid UTF-8".to_string()))?;

        // Check start is on character boundary
        if !content_str.is_char_boundary(byte_start) {
            return Err(SpliceError::Other(format!(
                "Byte start {} is not on UTF-8 character boundary",
                byte_start
            )));
        }

        // Check end is on character boundary
        if !content_str.is_char_boundary(byte_end) {
            return Err(SpliceError::Other(format!(
                "Byte end {} is not on UTF-8 character boundary",
                byte_end
            )));
        }

        Ok(())
    }

    /// Get all references for a symbol by entity ID.
    ///
    /// Returns ReferenceFact entries for all references to this symbol
    /// across all indexed files. This is the core data for cross-file rename.
    ///
    /// # Arguments
    /// * `entity_id` - Entity ID of the symbol definition
    ///
    /// # Returns
    /// Vector of ReferenceFact entries with byte spans
    pub fn get_all_references(
        &mut self,
        entity_id: i64,
    ) -> Result<Vec<magellan::references::ReferenceFact>> {
        self.inner.references_to_symbol(entity_id).map_err(|e| {
            SpliceError::Other(format!(
                "Failed to get references for entity {}: {}",
                entity_id, e
            ))
        })
    }

    /// Index references for a file into the graph.
    ///
    /// This extracts all references to known symbols in the file and creates
    /// Reference nodes with REFERENCES edges to the corresponding symbols.
    /// This must be called after `index_file` for proper cross-file reference tracking.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to index references for
    ///
    /// # Returns
    /// Number of references indexed
    ///
    /// # Errors
    /// Returns Other error if indexing fails
    pub fn index_references(&mut self, file_path: &Path) -> Result<usize> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| SpliceError::Other(format!("Invalid UTF-8 in path: {:?}", file_path)))?;

        let source = std::fs::read(file_path).map_err(|e| {
            SpliceError::Other(format!("Failed to read file {:?}: {}", file_path, e))
        })?;

        self.inner
            .index_references(file_path_str, &source)
            .map_err(|e| {
                SpliceError::Other(format!(
                    "Failed to index references for {:?}: {}",
                    file_path, e
                ))
            })
    }
}
