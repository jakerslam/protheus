#[test]
fn dashboard_system_prompt_vscode_hostbridge_workspace_review_terminal_tail_routes_contract_wave_390() {
    let root = tempfile::tempdir().expect("tempdir");

    let open_sidebar = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.openClineSidebarPanel.describe",
        &json!({}),
    );
    assert!(open_sidebar.ok);
    assert_eq!(
        open_sidebar
            .payload
            .unwrap_or_else(|| json!({}))
            .get("panel")
            .and_then(Value::as_str),
        Some("cline_sidebar")
    );

    let open_folder = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.openFolder.describe",
        &json!({"folder_path": "/tmp/proj", "force_new_window": true}),
    );
    assert!(open_folder.ok);
    let open_folder_payload = open_folder.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        open_folder_payload.get("folder_path").and_then(Value::as_str),
        Some("/tmp/proj")
    );
    assert_eq!(
        open_folder_payload
            .get("force_new_window")
            .and_then(Value::as_bool),
        Some(true)
    );

    let file_explorer = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.openInFileExplorerPanel.describe",
        &json!({}),
    );
    assert!(file_explorer.ok);
    assert_eq!(
        file_explorer
            .payload
            .unwrap_or_else(|| json!({}))
            .get("panel")
            .and_then(Value::as_str),
        Some("file_explorer")
    );

    let problems_panel = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.openProblemsPanel.describe",
        &json!({}),
    );
    assert!(problems_panel.ok);
    assert_eq!(
        problems_panel
            .payload
            .unwrap_or_else(|| json!({}))
            .get("panel")
            .and_then(Value::as_str),
        Some("problems")
    );

    let terminal_panel = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.openTerminalPanel.describe",
        &json!({}),
    );
    assert!(terminal_panel.ok);
    assert_eq!(
        terminal_panel
            .payload
            .unwrap_or_else(|| json!({}))
            .get("panel")
            .and_then(Value::as_str),
        Some("terminal")
    );

    let save_dirty_test = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.saveOpenDocumentIfDirtyTest.describe",
        &json!({}),
    );
    assert!(save_dirty_test.ok);
    assert_eq!(
        save_dirty_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("test_fixture")
    );

    let save_dirty = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.saveOpenDocumentIfDirty.describe",
        &json!({"uri": "file:///tmp/a.rs", "save_if_dirty": true}),
    );
    assert!(save_dirty.ok);
    let save_dirty_payload = save_dirty.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        save_dirty_payload.get("uri").and_then(Value::as_str),
        Some("file:///tmp/a.rs")
    );
    assert_eq!(
        save_dirty_payload
            .get("save_if_dirty")
            .and_then(Value::as_bool),
        Some(true)
    );

    let comment_review = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.review.vscodeCommentReviewController.describe",
        &json!({}),
    );
    assert!(comment_review.ok);
    assert_eq!(
        comment_review
            .payload
            .unwrap_or_else(|| json!({}))
            .get("controller")
            .and_then(Value::as_str),
        Some("comment_review")
    );

    let terminal_manager = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.terminal.vscodeTerminalManager.describe",
        &json!({"terminal_id": "term-1"}),
    );
    assert!(terminal_manager.ok);
    assert_eq!(
        terminal_manager
            .payload
            .unwrap_or_else(|| json!({}))
            .get("terminal_id")
            .and_then(Value::as_str),
        Some("term-1")
    );

    let terminal_process_test = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.terminal.vscodeTerminalProcessTest.describe",
        &json!({}),
    );
    assert!(terminal_process_test.ok);
    assert_eq!(
        terminal_process_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("test_fixture")
    );
}
