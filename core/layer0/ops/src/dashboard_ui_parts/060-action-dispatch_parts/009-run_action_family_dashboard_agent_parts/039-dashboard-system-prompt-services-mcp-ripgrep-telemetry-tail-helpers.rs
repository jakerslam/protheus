fn dashboard_prompt_services_mcp_oauth_manager_describe(payload: &Value) -> Value {
    let flow = clean_text(
        payload
            .get("flow")
            .and_then(Value::as_str)
            .unwrap_or("device_code"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_oauth_manager_describe",
        "flow": flow
    })
}

fn dashboard_prompt_services_mcp_oauth_redirect_resolver_describe(payload: &Value) -> Value {
    let redirect_uri = clean_text(
        payload
            .get("redirect_uri")
            .and_then(Value::as_str)
            .unwrap_or(""),
        1400,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_oauth_redirect_resolver_describe",
        "redirect_uri": redirect_uri
    })
}

fn dashboard_prompt_services_mcp_streamable_reconnect_handler_describe(payload: &Value) -> Value {
    let policy = clean_text(
        payload
            .get("policy")
            .and_then(Value::as_str)
            .unwrap_or("bounded_retry"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_streamable_reconnect_handler_describe",
        "policy": policy
    })
}

fn dashboard_prompt_services_mcp_constants_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_constants_describe",
        "contracts": ["default_reconnect_budget", "oauth_timeout_ms"]
    })
}

fn dashboard_prompt_services_mcp_schemas_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_schemas_describe",
        "contracts": ["mcp_request_schema", "mcp_response_schema", "mcp_event_schema"]
    })
}

fn dashboard_prompt_services_mcp_types_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_types_describe",
        "contracts": ["mcp_client", "mcp_server", "mcp_transport"]
    })
}

fn dashboard_prompt_services_ripgrep_index_describe(payload: &Value) -> Value {
    let query = clean_text(payload.get("query").and_then(Value::as_str).unwrap_or(""), 400);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_ripgrep_index_describe",
        "query": query
    })
}

fn dashboard_prompt_services_telemetry_provider_factory_describe(payload: &Value) -> Value {
    let provider = clean_text(
        payload
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("opentelemetry"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_provider_factory_describe",
        "provider": provider
    })
}

fn dashboard_prompt_services_telemetry_service_describe(payload: &Value) -> Value {
    let event = clean_text(
        payload
            .get("event")
            .and_then(Value::as_str)
            .unwrap_or("runtime_event"),
        220,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_service_describe",
        "event": event
    })
}

fn dashboard_prompt_services_telemetry_event_handler_base_describe(payload: &Value) -> Value {
    let handler = clean_text(
        payload
            .get("handler")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_event_handler_base_describe",
        "handler": handler
    })
}

fn dashboard_prompt_services_mcp_ripgrep_telemetry_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.services.mcp.mcpOAuthManager.describe" => {
            Some(dashboard_prompt_services_mcp_oauth_manager_describe(payload))
        }
        "dashboard.prompts.system.services.mcp.mcpOAuthRedirectResolver.describe" => {
            Some(dashboard_prompt_services_mcp_oauth_redirect_resolver_describe(payload))
        }
        "dashboard.prompts.system.services.mcp.streamableHttpReconnectHandler.describe" => {
            Some(dashboard_prompt_services_mcp_streamable_reconnect_handler_describe(payload))
        }
        "dashboard.prompts.system.services.mcp.constants.describe" => {
            Some(dashboard_prompt_services_mcp_constants_describe())
        }
        "dashboard.prompts.system.services.mcp.schemas.describe" => {
            Some(dashboard_prompt_services_mcp_schemas_describe())
        }
        "dashboard.prompts.system.services.mcp.types.describe" => {
            Some(dashboard_prompt_services_mcp_types_describe())
        }
        "dashboard.prompts.system.services.ripgrep.index.describe" => {
            Some(dashboard_prompt_services_ripgrep_index_describe(payload))
        }
        "dashboard.prompts.system.services.telemetry.telemetryProviderFactory.describe" => {
            Some(dashboard_prompt_services_telemetry_provider_factory_describe(payload))
        }
        "dashboard.prompts.system.services.telemetry.telemetryService.describe" => {
            Some(dashboard_prompt_services_telemetry_service_describe(payload))
        }
        "dashboard.prompts.system.services.telemetry.events.eventHandlerBase.describe" => {
            Some(dashboard_prompt_services_telemetry_event_handler_base_describe(payload))
        }
        _ => dashboard_prompt_services_telemetry_providers_temp_tail_route_extension(
            root, normalized, payload,
        ),
    }
}

include!("040-dashboard-system-prompt-services-telemetry-providers-temp-tail-helpers.rs");
