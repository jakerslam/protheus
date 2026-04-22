fn dashboard_prompt_host_vscode_terminal_process_describe(payload: &Value) -> Value {
    let terminal_id = clean_text(
        payload
            .get("terminal_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    );
    let status = clean_text(
        payload
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("running"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_terminal_process_describe",
        "terminal_id": terminal_id,
        "status": status
    })
}

fn dashboard_prompt_host_vscode_terminal_registry_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_host_vscode_terminal_registry_describe",
        "scope": scope
    })
}

fn dashboard_prompt_host_vscode_terminal_ansi_utils_describe(payload: &Value) -> Value {
    let sample = clean_text(payload.get("sample").and_then(Value::as_str).unwrap_or(""), 1200);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_terminal_ansi_utils_describe",
        "sample": sample
    })
}

fn dashboard_prompt_host_vscode_terminal_get_latest_output_describe(payload: &Value) -> Value {
    let terminal_id = clean_text(
        payload
            .get("terminal_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    );
    let max_chars = payload
        .get("max_chars")
        .and_then(Value::as_u64)
        .unwrap_or(2000);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_terminal_get_latest_output_describe",
        "terminal_id": terminal_id,
        "max_chars": max_chars
    })
}

fn dashboard_prompt_host_vscode_to_file_migration_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("detect"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_to_file_migration_describe",
        "mode": mode
    })
}

fn dashboard_prompt_integrations_checkpoint_exclusions_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_exclusions_describe",
        "profile": profile
    })
}

fn dashboard_prompt_integrations_checkpoint_git_ops_describe(payload: &Value) -> Value {
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
    let lock_scope = clean_text(
        payload
            .get("lock_scope")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_lock_utils_describe",
        "lock_scope": lock_scope
    })
}

fn dashboard_prompt_integrations_checkpoint_migration_describe(payload: &Value) -> Value {
    let strategy = clean_text(
        payload
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or("safe"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_migration_describe",
        "strategy": strategy
    })
}

fn dashboard_prompt_integrations_checkpoint_tracker_describe(payload: &Value) -> Value {
    let tracker_id = clean_text(
        payload
            .get("tracker_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_tracker_describe",
        "tracker_id": tracker_id
    })
}

fn dashboard_prompt_host_vscode_terminal_checkpoint_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.vscode.terminal.vscodeTerminalProcess.describe" => {
            Some(dashboard_prompt_host_vscode_terminal_process_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.terminal.vscodeTerminalRegistry.describe" => {
            Some(dashboard_prompt_host_vscode_terminal_registry_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.terminal.ansiUtils.describe" => {
            Some(dashboard_prompt_host_vscode_terminal_ansi_utils_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.terminal.getLatestOutput.describe" => {
            Some(dashboard_prompt_host_vscode_terminal_get_latest_output_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.vscodeToFileMigration.describe" => {
            Some(dashboard_prompt_host_vscode_to_file_migration_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.checkpointExclusions.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_exclusions_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.checkpointGitOperations.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_git_ops_describe(payload))
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
        _ => dashboard_prompt_integrations_checkpoint_claude_diagnostics_tail_route_extension(
            root, normalized, payload,
        ),
    }
}

include!("033-dashboard-system-prompt-integrations-checkpoint-claude-diagnostics-tail-helpers.rs");
