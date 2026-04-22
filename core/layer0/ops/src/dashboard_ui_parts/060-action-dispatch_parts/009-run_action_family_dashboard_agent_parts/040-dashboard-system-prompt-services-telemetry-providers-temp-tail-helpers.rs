fn dashboard_prompt_services_telemetry_index_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_index_describe",
        "exports": ["telemetry_service", "provider_factory", "providers", "events"]
    })
}

fn dashboard_prompt_services_telemetry_i_provider_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_i_provider_describe",
        "contract": "telemetry_provider"
    })
}

fn dashboard_prompt_services_telemetry_otel_client_provider_describe(payload: &Value) -> Value {
    let transport = clean_text(
        payload
            .get("transport")
            .and_then(Value::as_str)
            .unwrap_or("http"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_otel_client_provider_describe",
        "transport": transport
    })
}

fn dashboard_prompt_services_telemetry_otel_exporter_factory_describe(payload: &Value) -> Value {
    let exporter = clean_text(
        payload
            .get("exporter")
            .and_then(Value::as_str)
            .unwrap_or("otlp_http"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_otel_exporter_factory_describe",
        "exporter": exporter
    })
}

fn dashboard_prompt_services_telemetry_otel_provider_describe(payload: &Value) -> Value {
    let service_name = clean_text(
        payload
            .get("service_name")
            .and_then(Value::as_str)
            .unwrap_or("infring"),
        200,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_otel_provider_describe",
        "service_name": service_name
    })
}

fn dashboard_prompt_services_telemetry_otel_exporter_diagnostics_describe(payload: &Value) -> Value {
    let probe = clean_text(
        payload
            .get("probe")
            .and_then(Value::as_str)
            .unwrap_or("connectivity"),
        180,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_otel_exporter_diagnostics_describe",
        "probe": probe
    })
}

fn dashboard_prompt_services_telemetry_posthog_client_provider_describe(payload: &Value) -> Value {
    let project = clean_text(
        payload
            .get("project")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        180,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_posthog_client_provider_describe",
        "project": project
    })
}

fn dashboard_prompt_services_telemetry_posthog_provider_describe(payload: &Value) -> Value {
    let distinct_id = clean_text(
        payload
            .get("distinct_id")
            .and_then(Value::as_str)
            .unwrap_or("anonymous"),
        220,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_telemetry_posthog_provider_describe",
        "distinct_id": distinct_id
    })
}

fn dashboard_prompt_services_temp_cline_temp_manager_describe(payload: &Value) -> Value {
    let scope = clean_text(
        payload
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or("session"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_temp_cline_temp_manager_describe",
        "scope": scope
    })
}

fn dashboard_prompt_services_temp_index_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_temp_index_describe",
        "exports": ["temp_manager"]
    })
}

fn dashboard_prompt_services_telemetry_providers_temp_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.services.telemetry.index.describe" => {
            Some(dashboard_prompt_services_telemetry_index_describe())
        }
        "dashboard.prompts.system.services.telemetry.providers.iTelemetryProvider.describe" => {
            Some(dashboard_prompt_services_telemetry_i_provider_describe())
        }
        "dashboard.prompts.system.services.telemetry.providers.openTelemetry.openTelemetryClientProvider.describe" => {
            Some(dashboard_prompt_services_telemetry_otel_client_provider_describe(payload))
        }
        "dashboard.prompts.system.services.telemetry.providers.openTelemetry.openTelemetryExporterFactory.describe" => {
            Some(dashboard_prompt_services_telemetry_otel_exporter_factory_describe(payload))
        }
        "dashboard.prompts.system.services.telemetry.providers.openTelemetry.openTelemetryTelemetryProvider.describe" => {
            Some(dashboard_prompt_services_telemetry_otel_provider_describe(payload))
        }
        "dashboard.prompts.system.services.telemetry.providers.openTelemetry.otelExporterDiagnostics.describe" => {
            Some(dashboard_prompt_services_telemetry_otel_exporter_diagnostics_describe(payload))
        }
        "dashboard.prompts.system.services.telemetry.providers.posthog.postHogClientProvider.describe" => {
            Some(dashboard_prompt_services_telemetry_posthog_client_provider_describe(payload))
        }
        "dashboard.prompts.system.services.telemetry.providers.posthog.postHogTelemetryProvider.describe" => {
            Some(dashboard_prompt_services_telemetry_posthog_provider_describe(payload))
        }
        "dashboard.prompts.system.services.temp.clineTempManager.describe" => {
            Some(dashboard_prompt_services_temp_cline_temp_manager_describe(payload))
        }
        "dashboard.prompts.system.services.temp.index.describe" => {
            Some(dashboard_prompt_services_temp_index_describe())
        }
        _ => dashboard_prompt_services_test_tree_sitter_tail_route_extension(root, normalized, payload),
    }
}

include!("041-dashboard-system-prompt-services-test-tree-sitter-tail-helpers.rs");
