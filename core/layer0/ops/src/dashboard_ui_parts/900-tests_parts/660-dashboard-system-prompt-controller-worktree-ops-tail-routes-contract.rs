#[test]
fn dashboard_system_prompt_controller_worktree_ops_tail_routes_contract_wave_660() {
    let root = tempfile::tempdir().expect("tempdir");

    let create_worktree = run_action(
        root.path(),
        "dashboard.prompts.system.controller.worktree.createWorktree.describe",
        &json!({"branch": "feature"}),
    );
    assert!(create_worktree.ok);
    assert_eq!(
        create_worktree
            .payload
            .unwrap_or_else(|| json!({}))
            .get("branch")
            .and_then(Value::as_str),
        Some("feature")
    );

    let create_worktree_include = run_action(
        root.path(),
        "dashboard.prompts.system.controller.worktree.createWorktreeInclude.describe",
        &json!({"include": "default"}),
    );
    assert!(create_worktree_include.ok);
    assert_eq!(
        create_worktree_include
            .payload
            .unwrap_or_else(|| json!({}))
            .get("include")
            .and_then(Value::as_str),
        Some("default")
    );

    let delete_worktree = run_action(
        root.path(),
        "dashboard.prompts.system.controller.worktree.deleteWorktree.describe",
        &json!({"mode": "safe"}),
    );
    assert!(delete_worktree.ok);
    assert_eq!(
        delete_worktree
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("safe")
    );

    let available_branches = run_action(
        root.path(),
        "dashboard.prompts.system.controller.worktree.getAvailableBranches.describe",
        &json!({"scope": "local"}),
    );
    assert!(available_branches.ok);
    assert_eq!(
        available_branches
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("local")
    );

    let worktree_defaults = run_action(
        root.path(),
        "dashboard.prompts.system.controller.worktree.getWorktreeDefaults.describe",
        &json!({"profile": "default"}),
    );
    assert!(worktree_defaults.ok);
    assert_eq!(
        worktree_defaults
            .payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("default")
    );

    let include_status = run_action(
        root.path(),
        "dashboard.prompts.system.controller.worktree.getWorktreeIncludeStatus.describe",
        &json!({"include_mode": "tracked"}),
    );
    assert!(include_status.ok);
    assert_eq!(
        include_status
            .payload
            .unwrap_or_else(|| json!({}))
            .get("include_mode")
            .and_then(Value::as_str),
        Some("tracked")
    );

    let list_worktrees = run_action(
        root.path(),
        "dashboard.prompts.system.controller.worktree.listWorktrees.describe",
        &json!({"list_mode": "active"}),
    );
    assert!(list_worktrees.ok);
    assert_eq!(
        list_worktrees
            .payload
            .unwrap_or_else(|| json!({}))
            .get("list_mode")
            .and_then(Value::as_str),
        Some("active")
    );

    let merge_worktree = run_action(
        root.path(),
        "dashboard.prompts.system.controller.worktree.mergeWorktree.describe",
        &json!({"strategy": "fast-forward"}),
    );
    assert!(merge_worktree.ok);
    assert_eq!(
        merge_worktree
            .payload
            .unwrap_or_else(|| json!({}))
            .get("strategy")
            .and_then(Value::as_str),
        Some("fast-forward")
    );

    let switch_worktree = run_action(
        root.path(),
        "dashboard.prompts.system.controller.worktree.switchWorktree.describe",
        &json!({"destination": "default"}),
    );
    assert!(switch_worktree.ok);
    assert_eq!(
        switch_worktree
            .payload
            .unwrap_or_else(|| json!({}))
            .get("destination")
            .and_then(Value::as_str),
        Some("default")
    );

    let track_worktree_view_opened = run_action(
        root.path(),
        "dashboard.prompts.system.controller.worktree.trackWorktreeViewOpened.describe",
        &json!({"surface": "worktree_view"}),
    );
    assert!(track_worktree_view_opened.ok);
    assert_eq!(
        track_worktree_view_opened
            .payload
            .unwrap_or_else(|| json!({}))
            .get("surface")
            .and_then(Value::as_str),
        Some("worktree_view")
    );
}
