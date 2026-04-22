#[test]
fn dashboard_system_prompt_controller_task_ui_tail_routes_contract_wave_630() {
    let root = tempfile::tempdir().expect("tempdir");

    let explain_changes = run_action(
        root.path(),
        "dashboard.prompts.system.controller.task.explainChangesShared.describe",
        &json!({"format": "summary"}),
    );
    assert!(explain_changes.ok);
    assert_eq!(
        explain_changes
            .payload
            .unwrap_or_else(|| json!({}))
            .get("format")
            .and_then(Value::as_str),
        Some("summary")
    );

    let export_with_id = run_action(
        root.path(),
        "dashboard.prompts.system.controller.task.exportTaskWithId.describe",
        &json!({"export_mode": "json"}),
    );
    assert!(export_with_id.ok);
    assert_eq!(
        export_with_id
            .payload
            .unwrap_or_else(|| json!({}))
            .get("export_mode")
            .and_then(Value::as_str),
        Some("json")
    );

    let task_history = run_action(
        root.path(),
        "dashboard.prompts.system.controller.task.getTaskHistory.describe",
        &json!({"window": "recent"}),
    );
    assert!(task_history.ok);
    assert_eq!(
        task_history
            .payload
            .unwrap_or_else(|| json!({}))
            .get("window")
            .and_then(Value::as_str),
        Some("recent")
    );

    let total_size = run_action(
        root.path(),
        "dashboard.prompts.system.controller.task.getTotalTasksSize.describe",
        &json!({"unit": "bytes"}),
    );
    assert!(total_size.ok);
    assert_eq!(
        total_size
            .payload
            .unwrap_or_else(|| json!({}))
            .get("unit")
            .and_then(Value::as_str),
        Some("bytes")
    );

    let new_task = run_action(
        root.path(),
        "dashboard.prompts.system.controller.task.newTask.describe",
        &json!({"template": "default"}),
    );
    assert!(new_task.ok);
    assert_eq!(
        new_task
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template")
            .and_then(Value::as_str),
        Some("default")
    );

    let show_with_id = run_action(
        root.path(),
        "dashboard.prompts.system.controller.task.showTaskWithId.describe",
        &json!({"detail": "full"}),
    );
    assert!(show_with_id.ok);
    assert_eq!(
        show_with_id
            .payload
            .unwrap_or_else(|| json!({}))
            .get("detail")
            .and_then(Value::as_str),
        Some("full")
    );

    let completion_view_changes = run_action(
        root.path(),
        "dashboard.prompts.system.controller.task.taskCompletionViewChanges.describe",
        &json!({"mode": "diff"}),
    );
    assert!(completion_view_changes.ok);
    assert_eq!(
        completion_view_changes
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("diff")
    );

    let task_feedback = run_action(
        root.path(),
        "dashboard.prompts.system.controller.task.taskFeedback.describe",
        &json!({"channel": "inline"}),
    );
    assert!(task_feedback.ok);
    assert_eq!(
        task_feedback
            .payload
            .unwrap_or_else(|| json!({}))
            .get("channel")
            .and_then(Value::as_str),
        Some("inline")
    );

    let toggle_favorite = run_action(
        root.path(),
        "dashboard.prompts.system.controller.task.toggleTaskFavorite.describe",
        &json!({"state": "toggle"}),
    );
    assert!(toggle_favorite.ok);
    assert_eq!(
        toggle_favorite
            .payload
            .unwrap_or_else(|| json!({}))
            .get("state")
            .and_then(Value::as_str),
        Some("toggle")
    );

    let webview_html = run_action(
        root.path(),
        "dashboard.prompts.system.controller.ui.getWebviewHtml.describe",
        &json!({"shell": "webview"}),
    );
    assert!(webview_html.ok);
    assert_eq!(
        webview_html
            .payload
            .unwrap_or_else(|| json!({}))
            .get("shell")
            .and_then(Value::as_str),
        Some("webview")
    );
}
