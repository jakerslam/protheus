#[test]
fn dashboard_system_prompt_utils_core_tail_routes_contract_wave_590() {
    let root = tempfile::tempdir().expect("tempdir");

    let announcements = run_action(
        root.path(),
        "dashboard.prompts.system.utils.announcements.describe",
        &json!({"channel": "release"}),
    );
    assert!(announcements.ok);
    assert_eq!(
        announcements
            .payload
            .unwrap_or_else(|| json!({}))
            .get("channel")
            .and_then(Value::as_str),
        Some("release")
    );

    let cli_detector = run_action(
        root.path(),
        "dashboard.prompts.system.utils.cliDetector.describe",
        &json!({"shell": "zsh"}),
    );
    assert!(cli_detector.ok);
    assert_eq!(
        cli_detector
            .payload
            .unwrap_or_else(|| json!({}))
            .get("shell")
            .and_then(Value::as_str),
        Some("zsh")
    );

    let cost = run_action(
        root.path(),
        "dashboard.prompts.system.utils.cost.describe",
        &json!({"model": "gpt-5"}),
    );
    assert!(cost.ok);
    assert_eq!(
        cost.payload
            .unwrap_or_else(|| json!({}))
            .get("model")
            .and_then(Value::as_str),
        Some("gpt-5")
    );

    let env = run_action(
        root.path(),
        "dashboard.prompts.system.utils.env.describe",
        &json!({"profile": "default"}),
    );
    assert!(env.ok);
    assert_eq!(
        env.payload
            .unwrap_or_else(|| json!({}))
            .get("profile")
            .and_then(Value::as_str),
        Some("default")
    );

    let env_expansion = run_action(
        root.path(),
        "dashboard.prompts.system.utils.envExpansion.describe",
        &json!({"pattern": "${HOME}"}),
    );
    assert!(env_expansion.ok);
    assert_eq!(
        env_expansion
            .payload
            .unwrap_or_else(|| json!({}))
            .get("pattern")
            .and_then(Value::as_str),
        Some("${HOME}")
    );

    let fs = run_action(
        root.path(),
        "dashboard.prompts.system.utils.fs.describe",
        &json!({"operation": "read"}),
    );
    assert!(fs.ok);
    assert_eq!(
        fs.payload
            .unwrap_or_else(|| json!({}))
            .get("operation")
            .and_then(Value::as_str),
        Some("read")
    );

    let git_worktree = run_action(
        root.path(),
        "dashboard.prompts.system.utils.gitWorktree.describe",
        &json!({"action": "attach"}),
    );
    assert!(git_worktree.ok);
    assert_eq!(
        git_worktree
            .payload
            .unwrap_or_else(|| json!({}))
            .get("action")
            .and_then(Value::as_str),
        Some("attach")
    );

    let git = run_action(
        root.path(),
        "dashboard.prompts.system.utils.git.describe",
        &json!({"verb": "status"}),
    );
    assert!(git.ok);
    assert_eq!(
        git.payload
            .unwrap_or_else(|| json!({}))
            .get("verb")
            .and_then(Value::as_str),
        Some("status")
    );

    let github_url_utils = run_action(
        root.path(),
        "dashboard.prompts.system.utils.githubUrlUtils.describe",
        &json!({"host": "github.com"}),
    );
    assert!(github_url_utils.ok);
    assert_eq!(
        github_url_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("host")
            .and_then(Value::as_str),
        Some("github.com")
    );

    let mcp_auth = run_action(
        root.path(),
        "dashboard.prompts.system.utils.mcpAuth.describe",
        &json!({"flow": "device_code"}),
    );
    assert!(mcp_auth.ok);
    assert_eq!(
        mcp_auth
            .payload
            .unwrap_or_else(|| json!({}))
            .get("flow")
            .and_then(Value::as_str),
        Some("device_code")
    );
}
