#[test]
fn dashboard_system_prompt_vscode_hostbridge_env_window_tail_routes_contract_wave_370() {
    let root = tempfile::tempdir().expect("tempdir");

    let open_external = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.openExternal.describe",
        &json!({"url": "https://example.com"}),
    );
    assert!(open_external.ok);
    assert_eq!(
        open_external
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_env_open_external_describe")
    );

    let shutdown = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.shutdown.describe",
        &json!({"reason": "maintenance"}),
    );
    assert!(shutdown.ok);
    assert_eq!(
        shutdown
            .payload
            .unwrap_or_else(|| json!({}))
            .get("reason")
            .and_then(Value::as_str),
        Some("maintenance")
    );

    let telemetry_sub = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.subscribeToTelemetrySettings.describe",
        &json!({}),
    );
    assert!(telemetry_sub.ok);
    assert_eq!(
        telemetry_sub
            .payload
            .unwrap_or_else(|| json!({}))
            .get("operation")
            .and_then(Value::as_str),
        Some("subscribe_telemetry_settings")
    );

    let webview_html = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.testing.getWebviewHtml.describe",
        &json!({"panel_id": "chat"}),
    );
    assert!(webview_html.ok);
    assert_eq!(
        webview_html
            .payload
            .unwrap_or_else(|| json!({}))
            .get("panel_id")
            .and_then(Value::as_str),
        Some("chat")
    );

    let active_editor = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.getActiveEditor.describe",
        &json!({}),
    );
    assert!(active_editor.ok);
    assert_eq!(
        active_editor
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_window_get_active_editor_describe")
    );

    let open_tabs_test = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.getOpenTabsTest.describe",
        &json!({}),
    );
    assert!(open_tabs_test.ok);
    assert_eq!(
        open_tabs_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("test_fixture")
    );

    let open_tabs = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.getOpenTabs.describe",
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
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.getVisibleTabsTest.describe",
        &json!({}),
    );
    assert!(visible_tabs_test.ok);
    assert_eq!(
        visible_tabs_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("test_fixture")
    );

    let visible_tabs = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.getVisibleTabs.describe",
        &json!({"editor_group": "active"}),
    );
    assert!(visible_tabs.ok);
    assert_eq!(
        visible_tabs
            .payload
            .unwrap_or_else(|| json!({}))
            .get("editor_group")
            .and_then(Value::as_str),
        Some("active")
    );

    let open_file = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.openFile.describe",
        &json!({"path": "/tmp/a.ts", "preview": false}),
    );
    assert!(open_file.ok);
    let open_file_payload = open_file.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        open_file_payload.get("path").and_then(Value::as_str),
        Some("/tmp/a.ts")
    );
    assert_eq!(
        open_file_payload.get("preview").and_then(Value::as_bool),
        Some(false)
    );
}
