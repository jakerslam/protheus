fn dashboard_prompt_services_mcp_mcp_oauth_manager_describe(payload: &Value) -> Value {
    let oauth_mode = clean_text(
        payload
            .get("oauth_mode")
            .and_then(Value::as_str)
            .unwrap_or("managed"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_mcp_oauth_manager_describe",
        "oauth_mode": oauth_mode
    })
}

fn dashboard_prompt_services_mcp_mcp_oauth_redirect_resolver_describe(payload: &Value) -> Value {
    let redirect_mode = clean_text(
        payload
            .get("redirect_mode")
            .and_then(Value::as_str)
            .unwrap_or("strict"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_mcp_oauth_redirect_resolver_describe",
        "redirect_mode": redirect_mode
    })
}

fn dashboard_prompt_services_mcp_streamable_http_reconnect_handler_describe(payload: &Value) -> Value {
    let reconnect_mode = clean_text(
        payload
            .get("reconnect_mode")
            .and_then(Value::as_str)
            .unwrap_or("backoff"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_streamable_http_reconnect_handler_describe",
        "reconnect_mode": reconnect_mode
    })
}

fn dashboard_prompt_services_mcp_constants_describe(payload: &Value) -> Value {
    let constant_set = clean_text(
        payload
            .get("constant_set")
            .and_then(Value::as_str)
            .unwrap_or("runtime"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_constants_describe",
        "constant_set": constant_set
    })
}

fn dashboard_prompt_services_mcp_schemas_describe(payload: &Value) -> Value {
    let schema_set = clean_text(
        payload
            .get("schema_set")
            .and_then(Value::as_str)
            .unwrap_or("core"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_schemas_describe",
        "schema_set": schema_set
    })
}

fn dashboard_prompt_services_mcp_types_describe(payload: &Value) -> Value {
    let type_set = clean_text(
        payload
            .get("type_set")
            .and_then(Value::as_str)
            .unwrap_or("runtime"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_types_describe",
        "type_set": type_set
    })
}

fn dashboard_prompt_services_ripgrep_index_describe(payload: &Value) -> Value {
    let index_mode = clean_text(
        payload
            .get("index_mode")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_ripgrep_index_describe",
        "index_mode": index_mode
    })
}

fn dashboard_prompt_services_telemetry_provider_factory_describe(payload: &Value) -> Value {
    let provider_mode = clean_text(
        payload
            .get("provider_mode")
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_provider_factory_describe",
        "provider_mode": provider_mode
    })
}

fn dashboard_prompt_services_telemetry_service_describe(payload: &Value) -> Value {
    let telemetry_mode = clean_text(
        payload
            .get("telemetry_mode")
            .and_then(Value::as_str)
            .unwrap_or("standard"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_service_describe",
        "telemetry_mode": telemetry_mode
    })
}

fn dashboard_prompt_services_telemetry_events_event_handler_base_describe(payload: &Value) -> Value {
    let handler_mode = clean_text(
        payload
            .get("handler_mode")
            .and_then(Value::as_str)
            .unwrap_or("base"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_events_event_handler_base_describe",
        "handler_mode": handler_mode
    })
}

fn dashboard_prompt_hosts_surface_tail_services_mcp_ripgrep_telemetry_route_extension(
    _root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.services.mcp.mcpOAuthManager.describe" => {
            Some(dashboard_prompt_services_mcp_mcp_oauth_manager_describe(payload))
        }
        "dashboard.prompts.system.services.mcp.mcpOAuthRedirectResolver.describe" => {
            Some(dashboard_prompt_services_mcp_mcp_oauth_redirect_resolver_describe(payload))
        }
        "dashboard.prompts.system.services.mcp.streamableHttpReconnectHandler.describe" => {
            Some(dashboard_prompt_services_mcp_streamable_http_reconnect_handler_describe(payload))
        }
        "dashboard.prompts.system.services.mcp.constants.describe" => {
            Some(dashboard_prompt_services_mcp_constants_describe(payload))
        }
        "dashboard.prompts.system.services.mcp.schemas.describe" => {
            Some(dashboard_prompt_services_mcp_schemas_describe(payload))
        }
        "dashboard.prompts.system.services.mcp.types.describe" => {
            Some(dashboard_prompt_services_mcp_types_describe(payload))
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
            Some(dashboard_prompt_services_telemetry_events_event_handler_base_describe(
                payload,
            ))
        }
        _ => None,
    }
}
