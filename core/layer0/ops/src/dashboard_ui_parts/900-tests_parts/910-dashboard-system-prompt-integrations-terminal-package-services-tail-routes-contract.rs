#[test]
fn dashboard_system_prompt_integrations_terminal_package_services_tail_routes_contract_wave_910() {
    let root = tempfile::tempdir().expect("tempdir");

    let standalone_process = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.standalone.standaloneTerminalProcess.describe",
        &json!({"lifecycle": "managed"}),
    );
    assert!(standalone_process.ok);
    assert_eq!(
        standalone_process
            .payload
            .unwrap_or_else(|| json!({}))
            .get("lifecycle")
            .and_then(Value::as_str),
        Some("managed")
    );

    let standalone_registry = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.standalone.standaloneTerminalRegistry.describe",
        &json!({"scope": "workspace"}),
    );
    assert!(standalone_registry.ok);
    assert_eq!(
        standalone_registry
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let terminal_types = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.types.describe",
        &json!({"type_set": "core"}),
    );
    assert!(terminal_types.ok);
    assert_eq!(
        terminal_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type_set")
            .and_then(Value::as_str),
        Some("core")
    );

    let package_execa = run_action(
        root.path(),
        "dashboard.prompts.system.packages.execa.describe",
        &json!({"exec_mode": "spawn"}),
    );
    assert!(package_execa.ok);
    assert_eq!(
        package_execa
            .payload
            .unwrap_or_else(|| json!({}))
            .get("exec_mode")
            .and_then(Value::as_str),
        Some("spawn")
    );

    let registry = run_action(
        root.path(),
        "dashboard.prompts.system.registry.describe",
        &json!({"registry_scope": "global"}),
    );
    assert!(registry.ok);
    assert_eq!(
        registry
            .payload
            .unwrap_or_else(|| json!({}))
            .get("registry_scope")
            .and_then(Value::as_str),
        Some("global")
    );

    let env_utils = run_action(
        root.path(),
        "dashboard.prompts.system.services.envUtils.describe",
        &json!({"env_profile": "default"}),
    );
    assert!(env_utils.ok);
    assert_eq!(
        env_utils
            .payload
            .unwrap_or_else(|| json!({}))
            .get("env_profile")
            .and_then(Value::as_str),
        Some("default")
    );

    let account_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.account.clineAccountService.describe",
        &json!({"account_mode": "interactive"}),
    );
    assert!(account_service.ok);
    assert_eq!(
        account_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("account_mode")
            .and_then(Value::as_str),
        Some("interactive")
    );

    let auth_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.auth.authService.describe",
        &json!({"auth_mode": "standard"}),
    );
    assert!(auth_service.ok);
    assert_eq!(
        auth_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("auth_mode")
            .and_then(Value::as_str),
        Some("standard")
    );

    let auth_service_mock = run_action(
        root.path(),
        "dashboard.prompts.system.services.auth.authServiceMock.describe",
        &json!({"mock_mode": "disabled"}),
    );
    assert!(auth_service_mock.ok);
    assert_eq!(
        auth_service_mock
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mock_mode")
            .and_then(Value::as_str),
        Some("disabled")
    );

    let oca_auth_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.auth.oca.ocaAuthService.describe",
        &json!({"provider": "oca"}),
    );
    assert!(oca_auth_service.ok);
    assert_eq!(
        oca_auth_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("oca")
    );
}
