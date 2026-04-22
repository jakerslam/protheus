fn dashboard_prompt_hooks_model_context_describe(payload: &Value) -> Value {
    let context_window = clean_text(
        payload
            .get("context_window")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_model_context_describe",
        "context_window": context_window
    })
}

fn dashboard_prompt_hooks_runtime_utils_describe(payload: &Value) -> Value {
    let helper = clean_text(
        payload
            .get("helper")
            .and_then(Value::as_str)
            .unwrap_or("normalize"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_runtime_utils_describe",
        "helper": helper
    })
}

fn dashboard_prompt_hooks_notification_hook_describe(payload: &Value) -> Value {
    let delivery_channel = clean_text(
        payload
            .get("delivery_channel")
            .and_then(Value::as_str)
            .unwrap_or("inline"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_notification_hook_describe",
        "delivery_channel": delivery_channel
    })
}

fn dashboard_prompt_hooks_precompact_executor_describe(payload: &Value) -> Value {
    let compaction_mode = clean_text(
        payload
            .get("compaction_mode")
            .and_then(Value::as_str)
            .unwrap_or("bounded"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_precompact_executor_describe",
        "compaction_mode": compaction_mode
    })
}

fn dashboard_prompt_hooks_shell_escape_runtime_describe(payload: &Value) -> Value {
    let quote_style = clean_text(
        payload
            .get("quote_style")
            .and_then(Value::as_str)
            .unwrap_or("single"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_shell_escape_runtime_describe",
        "quote_style": quote_style
    })
}

fn dashboard_prompt_hooks_templates_describe(payload: &Value) -> Value {
    let template_set = clean_text(
        payload
            .get("template_set")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_templates_describe",
        "template_set": template_set
    })
}

fn dashboard_prompt_hooks_misc_utils_describe(payload: &Value) -> Value {
    let utility_scope = clean_text(
        payload
            .get("utility_scope")
            .and_then(Value::as_str)
            .unwrap_or("hooks"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hooks_misc_utils_describe",
        "utility_scope": utility_scope
    })
}

fn dashboard_prompt_ignore_controller_describe(payload: &Value) -> Value {
    let ignore_profile = clean_text(
        payload
            .get("ignore_profile")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_ignore_controller_describe",
        "ignore_profile": ignore_profile
    })
}

fn dashboard_prompt_locks_folder_utils_describe(payload: &Value) -> Value {
    let lock_scope = clean_text(
        payload
            .get("lock_scope")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_locks_folder_utils_describe",
        "lock_scope": lock_scope
    })
}

fn dashboard_prompt_locks_sqlite_manager_describe(payload: &Value) -> Value {
    let lock_backend = clean_text(
        payload
            .get("lock_backend")
            .and_then(Value::as_str)
            .unwrap_or("sqlite"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_locks_sqlite_manager_describe",
        "lock_backend": lock_backend
    })
}

fn dashboard_prompt_hooks_runtime_locks_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hooks.hookModelContext.describe" => {
            Some(dashboard_prompt_hooks_model_context_describe(payload))
        }
        "dashboard.prompts.system.hooks.hooksUtils.describe" => {
            Some(dashboard_prompt_hooks_runtime_utils_describe(payload))
        }
        "dashboard.prompts.system.hooks.notificationHook.describe" => {
            Some(dashboard_prompt_hooks_notification_hook_describe(payload))
        }
        "dashboard.prompts.system.hooks.precompactExecutor.describe" => {
            Some(dashboard_prompt_hooks_precompact_executor_describe(payload))
        }
        "dashboard.prompts.system.hooks.shellEscape.describe" => {
            Some(dashboard_prompt_hooks_shell_escape_runtime_describe(payload))
        }
        "dashboard.prompts.system.hooks.templates.describe" => {
            Some(dashboard_prompt_hooks_templates_describe(payload))
        }
        "dashboard.prompts.system.hooks.utils.describe" => {
            Some(dashboard_prompt_hooks_misc_utils_describe(payload))
        }
        "dashboard.prompts.system.ignore.clineIgnoreController.describe" => {
            Some(dashboard_prompt_ignore_controller_describe(payload))
        }
        "dashboard.prompts.system.locks.folderLockUtils.describe" => {
            Some(dashboard_prompt_locks_folder_utils_describe(payload))
        }
        "dashboard.prompts.system.locks.sqliteLockManager.describe" => {
            Some(dashboard_prompt_locks_sqlite_manager_describe(payload))
        }
        _ => dashboard_prompt_locks_mentions_permissions_prompts_tail_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}

include!("062-dashboard-system-prompt-locks-mentions-permissions-prompts-tail-helpers.rs");
