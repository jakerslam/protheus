fn dashboard_prompt_services_auth_types_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_auth_types_describe",
        "contracts": ["auth_context", "auth_result", "auth_provider"]
    })
}

fn dashboard_prompt_services_banner_service_describe(payload: &Value) -> Value {
    let banner_id = clean_text(
        payload
            .get("banner_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_banner_service_describe",
        "banner_id": banner_id
    })
}

fn dashboard_prompt_services_browser_discovery_describe(payload: &Value) -> Value {
    let strategy = clean_text(
        payload
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_browser_discovery_describe",
        "strategy": strategy
    })
}

fn dashboard_prompt_services_browser_session_describe(payload: &Value) -> Value {
    let session_id = clean_text(
        payload
            .get("session_id")
            .and_then(Value::as_str)
            .unwrap_or("session-default"),
        180,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_browser_session_describe",
        "session_id": session_id
    })
}

fn dashboard_prompt_services_url_content_fetcher_describe(payload: &Value) -> Value {
    let url = clean_text(payload.get("url").and_then(Value::as_str).unwrap_or(""), 1400);
    let timeout_ms = payload
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(15000);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_url_content_fetcher_describe",
        "url": url,
        "timeout_ms": timeout_ms
    })
}

fn dashboard_prompt_services_browser_utils_describe(payload: &Value) -> Value {
    let operation = clean_text(
        payload
            .get("operation")
            .and_then(Value::as_str)
            .unwrap_or("normalize"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_browser_utils_describe",
        "operation": operation
    })
}

fn dashboard_prompt_services_error_cline_error_describe(payload: &Value) -> Value {
    let code = clean_text(
        payload
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("unknown_error"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_error_cline_error_describe",
        "code": code
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
    let level = clean_text(
        payload
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("error"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_error_service_describe",
        "level": level
    })
}

fn dashboard_prompt_services_error_index_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_error_index_describe",
        "exports": ["error_service", "provider_factory", "providers"]
    })
}

fn dashboard_prompt_services_auth_browser_error_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.services.auth.types.describe" => {
            Some(dashboard_prompt_services_auth_types_describe())
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
            Some(dashboard_prompt_services_url_content_fetcher_describe(payload))
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
            Some(dashboard_prompt_services_error_index_describe())
        }
        _ => dashboard_prompt_services_featureflags_mcp_tail_route_extension(root, normalized, payload),
    }
}

include!("038-dashboard-system-prompt-services-featureflags-mcp-tail-helpers.rs");
