#[test]
fn dashboard_system_prompt_hooks_runtime_locks_tail_routes_contract_wave_690() {
    let root = tempfile::tempdir().expect("tempdir");

    let model_context = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.hookModelContext.describe",
        &json!({"context_window": "default"}),
    );
    assert!(model_context.ok);
    assert_eq!(
        model_context
            .payload
            .unwrap_or_else(|| json!({}))
            .get("context_window")
            .and_then(Value::as_str),
        Some("default")
    );

    let hooks_utils = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.hooksUtils.describe",
        &json!({"helper": "normalize"}),
    );
    assert!(hooks_utils.ok);
    assert_eq!(
        hooks_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("helper")
            .and_then(Value::as_str),
        Some("normalize")
    );

    let notification_hook = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.notificationHook.describe",
        &json!({"delivery_channel": "inline"}),
    );
    assert!(notification_hook.ok);
    assert_eq!(
        notification_hook
            .payload
            .unwrap_or_else(|| json!({}))
            .get("delivery_channel")
            .and_then(Value::as_str),
        Some("inline")
    );

    let precompact_executor = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.precompactExecutor.describe",
        &json!({"compaction_mode": "bounded"}),
    );
    assert!(precompact_executor.ok);
    assert_eq!(
        precompact_executor
            .payload
            .unwrap_or_else(|| json!({}))
            .get("compaction_mode")
            .and_then(Value::as_str),
        Some("bounded")
    );

    let shell_escape = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.shellEscape.describe",
        &json!({"quote_style": "single"}),
    );
    assert!(shell_escape.ok);
    assert_eq!(
        shell_escape
            .payload
            .unwrap_or_else(|| json!({}))
            .get("quote_style")
            .and_then(Value::as_str),
        Some("single")
    );

    let templates = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.templates.describe",
        &json!({"template_set": "default"}),
    );
    assert!(templates.ok);
    assert_eq!(
        templates
            .payload
            .unwrap_or_else(|| json!({}))
            .get("template_set")
            .and_then(Value::as_str),
        Some("default")
    );

    let hooks_misc_utils = run_action(
        root.path(),
        "dashboard.prompts.system.hooks.utils.describe",
        &json!({"utility_scope": "hooks"}),
    );
    assert!(hooks_misc_utils.ok);
    assert_eq!(
        hooks_misc_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("utility_scope")
            .and_then(Value::as_str),
        Some("hooks")
    );

    let ignore_controller = run_action(
        root.path(),
        "dashboard.prompts.system.ignore.clineIgnoreController.describe",
        &json!({"ignore_profile": "default"}),
    );
    assert!(ignore_controller.ok);
    assert_eq!(
        ignore_controller
            .payload
            .unwrap_or_else(|| json!({}))
            .get("ignore_profile")
            .and_then(Value::as_str),
        Some("default")
    );

    let folder_lock_utils = run_action(
        root.path(),
        "dashboard.prompts.system.locks.folderLockUtils.describe",
        &json!({"lock_scope": "workspace"}),
    );
    assert!(folder_lock_utils.ok);
    assert_eq!(
        folder_lock_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("lock_scope")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let sqlite_lock_manager = run_action(
        root.path(),
        "dashboard.prompts.system.locks.sqliteLockManager.describe",
        &json!({"lock_backend": "sqlite"}),
    );
    assert!(sqlite_lock_manager.ok);
    assert_eq!(
        sqlite_lock_manager
            .payload
            .unwrap_or_else(|| json!({}))
            .get("lock_backend")
            .and_then(Value::as_str),
        Some("sqlite")
    );
}
