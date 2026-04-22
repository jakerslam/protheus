#[test]
fn dashboard_system_prompt_shared_constants_messages_tail_routes_contract_wave_540() {
    let root = tempfile::tempdir().expect("tempdir");

    let constants = run_action(
        root.path(),
        "dashboard.prompts.system.shared.constants.describe",
        &json!({"scope": "runtime"}),
    );
    assert!(constants.ok);
    assert_eq!(
        constants
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("runtime")
    );

    let content_limits = run_action(
        root.path(),
        "dashboard.prompts.system.shared.contentLimits.describe",
        &json!({"lane": "default"}),
    );
    assert!(content_limits.ok);
    assert_eq!(
        content_limits
            .payload
            .unwrap_or_else(|| json!({}))
            .get("lane")
            .and_then(Value::as_str),
        Some("default")
    );

    let context_mentions = run_action(
        root.path(),
        "dashboard.prompts.system.shared.contextMentions.describe",
        &json!({"source": "conversation"}),
    );
    assert!(context_mentions.ok);
    assert_eq!(
        context_mentions
            .payload
            .unwrap_or_else(|| json!({}))
            .get("source")
            .and_then(Value::as_str),
        Some("conversation")
    );

    let focus_chain_utils = run_action(
        root.path(),
        "dashboard.prompts.system.shared.focusChainUtils.describe",
        &json!({"strategy": "balanced"}),
    );
    assert!(focus_chain_utils.ok);
    assert_eq!(
        focus_chain_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("strategy")
            .and_then(Value::as_str),
        Some("balanced")
    );

    let api_metrics = run_action(
        root.path(),
        "dashboard.prompts.system.shared.getApiMetrics.describe",
        &json!({"window": "5m"}),
    );
    assert!(api_metrics.ok);
    assert_eq!(
        api_metrics
            .payload
            .unwrap_or_else(|| json!({}))
            .get("window")
            .and_then(Value::as_str),
        Some("5m")
    );

    let internal_account = run_action(
        root.path(),
        "dashboard.prompts.system.shared.internal.account.describe",
        &json!({"account_type": "default"}),
    );
    assert!(internal_account.ok);
    assert_eq!(
        internal_account
            .payload
            .unwrap_or_else(|| json!({}))
            .get("account_type")
            .and_then(Value::as_str),
        Some("default")
    );

    let mcp = run_action(
        root.path(),
        "dashboard.prompts.system.shared.mcp.describe",
        &json!({"transport": "streamable_http"}),
    );
    assert!(mcp.ok);
    assert_eq!(
        mcp
            .payload
            .unwrap_or_else(|| json!({}))
            .get("transport")
            .and_then(Value::as_str),
        Some("streamable_http")
    );

    let messages_constants = run_action(
        root.path(),
        "dashboard.prompts.system.shared.messages.constants.describe",
        &json!({}),
    );
    assert!(messages_constants.ok);
    assert_eq!(
        messages_constants
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_shared_messages_constants_describe")
    );

    let messages_content = run_action(
        root.path(),
        "dashboard.prompts.system.shared.messages.content.describe",
        &json!({"content_type": "text"}),
    );
    assert!(messages_content.ok);
    assert_eq!(
        messages_content
            .payload
            .unwrap_or_else(|| json!({}))
            .get("content_type")
            .and_then(Value::as_str),
        Some("text")
    );

    let messages_index = run_action(
        root.path(),
        "dashboard.prompts.system.shared.messages.index.describe",
        &json!({}),
    );
    assert!(messages_index.ok);
    assert_eq!(
        messages_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_shared_messages_index_describe")
    );
}
