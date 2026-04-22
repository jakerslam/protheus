#[test]
fn dashboard_system_prompt_locks_mentions_permissions_prompts_tail_routes_contract_wave_700() {
    let root = tempfile::tempdir().expect("tempdir");

    let locks_types = run_action(
        root.path(),
        "dashboard.prompts.system.locks.types.describe",
        &json!({"lock_kind": "exclusive"}),
    );
    assert!(locks_types.ok);
    assert_eq!(
        locks_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("lock_kind")
            .and_then(Value::as_str),
        Some("exclusive")
    );

    let mentions_index = run_action(
        root.path(),
        "dashboard.prompts.system.mentions.index.describe",
        &json!({"mention_scope": "workspace"}),
    );
    assert!(mentions_index.ok);
    assert_eq!(
        mentions_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mention_scope")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let permissions_controller = run_action(
        root.path(),
        "dashboard.prompts.system.permissions.commandPermissionController.describe",
        &json!({"decision_mode": "fail_closed"}),
    );
    assert!(permissions_controller.ok);
    assert_eq!(
        permissions_controller
            .payload
            .unwrap_or_else(|| json!({}))
            .get("decision_mode")
            .and_then(Value::as_str),
        Some("fail_closed")
    );

    let permissions_index = run_action(
        root.path(),
        "dashboard.prompts.system.permissions.index.describe",
        &json!({"policy_view": "effective"}),
    );
    assert!(permissions_index.ok);
    assert_eq!(
        permissions_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("policy_view")
            .and_then(Value::as_str),
        Some("effective")
    );

    let permissions_types = run_action(
        root.path(),
        "dashboard.prompts.system.permissions.types.describe",
        &json!({"permission_shape": "structured"}),
    );
    assert!(permissions_types.ok);
    assert_eq!(
        permissions_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("permission_shape")
            .and_then(Value::as_str),
        Some("structured")
    );

    let context_management = run_action(
        root.path(),
        "dashboard.prompts.system.prompts.contextManagement.describe",
        &json!({"retention_mode": "bounded"}),
    );
    assert!(context_management.ok);
    assert_eq!(
        context_management
            .payload
            .unwrap_or_else(|| json!({}))
            .get("retention_mode")
            .and_then(Value::as_str),
        Some("bounded")
    );

    let load_mcp_documentation = run_action(
        root.path(),
        "dashboard.prompts.system.prompts.loadMcpDocumentation.describe",
        &json!({"doc_scope": "mcp"}),
    );
    assert!(load_mcp_documentation.ok);
    assert_eq!(
        load_mcp_documentation
            .payload
            .unwrap_or_else(|| json!({}))
            .get("doc_scope")
            .and_then(Value::as_str),
        Some("mcp")
    );

    let responses = run_action(
        root.path(),
        "dashboard.prompts.system.prompts.responses.describe",
        &json!({"response_mode": "structured"}),
    );
    assert!(responses.ok);
    assert_eq!(
        responses
            .payload
            .unwrap_or_else(|| json!({}))
            .get("response_mode")
            .and_then(Value::as_str),
        Some("structured")
    );

    let local_models_compact = run_action(
        root.path(),
        "dashboard.prompts.system.prompts.legacy.localModels.compactSystemPrompt.describe",
        &json!({"compactness": "high"}),
    );
    assert!(local_models_compact.ok);
    assert_eq!(
        local_models_compact
            .payload
            .unwrap_or_else(|| json!({}))
            .get("compactness")
            .and_then(Value::as_str),
        Some("high")
    );

    let next_gen_gpt5 = run_action(
        root.path(),
        "dashboard.prompts.system.prompts.legacy.nextGen.gpt5.describe",
        &json!({"profile": "gpt-5"}),
    );
    assert!(next_gen_gpt5.ok);
    assert_eq!(
        next_gen_gpt5
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("gpt-5")
    );
}
