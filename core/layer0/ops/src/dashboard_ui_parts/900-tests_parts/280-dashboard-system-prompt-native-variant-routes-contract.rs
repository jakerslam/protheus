#[test]
fn dashboard_system_prompt_native_variant_catalog_and_resolve_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let catalog = run_action(
        root.path(),
        "dashboard.prompts.system.variant.native.catalog",
        &json!({}),
    );
    assert!(catalog.ok);
    let catalog_payload = catalog.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        catalog_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_native_catalog")
    );
    assert!(
        catalog_payload
            .get("profiles")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .any(|row| row.as_str() == Some("native-gpt-5-1"))
    );

    let resolve = run_action(
        root.path(),
        "dashboard.prompts.system.variant.native.resolve",
        &json!({
            "profile": "native-gpt-5"
        }),
    );
    assert!(resolve.ok);
    let resolve_payload = resolve.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        resolve_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_native_resolve")
    );
    assert_eq!(
        resolve_payload.get("found").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn dashboard_system_prompt_native_variant_render_and_validate_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let render = run_action(
        root.path(),
        "dashboard.prompts.system.variant.native.render",
        &json!({
            "profile": "next-gen",
            "variant": "default",
            "mode": "plan"
        }),
    );
    assert!(render.ok);
    let render_payload = render.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        render_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_native_render")
    );
    let rendered = render_payload
        .get("rendered_text")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(rendered.contains("next-gen"));
    assert!(rendered.contains("plan"));

    let validate_ok = run_action(
        root.path(),
        "dashboard.prompts.system.variant.native.validate",
        &json!({"profile": "trinity"}),
    );
    assert!(validate_ok.ok);
    let validate_ok_payload = validate_ok.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        validate_ok_payload.get("valid").and_then(Value::as_bool),
        Some(true)
    );

    let validate_bad = run_action(
        root.path(),
        "dashboard.prompts.system.variant.native.validate",
        &json!({"profile": "unknown-native"}),
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
            .any(|row| row.as_str() == Some("native_profile_unknown"))
    );
}
