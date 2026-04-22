fn dashboard_prompt_controller_ui_subscribe_mcp_button_clicked_describe(payload: &Value) -> Value {
    let target = clean_text(
        payload
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or("mcp"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_subscribe_mcp_button_clicked_describe",
        "target": target
    })
}

fn dashboard_prompt_controller_ui_subscribe_partial_message_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("stream"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_subscribe_partial_message_describe",
        "mode": mode
    })
}

fn dashboard_prompt_controller_ui_subscribe_relinquish_control_describe(payload: &Value) -> Value {
    let authority = clean_text(
        payload
            .get("authority")
            .and_then(Value::as_str)
            .unwrap_or("operator"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_subscribe_relinquish_control_describe",
        "authority": authority
    })
}

fn dashboard_prompt_controller_ui_subscribe_settings_button_clicked_describe(
    payload: &Value,
) -> Value {
    let pane = clean_text(
        payload
            .get("pane")
            .and_then(Value::as_str)
            .unwrap_or("settings"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_subscribe_settings_button_clicked_describe",
        "pane": pane
    })
}

fn dashboard_prompt_controller_ui_subscribe_show_webview_describe(payload: &Value) -> Value {
    let view = clean_text(
        payload
            .get("view")
            .and_then(Value::as_str)
            .unwrap_or("main"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_subscribe_show_webview_describe",
        "view": view
    })
}

fn dashboard_prompt_controller_ui_subscribe_worktrees_button_clicked_describe(
    payload: &Value,
) -> Value {
    let workspace = clean_text(
        payload
            .get("workspace")
            .and_then(Value::as_str)
            .unwrap_or("current"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_subscribe_worktrees_button_clicked_describe",
        "workspace": workspace
    })
}

fn dashboard_prompt_controller_web_check_is_image_url_describe(payload: &Value) -> Value {
    let scheme = clean_text(
        payload
            .get("scheme")
            .and_then(Value::as_str)
            .unwrap_or("https"),
        40,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_web_check_is_image_url_describe",
        "scheme": scheme
    })
}

fn dashboard_prompt_controller_web_fetch_open_graph_data_describe(payload: &Value) -> Value {
    let source = clean_text(
        payload
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("opengraph"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_web_fetch_open_graph_data_describe",
        "source": source
    })
}

fn dashboard_prompt_controller_web_open_in_browser_describe(payload: &Value) -> Value {
    let browser = clean_text(
        payload
            .get("browser")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_web_open_in_browser_describe",
        "browser": browser
    })
}

fn dashboard_prompt_controller_worktree_checkout_branch_describe(payload: &Value) -> Value {
    let branch = clean_text(
        payload
            .get("branch")
            .and_then(Value::as_str)
            .unwrap_or("main"),
        120,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_worktree_checkout_branch_describe",
        "branch": branch
    })
}

fn dashboard_prompt_controller_ui_web_worktree_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.controller.ui.subscribeToMcpButtonClicked.describe" => {
            Some(dashboard_prompt_controller_ui_subscribe_mcp_button_clicked_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.subscribeToPartialMessage.describe" => {
            Some(dashboard_prompt_controller_ui_subscribe_partial_message_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.subscribeToRelinquishControl.describe" => {
            Some(dashboard_prompt_controller_ui_subscribe_relinquish_control_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.subscribeToSettingsButtonClicked.describe" => {
            Some(dashboard_prompt_controller_ui_subscribe_settings_button_clicked_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.subscribeToShowWebview.describe" => {
            Some(dashboard_prompt_controller_ui_subscribe_show_webview_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.subscribeToWorktreesButtonClicked.describe" => {
            Some(dashboard_prompt_controller_ui_subscribe_worktrees_button_clicked_describe(payload))
        }
        "dashboard.prompts.system.controller.web.checkIsImageUrl.describe" => {
            Some(dashboard_prompt_controller_web_check_is_image_url_describe(payload))
        }
        "dashboard.prompts.system.controller.web.fetchOpenGraphData.describe" => {
            Some(dashboard_prompt_controller_web_fetch_open_graph_data_describe(payload))
        }
        "dashboard.prompts.system.controller.web.openInBrowser.describe" => {
            Some(dashboard_prompt_controller_web_open_in_browser_describe(payload))
        }
        "dashboard.prompts.system.controller.worktree.checkoutBranch.describe" => {
            Some(dashboard_prompt_controller_worktree_checkout_branch_describe(payload))
        }
        _ => dashboard_prompt_controller_worktree_ops_tail_route_extension(root, normalized, payload),
    }
}

include!("058-dashboard-system-prompt-controller-worktree-ops-tail-helpers.rs");
