#[test]
fn dashboard_system_prompt_shared_proto_conversions_tail_routes_contract_wave_550() {
    let root = tempfile::tempdir().expect("tempdir");

    let messages_metrics = run_action(
        root.path(),
        "dashboard.prompts.system.shared.messages.metrics.describe",
        &json!({"metric": "token_usage"}),
    );
    assert!(messages_metrics.ok);
    assert_eq!(
        messages_metrics
            .payload
            .unwrap_or_else(|| json!({}))
            .get("metric")
            .and_then(Value::as_str),
        Some("token_usage")
    );

    let multi_root_types = run_action(
        root.path(),
        "dashboard.prompts.system.shared.multiRoot.types.describe",
        &json!({"workspace_mode": "single_root"}),
    );
    assert!(multi_root_types.ok);
    assert_eq!(
        multi_root_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("workspace_mode")
            .and_then(Value::as_str),
        Some("single_root")
    );

    let net = run_action(
        root.path(),
        "dashboard.prompts.system.shared.net.describe",
        &json!({"transport": "https"}),
    );
    assert!(net.ok);
    assert_eq!(
        net.payload
            .unwrap_or_else(|| json!({}))
            .get("transport")
            .and_then(Value::as_str),
        Some("https")
    );

    let prompts = run_action(
        root.path(),
        "dashboard.prompts.system.shared.prompts.describe",
        &json!({"prompt_kind": "system"}),
    );
    assert!(prompts.ok);
    assert_eq!(
        prompts
            .payload
            .unwrap_or_else(|| json!({}))
            .get("prompt_kind")
            .and_then(Value::as_str),
        Some("system")
    );

    let cline_message = run_action(
        root.path(),
        "dashboard.prompts.system.shared.protoConversions.clineMessage.describe",
        &json!({"direction": "to_proto"}),
    );
    assert!(cline_message.ok);
    assert_eq!(
        cline_message
            .payload
            .unwrap_or_else(|| json!({}))
            .get("direction")
            .and_then(Value::as_str),
        Some("to_proto")
    );

    let search_result = run_action(
        root.path(),
        "dashboard.prompts.system.shared.protoConversions.file.searchResultConversion.describe",
        &json!({"file": "src/main.ts"}),
    );
    assert!(search_result.ok);
    assert_eq!(
        search_result
            .payload
            .unwrap_or_else(|| json!({}))
            .get("file")
            .and_then(Value::as_str),
        Some("src/main.ts")
    );

    let mcp_server = run_action(
        root.path(),
        "dashboard.prompts.system.shared.protoConversions.mcp.mcpServerConversion.describe",
        &json!({"server": "local"}),
    );
    assert!(mcp_server.ok);
    assert_eq!(
        mcp_server
            .payload
            .unwrap_or_else(|| json!({}))
            .get("server")
            .and_then(Value::as_str),
        Some("local")
    );

    let api_configuration = run_action(
        root.path(),
        "dashboard.prompts.system.shared.protoConversions.models.apiConfigurationConversion.describe",
        &json!({"model_family": "gpt"}),
    );
    assert!(api_configuration.ok);
    assert_eq!(
        api_configuration
            .payload
            .unwrap_or_else(|| json!({}))
            .get("model_family")
            .and_then(Value::as_str),
        Some("gpt")
    );

    let type_conversion = run_action(
        root.path(),
        "dashboard.prompts.system.shared.protoConversions.models.typeConversion.describe",
        &json!({"conversion": "strict"}),
    );
    assert!(type_conversion.ok);
    assert_eq!(
        type_conversion
            .payload
            .unwrap_or_else(|| json!({}))
            .get("conversion")
            .and_then(Value::as_str),
        Some("strict")
    );

    let vscode_models = run_action(
        root.path(),
        "dashboard.prompts.system.shared.protoConversions.models.vscodeLmModelsConversion.describe",
        &json!({"target": "chat"}),
    );
    assert!(vscode_models.ok);
    assert_eq!(
        vscode_models
            .payload
            .unwrap_or_else(|| json!({}))
            .get("target")
            .and_then(Value::as_str),
        Some("chat")
    );
}
