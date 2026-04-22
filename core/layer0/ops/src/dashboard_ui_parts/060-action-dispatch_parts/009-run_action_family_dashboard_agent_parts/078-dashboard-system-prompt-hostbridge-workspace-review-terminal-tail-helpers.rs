fn dashboard_prompt_hosts_surface_vscode_workspace_open_cline_sidebar_panel_describe(
    payload: &Value,
) -> Value {
    let focus = payload
        .get("focus")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_workspace_open_cline_sidebar_panel_describe",
        "focus": focus
    })
}

fn dashboard_prompt_hosts_surface_vscode_workspace_open_folder_describe(payload: &Value) -> Value {
    let uri = clean_text(
        payload
            .get("uri")
            .and_then(Value::as_str)
            .unwrap_or(""),
        300,
    );
    let force_new_window = payload
        .get("force_new_window")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_workspace_open_folder_describe",
        "uri": uri,
        "force_new_window": force_new_window
    })
}

fn dashboard_prompt_hosts_surface_vscode_workspace_open_file_explorer_panel_describe(
    payload: &Value,
) -> Value {
    let reveal = payload
        .get("reveal")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_workspace_open_file_explorer_panel_describe",
        "reveal": reveal
    })
}

fn dashboard_prompt_hosts_surface_vscode_workspace_open_problems_panel_describe(
    payload: &Value,
) -> Value {
    let auto_focus = payload
        .get("auto_focus")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_workspace_open_problems_panel_describe",
        "auto_focus": auto_focus
    })
}

fn dashboard_prompt_hosts_surface_vscode_workspace_open_terminal_panel_describe(
    payload: &Value,
) -> Value {
    let panel = clean_text(
        payload
            .get("panel")
            .and_then(Value::as_str)
            .unwrap_or("terminal"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_workspace_open_terminal_panel_describe",
        "panel": panel
    })
}

fn dashboard_prompt_hosts_surface_vscode_workspace_save_open_document_if_dirty_test_describe(
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
        "type": "dashboard_prompts_system_hosts_surface_vscode_workspace_save_open_document_if_dirty_test_describe",
        "test_profile": test_profile
    })
}

fn dashboard_prompt_hosts_surface_vscode_workspace_save_open_document_if_dirty_describe(
    payload: &Value,
) -> Value {
    let save_all = payload
        .get("save_all")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_workspace_save_open_document_if_dirty_describe",
        "save_all": save_all
    })
}

fn dashboard_prompt_hosts_surface_vscode_review_controller_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("inline"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_review_controller_describe",
        "mode": mode
    })
}

fn dashboard_prompt_hosts_surface_vscode_terminal_manager_describe(payload: &Value) -> Value {
    let strategy = clean_text(
        payload
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or("managed"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_hosts_surface_vscode_terminal_manager_describe",
        "strategy": strategy
    })
}

fn dashboard_prompt_hosts_surface_vscode_terminal_process_test_describe(payload: &Value) -> Value {
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
        "type": "dashboard_prompts_system_hosts_surface_vscode_terminal_process_test_describe",
        "test_profile": test_profile
    })
}

fn dashboard_prompt_hosts_surface_tail_hostbridge_workspace_review_terminal_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.openClineSidebarPanel.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_workspace_open_cline_sidebar_panel_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.openFolder.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_workspace_open_folder_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.openInFileExplorerPanel.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_workspace_open_file_explorer_panel_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.openProblemsPanel.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_workspace_open_problems_panel_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.openTerminalPanel.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_workspace_open_terminal_panel_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.saveOpenDocumentIfDirtyTest.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_workspace_save_open_document_if_dirty_test_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.workspace.saveOpenDocumentIfDirty.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_workspace_save_open_document_if_dirty_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.review.vscodeCommentReviewController.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_review_controller_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.terminal.vscodeTerminalManager.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_terminal_manager_describe(payload))
        }
        "dashboard.prompts.system.hosts.surface.vscode.terminal.vscodeTerminalProcessTest.describe" => {
            Some(dashboard_prompt_hosts_surface_vscode_terminal_process_test_describe(payload))
        }
        _ => dashboard_prompt_hosts_surface_tail_terminal_checkpoint_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}
include!("079-dashboard-system-prompt-terminal-checkpoint-tail-helpers.rs");
