fn dashboard_prompt_services_error_providers_i_error_provider_describe(payload: &Value) -> Value {
    let provider_contract = clean_text(
        payload
            .get("provider_contract")
            .and_then(Value::as_str)
            .unwrap_or("error_provider"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_error_providers_i_error_provider_describe",
        "provider_contract": provider_contract
    })
}

fn dashboard_prompt_services_error_providers_post_hog_error_provider_describe(
    payload: &Value,
) -> Value {
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
        "type": "dashboard_prompts_system_services_error_providers_post_hog_error_provider_describe",
        "provider": provider
    })
}

fn dashboard_prompt_services_feature_flags_provider_factory_describe(payload: &Value) -> Value {
    let factory_mode = clean_text(
        payload
            .get("factory_mode")
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_feature_flags_provider_factory_describe",
        "factory_mode": factory_mode
    })
}

fn dashboard_prompt_services_feature_flags_service_describe(payload: &Value) -> Value {
    let flags_mode = clean_text(
        payload
            .get("flags_mode")
            .and_then(Value::as_str)
            .unwrap_or("runtime"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_feature_flags_service_describe",
        "flags_mode": flags_mode
    })
}

fn dashboard_prompt_services_feature_flags_index_describe(payload: &Value) -> Value {
    let export_set = clean_text(
        payload
            .get("export_set")
            .and_then(Value::as_str)
            .unwrap_or("all"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_feature_flags_index_describe",
        "export_set": export_set
    })
}

fn dashboard_prompt_services_feature_flags_providers_i_feature_flags_provider_describe(
    payload: &Value,
) -> Value {
    let provider_contract = clean_text(
        payload
            .get("provider_contract")
            .and_then(Value::as_str)
            .unwrap_or("feature_flags_provider"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_feature_flags_providers_i_feature_flags_provider_describe",
        "provider_contract": provider_contract
    })
}

fn dashboard_prompt_services_feature_flags_providers_post_hog_feature_flags_provider_describe(
    payload: &Value,
) -> Value {
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
        "type": "dashboard_prompts_system_services_feature_flags_providers_post_hog_feature_flags_provider_describe",
        "provider": provider
    })
}

fn dashboard_prompt_services_glob_list_files_describe(payload: &Value) -> Value {
    let glob_mode = clean_text(
        payload
            .get("glob_mode")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_glob_list_files_describe",
        "glob_mode": glob_mode
    })
}

fn dashboard_prompt_services_logging_distinct_id_describe(payload: &Value) -> Value {
    let distinct_id_mode = clean_text(
        payload
            .get("distinct_id_mode")
            .and_then(Value::as_str)
            .unwrap_or("stable"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_logging_distinct_id_describe",
        "distinct_id_mode": distinct_id_mode
    })
}

fn dashboard_prompt_services_mcp_mcp_hub_describe(payload: &Value) -> Value {
    let hub_mode = clean_text(
        payload
            .get("hub_mode")
            .and_then(Value::as_str)
            .unwrap_or("managed"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_mcp_mcp_hub_describe",
        "hub_mode": hub_mode
    })
}

fn dashboard_prompt_hosts_surface_tail_services_error_feature_flags_glob_logging_mcp_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.services.error.providers.iErrorProvider.describe" => {
            Some(dashboard_prompt_services_error_providers_i_error_provider_describe(payload))
        }
        "dashboard.prompts.system.services.error.providers.postHogErrorProvider.describe" => {
            Some(dashboard_prompt_services_error_providers_post_hog_error_provider_describe(
                payload,
            ))
        }
        "dashboard.prompts.system.services.featureFlags.featureFlagsProviderFactory.describe" => {
            Some(dashboard_prompt_services_feature_flags_provider_factory_describe(payload))
        }
        "dashboard.prompts.system.services.featureFlags.featureFlagsService.describe" => {
            Some(dashboard_prompt_services_feature_flags_service_describe(payload))
        }
        "dashboard.prompts.system.services.featureFlags.index.describe" => {
            Some(dashboard_prompt_services_feature_flags_index_describe(payload))
        }
        "dashboard.prompts.system.services.featureFlags.providers.iFeatureFlagsProvider.describe" => {
            Some(
                dashboard_prompt_services_feature_flags_providers_i_feature_flags_provider_describe(
                    payload,
                ),
            )
        }
        "dashboard.prompts.system.services.featureFlags.providers.postHogFeatureFlagsProvider.describe" => {
            Some(
                dashboard_prompt_services_feature_flags_providers_post_hog_feature_flags_provider_describe(
                    payload,
                ),
            )
        }
        "dashboard.prompts.system.services.glob.listFiles.describe" => {
            Some(dashboard_prompt_services_glob_list_files_describe(payload))
        }
        "dashboard.prompts.system.services.logging.distinctId.describe" => {
            Some(dashboard_prompt_services_logging_distinct_id_describe(payload))
        }
        "dashboard.prompts.system.services.mcp.mcpHub.describe" => {
            Some(dashboard_prompt_services_mcp_mcp_hub_describe(payload))
        }
        _ => dashboard_prompt_hosts_surface_tail_services_mcp_ripgrep_telemetry_route_extension(
            root, normalized, payload,
        ),
    }
}
include!("086-dashboard-system-prompt-services-mcp-ripgrep-telemetry-tail-helpers.rs");
