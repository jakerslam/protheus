#[test]
fn dashboard_system_prompt_hostbridge_diff_grpc_tail_routes_contract_wave_820() {
    let root = tempfile::tempdir().expect("tempdir");

    let commit = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.commitMessageGenerator.describe",
        &json!({"style": "conventional"}),
    );
    assert!(commit.ok);
    assert_eq!(
        commit
            .payload
            .unwrap_or_else(|| json!({}))
            .get("style")
            .and_then(Value::as_str),
        Some("conventional")
    );

    let grpc_handler = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.grpcHandler.describe",
        &json!({"handler_mode": "streaming"}),
    );
    assert!(grpc_handler.ok);
    assert_eq!(
        grpc_handler
            .payload
            .unwrap_or_else(|| json!({}))
            .get("handler_mode")
            .and_then(Value::as_str),
        Some("streaming")
    );

    let grpc_service = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.grpcService.describe",
        &json!({"service_mode": "grpc"}),
    );
    assert!(grpc_service.ok);
    assert_eq!(
        grpc_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("service_mode")
            .and_then(Value::as_str),
        Some("grpc")
    );

    let client_base = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.client.hostGrpcClientBase.describe",
        &json!({"transport": "grpc"}),
    );
    assert!(client_base.ok);
    assert_eq!(
        client_base
            .payload
            .unwrap_or_else(|| json!({}))
            .get("transport")
            .and_then(Value::as_str),
        Some("grpc")
    );

    let client = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.client.hostGrpcClient.describe",
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

    let close_all = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.closeAllDiffs.describe",
        &json!({"scope": "workspace"}),
    );
    assert!(close_all.ok);
    assert_eq!(
        close_all
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let get_document_text = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.getDocumentText.describe",
        &json!({"uri": "file:///tmp/a.ts"}),
    );
    assert!(get_document_text.ok);
    assert_eq!(
        get_document_text
            .payload
            .unwrap_or_else(|| json!({}))
            .get("uri")
            .and_then(Value::as_str),
        Some("file:///tmp/a.ts")
    );

    let open_diff = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.openDiff.describe",
        &json!({"left": "a.ts", "right": "b.ts"}),
    );
    assert!(open_diff.ok);
    assert_eq!(
        open_diff
            .payload
            .unwrap_or_else(|| json!({}))
            .get("right")
            .and_then(Value::as_str),
        Some("b.ts")
    );

    let open_multi = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.openMultiFileDiff.describe",
        &json!({"files": ["a.ts", "b.ts", "c.ts"]}),
    );
    assert!(open_multi.ok);
    assert_eq!(
        open_multi
            .payload
            .unwrap_or_else(|| json!({}))
            .get("file_count")
            .and_then(Value::as_i64),
        Some(3)
    );

    let replace_text = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.surface.vscode.hostbridge.diff.replaceText.describe",
        &json!({"uri": "a.ts", "replacement": "const x = 1;"}),
    );
    assert!(replace_text.ok);
    assert_eq!(
        replace_text
            .payload
            .unwrap_or_else(|| json!({}))
            .get("replacement_len")
            .and_then(Value::as_i64),
        Some("const x = 1;".len() as i64)
    );
}
