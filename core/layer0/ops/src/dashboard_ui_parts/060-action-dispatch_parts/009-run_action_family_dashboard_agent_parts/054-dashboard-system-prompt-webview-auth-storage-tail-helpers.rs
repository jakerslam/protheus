fn dashboard_prompt_auth_oca_provider_describe(payload: &Value) -> Value {
    let realm = clean_text(
        payload
            .get("realm")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_auth_oca_provider_describe",
        "realm": realm
    })
}

fn dashboard_prompt_auth_oca_constants_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("base"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_auth_oca_constants_describe",
        "profile": profile
    })
}

fn dashboard_prompt_auth_oca_types_describe(payload: &Value) -> Value {
    let type_set = clean_text(
        payload
            .get("type_set")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_auth_oca_types_describe",
        "type_set": type_set
    })
}

fn dashboard_prompt_auth_oca_utils_describe(payload: &Value) -> Value {
    let utility = clean_text(
        payload
            .get("utility")
            .and_then(Value::as_str)
            .unwrap_or("normalize"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_auth_oca_utils_describe",
        "utility": utility
    })
}

fn dashboard_prompt_auth_cline_provider_describe(payload: &Value) -> Value {
    let provider = clean_text(
        payload
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("cline"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_auth_cline_provider_describe",
        "provider": provider
    })
}

fn dashboard_prompt_shared_storage_file_storage_describe(payload: &Value) -> Value {
    let medium = clean_text(
        payload
            .get("medium")
            .and_then(Value::as_str)
            .unwrap_or("disk"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_storage_file_storage_describe",
        "medium": medium
    })
}

fn dashboard_prompt_shared_storage_storage_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_shared_storage_storage_describe",
        "scope": scope
    })
}

fn dashboard_prompt_shared_storage_adapters_describe(payload: &Value) -> Value {
    let adapter = clean_text(
        payload
            .get("adapter")
            .and_then(Value::as_str)
            .unwrap_or("file"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_storage_adapters_describe",
        "adapter": adapter
    })
}

fn dashboard_prompt_shared_storage_index_describe(payload: &Value) -> Value {
    let index_mode = clean_text(
        payload
            .get("index_mode")
            .and_then(Value::as_str)
            .unwrap_or("provider-first"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_storage_index_describe",
        "index_mode": index_mode
    })
}

fn dashboard_prompt_shared_storage_provider_keys_describe(payload: &Value) -> Value {
    let key_set = clean_text(
        payload
            .get("key_set")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_storage_provider_keys_describe",
        "key_set": key_set
    })
}

fn dashboard_prompt_webview_auth_storage_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.auth.oca.provider.describe" => {
            Some(dashboard_prompt_auth_oca_provider_describe(payload))
        }
        "dashboard.prompts.system.auth.oca.constants.describe" => {
            Some(dashboard_prompt_auth_oca_constants_describe(payload))
        }
        "dashboard.prompts.system.auth.oca.types.describe" => {
            Some(dashboard_prompt_auth_oca_types_describe(payload))
        }
        "dashboard.prompts.system.auth.oca.utils.describe" => {
            Some(dashboard_prompt_auth_oca_utils_describe(payload))
        }
        "dashboard.prompts.system.auth.clineProvider.describe" => {
            Some(dashboard_prompt_auth_cline_provider_describe(payload))
        }
        "dashboard.prompts.system.shared.storage.fileStorage.describe" => {
            Some(dashboard_prompt_shared_storage_file_storage_describe(payload))
        }
        "dashboard.prompts.system.shared.storage.storage.describe" => {
            Some(dashboard_prompt_shared_storage_storage_describe(payload))
        }
        "dashboard.prompts.system.shared.storage.adapters.describe" => {
            Some(dashboard_prompt_shared_storage_adapters_describe(payload))
        }
        "dashboard.prompts.system.shared.storage.index.describe" => {
            Some(dashboard_prompt_shared_storage_index_describe(payload))
        }
        "dashboard.prompts.system.shared.storage.providerKeys.describe" => {
            Some(dashboard_prompt_shared_storage_provider_keys_describe(payload))
        }
        _ => dashboard_prompt_controller_task_ui_tail_route_extension(root, normalized, payload),
    }
}

include!("055-dashboard-system-prompt-controller-task-ui-tail-helpers.rs");
