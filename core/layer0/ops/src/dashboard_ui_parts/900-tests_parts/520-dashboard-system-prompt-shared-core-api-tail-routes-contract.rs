#[test]
fn dashboard_system_prompt_shared_core_api_tail_routes_contract_wave_520() {
    let root = tempfile::tempdir().expect("tempdir");

    let mcp_display_mode = run_action(
        root.path(),
        "dashboard.prompts.system.shared.mcpDisplayMode.describe",
        &json!({"mode": "compact"}),
    );
    assert!(mcp_display_mode.ok);
    assert_eq!(
        mcp_display_mode
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("compact")
    );

    let patch = run_action(
        root.path(),
        "dashboard.prompts.system.shared.patch.describe",
        &json!({"patch_kind": "unified"}),
    );
    assert!(patch.ok);
    assert_eq!(
        patch
            .payload
            .unwrap_or_else(|| json!({}))
            .get("patch_kind")
            .and_then(Value::as_str),
        Some("unified")
    );

    let telemetry_setting = run_action(
        root.path(),
        "dashboard.prompts.system.shared.telemetrySetting.describe",
        &json!({"level": "standard"}),
    );
    assert!(telemetry_setting.ok);
    assert_eq!(
        telemetry_setting
            .payload
            .unwrap_or_else(|| json!({}))
            .get("level")
            .and_then(Value::as_str),
        Some("standard")
    );

    let user_info = run_action(
        root.path(),
        "dashboard.prompts.system.shared.userInfo.describe",
        &json!({"role": "owner"}),
    );
    assert!(user_info.ok);
    assert_eq!(
        user_info
            .payload
            .unwrap_or_else(|| json!({}))
            .get("role")
            .and_then(Value::as_str),
        Some("owner")
    );

    let webview_message = run_action(
        root.path(),
        "dashboard.prompts.system.shared.webviewMessage.describe",
        &json!({"channel": "webview"}),
    );
    assert!(webview_message.ok);
    assert_eq!(
        webview_message
            .payload
            .unwrap_or_else(|| json!({}))
            .get("channel")
            .and_then(Value::as_str),
        Some("webview")
    );

    let shared_api = run_action(
        root.path(),
        "dashboard.prompts.system.shared.api.describe",
        &json!({"api": "primary"}),
    );
    assert!(shared_api.ok);
    assert_eq!(
        shared_api
            .payload
            .unwrap_or_else(|| json!({}))
            .get("api")
            .and_then(Value::as_str),
        Some("primary")
    );

    let shared_array = run_action(
        root.path(),
        "dashboard.prompts.system.shared.array.describe",
        &json!({"operation": "merge"}),
    );
    assert!(shared_array.ok);
    assert_eq!(
        shared_array
            .payload
            .unwrap_or_else(|| json!({}))
            .get("operation")
            .and_then(Value::as_str),
        Some("merge")
    );

    let requesty = run_action(
        root.path(),
        "dashboard.prompts.system.shared.clients.requesty.describe",
        &json!({"endpoint": "/v1/workflow"}),
    );
    assert!(requesty.ok);
    assert_eq!(
        requesty
            .payload
            .unwrap_or_else(|| json!({}))
            .get("endpoint")
            .and_then(Value::as_str),
        Some("/v1/workflow")
    );

    let cline_rules = run_action(
        root.path(),
        "dashboard.prompts.system.shared.clineRules.describe",
        &json!({"profile": "default"}),
    );
    assert!(cline_rules.ok);
    assert_eq!(
        cline_rules
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("default")
    );

    let cline_api = run_action(
        root.path(),
        "dashboard.prompts.system.shared.cline.api.describe",
        &json!({"provider": "openai"}),
    );
    assert!(cline_api.ok);
    assert_eq!(
        cline_api
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("openai")
    );
}
