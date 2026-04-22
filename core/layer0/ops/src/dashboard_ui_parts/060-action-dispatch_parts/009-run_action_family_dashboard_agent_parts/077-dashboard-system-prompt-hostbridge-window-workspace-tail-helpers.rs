fn dashboard_prompt_hosts_surface_vscode_window_open_settings_describe(payload: &Value) -> Value {
    let section = clean_text(
        payload
            .get("section")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_window_open_settings_describe",
        "section": section
    })
}

fn dashboard_prompt_hosts_surface_vscode_window_show_input_box_describe(payload: &Value) -> Value {
    let prompt = clean_text(
        payload
            .get("prompt")
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    let placeholder = clean_text(
        payload
            .get("placeholder")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_window_show_input_box_describe",
        "prompt": prompt,
        "placeholder": placeholder
    })
}

fn dashboard_prompt_hosts_surface_vscode_window_show_message_describe(payload: &Value) -> Value {
    let level = clean_text(
        payload
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("info"),
        60,
    )
    .to_ascii_lowercase();
    let message = clean_text(
        payload
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(""),
        260,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_window_show_message_describe",
        "level": level,
        "message": message
    })
}

fn dashboard_prompt_hosts_surface_vscode_window_show_open_dialogue_describe(
    payload: &Value,
) -> Value {
    let can_select_many = payload
        .get("can_select_many")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_window_show_open_dialogue_describe",
        "can_select_many": can_select_many
    })
}

fn dashboard_prompt_hosts_surface_vscode_window_show_save_dialog_describe(payload: &Value) -> Value {
    let default_uri = clean_text(
        payload
            .get("default_uri")
            .and_then(Value::as_str)
            .unwrap_or(""),
        300,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_window_show_save_dialog_describe",
        "default_uri": default_uri
    })
}

fn dashboard_prompt_hosts_surface_vscode_window_show_text_document_describe(payload: &Value) -> Value {
    let uri = clean_text(
        payload
            .get("uri")
            .and_then(Value::as_str)
            .unwrap_or(""),
        300,
    );
    let preserve_focus = payload
        .get("preserve_focus")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_window_show_text_document_describe",
        "uri": uri,
        "preserve_focus": preserve_focus
    })
}

fn dashboard_prompt_hosts_surface_vscode_workspace_execute_command_terminal_describe(
    payload: &Value,
) -> Value {
    let command = clean_text(
        payload
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or(""),
        260,
    );
    let terminal_name = clean_text(
        payload
            .get("terminal_name")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_workspace_execute_command_terminal_describe",
        "command": command,
        "terminal_name": terminal_name
    })
}

fn dashboard_prompt_hosts_surface_vscode_workspace_get_diagnostics_test_describe(
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
        "type": "dashboard_prompts_system_hosts_surface_vscode_workspace_get_diagnostics_test_describe",
        "test_profile": test_profile
    })
}

fn dashboard_prompt_hosts_surface_vscode_workspace_get_diagnostics_describe(payload: &Value) -> Value {
    let severity = clean_text(
        payload
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("all"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_workspace_get_diagnostics_describe",
        "severity": severity
    })
}

fn dashboard_prompt_hosts_surface_vscode_workspace_get_paths_describe(payload: &Value) -> Value {
    let include_virtual = payload
        .get("include_virtual")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_workspace_get_paths_describe",
        "include_virtual": include_virtual
    })
}

fn dashboard_prompt_hosts_surface_tail_hostbridge_window_workspace_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.openSettings.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_window_open_settings_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.showInputBox.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_window_show_input_box_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.showMessage.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_window_show_message_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.showOpenDialogue.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_window_show_open_dialogue_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.showSaveDialog.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_window_show_save_dialog_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.window.showTextDocument.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_window_show_text_document_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.executeCommandInTerminal.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_workspace_execute_command_terminal_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.getDiagnosticsTest.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_workspace_get_diagnostics_test_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.getDiagnostics.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_workspace_get_diagnostics_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.getWorkspacePaths.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_workspace_get_paths_describe(payload))
        }
        _ => dashboard_prompt_hosts_surface_tail_hostbridge_workspace_review_terminal_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}
include!("078-dashboard-system-prompt-hostbridge-workspace-review-terminal-tail-helpers.rs");
