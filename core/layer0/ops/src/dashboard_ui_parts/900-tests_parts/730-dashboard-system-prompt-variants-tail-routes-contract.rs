#[test]
fn dashboard_system_prompt_variants_tail_routes_contract_wave_730() {
    let root = tempfile::tempdir().expect("tempdir");

    let types = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.types.describe",
        &json!({"type_mode": "strict"}),
    );
    assert!(types.ok);
    assert_eq!(
        types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type_mode")
            .and_then(Value::as_str),
        Some("strict")
    );

    let config_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.configTemplate.describe",
        &json!({"template_profile": "base"}),
    );
    assert!(config_template.ok);
    assert_eq!(
        config_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_profile")
            .and_then(Value::as_str),
        Some("base")
    );

    let devstral_config = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.devstral.config.describe",
        &json!({"profile": "devstral"}),
    );
    assert!(devstral_config.ok);
    assert_eq!(
        devstral_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("devstral")
    );

    let devstral_overrides = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.devstral.overrides.describe",
        &json!({"override_mode": "safe"}),
    );
    assert!(devstral_overrides.ok);
    assert_eq!(
        devstral_overrides
            .payload
            .unwrap_or_else(|| json!({}))
            .get("override_mode")
            .and_then(Value::as_str),
        Some("safe")
    );

    let devstral_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.devstral.template.describe",
        &json!({"template_mode": "default"}),
    );
    assert!(devstral_template.ok);
    assert_eq!(
        devstral_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let gemini3_config = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.gemini3.config.describe",
        &json!({"profile": "gemini3"}),
    );
    assert!(gemini3_config.ok);
    assert_eq!(
        gemini3_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("gemini3")
    );

    let gemini3_overrides = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.gemini3.overrides.describe",
        &json!({"override_mode": "balanced"}),
    );
    assert!(gemini3_overrides.ok);
    assert_eq!(
        gemini3_overrides
            .payload
            .unwrap_or_else(|| json!({}))
            .get("override_mode")
            .and_then(Value::as_str),
        Some("balanced")
    );

    let gemini3_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.gemini3.template.describe",
        &json!({"template_mode": "concise"}),
    );
    assert!(gemini3_template.ok);
    assert_eq!(
        gemini3_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_mode")
            .and_then(Value::as_str),
        Some("concise")
    );

    let generic_config = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.generic.config.describe",
        &json!({"profile": "generic"}),
    );
    assert!(generic_config.ok);
    assert_eq!(
        generic_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("generic")
    );

    let generic_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.generic.template.describe",
        &json!({"template_mode": "generic"}),
    );
    assert!(generic_template.ok);
    assert_eq!(
        generic_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_mode")
            .and_then(Value::as_str),
        Some("generic")
    );
}
