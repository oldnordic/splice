//! Verify and validate-proof command handlers.

use std::path::Path;

pub(crate) fn execute_validate_proof(
    proof_path: &Path,
    output: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use serde_json::json;
    use splice::proof::validate_proof_file;

    let is_valid = validate_proof_file(proof_path)?;

    if output.is_json() || json_output {
        let result = json!({
            "proof_path": proof_path.to_string_lossy(),
            "checksums_valid": is_valid,
            "status": if is_valid { "valid" } else { "no_checksums" }
        });

        let json_output = output
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);

        Ok(splice::cli::CliSuccessPayload::message_only(if is_valid {
            "Proof checksums are valid".to_string()
        } else {
            "Proof has no checksums".to_string()
        })
        .already_emitted())
    } else {
        println!("Proof Validation: {}", proof_path.display());
        println!();

        if is_valid {
            println!("Status: VALID ✓");
            println!();
            println!("All SHA-256 checksums verified:");
            println!("  ✓ Before snapshot hash");
            println!("  ✓ After snapshot hash");
            println!("  ✓ Overall proof hash");
            println!();
            println!("Audit trail integrity is confirmed.");
        } else {
            println!("Status: NO CHECKSUMS ⚠");
            println!();
            println!("This proof does not include checksums.");
            println!("Checksums were added in Splice 2.2.4.");
            println!("Proof may have been generated with an older version.");
        }

        Ok(splice::cli::CliSuccessPayload::message_only(if is_valid {
            "Proof validation complete".to_string()
        } else {
            "Proof validation complete (no checksums)".to_string()
        }))
    }
}

pub(crate) fn execute_verify(
    before_path: &Path,
    after_path: &Path,
    detailed: bool,
    output_format: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use serde_json::json;
    use splice::proof::{compare_snapshots, storage::SnapshotStorage};

    let storage = SnapshotStorage::new()?;
    let before = storage.load_snapshot(before_path)?;
    let after = storage.load_snapshot(after_path)?;

    let diff = compare_snapshots(&before, &after)?;

    let has_symbol_changes =
        diff.symbols_added > 0 || diff.symbols_removed > 0 || diff.symbols_modified > 0;
    let has_edge_changes = diff.edges_added > 0 || diff.edges_removed > 0;
    let has_invariant_failures = diff.invariant_results.iter().any(|c| !c.passed);
    let has_differences = has_symbol_changes || has_edge_changes || has_invariant_failures;

    if output_format.is_json() || json_output {
        let result = json!(diff);
        let json_output = output_format
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);

        let mut payload = splice::cli::CliSuccessPayload::with_data(
            format!(
                "Snapshot comparison: {} differences",
                if has_differences { "found" } else { "none" }
            ),
            result,
        )
        .already_emitted();

        if has_differences {
            payload = payload.with_pending_changes();
        }

        Ok(payload)
    } else {
        println!("Snapshot Comparison");
        println!();
        println!("Before: {}", before_path.display());
        println!("After: {}", after_path.display());
        println!();

        println!("Summary:");
        if !has_differences {
            println!("  No differences detected - snapshots are identical");
        } else {
            if diff.symbols_added > 0 {
                println!("  Symbols added: {}", diff.symbols_added);
            }
            if diff.symbols_removed > 0 {
                println!("  Symbols removed: {}", diff.symbols_removed);
            }
            if diff.symbols_modified > 0 {
                println!("  Symbols modified: {}", diff.symbols_modified);
            }
            if diff.edges_added > 0 {
                println!("  Edges added: {}", diff.edges_added);
            }
            if diff.edges_removed > 0 {
                println!("  Edges removed: {}", diff.edges_removed);
            }
            if has_invariant_failures {
                let failed = diff.invariant_results.iter().filter(|c| !c.passed).count();
                println!("  Invariant failures: {}", failed);
            }
        }
        println!();

        if detailed && !diff.symbol_details.is_empty() {
            println!("Symbol Details:");
            for sym_diff in &diff.symbol_details {
                let change_marker = match sym_diff.change_type {
                    splice::proof::ChangeType::Added => "+",
                    splice::proof::ChangeType::Removed => "-",
                    splice::proof::ChangeType::Modified => "~",
                    splice::proof::ChangeType::Unchanged => " ",
                };
                println!("  {} {} ({})", change_marker, sym_diff.name, sym_diff.id);

                if sym_diff.change_type == splice::proof::ChangeType::Modified {
                    if let Some(before) = &sym_diff.before {
                        println!("    Before: {} @ {}", before.name, before.file_path);
                    }
                    if let Some(after) = &sym_diff.after {
                        println!("    After:  {} @ {}", after.name, after.file_path);
                    }
                }
            }
            println!();
        }

        if has_invariant_failures {
            println!("Invariant Validation:");
            for check in &diff.invariant_results {
                if !check.passed {
                    println!("  FAILED: {}", check.invariant_name);
                    for violation in &check.violations {
                        println!("    - {}: {}", violation.subject, violation.message);
                    }
                }
            }
            println!();
        }

        let mut payload = splice::cli::CliSuccessPayload::message_only(if has_differences {
            "Snapshots differ".to_string()
        } else {
            "Snapshots are identical".to_string()
        });

        if has_differences {
            payload = payload.with_pending_changes();
        }

        Ok(payload)
    }
}
