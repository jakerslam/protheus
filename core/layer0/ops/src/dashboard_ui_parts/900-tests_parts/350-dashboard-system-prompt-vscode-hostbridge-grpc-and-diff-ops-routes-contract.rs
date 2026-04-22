#[test]
fn dashboard_system_prompt_vscode_hostbridge_grpc_routes_contract_wave_350() {
    let root = tempfile::tempdir().expect("tempdir");

    let commit = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.commitMessageGenerator.describe",
        &json!({"style": "conventional"}),
    );
    assert!(commit.ok);
    assert_eq!(
        commit
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_commit_message_generator_describe")
    );

    let handler = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.grpcHandler.describe",
        &json!({}),
    );
    assert!(handler.ok);
    assert_eq!(
        handler
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_hostbridge_grpc_handler_describe")
    );

    let service = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.grpcService.describe",
        &json!({}),
    );
    assert!(service.ok);
    assert_eq!(
        service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_hostbridge_grpc_service_describe")
    );

    let client_base = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.client.hostGrpcClientBase.describe",
        &json!({}),
    );
    assert!(client_base.ok);
    assert_eq!(
        client_base
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_host_grpc_client_base_describe")
    );

    let client = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.client.hostGrpcClient.describe",
        &json!({"channel": "main"}),
    );
    assert!(client.ok);
    assert_eq!(
        client
            .payload
            .unwrap_or_else(|| json!({}))
            .get("channel")
            .and_then(Value::as_str),
        Some("main")
    );
}

#[test]
fn dashboard_system_prompt_vscode_hostbridge_diff_routes_contract_wave_350() {
    let root = tempfile::tempdir().expect("tempdir");

    let close_all = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.closeAllDiffs.describe",
        &json!({}),
    );
    assert!(close_all.ok);
    assert_eq!(
        close_all
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_diff_close_all_describe")
    );

    let get_doc = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.getDocumentText.describe",
        &json!({"uri": "file:///tmp/a.ts"}),
    );
    assert!(get_doc.ok);
    assert_eq!(
        get_doc
            .payload
            .unwrap_or_else(|| json!({}))
            .get("uri")
            .and_then(Value::as_str),
        Some("file:///tmp/a.ts")
    );

    let open_diff = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.openDiff.describe",
        &json!({"left": "a.ts", "right": "b.ts"}),
    );
    assert!(open_diff.ok);
    assert_eq!(
        open_diff
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_diff_open_diff_describe")
    );

    let open_multi = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.openMultiFileDiff.describe",
        &json!({"files": ["a.ts", "b.ts"]}),
    );
    assert!(open_multi.ok);
    assert_eq!(
        open_multi
            .payload
            .unwrap_or_else(|| json!({}))
            .get("file_count")
            .and_then(Value::as_i64),
        Some(2)
    );

    let replace = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.hostbridge.diff.replaceText.describe",
        &json!({"uri": "a.ts", "replacement": "const x = 1;"}),
    );
    assert!(replace.ok);
    assert_eq!(
        replace
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_diff_replace_text_describe")
    );
}
