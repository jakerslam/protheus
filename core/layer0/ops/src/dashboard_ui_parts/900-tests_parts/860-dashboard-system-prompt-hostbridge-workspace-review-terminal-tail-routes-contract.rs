#[test]
fn dashboard_system_prompt_hostbridge_workspace_review_terminal_tail_routes_contract_wave_860() {
    let root = tempfile::tempdir().expect("tempdir");

    let open_sidebar = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.openClineSidebarPanel.describe",
        &json!({"focus": true}),
    );
    assert!(open_sidebar.ok);
    assert_eq!(
        open_sidebar
            .payload
            .unwrap_or_else(|| json!({}))
            .get("focus")
            .and_then(Value::as_bool),
        Some(true)
    );

    let open_folder = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.openFolder.describe",
        &json!({"uri": "file:///tmp/project", "force_new_window": false}),
    );
    assert!(open_folder.ok);
    assert_eq!(
        open_folder
            .payload
            .unwrap_or_else(|| json!({}))
            .get("force_new_window")
            .and_then(Value::as_bool),
        Some(false)
    );

    let open_explorer = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.openInFileExplorerPanel.describe",
        &json!({"reveal": true}),
    );
    assert!(open_explorer.ok);
    assert_eq!(
        open_explorer
            .payload
            .unwrap_or_else(|| json!({}))
            .get("reveal")
            .and_then(Value::as_bool),
        Some(true)
    );

    let open_problems = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.openProblemsPanel.describe",
        &json!({"auto_focus": true}),
    );
    assert!(open_problems.ok);
    assert_eq!(
        open_problems
            .payload
            .unwrap_or_else(|| json!({}))
            .get("auto_focus")
            .and_then(Value::as_bool),
        Some(true)
    );

    let open_terminal = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.openTerminalPanel.describe",
        &json!({"panel": "terminal"}),
    );
    assert!(open_terminal.ok);
    assert_eq!(
        open_terminal
            .payload
            .unwrap_or_else(|| json!({}))
            .get("panel")
            .and_then(Value::as_str),
        Some("terminal")
    );

    let save_if_dirty_test = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.saveOpenDocumentIfDirtyTest.describe",
        &json!({"test_profile": "contract"}),
    );
    assert!(save_if_dirty_test.ok);
    assert_eq!(
        save_if_dirty_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("test_profile")
            .and_then(Value::as_str),
        Some("contract")
    );

    let save_if_dirty = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.saveOpenDocumentIfDirty.describe",
        &json!({"save_all": true}),
    );
    assert!(save_if_dirty.ok);
    assert_eq!(
        save_if_dirty
            .payload
            .unwrap_or_else(|| json!({}))
            .get("save_all")
            .and_then(Value::as_bool),
        Some(true)
    );

    let review_controller = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.review.vscodeCommentReviewController.describe",
        &json!({"mode": "inline"}),
    );
    assert!(review_controller.ok);
    assert_eq!(
        review_controller
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("inline")
    );

    let terminal_manager = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.terminal.vscodeTerminalManager.describe",
        &json!({"strategy": "managed"}),
    );
    assert!(terminal_manager.ok);
    assert_eq!(
        terminal_manager
            .payload
            .unwrap_or_else(|| json!({}))
            .get("strategy")
            .and_then(Value::as_str),
        Some("managed")
    );

    let terminal_process_test = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.terminal.vscodeTerminalProcessTest.describe",
        &json!({"test_profile": "contract"}),
    );
    assert!(terminal_process_test.ok);
    assert_eq!(
        terminal_process_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("test_profile")
            .and_then(Value::as_str),
        Some("contract")
    );
}
