#[test]
fn dashboard_system_prompt_controller_ui_web_worktree_tail_routes_contract_wave_650() {
    let root = tempfile::tempdir().expect("tempdir");

    let subscribe_mcp = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.subscribeToMcpButtonClicked.describe",
        &json!({"target": "mcp"}),
    );
    assert!(subscribe_mcp.ok);
    assert_eq!(
        subscribe_mcp
            .payload
            .unwrap_or_else(|| json!({}))
            .get("target")
            .and_then(Value::as_str),
        Some("mcp")
    );

    let subscribe_partial = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.subscribeToPartialMessage.describe",
        &json!({"mode": "stream"}),
    );
    assert!(subscribe_partial.ok);
    assert_eq!(
        subscribe_partial
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("stream")
    );

    let subscribe_relinquish = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.subscribeToRelinquishControl.describe",
        &json!({"authority": "operator"}),
    );
    assert!(subscribe_relinquish.ok);
    assert_eq!(
        subscribe_relinquish
            .payload
            .unwrap_or_else(|| json!({}))
            .get("authority")
            .and_then(Value::as_str),
        Some("operator")
    );

    let subscribe_settings = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.subscribeToSettingsButtonClicked.describe",
        &json!({"pane": "settings"}),
    );
    assert!(subscribe_settings.ok);
    assert_eq!(
        subscribe_settings
            .payload
            .unwrap_or_else(|| json!({}))
            .get("pane")
            .and_then(Value::as_str),
        Some("settings")
    );

    let subscribe_show_webview = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.subscribeToShowWebview.describe",
        &json!({"view": "main"}),
    );
    assert!(subscribe_show_webview.ok);
    assert_eq!(
        subscribe_show_webview
            .payload
            .unwrap_or_else(|| json!({}))
            .get("view")
            .and_then(Value::as_str),
        Some("main")
    );

    let subscribe_worktrees = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.subscribeToWorktreesButtonClicked.describe",
        &json!({"workspace": "current"}),
    );
    assert!(subscribe_worktrees.ok);
    assert_eq!(
        subscribe_worktrees
            .payload
            .unwrap_or_else(|| json!({}))
            .get("workspace")
            .and_then(Value::as_str),
        Some("current")
    );

    let check_is_image_url = run_action(
        root.path(),
        "dashboard.prompts.system.controller.web.checkIsImageUrl.describe",
        &json!({"scheme": "https"}),
    );
    assert!(check_is_image_url.ok);
    assert_eq!(
        check_is_image_url
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scheme")
            .and_then(Value::as_str),
        Some("https")
    );

    let fetch_open_graph_data = run_action(
        root.path(),
        "dashboard.prompts.system.controller.web.fetchOpenGraphData.describe",
        &json!({"source": "opengraph"}),
    );
    assert!(fetch_open_graph_data.ok);
    assert_eq!(
        fetch_open_graph_data
            .payload
            .unwrap_or_else(|| json!({}))
            .get("source")
            .and_then(Value::as_str),
        Some("opengraph")
    );

    let open_in_browser = run_action(
        root.path(),
        "dashboard.prompts.system.controller.web.openInBrowser.describe",
        &json!({"browser": "default"}),
    );
    assert!(open_in_browser.ok);
    assert_eq!(
        open_in_browser
            .payload
            .unwrap_or_else(|| json!({}))
            .get("browser")
            .and_then(Value::as_str),
        Some("default")
    );

    let checkout_branch = run_action(
        root.path(),
        "dashboard.prompts.system.controller.worktree.checkoutBranch.describe",
        &json!({"branch": "main"}),
    );
    assert!(checkout_branch.ok);
    assert_eq!(
        checkout_branch
            .payload
            .unwrap_or_else(|| json!({}))
            .get("branch")
            .and_then(Value::as_str),
        Some("main")
    );
}
