fn dashboard_prompt_controller_task_explain_changes_shared_describe(payload: &Value) -> Value {
    let format = clean_text(
        payload
            .get("format")
            .and_then(Value::as_str)
            .unwrap_or("summary"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_task_explain_changes_shared_describe",
        "format": format
    })
}

fn dashboard_prompt_controller_task_export_with_id_describe(payload: &Value) -> Value {
    let export_mode = clean_text(
        payload
            .get("export_mode")
            .and_then(Value::as_str)
            .unwrap_or("json"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_task_export_with_id_describe",
        "export_mode": export_mode
    })
}

fn dashboard_prompt_controller_task_history_describe(payload: &Value) -> Value {
    let window = clean_text(
        payload
            .get("window")
            .and_then(Value::as_str)
            .unwrap_or("recent"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_task_history_describe",
        "window": window
    })
}

fn dashboard_prompt_controller_task_total_size_describe(payload: &Value) -> Value {
    let unit = clean_text(
        payload
            .get("unit")
            .and_then(Value::as_str)
            .unwrap_or("bytes"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_task_total_size_describe",
        "unit": unit
    })
}

fn dashboard_prompt_controller_task_new_describe(payload: &Value) -> Value {
    let template = clean_text(
        payload
            .get("template")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_task_new_describe",
        "template": template
    })
}

fn dashboard_prompt_controller_task_show_with_id_describe(payload: &Value) -> Value {
    let detail = clean_text(
        payload
            .get("detail")
            .and_then(Value::as_str)
            .unwrap_or("full"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_task_show_with_id_describe",
        "detail": detail
    })
}

fn dashboard_prompt_controller_task_completion_view_changes_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("diff"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_task_completion_view_changes_describe",
        "mode": mode
    })
}

fn dashboard_prompt_controller_task_feedback_describe(payload: &Value) -> Value {
    let channel = clean_text(
        payload
            .get("channel")
            .and_then(Value::as_str)
            .unwrap_or("inline"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_task_feedback_describe",
        "channel": channel
    })
}

fn dashboard_prompt_controller_task_toggle_favorite_describe(payload: &Value) -> Value {
    let state = clean_text(
        payload
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("toggle"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_task_toggle_favorite_describe",
        "state": state
    })
}

fn dashboard_prompt_controller_ui_webview_html_describe(payload: &Value) -> Value {
    let shell = clean_text(
        payload
            .get("shell")
            .and_then(Value::as_str)
            .unwrap_or("webview"),
        80,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_controller_ui_webview_html_describe",
        "shell": shell
    })
}

fn dashboard_prompt_controller_task_ui_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.controller.task.explainChangesShared.describe" => {
            Some(dashboard_prompt_controller_task_explain_changes_shared_describe(payload))
        }
        "dashboard.prompts.system.controller.task.exportTaskWithId.describe" => {
            Some(dashboard_prompt_controller_task_export_with_id_describe(payload))
        }
        "dashboard.prompts.system.controller.task.getTaskHistory.describe" => {
            Some(dashboard_prompt_controller_task_history_describe(payload))
        }
        "dashboard.prompts.system.controller.task.getTotalTasksSize.describe" => {
            Some(dashboard_prompt_controller_task_total_size_describe(payload))
        }
        "dashboard.prompts.system.controller.task.newTask.describe" => {
            Some(dashboard_prompt_controller_task_new_describe(payload))
        }
        "dashboard.prompts.system.controller.task.showTaskWithId.describe" => {
            Some(dashboard_prompt_controller_task_show_with_id_describe(payload))
        }
        "dashboard.prompts.system.controller.task.taskCompletionViewChanges.describe" => {
            Some(dashboard_prompt_controller_task_completion_view_changes_describe(payload))
        }
        "dashboard.prompts.system.controller.task.taskFeedback.describe" => {
            Some(dashboard_prompt_controller_task_feedback_describe(payload))
        }
        "dashboard.prompts.system.controller.task.toggleTaskFavorite.describe" => {
            Some(dashboard_prompt_controller_task_toggle_favorite_describe(payload))
        }
        "dashboard.prompts.system.controller.ui.getWebviewHtml.describe" => {
            Some(dashboard_prompt_controller_ui_webview_html_describe(payload))
        }
        _ => dashboard_prompt_controller_ui_event_tail_route_extension(root, normalized, payload),
    }
}

include!("056-dashboard-system-prompt-controller-ui-event-tail-helpers.rs");
