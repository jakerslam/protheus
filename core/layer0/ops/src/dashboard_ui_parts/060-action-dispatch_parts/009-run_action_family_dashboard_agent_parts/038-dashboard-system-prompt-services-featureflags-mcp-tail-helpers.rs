fn dashboard_prompt_services_error_i_provider_describe(payload: &Value) -> Value {
    let provider_key = clean_text(
        payload
            .get("provider_key")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_error_i_provider_describe",
        "provider_key": provider_key
    })
}

fn dashboard_prompt_services_error_posthog_provider_describe(payload: &Value) -> Value {
    let event = clean_text(
        payload
            .get("event")
            .and_then(Value::as_str)
            .unwrap_or("runtime_error"),
        200,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_error_posthog_provider_describe",
        "event": event
    })
}

fn dashboard_prompt_services_feature_flags_provider_factory_describe(payload: &Value) -> Value {
    let provider = clean_text(
        payload
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("posthog"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_feature_flags_provider_factory_describe",
        "provider": provider
    })
}

fn dashboard_prompt_services_feature_flags_service_describe(payload: &Value) -> Value {
    let flag_key = clean_text(payload.get("flag_key").and_then(Value::as_str).unwrap_or(""), 220);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_feature_flags_service_describe",
        "flag_key": flag_key
    })
}

fn dashboard_prompt_services_feature_flags_index_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_feature_flags_index_describe",
        "exports": ["feature_flags_service", "provider_factory", "providers"]
    })
}

fn dashboard_prompt_services_feature_flags_i_provider_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_feature_flags_i_provider_describe",
        "contract": "feature_flags_provider"
    })
}

fn dashboard_prompt_services_feature_flags_posthog_provider_describe(payload: &Value) -> Value {
    let distinct_id = clean_text(
        payload
            .get("distinct_id")
            .and_then(Value::as_str)
            .unwrap_or("anonymous"),
        200,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_feature_flags_posthog_provider_describe",
        "distinct_id": distinct_id
    })
}

fn dashboard_prompt_services_glob_list_files_describe(payload: &Value) -> Value {
    let pattern = clean_text(
        payload
            .get("pattern")
            .and_then(Value::as_str)
            .unwrap_or("**/*"),
        400,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_glob_list_files_describe",
        "pattern": pattern
    })
}

fn dashboard_prompt_services_logging_distinct_id_describe(payload: &Value) -> Value {
    let seed = clean_text(payload.get("seed").and_then(Value::as_str).unwrap_or("runtime"), 200);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_logging_distinct_id_describe",
        "seed": seed
    })
}

fn dashboard_prompt_services_mcp_hub_describe(payload: &Value) -> Value {
    let command = clean_text(
        payload
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("status"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_hub_describe",
        "command": command
    })
}

fn dashboard_prompt_services_featureflags_mcp_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.services.error.providers.iErrorProvider.describe" => {
            Some(dashboard_prompt_services_error_i_provider_describe(payload))
        }
        "dashboard.prompts.system.services.error.providers.postHogErrorProvider.describe" => {
            Some(dashboard_prompt_services_error_posthog_provider_describe(payload))
        }
        "dashboard.prompts.system.services.featureFlags.featureFlagsProviderFactory.describe" => {
            Some(dashboard_prompt_services_feature_flags_provider_factory_describe(payload))
        }
        "dashboard.prompts.system.services.featureFlags.featureFlagsService.describe" => {
            Some(dashboard_prompt_services_feature_flags_service_describe(payload))
        }
        "dashboard.prompts.system.services.featureFlags.index.describe" => {
            Some(dashboard_prompt_services_feature_flags_index_describe())
        }
        "dashboard.prompts.system.services.featureFlags.providers.iFeatureFlagsProvider.describe" => {
            Some(dashboard_prompt_services_feature_flags_i_provider_describe())
        }
        "dashboard.prompts.system.services.featureFlags.providers.postHogFeatureFlagsProvider.describe" => {
            Some(dashboard_prompt_services_feature_flags_posthog_provider_describe(payload))
        }
        "dashboard.prompts.system.services.glob.listFiles.describe" => {
            Some(dashboard_prompt_services_glob_list_files_describe(payload))
        }
        "dashboard.prompts.system.services.logging.distinctId.describe" => {
            Some(dashboard_prompt_services_logging_distinct_id_describe(payload))
        }
        "dashboard.prompts.system.services.mcp.mcpHub.describe" => {
            Some(dashboard_prompt_services_mcp_hub_describe(payload))
        }
        _ => dashboard_prompt_services_mcp_ripgrep_telemetry_tail_route_extension(
            root, normalized, payload,
        ),
    }
}

include!("039-dashboard-system-prompt-services-mcp-ripgrep-telemetry-tail-helpers.rs");
