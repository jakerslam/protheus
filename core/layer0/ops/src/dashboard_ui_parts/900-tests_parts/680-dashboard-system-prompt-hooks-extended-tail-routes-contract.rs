#[test]
fn dashboard_system_prompt_hooks_extended_tail_routes_contract_wave_680() {
    let root = tempfile::tempdir().expect("tempdir");

    let setup = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.setup.describe",
        &json!({"fixture": "default"}),
    );
    assert!(setup.ok);
    assert_eq!(
        setup.payload
            .unwrap_or_else(|| json!({}))
            .get("fixture")
            .and_then(Value::as_str),
        Some("default")
    );

    let shell_escape = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.shellEscape.describe",
        &json!({"shell": "zsh"}),
    );
    assert!(shell_escape.ok);
    assert_eq!(
        shell_escape
            .payload
            .unwrap_or_else(|| json!({}))
            .get("shell")
            .and_then(Value::as_str),
        Some("zsh")
    );

    let task_cancel = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.taskCancel.describe",
        &json!({"cancel_mode": "graceful"}),
    );
    assert!(task_cancel.ok);
    assert_eq!(
        task_cancel
            .payload
            .unwrap_or_else(|| json!({}))
            .get("cancel_mode")
            .and_then(Value::as_str),
        Some("graceful")
    );

    let task_complete = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.taskComplete.describe",
        &json!({"completion": "success"}),
    );
    assert!(task_complete.ok);
    assert_eq!(
        task_complete
            .payload
            .unwrap_or_else(|| json!({}))
            .get("completion")
            .and_then(Value::as_str),
        Some("success")
    );

    let task_resume = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.taskResume.describe",
        &json!({"resume_mode": "from_checkpoint"}),
    );
    assert!(task_resume.ok);
    assert_eq!(
        task_resume
            .payload
            .unwrap_or_else(|| json!({}))
            .get("resume_mode")
            .and_then(Value::as_str),
        Some("from_checkpoint")
    );

    let task_start = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.taskStart.describe",
        &json!({"start_mode": "fresh"}),
    );
    assert!(task_start.ok);
    assert_eq!(
        task_start
            .payload
            .unwrap_or_else(|| json!({}))
            .get("start_mode")
            .and_then(Value::as_str),
        Some("fresh")
    );

    let test_utils = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.testUtils.describe",
        &json!({"helper": "mock_context"}),
    );
    assert!(test_utils.ok);
    assert_eq!(
        test_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("helper")
            .and_then(Value::as_str),
        Some("mock_context")
    );

    let user_prompt_submit = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.tests.userPromptSubmit.describe",
        &json!({"submit_mode": "interactive"}),
    );
    assert!(user_prompt_submit.ok);
    assert_eq!(
        user_prompt_submit
            .payload
            .unwrap_or_else(|| json!({}))
            .get("submit_mode")
            .and_then(Value::as_str),
        Some("interactive")
    );

    let hook_executor = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.hookExecutor.describe",
        &json!({"executor": "default"}),
    );
    assert!(hook_executor.ok);
    assert_eq!(
        hook_executor
            .payload
            .unwrap_or_else(|| json!({}))
            .get("executor")
            .and_then(Value::as_str),
        Some("default")
    );

    let hook_factory = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.hookFactory.describe",
        &json!({"factory": "default"}),
    );
    assert!(hook_factory.ok);
    assert_eq!(
        hook_factory
            .payload
            .unwrap_or_else(|| json!({}))
            .get("factory")
            .and_then(Value::as_str),
        Some("default")
    );
}
