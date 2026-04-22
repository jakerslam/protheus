#[test]
fn dashboard_system_prompt_vscode_terminal_checkpoint_tail_routes_contract_wave_400() {
    let root = tempfile::tempdir().expect("tempdir");

    let terminal_process = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.terminal.vscodeTerminalProcess.describe",
        &json!({"terminal_id": "term-a", "status": "idle"}),
    );
    assert!(terminal_process.ok);
    let terminal_process_payload = terminal_process.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        terminal_process_payload
            .get("terminal_id")
            .and_then(Value::as_str),
        Some("term-a")
    );
    assert_eq!(
        terminal_process_payload.get("status").and_then(Value::as_str),
        Some("idle")
    );

    let terminal_registry = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.terminal.vscodeTerminalRegistry.describe",
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
        "dashboard.prompts.system.hosts.vscode.terminal.ansiUtils.describe",
        &json!({"sample": "\\u001b[32mok\\u001b[0m"}),
    );
    assert!(ansi_utils.ok);
    assert_eq!(
        ansi_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("sample")
            .and_then(Value::as_str),
        Some("\\u001b[32mok\\u001b[0m")
    );

    let latest_output = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.terminal.getLatestOutput.describe",
        &json!({"terminal_id": "term-b", "max_chars": 512}),
    );
    assert!(latest_output.ok);
    let latest_output_payload = latest_output.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        latest_output_payload
            .get("terminal_id")
            .and_then(Value::as_str),
        Some("term-b")
    );
    assert_eq!(
        latest_output_payload.get("max_chars").and_then(Value::as_u64),
        Some(512)
    );

    let migration = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.vscodeToFileMigration.describe",
        &json!({"mode": "apply"}),
    );
    assert!(migration.ok);
    assert_eq!(
        migration
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("apply")
    );

    let checkpoint_exclusions = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.checkpointExclusions.describe",
        &json!({"profile": "strict"}),
    );
    assert!(checkpoint_exclusions.ok);
    assert_eq!(
        checkpoint_exclusions
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("strict")
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
        &json!({"lock_scope": "workspace"}),
    );
    assert!(checkpoint_lock_utils.ok);
    assert_eq!(
        checkpoint_lock_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("lock_scope")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let checkpoint_migration = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.checkpointMigration.describe",
        &json!({"strategy": "safe"}),
    );
    assert!(checkpoint_migration.ok);
    assert_eq!(
        checkpoint_migration
            .payload
            .unwrap_or_else(|| json!({}))
            .get("strategy")
            .and_then(Value::as_str),
        Some("safe")
    );

    let checkpoint_tracker = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.checkpoints.checkpointTracker.describe",
        &json!({"tracker_id": "trk-1"}),
    );
    assert!(checkpoint_tracker.ok);
    assert_eq!(
        checkpoint_tracker
            .payload
            .unwrap_or_else(|| json!({}))
            .get("tracker_id")
            .and_then(Value::as_str),
        Some("trk-1")
    );
}
