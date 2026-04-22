fn dashboard_prompt_integrations_checkpoint_utils_describe(payload: &Value) -> Value {
    let operation = clean_text(
        payload
            .get("operation")
            .and_then(Value::as_str)
            .unwrap_or("normalize"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_utils_describe",
        "operation": operation
    })
}

fn dashboard_prompt_integrations_multi_root_checkpoint_manager_describe(payload: &Value) -> Value {
    let workspace_count = payload
        .get("workspace_count")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_multi_root_checkpoint_manager_describe",
        "workspace_count": workspace_count
    })
}

fn dashboard_prompt_integrations_checkpoint_factory_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_factory_describe",
        "profile": profile
    })
}

fn dashboard_prompt_integrations_checkpoint_index_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_index_describe",
        "exports": ["factory", "initializer", "types", "manager"]
    })
}

fn dashboard_prompt_integrations_checkpoint_initializer_describe(payload: &Value) -> Value {
    let boot_mode = clean_text(
        payload
            .get("boot_mode")
            .and_then(Value::as_str)
            .unwrap_or("lazy"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_initializer_describe",
        "boot_mode": boot_mode
    })
}

fn dashboard_prompt_integrations_checkpoint_types_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_types_describe",
        "contracts": ["checkpoint_id", "checkpoint_state", "checkpoint_scope"]
    })
}

fn dashboard_prompt_integrations_claude_message_filter_describe(payload: &Value) -> Value {
    let policy = clean_text(
        payload
            .get("policy")
            .and_then(Value::as_str)
            .unwrap_or("safe"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_claude_message_filter_describe",
        "policy": policy
    })
}

fn dashboard_prompt_integrations_claude_run_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("interactive"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_claude_run_describe",
        "mode": mode
    })
}

fn dashboard_prompt_integrations_claude_types_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_claude_types_describe",
        "contracts": ["claude_run_request", "claude_run_result"]
    })
}

fn dashboard_prompt_integrations_diagnostics_index_describe(payload: &Value) -> Value {
    let detail = clean_text(
        payload
            .get("detail")
            .and_then(Value::as_str)
            .unwrap_or("summary"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_diagnostics_index_describe",
        "detail": detail
    })
}

fn dashboard_prompt_integrations_checkpoint_claude_diagnostics_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.integrations.checkpoints.checkpointUtils.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_utils_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.multiRootCheckpointManager.describe" => {
            Some(dashboard_prompt_integrations_multi_root_checkpoint_manager_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.factory.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_factory_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.index.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_index_describe())
        }
        "dashboard.prompts.system.integrations.checkpoints.initializer.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_initializer_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.types.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_types_describe())
        }
        "dashboard.prompts.system.integrations.claudeCode.messageFilter.describe" => {
            Some(dashboard_prompt_integrations_claude_message_filter_describe(payload))
        }
        "dashboard.prompts.system.integrations.claudeCode.run.describe" => {
            Some(dashboard_prompt_integrations_claude_run_describe(payload))
        }
        "dashboard.prompts.system.integrations.claudeCode.types.describe" => {
            Some(dashboard_prompt_integrations_claude_types_describe())
        }
        "dashboard.prompts.system.integrations.diagnostics.index.describe" => {
            Some(dashboard_prompt_integrations_diagnostics_index_describe(payload))
        }
        _ => dashboard_prompt_integrations_editor_misc_tail_route_extension(root, normalized, payload),
    }
}

include!("034-dashboard-system-prompt-integrations-editor-misc-tail-helpers.rs");
