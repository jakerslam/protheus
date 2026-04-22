#[test]
fn dashboard_system_prompt_services_auth_browser_error_tail_routes_contract_wave_450() {
    let root = tempfile::tempdir().expect("tempdir");

    let auth_types = run_action(
        root.path(),
        "dashboard.prompts.system.services.auth.types.describe",
        &json!({}),
    );
    assert!(auth_types.ok);
    assert_eq!(
        auth_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_services_auth_types_describe")
    );

    let banner = run_action(
        root.path(),
        "dashboard.prompts.system.services.banner.bannerService.describe",
        &json!({"banner_id": "bnr-1"}),
    );
    assert!(banner.ok);
    assert_eq!(
        banner
            .payload
            .unwrap_or_else(|| json!({}))
            .get("banner_id")
            .and_then(Value::as_str),
        Some("bnr-1")
    );

    let browser_discovery = run_action(
        root.path(),
        "dashboard.prompts.system.services.browser.browserDiscovery.describe",
        &json!({"strategy": "auto"}),
    );
    assert!(browser_discovery.ok);
    assert_eq!(
        browser_discovery
            .payload
            .unwrap_or_else(|| json!({}))
            .get("strategy")
            .and_then(Value::as_str),
        Some("auto")
    );

    let browser_session = run_action(
        root.path(),
        "dashboard.prompts.system.services.browser.browserSession.describe",
        &json!({"session_id": "sess-1"}),
    );
    assert!(browser_session.ok);
    assert_eq!(
        browser_session
            .payload
            .unwrap_or_else(|| json!({}))
            .get("session_id")
            .and_then(Value::as_str),
        Some("sess-1")
    );

    let url_fetch = run_action(
        root.path(),
        "dashboard.prompts.system.services.browser.urlContentFetcher.describe",
        &json!({"url": "https://example.com", "timeout_ms": 10000}),
    );
    assert!(url_fetch.ok);
    let url_fetch_payload = url_fetch.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        url_fetch_payload.get("url").and_then(Value::as_str),
        Some("https://example.com")
    );
    assert_eq!(
        url_fetch_payload.get("timeout_ms").and_then(Value::as_u64),
        Some(10000)
    );

    let browser_utils = run_action(
        root.path(),
        "dashboard.prompts.system.services.browser.utils.describe",
        &json!({"operation": "normalize"}),
    );
    assert!(browser_utils.ok);
    assert_eq!(
        browser_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("operation")
            .and_then(Value::as_str),
        Some("normalize")
    );

    let cline_error = run_action(
        root.path(),
        "dashboard.prompts.system.services.error.clineError.describe",
        &json!({"code": "auth_failed"}),
    );
    assert!(cline_error.ok);
    assert_eq!(
        cline_error
            .payload
            .unwrap_or_else(|| json!({}))
            .get("code")
            .and_then(Value::as_str),
        Some("auth_failed")
    );

    let provider_factory = run_action(
        root.path(),
        "dashboard.prompts.system.services.error.errorProviderFactory.describe",
        &json!({"provider": "posthog"}),
    );
    assert!(provider_factory.ok);
    assert_eq!(
        provider_factory
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("posthog")
    );

    let error_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.error.errorService.describe",
        &json!({"level": "error"}),
    );
    assert!(error_service.ok);
    assert_eq!(
        error_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("level")
            .and_then(Value::as_str),
        Some("error")
    );

    let error_index = run_action(
        root.path(),
        "dashboard.prompts.system.services.error.index.describe",
        &json!({}),
    );
    assert!(error_index.ok);
    assert_eq!(
        error_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_services_error_index_describe")
    );
}
