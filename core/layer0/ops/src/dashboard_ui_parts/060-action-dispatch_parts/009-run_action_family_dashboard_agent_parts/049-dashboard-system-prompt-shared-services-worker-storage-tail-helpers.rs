fn dashboard_prompt_shared_services_config_posthog_describe(payload: &Value) -> Value {
    let host = clean_text(
        payload
            .get("host")
            .and_then(Value::as_str)
            .unwrap_or("https://app.posthog.com"),
        320,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_services_config_posthog_describe",
        "host": host
    })
}

fn dashboard_prompt_shared_services_feature_flags_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_services_feature_flags_describe",
        "profile": profile
    })
}

fn dashboard_prompt_shared_services_worker_backfill_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("incremental"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_services_worker_backfill_describe",
        "mode": mode
    })
}

fn dashboard_prompt_shared_services_worker_queue_describe(payload: &Value) -> Value {
    let policy = clean_text(
        payload
            .get("policy")
            .and_then(Value::as_str)
            .unwrap_or("bounded"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_services_worker_queue_describe",
        "policy": policy
    })
}

fn dashboard_prompt_shared_services_worker_sync_describe(payload: &Value) -> Value {
    let cadence = clean_text(
        payload
            .get("cadence")
            .and_then(Value::as_str)
            .unwrap_or("scheduled"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_services_worker_sync_describe",
        "cadence": cadence
    })
}

fn dashboard_prompt_shared_services_worker_utils_describe(payload: &Value) -> Value {
    let helper = clean_text(
        payload
            .get("helper")
            .and_then(Value::as_str)
            .unwrap_or("retry_budget"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_services_worker_utils_describe",
        "helper": helper
    })
}

fn dashboard_prompt_shared_services_worker_worker_describe(payload: &Value) -> Value {
    let worker = clean_text(
        payload
            .get("worker")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_services_worker_worker_describe",
        "worker": worker
    })
}

fn dashboard_prompt_shared_skills_describe(payload: &Value) -> Value {
    let pack = clean_text(
        payload
            .get("pack")
            .and_then(Value::as_str)
            .unwrap_or("core"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_skills_describe",
        "pack": pack
    })
}

fn dashboard_prompt_shared_slash_commands_describe(payload: &Value) -> Value {
    let command = clean_text(
        payload
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("/help"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_slash_commands_describe",
        "command": command
    })
}

fn dashboard_prompt_shared_storage_cline_blob_storage_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_shared_storage_cline_blob_storage_describe",
        "backend": backend
    })
}

fn dashboard_prompt_shared_services_worker_storage_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.shared.services.config.posthogConfig.describe" => {
            Some(dashboard_prompt_shared_services_config_posthog_describe(payload))
        }
        "dashboard.prompts.system.shared.services.featureFlags.featureFlags.describe" => {
            Some(dashboard_prompt_shared_services_feature_flags_describe(payload))
        }
        "dashboard.prompts.system.shared.services.worker.backfill.describe" => {
            Some(dashboard_prompt_shared_services_worker_backfill_describe(payload))
        }
        "dashboard.prompts.system.shared.services.worker.queue.describe" => {
            Some(dashboard_prompt_shared_services_worker_queue_describe(payload))
        }
        "dashboard.prompts.system.shared.services.worker.sync.describe" => {
            Some(dashboard_prompt_shared_services_worker_sync_describe(payload))
        }
        "dashboard.prompts.system.shared.services.worker.utils.describe" => {
            Some(dashboard_prompt_shared_services_worker_utils_describe(payload))
        }
        "dashboard.prompts.system.shared.services.worker.worker.describe" => {
            Some(dashboard_prompt_shared_services_worker_worker_describe(payload))
        }
        "dashboard.prompts.system.shared.skills.describe" => {
            Some(dashboard_prompt_shared_skills_describe(payload))
        }
        "dashboard.prompts.system.shared.slashCommands.describe" => {
            Some(dashboard_prompt_shared_slash_commands_describe(payload))
        }
        "dashboard.prompts.system.shared.storage.clineBlobStorage.describe" => {
            Some(dashboard_prompt_shared_storage_cline_blob_storage_describe(payload))
        }
        _ => dashboard_prompt_shared_utils_standalone_tail_route_extension(root, normalized, payload),
    }
}

include!("050-dashboard-system-prompt-shared-utils-standalone-tail-helpers.rs");
