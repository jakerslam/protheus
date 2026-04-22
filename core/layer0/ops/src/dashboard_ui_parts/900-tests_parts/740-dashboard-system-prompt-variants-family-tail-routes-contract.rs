#[test]
fn dashboard_system_prompt_variants_family_tail_routes_contract_wave_740() {
    let root = tempfile::tempdir().expect("tempdir");

    let glm_config = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.glm.config.describe",
        &json!({"profile": "glm"}),
    );
    assert!(glm_config.ok);
    assert_eq!(
        glm_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("glm")
    );

    let glm_overrides = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.glm.overrides.describe",
        &json!({"override_mode": "balanced"}),
    );
    assert!(glm_overrides.ok);
    assert_eq!(
        glm_overrides
            .payload
            .unwrap_or_else(|| json!({}))
            .get("override_mode")
            .and_then(Value::as_str),
        Some("balanced")
    );

    let glm_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.glm.template.describe",
        &json!({"template_mode": "default"}),
    );
    assert!(glm_template.ok);
    assert_eq!(
        glm_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let gpt5_config = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.gpt5.config.describe",
        &json!({"profile": "gpt5"}),
    );
    assert!(gpt5_config.ok);
    assert_eq!(
        gpt5_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("gpt5")
    );

    let gpt5_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.gpt5.template.describe",
        &json!({"template_mode": "default"}),
    );
    assert!(gpt5_template.ok);
    assert_eq!(
        gpt5_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let hermes_config = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.hermes.config.describe",
        &json!({"profile": "hermes"}),
    );
    assert!(hermes_config.ok);
    assert_eq!(
        hermes_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("hermes")
    );

    let hermes_overrides = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.hermes.overrides.describe",
        &json!({"override_mode": "balanced"}),
    );
    assert!(hermes_overrides.ok);
    assert_eq!(
        hermes_overrides
            .payload
            .unwrap_or_else(|| json!({}))
            .get("override_mode")
            .and_then(Value::as_str),
        Some("balanced")
    );

    let hermes_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.hermes.template.describe",
        &json!({"template_mode": "default"}),
    );
    assert!(hermes_template.ok);
    assert_eq!(
        hermes_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let variants_index = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.index.describe",
        &json!({"index_scope": "all"}),
    );
    assert!(variants_index.ok);
    assert_eq!(
        variants_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("index_scope")
            .and_then(Value::as_str),
        Some("all")
    );

    let native_gpt51_config = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.nativeGpt51.config.describe",
        &json!({"profile": "native-gpt-5-1"}),
    );
    assert!(native_gpt51_config.ok);
    assert_eq!(
        native_gpt51_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("native-gpt-5-1")
    );
}
