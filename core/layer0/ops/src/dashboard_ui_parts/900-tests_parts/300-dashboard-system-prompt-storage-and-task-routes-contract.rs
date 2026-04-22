#[test]
fn dashboard_system_prompt_storage_routes_contract_wave_300() {
    let root = tempfile::tempdir().expect("tempdir");

    let sync = run_action(
        root.path(),
        "dashboard.prompts.system.storage.remoteConfig.sync",
        &json!({"source": "remote-mcp", "dry_run": false}),
    );
    assert!(sync.ok);
    let sync_payload = sync.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        sync_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_storage_remote_config_sync")
    );

    let plan = run_action(
        root.path(),
        "dashboard.prompts.system.storage.stateMigrations.plan",
        &json!({}),
    );
    assert!(plan.ok);
    assert_eq!(
        plan.payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_storage_state_migrations_plan")
    );

    let apply = run_action(
        root.path(),
        "dashboard.prompts.system.storage.stateMigrations.apply",
        &json!({}),
    );
    assert!(apply.ok);
    assert_eq!(
        apply.payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_storage_state_migrations_apply")
    );

    let normalize = run_action(
        root.path(),
        "dashboard.prompts.system.storage.stateHelpers.normalize",
        &json!({"keys": ["Prompt_Variants", "prompt_registry", "Prompt_Variants"]}),
    );
    assert!(normalize.ok);
    let normalize_payload = normalize.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        normalize_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_storage_state_helpers_normalize")
    );
    assert!(
        normalize_payload
            .get("normalized_keys")
            .and_then(Value::as_array)
            .map(|rows| rows.len() >= 2)
            .unwrap_or(false)
    );
}

#[test]
fn dashboard_system_prompt_task_routes_contract_wave_300() {
    let root = tempfile::tempdir().expect("tempdir");

    let chunk_plan = run_action(
        root.path(),
        "dashboard.prompts.system.task.stream.chunkPlan",
        &json!({"chunk_size": 256, "message_size": 1024}),
    );
    assert!(chunk_plan.ok);
    let chunk_plan_payload = chunk_plan.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        chunk_plan_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_task_stream_chunk_plan")
    );
    assert_eq!(
        chunk_plan_payload.get("chunk_count").and_then(Value::as_i64),
        Some(4)
    );

    let assemble = run_action(
        root.path(),
        "dashboard.prompts.system.task.stream.responseAssemble",
        &json!({"chunks": ["hello ", "world"]}),
    );
    assert!(assemble.ok);
    assert_eq!(
        assemble
            .payload
            .unwrap_or_else(|| json!({}))
            .get("response_text")
            .and_then(Value::as_str),
        Some("hello world")
    );

    let lock_sim = run_action(
        root.path(),
        "dashboard.prompts.system.task.lockUtils.simulate",
        &json!({"lock_key": "task-42", "mode": "acquire"}),
    );
    assert!(lock_sim.ok);
    assert_eq!(
        lock_sim
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_task_lock_utils_simulate")
    );

    let tool_plan = run_action(
        root.path(),
        "dashboard.prompts.system.task.toolExecutor.plan",
        &json!({"tools": ["read_file", "apply_patch"]}),
    );
    assert!(tool_plan.ok);
    let tool_plan_payload = tool_plan.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        tool_plan_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_task_tool_executor_plan")
    );
    assert_eq!(
        tool_plan_payload.get("tool_count").and_then(Value::as_i64),
        Some(2)
    );
}
