#[test]
fn dashboard_system_prompt_services_mcp_ripgrep_telemetry_tail_routes_contract_wave_940() {
    let root = tempfile::tempdir().expect("tempdir");

    let mcp_oauth_manager = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.mcpOAuthManager.describe",
        &json!({"oauth_mode": "managed"}),
    );
    assert!(mcp_oauth_manager.ok);
    assert_eq!(
        mcp_oauth_manager
            .payload
            .unwrap_or_else(|| json!({}))
            .get("oauth_mode")
            .and_then(Value::as_str),
        Some("managed")
    );

    let mcp_oauth_redirect_resolver = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.mcpOAuthRedirectResolver.describe",
        &json!({"redirect_mode": "strict"}),
    );
    assert!(mcp_oauth_redirect_resolver.ok);
    assert_eq!(
        mcp_oauth_redirect_resolver
            .payload
            .unwrap_or_else(|| json!({}))
            .get("redirect_mode")
            .and_then(Value::as_str),
        Some("strict")
    );

    let streamable_http_reconnect_handler = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.streamableHttpReconnectHandler.describe",
        &json!({"reconnect_mode": "backoff"}),
    );
    assert!(streamable_http_reconnect_handler.ok);
    assert_eq!(
        streamable_http_reconnect_handler
            .payload
            .unwrap_or_else(|| json!({}))
            .get("reconnect_mode")
            .and_then(Value::as_str),
        Some("backoff")
    );

    let mcp_constants = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.constants.describe",
        &json!({"constant_set": "runtime"}),
    );
    assert!(mcp_constants.ok);
    assert_eq!(
        mcp_constants
            .payload
            .unwrap_or_else(|| json!({}))
            .get("constant_set")
            .and_then(Value::as_str),
        Some("runtime")
    );

    let mcp_schemas = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.schemas.describe",
        &json!({"schema_set": "core"}),
    );
    assert!(mcp_schemas.ok);
    assert_eq!(
        mcp_schemas
            .payload
            .unwrap_or_else(|| json!({}))
            .get("schema_set")
            .and_then(Value::as_str),
        Some("core")
    );

    let mcp_types = run_action(
        root.path(),
        "dashboard.prompts.system.services.mcp.types.describe",
        &json!({"type_set": "runtime"}),
    );
    assert!(mcp_types.ok);
    assert_eq!(
        mcp_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type_set")
            .and_then(Value::as_str),
        Some("runtime")
    );

    let ripgrep_index = run_action(
        root.path(),
        "dashboard.prompts.system.services.ripgrep.index.describe",
        &json!({"index_mode": "workspace"}),
    );
    assert!(ripgrep_index.ok);
    assert_eq!(
        ripgrep_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("index_mode")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let telemetry_provider_factory = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.telemetryProviderFactory.describe",
        &json!({"provider_mode": "auto"}),
    );
    assert!(telemetry_provider_factory.ok);
    assert_eq!(
        telemetry_provider_factory
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider_mode")
            .and_then(Value::as_str),
        Some("auto")
    );

    let telemetry_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.telemetryService.describe",
        &json!({"telemetry_mode": "standard"}),
    );
    assert!(telemetry_service.ok);
    assert_eq!(
        telemetry_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("telemetry_mode")
            .and_then(Value::as_str),
        Some("standard")
    );

    let telemetry_event_handler_base = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.events.eventHandlerBase.describe",
        &json!({"handler_mode": "base"}),
    );
    assert!(telemetry_event_handler_base.ok);
    assert_eq!(
        telemetry_event_handler_base
            .payload
            .unwrap_or_else(|| json!({}))
            .get("handler_mode")
            .and_then(Value::as_str),
        Some("base")
    );
}
