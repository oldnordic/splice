//! Rename command handler.

use std::fs;
use std::path::{Path, PathBuf};

use super::helpers::capture_snapshot;

#[allow(
    clippy::too_many_arguments,
    reason = "CLI handler aggregates clap-parsed flags"
)]
pub(crate) fn execute_rename(
    symbol_id: Option<&str>,
    name: Option<&str>,
    file: Option<&PathBuf>,
    new_name: &str,
    db_path: &Path,
    preview: bool,
    proof: bool,
    _backup_dir: Option<&PathBuf>,
    no_backup: bool,
    snapshot_before: bool,
    impact_graph: bool,
    _json_output: bool,
    kind_filter: Option<String>,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::MagellanIntegration;
    use splice::proof::generation::generate_snapshot;
    use splice::proof::{generate_proof, write_proof};

    // Capture snapshot before operation if requested
    if snapshot_before {
        if let Err(e) = capture_snapshot(db_path, "rename") {
            eprintln!("Warning: Failed to capture snapshot: {}", e);
        }
    }

    // Validate input: either --symbol or (--name AND --file) must be provided
    let (lookup_id, lookup_name, lookup_file) = match (symbol_id, name, file) {
        (Some(id), None, None) => (Some(id), None, None),
        (None, Some(n), Some(f)) => (None, Some(n), Some(f)),
        (None, None, _) => {
            return Err(splice::SpliceError::RenameFailed {
                reason: "Either --symbol or --name (with --file) must be provided".to_string(),
                symbol: new_name.to_string(),
            });
        }
        (Some(_), Some(_), _) => {
            return Err(splice::SpliceError::RenameFailed {
                reason: "--symbol and --name are mutually exclusive".to_string(),
                symbol: new_name.to_string(),
            });
        }
        (None, Some(_), None) => {
            return Err(splice::SpliceError::RenameFailed {
                reason: "--file is required when using --name".to_string(),
                symbol: name.expect("invariant: matched Some(_) above").to_string(),
            });
        }
        _ => {
            return Err(splice::SpliceError::RenameFailed {
                reason: if symbol_id.is_some() && file.is_some() {
                    "--file is not needed with --symbol (the symbol ID uniquely identifies the target)".to_string()
                } else if name.is_some() && file.is_none() {
                    "--file is required when using --name".to_string()
                } else {
                    "Provide either --symbol <id> or --name <name> --file <path>".to_string()
                },
                symbol: new_name.to_string(),
            });
        }
    };

    // Open Magellan database
    let mut magellan =
        MagellanIntegration::open(db_path).map_err(|e| splice::SpliceError::RenameFailed {
            reason: format!("Failed to open database: {}", e),
            symbol: new_name.to_string(),
        })?;

    // Resolve symbol (ID-first with name+path fallback)
    let symbol_info = if let Some(id) = lookup_id {
        // Lookup by symbol ID (32-char BLAKE3 or 16-char SHA-256)
        magellan
            .find_symbol_by_id(id)
            .map_err(|e| splice::SpliceError::RenameFailed {
                reason: format!("Failed to lookup symbol ID: {}", e),
                symbol: id.to_string(),
            })?
            .ok_or_else(|| splice::SpliceError::RenameFailed {
                reason: format!("Symbol ID '{}' not found in database", id),
                symbol: id.to_string(),
            })?
    } else {
        // Lookup by name+path
        let name_str = lookup_name.expect("invariant: lookup_id is None, so name is Some");
        let file_path = lookup_file.expect("invariant: lookup_id is None, so file is Some");
        let file_path = splice::resolve::normalize_lookup_path(file_path);
        let file_path_str = file_path.to_string_lossy().to_string();

        // First, find ALL matches (ambiguous=true) to provide complete error context
        let mut all_matches = magellan.find_symbol_by_name(name_str, true).map_err(|e| {
            splice::SpliceError::RenameFailed {
                reason: format!("Failed to lookup symbol name: {}", e),
                symbol: name_str.to_string(),
            }
        })?;

        // Apply kind filter if provided (e.g., "fn" vs "function" normalized to magellan's kind)
        if let Some(ref kind) = kind_filter {
            let normalized = splice::graph::magellan_integration::normalize_kind(kind);
            all_matches.retain(|s| {
                splice::graph::magellan_integration::normalize_kind(&s.kind) == normalized
            });
        }

        if all_matches.is_empty() {
            return Err(splice::SpliceError::RenameFailed {
                reason: format!(
                    "Symbol '{}' not found in file '{}'",
                    name_str,
                    file_path.display()
                ),
                symbol: name_str.to_string(),
            });
        }

        // Check if symbol name is ambiguous across multiple files
        if all_matches.len() > 1 {
            // Filter to just the specified file
            all_matches.retain(|s| s.file_path == file_path_str);

            if all_matches.is_empty() {
                // Symbol exists in other files but not in the specified one
                let all_files: Vec<String> = magellan
                    .find_symbol_by_name(name_str, true)
                    .map_err(|e| splice::SpliceError::RenameFailed {
                        reason: format!("Failed to lookup symbol name: {}", e),
                        symbol: name_str.to_string(),
                    })?
                    .into_iter()
                    .map(|s| s.file_path)
                    .collect();

                return Err(splice::SpliceError::RenameFailed {
                    reason: format!(
                        "Symbol '{}' not found in file '{}' (found in {} other file(s): {})",
                        name_str,
                        file_path.display(),
                        all_files.len(),
                        all_files.join(", ")
                    ),
                    symbol: name_str.to_string(),
                });
            }

            if all_matches.len() > 1 {
                // Still multiple matches in the same file - return AmbiguousSymbol error
                let files: Vec<String> = all_matches
                    .iter()
                    .map(|s| format!("{}:{}", s.file_path, s.kind))
                    .collect();
                return Err(splice::SpliceError::AmbiguousSymbol {
                    name: name_str.to_string(),
                    files,
                });
            }
        }

        all_matches.remove(0)
    };

    // Generate before snapshot if proof generation is requested
    let before_snapshot = if proof {
        Some(
            generate_snapshot(db_path).map_err(|e| splice::SpliceError::RenameFailed {
                reason: format!("Failed to generate before snapshot: {}", e),
                symbol: new_name.to_string(),
            })?,
        )
    } else {
        None
    };

    // Pre-flight validation: symbol must exist and have references
    let entity_id = symbol_info.entity_id;
    let mut references =
        magellan
            .get_all_references(entity_id)
            .map_err(|e| splice::SpliceError::RenameFailed {
                reason: format!("Failed to get references: {}", e),
                symbol: symbol_info.name.clone(),
            })?;

    // Include the definition *name* site in the rename.
    // Magellan's byte_start/byte_end covers the full declaration body.
    // We locate the first occurrence of the symbol name within that span
    // to produce a name-only offset that won't corrupt the declaration.
    let decl_content =
        fs::read(&symbol_info.file_path).map_err(|e| splice::SpliceError::RenameFailed {
            reason: format!(
                "Failed to read definition file '{}': {}",
                symbol_info.file_path, e
            ),
            symbol: symbol_info.name.clone(),
        })?;
    let decl_slice = &decl_content[symbol_info.byte_start..symbol_info.byte_end];
    if let Some(name_offset) = decl_slice
        .windows(symbol_info.name.len())
        .position(|w| w == symbol_info.name.as_bytes())
    {
        let name_byte_start = symbol_info.byte_start + name_offset;
        let name_byte_end = name_byte_start + symbol_info.name.len();
        references.push(magellan::references::ReferenceFact {
            file_path: PathBuf::from(&symbol_info.file_path),
            referenced_symbol: symbol_info.name.clone(),
            byte_start: name_byte_start,
            byte_end: name_byte_end,
            start_line: symbol_info.start_line.unwrap_or(1),
            start_col: 0,
            end_line: symbol_info.end_line.unwrap_or(1),
            end_col: 0,
        });
    } else {
        return Err(splice::SpliceError::RenameFailed {
            reason: format!(
                "Symbol name '{}' not found inside its own declaration span ({}..{}) in {}",
                symbol_info.name,
                symbol_info.byte_start,
                symbol_info.byte_end,
                symbol_info.file_path
            ),
            symbol: symbol_info.name,
        });
    }
    // Sort references for safe in-order replacement
    // Descending by byte_start within each file prevents offset shifts

    // Deduplicate references: magellan may already include the definition site,
    // and we also add it below. Overlapping duplicates produce double renames.
    {
        let mut seen = std::collections::HashSet::new();
        references.retain(|r| {
            let key = (r.file_path.clone(), r.byte_start, r.byte_end);
            seen.insert(key)
        });
    }

    splice::graph::MagellanIntegration::sort_references_for_replacement(&mut references);

    // Group references by file
    use splice::graph::rename;
    let grouped = rename::group_references_by_file(&references);

    // Calculate stats for preview
    let files_affected: Vec<&PathBuf> = grouped.keys().collect();
    let total_references = references.len();

    // PREVIEW MODE: Generate unified diffs without filesystem writes
    // Preview is pure - no backup creation, no file modifications
    if preview {
        // Generate impact graph if requested
        if impact_graph {
            use splice::cli::ReachabilityDirection;
            use splice::graph::magellan_integration::ImpactDotConfig;
            let symbol_id = format!("{}:{}", symbol_info.file_path, symbol_info.name);
            let config = ImpactDotConfig {
                show_symbol_kinds: true,
                max_depth: Some(10),
                highlight_symbol: Some(symbol_info.name.clone()),
            };
            let dot =
                magellan.generate_impact_dot(&symbol_id, &ReachabilityDirection::Both, &config)?;
            println!("{}", dot);
        }

        let mut diffs = Vec::new();
        for (file_path, refs) in &grouped {
            let content = fs::read_to_string(file_path).map_err(|e| splice::SpliceError::Io {
                path: file_path.clone(),
                source: e,
            })?;
            let modified =
                rename::simulate_replacements_content(&content, refs, &symbol_info.name, new_name)?;
            let diff = rename::generate_colored_preview(file_path, &content, &modified);
            diffs.push(diff);
        }

        let summary = format!(
            "Preview: {} files, {} references\n\n{}",
            files_affected.len(),
            total_references,
            diffs.join("\n")
        );
        return Ok(splice::cli::CliSuccessPayload::message_only(summary).with_pending_changes());
    }

    // DETERMINE WORKSPACE ROOT
    let workspace_root = std::env::current_dir()
        .map_err(|e| splice::SpliceError::Other(format!("Failed to get workspace root: {}", e)))?;

    // CREATE BACKUP (unless skipped)
    let backup_dir_path = if !no_backup {
        let files_to_backup: Vec<PathBuf> = files_affected.iter().map(|p| (**p).clone()).collect();
        Some(rename::create_rename_backup(
            &workspace_root,
            symbol_id.unwrap_or("unknown"),
            &files_to_backup,
        )?)
    } else {
        None
    };

    // APPLY REPLACEMENTS WITH TRANSACTION
    let mut transaction = rename::RenameTransaction::new();
    if let Some(backup_path) = backup_dir_path.as_ref() {
        transaction = transaction.with_backup(backup_path.clone(), workspace_root.clone());
    }

    let mut modified_count = 0;
    let mut last_error: Option<splice::SpliceError> = None;

    for (file_path, refs) in grouped {
        match rename::apply_replacements_in_file(&file_path, &symbol_info.name, new_name, &refs) {
            Ok(count) => {
                if count > 0 {
                    modified_count += 1;
                    transaction.track_modified(file_path.to_path_buf());
                }
            }
            Err(e) => {
                // Rollback on error
                last_error = Some(e);
                break;
            }
        }
    }

    // If any file failed, rollback the entire transaction
    if let Some(error) = last_error {
        let _ = transaction.rollback(); // Best-effort rollback
        return Err(error);
    }

    let message = if let Some(ref backup_path) = backup_dir_path {
        format!(
            "Renamed '{}' to '{}' in {} files\nBackup: {}",
            symbol_info.name,
            new_name,
            modified_count,
            backup_path.display()
        )
    } else {
        format!(
            "Renamed '{}' to '{}' in {} files (no backup)",
            symbol_info.name, new_name, modified_count
        )
    };

    // Generate after snapshot and write proof if requested
    if proof {
        if let Some(before) = before_snapshot {
            let after_snapshot =
                generate_snapshot(db_path).map_err(|e| splice::SpliceError::RenameFailed {
                    reason: format!("Failed to generate after snapshot: {}", e),
                    symbol: new_name.to_string(),
                })?;

            // Use generate_proof to create complete proof with invariant validation
            let proof_data =
                generate_proof("rename", db_path, before, after_snapshot).map_err(|e| {
                    splice::SpliceError::RenameFailed {
                        reason: format!("Failed to generate proof: {}", e),
                        symbol: new_name.to_string(),
                    }
                })?;

            // Check invariant validation results
            let failed_invariants: Vec<_> =
                proof_data.invariants.iter().filter(|c| !c.passed).collect();

            let invariant_status = if failed_invariants.is_empty() {
                "All invariants passed".to_string()
            } else {
                let failed_names: Vec<&str> = failed_invariants
                    .iter()
                    .map(|c| c.invariant_name.as_str())
                    .collect();
                format!(
                    "Warning: {} invariant(s) failed: {}",
                    failed_invariants.len(),
                    failed_names.join(", ")
                )
            };

            let proof_dir = std::path::PathBuf::from(".splice/proofs");
            let proof_path = write_proof(&proof_data, &proof_dir).map_err(|e| {
                splice::SpliceError::RenameFailed {
                    reason: format!("Failed to write proof: {}", e),
                    symbol: new_name.to_string(),
                }
            })?;

            return Ok(splice::cli::CliSuccessPayload::with_data(
                format!(
                    "{}\nProof written to: {}\nInvariant status: {}",
                    message,
                    proof_path.display(),
                    invariant_status
                ),
                serde_json::json!({
                    "old_name": symbol_info.name,
                    "new_name": new_name,
                    "files_modified": modified_count,
                    "total_references": total_references,
                    "backup": backup_dir_path.as_ref().map(|p| p.display().to_string()),
                    "proof": proof_path.display().to_string(),
                    "invariants": {
                        "total": proof_data.invariants.len(),
                        "passed": proof_data.invariants.iter().filter(|c| c.passed).count(),
                        "failed": failed_invariants.len(),
                        "details": proof_data.invariants.iter().map(|c| serde_json::json!({
                            "name": c.invariant_name,
                            "passed": c.passed,
                            "violations": c.violations.len()
                        })).collect::<Vec<_>>()
                    }
                }),
            ));
        }
    }

    Ok(splice::cli::CliSuccessPayload::with_data(
        message,
        serde_json::json!({
            "old_name": symbol_info.name,
            "new_name": new_name,
            "files_modified": modified_count,
            "total_references": total_references,
            "backup": backup_dir_path.as_ref().map(|p| p.display().to_string()),
        }),
    ))
}
