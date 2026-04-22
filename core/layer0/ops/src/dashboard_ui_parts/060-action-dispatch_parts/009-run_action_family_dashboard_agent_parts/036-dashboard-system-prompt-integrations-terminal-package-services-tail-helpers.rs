fn dashboard_prompt_integrations_terminal_standalone_process_describe(payload: &Value) -> Value {
    let process_id = clean_text(
        payload
            .get("process_id")
            .and_then(Value::as_str)
            .unwrap_or("proc-default"),
        160,
    );
    let status = clean_text(
        payload
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("running"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_terminal_standalone_process_describe",
        "process_id": process_id,
        "status": status
    })
}

fn dashboard_prompt_integrations_terminal_standalone_registry_describe(payload: &Value) -> Value {
    let namespace = clean_text(
        payload
            .get("namespace")
            .and_then(Value::as_str)
            .unwrap_or("workspace"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_terminal_standalone_registry_describe",
        "namespace": namespace
    })
}

fn dashboard_prompt_integrations_terminal_types_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_integrations_terminal_types_describe",
        "contracts": ["command_request", "command_result", "terminal_session"]
    })
}

fn dashboard_prompt_packages_execa_describe(payload: &Value) -> Value {
    let command = clean_text(payload.get("command").and_then(Value::as_str).unwrap_or(""), 2400);
    let timeout_ms = payload
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(30000);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_packages_execa_describe",
        "command": command,
        "timeout_ms": timeout_ms
    })
}

fn dashboard_prompt_system_registry_describe(payload: &Value) -> Value {
    let scope = clean_text(
        payload
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or("runtime"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_registry_describe",
        "scope": scope
    })
}

fn dashboard_prompt_services_env_utils_describe(payload: &Value) -> Value {
    let key = clean_text(payload.get("key").and_then(Value::as_str).unwrap_or(""), 200);
    let required = payload
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_env_utils_describe",
        "key": key,
        "required": required
    })
}

fn dashboard_prompt_services_account_cline_account_describe(payload: &Value) -> Value {
    let account_id = clean_text(
        payload
            .get("account_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_account_cline_account_describe",
        "account_id": account_id
    })
}

fn dashboard_prompt_services_auth_service_describe(payload: &Value) -> Value {
    let provider = clean_text(
        payload
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_auth_service_describe",
        "provider": provider
    })
}

fn dashboard_prompt_services_auth_service_mock_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("test"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_auth_service_mock_describe",
        "mode": mode
    })
}

fn dashboard_prompt_services_auth_oca_service_describe(payload: &Value) -> Value {
    let tenant = clean_text(
        payload
            .get("tenant")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_auth_oca_service_describe",
        "tenant": tenant
    })
}

fn dashboard_prompt_integrations_terminal_package_services_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.integrations.terminal.standaloneTerminalProcess.describe" => {
            Some(dashboard_prompt_integrations_terminal_standalone_process_describe(payload))
        }
        "dashboard.prompts.system.integrations.terminal.standaloneTerminalRegistry.describe" => {
            Some(dashboard_prompt_integrations_terminal_standalone_registry_describe(payload))
        }
        "dashboard.prompts.system.integrations.terminal.types.describe" => {
            Some(dashboard_prompt_integrations_terminal_types_describe())
        }
        "dashboard.prompts.system.packages.execa.describe" => {
            Some(dashboard_prompt_packages_execa_describe(payload))
        }
        "dashboard.prompts.system.registry.describe" => {
            Some(dashboard_prompt_system_registry_describe(payload))
        }
        "dashboard.prompts.system.services.envUtils.describe" => {
            Some(dashboard_prompt_services_env_utils_describe(payload))
        }
        "dashboard.prompts.system.services.account.clineAccountService.describe" => {
            Some(dashboard_prompt_services_account_cline_account_describe(payload))
        }
        "dashboard.prompts.system.services.auth.authService.describe" => {
            Some(dashboard_prompt_services_auth_service_describe(payload))
        }
        "dashboard.prompts.system.services.auth.authServiceMock.describe" => {
            Some(dashboard_prompt_services_auth_service_mock_describe(payload))
        }
        "dashboard.prompts.system.services.auth.oca.ocaAuthService.describe" => {
            Some(dashboard_prompt_services_auth_oca_service_describe(payload))
        }
        _ => dashboard_prompt_services_auth_browser_error_tail_route_extension(root, normalized, payload),
    }
}

include!("037-dashboard-system-prompt-services-auth-browser-error-tail-helpers.rs");
