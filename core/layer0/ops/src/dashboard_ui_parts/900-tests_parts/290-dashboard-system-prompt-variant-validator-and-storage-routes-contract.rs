#[test]
fn dashboard_system_prompt_variant_validator_routes_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let classify = run_action(
        root.path(),
        "dashboard.prompts.system.variant.classifyProfile",
        &json!({"profile": "xs"}),
    );
    assert!(classify.ok);
    let classify_payload = classify.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        classify_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_classify_profile")
    );
    assert_eq!(
        classify_payload.get("profile_class").and_then(Value::as_str),
        Some("xs")
    );

    let strict_ok = run_action(
        root.path(),
        "dashboard.prompts.system.variant.template.renderStrict",
        &json!({
            "profile": "trinity",
            "variant": "default",
            "mode": "plan",
            "template": "profile={{profile}} variant={{variant}} mode={{mode}}"
        }),
    );
    assert!(strict_ok.ok);
    let strict_ok_payload = strict_ok.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        strict_ok_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_template_render_strict")
    );
    assert!(
        strict_ok_payload
            .get("rendered_text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("trinity")
    );

    let matrix = run_action(
        root.path(),
        "dashboard.prompts.system.variant.validator.auditMatrix",
        &json!({
            "rows": [
                {"profile": "trinity", "variant": "default", "template": "x={{profile}}"},
                {"profile": "unknown", "variant": "default", "template": "x={{profile}}"}
            ]
        }),
    );
    assert!(matrix.ok);
    let matrix_payload = matrix.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        matrix_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_variant_validator_audit_matrix")
    );
    assert_eq!(
        matrix_payload.get("invalid_count").and_then(Value::as_i64),
        Some(1)
    );
}

#[test]
fn dashboard_system_prompt_storage_routes_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let seed = run_action(
        root.path(),
        "dashboard.prompts.system.storage.remoteConfig.seed",
        &json!({
            "source": "remote-sync",
            "etag": "abc123",
            "config": {"policy": "strict"}
        }),
    );
    assert!(seed.ok);
    let seed_payload = seed.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        seed_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_storage_remote_config_seed")
    );

    let fetch = run_action(
        root.path(),
        "dashboard.prompts.system.storage.remoteConfig.fetch",
        &json!({}),
    );
    assert!(fetch.ok);
    let fetch_payload = fetch.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        fetch_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_storage_remote_config_fetch")
    );
    assert_eq!(
        fetch_payload.get("has_remote").and_then(Value::as_bool),
        Some(true)
    );

    let snapshot = run_action(
        root.path(),
        "dashboard.prompts.system.storage.snapshot",
        &json!({}),
    );
    assert!(snapshot.ok);
    let snapshot_payload = snapshot.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        snapshot_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_storage_snapshot")
    );
    assert!(
        snapshot_payload
            .pointer("/storage/state_bytes")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= 0
    );

    let error_message = run_action(
        root.path(),
        "dashboard.prompts.system.storage.errorMessage",
        &json!({"code": "template_invalid"}),
    );
    assert!(error_message.ok);
    let error_message_payload = error_message.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        error_message_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_storage_error_message")
    );
    assert!(
        error_message_payload
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("invalid")
    );
}
