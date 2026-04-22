fn dashboard_prompt_shared_mcp_display_mode_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("compact"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_mcp_display_mode_describe",
        "mode": mode
    })
}

fn dashboard_prompt_shared_patch_describe(payload: &Value) -> Value {
    let patch_kind = clean_text(
        payload
            .get("patch_kind")
            .and_then(Value::as_str)
            .unwrap_or("unified"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_patch_describe",
        "patch_kind": patch_kind
    })
}

fn dashboard_prompt_shared_telemetry_setting_describe(payload: &Value) -> Value {
    let level = clean_text(
        payload
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("standard"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_telemetry_setting_describe",
        "level": level
    })
}

fn dashboard_prompt_shared_user_info_describe(payload: &Value) -> Value {
    let role = clean_text(
        payload
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("owner"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_user_info_describe",
        "role": role
    })
}

fn dashboard_prompt_shared_webview_message_describe(payload: &Value) -> Value {
    let channel = clean_text(
        payload
            .get("channel")
            .and_then(Value::as_str)
            .unwrap_or("webview"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_webview_message_describe",
        "channel": channel
    })
}

fn dashboard_prompt_shared_api_describe(payload: &Value) -> Value {
    let api = clean_text(
        payload
            .get("api")
            .and_then(Value::as_str)
            .unwrap_or("primary"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_api_describe",
        "api": api
    })
}

fn dashboard_prompt_shared_array_describe(payload: &Value) -> Value {
    let op = clean_text(
        payload
            .get("operation")
            .and_then(Value::as_str)
            .unwrap_or("merge"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_array_describe",
        "operation": op
    })
}

fn dashboard_prompt_shared_clients_requesty_describe(payload: &Value) -> Value {
    let endpoint = clean_text(
        payload
            .get("endpoint")
            .and_then(Value::as_str)
            .unwrap_or("/"),
        600,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_clients_requesty_describe",
        "endpoint": endpoint
    })
}

fn dashboard_prompt_shared_cline_rules_describe(payload: &Value) -> Value {
    let profile = clean_text(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_cline_rules_describe",
        "profile": profile
    })
}

fn dashboard_prompt_shared_cline_api_describe(payload: &Value) -> Value {
    let provider = clean_text(
        payload
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("openai"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_cline_api_describe",
        "provider": provider
    })
}

fn dashboard_prompt_shared_core_api_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.shared.mcpDisplayMode.describe" => {
            Some(dashboard_prompt_shared_mcp_display_mode_describe(payload))
        }
        "dashboard.prompts.system.shared.patch.describe" => {
            Some(dashboard_prompt_shared_patch_describe(payload))
        }
        "dashboard.prompts.system.shared.telemetrySetting.describe" => {
            Some(dashboard_prompt_shared_telemetry_setting_describe(payload))
        }
        "dashboard.prompts.system.shared.userInfo.describe" => {
            Some(dashboard_prompt_shared_user_info_describe(payload))
        }
        "dashboard.prompts.system.shared.webviewMessage.describe" => {
            Some(dashboard_prompt_shared_webview_message_describe(payload))
        }
        "dashboard.prompts.system.shared.api.describe" => {
            Some(dashboard_prompt_shared_api_describe(payload))
        }
        "dashboard.prompts.system.shared.array.describe" => {
            Some(dashboard_prompt_shared_array_describe(payload))
        }
        "dashboard.prompts.system.shared.clients.requesty.describe" => {
            Some(dashboard_prompt_shared_clients_requesty_describe(payload))
        }
        "dashboard.prompts.system.shared.clineRules.describe" => {
            Some(dashboard_prompt_shared_cline_rules_describe(payload))
        }
        "dashboard.prompts.system.shared.cline.api.describe" => {
            Some(dashboard_prompt_shared_cline_api_describe(payload))
        }
        _ => dashboard_prompt_shared_cline_combine_tail_route_extension(root, normalized, payload),
    }
}

include!("045-dashboard-system-prompt-shared-cline-combine-tail-helpers.rs");
