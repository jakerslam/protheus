#[test]
fn dashboard_system_prompt_variants_native_tail_routes_contract_wave_750() {
    let root = tempfile::tempdir().expect("tempdir");

    let native_gpt51_overrides = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.nativeGpt51.overrides.describe",
        &json!({"override_mode": "balanced"}),
    );
    assert!(native_gpt51_overrides.ok);
    assert_eq!(
        native_gpt51_overrides
            .payload
            .unwrap_or_else(|| json!({}))
            .get("override_mode")
            .and_then(Value::as_str),
        Some("balanced")
    );

    let native_gpt51_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.nativeGpt51.template.describe",
        &json!({"template_mode": "default"}),
    );
    assert!(native_gpt51_template.ok);
    assert_eq!(
        native_gpt51_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let native_gpt5_config = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.nativeGpt5.config.describe",
        &json!({"profile": "native-gpt-5"}),
    );
    assert!(native_gpt5_config.ok);
    assert_eq!(
        native_gpt5_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("native-gpt-5")
    );

    let native_gpt5_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.nativeGpt5.template.describe",
        &json!({"template_mode": "default"}),
    );
    assert!(native_gpt5_template.ok);
    assert_eq!(
        native_gpt5_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let native_next_gen_config = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.nativeNextGen.config.describe",
        &json!({"profile": "native-next-gen"}),
    );
    assert!(native_next_gen_config.ok);
    assert_eq!(
        native_next_gen_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("native-next-gen")
    );

    let native_next_gen_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.nativeNextGen.template.describe",
        &json!({"template_mode": "default"}),
    );
    assert!(native_next_gen_template.ok);
    assert_eq!(
        native_next_gen_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let next_gen_config = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.nextGen.config.describe",
        &json!({"profile": "next-gen"}),
    );
    assert!(next_gen_config.ok);
    assert_eq!(
        next_gen_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("next-gen")
    );

    let next_gen_template = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.nextGen.template.describe",
        &json!({"template_mode": "default"}),
    );
    assert!(next_gen_template.ok);
    assert_eq!(
        next_gen_template
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let trinity_config = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.trinity.config.describe",
        &json!({"profile": "trinity"}),
    );
    assert!(trinity_config.ok);
    assert_eq!(
        trinity_config
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("trinity")
    );

    let trinity_overrides = run_action(
        root.path(),
        "dashboard.prompts.system.systemPrompt.variants.trinity.overrides.describe",
        &json!({"override_mode": "balanced"}),
    );
    assert!(trinity_overrides.ok);
    assert_eq!(
        trinity_overrides
            .payload
            .unwrap_or_else(|| json!({}))
            .get("override_mode")
            .and_then(Value::as_str),
        Some("balanced")
    );
}
