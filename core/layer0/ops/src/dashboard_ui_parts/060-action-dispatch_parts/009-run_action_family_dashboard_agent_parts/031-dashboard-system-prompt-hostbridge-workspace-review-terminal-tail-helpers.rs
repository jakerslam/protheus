fn dashboard_prompt_host_vscode_workspace_open_cline_sidebar_panel_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_workspace_open_cline_sidebar_panel_describe",
        "panel": "cline_sidebar"
    })
}

fn dashboard_prompt_host_vscode_workspace_open_folder_describe(payload: &Value) -> Value {
    let folder_path = clean_text(
        payload
            .get("folder_path")
            .or_else(|| payload.get("path"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        1200,
    );
    let force_new_window = payload
        .get("force_new_window")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_workspace_open_folder_describe",
        "folder_path": folder_path,
        "force_new_window": force_new_window
    })
}

fn dashboard_prompt_host_vscode_workspace_open_file_explorer_panel_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_workspace_open_file_explorer_panel_describe",
        "panel": "file_explorer"
    })
}

fn dashboard_prompt_host_vscode_workspace_open_problems_panel_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_workspace_open_problems_panel_describe",
        "panel": "problems"
    })
}

fn dashboard_prompt_host_vscode_workspace_open_terminal_panel_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_workspace_open_terminal_panel_describe",
        "panel": "terminal"
    })
}

fn dashboard_prompt_host_vscode_workspace_save_open_document_dirty_test_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_workspace_save_open_document_dirty_test_describe",
        "mode": "test_fixture"
    })
}

fn dashboard_prompt_host_vscode_workspace_save_open_document_dirty_describe(payload: &Value) -> Value {
    let uri = clean_text(payload.get("uri").and_then(Value::as_str).unwrap_or(""), 1200);
    let save_if_dirty = payload
        .get("save_if_dirty")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_workspace_save_open_document_dirty_describe",
        "uri": uri,
        "save_if_dirty": save_if_dirty
    })
}

fn dashboard_prompt_host_vscode_review_comment_review_controller_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_review_comment_review_controller_describe",
        "controller": "comment_review"
    })
}

fn dashboard_prompt_host_vscode_terminal_manager_describe(payload: &Value) -> Value {
    let terminal_id = clean_text(
        payload
            .get("terminal_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_terminal_manager_describe",
        "terminal_id": terminal_id
    })
}

fn dashboard_prompt_host_vscode_terminal_process_test_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_host_vscode_terminal_process_test_describe",
        "mode": "test_fixture"
    })
}

fn dashboard_prompt_hostbridge_workspace_review_terminal_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.openClineSidebarPanel.describe" => {
            Some(dashboard_prompt_host_vscode_workspace_open_cline_sidebar_panel_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.openFolder.describe" => {
            Some(dashboard_prompt_host_vscode_workspace_open_folder_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.openInFileExplorerPanel.describe" => {
            Some(dashboard_prompt_host_vscode_workspace_open_file_explorer_panel_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.openProblemsPanel.describe" => {
            Some(dashboard_prompt_host_vscode_workspace_open_problems_panel_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.openTerminalPanel.describe" => {
            Some(dashboard_prompt_host_vscode_workspace_open_terminal_panel_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.saveOpenDocumentIfDirtyTest.describe" => {
            Some(dashboard_prompt_host_vscode_workspace_save_open_document_dirty_test_describe())
        }
        "dashboard.prompts.system.hosts.vscode.hostbridge.workspace.saveOpenDocumentIfDirty.describe" => {
            Some(dashboard_prompt_host_vscode_workspace_save_open_document_dirty_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.review.vscodeCommentReviewController.describe" => {
            Some(dashboard_prompt_host_vscode_review_comment_review_controller_describe())
        }
        "dashboard.prompts.system.hosts.vscode.terminal.vscodeTerminalManager.describe" => {
            Some(dashboard_prompt_host_vscode_terminal_manager_describe(payload))
        }
        "dashboard.prompts.system.hosts.vscode.terminal.vscodeTerminalProcessTest.describe" => {
            Some(dashboard_prompt_host_vscode_terminal_process_test_describe())
        }
        _ => dashboard_prompt_host_vscode_terminal_checkpoint_tail_route_extension(
            root, normalized, payload,
        ),
    }
}

include!("032-dashboard-system-prompt-hostbridge-terminal-checkpoint-tail-helpers.rs");
