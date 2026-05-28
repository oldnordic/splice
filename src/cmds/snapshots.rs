//! Snapshot management command handlers.

use super::helpers::confirm_action;

pub(crate) fn execute_snapshots(
    cmd: splice::cli::SnapshotsCommands,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    match cmd {
        splice::cli::SnapshotsCommands::List {
            operation,
            limit,
            disk_usage,
            output,
        } => execute_snapshots_list(operation, limit, disk_usage, output, json_output),
        splice::cli::SnapshotsCommands::Delete { id, force } => {
            execute_snapshots_delete(&id, force, json_output)
        }
        splice::cli::SnapshotsCommands::Cleanup { keep, dry_run, yes } => {
            execute_snapshots_cleanup(keep, dry_run, yes, json_output)
        }
    }
}

pub(crate) fn execute_snapshots_list(
    operation_filter: Option<String>,
    limit: Option<usize>,
    disk_usage: bool,
    output_format: splice::cli::OutputFormat,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use serde_json::json;
    use splice::proof::storage::SnapshotStorage;

    let storage = SnapshotStorage::new()?;
    let snapshots = storage.list_snapshots_filtered(operation_filter.as_deref(), limit)?;

    let total_size = if disk_usage {
        Some(storage.get_total_size()?)
    } else {
        None
    };

    if output_format.is_json() || json_output {
        let snapshot_data: Vec<serde_json::Value> = snapshots
            .iter()
            .map(|meta| {
                json!({
                    "operation": meta.operation,
                    "timestamp": meta.timestamp,
                    "snapshot_path": meta.snapshot_path,
                    "symbols_count": meta.symbols_count,
                    "edges_count": meta.edges_count,
                })
            })
            .collect();

        let mut result = json!({
            "snapshots": snapshot_data,
            "count": snapshots.len(),
        });

        if let Some(size) = total_size {
            result["total_size_bytes"] = json!(size);
        }

        let json_output = output_format
            .format_json(&result)
            .map_err(|e| splice::SpliceError::Other(format!("JSON serialization error: {}", e)))?;
        println!("{}", json_output);

        Ok(splice::cli::CliSuccessPayload::with_data(
            format!("Listed {} snapshots", snapshots.len()),
            result,
        )
        .already_emitted())
    } else {
        if snapshots.is_empty() {
            println!("No snapshots found.");
            return Ok(splice::cli::CliSuccessPayload::message_only(
                "No snapshots found".to_string(),
            ));
        }

        println!("Snapshots ({} total)", snapshots.len());
        println!();

        for meta in &snapshots {
            let timestamp_str = chrono::DateTime::from_timestamp(meta.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            println!(
                "  {} | {} | {} symbols, {} edges",
                timestamp_str, meta.operation, meta.symbols_count, meta.edges_count,
            );
            println!("    Path: {}", meta.snapshot_path.display());
        }

        if let Some(size) = total_size {
            println!();
            println!("Total disk usage: {} bytes ({} MB)", size, size / 1_048_576,);
        }

        Ok(splice::cli::CliSuccessPayload::message_only(format!(
            "Listed {} snapshots",
            snapshots.len()
        )))
    }
}

pub(crate) fn execute_snapshots_delete(
    snapshot_id: &str,
    force: bool,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::proof::storage::SnapshotStorage;

    let storage = SnapshotStorage::new()?;

    let snapshot_info = storage.get_by_id(snapshot_id)?;

    if snapshot_info.is_none() {
        return Err(splice::SpliceError::Other(format!(
            "Snapshot '{}' not found",
            snapshot_id
        )));
    }

    let (path, meta) = snapshot_info.expect("invariant: None case returned Err above");

    if !force
        && !confirm_action(&format!(
            "Delete snapshot '{}' from {}? (y/N): ",
            meta.operation,
            chrono::DateTime::from_timestamp(meta.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        ))?
    {
        println!("Deletion cancelled.");
        return Ok(splice::cli::CliSuccessPayload::message_only(
            "Deletion cancelled".to_string(),
        ));
    }

    let deleted = storage.delete_by_id(snapshot_id)?;

    if !deleted {
        return Err(splice::SpliceError::Other(format!(
            "Failed to delete snapshot '{}'",
            snapshot_id
        )));
    }

    if json_output {
        let result = serde_json::json!({
            "deleted": true,
            "snapshot_id": snapshot_id,
            "snapshot_path": path,
        });
        println!("{}", result);
        Ok(splice::cli::CliSuccessPayload::with_data(
            format!("Deleted snapshot '{}'", snapshot_id),
            result,
        )
        .already_emitted())
    } else {
        println!("Deleted snapshot: {}", path.display());
        Ok(splice::cli::CliSuccessPayload::message_only(format!(
            "Deleted snapshot '{}'",
            snapshot_id
        )))
    }
}

pub(crate) fn execute_snapshots_cleanup(
    keep: usize,
    dry_run: bool,
    yes: bool,
    json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::proof::storage::SnapshotStorage;

    const BULK_DELETE_THRESHOLD: usize = 50;

    let storage = SnapshotStorage::new()?;
    let snapshots = storage.list_snapshots()?;

    if snapshots.len() <= keep {
        let message = format!(
            "Only {} snapshots exist (keeping {}), nothing to clean up",
            snapshots.len(),
            keep
        );

        if json_output {
            let result = serde_json::json!({
                "deleted_count": 0,
                "kept_count": snapshots.len(),
                "keep": keep,
                "dry_run": dry_run,
            });
            println!("{}", result);
            return Ok(splice::cli::CliSuccessPayload::with_data(message, result).already_emitted());
        } else {
            println!("{}", message);
            return Ok(splice::cli::CliSuccessPayload::message_only(message));
        }
    }

    let to_delete_count = snapshots.len() - keep;
    let to_delete = &snapshots[keep..];

    if dry_run {
        let message = format!(
            "Would delete {} snapshots (keeping {} most recent)",
            to_delete_count, keep
        );

        if json_output {
            let snapshot_paths: Vec<String> = to_delete
                .iter()
                .map(|s| s.snapshot_path.to_string_lossy().to_string())
                .collect();

            let result = serde_json::json!({
                "dry_run": true,
                "would_delete_count": to_delete_count,
                "kept_count": keep,
                "to_delete": snapshot_paths,
            });
            println!("{}", result);
            return Ok(splice::cli::CliSuccessPayload::with_data(message, result).already_emitted());
        } else {
            println!("{}", message);
            println!();
            println!("Snapshots to delete:");
            for meta in to_delete {
                println!("  - {} ({})", meta.snapshot_path.display(), meta.operation);
            }
            return Ok(splice::cli::CliSuccessPayload::message_only(message));
        }
    }

    if !yes && to_delete_count > BULK_DELETE_THRESHOLD {
        return Err(splice::SpliceError::Other(format!(
            "Refusing to delete {} snapshots (threshold {}). Re-run with --yes to confirm \
             the bulk delete, or use --dry-run to preview what would be removed.",
            to_delete_count, BULK_DELETE_THRESHOLD
        )));
    }
    let deleted_paths = storage.cleanup_old_snapshots(keep)?;

    if json_output {
        let deleted_paths_str: Vec<String> = deleted_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let result = serde_json::json!({
            "deleted_count": deleted_paths.len(),
            "kept_count": keep,
            "deleted_paths": deleted_paths_str,
        });
        println!("{}", result);
        Ok(splice::cli::CliSuccessPayload::with_data(
            format!("Deleted {} snapshots", deleted_paths.len()),
            result,
        )
        .already_emitted())
    } else {
        println!(
            "Deleted {} snapshots (kept {} most recent)",
            deleted_paths.len(),
            keep
        );
        for path in &deleted_paths {
            println!("  - {}", path.display());
        }
        Ok(splice::cli::CliSuccessPayload::message_only(format!(
            "Deleted {} snapshots",
            deleted_paths.len()
        )))
    }
}
