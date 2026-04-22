fn dashboard_prompt_controller_ui_initialize_webview_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("initialize"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_initialize_webview_describe",
        "mode": mode
    })
}

fn dashboard_prompt_controller_ui_did_show_announcement_describe(payload: &Value) -> Value {
    let announcement = clean_text(
        payload
            .get("announcement")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_did_show_announcement_describe",
        "announcement": announcement
    })
}

fn dashboard_prompt_controller_ui_open_url_describe(payload: &Value) -> Value {
    let url = clean_text(
        payload
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or("https://example.invalid"),
        512,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_open_url_describe",
        "url": url
    })
}

fn dashboard_prompt_controller_ui_open_walkthrough_describe(payload: &Value) -> Value {
    let topic = clean_text(
        payload
            .get("topic")
            .and_then(Value::as_str)
            .unwrap_or("intro"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_open_walkthrough_describe",
        "topic": topic
    })
}

fn dashboard_prompt_controller_ui_scroll_to_settings_describe(payload: &Value) -> Value {
    let section = clean_text(
        payload
            .get("section")
            .and_then(Value::as_str)
            .unwrap_or("general"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_scroll_to_settings_describe",
        "section": section
    })
}

fn dashboard_prompt_controller_ui_set_terminal_execution_mode_describe(payload: &Value) -> Value {
    let terminal_mode = clean_text(
        payload
            .get("terminal_mode")
            .and_then(Value::as_str)
            .unwrap_or("safe"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_set_terminal_execution_mode_describe",
        "terminal_mode": terminal_mode
    })
}

fn dashboard_prompt_controller_ui_subscribe_account_button_clicked_describe(
    payload: &Value,
) -> Value {
    let action = clean_text(
        payload
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or("open_account"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_subscribe_account_button_clicked_describe",
        "action": action
    })
}

fn dashboard_prompt_controller_ui_subscribe_add_to_input_describe(payload: &Value) -> Value {
    let source = clean_text(
        payload
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("selection"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_subscribe_add_to_input_describe",
        "source": source
    })
}

fn dashboard_prompt_controller_ui_subscribe_chat_button_clicked_describe(payload: &Value) -> Value {
    let target = clean_text(
        payload
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or("chat"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_subscribe_chat_button_clicked_describe",
        "target": target
    })
}

fn dashboard_prompt_controller_ui_subscribe_history_button_clicked_describe(
    payload: &Value,
) -> Value {
    let history_mode = clean_text(
        payload
            .get("history_mode")
            .and_then(Value::as_str)
            .unwrap_or("recent"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_subscribe_history_button_clicked_describe",
        "history_mode": history_mode
    })
}

fn dashboard_prompt_controller_ui_event_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.controller.ui.initializeWebview.describe" => {
            Some(dashboard_prompt_controller_ui_initialize_webview_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.onDidShowAnnouncement.describe" => {
            Some(dashboard_prompt_controller_ui_did_show_announcement_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.openUrl.describe" => {
            Some(dashboard_prompt_controller_ui_open_url_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.openWalkthrough.describe" => {
            Some(dashboard_prompt_controller_ui_open_walkthrough_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.scrollToSettings.describe" => {
            Some(dashboard_prompt_controller_ui_scroll_to_settings_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.setTerminalExecutionMode.describe" => {
            Some(dashboard_prompt_controller_ui_set_terminal_execution_mode_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.subscribeToAccountButtonClicked.describe" => {
            Some(dashboard_prompt_controller_ui_subscribe_account_button_clicked_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.subscribeToAddToInput.describe" => {
            Some(dashboard_prompt_controller_ui_subscribe_add_to_input_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.subscribeToChatButtonClicked.describe" => {
            Some(dashboard_prompt_controller_ui_subscribe_chat_button_clicked_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.subscribeToHistoryButtonClicked.describe" => {
            Some(dashboard_prompt_controller_ui_subscribe_history_button_clicked_describe(payload))
        }
        _ => dashboard_prompt_controller_ui_web_worktree_tail_route_extension(root, normalized, payload),
    }
}

include!("057-dashboard-system-prompt-controller-ui-web-worktree-tail-helpers.rs");
