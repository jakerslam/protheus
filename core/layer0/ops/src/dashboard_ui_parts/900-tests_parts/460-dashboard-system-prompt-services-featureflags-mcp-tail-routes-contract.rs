#[test]
fn dashboard_system_prompt_services_featureflags_mcp_tail_routes_contract_wave_460() {
    let root = tempfile::tempdir().expect("tempdir");

    let i_error_provider = run_action(
        root.path(),
        "dashboard.prompts.system.services.error.providers.iErrorProvider.describe",
        &json!({"provider_key": "posthog"}),
    );
    assert!(i_error_provider.ok);
    assert_eq!(
        i_error_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider_key")
            .and_then(Value::as_str),
        Some("posthog")
    );

    let posthog_error_provider = run_action(
        root.path(),
        "dashboard.prompts.system.services.error.providers.postHogErrorProvider.describe",
        &json!({"event": "runtime_error"}),
    );
    assert!(posthog_error_provider.ok);
    assert_eq!(
        posthog_error_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("event")
            .and_then(Value::as_str),
        Some("runtime_error")
    );

    let ff_provider_factory = run_action(
        root.path(),
        "dashboard.prompts.system.services.featureFlags.featureFlagsProviderFactory.describe",
        &json!({"provider": "posthog"}),
    );
    assert!(ff_provider_factory.ok);
    assert_eq!(
        ff_provider_factory
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("posthog")
    );

    let ff_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.featureFlags.featureFlagsService.describe",
        &json!({"flag_key": "beta_mode"}),
    );
    assert!(ff_service.ok);
    assert_eq!(
        ff_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("flag_key")
            .and_then(Value::as_str),
        Some("beta_mode")
    );

    let ff_index = run_action(
        root.path(),
        "dashboard.prompts.system.services.featureFlags.index.describe",
        &json!({}),
    );
    assert!(ff_index.ok);
    assert_eq!(
        ff_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_services_feature_flags_index_describe")
    );

    let i_ff_provider = run_action(
        root.path(),
        "dashboard.prompts.system.services.featureFlags.providers.iFeatureFlagsProvider.describe",
        &json!({}),
    );
    assert!(i_ff_provider.ok);
    assert_eq!(
        i_ff_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_services_feature_flags_i_provider_describe")
    );

    let posthog_ff_provider = run_action(
        root.path(),
        "dashboard.prompts.system.services.featureFlags.providers.postHogFeatureFlagsProvider.describe",
        &json!({"distinct_id": "user-123"}),
    );
    assert!(posthog_ff_provider.ok);
    assert_eq!(
        posthog_ff_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("distinct_id")
            .and_then(Value::as_str),
        Some("user-123")
    );

    let glob_list_files = run_action(
        root.path(),
        "dashboard.prompts.system.services.glob.listFiles.describe",
        &json!({"pattern": "src/**/*.ts"}),
    );
    assert!(glob_list_files.ok);
    assert_eq!(
        glob_list_files
            .payload
            .unwrap_or_else(|| json!({}))
            .get("pattern")
            .and_then(Value::as_str),
        Some("src/**/*.ts")
    );

    let logging_distinct_id = run_action(
        root.path(),
        "dashboard.prompts.system.services.logging.distinctId.describe",
        &json!({"seed": "runtime"}),
    );
    assert!(logging_distinct_id.ok);
    assert_eq!(
        logging_distinct_id
            .payload
            .unwrap_or_else(|| json!({}))
            .get("seed")
            .and_then(Value::as_str),
        Some("runtime")
    );

    let mcp_hub = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.mcpHub.describe",
        &json!({"command": "status"}),
    );
    assert!(mcp_hub.ok);
    assert_eq!(
        mcp_hub
            .payload
            .unwrap_or_else(|| json!({}))
            .get("command")
            .and_then(Value::as_str),
        Some("status")
    );
}
