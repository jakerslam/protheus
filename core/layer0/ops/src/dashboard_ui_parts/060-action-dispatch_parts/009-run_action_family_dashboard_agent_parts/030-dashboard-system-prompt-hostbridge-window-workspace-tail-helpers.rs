fn dashboard_prompt_host_vscode_window_open_settings_describe(payload: &Value) -> Value {
    let query = clean_text(payload.get("query").and_then(Value::as_str).unwrap_or(""), 400);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_window_open_settings_describe",
        "query": query
    })
}

fn dashboard_prompt_host_vscode_window_show_input_box_describe(payload: &Value) -> Value {
    let prompt = clean_text(payload.get("prompt").and_then(Value::as_str).unwrap_or(""), 600);
    let value = clean_text(payload.get("value").and_then(Value::as_str).unwrap_or(""), 600);
    let place_holder = clean_text(
        payload
            .get("place_holder")
            .or_else(|| payload.get("placeholder"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        600,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_window_show_input_box_describe",
        "prompt": prompt,
        "value": value,
        "place_holder": place_holder
    })
}

fn dashboard_prompt_host_vscode_window_show_message_describe(payload: &Value) -> Value {
    let message = clean_text(payload.get("message").and_then(Value::as_str).unwrap_or(""), 1200);
    let level = clean_text(
        payload
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("info"),
        120,
    )
    .to_ascii_lowercase();
    let modal = payload
        .get("modal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_window_show_message_describe",
        "message": message,
        "level": level,
        "modal": modal
    })
}

fn dashboard_prompt_host_vscode_window_show_open_dialogue_describe(payload: &Value) -> Value {
    let can_select_files = payload
        .get("can_select_files")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let can_select_folders = payload
        .get("can_select_folders")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_window_show_open_dialogue_describe",
        "can_select_files": can_select_files,
        "can_select_folders": can_select_folders
    })
}

fn dashboard_prompt_host_vscode_window_show_save_dialog_describe(payload: &Value) -> Value {
    let default_uri = clean_text(
        payload
            .get("default_uri")
            .and_then(Value::as_str)
            .unwrap_or(""),
        1200,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_window_show_save_dialog_describe",
        "default_uri": default_uri
    })
}

fn dashboard_prompt_host_vscode_window_show_text_document_describe(payload: &Value) -> Value {
    let uri = clean_text(payload.get("uri").and_then(Value::as_str).unwrap_or(""), 1200);
    let preview = payload
        .get("preview")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_window_show_text_document_describe",
        "uri": uri,
        "preview": preview
    })
}

fn dashboard_prompt_host_vscode_workspace_execute_command_terminal_describe(payload: &Value) -> Value {
    let command = clean_text(payload.get("command").and_then(Value::as_str).unwrap_or(""), 2400);
    let cwd = clean_text(payload.get("cwd").and_then(Value::as_str).unwrap_or(""), 1200);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_workspace_execute_command_terminal_describe",
        "command": command,
        "cwd": cwd
    })
}

fn dashboard_prompt_host_vscode_workspace_get_diagnostics_test_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_workspace_get_diagnostics_test_describe",
        "mode": "test_fixture"
    })
}

fn dashboard_prompt_host_vscode_workspace_get_diagnostics_describe(payload: &Value) -> Value {
    let uri = clean_text(payload.get("uri").and_then(Value::as_str).unwrap_or(""), 1200);
    let severity_min = clean_text(
        payload
            .get("severity_min")
            .and_then(Value::as_str)
            .unwrap_or("hint"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_workspace_get_diagnostics_describe",
        "uri": uri,
        "severity_min": severity_min
    })
}

fn dashboard_prompt_host_vscode_workspace_get_workspace_paths_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_workspace_get_workspace_paths_describe",
        "fields": ["workspace_folders", "active_workspace"]
    })
}

fn dashboard_prompt_hostbridge_window_workspace_tail_route_extension(
    _root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.openSettings.describe" => {
            Some(dashboard_prompt_host_vscode_window_open_settings_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.showInputBox.describe" => {
            Some(dashboard_prompt_host_vscode_window_show_input_box_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.showMessage.describe" => {
            Some(dashboard_prompt_host_vscode_window_show_message_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.showOpenDialogue.describe" => {
            Some(dashboard_prompt_host_vscode_window_show_open_dialogue_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.showSaveDialog.describe" => {
            Some(dashboard_prompt_host_vscode_window_show_save_dialog_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.window.showTextDocument.describe" => {
            Some(dashboard_prompt_host_vscode_window_show_text_document_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.executeCommandInTerminal.describe" => {
            Some(dashboard_prompt_host_vscode_workspace_execute_command_terminal_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.getDiagnosticsTest.describe" => {
            Some(dashboard_prompt_host_vscode_workspace_get_diagnostics_test_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.getDiagnostics.describe" => {
            Some(dashboard_prompt_host_vscode_workspace_get_diagnostics_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.getWorkspacePaths.describe" => {
            Some(dashboard_prompt_host_vscode_workspace_get_workspace_paths_describe())
        }
        _ => None,
    }
}
