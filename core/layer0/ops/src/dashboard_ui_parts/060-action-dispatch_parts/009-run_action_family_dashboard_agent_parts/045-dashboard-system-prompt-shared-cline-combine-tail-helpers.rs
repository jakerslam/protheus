fn dashboard_prompt_shared_cline_banner_nested_describe(payload: &Value) -> Value {
    let banner = clean_text(
        payload
            .get("banner")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_cline_banner_nested_describe",
        "banner": banner
    })
}

fn dashboard_prompt_shared_cline_context_describe(payload: &Value) -> Value {
    let context = clean_text(
        payload
            .get("context")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        220,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_cline_context_describe",
        "context": context
    })
}

fn dashboard_prompt_shared_cline_index_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_cline_index_describe",
        "exports": ["api", "banner", "context", "onboarding", "recommended_models"]
    })
}

fn dashboard_prompt_shared_cline_onboarding_describe(payload: &Value) -> Value {
    let stage = clean_text(
        payload
            .get("stage")
            .and_then(Value::as_str)
            .unwrap_or("welcome"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_cline_onboarding_describe",
        "stage": stage
    })
}

fn dashboard_prompt_shared_cline_recommended_models_describe(payload: &Value) -> Value {
    let family = clean_text(
        payload
            .get("family")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_cline_recommended_models_describe",
        "family": family
    })
}

fn dashboard_prompt_shared_combine_api_requests_describe(payload: &Value) -> Value {
    let strategy = clean_text(
        payload
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or("sequential"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_combine_api_requests_describe",
        "strategy": strategy
    })
}

fn dashboard_prompt_shared_combine_command_sequences_describe(payload: &Value) -> Value {
    let strategy = clean_text(
        payload
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or("preserve_order"),
        140,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_combine_command_sequences_describe",
        "strategy": strategy
    })
}

fn dashboard_prompt_shared_combine_error_retry_messages_describe(payload: &Value) -> Value {
    let policy = clean_text(
        payload
            .get("policy")
            .and_then(Value::as_str)
            .unwrap_or("retry_then_surface"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_combine_error_retry_messages_describe",
        "policy": policy
    })
}

fn dashboard_prompt_shared_combine_hook_sequences_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("ordered"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_combine_hook_sequences_describe",
        "mode": mode
    })
}

fn dashboard_prompt_shared_config_types_describe(payload: &Value) -> Value {
    let schema = clean_text(
        payload
            .get("schema")
            .and_then(Value::as_str)
            .unwrap_or("runtime_config"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_config_types_describe",
        "schema": schema
    })
}

fn dashboard_prompt_shared_cline_combine_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.shared.cline.banner.describe" => {
            Some(dashboard_prompt_shared_cline_banner_nested_describe(payload))
        }
        "dashboard.prompts.system.shared.cline.context.describe" => {
            Some(dashboard_prompt_shared_cline_context_describe(payload))
        }
        "dashboard.prompts.system.shared.cline.index.describe" => {
            Some(dashboard_prompt_shared_cline_index_describe())
        }
        "dashboard.prompts.system.shared.cline.onboarding.describe" => {
            Some(dashboard_prompt_shared_cline_onboarding_describe(payload))
        }
        "dashboard.prompts.system.shared.cline.recommendedModels.describe" => {
            Some(dashboard_prompt_shared_cline_recommended_models_describe(payload))
        }
        "dashboard.prompts.system.shared.combineApiRequests.describe" => {
            Some(dashboard_prompt_shared_combine_api_requests_describe(payload))
        }
        "dashboard.prompts.system.shared.combineCommandSequences.describe" => {
            Some(dashboard_prompt_shared_combine_command_sequences_describe(payload))
        }
        "dashboard.prompts.system.shared.combineErrorRetryMessages.describe" => {
            Some(dashboard_prompt_shared_combine_error_retry_messages_describe(payload))
        }
        "dashboard.prompts.system.shared.combineHookSequences.describe" => {
            Some(dashboard_prompt_shared_combine_hook_sequences_describe(payload))
        }
        "dashboard.prompts.system.shared.configTypes.describe" => {
            Some(dashboard_prompt_shared_config_types_describe(payload))
        }
        _ => dashboard_prompt_shared_constants_messages_tail_route_extension(root, normalized, payload),
    }
}

include!("046-dashboard-system-prompt-shared-constants-messages-tail-helpers.rs");
