fn dashboard_prompt_integrations_checkpoint_utils_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("standard"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_utils_describe",
        "mode": mode
    })
}

fn dashboard_prompt_integrations_checkpoint_multi_root_manager_describe(payload: &Value) -> Value {
    let root_count = payload
        .get("root_count")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_multi_root_manager_describe",
        "root_count": root_count
    })
}

fn dashboard_prompt_integrations_checkpoint_factory_describe(payload: &Value) -> Value {
    let factory_mode = clean_text(
        payload
            .get("factory_mode")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_factory_describe",
        "factory_mode": factory_mode
    })
}

fn dashboard_prompt_integrations_checkpoint_index_describe(payload: &Value) -> Value {
    let export_set = clean_text(
        payload
            .get("export_set")
            .and_then(Value::as_str)
            .unwrap_or("all"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_index_describe",
        "export_set": export_set
    })
}

fn dashboard_prompt_integrations_checkpoint_initializer_describe(payload: &Value) -> Value {
    let init_mode = clean_text(
        payload
            .get("init_mode")
            .and_then(Value::as_str)
            .unwrap_or("lazy"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_initializer_describe",
        "init_mode": init_mode
    })
}

fn dashboard_prompt_integrations_checkpoint_types_describe(payload: &Value) -> Value {
    let type_set = clean_text(
        payload
            .get("type_set")
            .and_then(Value::as_str)
            .unwrap_or("core"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_checkpoint_types_describe",
        "type_set": type_set
    })
}

fn dashboard_prompt_integrations_claude_code_message_filter_describe(payload: &Value) -> Value {
    let filter_mode = clean_text(
        payload
            .get("filter_mode")
            .and_then(Value::as_str)
            .unwrap_or("strict"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_claude_code_message_filter_describe",
        "filter_mode": filter_mode
    })
}

fn dashboard_prompt_integrations_claude_code_run_describe(payload: &Value) -> Value {
    let run_mode = clean_text(
        payload
            .get("run_mode")
            .and_then(Value::as_str)
            .unwrap_or("interactive"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_claude_code_run_describe",
        "run_mode": run_mode
    })
}

fn dashboard_prompt_integrations_claude_code_types_describe(payload: &Value) -> Value {
    let schema = clean_text(
        payload
            .get("schema")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_claude_code_types_describe",
        "schema": schema
    })
}

fn dashboard_prompt_integrations_diagnostics_index_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_integrations_diagnostics_index_describe",
        "scope": scope
    })
}

fn dashboard_prompt_hosts_surface_tail_checkpoint_claude_diagnostics_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.integrations.checkpoints.checkpointUtils.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_utils_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.multiRootCheckpointManager.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_multi_root_manager_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.factory.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_factory_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.index.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_index_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.initializer.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_initializer_describe(payload))
        }
        "dashboard.prompts.system.integrations.checkpoints.types.describe" => {
            Some(dashboard_prompt_integrations_checkpoint_types_describe(payload))
        }
        "dashboard.prompts.system.integrations.claudeCode.messageFilter.describe" => {
            Some(dashboard_prompt_integrations_claude_code_message_filter_describe(payload))
        }
        "dashboard.prompts.system.integrations.claudeCode.run.describe" => {
            Some(dashboard_prompt_integrations_claude_code_run_describe(payload))
        }
        "dashboard.prompts.system.integrations.claudeCode.types.describe" => {
            Some(dashboard_prompt_integrations_claude_code_types_describe(payload))
        }
        "dashboard.prompts.system.integrations.diagnostics.index.describe" => {
            Some(dashboard_prompt_integrations_diagnostics_index_describe(payload))
        }
        _ => dashboard_prompt_hosts_surface_tail_integrations_editor_misc_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}
include!("081-dashboard-system-prompt-integrations-editor-misc-tail-helpers.rs");
