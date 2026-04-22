#[test]
fn dashboard_system_prompt_shared_settings_messages_tail_routes_contract_wave_510() {
    let root = tempfile::tempdir().expect("tempdir");

    let auto_approval = run_action(
        root.path(),
        "dashboard.prompts.system.shared.autoApprovalSettings.describe",
        &json!({"policy": "review_required"}),
    );
    assert!(auto_approval.ok);
    assert_eq!(
        auto_approval
            .payload
            .unwrap_or_else(|| json!({}))
            .get("policy")
            .and_then(Value::as_str),
        Some("review_required")
    );

    let browser_settings = run_action(
        root.path(),
        "dashboard.prompts.system.shared.browserSettings.describe",
        &json!({"browser": "system"}),
    );
    assert!(browser_settings.ok);
    assert_eq!(
        browser_settings
            .payload
            .unwrap_or_else(|| json!({}))
            .get("browser")
            .and_then(Value::as_str),
        Some("system")
    );

    let chat_content = run_action(
        root.path(),
        "dashboard.prompts.system.shared.chatContent.describe",
        &json!({"kind": "text"}),
    );
    assert!(chat_content.ok);
    assert_eq!(
        chat_content
            .payload
            .unwrap_or_else(|| json!({}))
            .get("kind")
            .and_then(Value::as_str),
        Some("text")
    );

    let cline_account = run_action(
        root.path(),
        "dashboard.prompts.system.shared.clineAccount.describe",
        &json!({"provider": "openai"}),
    );
    assert!(cline_account.ok);
    assert_eq!(
        cline_account
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("openai")
    );

    let cline_banner = run_action(
        root.path(),
        "dashboard.prompts.system.shared.clineBanner.describe",
        &json!({"banner": "default"}),
    );
    assert!(cline_banner.ok);
    assert_eq!(
        cline_banner
            .payload
            .unwrap_or_else(|| json!({}))
            .get("banner")
            .and_then(Value::as_str),
        Some("default")
    );

    let cline_feature = run_action(
        root.path(),
        "dashboard.prompts.system.shared.clineFeatureSetting.describe",
        &json!({"feature": "default"}),
    );
    assert!(cline_feature.ok);
    assert_eq!(
        cline_feature
            .payload
            .unwrap_or_else(|| json!({}))
            .get("feature")
            .and_then(Value::as_str),
        Some("default")
    );

    let extension_message = run_action(
        root.path(),
        "dashboard.prompts.system.shared.extensionMessage.describe",
        &json!({"channel": "ui"}),
    );
    assert!(extension_message.ok);
    assert_eq!(
        extension_message
            .payload
            .unwrap_or_else(|| json!({}))
            .get("channel")
            .and_then(Value::as_str),
        Some("ui")
    );

    let focus_chain = run_action(
        root.path(),
        "dashboard.prompts.system.shared.focusChainSettings.describe",
        &json!({"mode": "balanced"}),
    );
    assert!(focus_chain.ok);
    assert_eq!(
        focus_chain
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("balanced")
    );

    let history_item = run_action(
        root.path(),
        "dashboard.prompts.system.shared.historyItem.describe",
        &json!({"id": "h-001"}),
    );
    assert!(history_item.ok);
    assert_eq!(
        history_item
            .payload
            .unwrap_or_else(|| json!({}))
            .get("id")
            .and_then(Value::as_str),
        Some("h-001")
    );

    let languages = run_action(
        root.path(),
        "dashboard.prompts.system.shared.languages.describe",
        &json!({"locale": "en"}),
    );
    assert!(languages.ok);
    assert_eq!(
        languages
            .payload
            .unwrap_or_else(|| json!({}))
            .get("locale")
            .and_then(Value::as_str),
        Some("en")
    );
}
