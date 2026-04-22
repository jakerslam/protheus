#[test]
fn dashboard_system_prompt_hostbridge_env_window_tail_routes_contract_wave_840() {
    let root = tempfile::tempdir().expect("tempdir");

    let open_external = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.openExternal.describe",
        &json!({"target": "https://example.com"}),
    );
    assert!(open_external.ok);
    assert_eq!(
        open_external
            .payload
            .unwrap_or_else(|| json!({}))
            .get("target")
            .and_then(Value::as_str),
        Some("https://example.com")
    );

    let shutdown = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.shutdown.describe",
        &json!({"shutdown_mode": "graceful"}),
    );
    assert!(shutdown.ok);
    assert_eq!(
        shutdown
            .payload
            .unwrap_or_else(|| json!({}))
            .get("shutdown_mode")
            .and_then(Value::as_str),
        Some("graceful")
    );

    let subscribe = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.subscribeToTelemetrySettings.describe",
        &json!({"stream": "settings"}),
    );
    assert!(subscribe.ok);
    assert_eq!(
        subscribe
            .payload
            .unwrap_or_else(|| json!({}))
            .get("stream")
            .and_then(Value::as_str),
        Some("settings")
    );

    let webview_html = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.testing.getWebviewHtml.describe",
        &json!({"view_id": "main"}),
    );
    assert!(webview_html.ok);
    assert_eq!(
        webview_html
            .payload
            .unwrap_or_else(|| json!({}))
            .get("view_id")
            .and_then(Value::as_str),
        Some("main")
    );

    let active_editor = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.getActiveEditor.describe",
        &json!({"include_uri": true}),
    );
    assert!(active_editor.ok);
    assert_eq!(
        active_editor
            .payload
            .unwrap_or_else(|| json!({}))
            .get("include_uri")
            .and_then(Value::as_bool),
        Some(true)
    );

    let open_tabs_test = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.getOpenTabsTest.describe",
        &json!({"test_profile": "contract"}),
    );
    assert!(open_tabs_test.ok);
    assert_eq!(
        open_tabs_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("test_profile")
            .and_then(Value::as_str),
        Some("contract")
    );

    let open_tabs = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.getOpenTabs.describe",
        &json!({"group": "all"}),
    );
    assert!(open_tabs.ok);
    assert_eq!(
        open_tabs
            .payload
            .unwrap_or_else(|| json!({}))
            .get("group")
            .and_then(Value::as_str),
        Some("all")
    );

    let visible_tabs_test = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.getVisibleTabsTest.describe",
        &json!({"test_profile": "contract"}),
    );
    assert!(visible_tabs_test.ok);
    assert_eq!(
        visible_tabs_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("test_profile")
            .and_then(Value::as_str),
        Some("contract")
    );

    let visible_tabs = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.getVisibleTabs.describe",
        &json!({"scope": "visible"}),
    );
    assert!(visible_tabs.ok);
    assert_eq!(
        visible_tabs
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("visible")
    );

    let open_file = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.openFile.describe",
        &json!({"path": "/tmp/a.ts", "preview": true}),
    );
    assert!(open_file.ok);
    assert_eq!(
        open_file
            .payload
            .unwrap_or_else(|| json!({}))
            .get("preview")
            .and_then(Value::as_bool),
        Some(true)
    );
}
