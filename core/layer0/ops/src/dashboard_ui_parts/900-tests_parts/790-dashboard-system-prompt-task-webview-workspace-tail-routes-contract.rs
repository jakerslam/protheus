#[test]
fn dashboard_system_prompt_task_webview_workspace_tail_routes_contract_wave_790() {
    let root = tempfile::tempdir().expect("tempdir");

    let hook_execution_type = run_action(
        root.path(),
        "dashboard.prompts.system.task.types.hookExecution.describe",
        &json!({"hook_mode": "pre_tool_use"}),
    );
    assert!(hook_execution_type.ok);
    assert_eq!(
        hook_execution_type
            .payload
            .unwrap_or_else(|| json!({}))
            .get("hook_mode")
            .and_then(Value::as_str),
        Some("pre_tool_use")
    );

    let task_utils = run_action(
        root.path(),
        "dashboard.prompts.system.task.utils.describe",
        &json!({"utility": "task_utils"}),
    );
    assert!(task_utils.ok);
    assert_eq!(
        task_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("utility")
            .and_then(Value::as_str),
        Some("task_utils")
    );

    let build_user_feedback_content = run_action(
        root.path(),
        "dashboard.prompts.system.task.utils.buildUserFeedbackContent.describe",
        &json!({"feedback_profile": "concise"}),
    );
    assert!(build_user_feedback_content.ok);
    assert_eq!(
        build_user_feedback_content
            .payload
            .unwrap_or_else(|| json!({}))
            .get("feedback_profile")
            .and_then(Value::as_str),
        Some("concise")
    );

    let extract_user_prompt_from_content = run_action(
        root.path(),
        "dashboard.prompts.system.task.utils.extractUserPromptFromContent.describe",
        &json!({"extraction_mode": "strict"}),
    );
    assert!(extract_user_prompt_from_content.ok);
    assert_eq!(
        extract_user_prompt_from_content
            .payload
            .unwrap_or_else(|| json!({}))
            .get("extraction_mode")
            .and_then(Value::as_str),
        Some("strict")
    );

    let webview_provider = run_action(
        root.path(),
        "dashboard.prompts.system.webview.webviewProvider.describe",
        &json!({"provider_mode": "embedded"}),
    );
    assert!(webview_provider.ok);
    assert_eq!(
        webview_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider_mode")
            .and_then(Value::as_str),
        Some("embedded")
    );

    let webview_nonce = run_action(
        root.path(),
        "dashboard.prompts.system.webview.getNonce.describe",
        &json!({"nonce_policy": "per_render"}),
    );
    assert!(webview_nonce.ok);
    assert_eq!(
        webview_nonce
            .payload
            .unwrap_or_else(|| json!({}))
            .get("nonce_policy")
            .and_then(Value::as_str),
        Some("per_render")
    );

    let webview_index = run_action(
        root.path(),
        "dashboard.prompts.system.webview.index.describe",
        &json!({"index_scope": "webview"}),
    );
    assert!(webview_index.ok);
    assert_eq!(
        webview_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("index_scope")
            .and_then(Value::as_str),
        Some("webview")
    );

    let migration_reporter = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.migrationReporter.describe",
        &json!({"report_mode": "summary"}),
    );
    assert!(migration_reporter.ok);
    assert_eq!(
        migration_reporter
            .payload
            .unwrap_or_else(|| json!({}))
            .get("report_mode")
            .and_then(Value::as_str),
        Some("summary")
    );

    let workspace_path_adapter = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.workspacePathAdapter.describe",
        &json!({"adapter_mode": "workspace_relative"}),
    );
    assert!(workspace_path_adapter.ok);
    assert_eq!(
        workspace_path_adapter
            .payload
            .unwrap_or_else(|| json!({}))
            .get("adapter_mode")
            .and_then(Value::as_str),
        Some("workspace_relative")
    );

    let workspace_resolver = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.workspaceResolver.describe",
        &json!({"resolver_mode": "strict"}),
    );
    assert!(workspace_resolver.ok);
    assert_eq!(
        workspace_resolver
            .payload
            .unwrap_or_else(|| json!({}))
            .get("resolver_mode")
            .and_then(Value::as_str),
        Some("strict")
    );
}
