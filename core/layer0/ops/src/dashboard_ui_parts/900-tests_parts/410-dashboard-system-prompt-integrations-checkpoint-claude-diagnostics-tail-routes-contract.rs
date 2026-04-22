#[test]
fn dashboard_system_prompt_integrations_checkpoint_claude_diagnostics_tail_routes_contract_wave_410()
{
    let root = tempfile::tempdir().expect("tempdir");

    let checkpoint_utils = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.checkpointUtils.describe",
        &json!({"operation": "normalize"}),
    );
    assert!(checkpoint_utils.ok);
    assert_eq!(
        checkpoint_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("operation")
            .and_then(Value::as_str),
        Some("normalize")
    );

    let multi_root = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.multiRootCheckpointManager.describe",
        &json!({"workspace_count": 3}),
    );
    assert!(multi_root.ok);
    assert_eq!(
        multi_root
            .payload
            .unwrap_or_else(|| json!({}))
            .get("workspace_count")
            .and_then(Value::as_u64),
        Some(3)
    );

    let factory = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.factory.describe",
        &json!({"profile": "default"}),
    );
    assert!(factory.ok);
    assert_eq!(
        factory
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("default")
    );

    let index = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.index.describe",
        &json!({}),
    );
    assert!(index.ok);
    assert_eq!(
        index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_integrations_checkpoint_index_describe")
    );

    let initializer = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.initializer.describe",
        &json!({"boot_mode": "lazy"}),
    );
    assert!(initializer.ok);
    assert_eq!(
        initializer
            .payload
            .unwrap_or_else(|| json!({}))
            .get("boot_mode")
            .and_then(Value::as_str),
        Some("lazy")
    );

    let types = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.types.describe",
        &json!({}),
    );
    assert!(types.ok);
    assert_eq!(
        types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_integrations_checkpoint_types_describe")
    );

    let claude_filter = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.claudeCode.messageFilter.describe",
        &json!({"policy": "safe"}),
    );
    assert!(claude_filter.ok);
    assert_eq!(
        claude_filter
            .payload
            .unwrap_or_else(|| json!({}))
            .get("policy")
            .and_then(Value::as_str),
        Some("safe")
    );

    let claude_run = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.claudeCode.run.describe",
        &json!({"mode": "interactive"}),
    );
    assert!(claude_run.ok);
    assert_eq!(
        claude_run
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("interactive")
    );

    let claude_types = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.claudeCode.types.describe",
        &json!({}),
    );
    assert!(claude_types.ok);
    assert_eq!(
        claude_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_integrations_claude_types_describe")
    );

    let diagnostics = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.diagnostics.index.describe",
        &json!({"detail": "summary"}),
    );
    assert!(diagnostics.ok);
    assert_eq!(
        diagnostics
            .payload
            .unwrap_or_else(|| json!({}))
            .get("detail")
            .and_then(Value::as_str),
        Some("summary")
    );
}
