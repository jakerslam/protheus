#[test]
fn dashboard_system_prompt_registry_routes_round_trip() {
    let root = tempfile::tempdir().expect("tempdir");

    let upsert = run_action(
        root.path(),
        "dashboard.prompts.system.registry.upsert",
        &json!({
            "registry_key": "ops_default",
            "profile": "gpt5",
            "components": ["objective", "rules", "skills", "system_info"],
            "objective": "Preserve deterministic synthesis contracts",
            "rules": ["fail closed", "no authoritative shell mutations"],
            "mcp_policy": "allow_docs_only"
        }),
    );
    assert!(upsert.ok);
    let upsert_payload = upsert.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        upsert_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_registry_upsert")
    );

    let toolset = run_action(
        root.path(),
        "dashboard.prompts.system.registry.upsertToolSet",
        &json!({
            "toolset_id": "default",
            "tools": ["read_file", "apply_patch", "exec_command"]
        }),
    );
    assert!(toolset.ok);
    let toolset_payload = toolset.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        toolset_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_registry_upsert_toolset")
    );

    let list = run_action(
        root.path(),
        "dashboard.prompts.system.registry.list",
        &json!({}),
    );
    assert!(list.ok);
    let list_payload = list.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        list_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_registry_list")
    );
    assert!(
        list_payload
            .get("entries")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
    );

    let build = run_action(
        root.path(),
        "dashboard.prompts.system.registry.build",
        &json!({
            "registry_key": "ops_default",
            "mode": "plan"
        }),
    );
    assert!(build.ok);
    let build_payload = build.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        build_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_registry_build")
    );
    assert_eq!(
        build_payload.pointer("/compose/type").and_then(Value::as_str),
        Some("dashboard_prompts_system_compose")
    );
}

#[test]
fn dashboard_system_prompt_index_and_spec_routes_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let index = run_action(
        root.path(),
        "dashboard.prompts.system.index",
        &json!({"profile": "gpt5"}),
    );
    assert!(index.ok);
    let index_payload = index.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        index_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_index")
    );
    let components = index_payload
        .get("components")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    assert!(components.iter().any(|row| row == "skills"));
    assert!(components.iter().any(|row| row == "spec"));

    let spec_valid = run_action(
        root.path(),
        "dashboard.prompts.system.spec.validate",
        &json!({
            "profile": "gpt5",
            "components": ["objective", "rules", "skills", "system_info"]
        }),
    );
    assert!(spec_valid.ok);
    let spec_valid_payload = spec_valid.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        spec_valid_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_spec_validate")
    );
    assert_eq!(
        spec_valid_payload.get("valid").and_then(Value::as_bool),
        Some(true)
    );

    let spec_invalid = run_action(
        root.path(),
        "dashboard.prompts.system.spec.validate",
        &json!({
            "profile": "gpt5",
            "components": ["objective", "unknown_component"]
        }),
    );
    assert!(spec_invalid.ok);
    let spec_invalid_payload = spec_invalid.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        spec_invalid_payload.get("valid").and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        spec_invalid_payload
            .get("unknown_components")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .any(|row| row.as_str() == Some("unknown_component"))
    );
}
