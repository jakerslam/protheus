fn dashboard_prompt_host_vscode_diff_save_document_describe(payload: &Value) -> Value {
    let uri = clean_text(payload.get("uri").and_then(Value::as_str).unwrap_or(""), 600);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_diff_save_document_describe",
        "uri": uri,
        "operation": "save_document"
    })
}

fn dashboard_prompt_host_vscode_diff_scroll_diff_describe(payload: &Value) -> Value {
    let direction = clean_text(
        payload
            .get("direction")
            .and_then(Value::as_str)
            .unwrap_or("down"),
        40,
    )
    .to_ascii_lowercase();
    let lines = payload
        .get("lines")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .clamp(1, 10_000);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_diff_scroll_diff_describe",
        "direction": direction,
        "lines": lines
    })
}

fn dashboard_prompt_host_vscode_diff_truncate_document_describe(payload: &Value) -> Value {
    let uri = clean_text(payload.get("uri").and_then(Value::as_str).unwrap_or(""), 600);
    let max_bytes = payload
        .get("max_bytes")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_diff_truncate_document_describe",
        "uri": uri,
        "max_bytes": max_bytes
    })
}

fn dashboard_prompt_host_vscode_env_clipboard_read_text_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_env_clipboard_read_text_describe",
        "operation": "clipboard_read_text"
    })
}

fn dashboard_prompt_host_vscode_env_clipboard_write_text_describe(payload: &Value) -> Value {
    let text_len = clean_text(payload.get("text").and_then(Value::as_str).unwrap_or(""), 10_000).len()
        as i64;
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_env_clipboard_write_text_describe",
        "text_len": text_len
    })
}

fn dashboard_prompt_host_vscode_env_debug_log_describe(payload: &Value) -> Value {
    let level = clean_text(
        payload
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("info"),
        40,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_env_debug_log_describe",
        "level": level
    })
}

fn dashboard_prompt_host_vscode_env_get_host_version_test_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_env_get_host_version_test_describe",
        "mode": "test_fixture"
    })
}

fn dashboard_prompt_host_vscode_env_get_host_version_describe(payload: &Value) -> Value {
    let expected = clean_text(
        payload
            .get("expected")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_env_get_host_version_describe",
        "expected": expected
    })
}

fn dashboard_prompt_host_vscode_env_get_ide_redirect_uri_describe(payload: &Value) -> Value {
    let ide = clean_text(payload.get("ide").and_then(Value::as_str).unwrap_or("vscode"), 80)
        .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_env_get_ide_redirect_uri_describe",
        "ide": ide
    })
}

fn dashboard_prompt_host_vscode_env_get_telemetry_settings_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_env_get_telemetry_settings_describe",
        "fields": ["enabled", "sample_rate", "destination"]
    })
}

fn dashboard_prompt_hostbridge_diff_env_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.saveDocument.describe" => {
            Some(dashboard_prompt_host_vscode_diff_save_document_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.scrollDiff.describe" => {
            Some(dashboard_prompt_host_vscode_diff_scroll_diff_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.truncateDocument.describe" => {
            Some(dashboard_prompt_host_vscode_diff_truncate_document_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.clipboardReadText.describe" => {
            Some(dashboard_prompt_host_vscode_env_clipboard_read_text_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.clipboardWriteText.describe" => {
            Some(dashboard_prompt_host_vscode_env_clipboard_write_text_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.debugLog.describe" => {
            Some(dashboard_prompt_host_vscode_env_debug_log_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.getHostVersionTest.describe" => {
            Some(dashboard_prompt_host_vscode_env_get_host_version_test_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.getHostVersion.describe" => {
            Some(dashboard_prompt_host_vscode_env_get_host_version_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.getIdeRedirectUri.describe" => {
            Some(dashboard_prompt_host_vscode_env_get_ide_redirect_uri_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.env.getTelemetrySettings.describe" => {
            Some(dashboard_prompt_host_vscode_env_get_telemetry_settings_describe())
        }
        _ => dashboard_prompt_hostbridge_env_window_tail_route_extension(root, normalized, payload),
    }
}

include!("029-dashboard-system-prompt-hostbridge-env-window-tail-helpers.rs");
