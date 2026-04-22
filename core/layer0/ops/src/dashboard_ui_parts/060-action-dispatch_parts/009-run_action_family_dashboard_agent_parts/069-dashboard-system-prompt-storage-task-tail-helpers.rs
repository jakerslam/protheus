fn dashboard_prompt_storage_remote_config_sync_servers_describe(payload: &Value) -> Value {
    let sync_mode = clean_text(
        payload
            .get("sync_mode")
            .and_then(Value::as_str)
            .unwrap_or("full"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_remote_config_sync_servers_describe",
        "sync_mode": sync_mode
    })
}

fn dashboard_prompt_storage_remote_config_utils_describe(payload: &Value) -> Value {
    let helper = clean_text(
        payload
            .get("helper")
            .and_then(Value::as_str)
            .unwrap_or("normalize"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_remote_config_utils_describe",
        "helper": helper
    })
}

fn dashboard_prompt_storage_state_migrations_describe(payload: &Value) -> Value {
    let migration_mode = clean_text(
        payload
            .get("migration_mode")
            .and_then(Value::as_str)
            .unwrap_or("forward_only"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_state_migrations_describe",
        "migration_mode": migration_mode
    })
}

fn dashboard_prompt_storage_state_helpers_describe(payload: &Value) -> Value {
    let helper = clean_text(
        payload
            .get("helper")
            .and_then(Value::as_str)
            .unwrap_or("state_helpers"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_state_helpers_describe",
        "helper": helper
    })
}

fn dashboard_prompt_task_stream_chunk_coordinator_describe(payload: &Value) -> Value {
    let chunk_mode = clean_text(
        payload
            .get("chunk_mode")
            .and_then(Value::as_str)
            .unwrap_or("ordered"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_stream_chunk_coordinator_describe",
        "chunk_mode": chunk_mode
    })
}

fn dashboard_prompt_task_stream_response_handler_describe(payload: &Value) -> Value {
    let response_mode = clean_text(
        payload
            .get("response_mode")
            .and_then(Value::as_str)
            .unwrap_or("streaming"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_stream_response_handler_describe",
        "response_mode": response_mode
    })
}

fn dashboard_prompt_task_lock_utils_describe(payload: &Value) -> Value {
    let lock_strategy = clean_text(
        payload
            .get("lock_strategy")
            .and_then(Value::as_str)
            .unwrap_or("session"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_lock_utils_describe",
        "lock_strategy": lock_strategy
    })
}

fn dashboard_prompt_task_presentation_scheduler_describe(payload: &Value) -> Value {
    let schedule_mode = clean_text(
        payload
            .get("schedule_mode")
            .and_then(Value::as_str)
            .unwrap_or("deterministic"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_presentation_scheduler_describe",
        "schedule_mode": schedule_mode
    })
}

fn dashboard_prompt_task_state_describe(payload: &Value) -> Value {
    let state_profile = clean_text(
        payload
            .get("state_profile")
            .and_then(Value::as_str)
            .unwrap_or("active"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_state_describe",
        "state_profile": state_profile
    })
}

fn dashboard_prompt_task_tool_executor_describe(payload: &Value) -> Value {
    let executor_mode = clean_text(
        payload
            .get("executor_mode")
            .and_then(Value::as_str)
            .unwrap_or("guarded"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_tool_executor_describe",
        "executor_mode": executor_mode
    })
}

fn dashboard_prompt_storage_task_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.storage.remoteConfig.syncRemoteMcpServers.describe" => {
            Some(dashboard_prompt_storage_remote_config_sync_servers_describe(payload))
        }
        "dashboard.prompts.system.storage.remoteConfig.utils.describe" => {
            Some(dashboard_prompt_storage_remote_config_utils_describe(payload))
        }
        "dashboard.prompts.system.storage.stateMigrations.describe" => {
            Some(dashboard_prompt_storage_state_migrations_describe(payload))
        }
        "dashboard.prompts.system.storage.stateHelpers.describe" => {
            Some(dashboard_prompt_storage_state_helpers_describe(payload))
        }
        "dashboard.prompts.system.task.streamChunkCoordinator.describe" => {
            Some(dashboard_prompt_task_stream_chunk_coordinator_describe(payload))
        }
        "dashboard.prompts.system.task.streamResponseHandler.describe" => {
            Some(dashboard_prompt_task_stream_response_handler_describe(payload))
        }
        "dashboard.prompts.system.task.taskLockUtils.describe" => {
            Some(dashboard_prompt_task_lock_utils_describe(payload))
        }
        "dashboard.prompts.system.task.taskPresentationScheduler.describe" => {
            Some(dashboard_prompt_task_presentation_scheduler_describe(payload))
        }
        "dashboard.prompts.system.task.taskState.describe" => {
            Some(dashboard_prompt_task_state_describe(payload))
        }
        "dashboard.prompts.system.task.toolExecutor.describe" => {
            Some(dashboard_prompt_task_tool_executor_describe(payload))
        }
        _ => dashboard_prompt_task_focus_chain_route_extension(root, normalized, payload),
    }
}

// NOTE: 070+ helper chain mirrors the existing 023+ chain and causes duplicate
// symbol definitions when both are included in this module.
// Keep 069 as the terminal include in the active chain.
