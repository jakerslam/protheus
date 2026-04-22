fn dashboard_prompt_integrations_terminal_standalone_process_describe(payload: &Value) -> Value {
    let lifecycle = clean_text(
        payload
            .get("lifecycle")
            .and_then(Value::as_str)
            .unwrap_or("managed"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_terminal_standalone_process_describe",
        "lifecycle": lifecycle
    })
}

fn dashboard_prompt_integrations_terminal_standalone_registry_describe(payload: &Value) -> Value {
    let scope = clean_text(
        payload
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_terminal_standalone_registry_describe",
        "scope": scope
    })
}

fn dashboard_prompt_integrations_terminal_types_describe(payload: &Value) -> Value {
    let type_set = clean_text(
        payload
            .get("type_set")
            .and_then(Value::as_str)
            .unwrap_or("core"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_terminal_types_describe",
        "type_set": type_set
    })
}

fn dashboard_prompt_packages_execa_describe(payload: &Value) -> Value {
    let exec_mode = clean_text(
        payload
            .get("exec_mode")
            .and_then(Value::as_str)
            .unwrap_or("spawn"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_packages_execa_describe",
        "exec_mode": exec_mode
    })
}

fn dashboard_prompt_registry_describe(payload: &Value) -> Value {
    let registry_scope = clean_text(
        payload
            .get("registry_scope")
            .and_then(Value::as_str)
            .unwrap_or("global"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_registry_describe",
        "registry_scope": registry_scope
    })
}

fn dashboard_prompt_services_env_utils_describe(payload: &Value) -> Value {
    let env_profile = clean_text(
        payload
            .get("env_profile")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_env_utils_describe",
        "env_profile": env_profile
    })
}

fn dashboard_prompt_services_account_cline_account_service_describe(payload: &Value) -> Value {
    let account_mode = clean_text(
        payload
            .get("account_mode")
            .and_then(Value::as_str)
            .unwrap_or("interactive"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_account_cline_account_service_describe",
        "account_mode": account_mode
    })
}

fn dashboard_prompt_services_auth_auth_service_describe(payload: &Value) -> Value {
    let auth_mode = clean_text(
        payload
            .get("auth_mode")
            .and_then(Value::as_str)
            .unwrap_or("standard"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_auth_auth_service_describe",
        "auth_mode": auth_mode
    })
}

fn dashboard_prompt_services_auth_auth_service_mock_describe(payload: &Value) -> Value {
    let mock_mode = clean_text(
        payload
            .get("mock_mode")
            .and_then(Value::as_str)
            .unwrap_or("disabled"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_auth_auth_service_mock_describe",
        "mock_mode": mock_mode
    })
}

fn dashboard_prompt_services_auth_oca_auth_service_describe(payload: &Value) -> Value {
    let provider = clean_text(
        payload
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("oca"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_auth_oca_auth_service_describe",
        "provider": provider
    })
}

fn dashboard_prompt_hosts_surface_tail_integrations_terminal_package_services_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.integrations.terminal.standalone.standaloneTerminalProcess.describe" => {
            Some(dashboard_prompt_integrations_terminal_standalone_process_describe(payload))
        }
        "dashboard.prompts.system.integrations.terminal.standalone.standaloneTerminalRegistry.describe" => {
            Some(dashboard_prompt_integrations_terminal_standalone_registry_describe(payload))
        }
        "dashboard.prompts.system.integrations.terminal.types.describe" => {
            Some(dashboard_prompt_integrations_terminal_types_describe(payload))
        }
        "dashboard.prompts.system.packages.execa.describe" => {
            Some(dashboard_prompt_packages_execa_describe(payload))
        }
        "dashboard.prompts.system.registry.describe" => {
            Some(dashboard_prompt_registry_describe(payload))
        }
        "dashboard.prompts.system.services.envUtils.describe" => {
            Some(dashboard_prompt_services_env_utils_describe(payload))
        }
        "dashboard.prompts.system.services.account.clineAccountService.describe" => {
            Some(dashboard_prompt_services_account_cline_account_service_describe(payload))
        }
        "dashboard.prompts.system.services.auth.authService.describe" => {
            Some(dashboard_prompt_services_auth_auth_service_describe(payload))
        }
        "dashboard.prompts.system.services.auth.authServiceMock.describe" => {
            Some(dashboard_prompt_services_auth_auth_service_mock_describe(payload))
        }
        "dashboard.prompts.system.services.auth.oca.ocaAuthService.describe" => {
            Some(dashboard_prompt_services_auth_oca_auth_service_describe(payload))
        }
        _ => dashboard_prompt_hosts_surface_tail_services_auth_browser_error_route_extension(
            root,
            normalized,
            payload,
        ),
    }
}
include!("084-dashboard-system-prompt-services-auth-browser-error-tail-helpers.rs");
