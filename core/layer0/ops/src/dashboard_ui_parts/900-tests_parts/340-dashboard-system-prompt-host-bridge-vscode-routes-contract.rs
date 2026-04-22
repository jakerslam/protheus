#[test]
fn dashboard_system_prompt_external_host_bridge_routes_contract_wave_340() {
    let root = tempfile::tempdir().expect("tempdir");

    let diff = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.external.diffviewProvider.describe",
        &json!({"lane": "external_diff"}),
    );
    assert!(diff.ok);
    assert_eq!(
        diff.payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_external_diffview_provider_describe")
    );

    let webview = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.external.webviewProvider.describe",
        &json!({"host": "external"}),
    );
    assert!(webview.ok);
    assert_eq!(
        webview
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_external_webview_provider_describe")
    );

    let grpc = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.external.grpcTypes.describe",
        &json!({}),
    );
    assert!(grpc.ok);
    assert!(
        grpc.payload
            .unwrap_or_else(|| json!({}))
            .get("types")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
    );

    let manager = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.external.hostBridgeClientManager.describe",
        &json!({"strategy": "reuse"}),
    );
    assert!(manager.ok);
    assert_eq!(
        manager
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_external_bridge_client_manager_describe")
    );
}

#[test]
fn dashboard_system_prompt_vscode_host_routes_contract_wave_340() {
    let root = tempfile::tempdir().expect("tempdir");

    let provider_types = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.providerTypes.describe",
        &json!({}),
    );
    assert!(provider_types.ok);
    assert_eq!(
        provider_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_provider_types_describe")
    );

    let provider = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.provider.describe",
        &json!({"selected": "vscode"}),
    );
    assert!(provider.ok);
    assert_eq!(
        provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_provider_describe")
    );

    let decoration = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.decorationController.describe",
        &json!({}),
    );
    assert!(decoration.ok);
    assert_eq!(
        decoration
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_decoration_controller_describe")
    );

    let notebook = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.notebookDiffView.describe",
        &json!({}),
    );
    assert!(notebook.ok);
    assert_eq!(
        notebook
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_notebook_diff_view_describe")
    );

    let diffview = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.diffviewProvider.describe",
        &json!({}),
    );
    assert!(diffview.ok);
    assert_eq!(
        diffview
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_diffview_provider_describe")
    );

    let webview = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.webviewProvider.describe",
        &json!({}),
    );
    assert!(webview.ok);
    assert_eq!(
        webview
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_host_vscode_webview_provider_describe")
    );
}
