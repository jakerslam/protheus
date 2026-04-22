fn dashboard_prompt_host_vscode_env_open_external_describe(payload: &Value) -> Value {
    let url = clean_text(payload.get("url").and_then(Value::as_str).unwrap_or(""), 1200);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_env_open_external_describe",
        "url": url,
        "operation": "open_external"
    })
}

fn dashboard_prompt_host_vscode_env_shutdown_describe(payload: &Value) -> Value {
    let reason = clean_text(
        payload
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("manual"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_env_shutdown_describe",
        "reason": reason
    })
}

fn dashboard_prompt_host_vscode_env_subscribe_telemetry_settings_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_env_subscribe_telemetry_settings_describe",
        "operation": "subscribe_telemetry_settings"
    })
}

fn dashboard_prompt_host_vscode_testing_get_webview_html_describe(payload: &Value) -> Value {
    let panel_id = clean_text(
        payload
            .get("panel_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_testing_get_webview_html_describe",
        "panel_id": panel_id
    })
}

fn dashboard_prompt_host_vscode_window_get_active_editor_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_window_get_active_editor_describe",
        "fields": ["uri", "language_id", "dirty"]
    })
}

fn dashboard_prompt_host_vscode_window_get_open_tabs_test_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_window_get_open_tabs_test_describe",
        "mode": "test_fixture"
    })
}

fn dashboard_prompt_host_vscode_window_get_open_tabs_describe(payload: &Value) -> Value {
    let group = clean_text(
        payload
            .get("group")
            .and_then(Value::as_str)
            .unwrap_or("all"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_window_get_open_tabs_describe",
        "group": group
    })
}

fn dashboard_prompt_host_vscode_window_get_visible_tabs_test_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_window_get_visible_tabs_test_describe",
        "mode": "test_fixture"
    })
}

fn dashboard_prompt_host_vscode_window_get_visible_tabs_describe(payload: &Value) -> Value {
    let editor_group = clean_text(
        payload
            .get("editor_group")
            .and_then(Value::as_str)
            .unwrap_or("active"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_window_get_visible_tabs_describe",
        "editor_group": editor_group
    })
}

fn dashboard_prompt_host_vscode_window_open_file_describe(payload: &Value) -> Value {
    let path = clean_text(payload.get("path").and_then(Value::as_str).unwrap_or(""), 1200);
    let preview = payload
        .get("preview")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_window_open_file_describe",
        "path": path,
        "preview": preview
    })
}

fn dashboard_prompt_hostbridge_env_window_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.openExternal.describe" => {
            Some(dashboard_prompt_host_vscode_env_open_external_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.shutdown.describe" => {
            Some(dashboard_prompt_host_vscode_env_shutdown_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.subscribeToTelemetrySettings.describe" => {
            Some(dashboard_prompt_host_vscode_env_subscribe_telemetry_settings_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.testing.getWebviewHtml.describe" => {
            Some(dashboard_prompt_host_vscode_testing_get_webview_html_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.getActiveEditor.describe" => {
            Some(dashboard_prompt_host_vscode_window_get_active_editor_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.getOpenTabsTest.describe" => {
            Some(dashboard_prompt_host_vscode_window_get_open_tabs_test_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.getOpenTabs.describe" => {
            Some(dashboard_prompt_host_vscode_window_get_open_tabs_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.getVisibleTabsTest.describe" => {
            Some(dashboard_prompt_host_vscode_window_get_visible_tabs_test_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.getVisibleTabs.describe" => {
            Some(dashboard_prompt_host_vscode_window_get_visible_tabs_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.openFile.describe" => {
            Some(dashboard_prompt_host_vscode_window_open_file_describe(payload))
        }
        _ => dashboard_prompt_hostbridge_window_workspace_tail_route_extension(root, normalized, payload),
    }
}

include!("030-dashboard-system-prompt-hostbridge-window-workspace-tail-helpers.rs");
