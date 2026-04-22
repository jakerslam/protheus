fn dashboard_prompt_hosts_surface_vscode_terminal_process_describe(payload: &Value) -> Value {
    let lifecycle = clean_text(
        payload
            .get("lifecycle")
            .and_then(Value::as_str)
            .unwrap_or("managed"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_terminal_process_describe",
        "lifecycle": lifecycle
    })
}

fn dashboard_prompt_hosts_surface_vscode_terminal_registry_describe(payload: &Value) -> Value {
    let scope = clean_text(
        payload
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_terminal_registry_describe",
        "scope": scope
    })
}

fn dashboard_prompt_hosts_surface_vscode_terminal_ansi_utils_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("sanitize"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_terminal_ansi_utils_describe",
        "mode": mode
    })
}

fn dashboard_prompt_hosts_surface_vscode_terminal_latest_output_describe(payload: &Value) -> Value {
    let max_lines = payload
        .get("max_lines")
        .and_then(Value::as_u64)
        .unwrap_or(200);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_terminal_latest_output_describe",
        "max_lines": max_lines
    })
}

fn dashboard_prompt_hosts_surface_vscode_to_file_migration_describe(payload: &Value) -> Value {
    let migration_mode = clean_text(
        payload
            .get("migration_mode")
            .and_then(Value::as_str)
            .unwrap_or("safe"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_to_file_migration_describe",
        "migration_mode": migration_mode
    })
}

fn dashboard_prompt_integrations_checkpoint_exclusions_describe(payload: &Value) -> Value {
    let policy = clean_text(
        payload
            .get("policy")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_exclusions_describe",
        "policy": policy
    })
}

fn dashboard_prompt_integrations_checkpoint_git_operations_describe(payload: &Value) -> Value {
    let operation = clean_text(
        payload
            .get("operation")
            .and_then(Value::as_str)
            .unwrap_or("status"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_git_operations_describe",
        "operation": operation
    })
}

fn dashboard_prompt_integrations_checkpoint_lock_utils_describe(payload: &Value) -> Value {
    let lock_mode = clean_text(
        payload
            .get("lock_mode")
            .and_then(Value::as_str)
            .unwrap_or("advisory"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_lock_utils_describe",
        "lock_mode": lock_mode
    })
}

fn dashboard_prompt_integrations_checkpoint_migration_describe(payload: &Value) -> Value {
    let migration_stage = clean_text(
        payload
            .get("migration_stage")
            .and_then(Value::as_str)
            .unwrap_or("plan"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_migration_describe",
        "migration_stage": migration_stage
    })
}

fn dashboard_prompt_integrations_checkpoint_tracker_describe(payload: &Value) -> Value {
    let tracker_scope = clean_text(
        payload
            .get("tracker_scope")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_tracker_describe",
        "tracker_scope": tracker_scope
    })
}

fn dashboard_prompt_hosts_surface_tail_terminal_checkpoint_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.surface.vscode.terminal.vscodeTerminalProcess.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_terminal_process_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.terminal.vscodeTerminalRegistry.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_terminal_registry_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.terminal.ansiUtils.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_terminal_ansi_utils_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.terminal.getLatestOutput.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_terminal_latest_output_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.vscodeToFileMigration.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_to_file_migration_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.checkpointExclusions.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_exclusions_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.checkpointGitOperations.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_git_operations_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.checkpointLockUtils.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_lock_utils_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.checkpointMigration.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_migration_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.checkpointTracker.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_tracker_describe(payload))
        }
        _ => dashboard_prompt_hosts_surface_tail_checkpoint_claude_diagnostics_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}
include!("080-dashboard-system-prompt-checkpoint-claude-diagnostics-tail-helpers.rs");
