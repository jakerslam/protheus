fn dashboard_prompt_shared_constants_describe(payload: &Value) -> Value {
    let scope = clean_text(
        payload
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or("runtime"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_constants_describe",
        "scope": scope
    })
}

fn dashboard_prompt_shared_content_limits_describe(payload: &Value) -> Value {
    let lane = clean_text(
        payload
            .get("lane")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_content_limits_describe",
        "lane": lane
    })
}

fn dashboard_prompt_shared_context_mentions_describe(payload: &Value) -> Value {
    let source = clean_text(
        payload
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("conversation"),
        140,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_context_mentions_describe",
        "source": source
    })
}

fn dashboard_prompt_shared_focus_chain_utils_describe(payload: &Value) -> Value {
    let strategy = clean_text(
        payload
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        140,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_focus_chain_utils_describe",
        "strategy": strategy
    })
}

fn dashboard_prompt_shared_get_api_metrics_describe(payload: &Value) -> Value {
    let window = clean_text(
        payload
            .get("window")
            .and_then(Value::as_str)
            .unwrap_or("5m"),
        60,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_get_api_metrics_describe",
        "window": window
    })
}

fn dashboard_prompt_shared_internal_account_describe(payload: &Value) -> Value {
    let account_type = clean_text(
        payload
            .get("account_type")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_internal_account_describe",
        "account_type": account_type
    })
}

fn dashboard_prompt_shared_mcp_describe(payload: &Value) -> Value {
    let transport = clean_text(
        payload
            .get("transport")
            .and_then(Value::as_str)
            .unwrap_or("streamable_http"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_mcp_describe",
        "transport": transport
    })
}

fn dashboard_prompt_shared_messages_constants_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_messages_constants_describe",
        "exports": ["message_types", "event_kinds"]
    })
}

fn dashboard_prompt_shared_messages_content_describe(payload: &Value) -> Value {
    let content_type = clean_text(
        payload
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or("text"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_messages_content_describe",
        "content_type": content_type
    })
}

fn dashboard_prompt_shared_messages_index_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_messages_index_describe",
        "exports": ["constants", "content", "metrics"]
    })
}

fn dashboard_prompt_shared_constants_messages_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.shared.constants.describe" => {
            Some(dashboard_prompt_shared_constants_describe(payload))
        }
        "dashboard.prompts.system.shared.contentLimits.describe" => {
            Some(dashboard_prompt_shared_content_limits_describe(payload))
        }
        "dashboard.prompts.system.shared.contextMentions.describe" => {
            Some(dashboard_prompt_shared_context_mentions_describe(payload))
        }
        "dashboard.prompts.system.shared.focusChainUtils.describe" => {
            Some(dashboard_prompt_shared_focus_chain_utils_describe(payload))
        }
        "dashboard.prompts.system.shared.getApiMetrics.describe" => {
            Some(dashboard_prompt_shared_get_api_metrics_describe(payload))
        }
        "dashboard.prompts.system.shared.internal.account.describe" => {
            Some(dashboard_prompt_shared_internal_account_describe(payload))
        }
        "dashboard.prompts.system.shared.mcp.describe" => {
            Some(dashboard_prompt_shared_mcp_describe(payload))
        }
        "dashboard.prompts.system.shared.messages.constants.describe" => {
            Some(dashboard_prompt_shared_messages_constants_describe())
        }
        "dashboard.prompts.system.shared.messages.content.describe" => {
            Some(dashboard_prompt_shared_messages_content_describe(payload))
        }
        "dashboard.prompts.system.shared.messages.index.describe" => {
            Some(dashboard_prompt_shared_messages_index_describe())
        }
        _ => dashboard_prompt_shared_proto_conversions_tail_route_extension(root, normalized, payload),
    }
}

include!("047-dashboard-system-prompt-shared-proto-conversions-tail-helpers.rs");
