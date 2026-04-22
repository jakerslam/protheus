fn dashboard_prompt_shared_auto_approval_settings_describe(payload: &Value) -> Value {
    let policy = clean_text(
        payload
            .get("policy")
            .and_then(Value::as_str)
            .unwrap_or("review_required"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_auto_approval_settings_describe",
        "policy": policy
    })
}

fn dashboard_prompt_shared_browser_settings_describe(payload: &Value) -> Value {
    let browser = clean_text(
        payload
            .get("browser")
            .and_then(Value::as_str)
            .unwrap_or("system"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_browser_settings_describe",
        "browser": browser
    })
}

fn dashboard_prompt_shared_chat_content_describe(payload: &Value) -> Value {
    let kind = clean_text(
        payload
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("text"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_chat_content_describe",
        "kind": kind
    })
}

fn dashboard_prompt_shared_cline_account_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_shared_cline_account_describe",
        "provider": provider
    })
}

fn dashboard_prompt_shared_cline_banner_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_shared_cline_banner_describe",
        "banner": banner
    })
}

fn dashboard_prompt_shared_cline_feature_setting_describe(payload: &Value) -> Value {
    let feature = clean_text(
        payload
            .get("feature")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_cline_feature_setting_describe",
        "feature": feature
    })
}

fn dashboard_prompt_shared_extension_message_describe(payload: &Value) -> Value {
    let channel = clean_text(
        payload
            .get("channel")
            .and_then(Value::as_str)
            .unwrap_or("ui"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_extension_message_describe",
        "channel": channel
    })
}

fn dashboard_prompt_shared_focus_chain_settings_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_focus_chain_settings_describe",
        "mode": mode
    })
}

fn dashboard_prompt_shared_history_item_describe(payload: &Value) -> Value {
    let item_id = clean_text(
        payload.get("id").and_then(Value::as_str).unwrap_or(""),
        200,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_history_item_describe",
        "id": item_id
    })
}

fn dashboard_prompt_shared_languages_describe(payload: &Value) -> Value {
    let locale = clean_text(
        payload
            .get("locale")
            .and_then(Value::as_str)
            .unwrap_or("en"),
        40,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_shared_languages_describe",
        "locale": locale
    })
}

fn dashboard_prompt_shared_settings_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.shared.autoApprovalSettings.describe" => {
            Some(dashboard_prompt_shared_auto_approval_settings_describe(payload))
        }
        "dashboard.prompts.system.shared.browserSettings.describe" => {
            Some(dashboard_prompt_shared_browser_settings_describe(payload))
        }
        "dashboard.prompts.system.shared.chatContent.describe" => {
            Some(dashboard_prompt_shared_chat_content_describe(payload))
        }
        "dashboard.prompts.system.shared.clineAccount.describe" => {
            Some(dashboard_prompt_shared_cline_account_describe(payload))
        }
        "dashboard.prompts.system.shared.clineBanner.describe" => {
            Some(dashboard_prompt_shared_cline_banner_describe(payload))
        }
        "dashboard.prompts.system.shared.clineFeatureSetting.describe" => {
            Some(dashboard_prompt_shared_cline_feature_setting_describe(payload))
        }
        "dashboard.prompts.system.shared.extensionMessage.describe" => {
            Some(dashboard_prompt_shared_extension_message_describe(payload))
        }
        "dashboard.prompts.system.shared.focusChainSettings.describe" => {
            Some(dashboard_prompt_shared_focus_chain_settings_describe(payload))
        }
        "dashboard.prompts.system.shared.historyItem.describe" => {
            Some(dashboard_prompt_shared_history_item_describe(payload))
        }
        "dashboard.prompts.system.shared.languages.describe" => {
            Some(dashboard_prompt_shared_languages_describe(payload))
        }
        _ => dashboard_prompt_shared_core_api_tail_route_extension(root, normalized, payload),
    }
}

include!("044-dashboard-system-prompt-shared-core-api-tail-helpers.rs");
