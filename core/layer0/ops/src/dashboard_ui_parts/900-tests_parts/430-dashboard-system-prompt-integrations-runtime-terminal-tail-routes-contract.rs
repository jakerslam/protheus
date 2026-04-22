#[test]
fn dashboard_system_prompt_integrations_runtime_terminal_tail_routes_contract_wave_430() {
    let root = tempfile::tempdir().expect("tempdir");

    let open_file = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.openFile.describe",
        &json!({"path": "/tmp/a.txt", "focus": true}),
    );
    assert!(open_file.ok);
    let open_file_payload = open_file.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        open_file_payload.get("path").and_then(Value::as_str),
        Some("/tmp/a.txt")
    );
    assert_eq!(
        open_file_payload.get("focus").and_then(Value::as_bool),
        Some(true)
    );

    let process_files = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.misc.processFiles.describe",
        &json!({"mode": "batch", "file_count": 4}),
    );
    assert!(process_files.ok);
    let process_files_payload = process_files.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        process_files_payload.get("mode").and_then(Value::as_str),
        Some("batch")
    );
    assert_eq!(
        process_files_payload
            .get("file_count")
            .and_then(Value::as_u64),
        Some(4)
    );

    let notifications = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.notifications.index.describe",
        &json!({"channel": "runtime"}),
    );
    assert!(notifications.ok);
    assert_eq!(
        notifications
            .payload
            .unwrap_or_else(|| json!({}))
            .get("channel")
            .and_then(Value::as_str),
        Some("runtime")
    );

    let oauth = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.openaiCodex.oauth.describe",
        &json!({"provider": "openai"}),
    );
    assert!(oauth.ok);
    assert_eq!(
        oauth
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("openai")
    );

    let command_executor = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.commandExecutor.describe",
        &json!({"command": "echo hi"}),
    );
    assert!(command_executor.ok);
    assert_eq!(
        command_executor
            .payload
            .unwrap_or_else(|| json!({}))
            .get("command")
            .and_then(Value::as_str),
        Some("echo hi")
    );

    let orchestrator = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.commandOrchestrator.describe",
        &json!({"strategy": "serial"}),
    );
    assert!(orchestrator.ok);
    assert_eq!(
        orchestrator
            .payload
            .unwrap_or_else(|| json!({}))
            .get("strategy")
            .and_then(Value::as_str),
        Some("serial")
    );

    let constants = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.constants.describe",
        &json!({}),
    );
    assert!(constants.ok);
    assert_eq!(
        constants
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_integrations_terminal_constants_describe")
    );

    let terminal_index = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.index.describe",
        &json!({}),
    );
    assert!(terminal_index.ok);
    assert_eq!(
        terminal_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_integrations_terminal_index_describe")
    );

    let standalone_terminal = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.standaloneTerminal.describe",
        &json!({"terminal_id": "st-1"}),
    );
    assert!(standalone_terminal.ok);
    assert_eq!(
        standalone_terminal
            .payload
            .unwrap_or_else(|| json!({}))
            .get("terminal_id")
            .and_then(Value::as_str),
        Some("st-1")
    );

    let standalone_manager = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.standaloneTerminalManager.describe",
        &json!({"scope": "workspace"}),
    );
    assert!(standalone_manager.ok);
    assert_eq!(
        standalone_manager
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("workspace")
    );
}
