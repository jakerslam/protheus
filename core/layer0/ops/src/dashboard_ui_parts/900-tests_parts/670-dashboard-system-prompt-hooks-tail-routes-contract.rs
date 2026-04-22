#[test]
fn dashboard_system_prompt_hooks_tail_routes_contract_wave_670() {
    let root = tempfile::tempdir().expect("tempdir");

    let hook_discovery_cache = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.hookDiscoveryCache.describe",
        &json!({"cache_mode": "bounded"}),
    );
    assert!(hook_discovery_cache.ok);
    assert_eq!(
        hook_discovery_cache
            .payload
            .unwrap_or_else(|| json!({}))
            .get("cache_mode")
            .and_then(Value::as_str),
        Some("bounded")
    );

    let hook_error = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.hookError.describe",
        &json!({"error_kind": "hook_error"}),
    );
    assert!(hook_error.ok);
    assert_eq!(
        hook_error
            .payload
            .unwrap_or_else(|| json!({}))
            .get("error_kind")
            .and_then(Value::as_str),
        Some("hook_error")
    );

    let hook_process = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.hookProcess.describe",
        &json!({"process_mode": "managed"}),
    );
    assert!(hook_process.ok);
    assert_eq!(
        hook_process
            .payload
            .unwrap_or_else(|| json!({}))
            .get("process_mode")
            .and_then(Value::as_str),
        Some("managed")
    );

    let hook_process_registry = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.hookProcessRegistry.describe",
        &json!({"registry_scope": "session"}),
    );
    assert!(hook_process_registry.ok);
    assert_eq!(
        hook_process_registry
            .payload
            .unwrap_or_else(|| json!({}))
            .get("registry_scope")
            .and_then(Value::as_str),
        Some("session")
    );

    let pre_tool_use_hook_cancellation = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.preToolUseHookCancellationError.describe",
        &json!({"cancel_mode": "fail_closed"}),
    );
    assert!(pre_tool_use_hook_cancellation.ok);
    assert_eq!(
        pre_tool_use_hook_cancellation
            .payload
            .unwrap_or_else(|| json!({}))
            .get("cancel_mode")
            .and_then(Value::as_str),
        Some("fail_closed")
    );

    let hook_factory_test = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.hookFactory.describe",
        &json!({"suite": "hook_factory"}),
    );
    assert!(hook_factory_test.ok);
    assert_eq!(
        hook_factory_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("suite")
            .and_then(Value::as_str),
        Some("hook_factory")
    );

    let hook_model_context_test = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.hookModelContext.describe",
        &json!({"context_mode": "model_context"}),
    );
    assert!(hook_model_context_test.ok);
    assert_eq!(
        hook_model_context_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("context_mode")
            .and_then(Value::as_str),
        Some("model_context")
    );

    let hook_process_test = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.hookProcess.describe",
        &json!({"suite": "hook_process"}),
    );
    assert!(hook_process_test.ok);
    assert_eq!(
        hook_process_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("suite")
            .and_then(Value::as_str),
        Some("hook_process")
    );

    let hooks_utils_test = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.hooksUtils.describe",
        &json!({"utility": "hooks_utils"}),
    );
    assert!(hooks_utils_test.ok);
    assert_eq!(
        hooks_utils_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("utility")
            .and_then(Value::as_str),
        Some("hooks_utils")
    );

    let notification_hook_test = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.notificationHook.describe",
        &json!({"channel": "notification_hook"}),
    );
    assert!(notification_hook_test.ok);
    assert_eq!(
        notification_hook_test
            .payload
            .unwrap_or_else(|| json!({}))
            .get("channel")
            .and_then(Value::as_str),
        Some("notification_hook")
    );
}
