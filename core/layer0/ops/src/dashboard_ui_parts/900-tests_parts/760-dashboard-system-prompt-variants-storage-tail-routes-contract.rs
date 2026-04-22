#[test]
fn dashboard_system_prompt_variants_storage_tail_routes_contract_wave_760() {
    let root = tempfile::tempdir().expect("tempdir");

    let trinity_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.trinity.template.describe",
        &json!({"template_mode": "default"}),
    );
    assert!(trinity_template.ok);
    assert_eq!(
        trinity_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let variant_builder = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.variantBuilder.describe",
        &json!({"builder_mode": "composed"}),
    );
    assert!(variant_builder.ok);
    assert_eq!(
        variant_builder
            .payload
            .unwrap_or_else(|| json!({}))
            .get("builder_mode")
            .and_then(Value::as_str),
        Some("composed")
    );

    let variant_validator = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.variantValidator.describe",
        &json!({"validator_mode": "strict"}),
    );
    assert!(variant_validator.ok);
    assert_eq!(
        variant_validator
            .payload
            .unwrap_or_else(|| json!({}))
            .get("validator_mode")
            .and_then(Value::as_str),
        Some("strict")
    );

    let xs_config = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.xs.config.describe",
        &json!({"profile": "xs"}),
    );
    assert!(xs_config.ok);
    assert_eq!(
        xs_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("xs")
    );

    let xs_overrides = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.xs.overrides.describe",
        &json!({"override_mode": "balanced"}),
    );
    assert!(xs_overrides.ok);
    assert_eq!(
        xs_overrides
            .payload
            .unwrap_or_else(|| json!({}))
            .get("override_mode")
            .and_then(Value::as_str),
        Some("balanced")
    );

    let xs_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.xs.template.describe",
        &json!({"template_mode": "default"}),
    );
    assert!(xs_template.ok);
    assert_eq!(
        xs_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let state_manager = run_action(
        root.path(),
        "dashboard.prompts.system.storage.stateManager.describe",
        &json!({"state_mode": "persisted"}),
    );
    assert!(state_manager.ok);
    assert_eq!(
        state_manager
            .payload
            .unwrap_or_else(|| json!({}))
            .get("state_mode")
            .and_then(Value::as_str),
        Some("persisted")
    );

    let disk = run_action(
        root.path(),
        "dashboard.prompts.system.storage.disk.describe",
        &json!({"disk_mode": "safe_write"}),
    );
    assert!(disk.ok);
    assert_eq!(
        disk.payload
            .unwrap_or_else(|| json!({}))
            .get("disk_mode")
            .and_then(Value::as_str),
        Some("safe_write")
    );

    let error_messages = run_action(
        root.path(),
        "dashboard.prompts.system.storage.errorMessages.describe",
        &json!({"error_catalog": "default"}),
    );
    assert!(error_messages.ok);
    assert_eq!(
        error_messages
            .payload
            .unwrap_or_else(|| json!({}))
            .get("error_catalog")
            .and_then(Value::as_str),
        Some("default")
    );

    let remote_config_fetch = run_action(
        root.path(),
        "dashboard.prompts.system.storage.remoteConfigFetch.describe",
        &json!({"fetch_policy": "network_first"}),
    );
    assert!(remote_config_fetch.ok);
    assert_eq!(
        remote_config_fetch
            .payload
            .unwrap_or_else(|| json!({}))
            .get("fetch_policy")
            .and_then(Value::as_str),
        Some("network_first")
    );
}
