#[test]
fn dashboard_hooks_test_scenario_extended_routes_cover_templates_ignore_and_digest() {
    let root = tempfile::tempdir().expect("tempdir");

    let render = run_action(
        root.path(),
        "dashboard.hooks.test.templates.render",
        &json!({
            "template": "hello {{name}} from {{city}}",
            "values": {
                "name": "jay",
                "city": "denver"
            }
        }),
    );
    assert!(render.ok);
    let render_payload = render.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        render_payload.get("type").and_then(Value::as_str),
        Some("dashboard_hooks_test_templates_render")
    );
    assert!(
        render_payload
            .get("rendered")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("jay")
    );

    let placeholders = run_action(
        root.path(),
        "dashboard.hooks.test.templates.placeholders",
        &json!({
            "template": "a {{first}} b {{second}} {{first}}"
        }),
    );
    assert!(placeholders.ok);
    let placeholders_payload = placeholders.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        placeholders_payload
            .pointer("/placeholders/0")
            .and_then(Value::as_str),
        Some("first")
    );

    let digest = run_action(
        root.path(),
        "dashboard.hooks.test.utils.digest",
        &json!({
            "x": 1,
            "y": "two"
        }),
    );
    assert!(digest.ok);
    let digest_payload = digest.payload.unwrap_or_else(|| json!({}));
    assert!(
        digest_payload
            .get("digest")
            .and_then(Value::as_str)
            .unwrap_or("")
            .len()
            > 8
    );

    let ignore = run_action(
        root.path(),
        "dashboard.hooks.test.ignore.evaluate",
        &json!({
            "path": "src/tmp/private/token.txt",
            "patterns": ["tmp/private", ".secrets"]
        }),
    );
    assert!(ignore.ok);
    let ignore_payload = ignore.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        ignore_payload
            .get("ignored")
            .and_then(Value::as_bool),
        Some(true)
    );

    let precompact = run_action(
        root.path(),
        "dashboard.hooks.test.precompact.evaluate",
        &json!({
            "before_bytes": 500,
            "after_bytes": 350
        }),
    );
    assert!(precompact.ok);
    let precompact_payload = precompact.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        precompact_payload
            .get("saved_bytes")
            .and_then(Value::as_i64),
        Some(150)
    );
}

#[test]
fn dashboard_hooks_test_scenario_extended_routes_cover_task_start_and_user_prompt_submit() {
    let root = tempfile::tempdir().expect("tempdir");

    let task_start = run_action(
        root.path(),
        "dashboard.hooks.test.taskStart.simulate",
        &json!({
            "hook_id": "hook.task.start"
        }),
    );
    assert!(task_start.ok);
    let task_start_payload = task_start.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        task_start_payload
            .pointer("/complete/status")
            .and_then(Value::as_str),
        Some("completed")
    );

    let submit = run_action(
        root.path(),
        "dashboard.hooks.test.userPromptSubmit.simulate",
        &json!({
            "hook_id": "hook.user.prompt.submit",
            "prompt": "please summarize this workflow"
        }),
    );
    assert!(submit.ok);
    let submit_payload = submit.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        submit_payload.get("type").and_then(Value::as_str),
        Some("dashboard_hooks_test_user_prompt_submit_simulate")
    );
    assert!(
        submit_payload
            .get("prompt_length")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            > 5
    );
}
