#[test]
fn dashboard_system_prompt_checkpoint_claude_diagnostics_tail_routes_contract_wave_880() {
    let root = tempfile::tempdir().expect("tempdir");

    let checkpoint_utils = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.checkpointUtils.describe",
        &json!({"mode": "standard"}),
    );
    assert!(checkpoint_utils.ok);
    assert_eq!(
        checkpoint_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("standard")
    );

    let checkpoint_multi_root = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.multiRootCheckpointManager.describe",
        &json!({"root_count": 2}),
    );
    assert!(checkpoint_multi_root.ok);
    assert_eq!(
        checkpoint_multi_root
            .payload
            .unwrap_or_else(|| json!({}))
            .get("root_count")
            .and_then(Value::as_u64),
        Some(2)
    );

    let checkpoint_factory = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.factory.describe",
        &json!({"factory_mode": "default"}),
    );
    assert!(checkpoint_factory.ok);
    assert_eq!(
        checkpoint_factory
            .payload
            .unwrap_or_else(|| json!({}))
            .get("factory_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let checkpoint_index = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.index.describe",
        &json!({"export_set": "all"}),
    );
    assert!(checkpoint_index.ok);
    assert_eq!(
        checkpoint_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("export_set")
            .and_then(Value::as_str),
        Some("all")
    );

    let checkpoint_initializer = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.initializer.describe",
        &json!({"init_mode": "lazy"}),
    );
    assert!(checkpoint_initializer.ok);
    assert_eq!(
        checkpoint_initializer
            .payload
            .unwrap_or_else(|| json!({}))
            .get("init_mode")
            .and_then(Value::as_str),
        Some("lazy")
    );

    let checkpoint_types = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.types.describe",
        &json!({"type_set": "core"}),
    );
    assert!(checkpoint_types.ok);
    assert_eq!(
        checkpoint_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type_set")
            .and_then(Value::as_str),
        Some("core")
    );

    let claude_message_filter = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.claudeCode.messageFilter.describe",
        &json!({"filter_mode": "strict"}),
    );
    assert!(claude_message_filter.ok);
    assert_eq!(
        claude_message_filter
            .payload
            .unwrap_or_else(|| json!({}))
            .get("filter_mode")
            .and_then(Value::as_str),
        Some("strict")
    );

    let claude_run = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.claudeCode.run.describe",
        &json!({"run_mode": "interactive"}),
    );
    assert!(claude_run.ok);
    assert_eq!(
        claude_run
            .payload
            .unwrap_or_else(|| json!({}))
            .get("run_mode")
            .and_then(Value::as_str),
        Some("interactive")
    );

    let claude_types = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.claudeCode.types.describe",
        &json!({"schema": "default"}),
    );
    assert!(claude_types.ok);
    assert_eq!(
        claude_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("schema")
            .and_then(Value::as_str),
        Some("default")
    );

    let diagnostics_index = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.diagnostics.index.describe",
        &json!({"scope": "workspace"}),
    );
    assert!(diagnostics_index.ok);
    assert_eq!(
        diagnostics_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("workspace")
    );
}
