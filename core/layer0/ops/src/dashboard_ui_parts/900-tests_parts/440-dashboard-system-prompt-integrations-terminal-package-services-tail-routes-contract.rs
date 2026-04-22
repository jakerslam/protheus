#[test]
fn dashboard_system_prompt_integrations_terminal_package_services_tail_routes_contract_wave_440() {
    let root = tempfile::tempdir().expect("tempdir");

    let standalone_process = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.standaloneTerminalProcess.describe",
        &json!({"process_id": "proc-1", "status": "idle"}),
    );
    assert!(standalone_process.ok);
    let standalone_process_payload = standalone_process.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        standalone_process_payload
            .get("process_id")
            .and_then(Value::as_str),
        Some("proc-1")
    );
    assert_eq!(
        standalone_process_payload.get("status").and_then(Value::as_str),
        Some("idle")
    );

    let standalone_registry = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.standaloneTerminalRegistry.describe",
        &json!({"namespace": "workspace"}),
    );
    assert!(standalone_registry.ok);
    assert_eq!(
        standalone_registry
            .payload
            .unwrap_or_else(|| json!({}))
            .get("namespace")
            .and_then(Value::as_str),
        Some("workspace")
    );

    let terminal_types = run_action(
        root.path(),
        "dashboard.prompts.system.integrations.terminal.types.describe",
        &json!({}),
    );
    assert!(terminal_types.ok);
    assert_eq!(
        terminal_types
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_integrations_terminal_types_describe")
    );

    let execa = run_action(
        root.path(),
        "dashboard.prompts.system.packages.execa.describe",
        &json!({"command": "npm test", "timeout_ms": 15000}),
    );
    assert!(execa.ok);
    let execa_payload = execa.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        execa_payload.get("command").and_then(Value::as_str),
        Some("npm test")
    );
    assert_eq!(
        execa_payload.get("timeout_ms").and_then(Value::as_u64),
        Some(15000)
    );

    let registry = run_action(
        root.path(),
        "dashboard.prompts.system.registry.describe",
        &json!({"scope": "runtime"}),
    );
    assert!(registry.ok);
    assert_eq!(
        registry
            .payload
            .unwrap_or_else(|| json!({}))
            .get("scope")
            .and_then(Value::as_str),
        Some("runtime")
    );

    let env_utils = run_action(
        root.path(),
        "dashboard.prompts.system.services.envUtils.describe",
        &json!({"key": "OPENAI_API_KEY", "required": true}),
    );
    assert!(env_utils.ok);
    let env_utils_payload = env_utils.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        env_utils_payload.get("key").and_then(Value::as_str),
        Some("OPENAI_API_KEY")
    );
    assert_eq!(
        env_utils_payload.get("required").and_then(Value::as_bool),
        Some(true)
    );

    let account_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.account.clineAccountService.describe",
        &json!({"account_id": "acct-1"}),
    );
    assert!(account_service.ok);
    assert_eq!(
        account_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("account_id")
            .and_then(Value::as_str),
        Some("acct-1")
    );

    let auth_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.auth.authService.describe",
        &json!({"provider": "oca"}),
    );
    assert!(auth_service.ok);
    assert_eq!(
        auth_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("provider")
            .and_then(Value::as_str),
        Some("oca")
    );

    let auth_service_mock = run_action(
        root.path(),
        "dashboard.prompts.system.services.auth.authServiceMock.describe",
        &json!({"mode": "test"}),
    );
    assert!(auth_service_mock.ok);
    assert_eq!(
        auth_service_mock
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("test")
    );

    let oca_auth_service = run_action(
        root.path(),
        "dashboard.prompts.system.services.auth.oca.ocaAuthService.describe",
        &json!({"tenant": "prod"}),
    );
    assert!(oca_auth_service.ok);
    assert_eq!(
        oca_auth_service
            .payload
            .unwrap_or_else(|| json!({}))
            .get("tenant")
            .and_then(Value::as_str),
        Some("prod")
    );
}
