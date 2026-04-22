#[test]
fn dashboard_system_prompt_vscode_hostbridge_window_workspace_tail_routes_contract_wave_380() {
    let root = tempfile::tempdir().expect("tempdir");

    let open_settings = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.openSettings.describe",
        &json!({"query": "editor.fontSize"}),
    );
    assert!(open_settings.ok);
    assert_eq!(
        open_settings
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("editor.fontSize")
    );

    let show_input = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.showInputBox.describe",
        &json!({"prompt": "Name", "value": "workspace", "place_holder": "optional"}),
    );
    assert!(show_input.ok);
    let show_input_payload = show_input.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        show_input_payload.get("prompt").and_then(Value::as_str),
        Some("Name")
    );
    assert_eq!(
        show_input_payload.get("place_holder").and_then(Value::as_str),
        Some("optional")
    );

    let show_message = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.showMessage.describe",
        &json!({"level": "warning", "message": "Heads up", "modal": true}),
    );
    assert!(show_message.ok);
    let show_message_payload = show_message.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        show_message_payload.get("level").and_then(Value::as_str),
        Some("warning")
    );
    assert_eq!(
        show_message_payload.get("modal").and_then(Value::as_bool),
        Some(true)
    );

    let open_dialogue = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.showOpenDialogue.describe",
        &json!({"can_select_files": false, "can_select_folders": true}),
    );
    assert!(open_dialogue.ok);
    let open_dialogue_payload = open_dialogue.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        open_dialogue_payload
            .get("can_select_files")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        open_dialogue_payload
            .get("can_select_folders")
            .and_then(Value::as_bool),
        Some(true)
    );

    let save_dialog = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.showSaveDialog.describe",
        &json!({"default_uri": "/tmp/out.md"}),
    );
    assert!(save_dialog.ok);
    assert_eq!(
        save_dialog
            .payload
            .unwrap_or_else(|| json!({}))
            .get("default_uri")
            .and_then(Value::as_str),
        Some("/tmp/out.md")
    );

    let show_text_doc = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.showTextDocument.describe",
        &json!({"uri": "file:///tmp/a.md", "preview": false}),
    );
    assert!(show_text_doc.ok);
    let show_text_doc_payload = show_text_doc.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        show_text_doc_payload.get("uri").and_then(Value::as_str),
        Some("file:///tmp/a.md")
    );
    assert_eq!(
        show_text_doc_payload.get("preview").and_then(Value::as_bool),
        Some(false)
    );

    let exec_terminal = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.executeCommandInTerminal.describe",
        &json!({"command": "cargo check", "cwd": "/tmp/proj"}),
    );
    assert!(exec_terminal.ok);
    let exec_terminal_payload = exec_terminal.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        exec_terminal_payload.get("command").and_then(Value::as_str),
        Some("cargo check")
    );
    assert_eq!(
        exec_terminal_payload.get("cwd").and_then(Value::as_str),
        Some("/tmp/proj")
    );

    let diagnostics_test = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.getDiagnosticsTest.describe",
        &json!({}),
    );
    assert!(diagnostics_test.ok);
    assert_eq!(
        diagnostics_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("test_fixture")
    );

    let diagnostics = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.getDiagnostics.describe",
        &json!({"uri": "file:///tmp/main.rs", "severity_min": "error"}),
    );
    assert!(diagnostics.ok);
    let diagnostics_payload = diagnostics.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        diagnostics_payload.get("uri").and_then(Value::as_str),
        Some("file:///tmp/main.rs")
    );
    assert_eq!(
        diagnostics_payload
            .get("severity_min")
            .and_then(Value::as_str),
        Some("error")
    );

    let workspace_paths = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.getWorkspacePaths.describe",
        &json!({}),
    );
    assert!(workspace_paths.ok);
    assert_eq!(
        workspace_paths
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_workspace_get_workspace_paths_describe")
    );
}
