fn dashboard_prompt_shared_proto_state_telemetry_setting_describe(payload: &Value) -> Value {
    let direction = clean_text(
        payload
            .get("direction")
            .and_then(Value::as_str)
            .unwrap_or("to_proto"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_proto_state_telemetry_setting_describe",
        "direction": direction
    })
}

fn dashboard_prompt_shared_proto_web_open_graph_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("summary"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_proto_web_open_graph_describe",
        "mode": mode
    })
}

fn dashboard_prompt_shared_providers_bedrock_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_providers_bedrock_describe",
        "profile": profile
    })
}

fn dashboard_prompt_shared_providers_index_describe(payload: &Value) -> Value {
    let catalog = clean_text(
        payload
            .get("catalog")
            .and_then(Value::as_str)
            .unwrap_or("primary"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_providers_index_describe",
        "catalog": catalog
    })
}

fn dashboard_prompt_shared_providers_vertex_describe(payload: &Value) -> Value {
    let region = clean_text(
        payload
            .get("region")
            .and_then(Value::as_str)
            .unwrap_or("us-central1"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_providers_vertex_describe",
        "region": region
    })
}

fn dashboard_prompt_shared_remote_config_constants_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_remote_config_constants_describe",
        "exports": ["default_refresh_interval", "schema_version"]
    })
}

fn dashboard_prompt_shared_remote_config_schema_describe(payload: &Value) -> Value {
    let schema = clean_text(
        payload
            .get("schema")
            .and_then(Value::as_str)
            .unwrap_or("remote_config_v1"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_remote_config_schema_describe",
        "schema": schema
    })
}

fn dashboard_prompt_shared_services_logger_describe(payload: &Value) -> Value {
    let level = clean_text(
        payload
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("info"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_services_logger_describe",
        "level": level
    })
}

fn dashboard_prompt_shared_services_session_describe(payload: &Value) -> Value {
    let session_scope = clean_text(
        payload
            .get("session_scope")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_services_session_describe",
        "session_scope": session_scope
    })
}

fn dashboard_prompt_shared_services_config_otel_describe(payload: &Value) -> Value {
    let exporter = clean_text(
        payload
            .get("exporter")
            .and_then(Value::as_str)
            .unwrap_or("otlp_http"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_services_config_otel_describe",
        "exporter": exporter
    })
}

fn dashboard_prompt_shared_provider_remote_services_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.shared.protoConversions.state.telemetrySettingConversion.describe" => {
            Some(dashboard_prompt_shared_proto_state_telemetry_setting_describe(payload))
        }
        "dashboard.prompts.system.shared.protoConversions.web.openGraphConversion.describe" => {
            Some(dashboard_prompt_shared_proto_web_open_graph_describe(payload))
        }
        "dashboard.prompts.system.shared.providers.bedrock.describe" => {
            Some(dashboard_prompt_shared_providers_bedrock_describe(payload))
        }
        "dashboard.prompts.system.shared.providers.providers.describe" => {
            Some(dashboard_prompt_shared_providers_index_describe(payload))
        }
        "dashboard.prompts.system.shared.providers.vertex.describe" => {
            Some(dashboard_prompt_shared_providers_vertex_describe(payload))
        }
        "dashboard.prompts.system.shared.remoteConfig.constants.describe" => {
            Some(dashboard_prompt_shared_remote_config_constants_describe())
        }
        "dashboard.prompts.system.shared.remoteConfig.schema.describe" => {
            Some(dashboard_prompt_shared_remote_config_schema_describe(payload))
        }
        "dashboard.prompts.system.shared.services.logger.describe" => {
            Some(dashboard_prompt_shared_services_logger_describe(payload))
        }
        "dashboard.prompts.system.shared.services.session.describe" => {
            Some(dashboard_prompt_shared_services_session_describe(payload))
        }
        "dashboard.prompts.system.shared.services.config.otelConfig.describe" => {
            Some(dashboard_prompt_shared_services_config_otel_describe(payload))
        }
        _ => dashboard_prompt_shared_services_worker_storage_tail_route_extension(
            root, normalized, payload,
        ),
    }
}

include!("049-dashboard-system-prompt-shared-services-worker-storage-tail-helpers.rs");
