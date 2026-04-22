#[test]
fn dashboard_system_prompt_services_error_featureflags_glob_logging_mcp_tail_routes_contract_wave_930(
) {
    let root = tempfile::tempdir().expect("tempdir");

    let i_error_provider = run_action(
        root.path(),
        "dashboard.prompts.system.services.error.providers.iErrorProvider.describe",
        &json!({"provider_contract": "error_provider"}),
    );
    assert!(i_error_provider.ok);
    assert_eq!(
        i_error_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider_contract")
            .and_then(Value::as_str),
        Some("error_provider")
    );

    let post_hog_error_provider = run_action(
        root.path(),
        "dashboard.prompts.system.services.error.providers.postHogErrorProvider.describe",
        &json!({"provider": "posthog"}),
    );
    assert!(post_hog_error_provider.ok);
    assert_eq!(
        post_hog_error_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("posthog")
    );

    let feature_flags_provider_factory = run_action(
        root.path(),
        "dashboard.prompts.system.services.featureFlags.featureFlagsProviderFactory.describe",
        &json!({"factory_mode": "auto"}),
    );
    assert!(feature_flags_provider_factory.ok);
    assert_eq!(
        feature_flags_provider_factory
            .payload
            .unwrap_or_else(|| json!({}))
            .get("factory_mode")
            .and_then(Value::as_str),
        Some("auto")
    );

    let feature_flags_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.featureFlags.featureFlagsService.describe",
        &json!({"flags_mode": "runtime"}),
    );
    assert!(feature_flags_service.ok);
    assert_eq!(
        feature_flags_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("flags_mode")
            .and_then(Value::as_str),
        Some("runtime")
    );

    let feature_flags_index = run_action(
        root.path(),
        "dashboard.prompts.system.services.featureFlags.index.describe",
        &json!({"export_set": "all"}),
    );
    assert!(feature_flags_index.ok);
    assert_eq!(
        feature_flags_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("export_set")
            .and_then(Value::as_str),
        Some("all")
    );

    let i_feature_flags_provider = run_action(
        root.path(),
        "dashboard.prompts.system.services.featureFlags.providers.iFeatureFlagsProvider.describe",
        &json!({"provider_contract": "feature_flags_provider"}),
    );
    assert!(i_feature_flags_provider.ok);
    assert_eq!(
        i_feature_flags_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider_contract")
            .and_then(Value::as_str),
        Some("feature_flags_provider")
    );

    let post_hog_feature_flags_provider = run_action(
        root.path(),
        "dashboard.prompts.system.services.featureFlags.providers.postHogFeatureFlagsProvider.describe",
        &json!({"provider": "posthog"}),
    );
    assert!(post_hog_feature_flags_provider.ok);
    assert_eq!(
        post_hog_feature_flags_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("posthog")
    );

    let glob_list_files = run_action(
        root.path(),
        "dashboard.prompts.system.services.glob.listFiles.describe",
        &json!({"glob_mode": "workspace"}),
    );
    assert!(glob_list_files.ok);
    assert_eq!(
        glob_list_files
            .payload
            .unwrap_or_else(|| json!({}))
            .get("glob_mode")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let logging_distinct_id = run_action(
        root.path(),
        "dashboard.prompts.system.services.logging.distinctId.describe",
        &json!({"distinct_id_mode": "stable"}),
    );
    assert!(logging_distinct_id.ok);
    assert_eq!(
        logging_distinct_id
            .payload
            .unwrap_or_else(|| json!({}))
            .get("distinct_id_mode")
            .and_then(Value::as_str),
        Some("stable")
    );

    let mcp_hub = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.mcpHub.describe",
        &json!({"hub_mode": "managed"}),
    );
    assert!(mcp_hub.ok);
    assert_eq!(
        mcp_hub
            .payload
            .unwrap_or_else(|| json!({}))
            .get("hub_mode")
            .and_then(Value::as_str),
        Some("managed")
    );
}
