fn dashboard_prompt_hosts_surface_vscode_env_open_external_describe(payload: &Value) -> Value {
    let target = clean_text(
        payload
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or(""),
        320,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_env_open_external_describe",
        "target": target
    })
}

fn dashboard_prompt_hosts_surface_vscode_env_shutdown_describe(payload: &Value) -> Value {
    let shutdown_mode = clean_text(
        payload
            .get("shutdown_mode")
            .and_then(Value::as_str)
            .unwrap_or("graceful"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_env_shutdown_describe",
        "shutdown_mode": shutdown_mode
    })
}

fn dashboard_prompt_hosts_surface_vscode_env_subscribe_telemetry_describe(payload: &Value) -> Value {
    let stream = clean_text(
        payload
            .get("stream")
            .and_then(Value::as_str)
            .unwrap_or("settings"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_env_subscribe_telemetry_describe",
        "stream": stream
    })
}

fn dashboard_prompt_hosts_surface_vscode_testing_get_webview_html_describe(payload: &Value) -> Value {
    let view_id = clean_text(
        payload
            .get("view_id")
            .and_then(Value::as_str)
            .unwrap_or("main"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_testing_get_webview_html_describe",
        "view_id": view_id
    })
}

fn dashboard_prompt_hosts_surface_vscode_window_get_active_editor_describe(payload: &Value) -> Value {
    let include_uri = payload
        .get("include_uri")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_window_get_active_editor_describe",
        "include_uri": include_uri
    })
}

fn dashboard_prompt_hosts_surface_vscode_window_get_open_tabs_test_describe(payload: &Value) -> Value {
    let test_profile = clean_text(
        payload
            .get("test_profile")
            .and_then(Value::as_str)
            .unwrap_or("contract"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_window_get_open_tabs_test_describe",
        "test_profile": test_profile
    })
}

fn dashboard_prompt_hosts_surface_vscode_window_get_open_tabs_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_hosts_surface_vscode_window_get_open_tabs_describe",
        "group": group
    })
}

fn dashboard_prompt_hosts_surface_vscode_window_get_visible_tabs_test_describe(
    payload: &Value,
) -> Value {
    let test_profile = clean_text(
        payload
            .get("test_profile")
            .and_then(Value::as_str)
            .unwrap_or("contract"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_window_get_visible_tabs_test_describe",
        "test_profile": test_profile
    })
}

fn dashboard_prompt_hosts_surface_vscode_window_get_visible_tabs_describe(payload: &Value) -> Value {
    let scope = clean_text(
        payload
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or("visible"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_window_get_visible_tabs_describe",
        "scope": scope
    })
}

fn dashboard_prompt_hosts_surface_vscode_window_open_file_describe(payload: &Value) -> Value {
    let path = clean_text(
        payload
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or(""),
        320,
    );
    let preview = payload
        .get("preview")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_window_open_file_describe",
        "path": path,
        "preview": preview
    })
}

fn dashboard_prompt_hosts_surface_tail_hostbridge_env_window_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.openExternal.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_env_open_external_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.shutdown.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_env_shutdown_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.subscribeToTelemetrySettings.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_env_subscribe_telemetry_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.testing.getWebviewHtml.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_testing_get_webview_html_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.getActiveEditor.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_window_get_active_editor_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.getOpenTabsTest.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_window_get_open_tabs_test_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.getOpenTabs.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_window_get_open_tabs_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.getVisibleTabsTest.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_window_get_visible_tabs_test_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.getVisibleTabs.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_window_get_visible_tabs_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.openFile.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_window_open_file_describe(payload))
        }
        _ => dashboard_prompt_hosts_surface_tail_hostbridge_window_workspace_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}
include!("077-dashboard-system-prompt-hostbridge-window-workspace-tail-helpers.rs");
