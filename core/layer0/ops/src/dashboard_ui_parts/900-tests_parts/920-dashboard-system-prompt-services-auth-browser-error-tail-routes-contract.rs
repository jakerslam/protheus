#[test]
fn dashboard_system_prompt_services_auth_browser_error_tail_routes_contract_wave_920() {
    let root = tempfile::tempdir().expect("tempdir");

    let auth_types = run_action(
        root.path(),
        "dashboard.prompts.system.services.auth.types.describe",
        &json!({"type_set": "core"}),
    );
    assert!(auth_types.ok);
    assert_eq!(
        auth_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type_set")
            .and_then(Value::as_str),
        Some("core")
    );

    let banner_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.banner.bannerService.describe",
        &json!({"banner_mode": "default"}),
    );
    assert!(banner_service.ok);
    assert_eq!(
        banner_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("banner_mode")
            .and_then(Value::as_str),
        Some("default")
    );

    let browser_discovery = run_action(
        root.path(),
        "dashboard.prompts.system.services.browser.browserDiscovery.describe",
        &json!({"discovery_mode": "auto"}),
    );
    assert!(browser_discovery.ok);
    assert_eq!(
        browser_discovery
            .payload
            .unwrap_or_else(|| json!({}))
            .get("discovery_mode")
            .and_then(Value::as_str),
        Some("auto")
    );

    let browser_session = run_action(
        root.path(),
        "dashboard.prompts.system.services.browser.browserSession.describe",
        &json!({"session_mode": "ephemeral"}),
    );
    assert!(browser_session.ok);
    assert_eq!(
        browser_session
            .payload
            .unwrap_or_else(|| json!({}))
            .get("session_mode")
            .and_then(Value::as_str),
        Some("ephemeral")
    );

    let url_content_fetcher = run_action(
        root.path(),
        "dashboard.prompts.system.services.browser.urlContentFetcher.describe",
        &json!({"fetch_mode": "safe"}),
    );
    assert!(url_content_fetcher.ok);
    assert_eq!(
        url_content_fetcher
            .payload
            .unwrap_or_else(|| json!({}))
            .get("fetch_mode")
            .and_then(Value::as_str),
        Some("safe")
    );

    let browser_utils = run_action(
        root.path(),
        "dashboard.prompts.system.services.browser.utils.describe",
        &json!({"utils_profile": "default"}),
    );
    assert!(browser_utils.ok);
    assert_eq!(
        browser_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("utils_profile")
            .and_then(Value::as_str),
        Some("default")
    );

    let cline_error = run_action(
        root.path(),
        "dashboard.prompts.system.services.error.clineError.describe",
        &json!({"severity": "error"}),
    );
    assert!(cline_error.ok);
    assert_eq!(
        cline_error
            .payload
            .unwrap_or_else(|| json!({}))
            .get("severity")
            .and_then(Value::as_str),
        Some("error")
    );

    let error_provider_factory = run_action(
        root.path(),
        "dashboard.prompts.system.services.error.errorProviderFactory.describe",
        &json!({"provider": "default"}),
    );
    assert!(error_provider_factory.ok);
    assert_eq!(
        error_provider_factory
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("default")
    );

    let error_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.error.errorService.describe",
        &json!({"service_mode": "standard"}),
    );
    assert!(error_service.ok);
    assert_eq!(
        error_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("service_mode")
            .and_then(Value::as_str),
        Some("standard")
    );

    let error_index = run_action(
        root.path(),
        "dashboard.prompts.system.services.error.index.describe",
        &json!({"export_set": "all"}),
    );
    assert!(error_index.ok);
    assert_eq!(
        error_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("export_set")
            .and_then(Value::as_str),
        Some("all")
    );
}
