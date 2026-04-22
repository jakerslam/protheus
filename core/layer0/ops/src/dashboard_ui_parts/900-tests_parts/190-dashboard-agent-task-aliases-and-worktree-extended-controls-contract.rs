#[test]
fn dashboard_agent_task_aliases_resolve_to_runtime_contracts() {
    let root = tempfile::tempdir().expect("tempdir");

    let created = run_action(
        root.path(),
        "dashboard.agent.task.newTask",
        &json!({
            "title": "Alias Task",
            "description": "created through newTask alias",
            "timeout_secs": 120
        }),
    );
    assert!(created.ok);
    let created_payload = created.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        created_payload.get("type").and_then(Value::as_str),
        Some("dashboard_agent_task_created")
    );
    let task_id = clean_text(
        created_payload
            .pointer("/task/id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    assert!(!task_id.is_empty());

    let shown = run_action(
        root.path(),
        "dashboard.agent.task.showTaskWithId",
        &json!({
            "task_id": task_id
        }),
    );
    assert!(shown.ok);
    let shown_payload = shown.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        shown_payload.get("type").and_then(Value::as_str),
        Some("dashboard_agent_task_show_with_id")
    );
    assert_eq!(
        shown_payload
            .pointer("/task/id")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 80)),
        Some(task_id.clone())
    );

    let completion_view_changes = run_action(
        root.path(),
        "dashboard.agent.task.taskCompletionViewChanges",
        &json!({
            "before": {
                "id": task_id,
                "status": "running",
                "completion_percent": 20
            },
            "after": {
                "id": task_id,
                "status": "completed",
                "completion_percent": 100
            }
        }),
    );
    assert!(completion_view_changes.ok);
    let changes_payload = completion_view_changes.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        changes_payload.get("type").and_then(Value::as_str),
        Some("dashboard_agent_task_completion_view_changes")
    );
    assert!(
        changes_payload
            .get("changed_count")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= 1
    );
}

#[test]
fn dashboard_worktree_extended_controls_routes_follow_state_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let defaults = run_action(root.path(), "dashboard.worktree.getWorktreeDefaults", &json!({}));
    assert!(defaults.ok);
    let defaults_payload = defaults.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        defaults_payload.get("type").and_then(Value::as_str),
        Some("dashboard_worktree_defaults")
    );

    let create = run_action(
        root.path(),
        "dashboard.worktree.createWorktree",
        &json!({
            "path": "/tmp/infring-wt-a",
            "branch": "feature/a"
        }),
    );
    assert!(create.ok);

    let checkout = run_action(
        root.path(),
        "dashboard.worktree.checkoutBranch",
        &json!({
            "branch": "feature/b"
        }),
    );
    assert!(checkout.ok);
    let checkout_payload = checkout.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        checkout_payload
            .get("branch")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 200)),
        Some("feature/b".to_string())
    );

    let include = run_action(
        root.path(),
        "dashboard.worktree.createWorktreeInclude",
        &json!({
            "path": "/tmp/infring-wt-a"
        }),
    );
    assert!(include.ok);
    let include_payload = include.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        include_payload.get("type").and_then(Value::as_str),
        Some("dashboard_worktree_create_include")
    );

    let include_status = run_action(
        root.path(),
        "dashboard.worktree.getWorktreeIncludeStatus",
        &json!({}),
    );
    assert!(include_status.ok);
    let include_status_payload = include_status.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        include_status_payload
            .get("include_enabled")
            .and_then(Value::as_bool),
        Some(true)
    );

    let merge = run_action(
        root.path(),
        "dashboard.worktree.mergeWorktree",
        &json!({
            "source_branch": "feature/b",
            "target_branch": "main"
        }),
    );
    assert!(merge.ok);
    let merge_payload = merge.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        merge_payload
            .get("target_branch")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 200)),
        Some("main".to_string())
    );

    let view = run_action(
        root.path(),
        "dashboard.worktree.trackWorktreeViewOpened",
        &json!({
            "view": "sidebar"
        }),
    );
    assert!(view.ok);
    let view_payload = view.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        view_payload
            .get("view")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 120)),
        Some("sidebar".to_string())
    );
}
