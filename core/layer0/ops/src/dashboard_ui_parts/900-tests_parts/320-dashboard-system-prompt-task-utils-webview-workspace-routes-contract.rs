#[test]
fn dashboard_system_prompt_task_utils_routes_contract_wave_320() {
    let root = tempfile::tempdir().expect("tempdir");

    let hook = run_action(
        root.path(),
        "dashboard.prompts.system.task.hookExecution.describe",
        &json!({"hook_name": "on_task_start", "phase": "pre", "blocking": true}),
    );
    assert!(hook.ok);
    let hook_payload = hook.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        hook_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_task_hook_execution_describe")
    );
    assert_eq!(
        hook_payload.get("blocking").and_then(Value::as_bool),
        Some(true)
    );

    let feedback = run_action(
        root.path(),
        "dashboard.prompts.system.task.utils.buildUserFeedbackContent",
        &json!({
            "summary": "Completed patch wave",
            "bullets": ["Added route", "Added test"]
        }),
    );
    assert!(feedback.ok);
    let feedback_payload = feedback.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        feedback_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_task_utils_build_user_feedback_content")
    );
    assert_eq!(
        feedback_payload.get("bullet_count").and_then(Value::as_i64),
        Some(2)
    );

    let extracted = run_action(
        root.path(),
        "dashboard.prompts.system.task.utils.extractUserPromptFromContent",
        &json!({"content": "System: hi\nUser: please run diagnostics"}),
    );
    assert!(extracted.ok);
    assert_eq!(
        extracted
            .payload
            .unwrap_or_else(|| json!({}))
            .get("user_prompt")
            .and_then(Value::as_str),
        Some("please run diagnostics")
    );
}

#[test]
fn dashboard_system_prompt_webview_and_workspace_routes_contract_wave_320() {
    let root = tempfile::tempdir().expect("tempdir");

    let nonce = run_action(
        root.path(),
        "dashboard.prompts.system.webview.getNonce",
        &json!({"seed": "abc"}),
    );
    assert!(nonce.ok);
    let nonce_payload = nonce.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        nonce_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_webview_get_nonce")
    );
    assert!(
        nonce_payload
            .get("nonce")
            .and_then(Value::as_str)
            .unwrap_or("")
            .starts_with("nonce-")
    );

    let path_resolve = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.pathAdapter.resolve",
        &json!({"workspace_root": "/tmp/workspace", "path": "src/main.rs"}),
    );
    assert!(path_resolve.ok);
    let path_resolve_payload = path_resolve.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        path_resolve_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_workspace_path_adapter_resolve")
    );
    assert!(
        path_resolve_payload
            .get("resolved_path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("/tmp/workspace")
    );

    let workspace_resolve = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.resolver.resolve",
        &json!({"candidates": ["/a", "/b"]}),
    );
    assert!(workspace_resolve.ok);
    assert_eq!(
        workspace_resolve
            .payload
            .unwrap_or_else(|| json!({}))
            .get("selected")
            .and_then(Value::as_str),
        Some("/a")
    );

    let migration = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.migrationReporter.report",
        &json!({"entries": [{"status": "ok"}, {"status": "failed"}]}),
    );
    assert!(migration.ok);
    assert_eq!(
        migration
            .payload
            .unwrap_or_else(|| json!({}))
            .get("failed")
            .and_then(Value::as_i64),
        Some(1)
    );
}
