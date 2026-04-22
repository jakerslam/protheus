#[test]
fn dashboard_system_prompt_hostbridge_window_workspace_tail_routes_contract_wave_850() {
    let root = tempfile::tempdir().expect("tempdir");

    let open_settings = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.openSettings.describe",
        &json!({"section": "workbench.editor"}),
    );
    assert!(open_settings.ok);
    assert_eq!(
        open_settings
            .payload
            .unwrap_or_else(|| json!({}))
            .get("section")
            .and_then(Value::as_str),
        Some("workbench.editor")
    );

    let show_input_box = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.showInputBox.describe",
        &json!({"prompt": "Name", "placeholder": "Type here"}),
    );
    assert!(show_input_box.ok);
    assert_eq!(
        show_input_box
            .payload
            .unwrap_or_else(|| json!({}))
            .get("placeholder")
            .and_then(Value::as_str),
        Some("Type here")
    );

    let show_message = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.showMessage.describe",
        &json!({"level": "info", "message": "Hello"}),
    );
    assert!(show_message.ok);
    assert_eq!(
        show_message
            .payload
            .unwrap_or_else(|| json!({}))
            .get("level")
            .and_then(Value::as_str),
        Some("info")
    );

    let show_open_dialogue = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.showOpenDialogue.describe",
        &json!({"can_select_many": true}),
    );
    assert!(show_open_dialogue.ok);
    assert_eq!(
        show_open_dialogue
            .payload
            .unwrap_or_else(|| json!({}))
            .get("can_select_many")
            .and_then(Value::as_bool),
        Some(true)
    );

    let show_save_dialog = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.showSaveDialog.describe",
        &json!({"default_uri": "file:///tmp/a.txt"}),
    );
    assert!(show_save_dialog.ok);
    assert_eq!(
        show_save_dialog
            .payload
            .unwrap_or_else(|| json!({}))
            .get("default_uri")
            .and_then(Value::as_str),
        Some("file:///tmp/a.txt")
    );

    let show_text_document = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.showTextDocument.describe",
        &json!({"uri": "file:///tmp/a.ts", "preserve_focus": true}),
    );
    assert!(show_text_document.ok);
    assert_eq!(
        show_text_document
            .payload
            .unwrap_or_else(|| json!({}))
            .get("preserve_focus")
            .and_then(Value::as_bool),
        Some(true)
    );

    let execute_terminal = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.executeCommandInTerminal.describe",
        &json!({"command": "npm test", "terminal_name": "main"}),
    );
    assert!(execute_terminal.ok);
    assert_eq!(
        execute_terminal
            .payload
            .unwrap_or_else(|| json!({}))
            .get("terminal_name")
            .and_then(Value::as_str),
        Some("main")
    );

    let diagnostics_test = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.getDiagnosticsTest.describe",
        &json!({"test_profile": "contract"}),
    );
    assert!(diagnostics_test.ok);
    assert_eq!(
        diagnostics_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("test_profile")
            .and_then(Value::as_str),
        Some("contract")
    );

    let diagnostics = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.getDiagnostics.describe",
        &json!({"severity": "all"}),
    );
    assert!(diagnostics.ok);
    assert_eq!(
        diagnostics
            .payload
            .unwrap_or_else(|| json!({}))
            .get("severity")
            .and_then(Value::as_str),
        Some("all")
    );

    let workspace_paths = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.getWorkspacePaths.describe",
        &json!({"include_virtual": false}),
    );
    assert!(workspace_paths.ok);
    assert_eq!(
        workspace_paths
            .payload
            .unwrap_or_else(|| json!({}))
            .get("include_virtual")
            .and_then(Value::as_bool),
        Some(false)
    );
}
