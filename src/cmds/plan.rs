//! Plan and undo command handlers.

use std::path::Path;

use serde_json::{json, Value};

use super::helpers::log_execution_error;

pub(crate) fn execute_plan(
    plan_path: &Path,
    operation_id: Option<String>,
    metadata: Option<String>,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::execution::log;
    use splice::output::{OperationData, OperationResult, PlanResult, StepResult};
    use splice::plan::execute_plan;

    let start = std::time::Instant::now();
    let command_line = std::env::args().collect::<Vec<_>>().join(" ");

    let workspace_dir = plan_path.parent().ok_or_else(|| {
        splice::SpliceError::Other(
            "Cannot determine workspace directory from plan path".to_string(),
        )
    })?;

    let messages = execute_plan(plan_path, workspace_dir)?;
    let step_count = messages.len();

    if _json_output {
        let steps: Vec<StepResult> = messages
            .iter()
            .enumerate()
            .map(|(idx, msg)| StepResult {
                step: idx + 1,
                status: "ok".to_string(),
                message: msg.clone(),
                file: plan_path.to_string_lossy().to_string(),
                symbol: "plan".to_string(),
            })
            .collect();

        let plan_result = PlanResult {
            total_steps: messages.len(),
            steps_completed: messages.len(),
            steps,
            files_affected: {
                let mut files = vec![plan_path.to_string_lossy().to_string()];
                files.sort();
                files
            },
            total_bytes_changed: 0,
        };

        let message = format!(
            "Plan executed successfully: {} steps completed",
            messages.len()
        );

        let result = OperationResult::with_execution_id("plan".to_string(), operation_id.clone())
            .success(message)
            .with_result(OperationData::Plan(plan_result));

        println!(
            "{}",
            serde_json::to_string_pretty(&result)
                .expect("invariant: serde_json serialization never fails on serializable types")
        );

        let duration_ms = start.elapsed().as_millis() as i64;
        let parameters = serde_json::json!({
            "plan_file": plan_path.to_string_lossy(),
            "step_count": step_count,
        });
        if let Err(e) = log::record_execution_with_params(
            &splice::output::OperationResult::with_execution_id(
                "plan".to_string(),
                operation_id.clone(),
            )
            .success(format!(
                "Plan executed successfully: {} steps completed",
                step_count
            )),
            duration_ms,
            Some(command_line.clone()),
            parameters,
        ) {
            log_execution_error("plan", &e);
        }

        return Ok(
            splice::cli::CliSuccessPayload::message_only("OK".to_string()).already_emitted(),
        );
    }

    let mut response_data = serde_json::Map::new();
    response_data.insert("steps_completed".to_string(), json!(messages.len()));

    if let Some(ref op_id) = operation_id {
        response_data.insert("operation_id".to_string(), json!(op_id));
    }

    if let Some(ref meta) = metadata {
        if let Ok(parsed) = serde_json::from_str::<Value>(meta) {
            response_data.insert("metadata".to_string(), parsed);
        } else {
            response_data.insert("metadata".to_string(), json!(meta));
        }
    }

    let duration_ms = start.elapsed().as_millis() as i64;
    let message = format!(
        "Plan executed successfully: {} steps completed",
        messages.len()
    );
    let parameters = serde_json::json!({
        "plan_file": plan_path.to_string_lossy(),
        "step_count": step_count,
    });
    if let Err(e) = log::record_execution_with_params(
        &splice::output::OperationResult::with_execution_id(
            "plan".to_string(),
            operation_id.clone(),
        )
        .success(message.clone()),
        duration_ms,
        Some(command_line.clone()),
        parameters,
    ) {
        log_execution_error("plan", &e);
    }

    Ok(splice::cli::CliSuccessPayload::with_data(
        message,
        serde_json::Value::Object(response_data),
    ))
}

pub(crate) fn execute_undo(
    manifest_path: &Path,
    _json_output: bool,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::patch::restore_from_manifest;

    let backup_dir = manifest_path.parent().ok_or_else(|| {
        splice::SpliceError::Other("Manifest has no parent directory".to_string())
    })?;

    let splice_backup_dir = backup_dir.parent().ok_or_else(|| {
        splice::SpliceError::Other("Backup directory has no parent directory".to_string())
    })?;

    let workspace_root = splice_backup_dir.parent().ok_or_else(|| {
        splice::SpliceError::Other("Cannot determine workspace root from manifest path".to_string())
    })?;

    let restored_count = restore_from_manifest(manifest_path, workspace_root)?;

    Ok(splice::cli::CliSuccessPayload::message_only(format!(
        "Restored {} file(s) from backup.",
        restored_count
    )))
}
