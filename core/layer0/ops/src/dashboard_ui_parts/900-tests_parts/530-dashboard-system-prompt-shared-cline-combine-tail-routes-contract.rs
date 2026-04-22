#[test]
fn dashboard_system_prompt_shared_cline_combine_tail_routes_contract_wave_530() {
    let root = tempfile::tempdir().expect("tempdir");

    let cline_banner = run_action(
        root.path(),
        "dashboard.prompts.system.shared.cline.banner.describe",
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

    let cline_context = run_action(
        root.path(),
        "dashboard.prompts.system.shared.cline.context.describe",
        &json!({"context": "workspace"}),
    );
    assert!(cline_context.ok);
    assert_eq!(
        cline_context
            .payload
            .unwrap_or_else(|| json!({}))
            .get("context")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let cline_index = run_action(
        root.path(),
        "dashboard.prompts.system.shared.cline.index.describe",
        &json!({}),
    );
    assert!(cline_index.ok);
    assert_eq!(
        cline_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_shared_cline_index_describe")
    );

    let cline_onboarding = run_action(
        root.path(),
        "dashboard.prompts.system.shared.cline.onboarding.describe",
        &json!({"stage": "welcome"}),
    );
    assert!(cline_onboarding.ok);
    assert_eq!(
        cline_onboarding
            .payload
            .unwrap_or_else(|| json!({}))
            .get("stage")
            .and_then(Value::as_str),
        Some("welcome")
    );

    let cline_recommended = run_action(
        root.path(),
        "dashboard.prompts.system.shared.cline.recommendedModels.describe",
        &json!({"family": "balanced"}),
    );
    assert!(cline_recommended.ok);
    assert_eq!(
        cline_recommended
            .payload
            .unwrap_or_else(|| json!({}))
            .get("family")
            .and_then(Value::as_str),
        Some("balanced")
    );

    let combine_api_requests = run_action(
        root.path(),
        "dashboard.prompts.system.shared.combineApiRequests.describe",
        &json!({"strategy": "sequential"}),
    );
    assert!(combine_api_requests.ok);
    assert_eq!(
        combine_api_requests
            .payload
            .unwrap_or_else(|| json!({}))
            .get("strategy")
            .and_then(Value::as_str),
        Some("sequential")
    );

    let combine_command_sequences = run_action(
        root.path(),
        "dashboard.prompts.system.shared.combineCommandSequences.describe",
        &json!({"strategy": "preserve_order"}),
    );
    assert!(combine_command_sequences.ok);
    assert_eq!(
        combine_command_sequences
            .payload
            .unwrap_or_else(|| json!({}))
            .get("strategy")
            .and_then(Value::as_str),
        Some("preserve_order")
    );

    let combine_error_retry_messages = run_action(
        root.path(),
        "dashboard.prompts.system.shared.combineErrorRetryMessages.describe",
        &json!({"policy": "retry_then_surface"}),
    );
    assert!(combine_error_retry_messages.ok);
    assert_eq!(
        combine_error_retry_messages
            .payload
            .unwrap_or_else(|| json!({}))
            .get("policy")
            .and_then(Value::as_str),
        Some("retry_then_surface")
    );

    let combine_hook_sequences = run_action(
        root.path(),
        "dashboard.prompts.system.shared.combineHookSequences.describe",
        &json!({"mode": "ordered"}),
    );
    assert!(combine_hook_sequences.ok);
    assert_eq!(
        combine_hook_sequences
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("ordered")
    );

    let config_types = run_action(
        root.path(),
        "dashboard.prompts.system.shared.configTypes.describe",
        &json!({"schema": "runtime_config"}),
    );
    assert!(config_types.ok);
    assert_eq!(
        config_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("schema")
            .and_then(Value::as_str),
        Some("runtime_config")
    );
}
