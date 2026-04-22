#[test]
fn dashboard_system_prompt_component_route_covers_core_components() {
    let root = tempfile::tempdir().expect("tempdir");

    let rules = run_action(
        root.path(),
        "dashboard.prompts.system.component",
        &json!({
            "component": "rules",
            "rules": ["no destructive writes", "preserve layer authority"]
        }),
    );
    assert!(rules.ok);
    let rules_payload = rules.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        rules_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_component")
    );
    assert!(
        rules_payload
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("no destructive writes")
    );

    let capabilities = run_action(
        root.path(),
        "dashboard.prompts.system.component",
        &json!({
            "component": "capabilities",
            "capabilities": ["read", "write", "terminal"]
        }),
    );
    assert!(capabilities.ok);
    let capabilities_payload = capabilities.payload.unwrap_or_else(|| json!({}));
    assert!(
        capabilities_payload
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("terminal")
    );
}

#[test]
fn dashboard_system_prompt_compose_route_covers_legacy_and_gpt5_profiles() {
    let root = tempfile::tempdir().expect("tempdir");

    let legacy = run_action(
        root.path(),
        "dashboard.prompts.system.compose",
        &json!({
            "profile": "legacy_compact",
            "components": ["objective", "rules", "mcp", "feedback"],
            "objective": "Harden runtime behavior",
            "rules": ["fail closed"],
            "mcp_policy": "allow_docs_only"
        }),
    );
    assert!(legacy.ok);
    let legacy_payload = legacy.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        legacy_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_compose")
    );
    assert!(
        legacy_payload
            .get("prompt_text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("Legacy Compact")
    );

    let gpt5 = run_action(
        root.path(),
        "dashboard.prompts.system.compose",
        &json!({
            "profile": "gpt5",
            "components": ["objective", "capabilities", "editing_files", "act_vs_plan_mode"],
            "objective": "Reliable synthesis",
            "capabilities": ["analysis", "patching"],
            "mode": "plan"
        }),
    );
    assert!(gpt5.ok);
    let gpt5_payload = gpt5.payload.unwrap_or_else(|| json!({}));
    assert!(
        gpt5_payload
            .get("prompt_text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("GPT-5 Next-Gen")
    );
    assert!(
        gpt5_payload
            .get("prompt_text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("Mode: PLAN")
    );
}
