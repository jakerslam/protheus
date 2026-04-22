#[test]
fn dashboard_system_prompt_variant_routes_round_trip() {
    let root = tempfile::tempdir().expect("tempdir");

    let upsert = run_action(
        root.path(),
        "dashboard.prompts.system.variant.upsert",
        &json!({
            "profile": "devstral",
            "variant": "default",
            "template": "You are {{profile}}::{{variant}} mode={{mode}}",
            "config": {"temperature": 0.2},
            "overrides": {"max_output_tokens": 600}
        }),
    );
    assert!(upsert.ok);
    let upsert_payload = upsert.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        upsert_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_upsert")
    );

    let list = run_action(
        root.path(),
        "dashboard.prompts.system.variant.list",
        &json!({"profile": "devstral"}),
    );
    assert!(list.ok);
    let list_payload = list.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        list_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_list")
    );
    assert!(
        list_payload
            .get("rows")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
    );

    let resolve = run_action(
        root.path(),
        "dashboard.prompts.system.variant.resolve",
        &json!({"profile": "devstral", "variant": "default"}),
    );
    assert!(resolve.ok);
    let resolve_payload = resolve.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        resolve_payload.get("found").and_then(Value::as_bool),
        Some(true)
    );

    let render = run_action(
        root.path(),
        "dashboard.prompts.system.variant.render",
        &json!({
            "profile": "devstral",
            "variant": "default",
            "context": {"mode": "plan"}
        }),
    );
    assert!(render.ok);
    let render_payload = render.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        render_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_render")
    );
    let rendered_text = render_payload
        .get("rendered_text")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(rendered_text.contains("devstral"));
    assert!(rendered_text.contains("mode=plan"));
}

#[test]
fn dashboard_system_prompt_variant_validate_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let valid = run_action(
        root.path(),
        "dashboard.prompts.system.variant.validate",
        &json!({
            "profile": "gemini-3",
            "variant": "fast",
            "template": "profile={{profile}} variant={{variant}} mode={{mode}}"
        }),
    );
    assert!(valid.ok);
    let valid_payload = valid.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        valid_payload.get("valid").and_then(Value::as_bool),
        Some(true)
    );

    let invalid = run_action(
        root.path(),
        "dashboard.prompts.system.variant.validate",
        &json!({
            "profile": "gemini-3",
            "variant": "fast",
            "template": "no_tokens_present_here"
        }),
    );
    assert!(invalid.ok);
    let invalid_payload = invalid.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        invalid_payload.get("valid").and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        invalid_payload
            .get("reasons")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .any(|row| row.as_str() == Some("template_requires_known_tokens"))
    );
}
