fn dashboard_prompt_hooks_discovery_cache_describe(payload: &Value) -> Value {
    let cache_mode = clean_text(
        payload
            .get("cache_mode")
            .and_then(Value::as_str)
            .unwrap_or("bounded"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_discovery_cache_describe",
        "cache_mode": cache_mode
    })
}

fn dashboard_prompt_hooks_error_describe(payload: &Value) -> Value {
    let error_kind = clean_text(
        payload
            .get("error_kind")
            .and_then(Value::as_str)
            .unwrap_or("hook_error"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_error_describe",
        "error_kind": error_kind
    })
}

fn dashboard_prompt_hooks_process_describe(payload: &Value) -> Value {
    let process_mode = clean_text(
        payload
            .get("process_mode")
            .and_then(Value::as_str)
            .unwrap_or("managed"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_process_describe",
        "process_mode": process_mode
    })
}

fn dashboard_prompt_hooks_process_registry_describe(payload: &Value) -> Value {
    let registry_scope = clean_text(
        payload
            .get("registry_scope")
            .and_then(Value::as_str)
            .unwrap_or("session"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_process_registry_describe",
        "registry_scope": registry_scope
    })
}

fn dashboard_prompt_hooks_pre_tool_use_cancellation_describe(payload: &Value) -> Value {
    let cancel_mode = clean_text(
        payload
            .get("cancel_mode")
            .and_then(Value::as_str)
            .unwrap_or("fail_closed"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_pre_tool_use_cancellation_describe",
        "cancel_mode": cancel_mode
    })
}

fn dashboard_prompt_hooks_test_factory_describe(payload: &Value) -> Value {
    let suite = clean_text(
        payload
            .get("suite")
            .and_then(Value::as_str)
            .unwrap_or("hook_factory"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_factory_describe",
        "suite": suite
    })
}

fn dashboard_prompt_hooks_test_model_context_describe(payload: &Value) -> Value {
    let context_mode = clean_text(
        payload
            .get("context_mode")
            .and_then(Value::as_str)
            .unwrap_or("model_context"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_model_context_describe",
        "context_mode": context_mode
    })
}

fn dashboard_prompt_hooks_test_process_describe(payload: &Value) -> Value {
    let suite = clean_text(
        payload
            .get("suite")
            .and_then(Value::as_str)
            .unwrap_or("hook_process"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_process_describe",
        "suite": suite
    })
}

fn dashboard_prompt_hooks_test_utils_describe(payload: &Value) -> Value {
    let utility = clean_text(
        payload
            .get("utility")
            .and_then(Value::as_str)
            .unwrap_or("hooks_utils"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_utils_describe",
        "utility": utility
    })
}

fn dashboard_prompt_hooks_test_notification_describe(payload: &Value) -> Value {
    let channel = clean_text(
        payload
            .get("channel")
            .and_then(Value::as_str)
            .unwrap_or("notification_hook"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_test_notification_describe",
        "channel": channel
    })
}

fn dashboard_prompt_hooks_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hooks.hookDiscoveryCache.describe" => {
            Some(dashboard_prompt_hooks_discovery_cache_describe(payload))
        }
        "dashboard.prompts.system.hooks.hookError.describe" => {
            Some(dashboard_prompt_hooks_error_describe(payload))
        }
        "dashboard.prompts.system.hooks.hookProcess.describe" => {
            Some(dashboard_prompt_hooks_process_describe(payload))
        }
        "dashboard.prompts.system.hooks.hookProcessRegistry.describe" => {
            Some(dashboard_prompt_hooks_process_registry_describe(payload))
        }
        "dashboard.prompts.system.hooks.preToolUseHookCancellationError.describe" => {
            Some(dashboard_prompt_hooks_pre_tool_use_cancellation_describe(payload))
        }
        "dashboard.prompts.system.hooks.tests.hookFactory.describe" => {
            Some(dashboard_prompt_hooks_test_factory_describe(payload))
        }
        "dashboard.prompts.system.hooks.tests.hookModelContext.describe" => {
            Some(dashboard_prompt_hooks_test_model_context_describe(payload))
        }
        "dashboard.prompts.system.hooks.tests.hookProcess.describe" => {
            Some(dashboard_prompt_hooks_test_process_describe(payload))
        }
        "dashboard.prompts.system.hooks.tests.hooksUtils.describe" => {
            Some(dashboard_prompt_hooks_test_utils_describe(payload))
        }
        "dashboard.prompts.system.hooks.tests.notificationHook.describe" => {
            Some(dashboard_prompt_hooks_test_notification_describe(payload))
        }
        _ => dashboard_prompt_hooks_extended_tail_route_extension(root, normalized, payload),
    }
}

include!("060-dashboard-system-prompt-hooks-extended-tail-helpers.rs");
