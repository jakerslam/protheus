#[test]
fn dashboard_system_prompt_shared_provider_remote_services_tail_routes_contract_wave_560() {
    let root = tempfile::tempdir().expect("tempdir");

    let telemetry_setting = run_action(
        root.path(),
        "dashboard.prompts.system.shared.protoConversions.state.telemetrySettingConversion.describe",
        &json!({"direction": "to_proto"}),
    );
    assert!(telemetry_setting.ok);
    assert_eq!(
        telemetry_setting
            .payload
            .unwrap_or_else(|| json!({}))
            .get("direction")
            .and_then(Value::as_str),
        Some("to_proto")
    );

    let open_graph = run_action(
        root.path(),
        "dashboard.prompts.system.shared.protoConversions.web.openGraphConversion.describe",
        &json!({"mode": "summary"}),
    );
    assert!(open_graph.ok);
    assert_eq!(
        open_graph
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("summary")
    );

    let providers_bedrock = run_action(
        root.path(),
        "dashboard.prompts.system.shared.providers.bedrock.describe",
        &json!({"profile": "default"}),
    );
    assert!(providers_bedrock.ok);
    assert_eq!(
        providers_bedrock
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("default")
    );

    let providers_index = run_action(
        root.path(),
        "dashboard.prompts.system.shared.providers.providers.describe",
        &json!({"catalog": "primary"}),
    );
    assert!(providers_index.ok);
    assert_eq!(
        providers_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("catalog")
            .and_then(Value::as_str),
        Some("primary")
    );

    let providers_vertex = run_action(
        root.path(),
        "dashboard.prompts.system.shared.providers.vertex.describe",
        &json!({"region": "us-central1"}),
    );
    assert!(providers_vertex.ok);
    assert_eq!(
        providers_vertex
            .payload
            .unwrap_or_else(|| json!({}))
            .get("region")
            .and_then(Value::as_str),
        Some("us-central1")
    );

    let remote_constants = run_action(
        root.path(),
        "dashboard.prompts.system.shared.remoteConfig.constants.describe",
        &json!({}),
    );
    assert!(remote_constants.ok);
    assert_eq!(
        remote_constants
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_shared_remote_config_constants_describe")
    );

    let remote_schema = run_action(
        root.path(),
        "dashboard.prompts.system.shared.remoteConfig.schema.describe",
        &json!({"schema": "remote_config_v1"}),
    );
    assert!(remote_schema.ok);
    assert_eq!(
        remote_schema
            .payload
            .unwrap_or_else(|| json!({}))
            .get("schema")
            .and_then(Value::as_str),
        Some("remote_config_v1")
    );

    let services_logger = run_action(
        root.path(),
        "dashboard.prompts.system.shared.services.logger.describe",
        &json!({"level": "info"}),
    );
    assert!(services_logger.ok);
    assert_eq!(
        services_logger
            .payload
            .unwrap_or_else(|| json!({}))
            .get("level")
            .and_then(Value::as_str),
        Some("info")
    );

    let services_session = run_action(
        root.path(),
        "dashboard.prompts.system.shared.services.session.describe",
        &json!({"session_scope": "workspace"}),
    );
    assert!(services_session.ok);
    assert_eq!(
        services_session
            .payload
            .unwrap_or_else(|| json!({}))
            .get("session_scope")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let services_otel = run_action(
        root.path(),
        "dashboard.prompts.system.shared.services.config.otelConfig.describe",
        &json!({"exporter": "otlp_http"}),
    );
    assert!(services_otel.ok);
    assert_eq!(
        services_otel
            .payload
            .unwrap_or_else(|| json!({}))
            .get("exporter")
            .and_then(Value::as_str),
        Some("otlp_http")
    );
}
