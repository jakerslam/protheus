#[test]
fn dashboard_system_prompt_variant_profile_defaults_and_compose_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let defaults = run_action(
        root.path(),
        "dashboard.prompts.system.variant.profileDefaults",
        &json!({
            "profile": "gpt-5"
        }),
    );
    assert!(defaults.ok);
    let defaults_payload = defaults.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        defaults_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_profile_defaults")
    );
    assert!(
        defaults_payload
            .pointer("/defaults/template")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("{{mode}}")
    );

    let compose = run_action(
        root.path(),
        "dashboard.prompts.system.variant.composeFromProfile",
        &json!({
            "profile": "gpt-5",
            "variant": "strict",
            "mode": "plan"
        }),
    );
    assert!(compose.ok);
    let compose_payload = compose.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        compose_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_compose_from_profile")
    );
    let rendered = compose_payload
        .get("rendered_text")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(rendered.contains("gpt-5"));
    assert!(rendered.contains("strict"));
    assert!(rendered.contains("plan"));
}

#[test]
fn dashboard_system_prompt_variant_builder_routes_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let index = run_action(
        root.path(),
        "dashboard.prompts.system.variant.index",
        &json!({}),
    );
    assert!(index.ok);
    let index_payload = index.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        index_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_index")
    );
    assert!(
        index_payload
            .get("profiles")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .any(|row| row.as_str() == Some("hermes"))
    );

    let preview = run_action(
        root.path(),
        "dashboard.prompts.system.variant.builder.preview",
        &json!({
            "profile": "hermes",
            "variant": "concise",
            "components": ["objective", "rules", "skills"]
        }),
    );
    assert!(preview.ok);
    let preview_payload = preview.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        preview_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_builder_preview")
    );
    assert!(
        preview_payload
            .get("template")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("components:")
    );

    let validate_ok = run_action(
        root.path(),
        "dashboard.prompts.system.variant.builder.validate",
        &json!({
            "profile": "hermes",
            "variant": "concise",
            "components": ["objective"]
        }),
    );
    assert!(validate_ok.ok);
    let validate_ok_payload = validate_ok.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        validate_ok_payload.get("valid").and_then(Value::as_bool),
        Some(true)
    );

    let validate_bad = run_action(
        root.path(),
        "dashboard.prompts.system.variant.builder.validate",
        &json!({
            "profile": "hermes",
            "variant": "concise",
            "components": []
        }),
    );
    assert!(validate_bad.ok);
    let validate_bad_payload = validate_bad.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        validate_bad_payload.get("valid").and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        validate_bad_payload
            .get("reasons")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .any(|row| row.as_str() == Some("components_required"))
    );
}
