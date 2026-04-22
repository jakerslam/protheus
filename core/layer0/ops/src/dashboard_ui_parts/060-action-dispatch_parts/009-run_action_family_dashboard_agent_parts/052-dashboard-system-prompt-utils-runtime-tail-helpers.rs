fn dashboard_prompt_utils_model_utils_describe(payload: &Value) -> Value {
    let family = clean_text(
        payload
            .get("family")
            .and_then(Value::as_str)
            .unwrap_or("gpt"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_model_utils_describe",
        "family": family
    })
}

fn dashboard_prompt_utils_path_describe(payload: &Value) -> Value {
    let scope = clean_text(
        payload
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_path_describe",
        "scope": scope
    })
}

fn dashboard_prompt_utils_powershell_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("compatible"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_powershell_describe",
        "mode": mode
    })
}

fn dashboard_prompt_utils_process_termination_describe(payload: &Value) -> Value {
    let strategy = clean_text(
        payload
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or("graceful_then_force"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_process_termination_describe",
        "strategy": strategy
    })
}

fn dashboard_prompt_utils_retry_describe(payload: &Value) -> Value {
    let policy = clean_text(
        payload
            .get("policy")
            .and_then(Value::as_str)
            .unwrap_or("bounded_exponential"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_retry_describe",
        "policy": policy
    })
}

fn dashboard_prompt_utils_shell_describe(payload: &Value) -> Value {
    let shell = clean_text(
        payload
            .get("shell")
            .and_then(Value::as_str)
            .unwrap_or("zsh"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_shell_describe",
        "shell": shell
    })
}

fn dashboard_prompt_utils_storage_describe(payload: &Value) -> Value {
    let backend = clean_text(
        payload
            .get("backend")
            .and_then(Value::as_str)
            .unwrap_or("sqlite"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_storage_describe",
        "backend": backend
    })
}

fn dashboard_prompt_utils_string_describe(payload: &Value) -> Value {
    let operation = clean_text(
        payload
            .get("operation")
            .and_then(Value::as_str)
            .unwrap_or("trim"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_string_describe",
        "operation": operation
    })
}

fn dashboard_prompt_utils_tab_filtering_describe(payload: &Value) -> Value {
    let filter = clean_text(
        payload
            .get("filter")
            .and_then(Value::as_str)
            .unwrap_or("visible"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_tab_filtering_describe",
        "filter": filter
    })
}

fn dashboard_prompt_utils_time_describe(payload: &Value) -> Value {
    let clock = clean_text(
        payload
            .get("clock")
            .and_then(Value::as_str)
            .unwrap_or("utc"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_utils_time_describe",
        "clock": clock
    })
}

fn dashboard_prompt_utils_runtime_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.utils.modelUtils.describe" => {
            Some(dashboard_prompt_utils_model_utils_describe(payload))
        }
        "dashboard.prompts.system.utils.path.describe" => {
            Some(dashboard_prompt_utils_path_describe(payload))
        }
        "dashboard.prompts.system.utils.powershell.describe" => {
            Some(dashboard_prompt_utils_powershell_describe(payload))
        }
        "dashboard.prompts.system.utils.processTermination.describe" => {
            Some(dashboard_prompt_utils_process_termination_describe(payload))
        }
        "dashboard.prompts.system.utils.retry.describe" => {
            Some(dashboard_prompt_utils_retry_describe(payload))
        }
        "dashboard.prompts.system.utils.shell.describe" => {
            Some(dashboard_prompt_utils_shell_describe(payload))
        }
        "dashboard.prompts.system.utils.storage.describe" => {
            Some(dashboard_prompt_utils_storage_describe(payload))
        }
        "dashboard.prompts.system.utils.string.describe" => {
            Some(dashboard_prompt_utils_string_describe(payload))
        }
        "dashboard.prompts.system.utils.tabFiltering.describe" => {
            Some(dashboard_prompt_utils_tab_filtering_describe(payload))
        }
        "dashboard.prompts.system.utils.time.describe" => {
            Some(dashboard_prompt_utils_time_describe(payload))
        }
        _ => dashboard_prompt_webview_account_tail_route_extension(root, normalized, payload),
    }
}

include!("053-dashboard-system-prompt-webview-account-tail-helpers.rs");
