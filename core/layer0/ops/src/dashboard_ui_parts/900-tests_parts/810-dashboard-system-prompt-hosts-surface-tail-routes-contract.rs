#[test]
fn dashboard_system_prompt_hosts_surface_tail_routes_contract_wave_810() {
    let root = tempfile::tempdir().expect("tempdir");

    let external_diffview_provider = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.external.externalDiffviewProvider.describe",
        &json!({"provider_mode": "external"}),
    );
    assert!(external_diffview_provider.ok);
    assert_eq!(
        external_diffview_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider_mode")
            .and_then(Value::as_str),
        Some("external")
    );

    let external_webview_provider = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.external.externalWebviewProvider.describe",
        &json!({"provider_mode": "external"}),
    );
    assert!(external_webview_provider.ok);
    assert_eq!(
        external_webview_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider_mode")
            .and_then(Value::as_str),
        Some("external")
    );

    let external_grpc_types = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.external.grpcTypes.describe",
        &json!({"grpc_profile": "default"}),
    );
    assert!(external_grpc_types.ok);
    assert_eq!(
        external_grpc_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("grpc_profile")
            .and_then(Value::as_str),
        Some("default")
    );

    let host_bridge_client_manager = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.external.hostBridgeClientManager.describe",
        &json!({"bridge_mode": "managed"}),
    );
    assert!(host_bridge_client_manager.ok);
    assert_eq!(
        host_bridge_client_manager
            .payload
            .unwrap_or_else(|| json!({}))
            .get("bridge_mode")
            .and_then(Value::as_str),
        Some("managed")
    );

    let host_provider_types = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.hostProviderTypes.describe",
        &json!({"provider_types": "default"}),
    );
    assert!(host_provider_types.ok);
    assert_eq!(
        host_provider_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider_types")
            .and_then(Value::as_str),
        Some("default")
    );

    let host_provider = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.hostProvider.describe",
        &json!({"host_mode": "vscode"}),
    );
    assert!(host_provider.ok);
    assert_eq!(
        host_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("host_mode")
            .and_then(Value::as_str),
        Some("vscode")
    );

    let vscode_decoration_controller = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.decorationController.describe",
        &json!({"decoration_mode": "inline"}),
    );
    assert!(vscode_decoration_controller.ok);
    assert_eq!(
        vscode_decoration_controller
            .payload
            .unwrap_or_else(|| json!({}))
            .get("decoration_mode")
            .and_then(Value::as_str),
        Some("inline")
    );

    let vscode_notebook_diffview = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.notebookDiffView.describe",
        &json!({"diff_mode": "notebook"}),
    );
    assert!(vscode_notebook_diffview.ok);
    assert_eq!(
        vscode_notebook_diffview
            .payload
            .unwrap_or_else(|| json!({}))
            .get("diff_mode")
            .and_then(Value::as_str),
        Some("notebook")
    );

    let vscode_diffview_provider = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.vscodeDiffViewProvider.describe",
        &json!({"provider_mode": "vscode"}),
    );
    assert!(vscode_diffview_provider.ok);
    assert_eq!(
        vscode_diffview_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider_mode")
            .and_then(Value::as_str),
        Some("vscode")
    );

    let vscode_webview_provider = run_action(
        root.path(),
        "dashboard.prompts.system.hosts.vscode.vscodeWebviewProvider.describe",
        &json!({"provider_mode": "vscode"}),
    );
    assert!(vscode_webview_provider.ok);
    assert_eq!(
        vscode_webview_provider
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider_mode")
            .and_then(Value::as_str),
        Some("vscode")
    );
}
