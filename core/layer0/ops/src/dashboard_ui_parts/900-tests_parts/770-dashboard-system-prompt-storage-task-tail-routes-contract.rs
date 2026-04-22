#[test]
fn dashboard_system_prompt_storage_task_tail_routes_contract_wave_770() {
    let root = tempfile::tempdir().expect("tempdir");

    let sync_remote_mcp_servers = run_action(
        root.path(),
        "dashboard.prompts.system.storage.remoteConfig.syncRemoteMcpServers.describe",
        &json!({"sync_mode": "full"}),
    );
    assert!(sync_remote_mcp_servers.ok);
    assert_eq!(
        sync_remote_mcp_servers
            .payload
            .unwrap_or_else(|| json!({}))
            .get("sync_mode")
            .and_then(Value::as_str),
        Some("full")
    );

    let remote_config_utils = run_action(
        root.path(),
        "dashboard.prompts.system.storage.remoteConfig.utils.describe",
        &json!({"helper": "normalize"}),
    );
    assert!(remote_config_utils.ok);
    assert_eq!(
        remote_config_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("helper")
            .and_then(Value::as_str),
        Some("normalize")
    );

    let state_migrations = run_action(
        root.path(),
        "dashboard.prompts.system.storage.stateMigrations.describe",
        &json!({"migration_mode": "forward_only"}),
    );
    assert!(state_migrations.ok);
    assert_eq!(
        state_migrations
            .payload
            .unwrap_or_else(|| json!({}))
            .get("migration_mode")
            .and_then(Value::as_str),
        Some("forward_only")
    );

    let state_helpers = run_action(
        root.path(),
        "dashboard.prompts.system.storage.stateHelpers.describe",
        &json!({"helper": "state_helpers"}),
    );
    assert!(state_helpers.ok);
    assert_eq!(
        state_helpers
            .payload
            .unwrap_or_else(|| json!({}))
            .get("helper")
            .and_then(Value::as_str),
        Some("state_helpers")
    );

    let stream_chunk_coordinator = run_action(
        root.path(),
        "dashboard.prompts.system.task.streamChunkCoordinator.describe",
        &json!({"chunk_mode": "ordered"}),
    );
    assert!(stream_chunk_coordinator.ok);
    assert_eq!(
        stream_chunk_coordinator
            .payload
            .unwrap_or_else(|| json!({}))
            .get("chunk_mode")
            .and_then(Value::as_str),
        Some("ordered")
    );

    let stream_response_handler = run_action(
        root.path(),
        "dashboard.prompts.system.task.streamResponseHandler.describe",
        &json!({"response_mode": "streaming"}),
    );
    assert!(stream_response_handler.ok);
    assert_eq!(
        stream_response_handler
            .payload
            .unwrap_or_else(|| json!({}))
            .get("response_mode")
            .and_then(Value::as_str),
        Some("streaming")
    );

    let task_lock_utils = run_action(
        root.path(),
        "dashboard.prompts.system.task.taskLockUtils.describe",
        &json!({"lock_strategy": "session"}),
    );
    assert!(task_lock_utils.ok);
    assert_eq!(
        task_lock_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("lock_strategy")
            .and_then(Value::as_str),
        Some("session")
    );

    let task_presentation_scheduler = run_action(
        root.path(),
        "dashboard.prompts.system.task.taskPresentationScheduler.describe",
        &json!({"schedule_mode": "deterministic"}),
    );
    assert!(task_presentation_scheduler.ok);
    assert_eq!(
        task_presentation_scheduler
            .payload
            .unwrap_or_else(|| json!({}))
            .get("schedule_mode")
            .and_then(Value::as_str),
        Some("deterministic")
    );

    let task_state = run_action(
        root.path(),
        "dashboard.prompts.system.task.taskState.describe",
        &json!({"state_profile": "active"}),
    );
    assert!(task_state.ok);
    assert_eq!(
        task_state
            .payload
            .unwrap_or_else(|| json!({}))
            .get("state_profile")
            .and_then(Value::as_str),
        Some("active")
    );

    let tool_executor = run_action(
        root.path(),
        "dashboard.prompts.system.task.toolExecutor.describe",
        &json!({"executor_mode": "guarded"}),
    );
    assert!(tool_executor.ok);
    assert_eq!(
        tool_executor
            .payload
            .unwrap_or_else(|| json!({}))
            .get("executor_mode")
            .and_then(Value::as_str),
        Some("guarded")
    );
}
