#[test]
fn dashboard_system_prompt_integrations_runtime_terminal_tail_routes_contract_wave_900() {
    let root = tempfile::tempdir().expect("tempdir");

    let open_file = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.openFile.describe",
        &json!({"path": "/tmp/a.txt", "focus": true}),
    );
    assert!(open_file.ok);
    assert_eq!(
        open_file
            .payload
            .unwrap_or_else(|| json!({}))
            .get("focus")
            .and_then(Value::as_bool),
        Some(true)
    );

    let process_files = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.processFiles.describe",
        &json!({"mode": "batch", "files": ["a.txt", "b.txt"]}),
    );
    assert!(process_files.ok);
    assert_eq!(
        process_files
            .payload
            .unwrap_or_else(|| json!({}))
            .get("file_count")
            .and_then(Value::as_i64),
        Some(2)
    );

    let notifications_index = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.notifications.index.describe",
        &json!({"channel": "default"}),
    );
    assert!(notifications_index.ok);
    assert_eq!(
        notifications_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("channel")
            .and_then(Value::as_str),
        Some("default")
    );

    let openai_oauth = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.openaiCodex.oauth.describe",
        &json!({"scope": "default"}),
    );
    assert!(openai_oauth.ok);
    assert_eq!(
        openai_oauth
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("default")
    );

    let command_executor = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.commandExecutor.describe",
        &json!({"command": "echo hi", "shell": "default"}),
    );
    assert!(command_executor.ok);
    assert_eq!(
        command_executor
            .payload
            .unwrap_or_else(|| json!({}))
            .get("shell")
            .and_then(Value::as_str),
        Some("default")
    );

    let command_orchestrator = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.commandOrchestrator.describe",
        &json!({"strategy": "sequential"}),
    );
    assert!(command_orchestrator.ok);
    assert_eq!(
        command_orchestrator
            .payload
            .unwrap_or_else(|| json!({}))
            .get("strategy")
            .and_then(Value::as_str),
        Some("sequential")
    );

    let terminal_constants = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.constants.describe",
        &json!({"profile": "default"}),
    );
    assert!(terminal_constants.ok);
    assert_eq!(
        terminal_constants
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("default")
    );

    let terminal_index = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.index.describe",
        &json!({"export_set": "all"}),
    );
    assert!(terminal_index.ok);
    assert_eq!(
        terminal_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("export_set")
            .and_then(Value::as_str),
        Some("all")
    );

    let standalone_terminal = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.standalone.standaloneTerminal.describe",
        &json!({"mode": "standalone"}),
    );
    assert!(standalone_terminal.ok);
    assert_eq!(
        standalone_terminal
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("standalone")
    );

    let standalone_terminal_manager = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.standalone.standaloneTerminalManager.describe",
        &json!({"manager_mode": "managed"}),
    );
    assert!(standalone_terminal_manager.ok);
    assert_eq!(
        standalone_terminal_manager
            .payload
            .unwrap_or_else(|| json!({}))
            .get("manager_mode")
            .and_then(Value::as_str),
        Some("managed")
    );
}
