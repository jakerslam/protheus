#[test]
fn dashboard_hooks_test_scenario_routes_cover_setup_factory_and_utils() {
    let root = tempfile::tempdir().expect("tempdir");

    let setup = run_action(
        root.path(),
        "dashboard.hooks.test.setupFixture",
        &json!({"fixture":"hooks-fixture-a"}),
    );
    assert!(setup.ok);
    let setup_payload = setup.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        setup_payload.get("type").and_then(Value::as_str),
        Some("dashboard_hooks_test_setup_fixture")
    );

    let factory = run_action(
        root.path(),
        "dashboard.hooks.test.factory.validate",
        &json!({
            "hook_id":"hook.test.factory",
            "phase":"pre_tool_use",
            "command":"echo ok"
        }),
    );
    assert!(factory.ok);
    let factory_payload = factory.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        factory_payload.get("valid").and_then(Value::as_bool),
        Some(true)
    );

    let normalize = run_action(
        root.path(),
        "dashboard.hooks.test.utils.normalize",
        &json!({"values":["Alpha"," alpha ","BETA"]}),
    );
    assert!(normalize.ok);
    let normalize_payload = normalize.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        normalize_payload
            .pointer("/normalized/0")
            .and_then(Value::as_str),
        Some("alpha")
    );

    let shell = run_action(
        root.path(),
        "dashboard.hooks.test.shellEscape.inspect",
        &json!({"input":"say \"hello\""}),
    );
    assert!(shell.ok);
    let shell_payload = shell.payload.unwrap_or_else(|| json!({}));
    assert!(
        shell_payload
            .get("escaped")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("\\\"hello\\\"")
    );
}

#[test]
fn dashboard_hooks_test_scenario_routes_cover_task_lifecycle_and_notifications() {
    let root = tempfile::tempdir().expect("tempdir");

    let notify = run_action(
        root.path(),
        "dashboard.hooks.test.notification.emit",
        &json!({"level":"warn","message":"policy drift"}),
    );
    assert!(notify.ok);

    let cancel = run_action(
        root.path(),
        "dashboard.hooks.test.taskCancel.simulate",
        &json!({"hook_id":"hook.task.cancel"}),
    );
    assert!(cancel.ok);
    let cancel_payload = cancel.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        cancel_payload
            .pointer("/complete/status")
            .and_then(Value::as_str),
        Some("cancelled")
    );

    let complete = run_action(
        root.path(),
        "dashboard.hooks.test.taskComplete.simulate",
        &json!({"hook_id":"hook.task.complete"}),
    );
    assert!(complete.ok);
    let complete_payload = complete.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        complete_payload
            .pointer("/complete/status")
            .and_then(Value::as_str),
        Some("completed")
    );

    let resume = run_action(
        root.path(),
        "dashboard.hooks.test.taskResume.simulate",
        &json!({"hook_id":"hook.task.resume"}),
    );
    assert!(resume.ok);
    let resume_payload = resume.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        resume_payload
            .pointer("/complete/status")
            .and_then(Value::as_str),
        Some("completed")
    );
}
