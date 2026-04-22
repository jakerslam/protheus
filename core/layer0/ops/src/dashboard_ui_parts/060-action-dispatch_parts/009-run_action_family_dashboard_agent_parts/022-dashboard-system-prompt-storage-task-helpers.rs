fn dashboard_prompt_storage_remote_sync(root: &Path, payload: &Value) -> Value {
    let dry_run = payload
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let source = clean_text(
        payload
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("remote-mcp"),
        120,
    );
    let state = dashboard_lpp_mutate_state(root, |state| {
        state["remote_prompt_sync"] = json!({
            "source": source,
            "dry_run": dry_run,
            "synced_at": crate::now_iso()
        });
        state["remote_prompt_sync_count"] = Value::from(
            i64_from_value(state.get("remote_prompt_sync_count"), 0).saturating_add(1),
        );
    });
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_remote_config_sync",
        "dry_run": dry_run,
        "source": source,
        "state": state
    })
}

fn dashboard_prompt_storage_remote_utils(payload: &Value) -> Value {
    let etag = clean_text(payload.get("etag").and_then(Value::as_str).unwrap_or(""), 120);
    let checksum = clean_text(
        payload
            .get("checksum")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let cache_key = if etag.is_empty() {
        format!("remote-config:{checksum}")
    } else {
        format!("remote-config:{etag}:{checksum}")
    };
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_remote_config_utils",
        "cache_key": cache_key,
        "etag_present": !etag.is_empty(),
        "checksum_present": !checksum.is_empty()
    })
}

fn dashboard_prompt_storage_state_migrations_plan(root: &Path) -> Value {
    let state = dashboard_lpp_read_state(root);
    let has_variants = state
        .get("prompt_variants")
        .and_then(Value::as_object)
        .map(|m| !m.is_empty())
        .unwrap_or(false);
    let has_registry = state
        .get("prompt_registry")
        .and_then(Value::as_object)
        .map(|m| !m.is_empty())
        .unwrap_or(false);
    let required_steps = vec![
        json!({"step": "ensure_prompt_registry_object", "required": !has_registry}),
        json!({"step": "ensure_prompt_variants_object", "required": !has_variants}),
        json!({"step": "ensure_remote_prompt_config_object", "required": !state.get("remote_prompt_config").map(Value::is_object).unwrap_or(false)})
    ];
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_state_migrations_plan",
        "steps": required_steps
    })
}

fn dashboard_prompt_storage_state_migrations_apply(root: &Path) -> Value {
    let state = dashboard_lpp_mutate_state(root, |state| {
        if !state
            .get("prompt_registry")
            .map(Value::is_object)
            .unwrap_or(false)
        {
            state["prompt_registry"] = json!({});
        }
        if !state
            .get("prompt_variants")
            .map(Value::is_object)
            .unwrap_or(false)
        {
            state["prompt_variants"] = json!({});
        }
        if !state
            .get("remote_prompt_config")
            .map(Value::is_object)
            .unwrap_or(false)
        {
            state["remote_prompt_config"] = json!({});
        }
        state["state_migrations_last_applied_at"] = Value::String(crate::now_iso());
    });
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_state_migrations_apply",
        "state": state
    })
}

fn dashboard_prompt_storage_state_helpers_normalize(payload: &Value) -> Value {
    let mut keys = payload
        .get("keys")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 120).to_ascii_lowercase()))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_state_helpers_normalize",
        "normalized_keys": keys
    })
}

fn dashboard_prompt_task_stream_chunk_plan(payload: &Value) -> Value {
    let chunk_size = payload
        .get("chunk_size")
        .and_then(Value::as_i64)
        .unwrap_or(1024)
        .clamp(64, 16384);
    let message_size = payload
        .get("message_size")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let chunk_count = if message_size == 0 {
        0
    } else {
        ((message_size + chunk_size - 1) / chunk_size).max(1)
    };
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_stream_chunk_plan",
        "chunk_size": chunk_size,
        "message_size": message_size,
        "chunk_count": chunk_count
    })
}

