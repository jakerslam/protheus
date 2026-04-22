fn dashboard_prompt_system_prompt_variants_trinity_template_describe(payload: &Value) -> Value {
    let template_mode = clean_text(
        payload
            .get("template_mode")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_trinity_template_describe",
        "template_mode": template_mode
    })
}

fn dashboard_prompt_system_prompt_variants_variant_builder_describe(payload: &Value) -> Value {
    let builder_mode = clean_text(
        payload
            .get("builder_mode")
            .and_then(Value::as_str)
            .unwrap_or("composed"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_variant_builder_describe",
        "builder_mode": builder_mode
    })
}

fn dashboard_prompt_system_prompt_variants_variant_validator_describe(payload: &Value) -> Value {
    let validator_mode = clean_text(
        payload
            .get("validator_mode")
            .and_then(Value::as_str)
            .unwrap_or("strict"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_variant_validator_describe",
        "validator_mode": validator_mode
    })
}

fn dashboard_prompt_system_prompt_variants_xs_config_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("xs"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_xs_config_describe",
        "profile": profile
    })
}

fn dashboard_prompt_system_prompt_variants_xs_overrides_describe(payload: &Value) -> Value {
    let override_mode = clean_text(
        payload
            .get("override_mode")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_xs_overrides_describe",
        "override_mode": override_mode
    })
}

fn dashboard_prompt_system_prompt_variants_xs_template_describe(payload: &Value) -> Value {
    let template_mode = clean_text(
        payload
            .get("template_mode")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_system_prompt_variants_xs_template_describe",
        "template_mode": template_mode
    })
}

fn dashboard_prompt_storage_state_manager_describe(payload: &Value) -> Value {
    let state_mode = clean_text(
        payload
            .get("state_mode")
            .and_then(Value::as_str)
            .unwrap_or("persisted"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_state_manager_describe",
        "state_mode": state_mode
    })
}

fn dashboard_prompt_storage_disk_describe(payload: &Value) -> Value {
    let disk_mode = clean_text(
        payload
            .get("disk_mode")
            .and_then(Value::as_str)
            .unwrap_or("safe_write"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_disk_describe",
        "disk_mode": disk_mode
    })
}

fn dashboard_prompt_storage_error_messages_describe(payload: &Value) -> Value {
    let error_catalog = clean_text(
        payload
            .get("error_catalog")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_error_messages_describe",
        "error_catalog": error_catalog
    })
}

fn dashboard_prompt_storage_remote_config_fetch_describe(payload: &Value) -> Value {
    let fetch_policy = clean_text(
        payload
            .get("fetch_policy")
            .and_then(Value::as_str)
            .unwrap_or("network_first"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_storage_remote_config_fetch_describe",
        "fetch_policy": fetch_policy
    })
}

fn dashboard_prompt_system_prompt_variants_storage_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.systemPrompt.variants.trinity.template.describe" => {
            Some(dashboard_prompt_system_prompt_variants_trinity_template_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.variantBuilder.describe" => {
            Some(dashboard_prompt_system_prompt_variants_variant_builder_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.variantValidator.describe" => {
            Some(dashboard_prompt_system_prompt_variants_variant_validator_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.xs.config.describe" => {
            Some(dashboard_prompt_system_prompt_variants_xs_config_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.xs.overrides.describe" => {
            Some(dashboard_prompt_system_prompt_variants_xs_overrides_describe(payload))
        }
        "dashboard.prompts.system.systemPrompt.variants.xs.template.describe" => {
            Some(dashboard_prompt_system_prompt_variants_xs_template_describe(payload))
        }
        "dashboard.prompts.system.storage.stateManager.describe" => {
            Some(dashboard_prompt_storage_state_manager_describe(payload))
        }
        "dashboard.prompts.system.storage.disk.describe" => {
            Some(dashboard_prompt_storage_disk_describe(payload))
        }
        "dashboard.prompts.system.storage.errorMessages.describe" => {
            Some(dashboard_prompt_storage_error_messages_describe(payload))
        }
        "dashboard.prompts.system.storage.remoteConfigFetch.describe" => {
            Some(dashboard_prompt_storage_remote_config_fetch_describe(payload))
        }
        _ => dashboard_prompt_storage_task_tail_route_extension(root, normalized, payload),
    }
}

include!("069-dashboard-system-prompt-storage-task-tail-helpers.rs");
