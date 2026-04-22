#[test]
fn dashboard_system_prompt_terminal_checkpoint_tail_routes_contract_wave_870() {
    let root = tempfile::tempdir().expect("tempdir");

    let terminal_process = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.terminal.vscodeTerminalProcess.describe",
        &json!({"lifecycle": "managed"}),
    );
    assert!(terminal_process.ok);
    assert_eq!(
        terminal_process
            .payload
            .unwrap_or_else(|| json!({}))
            .get("lifecycle")
            .and_then(Value::as_str),
        Some("managed")
    );

    let terminal_registry = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.terminal.vscodeTerminalRegistry.describe",
        &json!({"scope": "workspace"}),
    );
    assert!(terminal_registry.ok);
    assert_eq!(
        terminal_registry
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let ansi_utils = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.terminal.ansiUtils.describe",
        &json!({"mode": "sanitize"}),
    );
    assert!(ansi_utils.ok);
    assert_eq!(
        ansi_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("sanitize")
    );

    let latest_output = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.terminal.getLatestOutput.describe",
        &json!({"max_lines": 128}),
    );
    assert!(latest_output.ok);
    assert_eq!(
        latest_output
            .payload
            .unwrap_or_else(|| json!({}))
            .get("max_lines")
            .and_then(Value::as_u64),
        Some(128)
    );

    let file_migration = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.vscodeToFileMigration.describe",
        &json!({"migration_mode": "safe"}),
    );
    assert!(file_migration.ok);
    assert_eq!(
        file_migration
            .payload
            .unwrap_or_else(|| json!({}))
            .get("migration_mode")
            .and_then(Value::as_str),
        Some("safe")
    );

    let checkpoint_exclusions = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.checkpointExclusions.describe",
        &json!({"policy": "default"}),
    );
    assert!(checkpoint_exclusions.ok);
    assert_eq!(
        checkpoint_exclusions
            .payload
            .unwrap_or_else(|| json!({}))
            .get("policy")
            .and_then(Value::as_str),
        Some("default")
    );

    let checkpoint_git_ops = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.checkpointGitOperations.describe",
        &json!({"operation": "status"}),
    );
    assert!(checkpoint_git_ops.ok);
    assert_eq!(
        checkpoint_git_ops
            .payload
            .unwrap_or_else(|| json!({}))
            .get("operation")
            .and_then(Value::as_str),
        Some("status")
    );

    let checkpoint_lock_utils = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.checkpointLockUtils.describe",
        &json!({"lock_mode": "advisory"}),
    );
    assert!(checkpoint_lock_utils.ok);
    assert_eq!(
        checkpoint_lock_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("lock_mode")
            .and_then(Value::as_str),
        Some("advisory")
    );

    let checkpoint_migration = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.checkpointMigration.describe",
        &json!({"migration_stage": "plan"}),
    );
    assert!(checkpoint_migration.ok);
    assert_eq!(
        checkpoint_migration
            .payload
            .unwrap_or_else(|| json!({}))
            .get("migration_stage")
            .and_then(Value::as_str),
        Some("plan")
    );

    let checkpoint_tracker = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.checkpointTracker.describe",
        &json!({"tracker_scope": "workspace"}),
    );
    assert!(checkpoint_tracker.ok);
    assert_eq!(
        checkpoint_tracker
            .payload
            .unwrap_or_else(|| json!({}))
            .get("tracker_scope")
            .and_then(Value::as_str),
        Some("workspace")
    );
}