fn dashboard_prompt_task_stream_response_assemble(payload: &Value) -> Value {
    let chunks = payload
        .get("chunks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 2000)))
        .collect::<Vec<_>>();
    let response = chunks.join("");
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_stream_response_assemble",
        "chunk_count": chunks.len() as i64,
        "response_text": response
    })
}

fn dashboard_prompt_task_lock_utils_simulate(root: &Path, payload: &Value) -> Value {
    let lock_key = clean_text(
        payload
            .get("lock_key")
            .and_then(Value::as_str)
            .unwrap_or("task_default"),
        120,
    );
    let mode = clean_text(payload.get("mode").and_then(Value::as_str).unwrap_or("acquire"), 40)
        .to_ascii_lowercase();
    let state = dashboard_lpp_mutate_state(root, |state| {
        if !state.get("task_locks").map(Value::is_object).unwrap_or(false) {
            state["task_locks"] = json!({});
        }
        state["task_locks"][lock_key.as_str()] = json!({
            "mode": mode,
            "updated_at": crate::now_iso()
        });
    });
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_lock_utils_simulate",
        "lock_key": lock_key,
        "mode": mode,
        "state": state
    })
}

fn dashboard_prompt_task_presentation_schedule(payload: &Value) -> Value {
    let priority = clean_text(
        payload
            .get("priority")
            .and_then(Value::as_str)
            .unwrap_or("normal"),
        40,
    )
    .to_ascii_lowercase();
    let lane = clean_text(payload.get("lane").and_then(Value::as_str).unwrap_or("default"), 80);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_presentation_schedule",
        "priority": priority,
        "lane": lane
    })
}

fn dashboard_prompt_task_state_snapshot(root: &Path) -> Value {
    let state = dashboard_lpp_read_state(root);
    let lock_count = state
        .get("task_locks")
        .and_then(Value::as_object)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_state_snapshot",
        "lock_count": lock_count,
        "state": state
    })
}

fn dashboard_prompt_task_tool_executor_plan(payload: &Value) -> Value {
    let tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|raw| clean_text(raw, 120)))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let plan = tools
        .iter()
        .enumerate()
        .map(|(idx, tool)| {
            json!({
                "order": (idx as i64) + 1,
                "tool": tool,
                "mode": "execute"
            })
        })
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_task_tool_executor_plan",
        "tool_count": tools.len() as i64,
        "plan": plan
    })
}

fn dashboard_prompt_storage_task_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.storage.remoteConfig.sync" => {
            Some(dashboard_prompt_storage_remote_sync(root, payload))
        }
        "dashboard.prompts.system.storage.remoteConfig.utils" => {
            Some(dashboard_prompt_storage_remote_utils(payload))
        }
        "dashboard.prompts.system.storage.stateMigrations.plan" => {
            Some(dashboard_prompt_storage_state_migrations_plan(root))
        }
        "dashboard.prompts.system.storage.stateMigrations.apply" => {
            Some(dashboard_prompt_storage_state_migrations_apply(root))
        }
        "dashboard.prompts.system.storage.stateHelpers.normalize" => {
            Some(dashboard_prompt_storage_state_helpers_normalize(payload))
        }
        "dashboard.prompts.system.task.stream.chunkPlan" => {
            Some(dashboard_prompt_task_stream_chunk_plan(payload))
        }
        "dashboard.prompts.system.task.stream.responseAssemble" => {
            Some(dashboard_prompt_task_stream_response_assemble(payload))
        }
        "dashboard.prompts.system.task.lockUtils.simulate" => {
            Some(dashboard_prompt_task_lock_utils_simulate(root, payload))
        }
        "dashboard.prompts.system.task.presentation.schedule" => {
            Some(dashboard_prompt_task_presentation_schedule(payload))
        }
        "dashboard.prompts.system.task.state.snapshot" => {
            Some(dashboard_prompt_task_state_snapshot(root))
        }
        "dashboard.prompts.system.task.toolExecutor.plan" => {
            Some(dashboard_prompt_task_tool_executor_plan(payload))
        }
        _ => dashboard_prompt_task_focus_chain_route_extension(root, normalized, payload),
    }
}

include!("023-dashboard-system-prompt-task-focus-chain-helpers.rs");
