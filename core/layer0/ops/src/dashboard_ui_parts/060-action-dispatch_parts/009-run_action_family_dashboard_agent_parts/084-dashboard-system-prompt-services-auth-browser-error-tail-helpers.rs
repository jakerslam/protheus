fn dashboard_prompt_services_auth_types_describe(payload: &Value) -> Value {
    let type_set = clean_text(
        payload
            .get("type_set")
            .and_then(Value::as_str)
            .unwrap_or("core"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_auth_types_describe",
        "type_set": type_set
    })
}

fn dashboard_prompt_services_banner_service_describe(payload: &Value) -> Value {
    let banner_mode = clean_text(
        payload
            .get("banner_mode")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_banner_service_describe",
        "banner_mode": banner_mode
    })
}

fn dashboard_prompt_services_browser_discovery_describe(payload: &Value) -> Value {
    let discovery_mode = clean_text(
        payload
            .get("discovery_mode")
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_browser_discovery_describe",
        "discovery_mode": discovery_mode
    })
}

fn dashboard_prompt_services_browser_session_describe(payload: &Value) -> Value {
    let session_mode = clean_text(
        payload
            .get("session_mode")
            .and_then(Value::as_str)
            .unwrap_or("ephemeral"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_browser_session_describe",
        "session_mode": session_mode
    })
}

fn dashboard_prompt_services_browser_url_content_fetcher_describe(payload: &Value) -> Value {
    let fetch_mode = clean_text(
        payload
            .get("fetch_mode")
            .and_then(Value::as_str)
            .unwrap_or("safe"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_browser_url_content_fetcher_describe",
        "fetch_mode": fetch_mode
    })
}

fn dashboard_prompt_services_browser_utils_describe(payload: &Value) -> Value {
    let utils_profile = clean_text(
        payload
            .get("utils_profile")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_browser_utils_describe",
        "utils_profile": utils_profile
    })
}

fn dashboard_prompt_services_error_cline_error_describe(payload: &Value) -> Value {
    let severity = clean_text(
        payload
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("error"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_error_cline_error_describe",
        "severity": severity
    })
}

fn dashboard_prompt_services_error_provider_factory_describe(payload: &Value) -> Value {
    let provider = clean_text(
        payload
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_error_provider_factory_describe",
        "provider": provider
    })
}

fn dashboard_prompt_services_error_service_describe(payload: &Value) -> Value {
    let service_mode = clean_text(
        payload
            .get("service_mode")
            .and_then(Value::as_str)
            .unwrap_or("standard"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_error_service_describe",
        "service_mode": service_mode
    })
}

fn dashboard_prompt_services_error_index_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_services_error_index_describe",
        "export_set": export_set
    })
}

fn dashboard_prompt_hosts_surface_tail_services_auth_browser_error_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.services.auth.types.describe" => {
            Some(dashboard_prompt_services_auth_types_describe(payload))
        }
        "dashboard.prompts.system.services.banner.bannerService.describe" => {
            Some(dashboard_prompt_services_banner_service_describe(payload))
        }
        "dashboard.prompts.system.services.browser.browserDiscovery.describe" => {
            Some(dashboard_prompt_services_browser_discovery_describe(payload))
        }
        "dashboard.prompts.system.services.browser.browserSession.describe" => {
            Some(dashboard_prompt_services_browser_session_describe(payload))
        }
        "dashboard.prompts.system.services.browser.urlContentFetcher.describe" => {
            Some(dashboard_prompt_services_browser_url_content_fetcher_describe(payload))
        }
        "dashboard.prompts.system.services.browser.utils.describe" => {
            Some(dashboard_prompt_services_browser_utils_describe(payload))
        }
        "dashboard.prompts.system.services.error.clineError.describe" => {
            Some(dashboard_prompt_services_error_cline_error_describe(payload))
        }
        "dashboard.prompts.system.services.error.errorProviderFactory.describe" => {
            Some(dashboard_prompt_services_error_provider_factory_describe(payload))
        }
        "dashboard.prompts.system.services.error.errorService.describe" => {
            Some(dashboard_prompt_services_error_service_describe(payload))
        }
        "dashboard.prompts.system.services.error.index.describe" => {
            Some(dashboard_prompt_services_error_index_describe(payload))
        }
        _ => dashboard_prompt_hosts_surface_tail_services_error_feature_flags_glob_logging_mcp_route_extension(
            root, normalized, payload,
        ),
    }
}
include!("085-dashboard-system-prompt-services-error-featureflags-glob-logging-mcp-tail-helpers.rs");
