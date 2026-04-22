#[test]
fn dashboard_system_prompt_controller_ui_event_tail_routes_contract_wave_640() {
    let root = tempfile::tempdir().expect("tempdir");

    let initialize_webview = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.initializeWebview.describe",
        &json!({"mode": "initialize"}),
    );
    assert!(initialize_webview.ok);
    assert_eq!(
        initialize_webview
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("initialize")
    );

    let did_show_announcement = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.onDidShowAnnouncement.describe",
        &json!({"announcement": "default"}),
    );
    assert!(did_show_announcement.ok);
    assert_eq!(
        did_show_announcement
            .payload
            .unwrap_or_else(|| json!({}))
            .get("announcement")
            .and_then(Value::as_str),
        Some("default")
    );

    let open_url = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.openUrl.describe",
        &json!({"url": "https://example.invalid"}),
    );
    assert!(open_url.ok);
    assert_eq!(
        open_url
            .payload
            .unwrap_or_else(|| json!({}))
            .get("url")
            .and_then(Value::as_str),
        Some("https://example.invalid")
    );

    let open_walkthrough = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.openWalkthrough.describe",
        &json!({"topic": "intro"}),
    );
    assert!(open_walkthrough.ok);
    assert_eq!(
        open_walkthrough
            .payload
            .unwrap_or_else(|| json!({}))
            .get("topic")
            .and_then(Value::as_str),
        Some("intro")
    );

    let scroll_to_settings = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.scrollToSettings.describe",
        &json!({"section": "general"}),
    );
    assert!(scroll_to_settings.ok);
    assert_eq!(
        scroll_to_settings
            .payload
            .unwrap_or_else(|| json!({}))
            .get("section")
            .and_then(Value::as_str),
        Some("general")
    );

    let set_terminal_execution_mode = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.setTerminalExecutionMode.describe",
        &json!({"terminal_mode": "safe"}),
    );
    assert!(set_terminal_execution_mode.ok);
    assert_eq!(
        set_terminal_execution_mode
            .payload
            .unwrap_or_else(|| json!({}))
            .get("terminal_mode")
            .and_then(Value::as_str),
        Some("safe")
    );

    let subscribe_account_button = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.subscribeToAccountButtonClicked.describe",
        &json!({"action": "open_account"}),
    );
    assert!(subscribe_account_button.ok);
    assert_eq!(
        subscribe_account_button
            .payload
            .unwrap_or_else(|| json!({}))
            .get("action")
            .and_then(Value::as_str),
        Some("open_account")
    );

    let subscribe_add_to_input = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.subscribeToAddToInput.describe",
        &json!({"source": "selection"}),
    );
    assert!(subscribe_add_to_input.ok);
    assert_eq!(
        subscribe_add_to_input
            .payload
            .unwrap_or_else(|| json!({}))
            .get("source")
            .and_then(Value::as_str),
        Some("selection")
    );

    let subscribe_chat_button = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.subscribeToChatButtonClicked.describe",
        &json!({"target": "chat"}),
    );
    assert!(subscribe_chat_button.ok);
    assert_eq!(
        subscribe_chat_button
            .payload
            .unwrap_or_else(|| json!({}))
            .get("target")
            .and_then(Value::as_str),
        Some("chat")
    );

    let subscribe_history_button = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.subscribeToHistoryButtonClicked.describe",
        &json!({"history_mode": "recent"}),
    );
    assert!(subscribe_history_button.ok);
    assert_eq!(
        subscribe_history_button
            .payload
            .unwrap_or_else(|| json!({}))
            .get("history_mode")
            .and_then(Value::as_str),
        Some("recent")
    );
}
