fn dashboard_prompt_hosts_surface_vscode_diff_save_document_describe(payload: &Value) -> Value {
    let uri = clean_text(
        payload
            .get("uri")
            .and_then(Value::as_str)
            .unwrap_or(""),
        300,
    );
    let save_mode = clean_text(
        payload
            .get("save_mode")
            .and_then(Value::as_str)
            .unwrap_or("write"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_diff_save_document_describe",
        "uri": uri,
        "save_mode": save_mode
    })
}

fn dashboard_prompt_hosts_surface_vscode_diff_scroll_diff_describe(payload: &Value) -> Value {
    let direction = clean_text(
        payload
            .get("direction")
            .and_then(Value::as_str)
            .unwrap_or("down"),
        60,
    )
    .to_ascii_lowercase();
    let lines = payload.get("lines").and_then(Value::as_i64).unwrap_or(1);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_diff_scroll_diff_describe",
        "direction": direction,
        "lines": lines
    })
}

fn dashboard_prompt_hosts_surface_vscode_diff_truncate_document_describe(payload: &Value) -> Value {
    let uri = clean_text(
        payload
            .get("uri")
            .and_then(Value::as_str)
            .unwrap_or(""),
        300,
    );
    let max_bytes = payload
        .get("max_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(65536);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_diff_truncate_document_describe",
        "uri": uri,
        "max_bytes": max_bytes
    })
}

fn dashboard_prompt_hosts_surface_vscode_env_clipboard_read_text_describe(payload: &Value) -> Value {
    let scope = clean_text(
        payload
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or("global"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_env_clipboard_read_text_describe",
        "scope": scope
    })
}

fn dashboard_prompt_hosts_surface_vscode_env_clipboard_write_text_describe(payload: &Value) -> Value {
    let text_len = payload
        .get("text")
        .and_then(Value::as_str)
        .map(str::len)
        .unwrap_or(0);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_env_clipboard_write_text_describe",
        "text_len": text_len
    })
}

fn dashboard_prompt_hosts_surface_vscode_env_debug_log_describe(payload: &Value) -> Value {
    let level = clean_text(
        payload
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("info"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_env_debug_log_describe",
        "level": level
    })
}

fn dashboard_prompt_hosts_surface_vscode_env_get_host_version_test_describe(
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
        "type": "dashboard_prompts_system_hosts_surface_vscode_env_get_host_version_test_describe",
        "test_profile": test_profile
    })
}

fn dashboard_prompt_hosts_surface_vscode_env_get_host_version_describe(payload: &Value) -> Value {
    let release_channel = clean_text(
        payload
            .get("release_channel")
            .and_then(Value::as_str)
            .unwrap_or("stable"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_env_get_host_version_describe",
        "release_channel": release_channel
    })
}

fn dashboard_prompt_hosts_surface_vscode_env_get_ide_redirect_uri_describe(payload: &Value) -> Value {
    let provider = clean_text(
        payload
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_env_get_ide_redirect_uri_describe",
        "provider": provider
    })
}

fn dashboard_prompt_hosts_surface_vscode_env_get_telemetry_settings_describe(
    payload: &Value,
) -> Value {
    let surface = clean_text(
        payload
            .get("surface")
            .and_then(Value::as_str)
            .unwrap_or("vscode"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_env_get_telemetry_settings_describe",
        "surface": surface
    })
}

fn dashboard_prompt_hosts_surface_tail_hostbridge_diff_env_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.saveDocument.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_diff_save_document_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.scrollDiff.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_diff_scroll_diff_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.truncateDocument.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_diff_truncate_document_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.clipboardReadText.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_env_clipboard_read_text_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.clipboardWriteText.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_env_clipboard_write_text_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.debugLog.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_env_debug_log_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.getHostVersionTest.describe" => Some(
            dashboard_prompt_hosts_surface_vscode_env_get_host_version_test_describe(payload),
        ),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.getHostVersion.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_env_get_host_version_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.getIdeRedirectUri.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_env_get_ide_redirect_uri_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.env.getTelemetrySettings.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_env_get_telemetry_settings_describe(payload))
        }
        _ => dashboard_prompt_hosts_surface_tail_hostbridge_env_window_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}
include!("076-dashboard-system-prompt-hostbridge-env-window-tail-helpers.rs");
