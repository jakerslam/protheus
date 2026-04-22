#[test]
fn dashboard_system_prompt_components_tail_routes_contract_wave_710() {
    let root = tempfile::tempdir().expect("tempdir");

    let act_vs_plan = run_action(
        root.path(),
        "dashboard.prompts.system.components.actVsPlanMode.describe",
        &json!({"mode": "act"}),
    );
    assert!(act_vs_plan.ok);
    assert_eq!(
        act_vs_plan
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("act")
    );

    let capabilities = run_action(
        root.path(),
        "dashboard.prompts.system.components.capabilities.describe",
        &json!({"capability_scope": "core"}),
    );
    assert!(capabilities.ok);
    assert_eq!(
        capabilities
            .payload
            .unwrap_or_else(|| json!({}))
            .get("capability_scope")
            .and_then(Value::as_str),
        Some("core")
    );

    let editing_files = run_action(
        root.path(),
        "dashboard.prompts.system.components.editingFiles.describe",
        &json!({"edit_policy": "safe"}),
    );
    assert!(editing_files.ok);
    assert_eq!(
        editing_files
            .payload
            .unwrap_or_else(|| json!({}))
            .get("edit_policy")
            .and_then(Value::as_str),
        Some("safe")
    );

    let feedback = run_action(
        root.path(),
        "dashboard.prompts.system.components.feedback.describe",
        &json!({"feedback_tone": "direct"}),
    );
    assert!(feedback.ok);
    assert_eq!(
        feedback
            .payload
            .unwrap_or_else(|| json!({}))
            .get("feedback_tone")
            .and_then(Value::as_str),
        Some("direct")
    );

    let index = run_action(
        root.path(),
        "dashboard.prompts.system.components.index.describe",
        &json!({"index_scope": "components"}),
    );
    assert!(index.ok);
    assert_eq!(
        index.payload
            .unwrap_or_else(|| json!({}))
            .get("index_scope")
            .and_then(Value::as_str),
        Some("components")
    );

    let mcp = run_action(
        root.path(),
        "dashboard.prompts.system.components.mcp.describe",
        &json!({"mcp_mode": "connected"}),
    );
    assert!(mcp.ok);
    assert_eq!(
        mcp.payload
            .unwrap_or_else(|| json!({}))
            .get("mcp_mode")
            .and_then(Value::as_str),
        Some("connected")
    );

    let objective = run_action(
        root.path(),
        "dashboard.prompts.system.components.objective.describe",
        &json!({"objective_mode": "deliverable"}),
    );
    assert!(objective.ok);
    assert_eq!(
        objective
            .payload
            .unwrap_or_else(|| json!({}))
            .get("objective_mode")
            .and_then(Value::as_str),
        Some("deliverable")
    );

    let rules = run_action(
        root.path(),
        "dashboard.prompts.system.components.rules.describe",
        &json!({"ruleset": "default"}),
    );
    assert!(rules.ok);
    assert_eq!(
        rules.payload
            .unwrap_or_else(|| json!({}))
            .get("ruleset")
            .and_then(Value::as_str),
        Some("default")
    );

    let skills = run_action(
        root.path(),
        "dashboard.prompts.system.components.skills.describe",
        &json!({"skill_scope": "active"}),
    );
    assert!(skills.ok);
    assert_eq!(
        skills
            .payload
            .unwrap_or_else(|| json!({}))
            .get("skill_scope")
            .and_then(Value::as_str),
        Some("active")
    );

    let system_info = run_action(
        root.path(),
        "dashboard.prompts.system.components.systemInfo.describe",
        &json!({"info_scope": "runtime"}),
    );
    assert!(system_info.ok);
    assert_eq!(
        system_info
            .payload
            .unwrap_or_else(|| json!({}))
            .get("info_scope")
            .and_then(Value::as_str),
        Some("runtime")
    );
}
