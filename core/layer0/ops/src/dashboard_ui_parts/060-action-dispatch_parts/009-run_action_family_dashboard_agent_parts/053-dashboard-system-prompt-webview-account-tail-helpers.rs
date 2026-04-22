fn dashboard_prompt_webview_app_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("chat"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_app_describe",
        "mode": mode
    })
}

fn dashboard_prompt_webview_custom_posthog_provider_describe(payload: &Value) -> Value {
    let provider = clean_text(
        payload
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("posthog"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_custom_posthog_provider_describe",
        "provider": provider
    })
}

fn dashboard_prompt_webview_providers_describe(payload: &Value) -> Value {
    let stack = clean_text(
        payload
            .get("stack")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_providers_describe",
        "stack": stack
    })
}

fn dashboard_prompt_webview_account_view_describe(payload: &Value) -> Value {
    let tab = clean_text(
        payload
            .get("tab")
            .and_then(Value::as_str)
            .unwrap_or("profile"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_account_view_describe",
        "tab": tab
    })
}

fn dashboard_prompt_webview_account_welcome_view_describe(payload: &Value) -> Value {
    let state = clean_text(
        payload
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("welcome"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_account_welcome_view_describe",
        "state": state
    })
}

fn dashboard_prompt_webview_credit_balance_describe(payload: &Value) -> Value {
    let currency = clean_text(
        payload
            .get("currency")
            .and_then(Value::as_str)
            .unwrap_or("usd"),
        40,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_credit_balance_describe",
        "currency": currency
    })
}

fn dashboard_prompt_webview_credits_history_table_describe(payload: &Value) -> Value {
    let range = clean_text(
        payload
            .get("range")
            .and_then(Value::as_str)
            .unwrap_or("30d"),
        40,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_credits_history_table_describe",
        "range": range
    })
}

fn dashboard_prompt_webview_remote_config_toggle_describe(payload: &Value) -> Value {
    let toggle = clean_text(
        payload
            .get("toggle")
            .and_then(Value::as_str)
            .unwrap_or("off"),
        40,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_remote_config_toggle_describe",
        "toggle": toggle
    })
}

fn dashboard_prompt_webview_styled_credit_display_describe(payload: &Value) -> Value {
    let style = clean_text(
        payload
            .get("style")
            .and_then(Value::as_str)
            .unwrap_or("compact"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_styled_credit_display_describe",
        "style": style
    })
}

fn dashboard_prompt_webview_account_helpers_describe(payload: &Value) -> Value {
    let helper = clean_text(
        payload
            .get("helper")
            .and_then(Value::as_str)
            .unwrap_or("format_credit"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_webview_account_helpers_describe",
        "helper": helper
    })
}

fn dashboard_prompt_webview_account_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.webview.app.describe" => {
            Some(dashboard_prompt_webview_app_describe(payload))
        }
        "dashboard.prompts.system.webview.customPostHogProvider.describe" => {
            Some(dashboard_prompt_webview_custom_posthog_provider_describe(payload))
        }
        "dashboard.prompts.system.webview.providers.describe" => {
            Some(dashboard_prompt_webview_providers_describe(payload))
        }
        "dashboard.prompts.system.webview.account.accountView.describe" => {
            Some(dashboard_prompt_webview_account_view_describe(payload))
        }
        "dashboard.prompts.system.webview.account.accountWelcomeView.describe" => {
            Some(dashboard_prompt_webview_account_welcome_view_describe(payload))
        }
        "dashboard.prompts.system.webview.account.creditBalance.describe" => {
            Some(dashboard_prompt_webview_credit_balance_describe(payload))
        }
        "dashboard.prompts.system.webview.account.creditsHistoryTable.describe" => {
            Some(dashboard_prompt_webview_credits_history_table_describe(payload))
        }
        "dashboard.prompts.system.webview.account.remoteConfigToggle.describe" => {
            Some(dashboard_prompt_webview_remote_config_toggle_describe(payload))
        }
        "dashboard.prompts.system.webview.account.styledCreditDisplay.describe" => {
            Some(dashboard_prompt_webview_styled_credit_display_describe(payload))
        }
        "dashboard.prompts.system.webview.account.helpers.describe" => {
            Some(dashboard_prompt_webview_account_helpers_describe(payload))
        }
        _ => dashboard_prompt_webview_auth_storage_tail_route_extension(root, normalized, payload),
    }
}

include!("054-dashboard-system-prompt-webview-auth-storage-tail-helpers.rs");
