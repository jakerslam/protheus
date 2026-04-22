#[test]
fn dashboard_system_prompt_workspace_routes_contract_wave_330() {
    let root = tempfile::tempdir().expect("tempdir");

    let roots = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.rootManager.describe",
        &json!({"roots": ["/repo/a", "/repo/b"]}),
    );
    assert!(roots.ok);
    let roots_payload = roots.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        roots_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_workspace_root_manager_describe")
    );
    assert_eq!(roots_payload.get("root_count").and_then(Value::as_i64), Some(2));

    let setup = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.setup.plan",
        &json!({"workspace": "/repo/a", "defaults": true}),
    );
    assert!(setup.ok);
    let setup_payload = setup.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        setup_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_workspace_setup_plan")
    );

    let parse = run_action(
        root.path(),
        "dashboard.prompts.system.workspace.parseInlinePath",
        &json!({"inline_path": "src/main.ts:120"}),
    );
    assert!(parse.ok);
    assert_eq!(
        parse.payload
            .unwrap_or_else(|| json!({}))
            .get("path")
            .and_then(Value::as_str),
        Some("src/main.ts")
    );
}

#[test]
fn dashboard_system_prompt_extension_host_routes_contract_wave_330() {
    let root = tempfile::tempdir().expect("tempdir");

    let ext = run_action(
        root.path(),
        "dashboard.prompts.system.extension.bootstrap.describe",
        &json!({"host": "vscode"}),
    );
    assert!(ext.ok);
    let ext_payload = ext.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        ext_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_extension_bootstrap_describe")
    );

    let auth = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.external.auth.describe",
        &json!({"provider": "external"}),
    );
    assert!(auth.ok);
    let auth_payload = auth.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        auth_payload.get("type").and_then(Value::as_str),
        Some("dashboard_prompts_system_host_external_auth_describe")
    );
    assert!(
        auth_payload
            .get("flows")
            .and_then(Value::as_array)
            .map(|rows| rows.len() >= 3)
            .unwrap_or(false)
    );

    let comments = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.external.commentReview.describe",
        &json!({"review_mode": "inline"}),
    );
    assert!(comments.ok);
    assert_eq!(
        comments
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_external_comment_review_describe")
    );
}
