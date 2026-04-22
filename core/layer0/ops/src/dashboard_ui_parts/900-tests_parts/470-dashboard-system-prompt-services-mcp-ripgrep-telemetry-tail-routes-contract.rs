#[test]
fn dashboard_system_prompt_services_mcp_ripgrep_telemetry_tail_routes_contract_wave_470() {
    let root = tempfile::tempdir().expect("tempdir");

    let oauth_manager = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.mcpOAuthManager.describe",
        &json!({"flow": "device_code"}),
    );
    assert!(oauth_manager.ok);
    assert_eq!(
        oauth_manager
            .payload
            .unwrap_or_else(|| json!({}))
            .get("flow")
            .and_then(Value::as_str),
        Some("device_code")
    );

    let oauth_redirect = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.mcpOAuthRedirectResolver.describe",
        &json!({"redirect_uri": "https://example.com/callback"}),
    );
    assert!(oauth_redirect.ok);
    assert_eq!(
        oauth_redirect
            .payload
            .unwrap_or_else(|| json!({}))
            .get("redirect_uri")
            .and_then(Value::as_str),
        Some("https://example.com/callback")
    );

    let reconnect_handler = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.streamableHttpReconnectHandler.describe",
        &json!({"policy": "bounded_retry"}),
    );
    assert!(reconnect_handler.ok);
    assert_eq!(
        reconnect_handler
            .payload
            .unwrap_or_else(|| json!({}))
            .get("policy")
            .and_then(Value::as_str),
        Some("bounded_retry")
    );

    let mcp_constants = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.constants.describe",
        &json!({}),
    );
    assert!(mcp_constants.ok);
    assert_eq!(
        mcp_constants
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_services_mcp_constants_describe")
    );

    let mcp_schemas = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.schemas.describe",
        &json!({}),
    );
    assert!(mcp_schemas.ok);
    assert_eq!(
        mcp_schemas
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_services_mcp_schemas_describe")
    );

    let mcp_types = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.types.describe",
        &json!({}),
    );
    assert!(mcp_types.ok);
    assert_eq!(
        mcp_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_services_mcp_types_describe")
    );

    let ripgrep_index = run_action(
        root.path(),
        "dashboard.prompts.system.services.ripgrep.index.describe",
        &json!({"query": "TODO"}),
    );
    assert!(ripgrep_index.ok);
    assert_eq!(
        ripgrep_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("TODO")
    );

    let telemetry_factory = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.telemetryProviderFactory.describe",
        &json!({"provider": "opentelemetry"}),
    );
    assert!(telemetry_factory.ok);
    assert_eq!(
        telemetry_factory
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("opentelemetry")
    );

    let telemetry_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.telemetryService.describe",
        &json!({"event": "runtime_event"}),
    );
    assert!(telemetry_service.ok);
    assert_eq!(
        telemetry_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("event")
            .and_then(Value::as_str),
        Some("runtime_event")
    );

    let event_handler = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.events.eventHandlerBase.describe",
        &json!({"handler": "default"}),
    );
    assert!(event_handler.ok);
    assert_eq!(
        event_handler
            .payload
            .unwrap_or_else(|| json!({}))
            .get("handler")
            .and_then(Value::as_str),
        Some("default")
    );
}
