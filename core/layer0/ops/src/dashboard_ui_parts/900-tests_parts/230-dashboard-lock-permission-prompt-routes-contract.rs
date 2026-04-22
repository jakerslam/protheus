#[test]
fn dashboard_lock_and_permission_routes_follow_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let acquire = run_action(
        root.path(),
        "dashboard.locks.acquire",
        &json!({
            "lock_key": "workspace/main",
            "holder": "agent-a",
            "mode": "exclusive"
        }),
    );
    assert!(acquire.ok);
    let acquire_payload = acquire.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        acquire_payload.get("type").and_then(Value::as_str),
        Some("dashboard_lock_acquire")
    );

    let locked = run_action(
        root.path(),
        "dashboard.locks.acquire",
        &json!({
            "lock_key": "workspace/main",
            "holder": "agent-b"
        }),
    );
    assert!(!locked.ok);

    let status = run_action(
        root.path(),
        "dashboard.locks.status",
        &json!({
            "lock_key": "workspace/main"
        }),
    );
    assert!(status.ok);
    let status_payload = status.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        status_payload.get("locked").and_then(Value::as_bool),
        Some(true)
    );

    let release = run_action(
        root.path(),
        "dashboard.locks.release",
        &json!({
            "lock_key": "workspace/main",
            "holder": "agent-a"
        }),
    );
    assert!(release.ok);

    let set_policy = run_action(
        root.path(),
        "dashboard.permissions.setPolicy",
        &json!({
            "allow_commands": ["git status", "read*"],
            "deny_commands": ["rm *"],
            "default_decision": "deny"
        }),
    );
    assert!(set_policy.ok);

    let eval_allow = run_action(
        root.path(),
        "dashboard.permissions.evaluateCommand",
        &json!({
            "command": "git status"
        }),
    );
    assert!(eval_allow.ok);
    let eval_allow_payload = eval_allow.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        eval_allow_payload.get("allowed").and_then(Value::as_bool),
        Some(true)
    );

    let eval_deny = run_action(
        root.path(),
        "dashboard.permissions.evaluateCommand",
        &json!({
            "command": "rm -rf /tmp"
        }),
    );
    assert!(eval_deny.ok);
    let eval_deny_payload = eval_deny.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        eval_deny_payload.get("allowed").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn dashboard_mentions_and_prompt_routes_follow_contract() {
    let root = tempfile::tempdir().expect("tempdir");

    let mentions = run_action(
        root.path(),
        "dashboard.mentions.extract",
        &json!({
            "text": "ping @alice and @bob.about-build now"
        }),
    );
    assert!(mentions.ok);
    let mentions_payload = mentions.payload.unwrap_or_else(|| json!({}));
    assert!(
        mentions_payload
            .pointer("/mentions/0")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("alice")
    );

    let context = run_action(
        root.path(),
        "dashboard.prompts.context.manage",
        &json!({
            "op": "set",
            "key": "active_goal",
            "value": "stabilize lock policy"
        }),
    );
    assert!(context.ok);
    let context_payload = context.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        context_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_context_manage")
    );

    let mcp = run_action(
        root.path(),
        "dashboard.prompts.loadMcpDocumentation",
        &json!({
            "mcp_ids": ["mcp.search", "mcp.files"]
        }),
    );
    assert!(mcp.ok);
    let mcp_payload = mcp.payload.unwrap_or_else(|| json!({}));
    assert!(
        mcp_payload
            .get("loaded_count")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= 2
    );

    let compose = run_action(
        root.path(),
        "dashboard.prompts.response.compose",
        &json!({
            "tone": "concise",
            "summary": "Lock policy updated.",
            "bullets": ["deny destructive commands", "allow safe reads"]
        }),
    );
    assert!(compose.ok);
    let compose_payload = compose.payload.unwrap_or_else(|| json!({}));
    assert!(
        compose_payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("Lock policy updated.")
    );
}
