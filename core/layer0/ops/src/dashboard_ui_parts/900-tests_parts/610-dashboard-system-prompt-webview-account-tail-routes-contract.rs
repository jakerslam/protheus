#[test]
fn dashboard_system_prompt_webview_account_tail_routes_contract_wave_610() {
    let root = tempfile::tempdir().expect("tempdir");

    let app = run_action(
        root.path(),
        "dashboard.prompts.system.webview.app.describe",
        &json!({"mode": "chat"}),
    );
    assert!(app.ok);
    assert_eq!(
        app.payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("chat")
    );

    let custom_posthog_provider = run_action(
        root.path(),
        "dashboard.prompts.system.webview.customPostHogProvider.describe",
        &json!({"provider": "posthog"}),
    );
    assert!(custom_posthog_provider.ok);
    assert_eq!(
        custom_posthog_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("posthog")
    );

    let providers = run_action(
        root.path(),
        "dashboard.prompts.system.webview.providers.describe",
        &json!({"stack": "default"}),
    );
    assert!(providers.ok);
    assert_eq!(
        providers
            .payload
            .unwrap_or_else(|| json!({}))
            .get("stack")
            .and_then(Value::as_str),
        Some("default")
    );

    let account_view = run_action(
        root.path(),
        "dashboard.prompts.system.webview.account.accountView.describe",
        &json!({"tab": "profile"}),
    );
    assert!(account_view.ok);
    assert_eq!(
        account_view
            .payload
            .unwrap_or_else(|| json!({}))
            .get("tab")
            .and_then(Value::as_str),
        Some("profile")
    );

    let account_welcome_view = run_action(
        root.path(),
        "dashboard.prompts.system.webview.account.accountWelcomeView.describe",
        &json!({"state": "welcome"}),
    );
    assert!(account_welcome_view.ok);
    assert_eq!(
        account_welcome_view
            .payload
            .unwrap_or_else(|| json!({}))
            .get("state")
            .and_then(Value::as_str),
        Some("welcome")
    );

    let credit_balance = run_action(
        root.path(),
        "dashboard.prompts.system.webview.account.creditBalance.describe",
        &json!({"currency": "usd"}),
    );
    assert!(credit_balance.ok);
    assert_eq!(
        credit_balance
            .payload
            .unwrap_or_else(|| json!({}))
            .get("currency")
            .and_then(Value::as_str),
        Some("usd")
    );

    let credits_history_table = run_action(
        root.path(),
        "dashboard.prompts.system.webview.account.creditsHistoryTable.describe",
        &json!({"range": "30d"}),
    );
    assert!(credits_history_table.ok);
    assert_eq!(
        credits_history_table
            .payload
            .unwrap_or_else(|| json!({}))
            .get("range")
            .and_then(Value::as_str),
        Some("30d")
    );

    let remote_config_toggle = run_action(
        root.path(),
        "dashboard.prompts.system.webview.account.remoteConfigToggle.describe",
        &json!({"toggle": "off"}),
    );
    assert!(remote_config_toggle.ok);
    assert_eq!(
        remote_config_toggle
            .payload
            .unwrap_or_else(|| json!({}))
            .get("toggle")
            .and_then(Value::as_str),
        Some("off")
    );

    let styled_credit_display = run_action(
        root.path(),
        "dashboard.prompts.system.webview.account.styledCreditDisplay.describe",
        &json!({"style": "compact"}),
    );
    assert!(styled_credit_display.ok);
    assert_eq!(
        styled_credit_display
            .payload
            .unwrap_or_else(|| json!({}))
            .get("style")
            .and_then(Value::as_str),
        Some("compact")
    );

    let account_helpers = run_action(
        root.path(),
        "dashboard.prompts.system.webview.account.helpers.describe",
        &json!({"helper": "format_credit"}),
    );
    assert!(account_helpers.ok);
    assert_eq!(
        account_helpers
            .payload
            .unwrap_or_else(|| json!({}))
            .get("helper")
            .and_then(Value::as_str),
        Some("format_credit")
    );
}
