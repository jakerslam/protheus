#[test]
fn dashboard_system_prompt_services_telemetry_providers_temp_tail_routes_contract_wave_480() {
    let root = tempfile::tempdir().expect("tempdir");

    let telemetry_index = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.index.describe",
        &json!({}),
    );
    assert!(telemetry_index.ok);
    assert_eq!(
        telemetry_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_services_telemetry_index_describe")
    );

    let telemetry_i_provider = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.providers.iTelemetryProvider.describe",
        &json!({}),
    );
    assert!(telemetry_i_provider.ok);
    assert_eq!(
        telemetry_i_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_services_telemetry_i_provider_describe")
    );

    let otel_client = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.providers.openTelemetry.openTelemetryClientProvider.describe",
        &json!({"transport": "http"}),
    );
    assert!(otel_client.ok);
    assert_eq!(
        otel_client
            .payload
            .unwrap_or_else(|| json!({}))
            .get("transport")
            .and_then(Value::as_str),
        Some("http")
    );

    let otel_exporter_factory = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.providers.openTelemetry.openTelemetryExporterFactory.describe",
        &json!({"exporter": "otlp_http"}),
    );
    assert!(otel_exporter_factory.ok);
    assert_eq!(
        otel_exporter_factory
            .payload
            .unwrap_or_else(|| json!({}))
            .get("exporter")
            .and_then(Value::as_str),
        Some("otlp_http")
    );

    let otel_provider = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.providers.openTelemetry.openTelemetryTelemetryProvider.describe",
        &json!({"service_name": "infring"}),
    );
    assert!(otel_provider.ok);
    assert_eq!(
        otel_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("service_name")
            .and_then(Value::as_str),
        Some("infring")
    );

    let otel_diag = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.providers.openTelemetry.otelExporterDiagnostics.describe",
        &json!({"probe": "connectivity"}),
    );
    assert!(otel_diag.ok);
    assert_eq!(
        otel_diag
            .payload
            .unwrap_or_else(|| json!({}))
            .get("probe")
            .and_then(Value::as_str),
        Some("connectivity")
    );

    let posthog_client = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.providers.posthog.postHogClientProvider.describe",
        &json!({"project": "main"}),
    );
    assert!(posthog_client.ok);
    assert_eq!(
        posthog_client
            .payload
            .unwrap_or_else(|| json!({}))
            .get("project")
            .and_then(Value::as_str),
        Some("main")
    );

    let posthog_provider = run_action(
        root.path(),
        "dashboard.prompts.system.services.telemetry.providers.posthog.postHogTelemetryProvider.describe",
        &json!({"distinct_id": "user-1"}),
    );
    assert!(posthog_provider.ok);
    assert_eq!(
        posthog_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("distinct_id")
            .and_then(Value::as_str),
        Some("user-1")
    );

    let temp_manager = run_action(
        root.path(),
        "dashboard.prompts.system.services.temp.clineTempManager.describe",
        &json!({"scope": "session"}),
    );
    assert!(temp_manager.ok);
    assert_eq!(
        temp_manager
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("session")
    );

    let temp_index = run_action(
        root.path(),
        "dashboard.prompts.system.services.temp.index.describe",
        &json!({}),
    );
    assert!(temp_index.ok);
    assert_eq!(
        temp_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_services_temp_index_describe")
    );
}
