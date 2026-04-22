#[test]
fn dashboard_hooks_registry_and_discovery_cache_routes_persist_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let register = run_action(
        root.path(),
        "dashboard.hooks.registry.register",
        &json!({
            "hook_id": "hook.pretool.policy_gate",
            "phase": "pre_tool_use",
            "command": "cargo check",
            "description": "Fail closed on unsafe tool attempts",
            "enabled": true
        }),
    );
    assert!(register.ok);
    let register_payload = register.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        register_payload.get("type").and_then(Value::as_str),
        Some("dashboard_hooks_registry_register")
    );
    assert_eq!(
        register_payload
            .pointer("/hook/hook_id")
            .and_then(Value::as_str),
        Some("hook.pretool.policy_gate")
    );

    let list = run_action(root.path(), "dashboard.hooks.registry.list", &json!({}));
    assert!(list.ok);
    let list_payload = list.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        list_payload.get("count").and_then(Value::as_i64),
        Some(1)
    );

    let refresh = run_action(
        root.path(),
        "dashboard.hooks.discoveryCache.refresh",
        &json!({}),
    );
    assert!(refresh.ok);
    let refresh_payload = refresh.payload.unwrap_or_else(|| json!({}));
    assert!(
        refresh_payload
            .get("count")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= 1
    );

    let cache = run_action(root.path(), "dashboard.hooks.discoveryCache.get", &json!({}));
    assert!(cache.ok);
    let cache_payload = cache.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        cache_payload.get("type").and_then(Value::as_str),
        Some("dashboard_hooks_discovery_cache_get")
    );
    assert!(
        cache_payload
            .get("count")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= 1
    );
}

#[test]
fn dashboard_hooks_process_registry_and_cancellation_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let start = run_action(
        root.path(),
        "dashboard.hooks.process.start",
        &json!({
            "hook_id": "hook.pretool.policy_gate",
            "phase": "pre_tool_use",
            "context": "user requested file write"
        }),
    );
    assert!(start.ok);
    let start_payload = start.payload.unwrap_or_else(|| json!({}));
    let run_id = clean_text(
        start_payload
            .pointer("/run/run_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    assert!(!run_id.is_empty());

    let complete = run_action(
        root.path(),
        "dashboard.hooks.process.complete",
        &json!({
            "run_id": run_id,
            "error_code": "pre_tool_use_cancelled",
            "message": "policy denied tool invocation"
        }),
    );
    assert!(complete.ok);
    let complete_payload = complete.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        complete_payload
            .get("pre_tool_use_cancelled")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        complete_payload
            .get("error_class")
            .and_then(Value::as_str),
        Some("pre_tool_use_cancellation")
    );

    let registry = run_action(root.path(), "dashboard.hooks.process.registry", &json!({}));
    assert!(registry.ok);
    let registry_payload = registry.payload.unwrap_or_else(|| json!({}));
    assert!(
        registry_payload
            .get("run_count")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= 1
    );
    assert_eq!(
        registry_payload
            .pointer("/runs/0/pre_tool_use_cancelled")
            .and_then(Value::as_bool),
        Some(true)
    );
}
